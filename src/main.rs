use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use emu::ppu::Ppu;
use error_iter::ErrorIter as _;
use log::error;
use renderer::Renderer;
use rusttype::Font;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use emu::bus::Bus;
use emu::cpu::Cpu6502;
use emu::instructions::{AddressMode, Instruction};

mod emu;
mod renderer;

const WIDTH: u32 = 960;
const HEIGHT: u32 = 720;

pub fn main() -> Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Hello Pixels")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let font_data = include_bytes!("../assets/nes-arcade-font-2-1-monospaced.ttf");
    let font = Font::try_from_bytes(font_data as &[u8]).ok_or(anyhow!("Error loading font"))?;

    let mut renderer = Renderer::new(font, &window, WIDTH, HEIGHT)?;

    let bus = Rc::new(RefCell::new(Bus::new()));
    let cpu = Rc::new(RefCell::new(Cpu6502::new(Rc::downgrade(&bus))));
    let ppu = Rc::new(RefCell::new(Ppu::new(Rc::downgrade(&bus))));
    bus.borrow_mut().attach_cpu(cpu.clone());

    {
        let mut cpu_mut = cpu.borrow_mut();

        cpu_mut.load_instructions(0x1500, vec![0xF6, 0x00, 0xE8, 0x4C, 0x00, 0x15]);
        cpu_mut.reset(0x1500);
    }

    event_loop.run(move |event, target| {
        // Draw the current frame
        if let Event::WindowEvent {
            window_id: _,
            event: WindowEvent::RedrawRequested,
        } = event
        {
            renderer.clear();
            for (i, line) in bus.borrow().page_str(0).split('\n').enumerate() {
                renderer.draw_text(line, 0, 240 + (i as u32) * 20);
            }
            draw_cpu_info(&mut renderer, &cpu.borrow(), 720, 240);

            if let Err(err) = renderer.render() {
                log_error("pixels.render", err);
                target.exit();
            }
        }

        // Handle input events
        if input.update(&event) {
            // Close events
            if input.close_requested() {
                target.exit();
            }

            if input.key_pressed(KeyCode::Space) || input.key_held(KeyCode::Space) {
                cpu.borrow_mut().clock();
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = renderer.pixels().resize_surface(size.width, size.height) {
                    log_error("pixels.resize_surface", err);
                    target.exit();
                }
            }

            window.request_redraw();
        }
    })?;

    Ok(())
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}

fn draw_page_zero(renderer: &mut Renderer, bus: Rc<RefCell<Bus>>, x: u32, y: u32) {
    for i in 0..16 {
        renderer.draw_text(&format!("{:X}", i), x + 20, y + (i + 2) as u32 * 20);
    }
    for i in 0..16 {
        renderer.draw_text(&format!("{:X}", i), x + (i + 2) as u32 * 20, y + 20);
    }

    for i in 0..16 {
        for j in 0..16 {
            renderer.draw_text(
                &format!("{:02X} ", bus.borrow().cpu_read(i * 0x10 + j) as usize),
                x + (j + 2) as u32 * 20,
                y + (i + 2) as u32 * 20,
            )
        }
    }
}

fn draw_cpu_info(renderer: &mut Renderer, cpu: &Cpu6502, x: u32, y: u32) {
    renderer.draw_text("Registers:", x, y);
    renderer.draw_text(&format!("A: {:#04X}", cpu.a()), x, y + 20);
    renderer.draw_text(&format!("X: {:#04X}", cpu.x()), x, y + 40);
    renderer.draw_text(&format!("Y: {:#04X}", cpu.y()), x, y + 60);
    renderer.draw_text(&format!("Stack: {:#04X}", cpu.stkp()), x, y + 80);
    renderer.draw_text(&format!("Status: {:08b}", cpu.status().bits()), x, y + 100);
    renderer.draw_text(&format!("PC: {:#06X}", cpu.pc()), x, y + 120);

    renderer.draw_text(&format!("Cycles: {}", cpu.cycles()), x, y + 160);

    let mut addr = cpu.pc();

    for i in 0..10 {
        let instruction = Instruction::from_opcode(cpu.read(addr));
        let arg_addr = addr + 1;
        let (arg_size, arg_repr) = match instruction.address_mode {
            AddressMode::Imp => (0, String::new()),
            AddressMode::Imm => (1, format!("#${:02X}", cpu.read(arg_addr))),
            AddressMode::Zp0 => (1, format!("${:02X}", cpu.read(arg_addr))),
            AddressMode::Zpx => (1, format!("${:02X},X", cpu.read(arg_addr))),
            AddressMode::Zpy => (1, format!("${:02X},Y", cpu.read(arg_addr))),
            AddressMode::Rel => (1, format!("${:02X}", cpu.read(arg_addr))),
            AddressMode::Abs => (2, format!("${:04X}", cpu.read_u16(arg_addr))),
            AddressMode::Abx => (2, format!("${:04X},X", cpu.read_u16(arg_addr))),
            AddressMode::Aby => (2, format!("${:04X},Y", cpu.read_u16(arg_addr))),
            AddressMode::Ind => (2, format!("(${:04X})", cpu.read_u16(arg_addr))),
            AddressMode::Izx => (1, format!("(${:02X},X)", cpu.read(arg_addr))),
            AddressMode::Izy => (1, format!("(${:02X}),Y", cpu.read(arg_addr))),
        };

        renderer.draw_text(
            &format!(
                "${:4X}: {} {}",
                addr,
                instruction.instruction_type.as_ref().to_uppercase(),
                arg_repr
            ),
            x,
            y + 200 + i * 20,
        );
        addr += arg_size + 1;
    }
}
