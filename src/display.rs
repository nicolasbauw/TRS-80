use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::video::Window;
use std::error::Error;
use zilog_silicon::renderer::Renderer;

const COLS: usize = 64;
const ROWS: usize = 16;

/// Display zoom level (F1-F4, or `default_zoom` in config.toml). F1 is
/// "Half" rather than the native resolution itself: with the default
/// 48pt font, the native size alone already fills or overflows a 1080p
/// screen, so the useful range shifts down a notch from what a factor of
/// 1/2/3 would suggest - F1=half native, F2=native, F3=2x native, F4=
/// fullscreen. X3 (3x) was dropped: it overflowed even a 4K screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DisplayMode {
    Half,
    Normal,
    X2,
    Fullscreen,
}

impl DisplayMode {
    /// Reads `default_zoom` (config.toml, [display] section). An absent or
    /// unrecognized value silently falls back to Normal.
    pub fn from_config(value: Option<&str>) -> Self {
        match value {
            Some("half") => DisplayMode::Half,
            Some("x2") => DisplayMode::X2,
            Some("fullscreen") => DisplayMode::Fullscreen,
            _ => DisplayMode::Normal,
        }
    }

    pub fn as_config_str(self) -> &'static str {
        match self {
            DisplayMode::Half => "half",
            DisplayMode::Normal => "normal",
            DisplayMode::X2 => "x2",
            DisplayMode::Fullscreen => "fullscreen",
        }
    }
}

pub struct Display {
    renderer: Renderer,
    screen_width: usize,
    screen_height: usize,
    cell_height: usize,
    // RGB24 buffer matching the native (screen_width x screen_height)
    // resolution zilog_silicon's wgpu pipeline expects - built fresh from
    // VRAM every frame in draw(), then handed to Renderer::present.
    frame: Vec<u8>,
    crt_panel_visible: bool,
    // Previous frame's raw VRAM, to skip re-rendering (SDL_ttf/FreeType
    // shaping plus the manual alpha composite) rows whose text hasn't
    // changed - re-running that for all 16 rows unconditionally every
    // frame measured at ~23ms regardless of whether anything moved, which
    // pegged a CPU core (bytebox's own video::render has no equivalent
    // cost: it's a pixel-buffer lookup, not FreeType text shaping). `None`
    // forces a full redraw on the first frame.
    last_vram: Option<Vec<u8>>,
    current_zoom: DisplayMode,
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

        // pixels_per_scanline is how many *buffer* rows make up one real CRT
        // scanline (the CPC's renderer passes 2: its buffer doubles vertical
        // resolution, so every pair of rows is one scanline). We don't
        // double anything - each rendered pixel row already is one scanline
        // - so this is 1.0, not cell_height (a whole character cell's
        // height, tens of pixels): passing that made the shader treat an
        // entire text row as a single "scanline" period, producing the
        // garbled banding seen when the CRT shader was enabled.
        let renderer = Renderer::new(window, screen_width, screen_height, 1.0)?;

