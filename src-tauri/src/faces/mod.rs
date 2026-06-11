// Cybermanju Drive — Face Detection & Clustering Module
// Pipeline: detect → embed → cluster (DBSCAN)
//
// In production this loads ONNX models via ort:
//   - RetinaFace / SCRFD for face detection
//   - ArcFace / MobileFaceNet for 512-d (or 128-d) embedding extraction
//
// For now, a deterministic hash-based fake-embedding pipeline is used so
// that the clustering logic actually works end-to-end.

use anyhow::Result;
use crate::db::schema::FileNode;

/// Number of dimensions in each face embedding vector.
/// ArcFace produces 512-d; we use 128-d for the placeholder.
const EMBEDDING_DIM: usize = 128;

// ---------------------------------------------------------------------------
// Deterministic fake-embedding generator
// ---------------------------------------------------------------------------

/// Produce a deterministic 128-dim embedding for a given seed string.
/// Uses BLAKE3 to hash the seed, then expands into f32 values in [-1, 1].
/// Same seed always → same embedding, so clustering is reproducible.
fn seed_to_embedding(seed: &str) -> Vec<f32> {
    let hash = blake3::hash(seed.as_bytes());
    let hash_bytes = hash.as_bytes(); // 32 bytes

    let mut embedding = Vec::with_capacity(EMBEDDING_DIM);
    for i in 0..EMBEDDING_DIM {
        // Cycle through hash bytes, mixing in the index for uniqueness
        let byte_idx = i % hash_bytes.len();
        let byte_val = hash_bytes[byte_idx] as f64;
        // Mix position to break patterns: use (byte + i*7) % 256
        let mixed = ((hash_bytes[byte_idx] as usize).wrapping_add(i * 7)) % 256;
        // Map 0..255 → -1.0..1.0, centred around 0
        let val = (mixed as f32) / 127.5 - 1.0;
        embedding.push(val);
    }

    // L2-normalise so cosine distance is meaningful
    let norm: f32 = embedding.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in embedding.iter_mut() {
            *v /= norm;
        }
    }

    embedding
}

// ---------------------------------------------------------------------------
// Public API — called by commands::faces
// ---------------------------------------------------------------------------

/// Detect faces in a file and return one 128-dim embedding per face.
///
/// In production: reads the image bytes from disk via `file_node` metadata,
/// preprocesses to 112×112 RGB, runs RetinaFace ONNX model for bounding
/// boxes, then ArcFace ONNX model on each crop to produce embeddings.
///
/// Current implementation: generates a deterministic fake embedding from the
/// file's blake3 hash so that clustering produces stable, reproducible groups.
pub fn detect_faces_in_file(file_node: &FileNode) -> Result<Vec<Vec<f32>>> {
    // Use the file's BLAKE3 hash as the deterministic seed.
    // If the file doesn't have a hash yet, fall back to its ID.
    let seed = file_node
        .hash_blake3
        .as_deref()
        .unwrap_or(&file_node.id);

    // Simulate between 1-3 detected faces depending on file properties.
    // The number of faces is deterministic per file.
    let face_count = if file_node.file_type == "folder" {
        0
    } else {
        // Derive face count from seed hash: 1, 2, or 3
        let h = blake3::hash(seed.as_bytes());
        let first_byte = h.as_bytes()[0];
        match first_byte % 3 {
            0 => 1,
            1 => 2,
            _ => 3,
        }
    };

    let mut embeddings = Vec::with_capacity(face_count);
    for i in 0..face_count {
        // Each face in the same file gets a slightly different embedding
        let face_seed = format!("{}:face:{}", seed, i);
        embeddings.push(seed_to_embedding(&face_seed));
    }

    Ok(embeddings)
}

/// Compute cosine distance between two embedding vectors.
///
/// Formula: `1.0 - dot(a, b) / (norm(a) * norm(b))`
///
/// Returns 0.0 for identical vectors, 2.0 for maximally dissimilar.
/// Handles the zero-norm edge case by returning 1.0 (max distance).
pub fn embedding_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 1.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 1.0;
    }

    let cosine_similarity = dot / (norm_a * norm_b);
    // Clamp to [-1, 1] to avoid floating-point drift
    let clamped = cosine_similarity.max(-1.0).min(1.0);
    1.0 - clamped
}

