use super::{MapRead, MapWrite, Mapper};
use crate::cartridge::Mirroring;

const PRG_BANK_SIZE: usize = 8 * 1024;
const CHR_BANK_SIZE: usize = 1024;

enum PRGROMBankMode {
    BottomSwappable,
    TopSwappable,
}

impl PRGROMBankMode {
    pub fn from_bit(bit: u8) -> Self {
        if bit == 0 {
            PRGROMBankMode::BottomSwappable
        } else {
            PRGROMBankMode::TopSwappable
        }
    }
}

enum CHRROMBankMode {
    Bottom2KB,
    Top2KB,
}

impl CHRROMBankMode {
    pub fn from_bit(bit: u8) -> Self {
        if bit == 0 {
            CHRROMBankMode::Bottom2KB
        } else {
            CHRROMBankMode::Top2KB
        }
    }
}

pub struct Mapper4 {
    prg_banks: usize,
    chr_banks: usize,

    prg_ram: [u8; 8 * 1024],

    update_bank: u8,
    prg_rom_bank_mode: PRGROMBankMode,
    chr_rom_bank_mode: CHRROMBankMode,

    chr_bank_r0: u8, // 2 KB
    chr_bank_r1: u8, // 2 KB
    chr_bank_r2: u8, // 1 KB
    chr_bank_r3: u8, // 1 KB
    chr_bank_r4: u8, // 1 KB
    chr_bank_r5: u8, // 1 KB

    prg_bank_r6: u8, // 8 KB
    prg_bank_r7: u8, // 8 KB

    mirroring: Mirroring,

    irq_counter_reload: u8,
    irq_counter: u8,
    irq_disabled: bool,
}

impl Mapper4 {
    pub fn new(prg_banks: usize, chr_banks: usize) -> Self {
        Self {
            prg_banks,
            chr_banks,

            prg_ram: [0; 8 * 1024],

            update_bank: 0,
            prg_rom_bank_mode: PRGROMBankMode::BottomSwappable,
            chr_rom_bank_mode: CHRROMBankMode::Bottom2KB,

            chr_bank_r0: 0,
            chr_bank_r1: 0,
            chr_bank_r2: 0,
            chr_bank_r3: 0,
            chr_bank_r4: 0,
            chr_bank_r5: 0,

            prg_bank_r6: 0,
            prg_bank_r7: 0,

            mirroring: Mirroring::Vertical,

            irq_counter_reload: 0,
            irq_counter: 0,
            irq_disabled: false,
        }
    }
}

impl Mapper for Mapper4 {
    fn map_prg_read(&self, addr: u16) -> Option<MapRead> {
        let bank = match self.prg_rom_bank_mode {
            PRGROMBankMode::BottomSwappable => match addr {
                0x6000..=0x7FFF => {
                    return Some(MapRead::RAMData(self.prg_ram[(addr & 0x1FFF) as usize]))
                }
                0x8000..=0x9FFF => self.prg_bank_r6,
                0xA000..=0xBFFF => self.prg_bank_r7,
                0xC000..=0xDFFF => (self.prg_banks - 2) as u8,
                0xE000..=0xFFFF => (self.prg_banks - 1) as u8,
                _ => return None,
            },
            PRGROMBankMode::TopSwappable => match addr {
                0x6000..=0x7FFF => {
                    return Some(MapRead::RAMData(self.prg_ram[(addr & 0x1FFF) as usize]))
                }
                0x8000..=0x9FFF => (self.prg_banks - 2) as u8,
                0xA000..=0xBFFF => self.prg_bank_r7,
                0xC000..=0xDFFF => self.prg_bank_r6,
                0xE000..=0xFFFF => (self.prg_banks - 1) as u8,
                _ => return None,
            },
        } as usize;

        let mapped = bank * PRG_BANK_SIZE + (addr as usize & 0x1FFF);
        Some(MapRead::Address(mapped))
    }

