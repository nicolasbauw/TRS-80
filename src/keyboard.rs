//! SDL2-facing keyboard adapter: translates SDL2 events into
//! `trust_80_core::keys::Keycode` and drives a core `Keyboard`. The matrix
//! mapping itself, and everything else about how the TRS-80 keyboard
//! actually behaves, lives in the core crate - this file's only job is the
//! SDL2 event plumbing, so a future non-SDL2 frontend doesn't need any of it.

use trust_80_core::keyboard::Keyboard as CoreKeyboard;
use trust_80_core::keys::Keycode;

/// Translates an SDL2 keycode - the character it produces, already
/// layout/shift-resolved by the host - into the core's own
/// host-independent `Keycode`. `None` for anything the TRS-80 matrix has no
/// use for (the very large majority of SDL2's keycodes).
fn from_sdl(k: sdl2::keyboard::Keycode) -> Option<Keycode> {
    use sdl2::keyboard::Keycode as Sdl;
    Some(match k {
        Sdl::A => Keycode::A,
        Sdl::B => Keycode::B,
        Sdl::C => Keycode::C,
        Sdl::D => Keycode::D,
        Sdl::E => Keycode::E,
        Sdl::F => Keycode::F,
        Sdl::G => Keycode::G,
        Sdl::H => Keycode::H,
        Sdl::I => Keycode::I,
        Sdl::J => Keycode::J,
        Sdl::K => Keycode::K,
        Sdl::L => Keycode::L,
        Sdl::M => Keycode::M,
        Sdl::N => Keycode::N,
        Sdl::O => Keycode::O,
        Sdl::P => Keycode::P,
        Sdl::Q => Keycode::Q,
        Sdl::R => Keycode::R,
        Sdl::S => Keycode::S,
        Sdl::T => Keycode::T,
        Sdl::U => Keycode::U,
        Sdl::V => Keycode::V,
        Sdl::W => Keycode::W,
        Sdl::X => Keycode::X,
        Sdl::Y => Keycode::Y,
        Sdl::Z => Keycode::Z,
        Sdl::Num0 => Keycode::Num0,
        Sdl::Num1 => Keycode::Num1,
        Sdl::Num2 => Keycode::Num2,
        Sdl::Num3 => Keycode::Num3,
        Sdl::Num4 => Keycode::Num4,
        Sdl::Num5 => Keycode::Num5,
        Sdl::Num6 => Keycode::Num6,
        Sdl::Num7 => Keycode::Num7,
        Sdl::Num8 => Keycode::Num8,
        Sdl::Num9 => Keycode::Num9,
        Sdl::Kp0 => Keycode::Kp0,
        Sdl::Kp1 => Keycode::Kp1,
        Sdl::Kp2 => Keycode::Kp2,
        Sdl::Kp3 => Keycode::Kp3,
        Sdl::Kp4 => Keycode::Kp4,
        Sdl::Kp5 => Keycode::Kp5,
        Sdl::Kp6 => Keycode::Kp6,
        Sdl::Kp7 => Keycode::Kp7,
        Sdl::Kp8 => Keycode::Kp8,
        Sdl::Kp9 => Keycode::Kp9,
        Sdl::KpAt => Keycode::KpAt,
        Sdl::KpPercent => Keycode::KpPercent,
        Sdl::KpAmpersand => Keycode::KpAmpersand,
        Sdl::KpLeftParen => Keycode::KpLeftParen,
        Sdl::KpRightParen => Keycode::KpRightParen,
        Sdl::KpColon => Keycode::KpColon,
        Sdl::KpMultiply => Keycode::KpMultiply,
        Sdl::KpPlus => Keycode::KpPlus,
        Sdl::KpComma => Keycode::KpComma,
        Sdl::KpLess => Keycode::KpLess,
        Sdl::KpMinus => Keycode::KpMinus,
        Sdl::KpEquals => Keycode::KpEquals,
        Sdl::KpPeriod => Keycode::KpPeriod,
        Sdl::KpGreater => Keycode::KpGreater,
        Sdl::KpDivide => Keycode::KpDivide,
        Sdl::KpEnter => Keycode::KpEnter,
        Sdl::At => Keycode::At,
        Sdl::Exclaim => Keycode::Exclaim,
        Sdl::Quotedbl => Keycode::Quotedbl,
        Sdl::Hash => Keycode::Hash,
        Sdl::Dollar => Keycode::Dollar,
        Sdl::Percent => Keycode::Percent,
        Sdl::Ampersand => Keycode::Ampersand,
        Sdl::Quote => Keycode::Quote,
        Sdl::LeftParen => Keycode::LeftParen,
        Sdl::RightParen => Keycode::RightParen,
        Sdl::Colon => Keycode::Colon,
        Sdl::Asterisk => Keycode::Asterisk,
        Sdl::Semicolon => Keycode::Semicolon,
        Sdl::Plus => Keycode::Plus,
        Sdl::Comma => Keycode::Comma,
        Sdl::Less => Keycode::Less,
        Sdl::Minus => Keycode::Minus,
        Sdl::Equals => Keycode::Equals,
        Sdl::Period => Keycode::Period,
        Sdl::Greater => Keycode::Greater,
        Sdl::Slash => Keycode::Slash,
        Sdl::Question => Keycode::Question,
        Sdl::Return => Keycode::Return,
        Sdl::Home => Keycode::Home,
        Sdl::End => Keycode::End,
        Sdl::Up => Keycode::Up,
        Sdl::Down => Keycode::Down,
        Sdl::Left => Keycode::Left,
        Sdl::Right => Keycode::Right,
        Sdl::Backspace => Keycode::Backspace,
        Sdl::Space => Keycode::Space,
        Sdl::LShift => Keycode::LShift,
        Sdl::RShift => Keycode::RShift,
        Sdl::LCtrl => Keycode::LCtrl,
        Sdl::RAlt => Keycode::RAlt,
        _ => return None,
    })
}

