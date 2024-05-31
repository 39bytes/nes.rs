use crate::emu::bits::IntoBit;

use super::components::{Divider, Envelope, LengthCounter, Sweep};

#[derive(Default, Debug, PartialEq, Eq)]
pub enum PulseChannelNumber {
    #[default]
    One,
    Two,
}

#[derive(Default, Debug)]
pub struct PulseChannel {
    enabled: bool,
    sequence: u8,
    sequence_position: u8,

    pub timer: Divider<u16>,
    pub envelope: Envelope,
    pub length_counter: LengthCounter,
    pub sweep: Sweep,
}

impl PulseChannel {
    pub fn new(channel: PulseChannelNumber) -> Self {
        Self {
            sweep: Sweep::new(channel),
            ..Default::default()
        }
    }

    pub fn clock(&mut self) {
        if !self.enabled {
            return;
        }

        if self.timer.clock() {
            if self.sequence_position == 0 {
                self.sequence_position = 7;
            } else {
                self.sequence_position -= 1;
            }
        }
    }

    pub fn sample(&self) -> u8 {
        ((self.sequence << self.sequence_position) & 0x80).into_bit()
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

    pub fn restart(&mut self) {
        self.sequence_position = 0;
        self.envelope.restart();
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
