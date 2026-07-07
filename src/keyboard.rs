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
            // On ignore les modificateurs physiques bruts, leur logique est déjà portée par les autres touches
            if *k == Scancode::LShift || *k == Scancode::RShift {
                continue;
            }

            // 1. On récupère le scancode ET le shift corrigés pour cette touche
            let (logical_key, logical_shift) = to_logical_scancode(*k, azerty, shift);

            // 2. On applique dynamiquement l'état Shift requis par la touche cible dans le bus
            if azerty {
                bus.write_byte(0x3880, if logical_shift { 0x01 } else { 0x00 });
                shift = logical_shift; // On met à jour la variable pour la persistance en fin de frame
            }
            msg = match logical_key {
                // --- LETTRES (Positions physiques basées sur le standard QWERTY) ---
                Scancode::A => (0x3801, 0x02),
                Scancode::B => (0x3801, 0x04),
                Scancode::C => (0x3801, 0x08),
                Scancode::D => (0x3801, 0x10),
                Scancode::E => (0x3801, 0x20),
                Scancode::F => (0x3801, 0x40),
                Scancode::G => (0x3801, 0x80),
                Scancode::H => (0x3802, 0x01),
                Scancode::I => (0x3802, 0x02),
                Scancode::J => (0x3802, 0x04),
                Scancode::K => (0x3802, 0x08),
                Scancode::L => (0x3802, 0x10),
                Scancode::M => (0x3802, 0x20),
                Scancode::N => (0x3802, 0x40),
                Scancode::O => (0x3802, 0x80),
                Scancode::P => (0x3804, 0x01),
                Scancode::Q => (0x3804, 0x02),
                Scancode::R => (0x3804, 0x04),
                Scancode::S => (0x3804, 0x08),
                Scancode::T => (0x3804, 0x10),
                Scancode::U => (0x3804, 0x20),
                Scancode::V => (0x3804, 0x40),
                Scancode::W => (0x3804, 0x80),
                Scancode::X => (0x3808, 0x01),
                Scancode::Y => (0x3808, 0x02),
                Scancode::Z => (0x3808, 0x04),

                // --- CHIFFRES (Ligne supérieure + Pavé numérique) ---
                Scancode::_0 | Scancode::Kp0 => (0x3810, 0x01),
                Scancode::_1 | Scancode::Kp1 => (0x3810, 0x02),
                Scancode::_2 | Scancode::Kp2 => (0x3810, 0x04),
                Scancode::_3 | Scancode::Kp3 => (0x3810, 0x08),
                Scancode::_4 | Scancode::Kp4 => (0x3810, 0x10),
                Scancode::_5 | Scancode::Kp5 => (0x3810, 0x20),
                Scancode::_6 | Scancode::Kp6 => (0x3810, 0x40),
                Scancode::_7 | Scancode::Kp7 => (0x3810, 0x80),
                Scancode::_8 | Scancode::Kp8 => (0x3820, 0x01),
                Scancode::_9 | Scancode::Kp9 => (0x3820, 0x02),

                // --- SYMBOLES ET PAVÉ NUMÉRIQUE ---
                Scancode::KpMultiply => (0x3820, 0x04),
                Scancode::KpPlus => (0x3820, 0x08),

                // --- AJUSTEMENTS SYMBOLES (Ajoute les versions standards à côté du pavé numérique) ---
                Scancode::Slash | Scancode::KpDivide => (0x3820, 0x80), // Reçoit le '/' et le '?'
                Scancode::Minus | Scancode::KpMinus => (0x3820, 0x20),  // Reçoit le '-' et le '_'
                Scancode::Period | Scancode::KpPeriod => (0x3820, 0x40), // Reçoit le '.'

                // Mappage des touches de ponctuation principales
                Scancode::Semicolon => (0x3820, 0x08), // Gère ';' et ':'
                Scancode::Comma => (0x3820, 0x10),     // Gère ','
                Scancode::Equals => (0x3820, 0x20),    // Gère '=' et '+'
                Scancode::Grave => (0x3801, 0x01),

                // --- NAVIGATION / CONTRÔLE ---
                Scancode::Return | Scancode::KpEnter => (0x3840, 0x01),
                Scancode::Home => (0x3840, 0x02),
                Scancode::End => (0x3840, 0x04),
                Scancode::Up => (0x3840, 0x08),
                Scancode::Down => (0x3840, 0x10),
                Scancode::Left | Scancode::Backspace => (0x3840, 0x20),
                Scancode::Right => (0x3840, 0x40),
                Scancode::Space => (0x3840, 0x80),
                _ => continue,
            };

            // Raccourci AltGr + 0 (ou Ctrl + Alt + 0) pour forcer le symbole '@'
            if keys.contains(&Scancode::LCtrl)
                && keys.contains(&Scancode::RAlt)
                && keys.contains(&Scancode::_0)
            {
                msg = (0x3801, 0x01)
            };

            if keys.contains(&Scancode::KpPlus) || keys.contains(&Scancode::KpMultiply) {
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

fn to_logical_scancode(physical: Scancode, azerty: bool, physical_shift: bool) -> (Scancode, bool) {
    if !azerty {
        return (physical, physical_shift);
    }

    match (physical, physical_shift) {
        // --- LETTRES ---
        (Scancode::Q, s) => (Scancode::A, s),
        (Scancode::A, s) => (Scancode::Q, s),
        (Scancode::W, s) => (Scancode::Z, s),
        (Scancode::Z, s) => (Scancode::W, s),
        (Scancode::Semicolon, s) => (Scancode::M, s), // Touche physique M sur AZERTY Mac (lettre M)

        // --- PONCTUATION BASSE (AZERTY MAC FR) ---
        // Touche physique M (?, et ,)
        (Scancode::M, false) => (Scancode::Comma, false), // ',' -> TRS-80 ',' (0x3820, 0x10)
        (Scancode::M, true) => (Scancode::Slash, true), // '?' -> TRS-80 Shift + '/' (0x3820, 0x80)

        // Touche physique Comma (. et ;)
        (Scancode::Comma, false) => (Scancode::Semicolon, false), // ';' -> TRS-80 ';' (0x3820, 0x08)
        (Scancode::Comma, true) => (Scancode::Period, false), // '.' -> TRS-80 '.' (0x3820, 0x40)

        // Touche physique Period (/ et :)
        (Scancode::Period, false) => (Scancode::KpMultiply, false), // ':' -> TRS-80 ':' (0x3820, 0x04) via KpMultiply
        (Scancode::Period, true) => (Scancode::Slash, false), // '/' -> TRS-80 '/' (0x3820, 0x80)

        // Touche physique Slash (+ et =)
        (Scancode::Slash, false) => (Scancode::Equals, true), // '=' -> TRS-80 Shift + '-' (0x3820, 0x20 avec Shift)
        (Scancode::Slash, true) => (Scancode::Semicolon, true), // '+' -> TRS-80 Shift + ';' (0x3820, 0x08 avec Shift)

        // --- HAUT DROITE LIGNE NUMÉRIQUE ---
        // Touche physique Equals (contient - et _ sur AZERTY Mac)
        (Scancode::Equals, false) => (Scancode::Minus, false), // '-' -> TRS-80 '-' (0x3820, 0x20)
        (Scancode::Equals, true) => (Scancode::Minus, true), // '_' -> TRS-80 Shift + '-' (0x3820, 0x20)

        // Touche physique Minus (contient ) et ° sur AZERTY Mac)
        (Scancode::Minus, false) => (Scancode::_9, true), // ')' -> TRS-80 Shift + '9' (0x3820, 0x02)
        (Scancode::Minus, true) => (Scancode::Equals, true), // '°' (non géré)

        // Touche '=' du pavé numérique Mac
        (Scancode::KpEquals, _) => (Scancode::Equals, true), // Redirige vers le '=' logique (Equals avec shift pour faire Shift + '-')

        // --- TOUCHES ENCADRÉES (IMAGE REFERENCE) ---
        // Touche physique Grave (tout en haut à gauche, contient @ et # sur AZERTY Mac)
        (Scancode::Grave, false) => (Scancode::Grave, false), // '@' -> TRS-80 '@' (0x3801, 0x01)
        (Scancode::Grave, true) => (Scancode::_3, true), // '#' -> TRS-80 Shift + '3' (0x3810, 0x08)

        // Touche physique NonUsBackslash (en bas à gauche à côté de W/Q, contient < et > sur AZERTY Mac)
        (Scancode::NonUsBackslash, false) => (Scancode::Comma, true), // '<' -> TRS-80 Shift + ',' (0x3820, 0x10)
        (Scancode::NonUsBackslash, true) => (Scancode::Period, true), // '>' -> TRS-80 Shift + '.' (0x3820, 0x40)

        // --- AUTRES TOUCHES ---
        (Scancode::Apostrophe, true) => (Scancode::_5, true), // '%' (Shift + 5 en QWERTY)
        (Scancode::RightBracket, false) => (Scancode::_4, true), // '$' (Shift + 4 en QWERTY)

        // --- LIGNE NUMÉRIQUE : Quand SHIFT est pressé (L'utilisateur veut le CHIFFRE) ---
        (Scancode::_1, true) => (Scancode::_1, false),
        (Scancode::_2, true) => (Scancode::_2, false),
        (Scancode::_3, true) => (Scancode::_3, false),
        (Scancode::_4, true) => (Scancode::_4, false),
        (Scancode::_5, true) => (Scancode::_5, false),
        (Scancode::_6, true) => (Scancode::_6, false),
        (Scancode::_7, true) => (Scancode::_7, false),
        (Scancode::_8, true) => (Scancode::_8, false),
        (Scancode::_9, true) => (Scancode::_9, false),
        (Scancode::_0, true) => (Scancode::_0, false),

        // --- LIGNE NUMÉRIQUE (SYMBOLES AZERTY MAC) ---
        (Scancode::_1, false) => (Scancode::_6, true), // & -> TRS-80 Shift + 6
        (Scancode::_3, false) => (Scancode::_2, true), // " -> TRS-80 Shift + 2
        (Scancode::_4, false) => (Scancode::Apostrophe, false), // '
        (Scancode::_5, false) => (Scancode::_8, true), // ( -> TRS-80 Shift + 8
        (Scancode::_8, false) => (Scancode::_1, true), // ! -> TRS-80 Shift + 1

        _ => (physical, physical_shift),
    }
}
