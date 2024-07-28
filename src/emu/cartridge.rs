use anyhow::{anyhow, Result};
use bitflags::bitflags;
use std::{
    fs::File,
    io::{prelude::*, SeekFrom},
    path::Path,
};

use super::mappers::*;

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
#[derive(Debug)]
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
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Header {
            name: [bytes[0], bytes[1], bytes[2], bytes[3]],
            prg_rom_chunks: bytes[4],
            chr_rom_chunks: bytes[5],
            flags6: Flags6::from_bits_truncate(bytes[6]),
            flags7: Flags7::from_bits_truncate(bytes[7]),
            mapper_num: (bytes[7] & 0xF0) | (bytes[6] >> 4),
            prg_ram_size: bytes[8],
            tv_system1: bytes[9],
            tv_system2: bytes[10],
        }
    }
}

#[derive(Clone, Copy)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    SingleScreenLower,
    SingleScreenUpper,
}

pub struct Cartridge {
    prg_memory: Vec<u8>,
    chr_memory: Vec<u8>,

    mapper: Box<dyn Mapper>,
    mirroring: Mirroring,
}

impl Cartridge {
    pub fn new<T: AsRef<Path>>(rom_path: T) -> Result<Self> {
        let path = rom_path.as_ref();

        log::info!("Loading ROM: {}", path.display());
        let mut f = File::open(rom_path)?;

        let mut header_buf = [0; 16];
        f.read_exact(&mut header_buf)?;

        let header = Header::from_bytes(header_buf);
        log::info!("Header: {:?}", header);

        if header.flags6.contains(Flags6::HasTrainer) {
            log::info!("Rom has trainer info, skipping 512 bytes");
            f.seek(SeekFrom::Current(512))?;
        }

        let mirroring = if header.flags6.contains(Flags6::Mirroring) {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let (prg_rom, chr_rom) = Cartridge::from_ines1(f, &header)?;

        let mapper: Box<dyn Mapper> = match header.mapper_num {
            0 => Box::new(Mapper0::new(header.prg_rom_chunks)),
            1 => Box::new(Mapper1::new(header.prg_rom_chunks, header.chr_rom_chunks)),
            2 => Box::new(Mapper2::new(header.prg_rom_chunks, header.chr_rom_chunks)),
            3 => Box::new(Mapper3::new(header.prg_rom_chunks, header.chr_rom_chunks)),
            4 => Box::new(Mapper4::new(
                header.prg_rom_chunks * 2,
                header.chr_rom_chunks * 8,
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
            prg_memory: prg_rom,
            chr_memory: chr_rom,
            mapper,
            mirroring,
        })
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mapper.mirroring().unwrap_or(self.mirroring)
    }

    fn from_ines1(mut f: File, header: &Header) -> Result<(Vec<u8>, Vec<u8>)> {
        let prg_rom_size = (header.prg_rom_chunks as usize) * PRG_ROM_CHUNK_SIZE;
        log::info!("Reading {} bytes of program ROM", prg_rom_size);

        let mut prg_mem = vec![0u8; prg_rom_size];
        f.read_exact(prg_mem.as_mut_slice())?;

        let mut chr_mem;

        if header.chr_rom_chunks == 0 {
            log::info!("No character ROM, allocating 8 KB of character RAM");

            chr_mem = vec![0u8; CHR_ROM_CHUNK_SIZE];
        } else {
            let chr_rom_size = (header.chr_rom_chunks as usize) * CHR_ROM_CHUNK_SIZE;
            log::info!("Reading {} bytes of character ROM", chr_rom_size);

            chr_mem = vec![0u8; chr_rom_size];
            f.read_exact(chr_mem.as_mut_slice())?;
        }

        Ok((prg_mem, chr_mem))
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        if let Some(MapWrite::Address(addr)) = self.mapper.map_prg_write(addr, data) {
            self.prg_memory[addr] = data;
        }
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
}
