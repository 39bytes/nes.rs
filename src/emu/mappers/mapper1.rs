use super::{MapRead, MapWrite, Mapper};
use anyhow::Result;

pub struct Mapper1 {}

impl Mapper for Mapper1 {
    fn map_prg_read(&self, addr: u16) -> Result<MapRead> {
        todo!()
    }

    fn map_prg_write(&mut self, addr: u16, data: u8) -> Result<MapWrite> {
        todo!()
    }

    fn map_chr_read(&self, addr: u16) -> Result<MapRead> {
        todo!()
    }

    fn map_chr_write(&self, addr: u16) -> Result<MapWrite> {
        todo!()
    }
}
