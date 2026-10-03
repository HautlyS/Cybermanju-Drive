pub mod compression;
pub mod crypto;

use wasm_bindgen::prelude::*;

pub use compression::*;
pub use crypto::*;

#[wasm_bindgen(start)]
pub fn init() {
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("Cybermanju Drive WASM module initialized");
}
