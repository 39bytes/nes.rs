use super::emu::consts::CLOCK_SPEED;
use ringbuf::{storage::Heap, traits::*, wrap::caching::Caching, HeapRb, SharedRb};
use std::sync::Arc;

const TIME_PER_CLOCK: f64 = 1.0 / CLOCK_SPEED as f64;

pub type AudioBufferProducer = Caching<Arc<SharedRb<Heap<f32>>>, true, false>;
pub type AudioBufferConsumer = Caching<Arc<SharedRb<Heap<f32>>>, false, true>;

pub struct AudioOutput {
    acc: f64,
    time_between_samples: f64,
    producer: AudioBufferProducer,
    buffer: Vec<f32>,
    buffer_sample_index: usize,
}

impl AudioOutput {
    pub fn new(sample_rate: usize) -> (Self, AudioBufferConsumer) {
        let sample_rate = sample_rate as f64;

        let latency_frames = (128.0 / 1000.0) * sample_rate;
        let latency_samples = latency_frames as usize;

        let rb = HeapRb::<f32>::new(latency_samples);

        let (mut prod, cons) = rb.split();

        let buf = vec![0.0; latency_samples];

        // Fill with some silence to start
        prod.push_slice(&buf);

        (
            AudioOutput {
                acc: 0.0,
                time_between_samples: 1.0 / sample_rate,
                producer: prod,
                buffer: vec![0.0; 256],
                buffer_sample_index: 0,
            },
            cons,
        )
    }

    pub fn try_push_sample(&mut self, sample: f32) {
        self.acc += TIME_PER_CLOCK;
        while self.acc >= self.time_between_samples {
            self.buffer[self.buffer_sample_index] = sample;
            self.buffer_sample_index += 1;

            if self.buffer_sample_index == self.buffer.len() {
                self.producer.push_slice(&self.buffer);
                self.buffer_sample_index = 0;
            }

            self.acc -= self.time_between_samples;
        }
    }
}
