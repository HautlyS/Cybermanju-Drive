pub mod compression;
pub mod crypto;
pub mod os;

use wasm_bindgen::prelude::*;

pub use compression::*;
pub use crypto::*;
pub use os::*;

#[wasm_bindgen(start)]
pub fn init() {
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("Cybermanju Drive WASM module initialized");
}
