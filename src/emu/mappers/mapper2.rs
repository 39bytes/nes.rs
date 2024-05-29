use super::{MapRead, MapWrite, Mapper};
use anyhow::{anyhow, Result};

pub struct Mapper2 {
    prg_banks: u8,
    chr_banks: u8,
    bank_select: u8,
}

const BANK_SIZE: usize = 16 * 1024;

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
    fn map_prg_read(&self, addr: u16) -> Result<MapRead> {
        match addr {
            0x8000..=0xBFFF => {
                let bank = self.bank_select as usize;
                let addr = bank * BANK_SIZE + (addr & 0x3FFF) as usize;
                Ok(MapRead::Address(addr))
            }
            0xC000..=0xFFFF => {
                let bank = (self.prg_banks - 1) as usize;
                let addr = bank * BANK_SIZE + (addr & 0x3FFF) as usize;
                Ok(MapRead::Address(addr))
            }
            _ => Err(anyhow!("Address {:#06X} out of range", addr)),
        }
    }
    fn map_prg_write(&mut self, addr: u16, data: u8) -> Result<MapWrite> {
        match addr {
            0x8000..=0xFFFF => {
                self.bank_select = data & 0x0F;
                Ok(MapWrite::WroteRegister)
            }
            _ => Err(anyhow!("Address {:#06X} out of range", addr)),
        }
    }

    fn map_chr_read(&self, addr: u16) -> Result<MapRead> {
        if addr > 0x1FFF {
            return Err(anyhow!("Address {:#06X} out of range", addr));
        }

        Ok(MapRead::Address(addr as usize))
    }

    fn map_chr_write(&self, addr: u16) -> Result<MapWrite> {
        if addr > 0x1FFF {
            return Err(anyhow!("Address {:#06X} out of range", addr));
        }
        if self.chr_banks > 0 {
            return Err(anyhow!("Can't write to ROM"));
        }

        Ok(MapWrite::Address(addr as usize))
    }
}
