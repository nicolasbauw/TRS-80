use crate::bus::TrsBus;
use crate::keys::Keycode;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use zilog_z80::bus::Bus;

// Real TRS-80 keyboard hardware had no auto-repeat: the ROM's keyboard
// scan is edge-triggered, so a key held down just reads as pressed once,
// same as here for every other key. Backspace is the one exception we
// add back as a deliberate modern convenience - holding it down should
// delete continuously, like on any keyboard since. REPEAT_DELAY is how
// long to wait before the first repeat; REPEAT_INTERVAL is the pace of
// repeats after that. The bit has to be pulsed off partway through each
// interval (not just re-asserted) so the ROM's edge-triggered scan sees a
// fresh press each time, rather than one continuous hold it only counts
// once.
const BACKSPACE_REPEAT_DELAY: Duration = Duration::from_millis(500);
const BACKSPACE_REPEAT_INTERVAL: Duration = Duration::from_millis(80);

/// TRS-80 Model I keyboard matrix: 8 memory-mapped rows, one address bit
/// per row (0x3801, 0x3802, 0x3804, 0x3808, 0x3810, 0x3820, 0x3840,
/// 0x3880), 8 columns (bits) each. Row 0x3880's only meaningful bit on
/// Model I is SHIFT.
const ROW_ADDRESSES: [u16; 8] = [
    0x3801, 0x3802, 0x3804, 0x3808, 0x3810, 0x3820, 0x3840, 0x3880,
];
const SHIFT_ADDR: u16 = 0x3880;
const SHIFT_BIT: u8 = 0x01;

/// Maps a host `Keycode` - already resolved to the character it produces,
/// independent of physical layout or which host modifier combo produced
/// it - to the TRS-80 matrix position that types the *same character*.
/// `shift` is whether the *emulated* TRS-80 shift key must be asserted to
/// get that character; it's a property of the TRS-80's own keyboard
/// legend and has nothing to do with whether the host needed shift (e.g.
/// a US host produces '(' via Shift+9, but on the TRS-80's own keyboard,
/// '(' lives on Shift+8 - this table only cares about the '(' end result,
/// so typing it on either keyboard just works).
fn key_target(k: Keycode) -> Option<(u16, u8, bool)> {
    use Keycode as K;
    Some(match k {
        K::At | K::KpAt => (0x3801, 0x01, false),
        K::A => (0x3801, 0x02, false),
        K::B => (0x3801, 0x04, false),
        K::C => (0x3801, 0x08, false),
        K::D => (0x3801, 0x10, false),
        K::E => (0x3801, 0x20, false),
        K::F => (0x3801, 0x40, false),
        K::G => (0x3801, 0x80, false),
        K::H => (0x3802, 0x01, false),
        K::I => (0x3802, 0x02, false),
        K::J => (0x3802, 0x04, false),
        K::K => (0x3802, 0x08, false),
        K::L => (0x3802, 0x10, false),
        K::M => (0x3802, 0x20, false),
        K::N => (0x3802, 0x40, false),
        K::O => (0x3802, 0x80, false),
        K::P => (0x3804, 0x01, false),
        K::Q => (0x3804, 0x02, false),
        K::R => (0x3804, 0x04, false),
        K::S => (0x3804, 0x08, false),
        K::T => (0x3804, 0x10, false),
        K::U => (0x3804, 0x20, false),
        K::V => (0x3804, 0x40, false),
        K::W => (0x3804, 0x80, false),
        K::X => (0x3808, 0x01, false),
        K::Y => (0x3808, 0x02, false),
        K::Z => (0x3808, 0x04, false),

        K::Num0 | K::Kp0 => (0x3810, 0x01, false),
        K::Num1 | K::Kp1 => (0x3810, 0x02, false),
        K::Exclaim => (0x3810, 0x02, true),
        K::Num2 | K::Kp2 => (0x3810, 0x04, false),
        K::Quotedbl => (0x3810, 0x04, true),
        K::Num3 | K::Kp3 => (0x3810, 0x08, false),
        K::Hash => (0x3810, 0x08, true),
        K::Num4 | K::Kp4 => (0x3810, 0x10, false),
        K::Dollar => (0x3810, 0x10, true),
        K::Num5 | K::Kp5 => (0x3810, 0x20, false),
        K::Percent | K::KpPercent => (0x3810, 0x20, true),
        K::Num6 | K::Kp6 => (0x3810, 0x40, false),
        K::Ampersand | K::KpAmpersand => (0x3810, 0x40, true),
        K::Num7 | K::Kp7 => (0x3810, 0x80, false),
        K::Quote => (0x3810, 0x80, true),

        K::Num8 | K::Kp8 => (0x3820, 0x01, false),
        K::LeftParen | K::KpLeftParen => (0x3820, 0x01, true),
        K::Num9 | K::Kp9 => (0x3820, 0x02, false),
        K::RightParen | K::KpRightParen => (0x3820, 0x02, true),
        K::Colon | K::KpColon => (0x3820, 0x04, false),
        K::Asterisk | K::KpMultiply => (0x3820, 0x04, true),
        K::Semicolon => (0x3820, 0x08, false),
        K::Plus | K::KpPlus => (0x3820, 0x08, true),
        K::Comma | K::KpComma => (0x3820, 0x10, false),
        K::Less | K::KpLess => (0x3820, 0x10, true),
        K::Minus | K::KpMinus => (0x3820, 0x20, false),
        K::Equals | K::KpEquals => (0x3820, 0x20, true),
        K::Period | K::KpPeriod => (0x3820, 0x40, false),
        K::Greater | K::KpGreater => (0x3820, 0x40, true),
        K::Slash | K::KpDivide => (0x3820, 0x80, false),
        K::Question => (0x3820, 0x80, true),

        K::Return | K::KpEnter => (0x3840, 0x01, false),
        K::Home => (0x3840, 0x02, false),
        K::End => (0x3840, 0x04, false),
        K::Up => (0x3840, 0x08, false),
        K::Down => (0x3840, 0x10, false),
        K::Left | K::Backspace => (0x3840, 0x20, false),
        K::Right => (0x3840, 0x40, false),
        K::Space => (0x3840, 0x80, false),

        K::LShift | K::RShift => (SHIFT_ADDR, SHIFT_BIT, false),

        _ => return None,
    })
}

