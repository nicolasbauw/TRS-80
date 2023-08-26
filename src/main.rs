//use crate::config::Config;
use sdl2::{
    event::Event,
    keyboard::Keycode,
};
use std::{error::Error, fs, thread, time::Duration};
use zilog_z80::cpu::CPU;
mod display;
mod keyboard;
mod cassette;
mod config;
//mod console;

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

    // Setting up CPU
    let mut c = CPU::new(0xFFFF);
    c.debug.io = config.debug.iodevices.unwrap_or(false);
    c.debug.instr_in = config.debug.iodevices.unwrap_or(false);
    //c.debug.opcode = true;
    if refresh_rate == 50 {
        c.set_slice_duration(20); // Matching a 50 Hz refresh rate
    }
    c.set_freq(1.77); // Model 1 : 1.77 MHz
    c.bus.load_bin(&config.memory.rom, 0)?;
    let rom_space = fs::metadata(&config.memory.rom)?.len();
    c.bus.set_romspace(0, (rom_space - 1) as u16);

    let mut tape = cassette::CassetteReader::new();
    let mut keyboard = keyboard::Keyboard::new();
    let mut display = display::Display::new(window)?;
    tape.load("bin/invade.cas".into())?;

    'running: loop {
        // CPU loop
        loop {
            let opcode = c.bus.read_byte(c.reg.pc);
            match opcode {
                0xdb => {
                    let port = c.bus.read_byte(c.reg.pc + 1);
                    if let Some(true) = config.debug.iodevices {
                        println!("IN on port {}", port);
                    }
                    // cassette reader port ?
                    if port == 0xFF {
                        c.reg.a = tape.read();
                    }
                }
                0xd3 => {
                    let port = c.bus.read_byte(c.reg.pc + 1);
                    if let Some(true) = config.debug.iodevices {
                        println!("OUT {} on port {}", c.reg.a, port);
                    }
                    if port == 0xFF {}
                }
                _ => {}
            }

            // executes slice_max_cycles number of cycles
            if let Some(t) = c.execute_timed() {
                thread::sleep(Duration::from_millis(t.into()));
                break;
            }
        }
        
        // Display
        let vram = c.bus.read_mem_slice(0x3C00, 0x4000);
        display.canvas.clear();
        display.draw(vram, &config).unwrap();
        display.canvas.present();

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

        // Keyboard MMIO peripheral
        keyboard.update(events, &mut  c.bus);

    }
    Ok(())
}
