use std::{io, fs::File, io::prelude::*, path::PathBuf};
//use crate::config;

pub struct CassetteReader {
    inserted_tape: Vec<u8>,
    pub serialized_tape: Vec<u8>,
    pub tape_position: usize,
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
                bits.push(1);                                   // Sync pulse
                bits.push(((byte & (1 << bit)) != 0) as u8);    // Data bit
            }
        }
        bits
    }

    pub fn load(&mut self, filename: PathBuf) -> io::Result<()> {
        let mut f = File::open(filename)?;
        f.read_to_end(&mut self.inserted_tape)?;
        self.serialized_tape = self.serialize();
        Ok(())
    }
}
