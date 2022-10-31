use std::thread;
use sdl2::keyboard::Keycode;
use zilog_z80::crossbeam_channel::{Sender, Receiver};
use std::{fs, fs::File, io::prelude::*};

pub fn serialize(input: Vec<u8>) -> Vec<u8> {
    let mut bits = Vec::new();
    for byte in input.iter() {
        for bit in (0..=7).rev() {
            bits.push(1);                                   // Sync pulse
            bits.push(((byte & (1 << bit)) != 0) as u8);    // Data bit
        }
    }
    bits
}

pub fn load() -> Vec<u8> {
    let tape_filename = fs::read_to_string("tape/filename.txt").expect("Could not get tape image file name");
    let mut f = File::open(tape_filename).expect("Could open tape image");
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("Could not read tape image file");
    serialize(buf)
}

pub fn launch(cassette_receiver: Receiver<(u8,u8)>, cassette_sender: Sender<(u8,u8)>, cassette_req: Receiver<u8>, cassette_cmd_rx: Receiver<Keycode>) {
    // 0xFF IO peripheral (Cassette) CPU -> Cassette
    thread::spawn(move || {
        loop {
            // Data sent from CPU to cassette ? (OUT)
            if let Ok((device, _)) = cassette_receiver.recv() {
                if device == 0xFF {
                     continue;
                }
            }
        }
    });

    // 0xFF IO peripheral (Cassette) Cassette -> CPU
    thread::spawn(move || {
        let mut tape_bits = load();
        let mut tape_pos = 0;
        loop {
            if let Ok(device) = cassette_req.try_recv() {
                // IN instruction for the 0xFF device ?
                if device == 0xFF && tape_pos < tape_bits.len() {
                    // We send the data through the io_in channel
                    if tape_pos < tape_bits.len() {
                        cassette_sender.send((0xFF, tape_bits[tape_pos] << 7)).expect("Cassette message send error");
                    }
                } else if device == 0xFF && tape_pos >= tape_bits.len() {
                    cassette_sender.send((0xFF, 0x00)).expect("Cassette message send error");
                }
                if tape_pos < tape_bits.len() { tape_pos += 1; }
            }
            if let Ok(Keycode::F7) = cassette_cmd_rx.try_recv() {
                tape_bits = load();
                tape_pos = 0;
            }
        }
    });
}