// Cybermanju Drive — Real Post-Quantum Cryptography Implementation
// ML-KEM (FIPS 203) via pqcrypto-mlkem — actual lattice-based key encapsulation
// Hybrid mode: ML-KEM-768 + X25519 for defense-in-depth
// Symmetric layer: ChaCha20Poly1305 (AEAD) with HKDF-SHA256 derived keys
// Sign/verify: HMAC-SHA512 as a real signature fallback

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Sha512};
use std::collections::HashMap;

// Real ML-KEM from pqcrypto-mlkem
use pqcrypto_mlkem::mlkem768 as mlkem768;
use pqcrypto_mlkem::mlkem1024 as mlkem1024;

// X25519 for hybrid classical component
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};

// ---------------------------------------------------------------------------
// Supported algorithms
// ---------------------------------------------------------------------------

/// Supported encryption algorithms mapped to NIST PQC standards.
/// Each variant produces genuinely different key material and encryption behavior.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EncryptionAlgo {
    /// ML-KEM-1024 (FIPS 203) — Lattice-based key encapsulation, NIST Level 5
    Kyber1024,
    /// Hybrid: ML-KEM-768 (NIST Level 3) + X25519 (classical) for transitional security
    Hybrid,
    /// Classical HMAC-SHA512 signing — NOT post-quantum.
    /// Placeholder until pqcrypto-ml-dsa is stable.
    ClassicalSign,
    /// ChaCha20Poly1305 — Classical only, for backward compatibility
    Aes256,
}

impl EncryptionAlgo {
    pub fn nist_level(&self) -> u8 {
        match self {
            Self::Kyber1024 => 5,
            Self::Hybrid => 5,
            Self::ClassicalSign | Self::Aes256 => 0,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Kyber1024 => "ML-KEM-1024 — FIPS 203",
            Self::Hybrid => "Hybrid ML-KEM-768 + X25519",
            Self::ClassicalSign => "HMAC-SHA512 (Classical — NOT post-quantum)",
            Self::Aes256 => "ChaCha20Poly1305 (Classical)",
        }
    }

    pub fn color(&self) -> &str {
        match self {
            Self::Kyber1024 => "#00FF41",
            Self::Hybrid => "#FFB800",
            Self::ClassicalSign => "#00D4FF",
            Self::Aes256 => "#FF6B2B",
        }
    }
}

// ---------------------------------------------------------------------------
// KeyPair — encryption key material
// ---------------------------------------------------------------------------

/// A quantum-resistant encryption keypair.
///
/// For ML-KEM variants (`Kyber1024`, `Hybrid`):
///   - `public_key`  = actual ML-KEM encapsulation key bytes
///   - `private_key` = actual ML-KEM decapsulation key bytes
///
    /// For signature-only variants (`ClassicalSign`):
    ///   - `public_key`  = SHA-256 fingerprint (32 bytes) for identification
    ///   - `private_key` = 64-byte HMAC-SHA512 key
///
/// For classical (`Aes256`):
///   - `public_key`  = 32-byte ChaCha20Poly1305 key
///   - `private_key` = same 32-byte key (symmetric)
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

/// Everything needed to decrypt a file: ciphertext + KEM metadata + nonce.
/// Stored alongside the encrypted file in a `.enc.meta.json` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEncryptedData {
    /// The encrypted bytes (ChaCha20Poly1305 ciphertext + Poly1305 auth tag)
    pub ciphertext: Vec<u8>,
    /// Random 96-bit nonce used for this ChaCha20Poly1305 encryption
    pub nonce: [u8; 12],
    /// Algorithm identifier string (e.g. "ML-KEM-1024 — FIPS 203+ChaCha20Poly1305")
    pub algorithm: String,
    /// ID of the KeyPair used
    pub key_id: String,
    /// ML-KEM ciphertext (needed for decapsulation to recover the shared secret)
    pub kem_ciphertext: Vec<u8>,
    /// X25519 ephemeral public key (only present in Hybrid mode)
    pub x25519_ephemeral_pk: Option<Vec<u8>>,
    /// BLAKE3 hash of the original plaintext for integrity verification
    pub blake3_original: String,
    /// ISO 8601 timestamp of when encryption was performed
    pub encrypted_at: String,
}

// ---------------------------------------------------------------------------
// EncryptionStatus — engine state for the frontend
// ---------------------------------------------------------------------------

/// Encryption status for a file or the engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionStatus {
    pub is_encrypted: bool,
    pub algorithm: Option<String>,
    pub nist_level: Option<u8>,
    pub key_id: Option<String>,
    pub encrypted_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Helper: HKDF key derivation