    fn map_prg_write(&mut self, addr: u16, data: u8) -> Option<MapWrite> {
        match addr {
            0x6000..=0x7FFF => {
                self.prg_ram[(addr & 0x1FFF) as usize] = data;
                Some(MapWrite::RAMWritten)
            }
            // Bank select
            0x8000..=0x9FFF if addr % 2 == 0 => {
                self.update_bank = data & 0b0000_0111;
                // TODO: Emulate MMC6 PRG RAM controls
                self.prg_rom_bank_mode = PRGROMBankMode::from_bit(data & 0b0100_0000);
                self.chr_rom_bank_mode = CHRROMBankMode::from_bit(data & 0b1000_0000);

                Some(MapWrite::WroteRegister)
            }
            // Bank data
            0x8000..=0x9FFF if addr % 2 == 1 => {
                match self.update_bank {
                    0 => self.chr_bank_r0 = data & 0b1111_1110,
                    1 => self.chr_bank_r1 = data & 0b1111_1110,
                    2 => self.chr_bank_r2 = data,
                    3 => self.chr_bank_r3 = data,
                    4 => self.chr_bank_r4 = data,
                    5 => self.chr_bank_r5 = data,
                    6 => self.prg_bank_r6 = data & 0b0011_1111,
                    7 => self.prg_bank_r7 = data & 0b0011_1111,
                    _ => unreachable!(),
                }
                Some(MapWrite::WroteRegister)
            }
            0xA000..=0xBFFF if addr % 2 == 0 => {
                self.mirroring = if data % 2 == 0 {
                    Mirroring::Vertical
                } else {
                    Mirroring::Horizontal
                };

                Some(MapWrite::WroteRegister)
            }
            0xA000..=0xBFFF if addr % 2 == 1 => {
                // NOTE: Should this be implemented? Causes incompatability with MMC6
                None
            }
            0xC000..=0xDFFF if addr % 2 == 0 => {
                self.irq_counter_reload = data;

                Some(MapWrite::WroteRegister)
            }
            0xC000..=0xDFFF if addr % 2 == 1 => {
                self.irq_counter = 0;

                Some(MapWrite::WroteRegister)
            }
            0xE000..=0xFFFF if addr % 2 == 0 => {
                self.irq_disabled = true;

                Some(MapWrite::AcknowledgeIRQ)
            }
            0xE000..=0xFFFF if addr % 2 == 1 => {
                self.irq_disabled = false;

                Some(MapWrite::WroteRegister)
            }
            _ => None,
        }
    }

    fn map_chr_read(&mut self, addr: u16) -> Option<MapRead> {
        let (bank, offset) = match self.chr_rom_bank_mode {
            CHRROMBankMode::Bottom2KB => match addr {
                0x0000..=0x07FF => (self.chr_bank_r0, addr & 0x07FF),
                0x0800..=0x0FFF => (self.chr_bank_r1, addr & 0x07FF),
                0x1000..=0x13FF => (self.chr_bank_r2, addr & 0x03FF),
                0x1400..=0x17FF => (self.chr_bank_r3, addr & 0x03FF),
                0x1800..=0x1BFF => (self.chr_bank_r4, addr & 0x03FF),
                0x1C00..=0x1FFF => (self.chr_bank_r5, addr & 0x03FF),
                _ => return None,
            },
            CHRROMBankMode::Top2KB => match addr {
                0x0000..=0x03FF => (self.chr_bank_r2, addr & 0x03FF),
                0x0400..=0x07FF => (self.chr_bank_r3, addr & 0x03FF),
                0x0800..=0x0BFF => (self.chr_bank_r4, addr & 0x03FF),
                0x0C00..=0x0FFF => (self.chr_bank_r5, addr & 0x03FF),
                0x1000..=0x17FF => (self.chr_bank_r0, addr & 0x07FF),
                0x1800..=0x1FFF => (self.chr_bank_r1, addr & 0x07FF),
                _ => return None,
            },
        };

        Some(MapRead::Address(
            bank as usize * CHR_BANK_SIZE + offset as usize,
        ))
    }

    fn map_chr_write(&self, addr: u16) -> Option<MapWrite> {
        if addr > 0x1FFF {
            return None;
        }
        if self.chr_banks > 0 {
            return None;
        }

        Some(MapWrite::Address(addr as usize))
    }

    fn mirroring(&self) -> Option<Mirroring> {
        Some(self.mirroring)
    }

    fn on_scanline_hblank(&mut self) -> bool {
        if self.irq_counter == 0 {
            self.irq_counter = self.irq_counter_reload;
        } else {
            self.irq_counter -= 1;
        }

        if !self.irq_disabled && self.irq_counter == 0 {
            return true;
        }

        false
    }

    fn onboard_ram(&self) -> Option<&[u8]> {
        Some(&self.prg_ram)
    }

    fn load_onboard_ram(&mut self, ram: &[u8]) {
        self.prg_ram.copy_from_slice(ram);
    }
}
