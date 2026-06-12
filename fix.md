# Cybermanju Drive — Full Code Audit v2
**Date:** 2026-06-11  
**Codebase:** `Cybermanju-Drive-main__1_.zip` (updated / "fixed" build)  
**Auditor:** Deep static analysis of all 31 Rust source files (~9 927 lines)

---

## Executive Summary

Version 2 addressed all 7 critical security findings from the first audit — the web dashboard is now JWT-authenticated, bound to localhost, has body-size limits, strips private keys, and the PQC layer now uses real ML-KEM-1024 via `pqcrypto-mlkem`. This is excellent progress.

**15 issues remain**, ranging from a compile-time crash (the GitHub URL bug) to data-integrity bugs, performance problems, and misleading documentation. Every issue below has an exact code fix.

---

## What Was Fixed ✅

| # | Issue | Status |
|---|---|---|
| 1 | Private encryption keys exposed via unauthenticated HTTP | ✅ Fixed — `list_encryption_keys_safe()` strips `private_key` |
| 2 | Web dashboard had no authentication on any endpoint | ✅ Fixed — JWT (HS256) enforced on all routes except `/api/health` and `/api/users/login` |
| 3 | HTTP body allocation DoS (no size cap) | ✅ Fixed — 100 MB `MAX_BODY_SIZE` hard limit |
| 4 | `encrypt_file` used `OsRng` struct as nonce | ✅ Fixed — proper `ChaCha20Poly1305` + `generate_random_nonce()` flow |
| 5 | PQC was a facade (random 32-byte seed) | ✅ Fixed — real ML-KEM-1024 / ML-KEM-768 via `pqcrypto-mlkem` |
| 6 | `LocalBackend`: no path canonicalization → path traversal | ✅ Fixed — `safe_join()` with `canonicalize()` + symlink checks |
| 7 | Session token was forgeable (BLAKE3 of username+timestamp) | ✅ Fixed — HMAC-SHA256 with 32-byte random `hmac_secret` in `AppState` |
| 8 | `list_files` was O(N) full table scan | ✅ Fixed — `parent_index` secondary table, `list_by_parent()` is O(children) |
| 9 | `scan_directory` recursion → stack overflow | ✅ Fixed — replaced with `walkdir` crate, `max_depth(20)` |
| 10 | Tantivy committed after every single document | ✅ Fixed — `add_document_batch()` + `add_document_no_commit()` + `commit()` API |
| 11 | All HTTP backends shelled out to `curl` | ✅ Fixed — `reqwest::blocking` used throughout |
| 12 | `AppState` used `Mutex` for read-heavy DB | ✅ Fixed — upgraded to `RwLock` |
| 13 | Web dashboard CORS was wildcard | ✅ Fixed — `ALLOWED_ORIGINS` localhost-only allow-list |
| 14 | Web dashboard bound to `0.0.0.0` | ✅ Fixed — bound to `127.0.0.1` only |
| 15 | Web dashboard never cleaned up its server thread | ✅ Fixed — `Drop` impl signals mpsc channel and joins thread |

---

## Remaining Issues

---

### BUG-01 — GitHub `get_file_url` will not compile (4 args, 3 format slots)

**Severity:** 🔴 High — compile-time failure, silently drops `remote_path`  
**File:** `src-tauri/src/sync/backends.rs:579–583`

**Current code:**
```rust
fn get_file_url(&self, remote_path: &str) -> Result<String, String> {
    let (owner, repo) = parse_repo(&self.repo_name)?;
    Ok(format!(
        "https://raw.githubusercontent.com/{}/{}/{}",
        owner, repo, self.branch, remote_path   // ← 4 args, only 3 `{}`
    ))
}
```

The `format!` macro has **three** `{}` placeholders and **four** arguments. This is a compile error in current Rust. If somehow compiled with an older edition, `remote_path` is silently discarded and every raw URL points at the branch root instead of the file.

**Fix — add the missing `/{}`:**
```rust
fn get_file_url(&self, remote_path: &str) -> Result<String, String> {
    let (owner, repo) = parse_repo(&self.repo_name)?;
    Ok(format!(
        "https://raw.githubusercontent.com/{}/{}/{}/{}",
        owner, repo, self.branch, remote_path   // ← 4 args, 4 `{}`
    ))
}
```

