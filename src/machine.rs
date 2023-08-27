use zilog_z80::cpu::CPU;
mod display;
mod keyboard;
mod cassette;
mod config;

struct Machine {
    cpu: CPU,
    display: display::Display,
    keyboard: keyboard::Keyboard,
    tape: cassette::CassetteReader,
}

impl Machine {
    pub fn new(window: Window) -> Machine {
        Self {
            cpu: CPU::new(0xFFFF),
            display: display::Display::new(window)?,
            keyboard: keyboard::Keyboard::new(),
            tape: cassette::CassetteReader::new(),
        }
    }
}