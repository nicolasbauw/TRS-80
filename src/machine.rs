use sdl2::video::Window;
use std::{error::Error, fs, path::PathBuf, sync::mpsc, thread, time::Duration};
use zilog_z80::cpu::CPU;

pub struct Machine {
    pub cpu: CPU,
    pub display: crate::display::Display,
    pub keyboard: crate::keyboard::Keyboard,
    pub tape: crate::cassette::CassetteReader,
    config: crate::config::Config,
    cmd_channel: (
        mpsc::Sender<(String, String)>,
        mpsc::Receiver<(String, String)>,
    ),
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
            cmd_channel: mpsc::channel(),
        };
        m.cpu.debug.io = m.config.debug.iodevices.unwrap_or(false);
        m.cpu.debug.instr_in = m.config.debug.iodevices.unwrap_or(false);
        m.cpu.set_freq(1.77); // Model 1 : 1.77 MHz
        m.cpu.bus.load_bin(&m.config.memory.rom, 0)?;
        let rom_space = fs::metadata(&m.config.memory.rom)?.len();
        m.cpu.bus.set_romspace(0, (rom_space - 1) as u16);
        crate::console::launch(m.cmd_channel.0.clone())?;
        Ok(m)
    }

    pub fn cpu_loop(&mut self) {
        loop {
            let opcode = self.cpu.bus.read_byte(self.cpu.reg.pc);
            match opcode {
                0xdb => {
                    let port = self.cpu.bus.read_byte(self.cpu.reg.pc + 1);
                    if let Some(true) = self.config.debug.iodevices {
                        println!("IN on port {}", port);
                    }
                    // cassette reader port ?
                    if port == 0xFF {
                        self.cpu.reg.a = self.tape.read();
                    }
                }
                0xd3 => {
                    let port = self.cpu.bus.read_byte(self.cpu.reg.pc + 1);
                    if let Some(true) = self.config.debug.iodevices {
                        println!("OUT {} on port {}", self.cpu.reg.a, port);
                    }
                    if port == 0xFF {}
                }
                _ => {}
            }

            // executes slice_max_cycles number of cycles
            if let Some(t) = self.cpu.execute_timed() {
                thread::sleep(Duration::from_millis(t.into()));
                break;
            }
        }
    }

    pub fn console(&mut self) -> Result<(), Box<dyn Error>> {
        let (command, data) = self.cmd_channel.1.try_recv()?;

        match command.as_str() {
            "reset" => {
                self.cpu.reg.pc = 0;
                println!("RESET DONE !");
            }
            "tape" => {
                if data == *"rewind" {
                    self.tape.rewind();
                    println!("TAPE REWOUND !");
                    return Ok(());
                }
                let mut tape_path: PathBuf = self.config.storage.tape_path.clone();
                tape_path.push(data);
                match self.tape.load(tape_path) {
                    Ok(()) => {
                        println!("TAPE LOADED !")
                    }
                    Err(_) => {
                        println!("FILE NOT FOUND !")
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
