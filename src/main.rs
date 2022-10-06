use sdl2::{pixels::Color,event::Event,keyboard::Keycode};
use std::error::Error;
mod square;

fn main() -> Result<(), Box<dyn Error>> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
 
    let window = video_subsystem.window("rust sdl2 template", 800, 600)
        .position_centered()
        .build()?;
 
    let mut canvas = window.into_canvas()
    .present_vsync()
    .build()?;

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
        square::do_square(&mut canvas)?;
        canvas.present();
    }
    Ok(())
}