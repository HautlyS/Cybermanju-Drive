// Cybermanju Drive — Chunk manifest + multi-provider placement (AGENT-2 item 10)
//
// The "decentralized PC" core: a file is split into 4 MiB chunks, each
// addressed by the BLAKE3 of its **plaintext**, and the chunks are placed
// round-robin across every enabled config. The manifest — which copy lives
// where, and the hash of each uploaded artifact — is stored in the
// `sync_files` row (`manifest_ref`), so restore works after eviction and
// across restarts.
//
// Parity choice (documented per the work item): **simple replication, not
// Reed–Solomon.** Chunk `i` is also uploaded to the next
// `min(parity, n-1)` configs in the plan, so every chunk survives that many
// provider losses. Reed–Solomon erasure coding is deferred until there is a
// real n-of-m story in the UI; replication is honest today and degrades to
// the same restore path.
//
// Degrade rule: striping with fewer than 2 enabled configs has nothing to
// stripe across — `pipeline` falls back to whole-file mode with a WARNING
// instead of pretending to be decentralized.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use cybermanju_compression::TripleCompressor;
use cybermanju_crypto::keystore;
use cybermanju_types::sync::SyncConfig;
use serde::{Deserialize, Serialize};

use crate::backends::create_backend;
use crate::pipeline::CYBE_MAGIC;
use crate::transfer;

/// Chunk size for striped placement (4 MiB).
pub const CHUNK_SIZE: u64 = 4 * 1024 * 1024;

/// Manifest format version — bumped when the struct changes shape.
pub const MANIFEST_VERSION: u32 = 1;

/// Where one copy of a chunk lives.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChunkLoc {
    /// Sync config (provider binding) holding this copy.
    pub config_id: String,
    /// Remote locator (always `cybermanju_sync/chunks/{plaintext_hash}` —
    /// content-addressed, so a re-sync overwrites in place).
    pub remote_path: String,
    /// BLAKE3 of the exact bytes uploaded here (ciphertext when encryption
    /// is on) — the download-back verification baseline for this copy.
    pub artifact_hash: String,
}

/// One plaintext chunk and every copy of it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChunkEntry {
    /// Position in the file (0-based).
    pub index: u32,
    /// BLAKE3 of the plaintext chunk — the chunk's address and restore
    /// verification baseline.
    pub hash: String,
    /// Plaintext size of this chunk (`CHUNK_SIZE` except the last).
    pub size: u64,
    pub primary: ChunkLoc,
    /// Additional copies (see parity in [`placements`]).
    pub replicas: Vec<ChunkLoc>,
}

/// The full placement record for one striped file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChunkManifest {
    pub version: u32,
    /// BLAKE3 of the whole plaintext file (restore's final check).
    pub file_hash: String,
    pub chunk_size: u64,
    pub total_size: u64,
    pub chunks: Vec<ChunkEntry>,
    /// Replicas per chunk at plan time (for diagnostics; the actual copies
    /// are recorded per entry).
    pub parity: u8,
}

/// Placement of chunk `index` across `n` participants: primary index plus
/// replica indices. `n >= 1`; replicas are clamped to `n - 1` so a copy
/// never lands twice on one provider.
pub fn placements(index: usize, n: usize, parity: u8) -> (usize, Vec<usize>) {
    let n = n.max(1);
    let replicas = (parity as usize).min(n.saturating_sub(1));
    let primary = index % n;
    let replica_idxs = (1..=replicas).map(|j| (index + j) % n).collect();
    (primary, replica_idxs)
}

/// Content-addressed remote path for a chunk (plaintext hash).
pub fn chunk_remote_path(plaintext_hash: &str) -> String {
    format!("cybermanju_sync/chunks/{}", plaintext_hash)
}

/// Undo whatever the sync pipeline did to a payload: decrypt (magic-prefixed
/// sealed blob), then decompress when it is a compressed artifact. Best
/// effort — the caller must still verify the plaintext hash; this function
/// never fabricates data, it only unwraps known layers.
pub fn decode_artifact(mut bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    if bytes.starts_with(CYBE_MAGIC) {
        let passphrase = keystore::master_passphrase().ok_or_else(|| {
            "integrity: artifact is encrypted but no master passphrase is available".to_string()
        })?;
        bytes = keystore::open_sealed(&passphrase, &bytes[CYBE_MAGIC.len()..])
            .map_err(|e| format!("integrity: could not decrypt: {}", e))?;
    }
    if let Ok((plain, _size)) = TripleCompressor::new().decompress_triple(&bytes) {
        return Ok(plain);
    }
    Ok(bytes)
}

