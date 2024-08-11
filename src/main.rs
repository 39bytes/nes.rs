use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use ui::{draw_cpu_info, draw_palettes, draw_pattern_tables, draw_ppu_info};
use utils::FpsCounter;

use anyhow::Result;
use audio_output::AudioOutput;
use renderer::Renderer;
use sdl2::keyboard::Keycode;
use sdl2::{event::Event, keyboard::Scancode};

use emu::{
    cartridge::Cartridge,
    consts::FRAME_TIME,
    input::{ControllerButtons, ControllerInput},
    nes::Nes,
    palette::Palette,
};

use clap::{arg, Parser};

mod audio_output;
mod emu;
mod renderer;
#[allow(dead_code)]
mod ui;
mod utils;

trait Toggle {
    fn toggle(&self);
}

impl Toggle for Arc<AtomicBool> {
    fn toggle(&self) {
        self.store(!self.load(Ordering::Relaxed), Ordering::Relaxed);
    }
}

#[derive(Parser)]
struct Args {
    rom_path: PathBuf,

    #[arg(long, action)]
    disable_audio: bool,

    #[arg(long, action)]
    draw_debug_info: bool,
}

pub fn main() -> Result<()> {
    env_logger::builder().format_timestamp_millis().init();

    let args = Args::parse();

    // SDL2 setup
    let sdl_context = sdl2::init().unwrap();

    let (width, height) = if args.draw_debug_info {
        (900, 720)
    } else {
        (512, 480)
    };
    let mut event_pump = sdl_context.event_pump().unwrap();

    let (mut nes, paused) = setup_emulator(&args, &sdl_context)?;
    let mut renderer = Renderer::new(&sdl_context, width, height)?;
    let mut fps_counter = FpsCounter::new();

    let mut acc = 0.0;
    let mut now = Instant::now();

    'running: loop {
        renderer.clear();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => match key {
                    Keycode::Space => paused.toggle(),
                    Keycode::N if paused.load(Ordering::Relaxed) => nes.next_instruction(),
                    _ => {}
                },
                _ => {}
            }
        }
        handle_input(&mut event_pump, &mut nes);

        // The rest of the game loop goes here...
        acc += now.elapsed().as_secs_f64();
        now = Instant::now();
        while acc >= FRAME_TIME {
            fps_counter.tick();
            nes.advance_frame();
            acc -= FRAME_TIME;

            // TODO: Implement not drawing overscan
            // https://www.nesdev.org/wiki/Overscan
            if args.draw_debug_info {
                draw_with_debug_info(&mut renderer, &nes, &fps_counter)
            } else {
                renderer.draw_scaled(nes.screen(), 0, 0, 2);
            }
            renderer.render();
        }
    }

    Ok(())
}

fn handle_input(event_pump: &mut sdl2::EventPump, nes: &mut Nes) {
    let mut buttons = ControllerButtons::empty();
    let key_state = event_pump.keyboard_state();
    if key_state.is_scancode_pressed(Scancode::X) {
        buttons.insert(ControllerButtons::A);
    }
    if key_state.is_scancode_pressed(Scancode::Z) {
        buttons.insert(ControllerButtons::B);
    }
    if key_state.is_scancode_pressed(Scancode::A) {
        buttons.insert(ControllerButtons::Select);
    }
    if key_state.is_scancode_pressed(Scancode::S) {
        buttons.insert(ControllerButtons::Start);
    }
    if key_state.is_scancode_pressed(Scancode::Up) {
        buttons.insert(ControllerButtons::Up);
    }
    if key_state.is_scancode_pressed(Scancode::Down) {
        buttons.insert(ControllerButtons::Down);
    }
    if key_state.is_scancode_pressed(Scancode::Left) {
        buttons.insert(ControllerButtons::Left);
    }
    if key_state.is_scancode_pressed(Scancode::Right) {
        buttons.insert(ControllerButtons::Right);
    }
    nes.trigger_inputs(ControllerInput::One(buttons));
}

fn setup_emulator(args: &Args, sdl_context: &sdl2::Sdl) -> Result<(Nes, Arc<AtomicBool>)> {
    // Emulator setup
    let palette = Palette::load("assets/palettes/2C02G.pal")?;
    let cartridge = Cartridge::new(args.rom_path.as_path())?;

    let paused = Arc::new(AtomicBool::new(false));

    let mut nes = Nes::new(palette.clone());
    if !args.disable_audio {
        nes.with_audio(AudioOutput::new(sdl_context)?)
    }

    // Start
    nes.load_cartridge(cartridge);
    nes.reset();

    Ok((nes, paused))
}

fn draw_with_debug_info(renderer: &mut Renderer, nes: &Nes, fps_counter: &FpsCounter) {
    renderer.draw_scaled(nes.screen(), 0, 180, 2);

    renderer.draw_text(&format!("FPS: {:.1}", fps_counter.get_fps()), 0, 160);

    draw_ppu_info(renderer, &nes.ppu(), 0, 0);
    draw_palettes(renderer, &nes.ppu(), 240, 0);
    draw_pattern_tables(renderer, &nes.ppu(), 576, 0);
    draw_cpu_info(renderer, nes, 576, 180);
}
