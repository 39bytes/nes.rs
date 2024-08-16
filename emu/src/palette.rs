use anyhow::{bail, Result};
use std::{fs::File, io::prelude::*};

#[derive(Debug, Clone, Copy, Default)]
pub struct Color(pub u8, pub u8, pub u8);

impl Color {
    pub const WHITE: Self = Color(255, 255, 255);
    pub const GRAY: Self = Color(128, 128, 128);
    pub const BLACK: Self = Color(0, 0, 0);
}

#[derive(Debug, Clone)]
pub struct Palette {
    colors: Vec<Color>,
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

        Self::from_bytes(&buf)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 192 {
            bail!("Palette length must be exactly 192 bytes");
        }

        let colors = bytes
            .chunks_exact(3)
            .map(|c| Color(c[0], c[1], c[2]))
            .collect::<Vec<_>>();

        Ok(Self { colors })
    }

    pub fn colors(&self) -> &Vec<Color> {
        &self.colors
    }

    pub fn get_color(&self, color: u8) -> Color {
        self.colors[(color % 64) as usize]
    }
}
