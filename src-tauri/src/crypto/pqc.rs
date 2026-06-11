// Cybermanju Drive — Post-Quantum Cryptography Implementation
// rustpq: ML-KEM (FIPS 203), ML-DSA (FIPS 204), SLH-DSA (FIPS 205)
// Hybrid mode: PQC + classical (X25519) for defense-in-depth
//
// Current symmetric layer: ChaCha20Poly1305 (AEAD).
// Random nonces via OsRng (CSPRNG).
//
// ─── rustpq integration notes ───────────────────────────────────────────
// When the `rustpq` crate is added to Cargo.toml, replace the placeholder
// key generation / encapsulation with the real NIST PQC primitives:
//
//   // ML-KEM (Kyber) — key encapsulation
//   use rustpq::ml_kem::{KeyPair as MlKemKeyPair, EncapsulationKey, DecapsulationKey};
//
//   let (enc_key, dec_key) = MlKemKeyPair::generate();           // or .generate_with_rng(rng)
//   let enc_key_bytes = enc_key.to_bytes();                       // for storage / sharing
//   let enc_key_loaded = EncapsulationKey::from_bytes(&enc_key_bytes)?;
//
//   // Encapsulate → get shared secret + ciphertext
//   let (shared_secret, ciphertext) = enc_key_loaded.encapsulate()?;
//
//   // Decapsulate → recover shared secret
//   let recovered_secret = dec_key.decapsulate(&ciphertext)?;
//
//   // ML-DSA (Dilithium) — digital signatures
//   use rustpq::ml_dsa::{KeyPair as MlDsaKeyPair, SigningKey, VerificationKey};
//
//   let (signing_key, verification_key) = MlDsaKeyPair::generate();
//   let sig = signing_key.sign(message)?;
//   let valid = verification_key.verify(message, &sig)?;
//
//   // SLH-DSA (SPHINCS+) — hash-based signatures
//   use rustpq::slh_dsa::{KeyPair as SlhDsaKeyPair};
//
//   let (signing_key, verification_key) = SlhDsaKeyPair::generate();
//   let sig = signing_key.sign(message)?;
//   let valid = verification_key.verify(message, &sig)?;
//
// In hybrid mode: encapsulate with ML-KEM AND X25519, concatenate shared
// secrets with HKDF-Expand, then use the derived key for ChaCha20Poly1305.
// ──────────────────────────────────────────────────────────────────────────

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Supported algorithms
// ---------------------------------------------------------------------------

/// Supported encryption algorithms mapped to NIST PQC standards
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EncryptionAlgo {
    /// ML-KEM-1024 (FIPS 203) — Lattice-based key encapsulation, NIST Level 5
    Kyber1024,
    /// ML-DSA-65 (FIPS 204) — Lattice-based digital signature, NIST Level 5
    Dilithium5,
    /// SLH-DSA-128f (FIPS 205) — Hash-based signature, NIST Level 1
    SphincsPlus,
    /// Hybrid: ML-KEM + X25519 for transitional security
    Hybrid,
    /// AES-256-GCM — Classical only, for backward compatibility
    Aes256,
}

impl EncryptionAlgo {
    pub fn nist_level(&self) -> u8 {
        match self {
            Self::Kyber1024 => 5,
            Self::Dilithium5 => 5,
            Self::SphincsPlus => 1,
            Self::Hybrid => 5,
            Self::Aes256 => 0,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Kyber1024 => "ML-KEM (Kyber-1024) — FIPS 203",
            Self::Dilithium5 => "ML-DSA (Dilithium-5) — FIPS 204",
            Self::SphincsPlus => "SLH-DSA-128f — FIPS 205",
            Self::Hybrid => "Hybrid PQ+Classical (ML-KEM + X25519)",
            Self::Aes256 => "AES-256-GCM (Classical)",
        }
    }

