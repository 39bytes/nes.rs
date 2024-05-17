use std::{cell::RefCell, rc::Rc};

use crate::renderer::{Color, Pixel, Sprite};

use super::{
    bits::IntoBit,
    cartridge::{Cartridge, Mirroring},
    cpu::Cpu6502,
    palette::Palette,
};
use bitflags::bitflags;
use modular_bitfield::prelude::*;

pub enum PatternTable {
    Left,
    Right,
}

impl From<bool> for PatternTable {
    fn from(value: bool) -> Self {
        match value {
            false => PatternTable::Left,
            true => PatternTable::Right,
        }
    }
}

impl PatternTable {
    fn addr(self) -> u16 {
        match self {
            PatternTable::Left => 0x0000,
            PatternTable::Right => 0x1000,
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PpuCtrl: u8 {
        /// Base nametable address
        /// 0: $2000; 1: $2400; 2: $2800; 3: $2C00
        const NametableLSB = 1 << 0;
        const NametableMSB = 1 << 1;
        /// VRAM address increment per CPU read/write of PPUDATA
        /// 0: add 1, going across; 1: add 32, going down
        const VRamAddressIncrement = 1 << 2;
        /// Sprite pattern table address for 8x8 sprites
        /// 0: $0000; 1: $1000; ignored in 8x16 mode
        const SpritePatternTable = 1 << 3;
        /// Background pattern table address
        /// 0: $0000; 1: $1000
        const BackgroundPatternTable = 1 << 4;
        /// Sprite size
        /// 0: 8x8; 1: 8x16
        const SpriteSize = 1 << 5;
        const MasterSlave = 1 << 6;
        /// Generate NMI at start of vertical blanking interval
        const GenerateNMI = 1 << 7;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PpuMask: u8 {
        const Greyscale = 1 << 0;
        const ShowBackgroundLeft = 1 << 1;
        const ShowSpritesLeft = 1 << 2;
        const ShowBackground = 1 << 3;
        const ShowSprites = 1 << 4;
        const EmphasizeRed = 1 << 5;
        const EmphasizeGreen = 1 << 6;
        const EmphasizeBlue = 1 << 7;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PpuStatus: u8 {
        const SpriteOverflow = 1 << 5;
        const Sprite0Hit = 1 << 6;
        const VerticalBlank = 1 << 7;
    }
}

#[bitfield]
#[derive(Clone, Copy, Debug)]
struct VRAMAddr {
    coarse_x: B5,
    coarse_y: B5,
    nametable_x: B1,
    nametable_y: B1,
    fine_y: B3,
    #[skip]
    padding: B1,
}

impl VRAMAddr {
    fn increment_coarse_x(&mut self) {
        self.set_coarse_x(self.coarse_x() + 1);
    }

    fn increment_coarse_y(&mut self) {
        self.set_coarse_y(self.coarse_y() + 1);
    }

    fn increment_nametable_x(&mut self) {
        self.set_nametable_x((self.nametable_x() + 1) % 2);
    }

    fn increment_nametable_y(&mut self) {
        self.set_nametable_y((self.nametable_x() + 1) % 2);
    }

    fn increment_fine_y(&mut self) {
        self.set_fine_y(self.fine_y() + 1);
    }
}

impl From<u16> for VRAMAddr {
    fn from(addr: u16) -> Self {
        let low = addr & 0x00FF;
        let high = addr >> 8;

        VRAMAddr::from_bytes([low as u8, high as u8])
    }
}

impl From<VRAMAddr> for u16 {
    fn from(vram_addr: VRAMAddr) -> Self {
        let [low, high] = vram_addr.into_bytes();

        ((high as u16) << 8) | low as u16
    }
}

pub struct PpuClockResult {
    pub pixel: Option<Pixel>,
    pub nmi: bool,
}

const PALETTE_RAM_SIZE: usize = 32;
const NAMETABLE_SIZE: usize = 1024;
const OAM_SIZE: usize = 256;

struct PpuSprite {
    x: u8,
    y: u8,
    tile_id: u8,
    attribute: u8,
}

pub struct Ppu {
    palette: Palette,

    // X and Y positions that the PPU is currently rendering
    cycle: u16,
    scanline: i16,

    // Registers
    ctrl: PpuCtrl,
    mask: PpuMask,
    status: PpuStatus,
    oam_addr: u8,

    // Internal PPU registers
    // See https://www.nesdev.org/wiki/PPU_scrolling
    vram_addr: VRAMAddr,
    temp_vram_addr: VRAMAddr,
    fine_x: u8,

    write_latch: bool,
    data_buffer: u8,

    // Buffer variables for holding data for the next render cycle
    next_bg_tile_id: u8,
    next_bg_tile_palette_id: u8,
    next_bg_tile_lsb: u8,
    next_bg_tile_msb: u8,

    // Shift registers used for rendering
    bg_tile_low_shifter: u16,
    bg_tile_high_shifter: u16,
    bg_tile_palette_low_shifter: u16,
    bg_tile_palette_high_shifter: u16,

    // Sprite rendering
    scanline_sprites: Vec<PpuSprite>,
    sprite_tile_low_shifter: [u8; 8],
    sprite_tile_high_shifter: [u8; 8],

    // Memory
    nametable_ram: [u8; NAMETABLE_SIZE * 2],
    palette_ram: [u8; PALETTE_RAM_SIZE],
    oam: [u8; OAM_SIZE],

    // Other components
    cpu: Rc<RefCell<Cpu6502>>,
    cartridge: Option<Rc<RefCell<Cartridge>>>,
}

impl Ppu {
    pub fn new(palette: Palette, cpu: Rc<RefCell<Cpu6502>>) -> Self {
        Ppu {
            palette,

            scanline: -1,
            cycle: 0,

            write_latch: false,
            data_buffer: 0x00,

            ctrl: PpuCtrl::empty(),
            mask: PpuMask::empty(),
            status: PpuStatus::VerticalBlank,
            oam_addr: 0x00,

            vram_addr: VRAMAddr::new(),
            temp_vram_addr: VRAMAddr::new(),
            fine_x: 0x00,

            next_bg_tile_id: 0x00,
            next_bg_tile_palette_id: 0x00,
            next_bg_tile_lsb: 0x00,
            next_bg_tile_msb: 0x00,

            bg_tile_low_shifter: 0x0000,
            bg_tile_high_shifter: 0x0000,
            bg_tile_palette_low_shifter: 0x0000,
            bg_tile_palette_high_shifter: 0x0000,

            scanline_sprites: Vec::new(),
            sprite_tile_low_shifter: [0; 8],
            sprite_tile_high_shifter: [0; 8],

            nametable_ram: [0; NAMETABLE_SIZE * 2],
            palette_ram: [0; PALETTE_RAM_SIZE],
            oam: [0; OAM_SIZE],

            cpu,
            cartridge: None,
        }
    }

    pub fn ctrl(&self) -> PpuCtrl {
        self.ctrl
    }

    pub fn mask(&self) -> PpuMask {
        self.mask
    }

    pub fn status(&self) -> PpuStatus {
        self.status
    }

    pub fn oam_addr(&self) -> u8 {
        self.oam_addr
    }

    pub fn addr(&self) -> u16 {
        self.vram_addr.into()
    }

    pub fn data(&self) -> u8 {
        self.data_buffer
    }

    pub fn scanline(&self) -> i16 {
        self.scanline
    }

    pub fn cycle(&self) -> u16 {
        self.cycle
    }

    pub fn nametables(&self) -> &[u8] {
        &self.nametable_ram
    }

    pub fn load_cartridge(&mut self, cartridge: Rc<RefCell<Cartridge>>) {
        self.cartridge = Some(cartridge);
    }

    /// See: https://www.nesdev.org/wiki/PPU_rendering
    /// for details on how this works.
    pub fn clock(&mut self) -> PpuClockResult {
        // Rendering during the visible region
        if self.scanline >= -1 && self.scanline < 240 {
            // Rendering a new frame so reset vertical blank
            if self.scanline == -1 && self.cycle == 1 {
                self.status.set(PpuStatus::VerticalBlank, false);
            }

            // TODO: Check that this is correct
            let is_visible_region = self.cycle >= 2 && self.cycle < 258;
            let is_preparing_next_scanline = self.cycle >= 321 && self.cycle < 338;

            // Do the 8 cycle data fetching routine for rendering tile data.
            if is_visible_region || is_preparing_next_scanline {
                self.shift_shifters();

                match (self.cycle - 1) % 8 {
                    // Nametable fetch
                    0 => {
                        self.load_bg_shifters();
                        self.next_bg_tile_id = self.fetch_nametable_tile_id();
                    }
                    // Attribute memory fetch
                    2 => {
                        self.next_bg_tile_palette_id = self.fetch_tile_palette_id();
                    }
                    // Fetch LSB of tile from the pattern memory
                    4 => {
                        self.next_bg_tile_lsb = self.fetch_tile_row_lsb();
                    }
                    // Fetch MSB of tile from the pattern memory
                    6 => {
                        self.next_bg_tile_msb = self.fetch_tile_row_msb();
                    }
                    7 => {
                        self.increment_scroll_x();
                    }
                    _ => {}
                }
            }

            match self.cycle {
                256 => self.increment_scroll_y(),
                257 => self.copy_horizontal_position(),
                // Copy vertical position info at the end of VBlank
                280..=304 if self.scanline == -1 => self.copy_vertical_position(),
                // Unused nametable fetches
                338 | 340 => self.next_bg_tile_id = self.fetch_nametable_tile_id(),
                _ => {}
            }

            // TODO: Emulate sprite evaluation properly
            if self.cycle == 257 && self.scanline >= 0 {
                self.scanline_sprites = self.next_scanline_sprite_evaluation();
            }

            // Sprite rendering
            if self.cycle == 340 {
                let pattern_table =
                    PatternTable::from(self.ctrl.contains(PpuCtrl::SpritePatternTable));
                for sprite in self.scanline_sprites {
                    if !self.ctrl.contains(PpuCtrl::SpriteSize) {
                        let tile_row = self.fetch_tile_byte(pattern_table, sprite);
                    }
                }
            }
        }

        let mut nmi = false;
        // Finished rendering visible portion, entering vertical blank
        if self.scanline == 241 && self.cycle == 1 {
            self.status.set(PpuStatus::VerticalBlank, true);
            nmi = self.ctrl.contains(PpuCtrl::GenerateNMI);
        }

        let pixel = self.get_pixel();

        self.cycle += 1;
        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;

            if self.scanline > 260 {
                self.scanline = -1;
            }
        }

        PpuClockResult { pixel, nmi }
    }

    fn fetch_nametable_tile_id(&self) -> u8 {
        let offset = u16::from(self.vram_addr) & 0x0FFF;
        self.read(0x2000 + offset)
    }

    fn fetch_tile_palette_id(&self) -> u8 {
        const ATTRIBUTE_MEMORY_OFFSET: u16 = 0x2000 + 0x03C0;

        // Divide by 4 to break down the nametable space into 4x4 groups
        let tile_group_x = (self.vram_addr.coarse_x() >> 2) as u16;
        let tile_group_y = (self.vram_addr.coarse_y() >> 2) as u16;
        let nametable = (self.vram_addr.nametable_y() << 1 | self.vram_addr.nametable_x()) as u16;

        let mut attr_byte = self
            .read(ATTRIBUTE_MEMORY_OFFSET | (nametable << 10) | (tile_group_y << 3) | tile_group_x);

        // The byte holds the palettes for a 4x4 group of tiles,
        // but each palette only takes up 2 bits (used for a 2x2 group of tiles)
        // So we need to select the correct 2 bits for the current 2x2 tile group.
        // The groups are numbered
        // 0 1
        // 2 3
        // In the palette byte, this is how the palettes match up:
        // 33221100

        // We're in the right half of the 4x4 group
        if self.vram_addr.coarse_x() % 4 >= 2 {
            attr_byte >>= 2;
        }

        // We're in the bottom half of the 4x4 group
        if self.vram_addr.coarse_y() % 4 >= 2 {
            attr_byte >>= 4;
        }

        attr_byte & 0x03
    }

    fn fetch_tile_row_lsb(&self) -> u8 {
        let pattern_table = PatternTable::from(self.ctrl.contains(PpuCtrl::BackgroundPatternTable));
        self.fetch_tile_byte(
            pattern_table,
            self.next_bg_tile_id,
            self.vram_addr.fine_y(),
            false,
        )
    }

    fn fetch_tile_row_msb(&self) -> u8 {
        let pattern_table = PatternTable::from(self.ctrl.contains(PpuCtrl::BackgroundPatternTable));
        // High plane comes 8 bytes after low plane so need to offset by 8
        self.fetch_tile_byte(
            pattern_table,
            self.next_bg_tile_id,
            self.vram_addr.fine_y(),
            true,
        )
    }

    fn fetch_tile_byte(
        &self,
        pattern_table: PatternTable,
        tile_id: u8,
        row: u8,
        high_plane: bool,
    ) -> u8 {
        assert!(row < 8);
        let tile_offset = (tile_id as u16) << 4;

        let addr = pattern_table.addr() + tile_offset + (row as u16);
        if !high_plane {
            self.read(addr)
        } else {
            self.read(addr + 8)
        }
    }

    fn rendering_enabled(&self) -> bool {
        self.mask.contains(PpuMask::ShowBackground) || self.mask.contains(PpuMask::ShowSprites)
    }

    // See: https://www.nesdev.org/wiki/PPU_scrolling
    fn increment_scroll_x(&mut self) {
        if !self.rendering_enabled() {
            return;
        }

        if self.vram_addr.coarse_x() < 31 {
            self.vram_addr.increment_coarse_x();
            return;
        }

        self.vram_addr.set_coarse_x(0);
        // Wrap to the next horizontal nametable
        self.vram_addr.increment_nametable_x();
    }

    fn increment_scroll_y(&mut self) {
        if !self.rendering_enabled() {
            return;
        }

        if self.vram_addr.fine_y() < 7 {
            self.vram_addr.increment_fine_y();
            return;
        }

        self.vram_addr.set_fine_y(0);

        // Last 2 tile rows of nametable are attribute memory,
        // so the last tile row is actually row 29
        let coarse_y = self.vram_addr.coarse_y();
        if coarse_y == 29 {
            self.vram_addr.set_coarse_y(0);
            // Wrap to the next vertical nametable
            self.vram_addr.increment_nametable_y();
        // Wrap in case coarse y is set to out of bounds
        } else if coarse_y == 31 {
            self.vram_addr.set_coarse_y(0)
        } else {
            self.vram_addr.increment_coarse_y();
        }
    }

    /// Copy coarse X and nametable X from temp_vram_addr to vram_addr if rendering enabled.
    fn copy_horizontal_position(&mut self) {
        if !self.rendering_enabled() {
            return;
        }

        self.vram_addr.set_coarse_x(self.temp_vram_addr.coarse_x());
        self.vram_addr
            .set_nametable_x(self.temp_vram_addr.nametable_x());
    }

    /// Copy coarse Y, fine Y, and nametable Y from temp_vram_addr to vram_addr if rendering enabled
    fn copy_vertical_position(&mut self) {
        if !self.rendering_enabled() {
            return;
        }

        self.vram_addr.set_coarse_y(self.temp_vram_addr.coarse_y());
        self.vram_addr.set_fine_y(self.temp_vram_addr.fine_y());
        self.vram_addr
            .set_nametable_y(self.temp_vram_addr.nametable_y());
    }

    fn load_bg_shifters(&mut self) {
        self.bg_tile_low_shifter =
            (self.bg_tile_low_shifter & 0xFF00) | self.next_bg_tile_lsb as u16;
        self.bg_tile_high_shifter =
            (self.bg_tile_high_shifter & 0xFF00) | self.next_bg_tile_msb as u16;

        let fill = if self.next_bg_tile_palette_id & 0x01 == 0 {
            0x00
        } else {
            0xFF
        };
        self.bg_tile_palette_low_shifter = (self.bg_tile_palette_low_shifter & 0xFF00) | fill;

        let fill = if self.next_bg_tile_palette_id & 0x10 == 0 {
            0x00
        } else {
            0xFF
        };
        self.bg_tile_palette_high_shifter = (self.bg_tile_palette_high_shifter & 0xFF00) | fill;
    }

    fn shift_shifters(&mut self) {
        if !self.mask.contains(PpuMask::ShowBackground) {
            return;
        }

        self.bg_tile_low_shifter <<= 1;
        self.bg_tile_high_shifter <<= 1;
        self.bg_tile_palette_low_shifter <<= 1;
        self.bg_tile_palette_high_shifter <<= 1;
    }

    fn next_scanline_sprite_evaluation(&mut self) -> Vec<PpuSprite> {
        let mut next_scanline_sprites = Vec::new();

        for sprite in self.oam.chunks_exact(4) {
            if next_scanline_sprites.len() >= 9 {
                break;
            }

            let sprite = PpuSprite {
                y: sprite[0],
                tile_id: sprite[1],
                attribute: sprite[2],
                x: sprite[3],
            };

            let height = if self.ctrl.contains(PpuCtrl::SpriteSize) {
                16
            } else {
                8
            };

            if (self.scanline..self.scanline + height).contains(&(sprite.y as i16)) {
                next_scanline_sprites.push(sprite);
            }
        }

        if next_scanline_sprites.len() >= 9 {
            next_scanline_sprites.pop();
            self.status.insert(PpuStatus::SpriteOverflow);
        }

        next_scanline_sprites
    }

    fn get_pixel(&self) -> Option<Pixel> {
        // Don't emit a pixel if we're outside of the visible region
        if !(0..240).contains(&self.scanline) || !(1..257).contains(&self.cycle) {
            return None;
        }

        let (palette, pixel) = if self.mask.contains(PpuMask::ShowBackground) {
            let mask = 0x8000 >> self.fine_x;

            let pixel_low = (self.bg_tile_low_shifter & mask).into_bit();
            let pixel_high = (self.bg_tile_high_shifter & mask).into_bit();

            let palette_low = (self.bg_tile_palette_low_shifter & mask).into_bit();
            let palette_high = (self.bg_tile_palette_high_shifter & mask).into_bit();

            let pixel = (pixel_high << 1) | pixel_low;
            let palette = (palette_high << 1) | palette_low;

            (palette, pixel)
        } else {
            (0, 0)
        };

        let px = Pixel {
            x: (self.cycle - 1) as usize,
            y: (self.scanline) as usize,
            color: self.get_palette_color(palette, pixel),
        };

        Some(px)
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        assert!((0x2000..=0x3FFF).contains(&addr), "Invalid PPU address");

        let register = addr % 8;
        match register {
            0 => {
                self.ctrl = PpuCtrl::from_bits_truncate(data);

                let bits = self.ctrl.bits();
                self.temp_vram_addr.set_nametable_x(bits & 0x01);
                self.temp_vram_addr.set_nametable_y((bits >> 1) & 0x01);
            }
            1 => self.mask = PpuMask::from_bits_truncate(data),
            2 => {}
            3 => self.oam_addr = data,
            4 => self.oam[self.oam_addr as usize] = data,
            5 => {
                if !self.write_latch {
                    self.fine_x = data & 0b0000_0111;
                    self.temp_vram_addr.set_coarse_x(data >> 3);

                    self.write_latch = true;
                } else {
                    self.temp_vram_addr.set_fine_y(data & 0b0000_0111);
                    self.temp_vram_addr.set_coarse_y(data >> 3);

                    self.write_latch = false;
                }
            }
            6 => {
                let data = data as u16;
                // Write latch is false on first write, false on second
                // We write the high byte first.
                if !self.write_latch {
                    let addr = (u16::from(self.temp_vram_addr) & 0x00FF) | data << 8;
                    self.temp_vram_addr = VRAMAddr::from(addr);
                    self.write_latch = true;
                } else {
                    let addr = (u16::from(self.temp_vram_addr) & 0xFF00) | data;
                    self.temp_vram_addr = VRAMAddr::from(addr);
                    self.vram_addr = self.temp_vram_addr;
                    self.write_latch = false;
                }
            }
            7 => {
                self.write(self.vram_addr.into(), data);
                if self.ctrl.contains(PpuCtrl::VRamAddressIncrement) {
                    self.vram_addr = VRAMAddr::from(u16::from(self.vram_addr) + 32);
                } else {
                    self.vram_addr = VRAMAddr::from(u16::from(self.vram_addr) + 1);
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        assert!((0x2000..=0x3FFF).contains(&addr), "Invalid PPU address");

        let register = addr % 8;
        match register {
            0 => self.ctrl.bits(),
            1 => self.mask.bits(),
            2 => {
                let data = self.status.bits();

                self.status.set(PpuStatus::VerticalBlank, false);
                self.write_latch = false;

                data
            }
            3 => 0,
            4 => self.oam[self.oam_addr as usize],
            5 => 0,
            6 => 0,
            7 => {
                let mut temp = self.data_buffer;
                self.data_buffer = self.read(self.vram_addr.into());

                if u16::from(self.vram_addr) >= 0x3F00 {
                    temp = self.data_buffer;
                }
                if self.ctrl.contains(PpuCtrl::VRamAddressIncrement) {
                    self.vram_addr = VRAMAddr::from(u16::from(self.vram_addr) + 32);
                } else {
                    self.vram_addr = VRAMAddr::from(u16::from(self.vram_addr) + 1);
                }

                temp
            }
            _ => unreachable!(),
        }
    }

    /// CPU Read but doesn't affect state
    pub fn cpu_read_debug(&self, addr: u16) -> u8 {
        assert!((0x2000..=0x3FFF).contains(&addr), "Invalid PPU address");

        let register = addr % 8;
        match register {
            0 => self.ctrl.bits(),
            1 => self.mask.bits(),
            2 => self.status.bits(),
            3 => 0,
            4 => 0,
            5 => 0,
            6 => 0,
            7 => self.data_buffer,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        let cartridge = self.cartridge.as_ref().expect("Cartridge not attached");
        if let Ok(()) = cartridge.borrow_mut().ppu_write(addr, data) {
            return;
        }

        match addr {
            0x0000..=0x1FFF => {}
            0x2000..=0x3EFF => {
                let mirroring = cartridge.borrow().mirroring();

                let index = map_addr_to_nametable(mirroring, addr);
                self.nametable_ram[index] = data;
            }
            0x3F00..=0x3FFF => {
                // Palette ram is from 0x3F00 to 0x3F1F, but mirrored from 0x3F20-0x3FFF
                let i = addr & 0x1F;
                let i = match i {
                    0x10 | 0x14 | 0x18 | 0x1C => i - 0x10,
                    x => x,
                };

                self.palette_ram[i as usize] = data;
            }
            // _ => panic!("Writing to PPU address {:04X} not implemented yet", addr),
            _ => {}
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        let cartridge = self.cartridge.as_ref().expect("Cartridge not attached");
        if let Ok(data) = cartridge.borrow().ppu_read(addr) {
            return data;
        }

        match addr {
            0x2000..=0x3EFF => {
                let mirroring = cartridge.borrow().mirroring();

                let index = map_addr_to_nametable(mirroring, addr);
                self.nametable_ram[index]
            }
            0x3F00..=0x3FFF => {
                // Palette ram is from 0x3F00 to 0x3F1F, but mirrored from 0x3F20-0x3FFF
                let i = addr & 0x1F;
                let i = match i {
                    // Mirrored on these addresses
                    0x10 | 0x14 | 0x18 | 0x1C => i - 0x10,
                    x => x,
                };

                self.palette_ram[i as usize]
            }
            // _ => todo!("Reading from PPU address {:04X} not implemented yet", addr),
            _ => 0,
        }
    }

    pub fn dma_oam_write(&mut self, index: u8, data: u8) {
        self.oam[index as usize] = data;
    }

    pub fn get_palette_color(&self, palette: u8, pixel: u8) -> Color {
        let offset = palette * 4 + pixel;
        let color_index = self.read(0x3F00 + offset as u16);

        self.palette
            .get_color(color_index)
            .unwrap_or_else(|| panic!("Invalid palette color {}", color_index))
    }

    pub fn get_pattern_table(&self, table: PatternTable) -> Sprite {
        let table_offset = match table {
            PatternTable::Left => 0x0000,
            PatternTable::Right => 0x1000,
        };

        let mut buf = [Color::default(); 128 * 128];

        for i in 0..16 {
            for j in 0..16 {
                let tile_offset = i * 256 + j * 16;
                for tile_row in 0..8 {
                    let row_addr = table_offset + tile_offset + tile_row;
                    let tile_lsb = self.read(row_addr);
                    let tile_msb = self.read(row_addr + 8);

                    for tile_col in 0..8 {
                        let lsb = (tile_lsb >> tile_col) & 0x01;
                        let msb = (tile_msb >> tile_col) & 0x01;

                        let pixel = (msb << 1) | lsb;
                        let pixel_index = (i * 8 + tile_row) * 128 + (j * 8 + 7 - tile_col);
                        // TODO: Don't hardcode palette
                        buf[pixel_index as usize] = self.get_palette_color(0, pixel);
                    }
                }
            }
        }

        Sprite::new(Vec::from(buf), 128, 128).expect("Failed to create sprite from pattern table")
    }
}

/// Returns the index to write to in the nametable.
fn map_addr_to_nametable(mirroring: Mirroring, address: u16) -> usize {
    assert!(
        (0x2000..=0x3FFF).contains(&address),
        "Invalid nametable address"
    );

    // Address mirroring (0x3000-0x3EFF is mapped to 0x2000-0x2EFF)
    let address = if address >= 0x3000 {
        address - 0x1000
    } else {
        address
    };

    let offset = match mirroring {
        Mirroring::Horizontal => {
            if (0x2000..0x2800).contains(&address) {
                0
            } else {
                0x1000
            }
        }
        Mirroring::Vertical => {
            if (0x2000..0x2400).contains(&address) || (0x2800..0x2C00).contains(&address) {
                0
            } else {
                0x100
            }
        }
    } as usize;

    let index = (address - 0x2000) as usize % NAMETABLE_SIZE;

    offset + index
}
