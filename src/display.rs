use sdl2::rect::Rect;
use sdl2::pixels::Color;
use sdl2::render::Canvas;

pub fn display(canvas: &mut Canvas<sdl2::video::Window>, bytes: Vec<u8>, config: &crate::config::Config) -> Result<(), Box<dyn std::error::Error>> {
    let mut start = 0x0000;
    let mut end = 0x0040;
    let mut line_bytes: Vec<&[u16]> = Vec::new();

    // Converting VRAM data to UTF-16
    let mut utf_data: Vec<u16> = Vec::new();
    for c in bytes.iter() {
        let d = (0xE0 << 8) | u16::from(*c);
        utf_data.push(d);
    }
    
    // Cutting the video memory into lines of bytes
    for _ in 0..=15 {
        line_bytes.push(&utf_data[start..end]);
        start += 0x40;
        end += 0x40;
    }

    // Creating the text lines
    let mut s = Vec::new();
    for line in line_bytes.iter() {
        s.push(String::from_utf16_lossy(&line));
    }

    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;
    let font = ttf_context.load_font(&config.display.font, config.display.font_size)?;

    let texture_creator = canvas.texture_creator();
    let mut y = 0;
    for line in s.iter() {
        let surf = font
            .render(&line)
            .blended(Color::RGBA(219, 220, 250, 255))
            .map_err(|e| e.to_string())?;

        let r = Rect::new(0, y, config.display.width, config.display.height/16);
        y += (config.display.height as i32)/16;
        let text_tex = texture_creator
                .create_texture_from_surface(surf)?;
        canvas.copy(&text_tex, None, Some(r))?;
    }
    Ok(())
}