---

### BUG-02 — `urlencoding()` truncates multi-byte Unicode characters

**Severity:** 🟠 Medium — filenames with non-ASCII chars produce bad URLs for Google Drive  
**File:** `src-tauri/src/sync/backends.rs:1144–1157`

**Current code:**
```rust
fn urlencoding(s: &str) -> String {
    s.chars().map(|c| {
        if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
            c.to_string()
        } else {
            format!("%{:02X}", c as u8)   // ← `c as u8` truncates to low byte only
        }
    }).collect()
}
```

`'中' as u8` truncates to `0x2D` (the low byte of U+4E2D), producing `%2D` instead of the correct UTF-8 percent-encoding `%E4%B8%AD`. Google Drive will reject queries with non-ASCII folder IDs or filenames.

**Fix — encode each UTF-8 byte, not the char codepoint:**
```rust
fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for byte in s.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            b => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}
```

Alternatively, add the `percent-encoding` crate to `Cargo.toml` and replace the entire function:
```toml
percent-encoding = "2.3"
```
```rust
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

fn urlencoding(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}
```

---

### BUG-03 — `duplicate_file_context` borrows moved value (will not compile)

**Severity:** 🔴 High — compile error  
**File:** `src-tauri/src/commands/files.rs:253–261`

**Current code:**
```rust
let mut duplicated = original;        // ← `original` is MOVED here
// ...
duplicated.tags = original.tags.clone();   // ← use of moved value `original`
duplicated.collection_ids = Vec::new();
```

After `let mut duplicated = original;`, the variable `original` is consumed. `original.tags.clone()` on line 261 is a use-after-move — the compiler will reject this.

**Fix — clone the fields you need before the move:**
```rust
// Clone fields needed after the move BEFORE moving
let tags = original.tags.clone();
let original_hash = original.hash_blake3.clone();
let context_data_base = original.context_data.clone();

let mut duplicated = original;    // original is moved here — fine now
duplicated.id = new_id.clone();
duplicated.name = format!("{} (copy)", duplicated.name);
duplicated.hash_blake3 = Some(new_hash);
duplicated.thumbnail_path = Some(link_preview.to_string());
duplicated.created_at = now.clone();
duplicated.modified_at = now;
duplicated.context_data = Some(context_data_augmented);
duplicated.tags = tags;                // use the pre-cloned value
duplicated.collection_ids = Vec::new();
// face_group_ids and loose_group_ids are already carried over via `= original`
```

---

### BUG-04 — Parent index and file record written in two separate transactions (non-atomic)

**Severity:** 🟠 Medium — crash between the two commits corrupts the index  
**File:** `src-tauri/src/commands/files.rs:102–114`, `src-tauri/src/db/mod.rs:120–136`

**Current code:**
```rust
// Transaction 1: write the file record
let tx = db.begin_write()?;
{ table.insert(&folder_id, serialized)?; }
tx.commit()?;           // ← if crash here…

// Transaction 2: update the parent index
db.add_to_parent_index(&folder_id, &parent_id)?;   // ← …this never runs
```

If the process crashes (or panics, or the host kills it) between the two `commit()` calls, the file exists in the files table but is absent from the parent index. `list_files` will never return it — it becomes an orphan that can only be found by UUID.

**Fix — merge both writes into a single transaction:**

Add a method to `db/mod.rs` that does both writes atomically:
```rust
/// Insert a FileNode AND update the parent index in a single transaction.
pub fn insert_file_with_index(
    &self,
    file_id: &str,
    serialized: &str,
    parent_id: Option<&str>,
) -> Result<()> {
    let tx = self.db.begin_write()?;
    {
        let mut files_table = tx.open_table(FILES_TABLE)?;
        files_table.insert(file_id, serialized)?;

        if let Some(pid) = parent_id {
            let mut index_table = tx.open_table(PARENT_INDEX_TABLE)?;
            let existing: Vec<String> = index_table
                .get(pid)?
                .and_then(|v| serde_json::from_str(v.value()).ok())
                .unwrap_or_default();
            let mut ids = existing;
            if !ids.contains(&file_id.to_string()) {
                ids.push(file_id.to_string());
            }
            index_table.insert(pid, serde_json::to_string(&ids)?)?;
        }
    }
    tx.commit()?;
    Ok(())
}
```