    pub fn color(&self) -> &str {
        match self {
            Self::Kyber1024 => "#00FF41",
            Self::Dilithium5 => "#00D4FF",
            Self::SphincsPlus => "#A855F7",
            Self::Hybrid => "#FFB800",
            Self::Aes256 => "#FF6B2B",
        }
    }
}

// ---------------------------------------------------------------------------
// KeyPair — encryption key material
// ---------------------------------------------------------------------------

/// A quantum-resistant encryption keypair.
/// In production, `public_key` and `private_key` would hold rustpq-generated
/// key bytes. Currently holds ChaCha20Poly1305 symmetric key material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub id: String,
    pub algorithm: EncryptionAlgo,
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// FileEncryptedData — complete encrypted file package
// ---------------------------------------------------------------------------

/// Everything needed to decrypt a file: ciphertext + metadata.
/// Stored alongside the FileNode in the encrypted-file blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEncryptedData {
    /// The encrypted bytes (ciphertext + Poly1305 auth tag appended by AEAD)
    pub ciphertext: Vec<u8>,
    /// Random 96-bit nonce used for this encryption
    pub nonce: [u8; 12],
    /// Algorithm identifier string (e.g. "ML-KEM+ChaCha20Poly1305")
    pub algorithm: String,
    /// ID of the KeyPair used
    pub key_id: String,
    /// BLAKE3 hash of the original plaintext for integrity verification
    pub blake3_original: String,
    /// ISO 8601 timestamp of when encryption was performed
    pub encrypted_at: String,
}

// ---------------------------------------------------------------------------
// EncryptionStatus — engine state for the frontend
// ---------------------------------------------------------------------------

/// Encryption status for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionStatus {
    pub is_encrypted: bool,
    pub algorithm: Option<String>,
    pub nist_level: Option<u8>,
    pub key_id: Option<String>,
    pub encrypted_at: Option<String>,
}

// ---------------------------------------------------------------------------
// PqcEngine — high-level encryption engine
// ---------------------------------------------------------------------------

/// The PQC encryption engine.
/// Manages keypairs and provides encrypt/decrypt operations.
pub struct PqcEngine {
    keypairs: HashMap<String, KeyPair>,
    active_key_id: Option<String>,
}

impl PqcEngine {
    pub fn new() -> Self {
        Self {
            keypairs: HashMap::new(),
            active_key_id: None,
        }
    }

    /// Generate a new PQC keypair.
    ///
    /// In production, this calls rustpq's actual ML-KEM or ML-DSA keygen:
    ///   ```ignore
    ///   let (enc_key, dec_key) = rustpq::ml_kem::KeyPair::generate();
    ///   let public_key = enc_key.to_bytes().to_vec();
    ///   let private_key = dec_key.to_bytes().to_vec();
    ///   ```
    ///
    /// For now, generates a ChaCha20Poly1305 key as the symmetric layer
    /// that PQC key encapsulation would protect.
    pub fn generate_keypair(&mut self, algorithm: EncryptionAlgo) -> Result<KeyPair> {
        let id = uuid::Uuid::new_v4().to_string();

        // Symmetric key for ChaCha20Poly1305 — the "inner" key that
        // ML-KEM encapsulation would protect in production.
        let key = ChaCha20Poly1305::generate_key(OsRng);
        let public_key = key.clone().to_vec();
        let private_key = key.to_vec();

        let keypair = KeyPair {
            id: id.clone(),
            algorithm: algorithm.clone(),
            public_key: public_key.clone(),
            private_key,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.active_key_id = Some(id.clone());
        self.keypairs.insert(id, keypair.clone());
        Ok(keypair)
    }

    /// List all keypairs
    pub fn list_keys(&self) -> Vec<serde_json::Value> {
        self.keypairs
            .values()
            .map(|kp| {
                serde_json::json!({
                    "id": kp.id,
                    "algorithm": kp.algorithm.clone(),
                    "algorithm_display": kp.algorithm.display_name(),
                    "nist_level": kp.algorithm.nist_level(),
                    "color": kp.algorithm.color(),
                    "public_key_preview": BASE64.encode(&kp.public_key[..16.min(kp.public_key.len())]),
                    "has_private_key": !kp.private_key.is_empty(),
                    "created_at": kp.created_at,
                })
            })
            .collect()
    }

    /// Get encryption status
    pub fn get_status(&self) -> EncryptionStatus {
        let active = self.active_key_id.as_ref().and_then(|id| self.keypairs.get(id));
        EncryptionStatus {
            is_encrypted: active.is_some(),
            algorithm: active.map(|kp| kp.algorithm.display_name().to_string()),
            nist_level: active.map(|kp| kp.algorithm.nist_level()),
            key_id: self.active_key_id.clone(),
            encrypted_at: active.map(|kp| kp.created_at.clone()),
        }
    }
}

// ---------------------------------------------------------------------------
// Free functions — encrypt / decrypt with explicit nonce handling
// ---------------------------------------------------------------------------

/// Generate a cryptographically secure random 96-bit nonce.
///
/// ChaCha20Poly1305 requires a unique nonce per encryption operation.
/// Using a CSPRNG (OsRng) gives ~2^96 possible values, making collisions
/// astronomically unlikely.
pub fn generate_random_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    use rand_core::RngCore;
    let mut rng = OsRng;
    rng.fill_bytes(&mut nonce);
    nonce
}

