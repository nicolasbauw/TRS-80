use sdl2::rect::Rect;
use sdl2::pixels::Color;
use sdl2::render::Canvas;

pub fn do_square(canvas: &mut Canvas<sdl2::video::Window>) -> Result<(), String> {
    canvas.set_draw_color(Color::RGB(255, 255, 255));
    let r = Rect::new(0, 0, 32, 32);
    canvas.fill_rect(r)
}