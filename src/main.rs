use sdl2::{pixels::Color,event::Event,keyboard::Keycode};
use zilog_z80::cpu::CPU;
use std::{error::Error, thread, time::Duration};
mod display;
use display::display;

fn main() -> Result<(), Box<dyn Error>> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
 
    let window = video_subsystem.window("TRuSt-80", 800, 600)
        .position_centered()
        .build()?;
 
    let mut canvas = window.into_canvas()
        .accelerated()
        .present_vsync()
        .build()?;

    let mut c = CPU::new(0xFFFF);
    c.set_freq(1.77);
    c.bus.load_bin("bin/trs80m13diag.bin", 0).unwrap();
    let mem_receiver1 = c.bus.mmio.1.clone();
    let io_receiver1 = c.bus.io.1.clone();

    // Dummy IO peripheral
    thread::spawn(move || {
        loop {
            if let Ok((device, data)) = io_receiver1.recv() {
                if device == 0xFF { eprintln!("Device 0xFF received {:#04X}", data)}
            }
        }
    });

    // CPU lives its life on his own thread
    thread::spawn(move || {
        // Letting time for peripherals to start on their own thread
        thread::sleep(Duration::from_millis(1000));

        loop {
            for _ in 0..5000 {
                c.execute_slice();
            }
            if let Err(_) = c.bus.channel_send(0x3C00, 0x3FFF) {
                eprintln!("VRAM Send error");
            }
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

        // Received VRAM data from the CPU thread ?
        if let Ok((_, data)) = mem_receiver1.recv() {
            display(&mut canvas, data);
        };

        canvas.present();
    }
    Ok(())
}