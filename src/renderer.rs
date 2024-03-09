use std::fmt::Display;

use anyhow::{anyhow, Result};
use pixels::{Pixels, SurfaceTexture};
use rusttype::{point, Font, Scale};
use winit::window::Window;

use crate::emu::palette::{Color, Palette};

pub struct Sprite {
    pixels: Vec<Color>,
    width: usize,
    height: usize,
}

impl Sprite {
    pub fn new(pixels: Vec<Color>, width: usize, height: usize) -> Result<Self> {
        if pixels.len() != width * height {
            return Err(anyhow!(
                "Width {} and height {} not assignable to pixel buffer of length {}",
                width,
                height,
                pixels.len()
            ));
        }

        Ok(Sprite {
            pixels,
            width,
            height,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn scale(self, scale: usize) -> Sprite {
        let new_width = self.width * scale;
        let new_height = self.height * scale;
        let mut scaled = Vec::with_capacity(new_width * new_height);

        for i in 0..new_height {
            for j in 0..new_width {
                scaled.push(self.pixels[(i / scale) * self.width + (j / scale)]);
            }
        }

        Sprite::new(scaled, self.width * scale, self.height * scale).unwrap()
    }
}

impl From<Palette> for Sprite {
    fn from(value: Palette) -> Self {
        Sprite::new(value.colors().clone(), 16, 4).unwrap()
    }
}

pub struct Renderer {
    font: Font<'static>,
    pixels: Pixels,
    width: usize,
    height: usize,
}

impl Renderer {
    const FONT_SIZE: usize = 20;

    pub fn new(font: Font<'static>, window: &Window, width: usize, height: usize) -> Result<Self> {
        let pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, window);
            Pixels::new(width as u32, height as u32, surface_texture)?
        };

        Ok(Renderer {
            font,
            pixels,
            width,
            height,
        })
    }

    pub fn clear(&mut self) {
        let frame = self.pixels.frame_mut();
        for x in frame.iter_mut() {
            *x = 0;
        }
    }

    pub fn pixels(&mut self) -> &mut Pixels {
        &mut self.pixels
    }

    pub fn render(&self) -> Result<(), pixels::Error> {
        self.pixels.render()
    }

    pub fn draw_sprite(&mut self, sprite: &Sprite, x: usize, y: usize) {
        for i in 0..sprite.height {
            for j in 0..sprite.width {
                let px = self.pixel_index(x + j, y + i);
                let frame = self.pixels.frame_mut();
                let pixel = sprite.pixels[i * sprite.width + j];

                frame[px] = pixel.0;
                frame[px + 1] = pixel.1;
                frame[px + 2] = pixel.2;
                frame[px + 3] = 255;
            }
        }
    }

    pub fn draw_text(&mut self, text: &str, x: usize, y: usize) {
        let scale = Scale::uniform(Self::FONT_SIZE as f32);

        let v_metrics = self.font.v_metrics(scale);

        let glyphs = self
            .font
            .layout(text, scale, point(x as f32, y as f32 + v_metrics.ascent));
        let frame = self.pixels.frame_mut();

        for glyph in glyphs {
            if let Some(bb) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, v| {
                    let x = bb.min.x as u32 + x;
                    let y = bb.min.y as u32 + y;
                    let i = (x as usize + y as usize * self.width) * 4;
                    if i + 3 >= frame.len() {
                        return;
                    }

                    let b = (v * 255.0) as u8;
                    let pixel = [b, b, b, 255];
                    frame[i..i + 4].copy_from_slice(&pixel);
                });
            }
        }
    }

    fn pixel_index(&self, x: usize, y: usize) -> usize {
        (x + y * self.width) * 4
    }
}
