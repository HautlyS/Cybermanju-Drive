// Cybermanju Drive — Core Type Definitions
// Neobrutalism × Buddhist-Nepalese × Matrix × Cyberpunk

export type ViewMode = 'grid' | 'list' | 'masonry'
export type PanelType = 'landing' | 'files' | 'preview' | 'encryption' | 'compression' | 'collections' | 'faces' | 'map' | 'code' | 'search' | 'style' | 'accounts' | 'loose-groups' | 'sync' | 'webdash' | 'users' | 'dashboard'
export type SidebarSection = 'tree' | 'locations' | 'collections' | 'people' | 'styles' | 'loose' | 'users' | 'sync' | 'dashboard'
export type EncryptionAlgo = 'kyber1024' | 'dilithium5' | 'frodokem1344' | 'hybrid' | 'aes256'
export type CompressionType = 'none' | 'lz4' | 'zstd' | 'triple'
export type AccountType = 'local' | 'cloud' | 'network'
export type CollectionType = 'highlights' | 'best_moments' | 'custom'

export interface FileNode {
  id: string
  name: string
  fileType: string               // "file" | "folder" — Rust: file_type
  parentId?: string              // Rust: parent_id
  sizeBytes: number              // Rust: size_bytes
  mimeType?: string              // Rust: mime_type
  hashBlake3?: string            // Rust: hash_blake3
  encrypted: boolean             // Rust: encrypted
  encryptionAlgorithm?: string   // Rust: encryption_algorithm
  compressionLayers: string[]    // Rust: compression_layers
  thumbnailPath?: string         // Rust: thumbnail_path
  contextData?: Record<string, unknown>  // Rust: context_data
  tags?: string[]                // Rust: tags
  collectionIds?: string[]       // Rust: collection_ids
  faceGroupIds?: string[]        // Rust: face_group_ids
  looseGroupIds?: string[]       // Rust: loose_group_ids
  gpsLat?: number                // Rust: gps_lat
  gpsLon?: number                // Rust: gps_lon
  createdAt: string              // Rust: created_at
  modifiedAt: string             // Rust: modified_at
  // Extended fields (not from Rust, added by frontend)
  isStarred?: boolean
  isHidden?: boolean
  path?: string
  children?: FileNode[]
  accountId?: string
  locationId?: string
  contentText?: string
  treeSitterAst?: string
  permissions?: FilePermission[]
}

export interface FilePermission {
  userId: string
  username: string
  access: 'read' | 'write' | 'admin'
}

export interface Account {
  id: string
  name: string
  accountType: string            // Rust: account_type
  path?: string
  color: string
  isActive: boolean              // Rust: is_active
  createdAt: string              // Rust: created_at
  updatedAt: string              // Rust: updated_at
}

export interface CloudAccount {
  id: string
  name: string
  backendType: SyncBackendType   // Rust: backend_type
  token?: string
  config: Record<string, unknown>
  createdAt: string              // Rust: created_at
  updatedAt: string              // Rust: updated_at
}

export interface Collection {
  id: string
  name: string
  collectionType: CollectionType  // Rust: collection_type
  color: string
  description?: string
  itemIds: string[]              // Rust: item_ids
  createdAt: string              // Rust: created_at
  updatedAt: string              // Rust: updated_at
}

export interface FaceGroup {
  id: string
  name: string
  color?: string
  icon?: string
  fileIds: string[]              // Rust: file_ids
  centroidEmbedding?: number[]   // Rust: centroid_embedding
  binaryHash?: number            // Rust: binary_hash — 64-bit SimHash code
  cohesion?: number              // Rust: cohesion — avg intra-cluster cosine distance
  embeddingCount: number         // Rust: embedding_count
  algorithm?: string             // Rust: algorithm — clustering algorithm used
  createdAt: string              // Rust: created_at
}

export interface EncryptionKeyInfo {
  id: string
  algorithm: string
  algorithmDisplay: string       // Rust: algorithm_display
  nistLevel: number              // Rust: nist_level
  color: string
  publicKeyPreview: string      // Rust: public_key_preview
  hasPrivateKey: boolean         // Rust: has_private_key
  createdAt: string              // Rust: created_at
}

export interface EncryptionStatus {
  isEncrypted: boolean           // Rust: is_encrypted
  algorithm?: string
  nistLevel?: number             // Rust: nist_level
  keyId?: string                 // Rust: key_id
  encryptedAt?: string           // Rust: encrypted_at
}

export interface LooseGroup {
  id: string
  name: string
  color: string
  icon?: string
  fileIds: string[]              // Rust: file_ids
  createdAt: string              // Rust: created_at
}