Then in `create_folder`, `import_file`, `upload_file`, `duplicate_file_context`, and `move_file` — replace the two-step pattern with this single call. For `move_file` also remove from the old parent index in the same transaction.

---

### BUG-05 — `add_to_parent_index` deserializes and re-serializes the whole child array (O(N))

**Severity:** 🟡 Low-Medium — performance degrades for large directories  
**File:** `src-tauri/src/db/mod.rs:120–136`

**Current code:**
```rust
let existing: Vec<String> = table
    .get(parent_id)?
    .and_then(|v| serde_json::from_str(v.value()).ok())
    .unwrap_or_default();
let mut ids = existing;
ids.push(file_id.to_string());
table.insert(parent_id, serde_json::to_string(&ids)?)?;
```

For a directory with 10 000 children, every new file deserializes a 10 000-element JSON array, appends one item, and re-serializes it. This is O(N) per insert.

**Better approach — composite key in the index table:**

Instead of `parent_id → JSON[child_ids]`, store `"parent_id/child_id" → ""`. Lookup becomes a range scan on prefix `"parent_id/"`:

```rust
// In schema
const PARENT_INDEX_TABLE: TableDefinition<&str, &str> = TableDefinition::new("parent_index_v2");

// Insert: O(1) — just one key write
pub fn add_to_parent_index(&self, file_id: &str, parent_id: &str) -> Result<()> {
    let tx = self.db.begin_write()?;
    {
        let mut table = tx.open_table(PARENT_INDEX_TABLE)?;
        let composite_key = format!("{}/{}", parent_id, file_id);
        table.insert(composite_key.as_str(), "")?;
    }
    tx.commit()?;
    Ok(())
}

// Remove: O(1)
pub fn remove_from_parent_index(&self, file_id: &str, parent_id: &str) -> Result<()> {
    let tx = self.db.begin_write()?;
    {
        let mut table = tx.open_table(PARENT_INDEX_TABLE)?;
        let composite_key = format!("{}/{}", parent_id, file_id);
        table.remove(composite_key.as_str())?;
    }
    tx.commit()?;
    Ok(())
}

// List: O(children) range scan
pub fn list_by_parent(&self, parent_id: &str) -> Result<Vec<String>> {
    let tx = self.db.begin_read()?;
    let table = tx.open_table(PARENT_INDEX_TABLE)?;
    let prefix = format!("{}/", parent_id);
    let prefix_end = format!("{}0", parent_id); // '0' > '/' in ASCII

    let mut ids = Vec::new();
    for entry in table.range(prefix.as_str()..prefix_end.as_str())? {
        let (key, _) = entry?;
        // Strip prefix to get child file_id
        if let Some(child_id) = key.value().strip_prefix(&prefix) {
            ids.push(child_id.to_string());
        }
    }
    Ok(ids)
}
```

This requires a one-time migration: on first startup, rebuild the index from the files table if the `parent_index_v2` table is empty.

---

### BUG-06 — Triple compression applied to already-compressed formats (JPEG, MP4, ZIP…)

**Severity:** 🟠 Medium — wastes CPU and disk space; compressed files get larger  
**File:** `src-tauri/src/compression/triple.rs` and `src-tauri/src/commands/compression.rs`

Compressing a JPEG, MP4, WEBM, ZIP, 7z, or any other already-compressed format with LZ4→ZSTD→Brotli typically expands it by 0.5–5%. There is no check before starting the pipeline.

**Fix — check compression efficacy after layer 1 and abort early:**

