use crate::hexconversion::HexStringToUnsigned;
use crate::monitor::{MonitorCmd, MonitorMessage};
use directories::UserDirs;
use sdl2::video::Window;
use std::{
    collections::HashSet,
    error,
    error::Error,
    fmt::{self, Write as _},
    path::PathBuf,
    sync::mpsc,
    sync::mpsc::SendError,
    thread,
    time::{Duration, Instant},
};
use zilog_z80::bus::{Bus, FlatBus};
use zilog_z80::cpu::CPU;
use zilog_z80::dasm;

const VERSION: &str = env!("CARGO_PKG_VERSION");

// TRS-80 Model I Z80 clock, matching the previous set_freq(1.77) call.
// 1e9 / 1_770_000 = 564.97ns; 565 is the nearest whole nanosecond - same
// precision tradeoff bytebox makes for its own (exact, 4MHz) clock, just
// with a ~0.005% bias here since 1.77MHz doesn't divide 1e9 evenly. Utterly
// immaterial for a BASIC-level emulator, and a large improvement over
// zilog_z80's own execute_timed(), which measured elapsed time with
// SystemTime (wall clock, not monotonic) truncated to whole milliseconds.
const CPU_TICK_NANOS: u64 = 565;

// How many T-states to run before yielding control back to the SDL loop
// for events/display/console - independent of the host display's refresh
// rate on purpose (a prior version derived this from the detected monitor
// refresh rate, which both tied emulated CPU speed to whatever monitor was
// plugged in, and got noticeably worse at higher refresh rates due to
// millisecond truncation: ~4% too fast at 60Hz, ~13% too fast at 144Hz).
const TICKS_PER_SLICE: u32 = 17_700; // 10ms of emulated time at 1.77MHz

fn emulated_duration(ticks: u32) -> Duration {
    Duration::from_nanos(ticks as u64 * CPU_TICK_NANOS)
}
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
    p               pauses execution immediately
    g               resumes execution (after a breakpoint or \"p\")
    n               executes a single instruction (usually while paused)
    r               displays the contents of flags and registers
    hw              displays hardware peripheral status (tape)";

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MachineError {
    ConfigFile,
    ConfigFileFmt,
    IOError,
    SendMsgError,
    SnapshotError,
    DisplayError,
    FontError,
}

impl fmt::Display for MachineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MachineError::ConfigFileFmt => "Bad config file format",
            MachineError::ConfigFile => "Can't load config file",
            MachineError::IOError => "I/O Error",
            MachineError::SendMsgError => "Message not sent",
            MachineError::SnapshotError => "Snapshot I/O error",
            MachineError::DisplayError => "SDL2 error",
            MachineError::FontError => "Can't load font",
        })
    }
}

impl From<std::io::Error> for MachineError {
    fn from(_e: std::io::Error) -> MachineError {
        MachineError::IOError
    }
}

impl From<toml::de::Error> for MachineError {
    fn from(_e: toml::de::Error) -> MachineError {
        MachineError::ConfigFileFmt
    }
}

impl From<SendError<MonitorMessage>> for MachineError {
    fn from(_e: SendError<MonitorMessage>) -> MachineError {
        MachineError::SendMsgError
    }
}

impl From<MachineError> for std::io::Error {
    fn from(e: MachineError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, e)
    }
}

impl error::Error for MachineError {}

pub struct Machine {
    pub cpu: CPU,
    pub bus: FlatBus,
    pub display: crate::display::Display,
    pub keyboard: crate::keyboard::Keyboard,
    pub tape: crate::cassette::CassetteReader,
    config: crate::config::Config,
    cmd_channel: (mpsc::Sender<MonitorMessage>, mpsc::Receiver<MonitorMessage>),
    breakpoints: HashSet<u16>,
    running: bool,
    rom_size: usize,
    next_slice: Instant,
}

