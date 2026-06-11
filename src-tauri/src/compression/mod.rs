// Cybermanju Drive — Triple-Layer Compression Module
// Layer 1: lz4_flex (ultra-fast, real-time previews)
// Layer 2: zstd (balanced, general storage)
// Layer 3: brotli (maximum, archival)

pub mod triple;

pub use triple::*;