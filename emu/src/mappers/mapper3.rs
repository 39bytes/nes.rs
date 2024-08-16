use super::{MapRead, MapWrite, Mapper};

pub struct Mapper3 {
    prg_banks: u8,
    chr_banks: u8,
    bank_select: u8,
}

const CHR_BANK_SIZE: usize = 8 * 1024;

impl Mapper3 {
    pub fn new(prg_banks: u8, chr_banks: u8) -> Self {
        Self {
            prg_banks,
            chr_banks,
            bank_select: 0,
        }
    }
}

impl Mapper for Mapper3 {
    fn map_prg_read(&self, addr: u16) -> Option<MapRead> {
        match addr {
            0x8000..=0xFFFF => {
                let addr = if self.prg_banks == 1 {
                    addr & 0x3FFF
                } else {
                    addr & 0x7FFF
                };

                Some(MapRead::Address(addr as usize))
            }
            _ => None,
        }
    }

    fn map_prg_write(&mut self, addr: u16, data: u8) -> Option<MapWrite> {
        match addr {
            0x8000..=0xFFFF => {
                self.bank_select = data & 0x03;
                Some(MapWrite::WroteRegister)
            }
            _ => None,
        }
    }

    fn map_chr_read(&mut self, addr: u16) -> Option<MapRead> {
        if addr > 0x1FFF {
            return None;
        }

        let bank = self.bank_select as usize;
        let addr = bank * CHR_BANK_SIZE + addr as usize;
        Some(MapRead::Address(addr))
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
}