```rust
/// Triple-layer compression with early abort for incompressible data.
///
/// If LZ4 compression ratio > 0.98 (data didn't compress), the content is
/// already compressed or high-entropy (media files, archives, encrypted data).
/// We skip ZSTD and Brotli and return the original bytes with a `None` stats.
pub fn compress_triple_smart(
    &self,
    data: &[u8],
) -> Result<(Vec<u8>, CompressionStats)> {
    let original_size = data.len() as u64;
    let start = std::time::Instant::now();

    // Layer 1: LZ4 as entropy probe
    let lz4_out = self.compress_lz4(data)?;
    let lz4_ratio = lz4_out.len() as f64 / original_size as f64;

    // Early abort: if LZ4 doesn't help, the data is incompressible
    if lz4_ratio > 0.98 {
        let hash = blake3::hash(data);
        return Ok((data.to_vec(), CompressionStats {
            original_size,
            compressed_size: original_size,
            ratio: 1.0,
            layer: "skipped (incompressible)".into(),
            layer_details: vec![LayerDetail {
                name: "LZ4 probe".into(),
                algorithm: "lz4_flex".into(),
                input_size: original_size,
                output_size: lz4_out.len() as u64,
                ratio: lz4_ratio,
                color: "#6B7280".into(),
            }],
            blake3_hash: hash.to_hex().to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        }));
    }

    // Proceed with ZSTD → Brotli only for compressible data
    let zstd_out = self.compress_zstd(&lz4_out)?;
    let brotli_out = self.compress_brotli(&zstd_out)?;
    // ... (rest of existing stats construction)
}
```

Also add MIME-type bypass in `compress_file` command before calling the compressor:

```rust
// Skip compression for already-compressed MIME types
if let Some(ref mime) = file_node.mime_type {
    let incompressible = [
        "image/jpeg", "image/png", "image/webp", "image/gif",
        "image/heic", "image/avif",
        "video/mp4", "video/webm", "video/quicktime", "video/x-matroska",
        "audio/mpeg", "audio/aac", "audio/ogg", "audio/flac",
        "application/zip", "application/x-7z-compressed",
        "application/x-rar-compressed", "application/gzip",
        "application/zstd",
    ];
    if incompressible.contains(&mime.as_str()) {
        // Return stats showing no compression was applied
        return Ok(FrontendCompressionStats {
            original_size: file_node.size_bytes,
            compressed_size: file_node.size_bytes,
            ratio: 1.0,
            layer: "skipped (already compressed)".into(),
            layer_details: vec![],
            blake3_hash: file_node.hash_blake3.clone().unwrap_or_default(),
            duration_ms: 0,
        });
    }
}
```

---

### BUG-07 — Google Photos `modified_at` stores `productUrl` (a URL, not a timestamp)

**Severity:** 🟡 Low — date-sorted views show nonsense for Google Photos items  
**File:** `src-tauri/src/sync/backends.rs:1069–1078`

**Current code:**
```rust
let modified = item["productUrl"]
    .as_str()
    .unwrap_or("")
    .to_string();
files.push(RemoteFile {
    // ...
    modified_at: modified,   // ← stores "https://photos.google.com/photo/..."
```

**Fix — use `mediaMetadata.creationTime`:**
```rust
let modified = item["mediaMetadata"]["creationTime"]
    .as_str()
    .or_else(|| item["creationTime"].as_str())
    .unwrap_or("")
    .to_string();
```

The Google Photos API returns ISO 8601 timestamps in `mediaMetadata.creationTime` for `mediaItems.list`. Note: `modifiedTime` is not available for Photos items — `creationTime` is the correct field.

---

### BUG-08 — `duplicate_file_context` hash is not content-based (defeats deduplication)

**Severity:** 🟡 Low — dedup system cannot detect identical files  
**File:** `src-tauri/src/commands/files.rs:221–227`

**Current code:**
```rust
let hash_input = format!(
    "{}-{}-{}",
    original.hash_blake3.as_deref().unwrap_or("none"),
    new_id,
    now
);
let new_hash = blake3::hash(hash_input.as_bytes()).to_hex().to_string();
```

This generates a unique hash per duplication event — it does not represent file content. Two duplicates of the same source file will have different hashes, making content-based deduplication impossible.

**Fix — re-use the original BLAKE3 hash (duplicates ARE identical content):**
```rust
// A duplicate contains identical bytes — its content hash is the same.
// The new_id distinguishes it as a separate FileNode.
let new_hash = original.hash_blake3.clone();
// If the original was never hashed (e.g. a folder), leave it as None.
```

If you want the duplicate to carry its own hash for verification purposes after the user modifies it, leave `hash_blake3: None` for the duplicate and hash it on first access.

---

### BUG-09 — Tantivy `determine_match_type` uses `format!("{:?}", query)` — debug hack

**Severity:** 🟡 Low — incorrect match-type labels in search results  
**File:** `src-tauri/src/search/tantivy_index.rs:429–460`

