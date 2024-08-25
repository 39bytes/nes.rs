use num_traits::clamp;

use crate::consts::CLOCK_SPEED;

const TIME_PER_CLOCK: f64 = 1.0 / CLOCK_SPEED as f64;
const INITIAL_BUFFER_CAPACITY: usize = 1024;

pub struct AudioSampleBuffer {
    acc: f64,
    time_between_samples: f64,
    buffer: Vec<f32>,
    volume: f32,
}

impl AudioSampleBuffer {
    pub fn new(sample_rate: u32) -> Self {
        AudioSampleBuffer {
            acc: 0.0,
            time_between_samples: 1.0 / (sample_rate as f64),
            buffer: Vec::with_capacity(INITIAL_BUFFER_CAPACITY),
            volume: 1.0,
        }
    }

    pub fn try_push_sample(&mut self, sample: f32) {
        self.acc += TIME_PER_CLOCK;
        while self.acc >= self.time_between_samples {
            self.buffer.push(sample * self.volume);
            self.acc -= self.time_between_samples;
        }
    }

    #[inline]
    pub fn samples(&mut self) -> &[f32] {
        self.buffer.as_slice()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn flush(&mut self, mut callback: impl FnMut(&[f32])) {
        callback(self.buffer.as_slice());
        self.clear();
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = clamp(volume, 0.0, 1.0);
    }
}