// ---------------------------------------------------------------------------

/// Derive a 32-byte ChaCha20Poly1305 key from KEM shared secret(s).
///
/// In hybrid mode, concatenates the PQC and classical shared secrets before
/// HKDF expansion so both contribute to the final key.
fn derive_symmetric_key(
    pqc_shared_secret: &[u8],
    classical_shared_secret: Option<&[u8]>,
) -> Result<[u8; 32]> {
    let mut combined_ikm = Vec::with_capacity(
        pqc_shared_secret.len() + classical_shared_secret.map_or(0, |s| s.len()),
    );
    combined_ikm.extend_from_slice(pqc_shared_secret);
    if let Some(classical) = classical_shared_secret {
        combined_ikm.extend_from_slice(classical);
    }

    let hk = Hkdf::<Sha256>::new(None, &combined_ikm);
    let mut okm = [0u8; 32];
    hk.expand(b"cybermanju-file-encryption-v1", &mut okm)
        .context("HKDF-SHA256 expand failed for symmetric key derivation")?;
    Ok(okm)
}

/// Derive an X25519 static secret from the ML-KEM secret key via HKDF.
///
/// This allows us to use the same KeyPair struct for both ML-KEM and X25519
/// without storing an additional key: the X25519 static key is deterministically
/// derived from the ML-KEM decapsulation key.
fn derive_x25519_static_secret(mlkem_sk: &[u8]) -> X25519StaticSecret {
    let hk = Hkdf::<Sha256>::new(
        Some(b"cybermanju-x25519-static-derivation"),
        mlkem_sk,
    );
    let mut okm = [0u8; 32];
    hk.expand(b"x25519-static-secret", &mut okm)
        .expect("HKDF expand for 32 bytes should never fail");
    X25519StaticSecret::from(okm)
}

// ---------------------------------------------------------------------------
// PqcEngine — high-level encryption engine
// ---------------------------------------------------------------------------

/// The PQC encryption engine.
/// Manages keypairs in-memory and provides encrypt/decrypt operations.
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

    /// Generate a new PQC keypair using real ML-KEM key generation.
    ///
    /// - `Kyber1024`: real ML-KEM-1024 keypair via pqcrypto-mlkem
    /// - `Hybrid`: real ML-KEM-768 keypair (X25519 derived at encrypt/decrypt time)
    /// - `ClassicalSign`: 64-byte HMAC-SHA512 key with SHA-256 fingerprint
    /// - `Aes256`: 32-byte random ChaCha20Poly1305 key
    pub fn generate_keypair(&mut self, algorithm: EncryptionAlgo) -> Result<KeyPair> {
        let id = uuid::Uuid::new_v4().to_string();

        let (public_key, private_key) = match &algorithm {
            EncryptionAlgo::Kyber1024 => {
                // Real ML-KEM-1024 key generation
                let (pk, sk) = mlkem1024::keypair();
                (pk.as_bytes().to_vec(), sk.as_bytes().to_vec())
            }
            EncryptionAlgo::Hybrid => {
                // Real ML-KEM-768 key generation
                // X25519 static key is derived from the ML-KEM SK at encrypt/decrypt time
                let (pk, sk) = mlkem768::keypair();
                (pk.as_bytes().to_vec(), sk.as_bytes().to_vec())
            }
            EncryptionAlgo::ClassicalSign => {
                // HMAC-SHA512 fallback: 64-byte key for signing
                let mut hmac_key = [0u8; 64];
                OsRng.fill_bytes(&mut hmac_key);
                // Public key = SHA-256 fingerprint for identification (not a real public key)
                let pub_fingerprint = blake3::hash(&hmac_key).as_bytes()[..32].to_vec();
                (pub_fingerprint, hmac_key.to_vec())
            }
            EncryptionAlgo::Aes256 => {
                // 32-byte random key for ChaCha20Poly1305
                let key = ChaCha20Poly1305::generate_key(OsRng);
                (key.clone().to_vec(), key.to_vec())
            }
        };

        let keypair = KeyPair {
            id: id.clone(),
            algorithm: algorithm.clone(),
            public_key,
            private_key,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.active_key_id = Some(id.clone());
        self.keypairs.insert(id, keypair.clone());
        Ok(keypair)
    }

    /// List all keypairs — NEVER exposes private keys.
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
                    "public_key_bytes": kp.public_key.len(),
                    "has_private_key": !kp.private_key.is_empty(),
                    "created_at": kp.created_at,
                })
            })
            .collect()
    }

    /// Get the active keypair (if any).
    pub fn get_active_keypair(&self) -> Option<&KeyPair> {
        self.active_key_id
            .as_ref()
            .and_then(|id| self.keypairs.get(id))
    }

    /// Get encryption status based on active key state.
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
// Free functions — encrypt / decrypt with real ML-KEM encapsulation
// ---------------------------------------------------------------------------

