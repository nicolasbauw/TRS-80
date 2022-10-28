use sdl2::{pixels::Color,event::Event,keyboard::Keycode};
use zilog_z80::cpu::CPU;
use std::{error::Error, thread, time::Duration};
use std::collections::HashSet;
mod display;
mod keyboard;
mod config;

fn main() -> Result<(), Box<dyn Error>> {
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

    let mut c = CPU::new(config.memory.ram);
    c.debug.io = config.debug.iodevices.unwrap_or(false);
    c.debug.instr_in = config.debug.iodevices.unwrap_or(false);
    c.debug.opcode = true;
    c.set_freq(1.77);
    c.bus.load_bin(&config.memory.rom, 0)?;
    let vram_receiver = c.bus.mmio_read.1.clone();
    let vram_req = c.bus.mmio_req.0.clone();
    let keyboard_sender = c.bus.mmio_write.0.clone();
    let (keys_tx, keys_rx) = zilog_z80::crossbeam_channel::bounded(0);
    let cassette_receiver = c.bus.io_out.1.clone();
    let cassette_sender = c.bus.io_in.0.clone();
    let cassette_req = c.bus.io_req.1.clone();
    let (motor_tx, motor_rx) = zilog_z80::crossbeam_channel::bounded(0);

    // 0xFF IO peripheral (Cassette) CPU -> Cassette
    thread::spawn(move || {
        let mut motor = false;
        loop {
            // Data sent from CPU to cassette ? (OUT)
            if let Ok((device, data)) = cassette_receiver.recv() {
                if device == 0xFF {
                    match data {
                        0x04 => {
                            motor = !motor;
                            println!("MOTOR REQUEST : {}", motor);
                            motor_tx.send_timeout(motor, Duration::from_millis(250)).expect("Could not send motor status message");
                        },
                        _ => continue,
                    }
                }
            }
        }
    });

    // 0xFF IO peripheral (Cassette) Cassette -> CPU
    thread::spawn(move || {
        let tape = include_bytes!("startrek.cas");
        let mut tape_pos = 0;
        let mut motor = false;
        loop {
            if let Ok(mt) = motor_rx.try_recv() {
                motor = mt;
                println!("MOTOR STATE : {}", motor);
            }
            if let Ok(device) = cassette_req.recv() {
                // IN instruction for the 0xFF device ?
                if device == 0xFF && motor == true && tape_pos < tape.len() {
                    // We send the data through the io_in channel
                    if tape_pos < tape.len() {
                        cassette_sender.send((0xFF, tape[tape_pos])).expect("Cassette message send error");
                        println!("The 0xFF peripheral puts {:#04X} on the data bus", tape[tape_pos]);
                    }
                } else if device == 0xFF && motor == true && tape_pos >= tape.len() {
                    cassette_sender.send((0xFF, 0x80)).expect("Cassette message send error");
                    println!("The 0xFF peripheral puts 0x80 on the data bus (end of tape)");
                }
                if tape_pos < tape.len() && motor == true { tape_pos += 1; }
                println!("Tape position : {}", tape_pos);
            }
            //else if device == 0xFF && motor == false {};
        }
        //thread::sleep(Duration::from_millis(50));
        
    });

    // Keyboard MMIO peripheral
    thread::spawn(move || {
        loop {
            if let Ok(keys) = keys_rx.recv() {
                if !keyboard::keyboard(keys, &keyboard_sender) { thread::sleep(Duration::from_millis(config.keyboard.repeat_delay)); }
            }
        }
    });

    // CPU lives its life on his own thread
    thread::spawn(move || {
        loop {
            c.execute_slice();
            //println!("{}", c.debug.string);
        }
    });

    let mut events = sdl_context.event_pump()?;
    'running: loop {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        for event in events.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }

        let keys: HashSet<Keycode> = events
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(Keycode::from_scancode)
            .collect();

        // Keys pressed ? We send a message to the keyboard peripheral thread
        if keys.is_empty() == false { keys_tx.send_timeout(keys, Duration::from_millis(config.keyboard.keypress_timeout)).unwrap_or_default() }

        // VRAM data request
        vram_req.send((0x3C00, 1024)).expect("Error while requesting VRAM data");

        // Received VRAM data ?
        if let Ok((_, data)) = vram_receiver.recv() {
            display::display(&mut canvas, data, &config);
        };

        thread::sleep(Duration::from_millis(16));
        canvas.present();
    }
    Ok(())
}