export interface SearchResult {
  fileId: string                 // Rust: file_id
  fileName: string               // Rust: file_name
  score: number
  snippet?: string
  matchType?: string             // Rust: match_type
}

export interface GeoMarker {
  fileId: string                 // Rust: file_id
  fileName: string               // Rust: file_name
  lat: number
  lng: number
  address?: string
  thumbnail?: string
}

export interface CompressionStats {
  originalSize: number           // Rust: original_size
  compressedSize: number         // Rust: compressed_size
  ratio: number
  layer: string
  layerDetails: LayerDetail[]    // Rust: layer_details
  blake3Hash: string             // Rust: blake3_hash
  durationMs: number             // Rust: duration_ms
}

export interface LayerDetail {
  name: string
  algorithm: string
  inputSize: number              // Rust: input_size
  outputSize: number             // Rust: output_size
  ratio: number
  color: string
}

export interface CodeSymbol {
  name: string
  kind: string
  startLine: number              // Rust: start_line
  endLine: number                // Rust: end_line
  detail?: string
  children: CodeSymbol[]
}

export interface ParseResult {
  filePath: string               // Rust: file_path
  language: string
  symbols: CodeSymbol[]
  totalLines: number             // Rust: total_lines
  parseTimeMs: number            // Rust: parse_time_ms
}

// User Management Types
export interface User {
  id: string
  username: string
  passwordHash?: string          // Rust: password_hash
  displayName?: string           // Rust: display_name
  role: 'admin' | 'user' | 'viewer'
  isActive: boolean              // Rust: is_active
  createdAt: string              // Rust: created_at
  updatedAt: string              // Rust: updated_at
}

export interface UserFilePermission {
  id: string
  userId: string                 // Rust: user_id
  fileId: string                 // Rust: file_id
  access: 'read' | 'write' | 'admin'
  grantedBy: string              // Rust: granted_by
  grantedAt: string              // Rust: granted_at
}

export interface AuthResult {
  userId: string                  // Rust: user_id
  username: string
  role: string
  displayName?: string           // Rust: display_name
  token: string
}

// Web Dashboard Types
export interface DashboardStatus {
  running: boolean
  port: number
  url: string
  activeConnections: number      // Rust: active_connections
}

export interface ApiEndpoint {
  method: string
  path: string
  description: string
}

// Cybermanju Design Tokens
export const CYBER = {
  bgDeep: '#0a0a0f',
  bgPanel: '#12121a',
  bgCard: '#1a1a2e',
  bgHover: '#252540',
  borderHeavy: '#000000',
  borderNeon: '#00FF41',
  borderGold: '#FFB800',
  saffronGold: '#FFB800',
  lotusPink: '#FF2D6F',
  templeOrange: '#FF6B2B',
  prayerBlue: '#1E3A8A',
  prayerWhite: '#F5F5F4',
  prayerRed: '#DC2626',
  prayerGreen: '#16A34A',
  prayerYellow: '#FACC15',
  matrixGreen: '#00FF41',
  matrixDarkGreen: '#003B00',
  cyberBlue: '#00D4FF',
  cyberPurple: '#A855F7',
  neonPink: '#FF00FF',
  neonYellow: '#EFFF00',
  textPrimary: '#F5F5F4',
  textSecondary: '#9CA3AF',
  textMuted: '#6B7280',
  textNeon: '#00FF41',
} as const

export const PRAYER_FLAGS = [
  CYBER.prayerBlue,
  CYBER.prayerWhite,
  CYBER.prayerRed,
  CYBER.prayerGreen,
  CYBER.prayerYellow,
] as const

export const ENCRYPTION_INFO: Record<EncryptionAlgo, { name: string; nistLevel: number; description: string; color: string }> = {
  kyber1024: {
    name: 'ML-KEM (Kyber-1024)',
    nistLevel: 5,
    description: 'NIST FIPS 203 — Lattice-based key encapsulation. Resistant to Shor\'s algorithm and all known quantum attacks.',
    color: '#00FF41',
  },
  dilithium5: {
    name: 'ML-DSA (Dilithium-5)',
    nistLevel: 5,
    description: 'NIST FIPS 204 — Lattice-based digital signature. Maximum security level, quantum-resistant signing.',
    color: '#00D4FF',
  },
  frodokem1344: {
    name: 'FrodoKEM-1344',
    nistLevel: 3,
    description: 'Learning-with-errors based. Conservative security estimates with classical ring structure.',
    color: '#A855F7',
  },
  hybrid: {
    name: 'Hybrid PQ+Classical',
    nistLevel: 5,
    description: 'Combines ML-KEM with X25519 for defense-in-depth transitional security.',
    color: '#FFB800',
  },
  aes256: {
    name: 'AES-256-GCM',
    nistLevel: 0,
    description: 'Classical symmetric encryption. Not quantum-resistant — recommended only in hybrid mode.',
    color: '#FF6B2B',
  },
}