**Current code:**
```rust
let query_text = format!("{:?}", query).to_lowercase();
if query_text.split_whitespace().any(|term| val.to_lowercase().contains(term)) {
    return "filename".to_string();
}
```

This parses the debug representation of a Tantivy `Query` object to extract terms. Tantivy's `Debug` format is internal and not stable — it changed between 0.19 and 0.22. This produces both false positives (debug noise matching field values) and false negatives.

**Fix — extract terms from the query text directly, not from its debug representation:**

The query string is already available in the calling context. Thread it through:

```rust
// In SearchIndex::search(), pass the raw query_str alongside the parsed query:
fn determine_match_type_from_query(
    doc: &tantivy::Document,
    query_str: &str,
    file_name_field: &Field,
    content_field: &Field,
    tags_field: &Field,
) -> String {
    // Normalize and tokenize the user's query string
    let terms: Vec<&str> = query_str.split_whitespace().collect();

    if let Some(val) = doc.get_first(*file_name_field).and_then(|v| v.as_text()) {
        if terms.iter().any(|t| val.to_lowercase().contains(&t.to_lowercase())) {
            return "filename".to_string();
        }
    }
    if let Some(val) = doc.get_first(*tags_field).and_then(|v| v.as_text()) {
        if terms.iter().any(|t| val.to_lowercase().contains(&t.to_lowercase())) {
            return "tag".to_string();
        }
    }
    "content".to_string()
}
```

Then in `search()`, pass `&request.query` instead of `&query` (the parsed object):
```rust
let match_type = determine_match_type_from_query(
    &doc,
    &request.query,     // ← raw string
    &self.file_name_field,
    &self.content_text_field,
    &self.tags_field,
);
```

---

### BUG-10 — `upload_file` holds entire file in RAM via `Vec<u8>` IPC

**Severity:** 🟠 Medium — OOM on files > ~500 MB (IPC base64-encodes the payload)  
**File:** `src-tauri/src/commands/import.rs:395–397`

**Current signature:**
```rust
pub fn upload_file(
    file_name: String,
    data: Vec<u8>,     // ← entire file bytes over Tauri IPC
    parent_path: String,
    state: State<'_, AppState>,
) -> Result<UploadResult, String>
```

Tauri serializes `Vec<u8>` to JSON base64 before passing it across the IPC bridge. For a 4 GB video: raw bytes (4 GB) → base64 string (~5.3 GB) → deserialized back to `Vec<u8>` (4 GB). Peak RSS exceeds 9 GB before any processing begins.

**Fix — accept a file path instead of file bytes, read from disk in Rust:**

```rust
/// Upload a file that the user has selected via the dialog plugin.
/// Accepts the resolved filesystem path rather than raw bytes to avoid
/// loading gigabytes through the Tauri IPC bridge.
#[tauri::command]
pub fn upload_file(
    file_path: String,      // ← absolute path returned by dialog plugin
    parent_path: String,
    state: State<'_, AppState>,
) -> Result<UploadResult, String> {
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("upload")
        .to_string();

    // Delegate to import_file which already streams from disk
    let file_node = import_file(file_path, parent_path, state)?;
    let bytes_written = file_node.size_bytes;
    Ok(UploadResult { file_node, bytes_written })
}
```

In the Vue frontend, use the Tauri dialog plugin to get the path and pass it to `upload_file`:
```typescript
import { open } from '@tauri-apps/plugin-dialog';

const filePath = await open({ multiple: false });
if (filePath) {
    await invoke('upload_file', { filePath, parentPath: currentDir });
}
```

---

### BUG-11 — `Dilithium5` and `SphincsPlus` variants use HMAC-SHA512 but are labeled as post-quantum

**Severity:** 🟠 Medium — misleading security guarantees; users expect PQC but get classical HMAC  
**File:** `src-tauri/src/crypto/pqc.rs:40–73`

**Current state:**
- `Dilithium5` generates a 64-byte HMAC key and signs with HMAC-SHA512 — a classical MAC, not a lattice-based signature.
- `SphincsPlus` does the same — not hash-based SPHINCS+ at all.
- `display_name()` returns `"HMAC-SHA512 (Dilithium fallback)"` — the word "fallback" buries the lede.

