//! Renders VRAM content into an RGB24 pixel buffer using the baked-in
//! bitmap charset (see `charset.rs`) - no font library, no GPU backend, so
//! any frontend (desktop wgpu, or eventually a wasm/web one) can just
//! upload whatever this produces.

use crate::bus::TrsBus;
use crate::charset::{CELL_H, CELL_W, CHARSET};
use zilog_z80::bus::Bus;

pub const COLS: usize = 64;
pub const ROWS: usize = 16;
pub const SCREEN_WIDTH: usize = COLS * CELL_W;
pub const SCREEN_HEIGHT: usize = ROWS * CELL_H;

const VRAM_START: u16 = 0x3C00;
const TEXT_COLOR: (u16, u16, u16) = (219, 220, 250);

/// Renders every row of VRAM into `buf`, an RGB24 buffer of exactly
/// `SCREEN_WIDTH * SCREEN_HEIGHT * 3` bytes.
pub fn render(bus: &TrsBus, buf: &mut [u8]) {
    debug_assert_eq!(buf.len(), SCREEN_WIDTH * SCREEN_HEIGHT * 3);
    for row in 0..ROWS {
        render_row(bus, row, buf);
    }
}

/// Renders a single text row (0..ROWS) into `buf`. Split out from
/// `render()` so a frontend can skip re-rendering rows whose VRAM content
/// hasn't changed since the last frame - re-rendering all 16 rows
/// unconditionally every frame is measurably wasteful once a frontend is
/// calling this dozens of times a second (the desktop `Display` does this
/// dirty-row tracking; this function itself has no notion of "changed").
pub fn render_row(bus: &TrsBus, row: usize, buf: &mut [u8]) {
    debug_assert_eq!(buf.len(), SCREEN_WIDTH * SCREEN_HEIGHT * 3);
    let y0 = row * CELL_H;
    for col in 0..COLS {
        let addr = VRAM_START + (row * COLS + col) as u16;
        let code = bus.read_byte(addr);
        let x0 = col * CELL_W;
        let glyph = &CHARSET[code as usize];
        for gy in 0..CELL_H {
            let dst_row_start = (y0 + gy) * SCREEN_WIDTH * 3;
            for gx in 0..CELL_W {
                let a = glyph[gy * CELL_W + gx] as u16;
                let dst = dst_row_start + (x0 + gx) * 3;
                buf[dst] = ((TEXT_COLOR.0 * a) / 255) as u8;
                buf[dst + 1] = ((TEXT_COLOR.1 * a) / 255) as u8;
                buf[dst + 2] = ((TEXT_COLOR.2 * a) / 255) as u8;
            }
        }
    }
}