export const COMPRESSION_INFO: Record<CompressionType, { name: string; description: string; color: string; speed: string }> = {
  none: { name: 'None', description: 'Uncompressed raw data', color: '#6B7280', speed: 'Instant' },
  lz4: { name: 'LZ4 (lz4_flex)', description: 'Ultra-fast pure Rust compression (~400 MB/s). Real-time previews and streaming.', color: '#00D4FF', speed: 'Ultra-Fast' },
  zstd: { name: 'Zstandard (zstd)', description: 'Facebook\'s algorithm. Excellent ratio/speed balance, configurable levels 1-22.', color: '#00FF41', speed: 'Fast' },
  triple: { name: 'Triple-Layer', description: 'LZ4 → ZSTD-15 → Brotli-11 cascading. Maximum compression for archival.', color: '#FFB800', speed: 'Slow' },
}

// Storage Sync Types
export type SyncBackendType = 'local' | 'github' | 'gitlab' | 'googleDrive' | 'googlePhotos' | 'telegram'
export type SyncStatusType = 'idle' | 'scanning' | 'compressing' | 'uploading' | 'linking' | 'cleaning' | 'error' | 'done'

export interface SyncConfig {
  id: string
  backendType: SyncBackendType
  enabled: boolean
  accountId?: string           // Multi-account support (links to CloudAccount.id)
  name?: string                // Display name for this sync config
  basePath?: string            // Local: directory path, GitLab: instance URL (e.g., https://gitlab.com)
  repoName?: string            // GitHub: owner/repo, GitLab: project ID or path
  branch?: string
  token?: string               // GitHub: PAT, GitLab: PAT/OAuth, Google: OAuth2 Bearer token, Telegram: bot token
  folderId?: string            // Google Drive: folder ID
  albumId?: string             // Google Photos: album ID
  chatId?: string              // Telegram: target chat ID (channel, group, or user)
  autoSync: boolean
  compressBeforeUpload: boolean
  createPreviews: boolean
  deleteRawAfterSync: boolean
  maxConcurrentUploads: number
  createdAt: string
  updatedAt: string
}

export interface SyncFile {
  id: string
  originalPath: string
  compressedPath?: string
  previewPath?: string
  remoteUrl?: string
  sizeBytes: number
  compressedSizeBytes?: number
  hashBlake3?: string
  backendType: SyncBackendType
  syncedAt?: string
  status: SyncStatusType
  errorMessage?: string
}

export interface SyncProgress {
  totalFiles: number
  processedFiles: number
  currentFile?: string
  status: SyncStatusType
  bytesUploaded: number
  errors: string[]
  startedAt: string
  estimatedRemainingSeconds: number
}

export interface SyncResult {
  filesSynced: number
  bytesUploaded: number
  bytesSavedByCompression: number
  errors: string[]
  durationMs: number
}

export interface RemoteFile {
  name: string
  path: string
  sizeBytes: number
  modifiedAt: string
  url: string
}

export const SYNC_BACKEND_INFO: Record<SyncBackendType, { name: string; description: string; color: string; icon: string }> = {
  local: {
    name: 'Local Storage',
    description: 'Sync files to a local directory on this machine. Fast, no network required.',
    color: '#00FF41',
    icon: 'HardDrive',
  },
  github: {
    name: 'GitHub',
    description: 'Sync files to a GitHub repository using the Contents API. Supports releases for large files.',
    color: '#F5F5F4',
    icon: 'Github',
  },
  gitlab: {
    name: 'GitLab',
    description: 'Sync files to a GitLab project repository. Full CRUD via GitLab API v4.',
    color: '#FC6D26',
    icon: 'GitBranch',
  },
  googleDrive: {
    name: 'Google Drive',
    description: 'Sync files to Google Drive folders. Full CRUD via Drive API v3.',
    color: '#00D4FF',
    icon: 'FolderSync',
  },
  googlePhotos: {
    name: 'Google Photos',
    description: 'Upload photos and videos to Google Photos. Optimized for media files.',
    color: '#FFB800',
    icon: 'Camera',
  },
  telegram: {
    name: 'Telegram',
    description: 'Send files to a Telegram chat, channel, or group via Bot API. Files up to 50 MB per upload.',
    color: '#0088CC',
    icon: 'MessageCircle',
  },
}