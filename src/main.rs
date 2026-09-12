use sdl2::{event::Event, keyboard::Keycode};
use std::{
    error::Error,
    process::ExitCode,
    time::{Duration, Instant},
};
mod bus;
mod cassette;
mod config;
mod console_window;
mod display;
mod hexconversion;
mod keyboard;
mod machine;
mod monitor;
use console_window::ConsoleWindow;
use display::DisplayMode;
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

/// Sets `assets/trust-80-logo-2.png` (a variant drawn specifically to stay
/// legible at title-bar/taskbar sizes, unlike the main logo) as the given
/// window's icon. Decoded via the `image` crate (not sdl2's own `"image"`
/// feature, which links the system `libSDL2_image` library) so there's no
/// extra dependency to install - same rationale and pattern as bytebox's
/// own icon loader.
fn set_window_icon(window: &mut sdl2::video::Window) -> Result<(), String> {
    let img = image::load_from_memory(include_bytes!("../assets/trust-80-logo-2.png"))
        .map_err(|e| e.to_string())?
        .into_rgba8();
    let (width, height) = img.dimensions();
    let mut pixels = img.into_raw();
    let pitch = width * 4;
    let surface = sdl2::surface::Surface::from_data(
        &mut pixels,
        width,
        height,
        pitch,
        sdl2::pixels::PixelFormatEnum::RGBA32,
    )
    .map_err(|e| e.to_string())?;
    window.set_icon(&surface);
    Ok(())
}

