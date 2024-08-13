use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use crate::{
    audio_output::AudioOutput,
    renderer::{Color, Sprite},
};

use super::{
    apu::Apu, cartridge::Cartridge, consts::FRAME_CLOCKS, cpu::Cpu, input::ControllerInput,
    palette::Palette, ppu::Ppu,
};

pub struct Nes {
    apu: Rc<RefCell<Apu>>,
    cpu: Rc<RefCell<Cpu>>,
    ppu: Rc<RefCell<Ppu>>,

    screen: Sprite,
    audio_output: Option<AudioOutput>,

    clock_count: u64,
    paused: bool,
}

impl Nes {
    pub fn new(palette: Palette) -> Self {
        let cpu = Rc::new(RefCell::new(Cpu::new()));
        let ppu = Rc::new(RefCell::new(Ppu::new(palette)));
        let apu = Rc::new(RefCell::new(Apu::new()));
        cpu.borrow_mut().with_ppu(ppu.clone());
        cpu.borrow_mut().with_apu(apu.clone());

        Nes {
            cpu,
            ppu,
            apu,

            screen: Sprite::monocolor(Color::BLACK, 256, 240),
            audio_output: None,

            clock_count: 0,
            paused: false,
        }
    }

    pub fn with_audio(&mut self, output: AudioOutput) {
        self.audio_output = Some(output);
    }

    pub fn cpu(&self) -> Ref<Cpu> {
        self.cpu.borrow()
    }

    pub fn ppu(&self) -> Ref<Ppu> {
        self.ppu.borrow()
    }

    pub fn screen(&self) -> &Sprite {
        &self.screen
    }

