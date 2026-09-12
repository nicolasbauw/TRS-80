use crate::charset::{CELL_H, CELL_W, CHARSET};
use sdl2::video::Window;
use std::error::Error;
use zilog_silicon::renderer::Renderer;

const COLS: usize = 64;
const ROWS: usize = 16;

/// Display zoom level (F1-F4, or `default_zoom` in config.toml). F1 is
/// "Half" rather than the native resolution itself: the native size
/// (1152x864, see charset.rs) already fills or overflows a 1080p screen,
/// so the useful range shifts down a notch from what a factor of 1/2/3
/// would suggest - F1=half native, F2=native, F3=2x native, F4=
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
    // RGB24 buffer matching the native (screen_width x screen_height)
    // resolution zilog_silicon's wgpu pipeline expects - built fresh from
    // VRAM every frame in draw(), then handed to Renderer::present.
    frame: Vec<u8>,
    crt_panel_visible: bool,
    // Previous frame's raw VRAM, to skip re-rendering rows whose text
    // hasn't changed. `None` forces a full redraw on the first frame.
    last_vram: Option<Vec<u8>>,
    current_zoom: DisplayMode,
}

impl Display {
    pub fn new(window: Window) -> Result<Display, Box<dyn Error>> {
        // The TRS-80 Model I's actual hardware resolution: a 384x192 pixel
        // screen holding 64x16 characters, each a 6x12 cell (see
        // charset.rs) - the fixed "screen" size zilog_silicon's renderer
        // letterboxes/scales into whatever the actual window size is.
        let screen_width = CELL_W * COLS;
        let screen_height = CELL_H * ROWS;

        // pixels_per_scanline is how many *buffer* rows make up one real CRT
        // scanline (the CPC's renderer passes 2: its buffer doubles vertical
        // resolution, so every pair of rows is one scanline). We don't
        // double anything - each rendered pixel row already is one scanline
        // - so this is 1.0.
        let renderer = Renderer::new(window, screen_width, screen_height, 1.0)?;

        Ok(Display {
            renderer,
            screen_width,
            screen_height,
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

    pub fn update(&mut self, bus: &crate::bus::TrsBus) -> Result<(), Box<dyn std::error::Error>> {
        self.draw(bus);

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

    fn draw(&mut self, bus: &crate::bus::TrsBus) {
        let bytes = bus.read_mem_slice(0x3C00, 0x4000);
        const TEXT_COLOR: (u16, u16, u16) = (219, 220, 250);

        for row in 0..ROWS {
            let row_bytes = &bytes[row * COLS..(row + 1) * COLS];
            let unchanged = self
                .last_vram
                .as_deref()
                .is_some_and(|prev| &prev[row * COLS..(row + 1) * COLS] == row_bytes);
            if unchanged {
                continue;
            }

            let y0 = row * CELL_H;
            for (col, &code) in row_bytes.iter().enumerate() {
                let x0 = col * CELL_W;
                let glyph = &CHARSET[code as usize];
                for gy in 0..CELL_H {
                    let dst_row_start = (y0 + gy) * self.screen_width * 3;
                    for gx in 0..CELL_W {
                        let a = glyph[gy * CELL_W + gx] as u16;
                        let dst = (dst_row_start + (x0 + gx) * 3)..;
                        self.frame[dst.start] = ((TEXT_COLOR.0 * a) / 255) as u8;
                        self.frame[dst.start + 1] = ((TEXT_COLOR.1 * a) / 255) as u8;
                        self.frame[dst.start + 2] = ((TEXT_COLOR.2 * a) / 255) as u8;
                    }
                }
            }
        }
        self.last_vram = Some(bytes);
    }
}
