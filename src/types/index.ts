export type ViewMode = 'grid' | 'list' | 'masonry'
export type PanelType = 'landing' | 'files' | 'preview' | 'encryption' | 'compression' | 'collections' | 'faces' | 'map' | 'code' | 'search' | 'style' | 'accounts' | 'loose-groups' | 'sync' | 'webdash' | 'users' | 'dashboard' | 'settings' | 'trash' | 'activity' | 'favorites' | 'recent' | 'storage'
export type SidebarSection = 'tree' | 'locations' | 'collections' | 'people' | 'styles' | 'loose' | 'users' | 'sync' | 'dashboard' | 'landing'

export interface ModuleInfo {
  id: PanelType
  label: string
  icon: string
  color: string
  gradient: string
  description: string
  requiresAuth: boolean
}
export type EncryptionAlgo = 'kyber1024' | 'dilithium5' | 'frodokem1344' | 'hybrid' | 'aes256'
export type CompressionType = 'none' | 'lz4' | 'zstd' | 'triple'
export type AccountType = 'local' | 'cloud' | 'network'
export type CollectionType = 'highlights' | 'best_moments' | 'custom'

export interface FileNode {
  id: string
  name: string
  fileType: string
  parentId?: string
  sizeBytes: number
  mimeType?: string
  hashBlake3?: string
  encrypted: boolean
  encryptionAlgorithm?: string
  compressionLayers: string[]
  thumbnailPath?: string
  contextData?: Record<string, unknown>
  tags?: string[]
  collectionIds?: string[]
  faceGroupIds?: string[]
  looseGroupIds?: string[]
  gpsLat?: number
  gpsLon?: number
  createdAt: string
  modifiedAt: string
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
  accountType: string
  path?: string
  color: string
  isActive: boolean
  createdAt: string
  updatedAt: string
}

export interface CloudAccount {
  id: string
  name: string
  backendType: SyncBackendType
  token?: string
  config: Record<string, unknown>
  createdAt: string
  updatedAt: string
}

export interface Collection {
  id: string
  name: string
  collectionType: CollectionType
  color: string
  description?: string
  itemIds: string[]
  createdAt: string
  updatedAt: string
}

export interface FaceGroup {
  id: string
  name: string
  color?: string
  icon?: string
  fileIds: string[]
  centroidEmbedding?: number[]
  binaryHash?: number
  cohesion?: number
  embeddingCount: number
  algorithm?: string
  createdAt: string
}

export interface EncryptionKeyInfo {
  id: string
  algorithm: string
  algorithmDisplay: string
  nistLevel: number
  color: string
  publicKeyPreview: string
  hasPrivateKey: boolean
  createdAt: string
}

export interface EncryptionStatus {
  isEncrypted: boolean
  algorithm?: string
  nistLevel?: number
  keyId?: string
  encryptedAt?: string
}

export interface LooseGroup {
  id: string
  name: string
  color: string
  icon?: string
  fileIds: string[]
  createdAt: string
}

export interface SearchResult {
  fileId: string
  fileName: string
  score: number
  snippet?: string
  matchType?: string
}

export interface GeoMarker {
  fileId: string
  fileName: string
  lat: number
  lng: number
  address?: string
  thumbnail?: string
}

export interface CompressionStats {
  originalSize: number
  compressedSize: number
  ratio: number
  layer: string
  layerDetails: LayerDetail[]
  blake3Hash: string
  durationMs: number
}

export interface LayerDetail {
  name: string
  algorithm: string
  inputSize: number
  outputSize: number
  ratio: number
  color: string
}

export interface CodeSymbol {
  name: string
  kind: string
  startLine: number
  endLine: number
  detail?: string
  children: CodeSymbol[]
}

export interface ParseResult {
  filePath: string
  language: string
  symbols: CodeSymbol[]
  totalLines: number
  parseTimeMs: number
}

export interface User {
  id: string
  username: string
  passwordHash?: string
  displayName?: string
  role: 'admin' | 'user' | 'viewer'
  isActive: boolean
  createdAt: string
  updatedAt: string
}

export interface UserFilePermission {
  id: string
  userId: string
  fileId: string
  access: 'read' | 'write' | 'admin'
  grantedBy: string
  grantedAt: string
}

export interface AuthResult {
  userId: string
  username: string
  role: string
  displayName?: string
  token: string
}

export interface DashboardStatus {
  running: boolean
  port: number
  url: string
  activeConnections: number
}

export interface ApiEndpoint {
  method: string
  path: string
  description: string
}

export interface TrashItem {
  id: string
  originalFile: FileNode
  deletedAt: string
  deletedBy?: string
  restorePath?: string
}

export interface AuditEntry {
  id: string
  action: string
  entityType: string
  entityId: string
  userId?: string
  details?: Record<string, unknown>
  timestamp: string
}

export interface FileVersion {
  id: string
  fileId: string
  versionNumber: number
  hashBlake3?: string
  sizeBytes: number
  snapshotData?: string
  createdAt: string
}

