use super::cartridge::Cartridge;
use super::cpu::Cpu6502;
use super::ppu::Ppu;
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;

const CPU_RAM_SIZE: usize = 2 * 1024;

pub struct Bus {
    cpu: Option<Rc<RefCell<Cpu6502>>>,
    cpu_ram: [u8; CPU_RAM_SIZE],

    ppu: Option<Rc<RefCell<Ppu>>>,
    pattern_ram: [u8; 2 * 1024],
    nametable_ram: [u8; 2 * 1024],
    palette_ram: [u8; 32],

    cartridge: Option<Rc<RefCell<Cartridge>>>,

    clock_count: u64,
}

impl Bus {
    pub fn new() -> Self {
        Bus {
            cpu: None,
            cpu_ram: [0; CPU_RAM_SIZE],

            ppu: None,
            pattern_ram: [0; 2 * 1024],
            nametable_ram: [0; 2 * 1024],
            palette_ram: [0; 32],

            cartridge: None,
            clock_count: 0,
        }
    }

    pub fn attach_cpu(&mut self, cpu: Rc<RefCell<Cpu6502>>) {
        self.cpu = Some(cpu);
    }

    pub fn attach_ppu(&mut self, ppu: Rc<RefCell<Ppu>>) {
        self.ppu = Some(ppu);
    }

    pub fn attach_cartridge(&mut self, cartridge: Rc<RefCell<Cartridge>>) {
        self.cartridge = Some(cartridge);
    }

    pub fn clock(&mut self) {
        self.clock_count += 1;
        match (&self.cpu, &self.ppu) {
            (Some(cpu), Some(ppu)) => {
                ppu.borrow_mut().clock();
                if self.clock_count % 3 == 0 {
                    cpu.borrow_mut().clock();
                }
                self.clock_count += 1;
            }
            _ => panic!("Not all devices attached to bus"),
        }
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => {
                let mapped_addr = addr as usize % CPU_RAM_SIZE;
                self.cpu_ram[mapped_addr] = data;
            }
            0x2000..=0x3FFF => match &self.ppu {
                Some(ppu) => ppu.borrow_mut().cpu_write(addr % 8, data),
                None => panic!("PPU not attached"),
            },
            0x4020..=0xFFFF => match &self.cartridge {
                Some(cartridge) => cartridge.borrow_mut().cpu_write(addr, data).unwrap(),
                None => panic!("Cartridge not attached"),
            },
            _ => panic!("Invalid CPU address: {:04X}", addr),
        }
    }

    pub fn cpu_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => {
                let mapped_addr = addr as usize % CPU_RAM_SIZE;
                self.cpu_ram[mapped_addr]
            }
            0x2000..=0x3FFF => match &self.ppu {
                Some(ppu) => ppu.borrow_mut().cpu_read(addr % 8),
                None => panic!("PPU not attached"),
            },
            0x4020..=0xFFFF => match &self.cartridge {
                Some(cartridge) => cartridge.borrow_mut().cpu_read(addr).unwrap(),
                None => panic!("Cartridge not attached"),
            },
            _ => panic!("Invalid CPU address: {:04X}", addr),
        }
    }

    pub fn ppu_write(&mut self, addr: u16, data: u8) {
        match &self.cartridge {
            Some(cartridge) => {
                if let Ok(()) = cartridge.borrow_mut().ppu_write(addr, data) {
                    return;
                }
            }
            None => panic!("Cartridge not attached"),
        };

        match addr {
            0x0000..=0x1FFF => self.pattern_ram[addr as usize] = data,
            0x2000..=0x3EFF => todo!(),
            0x3F00..=0x3FFF => {
                let i = match addr & 0x1F {
                    0x0010 | 0x0014 | 0x0018 | 0x001C => addr - 0x10,
                    x => x,
                };

                self.palette_ram[i as usize] = data;
            }
            _ => panic!("Invalid PPU address: {:04X}", addr),
        }
    }

    pub fn ppu_read(&self, addr: u16) -> u8 {
        match &self.cartridge {
            Some(cartridge) => {
                if let Ok(data) = cartridge.borrow().ppu_read(addr) {
                    return data;
                }
            }
            None => panic!("Cartridge not attached"),
        };

        match addr {
            0x0000..=0x1FFF => self.pattern_ram[addr as usize],
            0x2000..=0x3EFF => todo!(),
            0x3F00..=0x3FFF => {
                let i = match addr & 0x1F {
                    0x0010 | 0x0014 | 0x0018 | 0x001C => addr - 0x10,
                    x => x,
                };

                self.palette_ram[i as usize]
            }
            _ => panic!("Invalid PPU address: {:04X}", addr),
        }
    }

    pub fn page_str(&self, page: u8) -> String {
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
                s.push_str(&format!("{:02X} ", self.cpu_read(idx)));
            }
            s.push('\n');
        }

        s
    }
}

impl Display for Bus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.page_str(0))
    }
}
