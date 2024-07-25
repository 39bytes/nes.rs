use anyhow::Result;
use std::{fs::File, io::prelude::*};

use crate::renderer::{Color, Sprite};

#[derive(Debug, Clone)]
pub struct Palette {
    colors: Vec<Color>,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            colors: Vec::from([Color::BLACK; 64]),
        }
    }
}

impl Palette {
    pub fn load(path: &str) -> Result<Self> {
        let mut f = File::open(path)?;

        // Palettes are exactly 192 bytes, 3 bytes per color (64 colors)
        let mut buf = [0; 192];
        f.read_exact(&mut buf)?;

        let colors = buf
            .chunks_exact(3)
            .map(|c| Color(c[0], c[1], c[2]))
            .collect::<Vec<_>>();

        Ok(Palette { colors })
    }

    pub fn colors(&self) -> &Vec<Color> {
        &self.colors
    }

    pub fn get_color(&self, color: u8) -> Color {
        self.colors[(color % 64) as usize]
    }
}

impl From<Palette> for Sprite {
    fn from(value: Palette) -> Self {
        Sprite::new(value.colors().clone(), 16, 4).unwrap()
    }
}
