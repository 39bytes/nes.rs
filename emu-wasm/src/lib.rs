#![allow(dead_code)]

use emu::cartridge::Cartridge;
use emu::input::{ControllerButtons, ControllerInput};
use emu::palette::Palette;
use emu::save::SaveState;
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
    save_states: [Option<SaveState>; 5],
}

#[wasm_bindgen]
impl Nes {
    pub fn new(sample_rate: u32) -> Self {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
        const NONE: Option<SaveState> = None;

        Self {
            nes: emu::nes::Nes::new(Palette::default(), Some(sample_rate)),
            save_states: [NONE; 5],
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

    pub fn clear_save_states(&mut self) {
        const NONE: Option<SaveState> = None;
        self.save_states = [NONE; 5];
    }

    pub fn write_state(&mut self, slot: usize) {
        self.save_states[slot] = Some(self.nes.state());
    }

    pub fn load_state(&mut self, slot: usize) {
        if let Some(ref state) = self.save_states[slot] {
            self.nes.load_state(state);
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.nes.set_volume(volume);
    }
}
