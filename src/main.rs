use sdl2::{event::Event, keyboard::Keycode};
use std::{error::Error, process::ExitCode};
mod cassette;
mod config;
mod console;
mod display;
mod hexconversion;
mod keyboard;
mod machine;
use machine::{Machine, MachineError};

fn main() -> ExitCode {
    if let Err(err) = launch() {
        eprintln!("[Error]: {}", err);
        return ExitCode::from(1);
    }
    println!("\n");
    ExitCode::from(0)
}

fn launch() -> Result<(), Box<dyn Error>> {
    // Setting up SDL
    let config = config::load_config_file()?;
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let display_mode = video_subsystem.current_display_mode(0)?;
    let refresh_rate = display_mode.refresh_rate;
    let ttf_context = sdl2::ttf::init()?;

    let window = video_subsystem
        .window("TRuSt-80", config.display.width, config.display.height)
        .position_centered()
        .build()?;

    // Creating the TRS-80
    let mut trs80 = Machine::new(window)?;
    trs80.set_timings(refresh_rate);

    let Ok(font) = ttf_context.load_font(config.display.font, config.display.font_size) else {
        eprintln!("\nCan't load font");
        return Err(Box::new(MachineError::DisplayError));
    };

    // SDL loop
    'running: loop {
        // CPU loop
        trs80.cpu_loop();

        // SDL events
        let mut events = sdl_context.event_pump()?;
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::F12),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        // Handle SDL keyboard events (keyboard MMIO peripheral)
        trs80.keyboard.update(events, &mut trs80.cpu.bus);

        // Update display
        trs80.display.update(&trs80.cpu.bus, &font)?;

        // Handle console commands
        trs80.console().unwrap_or_default();
    }
    Ok(())
}
