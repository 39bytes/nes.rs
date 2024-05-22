mod channels;

pub struct Apu {}

impl Apu {
    pub fn new() -> Apu {
        Apu {}
    }

    pub fn clock(&mut self) {
        todo!()
    }

    pub fn write(&mut self, addr: u16) {
        match addr {
            0x4000..=0x4003 => {}
            0x4004..=0x4007 => {}
            0x4008..=0x400B => {}
            0x400C..=0x400F => {}
            0x4010..=0x4013 => {}
            0x4015 => {}
            0x4017 => {}
            _ => panic!("Invalid APU address {}", addr),
        }
    }
}