/// Reassemble a striped file: download each chunk (primary, then replicas),
/// verify every chunk against its plaintext hash, verify the assembled file
/// against `manifest.file_hash`, and only then publish `dest`.
///
/// Writes stream to `{dest}.restore.part` — a partial download never touches
/// the destination path, and any failure removes the sidecar.
pub fn restore(
    manifest: &ChunkManifest,
    configs: &[SyncConfig],
    dest: &str,
) -> Result<u64, String> {
    if manifest.version != MANIFEST_VERSION {
        return Err(format!(
            "unsupported: manifest version {} (this build speaks {})",
            manifest.version, MANIFEST_VERSION
        ));
    }

    let by_id: HashMap<&str, &SyncConfig> = configs.iter().map(|c| (c.id.as_str(), c)).collect();
    let part = format!("{}.restore.part", dest);

    let outcome = (|| -> Result<u64, String> {
        let mut out =
            fs::File::create(&part).map_err(|e| format!("restore create failed: {}", e))?;
        let mut written: u64 = 0;

        for entry in &manifest.chunks {
            let mut restored: Option<Vec<u8>> = None;
            let mut errors: Vec<String> = Vec::new();

            for loc in std::iter::once(&entry.primary).chain(entry.replicas.iter()) {
                let config = match by_id.get(loc.config_id.as_str()) {
                    Some(config) => *config,
                    None => {
                        errors.push(format!(
                            "chunk {}: config '{}' is gone",
                            entry.index, loc.config_id
                        ));
                        continue;
                    }
                };
                let backend = match create_backend(config) {
                    Ok(backend) => backend,
                    Err(e) => {
                        errors.push(format!("chunk {}: {}", entry.index, e));
                        continue;
                    }
                };

                // Unique per manifest (file hash prefix) so two concurrent
                // restores of different files never share a sidecar.
                let tmp = std::env::temp_dir().join(format!(
                    "cybermanju-chunk-{}-{}-{}-{}.part",
                    std::process::id(),
                    &manifest.file_hash[..8.min(manifest.file_hash.len())],
                    entry.index,
                    loc.config_id
                ));
                let tmp_str = tmp.to_string_lossy().to_string();

                let attempt = backend
                    .download_file(&loc.remote_path, &tmp_str)
                    .and_then(|()| {
                        fs::read(&tmp).map_err(|e| format!("restore read failed: {}", e))
                    });
                let _ = fs::remove_file(&tmp);

                match attempt {
                    Ok(bytes) => match decode_artifact(bytes) {
                        Ok(plain) => {
                            if transfer::blake3_hex(&plain) == entry.hash {
                                restored = Some(plain);
                                break;
                            }
                            errors.push(format!(
                                "chunk {}: copy on '{}' failed hash check",
                                entry.index, loc.config_id
                            ));
                        }
                        Err(e) => errors.push(format!("chunk {}: {}", entry.index, e)),
                    },
                    Err(e) => errors.push(format!(
                        "chunk {}: copy on '{}': {}",
                        entry.index, loc.config_id, e
                    )),
                }
            }

            let plain = restored.ok_or_else(|| {
                format!(
                    "integrity: chunk {} could not be restored from any copy ({})",
                    entry.index,
                    errors.join("; ")
                )
            })?;
            out.write_all(&plain)
                .map_err(|e| format!("restore write failed: {}", e))?;
            written += plain.len() as u64;
        }

        out.flush()
            .map_err(|e| format!("restore flush failed: {}", e))?;
        drop(out);

        if written != manifest.total_size {
            return Err(format!(
                "integrity: restored {} bytes, manifest says {}",
                written, manifest.total_size
            ));
        }
        let file_hash = transfer::hash_file(&part)?;
        if file_hash != manifest.file_hash {
            return Err("integrity: reassembled file does not match the manifest hash".to_string());
        }

        if let Some(parent) = Path::new(dest).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!("restore could not create '{}': {}", parent.display(), e)
                })?;
            }
        }
        fs::rename(&part, dest).map_err(|e| format!("restore publish failed: {}", e))?;
        Ok(written)
    })();

    if outcome.is_err() {
        let _ = fs::remove_file(&part);
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placements_round_robin_with_one_replica() {
        let n = 3;
        let parity = 1;
        assert_eq!(placements(0, n, parity), (0, vec![1]));
        assert_eq!(placements(1, n, parity), (1, vec![2]));
        assert_eq!(placements(2, n, parity), (2, vec![0]));
        assert_eq!(placements(3, n, parity), (0, vec![1]));
    }

    #[test]
    fn placements_clamp_replicas_to_n_minus_one() {
        // Asked for 5 copies on 3 providers — every provider gets one copy,
        // never two on the same one.
        let (primary, replicas) = placements(1, 3, 5);
        assert_eq!(primary, 1);
        assert_eq!(replicas, vec![2, 0]);
        // parity 0 → primary only
        assert_eq!(placements(4, 3, 0), (1, vec![]));
        // single participant → no replicas possible
        assert_eq!(placements(7, 1, 3), (0, vec![]));
    }

    #[test]
    fn chunk_paths_are_content_addressed() {
        let a = chunk_remote_path("abc123");
        let b = chunk_remote_path("abc123");
        assert_eq!(a, b);
        assert_eq!(a, "cybermanju_sync/chunks/abc123");
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let manifest = ChunkManifest {
            version: MANIFEST_VERSION,
            file_hash: "f".repeat(64),
            chunk_size: CHUNK_SIZE,
            total_size: CHUNK_SIZE + 10,
            chunks: vec![ChunkEntry {
                index: 0,
                hash: "a".repeat(64),
                size: CHUNK_SIZE,
                primary: ChunkLoc {
                    config_id: "cfg-1".to_string(),
                    remote_path: chunk_remote_path(&"a".repeat(64)),
                    artifact_hash: "b".repeat(64),
                },
                replicas: vec![ChunkLoc {
                    config_id: "cfg-2".to_string(),
                    remote_path: chunk_remote_path(&"a".repeat(64)),
                    artifact_hash: "b".repeat(64),
                }],
            }],
            parity: 1,
        };
        let json = serde_json::to_string(&manifest).expect("serialize");
        let back: ChunkManifest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(manifest, back);
        assert!(json.contains("artifactHash"));
    }

    #[test]
    fn decode_passes_plain_bytes_through() {
        let raw = b"not compressed, not sealed".to_vec();
        assert_eq!(decode_artifact(raw.clone()).unwrap(), raw);
    }
}
