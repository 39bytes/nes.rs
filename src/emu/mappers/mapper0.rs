use anyhow::{anyhow, Result};

use super::{MapRead, MapWrite, Mapper};

const BANK_SIZE: u16 = 16 * 1024;
const RAM_SIZE: usize = 8 * 1024;

pub struct Mapper0 {
    prg_banks: u8,
    ram: [u8; RAM_SIZE],
}

impl Mapper0 {
    pub fn new(num_banks: u8) -> Self {
        Self {
            prg_banks: num_banks,
            ram: [0; RAM_SIZE],
        }
    }
}

impl Mapper for Mapper0 {
    fn cpu_map_read(&self, addr: u16) -> Result<MapRead> {
        match addr {
            0x6000..=0x7FFF => Ok(MapRead::RAMData(self.ram[(addr - 0x6000) as usize])),
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
            0x6000..=0x7FFF => {
                self.ram[(addr - 0x6000) as usize] = data;
                Ok(MapWrite::RAMWritten)
            }
            0x8000..=0xFFFF => Err(anyhow!("PRG ROM not writable")),
            _ => Err(anyhow!("Address {:#06X} out of range", addr)),
        }
    }

    fn ppu_map_read(&self, addr: u16) -> Result<MapRead> {
        if addr > 0x1FFF {
            return Err(anyhow!("Address {:#06X} out of range", addr));
        }

        Ok(MapRead::Address(addr as usize))
    }

    fn ppu_map_write(&self, _addr: u16) -> Result<MapWrite> {
        Err(anyhow!("Can't write to ROM"))
    }
}
