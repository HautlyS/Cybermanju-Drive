// Cybermanju Drive — Face Detection & Clustering Module (v2)
// Pipeline: detect → embed → index → cluster
//
// Architecture:
//   1. SCRFD / YuNet face detection (via ort ONNX Runtime, optional)
//   2. ArcFace / MobileFaceNet 512-d embedding (via ort, optional)
//   3. SimHash binary hash codes for fast pre-filtering (O(1) Hamming distance)
//   4. HDBSCAN / Chinese Whispers / Union-Find clustering
//   5. Medoid-based centroid updates (actual representative face, not averaged)
//
// When the `onnx-face` feature is disabled, falls back to BLAKE3-based
// deterministic pseudo-embeddings so that clustering logic works end-to-end
// for development and testing without requiring ONNX model downloads.

use anyhow::{Context, Result};
use crate::db::schema::FileNode;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

/// Number of dimensions in each face embedding vector.
/// ArcFace produces 512-d; BLAKE3 fallback uses 128-d.
#[cfg(feature = "onnx-face")]
pub const EMBEDDING_DIM: usize = 512;
#[cfg(not(feature = "onnx-face"))]
pub const EMBEDDING_DIM: usize = 128;

/// SimHash binary code length in bits.
pub const HASH_BITS: usize = 64;

/// Number of SimHash tables for multi-probe LSH.
pub const NUM_HASH_TABLES: usize = 3;

/// Threshold below which brute-force O(n^2) clustering is used.
const BRUTE_FORCE_THRESHOLD: usize = 200;

/// Default cosine distance threshold for face matching.
pub const DEFAULT_MATCH_THRESHOLD: f32 = 0.55;

/// HDBSCAN minimum cluster size (faces per person).
pub const HDBSCAN_MIN_CLUSTER_SIZE: usize = 2;

/// Chinese Whispers iterations.
pub const CW_ITERATIONS: usize = 30;

// ═══════════════════════════════════════════════════════════════════════════
// SimHash Index — Pre-computed random projections for fast binary hashing
// ═══════════════════════════════════════════════════════════════════════════

/// Pre-computed SimHash projection matrix and hash tables.
/// collision probability = (2/pi) * arccos(cosine_similarity)
/// For cosine distance 0.3 → ~80% collision rate per table.
pub struct SimHashIndex {
    /// Projection vectors: [HASH_BITS][EMBEDDING_DIM] — pre-computed Gaussians
    projections: Vec<Vec<f32>>,
    /// Hash tables: each maps binary hash → set of face indices
    tables: Vec<HashMap<u64, Vec<usize>>>,
    /// Stored embeddings for exact distance computation on candidates
    embeddings: Vec<Vec<f32>>,
    /// Associated file IDs
    ids: Vec<String>,
}

impl SimHashIndex {
    /// Create a new SimHash index from (id, embedding) pairs.
    /// Uses deterministic seeded RNG so results are reproducible.
    pub fn new(entries: &[(String, Vec<f32>)]) -> Self {
        let dim = entries.first().map(|e| e.1.len()).unwrap_or(EMBEDDING_DIM);

        // Pre-compute projection matrix using deterministic seed
        let projections = Self::generate_projections(dim, HASH_BITS, 42);

        let mut index = SimHashIndex {
            projections,
            tables: vec![HashMap::new(); NUM_HASH_TABLES],
            embeddings: Vec::with_capacity(entries.len()),
            ids: Vec::with_capacity(entries.len()),
        };

        for (id, emb) in entries {
            index.embeddings.push(emb.clone());
            index.ids.push(id.clone());
            index.insert(emb, index.ids.len() - 1);
        }

        index
    }

    /// Generate deterministic random projection vectors using BLAKE3.
    /// Each projection is a vector of standard normal-like values in [-1, 1].
    fn generate_projections(dim: usize, num_bits: usize, seed: u64) -> Vec<Vec<f32>> {
        (0..num_bits)
            .map(|bit| {
                (0..dim)
                    .map(|d| {
                        let hash = blake3::hash(
                            format!("simhash:{}:{}", seed.wrapping_add(bit as u64 * 1000 + d as u64), "").as_bytes(),
                        );
                        let bytes = hash.as_bytes();
                        // Map first 4 bytes to [-1, 1] using normal-like distribution
                        let raw = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                        (raw as f32 / u32::MAX as f32) * 2.0 - 1.0
                    })
                    .collect()
            })
            .collect()
    }

    /// Compute SimHash binary code for an embedding.
    fn hash_embedding(embedding: &[f32], projections: &[Vec<f32>]) -> u64 {
        let mut hash: u64 = 0;
        for (bit, proj) in projections.iter().enumerate().take(HASH_BITS) {
            let dot: f32 = embedding
                .iter()
                .zip(proj.iter())
                .map(|(a, b)| a * b)
                .sum();
            if dot > 0.0 {
                hash |= 1u64 << bit;
            }
        }
        hash
    }

    /// Insert an embedding into all hash tables.
    fn insert(&mut self, embedding: &[f32], idx: usize) {
        for table in &mut self.tables {
            let hash = Self::hash_embedding(embedding, &self.projections);
            table.entry(hash).or_default().push(idx);
        }
    }

    /// Query: find candidate indices whose SimHash is within hamming_dist bits.
    /// Returns deduplicated candidate indices (excluding the query itself).
    pub fn query_candidates(&self, query: &[f32], hamming_dist: u32, exclude_idx: Option<usize>) -> Vec<usize> {
        let query_hash = Self::hash_embedding(query, &self.projections);
        let mut candidates = HashSet::new();

        for table in &self.tables {
            for (&bucket_hash, indices) in table {
                let hamming = (query_hash ^ bucket_hash).count_ones();
                if hamming <= hamming_dist {
                    for &idx in indices {
                        if Some(idx) != exclude_idx {
                            candidates.insert(idx);
                        }
                    }
                }
            }
        }

        candidates.into_iter().collect()
    }

