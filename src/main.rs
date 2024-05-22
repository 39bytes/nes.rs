use std::env;
use std::process;

use anyhow::{anyhow, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SizedSample,
};
use error_iter::ErrorIter as _;
use log::error;
use renderer::{Color, Renderer, Sprite};
use rusttype::Font;
use utils::FpsCounter;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use emu::cartridge::Cartridge;
use emu::cpu::instructions::{AddressMode, Instruction};
use emu::cpu::Cpu;
use emu::input::{ControllerButtons, ControllerInput};
use emu::nes::Nes;
use emu::palette::Palette;
use emu::ppu::PatternTable;
use emu::ppu::Ppu;

mod emu;
mod renderer;
mod utils;

const WIDTH: usize = 960;
const HEIGHT: usize = 720;

const CLOCK_SPEED: u32 = 5369318;
const FRAME_CLOCKS: u32 = CLOCK_SPEED / 60;

pub fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<_> = env::args().collect();
    if args.len() <= 1 {
        println!("Usage: {} <rom path>", args[0]);
        process::exit(1);
    }

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("nes.rs")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    // let (device, config) = setup_audio()?;
    //
    // match config.sample_format() {
    //     cpal::SampleFormat::F32 => run::<f32>(&device, &config.into()),
    //     sample_format => panic!("Unsupported sample format {}", sample_format),
    // }?;

    let rom_path = &args[1];

    let font_data = include_bytes!("../assets/fonts/nes-arcade-font-2-1-monospaced.ttf");
    let font = Font::try_from_bytes(font_data as &[u8]).ok_or(anyhow!("Error loading font"))?;

    let palette = Palette::load("assets/palettes/2C02G.pal")?;

    let mut renderer = Renderer::new(font, &window, WIDTH, HEIGHT)?;

    let mut nes = Nes::new(palette.clone());
    let cartridge = Cartridge::new(rom_path)?;
    nes.load_cartridge(cartridge);
    nes.reset();

    let mut displayed_page: u8 = 0;
    let mut paused = true;

    let mut fps_counter = FpsCounter::new();

    event_loop.run(move |event, target| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("Close button pressed, exiting");
                target.exit();
            }
            Event::AboutToWait => {
                if !paused {
                    for _ in 0..FRAME_CLOCKS {
                        nes.clock();
                    }
                }

                fps_counter.tick();

                renderer.clear();
                renderer.draw_text(&format!("FPS: {}", fps_counter.get_fps().round()), 0, 204);

                draw_ppu_info(&mut renderer, &nes.ppu(), 0, 0);
                draw_palettes(&mut renderer, &nes.ppu(), 240, 0);
                draw_pattern_tables(&mut renderer, &nes.ppu(), 596, 0);

                let screen = nes.screen().clone();
                renderer.draw_sprite(&screen.scale(2), 0, 224);

                // draw_mem_page(&mut renderer, &nes, displayed_page, 0, 320);
                // draw_nametable(&mut renderer, &nes.ppu(), 0, 0);
                draw_cpu_info(&mut renderer, &nes, 720, 240);

                if let Err(err) = renderer.render() {
                    log_error("pixels.render", err);
                    target.exit();
                }
            }

            _ => (),
        }

        // Handle input events
        if input.update(&event) {
            // Emulator meta events
            if input.key_pressed(KeyCode::Space) {
                paused = !paused;
            } else if input.key_pressed(KeyCode::KeyN) {
                nes.next_instruction();
            } else if input.key_pressed(KeyCode::KeyV) {
                displayed_page = displayed_page.wrapping_sub(1);
            } else if input.key_pressed(KeyCode::KeyB) {
                displayed_page = displayed_page.wrapping_add(1);
            }

            // Console input
            let mut buttons = ControllerButtons::empty();
            if input.key_held(KeyCode::KeyX) {
                buttons.insert(ControllerButtons::A);
            }
            if input.key_held(KeyCode::KeyZ) {
                buttons.insert(ControllerButtons::B);
            }
            if input.key_held(KeyCode::KeyA) {
                buttons.insert(ControllerButtons::Select);
            }
            if input.key_held(KeyCode::KeyS) {
                buttons.insert(ControllerButtons::Start);
            }
            if input.key_held(KeyCode::ArrowUp) {
                buttons.insert(ControllerButtons::Up);
            }
            if input.key_held(KeyCode::ArrowDown) {
                buttons.insert(ControllerButtons::Down);
            }
            if input.key_held(KeyCode::ArrowLeft) {
                buttons.insert(ControllerButtons::Left);
            }
            if input.key_held(KeyCode::ArrowRight) {
                buttons.insert(ControllerButtons::Right);
            }
            nes.trigger_inputs(ControllerInput::One(buttons));

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = renderer.pixels().resize_surface(size.width, size.height) {
                    log_error("pixels.resize_surface", err);
                    target.exit();
                }
            }
        }
    })?;

    Ok(())
}

