use std::{thread, io::stdin, io::stdout, io::Write};

pub fn launch(cassette_cmd: zilog_z80::crossbeam_channel::Sender<(String,String)>) {
    thread::Builder::new()
        .name(String::from("Console"))
        .spawn(move || {
            loop {
                print!("> ");
                stdout().flush().unwrap();

                let mut input = String::new();
                stdin().read_line(&mut input).unwrap();
                
                let mut parts = input.trim().split_whitespace();
                let command = parts.next().unwrap();
                let args = parts;

                match command {
                    "tape" => { cassette_cmd.send((String::from(command), args.peekable().peek().unwrap().to_string())).unwrap() },
                    _ => continue
                }
            }
        })
        .expect("Could not create console thread");
}