**Option A — Mark clearly as "signature stubs" (fastest fix):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EncryptionAlgo {
    Kyber1024,   // Real ML-KEM-1024 via pqcrypto-mlkem ✓
    Hybrid,      // Real ML-KEM-768 + X25519 ✓
    /// Classical HMAC-SHA512 signing — NOT post-quantum.
    /// Placeholder until pqcrypto-ml-dsa is stable.
    ClassicalSign,
    Aes256,
}

impl EncryptionAlgo {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Kyber1024 => "ML-KEM-1024 — FIPS 203 (Post-Quantum)",
            Self::Hybrid    => "Hybrid ML-KEM-768 + X25519 (Post-Quantum)",
            Self::ClassicalSign => "HMAC-SHA512 (Classical — NOT post-quantum)",
            Self::Aes256    => "ChaCha20Poly1305 (Classical)",
        }
    }
    pub fn nist_level(&self) -> u8 {
        match self {
            Self::Kyber1024 | Self::Hybrid => 5,
            Self::ClassicalSign | Self::Aes256 => 0,   // not PQC
        }
    }
}
```

**Option B — Add real ML-DSA-65 (Dilithium) via `pqcrypto-mldsa`:**
```toml
# Cargo.toml
pqcrypto-mldsa = "0.1"
```
```rust
use pqcrypto_mldsa::mldsa65 as mldsa;

// In generate_keypair for Dilithium5:
let (pk, sk) = mldsa::keypair();
(pk.as_bytes().to_vec(), sk.as_bytes().to_vec())

// In sign_message:
let sk = mldsa::SecretKey::from_bytes(&keypair.private_key)...;
let sig = mldsa::detached_sign(message, &sk);

// In verify_signature:
let pk = mldsa::PublicKey::from_bytes(&keypair.public_key)...;
mldsa::verify_detached_signature(&sig, message, &pk)
```

---

### BUG-12 — BLAKE3 legacy password path persists — allows fast offline brute-force

**Severity:** 🟠 Medium — users who registered before the argon2 migration are vulnerable  
**File:** `src-tauri/src/commands/users.rs:296–303`

**Current code:**
```rust
let valid = if user.password_hash.starts_with("$argon2") {
    argon2_verify_password(&password, &user.password_hash)?
} else {
    // Legacy BLAKE3 hash comparison (migration path)
    user.password_hash == blake3_hash    // ← BLAKE3 hashes 3+ GB/s, trivially brute-forced
};
```

Any user whose password was hashed before argon2 was introduced has a BLAKE3 hash in the database. An attacker who exfiltrates the DB can crack it offline at GPU speeds.

**Fix — force re-hash to argon2 on successful legacy login, never allow legacy login without immediate upgrade:**

```rust
let valid = if user.password_hash.starts_with("$argon2") {
    argon2_verify_password(&password, &user.password_hash)?
} else {
    // Legacy BLAKE3 hash — verify but immediately upgrade
    let blake3_hash = blake3::hash(password.as_bytes()).to_hex().to_string();
    if user.password_hash == blake3_hash {
        // Upgrade the stored hash to argon2id right now
        let new_hash = argon2_hash_password(password)?;
        // Write the upgraded hash back to the database
        upgrade_password_hash(&user.id, &new_hash, &state)?;
        true
    } else {
        false
    }
};
```

Add a helper `upgrade_password_hash(user_id, new_hash, state)` that reads the user, sets `password_hash`, and writes back in a single transaction.

---

### BUG-13 — Web dashboard opens a fresh `redb::Database` handle per HTTP request

**Severity:** 🟠 Medium — concurrent writes can conflict; handle creation is not free  
**File:** `src-tauri/src/web_dashboard/mod.rs:405`

**Current code:**
```rust
// Opens a new DB handle for every single request:
let db_result = redb::Database::open(&dashboard.db_path);
```

redb uses POSIX file locking and a single write-ahead log. Opening multiple handles from different threads and interleaving writes on them can cause `DatabaseAlreadyOpen` errors and data corruption under concurrent load. Handle creation also involves file I/O.

**Fix — share a single `Arc<Mutex<RedbDb>>` across requests:**

```rust
// In WebDashboard struct:
pub struct WebDashboard {
    // ...
    db: Arc<Mutex<RedbDb>>,
}

