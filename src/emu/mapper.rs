use anyhow::{anyhow, Result};

pub trait Mapper {
    fn cpu_map_read(&self, addr: u16) -> Result<u16>;
    fn cpu_map_write(&self, addr: u16) -> Result<u16>;
    fn ppu_map_read(&self, addr: u16) -> Result<u16>;
    fn ppu_map_write(&self, addr: u16) -> Result<u16>;
}

const BANK_SIZE: u16 = 16 * 1024;

pub struct Mapper0 {
    num_banks: u8,
}

impl Mapper0 {
    pub fn new(num_banks: u8) -> Self {
        Self { num_banks }
    }
}

impl Mapper for Mapper0 {
    fn cpu_map_read(&self, addr: u16) -> Result<u16> {
        if addr < 0x8000 {
            return Err(anyhow!("Address out of range"));
        }

        if self.num_banks == 1 {
            Ok((addr - 0x8000) % BANK_SIZE + 0x8000)
        } else {
            Ok(addr)
        }
    }

    fn cpu_map_write(&self, addr: u16) -> Result<u16> {
        if addr < 0x8000 {
            return Err(anyhow!("Address out of range"));
        }

        if self.num_banks == 1 {
            Ok((addr - 0x8000) % BANK_SIZE + 0x8000)
        } else {
            Ok(addr)
        }
    }

    fn ppu_map_read(&self, addr: u16) -> Result<u16> {
        if addr > 0x1FFF {
            return Err(anyhow!("Address out of range"));
        }

        Ok(addr)
    }

    fn ppu_map_write(&self, addr: u16) -> Result<u16> {
        Err(anyhow!("Can't write to ROM"))
    }
}
