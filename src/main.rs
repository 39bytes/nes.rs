use std::cell::RefCell;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use emu::ppu::{PatternTable, Ppu};
use error_iter::ErrorIter as _;
use log::error;
use renderer::{Renderer, Sprite};
use rusttype::Font;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use emu::bus::Bus;
use emu::cartridge::Cartridge;
use emu::cpu::Cpu6502;
use emu::instructions::{AddressMode, Instruction};
use emu::palette::Palette;

mod emu;
mod renderer;

const WIDTH: usize = 960;
const HEIGHT: usize = 720;

pub fn main() -> Result<()> {
    let nes = Rc::new(RefCell::new(Bus::new()));
    let cpu = Rc::new(RefCell::new(Cpu6502::new(Rc::downgrade(&nes))));

    let args: Vec<_> = env::args().collect();

    if args.len() <= 1 {
        println!("Usage: {} <rom path>", args[0]);
        process::exit(1);
    }

    let rom_path = &args[1];
    let cartridge = Rc::new(RefCell::new(Cartridge::new(rom_path)?));
    nes.borrow_mut().attach_cartridge(cartridge);

    {
        cpu.borrow_mut().reset_to(0xC000);
    }

    let file = File::create("log.txt")?;
    let mut file = BufWriter::new(file);

    let mut instruction_count = 0;

    let mut c = cpu.borrow_mut();

    while instruction_count < 60000 || c.pc() != 0xC66E {
        let line = format!(
            "{:04X} {:02X} {:31} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{}",
            c.pc(),
            c.read(c.pc()),
            get_instruction_repr(&c, c.pc()),
            c.a(),
            c.x(),
            c.y(),
            c.status().bits(),
            c.stkp(),
            c.total_cycles() + c.cycles() as u64
        );
        println!("{}", line);
        writeln!(&mut file, "{}", line)?;

        c.next_instruction();

        instruction_count += 1;
    }

    Ok(())
}

// pub fn main() -> Result<()> {
//     env_logger::init();
//     let event_loop = EventLoop::new().unwrap();
//     let mut input = WinitInputHelper::new();
//     let window = {
//         let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
//         WindowBuilder::new()
//             .with_title("Hello Pixels")
//             .with_inner_size(size)
//             .with_min_inner_size(size)
//             .build(&event_loop)
//             .unwrap()
//     };
//
//     let args: Vec<_> = env::args().collect();
//
//     if args.len() <= 1 {
//         println!("Usage: {} <rom path>", args[0]);
//         process::exit(1);
//     }
//
//     let rom_path = &args[1];
//
//     let font_data = include_bytes!("../assets/fonts/nes-arcade-font-2-1-monospaced.ttf");
//     let font = Font::try_from_bytes(font_data as &[u8]).ok_or(anyhow!("Error loading font"))?;
//
//     let palette = Palette::load("assets/palettes/2C02G.pal")?;
//
//     let mut renderer = Renderer::new(font, &window, WIDTH, HEIGHT)?;
//
//     let nes = Rc::new(RefCell::new(Bus::new()));
//     let cpu = Rc::new(RefCell::new(Cpu6502::new(Rc::downgrade(&nes))));
//     let ppu = Rc::new(RefCell::new(Ppu::new(Rc::downgrade(&nes), palette.clone())));
//     nes.borrow_mut().attach_cpu(cpu.clone());
//     nes.borrow_mut().attach_ppu(ppu.clone());
//
//     let cartridge = Rc::new(RefCell::new(Cartridge::new(rom_path)?));
//     nes.borrow_mut().attach_cartridge(cartridge);
//
//     cpu.borrow_mut().reset_to(0xC000);
//
//     let palette_sprite: Sprite = palette.into();
//     let palette_sprite = palette_sprite.scale(16);
//
//     let mut showing_page = 0;
//
//     event_loop.run(move |event, target| {
//         // Draw the current frame
//         if let Event::WindowEvent {
//             window_id: _,
//             event: WindowEvent::RedrawRequested,
//         } = event
//         {
//             renderer.clear();
//
//             let page_start = (showing_page as u16) * 0x100;
//             let page_end = page_start + 0xFF;
//
//             renderer.draw_text(&format!("{:#06X}-{:#06X}", page_start, page_end), 0, 320);
//             for (i, line) in nes.borrow().page_str(showing_page).split('\n').enumerate() {
//                 renderer.draw_text(line, 0, 360 + i * 20);
//             }
//             renderer.draw_sprite(
//                 &ppu.borrow().get_pattern_table(PatternTable::Left),
//                 120,
//                 120,
//             );
//             renderer.draw_sprite(&palette_sprite, 720, 640);
//             draw_cpu_info(&mut renderer, &cpu.borrow(), 720, 240);
//
//             if let Err(err) = renderer.render() {
//                 log_error("pixels.render", err);
//                 target.exit();
//             }
//         }
//
//         // Handle input events
//         if input.update(&event) {
//             // Close events
//             if input.close_requested() {
//                 target.exit();
//             }
//
//             if input.key_pressed(KeyCode::Space) || input.key_held(KeyCode::Space) {
//                 cpu.borrow_mut().clock();
//             } else if input.key_pressed(KeyCode::KeyN) {
//                 cpu.borrow_mut().next_instruction();
//             } else if input.key_pressed(KeyCode::KeyV) {
//                 showing_page = showing_page.wrapping_sub(1);
//             } else if input.key_pressed(KeyCode::KeyB) {
//                 showing_page = showing_page.wrapping_add(1);
//             }
//
//             // Resize the window
//             if let Some(size) = input.window_resized() {
//                 if let Err(err) = renderer.pixels().resize_surface(size.width, size.height) {
//                     log_error("pixels.resize_surface", err);
//                     target.exit();
//                 }
//             }
//
//             window.request_redraw();
//         }
//     })?;
//
//     Ok(())
// }
//
// fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
//     error!("{method_name}() failed: {err}");
//     for source in err.sources().skip(1) {
//         error!("  Caused by: {source}");
//     }
// }
//
fn draw_cpu_info(renderer: &mut Renderer, cpu: &Cpu6502, x: usize, y: usize) {
    renderer.draw_text("Registers:", x, y);
    renderer.draw_text(&format!("A: {:#04X}", cpu.a()), x, y + 20);
    renderer.draw_text(&format!("X: {:#04X}", cpu.x()), x, y + 40);
    renderer.draw_text(&format!("Y: {:#04X}", cpu.y()), x, y + 60);
    renderer.draw_text(&format!("Stack: {:#04X}", cpu.stkp()), x, y + 80);
    renderer.draw_text(&format!("Status: {:08b}", cpu.status().bits()), x, y + 100);
    renderer.draw_text("        NVUBDIZC", x, y + 120);
    renderer.draw_text(&format!("PC: {:#06X}", cpu.pc()), x, y + 140);

    renderer.draw_text(&format!("Cycles: {}", cpu.cycles()), x, y + 160);
    renderer.draw_text(&format!("Total cycles: {}", cpu.total_cycles()), x, y + 180);

    let mut addr = cpu.pc();

    for i in 0..10 {
        let instruction = Instruction::lookup(cpu.read(addr));
        let instruction_repr = get_instruction_repr(cpu, addr);

        renderer.draw_text(
            &format!("${:4X}: {}", addr, instruction_repr),
            x,
            y + 200 + i * 20,
        );
        addr += instruction.address_mode.arg_size() + 1;
    }
}