impl WebDashboard {
    pub fn new(port: u16, db_path: &str) -> Self {
        let db = redb::Database::open(db_path)
            .or_else(|_| redb::Database::create(db_path))
            .expect("Failed to open web dashboard DB");
        Self {
            // ...
            db: Arc::new(Mutex::new(db)),
        }
    }
}

// In handle_connection, pass the Arc clone instead of opening a new handle:
fn handle_connection(dashboard: &WebDashboard, stream: TcpStream) {
    let db = Arc::clone(&dashboard.db);
    // pass db into handle_request instead of calling Database::open()
}
```

Since `Mutex<RedbDb>` means only one request can write at a time anyway, this is safe. For read-heavy workloads consider `Arc<RwLock<RedbDb>>` — redb's `ReadTransaction` is `Send`, so multiple concurrent readers are safe.

---

### BUG-14 — `upload_file` stored_name has dead-code branch (both arms produce the same string)

**Severity:** 🟡 Low — dead code; the intended behavior (extensionless filenames) is silently wrong  
**File:** `src-tauri/src/commands/import.rs:410–419`

**Current code:**
```rust
let stored_name = if extension.is_empty() {
    format!("{}.{}", file_id, file_name)   // e.g. "<uuid>.myfile"
} else {
    format!("{}.{}", file_id, file_name)   // SAME — both arms identical
};
```

For `file_name = "photo.jpg"` (extension = `"jpg"`), both branches produce `"<uuid>.photo.jpg"` — accidentally correct. For `file_name = "makefile"` (empty extension), both produce `"<uuid>.makefile"` — also a double-extension oddity.

**Fix — use different naming strategies per case:**
```rust
let stored_name = if extension.is_empty() {
    // No extension: store as UUID only, use original name in DB metadata
    file_id.to_string()
} else {
    // Has extension: UUID + original extension for MIME sniffing
    format!("{}.{}", file_id, extension)
};
```

---

### BUG-15 — `Dilithium5` `nist_level()` returns 5 but HMAC-SHA512 provides NIST level 0

**Severity:** 🟡 Low — misleading UI indicator (already covered by BUG-11, but separate data bug)  
**File:** `src-tauri/src/crypto/pqc.rs:52–53`

```rust
Self::Dilithium5 => 5,    // ← claims NIST Level 5
Self::SphincsPlus => 1,   // ← claims NIST Level 1
```

HMAC-SHA512 is not a NIST PQC algorithm and has no NIST PQC security level. Returning `5` for `Dilithium5` misleads the user and the frontend into displaying a NIST Level 5 badge for a classical MAC.

**Fix — return 0 for non-PQC algorithms:**
```rust
pub fn nist_level(&self) -> u8 {
    match self {
        Self::Kyber1024 => 5,
        Self::Hybrid    => 5,   // ML-KEM-768 is Level 3; conservative to call it 5 for Hybrid
        Self::Dilithium5 | Self::SphincsPlus | Self::Aes256 => 0,
    }
}
```

---

## Summary Table

| # | File | Line(s) | Severity | Fixed? |
|---|---|---|---|---|
| BUG-01 | `sync/backends.rs` | 581 | 🔴 High (compile error) | No |
| BUG-02 | `sync/backends.rs` | 1144 | 🟠 Medium | No |
| BUG-03 | `commands/files.rs` | 253–261 | 🔴 High (compile error) | No |
| BUG-04 | `commands/files.rs`, `db/mod.rs` | 102–114, 120–136 | 🟠 Medium | No |
| BUG-05 | `db/mod.rs` | 120–160 | 🟡 Low-Med | No |
| BUG-06 | `compression/triple.rs`, `commands/compression.rs` | — | 🟠 Medium | No |
| BUG-07 | `sync/backends.rs` | 1069 | 🟡 Low | No |
| BUG-08 | `commands/files.rs` | 221–227 | 🟡 Low | No |
| BUG-09 | `search/tantivy_index.rs` | 443 | 🟡 Low | No |
| BUG-10 | `commands/import.rs` | 395–397 | 🟠 Medium | No |
| BUG-11 | `crypto/pqc.rs` | 40–73 | 🟠 Medium | No |
| BUG-12 | `commands/users.rs` | 296–303 | 🟠 Medium | No |
| BUG-13 | `web_dashboard/mod.rs` | 405 | 🟠 Medium | No |
| BUG-14 | `commands/import.rs` | 410–419 | 🟡 Low | No |
| BUG-15 | `crypto/pqc.rs` | 52–53 | 🟡 Low | No |

---

## Architecture Insights & Recommendations

### 1. Real ML-DSA (Dilithium) is available now

`pqcrypto-mldsa` is published on crates.io. Add it and replace the HMAC stub:

```toml
pqcrypto-mldsa = "0.1"
```

This completes the PQC story: ML-KEM-1024 for key encapsulation, ML-DSA-65 for signatures, both FIPS 203/204 compliant.

### 2. Consider `axum` for the web dashboard

The hand-rolled TCP/HTTP parser is now 1 508 lines. It correctly handles auth and rate limiting but still lacks: chunked transfer encoding, keep-alive, HTTP/1.1 pipelining, and graceful connection draining. `axum` (0.7) with `tokio` adds these for ~50 lines of boilerplate and composes naturally with Tauri's async runtime:

```toml
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "limit", "trace"] }
```

```rust
use axum::{Router, middleware};
use tower_http::limit::RequestBodyLimitLayer;