/// DBSCAN-like clustering of face embeddings.
///
/// `embeddings` — pairs of (file_id, embedding_vector)
/// `threshold`  — maximum cosine distance for two embeddings to be
///                considered neighbours (typical: 0.5–0.7)
///
/// Returns a list of (group_name, vec![file_id, ...]) where each group
/// contains file IDs whose embeddings are mutually within threshold.
///
/// Algorithm (simplified DBSCAN):
///   1. Compute full pairwise distance matrix.
///   2. Build an undirected adjacency graph: edge if dist ≤ threshold.
///   3. Extract connected components — each component is a cluster.
pub fn cluster_embeddings(
    embeddings: &[(String, Vec<f32>)],
    threshold: f32,
) -> Vec<(String, Vec<String>)> {
    if embeddings.is_empty() {
        return Vec::new();
    }

    let n = embeddings.len();

    // Union-Find (disjoint set) for connected-components clustering
    let mut parent: Vec<usize> = (0..n).collect();

    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]]; // path compression
            x = parent[x];
        }
        x
    }

    fn union(parent: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[ra] = rb;
        }
    }

    // Pairwise distance — connect embeddings within threshold
    for i in 0..n {
        for j in (i + 1)..n {
            let dist = embedding_distance(&embeddings[i].1, &embeddings[j].1);
            if dist <= threshold {
                union(&mut parent, i, j);
            }
        }
    }

    // Collect clusters by root
    let mut clusters: std::collections::HashMap<usize, Vec<String>> =
        std::collections::HashMap::new();
    for i in 0..n {
        let root = find(&mut parent, i);
        clusters
            .entry(root)
            .or_default()
            .push(embeddings[i].0.clone());
    }

    // Convert to output format, skipping singletons (noise in DBSCAN terms)
    let mut result: Vec<(String, Vec<String>)> = clusters
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1) // only real clusters
        .enumerate()
        .map(|(idx, (_, ids))| {
            (
                format!("Person {}", idx + 1),
                ids,
            )
        })
        .collect();

    // Sort by cluster size descending for nicer UX
    result.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_distance_identical() {
        let v = seed_to_embedding("test");
        let dist = embedding_distance(&v, &v);
        assert!((dist - 0.0).abs() < 1e-6, "identical vectors should have distance 0");
    }

    #[test]
    fn test_embedding_distance_orthogonal() {
        // Two orthogonal unit vectors should have distance 1.0
        let mut a = vec![0.0f32; 128];
        let mut b = vec![0.0f32; 128];
        a[0] = 1.0;
        b[1] = 1.0;
        let dist = embedding_distance(&a, &b);
        assert!((dist - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_embedding_distance_opposite() {
        let mut a = vec![0.0f32; 128];
        let mut b = vec![0.0f32; 128];
        a[0] = 1.0;
        b[0] = -1.0;
        let dist = embedding_distance(&a, &b);
        assert!((dist - 2.0).abs() < 1e-6, "opposite vectors should have distance 2");
    }

    #[test]
    fn test_zero_norm_handling() {
        let a = vec![0.0f32; 10];
        let b = vec![1.0f32; 10];
        let dist = embedding_distance(&a, &b);
        assert!((dist - 1.0).abs() < 1e-6, "zero-norm should return 1.0");
    }

    #[test]
    fn test_deterministic_embeddings() {
        let v1 = seed_to_embedding("same_seed");
        let v2 = seed_to_embedding("same_seed");
        assert_eq!(v1, v2, "same seed must produce same embedding");
    }

    #[test]
    fn test_cluster_embeddings_groups_similar() {
        // Create two groups: files 0-2 are similar, files 3-4 are similar
        let base1 = seed_to_embedding("group_a");
        let base2 = seed_to_embedding("group_b");

        let embeddings = vec![
            ("f1".to_string(), base1.clone()),
            ("f2".to_string(), {
                let mut v = base1.clone();
                v[0] += 0.01; // tiny perturbation → still within threshold
                v
            }),
            ("f3".to_string(), base1.clone()),
            ("f4".to_string(), base2.clone()),
            ("f5".to_string(), base2.clone()),
        ];

        let clusters = cluster_embeddings(&embeddings, 0.3);
        assert_eq!(clusters.len(), 2, "should find 2 clusters");
        // Each cluster should have the right files
        let all_ids: Vec<String> = clusters.iter().flat_map(|(_, ids)| ids.clone()).collect();
        assert!(all_ids.contains(&"f1".to_string()));
        assert!(all_ids.contains(&"f4".to_string()));
    }

    #[test]
    fn test_detect_faces_in_file_returns_embeddings() {
        let file_node = FileNode {
            id: "test-id".to_string(),
            name: "photo.jpg".to_string(),
            file_type: "file".to_string(),
            parent_id: None,
            size_bytes: 1024,
            mime_type: Some("image/jpeg".to_string()),
            hash_blake3: Some("abc123".to_string()),
            encrypted: false,
            encryption_algorithm: None,
            compression_layers: Vec::new(),
            thumbnail_path: None,
            context_data: None,
            tags: Vec::new(),
            collection_ids: Vec::new(),
            face_group_ids: Vec::new(),
            loose_group_ids: Vec::new(),
            gps_lat: None,
            gps_lon: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            modified_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let result = detect_faces_in_file(&file_node).unwrap();
        assert!(!result.is_empty(), "should detect at least one face");
        assert_eq!(result[0].len(), EMBEDDING_DIM);
    }
}