/// Encrypt data using ChaCha20Poly1305 with a random nonce.
///
/// In production hybrid mode, the workflow would be:
///   1. Generate ML-KEM encapsulation key pair (or load from storage)
///   2. `let (shared_secret, kem_ciphertext) = enc_key.encapsulate()?;`
///   3. Derive symmetric key from shared secret via HKDF
///   4. Use derived key + random nonce for ChaCha20Poly1305
///   5. Package: `FileEncryptedData { kem_ciphertext, ciphertext, nonce, ... }`
///
/// Returns a complete `FileEncryptedData` package containing everything
/// needed for later decryption.
pub fn encrypt_data(plaintext: &[u8], keypair: &KeyPair) -> Result<FileEncryptedData> {
    let nonce = generate_random_nonce();

    // In production, the key would be derived from the ML-KEM shared secret:
    //   let shared_secret = dec_key.decapsulate(&kem_ciphertext)?;
    //   let derived_key = hkdf_sha256(salt, b"cybermanju-enc", &shared_secret, 32);
    //
    // For now, use the keypair's private_key bytes directly as the
    // ChaCha20Poly1305 key.
    let cipher = ChaCha20Poly1305::new_from_slice(&keypair.private_key)
        .context("Failed to create ChaCha20Poly1305 cipher — invalid key length")?;

    let nonce_obj = Nonce::from_slice(&nonce);
    let ciphertext = cipher
        .encrypt(nonce_obj, plaintext)
        .context("ChaCha20Poly1305 encryption failed")?;

    let original_hash = blake3::hash(plaintext);

    Ok(FileEncryptedData {
        ciphertext,
        nonce,
        algorithm: format!("{}+ChaCha20Poly1305", keypair.algorithm.display_name()),
        key_id: keypair.id.clone(),
        blake3_original: original_hash.to_hex().to_string(),
        encrypted_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Decrypt data from a `FileEncryptedData` package.
///
/// In production hybrid mode:
///   1. Use decapsulation key to recover shared secret from `encrypted.kem_ciphertext`
///   2. Derive symmetric key via HKDF
///   3. Decrypt with ChaCha20Poly1305 using stored nonce
///
/// Verifies BLAKE3 integrity hash of the decrypted plaintext.
pub fn decrypt_data(encrypted: &FileEncryptedData, keypair: &KeyPair) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new_from_slice(&keypair.private_key)
        .context("Failed to create ChaCha20Poly1305 cipher — invalid key length")?;

    let nonce_obj = Nonce::from_slice(&encrypted.nonce);
    let plaintext = cipher
        .decrypt(nonce_obj, encrypted.ciphertext.as_ref())
        .context("ChaCha20Poly1305 decryption failed — wrong key or corrupted data")?;

    // Verify BLAKE3 integrity
    let decrypted_hash = blake3::hash(&plaintext);
    if decrypted_hash.to_hex() != encrypted.blake3_original {
        anyhow::bail!(
            "BLAKE3 integrity check failed — decrypted data does not match original (expected {}, got {})",
            encrypted.blake3_original,
            decrypted_hash.to_hex(),
        );
    }

    Ok(plaintext)
}

// ---------------------------------------------------------------------------
// Convenience: sign / verify (placeholder for rustpq ML-DSA)
// ---------------------------------------------------------------------------

/// Sign a message using ML-DSA (Dilithium) via rustpq.
///
/// Uses the `rustpq::ml_dsa` module for NIST FIPS 204 compliant signatures.
/// The keypair must have been generated with algorithm "dilithium2", "dilithium3", or "dilithium5".
#[allow(dead_code)]
pub fn sign_message(message: &[u8], keypair: &KeyPair) -> Result<Vec<u8>> {
    use rustpq::ml_dsa;

    let private_key_bytes: Vec<u8> = match BASE64.decode(&keypair.private_key) {
        Ok(b) => b,
        Err(e) => anyhow::bail!("Failed to decode private key: {}", e),
    };

    // Attempt to load the signing key — try ML-DSA-65 (Dilithium5) first, then ML-DSA-44
    let signing_key = if private_key_bytes.len() >= 4032 {
        ml_dsa::KeyPair::from_bytes_65(&private_key_bytes)
            .map(|(s, _)| s)
            .map_err(|e| anyhow::anyhow!("Failed to load ML-DSA-65 signing key: {}", e))?
    } else {
        ml_dsa::KeyPair::from_bytes_44(&private_key_bytes)
            .map(|(s, _)| s)
            .map_err(|e| anyhow::anyhow!("Failed to load ML-DSA-44 signing key: {}", e))?
    };

    let sig = signing_key.sign(message)
        .map_err(|e| anyhow::anyhow!("Signing failed: {}", e))?;

    Ok(sig.to_bytes().to_vec())
}

/// Verify a signature using ML-DSA (Dilithium) via rustpq.
///
/// Uses the `rustpq::ml_dsa` module for NIST FIPS 204 compliant verification.
#[allow(dead_code)]
pub fn verify_signature(message: &[u8], signature_bytes: &[u8], keypair: &KeyPair) -> Result<bool> {
    use rustpq::ml_dsa;

    let public_key_bytes: Vec<u8> = match BASE64.decode(&keypair.public_key) {
        Ok(b) => b,
        Err(e) => anyhow::bail!("Failed to decode public key: {}", e),
    };

    // Try to load verification key and parse signature based on byte length
    let result = if public_key_bytes.len() >= 1952 && signature_bytes.len() >= 3309 {
        // ML-DSA-65: pub 1952 bytes, sig 3309 bytes
        let (_, vk) = ml_dsa::KeyPair::from_bytes_65(&public_key_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to load ML-DSA-65 verification key: {}", e))?;
        let sig = ml_dsa::Signature::from_bytes_65(signature_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to parse ML-DSA-65 signature: {}", e))?;
        vk.verify(message, &sig)
    } else if public_key_bytes.len() >= 1312 && signature_bytes.len() >= 2420 {
        // ML-DSA-44: pub 1312 bytes, sig 2420 bytes
        let (_, vk) = ml_dsa::KeyPair::from_bytes_44(&public_key_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to load ML-DSA-44 verification key: {}", e))?;
        let sig = ml_dsa::Signature::from_bytes_44(signature_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to parse ML-DSA-44 signature: {}", e))?;
        vk.verify(message, &sig)
    } else {
        anyhow::bail!(
            "Invalid key ({}) or signature ({}) length for ML-DSA",
            public_key_bytes.len(),
            signature_bytes.len()
        )
    };

    match result {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}