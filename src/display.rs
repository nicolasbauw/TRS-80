use sdl2::rect::Rect;
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2_unifont::renderer::SurfaceRenderer;
use std::borrow::Cow;
use std::str;

pub fn display(canvas: &mut Canvas<sdl2::video::Window>, bytes: Vec<u8>) {
    let mut start = 0x0000;
    let mut end = 0x0040;
    let mut line_bytes: Vec<&[u8]> = Vec::new();
    
    // Cutting the video memory into lines of bytes
    for _ in 0..15 {
        line_bytes.push(&bytes[start..end]);
        start += 0x40;
        end += 0x40;
    }

    // Creating the text lines
    let mut s = Vec::new();
    for line in line_bytes.iter() {
        s.push(String::from_utf8_lossy(&line));
    }
    
    let surf = draw_lines(s);

    let texture_creator = canvas.texture_creator();
    let text_tex = texture_creator
            .create_texture_from_surface(surf)
            .unwrap();
        canvas.copy(&text_tex, None, None).unwrap();
    
}

fn draw_lines(chars: Vec<Cow<str>>) -> sdl2::surface::Surface {
    let mut start:i32 = 2;
    let mut screen = sdl2::surface::Surface::new(
        800,
        600,
        sdl2::pixels::PixelFormatEnum::RGBA8888,
    ).unwrap();

    for s in chars.iter() {
        let renderer = SurfaceRenderer::new(Color::RGB(255, 255, 255), Color::RGB(0, 0, 0));
        renderer
            .draw(&s)
            .unwrap()
            .blit(None, &mut screen, Rect::new(2, start, 0, 0))
            .unwrap();
        start += 20;
    }
    screen
}