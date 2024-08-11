use anyhow::{anyhow, Result};
use rusttype::{point, Font, Scale};
use sdl2::{
    pixels::PixelFormatEnum,
    rect::Rect,
    render::{Canvas, TextureCreator},
    video::{Window, WindowContext},
};

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

    pub fn as_slice(&self) -> [u8; 3] {
        [self.0, self.1, self.2]
    }
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

pub struct Renderer {
    canvas: Canvas<Window>,
    texture_creator: TextureCreator<WindowContext>,

    font: Font<'static>,
}

impl Renderer {
    const FONT_SIZE: u16 = 20;

    pub fn new(sdl_context: &sdl2::Sdl, width: u32, height: u32) -> Result<Self> {
        let video_subsystem = sdl_context.video().map_err(anyhow::Error::msg)?;

        let font_data = include_bytes!("../assets/fonts/nes-arcade-font-2-1-monospaced.ttf");
        let font = Font::try_from_bytes(font_data as &[u8]).ok_or(anyhow!("Error loading font"))?;

        let window = video_subsystem
            .window("nes.rs", width, height)
            .resizable()
            .build()?;
        let mut canvas = window.into_canvas().build()?;
        canvas.clear();
        canvas.present();
        let texture_creator = canvas.texture_creator();

        Ok(Renderer {
            canvas,
            texture_creator,
            font,
        })
    }

    pub fn clear(&mut self) {
        self.canvas.clear();
    }

    pub fn render(&mut self) {
        self.canvas.present();
    }

    pub fn draw_scaled<D: Draw>(&mut self, obj: &D, x: usize, y: usize, scale: usize) {
        let mut texture = self
            .texture_creator
            .create_texture_static(
                PixelFormatEnum::RGB24,
                obj.width() as u32,
                obj.height() as u32,
            )
            .unwrap();

        let mut pixel_data = Vec::with_capacity(obj.width() * obj.height() * 3);
        for i in 0..obj.height() {
            for j in 0..obj.width() {
                let pixel = obj.get_pixel(j, i);
                pixel_data.push(pixel.0);
                pixel_data.push(pixel.1);
                pixel_data.push(pixel.2);
            }
        }
        texture
            .update(None, pixel_data.as_slice(), obj.width() * 3)
            .unwrap();

        self.canvas
            .copy(
                &texture,
                None,
                Some(rect!(x, y, obj.width() * scale, obj.height() * scale)),
            )
            .unwrap();
    }

    pub fn draw<D: Draw>(&mut self, obj: &D, x: usize, y: usize) {
        let mut texture = self
            .texture_creator
            .create_texture_static(
                PixelFormatEnum::RGB24,
                obj.width() as u32,
                obj.height() as u32,
            )
            .unwrap();

        let mut pixel_data = Vec::with_capacity(obj.width() * obj.height() * 3);
        for i in 0..obj.height() {
            for j in 0..obj.width() {
                let pixel = obj.get_pixel(j, i);
                pixel_data.extend_from_slice(&pixel.as_slice());
            }
        }
        texture
            .update(None, pixel_data.as_slice(), obj.width() * 3)
            .unwrap();

        self.canvas
            .copy(&texture, None, Some(rect!(x, y, obj.width(), obj.height())))
            .unwrap();
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

                let mut texture = self
                    .texture_creator
                    .create_texture_static(PixelFormatEnum::RGB24, w as u32, h as u32)
                    .unwrap();
                let mut pixel_data = Vec::with_capacity(w as usize * h as usize * 3);

                glyph.draw(|_, _, v| {
                    let b = (v * 255.0) as u8;
                    pixel_data.extend_from_slice(&[b, b, b]);
                });
                texture
                    .update(None, pixel_data.as_slice(), w as usize * 3)
                    .unwrap();

                self.canvas
                    .copy(&texture, None, Some(rect!(bb.min.x, bb.min.y, w, h)))
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

                let mut texture = self
                    .texture_creator
                    .create_texture_static(PixelFormatEnum::RGB24, w as u32, h as u32)
                    .unwrap();
                let mut pixel_data = Vec::with_capacity(w as usize * h as usize * 3);

                glyph.draw(|_, _, v| {
                    let color = color_func(idx);
                    let pixel = [
                        (v * color.0 as f32) as u8,
                        (v * color.1 as f32) as u8,
                        (v * color.2 as f32) as u8,
                    ];
                    pixel_data.extend_from_slice(&pixel);
                });
                texture
                    .update(None, pixel_data.as_slice(), w as usize * 3)
                    .unwrap();

                self.canvas
                    .copy(&texture, None, Some(rect!(bb.min.x, bb.min.y, w, h)))
                    .unwrap();
            }
        }
    }
}