impl Machine {
    pub fn new(window: Window, font: &sdl2::ttf::Font) -> Result<Machine, MachineError> {
        let Ok(config) = crate::config::load_config_file() else {
            let user_dirs = UserDirs::new().ok_or(MachineError::ConfigFile)?;
            let mut cfg = user_dirs.home_dir().to_path_buf();
            cfg.push(".config/trust80/config.toml");
            eprintln!(
                "Can't load config file {}",
                cfg.to_str().unwrap_or_default()
            );
            return Err(MachineError::ConfigFile);
        };
        let Ok(display) = crate::display::Display::new(window, font) else {
            return Err(MachineError::DisplayError);
        };
        let mut m = Self {
            cpu: CPU::new(),
            bus: FlatBus::new(0xFFFF),
            display,
            keyboard: crate::keyboard::Keyboard::new(),
            tape: crate::cassette::CassetteReader::new(),
            config,
            cmd_channel: mpsc::channel(),
            breakpoints: HashSet::new(),
            running: true,
            rom_size: 0,
            next_slice: Instant::now(),
        };
        let Ok(s) = m.bus.load_bin(&m.config.memory.rom, 0) else {
            eprintln!("Can't load ROM file {}", &m.config.memory.rom);
            return Err(MachineError::IOError);
        };
        m.rom_size = s;
        m.bus.set_romspace(0, (m.rom_size) as u16);
        Ok(m)
    }

    /// Clone to hand to the console window (`console_window.rs`), the only
    /// producer of commands now that there's no stdin thread anymore - it
    /// replaces the terminal the emulator used to be launched from, the way
    /// bytebox's own console window replaced its `console.rs` stdin thread.
    pub fn command_sender(&self) -> mpsc::Sender<MonitorMessage> {
        self.cmd_channel.0.clone()
    }

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn is_running(&mut self) -> bool {
        self.running
    }

    pub fn cpu_loop(&mut self) {
        let mut slice_ticks: u32 = 0;
        loop {
            if !self.is_running() {
                return;
            }
            let pc = self.cpu.reg.pc;
            let opcode = self.bus.read_byte(pc);
            match opcode {
                0xdb => {
                    let port = self.bus.read_byte(self.cpu.reg.pc + 1);
                    if let Some(true) = self.config.debug.iodevices {
                        println!("IN on port {}", port);
                    }
                    // cassette reader port ?
                    if port == 0xFF {
                        self.cpu.reg.a = self.tape.read();
                    }
                }
                0xd3 => {
                    let port = self.bus.read_byte(self.cpu.reg.pc + 1);
                    if let Some(true) = self.config.debug.iodevices {
                        println!("OUT {} on port {}", self.cpu.reg.a, port);
                    }
                    if port == 0xFF {}
                }
                _ => {}
            }

            slice_ticks += self.cpu.execute(&mut self.bus);

            if self.breakpoints.contains(&self.cpu.reg.pc) {
                self.stop()
            }

            if slice_ticks >= TICKS_PER_SLICE {
                break;
            }
        }

        // Targets an absolute deadline rather than sleeping a fixed
        // duration: with a fixed sleep, the real period is "computation
        // time + sleep", so the emulation runs durably too slow. Advancing
        // next_slice by the emulated duration instead keeps each slice's
        // real-world budget exact regardless of how long this one took to
        // compute, so occasional slow slices don't accumulate into drift.
        let now = Instant::now();
        if now < self.next_slice {
            thread::sleep(self.next_slice - now);
            self.next_slice += emulated_duration(slice_ticks);
        } else {
            // Running behind (e.g. resuming after a breakpoint pause, or a
            // slow host): resync from now instead of trying to catch up,
            // which would otherwise burst the CPU faster than real time.
            self.next_slice = now + emulated_duration(slice_ticks);
        }
    }

    /// Registers and flags, as a display-ready block - shared by the "r"
    /// command and the machine-state panel.
    pub fn get_registers_string(&self) -> String {
        format!(
            "PC :{:#06X}   SP : {:#06X}\n\
             S : {}  Z : {}  H : {}  P : {}  N : {}  C : {}\n\
             BC : {:#06X}  DE : {:#06X}  HL : {:#06X}  AF : {:#06X}\n\
             BC': {:#06X}  DE': {:#06X}  HL': {:#06X}  AF': {:#06X}\n\
             IXH : {:#04X}  IXL : {:#04X}  IYH : {:#04X}  IYL : {:#04X}\n\
             (SP) : {:#06X}  IFF1 : {:<5}  IFF2 : {:<5}  IM : {}  Pending INT : {:<5}  Pending NMI : {:<5}",
            self.cpu.reg.pc,
            self.cpu.reg.sp,
            self.cpu.reg.flags.s as i32,
            self.cpu.reg.flags.z as i32,
            self.cpu.reg.flags.h as i32,
            self.cpu.reg.flags.p as i32,
            self.cpu.reg.flags.n as i32,
            self.cpu.reg.flags.c as i32,
            self.cpu.reg.get_bc(),
            self.cpu.reg.get_de(),
            self.cpu.reg.get_hl(),
            self.cpu.reg.get_af(),
            self.cpu.alt.get_bc(),
            self.cpu.alt.get_de(),
            self.cpu.alt.get_hl(),
            self.cpu.alt.get_af(),
            self.cpu.reg.ixh,
            self.cpu.reg.ixl,
            self.cpu.reg.iyh,
            self.cpu.reg.iyl,
            self.bus.read_word(self.cpu.reg.sp),
            self.cpu.iff1(),
            self.cpu.iff2(),
            self.cpu.im(),
            self.cpu.has_pending_int(),
            self.cpu.has_pending_nmi()
        )
    }

