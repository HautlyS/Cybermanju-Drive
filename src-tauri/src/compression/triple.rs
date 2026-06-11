// Cybermanju Drive — Triple-Layer Compression Pipeline
// lz4_flex → zstd-15 → brotli-11 cascading for maximum compression
// BLAKE3 content-defined chunking for deduplication

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::Write;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Compression layer type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompressionType {
    None,
    Lz4,
    Zstd,
    TripleLayer, // LZ4 → ZSTD → Brotli
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

/// Stats for a single compression operation
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

/// Detail for each compression layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerDetail {
    pub name: String,
    pub algorithm: String,
    pub input_size: u64,
    pub output_size: u64,
    pub ratio: f64,
    pub color: String,
}

// ---------------------------------------------------------------------------
// TripleCompressor
// ---------------------------------------------------------------------------

/// The triple-layer compression engine
pub struct TripleCompressor {
    zstd_level: i32,
    brotli_level: u32,
}

impl TripleCompressor {
    pub fn new() -> Self {
        Self {
            zstd_level: 15,   // High quality
            brotli_level: 11, // Maximum
        }
    }

    /// Layer 1: LZ4 compression — ultra-fast for real-time previews
    pub fn compress_lz4(&self, data: &[u8]) -> Result<Vec<u8>> {
        let compressed = lz4_flex::compress_prepend_size(data);
        Ok(compressed)
    }

    /// Layer 1: LZ4 decompression
    pub fn decompress_lz4(&self, data: &[u8]) -> Result<Vec<u8>> {
        let decompressed = lz4_flex::decompress_size_prepended(data)?;
        Ok(decompressed)
    }

    /// Layer 2: Zstandard compression — balanced ratio/speed
    pub fn compress_zstd(&self, data: &[u8]) -> Result<Vec<u8>> {
        let compressed = zstd::encode_all(data, self.zstd_level)?;
        Ok(compressed)
    }

    /// Layer 2: Zstandard decompression
    pub fn decompress_zstd(&self, data: &[u8]) -> Result<Vec<u8>> {
        let decompressed = zstd::decode_all(data)?;
        Ok(decompressed)
    }

    /// Layer 3: Brotli compression — maximum ratio for archival
    pub fn compress_brotli(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut compressed = Vec::new();
        let mut encoder = brotli::CompressorWriter::new(&mut compressed, 4096, self.brotli_level, 22);
        encoder.write_all(data)?;
        drop(encoder);
        Ok(compressed)
    }

    /// Layer 3: Brotli decompression
    pub fn decompress_brotli(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut decompressed = Vec::new();
        let mut decoder = brotli::DecompressorWriter::new(&mut decompressed, 4096);
        decoder.write_all(data)?;
        drop(decoder);
        Ok(decompressed)
    }

    // -----------------------------------------------------------------------
    // Single-layer compression with stats
    // -----------------------------------------------------------------------

    /// Compress data with a single named layer and return the result + detail.
    ///
    /// `layer` must be one of: "lz4", "zstd", "brotli".
    pub fn compress_data(&self, data: &[u8], layer: &str) -> Result<(Vec<u8>, LayerDetail)> {
        let input_size = data.len() as u64;
        let start = std::time::Instant::now();

        let (output, name, algorithm, color) = match layer {
            "lz4" => (
                self.compress_lz4(data)?,
                "Layer 1: LZ4".to_string(),
                "lz4_flex".to_string(),
                "#00D4FF".to_string(),
            ),
            "zstd" => (
                self.compress_zstd(data)?,
                "Layer 2: Zstandard".to_string(),
                format!("zstd level {}", self.zstd_level),
                "#00FF41".to_string(),
            ),
            "brotli" => (
                self.compress_brotli(data)?,
                "Layer 3: Brotli".to_string(),
                format!("brotli level {}", self.brotli_level),
                "#FFB800".to_string(),
            ),
            other => anyhow::bail!("Unknown compression layer: {}", other),
        };

        let output_size = output.len() as u64;
        let ratio = if input_size > 0 {
            output_size as f64 / input_size as f64
        } else {
            1.0
        };

        let detail = LayerDetail {
            name,
            algorithm,
            input_size,
            output_size,
            ratio,
            color,
        };

        let _duration_ms = start.elapsed().as_millis() as u64;

        Ok((output, detail))
    }

    // -----------------------------------------------------------------------
    // Triple-layer compression / decompression
    // -----------------------------------------------------------------------

    /// Triple-layer compression: LZ4 → ZSTD → Brotli.
    ///
    /// Returns the compressed bytes AND the full `CompressionStats` so callers
    /// can report timing, per-layer ratios, and BLAKE3 hash.
    pub fn compress_triple(&self, data: &[u8]) -> Result<(Vec<u8>, CompressionStats)> {
        let original_size = data.len() as u64;
        let start = std::time::Instant::now();

        // Layer 1: LZ4
        let lz4_out = self.compress_lz4(data)?;
        let lz4_detail = LayerDetail {
            name: "Layer 1: LZ4".into(),
            algorithm: "lz4_flex".into(),
            input_size: original_size,
            output_size: lz4_out.len() as u64,
            ratio: if original_size > 0 { lz4_out.len() as f64 / original_size as f64 } else { 1.0 },
            color: "#00D4FF".into(),
        };

        // Layer 2: ZSTD
        let zstd_out = self.compress_zstd(&lz4_out)?;
        let zstd_detail = LayerDetail {
            name: "Layer 2: Zstandard".into(),
            algorithm: format!("zstd level {}", self.zstd_level),
            input_size: lz4_out.len() as u64,
            output_size: zstd_out.len() as u64,
            ratio: if lz4_out.len() > 0 { zstd_out.len() as f64 / lz4_out.len() as f64 } else { 1.0 },
            color: "#00FF41".into(),
        };

        // Layer 3: Brotli
        let brotli_out = self.compress_brotli(&zstd_out)?;
        let brotli_detail = LayerDetail {
            name: "Layer 3: Brotli".into(),
            algorithm: format!("brotli level {}", self.brotli_level),
            input_size: zstd_out.len() as u64,
            output_size: brotli_out.len() as u64,
            ratio: if zstd_out.len() > 0 { brotli_out.len() as f64 / zstd_out.len() as f64 } else { 1.0 },
            color: "#FFB800".into(),
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        let hash = blake3::hash(data);

        let layer_details = vec![lz4_detail, zstd_detail, brotli_detail.clone()];

        let stats = CompressionStats {
            original_size,
            compressed_size: brotli_out.len() as u64,
            ratio: if original_size > 0 { brotli_out.len() as f64 / original_size as f64 } else { 1.0 },
            layer: "triple".into(),
            layer_details,
            blake3_hash: hash.to_hex().to_string(),
            duration_ms,
        };

        Ok((brotli_out, stats))
    }

    /// Triple-layer decompression: Brotli → ZSTD → LZ4.
    ///
    /// Returns the decompressed bytes AND the duration in milliseconds so
    /// callers can report decompression timing.
    pub fn decompress_triple(&self, data: &[u8]) -> Result<(Vec<u8>, u64)> {
        let start = std::time::Instant::now();

        let brotli_out = self.decompress_brotli(data)?;
        let zstd_out = self.decompress_zstd(&brotli_out)?;
        let lz4_out = self.decompress_lz4(&zstd_out)?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok((lz4_out, duration_ms))
    }

    /// Compute BLAKE3 hash for deduplication
    pub fn blake3_hash(data: &[u8]) -> String {
        blake3::hash(data).to_hex().to_string()
    }
}