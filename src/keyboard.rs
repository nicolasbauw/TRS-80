use sdl2::{keyboard::Keycode, EventPump};
use std::collections::HashSet;
use zilog_z80::bus::Bus;

pub struct Keyboard {
    last: u16,
    shift: bool,
    old_keys: HashSet<Keycode>,
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard {
            last: 0,
            shift: false,
            old_keys: HashSet::new(),
        }
    }

    pub fn update(&mut self, events: EventPump, bus: &mut Bus) {
        self.clear_ram(bus);
        self.set_ram(events, bus);
    }

    fn clear_ram(&mut self, bus: &mut Bus) {
        bus.write_byte(self.last, 0);
        bus.write_byte(0x387f, 0);
        if self.shift {
            bus.write_byte(0x3880, 0);
        }
    }

    fn set_ram(&mut self, events: EventPump, bus: &mut Bus) {
        // Reading keyboard events
        let new_keys: HashSet<Keycode> = events
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(Keycode::from_scancode)
            .collect();

        let compare_keys = &new_keys - &self.old_keys;
        let keys = match compare_keys.is_empty() {
            true => new_keys.clone(),
            false => self.old_keys.clone(),
        };
        self.old_keys = new_keys;

        // Neutral value for variable initialization
        let mut msg: (u16, u8) = (0x3880, 128);
        let mut shift = false;
        if keys.contains(&Keycode::RShift)
            | keys.contains(&Keycode::LShift)
            | keys.contains(&Keycode::LeftParen)
            | keys.contains(&Keycode::RightParen)
        {
            bus.write_byte(0x3880, 0x01);
            shift = true
        }
        for k in keys.iter() {
            msg = match k {
                &Keycode::At => (0x3801, 0x01),
                &Keycode::A => (0x3801, 0x02),
                &Keycode::B => (0x3801, 0x04),
                &Keycode::C => (0x3801, 0x08),
                &Keycode::D => (0x3801, 0x10),
                &Keycode::E => (0x3801, 0x20),
                &Keycode::F => (0x3801, 0x40),
                &Keycode::G => (0x3801, 0x80),
                &Keycode::H => (0x3802, 0x01),
                &Keycode::I => (0x3802, 0x02),
                &Keycode::J => (0x3802, 0x04),
                &Keycode::K => (0x3802, 0x08),
                &Keycode::L => (0x3802, 0x10),
                &Keycode::M => (0x3802, 0x20),
                &Keycode::N => (0x3802, 0x40),
                &Keycode::O => (0x3802, 0x80),
                &Keycode::P => (0x3804, 0x01),
                &Keycode::Q => (0x3804, 0x02),
                &Keycode::R => (0x3804, 0x04),
                &Keycode::S => (0x3804, 0x08),
                &Keycode::T => (0x3804, 0x10),
                &Keycode::U => (0x3804, 0x20),
                &Keycode::V => (0x3804, 0x40),
                &Keycode::W => (0x3804, 0x80),
                &Keycode::X => (0x3808, 0x01),
                &Keycode::Y => (0x3808, 0x02),
                &Keycode::Z => (0x3808, 0x04),
                &Keycode::Num0 | &Keycode::Kp0 => (0x3810, 0x01),
                &Keycode::Num1 | &Keycode::Kp1 => (0x3810, 0x02),
                &Keycode::Num2 | &Keycode::Kp2 => (0x3810, 0x04),
                &Keycode::Num3 | &Keycode::Kp3 => (0x3810, 0x08),
                &Keycode::Num4 | &Keycode::Kp4 => (0x3810, 0x10),
                &Keycode::Num5 | &Keycode::Kp5 => (0x3810, 0x20),
                &Keycode::Num6 | &Keycode::Kp6 => (0x3810, 0x40),
                &Keycode::Num7 | &Keycode::Kp7 => (0x3810, 0x80),
                &Keycode::Num8 | &Keycode::Kp8 | Keycode::LeftParen => (0x3820, 0x01),
                &Keycode::Num9 | &Keycode::Kp9 | Keycode::RightParen => (0x3820, 0x02),
                &Keycode::KpMultiply => (0x3820, 0x04),
                &Keycode::Colon => (0x3820, 0x04),
                &Keycode::KpPlus => (0x3820, 0x08),
                &Keycode::Semicolon => (0x3820, 0x08),
                &Keycode::Less => (0x3820, 0x10),
                &Keycode::Comma => (0x3820, 0x10),
                &Keycode::Equals => (0x3820, 0x20),
                &Keycode::KpMinus => (0x3820, 0x20),
                &Keycode::KpPeriod => (0x3820, 0x40),
                &Keycode::KpDivide => (0x3820, 0x80),
                &Keycode::Return | &Keycode::KpEnter => (0x3840, 0x01),
                &Keycode::Home => (0x3840, 0x02),
                &Keycode::End => (0x3840, 0x04),
                &Keycode::Up => (0x3840, 0x08),
                &Keycode::Down => (0x3840, 0x10),
                &Keycode::Left | &Keycode::Backspace => (0x3840, 0x20),
                &Keycode::Right => (0x3840, 0x40),
                &Keycode::Space => (0x3840, 0x80),
                _ => continue,
            };
            if keys.contains(&Keycode::LCtrl)
                && keys.contains(&Keycode::RAlt)
                && keys.contains(&Keycode::Num0)
            {
                msg = (0x3801, 0x01)
            };
            if keys.contains(&Keycode::Less) & shift {
                msg = (0x3820, 0x40)
            };
            if keys.contains(&Keycode::Comma) & shift {
                msg = (0x3820, 0x80)
            };
            if keys.contains(&Keycode::KpPlus)
                | keys.contains(&Keycode::Equals)
                | keys.contains(&Keycode::Less)
                | keys.contains(&Keycode::KpMultiply)
                | keys.contains(&Keycode::KpDecimal)
            {
                bus.write_byte(0x3880, 0x01);
                shift = true
            }
            bus.write_byte(msg.0, msg.1);
            //println!("KBD : wrote {} at address {:04X}", msg.1, msg.0);
        }

        // Some routines check this address to check all the columns
        bus.write_byte(0x387f, 1);

        // Returning the address to clear and the status of the shift key
        self.last = msg.0;
        self.shift = shift
    }
}
