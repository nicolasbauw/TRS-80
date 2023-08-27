use zilog_z80::cpu::CPU;
use sdl2::video::Window;
use std::{fs, error::Error};

pub struct Machine {
    pub cpu: CPU,
    pub display: crate::display::Display,
    pub keyboard: crate::keyboard::Keyboard,
    pub tape: crate::cassette::CassetteReader,
    config: crate::config::Config,
}

impl Machine {
    pub fn new(window: Window) -> Result<Machine, Box<dyn Error>> {
        let config = crate::config::load_config_file()?;
        let mut m = Self {
            cpu: CPU::new(0xFFFF),
            display: crate::display::Display::new(window, config.clone())?,
            keyboard: crate::keyboard::Keyboard::new(),
            tape: crate::cassette::CassetteReader::new(),
            config,
        };
        m.cpu.debug.io = m.config.debug.iodevices.unwrap_or(false);
        m.cpu.debug.instr_in = m.config.debug.iodevices.unwrap_or(false);
        m.cpu.set_freq(1.77); // Model 1 : 1.77 MHz
        m.cpu.bus.load_bin(&m.config.memory.rom, 0)?;
        let rom_space = fs::metadata(&m.config.memory.rom)?.len();
        m.cpu.bus.set_romspace(0, (rom_space - 1) as u16);
        Ok(m)
    }
}