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

pub fn flip_byte(mut byte: u8) -> u8 {
    let mut res = 0;
    for _ in 0..7 {
        res |= byte & 0x01;
        byte >>= 1;
        res <<= 1;
    }
    res | byte & 0x01
}

pub fn rotate_byte_right(byte: u8) -> u8 {
    (byte >> 1) | (byte << 7)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn flip_byte_works() {
        assert_eq!(flip_byte(0b0000_0001), 0b1000_0000);
        assert_eq!(flip_byte(0b0000_0000), 0b0000_0000);
        assert_eq!(flip_byte(0b1111_1111), 0b1111_1111);
        assert_eq!(flip_byte(0b0000_1111), 0b1111_0000);
        assert_eq!(flip_byte(0b0011_1011), 0b1101_1100);
    }
}
