use anyhow::Result;
use pixels::{Pixels, SurfaceTexture};
use rusttype::{point, Font, Scale};
use winit::window::Window;

pub struct Renderer {
    font: Font<'static>,
    pixels: Pixels,
    width: u32,
    height: u32,
}

impl Renderer {
    const FONT_SIZE: usize = 20;

    pub fn new(font: Font<'static>, window: &Window, width: u32, height: u32) -> Result<Self> {
        let pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, window);
            Pixels::new(width, height, surface_texture)?
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

    pub fn draw_text(&mut self, text: &str, x: u32, y: u32) {
        let scale = Scale::uniform(Self::FONT_SIZE as f32);

        let v_metrics = self.font.v_metrics(scale);

        let glyphs = self
            .font
            .layout(text, scale, point(x as f32, y as f32 + v_metrics.ascent));

        for glyph in glyphs {
            if let Some(bb) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, v| {
                    let frame = self.pixels.frame_mut();

                    let x = bb.min.x as u32 + x;
                    let y = bb.min.y as u32 + y;
                    let i = ((x + y * self.width) * 4) as usize;
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
}
