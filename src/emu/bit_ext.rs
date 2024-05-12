pub trait BitExt {
    fn into_bit(&self) -> u8;
}

impl BitExt for u16 {
    fn into_bit(&self) -> u8 {
        if *self > 0 {
            1
        } else {
            0
        }
    }
}