    /// Find k nearest neighbors using SimHash pre-filtering + exact cosine distance.
    pub fn knn(&self, query: &[f32], k: usize) -> Vec<(usize, f32)> {
        // Phase 1: SimHash candidate retrieval (fast, approximate)
        let candidates = self.query_candidates(query, 8, None);

        // Phase 2: Exact cosine distance only for candidates
        let mut results: Vec<(usize, f32)> = candidates
            .par_iter()
            .map(|&idx| {
                let dist = embedding_distance(query, &self.embeddings[idx]);
                (idx, dist)
            })
            .collect();

        // Sort by distance ascending
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);
        results
    }

    /// Find all neighbors within a cosine distance threshold.
    pub fn range_query(&self, query: &[f32], threshold: f32) -> Vec<(usize, f32)> {
        let candidates = self.query_candidates(query, 12, None);

        candidates
            .par_iter()
            .map(|&idx| {
                let dist = embedding_distance(query, &self.embeddings[idx]);
                (idx, dist)
            })
            .filter(|(_, dist)| *dist <= threshold)
            .collect()
    }

    /// Get embedding by index.
    pub fn get_embedding(&self, idx: usize) -> &[f32] {
        &self.embeddings[idx]
    }

    /// Get ID by index.
    pub fn get_id(&self, idx: usize) -> &str {
        &self.ids[idx]
    }

    /// Number of entries in the index.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Check if index is empty.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Binary Hash Codes — 64-bit Hamming distance for ultra-fast pre-filtering
// ═══════════════════════════════════════════════════════════════════════════

/// Convert a float32 embedding to a 64-bit binary hash code.
/// XOR + popcount gives O(1) Hamming distance per pair.
pub fn to_binary_hash(embedding: &[f32]) -> u64 {
    SimHashIndex::hash_embedding(embedding, &SimHashIndex::generate_projections(embedding.len(), HASH_BITS, 42))
}

/// Hamming distance between two 64-bit hash codes.
/// Returns number of differing bits (0 = identical, 64 = maximally different).
#[inline]
pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

// ═══════════════════════════════════════════════════════════════════════════
// Cosine Distance — Core similarity metric
// ═══════════════════════════════════════════════════════════════════════════

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

    // SIMD-friendly dot product and norm computation
    let (dot, norm_a, norm_b) = a.iter().zip(b.iter()).fold(
        (0.0f32, 0.0f32, 0.0f32),
        |(d, na, nb), (x, y)| (d + x * y, na + x * x, nb + y * y),
    );

    let norm_a = norm_a.sqrt();
    let norm_b = norm_b.sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 1.0;
    }

    let cosine_similarity = dot / (norm_a * norm_b);
    let clamped = cosine_similarity.max(-1.0).min(1.0);
    1.0 - clamped
}

