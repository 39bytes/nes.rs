use std::{cell::RefCell, rc::Rc};

use crate::renderer::Sprite;

use super::{
    cartridge::Cartridge,
    cpu::Cpu6502,
    palette::{Color, Palette},
};
use bitflags::bitflags;

pub enum PatternTable {
    Left,
    Right,
}

bitflags! {
    struct PpuCtrl: u8 {
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
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct PpuMask: u8 {
        const Greyscale = 1 << 0;
        const ShowBackgroundLeft = 1 << 1;
        const ShowSpritesLeft = 1 << 2;
        const ShowBackground = 1 << 3;
        const ShowSprites = 1 << 4;
        const EmphasizeRed = 1 << 5;
        const EmphasizeGreen = 1 << 6;
        const EmphasizeBlue = 1 << 7;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct PpuStatus: u8 {
        const SpriteOverflow = 1 << 5;
        const Sprite0Hit = 1 << 6;
        const VerticalBlank = 1 << 7;
    }
}

const PATTERN_RAM_SIZE: usize = 2 * 1024;
const NAMETABLE_RAM_SIZE: usize = 2 * 1024;
const PALETTE_RAM_SIZE: usize = 32;

pub struct Ppu {
    palette: Palette,

    scanline: i16,
    cycle: i16,

    addr_reg_high_byte: bool,
    data_buffer: u8,

    // Registers
    ctrl: PpuCtrl,
    mask: PpuMask,
    status: PpuStatus,
    oam_addr: u8,
    oam_data: u8,
    scroll: u16,
    addr: u16,
    data: u8,

    // Memory
    pattern_ram: [u8; PATTERN_RAM_SIZE],
    nametable_ram: [u8; NAMETABLE_RAM_SIZE],
    palette_ram: [u8; PALETTE_RAM_SIZE],

    // Other components
    cpu: Option<Rc<RefCell<Cpu6502>>>,
    cartridge: Option<Rc<RefCell<Cartridge>>>,
}

impl Ppu {
    pub fn new(palette: Palette) -> Self {
        Ppu {
            palette,

            scanline: 0,
            cycle: 0,

            addr_reg_high_byte: true,
            data_buffer: 0x00,

            ctrl: PpuCtrl::empty(),
            mask: PpuMask::empty(),
            status: PpuStatus::VerticalBlank,
            oam_addr: 0x00,
            oam_data: 0x00,
            scroll: 0x0000,
            addr: 0x0000,
            data: 0x00,

            pattern_ram: [0; PATTERN_RAM_SIZE],
            nametable_ram: [0; NAMETABLE_RAM_SIZE],
            palette_ram: [0; PALETTE_RAM_SIZE],

            cpu: None,
            cartridge: None,
        }
    }

    pub fn with_cpu(&mut self, cpu: Rc<RefCell<Cpu6502>>) {
        self.cpu = Some(cpu);
    }

    pub fn with_cartridge(&mut self, cartridge: Rc<RefCell<Cartridge>>) {
        self.cartridge = Some(cartridge);
    }

    pub fn clock(&mut self) {
        self.cycle += 1;

        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;

            if self.scanline > 260 {
                self.scanline = -1;
            }
        }
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000 => self.ctrl = PpuCtrl::from_bits_truncate(data),
            0x0001 => self.mask = PpuMask::from_bits_truncate(data),
            0x0002 => {}
            0x0003 => {}
            0x0004 => {}
            0x0005 => {}
            0x0006 => {
                let data = data as u16;
                if self.addr_reg_high_byte {
                    self.addr = (self.addr & 0x00FF) | (data << 8);
                    self.addr_reg_high_byte = false;
                } else {
                    self.addr = (self.addr & 0xFF00) | data;
                    self.addr_reg_high_byte = true;
                }
            }
            0x0007 => {
                self.write(self.addr, data);
                self.addr += 1;
            }
            _ => panic!("Invalid register"),
        }
    }

    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000 => 0,
            0x0001 => 0,
            0x0002 => {
                let data = self.status.bits();

                // self.status.remove(PpuStatus::VerticalBlank);
                self.addr_reg_high_byte = true;

                data
            }
            0x0003 => 0,
            0x0004 => 0,
            0x0005 => 0,
            0x0006 => 0,
            0x0007 => {
                let mut temp = self.data_buffer;
                self.data_buffer = self.read(self.addr);

                if self.addr >= 0x3F00 {
                    temp = self.data_buffer;
                }
                self.addr += 1;

                temp
            }
            _ => panic!("Invalid register"),
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match &self.cartridge {
            Some(cartridge) => {
                if let Ok(()) = cartridge.borrow_mut().ppu_write(addr, data) {
                    return;
                }
            }
            None => panic!("Cartridge not attached"),
        };

        match addr {
            0x0000..=0x1FFF => self.pattern_ram[addr as usize] = data,
            0x2000..=0x3EFF => todo!(),
            0x3F00..=0x3FFF => {
                let i = match addr & 0x1F {
                    0x0010 | 0x0014 | 0x0018 | 0x001C => addr - 0x10,
                    x => x,
                };

                self.palette_ram[i as usize] = data;
            }
            _ => panic!("Reading from PPU address {:04X} not implemented yet", addr),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match &self.cartridge {
            Some(cartridge) => {
                if let Ok(data) = cartridge.borrow().ppu_read(addr) {
                    return data;
                }
            }
            None => panic!("Cartridge not attached"),
        };

        match addr {
            0x0000..=0x1FFF => self.pattern_ram[addr as usize],
            0x2000..=0x3EFF => todo!(),
            0x3F00..=0x3FFF => {
                let i = match addr & 0x1F {
                    0x0010 | 0x0014 | 0x0018 | 0x001C => addr - 0x10,
                    x => x,
                };

                self.palette_ram[i as usize]
            }
            _ => todo!("Reading from PPU address {:04X} not implemented yet", addr),
        }
    }

    fn get_palette_color(&self, palette: u8, pixel: u8) -> Color {
        let offset = palette * 4 + pixel;

        self.palette
            .get_color(self.read(0x3F00 + offset as u16))
            .unwrap_or_default()
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
                    let tile_lsb = self.read(table_offset + tile_offset + tile_row);
                    let tile_msb = self.read(table_offset + tile_offset + tile_row + 8);

                    for tile_col in 0..8 {
                        let lsb = (tile_lsb >> tile_col) & 0x01;
                        let msb = (tile_msb >> tile_col) & 0x01;

                        let pixel = lsb | (msb << 1);
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
