use std::{thread, io::stdin, io::stdout, io::Write, time::Duration};

pub fn launch(cassette_cmd: zilog_z80::crossbeam_channel::Sender<(String,String)>) -> Result<(), Box<dyn std::error::Error>> {
    thread::Builder::new()
        .name(String::from("Console"))
        .spawn(move || -> Result<(), std::io::Error> {
            loop {
                print!("> ");
                stdout().flush()?;

                let mut input = String::new();
                if stdin().read_line(&mut input).is_err() { continue };
                
                let mut parts = input.trim().split_whitespace();
                let Some(command) = parts.next() else { continue };
                let args = parts;

                match command {
                    "tape" => {
                        match args.peekable().peek() {
                            Some(tape) => { cassette_cmd.send((String::from(command), tape.to_string())).unwrap() },
                            None => { println!("No tape filename !") }
                        }
                    },
                    _ => continue
                }
                thread::sleep(Duration::from_millis(75));
            }
        })?;
        Ok(())
}