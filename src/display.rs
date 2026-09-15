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
    keyboard_panel: crate::keyboard_panel::KeyboardPanel,
    keyboard_panel_visible: bool,
    // Changed each time the panel is (re)opened - part of its egui window
    // id, so its default position/size is recomputed on every reopen
    // instead of staying at wherever it first opened for the whole session
    // (egui remembers geometry per id, even while hidden) - same mechanism
    // as bytebox's own `keyboard_panel_generation`.
    keyboard_panel_generation: u64,
    // Previous frame's virtual-keyboard `Keycode`s, to diff against this
    // frame's and only send `virtual_key_down`/`virtual_key_up` for what
    // actually changed - see `KeyboardPanel::ui`'s own doc comment for why
    // this also has to run unconditionally (not just while the panel is
    // visible), so closing it releases anything still held.
    virtual_keyboard_pressed: std::collections::HashSet<trust_80_core::keys::Keycode>,
    keyboard_settings: crate::keyboard_panel::KeyboardSettings,
    // Same mechanism as `keyboard_panel_generation`, for the "Display" (F6)
    // window - see `resize()` for why both need bumping on a live resize,
    // not just on reopening.
    crt_panel_generation: u64,
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
        // NTSC-exact (see the comment on the `Renderer::new` call below):
        // 480 active lines / 192 TRS-80 logical lines.
        pixels_per_scanline: NTSC_THEORETICAL_PIXELS_PER_SCANLINE,
        ..zilog_silicon::renderer::CrtSettings::default()
    }
}

/// Theoretical value of `pixels_per_scanline` for an NTSC screen, for the F6
/// panel's marker button - see `tuned_crt_defaults`'s doc comment for the
/// calculation.
pub const NTSC_THEORETICAL_PIXELS_PER_SCANLINE: f32 = 2.5;

impl Display {
    pub fn new(
        window: Window,
        crt_config: &crate::config::CrtConfig,
        keyboard_config: &crate::config::KeyboardConfig,
    ) -> Result<Display, Box<dyn Error>> {
        // The native "screen" size (see trust_80_core::video) that
        // zilog_silicon's renderer letterboxes/scales into whatever the
        // actual window size is.
        //
        // pixels_per_scanline is how many *buffer* rows make up one real
        // CRT scanline. The real TRS-80 Model I screen was 384x192 pixels
        // (192 real scanlines), i.e. 864/192 = 4.5x vertical oversampling
        // for our own SCREEN_HEIGHT (864) - but since the shader now
        // samples displayed content at full native resolution regardless
        // of this value (see renderer_crt.wgsl's sample_native/scan_factor
        // split), it's free to be tuned for how convincing the scanlines
        // LOOK, not just hardware accuracy. 4.5 (one real monitor scanline
        // per TRS-80 pixel row) produced scanlines noticeably thicker/
        // coarser than bytebox's own (2.0) at the same window size.
        //
        // 2.5, not a clean divisor of 4.5: a real CRT's raster runs at a
        // fixed frequency that has no reason to line up with the source
        // computer's own logical pixel grid - the TRS-80 (like most 8-bit
        // micros of the era) doesn't genuinely interlace, it just feeds the
        // same content to both NTSC fields, so its ~192 logical lines
        // really do land arbitrarily across NTSC's ~480 active scanlines
        // (480/192 = 2.5, exactly this value) with no phase alignment to
        // speak of. What first looked like a bug (the scanline phase
        // "drifting" against the font's pixel grid at 2.5, unlike a clean
        // divisor such as 2.25) is arguably the physically accurate
        // behavior, not an artifact to avoid.
        // Passing 1.0 - claiming every buffer row is already a real
        // scanline - made them razor-thin relative to the actual output
        // size and the shader's scanline effect nearly invisible
        // regardless of scanline_beam/strength (see renderer_crt.wgsl's
        // own comment on line_height for exactly this failure mode).
        let mut renderer = Renderer::new(window, SCREEN_WIDTH, SCREEN_HEIGHT)?;

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
            keyboard_panel: crate::keyboard_panel::KeyboardPanel::new(),
            keyboard_panel_visible: false,
            keyboard_panel_generation: 0,
            virtual_keyboard_pressed: std::collections::HashSet::new(),
            keyboard_settings: crate::keyboard_panel::KeyboardSettings::from_config(
                keyboard_config,
            ),
            crt_panel_generation: 0,
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
        // A resize while a panel is already open leaves its `egui::Window`
        // at its previously memorized position/size (egui keys that by id,
        // and the id doesn't otherwise change) - bumping the generation
        // here gives it a fresh id, so it re-evaluates `default_pos`/
        // `default_width` against the new window size next frame, exactly
        // like reopening it would. Only when actually visible: bumping
        // while closed is pointless (nothing to move) and would just waste
        // the F6/F7-triggered bump's own effect the next time it opens -
        // same reasoning as bytebox's own `sdl.rs` (its own comment on
        // this exact ordering).
        if self.crt_panel_visible {
            self.crt_panel_generation += 1;
        }
        if self.keyboard_panel_visible {
            self.keyboard_panel_generation += 1;
        }
    }

    pub fn handle_event(&mut self, event: &sdl2::event::Event) {
        self.renderer.handle_event(event);
    }

    pub fn toggle_crt(&mut self) {
        self.renderer.toggle_crt();
    }

