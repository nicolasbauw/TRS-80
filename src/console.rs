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
                let command = parts.next().unwrap_or_default().to_string();
                let arg = parts.next().unwrap_or_default().to_string();
                let arg2 = parts.next().unwrap_or_default().to_string();

                cmd_channel.send((command, arg, arg2))?;
                thread::sleep(Duration::from_millis(75));
            }
        },
    )?;
    Ok(())
}
