use sdl2::video::Window;
use std::{error::Error, path::PathBuf, sync::mpsc, thread, time::Duration, collections::HashSet};
use zilog_z80::cpu::CPU;

use crate::hexconversion::HexStringToUnsigned;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const HELP: &str = "
Commands:
    reset           reboots the TRS-80
    powercycle      reboots the TRS-80 and clears RAM
    tape rewind     \"rewinds\" the tape
    tape [file]     \"inserts\" a .cas tape file
    
Monitor commands:
    d 0x0000        disassembles code at 0x0000 and the 20 next
                    instructions
    m 0xeeee        displays memory content at address 0xeeee
    m 0xeeee 0xaa   sets memory address 0xeeee to the 0xaa value
    j 0x0000        jumps to 0x0000 address
    b               displays set breakpoints
    b 0x0002        sets a breakpoint at address 0x0002
    f 0x0002        \"frees\" (deletes) breakpoint at address 0x0002
    g               resumes execution after a breakpoint has been used to
                    halt execution
    r               displays the contents of flags and registers";

pub struct Machine {
    pub cpu: CPU,
    pub display: crate::display::Display,
    pub keyboard: crate::keyboard::Keyboard,
    pub tape: crate::cassette::CassetteReader,
    config: crate::config::Config,
    cmd_channel: (
        mpsc::Sender<(String, String, String)>,
        mpsc::Receiver<(String, String, String)>,
    ),
    breakpoints: HashSet<u16>,
    running: bool,
    rom_size: usize
}

impl Machine {
    pub fn new(window: Window) -> Result<Machine, Box<dyn Error>> {
        let config = crate::config::load_config_file()?;
        let mut m = Self {
            cpu: CPU::new(0xFFFF),
            display: crate::display::Display::new(window)?,
            keyboard: crate::keyboard::Keyboard::new(),
            tape: crate::cassette::CassetteReader::new(),
            config,
            cmd_channel: mpsc::channel(),
            breakpoints: HashSet::new(),
            running: true,
            rom_size: 0,
        };
        m.cpu.debug.io = m.config.debug.iodevices.unwrap_or(false);
        m.cpu.debug.instr_in = m.config.debug.iodevices.unwrap_or(false);
        m.rom_size = m.cpu.bus.load_bin(&m.config.memory.rom, 0)?;
        m.cpu.bus.set_romspace(0, (m.rom_size) as u16);
        crate::console::launch(m.cmd_channel.0.clone())?;
        Ok(m)
    }

    pub fn start(&mut self) {
        self.running=true;
    }

    pub fn stop(&mut self) {
        self.running=false;
    }

    pub fn is_running(&mut self) -> bool {
        self.running
    }

    pub fn cpu_loop(&mut self) {
        loop {
            if !self.is_running() { return }
            let pc = self.cpu.reg.pc;
            let opcode = self.cpu.bus.read_byte(pc);
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

            if self.breakpoints.is_empty() { continue }
            if self.breakpoints.contains(&self.cpu.reg.pc) {
                self.stop()
            }
        }
    }

    pub fn set_timings(&mut self, refresh_rate: i32) {
        let s: f32 = (1.0 / (refresh_rate as f32)) * 1000.0;
        self.cpu.set_slice_duration(s as u32); // Adjusting slice_duration to detected refresh rate
        self.cpu.set_freq(1.77); // Adjusting slice_max_cycles to detected refresh rate
    }

    pub fn console(&mut self) -> Result<(), Box<dyn Error>> {
        let (command, arg, arg2 ) = self.cmd_channel.1.try_recv()?;

        match command.as_str() {
            "help" => {
                println!("Version {VERSION}");
                println!("{HELP}");
            }
            "reset" => {
                self.stop();
                self.cpu.reg.pc = 0;
                self.start();
                println!("Reset done !");
            }
            "powercycle" => {
                self.stop();
                self.cpu.bus.clear_mem_slice(self.rom_size, self.config.memory.ram as usize);
                self.cpu.reg.pc = 0;
                self.start();
                println!("Powercycle done !");
            }
            "tape" => {
                if arg == *"rewind" {
                    self.tape.rewind();
                    println!("Tape rewound !");
                    return Ok(());
                }
                let mut tape_path: PathBuf = self.config.storage.tape_path.clone();
                tape_path.push(arg);
                match self.tape.load(tape_path) {
                    Ok(()) => {
                        println!("Tape loaded !")
                    }
                    Err(_) => {
                        println!("File not found !")
                    }
                }
            }
            "d" => {
                let mut a = arg.to_u16()?;
                for _ in 0..=20 {
                    let d = self.cpu.dasm_1byte(a);
                    println!("{:04X}    {}", a, d.0);
                    a += (d.1) as u16;
                }
            },
            "m" => {
                let a = arg.to_u16()?;
                println!("{:04X}    {:02X}", a, self.cpu.bus.read_byte(a));
                if !arg2.is_empty() {
                    self.cpu.bus.write_byte(a, arg2.to_u8()?);
                    println!("{:04X} -> {:02X}", a, self.cpu.bus.read_byte(a));
                }
            },
            "j" => {
                let a = arg.to_u16()?;
                self.cpu.reg.pc = a;
            },
            "b" => {
                let Ok(a) = arg.to_u16() else {
                    if self.breakpoints.is_empty() { println!("No breakpoints !") }
                    for b in &self.breakpoints {
                        println!("{:#06X}", b);
                    }
                    return Ok(()); 
                };
                self.breakpoints.insert(a);
                println!("New breakpoint at {:#06X}", a);
            },
            "f" => {
                let a = arg.to_u16()?;
                if self.breakpoints.remove(&a) {
                    println!("Breakpoint at {:#06X} removed", a);
                }
            },
            "g" => {
                self.start();
            },
            "r" => {
                print!("PC :{:#06X}\tSP : {:#06X}\nS : {}\tZ : {}\tH : {}\tP : {}\tN : {}\tC : {}\nB : {:#04X}\tC : {:#04X}\nD : {:#04X}\tE : {:#04X}\nH : {:#04X}\tL : {:#04X}\nA : {:#04X}\t(SP) : {:#06X}\n", self.cpu.reg.pc, self.cpu.reg.sp, self.cpu.reg.flags.s as i32, self.cpu.reg.flags.z as i32, self.cpu.reg.flags.h as i32, self.cpu.reg.flags.p as i32, self.cpu.reg.flags.n as i32, self.cpu.reg.flags.c as i32, self.cpu.reg.b, self.cpu.reg.c, self.cpu.reg.d, self.cpu.reg.e, self.cpu.reg.h, self.cpu.reg.l, self.cpu.reg.a, self.cpu.bus.read_word(self.cpu.reg.sp))
            },
            _ => {}
        }
        Ok(())
    }
}
