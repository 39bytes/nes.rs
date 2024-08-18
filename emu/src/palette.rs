use anyhow::{bail, Result};
use std::{fs::File, io::prelude::*};

#[derive(Debug, Clone, Copy, Default)]
pub struct Color(pub u8, pub u8, pub u8);

impl Color {
    pub const WHITE: Self = Color(255, 255, 255);
    pub const GRAY: Self = Color(128, 128, 128);
    pub const BLACK: Self = Color(0, 0, 0);

    pub fn as_slice(&self) -> [u8; 3] {
        [self.0, self.1, self.2]
    }
}

#[derive(Debug, Clone)]
pub struct Palette {
    bytes: Vec<u8>,
}

pub struct Pixel {
    pub x: usize,
    pub y: usize,
    pub color: Color,
}

impl Default for Palette {
    fn default() -> Self {
        let pal = include_bytes!("../../assets/palettes/2C02G.pal");
        Self::from_bytes(pal).unwrap()
    }
}

impl Palette {
    #[allow(dead_code)]
    pub fn load(path: &str) -> Result<Self> {
        let mut f = File::open(path)?;

        // Palettes are exactly 192 bytes, 3 bytes per color (64 colors)
        let mut buf = [0; 192];
        f.read_exact(&mut buf)?;

        Ok(Self {
            bytes: buf.to_vec(),
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 192 {
            bail!("Palette length must be exactly 192 bytes");
        }

        Ok(Self {
            bytes: bytes.to_vec(),
        })
    }

    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    pub fn get_color(&self, color: u8) -> Color {
        let i = ((color % 64) * 3) as usize;
        Color(self.bytes[i], self.bytes[i + 1], self.bytes[i + 2])
    }
}
