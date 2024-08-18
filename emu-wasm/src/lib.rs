use emu::cartridge::Cartridge;
use emu::input::{ControllerButtons, ControllerInput};
use emu::palette::{Color, Palette};
use wasm_bindgen::prelude::*;
use web_sys::js_sys::{Float32Array, Uint8Array};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// A wrapper of the actual NES struct to generate WASM bindings for
#[wasm_bindgen]
struct Nes {
    nes: emu::nes::Nes,
}

#[wasm_bindgen]
impl Nes {
    pub fn new(sample_rate: u32) -> Self {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
        Self {
            nes: emu::nes::Nes::new(Palette::default(), Some(sample_rate)),
        }
    }

    pub fn advance_frame(&mut self) {
        self.nes.advance_frame();
    }

    pub fn screen(&self) -> Uint8Array {
        let screen = self.nes.screen();
        unsafe { Uint8Array::view(screen) }
    }

    pub fn audio_samples(&mut self) -> Float32Array {
        let samples = self.nes.audio_buffer().samples();
        unsafe { Float32Array::view(samples) }
    }

    pub fn clear_audio_samples(&mut self) {
        self.nes.audio_buffer().clear();
    }

    pub fn load_rom(&mut self, bytes: Vec<u8>) -> Result<(), JsError> {
        self.nes.load_cartridge(
            Cartridge::from_bytes(&bytes).map_err(|e| JsError::new(&e.to_string()))?,
        );
        Ok(())
    }

    pub fn reset(&mut self) {
        self.nes.reset();
    }

    pub fn trigger_inputs(&mut self, input_byte: u8) {
        self.nes.trigger_inputs(ControllerInput::One(
            ControllerButtons::from_bits(input_byte).unwrap(),
        ));
    }
}
