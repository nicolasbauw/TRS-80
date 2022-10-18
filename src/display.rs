use sdl2::rect::Rect;
use sdl2::pixels::Color;
use sdl2::render::Canvas;

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
    
    //let surf = draw_lines(s);

    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string()).expect("TTF Context error");
    let font = ttf_context.load_font("assets/AnotherMansTreasureMIB64C2X3Y.ttf", 128).expect("Could not load font");

    //println!("Chars : {:#?}", &s[0]);
    let surf = font
        .render("Test")
        .blended(Color::RGBA(255, 255, 255, 255))
        .map_err(|e| e.to_string()).expect("Error during line rendering");

    let texture_creator = canvas.texture_creator();
    let r = Rect::new(0, 0, 400, 16);
    let text_tex = texture_creator
            .create_texture_from_surface(surf)
            .expect("Could not create texture");
        canvas.copy(&text_tex, None, Some(r)).expect("Could not copy texture");
    
}
