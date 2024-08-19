use crate::extension_traits::*;
use anyhow::{bail, Result};
use sdl2::audio::{AudioQueue, AudioSpec, AudioSpecDesired};

const SAMPLE_RATE: u32 = 44100;
const DEFAULT_VOLUME: f32 = 0.5;
const BUFFER_SIZE: usize = 1024;

pub struct AudioOutput {
    volume: f32,
    spec: AudioSpec,
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
                    freq: Some(SAMPLE_RATE as i32),
                    samples: Some(BUFFER_SIZE as u16),
                    channels: Some(1),
                },
            )
            .into_anyhow()?;

        let spec = queue.spec();
        log::info!("Opened queue with spec: {:?}", spec);

        Ok(AudioOutput {
            volume: DEFAULT_VOLUME,
            spec: *spec,
            queue,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.spec.freq as u32
    }

    pub fn play(&mut self) {
        self.queue.resume();
    }

    pub fn pause(&mut self) {
        self.queue.pause();
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }

    pub fn queue(&mut self, samples: &[f32]) {
        if let Err(e) = self.queue.queue_audio(samples) {
            log::error!("SDL Audio Output: {}", e);
        }
    }

    #[allow(dead_code)]
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
    }
}
