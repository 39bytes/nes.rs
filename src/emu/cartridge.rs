use anyhow::{anyhow, Result};
use bitflags::bitflags;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;

use super::mapper::{Mapper, Mapper0};

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

/// The iNES format file header
#[derive(Debug)]
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

pub struct Cartridge {
    prg_memory: Vec<u8>,
    chr_memory: Vec<u8>,

    mapper: Box<dyn Mapper>,
}

impl Cartridge {
    pub fn new(rom_path: &Path) -> Result<Self> {
        log::info!("Loading ROM: {}", rom_path.display());
        let mut f = File::open(rom_path)?;

        let mut header_buf = [0; 16];
        f.read_exact(&mut header_buf)?;

        let header = Header::from_bytes(header_buf);
        log::info!("Header: {:?}", header);

        if header.flags6.contains(Flags6::HasTrainer) {
            log::info!("Rom has trainer info, skipping 512 bytes");
            f.seek(SeekFrom::Current(512))?;
        }

        let file_type = 1;

        let (prg_rom, chr_rom) = match file_type {
            0 => todo!(),
            1 => Cartridge::from_type1(f, &header)?,
            2 => todo!(),
            _ => panic!("Invalid file type"),
        };

        let mapper = match header.mapper_num {
            0 => Box::new(Mapper0::new(header.prg_rom_chunks)),
            _ => Err(anyhow!("Unimplemented mapper"))?,
        };

        Ok(Cartridge {
            prg_memory: prg_rom,
            chr_memory: chr_rom,
            mapper,
        })
    }

    fn from_type1(mut f: File, header: &Header) -> Result<(Vec<u8>, Vec<u8>)> {
        let prg_rom_size = (header.prg_rom_chunks as usize) * 16 * 1024;
        log::info!("Reading {} bytes of program ROM", prg_rom_size);

        let mut prg_rom = vec![0u8; prg_rom_size];
        f.read_exact(prg_rom.as_mut_slice())?;
        log::info!("Program ROM size: {}", prg_rom.len());

        let chr_rom_size = (header.chr_rom_chunks as usize) * 8 * 1024;
        log::info!("Reading {} bytes of character ROM", chr_rom_size);

        let mut chr_rom = vec![0u8; chr_rom_size];
        f.read_exact(chr_rom.as_mut_slice())?;
        log::info!("Character ROM size: {}", chr_rom.len());

        Ok((prg_rom, chr_rom))
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) -> Result<()> {
        self.prg_memory[self.mapper.cpu_map_write(addr)? as usize] = data;
        Ok(())
    }

    pub fn cpu_read(&self, addr: u16) -> Result<u8> {
        Ok(self.prg_memory[self.mapper.cpu_map_read(addr)? as usize])
    }

    pub fn ppu_write(&mut self, addr: u16, data: u8) -> Result<()> {
        self.chr_memory[self.mapper.ppu_map_write(addr)? as usize] = data;
        Ok(())
    }

    pub fn ppu_read(&self, addr: u16) -> Result<u8> {
        Ok(self.chr_memory[self.mapper.ppu_map_read(addr)? as usize])
    }
}