pub struct Keyboard {
    inner: CoreKeyboard,
    shift_held: bool,
    // Core's own clock, in seconds since this struct was created - see
    // `trust_80_core::keyboard::Keyboard::key_down`'s doc comment for why
    // core takes a caller-supplied `f64` rather than reading
    // `std::time::Instant` itself.
    start: std::time::Instant,
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard {
            inner: CoreKeyboard::new(),
            shift_held: false,
            start: std::time::Instant::now(),
        }
    }

    fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    // The ISO "< >" key (common on AZERTY and other European layouts, next
    // to left shift) doesn't reliably resolve to a shifted Keycode on every
    // platform/driver: this same physical key was observed always reporting
    // `Less`, even while shift was held, never `Greater`, under sdl2-compat
    // (SDL3 under an SDL2 shim - see the other sdl2-compat comments in this
    // codebase for the pattern of this compat layer surfacing gaps real
    // SDL2 didn't have). Since we already track physical Shift ourselves,
    // this corrects it by hand rather than trusting SDL's resolved keycode
    // for this one specific key.
    fn fixup_less_greater(&self, k: Keycode) -> Keycode {
        if k == Keycode::Less && self.shift_held {
            Keycode::Greater
        } else {
            k
        }
    }

    // sdl2's KeyboardState::pressed_scancodes() scans SDL's whole internal
    // key state array and decodes every raw value into a Scancode; under
    // sdl2-compat (SDL3 underneath) that array is sized/populated
    // differently and contains values the sdl2 crate's SDL2-only enum
    // doesn't recognize, which panics. Tracking press/release from the
    // event stream instead sidesteps that array entirely.
    pub fn handle_event(&mut self, event: &sdl2::event::Event) {
        match event {
            sdl2::event::Event::KeyDown {
                keycode: Some(k), ..
            } => {
                let Some(k) = from_sdl(*k) else { return };
                if k == Keycode::LShift || k == Keycode::RShift {
                    self.shift_held = true;
                }
                let k = self.fixup_less_greater(k);
                let now = self.now();
                self.inner.key_down(k, now);
            }
            sdl2::event::Event::KeyUp {
                keycode: Some(k), ..
            } => {
                let Some(k) = from_sdl(*k) else { return };
                if k == Keycode::LShift || k == Keycode::RShift {
                    self.shift_held = false;
                }
                if k == Keycode::Less {
                    // Whichever of the two this KeyDown was resolved to
                    // (see fixup_less_greater) - shift may have been
                    // released before this key, so its *current* state
                    // can't be trusted to say which one was inserted.
                    self.inner.key_up(Keycode::Less);
                    self.inner.key_up(Keycode::Greater);
                } else {
                    self.inner.key_up(k);
                }
            }
            // Losing focus (alt-tab, clicking another window...) means no
            // KeyUp will ever arrive for whatever was held at that point -
            // without this, that key reads as permanently pressed until
            // the user happens to press and release it again.
            sdl2::event::Event::Window {
                win_event: sdl2::event::WindowEvent::FocusLost,
                ..
            } => {
                self.inner.clear();
                self.shift_held = false;
            }
            _ => {}
        }
    }

    pub fn update(&mut self, bus: &mut trust_80_core::bus::TrsBus) {
        let now = self.now();
        self.inner.update(bus, now);
    }
}
