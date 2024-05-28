use anyhow::Result;

mod mapper0;
mod mapper1;
mod mapper2;
mod mapper3;

pub use mapper0::Mapper0;
pub use mapper1::Mapper1;
pub use mapper2::Mapper2;
pub use mapper3::Mapper3;

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
    fn cpu_map_read(&self, addr: u16) -> Result<MapRead>;
    fn cpu_map_write(&mut self, addr: u16, data: u8) -> Result<MapWrite>;
    fn ppu_map_read(&self, addr: u16) -> Result<MapRead>;
    fn ppu_map_write(&self, addr: u16) -> Result<MapWrite>;
}
