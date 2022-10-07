use sdl2::{pixels::Color,event::Event,keyboard::Keycode};
use zilog_z80::cpu::CPU;
use std::{error::Error, thread};
mod display;
use display::display;

fn main() -> Result<(), Box<dyn Error>> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
 
    let window = video_subsystem.window("rust sdl2 template", 800, 600)
        .position_centered()
        .build()?;
 
    let mut canvas = window.into_canvas()
        .build()?;

    let mut c = CPU::new();
    c.bus.load_bin("bin/trs80m13diag.bin", 0).unwrap();
    let mem_receiver1 = c.bus.rw.1.clone();

    // CPU lives its life on his own thread
    thread::spawn(move || {
        loop {
            c.execute_slice();
            c.bus.channel_send(0x3C00, 0x3FFF);
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
        let m: Vec<u8> = match mem_receiver1.recv() {
            Ok((_, data)) => data,
            Err(_) => Vec::new()
        };
        display(&mut canvas, m);

        canvas.present();
    }
    Ok(())
}