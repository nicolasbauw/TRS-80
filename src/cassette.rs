use std::{thread, fs, fs::File, io::prelude::*, sync::Arc, sync::Mutex};
use sdl2::keyboard::Keycode;
use zilog_z80::crossbeam_channel::{Sender, Receiver};

fn serialize(input: Vec<u8>) -> Vec<u8> {
    let mut bits = Vec::new();
    for byte in input.iter() {
        for bit in (0..=7).rev() {
            bits.push(1);                                   // Sync pulse
            bits.push(((byte & (1 << bit)) != 0) as u8);    // Data bit
        }
    }
    bits
}

fn load() -> Vec<u8> {
    let tape_filename = fs::read_to_string("tape/filename.txt").expect("Could not get tape image file name");
    let mut f = File::open(tape_filename).expect("Could open tape image");
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("Could not read tape image file");
    serialize(buf)
}

pub fn launch(cassette_receiver: Receiver<(u8,u8)>, cassette_sender: Sender<(u8,u8)>, cassette_req: Receiver<u8>, cassette_cmd_rx: Receiver<Keycode>) {
    let pos = Arc::new(Mutex::new(0));
    let t_pos = Arc::clone(&pos);
    let t_pos1 = Arc::clone(&pos);

    let bits = Arc::new(Mutex::new(load()));
    let t_bits = Arc::clone(&bits);
    let t_bits1 = Arc::clone(&bits);

    // 0xFF IO peripheral (Cassette) CPU -> Cassette
    thread::Builder::new()
        .name(String::from("Cassette writer"))
        .spawn(move || {
            loop {
                // Data sent from CPU to cassette ? (OUT)
                if let Ok((device, _)) = cassette_receiver.recv() {
                    if device == 0xFF {
                         continue;
                    }
                }
            }
        })
        .expect("Could not create cassette writer thread");

    // 0xFF IO peripheral (Cassette) Cassette -> CPU
    thread::Builder::new()
        .name(String::from("Cassette reader"))
        .spawn(move || {
            loop {
                if let Ok(device) = cassette_req.recv() {
                    let mut tape_pos = t_pos.lock().expect("Could not lock position counter");
                    let tape_bits = t_bits.lock().expect("Could not lock tape data");
                    // IN instruction for the 0xFF device ?
                    if device == 0xFF && *tape_pos < tape_bits.len() {
                        // We send the data through the io_in channel
                        if *tape_pos < tape_bits.len() {
                            cassette_sender.send((0xFF, tape_bits[*tape_pos] << 7)).expect("Cassette message send error");
                        }
                    } else if device == 0xFF && *tape_pos >= tape_bits.len() {
                        cassette_sender.send((0xFF, 0x00)).expect("Cassette message send error");
                    }
                    if *tape_pos < tape_bits.len() { *tape_pos += 1; }
                }
            }
        })
        .expect("Could not create cassette reader thread");

    thread::Builder::new()
        .name(String::from("Cassette data request"))
        .spawn(move || {
            loop {
                if let Ok(Keycode::F7) = cassette_cmd_rx.recv() {
                    let mut tape_bits = t_bits1.lock().expect("Could not lock tape data");
                    *tape_bits = load();
                    let mut pos = t_pos1.lock().expect("Could not lock position counter");
                    *pos = 0;
                }
            }
        })
        .expect("Could not create cassette data request thread");
}