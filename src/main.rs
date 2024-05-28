use cpal::StreamConfig;
use ringbuf::traits::*;
use std::env;
use std::process;
use std::thread;
use std::time::Instant;

use anyhow::{anyhow, Result};
use audio_output::AudioBufferConsumer;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SizedSample,
};
use error_iter::ErrorIter as _;
use log::error;
use renderer::Renderer;
use rusttype::Font;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use emu::cartridge::Cartridge;
use emu::consts::FRAME_TIME;
use emu::input::{ControllerButtons, ControllerInput};
use emu::nes::Nes;
use emu::palette::Palette;

mod audio_output;
mod emu;
mod renderer;
mod ui;
mod utils;

const WIDTH: usize = 256;
const HEIGHT: usize = 256;

pub fn main() -> Result<()> {
    env_logger::builder().format_timestamp_micros().init();

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

    let (device, config) = setup_audio()?;
    let stream_config: StreamConfig = config.into();

    let rom_path = &args[1];

    let font_data = include_bytes!("../assets/fonts/nes-arcade-font-2-1-monospaced.ttf");
    let font = Font::try_from_bytes(font_data as &[u8]).ok_or(anyhow!("Error loading font"))?;

    let palette = Palette::load("assets/palettes/2C02G.pal")?;

    let mut renderer = Renderer::new(font, &window, WIDTH, HEIGHT)?;

    let (mut nes, audio_consumer) = Nes::new(palette.clone()).with_audio(
        stream_config.sample_rate.0 as usize,
        stream_config.channels as usize,
    );

    let cartridge = Cartridge::new(rom_path)?;
    nes.load_cartridge(cartridge);
    nes.reset();

    thread::spawn(move || {
        start_audio::<f32>(&device, &stream_config, audio_consumer).expect("Could not start audio")
    });

    let mut displayed_page: u8 = 0;
    let mut paused = false;

    let mut acc = 0.0;
    let mut now = Instant::now();

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
                acc += now.elapsed().as_secs_f64();
                now = Instant::now();
                while acc >= FRAME_TIME {
                    nes.advance_frame();
                    acc -= FRAME_TIME;
                }

                renderer.clear();

                let screen = nes.screen();
                renderer.draw_sprite(screen, 0, 16);

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

pub fn start_audio<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut consumer: AudioBufferConsumer,
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

    let mut next_value = move || match consumer.try_pop() {
        Some(sample) => sample,
        None => {
            log::debug!("Audio buffer was exhausted, outputting 0");
            0.0
        }
    };

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            write_data(data, channels, &mut next_value)
        },
        err_fn,
        None,
    )?;
    stream.play()?;

    std::thread::park();

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
