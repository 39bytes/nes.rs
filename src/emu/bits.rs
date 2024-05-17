pub trait IntoBit {
    fn into_bit(self) -> u8;
}

impl IntoBit for u16 {
    fn into_bit(self) -> u8 {
        if self > 0 {
            1
        } else {
            0
        }
    }
}

impl IntoBit for u8 {
    fn into_bit(self) -> u8 {
        if self > 0 {
            1
        } else {
            0
        }
    }
}
