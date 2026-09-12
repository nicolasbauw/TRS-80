use sdl2::{event::Event, keyboard::Keycode};
use std::{error::Error, process::ExitCode};
mod cassette;
mod config;
mod console_window;
mod display;
mod hexconversion;
mod keyboard;
mod machine;
mod monitor;
use console_window::ConsoleWindow;
use machine::{Machine, MachineError};
use zilog_silicon::console_log::ConsoleLog;
use zilog_silicon::status_panel::StatusPanel;

fn main() -> ExitCode {
    if let Err(err) = launch() {
        eprintln!("[Error]: {}", err);
        return ExitCode::from(1);
    }
    println!("\n");
    ExitCode::from(0)
}

/// The window a keyboard/focus event belongs to, if any - used to keep
/// keystrokes typed into the console/status windows from leaking into the
/// emulated TRS-80 keyboard matrix, which only cares about the main window.
fn event_window_id(event: &Event) -> Option<u32> {
    match event {
        Event::KeyDown { window_id, .. }
        | Event::KeyUp { window_id, .. }
        | Event::Window { window_id, .. } => Some(*window_id),
        _ => None,
    }
}

fn launch() -> Result<(), Box<dyn Error>> {
    // Setting up SDL
    let config = config::load_config_file()?;
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init()?;

    let window = video_subsystem
        .window("TRuSt-80", config.display.width, config.display.height)
        .position_centered()
        .resizable()
        // No-op outside macOS; there, required for wgpu to create a
        // surface from this window's handle.
        .metal_view()
        .build()?;
    let main_window_id = window.id();

    let Ok(font) = ttf_context.load_font(config.display.font, config.display.font_size) else {
        return Err(Box::new(MachineError::FontError));
    };

    // Creating the TRS-80
    let mut trs80 = Machine::new(window, &font)?;
    let cmd_sender = trs80.command_sender();
    let mut console_log = ConsoleLog::new();

    // Console window (F11): replaces the old stdin-driven terminal console
    // entirely, on the same model as bytebox's own console window.
    let console_win = video_subsystem
        .window("TRuSt-80 Console", 900, 700)
        .position_centered()
        .hidden()
        .resizable()
        .metal_view()
        .always_on_top()
        .build()?;
    let console_window_id = console_win.id();
    let mut console_window = ConsoleWindow::new(console_win)?;
    let mut console_visible = false;

    // Machine status window (F12): registers and hardware peripheral state.
    let status_win = video_subsystem
        .window("TRuSt-80 Status", 640, 360)
        .position_centered()
        .hidden()
        .resizable()
        .metal_view()
        .always_on_top()
        .build()?;
    let status_window_id = status_win.id();
    let mut status_panel = StatusPanel::new(status_win, "trust-80 status panel device")?;
    let mut status_visible = false;

    let mut events = sdl_context.event_pump()?;

    // SDL loop
    'running: loop {
        // CPU loop
        trs80.cpu_loop();

        // SDL events
        for event in events.poll_iter() {
            // Only the main window's own keyboard/focus events should ever
            // reach the emulated TRS-80 keyboard matrix - typing a command
            // into the console window must not also "press" those same keys
            // on the machine, and a focus change on a different window
            // shouldn't clear keys held on this one.
            if event_window_id(&event).is_none_or(|id| id == main_window_id) {
                trs80.keyboard.handle_event(&event);
            }
            // egui_sdl2_event's key translation table doesn't know KpEnter,
            // so it never validates a text field (console window) when hit
            // from the numeric keypad - rewritten to Return for egui's sake
            // only. Must NOT reach trs80.keyboard above: on the real
            // matrix KpEnter and Return are different keys.
            let egui_event = match &event {
                Event::KeyDown {
                    timestamp,
                    window_id,
                    keycode: Some(Keycode::KpEnter),
                    scancode,
                    keymod,
                    repeat,
                } => Event::KeyDown {
                    timestamp: *timestamp,
                    window_id: *window_id,
                    keycode: Some(Keycode::Return),
                    scancode: *scancode,
                    keymod: *keymod,
                    repeat: *repeat,
                },
                Event::KeyUp {
                    timestamp,
                    window_id,
                    keycode: Some(Keycode::KpEnter),
                    scancode,
                    keymod,
                    repeat,
                } => Event::KeyUp {
                    timestamp: *timestamp,
                    window_id: *window_id,
                    keycode: Some(Keycode::Return),
                    scancode: *scancode,
                    keymod: *keymod,
                    repeat: *repeat,
                },
                _ => event.clone(),
            };
            // Each panel's own EguiSDL2State filters events by window_id
            // itself, so every event can be fed to every panel unconditionally.
            trs80.display.handle_event(&egui_event);
            console_window.handle_event(&egui_event);
            status_panel.handle_event(&egui_event);

            match event {
                Event::Quit { .. } => break 'running,
                Event::Window {
                    win_event: sdl2::event::WindowEvent::Close,
                    window_id,
                    ..
                } => {
                    if window_id == main_window_id {
                        break 'running;
                    } else if window_id == console_window_id {
                        console_visible = false;
                        console_window.window_mut().hide();
                    } else if window_id == status_window_id {
                        status_visible = false;
                        status_panel.window_mut().hide();
                    }
                }
                Event::Window {
                    win_event:
                        sdl2::event::WindowEvent::SizeChanged(..) | sdl2::event::WindowEvent::Resized(..),
                    window_id,
                    ..
                } => {
                    if window_id == main_window_id {
                        trs80.display.resize();
                    } else if window_id == console_window_id {
                        console_window.resize();
                    } else if window_id == status_window_id {
                        status_panel.resize();
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F5),
                    ..
                } => trs80.display.toggle_crt(),
                Event::KeyDown {
                    keycode: Some(Keycode::F6),
                    ..
                } => trs80.display.toggle_crt_panel(),
                Event::KeyDown {
                    keycode: Some(Keycode::F11),
                    ..
                } => {
                    console_visible = !console_visible;
                    if console_visible {
                        console_window.window_mut().show();
                        console_window.request_focus();
                    } else {
                        console_window.window_mut().hide();
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F12),
                    ..
                } => {
                    status_visible = !status_visible;
                    if status_visible {
                        status_panel.window_mut().show();
                    } else {
                        status_panel.window_mut().hide();
                    }
                }
                _ => {}
            }
        }

        // Handle SDL keyboard events (keyboard MMIO peripheral)
        trs80.keyboard.update(&mut trs80.bus);

        // Update display
        trs80.display.update(&trs80.bus, &font)?;

        // Handle console commands: processed before rendering the console
        // window so a command's output shows up the same frame it ran, not
        // one frame late.
        if let Ok(output) = trs80.process_console_commands() {
            if !output.is_empty() {
                console_log.push_output(&output);
            }
        }

        if console_visible {
            console_window.render(&mut console_log, &cmd_sender);
        }
        if status_visible {
            let registers = trs80.get_registers_string();
            status_panel.render(&registers, &trs80.tape.status());
        }
    }
    Ok(())
}