fn get_instruction_repr(cpu: &Cpu6502, addr: u16) -> String {
    let instruction = Instruction::lookup(cpu.read(addr));
    let arg_addr = addr + 1;

    let name = instruction.instruction_type.as_ref().to_uppercase();

    match instruction.address_mode {
        AddressMode::Imp => name,
        AddressMode::Acc => format!("{} A", name),
        AddressMode::Imm => format!("{} #${:02X}", name, cpu.read(arg_addr)),
        AddressMode::Zp0 => format!(
            "{} ${:02X} = {:02X}",
            name,
            cpu.read(arg_addr),
            cpu.read(cpu.read(arg_addr) as u16)
        ),
        AddressMode::Zpx => format!("{} ${:02X},X", name, cpu.read(arg_addr)),
        AddressMode::Zpy => format!("{} ${:02X},Y", name, cpu.read(arg_addr)),
        AddressMode::Rel => {
            let offset = cpu.read(arg_addr) as i8;
            let computed_addr = if offset < 0 {
                addr - (offset.unsigned_abs() as u16)
            } else {
                addr + offset as u16
            };
            format!("{} ${:02X}", name, computed_addr + 2)
        }
        AddressMode::Abs => format!("{} ${:04X}", name, cpu.read_u16(arg_addr)),
        AddressMode::Abx => format!("{} ${:04X},X", name, cpu.read_u16(arg_addr)),
        AddressMode::Aby => format!("{} ${:04X},Y", name, cpu.read_u16(arg_addr)),
        AddressMode::Ind => format!("{} (${:04X})", name, cpu.read_u16(arg_addr)),
        AddressMode::Izx => {
            let ptr = cpu.read(arg_addr).wrapping_add(cpu.x());

            let lo = cpu.read(ptr as u16) as u16;
            let hi = cpu.read(ptr.wrapping_add(1) as u16) as u16;

            let resolved = (hi << 8) | lo;
            format!(
                "{} (${:02X},X) @ {:02X} = {:04X} = {:02X}",
                name,
                cpu.read(arg_addr),
                ptr,
                resolved,
                cpu.read(resolved)
            )
        }
        AddressMode::Izy => format!("{} (${:02X}),Y", name, cpu.read(arg_addr)),
    }
}
