use emu_state::EmuState;
use std::{
    path::PathBuf,
    thread,
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
mod emu_state;
mod extension_traits;
mod renderer;
#[allow(dead_code)]
mod ui;
mod utils;

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

    let (width, height, scale) = if args.draw_debug_info {
        (900, 720, 1)
    } else {
        (256, 240, 2)
    };
    let mut event_pump = sdl_context.event_pump().unwrap();

    let (mut nes, mut emu_state) = setup_emulator(&args, &sdl_context)?;
    let mut renderer = Renderer::new(&sdl_context, width, height, scale)?;
    let mut fps_counter = FpsCounter::new();

    let frame_time_duration = Duration::from_secs_f64(FRAME_TIME);

    let mut acc = 0.0;
    let mut now = Instant::now();

    nes.reset();
    'running: loop {
        let frame_begin = Instant::now();
        let key_state = event_pump.keyboard_state();
        let shift_pressed = key_state.is_scancode_pressed(Scancode::LShift)
            || key_state.is_scancode_pressed(Scancode::RShift);

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => match key {
                    Keycode::Space => {
                        if emu_state.paused() {
                            nes.unpause();
                        }
                        emu_state.toggle_pause();
                    }
                    Keycode::N if emu_state.paused() => {
                        nes.next_instruction();
                    }
                    Keycode::M if emu_state.paused() => {
                        nes.advance_frame();
                    }
                    Keycode::P => {
                        emu_state.next_palette();
                    }
                    Keycode::Num1 => {
                        if !shift_pressed {
                            if let Some(state) = emu_state.save_state(1) {
                                log::info!("Loading state 1");
                                nes.load_state(state);
                            }
                        } else {
                            log::info!("Writing state 1");
                            emu_state.write_save_state(1, nes.state());
                        }
                    }
                    Keycode::Num2 => {
                        if !shift_pressed {
                            if let Some(state) = emu_state.save_state(2) {
                                log::info!("Loading state 2");
                                nes.load_state(state);
                            }
                        } else {
                            log::info!("Writing state 2");
                            emu_state.write_save_state(2, nes.state());
                        }
                    }
                    Keycode::Num3 => {
                        if !shift_pressed {
                            if let Some(state) = emu_state.save_state(3) {
                                log::info!("Loading state 3");
                                nes.load_state(state);
                            }
                        } else {
                            log::info!("Writing state 3");
                            emu_state.write_save_state(3, nes.state());
                        }
                    }
                    Keycode::Num4 => {
                        if !shift_pressed {
                            if let Some(state) = emu_state.save_state(4) {
                                log::info!("Loading state 4");
                                nes.load_state(state);
                            }
                        } else {
                            log::info!("Writing state 4");
                            emu_state.write_save_state(4, nes.state());
                        }
                    }
                    Keycode::Num5 => {
                        if !shift_pressed {
                            if let Some(state) = emu_state.save_state(5) {
                                log::info!("Loading state 5");
                                nes.load_state(state);
                            }
                        } else {
                            log::info!("Writing state 5");
                            emu_state.write_save_state(5, nes.state());
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        handle_input(&mut event_pump, &mut nes);

        // The rest of the game loop goes here...
        acc += now.elapsed().as_secs_f64();
        now = Instant::now();
        let mut frame_ticked = false;
        while acc >= FRAME_TIME {
            let before_emu_frame = Instant::now();
            if !emu_state.paused() {
                let should_pause = nes.advance_frame();
                if should_pause {
                    emu_state.set_paused(true);
                }
                fps_counter.tick();
                log::debug!(
                    "Frame time: {}ms",
                    before_emu_frame.elapsed().as_secs_f64() * 1000.0
                );
            }
            acc -= FRAME_TIME;
            frame_ticked = true;
        }

        if frame_ticked || emu_state.paused() {
            // TODO: Implement not drawing overscan
            // https://www.nesdev.org/wiki/Overscan
            let before_render = Instant::now();
            renderer.clear();
            if args.draw_debug_info {
                draw_with_debug_info(&mut renderer, &nes, &fps_counter, &emu_state)
            } else {
                renderer.draw(nes.screen(), 0, 0);
            }
            renderer.render();
            log::debug!(
                "Render time: {}ms",
                before_render.elapsed().as_secs_f64() * 1000.0
            );
        }

        let rem = frame_begin.elapsed();
        if rem < frame_time_duration {
            thread::sleep(frame_time_duration - rem);
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

fn setup_emulator(args: &Args, sdl_context: &sdl2::Sdl) -> Result<(Nes, EmuState)> {
    // Emulator setup
    let palette = Palette::default();
    let cartridge = Cartridge::new(args.rom_path.as_path())?;

    let mut nes = Nes::new(palette.clone());
    if !args.disable_audio {
        nes.with_audio(AudioOutput::new(sdl_context)?)
    }
    let emu_state = EmuState::new(&cartridge);

    nes.load_cartridge(cartridge);

    Ok((nes, emu_state))
}

fn draw_with_debug_info(
    renderer: &mut Renderer,
    nes: &Nes,
    fps_counter: &FpsCounter,
    emu_state: &EmuState,
) {
    renderer.draw(&nes.screen().scale(2), 0, 180);

    renderer.draw_text(&format!("FPS: {:.1}", fps_counter.get_fps()), 0, 160);
    if emu_state.paused() {
        renderer.draw_text("(Paused)", 120, 160);
    }

    draw_ppu_info(renderer, &nes.ppu(), 0, 0);
    draw_palettes(renderer, &nes.ppu(), 240, 0);
    draw_pattern_tables(
        renderer,
        &nes.ppu(),
        emu_state.pattern_table_palette(),
        576,
        0,
    );
    draw_cpu_info(renderer, nes, 576, 180);
    // draw_oam_sprites(renderer, &nes.ppu(), 600, 320)
}