    /// Processes one pending command from the console window, if any, and
    /// returns its textual output (empty if there was no command waiting,
    /// or it produced none) - to be pushed into the shared `ConsoleLog` by
    /// the caller. Replaces the old `println!`-based version: there's no
    /// terminal to print to anymore, only the console window's own log.
    pub fn process_console_commands(&mut self) -> Result<String, Box<dyn Error>> {
        let (command, arg, arg2) = match self.cmd_channel.1.try_recv() {
            Ok(msg) => msg,
            Err(_) => return Ok(String::new()),
        };
        let mut out = String::new();

        match command {
            MonitorCmd::Help => {
                writeln!(out, "Version {VERSION}")?;
                write!(out, "{HELP}")?;
            }
            MonitorCmd::Reset => {
                self.stop();
                self.cpu.reg.pc = 0;
                self.start();
                write!(out, "Reset done !")?;
            }
            MonitorCmd::PowerCycle => {
                self.stop();
                self.bus
                    .clear_mem_slice(self.rom_size, self.config.memory.ram as usize);
                self.cpu.reg.pc = 0;
                self.start();
                write!(out, "Powercycle done !")?;
            }
            MonitorCmd::Tape => {
                if arg == *"rewind" {
                    self.tape.rewind();
                    write!(out, "Tape rewound !")?;
                } else {
                    let mut tape_path: PathBuf = self.config.storage.tape_path.clone();
                    tape_path.push(arg);
                    match self.tape.load(tape_path) {
                        Ok(()) => write!(out, "Tape loaded !")?,
                        Err(_) => write!(out, "File not found !")?,
                    }
                }
            }
            MonitorCmd::Disassemble => {
                let mut a = arg.to_u16()?;
                for _ in 0..=20 {
                    let d = dasm::dasm(&self.bus, a);
                    writeln!(out, "{:04X}    {}", a, d.0)?;
                    a += (d.1) as u16;
                }
            }
            MonitorCmd::ReadMem => {
                let a = arg.to_u16()?;
                write!(out, "{:04X}    {:02X}", a, self.bus.read_byte(a))?;
            }
            MonitorCmd::WriteMem => {
                let a = arg.to_u16()?;
                self.bus.write_byte(a, arg2.to_u8()?);
                write!(out, "{:04X} -> {:02X}", a, self.bus.read_byte(a))?;
            }
            MonitorCmd::Jump => {
                let a = arg.to_u16()?;
                self.cpu.reg.pc = a;
                write!(out, "Jumped to {:#06X}", a)?;
            }
            MonitorCmd::ListBreakpoints => {
                if self.breakpoints.is_empty() {
                    write!(out, "No breakpoints !")?;
                } else {
                    for b in &self.breakpoints {
                        writeln!(out, "{:#06X}", b)?;
                    }
                }
            }
            MonitorCmd::AddBreakpoint => {
                let a = arg.to_u16()?;
                self.breakpoints.insert(a);
                write!(out, "New breakpoint at {:#06X}", a)?;
            }
            MonitorCmd::RemoveBreakpoint => {
                let a = arg.to_u16()?;
                if self.breakpoints.remove(&a) {
                    write!(out, "Breakpoint at {:#06X} removed", a)?;
                }
            }
            MonitorCmd::Pause => {
                self.stop();
                write!(out, "Paused.")?;
            }
            MonitorCmd::Resume => {
                self.start();
                write!(out, "Resumed.")?;
            }
            MonitorCmd::Step => {
                self.cpu.execute(&mut self.bus);
                let a = self.cpu.reg.pc;
                let d = dasm::dasm(&self.bus, a);
                write!(out, "{:04X}    {}", a, d.0)?;
            }
            MonitorCmd::Registers => {
                write!(out, "{}", self.get_registers_string())?;
            }
            MonitorCmd::Hardware => {
                write!(out, "{}", self.tape.status())?;
            }
            MonitorCmd::Unknown => {
                write!(out, "Unknown command.")?;
            }
        }
        Ok(out)
    }
}
