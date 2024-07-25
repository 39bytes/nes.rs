use anyhow::Result;

mod mapper0;
mod mapper1;
mod mapper2;
mod mapper3;
mod mapper9;

pub use mapper0::Mapper0;
pub use mapper1::Mapper1;
pub use mapper2::Mapper2;
pub use mapper3::Mapper3;
pub use mapper9::Mapper9;

use super::cartridge::Mirroring;

pub enum MapRead {
    Address(usize), // Mapper returns an address to index into the cartridge
    RAMData(u8),    // Mapper returns data from its onboard RAM
}

pub enum MapWrite {
    Address(usize),
    RAMWritten,
    WroteRegister,
}

pub trait Mapper {
    fn map_prg_read(&self, addr: u16) -> Result<MapRead>;
    fn map_prg_write(&mut self, addr: u16, data: u8) -> Result<MapWrite>;
    fn map_chr_read(&mut self, addr: u16) -> Result<MapRead>;
    fn map_chr_write(&self, addr: u16) -> Result<MapWrite>;
    fn mirroring(&self) -> Option<Mirroring> {
        None
    }
}
