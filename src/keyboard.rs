use sdl3::{EventPump, keyboard::Scancode};
use std::collections::HashSet;
use zilog_z80::bus::Bus;

pub struct Keyboard {
    last: u16,
    shift: bool,
    old_keys: HashSet<Scancode>,
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard {
            last: 0,
            shift: false,
            old_keys: HashSet::new(),
        }
    }

    pub fn update(&mut self, events: EventPump, bus: &mut Bus, azerty: bool) {
        self.clear_ram(bus);
        self.set_ram(events, bus, azerty);
    }

    fn clear_ram(&mut self, bus: &mut Bus) {
        bus.write_byte(self.last, 0);
        bus.write_byte(0x387f, 0);
        if self.shift {
            bus.write_byte(0x3880, 0);
        }
    }

    fn set_ram(&mut self, events: EventPump, bus: &mut Bus, azerty: bool) {
        // Lecture directe des états physiques (Scancodes) sans traduction logicielle
        let new_keys: HashSet<Scancode> = events.keyboard_state().pressed_scancodes().collect();

        let compare_keys = &new_keys - &self.old_keys;
        let keys = match compare_keys.is_empty() {
            true => new_keys.clone(),
            false => self.old_keys.clone(),
        };
        self.old_keys = new_keys;

        // Neutral value for variable initialization
        let mut msg: (u16, u8) = (0x3880, 128);
        let mut shift = false;

        // Plus besoin de vérifier Parenthèse Gauche/Droite ici, Shift suffit
        if keys.contains(&Scancode::RShift) || keys.contains(&Scancode::LShift) {
            bus.write_byte(0x3880, 0x01);
            shift = true;
        }

        for k in keys.iter() {
            msg = match k {
                // --- LETTRES (Positions physiques basées sur le standard QWERTY) ---
                &Scancode::A => (0x3801, 0x02),
                &Scancode::B => (0x3801, 0x04),
                &Scancode::C => (0x3801, 0x08),
                &Scancode::D => (0x3801, 0x10),
                &Scancode::E => (0x3801, 0x20),
                &Scancode::F => (0x3801, 0x40),
                &Scancode::G => (0x3801, 0x80),
                &Scancode::H => (0x3802, 0x01),
                &Scancode::I => (0x3802, 0x02),
                &Scancode::J => (0x3802, 0x04),
                &Scancode::K => (0x3802, 0x08),
                &Scancode::L => (0x3802, 0x10),
                &Scancode::M => (0x3802, 0x20),
                &Scancode::N => (0x3802, 0x40),
                &Scancode::O => (0x3802, 0x80),
                &Scancode::P => (0x3804, 0x01),
                &Scancode::Q => (0x3804, 0x02),
                &Scancode::R => (0x3804, 0x04),
                &Scancode::S => (0x3804, 0x08),
                &Scancode::T => (0x3804, 0x10),
                &Scancode::U => (0x3804, 0x20),
                &Scancode::V => (0x3804, 0x40),
                &Scancode::W => (0x3804, 0x80),
                &Scancode::X => (0x3808, 0x01),
                &Scancode::Y => (0x3808, 0x02),
                &Scancode::Z => (0x3808, 0x04),

                // --- CHIFFRES (Ligne supérieure + Pavé numérique) ---
                &Scancode::_0 | &Scancode::Kp0 => (0x3810, 0x01),
                &Scancode::_1 | &Scancode::Kp1 => (0x3810, 0x02),
                &Scancode::_2 | &Scancode::Kp2 => (0x3810, 0x04),
                &Scancode::_3 | &Scancode::Kp3 => (0x3810, 0x08),
                &Scancode::_4 | &Scancode::Kp4 => (0x3810, 0x10),
                &Scancode::_5 | &Scancode::Kp5 => (0x3810, 0x20),
                &Scancode::_6 | &Scancode::Kp6 => (0x3810, 0x40),
                &Scancode::_7 | &Scancode::Kp7 => (0x3810, 0x80),
                &Scancode::_8 | &Scancode::Kp8 => (0x3820, 0x01),
                &Scancode::_9 | &Scancode::Kp9 => (0x3820, 0x02),

                // --- SYMBOLES ET PAVÉ NUMÉRIQUE ---
                &Scancode::KpMultiply => (0x3820, 0x04),
                &Scancode::KpPlus => (0x3820, 0x08),
                &Scancode::KpMinus => (0x3820, 0x20),
                &Scancode::KpPeriod => (0x3820, 0x40),
                &Scancode::KpDivide => (0x3820, 0x80),

                // Mappage des touches de ponctuation principales
                &Scancode::Semicolon => (0x3820, 0x08), // Touche ';' / ':'
                &Scancode::Comma => (0x3820, 0x10),     // Touche ',' / '<'
                &Scancode::Equals => (0x3820, 0x20),    // Touche '=' / '+'
                &Scancode::Grave => (0x3801, 0x01), // Touche ` / ~ (utilisée pour le '@' du TRS-80)

                // --- NAVIGATION / CONTRÔLE ---
                &Scancode::Return | &Scancode::KpEnter => (0x3840, 0x01),
                &Scancode::Home => (0x3840, 0x02),
                &Scancode::End => (0x3840, 0x04),
                &Scancode::Up => (0x3840, 0x08),
                &Scancode::Down => (0x3840, 0x10),
                &Scancode::Left | &Scancode::Backspace => (0x3840, 0x20),
                &Scancode::Right => (0x3840, 0x40),
                &Scancode::Space => (0x3840, 0x80),
                _ => continue,
            };

            // Raccourci AltGr + 0 (ou Ctrl + Alt + 0) pour forcer le symbole '@'
            if keys.contains(&Scancode::LCtrl)
                && keys.contains(&Scancode::RAlt)
                && keys.contains(&Scancode::_0)
            {
                msg = (0x3801, 0x01)
            };

            if keys.contains(&Scancode::KpPlus)
                || keys.contains(&Scancode::Equals)
                || keys.contains(&Scancode::KpMultiply)
            {
                bus.write_byte(0x3880, 0x01);
                shift = true;
            }

            bus.write_byte(msg.0, msg.1);
        }

        // Informer la ROM que toutes les colonnes sont scannées
        bus.write_byte(0x387f, 1);

        self.last = msg.0;
        self.shift = shift;
    }
}
