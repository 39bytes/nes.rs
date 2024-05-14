use anyhow::{anyhow, Result};

pub enum MapRead {
    Address(u16), // Mapper returns an address to index into the cartridge
    RAMData(u8),  // Mapper returns data from its onboard RAM
}

pub enum MapWrite {
    Address(u16),
    RAMWritten,
}

pub trait Mapper {
    fn cpu_map_read(&self, addr: u16) -> Result<MapRead>;
    fn cpu_map_write(&mut self, addr: u16, data: u8) -> Result<MapWrite>;
    fn ppu_map_read(&self, addr: u16) -> Result<u16>;
    fn ppu_map_write(&self, addr: u16) -> Result<u16>;
}

const BANK_SIZE: u16 = 16 * 1024;
const RAM_SIZE: usize = 8 * 1024;

pub struct Mapper0 {
    num_banks: u8,
    ram: [u8; RAM_SIZE],
}

impl Mapper0 {
    pub fn new(num_banks: u8) -> Self {
        Self {
            num_banks,
            ram: [0; RAM_SIZE],
        }
    }
}

impl Mapper for Mapper0 {
    fn cpu_map_read(&self, addr: u16) -> Result<MapRead> {
        match addr {
            0x0000..=0x5FFF => Err(anyhow!("Address {:#06X} out of range", addr)),
            0x6000..=0x7FFF => Ok(MapRead::RAMData(self.ram[(addr - 0x6000) as usize])),
            0x8000..=0xFFFF => {
                if self.num_banks == 1 {
                    Ok(MapRead::Address(addr & 0x3FFF))
                } else {
                    Ok(MapRead::Address(addr & 0x7FFF))
                }
            }
        }
    }

    fn cpu_map_write(&mut self, addr: u16, data: u8) -> Result<MapWrite> {
        match addr {
            0x0000..=0x5FFF => Err(anyhow!("Address {:#06X} out of range", addr)),
            0x6000..=0x7FFF => {
                self.ram[(addr - 0x6000) as usize] = data;
                Ok(MapWrite::RAMWritten)
            }
            0x8000..=0xFFFF => {
                if self.num_banks == 1 {
                    Ok(MapWrite::Address(addr & 0x3FFF))
                } else {
                    Ok(MapWrite::Address(addr & 0x7FFF))
                }
            }
        }
    }

    fn ppu_map_read(&self, addr: u16) -> Result<u16> {
        if addr > 0x1FFF {
            return Err(anyhow!("Address {:#06X} out of range", addr));
        }

        Ok(addr)
    }

    fn ppu_map_write(&self, addr: u16) -> Result<u16> {
        Err(anyhow!("Can't write to ROM"))
    }
}
