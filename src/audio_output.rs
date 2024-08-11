use anyhow::{bail, Result};
use sdl2::audio::{AudioQueue, AudioSpecDesired};

use super::emu::consts::CLOCK_SPEED;

const TIME_PER_CLOCK: f64 = 1.0 / CLOCK_SPEED as f64;
const SAMPLE_RATE: i32 = 44100;

pub struct AudioOutput {
    acc: f64,
    time_between_samples: f64,
    queue: AudioQueue<f32>,
    buffer: [f32; 512],
    buffer_sample_index: usize,
}

impl AudioOutput {
    pub fn new(sdl_context: &sdl2::Sdl) -> Result<Self> {
        let audio = sdl_context.audio().map_err(anyhow::Error::msg)?;
        if audio.num_audio_playback_devices().is_none() {
            bail!("No audio playback devices found");
        }
        let default_device = audio
            .audio_playback_device_name(0)
            .map_err(anyhow::Error::msg)?;

        let queue = audio
            .open_queue(
                Some(default_device).as_deref(),
                &AudioSpecDesired {
                    freq: Some(SAMPLE_RATE),
                    samples: Some(128),
                    channels: Some(2),
                },
            )
            .map_err(anyhow::Error::msg)?;
        queue.resume();
        log::info!("Opened queue with spec: {:?}", queue.spec());

        Ok(AudioOutput {
            acc: 0.0,
            time_between_samples: 1.0 / (SAMPLE_RATE as f64),
            queue,
            buffer: [0.0; 512],
            buffer_sample_index: 0,
        })
    }

    pub fn try_push_sample(&mut self, sample: f32) {
        self.acc += TIME_PER_CLOCK;
        while self.acc >= self.time_between_samples {
            self.buffer[self.buffer_sample_index] = sample;
            self.buffer[self.buffer_sample_index + 1] = sample;
            self.buffer_sample_index += 2;

            if self.buffer_sample_index == self.buffer.len() {
                if let Err(e) = self.queue.queue_audio(&self.buffer) {
                    log::warn!("Audio Queue: {}", e);
                };
                self.buffer_sample_index = 0;
            }

            self.acc -= self.time_between_samples;
        }
    }
}
