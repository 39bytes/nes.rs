use anyhow::Result;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::emu::{cartridge::Cartridge, save_state::SaveState};

pub struct EmuState {
    paused: Arc<AtomicBool>,
    save_states: [Option<SaveState>; 10],
    pattern_table_palette: u8,
    cart_hash: u64,
}

impl EmuState {
    pub fn new(cartridge: &Cartridge) -> Self {
        const EMPTY: Option<SaveState> = None;
        let mut emu_state = Self {
            paused: Arc::new(AtomicBool::new(false)),
            save_states: [EMPTY; 10],
            pattern_table_palette: 0,

            cart_hash: cartridge.compute_hash(),
        };
        for i in 0..10 {
            if let Ok(state) = SaveState::load(i, emu_state.cart_hash) {
                emu_state.save_states[i] = Some(state);
                log::info!("Found existing save state {}", i);
            }
        }
        emu_state
    }

    pub fn toggle_pause(&mut self) {
        self.paused
            .store(!self.paused.load(Ordering::Relaxed), Ordering::Relaxed);
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn save_state(&self, number: usize) -> Option<&SaveState> {
        self.save_states.get(number).and_then(|o| o.as_ref())
    }

    pub fn write_save_state(&mut self, number: usize, save_state: SaveState) -> Result<()> {
        save_state.write(number, self.cart_hash)?;
        self.save_states[number] = Some(save_state);
        Ok(())
    }

    pub fn pattern_table_palette(&self) -> u8 {
        self.pattern_table_palette
    }

    pub fn next_palette(&mut self) {
        self.pattern_table_palette = (self.pattern_table_palette + 1) % 8;
    }
}
