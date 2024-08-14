use anyhow::{anyhow, Result};
use rusttype::{point, Font, Scale};
use sdl2::{
    pixels::PixelFormatEnum,
    rect::Rect,
    render::{BlendMode, Canvas, Texture},
    video::Window,
};

use crate::extension_traits::*;

macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

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

    pub fn scale(&self, scale: usize) -> Scaled {
        Scaled { orig: self, scale }
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
    orig: &'a Sprite,
    scale: usize,
}

impl<'a> Draw for Scaled<'a> {
    fn width(&self) -> usize {
        self.orig.width() * self.scale
    }

    fn height(&self) -> usize {
        self.orig.height() * self.scale
    }

    fn get_pixel(&self, x: usize, y: usize) -> &Color {
        self.orig.get_pixel(x / self.scale, y / self.scale)
    }
}

pub enum Layer {
    Screen,
    UI,
}

pub struct Renderer {
    width: u32,
    height: u32,

    canvas: Canvas<Window>,
    screen_texture: Texture,
    ui_texture: Texture,

    font: Font<'static>,
}

impl Renderer {
    const FONT_SIZE: u16 = 20;

    pub fn new(sdl_context: &sdl2::Sdl, width: u32, height: u32, scale: u32) -> Result<Self> {
        let video_subsystem = sdl_context.video().into_anyhow()?;

        let font_data = include_bytes!("../assets/fonts/nes-arcade-font-2-1-monospaced.ttf");
        let font = Font::try_from_bytes(font_data as &[u8]).ok_or(anyhow!("Error loading font"))?;

        let window = video_subsystem
            .window("nes.rs", width * scale, height * scale)
            .resizable()
            .build()?;
        let mut canvas = window.into_canvas().build()?;
        canvas.set_blend_mode(BlendMode::Blend);
        canvas.clear();
        canvas.present();

        let mut screen_texture =
            canvas.create_texture_streaming(PixelFormatEnum::RGBA32, width, height)?;
        screen_texture.set_blend_mode(BlendMode::Blend);
        let mut ui_texture =
            canvas.create_texture_streaming(PixelFormatEnum::RGBA32, width, height)?;
        ui_texture.set_blend_mode(BlendMode::Blend);

        Ok(Renderer {
            width,
            height,
            canvas,
            screen_texture,
            ui_texture,
            font,
        })
    }

    pub fn clear(&mut self) {
        self.screen_texture
            .update(
                None,
                &vec![0; (self.width * self.height * 4) as usize],
                (self.width as usize) * 4,
            )
            .unwrap();
        self.ui_texture
            .update(
                None,
                &vec![0; (self.width * self.height * 4) as usize],
                (self.width as usize) * 4,
            )
            .unwrap();
        self.canvas.clear();
    }

    pub fn render(&mut self) {
        let (window_w, window_h) = self.canvas.window().size();
        let scale_x = if window_w > self.width {
            window_w / self.width
        } else {
            1
        };
        let scale_y = if window_h > self.height {
            window_h / self.height
        } else {
            1
        };
        let scale = scale_x.min(scale_y);

        let width = self.width * scale;
        let height = self.height * scale;
        let x = if window_w > width {
            (window_w - width) / 2
        } else {
            0
        };
        let y = if window_h > height {
            (window_h - height) / 2
        } else {
            0
        };

        if let Err(e) = self
            .canvas
            .copy(&self.screen_texture, None, rect!(x, y, width, height))
        {
            log::warn!("Rendering error: {}", e);
        }
        if let Err(e) = self
            .canvas
            .copy(&self.ui_texture, None, rect!(x, y, width, height))
        {
            log::warn!("Rendering error: {}", e);
        }
        self.canvas.present();
    }

    pub fn draw<D: Draw>(&mut self, layer: Layer, obj: &D, x: usize, y: usize) {
        let res = self.select_texture(layer).with_lock(
            rect!(x, y, obj.width(), obj.height()),
            |buf: &mut [u8], pitch: usize| {
                for px_y in 0..obj.height() {
                    for px_x in 0..obj.width() {
                        let pixel = obj.get_pixel(px_x, px_y);
                        let index = px_y * pitch + px_x * 4;
                        buf[index] = pixel.0;
                        buf[index + 1] = pixel.1;
                        buf[index + 2] = pixel.2;
                        buf[index + 3] = 255;
                    }
                }
            },
        );
        if let Err(e) = res {
            log::warn!("Error trying to draw object: {}", e);
        }
    }

    /// Draws white text at a given screen position
    pub fn draw_text(&mut self, text: &str, x: usize, y: usize) {
        let scale = Scale::uniform(Self::FONT_SIZE as f32);

        let v_metrics = self.font.v_metrics(scale);

        let glyphs = self
            .font
            .layout(text, scale, point(x as f32, y as f32 + v_metrics.ascent));

        for glyph in glyphs {
            if let Some(bb) = glyph.pixel_bounding_box() {
                let w = bb.width();
                let h = bb.height();

                self.ui_texture
                    .with_lock(
                        rect!(bb.min.x, bb.min.y, w, h),
                        |buf: &mut [u8], pitch: usize| {
                            glyph.draw(|x, y, v| {
                                let index = y as usize * pitch + x as usize * 4;
                                if index + 2 >= buf.len() {
                                    return;
                                }

                                let b = (v * 255.0) as u8;
                                buf[index] = b;
                                buf[index + 1] = b;
                                buf[index + 2] = b;
                                buf[index + 3] = b;
                            });
                        },
                    )
                    .unwrap();
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

        for (idx, glyph) in glyphs.enumerate() {
            if let Some(bb) = glyph.pixel_bounding_box() {
                let w = bb.width();
                let h = bb.height();

                self.ui_texture
                    .with_lock(
                        rect!(bb.min.x, bb.min.y, w, h),
                        |buf: &mut [u8], pitch: usize| {
                            glyph.draw(|x, y, v| {
                                let index = y as usize * pitch + x as usize * 4;
                                if index + 2 >= buf.len() {
                                    return;
                                }

                                let color = color_func(idx);
                                buf[index] = (v * color.0 as f32) as u8;
                                buf[index + 1] = (v * color.1 as f32) as u8;
                                buf[index + 2] = (v * color.2 as f32) as u8;
                                buf[index + 3] = (v * 255.0) as u8;
                            });
                        },
                    )
                    .unwrap();
            }
        }
    }

    pub fn select_texture(&mut self, layer: Layer) -> &mut Texture {
        match layer {
            Layer::UI => &mut self.ui_texture,
            Layer::Screen => &mut self.screen_texture,
        }
    }
}
