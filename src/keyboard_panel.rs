//! Virtual keyboard (F7): the stylized illustration of the TRS-80 keyboard
//! (`assets/trs-80-keyboard.png`), with a clickable zone per key mapped
//! **directly** to a `trust_80_core::keys::Keycode` - driven through
//! `crate::keyboard::Keyboard::virtual_key_down`/`virtual_key_up`, which
//! shares the exact same underlying core `Keyboard` (and therefore matrix
//! state) as physical typing. Same overall approach as bytebox's own F7
//! virtual keyboard (`amstrad_cpc/bytebox/src/keyboard_panel.rs`), adapted
//! to this core's simpler `Keycode`-level API (no raw matrix line/bit/PSG
//! position to reason about here).
//!
//! The rectangles below were measured on the source image (2046x768) via a
//! one-off grid-overlay tool (not committed), to within a few pixels: this
//! artwork has soft, photographic-style bevels and drop shadows rather than
//! bytebox's flat vector illustration, so edges aren't as crisp - a small
//! margin of error in a click region is harmless here, unlike bytebox's own
//! pixel-exact approach.
//!
//! BREAK and CLEAR are wired to `Keycode::Break`/`Keycode::Clear`, which
//! share their matrix position with `Home`/`End` respectively (see
//! `key_target`'s own doc comment, core/src/keyboard.rs): on real Model I
//! hardware, that position IS CLEAR/BREAK - Home/End were only ever a
//! modern-keyboard convenience mapped onto it, since a PC keyboard has no
//! CLEAR/BREAK of its own. This panel exposes the real legends too, for
//! completeness, even though neither has a host key to trigger it from.
//!
//! SHIFT is UI-only, never sent to the core directly (mirrors the core's
//! own `key_target` doc comment: the core deliberately has no matrix
//! target for `LShift`/`RShift`, since two characters commonly share one
//! matrix cell and are disambiguated only by which resolved `Keycode` is
//! sent, e.g. `Keycode::Num1` vs `Keycode::Exclaim` - not by a raw shift
//! bit asserted alongside an otherwise-ambiguous one). Clicking SHIFT here
//! just flips which of a key's two `Keycode`s subsequent clicks resolve
//! to; like bytebox's own SHIFT, it's a one-shot latch (click to engage,
//! released automatically on the next non-modifier key click) since one
//! mouse can't hold SHIFT down AND click another key at the same time.

use trust_80_core::keys::Keycode;

/// What a virtual key does when clicked.
#[derive(Clone, Copy)]
enum Target {
    /// No shift ambiguity: letters, arrows, space, Return, numpad digits...
    Fixed(Keycode),
    /// Two legends on one keycap (e.g. "!" over "1") - which `Keycode` is
    /// sent depends on whether SHIFT is latched at the moment of the click.
    Shiftable(Keycode, Keycode),
    /// SHIFT itself: UI-only, see the module doc comment.
    Shift,
}

/// Size of the source image, in pixels - reference for every rectangle
/// below, independent of the panel's current on-screen size (rescaled each
/// frame in `ui()`, like bytebox's own panel).
const IMAGE_SIZE: egui::Vec2 = egui::vec2(2046.0, 768.0);

struct VirtualKey {
    rect: egui::Rect,
    target: Target,
}

const fn k(x0: f32, y0: f32, x1: f32, y1: f32, target: Target) -> VirtualKey {
    VirtualKey {
        rect: egui::Rect {
            min: egui::pos2(x0, y0),
            max: egui::pos2(x1, y1),
        },
        target,
    }
}

const fn fixed(x0: f32, y0: f32, x1: f32, y1: f32, kc: Keycode) -> VirtualKey {
    k(x0, y0, x1, y1, Target::Fixed(kc))
}

const fn shiftable(x0: f32, y0: f32, x1: f32, y1: f32, unshifted: Keycode, shifted: Keycode) -> VirtualKey {
    k(x0, y0, x1, y1, Target::Shiftable(unshifted, shifted))
}

