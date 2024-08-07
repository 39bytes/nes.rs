use std::{cell::RefCell, rc::Rc};

use crate::renderer::{Color, Pixel, Sprite};

pub use self::pattern_table::PatternTable;

use self::{
    flags::*,
    shift_register::{ShiftRegister16, ShiftRegister8},
    sprite::{Sprite as PpuSprite, SpriteAttribute},
    vram_addr::VRAMAddr,
};

use super::{
    bits::{extend_bit, flip_byte, IntoBit},
    cartridge::{Cartridge, Mirroring},
    palette::Palette,
};

mod flags;
mod pattern_table;
mod shift_register;
pub mod sprite;
mod vram_addr;

#[derive(Debug, Default)]
struct BgPixel {
    pub palette: u8,
    pub pixel: u8,
}

#[derive(Debug, Default)]
struct SpritePixel {
    pub palette: u8,
    pub pixel: u8,
    pub behind_background: bool,
    pub sprite0_hit: bool,
}

pub struct PpuClockResult {
    pub pixel: Option<Pixel>,
    pub nmi: bool,
    pub irq: bool,
    pub frame_complete: bool,
}

const PALETTE_RAM_SIZE: usize = 32;
const NAMETABLE_SIZE: usize = 1024;
const OAM_SIZE: usize = 256;

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
    bg_tile_id_shifter: ShiftRegister16,
    bg_tile_palette_shifter: ShiftRegister16,

    // Sprite rendering
    scanline_sprites: Vec<PpuSprite>,
    sprite_tile_shifters: [ShiftRegister8; 8],

    // Memory
    nametables: [[u8; NAMETABLE_SIZE]; 2],
    palette_ram: [u8; PALETTE_RAM_SIZE],

    oam: [u8; OAM_SIZE],

    // Other components
    cartridge: Option<Rc<RefCell<Cartridge>>>,

    odd_frame: bool,
}