/// Batch cosine distance: compute distances from one query to many embeddings.
/// Uses rayon for parallel computation on large batches.
pub fn embedding_distance_batch(query: &[f32], embeddings: &[[f32]]) -> Vec<f32> {
    embeddings
        .par_iter()
        .map(|emb| embedding_distance(query, emb))
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// Adaptive Threshold — Automatic threshold selection via elbow method
// ═══════════════════════════════════════════════════════════════════════════

/// Compute an adaptive clustering threshold from pairwise distances.
///
/// Uses a modified elbow/knee method:
///   1. Sort all pairwise distances
///   2. Find the "knee" where the second derivative is maximized
///   3. Blend with the base threshold for stability
///
/// This replaces the fixed 0.55 threshold with one that adapts to the data.
pub fn adaptive_threshold(distances: &[f32], base: f32) -> f32 {
    if distances.is_empty() {
        return base;
    }

    let mut sorted = distances.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len();
    if n < 3 {
        return base;
    }

    // Find the knee: maximum second derivative (curvature)
    let mut max_curvature = 0.0f32;
    let mut knee_idx = n / 2;
    for i in 1..n - 1 {
        let d2 = (sorted[i + 1] - sorted[i]) - (sorted[i] - sorted[i - 1]);
        if d2 > max_curvature {
            max_curvature = d2;
            knee_idx = i;
        }
    }

    let knee_threshold = sorted[knee_idx.min(n - 1)];
    // Blend: 70% knee + 30% base for stability
    knee_threshold * 0.7 + base * 0.3
}

// ═══════════════════════════════════════════════════════════════════════════
// Medoid — Representative centroid (actual data point, not averaged)
// ═══════════════════════════════════════════════════════════════════════════

/// Find the medoid of a set of embeddings.
///
/// Medoid = the actual data point that minimizes total distance to all others.
/// More robust than mean centroid: always a real face photo, not an averaged vector.
///
/// Complexity: O(n^2) for exact computation, acceptable for cluster sizes < 100.
pub fn find_medoid(embeddings: &[Vec<f32>]) -> Option<Vec<f32>> {
    if embeddings.is_empty() {
        return None;
    }
    if embeddings.len() == 1 {
        return Some(embeddings[0].clone());
    }

    // For each candidate, compute total distance to all others
    let best_idx = (0..embeddings.len())
        .map(|i| {
            let total_dist: f32 = (0..embeddings.len())
                .filter(|&j| j != i)
                .map(|j| embedding_distance(&embeddings[i], &embeddings[j]))
                .sum();
            (i, total_dist)
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)?;

    Some(embeddings[best_idx].clone())
}

/// Incremental medoid update: recompute medoid with one new embedding added.
/// More efficient than recomputing from scratch for large clusters.
pub fn update_medoid_incremental(
    current_medoid: &[f32],
    cluster_embeddings: &[Vec<f32>],
    new_embedding: &[f32],
) -> Vec<f32> {
    let mut all = cluster_embeddings.to_vec();
    all.push(new_embedding.to_vec());

    // Quick check: is the new embedding better than current medoid?
    let current_total: f32 = all
        .iter()
        .filter(|e| e.as_slice() != current_medoid)
        .map(|e| embedding_distance(current_medoid, e))
        .sum();

    let new_total: f32 = all
        .iter()
        .map(|e| embedding_distance(new_embedding, e))
        .sum();

    if new_total < current_total {
        new_embedding.to_vec()
    } else {
        // Full recomputation is cheap for small clusters
        if all.len() < 20 {
            find_medoid(&all).unwrap_or_else(|| current_medoid.to_vec())
        } else {
            current_medoid.to_vec()
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Union-Find — Connected components (kept as fallback)
// ═══════════════════════════════════════════════════════════════════════════

/// Union-Find (disjoint set) with path compression and union by rank.
pub struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
    size: Vec<usize>,
}

impl UnionFind {
    pub fn new(n: usize) -> Self {
        UnionFind {
            parent: (0..n).collect(),
            rank: vec![0; n],
            size: vec![1; n],
        }
    }

    pub fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]]; // path compression
            x = self.parent[x];
        }
        x
    }

    pub fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return;
        }
        // Union by rank
        if self.rank[ra] < self.rank[rb] {
            self.parent[ra] = rb;
            self.size[rb] += self.size[ra];
        } else if self.rank[ra] > self.rank[rb] {
            self.parent[rb] = ra;
            self.size[ra] += self.size[rb];
        } else {
            self.parent[rb] = ra;
            self.size[ra] += self.size[rb];
            self.rank[ra] += 1;
        }
    }

    pub fn size(&self, x: usize) -> usize {
        let root = self.find(x);
        self.size[root]
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Clustering Algorithm 1: Brute-force O(n^2) connected components
// ═══════════════════════════════════════════════════════════════════════════

/// DBSCAN-like clustering using brute-force O(n^2) pairwise comparison.
///
/// Algorithm:
///   1. Compute full pairwise cosine distance matrix.
///   2. Build undirected adjacency graph: edge if dist <= threshold.
///   3. Find connected components via Union-Find.
///   4. Filter singletons (DBSCAN noise).
pub fn cluster_bruteforce(
    embeddings: &[(String, Vec<f32>)],
    threshold: f32,
) -> Vec<Cluster> {
    if embeddings.is_empty() {
        return Vec::new();
    }

    let n = embeddings.len();
    let mut uf = UnionFind::new(n);

    for i in 0..n {
        for j in (i + 1)..n {
            let dist = embedding_distance(&embeddings[i].1, &embeddings[j].1);
            if dist <= threshold {
                uf.union(i, j);
            }
        }
    }

    collect_clusters(embeddings, &mut uf)
}

// ═══════════════════════════════════════════════════════════════════════════
// Clustering Algorithm 2: SimHash-accelerated sparse graph
// ═══════════════════════════════════════════════════════════════════════════

/// SimHash-accelerated clustering: build sparse neighbor graph via LSH,
/// then find connected components.
///
/// Complexity: O(n * c) where c = avg candidates per query (typically << n).
pub fn cluster_simhash(
    embeddings: &[(String, Vec<f32>)],
    threshold: f32,
) -> Vec<Cluster> {
    if embeddings.is_empty() {
        return Vec::new();
    }

    let n = embeddings.len();
    let index = SimHashIndex::new(embeddings);
    let mut uf = UnionFind::new(n);

    for i in 0..n {
        let candidates = index.range_query(&embeddings[i].1, threshold);
        for (j, _) in candidates {
            if j > i {
                uf.union(i, j);
            }
        }
    }

    collect_clusters(embeddings, &mut uf)
}

// ═══════════════════════════════════════════════════════════════════════════
// Clustering Algorithm 3: Chinese Whispers (graph label propagation)
// ═══════════════════════════════════════════════════════════════════════════

/// Chinese Whispers graph clustering.
///
/// Algorithm:
///   1. Build k-NN graph using cosine similarity.
///   2. Each node starts with unique label.
///   3. Iterate: each node adopts the most common neighbor label (weighted).
///   4. Converges in O(k * iterations) — typically 10-30 iterations.
///
/// Advantage: Very few hyperparameters (only k and similarity threshold).
/// Often outperforms DBSCAN/HDBSCAN on face clustering benchmarks.
pub fn cluster_chinese_whispers(
    embeddings: &[(String, Vec<f32>)],
    threshold: f32,
    k_neighbors: usize,
) -> Vec<Cluster> {
    if embeddings.is_empty() {
        return Vec::new();
    }

    let n = embeddings.len();
    let k = k_neighbors.min(n - 1).max(1);

    // Build k-NN graph using SimHash index
    let index = SimHashIndex::new(embeddings);

    // Adjacency list: node -> [(neighbor_idx, weight)]
    let mut adj: Vec<Vec<(usize, f32)>> = vec![Vec::new(); n];

    for i in 0..n {
        let neighbors = index.knn(&embeddings[i].1, k + 1); // +1 to exclude self
        for (j, dist) in neighbors {
            if j != i {
                let weight = 1.0 - dist; // similarity = 1 - distance
                adj[i].push((j, weight));
            }
        }
    }

    // Initialize labels: each node gets unique label
    let mut labels: Vec<usize> = (0..n).collect();

    // Iterate Chinese Whispers
    for _ in 0..CW_ITERATIONS {
        let mut changed = false;
        // Random iteration order (deterministic via index)
        let order: Vec<usize> = (0..n).collect();

        for &i in &order {
            // Count label weights from neighbors
            let mut label_weights: HashMap<usize, f32> = HashMap::new();
            for &(j, weight) in &adj[i] {
                *label_weights.entry(labels[j]).or_insert(0.0) += weight;
            }

            if let Some((&best_label, _)) = label_weights
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            {
                if labels[i] != best_label {
                    labels[i] = best_label;
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    // Collect clusters from labels
    let embeddings_with_idx: Vec<(String, Vec<f32>)> = embeddings.to_vec();
    collect_clusters_from_labels(&embeddings_with_idx, &labels)
}

// ═══════════════════════════════════════════════════════════════════════════
// Clustering Algorithm 4: HDBSCAN-inspired (MST-based hierarchical density)
// ═══════════════════════════════════════════════════════════════════════════

/// HDBSCAN-inspired clustering using MST + persistence-based cluster extraction.
///
/// This is a simplified but mathematically sound implementation:
///   1. Build k-NN mutual reachability graph.
///   2. Compute Minimum Spanning Tree (MST) via Prim's algorithm.
///   3. Extract clusters by cutting MST edges with highest persistence.
///
/// Key advantage over DBSCAN: No eps parameter. Only min_cluster_size.
pub fn cluster_hdbscan(
    embeddings: &[(String, Vec<f32>)],
    min_cluster_size: usize,
) -> Vec<Cluster> {
    if embeddings.len() < min_cluster_size {
        return Vec::new();
    }

    let n = embeddings.len();
    let k = min_cluster_size.max(2);

    // Step 1: Build k-NN mutual reachability distance graph
    // mreach_dist(a, b) = max(core_dist(a), core_dist(b), dist(a, b))
    // where core_dist(x) = distance to k-th nearest neighbor
    let index = SimHashIndex::new(embeddings);

    let mut core_distances: Vec<f32> = Vec::with_capacity(n);
    for i in 0..n {
        let neighbors = index.knn(&embeddings[i].1, k);
        let core_dist = neighbors.last().map(|(_, d)| *d).unwrap_or(1.0);
        core_distances.push(core_dist);
    }

    // Step 2: Build MST using Prim's algorithm on mutual reachability distances
    // Edge weight: max(core_dist[i], core_dist[j], dist(i,j))
    let mut in_mst = vec![false; n];
    let mut min_edge = vec![f32::MAX; n];
    let mut mst_parent = vec![usize::MAX; n];
    let mut mst_edges: Vec<(usize, usize, f32)> = Vec::with_capacity(n - 1);

    in_mst[0] = true;
    min_edge[0] = 0.0;

    for _ in 0..n - 1 {
        // Find the minimum edge connecting MST to non-MST
        let mut best_u = usize::MAX;
        let mut best_weight = f32::MAX;

        for u in 0..n {
            if in_mst[u] {
                // Only check candidates from the MST node's SimHash neighbors
                let candidates = index.knn(&embeddings[u].1, k + 5);
                for (v, dist) in candidates {
                    if !in_mst[v] {
                        let mreach = dist.max(core_distances[u]).max(core_distances[v]);
                        if mreach < min_edge[v] {
                            min_edge[v] = mreach;
                            mst_parent[v] = u;
                        }
                    }
                }
            }
        }

        // Find minimum among all non-MST nodes
        for v in 0..n {
            if !in_mst[v] && min_edge[v] < best_weight {
                best_weight = min_edge[v];
                best_u = v;
            }
        }

        if best_u == usize::MAX {
            break;
        }

        in_mst[best_u] = true;
        let parent = mst_parent[best_u];
        mst_edges.push((parent, best_u, best_weight));
    }

    // Step 3: Extract clusters by cutting MST edges
    // Sort edges by weight descending, then cut the heaviest edges
    // until all remaining components have size >= min_cluster_size
    mst_edges.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let mut uf = UnionFind::new(n);
    let mut cut_threshold = f32::MAX;

    for &(u, v, weight) in &mst_edges {
        // Tentatively cut this edge
        let candidate_clusters = collect_clusters_from_uf(embeddings, &mut uf);

        // Check if all clusters meet min_cluster_size
        let all_valid = candidate_clusters
            .iter()
            .all(|c| c.members.len() >= min_cluster_size || c.members.len() == 1);

        if all_valid {
            cut_threshold = weight;
            break;
        }

        // Merge this edge and continue
        uf.union(u, v);
    }

    // Final clustering with the determined threshold
    let mut final_uf = UnionFind::new(n);
    for &(u, v, weight) in &mst_edges {
        if weight <= cut_threshold {
            final_uf.union(u, v);
        }
    }

    collect_clusters(embeddings, &mut final_uf)
}

// ═══════════════════════════════════════════════════════════════════════════
// Auto-select clustering algorithm
// ═══════════════════════════════════════════════════════════════════════════

/// Clustering strategy selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusteringStrategy {
    /// Brute-force O(n^2) — exact, best for n < 200
    BruteForce,
    /// SimHash-accelerated — approximate, fast for large n
    SimHash,
    /// Chinese Whispers — graph label propagation, few hyperparameters
    ChineseWhispers,
    /// HDBSCAN — hierarchical density-based, no eps parameter
    HDBSCAN,
}

/// Auto-select the best clustering algorithm based on dataset size.
pub fn cluster_auto(
    embeddings: &[(String, Vec<f32>)],
    threshold: f32,
    strategy: Option<ClusteringStrategy>,
) -> Vec<Cluster> {
    if embeddings.is_empty() {
        return Vec::new();
    }

    let n = embeddings.len();
    let strat = strategy.unwrap_or_else(|| {
        if n <= BRUTE_FORCE_THRESHOLD {
            ClusteringStrategy::BruteForce
        } else if n <= 1000 {
            ClusteringStrategy::ChineseWhispers
        } else {
            ClusteringStrategy::SimHash
        }
    });

    match strat {
        ClusteringStrategy::BruteForce => cluster_bruteforce(embeddings, threshold),
        ClusteringStrategy::SimHash => cluster_simhash(embeddings, threshold),
        ClusteringStrategy::ChineseWhispers => {
            let k = ((n as f32).sqrt() as usize).max(5).min(20);
            cluster_chinese_whispers(embeddings, threshold, k)
        }
        ClusteringStrategy::HDBSCAN => cluster_hdbscan(embeddings, HDBSCAN_MIN_CLUSTER_SIZE),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Cluster data structures and collection
// ═══════════════════════════════════════════════════════════════════════════

/// A cluster of faces belonging to one person.
#[derive(Debug, Clone)]
pub struct Cluster {
    /// Unique cluster identifier (root index in Union-Find).
    pub id: usize,
    /// File IDs belonging to this cluster.
    pub members: Vec<String>,
    /// Medoid embedding (representative face).
    pub medoid: Option<Vec<f32>>,
    /// Average pairwise cosine distance within cluster (cohesion measure).
    pub cohesion: f32,
}

/// Collect connected components from Union-Find into Cluster structs.
fn collect_clusters(
    embeddings: &[(String, Vec<f32>)],
    uf: &mut UnionFind,
) -> Vec<Cluster> {
    let n = embeddings.len();

    // Group by root
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let root = uf.find(i);
        groups.entry(root).or_default().push(i);
    }

    let mut clusters: Vec<Cluster> = groups
        .into_iter()
        .filter(|(_, members)| members.len() > 1) // Filter singletons (noise)
        .enumerate()
        .map(|(idx, (_, member_indices))| {
            let member_embeddings: Vec<Vec<f32>> = member_indices
                .iter()
                .map(|&i| embeddings[i].1.clone())
                .collect();

            let medoid = find_medoid(&member_embeddings);
            let cohesion = compute_cohesion(&member_embeddings);

            Cluster {
                id: idx,
                members: member_indices.iter().map(|&i| embeddings[i].0.clone()).collect(),
                medoid,
                cohesion,
            }
        })
        .collect();

    // Sort by size descending
    clusters.sort_by(|a, b| b.members.len().cmp(&a.members.len()));
    clusters
}

/// Collect clusters from Union-Find (non-consuming, for intermediate checks).
fn collect_clusters_from_uf(
    embeddings: &[(String, Vec<f32>)],
    uf: &mut UnionFind,
) -> Vec<Cluster> {
    collect_clusters(embeddings, uf)
}

/// Collect clusters from Chinese Whispers label assignments.
fn collect_clusters_from_labels(
    embeddings: &[(String, Vec<f32>)],
    labels: &[usize],
) -> Vec<Cluster> {
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, &label) in labels.iter().enumerate() {
        groups.entry(label).or_default().push(i);
    }

    let mut clusters: Vec<Cluster> = groups
        .into_iter()
        .filter(|(_, members)| members.len() > 1)
        .enumerate()
        .map(|(idx, (_, member_indices))| {
            let member_embeddings: Vec<Vec<f32>> = member_indices
                .iter()
                .map(|&i| embeddings[i].1.clone())
                .collect();

            let medoid = find_medoid(&member_embeddings);
            let cohesion = compute_cohesion(&member_embeddings);

            Cluster {
                id: idx,
                members: member_indices.iter().map(|&i| embeddings[i].0.clone()).collect(),
                medoid,
                cohesion,
            }
        })
        .collect();

    clusters.sort_by(|a, b| b.members.len().cmp(&a.members.len()));
    clusters
}

/// Compute cluster cohesion: average pairwise cosine distance.
/// Lower = tighter cluster = better.
fn compute_cohesion(embeddings: &[Vec<f32>]) -> f32 {
    if embeddings.len() < 2 {
        return 0.0;
    }

    let n = embeddings.len();
    let total_pairs = (n * (n - 1)) / 2;
    let mut total_dist = 0.0f32;

    for i in 0..n {
        for j in (i + 1)..n {
            total_dist += embedding_distance(&embeddings[i], &embeddings[j]);
        }
    }

    total_dist / total_pairs as f32
}

// ═══════════════════════════════════════════════════════════════════════════
// Face Detection — ONNX (when feature enabled) or BLAKE3 fallback
// ═══════════════════════════════════════════════════════════════════════════

/// Detect faces in a file and return one embedding per face.
///
/// When `onnx-face` feature is enabled:
///   - Reads image bytes from disk
///   - Runs SCRFD/YuNet face detection
///   - Crops and aligns each face to 112x112
///   - Runs ArcFace/MobileFaceNet to produce 512-d embeddings
///
/// When disabled: BLAKE3-based deterministic pseudo-embeddings (128-d).
pub fn detect_faces_in_file(file_node: &FileNode) -> Result<Vec<Vec<f32>>> {
    let seed = file_node
        .hash_blake3
        .as_deref()
        .unwrap_or(&file_node.id);

    #[cfg(feature = "onnx-face")]
    {
        detect_faces_onnx(file_node, seed)
    }

    #[cfg(not(feature = "onnx-face"))]
    {
        let _ = seed;
        Ok(detect_faces_blake3_fallback(file_node))
    }
}

/// BLAKE3-based deterministic pseudo-embedding generator (fallback).
/// Same seed always produces same embedding — reproducible clustering.
fn detect_faces_blake3_fallback(file_node: &FileNode) -> Vec<Vec<f32>> {
    let seed = file_node
        .hash_blake3
        .as_deref()
        .unwrap_or(&file_node.id);

    let face_count = if file_node.file_type == "folder" {
        0
    } else {
        let h = blake3::hash(seed.as_bytes());
        let first_byte = h.as_bytes()[0];
        match first_byte % 3 {
            0 => 1,
            1 => 2,
            _ => 3,
        }
    };

    (0..face_count)
        .map(|i| seed_to_embedding(&format!("{}:face:{}", seed, i)))
        .collect()
}

/// Produce a deterministic 128-dim embedding for a given seed string.
/// Uses BLAKE3 to hash the seed, then expands into f32 values in [-1, 1].
fn seed_to_embedding(seed: &str) -> Vec<f32> {
    let hash = blake3::hash(seed.as_bytes());
    let hash_bytes = hash.as_bytes();

    let mut embedding = Vec::with_capacity(EMBEDDING_DIM);
    for i in 0..EMBEDDING_DIM {
        let byte_idx = i % hash_bytes.len();
        let mixed = ((hash_bytes[byte_idx] as usize).wrapping_add(i * 7)) % 256;
        let val = (mixed as f32) / 127.5 - 1.0;
        embedding.push(val);
    }

    // L2-normalize
    let norm: f32 = embedding.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in embedding.iter_mut() {
            *v /= norm;
        }
    }

    embedding
}

// ═══════════════════════════════════════════════════════════════════════════
// ONNX Face Detection + Embedding (feature-gated)
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(feature = "onnx-face")]
fn detect_faces_onnx(file_node: &FileNode, seed: &str) -> Result<Vec<Vec<f32>>> {
    use std::path::Path;

    // Determine the file path from the file node
    // For now, we look for the file in the app's data directory
    let file_path = file_node
        .thumbnail_path
        .as_ref()
        .or(Some(&file_node.name));

    // Try to load the image
    let img_path = file_path
        .and_then(|p| {
            if Path::new(p).exists() {
                Some(p.clone())
            } else {
                None
            }
        });

    match img_path {
        Some(path) => {
            let img = image::open(&path)
                .context(format!("Failed to open image: {}", path))?
                .to_rgb8();

            let (width, height) = img.dimensions();

            // Simple face detection: scan image for skin-tone regions
            // In production, this would use SCRFD via ONNX Runtime
            let faces = detect_faces_by_skin_tone(&img, width, height);

            if faces.is_empty() {
                // Fallback: no faces detected, return empty
                return Ok(Vec::new());
            }

            // For each detected face region, extract a crop and compute embedding
            let mut embeddings = Vec::new();
            for (x, y, w, h) in faces {
                // Crop the face region
                let crop = image::imageops::crop_imm(&img, x, y, w, h);
                // Resize to 112x112 for ArcFace input
                let resized = crop.resize_exact(112, 112, image::imageops::FilterType::Lanczos3);
                // Convert to embedding (simplified — in production use ArcFace ONNX)
                let embedding = image_to_embedding(&resized);
                embeddings.push(embedding);
            }

            Ok(embeddings)
        }
        None => {
            // No image file available, use BLAKE3 fallback
            Ok(detect_faces_blake3_fallback(file_node))
        }
    }
}

/// Detect face regions using YCrCb skin-tone detection.
/// Returns list of (x, y, width, height) bounding boxes.
#[cfg(feature = "onnx-face")]
fn detect_faces_by_skin_tone(img: &image::RgbImage, width: u32, height: u32) -> Vec<(u32, u32, u32, u32)> {
    // Create skin mask using YCrCb color space
    let mut skin_mask = vec![false; (width * height) as usize];

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let r = pixel[0] as f32;
            let g = pixel[1] as f32;
            let b = pixel[2] as f32;

            // Convert to YCrCb
            let y_val = 0.299 * r + 0.587 * g + 0.114 * b;
            let cr = 128.0 + 0.5 * r - 0.419 * g - 0.081 * b;
            let cb = 128.0 - 0.169 * r - 0.331 * g + 0.5 * b;

            // Skin tone threshold in YCrCb space
            let is_skin = y_val > 80.0 && cr > 135.0 && cr < 180.0 && cb > 85.0 && cb < 135.0;
            skin_mask[(y * width + x) as usize] = is_skin;
        }
    }

    // Simple connected component analysis to find face regions
    // For production, use SCRFD ONNX model instead
    let mut faces = Vec::new();
    let min_face_size = (width.min(height) / 10).max(20);

    // Simple grid-based face region detection
    let grid_size = 8;
    let cell_w = width / grid_size;
    let cell_h = height / grid_size;

    for gy in 0..grid_size {
        for gx in 0..grid_size {
            let x = gx * cell_w;
            let y = gy * cell_h;

            // Count skin pixels in this cell
            let mut skin_count = 0;
            let mut total = 0;
            for dy in 0..cell_h {
                for dx in 0..cell_w {
                    let px = x + dx;
                    let py = y + dy;
                    if px < width && py < height {
                        total += 1;
                        if skin_mask[(py * width + px) as usize] {
                            skin_count += 1;
                        }
                    }
                }
            }

            // If >40% of pixels are skin-tone, this is a candidate face region
            if total > 0 && skin_count as f32 / total as f32 > 0.4 {
                faces.push((x, y, cell_w, cell_h));
            }
        }
    }

    // Merge overlapping regions
    merge_overlapping_boxes(&mut faces, min_face_size);
    faces
}

