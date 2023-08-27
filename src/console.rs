use std::{thread, io::stdin, io::stdout, io::Write, time::Duration, sync::mpsc};

pub fn launch(cmd_channel: mpsc::Sender<(String,String)>) -> Result<(), Box<dyn std::error::Error>> {
    thread::Builder::new()
        .name(String::from("Console"))
        .spawn(move || -> Result<(), mpsc::SendError<(String,String)>> {
            loop {
                print!("> ");
                if stdout().flush().is_err() { continue };

                let mut input = String::new();
                if stdin().read_line(&mut input).is_err() { continue };
                
                let mut parts = input.trim().split_whitespace();
                let Some(command) = parts.next() else { continue };
                let args = parts;

                match command {
                    "tape" => {
                        match args.peekable().peek() {
                            Some(tape) => { cmd_channel.send((String::from(command), tape.to_string()))? },
                            None => { println!("No tape filename !") }
                        }
                    },
                    "reset" => { cmd_channel.send((String::from(command), String::new()))? }
                    _ => continue
                }
                thread::sleep(Duration::from_millis(75));
            }
        })?;
        Ok(())
}