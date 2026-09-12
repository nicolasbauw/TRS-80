use crate::cassette::CassetteReader;
use std::cell::RefCell;
use std::io;
use zilog_z80::bus::{Bus, FlatBus};

/// TRS-80 Model I bus: RAM/ROM (a plain `FlatBus`) plus the cassette reader,
/// wired to port 0xFF via `read_io` - the port zilog_z80's now fully
/// implemented `IN A,(n)`/`OUT (n),A` actually go through, unlike when this
/// app had to fake cassette input by pre-empting the opcode before
/// `execute()` and overwriting A behind its back.
pub struct TrsBus {
    mem: FlatBus,
    // RefCell: `read_io` takes `&self` (an ordinary port read has no
    // business needing `&mut` on paper), but advancing the tape on read is
    // inherently stateful - the same interior-mutability pattern bytebox
    // uses for its own peripherals reached through the Bus trait.
    pub tape: RefCell<CassetteReader>,
    pub debug_io: bool,
}

impl TrsBus {
    pub fn new(size: u16) -> Self {
        Self {
            mem: FlatBus::new(size),
            tape: RefCell::new(CassetteReader::new()),
            debug_io: false,
        }
    }

    pub fn set_romspace(&mut self, start: u16, end: u16) {
        self.mem.set_romspace(start, end);
    }

    pub fn read_mem_slice(&self, start: usize, end: usize) -> Vec<u8> {
        self.mem.read_mem_slice(start, end)
    }

    pub fn clear_mem_slice(&mut self, start: usize, end: usize) {
        self.mem.clear_mem_slice(start, end);
    }

    pub fn load_bin(&mut self, file: &str, org: u16) -> io::Result<usize> {
        self.mem.load_bin(file, org)
    }
}

impl Bus for TrsBus {
    fn read_byte(&self, address: u16) -> u8 {
        self.mem.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        self.mem.write_byte(address, data);
    }

    fn read_io(&self, port: u16) -> u8 {
        // The cassette port is decoded on the low byte only, ignoring
        // whatever's in A (the high byte of `port`, per the Z80's IN A,(n)
        // address formation) - matches the ROM's own "IN A,(0FFH)".
        let value = if port & 0xFF == 0xFF {
            self.tape.borrow_mut().read()
        } else {
            0xFF
        };
        if self.debug_io {
            println!("IN {:#04X} on port {:#06X}", value, port);
        }
        value
    }

    fn write_io(&mut self, port: u16, data: u8) {
        if self.debug_io {
            println!("OUT {:#04X} on port {:#06X}", data, port);
        }
    }
}
