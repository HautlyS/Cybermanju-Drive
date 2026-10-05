pub mod compression;
pub mod crypto;
pub mod db;
pub mod opfs_backend;
pub mod os;
pub mod agent;

use wasm_bindgen::prelude::*;

pub use compression::*;
pub use crypto::*;
pub use db::*;
pub use os::*;

#[wasm_bindgen(start)]
pub fn init() {
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("Cybermanju Drive WASM module initialized");
}
