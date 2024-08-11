use crate::extension_traits::*;
use anyhow::{bail, Result};
use sdl2::audio::{AudioQueue, AudioSpecDesired};

use super::emu::consts::CLOCK_SPEED;

const TIME_PER_CLOCK: f64 = 1.0 / CLOCK_SPEED as f64;
const SAMPLE_RATE: i32 = 44100;
const DEFAULT_VOLUME: f32 = 0.1;

pub struct AudioOutput {
    volume: f32,

    acc: f64,
    time_between_samples: f64,
    buffer: [f32; 64],
    buffer_sample_index: usize,
    queue: AudioQueue<f32>,
}

impl AudioOutput {
    pub fn new(sdl_context: &sdl2::Sdl) -> Result<Self> {
        let audio = sdl_context.audio().into_anyhow()?;
        if audio.num_audio_playback_devices().is_none() {
            bail!("No audio playback devices found");
        }
        let default_device = audio.audio_playback_device_name(0).into_anyhow()?;

        let queue = audio
            .open_queue(
                Some(default_device).as_deref(),
                &AudioSpecDesired {
                    freq: Some(SAMPLE_RATE),
                    samples: Some(512),
                    channels: Some(1),
                },
            )
            .into_anyhow()?;
        queue.resume();
        log::info!("Opened queue with spec: {:?}", queue.spec());

        Ok(AudioOutput {
            volume: DEFAULT_VOLUME,

            acc: 0.0,
            time_between_samples: 1.0 / (SAMPLE_RATE as f64),
            queue,
            buffer: [0.0; 64],
            buffer_sample_index: 0,
        })
    }

    pub fn try_push_sample(&mut self, sample: f32) {
        self.acc += TIME_PER_CLOCK;
        while self.acc >= self.time_between_samples {
            let adjusted = sample * self.volume;
            self.buffer[self.buffer_sample_index] = adjusted;
            self.buffer_sample_index += 1;

            if self.buffer_sample_index == self.buffer.len() {
                if let Err(e) = self.queue.queue_audio(&self.buffer) {
                    log::warn!("Audio Queue: {}", e);
                };
                self.buffer_sample_index = 0;
            }

            self.acc -= self.time_between_samples;
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
    }
}
