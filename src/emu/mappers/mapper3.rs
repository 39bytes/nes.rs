use super::{MapRead, MapWrite, Mapper};
use anyhow::{anyhow, Result};

pub struct Mapper3 {
    prg_banks: u8,
    chr_banks: u8,
    bank_select: u8,
}

const BANK_SIZE: usize = 8 * 1024;

impl Mapper3 {
    pub fn new(prg_banks: u8, chr_banks: u8) -> Self {
        Mapper3 {
            prg_banks,
            chr_banks,
            bank_select: 0,
        }
    }
}

impl Mapper for Mapper3 {
    fn cpu_map_read(&self, addr: u16) -> Result<MapRead> {
        match addr {
            0x8000..=0xFFFF => {
                let addr = if self.prg_banks == 1 {
                    addr & 0x3FFF
                } else {
                    addr & 0x7FFF
                };

                Ok(MapRead::Address(addr as usize))
            }
            _ => Err(anyhow!("Address {:#06X} out of range", addr)),
        }
    }

    fn cpu_map_write(&mut self, addr: u16, data: u8) -> Result<MapWrite> {
        match addr {
            0x8000..=0xFFFF => {
                self.bank_select = data;
                Ok(MapWrite::WroteRegister)
            }
            _ => Err(anyhow!("Address {:#06X} out of range", addr)),
        }
    }

    fn ppu_map_read(&self, addr: u16) -> Result<MapRead> {
        if addr > 0x1FFF {
            return Err(anyhow!("Address {:#06X} out of range", addr));
        }

        let bank = self.bank_select as usize;
        let addr = bank * BANK_SIZE + addr as usize;
        Ok(MapRead::Address(addr))
    }

    fn ppu_map_write(&self, addr: u16) -> Result<MapWrite> {
        if addr > 0x1FFF {
            return Err(anyhow!("Address {:#06X} out of range", addr));
        }
        if self.chr_banks > 0 {
            return Err(anyhow!("Can't write to ROM"));
        }

        Ok(MapWrite::Address(addr as usize))
    }
}
