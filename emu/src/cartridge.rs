use anyhow::{anyhow, bail, Result};
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
};

use super::{
    mappers::*,
    save::{load_bin, write_bin},
};

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct Flags6: u8 {
        /// Horizontal (0) or vertical (1)
        const Mirroring = 1 << 0;
        /// Cartridge contains battery-backed PRG RAM ($6000-7FFF) or other persistent memory
        const BatteryBacked = 1 << 1;
        /// Has 512 byte trainer data after header
        const HasTrainer = 1 << 2;
        /// Provide four-screen VRAM
        const IgnoreMirroring = 1 << 3;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct Flags7: u8 {
        const VSUnisystem = 1 << 0;
        /// 8 KB of Hint Screen data stored after CHR data
        const PlayChoice10 = 1 << 1;
        /// Flags 8-15 are in NES 2.0 format if these bit 1 is set but not bit 0
        const FlagFormatBit0 = 1 << 2;
        const FlagFormatBit1 = 1 << 3;
    }
}

const PRG_ROM_CHUNK_SIZE: usize = 16 * 1024;
const CHR_ROM_CHUNK_SIZE: usize = 8 * 1024;

/// The iNES format file header
#[derive(Debug, Hash)]
#[allow(dead_code)]
struct Header {
    name: [u8; 4],
    prg_rom_chunks: u8,
    chr_rom_chunks: u8,
    flags6: Flags6,
    flags7: Flags7,
    mapper_num: u8,
    prg_ram_size: u8,
    tv_system1: u8,
    tv_system2: u8,
}

impl Header {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 16 {
            bail!("Header must be exactly 16 bytes long");
        }

        Ok(Header {
            name: [bytes[0], bytes[1], bytes[2], bytes[3]],
            prg_rom_chunks: bytes[4],
            chr_rom_chunks: bytes[5],
            flags6: Flags6::from_bits_truncate(bytes[6]),
            flags7: Flags7::from_bits_truncate(bytes[7]),
            mapper_num: (bytes[7] & 0xF0) | (bytes[6] >> 4),
            prg_ram_size: bytes[8],
            tv_system1: bytes[9],
            tv_system2: bytes[10],
        })
    }
}

impl Header {
    fn has_chr_ram(&self) -> bool {
        self.chr_rom_chunks == 0
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    SingleScreenLower,
    SingleScreenUpper,
}

struct CartridgeData {
    pub prg_memory: Vec<u8>,
    pub chr_memory: Vec<u8>,
}

pub struct Cartridge {
    prg_memory: Vec<u8>,
    chr_memory: Vec<u8>,

    mapper: Box<dyn Mapper>,
    mirroring: Mirroring,