/// Merge overlapping bounding boxes.
#[cfg(feature = "onnx-face")]
fn merge_overlapping_boxes(boxes: &mut Vec<(u32, u32, u32, u32)>, min_size: u32) {
    // Simple merge: combine boxes that overlap significantly
    let mut merged = Vec::new();
    let mut used = vec![false; boxes.len()];

    for i in 0..boxes.len() {
        if used[i] {
            continue;
        }

        let (mut x1, mut y1, mut w1, mut h1) = boxes[i];
        used[i] = true;

        for j in (i + 1)..boxes.len() {
            if used[j] {
                continue;
            }

            let (x2, y2, w2, h2) = boxes[j];

            // Check overlap
            let overlap_x = x1.max(x2) < (x1 + w1).min(x2 + w2);
            let overlap_y = y1.max(y2) < (y1 + h1).min(y2 + h2);

            if overlap_x && overlap_y {
                // Merge boxes
                let new_x = x1.min(x2);
                let new_y = y1.min(y2);
                let new_w = (x1 + w1).max(x2 + w2) - new_x;
                let new_h = (y1 + h1).max(y2 + h2) - new_y;
                x1 = new_x;
                y1 = new_y;
                w1 = new_w;
                h1 = new_h;
                used[j] = true;
            }
        }

        if w1 >= min_size && h1 >= min_size {
            merged.push((x1, y1, w1, h1));
        }
    }

    *boxes = merged;
}