const fn shift(x0: f32, y0: f32, x1: f32, y1: f32) -> VirtualKey {
    k(x0, y0, x1, y1, Target::Shift)
}

#[rustfmt::skip]
const KEYS: &[VirtualKey] = &[
    // Row 1 (y 160-245): digits/symbols, BREAK.
    shiftable(173.0, 160.0, 255.0, 245.0, Keycode::Num1, Keycode::Exclaim),
    shiftable(272.0, 160.0, 354.0, 245.0, Keycode::Num2, Keycode::Quotedbl),
    shiftable(371.0, 160.0, 453.0, 245.0, Keycode::Num3, Keycode::Hash),
    shiftable(470.0, 160.0, 552.0, 245.0, Keycode::Num4, Keycode::Dollar),
    shiftable(569.0, 160.0, 651.0, 245.0, Keycode::Num5, Keycode::Percent),
    shiftable(668.0, 160.0, 750.0, 245.0, Keycode::Num6, Keycode::Ampersand),
    shiftable(767.0, 160.0, 849.0, 245.0, Keycode::Num7, Keycode::Quote),
    shiftable(866.0, 160.0, 948.0, 245.0, Keycode::Num8, Keycode::LeftParen),
    shiftable(965.0, 160.0, 1047.0, 245.0, Keycode::Num9, Keycode::RightParen),
    fixed(1064.0, 160.0, 1146.0, 245.0, Keycode::Num0),
    shiftable(1163.0, 160.0, 1245.0, 245.0, Keycode::Colon, Keycode::Asterisk),
    shiftable(1262.0, 160.0, 1344.0, 245.0, Keycode::Minus, Keycode::Equals),
    fixed(1360.0, 160.0, 1450.0, 245.0, Keycode::Break),

    // Row 2 (y 250-345): arrow-up, QWERTYUIOP, @, left/right arrows.
    // Unlike row 1, re-measured directly (a per-row luminance-plateau scan,
    // not a shared start/pitch assumption carried over from row 1) - this
    // artwork's rows are staggered horizontally relative to each other,
    // like a real keyboard, not column-aligned. Height corrected to match
    // the other rows' more generous margins - the first pass here only
    // spanned 262-327 (65px), visibly tighter than every other row (85-100px)
    // once those were fixed.
    fixed(138.0, 250.0, 211.0, 345.0, Keycode::Up),
    fixed(236.0, 250.0, 309.0, 345.0, Keycode::Q),
    fixed(333.0, 250.0, 405.0, 345.0, Keycode::W),
    fixed(431.0, 250.0, 502.0, 345.0, Keycode::E),
    fixed(529.0, 250.0, 601.0, 345.0, Keycode::R),
    fixed(628.0, 250.0, 698.0, 345.0, Keycode::T),
    fixed(725.0, 250.0, 797.0, 345.0, Keycode::Y),
    fixed(823.0, 250.0, 894.0, 345.0, Keycode::U),
    fixed(922.0, 250.0, 993.0, 345.0, Keycode::I),
    fixed(1021.0, 250.0, 1092.0, 345.0, Keycode::O),
    fixed(1120.0, 250.0, 1191.0, 345.0, Keycode::P),
    fixed(1220.0, 250.0, 1290.0, 345.0, Keycode::At),
    // "←" doubles as Backspace on this core (they share a matrix target -
    // see key_target's doc comment) - sent as Backspace, not Left, so
    // holding it down repeats like a real delete key (see
    // BACKSPACE_REPEAT_DELAY_SECS in core/src/keyboard.rs); Keycode::Left
    // has no such repeat.
    fixed(1319.0, 250.0, 1389.0, 345.0, Keycode::Backspace),
    fixed(1420.0, 250.0, 1498.0, 345.0, Keycode::Right),

    // Row 3 (y 352-452): arrow-down, ASDFGHJKL, ;/+, ENTER, CLEAR.
    // Re-measured directly, same reason as row 2.
    fixed(162.0, 352.0, 240.0, 452.0, Keycode::Down),
    fixed(263.0, 352.0, 337.0, 452.0, Keycode::A),
    fixed(364.0, 352.0, 436.0, 452.0, Keycode::S),
    fixed(461.0, 352.0, 533.0, 452.0, Keycode::D),
    fixed(559.0, 352.0, 632.0, 452.0, Keycode::F),
    fixed(658.0, 352.0, 731.0, 452.0, Keycode::G),
    fixed(759.0, 352.0, 829.0, 452.0, Keycode::H),
    fixed(856.0, 352.0, 927.0, 452.0, Keycode::J),
    fixed(954.0, 352.0, 1026.0, 452.0, Keycode::K),
    fixed(1054.0, 352.0, 1125.0, 452.0, Keycode::L),
    shiftable(1154.0, 352.0, 1225.0, 452.0, Keycode::Semicolon, Keycode::Plus),
    fixed(1245.0, 352.0, 1431.0, 452.0, Keycode::Return),
    fixed(1450.0, 352.0, 1540.0, 452.0, Keycode::Clear),

    // Row 4 (y 460-548): SHIFT, ZXCVBNM, ,/< ./> //?, SHIFT. Re-measured
    // directly, same reason as row 2.
    shift(162.0, 460.0, 290.0, 548.0),
    fixed(320.0, 460.0, 390.0, 548.0, Keycode::Z),
    fixed(421.0, 460.0, 489.0, 548.0, Keycode::X),
    fixed(515.0, 460.0, 587.0, 548.0, Keycode::C),
    fixed(612.0, 460.0, 685.0, 548.0, Keycode::V),
    fixed(711.0, 460.0, 780.0, 548.0, Keycode::B),
    fixed(810.0, 460.0, 881.0, 548.0, Keycode::N),
    fixed(908.0, 460.0, 979.0, 548.0, Keycode::M),
    shiftable(1006.0, 460.0, 1078.0, 548.0, Keycode::Comma, Keycode::Less),
    shiftable(1104.0, 460.0, 1177.0, 548.0, Keycode::Period, Keycode::Greater),
    shiftable(1203.0, 460.0, 1276.0, 548.0, Keycode::Slash, Keycode::Question),
    shift(1307.0, 460.0, 1426.0, 548.0),

    // Row 5 (y 550-628): SPACE. Re-measured directly, same reason as row 2.
    fixed(420.0, 550.0, 1160.0, 628.0, Keycode::Space),

    // Numeric keypad - Kp* variants map to the same matrix target as their
    // main-row counterparts, see core/keyboard.rs, so this is purely
    // cosmetic fidelity to the source image. Re-measured separately from
    // the rest of the table: its keys are noticeably smaller, so the
    // shadow/bevel margin that applies to the main rows (see the module
    // doc comment) doesn't translate directly in absolute pixels here.
    fixed(1625.0, 193.0, 1705.0, 293.0, Keycode::Kp7),
    fixed(1725.0, 193.0, 1805.0, 293.0, Keycode::Kp8),
    fixed(1825.0, 193.0, 1905.0, 293.0, Keycode::Kp9),
    fixed(1625.0, 303.0, 1705.0, 393.0, Keycode::Kp4),
    fixed(1725.0, 303.0, 1805.0, 393.0, Keycode::Kp5),
    fixed(1825.0, 303.0, 1905.0, 393.0, Keycode::Kp6),
    fixed(1625.0, 403.0, 1705.0, 493.0, Keycode::Kp1),
    fixed(1725.0, 403.0, 1805.0, 493.0, Keycode::Kp2),
    fixed(1825.0, 403.0, 1905.0, 493.0, Keycode::Kp3),
    fixed(1625.0, 503.0, 1705.0, 593.0, Keycode::Kp0),
    fixed(1725.0, 503.0, 1805.0, 593.0, Keycode::KpPeriod),
    // Just the tan keycap itself, not the whole gap to the case edge -
    // unlike the other numpad keys, ENTER's true right edge is nowhere
    // near a neighboring key to accidentally bleed into, but a rect that
    // wide still felt/looked wrong to click.
    fixed(1830.0, 503.0, 1910.0, 590.0, Keycode::KpEnter),
];

