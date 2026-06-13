use crate::types::LayerDetail;
use anyhow::Result;
use std::io::Write;

pub struct BrotliLayer;

impl BrotliLayer {
    pub const NAME: &'static str = "Layer 3: Brotli";
    pub const COLOR: &'static str = "#FFB800";
    pub const DEFAULT_LEVEL: u32 = 11;

    pub fn compress(data: &[u8], level: u32) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut encoder = brotli::CompressorWriter::new(&mut out, 4096, level, 22);
        encoder.write_all(data)?;
        drop(encoder);
        Ok(out)
    }

    pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut decoder = brotli::DecompressorWriter::new(&mut out, 4096);
        decoder.write_all(data)?;
        drop(decoder);
        Ok(out)
    }

    pub fn compress_with_detail(data: &[u8], level: u32) -> Result<(Vec<u8>, LayerDetail)> {
        let input_size = data.len() as u64;
        let output = Self::compress(data, level)?;
        let output_size = output.len() as u64;
        Ok((
            output,
            LayerDetail {
                name: Self::NAME.into(),
                algorithm: format!("brotli level {level}"),
                input_size,
                output_size,
                ratio: if input_size > 0 {
                    output_size as f64 / input_size as f64
                } else {
                    1.0
                },
                color: Self::COLOR.into(),
            },
        ))
    }
}
