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
    // Counts clock cycles until the timer reset/quarter + half frame clocks
    // are executed after a write to 0x4017
    status_write_effect_timer: u8,
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
            status_write_effect_timer: 0,
        }
    }

    pub fn sample(&self) -> f32 {
        // See: https://www.nesdev.org/wiki/APU_Mixer
        let p1 = self.pulse1.sample() as f32;
        let p2 = self.pulse2.sample() as f32;
        let pulse_out = 95.88 / ((8128.0 / (p1 + p2)) + 100.0);

        let triangle = self.triangle.sample() as f32;
        let noise = 0 as f32;
        let dmc = 0 as f32;

        let tnd = 1.0 / ((triangle / 8227.0) + (noise / 12241.0) + (dmc / 22638.0));
        let tnd_out = 159.79 / (tnd + 100.0);

        pulse_out + tnd_out
    }

    pub fn clock(&mut self) {
        self.cycle += 1;

        if self.cycle % 2 == 0 {
            self.pulse1.clock();
            self.pulse2.clock();
        }
        self.triangle.clock();

        if self.status_write_effect_timer > 0 {
            self.status_write_effect_timer -= 1;
            if self.status_write_effect_timer == 0 {
                self.cycle = 0;
                self.clock_quarter_frame();
                self.clock_half_frame();
            }
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
            self.clock_quarter_frame();
        }

        if half {
            self.clock_half_frame();
        }
    }

    fn clock_quarter_frame(&mut self) {
        self.pulse1.envelope.clock();
        self.pulse2.envelope.clock();
        self.triangle.linear_counter.clock();
    }

    fn clock_half_frame(&mut self) {
        self.pulse1.length_counter.clock();
        self.pulse2.length_counter.clock();
        self.triangle.length_counter.clock();

        if let Some(target) = self.pulse1.sweep.clock(self.pulse1.timer.reload) {
            self.pulse1.timer.reload = target;
        }
        if let Some(target) = self.pulse2.sweep.clock(self.pulse2.timer.reload) {
            self.pulse2.timer.reload = target;
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
            0x4008 => self.triangle.write_reg1(data),
            0x4009 => {}
            0x400A => self.triangle.write_reg2(data),
            0x400B => self.triangle.write_reg3(data),
            0x400C => {}
            0x400D => {}
            0x400E => {}
            0x400F => {}
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

                if self.cycle % 2 == 1 {
                    self.status_write_effect_timer = 3;
                } else {
                    self.status_write_effect_timer = 4;
                }
            }
            _ => panic!("Invalid APU address {}", addr),
        }
    }
}
