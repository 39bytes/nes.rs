use channels::{
    DMCChannel, DMCClockResult, NoiseChannel, PulseChannel, PulseChannelNumber, TriangleChannel,
};

mod channels;
mod components;

pub use channels::DMCDMARequest;

// Precompute lookup tables for sampling
// See https://www.nesdev.org/wiki/APU_Mixer#Lookup_Table
const PULSE_TABLE: [f32; 31] = const {
    let mut pulse_table = [0.0; 31];

    let mut n = 1;
    while n < 31 {
        pulse_table[n] = 95.52 / (8128.0 / (n as f32) + 100.0);
        n += 1;
    }

    pulse_table
};

const TND_TABLE: [f32; 203] = const {
    let mut tnd_table = [0.0; 203];

    let mut n = 1;
    while n < 203 {
        tnd_table[n] = 163.67 / (24329.0 / (n as f32) + 100.0);
        n += 1;
    }

    tnd_table
};

enum SequenceMode {
    FourStep,
    FiveStep,
}

pub struct Apu {
    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    dmc: DMCChannel,

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
            dmc: DMCChannel::new(),

            cycle: 0,
            mode: SequenceMode::FourStep,
            frame_interrupt: false,
            irq_disable: false,
            status_write_effect_timer: 0,
        }
    }

    pub fn sample(&self) -> f32 {
        // See: https://www.nesdev.org/wiki/APU_Mixer
        let p1 = self.pulse1.sample();
        let p2 = self.pulse2.sample();
        let pulse_out = PULSE_TABLE[(p1 + p2) as usize];
        // let pulse_out = match (p1, p2) {
        //     (0, 0) => 0.0,
        //     _ => 95.88 / ((8128.0 / (p1 as f32 + p2 as f32)) + 100.0),
        // };

        let triangle = self.triangle.sample();
        let noise = self.noise.sample();
        let dmc = self.dmc.sample();

        let tnd_out = TND_TABLE[(3 * triangle + 2 * noise + dmc) as usize];

        // let tnd_out = match (triangle, noise, dmc) {
        //     (0, 0, 0) => 0.0,
        //     _ => {
        //         let tnd = 1.0
        //             / ((triangle as f32 / 8227.0)
        //                 + (noise as f32 / 12241.0)
        //                 + (dmc as f32 / 22638.0));
        //         159.79 / (tnd + 100.0)
        //     }
        // };

        pulse_out + tnd_out
    }

    pub fn clock(&mut self, dma_sample: Option<u8>) -> Option<DMCClockResult> {
        self.cycle += 1;

        if let Some(sample) = dma_sample {
            self.dmc.write_sample_buffer(sample);
        }

        let mut res = None;

        if self.cycle % 2 == 0 {
            self.pulse1.clock();
            self.pulse2.clock();
            res = Some(self.dmc.clock());
        }
        self.triangle.clock();
        self.noise.clock();

        if self.status_write_effect_timer > 0 {
            self.status_write_effect_timer -= 1;
            if self.status_write_effect_timer == 0 {
                self.cycle = 0;
                self.clock_quarter_frame();
                self.clock_half_frame();
            }
            return res;
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

        res
    }

    fn clock_quarter_frame(&mut self) {
        self.pulse1.envelope.clock();
        self.pulse2.envelope.clock();
        self.triangle.linear_counter.clock();
        self.noise.envelope.clock();
    }

    fn clock_half_frame(&mut self) {
        self.pulse1.length_counter.clock();
        self.pulse2.length_counter.clock();
        self.triangle.length_counter.clock();
        self.noise.length_counter.clock();

        if let Some(target) = self.pulse1.sweep.clock(self.pulse1.timer.reload) {
            self.pulse1.timer.reload = target;
        }
        if let Some(target) = self.pulse2.sweep.clock(self.pulse2.timer.reload) {
            self.pulse2.timer.reload = target;
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) -> Option<DMCDMARequest> {
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
            0x400C => self.noise.write_reg1(data),
            0x400D => {}
            0x400E => self.noise.write_reg2(data),
            0x400F => self.noise.write_reg3(data),
            0x4010 => {
                self.dmc.irq_enabled = (data & 0b1000_0000) != 0;
                self.dmc.set_loop((data & 0b0100_0000) != 0);
                self.dmc.set_rate(data & 0b0000_1111);
            }
            0x4011 => {
                self.dmc.set_output_level(data & 0x7F);
            }
            0x4012 => {
                self.dmc.set_sample_address(data);
            }
            0x4013 => {
                self.dmc.set_sample_length(data);
            }
            0x4015 => {
                self.pulse1.set_enabled((data & 0x01) != 0);
                self.pulse2.set_enabled((data & 0x02) != 0);
                self.triangle.set_enabled((data & 0x04) != 0);
                self.noise.set_enabled((data & 0x08) != 0);

                // Could potentially request DMA after enabling
                return self.dmc.set_enabled((data & 0x10) != 0);
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

        None
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                let data = self.pulse1.length_counter.silenced() as u8
                    | (self.pulse2.length_counter.silenced() as u8) << 1
                    | (self.triangle.length_counter.silenced() as u8) << 2
                    | (self.noise.length_counter.silenced() as u8) << 3
                    | ((self.dmc.bytes_remaining() > 0) as u8) << 4
                    | (self.frame_interrupt as u8) << 6
                    | (self.dmc.irq_enabled as u8) << 7;

                self.frame_interrupt = false;

                data
            }
            _ => todo!(),
        }
    }
}
