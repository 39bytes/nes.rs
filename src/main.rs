use std::env;
use std::process;

use anyhow::{anyhow, Result};
use emu::ppu::PatternTable;
use error_iter::ErrorIter as _;
use log::error;
use renderer::Renderer;
use renderer::Sprite;
use rusttype::Font;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use emu::cartridge::Cartridge;
use emu::cpu::Cpu6502;
use emu::instructions::{AddressMode, Instruction};
use emu::nes::Nes;
use emu::palette::Palette;

mod emu;
mod renderer;

const WIDTH: usize = 960;
const HEIGHT: usize = 720;

const CLOCK_SPEED: u32 = 5361818;
const FRAME_CLOCKS: u32 = CLOCK_SPEED / 60;

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

    let args: Vec<_> = env::args().collect();

    if args.len() <= 1 {
        println!("Usage: {} <rom path>", args[0]);
        process::exit(1);
    }

    let rom_path = &args[1];

    let font_data = include_bytes!("../assets/fonts/nes-arcade-font-2-1-monospaced.ttf");
    let font = Font::try_from_bytes(font_data as &[u8]).ok_or(anyhow!("Error loading font"))?;

    let palette = Palette::load("assets/palettes/2C02G.pal")?;

    let mut renderer = Renderer::new(font, &window, WIDTH, HEIGHT)?;

    let mut nes = Nes::new(palette.clone());
    let cartridge = Cartridge::new(rom_path)?;
    nes.load_cartridge(cartridge)?;
    nes.reset();

    let palette_sprite: Sprite = Sprite::from(palette).scale(16);

    let mut displayed_page: u8 = 0;
    let mut last_time = 0;

    event_loop.run(move |event, target| {
        // Draw the current frame
        if let Event::WindowEvent {
            window_id: _,
            event: WindowEvent::RedrawRequested,
        } = event
        {
            renderer.clear();

            renderer.draw_sprite(&palette_sprite, 0, 0);
            let left_pattern_table = nes.ppu().get_pattern_table(PatternTable::Left);
            let right_pattern_table = nes.ppu().get_pattern_table(PatternTable::Right);
            renderer.draw_sprite(&left_pattern_table, 0, 64);
            renderer.draw_sprite(&right_pattern_table, 160, 64);
            for i in 0..8 {
                let bg_color = nes.ppu().get_palette_color(i, 0);
                let color1 = nes.ppu().get_palette_color(i, 1);
                let color2 = nes.ppu().get_palette_color(i, 2);
                let color3 = nes.ppu().get_palette_color(i, 3);
                let sprite = Sprite::new(vec![bg_color, color1, color2, color3], 4, 1)
                    .unwrap()
                    .scale(16);
                renderer.draw_sprite(&sprite, 0, (i as usize) * 16 + 224);
            }

            // draw_mem_page(&mut renderer, &nes, displayed_page, 0, 320);
            draw_cpu_info(&mut renderer, &nes, 720, 240);

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
                for _ in 0..FRAME_CLOCKS {
                    nes.clock();
                }
            } else if input.key_pressed(KeyCode::KeyN) {
                nes.next_instruction();
            } else if input.key_pressed(KeyCode::KeyV) {
                displayed_page = displayed_page.wrapping_sub(1);
            } else if input.key_pressed(KeyCode::KeyB) {
                displayed_page = displayed_page.wrapping_add(1);
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

fn draw_mem_page(renderer: &mut Renderer, nes: &Nes, page: u8, x: usize, y: usize) {
    let page_start = (page as u16) * 0x100;
    let page_end = page_start + 0xFF;

    renderer.draw_text(&format!("{:#06X}-{:#06X}", page_start, page_end), x, y);
    for (i, line) in nes.cpu_mem_page_str(page).split('\n').enumerate() {
        renderer.draw_text(line, x, y + 40 + i * 20);
    }
}

fn draw_cpu_info(renderer: &mut Renderer, nes: &Nes, x: usize, y: usize) {
    let cpu = nes.cpu();

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
        let instruction_repr = get_instruction_repr(&cpu, addr);

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
        AddressMode::Zp0 => format!("{} ${:02X}", name, cpu.read(arg_addr),),
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
        AddressMode::Izx => format!("{} (${:02X},X)", name, cpu.read(arg_addr)),
        AddressMode::Izy => format!("{} (${:02X}),Y", name, cpu.read(arg_addr)),
    }
}
