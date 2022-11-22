use std::{io, collections::HashSet, time::Duration, thread};
use sdl2::keyboard::Keycode;
use zilog_z80::crossbeam_channel::{Sender, Receiver};
use crate::config;

pub fn launch(keys_rx: Receiver<HashSet<Keycode>>, keyboard_sender: Sender<(u16,u8)>) -> Result<(), Box<dyn std::error::Error>> {
    // Keyboard MMIO peripheral
    thread::Builder::new()
        .name(String::from("TRS-80 Keyboard"))
        .spawn(move || -> io::Result<()> {
            let config = config::load_config_file()?;
            let mem_clr = config.keyboard.memclear_delay;
            loop {
                if let Ok(keys) = keys_rx.recv() {
                    if !keyboard(keys, &keyboard_sender, mem_clr) { thread::sleep(Duration::from_millis(config.keyboard.repeat_delay)); }
                }
            }
        })?;
        Ok(())
}

fn keyboard(keys: HashSet<Keycode>, tx: &zilog_z80::crossbeam_channel::Sender<(u16, u8)>, mem_clr: u64) -> bool {
    // Neutral value for variable initialization
    let mut msg: (u16, u8) = (0x3880, 128);
    let mut shift = false;
    if keys.contains(&Keycode::RShift) | keys.contains(&Keycode::LShift) { tx.send((0x3880, 0x01)).unwrap_or_default(); shift = true }
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
            &Keycode::Num8 | &Keycode::Kp8 => (0x3820, 0x01),
            &Keycode::Num9 | &Keycode::Kp9 => (0x3820, 0x02),
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
            _ => { continue }
        };
        if keys.contains(&Keycode::Less) & shift { msg = (0x3820, 0x40) };
        if keys.contains(&Keycode::Comma) & shift { msg = (0x3820, 0x80) };
        if keys.contains(&Keycode::KpPlus)
            | keys.contains(&Keycode::Equals)
            | keys.contains(&Keycode::Less)
            | keys.contains(&Keycode::KpMultiply)
            | keys.contains(&Keycode::KpDecimal)
            { tx.send((0x3880, 0x01)).unwrap_or_default(); shift = true }
        tx.send(msg).unwrap_or_default();
    }
    // Some routines check this address to check all the columns
    tx.send((0x387f, 1)).unwrap_or_default();
    // Clearing the RAM set by the key press
    thread::sleep(Duration::from_millis(mem_clr));
    tx.send((msg.0, 0)).unwrap_or_default();
    tx.send((0x387f, 0)).unwrap_or_default();
    if shift { tx.send((0x3880, 0)).unwrap_or_default(); }
    shift
}