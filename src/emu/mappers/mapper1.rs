use super::{MapRead, MapWrite, Mapper};
use anyhow::Result;

pub struct Mapper1 {}

impl Mapper for Mapper1 {
    fn cpu_map_read(&self, addr: u16) -> Result<MapRead> {
        todo!()
    }

    fn cpu_map_write(&mut self, addr: u16, data: u8) -> Result<MapWrite> {
        todo!()
    }

    fn ppu_map_read(&self, addr: u16) -> Result<MapRead> {
        todo!()
    }

    fn ppu_map_write(&self, addr: u16) -> Result<MapWrite> {
        todo!()
    }
}
