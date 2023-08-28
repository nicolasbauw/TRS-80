use sdl2::{
    event::Event,
    keyboard::Keycode,
};
use std::error::Error;
mod display;
mod keyboard;
mod cassette;
mod config;
mod machine;
mod console;
use machine::Machine;

fn main() -> Result<(), Box<dyn Error>> {
    // Setting up SDL
    let config = config::load_config_file()?;
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let display_mode = video_subsystem.current_display_mode(0)?;
    let refresh_rate = display_mode.refresh_rate;

    let window = video_subsystem
        .window("TRuSt-80", config.display.width, config.display.height)
        .position_centered()
        .build()?;

    // Creating the TRS-80
    let mut trs80  = Machine::new(window)?;
    if refresh_rate == 50 {
        trs80.cpu.set_slice_duration(20); // Matching a 50 Hz refresh rate
    }

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
        trs80.keyboard.update(events, &mut  trs80.cpu.bus);

        // Update display
        trs80.display.update(&mut trs80.cpu.bus);

        // Handle console commands
        trs80.console().unwrap_or_default();

    }
    Ok(())
}
