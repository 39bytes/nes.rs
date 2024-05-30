use super::emu::consts::CLOCK_SPEED;
use ringbuf::{storage::Heap, traits::*, wrap::caching::Caching, HeapRb, SharedRb};
use std::sync::Arc;
use std::time::Instant;

const TIME_PER_CLOCK: f64 = 1.0 / CLOCK_SPEED as f64;

pub type AudioBufferProducer = Caching<Arc<SharedRb<Heap<f32>>>, true, false>;
pub type AudioBufferConsumer = Caching<Arc<SharedRb<Heap<f32>>>, false, true>;

pub struct AudioOutput {
    last_sample_time: Instant,
    acc: f64,
    time_between_samples: f64,
    producer: AudioBufferProducer,
    buffer: Vec<f32>,
    buffer_sample_index: usize,
}

impl AudioOutput {
    pub fn new(sample_rate: usize, channels: usize) -> (Self, AudioBufferConsumer) {
        let sample_rate = sample_rate as f64;

        let latency_frames = (50.0 / 1000.0) * sample_rate;
        let latency_samples = latency_frames as usize;

        let rb = HeapRb::<f32>::new(latency_samples);

        let (mut prod, cons) = rb.split();

        let buf = vec![0.0; latency_samples / 2];

        // Fill with some silence to start
        prod.push_slice(&buf);

        (
            AudioOutput {
                acc: 0.0,
                last_sample_time: Instant::now(),
                time_between_samples: 1.0 / sample_rate,
                producer: prod,
                buffer: vec![0.0; 128],
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
                let pushed = self.producer.push_slice(&self.buffer);
                if pushed != self.buffer.len() {
                    log::warn!(
                        "Audio buffer is full, dropped {} samples",
                        self.buffer.len() - pushed
                    );
                }
                self.buffer_sample_index = 0;
            }

            // self.producer
            //     .try_push(sample)
            //     .map_err(|_| anyhow!("Audio buffer is full, dropping samples"))?;
            self.acc -= self.time_between_samples;
        }

        // self.acc += self.last_sample_time.elapsed().as_secs_f64();
        // self.last_sample_time = Instant::now();
        // while self.acc >= self.time_between_samples {
        //     self.acc -= self.time_between_samples;
        // }
    }
}