impl Ppu {
    pub fn new(palette: Palette) -> Self {
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

            bg_tile_id_shifter: ShiftRegister16::new(true),
            bg_tile_palette_shifter: ShiftRegister16::new(false),

            scanline_sprites: Vec::new(),
            sprite_tile_shifters: [ShiftRegister8::new(false); 8],

            nametables: [[0; NAMETABLE_SIZE]; 2],
            palette_ram: [0; PALETTE_RAM_SIZE],

            oam: [0; OAM_SIZE],

            cartridge: None,

            odd_frame: false,
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

    #[allow(dead_code)]
    pub fn scanline(&self) -> i16 {
        self.scanline
    }

    #[allow(dead_code)]
    pub fn cycle(&self) -> u16 {
        self.cycle
    }

    #[allow(dead_code)]
    pub fn nametables(&self) -> &[[u8; NAMETABLE_SIZE]; 2] {
        &self.nametables
    }

    #[allow(dead_code)]
    pub fn oam(&self) -> &[u8; OAM_SIZE] {
        &self.oam
    }

    pub fn load_cartridge(&mut self, cartridge: Rc<RefCell<Cartridge>>) {
        self.cartridge = Some(cartridge);
    }

    /// See: https://www.nesdev.org/wiki/PPU_rendering
    /// for details on how this works.
    pub fn clock(&mut self) -> PpuClockResult {
        let mut irq = false;

        // Rendering during the visible region
        if self.scanline < 240 {
            // Rendering a new frame so reset some flags
            if self.scanline == -1 && self.cycle == 1 {
                self.status.set(PpuStatus::VerticalBlank, false);
                self.status.set(PpuStatus::Sprite0Hit, false);
                self.status.set(PpuStatus::SpriteOverflow, false);

                self.scanline_sprites = Vec::new();
            }

            // Odd frame cycle skip
            if self.scanline == 0 && self.cycle == 0 && self.odd_frame {
                self.cycle = 1;
            }

            let is_visible_region = self.cycle >= 1 && self.cycle <= 257;
            let is_preparing_next_scanline = self.cycle >= 321 && self.cycle <= 337;

            // Background data fetching
            // Do the 8 cycle data fetching routine for rendering tile data.
            if is_visible_region || is_preparing_next_scanline {
                // if self.scanline >= 174 && self.scanline <= 177 {
                //     println!(
                //         "VRAM addr on menu scanline {} cycle {}: {:?} (hex: {:06X}) ",
                //         self.scanline,
                //         self.cycle,
                //         self.vram_addr,
                //         u16::from(self.vram_addr)
                //     );
                // }
                if self.cycle >= 2 {
                    self.shift_shifters();
                }

                match self.cycle % 8 {
                    // Nametable fetch
                    1 => {
                        self.load_bg_shifters();
                    }
                    2 => {
                        self.next_bg_tile_id = self.fetch_nametable_tile_id();
                    }
                    // Attribute memory fetch
                    4 => {
                        self.next_bg_tile_palette_id = self.fetch_tile_palette_id();
                    }
                    // Fetch LSB of tile from the pattern memory
                    6 => {
                        self.next_bg_tile_lsb = self.fetch_tile_row_lsb();
                    }
                    // Fetch MSB of tile from the pattern memory
                    0 => {
                        self.next_bg_tile_msb = self.fetch_tile_row_msb();
                        // Update v
                        self.increment_scroll_x();
                    }
                    _ => {}
                }
            }

            match self.cycle {
                256 => self.increment_scroll_y(),
                257 => {
                    self.copy_horizontal_position();
                    // garbage NT fetch
                    self.next_bg_tile_id = self.fetch_nametable_tile_id();
                }
                // garbage NT fetch
                259 => self.next_bg_tile_id = self.fetch_nametable_tile_id(),
                // Copy vertical position info at the end of VBlank
                280..=304 if self.scanline == -1 => self.copy_vertical_position(),
                // Unused nametable fetches
                338 | 340 => self.next_bg_tile_id = self.fetch_nametable_tile_id(),
                _ => {}
            }

            // Sprite evaluation
            if self.cycle == 257 && self.scanline >= 0 {
                self.scanline_sprites = self.next_scanline_sprite_evaluation();
            }

            // Sprite rendering
            if self.cycle == 340 {
                for (i, sprite) in self.scanline_sprites.iter().enumerate() {
                    let (lsb, msb) = if !self.ctrl.contains(PpuCtrl::SpriteSize) {
                        self.fetch_8x8_sprite_row(sprite)
                    } else {
                        self.fetch_8x16_sprite_row(sprite)
                    };
                    self.sprite_tile_shifters[i].load(lsb, msb);
                }
            }
        }

        let mut nmi = false;
        // Finished rendering visible portion, entering vertical blank
        if self.scanline == 241 && self.cycle == 1 {
            self.status.set(PpuStatus::VerticalBlank, true);
            nmi = self.ctrl.contains(PpuCtrl::GenerateNMI);
        }

        let pixel = match self.get_pixel() {
            Some((pixel, sprite0_hit)) => {
                if sprite0_hit {
                    self.status.set(PpuStatus::Sprite0Hit, true);
                }
                Some(pixel)
            }
            None => None,
        };

        self.cycle += 1;
        if self.rendering_enabled() && self.cycle == 260 && self.scanline < 240 {
            // Notify mapper of scanline end (mapper 3 IRQ clock)
            irq = self.notify_scanline_hblank();
        }

        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;

            if self.scanline > 260 {
                self.scanline = -1;
                self.odd_frame = !self.odd_frame
            }
        }

        PpuClockResult {
            pixel,
            nmi,
            irq,
            frame_complete: self.cycle == 0 && self.scanline == -1,
        }
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
        let tile_offset = (tile_id as u16) << 4;

        let addr = pattern_table.addr() + tile_offset + (row as u16);
        // The high bit plane is located 8 bytes further
        if !high_plane {
            self.read(addr)
        } else {
            self.read(addr + 8)
        }
    }

    fn fetch_8x8_sprite_row(&self, sprite: &PpuSprite) -> (u8, u8) {
        let pattern_table = PatternTable::from(self.ctrl.contains(PpuCtrl::SpritePatternTable));
        let dy = self.scanline - (sprite.y as i16);
        let row = if sprite.attribute.contains(SpriteAttribute::FlipVertically) {
            7 - dy
        } else {
            dy
        };

        let lsb = self.fetch_tile_byte(pattern_table, sprite.tile_id, row as u8, false);
        let msb = self.fetch_tile_byte(pattern_table, sprite.tile_id, row as u8, true);

        if sprite.attribute.contains(SpriteAttribute::FlipHorizontally) {
            (flip_byte(lsb), flip_byte(msb))
        } else {
            (lsb, msb)
        }
    }