// NOTE for a future wasm frontend: `Instant::now()` (used for the backspace
// auto-repeat timer below) compiles for wasm32-unknown-unknown but panics
// at runtime with no proper time source - bytebox-web works around the
// equivalent problem by deriving elapsed time from egui's own
// `ctx.input(|i| i.time)` instead of `Instant`. Not addressed here: this
// crate split is a prerequisite step, not the wasm port itself, and this
// is the one piece of code in this file with a real (if narrow) portability
// gap left to close when that work actually starts.
pub struct Keyboard {
    pressed: HashSet<Keycode>,
    backspace_pressed_at: Option<Instant>,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard {
            pressed: HashSet::new(),
            backspace_pressed_at: None,
        }
    }

    /// Registers `k` as held, until a matching `key_up`. Frontends translate
    /// their own input events (SDL2 keycodes, browser KeyboardEvents...)
    /// into `Keycode` before calling this - see e.g. the desktop app's
    /// `keyboard.rs::from_sdl`.
    pub fn key_down(&mut self, k: Keycode) {
        // Ignores the host's own OS-level typematic repeat: the repeat
        // timing below is driven by our own clock instead, so behavior
        // doesn't depend on the host's repeat-rate settings. Frontends are
        // expected to call this once per physical press, not once per
        // repeat event - a repeat calling this again is harmless either way
        // (HashSet insert, and backspace_pressed_at only ever set from None).
        if k == Keycode::Backspace && self.backspace_pressed_at.is_none() {
            self.backspace_pressed_at = Some(Instant::now());
        }
        self.pressed.insert(k);
    }

    pub fn key_up(&mut self, k: Keycode) {
        if k == Keycode::Backspace {
            self.backspace_pressed_at = None;
        }
        self.pressed.remove(&k);
    }

    /// Releases every held key - e.g. on losing window focus (alt-tab,
    /// clicking another window...), when no `key_up` will ever arrive for
    /// whatever was held at that point and it would otherwise read as
    /// permanently pressed until physically pressed and released again.
    pub fn clear(&mut self) {
        self.pressed.clear();
        self.backspace_pressed_at = None;
    }

    pub fn update(&mut self, bus: &mut TrsBus) {
        let mut rows: HashMap<u16, u8> = HashMap::new();
        for &k in &self.pressed {
            // Backspace is handled separately below, with auto-repeat.
            if k == Keycode::Backspace {
                continue;
            }
            if let Some((addr, bit, needs_shift)) = key_target(k) {
                *rows.entry(addr).or_insert(0) |= bit;
                if needs_shift {
                    *rows.entry(SHIFT_ADDR).or_insert(0) |= SHIFT_BIT;
                }
            }
        }

        if let Some(pressed_at) = self.backspace_pressed_at {
            let held = pressed_at.elapsed();
            let asserted = match held.checked_sub(BACKSPACE_REPEAT_DELAY) {
                None => true, // initial hold, within the pre-repeat delay
                Some(since_repeat_started) => {
                    let phase = since_repeat_started.as_millis() % BACKSPACE_REPEAT_INTERVAL.as_millis();
                    phase < BACKSPACE_REPEAT_INTERVAL.as_millis() / 2
                }
            };
            if asserted {
                let (addr, bit, _) =
                    key_target(Keycode::Backspace).expect("Backspace is a mapped key");
                *rows.entry(addr).or_insert(0) |= bit;
            }
        }
        // Kept from the previous mapping: Ctrl+AltGr+0 forces '@'. Purpose
        // undocumented (possibly expected by some software as a shortcut);
        // preserved for compatibility rather than dropped outright.
        if self.pressed.contains(&Keycode::LCtrl)
            && self.pressed.contains(&Keycode::RAlt)
            && self.pressed.contains(&Keycode::Num0)
        {
            *rows.entry(0x3801).or_insert(0) |= 0x01;
        }

        // Writing every row each frame (instead of tracking a single
        // "last written address" to clear) is what makes multiple keys
        // held across different matrix rows behave correctly - e.g.
        // holding a letter and a digit at once used to leave one of the
        // two rows stuck on, since only one address was remembered for
        // clearing on the next frame.
        for &addr in &ROW_ADDRESSES {
            bus.write_byte(addr, rows.get(&addr).copied().unwrap_or(0));
        }

        // Some ROM routines poll this address to check all the columns
        bus.write_byte(0x387f, 1);
    }
}