fn launch() -> Result<(), Box<dyn Error>> {
    // Setting up SDL
    let config = config::load_config_file()?;
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init()?;

    let mut window = video_subsystem
        .window("TRuSt-80", config.display.width, config.display.height)
        .position_centered()
        .resizable()
        // No-op outside macOS; there, required for wgpu to create a
        // surface from this window's handle.
        .metal_view()
        .build()?;
    let main_window_id = window.id();
    if let Err(e) = set_window_icon(&mut window) {
        eprintln!("Can't set window icon: {e}");
    }

    let Ok(font) = ttf_context.load_font(config.display.font, config.display.font_size) else {
        return Err(Box::new(MachineError::FontError));
    };

    // Creating the TRS-80
    let mut trs80 = Machine::new(window, &font)?;
    let cmd_sender = trs80.command_sender();
    let mut console_log = ConsoleLog::new();

    // Console window (F11): replaces the old stdin-driven terminal console
    // entirely, on the same model as bytebox's own console window.
    let mut console_win = video_subsystem
        .window("TRuSt-80 Console", 900, 700)
        .position_centered()
        .hidden()
        .resizable()
        .metal_view()
        .always_on_top()
        .build()?;
    if let Err(e) = set_window_icon(&mut console_win) {
        eprintln!("Can't set console window icon: {e}");
    }
    let console_window_id = console_win.id();
    let mut console_window = ConsoleWindow::new(console_win)?;
    let mut console_visible = false;

    // Machine status window (F12): registers and hardware peripheral state.
    let mut status_win = video_subsystem
        .window("TRuSt-80 Status", 800, 360)
        .position_centered()
        .hidden()
        .resizable()
        .metal_view()
        .always_on_top()
        .build()?;
    if let Err(e) = set_window_icon(&mut status_win) {
        eprintln!("Can't set status window icon: {e}");
    }
    let status_window_id = status_win.id();
    let mut status_panel = StatusPanel::new(status_win, "trust-80 status panel device")?;
    let mut status_visible = false;

    trs80
        .display
        .set_zoom(DisplayMode::from_config(config.display.default_zoom.as_deref()));

    let mut events = sdl_context.event_pump()?;

    // Explicit frame-rate governor, independent of vsync: `Renderer`
    // requests `PresentMode::Fifo`, which should already block each
    // `present()` until the next vsync, but that's not guaranteed to
    // actually throttle on every platform/driver/compositor combination -
    // without a hard backstop here, a vsync that doesn't block leaves this
    // loop free to spin as fast as the CPU allows, pegging a core for no
    // benefit (Machine::cpu_loop's tick budget is computed from actual
    // elapsed time regardless, so it stays correct no matter what paces
    // this loop - this is purely about not burning power). Same pattern
    // bytebox's own main loop uses on top of its own vsync-locked present.
    // 50Hz, same target bytebox paces its own (PAL CPC) frame loop to -
    // cuts the per-frame SDL_ttf/wgpu rendering work (the likely source of
    // high CPU use, not Z80 execution itself) by 5x compared to the 100Hz
    // this was first tried at, without perceptibly hurting responsiveness.
    const FRAME_INTERVAL: Duration = Duration::from_millis(20);
    let mut next_frame = Instant::now();

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
                // Display zoom (F1 half, F2 normal, F3 x2, F4 fullscreen) and
                // the other function-key toggles below: `repeat: false`
                // because these are one-shot toggles, not text to type -
                // without it, a key held a little too long sends repeated
                // KeyDown events from the OS's own typematic repeat and
                // toggles twice in a row almost instantly (bytebox found
                // this the hard way, chasing what looked like an OSD
                // flicker). `window_id == main_window_id`: same reasoning
                // - without it, any of these also fires from the console
                // (F11) or status (F12) window, which have their own focus.
                Event::KeyDown {
                    keycode: Some(Keycode::F1),
                    repeat: false,
                    window_id,
                    ..
                } if window_id == main_window_id => trs80.display.set_zoom(DisplayMode::Half),
                Event::KeyDown {
                    keycode: Some(Keycode::F2),
                    repeat: false,
                    window_id,
                    ..
                } if window_id == main_window_id => trs80.display.set_zoom(DisplayMode::Normal),
                Event::KeyDown {
                    keycode: Some(Keycode::F3),
                    repeat: false,
                    window_id,
                    ..
                } if window_id == main_window_id => trs80.display.set_zoom(DisplayMode::X2),
                Event::KeyDown {
                    keycode: Some(Keycode::F4),
                    repeat: false,
                    window_id,
                    ..
                } if window_id == main_window_id => {
                    // Toggle: F4 exits fullscreen if it's already active.
                    let mode = if trs80.display.current_zoom() == DisplayMode::Fullscreen {
                        DisplayMode::Normal
                    } else {
                        DisplayMode::Fullscreen
                    };
                    trs80.display.set_zoom(mode);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F5),
                    repeat: false,
                    window_id,
                    ..
                } if window_id == main_window_id => trs80.display.toggle_crt(),
                Event::KeyDown {
                    keycode: Some(Keycode::F6),
                    repeat: false,
                    window_id,
                    ..
                } if window_id == main_window_id => trs80.display.toggle_crt_panel(),
                // Also accepted from console_window_id itself, not just the
                // main window: opening it gives it focus (request_focus
                // below), so a re-press of F11 to close it arrives with
                // that window_id - the main_window_id filter alone (added
                // to stop OTHER function keys firing from this window, see
                // the comment on F1) would otherwise silently swallow it.
                Event::KeyDown {
                    keycode: Some(Keycode::F11),
                    repeat: false,
                    window_id,
                    ..
                } if window_id == main_window_id || window_id == console_window_id => {
                    console_visible = !console_visible;
                    if console_visible {
                        console_window.window_mut().show();
                        console_window.request_focus();
                    } else {
                        console_window.window_mut().hide();
                    }
                }
                // Same reasoning as F11 above, for status_window_id: the
                // user can click into the status window even though it
                // doesn't request focus on its own, and F12 should still
                // close it from there.
                Event::KeyDown {
                    keycode: Some(Keycode::F12),
                    repeat: false,
                    window_id,
                    ..
                } if window_id == main_window_id || window_id == status_window_id => {
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
            status_panel.render(&registers, &trs80.bus.tape.borrow().status());
        }

        let now = Instant::now();
        if now < next_frame {
            std::thread::sleep(next_frame - now);
            next_frame += FRAME_INTERVAL;
        } else {
            // Running behind: resync from now rather than trying to catch
            // up, which would just mean spinning flat out for a while.
            next_frame = now;
        }
    }
    Ok(())
}
