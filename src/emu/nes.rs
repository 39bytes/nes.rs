use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use super::{cartridge::Cartridge, cpu::Cpu6502, palette::Palette, ppu::Ppu};

pub struct Nes {
    cpu: Rc<RefCell<Cpu6502>>,
    ppu: Rc<RefCell<Ppu>>,
    cartridge: Option<Rc<RefCell<Cartridge>>>,

    clock_count: u64,
}

impl Nes {
    pub fn new(palette: Palette) -> Self {
        let cpu = Rc::new(RefCell::new(Cpu6502::new()));
        let ppu = Rc::new(RefCell::new(Ppu::new(palette)));
        ppu.borrow_mut().with_cpu(cpu.clone());
        cpu.borrow_mut().with_ppu(ppu.clone());

        Nes {
            cpu,
            ppu,
            cartridge: None,

            clock_count: 0,
        }
    }

    pub fn cpu(&self) -> Ref<Cpu6502> {
        self.cpu.borrow()
    }

    pub fn ppu(&self) -> Ref<Ppu> {
        self.ppu.borrow()
    }

    pub fn load_cartridge(&mut self, cartridge: Cartridge) {
        let cartridge = Rc::new(RefCell::new(cartridge));
        self.cpu.borrow_mut().with_cartridge(cartridge.clone());
        self.ppu.borrow_mut().with_cartridge(cartridge.clone());
    }

    pub fn reset(&mut self) {
        self.cpu.borrow_mut().reset();
    }

    pub fn clock(&mut self) {
        self.clock_count += 1;
        self.ppu.borrow_mut().clock();
        if self.clock_count % 3 == 0 {
            self.cpu.borrow_mut().clock();
        }
        self.clock_count += 1;
    }

    pub fn next_instruction(&mut self) {
        self.cpu.borrow_mut().next_instruction();
    }

    pub fn cpu_mem_page_str(&self, page: u8) -> String {
        let page_start = (page as u16) * 0x100;

        let mut s = String::new();
        s.push_str("   ");
        for i in 0..16 {
            s.push_str(&format!("{:X}  ", i));
        }
        s.push('\n');
        for i in 0..16 {
            s.push_str(&format!("{:X}  ", i));
            for j in 0..16 {
                let idx = page_start + i * 0x10 + j;
                s.push_str(&format!("{:02X} ", self.cpu.borrow().read(idx)));
            }
            s.push('\n');
        }

        s
    }
}
