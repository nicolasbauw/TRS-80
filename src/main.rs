use sdl2::{pixels::Color,event::Event,keyboard::Keycode};
use zilog_z80::cpu::CPU;
use std::error::Error;
mod display;
use display::display;

fn main() -> Result<(), Box<dyn Error>> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
 
    let window = video_subsystem.window("rust sdl2 template", 800, 600)
        .position_centered()
        .build()?;
 
    let mut canvas = window.into_canvas()
    //.present_vsync()
    .build()?;

    let mut c = CPU::new();
    c.bus.load_bin("bin/trs80m13diag.bin", 0).unwrap();

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
        // The rest of the game loop goes here...
        c.execute_slice();
        display(&mut canvas, c.bus.read_mem_slice(0x3c00, 0x3fff));
        canvas.present();
    }
    Ok(())
}