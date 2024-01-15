use std::{cell::RefCell, rc::Weak};

use super::bus::Bus;

pub struct Ppu {
    bus: Weak<RefCell<Bus>>,

    pattern_ram: [u8; 2 * 1024],
    nametable_ram: [u8; 2 * 1024],
    palette_ram: [u8; 32],

    scanline: i16,
    cycle: i16,
}

impl Ppu {
    pub fn new(bus: Weak<RefCell<Bus>>) -> Self {
        Ppu {
            bus,
            pattern_ram: [0; 2 * 1024],
            nametable_ram: [0; 2 * 1024],
            palette_ram: [0; 32],

            scanline: 0,
            cycle: 0,
        }
    }

    pub fn clock(&mut self) {
        self.cycle += 1;

        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;

            if self.scanline > 260 {
                self.scanline = -1;
            }
        }
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000 => {}
            0x0001 => {}
            0x0002 => {}
            0x0003 => {}
            0x0004 => {}
            0x0005 => {}
            0x0006 => {}
            0x0007 => {}
            _ => panic!("Invalid register"),
        }
    }

    pub fn cpu_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000 => 0,
            0x0001 => 0,
            0x0002 => 0,
            0x0003 => 0,
            0x0004 => 0,
            0x0005 => 0,
            0x0006 => 0,
            0x0007 => 0,
            _ => panic!("Invalid register"),
        }
    }
}
