use std::{collections::VecDeque, time::Instant};

const MAX_SAMPLES: usize = 120;

pub struct FpsCounter {
    last_frame: Instant,
    ticks: VecDeque<f32>,
    tick_sum: f32,
}

impl FpsCounter {
    pub fn new() -> Self {
        FpsCounter {
            last_frame: Instant::now(),
            ticks: VecDeque::new(),
            tick_sum: 0.0,
        }
    }

    pub fn tick(&mut self) {
        let frame_time = self.last_frame.elapsed().as_secs_f32();

        self.ticks.push_back(frame_time);
        self.tick_sum += frame_time;

        if self.ticks.len() > MAX_SAMPLES {
            self.tick_sum -= self.ticks.pop_front().unwrap_or(0.0);
        }

        self.last_frame = std::time::Instant::now();
    }

    pub fn get_fps(&self) -> f32 {
        1.0 / (self.tick_sum / MAX_SAMPLES as f32)
    }
}
