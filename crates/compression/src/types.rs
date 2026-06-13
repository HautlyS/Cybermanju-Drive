use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CompressionType {
    None,
    Lz4,
    Zstd,
    TripleLayer,
}

impl CompressionType {
    pub fn display_name(&self) -> &str {
        match self {
            Self::None => "Uncompressed",
            Self::Lz4 => "LZ4 (Real-Time)",
            Self::Zstd => "Zstandard (Balanced)",
            Self::TripleLayer => "Triple-Layer (LZ4→ZSTD→Brotli)",
        }
    }

    pub fn color(&self) -> &str {
        match self {
            Self::None => "#6B7280",
            Self::Lz4 => "#00D4FF",
            Self::Zstd => "#00FF41",
            Self::TripleLayer => "#FFB800",
        }
    }

    pub fn speed_label(&self) -> &str {
        match self {
            Self::None => "N/A",
            Self::Lz4 => "Ultra-Fast (~400 MB/s)",
            Self::Zstd => "Fast (configurable 1-22)",
            Self::TripleLayer => "Slow (maximum ratio)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    pub original_size: u64,
    pub compressed_size: u64,
    pub ratio: f64,
    pub layer: String,
    pub layer_details: Vec<LayerDetail>,
    pub blake3_hash: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerDetail {
    pub name: String,
    pub algorithm: String,
    pub input_size: u64,
    pub output_size: u64,
    pub ratio: f64,
    pub color: String,
}
