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
        .accelerated()
        .build()?;

    let mut c = CPU::new();
    c.bus.load_bin("bin/trs80m13diag.bin", 0).unwrap();
    c.debug.io = true;
    let mem_receiver1 = c.bus.rw.1.clone();
    let io_receiver1 = c.io.1.clone();

    // CPU lives its life on his own thread
    thread::spawn(move || {
        loop {
            c.execute_slice();
            c.bus.channel_send(0x3C00, 0x3FFF);
        }
    });

    // Dummy IO peripheral
    thread::spawn(move || {
        loop {
            if let Ok((device, data)) = io_receiver1.recv() {
                //if device == 0xFF { println!("Device 0xFF received {:04X}", data)}
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
            //println!("VRAM data received");
        };

        canvas.present();
    }
    Ok(())
}