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

                cmd_channel.send((command.unwrap_or_default().to_string(), arg.unwrap_or_default().to_string(), arg2.unwrap_or_default().to_string()))?;
                thread::sleep(Duration::from_millis(75));
            }
        },
    )?;
    Ok(())
}
