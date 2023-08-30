use std::{io::stdin, io::stdout, io::Write, sync::mpsc, thread, time::Duration};

pub fn launch(
    cmd_channel: mpsc::Sender<(String, String, String)>,
) -> Result<(), Box<dyn std::error::Error>> {
    thread::Builder::new().name(String::from("Console")).spawn(
        move || -> Result<(), mpsc::SendError<(String, String, String)>> {
            loop {
                print!("> ");
                if stdout().flush().is_err() {
                    continue;
                };

                let mut input = String::new();
                if stdin().read_line(&mut input).is_err() {
                    continue;
                };

                let mut parts = input.split_whitespace();
                let command = parts.next();
                let arg = parts.next();
                let arg2 = parts.next();

                match command {
                    Some("tape") => match arg {
                        Some(arg) => {
                            cmd_channel.send((String::from("tape"), arg.to_string(), String::new()))?
                        }
                        None => {
                            println!("No tape filename !")
                        }
                    },
                    Some("d") => match arg {
                        Some(address) => {
                            cmd_channel.send((String::from("d"), address.to_string(), String::new()))?
                        }
                        None => {
                            println!("No address !")
                        }
                    },
                    Some("m") => {
                        if arg2.is_none() { cmd_channel.send((String::from("m"), arg.unwrap().to_string(), String::new()))? } else {
                            cmd_channel.send((String::from("m"), arg.unwrap().to_string(), arg2.unwrap().to_string()))?
                        }
                    },
                    _ => cmd_channel.send((String::from(command.unwrap()), String::new(), String::new()))?,
                }
                thread::sleep(Duration::from_millis(75));
            }
        },
    )?;
    Ok(())
}
