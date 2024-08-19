mod mapper0;
mod mapper1;
mod mapper2;
mod mapper3;
mod mapper4;
mod mapper9;

pub use mapper0::Mapper0;
pub use mapper1::Mapper1;
pub use mapper2::Mapper2;
pub use mapper3::Mapper3;
pub use mapper4::Mapper4;
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
    AcknowledgeIRQ,
}

pub trait Mapper {
    fn map_prg_read(&self, addr: u16) -> Option<MapRead>;
    fn map_prg_write(&mut self, addr: u16, data: u8) -> Option<MapWrite>;
    fn map_chr_read(&mut self, addr: u16) -> Option<MapRead>;
    fn map_chr_write(&self, addr: u16) -> Option<MapWrite>;
    fn mirroring(&self) -> Option<Mirroring> {
        None
    }
    fn map_chr_read_debug(&mut self, addr: u16) -> Option<MapRead> {
        self.map_chr_read(addr)
    }
    // Returns a boolean indicating if an IRQ should be triggered
    // This is to support Mapper 4's IRQ functionality
    fn on_scanline_hblank(&mut self) -> bool {
        false
    }

    // Get the onboard ram if it exists (for storing save files)
    fn onboard_ram(&self) -> Option<&[u8]> {
        None
    }

    fn load_onboard_ram(&mut self, _ram: &[u8]) {}
}