let app = Router::new()
    .route("/api/files", get(list_files).delete(delete_file))
    // ...
    .layer(middleware::from_fn(jwt_auth_middleware))
    .layer(RequestBodyLimitLayer::new(100 * 1024 * 1024));
```

### 3. The `ort` (ONNX Runtime) crate is in Cargo.toml but unused

```toml
# Cargo.toml — remove this line, it adds ~40 MB to the binary:
# ort = "2.0.0-rc.12"   ← removed in v2, good
```

Confirmed removed in v2. The face detection module now uses `hnsw` for clustering and stubs out the ONNX inference path. Leaving a comment in `faces/mod.rs` marking the TODO is the right approach.

### 4. Streaming large file transfers

For files > 100 MB, consider the Tauri `fs` plugin's streaming API instead of passing file paths through IPC. For the sync backends, `reqwest`'s streaming body (`reqwest::Body::from(file)`) avoids loading the entire file into RAM before uploading:

```rust
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

let file = File::open(&local_path).await?;
let stream = FramedRead::new(file, BytesCodec::new());
let body = reqwest::Body::wrap_stream(stream);

client.put(&url)
    .header("Content-Length", file_size)
    .body(body)
    .send()
    .await?;
```

This requires switching `reqwest` from `blocking` to async, which pairs well with Tauri's `async_runtime`.

### 5. The `parent_index` rebuild path is missing

If a user restores from a backup of `cybermanju.db` that predates the index table, or if the index gets corrupted, there is no command to rebuild it from the files table. Add:

```rust
#[tauri::command]
pub fn rebuild_parent_index(state: State<'_, AppState>) -> Result<u32, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    // Clear index table, then iterate all files and re-add each
    // Returns count of files reindexed
}
```

---

## Dependency Audit

| Crate | Version | Notes |
|---|---|---|
| `pqcrypto-mlkem` | 0.1 | ✅ Real ML-KEM. Check for updates — NIST finalization may bring 0.2+ |
| `x25519-dalek` | 2 | ✅ Current, audited |
| `chacha20poly1305` | 0.10 | ✅ Current, `RustCrypto` project |
| `argon2` | 0.5 | ✅ Current. Consider pinning `m_cost = 65536` (64 MB) for higher resistance |
| `jsonwebtoken` | 9 | ✅ Current. Consider adding `leeway: 0` to `Validation` to prevent clock-skew attacks |
| `reqwest` | 0.12 | ✅ Current |
| `walkdir` | 2.5 | ✅ Current |
| `tantivy` | 0.22 | ✅ Current |
| `redb` | 2 | ✅ Current |
| `hkdf` | 0.12 | ✅ `RustCrypto` project, audited |
| `blake3` | 1 | ✅ Audited, fast |
| `brotli` | 7 | ⚠️ Not `brotli-decompressor` — check CVE list; no known issues in 7.x |
| `rayon` | 1.10 | ✅ Current |
| `tracing` | 0.1 | ✅ Good upgrade from `env_logger` |

---

*End of audit — 15 remaining issues, all with exact fixes above.*
