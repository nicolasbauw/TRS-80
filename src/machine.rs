use zilog_z80::cpu::CPU;
use sdl2::video::Window;
use std::error::Error;

pub struct Machine {
    pub cpu: CPU,
    pub display: crate::display::Display,
    pub keyboard: crate::keyboard::Keyboard,
    pub tape: crate::cassette::CassetteReader,
}

impl Machine {
    pub fn new(window: Window) -> Result<Machine, Box<dyn Error>> {
        let m = Self {
            cpu: CPU::new(0xFFFF),
            display: crate::display::Display::new(window)?,
            keyboard: crate::keyboard::Keyboard::new(),
            tape: crate::cassette::CassetteReader::new(),
        };
        Ok(m)
    }
}