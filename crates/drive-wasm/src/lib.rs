pub mod crypto;
pub mod compression;

use wasm_bindgen::prelude::*;

pub use crypto::*;
pub use compression::*;

#[wasm_bindgen(start)]
pub fn init() {
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("Cybermanju Drive WASM module initialized");
}
