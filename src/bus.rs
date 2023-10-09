use crate::cpu::Cpu6502;
use std::cell::RefCell;
use std::rc::Rc;

const RAM_SIZE: usize = 64 * 1024;

pub struct Bus {
    ram: [u8; RAM_SIZE],
}

impl Bus {
    pub fn new() -> Self {
        Bus { ram: [0; RAM_SIZE] }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        self.ram[addr as usize] = data;
    }

    pub fn read(&self, addr: u16, readonly: bool) -> u8 {
        self.ram[addr as usize]
    }
}