    #[allow(dead_code)]
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
        if let Some(output) = self.audio_output.as_mut() {
            output.pause();
            output.clear();
            output.play();
        }
    }

    pub fn trigger_inputs(&mut self, input: ControllerInput) {
        self.cpu.borrow_mut().trigger_inputs(input);
    }

    pub fn advance_frame(&mut self) -> bool {
        while !self.clock(false) {}

        self.paused
    }

    pub fn clock(&mut self, force: bool) -> bool {
        if self.paused && !force {
            return true;
        }
        self.clock_count += 1;

        let clock_res = self.ppu.borrow_mut().clock();

        if let Some(pixel) = clock_res.pixel {
            if let Err(e) = self.screen.set_pixel(pixel.x, pixel.y, pixel.color) {
                panic!("{}", e);
            }
        }

        let mut irq = clock_res.irq;

        if self.clock_count % 3 == 0 {
            let cpu_res = self.cpu.borrow_mut().clock(self.paused);
            if cpu_res.breakpoint_hit {
                self.paused = true;
            };
            let apu_res = self.apu.borrow_mut().clock(cpu_res.dmc_dma_sample);

            if let Some(res) = apu_res.dmc_res {
                if let Some(req) = res.dma_req {
                    self.cpu.borrow_mut().begin_dmc_dma(req);
                }

                irq = irq || res.interrupt;
            }
            irq = irq || apu_res.frame_interrupt;
        }

        if let Some(audio_output) = &mut self.audio_output {
            audio_output.try_push_sample(self.apu.borrow().sample());
        }

        if clock_res.nmi {
            self.cpu.borrow_mut().request_nmi();
        }

        if irq {
            log::info!(
                "Pending IRQ from NES on scanline {}",
                self.ppu.borrow().scanline()
            );
            self.cpu.borrow_mut().request_irq();
        }
        if clock_res.frame_complete {
            self.cpu.borrow_mut().mark_frame_complete();
        }

        clock_res.frame_complete
    }

    pub fn next_instruction(&mut self) {
        let cycles = self.cpu.borrow().cycles();
        let until_next_cpu_cycle = (3 - self.clock_count % 3) as u8;
        let to_next = until_next_cpu_cycle + cycles * 3;
        for _ in 0..to_next {
            self.clock(true);
        }
    }

    pub fn set_breakpoint(&mut self, breakpoint: u16) {
        self.cpu.borrow_mut().set_breakpoint(breakpoint);
    }

    pub fn unpause(&mut self) {
        self.paused = false;
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
    use super::*;

    fn rom_test(path: &str) {
        let mut nes = Nes::new(Palette::default());
        let cartridge = Cartridge::new(path).unwrap();
        nes.load_cartridge(cartridge);
        nes.reset();

        for _ in 0..50_000_000 {
            nes.clock(false);
        }

        // Read test status code
        let cpu = nes.cpu.borrow_mut();
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
        rom_test("assets/test_roms/instr_test-v5/01-basics.nes");
    }

    #[test]
    fn instr_test_v5_02_implied() {
        rom_test("assets/test_roms/instr_test-v5/02-implied.nes");
    }

    #[test]
    fn instr_test_v5_03_immediate() {
        rom_test("assets/test_roms/instr_test-v5/03-immediate.nes");
    }

    #[test]
    fn instr_test_v5_04_zero_page() {
        rom_test("assets/test_roms/instr_test-v5/04-zero_page.nes");
    }

    #[test]
    fn instr_test_v5_05_zp_xy() {
        rom_test("assets/test_roms/instr_test-v5/05-zp_xy.nes");
    }

    #[test]
    fn instr_test_v5_06_absolute() {
        rom_test("assets/test_roms/instr_test-v5/06-absolute.nes");
    }

    #[test]
    fn instr_test_v5_07_abs_xy() {
        rom_test("assets/test_roms/instr_test-v5/07-abs_xy.nes");
    }

    #[test]
    fn instr_test_v5_08_ind_x() {
        rom_test("assets/test_roms/instr_test-v5/08-ind_x.nes");
    }

    #[test]
    fn instr_test_v5_09_ind_y() {
        rom_test("assets/test_roms/instr_test-v5/09-ind_y.nes");
    }

    #[test]
    fn instr_test_v5_10_branches() {
        rom_test("assets/test_roms/instr_test-v5/10-branches.nes");
    }

    #[test]
    fn instr_test_v5_11_stack() {
        rom_test("assets/test_roms/instr_test-v5/11-stack.nes");
    }

    #[test]
    fn instr_test_v5_12_jmp_jsr() {
        rom_test("assets/test_roms/instr_test-v5/12-jmp_jsr.nes");
    }

    #[test]
    fn instr_test_v5_13_rts() {
        rom_test("assets/test_roms/instr_test-v5/13-rts.nes");
    }

    #[test]
    fn instr_test_v5_14_rti() {
        rom_test("assets/test_roms/instr_test-v5/14-rti.nes");
    }

    #[test]
    fn instr_test_v5_15_brk() {
        rom_test("assets/test_roms/instr_test-v5/15-brk.nes");
    }

    #[test]
    fn instr_test_v5_16_special() {
        rom_test("assets/test_roms/instr_test-v5/16-special.nes");
    }

    #[test]
    fn cpu_interrupts_v2_1_cli_latency() {
        rom_test("assets/test_roms/cpu_interrupts_v2/1-cli_latency.nes");
    }

    #[test]
    fn cpu_interrupts_v2_2_nmi_and_brk() {
        rom_test("assets/test_roms/cpu_interrupts_v2/2-nmi_and_brk.nes");
    }

    #[test]
    fn cpu_interrupts_v2_3_nmi_and_irq() {
        rom_test("assets/test_roms/cpu_interrupts_v2/3-nmi_and_irq.nes");
    }

    #[test]
    fn cpu_interrupts_v2_4_irq_and_dma() {
        rom_test("assets/test_roms/cpu_interrupts_v2/4-irq_and_dma.nes");
    }

    #[test]
    fn cpu_interrupts_v2_5_branch_delays_irq() {
        rom_test("assets/test_roms/cpu_interrupts_v2/5-branch_delays_irq.nes");
    }
}