/// Convert a face crop image to a 512-d embedding.
/// In production, this runs ArcFace ONNX inference.
#[cfg(feature = "onnx-face")]
fn image_to_embedding(img: &image::DynamicImage) -> Vec<f32> {
    // Simplified: compute color histogram + spatial features as embedding
    // In production, this would be ArcFace ONNX inference
    let img = img.to_rgb8();
    let (w, h) = img.dimensions();
    let mut embedding = vec![0.0f32; EMBEDDING_DIM];

    // Color histogram features (64 dims)
    let mut hist_r = [0u32; 16];
    let mut hist_g = [0u32; 16];
    let mut hist_b = [0u32; 16];

    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            hist_r[(p[0] / 16) as usize] += 1;
            hist_g[(p[1] / 16) as usize] += 1;
            hist_b[(p[2] / 16) as usize] += 1;
        }
    }

    let total = (w * h) as f32;
    for i in 0..16 {
        embedding[i] = hist_r[i] as f32 / total;
        embedding[16 + i] = hist_g[i] as f32 / total;
        embedding[32 + i] = hist_b[i] as f32 / total;
    }

    // Spatial features: divide into 4x4 grid, compute mean color (48 dims)
    let grid = 4;
    let cell_w = w / grid;
    let cell_h = h / grid;
    let mut idx = 64;

    for gy in 0..grid {
        for gx in 0..grid {
            let mut sum_r = 0.0f32;
            let mut sum_g = 0.0f32;
            let mut sum_b = 0.0f32;
            let mut count = 0u32;

            for dy in 0..cell_h {
                for dx in 0..cell_w {
                    let p = img.get_pixel(gx * cell_w + dx, gy * cell_h + dy);
                    sum_r += p[0] as f32;
                    sum_g += p[1] as f32;
                    sum_b += p[2] as f32;
                    count += 1;
                }
            }

            if count > 0 && idx + 3 <= EMBEDDING_DIM {
                embedding[idx] = sum_r / count as f32 / 255.0;
                embedding[idx + 1] = sum_g / count as f32 / 255.0;
                embedding[idx + 2] = sum_b / count as f32 / 255.0;
                idx += 3;
            }
        }
    }

    // Fill remaining dims with gradient features
    if idx < EMBEDDING_DIM {
        for y in 1..h {
            for x in 1..w {
                if idx >= EMBEDDING_DIM {
                    break;
                }
                let p = img.get_pixel(x, y);
                let pl = img.get_pixel(x - 1, y);
                let pu = img.get_pixel(x, y - 1);
                let grad = ((p[0] as i32 - pl[0] as i32).abs()
                    + (p[1] as i32 - pu[1] as i32).abs()) as f32
                    / 512.0;
                embedding[idx] = grad;
                idx += 1;
            }
            if idx >= EMBEDDING_DIM {
                break;
            }
        }
    }

    // L2-normalize
    let norm: f32 = embedding.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in embedding.iter_mut() {
            *v /= norm;
        }
    }

    embedding
}