/// F6 panel settings for this virtual keyboard - same idea, same default,
/// as bytebox's own `KeyboardSettings` (`amstrad_cpc/bytebox/src/
/// keyboard_panel.rs`).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct KeyboardSettings {
    /// Default size at opening, as a fraction of the main window's height -
    /// see the comment on `width_from_height_cap` in `ui()` for why height,
    /// not width: this panel is a resizable `egui::Window`, capped in BOTH
    /// dimensions so it doesn't cover the whole screen at once, and height
    /// is the more useful axis to tie that cap to (a wide window shouldn't
    /// force a huge keyboard, but a tall one should still allow a legible
    /// one).
    pub default_size_percent: f32,
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        Self {
            default_size_percent: 0.33,
        }
    }
}

impl KeyboardSettings {
    pub fn from_config(cfg: &crate::config::KeyboardConfig) -> Self {
        let d = Self::default();
        Self {
            default_size_percent: cfg.default_size_percent.unwrap_or(d.default_size_percent),
        }
    }

    pub fn to_config(self) -> crate::config::KeyboardConfig {
        crate::config::KeyboardConfig {
            default_size_percent: Some(self.default_size_percent),
        }
    }
}

pub struct KeyboardPanel {
    texture: Option<egui::TextureHandle>,
    /// Whether SHIFT is currently latched - see the module doc comment.
    shift_latched: bool,
}

