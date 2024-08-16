#[derive(Debug, Clone, Copy)]
pub enum PatternTable {
    Left,
    Right,
}

impl From<bool> for PatternTable {
    fn from(value: bool) -> Self {
        match value {
            false => PatternTable::Left,
            true => PatternTable::Right,
        }
    }
}

impl PatternTable {
    pub fn addr(self) -> u16 {
        match self {
            PatternTable::Left => 0x0000,
            PatternTable::Right => 0x1000,
        }
    }
}
