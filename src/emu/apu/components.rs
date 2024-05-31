use num_integer::Integer;
use num_traits::Unsigned;

use super::channels::PulseChannelNumber;

// https://www.nesdev.org/wiki/APU_Length_Counter
#[rustfmt::skip]
const LENGTHS: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
    12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30
];

#[derive(Default, Debug)]
pub struct LengthCounter {
    counter: u8,
    halted: bool,
}

impl LengthCounter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_counter(&mut self, val: u8) {
        assert!(val < 32);
        self.counter = LENGTHS[val as usize];
    }

    pub fn set_halted(&mut self, halted: bool) {
        self.halted = halted;
    }

    pub fn clock(&mut self) {
        if self.halted || self.counter == 0 {
            return;
        }

        self.counter -= 1;
    }

    pub fn silenced(&self) -> bool {
        self.counter == 0
    }
}

#[derive(Default, Debug)]
pub struct Divider<U: Unsigned + Integer + Default + Copy> {
    pub reload: U,
    counter: U,
}

impl<U: Unsigned + Integer + Default + Copy> Divider<U> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn force_reload(&mut self) {
        self.counter = self.reload;
    }

    pub fn clock(&mut self) -> bool {
        if self.counter.is_zero() {
            self.counter = self.reload;
            return true;
        }

        self.counter = self.counter - U::one();

        false
    }
}

#[derive(Default, Debug)]
pub struct Envelope {
    start: bool,
    divider: Divider<u8>,
    decay_level: u8,

    loop_: bool,
    constant_volume: bool,
}

impl Envelope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_param(&mut self, param: u8) {
        self.divider.reload = param;
    }

    pub fn set_loop(&mut self, loop_: bool) {
        self.loop_ = loop_;
    }

    pub fn set_constant_volume(&mut self, constant_volume: bool) {
        self.constant_volume = constant_volume;
    }

    pub fn clock(&mut self) {
        if self.start {
            self.restart();
            return;
        }

        if self.divider.clock() {
            if self.decay_level == 0 && self.loop_ {
                self.decay_level = 15;
            } else {
                self.decay_level -= 1;
            }
        }
    }

    pub fn get_output(&mut self) -> u8 {
        // The divider's reload specifies the volume when constant volume is set
        if self.constant_volume {
            return self.divider.reload;
        }

        self.decay_level
    }

    pub fn restart(&mut self) {
        self.start = false;
        self.decay_level = 15;
        self.divider.force_reload();
    }
}

// Clocked on half frames
#[derive(Default, Debug)]
pub struct Sweep {
    pub enabled: bool,
    pub shift_count: u8,
    divider: Divider<u8>,
    reload: bool,
    negate: bool,
    channel: PulseChannelNumber,
}

impl Sweep {
    pub fn new(channel: PulseChannelNumber) -> Self {
        Self {
            channel,
            ..Default::default()
        }
    }

    pub fn write(&mut self, data: u8) {
        self.enabled = data & 0b1000_0000 != 0;
        self.divider.reload = data & 0b0111_0000 >> 4;
        self.negate = data & 0b0000_1000 != 0;
        self.shift_count = data & 0b0000_0111;

        self.reload = true;
    }

    pub fn muted(&self, cur_period: u16) -> bool {
        cur_period < 8 || self.get_target_period(cur_period) > 0x7FF
    }

    pub fn get_target_period(&self, cur_period: u16) -> u16 {
        let mut change = cur_period >> self.shift_count;
        if !self.negate {
            return cur_period + change;
        }

        if self.channel == PulseChannelNumber::One {
            change += 1;
        }

        if change > cur_period {
            return 0;
        }

        cur_period - change
    }

    pub fn clock(&mut self) -> bool {
        let res = self.divider.clock();

        if self.reload {
            self.divider.force_reload();
            self.reload = false;
        }

        res
    }
}
