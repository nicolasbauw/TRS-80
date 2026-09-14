//! SDL2-facing keyboard adapter: translates SDL2 events into
//! `trust_80_core::keys::Keycode` and drives a core `Keyboard`. The matrix
//! mapping itself, and everything else about how the TRS-80 keyboard
//! actually behaves, lives in the core crate - this file's only job is the
//! SDL2 event plumbing, so a future non-SDL2 frontend doesn't need any of it.

use std::collections::HashMap;
use trust_80_core::keyboard::Keyboard as CoreKeyboard;
use trust_80_core::keys::Keycode;

/// Translates an SDL2 keycode into the core's own host-independent
/// `Keycode`. `None` for anything the TRS-80 matrix has no use for (the
/// very large majority of SDL2's keycodes).
///
/// Only used directly for keys that never produce composed text (arrows,
/// Return, Backspace, modifiers, the numpad...) and as a same-frame
/// fallback for text-producing keys, in case SDL doesn't follow up with a
/// `TextInput` event for some reason - see `handle_event`'s own doc
/// comment for why letters/digits/symbols don't trust this alone anymore.
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

/// `true` for the keys whose SDL `Keycode` isn't trustworthy on its own -
/// see `handle_event`'s doc comment. The numpad is deliberately excluded
/// even though it's "text-producing" with Num Lock on: its `Keycode`
/// already identifies the physical key directly (no layout to resolve),
/// and routing it through `TextInput` would only risk losing that
/// distinction (`Kp3`'s "3" would resolve to plain `Num3` instead) for no
/// benefit - they happen to land on the same matrix position either way,
/// but there's no reason to rely on that coincidence.
fn is_text_key(k: Keycode) -> bool {
    use Keycode::*;
    matches!(
        k,
        A | B
            | C
            | D
            | E
            | F
            | G
            | H
            | I
            | J
            | K
            | L
            | M
            | N
            | O
            | P
            | Q
            | R
            | S
            | T
            | U
            | V
            | W
            | X
            | Y
            | Z
            | Num0
            | Num1
            | Num2
            | Num3
            | Num4
            | Num5
            | Num6
            | Num7
            | Num8
            | Num9
            | At
            | Exclaim
            | Quotedbl
            | Hash
            | Dollar
            | Percent
            | Ampersand
            | Quote
            | LeftParen
            | RightParen
            | Colon
            | Asterisk
            | Semicolon
            | Plus
            | Comma
            | Less
            | Minus
            | Equals
            | Period
            | Greater
            | Slash
            | Question
    )
}

