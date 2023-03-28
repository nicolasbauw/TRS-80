use sdl2::{pixels::Color,event::Event,keyboard::Keycode};
use zilog_z80::cpu::CPU;
use std::{error::Error, thread, time::Duration, collections::HashSet, fs};
mod display;
mod keyboard;
mod cassette;
mod config;
mod console;

fn main() -> Result<(), Box<dyn Error>> {
    // Setting up SDL
    let config = config::load_config_file()?;
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
 
    let window = video_subsystem.window("TRuSt-80", config.display.width, config.display.height)
        .position_centered()
        .build()?;
 
    let mut canvas = window.into_canvas()
        .accelerated()
        .present_vsync()
        .build()?;

    let display_mode = video_subsystem.current_display_mode(0)?;
    let refresh_rate =  display_mode.refresh_rate;

    // Setting up CPU
    let mut c = CPU::new(config.memory.ram);
    c.debug.io = config.debug.iodevices.unwrap_or(false);
    c.debug.instr_in = config.debug.iodevices.unwrap_or(false);
    //c.debug.opcode = true;
    if refresh_rate == 50 { 
        c.set_slice_duration(20);       // Matching a 50 Hz refresh rate
    }
    c.set_freq(1.77);               // Model 1 : 1.77 MHz
    c.bus.load_bin(&config.memory.rom, 0)?;
    let rom_space = fs::metadata(&config.memory.rom)?.len();
    c.bus.set_romspace(0, (rom_space - 1) as u16);

    // Setting up channels
    let vram_receiver = c.bus.mmio_read.1.clone();
    let vram_req = c.bus.mmio_req.0.clone();
    let keyboard_sender = c.bus.mmio_write.0.clone();
    let (keys_tx, keys_rx) = zilog_z80::crossbeam_channel::bounded(0);
    let (cassette_cmd_tx, cassette_cmd_rx) = zilog_z80::crossbeam_channel::bounded(0);
    let (cpu_reset_tx, cpu_reset_rx) = zilog_z80::crossbeam_channel::bounded(0);
    let cassette_receiver = c.bus.io_out.1.clone();
    let cassette_sender = c.bus.io_in.0.clone();
    let cassette_req = c.bus.io_req.1.clone();
    let cassette_cmd = cassette_cmd_tx.clone();

    // Starting console
    console::launch(cassette_cmd)?;

    // Starting cassette IO device
    cassette::launch(cassette_receiver, cassette_sender, cassette_req, cassette_cmd_rx)?;

    // Starting keyboard MMIO device
    keyboard::launch(keys_rx, keyboard_sender)?;
    
    // Starting CPU
    thread::Builder::new()
        .name(String::from("CPU"))
        .spawn(move || -> Result<(), zilog_z80::crossbeam_channel::SendError<(u16,u16)>> {
            thread::sleep(Duration::from_millis(500));
            loop {
                if let Some(sleep_time) = c.execute_timed() {
                    // This part is executed when the slice_max_cycles is reached (frequency limit) - slice duration 16ms
                    vram_req.send((0x3C00, 1024))?;

                    #[cfg(windows)]
                    spin_sleep::sleep(Duration::from_millis(u64::from(sleep_time)));

                    #[cfg(not(windows))]
                    std::thread::sleep(Duration::from_millis(u64::from(sleep_time)));
                }
                if let Ok(reset) = cpu_reset_rx.try_recv() {
                    if reset { c.reg.pc = 0 }
                }
            }
        })?;

    // SDL event loop
    let mut old_keys: HashSet<Keycode> = HashSet::new();
    let mut events = sdl_context.event_pump()?;
    'running: loop {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        for event in events.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::F12), .. } => {
                    break 'running
                },
                _ => {}
            }
        }

        let new_keys: HashSet<Keycode> = events
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(Keycode::from_scancode)
            .collect();

        let keys = &new_keys - &old_keys;
        old_keys = new_keys;

        // F7 pressed ? we "rewind the tape
        if keys.contains(&Keycode::F7) {
            cassette_cmd_tx.send((String::from("tape"), String::from("rewind"))).unwrap_or_default();
            thread::sleep(Duration::from_millis(250));
        }

        // F8 pressed ? CPU reset
        if keys.contains(&Keycode::F8) {
            cpu_reset_tx.send(true).unwrap_or_default();
            thread::sleep(Duration::from_millis(50));
        }

        // Keys pressed ? We send a message to the keyboard peripheral thread
        if keys.is_empty() == false { keys_tx.send_timeout(keys, Duration::from_millis(config.keyboard.keypress_timeout)).unwrap_or_default() }

        // Received VRAM data (every 16ms) ?
        if let Ok(data) = vram_receiver.recv() {
            display::display(&mut canvas, data, &config)?;
        };

        canvas.present();
    }
    Ok(())
}