impl KeyboardPanel {
    pub fn new() -> Self {
        Self {
            texture: None,
            shift_latched: false,
        }
    }

    /// Dessine le panneau ; `open` reflète et contrôle sa visibilité, comme
    /// le panneau F6 (`Display::update`). `generation` doit changer à chaque
    /// réouverture (F7) - fait partie de l'id de la fenêtre egui, pour que
    /// sa position par défaut soit recalculée à chaque fois plutôt que
    /// figée à la toute première ouverture (même mécanisme que bytebox's
    /// own `keyboard_panel.rs`).
    ///
    /// Renvoie l'ensemble des `Keycode` actuellement tenus CETTE trame par
    /// le clavier virtuel - à l'appelant de comparer avec la trame
    /// précédente et d'appeler `virtual_key_down`/`virtual_key_up` pour ce
    /// qui a changé (voir `Display::update`), plutôt que d'agir directement
    /// ici : appelé UNIQUEMENT quand le panneau est visible, un état encore
    /// "tenu" au moment où F7 referme la fenêtre resterait sinon bloqué
    /// enfoncé indéfiniment côté matrice - le même souci, et la même
    /// solution (un ensemble vide par défaut quand ce `ui()` n'est pas
    /// appelé du tout), que le clavier virtuel de bytebox (`sdl.rs`,
    /// `apply_matrix_diff`).
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        open: &mut bool,
        generation: u64,
        window_size: egui::Vec2,
        settings: KeyboardSettings,
    ) -> std::collections::HashSet<Keycode> {
        if self.texture.is_none() {
            self.texture = Self::load_texture(ctx);
        }

        let aspect = IMAGE_SIZE.y / IMAGE_SIZE.x;
        let width_from_width_cap = window_size.x * 0.9;
        let width_from_height_cap =
            (window_size.y * settings.default_size_percent.clamp(0.0, 1.0)) / aspect;
        let default_width = width_from_width_cap.min(width_from_height_cap);
        let margin = 8.0;
        let default_pos = egui::pos2(window_size.x - margin, window_size.y - margin);

        let shift_latched = self.shift_latched;
        let mut release_shift = false;
        let mut active = std::collections::HashSet::new();

        egui::Window::new("Virtual keyboard")
            .id(egui::Id::new(("keyboard_panel_window", generation)))
            .open(open)
            .resizable(true)
            .default_width(default_width)
            .pivot(egui::Align2::RIGHT_BOTTOM)
            .default_pos(default_pos)
            .show(ctx, |ui| {
                let Some(texture) = &self.texture else {
                    ui.label("Couldn't decode the embedded keyboard image — see the console.");
                    return;
                };
                let width = ui.available_width().max(1.0);
                let scale = width / IMAGE_SIZE.x;
                let display_size = IMAGE_SIZE * scale;
                let (image_rect, _) = ui.allocate_exact_size(display_size, egui::Sense::hover());
                ui.painter().image(
                    texture.id(),
                    image_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );

                for (index, key) in KEYS.iter().enumerate() {
                    let screen_rect = egui::Rect::from_min_max(
                        image_rect.min + key.rect.min.to_vec2() * scale,
                        image_rect.min + key.rect.max.to_vec2() * scale,
                    );
                    let id = ui.id().with(("virtual_key", index));
                    let response = ui.interact(screen_rect, id, egui::Sense::click());

                    let lit = match key.target {
                        Target::Shift => {
                            if response.clicked() {
                                self.shift_latched = !self.shift_latched;
                            }
                            self.shift_latched
                        }
                        Target::Fixed(kc) => {
                            let held = response.is_pointer_button_down_on();
                            if held {
                                active.insert(kc);
                            }
                            if response.clicked() {
                                release_shift = true;
                            }
                            held
                        }
                        Target::Shiftable(unshifted, shifted) => {
                            let held = response.is_pointer_button_down_on();
                            if held {
                                active.insert(if shift_latched { shifted } else { unshifted });
                            }
                            if response.clicked() {
                                release_shift = true;
                            }
                            held
                        }
                    };

                    if lit || response.hovered() {
                        ui.painter().rect_stroke(
                            screen_rect,
                            4.0,
                            egui::Stroke::new(
                                2.0_f32,
                                if lit {
                                    egui::Color32::YELLOW
                                } else {
                                    egui::Color32::from_white_alpha(120)
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            });

        if release_shift {
            self.shift_latched = false;
        }
        active
    }

    fn load_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
        match image::load_from_memory(include_bytes!("../assets/trs-80-keyboard.png")) {
            Ok(img) => {
                let img = img.into_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
                Some(ctx.load_texture(
                    "virtual_keyboard",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ))
            }
            Err(e) => {
                eprintln!("Can't load the embedded keyboard image: {e}");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Filet de sécurité contre une coquille de transcription dans le
    /// tableau `KEYS` (mesuré à la main sur l'image source) : chaque
    /// rectangle doit rester dans les limites de l'image et avoir une aire
    /// strictement positive.
    #[test]
    fn every_key_rect_stays_within_the_source_image_and_has_positive_area() {
        for (index, key) in KEYS.iter().enumerate() {
            assert!(
                key.rect.min.x >= 0.0 && key.rect.max.x <= IMAGE_SIZE.x,
                "rectangle hors limites en x pour l'index {index} : {:?}",
                key.rect
            );
            assert!(
                key.rect.min.y >= 0.0 && key.rect.max.y <= IMAGE_SIZE.y,
                "rectangle hors limites en y pour l'index {index} : {:?}",
                key.rect
            );
            assert!(
                key.rect.min.x < key.rect.max.x && key.rect.min.y < key.rect.max.y,
                "rectangle degenere pour l'index {index} : {:?}",
                key.rect
            );
        }
    }

    /// Aucun rectangle de touche ne doit chevaucher un autre : un clic
    /// irait alors à la mauvaise touche selon l'ordre d'itération.
    #[test]
    fn key_rects_never_overlap() {
        for (i, a) in KEYS.iter().enumerate() {
            for (j, b) in KEYS.iter().enumerate().skip(i + 1) {
                assert!(
                    !a.rect.intersects(b.rect),
                    "chevauchement entre l'index {i} ({:?}) et l'index {j} ({:?})",
                    a.rect,
                    b.rect
                );
            }
        }
    }
}
