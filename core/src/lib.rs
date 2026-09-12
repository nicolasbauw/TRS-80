//! TRS-80 Model I emulation core: CPU/bus wiring, keyboard matrix, cassette,
//! monitor commands, and character-to-pixel rendering - no windowing, no
//! GPU/rendering backend, no SDL2. Frontends (the desktop app in `src/` at
//! the repo root, and eventually a wasm/web one) depend on this and supply
//! their own event translation, window management, and pixel presentation.

pub mod bus;
pub mod cassette;
pub mod charset;
pub mod hexconversion;
pub mod keyboard;
pub mod keys;
pub mod machine;
pub mod monitor;
pub mod video;
