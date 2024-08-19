use super::{MapRead, MapWrite, Mapper};

pub struct Mapper2 {
    prg_banks: u8,
    chr_banks: u8,
    bank_select: u8,
}

const PRG_BANK_SIZE: usize = 16 * 1024;

impl Mapper2 {
    pub fn new(prg_banks: u8, chr_banks: u8) -> Self {
        Self {
            prg_banks,
            chr_banks,
            bank_select: 0,
        }
    }
}

impl Mapper for Mapper2 {
    fn map_prg_read(&self, addr: u16) -> Option<MapRead> {
        match addr {
            0x8000..=0xBFFF => {
                let bank = self.bank_select as usize;
                let addr = bank * PRG_BANK_SIZE + (addr & 0x3FFF) as usize;
                Some(MapRead::Address(addr))
            }
            0xC000..=0xFFFF => {
                let bank = (self.prg_banks - 1) as usize;
                let addr = bank * PRG_BANK_SIZE + (addr & 0x3FFF) as usize;
                Some(MapRead::Address(addr))
            }
            _ => None,
        }
    }
    fn map_prg_write(&mut self, addr: u16, data: u8) -> Option<MapWrite> {
        match addr {
            0x8000..=0xFFFF => {
                self.bank_select = data & 0x0F;
                Some(MapWrite::WroteRegister)
            }
            _ => None,
        }
    }

    fn map_chr_read(&mut self, addr: u16) -> Option<MapRead> {
        if addr > 0x1FFF {
            return None;
        }

        Some(MapRead::Address(addr as usize))
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
