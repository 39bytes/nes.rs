use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PpuCtrl: u8 {
        /// Base nametable address
        /// 0: $2000; 1: $2400; 2: $2800; 3: $2C00
        const NametableLSB = 1 << 0;
        const NametableMSB = 1 << 1;
        /// VRAM address increment per CPU read/write of PPUDATA
        /// 0: add 1, going across; 1: add 32, going down
        const VRamAddressIncrement = 1 << 2;
        /// Sprite pattern table address for 8x8 sprites
        /// 0: $0000; 1: $1000; ignored in 8x16 mode
        const SpritePatternTable = 1 << 3;
        /// Background pattern table address
        /// 0: $0000; 1: $1000
        const BackgroundPatternTable = 1 << 4;
        /// Sprite size
        /// 0: 8x8; 1: 8x16
        const SpriteSize = 1 << 5;
        const MasterSlave = 1 << 6;
        /// Generate NMI at start of vertical blanking interval
        const GenerateNMI = 1 << 7;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PpuMask: u8 {
        const Greyscale = 1 << 0;
        const ShowBackgroundLeft = 1 << 1;
        const ShowSpritesLeft = 1 << 2;
        const ShowBackground = 1 << 3;
        const ShowSprites = 1 << 4;
        const EmphasizeRed = 1 << 5;
        const EmphasizeGreen = 1 << 6;
        const EmphasizeBlue = 1 << 7;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PpuStatus: u8 {
        const SpriteOverflow = 1 << 5;
        const Sprite0Hit = 1 << 6;
        const VerticalBlank = 1 << 7;
    }
}
