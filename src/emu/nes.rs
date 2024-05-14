use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use anyhow::Result;

use crate::renderer::{Color, Sprite};

use super::{
    cartridge::Cartridge, cpu::Cpu6502, input::ControllerButtons, palette::Palette, ppu::Ppu,
};

pub struct Nes {
    cpu: Rc<RefCell<Cpu6502>>,
    ppu: Rc<RefCell<Ppu>>,
    cartridge: Option<Rc<RefCell<Cartridge>>>,

    screen: Sprite,

    clock_count: u64,
}

impl Nes {
    pub fn new(palette: Palette) -> Self {
        let cpu = Rc::new(RefCell::new(Cpu6502::new()));
        let ppu = Rc::new(RefCell::new(Ppu::new(palette, cpu.clone())));
        cpu.borrow_mut().with_ppu(ppu.clone());

        Nes {
            cpu,
            ppu,
            cartridge: None,

            screen: Sprite::monocolor(Color::BLACK, 256, 240),

            clock_count: 0,
        }
    }

    pub fn cpu(&self) -> Ref<Cpu6502> {
        self.cpu.borrow()
    }

    pub fn cpu_mut(&self) -> RefMut<Cpu6502> {
        self.cpu.borrow_mut()
    }

    pub fn ppu(&self) -> Ref<Ppu> {
        self.ppu.borrow()
    }

    pub fn ppu_mut(&self) -> RefMut<Ppu> {
        self.ppu.borrow_mut()
    }

    pub fn screen(&self) -> &Sprite {
        &self.screen
    }

    pub fn clock_count(&self) -> u64 {
        self.clock_count
    }

    pub fn load_cartridge(&mut self, cartridge: Cartridge) {
        let cartridge = Rc::new(RefCell::new(cartridge));
        self.cpu.borrow_mut().load_cartridge(cartridge.clone());
        self.ppu.borrow_mut().load_cartridge(cartridge.clone());
    }

    pub fn reset(&mut self) {
        self.cpu.borrow_mut().reset();
    }

    pub fn trigger_inputs(&mut self, buttons: ControllerButtons) {
        self.cpu.borrow_mut().trigger_inputs(buttons);
    }

    pub fn clock(&mut self) {
        self.clock_count += 1;
        let clock_res = self.ppu.borrow_mut().clock();
        if let Some(pixel) = clock_res.pixel {
            if let Err(e) = self.screen.set_pixel(pixel.x, pixel.y, pixel.color) {
                panic!("{}", e);
            }
        }

        if self.clock_count % 3 == 0 {
            self.cpu.borrow_mut().clock();
        }

        if clock_res.nmi {
            self.cpu.borrow_mut().nmi();
        }
    }

    pub fn next_instruction(&mut self) {
        let cycles = self.cpu.borrow().cycles();
        let until_next_cpu_cycle = (3 - self.clock_count % 3) as u8;
        let to_next = until_next_cpu_cycle + cycles * 3;
        for _ in 0..to_next {
            self.clock();
        }
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
                s.push_str(&format!("{:02X} ", self.cpu.borrow_mut().read(idx)));
            }
            s.push('\n');
        }

        s
    }
}

#[cfg(test)]
mod test {
    use crate::emu::instructions::{Instruction, InstructionType};

    use super::*;

    fn rom_test(path: &str) {
        let mut nes = Nes::new(Palette::default());
        let cartridge = Cartridge::new(path).unwrap();
        nes.load_cartridge(cartridge);
        nes.reset();

        // Status of 0x80 means the test is still running
        // while nes.cpu().read_debug(0x6000) == 0x80 {
        for _ in 0..50_000_000 {
            nes.clock();
        }

        // Read test status code
        let cpu = nes.cpu_mut();
        let status = cpu.read_debug(0x6000);

        let mut char_addr = 0x6004;
        let mut message = String::new();

        loop {
            let byte = cpu.read_debug(char_addr);
            if byte == 0x00 {
                break;
            }
            message.push(byte as char);
            char_addr += 1;
        }

        assert_eq!(status, 0, "{}", message);
    }

    #[test]
    fn instr_test_v5_01_basics() {
        rom_test("assets/roms/instr_test-v5/01-basics.nes");
    }

    #[test]
    fn instr_test_v5_02_implied() {
        rom_test("assets/roms/instr_test-v5/02-implied.nes");
    }

    #[test]
    fn instr_test_v5_03_immediate() {
        rom_test("assets/roms/instr_test-v5/03-immediate.nes");
    }

    #[test]
    fn instr_test_v5_04_zero_page() {
        rom_test("assets/roms/instr_test-v5/04-zero_page.nes");
    }

    #[test]
    fn instr_test_v5_05_zp_xy() {
        rom_test("assets/roms/instr_test-v5/05-zp_xy.nes");
    }

    #[test]
    fn instr_test_v5_06_absolute() {
        rom_test("assets/roms/instr_test-v5/06-absolute.nes");
    }

    #[test]
    fn instr_test_v5_07_abs_xy() {
        rom_test("assets/roms/instr_test-v5/07-abs_xy.nes");
    }

    #[test]
    fn instr_test_v5_08_ind_x() {
        rom_test("assets/roms/instr_test-v5/08-ind_x.nes");
    }

    #[test]
    fn instr_test_v5_09_ind_y() {
        rom_test("assets/roms/instr_test-v5/09-ind_y.nes");
    }

    #[test]
    fn instr_test_v5_10_branches() {
        rom_test("assets/roms/instr_test-v5/10-branches.nes");
    }

    #[test]
    fn instr_test_v5_11_stack() {
        rom_test("assets/roms/instr_test-v5/11-stack.nes");
    }

    #[test]
    fn instr_test_v5_12_jmp_jsr() {
        rom_test("assets/roms/instr_test-v5/12-jmp_jsr.nes");
    }

    #[test]
    fn instr_test_v5_13_rts() {
        rom_test("assets/roms/instr_test-v5/13-rts.nes");
    }

    #[test]
    fn instr_test_v5_14_rti() {
        rom_test("assets/roms/instr_test-v5/14-rti.nes");
    }

    #[test]
    fn instr_test_v5_15_brk() {
        rom_test("assets/roms/instr_test-v5/15-brk.nes");
    }

    #[test]
    fn instr_test_v5_16_special() {
        rom_test("assets/roms/instr_test-v5/16-special.nes");
    }
}