        Ok(Display {
            renderer,
            screen_width,
            screen_height,
            cell_height,
            frame: vec![0u8; screen_width * screen_height * 3],
            crt_panel_visible: false,
            last_vram: None,
            current_zoom: DisplayMode::Normal,
        })
    }

    pub fn current_zoom(&self) -> DisplayMode {
        self.current_zoom
    }

    /// Applies a zoom level to the main window. The 4:3-style aspect-ratio
    /// letterboxing isn't this function's job - `Renderer::present`
    /// recomputes a viewport every frame for whatever the window's actual
    /// size ends up being (see `renderer.rs`) - so this only ever picks a
    /// window size or fullscreen.
    pub fn set_zoom(&mut self, mode: DisplayMode) {
        self.current_zoom = mode;
        let window = self.renderer.window_mut();
        match mode {
            DisplayMode::Fullscreen => {
                let _ = window.set_fullscreen(sdl2::video::FullscreenType::Desktop);
                return;
            }
            DisplayMode::Half | DisplayMode::Normal | DisplayMode::X2 => {
                let _ = window.set_fullscreen(sdl2::video::FullscreenType::Off);
            }
        }
        let factor = match mode {
            DisplayMode::Half => 0.5,
            DisplayMode::Normal => 1.0,
            DisplayMode::X2 => 2.0,
            DisplayMode::Fullscreen => unreachable!("handled above, with an early return"),
        };
        let _ = window.set_size(
            (self.screen_width as f32 * factor) as u32,
            (self.screen_height as f32 * factor) as u32,
        );
        window.set_position(
            sdl2::video::WindowPos::Centered,
            sdl2::video::WindowPos::Centered,
        );
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
        bus: &crate::bus::TrsBus,
        font: &sdl2::ttf::Font,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.draw(bus, font)?;

        if self.crt_panel_visible {
            let mut settings = self.renderer.crt_settings();
            let mut open = true;
            let mut requested_zoom: Option<DisplayMode> = None;
            let mut save_zoom_requested = false;
            let current_zoom = self.current_zoom;
            {
                let mut overlay = |ctx: &egui::Context| {
                    egui::Window::new("Display").open(&mut open).show(ctx, |ui| {
                        ui.label("Zoom");
                        ui.horizontal(|ui| {
                            if ui.button("Half").clicked() {
                                requested_zoom = Some(DisplayMode::Half);
                            }
                            if ui.button("Normal").clicked() {
                                requested_zoom = Some(DisplayMode::Normal);
                            }
                            if ui.button("x2").clicked() {
                                requested_zoom = Some(DisplayMode::X2);
                            }
                            if ui.button("Fullscreen").clicked() {
                                requested_zoom = Some(DisplayMode::Fullscreen);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(format!("Current zoom: {}", current_zoom.as_config_str()));
                            if ui.button("Save as startup default").clicked() {
                                save_zoom_requested = true;
                            }
                        });

                        ui.separator();
                        ui.label("CRT shader");
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
            if let Some(mode) = requested_zoom {
                self.set_zoom(mode);
            }
            if save_zoom_requested {
                match crate::config::load_config_file() {
                    Ok(mut config) => {
                        config.display.default_zoom = Some(self.current_zoom.as_config_str().to_string());
                        if let Err(e) = crate::config::save_display_config(&config.display) {
                            eprintln!("Can't save default zoom: {e}");
                        }
                    }
                    Err(e) => eprintln!("Can't save default zoom: {e}"),
                }
            }
        } else {
            self.renderer.present(&self.frame, None);
        }
        Ok(())
    }

    fn draw(
        &mut self,
        bus: &crate::bus::TrsBus,
        font: &sdl2::ttf::Font,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = bus.read_mem_slice(0x3C00, 0x4000);

        for row in 0..ROWS {
            let row_bytes = &bytes[row * COLS..(row + 1) * COLS];
            let unchanged = self
                .last_vram
                .as_deref()
                .is_some_and(|prev| &prev[row * COLS..(row + 1) * COLS] == row_bytes);
            if unchanged {
                continue;
            }

            // Converting VRAM data to UTF-16: the font maps the TRS-80
            // character set 1:1 onto a private-use-area block starting at
            // 0xE0.
            let mut utf_line: Vec<u16> = Vec::with_capacity(COLS);
            for c in row_bytes {
                utf_line.push((0xE0 << 8) | u16::from(*c));
            }
            let line = String::from_utf16_lossy(&utf_line);

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
            let cell_height = self.cell_height;
            let frame = &mut self.frame;

            // Clears this row's rectangle first: the composite loop below
            // only covers copy_w/copy_h, which can fall short of the full
            // cell (font metrics variance) and would otherwise leave stale
            // pixels from whatever used to be drawn there.
            for y in 0..cell_height {
                let dst_start = (y0 + y) * screen_width * 3;
                frame[dst_start..dst_start + screen_width * 3].fill(0);
            }
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
        self.last_vram = Some(bytes);
        Ok(())
    }
}
