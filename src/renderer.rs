use anyhow::{anyhow, Result};
use pixels::{Pixels, SurfaceTexture};
use rusttype::{point, Font, Scale};
use winit::window::Window;

#[derive(Debug, Clone, Copy, Default)]
pub struct Color(pub u8, pub u8, pub u8);

impl Color {
    pub const WHITE: Self = Color(255, 255, 255);
    pub const GRAY: Self = Color(128, 128, 128);
    pub const BLACK: Self = Color(0, 0, 0);
}

pub struct Pixel {
    pub x: usize,
    pub y: usize,
    pub color: Color,
}

pub trait Draw {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn get_pixel(&self, x: usize, y: usize) -> &Color;
}

#[derive(Clone)]
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

    pub fn monocolor(color: Color, width: usize, height: usize) -> Self {
        let mut pixels = Vec::with_capacity(width * height);

        for _ in 0..(width * height) {
            pixels.push(color);
        }

        Sprite {
            pixels,
            width,
            height,
        }
    }

    pub fn scale(&self, scale: usize) -> Scaled {
        Scaled {
            original: self,
            scale,
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) -> Result<()> {
        if !(0..self.width).contains(&x) || !(0..self.height).contains(&y) {
            return Err(anyhow!(
                "Coordinates ({}, {}) are outside of the bounds of the sprite",
                x,
                y
            ));
        }

        self.pixels[y * self.width + x] = color;
        Ok(())
    }
}

impl Draw for Sprite {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn get_pixel(&self, x: usize, y: usize) -> &Color {
        &self.pixels[self.width * y + x]
    }
}

pub struct Scaled<'a> {
    original: &'a Sprite,
    scale: usize,
}

impl<'a> Draw for Scaled<'a> {
    fn width(&self) -> usize {
        self.original.width() * self.scale
    }

    fn height(&self) -> usize {
        self.original.height() * self.scale
    }

    fn get_pixel(&self, x: usize, y: usize) -> &Color {
        self.original.get_pixel(x / self.scale, y / self.scale)
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

    pub fn draw<D: Draw>(&mut self, obj: &D, x: usize, y: usize) {
        for i in 0..obj.height() {
            for j in 0..obj.width() {
                let px = self.pixel_index(x + j, y + i);
                if let Some(px) = px {
                    let frame = self.pixels.frame_mut();
                    let pixel = obj.get_pixel(j, i);

                    frame[px] = pixel.0;
                    frame[px + 1] = pixel.1;
                    frame[px + 2] = pixel.2;
                    frame[px + 3] = 255;
                }
            }
        }
    }

    /// Draws white text at a given screen position
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

    /// Draws text with color for each character that can be computed from a closure.
    pub fn draw_text_with_computed_color<F>(
        &mut self,
        text: &str,
        x: usize,
        y: usize,
        color_func: F,
    ) where
        F: Fn(usize) -> Color,
    {
        let scale = Scale::uniform(Self::FONT_SIZE as f32);

        let v_metrics = self.font.v_metrics(scale);

        let glyphs = self
            .font
            .layout(text, scale, point(x as f32, y as f32 + v_metrics.ascent));
        let frame = self.pixels.frame_mut();

        for (idx, glyph) in glyphs.enumerate() {
            if let Some(bb) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, v| {
                    let x = bb.min.x as u32 + x;
                    let y = bb.min.y as u32 + y;
                    let i = (x as usize + y as usize * self.width) * 4;
                    if i + 3 >= frame.len() {
                        return;
                    }
                    let color = color_func(idx);

                    let pixel = [
                        (v * color.0 as f32) as u8,
                        (v * color.1 as f32) as u8,
                        (v * color.2 as f32) as u8,
                        255,
                    ];
                    frame[i..i + 4].copy_from_slice(&pixel);
                });
            }
        }
    }

    fn pixel_index(&self, x: usize, y: usize) -> Option<usize> {
        if !(0..self.width).contains(&x) || !(0..self.height).contains(&y) {
            None
        } else {
            Some((x + y * self.width) * 4)
        }
    }
}
