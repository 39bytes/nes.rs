use super::{MapRead, MapWrite, Mapper};

const PRG_RAM_SIZE: usize = 8 * 1024;

pub struct Mapper0 {
    prg_banks: u8,
    ram: [u8; PRG_RAM_SIZE],
    allow_chr_ram: bool,
}

impl Mapper0 {
    pub fn new(num_banks: u8, allow_chr_ram: bool) -> Self {
        Self {
            prg_banks: num_banks,
            ram: [0; PRG_RAM_SIZE],
            allow_chr_ram,
        }
    }
}

impl Mapper for Mapper0 {
    fn map_prg_read(&self, addr: u16) -> Option<MapRead> {
        match addr {
            0x6000..=0x7FFF => Some(MapRead::RAMData(self.ram[(addr - 0x6000) as usize])),
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
            0x6000..=0x7FFF => {
                self.ram[(addr - 0x6000) as usize] = data;
                Some(MapWrite::RAMWritten)
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
        if addr > 0x1FFF || !self.allow_chr_ram {
            return None;
        }

        Some(MapWrite::Address(addr as usize))
    }

    fn onboard_ram(&self) -> Option<&[u8]> {
        Some(&self.ram)
    }

    fn load_onboard_ram(&mut self, ram: &[u8]) {
        self.ram.copy_from_slice(ram);
    }
}
