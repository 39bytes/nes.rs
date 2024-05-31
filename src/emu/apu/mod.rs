use channels::{DCPMChannel, NoiseChannel, PulseChannel, TriangleChannel};

use self::channels::PulseChannelNumber;

mod channels;
mod components;

pub struct Apu {
    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    dcpm: DCPMChannel,
}

impl Apu {
    pub fn new() -> Apu {
        Apu {
            pulse1: PulseChannel::new(PulseChannelNumber::One),
            pulse2: PulseChannel::new(PulseChannelNumber::Two),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dcpm: DCPMChannel::new(),
        }
    }

    pub fn sample(&self) -> f32 {
        (self.pulse1.sample() as f32) / 200.0
    }

    pub fn clock(&mut self) {
        self.pulse1.clock();
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x4000 => {
                self.pulse1.set_duty_cycle((data & 0b1100_0000) >> 6);
                let l = (data & 0b0010_0000) != 0;
                self.pulse1.envelope.set_loop(l);
                self.pulse1.length_counter.set_halted(l);

                self.pulse1
                    .envelope
                    .set_constant_volume((data & 0b0001_0000) != 0);
                self.pulse1.envelope.set_param(data & 0b0000_1111);
            }
            0x4001 => {
                self.pulse1.sweep.write(data);
            }
            0x4002 => {
                self.pulse1.timer.reload = (self.pulse1.timer.reload & 0xFF00) | (data as u16);
            }
            0x4003 => {
                let timer_high = (data & 0x07) as u16;
                let length_counter_load = (data & 0xF8) >> 3;

                self.pulse1.timer.reload = (self.pulse1.timer.reload & 0x00FF) | (timer_high << 8);
                self.pulse1.timer.force_reload();
                self.pulse1.length_counter.set_counter(length_counter_load);
            }
            0x4004 => {}
            0x4005 => {}
            0x4006 => {}
            0x4007 => {}
            0x4008 => {}
            0x4009 => {}
            0x400a => {}
            0x400b => {}
            0x400c => {}
            0x400d => {}
            0x400e => {}
            0x400f => {}
            0x4010 => {}
            0x4011 => {}
            0x4012 => {}
            0x4013 => {}
            0x4015 => {
                self.pulse1.set_enabled((data & 0x01) != 0);
                self.pulse2.set_enabled((data & 0x02) != 0);
                self.triangle.set_enabled((data & 0x04) != 0);
                self.noise.set_enabled((data & 0x08) != 0);
                self.dcpm.set_enabled((data & 0x10) != 0);
            }
            0x4017 => {}
            _ => panic!("Invalid APU address {}", addr),
        }
    }
}
