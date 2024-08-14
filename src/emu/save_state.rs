use anyhow::{Context, Result};
use bincode::config;
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::PathBuf,
};

use super::{apu::ApuState, cpu::CpuState, ppu::PpuState};

#[derive(Serialize, Deserialize)]
pub struct SaveState {
    pub cpu_state: CpuState,
    pub ppu_state: PpuState,
    pub apu_state: ApuState,

    pub clock_count: u64,
    pub paused: bool,
}

impl SaveState {
    pub fn load(number: usize, rom_hash: u64) -> Result<Self> {
        let path = Self::rom_save_state_path(number, rom_hash)?;
        let bytes = std::fs::read(path)?;

        let config = config::standard();
        let (save_state, _): (SaveState, usize) =
            bincode::serde::decode_from_slice(bytes.as_slice(), config)?;

        Ok(save_state)
    }

    pub fn write(&self, number: usize, rom_hash: u64) -> Result<()> {
        let path = Self::rom_save_state_path(number, rom_hash)?;
        let parent = path
            .parent()
            .context("ROM save state directory has no parent")?;
        create_dir_all(parent)?;
        let mut file = File::create(path)?;

        let config = config::standard();
        let bytes = bincode::serde::encode_to_vec(self, config)?;
        file.write_all(bytes.as_slice())?;

        Ok(())
    }

    fn rom_save_state_path(number: usize, rom_hash: u64) -> Result<PathBuf> {
        let mut dir = data_dir().context("No data dir found")?;

        dir.push(format!("{:x}", rom_hash));
        dir.push(format!("{}.bin", number));

        Ok(dir)
    }
}

fn data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("nesrs"))
}