export interface ShareLink {
  id: string
  fileId: string
  token: string
  expiresAt: string
  url: string
}

export const CYBER = {
  bgDeep: '#000000',
  bgPanel: '#000000',
  bgCard: '#FFFFFF',
  bgHover: '#FFFFFF',
  borderHeavy: '#FFFFFF',
  borderNeon: '#FFFFFF',
  borderGold: '#FFFFFF',
  saffronGold: '#FFFFFF',
  lotusPink: '#FFFFFF',
  templeOrange: '#FFFFFF',
  prayerBlue: '#FFFFFF',
  prayerWhite: '#FFFFFF',
  prayerRed: '#FFFFFF',
  prayerGreen: '#FFFFFF',
  prayerYellow: '#FFFFFF',
  matrixGreen: '#FFFFFF',
  matrixDarkGreen: '#000000',
  cyberBlue: '#FFFFFF',
  cyberPurple: '#FFFFFF',
  neonPink: '#FFFFFF',
  neonYellow: '#FFFFFF',
  textPrimary: '#FFFFFF',
  textSecondary: '#FFFFFF',
  textMuted: '#FFFFFF',
  textNeon: '#FFFFFF',
} as const

export const PRAYER_FLAGS = ['#FFFFFF', '#000000', '#FFFFFF', '#000000', '#FFFFFF'] as const

export const ENCRYPTION_INFO: Record<EncryptionAlgo, { name: string; nistLevel: number; description: string; color: string }> = {
  kyber1024: {
    name: 'ML-KEM (Kyber-1024)',
    nistLevel: 5,
    description: 'NIST FIPS 203 - Lattice-based key encapsulation. Resistant to Shor\'s algorithm and all known quantum attacks.',
    color: '#FFFFFF',
  },
  dilithium5: {
    name: 'ML-DSA (Dilithium-5)',
    nistLevel: 5,
    description: 'NIST FIPS 204 - Lattice-based digital signature. Maximum security level, quantum-resistant signing.',
    color: '#FFFFFF',
  },
  frodokem1344: {
    name: 'FrodoKEM-1344',
    nistLevel: 3,
    description: 'Learning-with-errors based. Conservative security estimates with classical ring structure.',
    color: '#FFFFFF',
  },
  hybrid: {
    name: 'Hybrid PQ+Classical',
    nistLevel: 5,
    description: 'Combines ML-KEM with X25519 for defense-in-depth transitional security.',
    color: '#FFFFFF',
  },
  aes256: {
    name: 'AES-256-GCM',
    nistLevel: 0,
    description: 'Classical symmetric encryption. Not quantum-resistant - recommended only in hybrid mode.',
    color: '#FFFFFF',
  },
}

export const COMPRESSION_INFO: Record<CompressionType, { name: string; description: string; color: string; speed: string }> = {
  none: { name: 'None', description: 'Uncompressed raw data', color: '#FFFFFF', speed: 'Instant' },
  lz4: { name: 'LZ4 (lz4_flex)', description: 'Ultra-fast pure Rust compression (~400 MB/s). Real-time previews and streaming.', color: '#FFFFFF', speed: 'Ultra-Fast' },
  zstd: { name: 'Zstandard (zstd)', description: 'Facebook\'s algorithm. Excellent ratio/speed balance, configurable levels 1-22.', color: '#FFFFFF', speed: 'Fast' },
  triple: { name: 'Triple-Layer', description: 'LZ4 -> ZSTD-15 -> Brotli-11 cascading. Maximum compression for archival.', color: '#FFFFFF', speed: 'Slow' },
}

export type SyncBackendType = 'local' | 'github' | 'gitlab' | 'googleDrive' | 'googlePhotos' | 'telegram'
export type SyncStatusType = 'idle' | 'scanning' | 'compressing' | 'uploading' | 'linking' | 'cleaning' | 'error' | 'done'

