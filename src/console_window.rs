//! Console window (F11): a separate SDL2 window with its own wgpu/egui
//! context (`zilog_silicon::egui_gpu`), replacing the stdin-driven terminal
//! console entirely - there's no `console.rs` reading `stdin` anymore, the
//! way bytebox's own console window replaced its equivalent.

use crate::monitor::{MonitorMessage, parse_command};
use sdl2::video::Window;
use std::sync::mpsc::Sender;
use zilog_silicon::console_log::ConsoleLog;
use zilog_silicon::egui_gpu::EguiGpu;

pub struct ConsoleWindow {
    gpu: EguiGpu,
    start: std::time::Instant,
    input: String,
    /// Focus must be requested again every time the window is (re)shown.
    request_focus: bool,
    window: Window,
}

impl ConsoleWindow {
    pub fn new(window: Window) -> Result<Self, String> {
        let gpu = EguiGpu::new(&window, "trust-80 console window device")?;
        Ok(Self {
            gpu,
            start: std::time::Instant::now(),
            input: String::new(),
            request_focus: true,
            window,
        })
    }

    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }

    pub fn request_focus(&mut self) {
        self.request_focus = true;
    }

    pub fn handle_event(&mut self, event: &sdl2::event::Event) {
        self.gpu.handle_event(&self.window, event);
    }

    pub fn resize(&mut self) {
        self.gpu.resize(&self.window);
    }

    pub fn render(&mut self, log: &mut ConsoleLog, cmd_sender: &Sender<MonitorMessage>) {
        let input = &mut self.input;
        let request_focus = &mut self.request_focus;

        let bg = egui::Color32::from_rgb(15, 15, 25);
        self.gpu.present(&self.window, self.start, |ctx| {
            egui::TopBottomPanel::bottom("console_input")
                .frame(egui::Frame::default().fill(bg).inner_margin(10.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(">")
                                .monospace()
                                .color(egui::Color32::from_rgb(220, 220, 225)),
                        );
                        let response = ui.add(
                            egui::TextEdit::singleline(input)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace)
                                .hint_text("command (help for help)"),
                        );
                        if *request_focus {
                            response.request_focus();
                            *request_focus = false;
                        }
                        let submitted = response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if submitted {
                            let line = std::mem::take(input);
                            if !line.trim().is_empty() {
                                log.push_command(&line);
                                let _ = cmd_sender.send(parse_command(&line));
                            }
                            *request_focus = true;
                        }
                    });
                });
            egui::CentralPanel::default()
                .frame(egui::Frame::default().fill(bg).inner_margin(10.0))
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for line in log.lines() {
                                ui.label(
                                    egui::RichText::new(line)
                                        .monospace()
                                        .color(egui::Color32::from_rgb(220, 220, 225)),
                                );
                            }
                        });
                });
        });
    }
}
