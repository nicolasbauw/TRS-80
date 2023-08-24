use sdl2::{pixels::Color,event::Event,keyboard::Keycode, EventPump, render::Canvas, video::Window};
use zilog_z80::{bus::Bus, cpu::CPU};
use std::{error::Error, thread, time::Duration, collections::HashSet, fs};
use crate::config::Config;
mod display;
//mod keyboard;
//mod cassette;
mod config;
//mod console;

fn main() -> Result<(), Box<dyn Error>> {
    // Setting up SDL
    let config = config::load_config_file()?;
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
 
    let window = video_subsystem.window("TRuSt-80", config.display.width, config.display.height)
        .position_centered()
        .build()?;
 
    let mut canvas = window.into_canvas()
        .accelerated()
        .present_vsync()
        .build()?;

    let display_mode = video_subsystem.current_display_mode(0)?;
    let refresh_rate =  display_mode.refresh_rate;

    // Setting up CPU
    let bus = std::rc::Rc::new(std::cell::RefCell::new(Bus::new(config.memory.ram)));
    let mut c = CPU::new(bus.clone());
    c.debug.io = config.debug.iodevices.unwrap_or(false);
    c.debug.instr_in = config.debug.iodevices.unwrap_or(false);
    //c.debug.opcode = true;
    if refresh_rate == 50 { 
        c.set_slice_duration(20);       // Matching a 50 Hz refresh rate
    }
    c.set_freq(1.77);               // Model 1 : 1.77 MHz
    bus.borrow_mut().load_bin(&config.memory.rom, 0)?;
    let rom_space = fs::metadata(&config.memory.rom)?.len();
    bus.borrow_mut().set_romspace(0, (rom_space - 1) as u16);
    //let (vram_tx, vram_rx) = bounded(1);

    loop {
        loop {
            // executes slice_max_cycles number of cycles
            if let Some(t) = c.execute_timed() {
                //thread::sleep(Duration::from_millis(t.into()));
                //println!("{t}");
                break;
            } 
        }
        let vram = bus.borrow().read_mem_slice(0x3C00, 0x4000);
        tape_device(bus.clone());
        if !sdl_loop(&sdl_context, &mut canvas, vram, &config)? { break };
    }
    Ok(())
}

fn sdl_loop(ctx: &sdl2::Sdl, canvas: &mut Canvas<Window>, vram: Vec<u8>, config: &Config) -> Result<bool, Box<dyn Error>> {
    //canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    display::display(canvas, vram, &config).unwrap();
    canvas.present();
    let mut events = ctx.event_pump()?;
    for event in events.poll_iter() {
        match event {
            Event::Quit {..} |
            Event::KeyDown { keycode: Some(Keycode::F12), .. } => {
                return Ok(false)
            },
            _ => {}
        }
    }
    Ok(true)
}

fn tape_device(bus: std::rc::Rc<core::cell::RefCell<Bus>>) {
    let d = bus.borrow_mut().get_io_out(0xFF);
    if d != 0 {
        println!("Device 0xFF received {}", d);
    }
}