use crate::emu::bits::IntoBit;

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

        self.envelope.volume()
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
        let timer_high = (data & 0b0000_0111) as u16;
        let length_counter_load = (data & 0b1111_1000) >> 3;

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

const NOISE_PERIOD_LOOKUP: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

pub(crate) struct NoiseChannel {
    pub length_counter: LengthCounter,
    pub envelope: Envelope,
    mode: bool,
    timer: Divider<u16>,
    shift_register: u16,
}

impl Default for NoiseChannel {
    fn default() -> Self {
        Self {
            length_counter: LengthCounter::default(),
            envelope: Envelope::default(),
            mode: false,
            timer: Divider::default(),
            shift_register: 1,
        }
    }
}

impl NoiseChannel {
    pub fn new() -> Self {
        NoiseChannel::default()
    }

    pub fn sample(&self) -> u8 {
        if self.length_counter.silenced() || self.shift_register & 0x01 != 0 {
            return 0;
        }

        self.envelope.volume()
    }

    pub fn write_reg1(&mut self, data: u8) {
        self.length_counter.set_halted(data & 0b0010_0000 != 0);
        self.envelope.set_constant_volume(data & 0b0001_0000 != 0);
        self.envelope.set_param(data & 0b0000_1111);
    }

    pub fn write_reg2(&mut self, data: u8) {
        self.mode = data & 0b1000_0000 != 0;
        self.timer.reload = NOISE_PERIOD_LOOKUP[(data & 0x0F) as usize];
    }

    pub fn write_reg3(&mut self, data: u8) {
        self.length_counter.set_counter((data & 0xF8) >> 3);
        self.envelope.restart();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.length_counter.set_enabled(enabled);
    }

    pub fn clock(&mut self) {
        let bit0 = (self.shift_register & 0x01).into_bit();
        let other_bit = if self.mode {
            self.shift_register & 0x0040
        } else {
            self.shift_register & 0x0002
        }
        .into_bit();

        let feedback = (bit0 ^ other_bit) as u16;
        self.shift_register >>= 1;
        self.shift_register |= feedback << 14;
    }
}

const DMC_RATE_LOOKUP: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

// TODO: Implement this channel
//
//
#[derive(Copy, Clone)]
pub enum DMCDMARequest {
    Load(u16),
    Reload(u16),
}

pub struct DMCClockResult {
    pub dma_req: Option<DMCDMARequest>,
    pub interrupt: bool,
}

// NOTE:
// If the DMC bit is clear, the DMC bytes remaining will be set to 0 and the DMC will silence when it empties.
// If the DMC bit is set, the DMC sample will be restarted only if its bytes remaining is 0. If there are bits remaining in the 1-byte sample buffer, these will finish playing before the next sample is fetched.
// Writing to this register clears the DMC interrupt flag.
#[derive(Default)]
pub(crate) struct DMCChannel {
    enabled: bool,
    pub irq_enabled: bool,
    interrupt: bool,
    loop_: bool,

    sample_addr: u16,
    sample_length: u16,

    current_addr: u16,
    bytes_remaining: u16,
    sample_buffer: Option<u8>,

    timer: Divider<u16>,
    shifter: u8,
    bits_remaining: u8,
    output_level: u8,
    silence: bool,
}

impl DMCChannel {
    pub fn new() -> Self {
        DMCChannel::default()
    }

    pub fn clock(&mut self, dma_sample: Option<u8>) -> DMCClockResult {
        // New sample came in from DMA
        // https://www.nesdev.org/wiki/APU_DMC#Memory_reader
        if let Some(sample) = dma_sample {
            self.sample_buffer = Some(sample);
            if self.sample_addr == 0xFFFF {
                self.sample_addr = 0x8000;
            } else {
                self.sample_addr += 1;
            }

            self.bytes_remaining -= 1;
            if self.bytes_remaining == 0 {
                if self.loop_ {
                    self.restart();
                } else if self.irq_enabled {
                    self.interrupt = true;
                }
            }
        }

        // Advance in the output cycle
        // https://www.nesdev.org/wiki/APU_DMC#Output_unit
        if self.timer.clock() {
            if !self.silence {
                match self.shifter & 0x01 {
                    0 if self.output_level >= 2 => self.output_level -= 2,
                    1 if self.output_level <= 125 => self.output_level += 2,
                    _ => {}
                }
            }
            self.shifter >>= 1;
            self.bits_remaining -= 1;

            // New output cycle
            if self.bits_remaining == 0 {
                self.bits_remaining = 8;
                match self.sample_buffer {
                    Some(data) => {
                        self.silence = false;

                        // Empty the sample buffer into the shift register
                        self.shifter = data;
                        self.sample_buffer = None;

                        if self.bytes_remaining != 0 {
                            return DMCClockResult {
                                dma_req: Some(DMCDMARequest::Reload(self.current_addr)),
                                interrupt: self.interrupt,
                            };
                        }
                    }
                    None => {
                        self.silence = true;
                    }
                }
            }
        }

        return DMCClockResult {
            dma_req: None,
            interrupt: self.interrupt,
        };
    }

    pub fn sample(&self) -> u8 {
        self.output_level
    }

    pub fn bytes_remaining(&mut self) -> u16 {
        self.bytes_remaining
    }

    pub fn set_enabled(&mut self, enabled: bool) -> Option<DMCDMARequest> {
        self.enabled = enabled;
        self.interrupt = false;

        if !enabled {
            self.bytes_remaining = 0;
        } else if self.bytes_remaining == 0 {
            self.restart();
        }

        match self.sample_buffer {
            None if self.bytes_remaining != 0 => Some(DMCDMARequest::Load(self.current_addr)),
            _ => None,
        }
    }

    pub fn set_loop(&mut self, b: bool) {
        self.loop_ = b;
    }

    pub fn set_rate(&mut self, index: u8) {
        self.timer.reload = DMC_RATE_LOOKUP[index as usize];
    }

    pub fn set_output_level(&mut self, level: u8) {
        self.output_level = level;
    }

    pub fn set_sample_address(&mut self, addr_offset: u8) {
        self.sample_addr = 0xC000 + addr_offset as u16 * 64;
    }

    pub fn set_sample_length(&mut self, length: u8) {
        self.sample_length = length as u16 * 16 + 1;
    }

    pub fn restart(&mut self) {
        self.current_addr = self.sample_addr;
        self.bytes_remaining = self.sample_length;
    }
}
