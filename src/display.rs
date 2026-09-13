use sdl2::video::Window;
use std::error::Error;
use trust_80_core::bus::TrsBus;
use trust_80_core::video::{self, ROWS, SCREEN_HEIGHT, SCREEN_WIDTH};
use zilog_silicon::renderer::Renderer;

/// Display zoom level (F1-F4, or `default_zoom` in config.toml). F1 is
/// "Half" rather than the native resolution itself: the native size
/// (1152x864, see core's charset.rs) already fills or overflows a 1080p
/// screen,
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
    // RGB24 buffer matching the native (SCREEN_WIDTH x SCREEN_HEIGHT)
    // resolution zilog_silicon's wgpu pipeline expects - built fresh from
    // VRAM every frame in draw(), then handed to Renderer::present.
    frame: Vec<u8>,
    crt_panel_visible: bool,
    // Previous frame's raw VRAM, to skip re-rendering rows whose text
    // hasn't changed. `None` forces a full redraw on the first frame.
    last_vram: Option<Vec<u8>>,
    current_zoom: DisplayMode,
}

/// bytebox's own `CrtSettings::default()` (unchanged for the two fields not
/// overridden here - mask_cell_px/mask_min/mask_strength/beam_bloom) was
/// tuned by eye against its buffer's 2x vertical oversampling; ours is
/// 4.5x, so the same scanline_strength/horizontal_blur read as far too
/// strong, and far too blurry, at this finer pitch. scanline_beam was
/// further lowered from bytebox's own 9.0 to 6.5 (a wider beam) once the
/// shader stopped needlessly collapsing vertical resolution (see
/// renderer_crt.wgsl): at 9.0, our own font's finer detail made the beam
/// read as too narrow/harsh. These four are tuned by eye for our own
/// oversampling factor and font instead - still fully adjustable live via
/// the F6 panel. Must stay in sync with trust-80-web's own
/// `crt::CrtSettings::default()`.
///
/// The single source of truth for "no [crt] section saved yet" (`new()`)
/// AND for the F6 panel's own "Reset to defaults" button - using the bare
/// `zilog_silicon::renderer::CrtSettings::default()` for the latter was a
/// bug: it silently reset scanline_beam back to bytebox's 9.0 instead of
/// trust-80's own 6.5.
fn tuned_crt_defaults() -> zilog_silicon::renderer::CrtSettings {
    zilog_silicon::renderer::CrtSettings {
        scanline_beam: 6.5,
        scanline_strength: 0.5,
        horizontal_blur: 0.75,
        bright_boost: 1.05,
        ..zilog_silicon::renderer::CrtSettings::default()
    }
}

impl Display {
    pub fn new(window: Window, crt_config: &crate::config::CrtConfig) -> Result<Display, Box<dyn Error>> {
        // The native "screen" size (see trust_80_core::video) that
        // zilog_silicon's renderer letterboxes/scales into whatever the
        // actual window size is.
        //
        // pixels_per_scanline is how many *buffer* rows make up one real
        // CRT scanline. The real TRS-80 Model I screen was 384x192 pixels
        // (192 real scanlines) - our buffer renders at SCREEN_HEIGHT (864)
        // for legible antialiased text, an 864/192 = 4.5x vertical
        // oversampling versus real hardware, the same idea as the CPC
        // renderer's own 2x (its buffer doubles vertical resolution, so
        // pixels_per_scanline=2 there). Passing 1.0 here - claiming every
        // buffer row is already a real scanline - made them razor-thin
        // relative to the actual output size and the shader's scanline
        // effect nearly invisible regardless of scanline_beam/strength
        // (see renderer_crt.wgsl's own comment on line_height for exactly
        // this failure mode).
        let mut renderer = Renderer::new(window, SCREEN_WIDTH, SCREEN_HEIGHT, 4.5)?;

        // A saved [crt] section (F6's "Save" button) overrides this tuned
        // baseline, field by field - see `crt_settings_from_config`.
        renderer.set_crt_settings(crate::config::crt_settings_from_config(
            crt_config,
            tuned_crt_defaults(),
        ));

        Ok(Display {
            renderer,
            frame: vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT * 3],
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
            (SCREEN_WIDTH as f32 * factor) as u32,
            (SCREEN_HEIGHT as f32 * factor) as u32,
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

    pub fn update(&mut self, bus: &TrsBus) -> Result<(), Box<dyn std::error::Error>> {
        self.draw(bus);

        if self.crt_panel_visible {
            let mut settings = self.renderer.crt_settings();
            let mut open = true;
            let mut requested_zoom: Option<DisplayMode> = None;
            let mut save_requested = false;
            let current_zoom = self.current_zoom;
            // Real window size, read directly from SDL rather than egui's
            // own state this frame (same reasoning as zilog_silicon's
            // config_panel.rs, whose F6-equivalent this doesn't reuse - see
            // that file's own comment: reading the SDL size instead of
            // egui's avoids a frame of lag right after a zoom change).
            // Without this, the panel stayed a fixed, small size in
            // fullscreen/high zoom instead of scaling up with the window -
            // the F1-F4 zoom buttons and CRT sliders became disproportionately
            // tiny relative to the available "real estate" on a large or
            // high-DPI screen.
            let window_size = {
                let (w, h) = self.renderer.window().drawable_size();
                egui::vec2(w as f32, h as f32)
            };
            let scale = zilog_silicon::ui_scale::content_scale(window_size);
            {
                let mut overlay = |ctx: &egui::Context| {
                    egui::Window::new("Display")
                        .open(&mut open)
                        .default_width(260.0 * scale)
                        .show(ctx, |ui| {
                        ui.set_style(zilog_silicon::ui_scale::scaled_style(ui.style(), scale));
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
                        ui.label(format!("Current zoom: {}", current_zoom.as_config_str()));

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
                        ui.horizontal(|ui| {
                            if ui.button("Reset to defaults").clicked() {
                                settings = tuned_crt_defaults();
                            }
                            // One button for everything currently in this
                            // panel (zoom + CRT shader) - saving zoom alone
                            // while the CRT sliders were adjusted but not
                            // saved anywhere used to be the only option.
                            if ui.button("Save").clicked() {
                                save_requested = true;
                            }
                        });
                    });
                };
                self.renderer.present(&self.frame, Some(&mut overlay));
            }
            self.renderer.set_crt_settings(settings);
            self.crt_panel_visible = open;
            if let Some(mode) = requested_zoom {
                self.set_zoom(mode);
            }
            if save_requested {
                let display_config = crate::config::ScreenConfig {
                    default_zoom: Some(self.current_zoom.as_config_str().to_string()),
                };
                if let Err(e) = crate::config::save_display_config(&display_config) {
                    eprintln!("Can't save display settings: {e}");
                }
                let crt_config = crate::config::crt_settings_to_config(self.renderer.crt_settings());
                if let Err(e) = crate::config::save_crt_config(&crt_config) {
                    eprintln!("Can't save CRT settings: {e}");
                }
            }
        } else {
            self.renderer.present(&self.frame, None);
        }
        Ok(())
    }

    fn draw(&mut self, bus: &TrsBus) {
        let bytes = bus.read_mem_slice(0x3C00, 0x4000);

        for row in 0..ROWS {
            let row_bytes = &bytes[row * video::COLS..(row + 1) * video::COLS];
            let unchanged = self
                .last_vram
                .as_deref()
                .is_some_and(|prev| &prev[row * video::COLS..(row + 1) * video::COLS] == row_bytes);
            if unchanged {
                continue;
            }
            video::render_row(bus, row, &mut self.frame);
        }
        self.last_vram = Some(bytes);
    }
}