export interface SyncConfig {
  id: string
  backendType: SyncBackendType
  enabled: boolean
  accountId?: string
  name?: string
  basePath?: string
  repoName?: string
  branch?: string
  token?: string
  folderId?: string
  albumId?: string
  chatId?: string
  autoSync: boolean
  compressBeforeUpload: boolean
  createPreviews: boolean
  deleteRawAfterSync: boolean
  maxConcurrentUploads: number
  createdAt?: string
  updatedAt?: string
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
  startedAt?: string
  estimatedRemainingSeconds?: number
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

export const MODULE_METADATA: Record<PanelType, ModuleInfo> = {
  landing: { id: 'landing', label: 'HOME', icon: '[~]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0a0a0a 50%, #000000 100%)', description: 'Quantum-resistant encrypted file manager', requiresAuth: false },
  files: { id: 'files', label: 'FILES', icon: '[#]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0a0a1a 50%, #000000 100%)', description: 'Browse and manage your encrypted files', requiresAuth: true },
  search: { id: 'search', label: 'SEARCH', icon: '[S]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0a0a0a 50%, #000000 100%)', description: 'Tantivy BM25 full-text search', requiresAuth: true },
  collections: { id: 'collections', label: 'COLLECTIONS', icon: '[*]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0d0800 50%, #000000 100%)', description: 'Curate and organize file collections', requiresAuth: true },
  faces: { id: 'faces', label: 'PEOPLE', icon: '[+]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0d0008 50%, #000000 100%)', description: 'AI face detection and clustering', requiresAuth: true },
  map: { id: 'map', label: 'MAP', icon: '[@]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #000a0d 50%, #000000 100%)', description: 'GPS-tagged files on MapLibre GL', requiresAuth: true },
  code: { id: 'code', label: 'CODE', icon: '[T]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #000d08 50%, #000000 100%)', description: 'Tree-sitter code intelligence', requiresAuth: true },
  sync: { id: 'sync', label: 'SYNC', icon: '[~]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #00080d 50%, #000000 100%)', description: 'Multi-backend cloud sync', requiresAuth: true },
  accounts: { id: 'accounts', label: 'ACCOUNTS', icon: '[@]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0a0a0d 50%, #000000 100%)', description: 'Manage local and cloud accounts', requiresAuth: true },
  'loose-groups': { id: 'loose-groups', label: 'LOOSE', icon: '[%]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0d0d00 50%, #000000 100%)', description: 'Ad-hoc file grouping', requiresAuth: true },
  style: { id: 'style', label: 'TAGS', icon: '[&]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0a000d 50%, #000000 100%)', description: 'CLIP-based visual style tags', requiresAuth: true },
  users: { id: 'users', label: 'USERS', icon: '[!]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0d0000 50%, #000000 100%)', description: 'Multi-user access control', requiresAuth: true },
  dashboard: { id: 'dashboard', label: 'REMOTE', icon: '[D]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #00080d 50%, #000000 100%)', description: 'Web dashboard and API status', requiresAuth: true },
  webdash: { id: 'webdash', label: 'OVERLAY', icon: '[W]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #05050a 50%, #000000 100%)', description: 'Remote access dashboard', requiresAuth: true },
  preview: { id: 'preview', label: 'PREVIEW', icon: '[P]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0a0a0a 50%, #000000 100%)', description: 'File preview panel', requiresAuth: true },
  encryption: { id: 'encryption', label: 'ENCRYPT', icon: '[@]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0d0000 50%, #000000 100%)', description: 'Post-quantum encryption management', requiresAuth: true },
  compression: { id: 'compression', label: 'COMPRESS', icon: '[$]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #000d00 50%, #000000 100%)', description: 'Triple-layer compression pipeline', requiresAuth: true },
  settings: { id: 'settings', label: 'SETTINGS', icon: '[@]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0a0a0a 50%, #000000 100%)', description: 'Application settings and preferences', requiresAuth: true },
  trash: { id: 'trash', label: 'TRASH', icon: '[%]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0d0000 50%, #000000 100%)', description: 'Deleted files', requiresAuth: true },
  activity: { id: 'activity', label: 'ACTIVITY', icon: '[~]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #00080d 50%, #000000 100%)', description: 'File activity log', requiresAuth: true },
  favorites: { id: 'favorites', label: 'FAVORITES', icon: '[*]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #0d0800 50%, #000000 100%)', description: 'Starred files', requiresAuth: true },
  recent: { id: 'recent', label: 'RECENT', icon: '[T]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #080808 50%, #000000 100%)', description: 'Recently modified files', requiresAuth: true },
  storage: { id: 'storage', label: 'STORAGE', icon: '[$]', color: '#FFFFFF', gradient: 'linear-gradient(180deg, #000000 0%, #000a00 50%, #000000 100%)', description: 'Storage usage dashboard', requiresAuth: true },
}

export const SYNC_BACKEND_INFO: Record<SyncBackendType, { name: string; description: string; color: string; icon: string }> = {
  local: {
    name: 'Local Storage',
    description: 'Sync files to a local directory on this machine. Fast, no network required.',
    color: '#FFFFFF',
    icon: 'HardDrive',
  },
  github: {
    name: 'GitHub',
    description: 'Sync files to a GitHub repository using the Contents API. Supports releases for large files.',
    color: '#FFFFFF',
    icon: 'Github',
  },
  gitlab: {
    name: 'GitLab',
    description: 'Sync files to a GitLab project repository. Full CRUD via GitLab API v4.',
    color: '#FFFFFF',
    icon: 'GitBranch',
  },
  googleDrive: {
    name: 'Google Drive',
    description: 'Sync files to Google Drive folders. Full CRUD via Drive API v3.',
    color: '#FFFFFF',
    icon: 'FolderSync',
  },
  googlePhotos: {
    name: 'Google Photos',
    description: 'Upload photos and videos to Google Photos. Optimized for media files.',
    color: '#FFFFFF',
    icon: 'Camera',
  },
  telegram: {
    name: 'Telegram',
    description: 'Send files to a Telegram chat, channel, or group via Bot API. Files up to 50 MB per upload.',
    color: '#FFFFFF',
    icon: 'MessageCircle',
  },
}
