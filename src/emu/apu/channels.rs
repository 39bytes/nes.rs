use crate::emu::bits::{rotate_byte_right, IntoBit};

#[derive(Default, Debug)]
pub(crate) struct PulseChannel {
    enabled: bool,
    sequence: u8,

    timer_reset: u16,
    timer: u16,

    length_counter: u8,
    length_counter_halt: bool,

    constant_volume: bool,

    envelope_divider_period: u8,
}

impl PulseChannel {
    pub fn new() -> Self {
        PulseChannel::default()
    }

    pub fn clock(&mut self) {
        if !self.enabled {
            return;
        }

        if self.timer == 0 {
            self.timer = self.timer_reset;
            self.sequence = rotate_byte_right(self.sequence);
        } else {
            self.timer -= 1;
        }
    }

    pub fn sample(&self) -> u8 {
        (self.sequence & 0x80).into_bit()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_duty_cycle(&mut self, duty_cycle: u8) {
        let sequence = match duty_cycle {
            0 => 0b0000_0001,
            1 => 0b0000_0011,
            2 => 0b0000_1111,
            3 => 0b1111_1100,
            _ => panic!("Invalid duty cycle {}", duty_cycle),
        };
        self.sequence = sequence;
    }

    pub fn set_length_counter(&mut self, length_counter: u8) {
        self.length_counter = length_counter;
    }

    pub fn set_length_counter_halt(&mut self, length_counter_halt: bool) {
        self.length_counter_halt = length_counter_halt;
    }

    pub fn set_constant_volume(&mut self, constant_volume: bool) {
        self.constant_volume = constant_volume;
    }

    pub fn set_divider_period(&mut self, divider_period: u8) {
        self.envelope_divider_period = divider_period;
    }

    pub fn set_timer_high(&mut self, timer_high: u8) {
        self.timer_reset = (self.timer_reset & 0x00FF) | ((timer_high as u16) << 8);
        self.timer = self.timer_reset;
    }

    pub fn set_timer_low(&mut self, timer_low: u8) {
        self.timer_reset = (self.timer_reset & 0xFF00) | (timer_low as u16);
    }
}

#[derive(Default)]
pub(crate) struct TriangleChannel {
    enabled: bool,
}

impl TriangleChannel {
    pub fn new() -> Self {
        TriangleChannel::default()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[derive(Default)]
pub(crate) struct NoiseChannel {
    enabled: bool,
}

impl NoiseChannel {
    pub fn new() -> Self {
        NoiseChannel::default()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[derive(Default)]
pub(crate) struct DCPMChannel {
    enabled: bool,
}

impl DCPMChannel {
    pub fn new() -> Self {
        DCPMChannel::default()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}
