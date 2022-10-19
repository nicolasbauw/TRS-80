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
    let vram_receiver = c.bus.mmio_read.1.clone();
    let vram_req = c.bus.mmio_req.0.clone();
    let periph_ff_receiver = c.bus.io_out.1.clone();

    // Dummy IO peripheral
    thread::spawn(move || {
        loop {
            if let Ok((device, data)) = periph_ff_receiver.recv() {
                if device == 0xFF { eprintln!("Device 0xFF received {:#04X}", data)}
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
        vram_req.send((0x3C00, 1024)).unwrap();

        // Received VRAM data ?
        if let Ok((_, data)) = vram_receiver.recv() {
            display(&mut canvas, data);
        };

        thread::sleep(Duration::from_millis(16));
        canvas.present();
    }
    Ok(())
}