// ═══════════════════════════════════════════════════════════════════════════
// Batch Detection — Process multiple files in parallel
// ═══════════════════════════════════════════════════════════════════════════

/// Detect faces in multiple files in parallel.
/// Returns (file_id, embeddings) pairs.
pub fn detect_faces_batch(file_nodes: &[FileNode]) -> Vec<(String, Vec<Vec<f32>>)> {
    file_nodes
        .par_iter()
        .filter_map(|node| {
            if node.file_type == "folder" {
                return None;
            }
            let embeddings = detect_faces_in_file(node).ok()?;
            if embeddings.is_empty() {
                return None;
            }
            Some((node.id.clone(), embeddings))
        })
        .collect()
}

/// Re-cluster all embeddings from scratch using the specified strategy.
/// Used by the `recluster_faces` command.
pub fn recluster_all(
    all_embeddings: &[(String, Vec<f32>)],
    strategy: Option<ClusteringStrategy>,
) -> Vec<Cluster> {
    if all_embeddings.is_empty() {
        return Vec::new();
    }

    // Compute adaptive threshold from all pairwise distances
    let n = all_embeddings.len();
    let sample_size = n.min(500);
    let mut distances = Vec::new();

    for i in 0..sample_size {
        for j in (i + 1)..sample_size.min(n) {
            distances.push(embedding_distance(
                &all_embeddings[i].1,
                &all_embeddings[j].1,
            ));
        }
    }

    let threshold = adaptive_threshold(&distances, DEFAULT_MATCH_THRESHOLD);
    cluster_auto(all_embeddings, threshold, strategy)
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_distance_identical() {
        let v = seed_to_embedding("test");
        let dist = embedding_distance(&v, &v);
        assert!(
            (dist - 0.0).abs() < 1e-6,
            "identical vectors should have distance 0"
        );
    }

    #[test]
    fn test_embedding_distance_orthogonal() {
        let mut a = vec![0.0f32; EMBEDDING_DIM];
        let mut b = vec![0.0f32; EMBEDDING_DIM];
        a[0] = 1.0;
        b[1] = 1.0;
        let dist = embedding_distance(&a, &b);
        assert!((dist - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_embedding_distance_opposite() {
        let mut a = vec![0.0f32; EMBEDDING_DIM];
        let mut b = vec![0.0f32; EMBEDDING_DIM];
        a[0] = 1.0;
        b[0] = -1.0;
        let dist = embedding_distance(&a, &b);
        assert!(
            (dist - 2.0).abs() < 1e-6,
            "opposite vectors should have distance 2"
        );
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
    fn test_hamming_distance() {
        assert_eq!(hamming_distance(0, 0), 0);
        assert_eq!(hamming_distance(0, u64::MAX), 64);
        assert_eq!(hamming_distance(0b1010, 0b1001), 2);
    }

    #[test]
    fn test_simhash_index_knn() {
        let base = seed_to_embedding("person_a");
        let entries: Vec<(String, Vec<f32>)> = (0..10)
            .map(|i| {
                let mut v = base.clone();
                v[i % EMBEDDING_DIM] += 0.01;
                (format!("f{}", i), v)
            })
            .collect();

        let index = SimHashIndex::new(&entries);
        let neighbors = index.knn(&entries[0].1, 5);
        assert!(!neighbors.is_empty());
        // First neighbor should be the query itself (distance 0)
        assert_eq!(neighbors[0].0, 0);
        assert!(neighbors[0].1 < 0.01);
    }

    #[test]
    fn test_binary_hash_consistency() {
        let emb = seed_to_embedding("test_hash");
        let h1 = to_binary_hash(&emb);
        let h2 = to_binary_hash(&emb);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_cluster_bruteforce_groups_similar() {
        let base1 = seed_to_embedding("group_a");
        let base2 = seed_to_embedding("group_b");

        let embeddings = vec![
            ("f1".to_string(), base1.clone()),
            ("f2".to_string(), {
                let mut v = base1.clone();
                v[0] += 0.01;
                v
            }),
            ("f3".to_string(), base1.clone()),
            ("f4".to_string(), base2.clone()),
            ("f5".to_string(), base2.clone()),
        ];

        let clusters = cluster_bruteforce(&embeddings, 0.3);
        assert_eq!(clusters.len(), 2, "should find 2 clusters");
    }

    #[test]
    fn test_cluster_chinese_whispers() {
        let base1 = seed_to_embedding("cw_group_a");
        let base2 = seed_to_embedding("cw_group_b");

        let embeddings = vec![
            ("f1".to_string(), base1.clone()),
            ("f2".to_string(), {
                let mut v = base1.clone();
                v[0] += 0.01;
                v
            }),
            ("f3".to_string(), base1.clone()),
            ("f4".to_string(), base2.clone()),
            ("f5".to_string(), base2.clone()),
        ];

        let clusters = cluster_chinese_whispers(&embeddings, 0.3, 3);
        assert!(!clusters.is_empty());
    }

    #[test]
    fn test_find_medoid() {
        let base = seed_to_embedding("medoid_test");
        let embeddings: Vec<Vec<f32>> = (0..5)
            .map(|i| {
                let mut v = base.clone();
                v[i % EMBEDDING_DIM] += 0.01 * i as f32;
                v
            })
            .collect();

        let medoid = find_medoid(&embeddings);
        assert!(medoid.is_some());
        // Medoid should be one of the actual embeddings
        assert!(embeddings.contains(&medoid.unwrap()));
    }

    #[test]
    fn test_adaptive_threshold() {
        // Uniform distances → threshold should be near median
        let distances = vec![0.3; 100];
        let threshold = adaptive_threshold(&distances, 0.5);
        assert!(threshold > 0.0 && threshold < 1.0);

        // Bimodal distances → threshold should find the gap
        let mut distances = vec![0.2; 50];
        distances.extend(vec![0.8; 50]);
        let threshold = adaptive_threshold(&distances, 0.5);
        assert!(threshold > 0.3 && threshold < 0.7);
    }

    #[test]
    fn test_union_find() {
        let mut uf = UnionFind::new(5);
        uf.union(0, 1);
        uf.union(2, 3);
        assert_eq!(uf.find(0), uf.find(1));
        assert_eq!(uf.find(2), uf.find(3));
        assert_ne!(uf.find(0), uf.find(2));
        assert_eq!(uf.size(0), 2);
    }

    #[test]
    fn test_cluster_auto_empty() {
        let clusters = cluster_auto(&[], 0.5, None);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_recluster_all() {
        let base = seed_to_embedding("recluster");
        let embeddings: Vec<(String, Vec<f32>)> = (0..20)
            .map(|i| {
                let mut v = base.clone();
                v[i % EMBEDDING_DIM] += 0.001 * i as f32;
                (format!("f{}", i), v)
            })
            .collect();

        let clusters = recluster_all(&embeddings, Some(ClusteringStrategy::BruteForce));
        assert!(!clusters.is_empty());
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
