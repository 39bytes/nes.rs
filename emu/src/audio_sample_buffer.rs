use crate::consts::CLOCK_SPEED;

const TIME_PER_CLOCK: f64 = 1.0 / CLOCK_SPEED as f64;
const INITIAL_BUFFER_CAPACITY: usize = 1024;

pub struct AudioSampleBuffer {
    acc: f64,
    time_between_samples: f64,
    buffer: Vec<f32>,
}

impl AudioSampleBuffer {
    pub fn new(sample_rate: u32) -> Self {
        AudioSampleBuffer {
            acc: 0.0,
            time_between_samples: 1.0 / (sample_rate as f64),
            buffer: Vec::with_capacity(INITIAL_BUFFER_CAPACITY),
        }
    }

    pub fn try_push_sample(&mut self, sample: f32) {
        self.acc += TIME_PER_CLOCK;
        while self.acc >= self.time_between_samples {
            self.buffer.push(sample);
            self.acc -= self.time_between_samples;
        }
    }

    #[inline]
    pub fn flush(&mut self, mut callback: impl FnMut(&[f32])) {
        callback(self.buffer.as_slice());
        self.buffer.clear();
    }
}
