use super::IntoBit;

/// A shift register with 16 slots that outputs 2 bits at a time.
#[derive(Debug, Copy, Clone)]
pub struct ShiftRegister16 {
    low: u16,
    high: u16,
}

impl ShiftRegister16 {
    pub fn new() -> Self {
        Self {
            low: 0x0000,
            high: 0x0000,
        }
    }

    pub fn load(&mut self, low: u8, high: u8) {
        self.low = (self.low & 0xFF00) | low as u16;
        self.high = (self.high & 0xFF00) | high as u16;
    }

    pub fn shift(&mut self) {
        self.low <<= 1;
        self.high <<= 1;
    }

    pub fn get(&self) -> u8 {
        self.get_at(0)
    }

    pub fn get_at(&self, bit_num: u8) -> u8 {
        assert!(bit_num < 16);

        let mask = 0x8000 >> bit_num;

        let low = (self.low & mask).into_bit();
        let high = (self.high & mask).into_bit();

        (high << 1) | low
    }

    pub fn clear(&mut self) {
        self.low = 0x0000;
        self.high = 0x0000;
    }
}

/// A shift register with 8 slots that outputs 2 bits at a time.
#[derive(Debug, Copy, Clone)]
pub struct ShiftRegister8 {
    low: u8,
    high: u8,
}

impl ShiftRegister8 {
    pub fn new() -> Self {
        Self {
            low: 0x00,
            high: 0x00,
        }
    }

    pub fn load(&mut self, low: u8, high: u8) {
        self.low = low;
        self.high = high;
    }

    pub fn shift(&mut self) {
        self.low <<= 1;
        self.high <<= 1;
    }

    pub fn get(&self) -> u8 {
        self.get_at(0)
    }

    pub fn get_at(&self, bit_num: u8) -> u8 {
        assert!(bit_num < 8);

        let mask = 0x80 >> bit_num;

        let low = (self.low & mask).into_bit();
        let high = (self.high & mask).into_bit();

        (high << 1) | low
    }

    pub fn clear(&mut self) {
        self.low = 0x00;
        self.high = 0x00;
    }
}