/// Translates the text SDL's `TextInput` event actually composed for a key
/// press into a core `Keycode` - the character truly produced by the
/// active layout, unlike SDL's own `Keycode` for these same keys (see
/// `handle_event`). `None` for anything not on the TRS-80's ASCII-only
/// keyboard (accented letters and the like - same limitation
/// trust-80-web's own browser-based mapping has, for the same reason).
fn from_text(text: &str) -> Option<Keycode> {
    let mut chars = text.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return None;
    };
    if c.is_ascii_alphabetic() {
        return Some(match c.to_ascii_uppercase() {
            'A' => Keycode::A,
            'B' => Keycode::B,
            'C' => Keycode::C,
            'D' => Keycode::D,
            'E' => Keycode::E,
            'F' => Keycode::F,
            'G' => Keycode::G,
            'H' => Keycode::H,
            'I' => Keycode::I,
            'J' => Keycode::J,
            'K' => Keycode::K,
            'L' => Keycode::L,
            'M' => Keycode::M,
            'N' => Keycode::N,
            'O' => Keycode::O,
            'P' => Keycode::P,
            'Q' => Keycode::Q,
            'R' => Keycode::R,
            'S' => Keycode::S,
            'T' => Keycode::T,
            'U' => Keycode::U,
            'V' => Keycode::V,
            'W' => Keycode::W,
            'X' => Keycode::X,
            'Y' => Keycode::Y,
            'Z' => Keycode::Z,
            _ => unreachable!("is_ascii_alphabetic just confirmed this is A-Z"),
        });
    }
    Some(match c {
        '0' => Keycode::Num0,
        '1' => Keycode::Num1,
        '2' => Keycode::Num2,
        '3' => Keycode::Num3,
        '4' => Keycode::Num4,
        '5' => Keycode::Num5,
        '6' => Keycode::Num6,
        '7' => Keycode::Num7,
        '8' => Keycode::Num8,
        '9' => Keycode::Num9,
        '@' => Keycode::At,
        '!' => Keycode::Exclaim,
        '"' => Keycode::Quotedbl,
        '#' => Keycode::Hash,
        '$' => Keycode::Dollar,
        '%' => Keycode::Percent,
        '&' => Keycode::Ampersand,
        '\'' => Keycode::Quote,
        '(' => Keycode::LeftParen,
        ')' => Keycode::RightParen,
        ':' => Keycode::Colon,
        '*' => Keycode::Asterisk,
        ';' => Keycode::Semicolon,
        '+' => Keycode::Plus,
        ',' => Keycode::Comma,
        '<' => Keycode::Less,
        '-' => Keycode::Minus,
        '=' => Keycode::Equals,
        '.' => Keycode::Period,
        '>' => Keycode::Greater,
        '/' => Keycode::Slash,
        '?' => Keycode::Question,
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
    // Physical key -> the core `Keycode` actually asserted in the matrix
    // for it, so `KeyUp` releases exactly what was pressed - necessary
    // because for text-producing keys, that `Keycode` may only become
    // known once a `TextInput` event arrives, after `KeyDown` itself (see
    // `handle_event`).
    held: HashMap<sdl2::keyboard::Scancode, Keycode>,
    // A `KeyDown` for a text-producing key, not yet committed to `held` -
    // waiting on this same frame's `TextInput` to resolve it (or, failing
    // that, forced through on the next event using `fallback`).
    pending: Option<Pending>,
}

struct Pending {
    scancode: sdl2::keyboard::Scancode,
    fallback: Keycode,
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard {
            inner: CoreKeyboard::new(),
            shift_held: false,
            start: std::time::Instant::now(),
            held: HashMap::new(),
            pending: None,
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
    // SDL2 didn't have). Only applied to the same-frame `TextInput`
    // fallback now (see `handle_event`) - the normal path resolves this
    // key correctly through composed text instead of SDL's own `Keycode`.
    fn fixup_less_greater(&self, k: Keycode) -> Keycode {
        if k == Keycode::Less && self.shift_held {
            Keycode::Greater
        } else {
            k
        }
    }

    /// Commits a resolved key press: records it in `held` (for `KeyUp` to
    /// find later) and asserts it in the matrix.
    fn commit(&mut self, scancode: sdl2::keyboard::Scancode, k: Keycode, now: f64) {
        if k == Keycode::LShift || k == Keycode::RShift {
            self.shift_held = true;
        }
        self.held.insert(scancode, k);
        self.inner.key_down(k, now);
    }

    /// If a `KeyDown` is still waiting on a `TextInput` that never came
    /// (Ctrl/Alt held - text input is suppressed for those combinations on
    /// most platforms - or simply no matching text for this key/layout),
    /// forces it through with the SDL `Keycode`-based guess instead. Called
    /// at the start of handling any new event: SDL always delivers
    /// `TextInput` immediately after its `KeyDown`, in the same batch, so
    /// anything still pending by the time another event arrives isn't
    /// getting one.
    fn flush_pending(&mut self) {
        if let Some(p) = self.pending.take() {
            let now = self.now();
            let k = self.fixup_less_greater(p.fallback);
            self.commit(p.scancode, k, now);
        }
    }

    // sdl2's KeyboardState::pressed_scancodes() scans SDL's whole internal
    // key state array and decodes every raw value into a Scancode; under
    // sdl2-compat (SDL3 underneath) that array is sized/populated
    // differently and contains values the sdl2 crate's SDL2-only enum
    // doesn't recognize, which panics. Tracking press/release from the
    // event stream instead sidesteps that array entirely.
    //
    // Letters/digits/symbols specifically don't trust SDL's own `Keycode`
    // for what character a key produces: on AZERTY under sdl2-compat, the
    // physical key that types `"` (the unshifted "3" key) was observed
    // reporting SDL `Keycode::Num3` - the *positional* digit-row identity,
    // not the character AZERTY actually assigns that key when unshifted -
    // silently typing "3" instead of `"`. This isn't one isolated key like
    // the "< >" case below: it plausibly affects the whole AZERTY digit
    // row (`&éÙ'(-èÙçà`), each sharing a key with a digit. SDL's
    // `TextInput` event carries the text the OS actually composed for a
    // keypress - the same source of truth trust-80-web's own browser-based
    // `KeyboardEvent.key` mapping already relies on - so a text-producing
    // `KeyDown` is deferred (`pending`) until either that `TextInput`
    // arrives (the normal case) or another event proves it isn't coming
    // (`flush_pending`, using SDL's `Keycode` as a same-frame guess).
    pub fn handle_event(&mut self, event: &sdl2::event::Event) {
        match event {
            sdl2::event::Event::KeyDown {
                keycode: Some(k),
                scancode: Some(scancode),
                ..
            } => {
                self.flush_pending();
                let Some(k) = from_sdl(*k) else { return };
                if is_text_key(k) {
                    self.pending = Some(Pending {
                        scancode: *scancode,
                        fallback: k,
                    });
                } else {
                    let now = self.now();
                    self.commit(*scancode, k, now);
                }
            }
            sdl2::event::Event::TextInput { text, .. } => {
                if let Some(p) = self.pending.take() {
                    let k = from_text(text).unwrap_or(p.fallback);
                    let now = self.now();
                    self.commit(p.scancode, k, now);
                }
            }
            sdl2::event::Event::KeyUp {
                scancode: Some(scancode),
                ..
            } => {
                self.flush_pending();
                let Some(k) = self.held.remove(scancode) else {
                    return;
                };
                if k == Keycode::LShift || k == Keycode::RShift {
                    self.shift_held = false;
                }
                self.inner.key_up(k);
            }
            // Losing focus (alt-tab, clicking another window...) means no
            // KeyUp will ever arrive for whatever was held at that point -
            // without this, that key reads as permanently pressed until
            // the user happens to press and release it again.
            sdl2::event::Event::Window {
                win_event: sdl2::event::WindowEvent::FocusLost,
                ..
            } => {
                self.pending = None;
                self.held.clear();
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
