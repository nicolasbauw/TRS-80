use sdl2::{pixels::Color,event::Event,keyboard::Keycode};
use zilog_z80::cpu::CPU;
use std::{error::Error, thread, time::Duration};
mod display;
mod config;

fn main() -> Result<(), Box<dyn Error>> {
    let config = config::load_config_file()?;
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
 
    let window = video_subsystem.window("TRuSt-80", config.screen.width, config.screen.height)
        .position_centered()
        .build()?;
 
    let mut canvas = window.into_canvas()
        .accelerated()
        .present_vsync()
        .build()?;

    let mut c = CPU::new(config.memory.ram);
    c.set_freq(1.77);
    c.bus.load_bin(&config.memory.romfile, 0)?;
    let vram_receiver = c.bus.mmio_read.1.clone();
    let vram_req = c.bus.mmio_req.0.clone();
    let periph_ff_receiver = c.bus.io_out.1.clone();

    // Dummy IO peripheral
    thread::spawn(move || {
        loop {
            if let Ok((device, data)) = periph_ff_receiver.recv() {
                if device == 0xFF {
                    match config.debug.iodevices { 
                        Some(true) => { eprintln!("Device 0xFF received {:#04X}", data) },
                        None | Some(false) => continue,
                    }
                }
            }
        }
    });

    // CPU lives its life on his own thread
    thread::spawn(move || {
        loop {
            c.execute_slice();
        }
    });

    let mut event_pump = sdl_context.event_pump()?;
    'running: loop {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }

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