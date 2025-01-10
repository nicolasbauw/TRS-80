use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::error::Error;

pub struct Display {
    pub canvas: Canvas<sdl2::video::Window>,
    config: crate::config::Config,
}

impl Display {
    pub fn new(window: Window) -> Result<Display, Box<dyn Error>> {
        let config = crate::config::load_config_file()?;
        let d = Display {
            canvas: window.into_canvas().accelerated().present_vsync().build()?,
            config,
        };
        Ok(d)
    }

    pub fn update(&mut self, bus: &zilog_z80::bus::Bus, font: &sdl2::ttf::Font) -> Result<(), Box<dyn std::error::Error>> {
        self.canvas.clear();
        self.draw(bus, font)?;
        self.canvas.present();
        Ok(())
    }

    fn draw(&mut self, bus: &zilog_z80::bus::Bus, font: &sdl2::ttf::Font) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = bus.read_mem_slice(0x3C00, 0x4000);
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
            s.push(String::from_utf16_lossy(line));
        }

        let texture_creator = self.canvas.texture_creator();
        let mut y = 0;
        for line in s.iter() {
            let surf = font
                .render(line)
                .blended(Color::RGBA(219, 220, 250, 255))
                .map_err(|e| e.to_string())?;

            let r = Rect::new(
                0,
                y,
                self.config.display.width,
                self.config.display.height / 16,
            );
            y += (self.config.display.height as i32) / 16;
            let text_tex = texture_creator.create_texture_from_surface(surf)?;
            self.canvas.copy(&text_tex, None, Some(r))?;
        }
        Ok(())
    }
}
