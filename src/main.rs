use cpal::StreamConfig;
use ringbuf::traits::*;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Instant,
};
use ui::{draw_cpu_info, draw_palettes, draw_pattern_tables, draw_ppu_info};
use utils::FpsCounter;

use anyhow::{anyhow, Result};
use audio_output::AudioBufferConsumer;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, SizedSample,
};
use error_iter::ErrorIter as _;
use log::error;
use renderer::Renderer;
use rusttype::Font;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::KeyCode,
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

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

    // Window setup
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut input = WinitInputHelper::new();

    let (width, height) = if args.draw_debug_info {
        (900, 720)
    } else {
        (256, 240)
    };

    let window = {
        let size = LogicalSize::new(width, height);
        WindowBuilder::new()
            .with_title("nes.rs")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let (mut nes, paused) = setup_emulator(&args)?;
    nes.set_breakpoint(0x0029);

    let mut acc = 0.0;
    let mut now = Instant::now();

    let font_data = include_bytes!("../assets/fonts/nes-arcade-font-2-1-monospaced.ttf");
    let font = Font::try_from_bytes(font_data as &[u8]).ok_or(anyhow!("Error loading font"))?;

    let mut renderer = Renderer::new(font, &window, width as usize, height as usize)?;
    let mut fps_counter = FpsCounter::new();
    let mut pattern_table_palette = 0;

    event_loop.run(move |event, target| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                log::info!("Close button pressed, exiting");
                target.exit();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                renderer.clear();
                // TODO: Implement not drawing overscan
                // https://www.nesdev.org/wiki/Overscan
                if args.draw_debug_info {
                    draw_with_debug_info(&mut renderer, &nes, &fps_counter, pattern_table_palette);
                } else {
                    renderer.draw(nes.screen(), 0, 0);
                }

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
                let cur = paused.load(Ordering::Relaxed);
                if cur {
                    nes.unpause();
                }
                paused.store(!cur, Ordering::Relaxed);

                now = Instant::now();
            }
            if input.key_pressed(KeyCode::KeyN) && paused.load(Ordering::Relaxed) {
                nes.next_instruction();
            }
            if input.key_pressed(KeyCode::KeyM) && paused.load(Ordering::Relaxed) {
                nes.advance_frame();
            }
            if input.key_pressed(KeyCode::KeyP) {
                pattern_table_palette = (pattern_table_palette + 1) % 8;
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

            if !paused.load(Ordering::Relaxed) {
                acc += now.elapsed().as_secs_f64();
                now = Instant::now();
                while acc >= FRAME_TIME {
                    fps_counter.tick();
                    let should_pause = nes.advance_frame();
                    if should_pause {
                        paused.store(true, Ordering::Relaxed);
                    }
                    acc -= FRAME_TIME;
                }
            }
            window.request_redraw();
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

fn start_audio<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut consumer: AudioBufferConsumer,
    paused: Arc<AtomicBool>,
) -> Result<()>
where
    T: SizedSample + FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    log::info!(
        "Starting audio with sample rate {} and {} channels",
        sample_rate,
        channels
    );

    let mut next_sample = move || {
        if paused.load(Ordering::Relaxed) {
            return 0.0;
        }
        match consumer.try_pop() {
            Some(sample) => sample,
            None => {
                log::warn!("Audio buffer was exhausted, outputting 0");
                0.0
            }
        }
    };

    let err_fn = |err| log::error!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                let value: T = T::from_sample(next_sample());
                for sample in frame.iter_mut() {
                    *sample = value;
                }
            }
        },
        err_fn,
        None,
    )?;
    stream.play()?;

    std::thread::park();

    Ok(())
}

fn setup_emulator(args: &Args) -> Result<(Nes, Arc<AtomicBool>)> {
    // Emulator setup
    let palette = Palette::load("assets/palettes/2C02G.pal")?;
    let cartridge = Cartridge::new(args.rom_path.as_path())?;

    let paused = Arc::new(AtomicBool::new(true));

    let mut nes = if !args.disable_audio {
        let (device, config) = setup_audio()?;
        let stream_config: StreamConfig = config.into();

        let (nes, audio_consumer) =
            Nes::new(palette.clone()).with_audio(stream_config.sample_rate.0 as usize);

        let p = paused.clone();
        thread::spawn(move || {
            start_audio::<f32>(&device, &stream_config, audio_consumer, p)
                .expect("Could not start audio")
        });

        nes
    } else {
        Nes::new(palette.clone())
    };

    // Start
    nes.load_cartridge(cartridge);
    nes.reset();

    Ok((nes, paused))
}

fn draw_with_debug_info(renderer: &mut Renderer, nes: &Nes, fps_counter: &FpsCounter, palette: u8) {
    renderer.draw(&nes.screen().scale(2), 0, 180);

    renderer.draw_text(&format!("FPS: {:.1}", fps_counter.get_fps()), 0, 160);

    draw_ppu_info(renderer, &nes.ppu(), 0, 0);
    draw_palettes(renderer, &nes.ppu(), 240, 0);
    draw_pattern_tables(renderer, &nes.ppu(), palette, 576, 0);
    draw_cpu_info(renderer, nes, 576, 180);
    // draw_oam_sprites(renderer, &nes.ppu(), 600, 320)
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}
