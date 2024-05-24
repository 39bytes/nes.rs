use channels::{DCPMChannel, NoiseChannel, PulseChannel, TriangleChannel};

mod channels;

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
            pulse1: PulseChannel::new(),
            pulse2: PulseChannel::new(),
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
                let duty_cycle = (data & 0xC0) >> 6;
                let length_counter_halt = (data & 0x20) != 0;
                let constant_volume = (data & 0x10) != 0;
                let envelope_divider_period = data & 0x0F;

                self.pulse1.set_duty_cycle(duty_cycle);
                self.pulse1.set_length_counter_halt(length_counter_halt);
                self.pulse1.set_constant_volume(constant_volume);
                self.pulse1.set_divider_period(envelope_divider_period);
            }
            0x4001 => {}
            0x4002 => {
                self.pulse1.set_timer_low(data);
            }
            0x4003 => {
                let timer_high = data & 0x07;
                let length_counter_load = (data & 0xF8) >> 3;

                self.pulse1.set_timer_high(timer_high);
                self.pulse1.set_length_counter(length_counter_load);
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
