use std::{collections::HashSet, time::Duration, thread};
use sdl2::keyboard::Keycode;

pub fn keyboard(keys: HashSet<Keycode>, tx: &zilog_z80::crossbeam_channel::Sender<(u16, u8)>) {
    // Neutral value for variable initialization
    let mut msg: (u16, u8) = (0x3880, 128);
    let mut shift = false;
    if keys.contains(&Keycode::RShift) | keys.contains(&Keycode::LShift) { tx.send((0x3880, 0x01)).unwrap_or_default(); shift = true }
    for k in keys.iter() {
        msg = match k {
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
            &Keycode::Num8 | &Keycode::Kp8 => (0x3820, 0x01),
            &Keycode::Num9 | &Keycode::Kp9 => (0x3820, 0x02),
            &Keycode::Colon => (0x3820, 0x04),
            &Keycode::KpPlus => (0x3820, 0x08),
            &Keycode::Semicolon => (0x3820, 0x08),
            &Keycode::Equals => (0x3820, 0x20),
            &Keycode::KpMinus => (0x3820, 0x20),
            &Keycode::Return | &Keycode::KpEnter => (0x3840, 0x01),
            &Keycode::Home => (0x3840, 0x02),
            &Keycode::End => (0x3840, 0x04),
            &Keycode::Up => (0x3840, 0x08),
            &Keycode::Down => (0x3840, 0x10),
            &Keycode::Left | &Keycode::Backspace => (0x3840, 0x20),
            &Keycode::Right => (0x3840, 0x40),
            &Keycode::Space => (0x3840, 0x80),
            _ => { continue }
        };
        if keys.contains(&Keycode::KpPlus) | keys.contains(&Keycode::Equals) { tx.send((0x3880, 0x01)).unwrap_or_default(); shift = true }
        tx.send(msg).unwrap_or_default();
    }
    // Clearing the RAM set by the key press
    thread::sleep(Duration::from_millis(16));
    tx.send((msg.0, 0)).unwrap_or_default();
    if shift { tx.send((0x3880, 0)).unwrap_or_default(); }
}