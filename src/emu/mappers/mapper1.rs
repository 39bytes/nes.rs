use crate::emu::cartridge::Mirroring;

use super::{MapRead, MapWrite, Mapper};
use modular_bitfield::prelude::*;

const PRG_RAM_SIZE: usize = 32 * 1024;
// TODO: Emulate PRG RAM bank switching
#[allow(dead_code)]
const PRG_RAM_BANK_SIZE: usize = 8 * 1024;
const PRG_ROM_BANK_SIZE: usize = 16 * 1024;
const CHR_BANK_SIZE: usize = 4 * 1024;

#[bitfield]
#[derive(Debug)]
struct ControlRegister {
    mirroring: B2,
    prg_rom_bank_mode: B2,
    chr_bank_mode: B1,
    #[skip]
    padding: B3,
}

#[derive(Debug)]
pub struct Mapper1 {
    prg_bank_count: u8,
    chr_bank_count: u8,

    load: u8,
    load_write_count: u8,

    control: ControlRegister,
    chr_bank0: u8,
    chr_bank1: u8,
    prg_bank: u8,

    prg_ram: [u8; PRG_RAM_SIZE],
}

impl Mapper1 {
    pub fn new(prg_banks: u8, chr_banks: u8) -> Self {
        Self {
            prg_bank_count: prg_banks,
            chr_bank_count: chr_banks,
            load: 0x00,
            load_write_count: 0,

            control: ControlRegister::from_bytes([0x0C]),
            chr_bank0: 0x00,
            chr_bank1: 0x00,
            prg_bank: 0x00,
            prg_ram: [0; PRG_RAM_SIZE],
        }
    }

    fn reset(&mut self) {
        self.load = 0x00;
        self.load_write_count = 0;
        self.control.set_prg_rom_bank_mode(3);
    }
}

impl Mapper for Mapper1 {
    fn map_prg_read(&self, addr: u16) -> Option<MapRead> {
        match addr {
            0x6000..=0x7FFF => Some(MapRead::RAMData(self.prg_ram[(addr - 0x6000) as usize])),
            0x8000..=0xFFFF => {
                let bank_mode = self.control.prg_rom_bank_mode();
                let addr = match bank_mode {
                    // 32 KB mode
                    0 | 1 => {
                        let bank = (self.prg_bank >> 1) as usize;
                        bank * PRG_ROM_BANK_SIZE * 2 + (addr & 0x7FFF) as usize
                    }
                    // 16 KB mode
                    // Fix first bank at 0x8000
                    2 if addr < 0xC000 => (addr & 0x3FFF) as usize,
                    // Switch bank at 0xC000
                    2 if addr >= 0xC000 => {
                        let bank = self.prg_bank as usize;
                        bank * PRG_ROM_BANK_SIZE + (addr & 0x3FFF) as usize
                    }
                    // Fix first bank at 0x8000
                    3 if addr < 0xC000 => {
                        let bank = self.prg_bank as usize;
                        bank * PRG_ROM_BANK_SIZE + (addr & 0x3FFF) as usize
                    }
                    // Fix last bank at 0xC000
                    3 if addr >= 0xC000 => {
                        let bank = (self.prg_bank_count - 1) as usize;
                        bank * PRG_ROM_BANK_SIZE + (addr & 0x3FFF) as usize
                    }
                    _ => unreachable!(),
                };
                Some(MapRead::Address(addr))
            }
            _ => None,
        }
    }

    fn map_prg_write(&mut self, addr: u16, data: u8) -> Option<MapWrite> {
        match addr {
            0x6000..=0x7FFF => {
                self.prg_ram[(addr - 0x6000) as usize] = data;
                Some(MapWrite::RAMWritten)
            }
            // TODO: Emulate ignoring consecutive writes
            0x8000..=0xFFFF => {
                if data & 0x80 != 0 {
                    self.reset();
                    return Some(MapWrite::WroteRegister);
                }

                self.load >>= 1;
                self.load |= (data & 0x01) << 4;
                self.load_write_count += 1;
                if self.load_write_count == 5 {
                    match addr {
                        0x8000..=0x9FFF => {
                            self.control = ControlRegister::from_bytes([self.load]);
                        }
                        0xA000..=0xBFFF => {
                            self.chr_bank0 = self.load;
                        }
                        0xC000..=0xDFFF => {
                            self.chr_bank1 = self.load;
                        }
                        0xE000..=0xFFFF => {
                            self.prg_bank = self.load;
                        }
                        _ => unreachable!(),
                    }

                    self.load = 0x00;
                    self.load_write_count = 0;
                }
                Some(MapWrite::WroteRegister)
            }
            _ => None,
        }
    }

    fn map_chr_read(&mut self, addr: u16) -> Option<MapRead> {
        if addr > 0x1FFF {
            return None;
        }

        let addr = match self.control.chr_bank_mode() {
            // 8 KB mode
            0 => {
                let bank = (self.chr_bank0 >> 1) as usize;
                bank * CHR_BANK_SIZE * 2 + addr as usize
            }
            // 4 KB mode, bank 0
            1 if addr < 0x1000 => {
                let bank = self.chr_bank0 as usize;
                bank * CHR_BANK_SIZE + addr as usize
            }
            // 4 KB mode, bank 1
            1 if addr >= 0x1000 => {
                let bank = self.chr_bank1 as usize;
                bank * CHR_BANK_SIZE + (addr & 0x0FFF) as usize
            }
            _ => unreachable!(),
        };

        Some(MapRead::Address(addr))
    }

    fn map_chr_write(&self, addr: u16) -> Option<MapWrite> {
        if addr > 0x1FFF {
            return None;
        }
        if self.chr_bank_count > 0 {
            return None;
        }

        Some(MapWrite::Address(addr as usize))
    }

    fn mirroring(&self) -> Option<Mirroring> {
        Some(match self.control.mirroring() {
            0 => Mirroring::SingleScreenLower,
            1 => Mirroring::SingleScreenUpper,
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal,
            _ => unreachable!(),
        })
    }
}
