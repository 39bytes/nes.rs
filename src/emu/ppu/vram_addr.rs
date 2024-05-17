use modular_bitfield::prelude::*;

#[bitfield]
#[derive(Clone, Copy, Debug)]
pub(crate) struct VRAMAddr {
    pub coarse_x: B5,
    pub coarse_y: B5,
    pub nametable_x: B1,
    pub nametable_y: B1,
    pub fine_y: B3,
    #[skip]
    padding: B1,
}

impl VRAMAddr {
    pub(crate) fn increment_coarse_x(&mut self) {
        self.set_coarse_x(self.coarse_x() + 1);
    }

    pub(crate) fn increment_coarse_y(&mut self) {
        self.set_coarse_y(self.coarse_y() + 1);
    }

    pub(crate) fn increment_nametable_x(&mut self) {
        self.set_nametable_x((self.nametable_x() + 1) % 2);
    }

    pub(crate) fn increment_nametable_y(&mut self) {
        self.set_nametable_y((self.nametable_x() + 1) % 2);
    }

    pub(crate) fn increment_fine_y(&mut self) {
        self.set_fine_y(self.fine_y() + 1);
    }
}

impl From<u16> for VRAMAddr {
    fn from(addr: u16) -> Self {
        let low = addr & 0x00FF;
        let high = addr >> 8;

        VRAMAddr::from_bytes([low as u8, high as u8])
    }
}

impl From<VRAMAddr> for u16 {
    fn from(vram_addr: VRAMAddr) -> Self {
        let [low, high] = vram_addr.into_bytes();

        ((high as u16) << 8) | low as u16
    }
}
