use super::components::{Divider, Envelope, LengthCounter, LinearCounter, Sweep};

#[derive(Default, Debug, PartialEq, Eq)]
pub enum PulseChannelNumber {
    #[default]
    One,
    Two,
}

#[derive(Default, Debug)]
pub struct PulseChannel {
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
        if self.timer.clock() {
            if self.sequence_position == 0 {
                self.sequence_position = 7;
            } else {
                self.sequence_position -= 1;
            }
        }
    }

    pub fn sample(&self) -> u8 {
        let sample = (self.sequence << self.sequence_position) & 0x80;

        if sample == 0 || self.sweep.muted(self.timer.reload) || self.length_counter.silenced() {
            return 0;
        }

        self.envelope.get_volume()
    }

    pub fn write_reg1(&mut self, data: u8) {
        self.set_duty_cycle((data & 0b1100_0000) >> 6);
        let l = (data & 0b0010_0000) != 0;
        self.envelope.set_loop(l);
        self.length_counter.set_halted(l);

        self.envelope.set_constant_volume((data & 0b0001_0000) != 0);
        self.envelope.set_param(data & 0b0000_1111);
    }

    pub fn write_reg4(&mut self, data: u8) {
        let timer_high = (data & 0x07) as u16;
        let length_counter_load = (data & 0xF8) >> 3;

        self.timer.reload = (self.timer.reload & 0x00FF) | (timer_high << 8);
        self.timer.force_reload();
        self.length_counter.set_counter(length_counter_load);

        self.sequence_position = 0;
        self.envelope.restart();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.length_counter.set_enabled(enabled);
    }

    pub fn set_duty_cycle(&mut self, duty_cycle: u8) {
        assert!(duty_cycle < 4);
        self.sequence = match duty_cycle {
            0 => 0b0000_0001,
            1 => 0b0000_0011,
            2 => 0b0000_1111,
            3 => 0b1111_1100,
            _ => panic!("Invalid duty cycle {}", duty_cycle),
        };
    }
}

#[rustfmt::skip]
const TRIANGLE_SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];

#[derive(Default, Debug)]
pub(crate) struct TriangleChannel {
    pub length_counter: LengthCounter,
    pub linear_counter: LinearCounter,

    timer: Divider<u16>,
    sequence_position: usize,
}

impl TriangleChannel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sample(&self) -> u8 {
        TRIANGLE_SEQUENCE[self.sequence_position]
    }

    pub fn write_reg1(&mut self, data: u8) {
        let control = data & 0x80 > 0;
        self.linear_counter.set_control(control);
        self.length_counter.set_halted(control);
        self.linear_counter.set_reload(data & 0x7F);
    }

    pub fn write_reg2(&mut self, data: u8) {
        self.timer.reload = (self.timer.reload & 0xFF00) | data as u16;
    }

    pub fn write_reg3(&mut self, data: u8) {
        let timer_high = (data & 0x07) as u16;
        let length_counter_load = (data & 0xF8) >> 3;

        self.timer.reload = (self.timer.reload & 0x00FF) | (timer_high << 8);
        self.timer.force_reload();
        self.length_counter.set_counter(length_counter_load);

        self.linear_counter.set_reload_flag(true);
    }

    pub fn clock(&mut self) {
        if self.timer.clock() && !self.length_counter.silenced() && !self.linear_counter.silenced()
        {
            self.sequence_position = (self.sequence_position + 1) % 32;
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.length_counter.set_enabled(enabled);
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