    pub fn toggle_crt_panel(&mut self) {
        self.crt_panel_visible = !self.crt_panel_visible;
        if self.crt_panel_visible {
            self.crt_panel_generation += 1;
        }
    }

    pub fn toggle_keyboard_panel(&mut self) {
        self.keyboard_panel_visible = !self.keyboard_panel_visible;
        if self.keyboard_panel_visible {
            self.keyboard_panel_generation += 1;
        }
    }

    pub fn update(
        &mut self,
        bus: &TrsBus,
        keyboard: &mut crate::keyboard::Keyboard,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.draw(bus);

        // Populated inside the overlay closure below only if the keyboard
        // panel is actually visible this frame - stays empty otherwise, so
        // the diff against `self.virtual_keyboard_pressed` after the
        // if/else naturally releases anything still held the moment the
        // panel closes (see `KeyboardPanel::ui`'s own doc comment).
        let mut new_virtual_keys = std::collections::HashSet::new();

        if self.crt_panel_visible || self.keyboard_panel_visible {
            let mut settings = self.renderer.crt_settings();
            let crt_panel_visible = self.crt_panel_visible;
            let keyboard_panel_visible = self.keyboard_panel_visible;
            // Only a window actually shown this frame can flip its own
            // `open` back to false (its egui close button) - starting both
            // at their CURRENT visibility (not unconditionally `true`)
            // means a panel that isn't shown this frame doesn't get its
            // visibility silently reset to true below.
            let mut open = crt_panel_visible;
            let mut keyboard_open = keyboard_panel_visible;
            let crt_panel_generation = self.crt_panel_generation;
            let keyboard_panel_generation = self.keyboard_panel_generation;
            let keyboard_panel = &mut self.keyboard_panel;
            let mut keyboard_settings = self.keyboard_settings;
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
                    if crt_panel_visible {
                    egui::Window::new("Display")
                        .id(egui::Id::new(("display_panel_window", crt_panel_generation)))
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
                        ui.horizontal(|ui| {
                            let mut percent = keyboard_settings.default_size_percent * 100.0;
                            let response =
                                ui.add(egui::Slider::new(&mut percent, 10.0..=100.0).suffix(" %"));
                            ui.label("Virtual keyboard (F7) default size");
                            if response.changed() {
                                keyboard_settings.default_size_percent = percent / 100.0;
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
                        ui.horizontal(|ui| {
                            // `settings.pixels_per_scanline` is the raw
                            // shader parameter (`line_height`: how many
                            // *buffer* rows make up one real CRT scanline) -
                            // bigger means FEWER, coarser scanlines, the
                            // opposite of what the number suggests. Unlike
                            // bytebox's own CPC-specific correction (its
                            // buffer duplicates each raster line, a known
                            // 2x factor), the TRS-80's 2.5 default was
                            // deliberately chosen as-is, NOT as a corrected
                            // hardware ratio (see `tuned_crt_defaults`'s
                            // doc comment) - so the slider just shows the
                            // plain reciprocal (scanlines per buffer row),
                            // fixing the direction without pretending to a
                            // physical unit it doesn't have.
                            let mut scanlines_per_row = 1.0 / settings.pixels_per_scanline;
                            let response = ui.add(
                                egui::Slider::new(&mut scanlines_per_row, 0.2..=1.0)
                                    .text("Scanline fineness (higher = more, thinner scanlines)"),
                            );
                            if response.changed() {
                                settings.pixels_per_scanline = 1.0 / scanlines_per_row;
                            }
                            let ntsc_scanlines_per_row = 1.0 / NTSC_THEORETICAL_PIXELS_PER_SCANLINE;
                            if ui
                                .button("🎯 NTSC")
                                .on_hover_text(format!(
                                    "Theoretical value used for the NTSC-tuned default: {ntsc_scanlines_per_row:.2}"
                                ))
                                .clicked()
                            {
                                settings.pixels_per_scanline = NTSC_THEORETICAL_PIXELS_PER_SCANLINE;
                            }
                        });
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
                    }
                    if keyboard_panel_visible {
                        new_virtual_keys = keyboard_panel.ui(
                            ctx,
                            &mut keyboard_open,
                            keyboard_panel_generation,
                            window_size,
                            keyboard_settings,
                        );
                    }
                };
                self.renderer.present(&self.frame, Some(&mut overlay));
            }
            self.renderer.set_crt_settings(settings);
            self.crt_panel_visible = open;
            self.keyboard_panel_visible = keyboard_open;
            self.keyboard_settings = keyboard_settings;
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
                let keyboard_config = self.keyboard_settings.to_config();
                if let Err(e) = crate::config::save_keyboard_config(&keyboard_config) {
                    eprintln!("Can't save keyboard settings: {e}");
                }
            }
        } else {
            self.renderer.present(&self.frame, None);
        }

        // Unconditional (not just while the panel is visible - see
        // `KeyboardPanel::ui`'s doc comment): released `Keycode`s must
        // still reach the core the frame the panel closes, or F7 could
        // leave a key stuck held forever.
        for &kc in self.virtual_keyboard_pressed.difference(&new_virtual_keys) {
            keyboard.virtual_key_up(kc);
        }
        for &kc in new_virtual_keys.difference(&self.virtual_keyboard_pressed) {
            keyboard.virtual_key_down(kc);
        }
        self.virtual_keyboard_pressed = new_virtual_keys;

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
