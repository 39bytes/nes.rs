use crate::cartridge::Mirroring;

use super::{MapRead, MapWrite, Mapper};

const PRG_ROM_BANK_SIZE: usize = 8 * 1024;
const CHR_ROM_BANK_SIZE: usize = 4 * 1024;

enum Latch {
    FD,
    FE,
}

pub struct Mapper9 {
    prg_banks: u8,
    chr_banks: u8,
    prg_bank_select: u8,

    latch0: Latch,
    latch1: Latch,

    chr_fd_bank_select0: u8,
    chr_fe_bank_select0: u8,

    chr_fd_bank_select1: u8,
    chr_fe_bank_select1: u8,

    mirroring: Mirroring,
}

impl Mapper9 {
    pub fn new(prg_banks: u8, chr_banks: u8) -> Self {
        Self {
            prg_banks,
            chr_banks,
            prg_bank_select: 0,

            latch0: Latch::FD,
            chr_fd_bank_select0: 0,
            chr_fe_bank_select0: 0,

            latch1: Latch::FE,
            chr_fd_bank_select1: 0,
            chr_fe_bank_select1: 0,

            mirroring: Mirroring::Vertical,
        }
    }
}

impl Mapper for Mapper9 {
    fn map_prg_read(&self, addr: u16) -> Option<MapRead> {
        match addr {
            0x8000..=0x9FFF => {
                let bank = self.prg_bank_select as usize;
                let addr = bank * PRG_ROM_BANK_SIZE + (addr & 0x1FFF) as usize;
                Some(MapRead::Address(addr))
            }
            0xA000..=0xFFFF => {
                let bank_offset = (addr - 0xA000) / (PRG_ROM_BANK_SIZE as u16);

                let bank = (self.prg_banks - 3 + bank_offset as u8) as usize;
                let addr = bank * PRG_ROM_BANK_SIZE + (addr & 0x1FFF) as usize;
                Some(MapRead::Address(addr))
            }
            _ => None,
        }
    }

    fn map_prg_write(&mut self, addr: u16, data: u8) -> Option<MapWrite> {
        match addr {
            0xA000..=0xAFFF => {
                self.prg_bank_select = data & 0x0F;
                Some(MapWrite::WroteRegister)
            }
            0xB000..=0xBFFF => {
                self.chr_fd_bank_select0 = data & 0x1F;
                Some(MapWrite::WroteRegister)
            }
            0xC000..=0xCFFF => {
                self.chr_fe_bank_select0 = data & 0x1F;
                Some(MapWrite::WroteRegister)
            }
            0xD000..=0xDFFF => {
                self.chr_fd_bank_select1 = data & 0x1F;
                Some(MapWrite::WroteRegister)
            }
            0xE000..=0xEFFF => {
                self.chr_fe_bank_select1 = data & 0x1F;
                Some(MapWrite::WroteRegister)
            }
            0xF000..=0xFFFF => {
                if data & 0x01 == 0 {
                    self.mirroring = Mirroring::Vertical;
                } else {
                    self.mirroring = Mirroring::Horizontal;
                }
                Some(MapWrite::WroteRegister)
            }
            _ => None,
        }
    }

    fn map_chr_read(&mut self, addr: u16) -> Option<MapRead> {
        let mapped_addr = match addr {
            0x0000..=0x0FFF => {
                let bank = match self.latch0 {
                    Latch::FD => self.chr_fd_bank_select0,
                    Latch::FE => self.chr_fe_bank_select0,
                } as usize;

                bank * CHR_ROM_BANK_SIZE + addr as usize
            }
            0x1000..=0x1FFF => {
                let bank = match self.latch1 {
                    Latch::FD => self.chr_fd_bank_select1,
                    Latch::FE => self.chr_fe_bank_select1,
                } as usize;

                bank * CHR_ROM_BANK_SIZE + (addr - 0x1000) as usize
            }
            _ => return None,
        };

        match addr {
            0x0FD8 => self.latch0 = Latch::FD,
            0x0FE8 => self.latch0 = Latch::FE,
            0x1FD8..=0x1FDF => self.latch1 = Latch::FD,
            0x1FE8..=0x1FEF => self.latch1 = Latch::FE,
            _ => {}
        }

        Some(MapRead::Address(mapped_addr))
    }

    fn map_chr_read_debug(&mut self, addr: u16) -> Option<MapRead> {
        let mapped_addr = match addr {
            0x0000..=0x0FFF => {
                let bank = match self.latch0 {
                    Latch::FD => self.chr_fd_bank_select0,
                    Latch::FE => self.chr_fe_bank_select0,
                } as usize;

                bank * CHR_ROM_BANK_SIZE + addr as usize
            }
            0x1000..=0x1FFF => {
                let bank = match self.latch1 {
                    Latch::FD => self.chr_fd_bank_select1,
                    Latch::FE => self.chr_fe_bank_select1,
                } as usize;

                bank * CHR_ROM_BANK_SIZE + (addr - 0x1000) as usize
            }
            _ => return None,
        };

        Some(MapRead::Address(mapped_addr))
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
}
