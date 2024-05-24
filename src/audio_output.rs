use anyhow::{anyhow, Result};
use ringbuf::{storage::Heap, traits::*, wrap::caching::Caching, HeapRb, SharedRb};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub type AudioBufferProducer = Caching<Arc<SharedRb<Heap<f32>>>, true, false>;
pub type AudioBufferConsumer = Caching<Arc<SharedRb<Heap<f32>>>, false, true>;

pub struct AudioOutput {
    last_sample_time: Instant,
    time_between_samples: Duration,
    producer: AudioBufferProducer,
}

impl AudioOutput {
    pub fn new(sample_rate: usize, channels: usize) -> (Self, AudioBufferConsumer) {
        // Want to buffer 6 frames of audio,
        // 6 / 60 = 1 / 10, so we buffer 1/10th of a second
        // let buf_size = sample_rate / 10;

        let sample_emit_rate = (sample_rate * channels) as f64;

        let latency_frames = (50.0 / 1000.0) * sample_emit_rate;
        let latency_samples = latency_frames as usize;

        let rb = HeapRb::<f32>::new(latency_samples);

        let (mut prod, cons) = rb.split();

        for _ in 0..latency_samples {
            prod.try_push(0.0).unwrap();
        }

        (
            AudioOutput {
                last_sample_time: Instant::now(),
                time_between_samples: Duration::from_secs_f64(1.0 / sample_emit_rate),
                producer: prod,
            },
            cons,
        )
    }

    pub fn try_push_sample(&mut self, sample: f32) -> Result<()> {
        let elapsed = self.last_sample_time.elapsed();
        if elapsed < self.time_between_samples {
            return Ok(());
        }
        self.last_sample_time = Instant::now();
        self.producer
            .try_push(sample)
            .map_err(|_| anyhow!("Audio buffer is full, dropping samples"))?;
        Ok(())
    }
}
