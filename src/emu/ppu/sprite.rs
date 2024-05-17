use bitflags::bitflags;

bitflags! {
    pub struct SpriteAttribute: u8 {
        const PaletteLSB = 1 << 0;
        const PaletteMSB = 1 << 1;

        const Priority = 1 << 5;
        const FlipHorizontally = 1 << 6;
        const FlipVertically = 1 << 7;
    }
}

pub struct Sprite {
    pub x: u8,
    pub y: u8,
    pub tile_id: u8,
    pub attribute: SpriteAttribute,
}
