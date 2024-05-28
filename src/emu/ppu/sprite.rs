use bitflags::bitflags;

bitflags! {
    #[derive(Debug)]
    pub struct SpriteAttribute: u8 {
        const PaletteLSB = 1 << 0;
        const PaletteMSB = 1 << 1;

        const BehindBackground = 1 << 5;
        const FlipHorizontally = 1 << 6;
        const FlipVertically = 1 << 7;
    }
}

#[derive(Debug)]
pub struct Sprite {
    pub x: u8,
    pub y: u8,
    pub tile_id: u8,
    pub attribute: SpriteAttribute,
    pub oam_index: usize,
}

impl Default for Sprite {
    fn default() -> Self {
        Sprite {
            x: 0xFF,
            y: 0xFF,
            tile_id: 0xFF,
            attribute: SpriteAttribute::from_bits_truncate(0xFF),
            oam_index: 0x40,
        }
    }
}
