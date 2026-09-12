//! A command and its two textual arguments, as it travels over the channel
//! between whichever facade reads it (the console window, `console_window.rs`)
//! and `Machine`, which executes it.
pub type MonitorMessage = (MonitorCmd, String, String);

/// Translates a typed command line into a `MonitorMessage`. Single point of
/// truth for what a command line means: the console window is the only
/// facade left that reads one, but keeping the parsing here rather than
/// inlined in that file keeps `Machine::console` (the actual command
/// dispatch) decoupled from how the text was obtained.
pub fn parse_command(line: &str) -> MonitorMessage {
    let mut parts = line.split_whitespace();
    let cmd_part = parts.next().unwrap_or_default();
    let arg = parts.next().unwrap_or_default().to_string();
    let arg2 = parts.next().unwrap_or_default().to_string();

    let command = match cmd_part {
        "h" | "help" => MonitorCmd::Help,
        "reset" => MonitorCmd::Reset,
        "powercycle" | "pc" => MonitorCmd::PowerCycle,
        "tape" => MonitorCmd::Tape,
        "d" => MonitorCmd::Disassemble,
        "j" => MonitorCmd::Jump,
        "f" => MonitorCmd::RemoveBreakpoint,
        "b" => {
            if arg.is_empty() {
                MonitorCmd::ListBreakpoints
            } else {
                MonitorCmd::AddBreakpoint
            }
        }
        "m" => {
            if arg2.is_empty() {
                MonitorCmd::ReadMem
            } else {
                MonitorCmd::WriteMem
            }
        }
        "p" => MonitorCmd::Pause,
        "g" => MonitorCmd::Resume,
        "n" => MonitorCmd::Step,
        "r" => MonitorCmd::Registers,
        "hw" => MonitorCmd::Hardware,
        _ => MonitorCmd::Unknown,
    };

    (command, arg, arg2)
}

pub enum MonitorCmd {
    Help,
    Unknown,
    Reset,
    PowerCycle,
    Tape,
    Disassemble,
    ReadMem,
    WriteMem,
    Jump,
    ListBreakpoints,
    AddBreakpoint,
    RemoveBreakpoint,
    /// Halts execution immediately ("p"), independently of any breakpoint -
    /// trust-80 only had breakpoint-triggered and manual-resume ("g") stops
    /// before; ported from bytebox's monitor, which has the same pair.
    Pause,
    Resume,
    /// Executes exactly one instruction while paused ("n") and reports the
    /// disassembly of whatever is now at PC - also new, same rationale as
    /// `Pause`: a monitor without single-stepping is missing a basic.
    Step,
    Registers,
    /// Reports on peripherals outside the CPU/memory (currently just the
    /// cassette reader's position) - the TRS-80 equivalent of bytebox's
    /// "hw", scoped down to what this machine actually has: no PSG, no FDC,
    /// no mouse to report on.
    Hardware,
}
