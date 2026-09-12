use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::video::Window;
use std::error::Error;
use zilog_silicon::renderer::Renderer;

const COLS: usize = 64;
const ROWS: usize = 16;

pub struct Display {
    renderer: Renderer,
    screen_width: usize,
    cell_height: usize,
    // RGB24 buffer matching the native (screen_width x screen_height)
    // resolution zilog_silicon's wgpu pipeline expects - built fresh from
    // VRAM every frame in draw(), then handed to Renderer::present.
    frame: Vec<u8>,
    crt_panel_visible: bool,
}

impl Display {
    pub fn new(window: Window, font: &sdl2::ttf::Font) -> Result<Display, Box<dyn Error>> {
        // The AMTreasure font is fixed-width (one glyph per TRS-80
        // character-generator code point, see hexconversion/the VRAM->UTF-16
        // mapping in draw() below), so a single glyph's measured size gives
        // us the whole 64x16 grid's native pixel resolution - the fixed
        // "screen" size zilog_silicon's renderer letterboxes/scales into
        // whatever the actual window size is.
        let (cell_width, cell_height) = font.size_of_char('A')?;
        let (cell_width, cell_height) = (cell_width as usize, cell_height as usize);
        let screen_width = cell_width * COLS;
        let screen_height = cell_height * ROWS;

        let renderer = Renderer::new(window, screen_width, screen_height, cell_height as f32)?;

        Ok(Display {
            renderer,
            screen_width,
            cell_height,
            frame: vec![0u8; screen_width * screen_height * 3],
            crt_panel_visible: false,
        })
    }

    pub fn resize(&mut self) {
        self.renderer.resize();
    }

    pub fn handle_event(&mut self, event: &sdl2::event::Event) {
        self.renderer.handle_event(event);
    }

    pub fn toggle_crt(&mut self) {
        self.renderer.toggle_crt();
    }

    pub fn toggle_crt_panel(&mut self) {
        self.crt_panel_visible = !self.crt_panel_visible;
    }

    pub fn update(
        &mut self,
        bus: &zilog_z80::bus::FlatBus,
        font: &sdl2::ttf::Font,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.draw(bus, font)?;

        if self.crt_panel_visible {
            let mut settings = self.renderer.crt_settings();
            let mut open = true;
            {
                let mut overlay = |ctx: &egui::Context| {
                    egui::Window::new("CRT shader").open(&mut open).show(ctx, |ui| {
                        ui.add(egui::Slider::new(&mut settings.mask_cell_px, 1.0..=6.0).text("Mask cell (px)"));
                        ui.add(egui::Slider::new(&mut settings.mask_min, 0.0..=1.0).text("Mask min"));
                        ui.add(egui::Slider::new(&mut settings.mask_strength, 0.0..=1.0).text("Mask strength"));
                        ui.add(egui::Slider::new(&mut settings.scanline_beam, 1.0..=20.0).text("Scanline beam"));
                        ui.add(
                            egui::Slider::new(&mut settings.scanline_strength, 0.0..=1.0)
                                .text("Scanline strength"),
                        );
                        ui.add(egui::Slider::new(&mut settings.beam_bloom, 0.0..=2.0).text("Beam bloom"));
                        ui.add(egui::Slider::new(&mut settings.bright_boost, 0.5..=3.0).text("Bright boost"));
                        ui.add(
                            egui::Slider::new(&mut settings.horizontal_blur, 0.0..=1.0)
                                .text("Horizontal blur"),
                        );
                        if ui.button("Reset to defaults").clicked() {
                            settings = zilog_silicon::renderer::CrtSettings::default();
                        }
                    });
                };
                self.renderer.present(&self.frame, Some(&mut overlay));
            }
            self.renderer.set_crt_settings(settings);
            self.crt_panel_visible = open;
        } else {
            self.renderer.present(&self.frame, None);
        }
        Ok(())
    }

    fn draw(
        &mut self,
        bus: &zilog_z80::bus::FlatBus,
        font: &sdl2::ttf::Font,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.frame.fill(0);

        let bytes = bus.read_mem_slice(0x3C00, 0x4000);

        // Converting VRAM data to UTF-16: the font maps the TRS-80 character
        // set 1:1 onto a private-use-area block starting at 0xE0.
        let mut utf_data: Vec<u16> = Vec::with_capacity(bytes.len());
        for c in bytes.iter() {
            utf_data.push((0xE0 << 8) | u16::from(*c));
        }

        for row in 0..ROWS {
            let line = String::from_utf16_lossy(&utf_data[row * COLS..(row + 1) * COLS]);
            let surf = font
                .render(&line)
                .blended(Color::RGBA(219, 220, 250, 255))
                .map_err(|e| e.to_string())?;
            // `.blended()` fills the WHOLE surface with the text color and
            // varies only per-pixel alpha (0 = background, 255 = solid
            // stroke) - it's coverage, not an image. Converting straight to
            // RGB24 drops that alpha and leaves every pixel the same flat
            // text color, background included (a blank lavender screen).
            // RGBA32 keeps alpha in a byte order that's guaranteed regardless
            // of platform endianness, so it can be composited by hand onto
            // the black background below.
            let surf = surf
                .convert_format(PixelFormatEnum::RGBA32)
                .map_err(|e| e.to_string())?;

            let pitch = surf.pitch() as usize;
            let copy_w = (surf.width() as usize).min(self.screen_width);
            let copy_h = (surf.height() as usize).min(self.cell_height);
            let y0 = row * self.cell_height;
            let screen_width = self.screen_width;
            let frame = &mut self.frame;
            surf.with_lock(|pixels| {
                for y in 0..copy_h {
                    let src = &pixels[y * pitch..y * pitch + copy_w * 4];
                    let dst_start = (y0 + y) * screen_width * 3;
                    let dst = &mut frame[dst_start..dst_start + copy_w * 3];
                    for (d, s) in dst.chunks_exact_mut(3).zip(src.chunks_exact(4)) {
                        let a = s[3] as u16;
                        d[0] = ((s[0] as u16 * a) / 255) as u8;
                        d[1] = ((s[1] as u16 * a) / 255) as u8;
                        d[2] = ((s[2] as u16 * a) / 255) as u8;
                    }
                }
            });
        }
        Ok(())
    }
}