fn setup_audio() -> Result<(cpal::Device, cpal::SupportedStreamConfig)> {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("Failed to find output device");
    log::info!("Output device: {}", device.name()?);

    let config = device.default_output_config()?;

    log::info!("Sample format: {}", config.sample_format());
    log::info!("Sample rate: {}", config.sample_rate().0);

    Ok((device, config))
}

pub fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig) -> Result<()>
where
    T: SizedSample + FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    // Produce a sinusoid of maximum amplitude.
    let mut sample_clock = 0f32;
    let mut next_value = move || {
        sample_clock = (sample_clock + 1.0) % sample_rate;
        (sample_clock * 440.0 * 2.0 * std::f32::consts::PI / sample_rate).sin()
    };

    let err_fn = |err| log::error!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            write_data(data, channels, &mut next_value)
        },
        err_fn,
        None,
    )?;
    stream.play()?;

    std::thread::sleep(std::time::Duration::from_millis(1000));

    Ok(())
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> f32)
where
    T: Sample + FromSample<f32>,
{
    for frame in output.chunks_mut(channels) {
        let value: T = T::from_sample(next_sample());
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
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

fn draw_flags(renderer: &mut Renderer, flags: u8, text: &str, x: usize, y: usize) {
    renderer.draw_text_with_computed_color(text, x, y, |i| {
        if flags & (1 << (7 - i)) != 0 {
            Color::WHITE
        } else {
            Color::GRAY
        }
    });
}

fn draw_cpu_info(renderer: &mut Renderer, nes: &Nes, x: usize, y: usize) {
    let cpu = nes.cpu();

    renderer.draw_text("CPU Registers:", x, y);
    renderer.draw_text(&format!("A: {:#04X}", cpu.a()), x, y + 20);
    renderer.draw_text(&format!("X: {:#04X}", cpu.x()), x, y + 40);
    renderer.draw_text(&format!("Y: {:#04X}", cpu.y()), x, y + 60);
    renderer.draw_text(&format!("Stack: {:#04X}", cpu.stkp()), x, y + 80);
    renderer.draw_text("Status:", x, y + 100);

    draw_flags(renderer, cpu.status().bits(), "NVUBDIZC", x + 96, y + 100);
    renderer.draw_text(&format!("PC: {:#06X}", cpu.pc()), x, y + 140);

    renderer.draw_text(&format!("Cycles: {}", cpu.cycles()), x, y + 160);

    let mut addr = cpu.pc();

    for i in 0..10 {
        let instruction = Instruction::lookup(cpu.read_debug(addr));
        let instruction_repr = get_instruction_repr(&cpu, addr);

        renderer.draw_text(
            &format!("${:4X}: {}", addr, instruction_repr),
            x,
            y + 200 + i * 20,
        );
        addr += instruction.address_mode.arg_size() + 1;
    }
}

fn get_instruction_repr(cpu: &Cpu, addr: u16) -> String {
    let instruction = Instruction::lookup(cpu.read_debug(addr));
    let arg_addr = addr + 1;

    let name = instruction.instruction_type.as_ref().to_uppercase();

    match instruction.address_mode {
        AddressMode::Imp => name,
        AddressMode::Acc => format!("{} A", name),
        AddressMode::Imm => format!("{} #${:02X}", name, cpu.read_debug(arg_addr)),
        AddressMode::Zp0 => format!("{} ${:02X}", name, cpu.read_debug(arg_addr),),
        AddressMode::Zpx => format!("{} ${:02X},X", name, cpu.read_debug(arg_addr)),
        AddressMode::Zpy => format!("{} ${:02X},Y", name, cpu.read_debug(arg_addr)),
        AddressMode::Rel => {
            let offset = cpu.read_debug(arg_addr) as i8;
            let computed_addr = if offset < 0 {
                addr - (offset.unsigned_abs() as u16)
            } else {
                addr + offset as u16
            };
            format!("{} ${:02X}", name, computed_addr + 2)
        }
        AddressMode::Abs => format!("{} ${:04X}", name, cpu.read_debug_u16(arg_addr)),
        AddressMode::Abx => format!("{} ${:04X},X", name, cpu.read_debug_u16(arg_addr)),
        AddressMode::Aby => format!("{} ${:04X},Y", name, cpu.read_debug_u16(arg_addr)),
        AddressMode::Ind => format!("{} (${:04X})", name, cpu.read_debug_u16(arg_addr)),
        AddressMode::Izx => format!("{} (${:02X},X)", name, cpu.read_debug(arg_addr)),
        AddressMode::Izy => format!("{} (${:02X}),Y", name, cpu.read_debug(arg_addr)),
    }
}

fn draw_ppu_info(renderer: &mut Renderer, ppu: &Ppu, x: usize, y: usize) {
    renderer.draw_text("PPU Registers:", x, y);
    renderer.draw_text("CTRL: ", x, y + 20);
    draw_flags(renderer, ppu.ctrl().bits(), "VPHBSINN", x + 80, y + 20);

    renderer.draw_text("MASK: ", x, y + 40);
    draw_flags(renderer, ppu.mask().bits(), "BGRsbMmG", x + 80, y + 40);

    renderer.draw_text("STATUS: ", x, y + 60);
    draw_flags(renderer, ppu.status().bits(), "VSO-----", x + 96, y + 60);

    renderer.draw_text(&format!("OAMADDR: {:#06X}", ppu.oam_addr()), x, y + 80);
    renderer.draw_text(&format!("ADDR: {:#06X}", ppu.addr()), x, y + 100);
    renderer.draw_text(&format!("DATA: {:#06X}", ppu.data()), x, y + 120);
}

fn draw_pattern_tables(renderer: &mut Renderer, ppu: &Ppu, x: usize, y: usize) {
    let left_pattern_table = ppu.get_pattern_table(PatternTable::Left);
    let right_pattern_table = ppu.get_pattern_table(PatternTable::Right);

    renderer.draw_text("Pattern Tables", x, y);
    renderer.draw_sprite(&left_pattern_table, x, y + 24);
    renderer.draw_sprite(&right_pattern_table, x + 144, y + 24);
}

fn palette_sprite(ppu: &Ppu, palette_index: u8) -> Sprite {
    let bg_color = ppu.get_palette_color(palette_index, 0);
    let color1 = ppu.get_palette_color(palette_index, 1);
    let color2 = ppu.get_palette_color(palette_index, 2);
    let color3 = ppu.get_palette_color(palette_index, 3);

    Sprite::new(vec![bg_color, color1, color2, color3], 4, 1)
        .unwrap()
        .scale(16)
}

fn draw_palettes(renderer: &mut Renderer, ppu: &Ppu, x: usize, y: usize) {
    renderer.draw_text("Background", x, y);
    for i in 0..4 {
        renderer.draw_sprite(
            &palette_sprite(ppu, i),
            x + 72 * (i as usize % 2),
            y + 24 * (i as usize / 2) + 24,
        );
    }

    renderer.draw_text("Sprites", x + 160, y);
    for i in 4..8 {
        let sprite = palette_sprite(ppu, i);
        let i = i - 4;
        renderer.draw_sprite(
            &sprite,
            x + 72 * (i as usize % 2) + 160,
            y + 24 * (i as usize / 2) + 24,
        );
    }
}

fn draw_nametable(renderer: &mut Renderer, ppu: &Ppu, x: usize, y: usize) {
    const ATTRIBUTE_MEMORY_OFFSET: u16 = 0x2000 + 0x03C0;

    for i in 0..30 {
        for j in 0..32 {
            let group_offset = (i / 4) * 8 + j / 4;
            let mut palette_byte = ppu.read(ATTRIBUTE_MEMORY_OFFSET + group_offset);
            if i % 4 >= 2 {
                palette_byte >>= 4;
            }
            if j % 4 >= 2 {
                palette_byte >>= 2;
            }
            let palette = palette_byte & 0x03;

            renderer.draw_text(
                &format!("{:02X}", palette),
                (j as usize) * 24 + x,
                (i as usize) * 16 + y,
            );
        }
    }
}
