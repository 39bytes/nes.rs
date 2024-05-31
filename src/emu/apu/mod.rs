use channels::{DCPMChannel, NoiseChannel, PulseChannel, TriangleChannel};

use self::channels::PulseChannelNumber;

mod channels;
mod components;

enum SequenceMode {
    FourStep,
    FiveStep,
}

pub struct Apu {
    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    dcpm: DCPMChannel,

    // Frame sequencer
    cycle: u64,
    mode: SequenceMode,
    frame_interrupt: bool,
    irq_disable: bool,
}

impl Apu {
    pub fn new() -> Apu {
        Apu {
            pulse1: PulseChannel::new(PulseChannelNumber::One),
            pulse2: PulseChannel::new(PulseChannelNumber::Two),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dcpm: DCPMChannel::new(),

            cycle: 0,
            mode: SequenceMode::FourStep,
            frame_interrupt: false,
            irq_disable: false,
        }
    }

    pub fn sample(&self) -> f32 {
        (self.pulse1.sample() as f32) / 200.0
    }

    pub fn clock(&mut self) {
        self.cycle += 1;

        if self.cycle % 2 == 0 {
            self.pulse1.clock();
            self.pulse2.clock();
        }

        // See: https://www.nesdev.org/wiki/APU_Frame_Counter
        let (quarter, half) = match self.mode {
            SequenceMode::FourStep => match self.cycle {
                7457 => (true, false),
                14913 => (true, true),
                22371 => (true, false),
                29828 => {
                    self.frame_interrupt = !self.irq_disable;

                    (false, false)
                }
                29829 => {
                    self.frame_interrupt = !self.irq_disable;
                    (true, true)
                }
                29830 => {
                    self.frame_interrupt = !self.irq_disable;
                    self.cycle = 0;
                    (false, false)
                }
                _ => (false, false),
            },
            SequenceMode::FiveStep => match self.cycle {
                7457 => (true, false),
                14913 => (true, true),
                22371 => (true, false),
                37281 => (true, true),
                37282 => {
                    self.cycle = 0;
                    (false, false)
                }
                _ => (false, false),
            },
        };

        if quarter {
            self.pulse1.envelope.clock();
            self.pulse2.envelope.clock();
        }

        if half {
            self.pulse1.length_counter.clock();
            self.pulse2.length_counter.clock();
            if let Some(target) = self.pulse1.sweep.clock(self.pulse1.timer.reload) {
                self.pulse1.set_period(target);
            }
            if let Some(target) = self.pulse2.sweep.clock(self.pulse2.timer.reload) {
                self.pulse2.set_period(target);
            }
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x4000 => self.pulse1.write_reg1(data),
            0x4001 => self.pulse1.sweep.write(data),
            0x4002 => {
                self.pulse1.timer.reload = (self.pulse1.timer.reload & 0xFF00) | (data as u16)
            }
            0x4003 => self.pulse1.write_reg4(data),
            0x4004 => self.pulse2.write_reg1(data),
            0x4005 => self.pulse2.sweep.write(data),
            0x4006 => {
                self.pulse2.timer.reload = (self.pulse2.timer.reload & 0xFF00) | (data as u16)
            }
            0x4007 => self.pulse2.write_reg4(data),
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
            0x4017 => {
                self.mode = if data & 0x80 == 0 {
                    SequenceMode::FourStep
                } else {
                    SequenceMode::FiveStep
                };
                self.irq_disable = data & 0x40 != 0;
                if self.irq_disable {
                    self.frame_interrupt = false;
                }
            }
            _ => panic!("Invalid APU address {}", addr),
        }
    }
}