    fn fetch_8x16_sprite_row(&self, sprite: &PpuSprite) -> (u8, u8) {
        let pattern_table = PatternTable::from(sprite.tile_id & 0x01 == 1);
        let dy = self.scanline - (sprite.y as i16);

        let tile_id = sprite.tile_id & 0xFE;

        // 4 cases:
        let (tile_id, row) = if !sprite.attribute.contains(SpriteAttribute::FlipVertically) {
            // Top half - not flipped
            if dy < 8 {
                (tile_id, dy)
            // Bottom half - not flipped
            } else {
                (tile_id + 1, dy % 8)
            }
        } else {
            // Top half - flipped
            if dy < 8 {
                (tile_id + 1, 7 - dy)
            // Bottom half - flipped
            } else {
                (tile_id, 7 - (dy % 8))
            }
        };

        let lsb = self.fetch_tile_byte(pattern_table, tile_id, row as u8, false);
        let msb = self.fetch_tile_byte(pattern_table, tile_id, row as u8, true);

        if sprite.attribute.contains(SpriteAttribute::FlipHorizontally) {
            (flip_byte(lsb), flip_byte(msb))
        } else {
            (lsb, msb)
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
        self.bg_tile_id_shifter
            .load(self.next_bg_tile_lsb, self.next_bg_tile_msb);

        let fill_low = extend_bit(self.next_bg_tile_palette_id & 0x01);
        let fill_high = extend_bit(self.next_bg_tile_palette_id & 0x02);
        self.bg_tile_palette_shifter.load(fill_low, fill_high);
    }

    fn shift_shifters(&mut self) {
        if self.mask.contains(PpuMask::ShowBackground) {
            self.bg_tile_id_shifter.shift();
            self.bg_tile_palette_shifter.shift();
        }

        if self.mask.contains(PpuMask::ShowSprites) && (1..258).contains(&self.cycle) {
            for i in 0..self.scanline_sprites.len() {
                if self.scanline_sprites[i].x > 0 {
                    self.scanline_sprites[i].x -= 1;
                } else {
                    self.sprite_tile_shifters[i].shift();
                }
            }
        }
    }

    // See: https://www.nesdev.org/wiki/PPU_sprite_evaluation
    fn next_scanline_sprite_evaluation(&mut self) -> Vec<PpuSprite> {
        let mut next_scanline_sprites = Vec::new();

        let in_range = |y: u8| {
            // Outside of visible region anyway, so a sprite wouldn't be visible here
            if self.scanline == -1 || self.scanline >= 240 {
                return false;
            }

            let height = if self.ctrl.contains(PpuCtrl::SpriteSize) {
                16
            } else {
                8
            };

            let dy = self.scanline - y as i16;
            dy >= 0 && dy < height
        };

        for (i, sprite) in self.oam.chunks_exact(4).enumerate() {
            if next_scanline_sprites.len() == 8 {
                break;
            }

            let sprite = PpuSprite::from_bytes(sprite, i);
            if in_range(sprite.y) {
                next_scanline_sprites.push(sprite);
            }
        }

        if next_scanline_sprites.len() < 8 {
            next_scanline_sprites.resize_with(8, PpuSprite::default);
            return next_scanline_sprites;
        }

        // Sprite overflow check
        let mut n = next_scanline_sprites.last().unwrap().oam_index + 1;
        let mut m = 0;

        while n * 4 + m <= OAM_SIZE - 4 {
            let i = n * 4 + m;
            let y = self.oam[i];

            // Emulate the sprite overflow bug
            if in_range(y) {
                self.status.insert(PpuStatus::SpriteOverflow);
                break;
            }
            n += 1;
            m += 1;
        }

        next_scanline_sprites
    }

    fn get_pixel(&self) -> Option<(Pixel, bool)> {
        // Don't emit a pixel if we're outside of the visible region
        if !(0..240).contains(&self.scanline) || !(1..257).contains(&self.cycle) {
            return None;
        }

        let bg_pixel = self.get_bg_pixel();
        let sprite_pixel = self.get_sprite_pixel();

        let (palette, pixel, sprite0_hit) = match (bg_pixel.pixel, sprite_pixel.pixel) {
            (0, 0) => (0, 0, false),
            (0, sp_px) => (sprite_pixel.palette, sp_px, false),
            (bg_px, 0) => (bg_pixel.palette, bg_px, false),
            (bg_px, sp_px) => {
                let (palette, pixel) = if sprite_pixel.behind_background {
                    (bg_pixel.palette, bg_px)
                } else {
                    (sprite_pixel.palette, sp_px)
                };

                // Sprite 0 hit detection logic
                // See https://www.nesdev.org/wiki/PPU_OAM#Sprite_0_hits
                let left_clipping_enabled = !self.mask.contains(PpuMask::ShowBackgroundLeft)
                    || !self.mask.contains(PpuMask::ShowSpritesLeft);
                let in_left_clip_window = left_clipping_enabled && self.cycle < 9; // x < 8

                let sprite0_hit =
                    sprite_pixel.sprite0_hit && self.rendering_enabled() && !in_left_clip_window;

                (palette, pixel, sprite0_hit)
            }
        };

        let px = Pixel {
            x: (self.cycle - 1) as usize,
            y: (self.scanline) as usize,
            color: self.get_palette_color(palette, pixel),
        };

        Some((px, sprite0_hit))
    }

    fn get_bg_pixel(&self) -> BgPixel {
        if !self.mask.contains(PpuMask::ShowBackground) {
            return BgPixel::default();
        }

        let pixel = self.bg_tile_id_shifter.get_at(self.fine_x);
        let palette = self.bg_tile_palette_shifter.get_at(self.fine_x);

        BgPixel { palette, pixel }
    }

    fn get_sprite_pixel(&self) -> SpritePixel {
        if !self.mask.contains(PpuMask::ShowSprites) {
            return SpritePixel::default();
        }

        for i in 0..self.scanline_sprites.len() {
            let sprite = &self.scanline_sprites[i];
            if sprite.x != 0 {
                continue;
            }

            let pixel = self.sprite_tile_shifters[i].get();
            if pixel == 0 {
                continue;
            }

            let palette_low = sprite.attribute.contains(SpriteAttribute::PaletteLSB) as u8;
            let palette_high = sprite.attribute.contains(SpriteAttribute::PaletteMSB) as u8;
            let palette = (palette_high << 1) | palette_low;

            let behind_background = sprite.attribute.contains(SpriteAttribute::BehindBackground);

            return SpritePixel {
                // need to pick from sprite palettes instead, which is 4 after the bg palettes
                palette: palette + 4,
                pixel,
                behind_background,
                sprite0_hit: sprite.oam_index == 0,
            };
        }

        SpritePixel::default()
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) -> bool {
        debug_assert!((0x2000..=0x3FFF).contains(&addr), "Invalid PPU address");

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
            4 => {
                self.oam[self.oam_addr as usize] = data;
                self.oam_addr = self.oam_addr.wrapping_add(1);
            }
            5 => {
                if !self.write_latch {
                    self.fine_x = data & 0x07;
                    self.temp_vram_addr.set_coarse_x(data >> 3);
                } else {
                    self.temp_vram_addr.set_fine_y(data & 0x07);
                    self.temp_vram_addr.set_coarse_y(data >> 3);
                }
                self.write_latch = !self.write_latch;
            }
            6 => {
                let data = data as u16;
                let mut irq = false;
                // Write latch is false on first write, true on second
                // We write the high byte first.
                if !self.write_latch {
                    let old = u16::from(self.temp_vram_addr);
                    let addr = (old & 0x00FF) | ((data & 0x3F) << 8);

                    // A12 toggle, should trigger an MMC3 update
                    if (old & 0x1000) == 0 && (addr & 0x1000) != 0 {
                        irq = self.notify_scanline_hblank();
                    }
                    self.temp_vram_addr = VRAMAddr::from(addr);
                } else {
                    let addr = (u16::from(self.temp_vram_addr) & 0xFF00) | data;
                    self.temp_vram_addr = VRAMAddr::from(addr);
                    self.vram_addr = self.temp_vram_addr;
                    // println!("Wrote PPUADDR: {:04X} ({:?})", addr, self.vram_addr);
                }
                self.write_latch = !self.write_latch;

                return irq;
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
        false
    }

    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        debug_assert!((0x2000..=0x3FFF).contains(&addr), "Invalid PPU address");

        let register = addr % 8;
        match register {
            0 => 0,
            1 => 0,
            2 => {
                let data = self.status.bits();

                self.status.set(PpuStatus::VerticalBlank, false);
                self.write_latch = false;

                data
            }
            3 => 0,
            4 => {
                if (0..240).contains(&self.scanline) && (1..65).contains(&self.cycle) {
                    return 0xFF;
                }
                self.oam[self.oam_addr as usize]
            }
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
        debug_assert!((0x2000..=0x3FFF).contains(&addr), "Invalid PPU address");

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

        match addr {
            0x0000..=0x1FFF => cartridge.borrow_mut().ppu_write(addr, data),
            0x2000..=0x3EFF => {
                let mirroring = cartridge.borrow().mirroring();

                let (nt, index) = map_addr_to_nametable(mirroring, addr);
                self.nametables[nt][index] = data;
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

    fn read(&self, addr: u16) -> u8 {
        let cartridge = self.cartridge.as_ref().expect("Cartridge not attached");

        match addr {
            0x0000..=0x1FFF => cartridge.borrow_mut().ppu_read(addr),
            0x2000..=0x3EFF => {
                let mirroring = cartridge.borrow().mirroring();

                let (nt, index) = map_addr_to_nametable(mirroring, addr);
                self.nametables[nt][index]
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
            _ => 0,
        }
    }

    pub fn read_debug(&self, addr: u16) -> u8 {
        let cartridge = self.cartridge.as_ref().expect("Cartridge not attached");

        match addr {
            0x0000..=0x1FFF => cartridge.borrow_mut().ppu_read_debug(addr),
            0x2000..=0x3EFF => {
                let mirroring = cartridge.borrow().mirroring();

                let (nt, index) = map_addr_to_nametable(mirroring, addr);
                self.nametables[nt][index]
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
            _ => 0,
        }
    }

    pub fn dma_oam_write(&mut self, index: u8, data: u8) {
        self.oam[index as usize] = data;
    }

    pub fn get_palette_color(&self, palette: u8, pixel: u8) -> Color {
        let offset = (palette << 2) + pixel;
        let color_index = self.read(0x3F00 + offset as u16);

        self.palette.get_color(color_index)
    }

    pub fn get_pattern_table(&self, table: PatternTable, palette: u8, mode_8x16: bool) -> Sprite {
        // NOTE: Does the 8x16 bug only happen when sprites are stored in the right
        // pattern table ???
        let table_offset = match table {
            PatternTable::Left => 0x0000,
            PatternTable::Right => 0x1000,
        };

        let mut buf = [Color::default(); 128 * 128];

        for i in 0..16 {
            for j in 0..16 {
                let tile_offset = i * 256 + j * 16;
                let (tile_y, tile_x) = if mode_8x16 {
                    match (i % 2, j % 2) {
                        (0, 0) => (i, j / 2),
                        (0, 1) => (i + 1, j / 2),
                        (1, 0) => (i - 1, j / 2 + 8),
                        (1, 1) => (i, j / 2 + 8),
                        _ => unreachable!(),
                    }
                } else {
                    (i, j)
                };

                for tile_row in 0..8 {
                    let row_addr = table_offset + tile_offset + tile_row;
                    let tile_lsb = self.read_debug(row_addr);
                    let tile_msb = self.read_debug(row_addr + 8);

                    for tile_col in 0..8 {
                        let lsb = (tile_lsb >> tile_col) & 0x01;
                        let msb = (tile_msb >> tile_col) & 0x01;

                        let pixel = (msb << 1) | lsb;

                        let pixel_index =
                            (tile_y * 8 + tile_row) * 128 + (tile_x * 8 + 7 - tile_col);
                        // TODO: Don't hardcode palette
                        buf[pixel_index as usize] = self.get_palette_color(palette, pixel);
                    }
                }
            }
        }

        Sprite::new(Vec::from(buf), 128, 128).expect("Failed to create sprite from pattern table")
    }

    pub fn notify_scanline_hblank(&self) -> bool {
        self.cartridge
            .as_ref()
            .expect("Cartridge not attached")
            .borrow_mut()
            .on_scanline_hblank()
    }
}

/// Returns nametable (0 or 1) as well as the index within the nametable
/// See: https://www.nesdev.org/wiki/Mirroring
fn map_addr_to_nametable(mirroring: Mirroring, addr: u16) -> (usize, usize) {
    debug_assert!(
        (0x2000..=0x3FFF).contains(&addr),
        "Invalid nametable address"
    );

    let addr = if addr >= 0x3000 { addr - 0x1000 } else { addr } as usize;

    match mirroring {
        Mirroring::Horizontal => match addr {
            0x2000..=0x23FF => (0, addr - 0x2000),
            0x2400..=0x27FF => (0, addr - 0x2400),
            0x2800..=0x2BFF => (1, addr - 0x2800),
            0x2C00..=0x2FFF => (1, addr - 0x2C00),
            _ => unreachable!(),
        },
        Mirroring::Vertical => match addr {
            0x2000..=0x23FF => (0, addr - 0x2000),
            0x2400..=0x27FF => (1, addr - 0x2400),
            0x2800..=0x2BFF => (0, addr - 0x2800),
            0x2C00..=0x2FFF => (1, addr - 0x2C00),
            _ => unreachable!(),
        },
        Mirroring::SingleScreenLower => (0, addr & 0x03FF),
        Mirroring::SingleScreenUpper => (1, addr & 0x03FF),
    }
}
