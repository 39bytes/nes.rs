use anyhow::{Context, Result};
use bincode::config;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::PathBuf,
};

use crate::{apu::ApuState, cartridge::CartridgeState, cpu::CpuState, ppu::PpuState};

#[derive(Serialize, Deserialize)]
pub struct SaveState {
    pub cpu_state: CpuState,
    pub ppu_state: PpuState,
    pub apu_state: ApuState,

    pub cartridge_state: CartridgeState,

    pub clock_count: u64,
    pub paused: bool,
}

impl SaveState {
    pub fn load(number: usize, rom_hash: u64) -> Result<Self> {
        load_bin(rom_hash, &format!("{}.bin", number))
    }

    pub fn write(&self, number: usize, rom_hash: u64) -> Result<()> {
        write_bin(self, rom_hash, &format!("{}.bin", number))
    }
}

pub fn load_bin<D: DeserializeOwned>(rom_hash: u64, file_name: &str) -> Result<D> {
    let dir = data_dir(rom_hash).context("No data dir found")?;
    let bytes = std::fs::read(dir.join(file_name))?;

    let config = config::standard();
    let (data, _): (D, usize) = bincode::serde::decode_from_slice(bytes.as_slice(), config)?;

    Ok(data)
}

pub fn write_bin<S: Serialize>(obj: S, rom_hash: u64, file_name: &str) -> Result<()> {
    let dir = data_dir(rom_hash).context("No data dir found")?;
    let file_name = dir.join(file_name);
    create_dir_all(dir)?;
    let mut file = File::create(file_name)?;

    let config = config::standard();
    let bytes = bincode::serde::encode_to_vec(obj, config)?;
    file.write_all(bytes.as_slice())?;

    Ok(())
}

fn data_dir(rom_hash: u64) -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("nesrs").join(format!("{:x}", rom_hash)))
}