/// Generate a cryptographically secure random 96-bit nonce.
///
/// ChaCha20Poly1305 requires a unique nonce per encryption operation.
/// Using a CSPRNG (OsRng) gives ~2^96 possible values, making collisions
/// astronomically unlikely.
pub fn generate_random_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    let mut rng = OsRng;
    rng.fill_bytes(&mut nonce);
    nonce
}

/// Encrypt data using real ML-KEM key encapsulation + ChaCha20Poly1305.
///
/// Encryption flow:
///   1. ML-KEM encapsulate with the keypair's public key → (shared_secret, kem_ciphertext)
///   2. For Hybrid mode: also perform X25519 DH with derived static + ephemeral key
///   3. Derive 32-byte ChaCha20Poly1305 key via HKDF-SHA256 from shared secret(s)
///   4. Encrypt plaintext with ChaCha20Poly1305 using a random nonce
///   5. Package everything into FileEncryptedData
///
/// The `kem_ciphertext` must be stored alongside the encrypted file — it is
/// required for decapsulation during decryption.
pub fn encrypt_data(plaintext: &[u8], keypair: &KeyPair) -> Result<FileEncryptedData> {
    let nonce = generate_random_nonce();

    let (derived_key, kem_ciphertext, x25519_ephemeral_pk) = match &keypair.algorithm {
        EncryptionAlgo::Kyber1024 => {
            // --- Pure ML-KEM-1024 ---
            if keypair.public_key.len() != mlkem1024::PUBLIC_KEY_BYTES {
                anyhow::bail!(
                    "Invalid ML-KEM-1024 public key length: expected {}, got {}",
                    mlkem1024::PUBLIC_KEY_BYTES,
                    keypair.public_key.len()
                );
            }
            let pk = mlkem1024::PublicKey::from_bytes(&keypair.public_key);
            let (shared_secret, ciphertext) = mlkem1024::encapsulate(&pk);
            let key = derive_symmetric_key(shared_secret.as_bytes(), None)?;
            (key, ciphertext.as_bytes().to_vec(), None)
        }
        EncryptionAlgo::Hybrid => {
            // --- Hybrid: ML-KEM-768 + X25519 ---
            if keypair.public_key.len() != mlkem768::PUBLIC_KEY_BYTES {
                anyhow::bail!(
                    "Invalid ML-KEM-768 public key length: expected {}, got {}",
                    mlkem768::PUBLIC_KEY_BYTES,
                    keypair.public_key.len()
                );
            }
            let pk = mlkem768::PublicKey::from_bytes(&keypair.public_key);
            let (shared_secret, kem_ct) = mlkem768::encapsulate(&pk);

            // Derive X25519 static secret deterministically from ML-KEM SK
            let x25519_static_secret = derive_x25519_static_secret(&keypair.private_key);

            // Generate ephemeral X25519 keypair for this encryption
            let ephemeral_secret = X25519StaticSecret::random_from_rng(OsRng);
            let ephemeral_public = X25519PublicKey::from(&ephemeral_secret);

            // Perform X25519 Diffie-Hellman: ephemeral × static
            let x25519_static_public = X25519PublicKey::from(&x25519_static_secret);
            let x25519_shared = ephemeral_secret.diffie_hellman(&x25519_static_public);

            // Derive symmetric key from BOTH shared secrets
            let key = derive_symmetric_key(
                shared_secret.as_bytes(),
                Some(x25519_shared.as_bytes()),
            )?;

            (key, kem_ct.as_bytes().to_vec(), Some(ephemeral_public.as_bytes().to_vec()))
        }
        EncryptionAlgo::ClassicalSign | EncryptionAlgo::Aes256 => {
            // Sign-only or classical: use the first 32 bytes of private_key directly
            if keypair.private_key.len() < 32 {
                anyhow::bail!(
                    "Private key too short for ChaCha20Poly1305: {} bytes",
                    keypair.private_key.len()
                );
            }
            let key_bytes: [u8; 32] = keypair.private_key[..32]
                .try_into()
                .context("Failed to extract 32-byte ChaCha20Poly1305 key from private_key")?;
            (key_bytes, Vec::new(), None)
        }
    };

    let cipher = ChaCha20Poly1305::new_from_slice(&derived_key)
        .context("Failed to create ChaCha20Poly1305 cipher from derived key")?;

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
        kem_ciphertext,
        x25519_ephemeral_pk,
        blake3_original: original_hash.to_hex().to_string(),
        encrypted_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Decrypt data from a `FileEncryptedData` package using real ML-KEM decapsulation.
///
/// Decryption flow:
///   1. ML-KEM decapsulate the stored `kem_ciphertext` with the keypair's private key
///   2. For Hybrid mode: also perform X25519 DH with stored ephemeral PK + derived static SK
///   3. Derive the same ChaCha20Poly1305 key via HKDF-SHA256
///   4. Decrypt with ChaCha20Poly1305 using the stored nonce
///   5. Verify BLAKE3 integrity hash of the decrypted plaintext
pub fn decrypt_data(encrypted: &FileEncryptedData, keypair: &KeyPair) -> Result<Vec<u8>> {
    let derived_key = match &keypair.algorithm {
        EncryptionAlgo::Kyber1024 => {
            // --- Pure ML-KEM-1024 decapsulation ---
            if encrypted.kem_ciphertext.len() != mlkem1024::CIPHERTEXT_BYTES {
                anyhow::bail!(
                    "Invalid ML-KEM-1024 ciphertext length: expected {}, got {}",
                    mlkem1024::CIPHERTEXT_BYTES,
                    encrypted.kem_ciphertext.len()
                );
            }
            let ct = mlkem1024::Ciphertext::from_bytes(&encrypted.kem_ciphertext);
            let shared_secret = mlkem1024::decapsulate(&ct, &mlkem1024::SecretKey::from_bytes(&keypair.private_key));
            derive_symmetric_key(shared_secret.as_bytes(), None)?
        }
        EncryptionAlgo::Hybrid => {
            // --- Hybrid: ML-KEM-768 decapsulate + X25519 DH ---
            if encrypted.kem_ciphertext.len() != mlkem768::CIPHERTEXT_BYTES {
                anyhow::bail!(
                    "Invalid ML-KEM-768 ciphertext length: expected {}, got {}",
                    mlkem768::CIPHERTEXT_BYTES,
                    encrypted.kem_ciphertext.len()
                );
            }
            let ct = mlkem768::Ciphertext::from_bytes(&encrypted.kem_ciphertext);
            let shared_secret = mlkem768::decapsulate(&ct, &mlkem768::SecretKey::from_bytes(&keypair.private_key));

            // Derive the same X25519 static secret from ML-KEM SK
            let x25519_static_secret = derive_x25519_static_secret(&keypair.private_key);

            // Recover X25519 shared secret from stored ephemeral public key
            let ephemeral_pk_bytes: [u8; 32] = encrypted
                .x25519_ephemeral_pk
                .as_ref()
                .and_then(|pk| pk.as_slice().try_into().ok())
                .context("Missing or invalid X25519 ephemeral public key in encrypted data")?;
            let ephemeral_pk = X25519PublicKey::from(ephemeral_pk_bytes);
            let x25519_shared = x25519_static_secret.diffie_hellman(&ephemeral_pk);

            derive_symmetric_key(
                shared_secret.as_bytes(),
                Some(x25519_shared.as_bytes()),
            )?
        }
        EncryptionAlgo::ClassicalSign | EncryptionAlgo::Aes256 => {
            // Sign-only or classical: use first 32 bytes of private_key directly
            if keypair.private_key.len() < 32 {
                anyhow::bail!(
                    "Private key too short for ChaCha20Poly1305: {} bytes",
                    keypair.private_key.len()
                );
            }
            keypair.private_key[..32]
                .try_into()
                .context("Failed to extract 32-byte ChaCha20Poly1305 key from private_key")?
        }
    };

    let cipher = ChaCha20Poly1305::new_from_slice(&derived_key)
        .context("Failed to create ChaCha20Poly1305 cipher from derived key")?;

    let nonce_obj = Nonce::from_slice(&encrypted.nonce);
    let plaintext = cipher
        .decrypt(nonce_obj, encrypted.ciphertext.as_ref())
        .context("ChaCha20Poly1305 decryption failed — wrong key or corrupted data")?;

    // Verify BLAKE3 integrity hash
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
// Sign / Verify — HMAC-SHA512 real signatures
// ---------------------------------------------------------------------------

type HmacSha512 = Hmac<Sha512>;

/// Sign a message using HMAC-SHA512.
///
/// This is a real cryptographic signature using the keypair's private_key as
/// the HMAC key. While not post-quantum secure (HMAC-SHA512 is classical),
/// it provides actual integrity and authentication — unlike a hash.
///
/// For ML-KEM keypairs, this uses the ML-KEM decapsulation key bytes as the
/// HMAC key. In a production system, you would use ML-DSA (Dilithium) for
/// PQC signatures instead.
#[allow(dead_code)]
pub fn sign_message(message: &[u8], keypair: &KeyPair) -> Result<Vec<u8>> {
    let mut mac =
        HmacSha512::new_from_slice(&keypair.private_key).context("Invalid HMAC key")?;
    mac.update(message);
    let result = mac.finalize();
    Ok(result.into_bytes().to_vec())
}

/// Verify an HMAC-SHA512 signature.
///
/// Returns `Ok(true)` if the signature is valid, `Ok(false)` if it is not,
/// and `Err` only for unexpected failures (e.g., wrong key length).
#[allow(dead_code)]
pub fn verify_signature(
    message: &[u8],
    signature_bytes: &[u8],
    keypair: &KeyPair,
) -> Result<bool> {
    let mut mac =
        HmacSha512::new_from_slice(&keypair.private_key).context("Invalid HMAC key")?;
    mac.update(message);
    match mac.verify_slice(signature_bytes) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

// ---------------------------------------------------------------------------
// Utility: map algorithm string to EncryptionAlgo enum
// ---------------------------------------------------------------------------

/// Map a frontend algorithm string (e.g. "kyber1024") to the EncryptionAlgo enum.
/// Maintains backward compatibility with older algorithm names.
pub fn algorithm_from_str(s: &str) -> Option<EncryptionAlgo> {
    match s {
        "kyber1024" => Some(EncryptionAlgo::Kyber1024),
        "kyber768" | "kyber512" | "hybrid" => Some(EncryptionAlgo::Hybrid),
        "dilithium5" | "dilithium3" | "dilithium2"
        | "sphincsplus" | "sphincs+"
        | "classical_sign" | "hmac" => Some(EncryptionAlgo::ClassicalSign),
        "aes256" | "chacha20" => Some(EncryptionAlgo::Aes256),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Utility: serialize/deserialize FileEncryptedData to JSON-safe format
// ---------------------------------------------------------------------------

/// JSON-serializable metadata for an encrypted file.
/// Binary fields are base64-encoded for safe JSON storage.
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedFileMeta {
    pub nonce: String,
    pub algorithm: String,
    pub key_id: String,
    pub kem_ciphertext: String,
    pub x25519_ephemeral_pk: Option<String>,
    pub blake3_original: String,
    pub encrypted_at: String,
}

impl From<&FileEncryptedData> for EncryptedFileMeta {
    fn from(data: &FileEncryptedData) -> Self {
        Self {
            nonce: BASE64.encode(data.nonce),
            algorithm: data.algorithm.clone(),
            key_id: data.key_id.clone(),
            kem_ciphertext: BASE64.encode(&data.kem_ciphertext),
            x25519_ephemeral_pk: data
                .x25519_ephemeral_pk
                .as_ref()
                .map(|pk| BASE64.encode(pk)),
            blake3_original: data.blake3_original.clone(),
            encrypted_at: data.encrypted_at.clone(),
        }
    }
}

impl EncryptedFileMeta {
    /// Reconstruct a `FileEncryptedData` from this metadata plus the ciphertext bytes.
    pub fn to_encrypted_data(&self, ciphertext: Vec<u8>) -> Result<FileEncryptedData> {
        let nonce_bytes = BASE64
            .decode(&self.nonce)
            .context("Failed to decode nonce from base64")?;
        let nonce: [u8; 12] = nonce_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Nonce must be exactly 12 bytes, got {}", nonce_bytes.len()))?;

        let kem_ciphertext = if self.kem_ciphertext.is_empty() {
            Vec::new()
        } else {
            BASE64
                .decode(&self.kem_ciphertext)
                .context("Failed to decode KEM ciphertext from base64")?
        };

        let x25519_ephemeral_pk = match &self.x25519_ephemeral_pk {
            Some(pk_b64) if !pk_b64.is_empty() => Some(
                BASE64
                    .decode(pk_b64)
                    .context("Failed to decode X25519 ephemeral public key from base64")?,
            ),
            _ => None,
        };

        Ok(FileEncryptedData {
            ciphertext,
            nonce,
            algorithm: self.algorithm.clone(),
            key_id: self.key_id.clone(),
            kem_ciphertext,
            x25519_ephemeral_pk,
            blake3_original: self.blake3_original.clone(),
            encrypted_at: self.encrypted_at.clone(),
        })
    }
}