    header: Header,
}

impl Cartridge {
    pub fn new<T: AsRef<Path>>(rom_path: T) -> Result<Self> {
        let path = rom_path.as_ref();

        log::info!("Loading ROM: {}", path.display());
        let bytes = std::fs::read(path)?;

        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let header = Header::from_bytes(&bytes[0..16])?;
        log::info!("Header: {:?}", header);

        let mut i = 16;

        if header.flags6.contains(Flags6::HasTrainer) {
            log::info!("Rom has trainer info, skipping 512 bytes");
            i += 512;
        }

        let mirroring = if header.flags6.contains(Flags6::Mirroring) {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let CartridgeData {
            prg_memory,
            chr_memory,
        } = Cartridge::from_ines1(&bytes[i..], &header)?;

        let mapper: Box<dyn Mapper> = match header.mapper_num {
            0 => Box::new(Mapper0::new(header.prg_rom_chunks, header.has_chr_ram())),
            1 => Box::new(Mapper1::new(header.prg_rom_chunks, header.chr_rom_chunks)),
            2 => Box::new(Mapper2::new(header.prg_rom_chunks, header.chr_rom_chunks)),
            3 => Box::new(Mapper3::new(header.prg_rom_chunks, header.chr_rom_chunks)),
            4 => Box::new(Mapper4::new(
                (header.prg_rom_chunks as usize) * 2,
                (header.chr_rom_chunks as usize) * 8,
            )),
            9 => Box::new(Mapper9::new(
                header.prg_rom_chunks * 2,
                header.chr_rom_chunks * 2,
            )),
            _ => Err(anyhow!(
                "This game uses mapper {}, which isn't implemented",
                header.mapper_num
            ))?,
        };

        Ok(Cartridge {
            prg_memory,
            chr_memory,
            mapper,
            mirroring,
            header,
        })
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mapper.mirroring().unwrap_or(self.mirroring)
    }

    fn from_ines1(bytes: &[u8], header: &Header) -> Result<CartridgeData> {
        let prg_rom_size = (header.prg_rom_chunks as usize) * PRG_ROM_CHUNK_SIZE;
        log::info!("Reading {} bytes of program ROM", prg_rom_size);

        let prg_memory = bytes[..prg_rom_size].to_vec();

        let chr_memory = if header.has_chr_ram() {
            log::info!("No character ROM, allocating 8 KB of character RAM");
            vec![0u8; CHR_ROM_CHUNK_SIZE]
        } else {
            let chr_rom_size = (header.chr_rom_chunks as usize) * CHR_ROM_CHUNK_SIZE;
            log::info!("Reading {} bytes of character ROM", chr_rom_size);
            bytes[prg_rom_size..prg_rom_size + chr_rom_size].to_vec()
        };

        Ok(CartridgeData {
            prg_memory,
            chr_memory,
        })
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) -> Option<MapWrite> {
        let res = self.mapper.map_prg_write(addr, data);
        if let Some(MapWrite::Address(addr)) = res {
            self.prg_memory[addr] = data;
        }
        res
    }

    pub fn cpu_read(&self, addr: u16) -> u8 {
        match self.mapper.map_prg_read(addr) {
            Some(MapRead::Address(addr)) => self.prg_memory[addr],
            Some(MapRead::RAMData(data)) => data,
            None => 0,
        }
    }

    pub fn ppu_write(&mut self, addr: u16, data: u8) {
        if let Some(MapWrite::Address(addr)) = self.mapper.map_chr_write(addr) {
            self.chr_memory[addr] = data;
        }
    }

    pub fn ppu_read(&mut self, addr: u16) -> u8 {
        match self.mapper.map_chr_read(addr) {
            Some(MapRead::Address(addr)) => self.chr_memory[addr],
            Some(MapRead::RAMData(data)) => data,
            None => 0,
        }
    }

    pub fn ppu_read_debug(&mut self, addr: u16) -> u8 {
        match self.mapper.map_chr_read_debug(addr) {
            Some(MapRead::Address(addr)) => self.chr_memory[addr],
            Some(MapRead::RAMData(data)) => data,
            None => 0,
        }
    }

    pub fn on_scanline_hblank(&mut self) -> bool {
        self.mapper.on_scanline_hblank()
    }

    pub fn compute_hash(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }

    pub fn write_save_file(&self) -> Result<()> {
        if !self.header.flags6.contains(Flags6::BatteryBacked) {
            log::info!("Cartridge does not have battery backed RAM");
            return Ok(());
        }

        if let Some(ram) = self.mapper.onboard_ram() {
            write_bin(ram, self.compute_hash(), "save.bin")?;
            log::info!("Wrote save file");
        }

        Ok(())
    }

    pub fn load_save_file(&mut self) {
        if !self.header.flags6.contains(Flags6::BatteryBacked) {
            log::error!("Cartridge does not have battery backed RAM");
            return;
        }

        match load_bin::<Vec<u8>>(self.compute_hash(), "save.bin") {
            Ok(ram) => self.mapper.load_onboard_ram(ram.as_slice()),
            Err(e) => {
                log::error!("Failed to load save data for ROM: {}", e);
                return;
            }
        }

        log::info!("Loading save file");
    }

    pub fn state(&self) -> CartridgeState {
        CartridgeState {
            mapper_onboard_ram: self.mapper.onboard_ram().map(|ram| ram.to_vec()),
            chr_ram: (self.header.chr_rom_chunks == 0).then(|| self.chr_memory.clone()),
        }
    }

    pub fn load_state(&mut self, state: &CartridgeState) {
        let CartridgeState {
            chr_ram,
            mapper_onboard_ram,
        } = state;

        if let Some(ram) = chr_ram {
            if self.header.has_chr_ram() {
                self.chr_memory.clone_from(ram);
            }
        }
        if let Some(ram) = mapper_onboard_ram {
            self.mapper.load_onboard_ram(ram.as_slice());
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct CartridgeState {
    pub mapper_onboard_ram: Option<Vec<u8>>,
    pub chr_ram: Option<Vec<u8>>,
}

impl Hash for Cartridge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.prg_memory.hash(state);
        self.header.hash(state);
    }
}
