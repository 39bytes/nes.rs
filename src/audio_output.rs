use super::emu::consts::CLOCK_SPEED;
use anyhow::{anyhow, Result};
use ringbuf::{storage::Heap, traits::*, wrap::caching::Caching, HeapRb, SharedRb};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub type AudioBufferProducer = Caching<Arc<SharedRb<Heap<f32>>>, true, false>;
pub type AudioBufferConsumer = Caching<Arc<SharedRb<Heap<f32>>>, false, true>;

pub struct AudioOutput {
    last_sample_time: Instant,
    acc: f64,
    time_between_samples: f64,
    time_per_clock: f64,
    producer: AudioBufferProducer,
    buffer: Vec<f32>,
    buffer_sample_index: usize,
}

impl AudioOutput {
    pub fn new(sample_rate: usize, channels: usize) -> (Self, AudioBufferConsumer) {
        // Want to buffer 6 frames of audio,
        // 6 / 60 = 1 / 10, so we buffer 1/10th of a second
        // let buf_size = sample_rate / 10;

        let sample_emit_rate = (sample_rate) as f64;

        let latency_frames = (100.0 / 1000.0) * sample_emit_rate;
        let latency_samples = (latency_frames as usize) * 2;

        let rb = HeapRb::<f32>::new(latency_samples);

        let (mut prod, cons) = rb.split();

        prod.push_slice(&vec![0.0; latency_samples / 2]);

        (
            AudioOutput {
                acc: 0.0,
                last_sample_time: Instant::now(),
                time_between_samples: 1.0 / sample_emit_rate,
                time_per_clock: 1.0 / CLOCK_SPEED as f64,
                producer: prod,
                buffer: vec![0.0; latency_samples / 2],
                buffer_sample_index: 0,
            },
            cons,
        )
    }

    pub fn try_push_sample(&mut self, sample: f32) {
        self.acc += self.last_sample_time.elapsed().as_secs_f64();
        self.last_sample_time = Instant::now();
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
