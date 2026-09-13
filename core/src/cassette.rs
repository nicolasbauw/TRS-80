#[cfg(feature = "native")]
use std::{fs::File, io, io::prelude::*, path::PathBuf};

pub struct CassetteReader {
    inserted_tape: Vec<u8>,
    serialized_tape: Vec<u8>,
    tape_position: usize,
}

impl Default for CassetteReader {
    fn default() -> Self {
        Self::new()
    }
}

impl CassetteReader {
    pub fn new() -> Self {
        Self {
            inserted_tape: Vec::new(),
            serialized_tape: Vec::new(),
            tape_position: 0,
        }
    }

    fn serialize(&mut self) -> Vec<u8> {
        let mut bits = Vec::new();
        for byte in self.inserted_tape.iter() {
            for bit in (0..=7).rev() {
                bits.push(1); // Sync pulse
                bits.push(((byte & (1 << bit)) != 0) as u8); // Data bit
            }
        }
        bits
    }

    /// Loads tape content already in memory - the only way to load one on
    /// a target with no filesystem (e.g. a wasm/browser build, where a
    /// dropped/picked file arrives as bytes, never a path).
    pub fn load_from_bytes(&mut self, bytes: &[u8]) {
        self.inserted_tape.clear();
        self.inserted_tape.extend_from_slice(bytes);
        self.serialized_tape = self.serialize();
        self.tape_position = 0;
    }

    #[cfg(feature = "native")]
    pub fn load(&mut self, filename: PathBuf) -> io::Result<()> {
        let mut f = File::open(filename)?;
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes)?;
        self.load_from_bytes(&bytes);
        Ok(())
    }

    // Reads the tape and increments its "position"
    pub fn read(&mut self) -> u8 {
        match self.is_end() {
            false => {
                let r = self.serialized_tape[self.tape_position] << 7;
                self.tape_position += 1;
                r
            }
            true => 0,
        }
    }

    // Rewinds the tape
    pub fn rewind(&mut self) {
        self.tape_position = 0;
    }

    // Tests if we have reached the end of the tape data
    fn is_end(&mut self) -> bool {
        self.tape_position >= self.serialized_tape.len()
    }

    pub fn status(&self) -> String {
        if self.serialized_tape.is_empty() {
            "Tape: no tape inserted".to_string()
        } else {
            format!(
                "Tape: position {}/{} bits",
                self.tape_position,
                self.serialized_tape.len()
            )
        }
    }
}
