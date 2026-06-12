// Cybermanju Drive — Pinia Store
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@/composables/useTauri'
import type {
  FileNode, Account, Collection, FaceGroup, LooseGroup,
  SearchResult, EncryptionStatus, EncryptionKeyInfo,
  CompressionStats, ParseResult, GeoMarker,
  ViewMode, PanelType, SidebarSection,
  SyncConfig, SyncProgress, SyncResult, RemoteFile,
} from '@/types'

export const useAppStore = defineStore('cybermanju', () => {
  // ── Navigation State ──────────────────────────────────────
  const currentPath = ref('/')
  const currentPanel = ref<PanelType>('files')
  const viewMode = ref<ViewMode>('grid')
  const selectedFileId = ref<string | null>(null)
  const sidebarSection = ref<SidebarSection>('tree')
  const sidebarCollapsed = ref(false)

  // ── Data State ────────────────────────────────────────────
  const files = ref<FileNode[]>([])
  const accounts = ref<Account[]>([])
  const activeAccountId = ref<string | null>(null)
  const collections = ref<Collection[]>([])
  const faceGroups = ref<FaceGroup[]>([])
  const looseGroups = ref<LooseGroup[]>([])
  const searchResults = ref<SearchResult[]>([])
  const geoMarkers = ref<GeoMarker[]>([])

  // ── Encryption State ──────────────────────────────────────
  const encryptionStatus = ref<EncryptionStatus>({ isEncrypted: false })
  const encryptionKeys = ref<EncryptionKeyInfo[]>([])

  // ── Compression State ─────────────────────────────────────
  const compressionStats = ref<CompressionStats | null>(null)

  // ── Sync State ────────────────────────────────────────────
  const syncConfigs = ref<SyncConfig[]>([])
  const syncProgress = ref<SyncProgress | null>(null)

  // ── Code Intelligence State ───────────────────────────────
  const parseResult = ref<ParseResult | null>(null)

  // ── UI State ──────────────────────────────────────────────
  const searchQuery = ref('')
  const isSearching = ref(false)
  const isLoading = ref(false)
  const lastError = ref<string | null>(null)
  const matrixRainEnabled = ref(true)
  const showEncryptionPanel = ref(false)
  const showCompressionPanel = ref(false)
  const commandPaletteOpen = ref(false)

  // ── Computed ──────────────────────────────────────────────
  const selectedFile = computed(() =>
    files.value.find(f => f.id === selectedFileId.value) || null
  )

  const activeAccount = computed(() =>
    accounts.value.find(a => a.id === activeAccountId.value) || accounts.value[0] || null
  )

  const encryptedFiles = computed(() =>
    files.value.filter(f => f.encrypted)
  )

  const compressedFiles = computed(() =>
    files.value.filter(f => f.compressionLayers && f.compressionLayers.length > 0 && !f.compressionLayers.includes('none'))
  )

  const starredFiles = computed(() =>
    files.value.filter(f => f.isStarred)
  )

  const folders = computed(() =>
    files.value.filter(f => f.fileType === 'folder')
  )

  const currentFolderFiles = computed(() =>
    files.value.filter(f => f.parentId === selectedFileId.value || (selectedFileId.value === null && !f.parentId))
  )

  // ── Error Helper ──────────────────────────────────────────
  function setError(msg: string, error: unknown) {
    const detail = error instanceof Error ? error.message : String(error)
    lastError.value = `${msg}: ${detail}`
    console.error(lastError.value)
  }

  function clearError() {
    lastError.value = null
  }

  // ── Actions: Init ─────────────────────────────────────────
  async function initialize() {
    isLoading.value = true
    clearError()
    try {
      await Promise.allSettled([
        fetchFiles(),
        fetchAccounts(),
        fetchCollections(),
        fetchFaceGroups(),
        fetchLooseGroups(),
        fetchEncryptionStatus(),
        listKeys(),
        fetchSyncConfigs(),
      ])
    } finally {
      isLoading.value = false
    }
  }

  // ── Actions: Selection ────────────────────────────────────
  function selectFile(fileId: string | null) {
    selectedFileId.value = fileId
  }

  function toggleStar(fileId: string) {
    const file = files.value.find(f => f.id === fileId)
    if (file) {
      file.isStarred = !file.isStarred
    }
  }

  // ── Actions: Files ────────────────────────────────────────
  async function fetchFiles(parentPath?: string) {
    isLoading.value = true
    clearError()
    try {
      const path = parentPath || currentPath.value
      const result = await invoke<FileNode[]>('list_files', { parentPath: path })
      files.value = result
    } catch (e) {
      setError('Failed to fetch files', e)
    } finally {
      isLoading.value = false
    }
  }

  async function getFile(fileId: string) {
    try {
      return await invoke<FileNode>('get_file', { fileId })
    } catch (e) {
      setError('Failed to get file', e)
      return null
    }
  }

  async function createFolder(name: string, parentId: string) {
    try {
      await invoke('create_folder', { name, parentId })
      await fetchFiles()
    } catch (e) {
      setError('Failed to create folder', e)
    }
  }

  async function deleteFile(fileId: string) {
    try {
      await invoke('delete_file', { fileId })
      files.value = files.value.filter(f => f.id !== fileId)
      if (selectedFileId.value === fileId) selectedFileId.value = null
    } catch (e) {
      setError('Failed to delete file', e)
    }
  }

  async function renameFile(fileId: string, newName: string) {
    try {
      await invoke('rename_file', { fileId, newName })
      await fetchFiles()
    } catch (e) {
      setError('Failed to rename file', e)
    }
  }

  async function duplicateFileContext(fileId: string) {
    try {
      await invoke('duplicate_file_context', { fileId })
      await fetchFiles()
    } catch (e) {
      setError('Failed to duplicate file', e)
    }
  }

  // ── Actions: Search ───────────────────────────────────────
  async function searchFiles(query: string) {
    if (!query.trim()) { searchResults.value = []; return }
    isSearching.value = true
    clearError()
    try {
      searchResults.value = await invoke<SearchResult[]>('search_files', { query, limit: 50 })
    } catch (e) {
      setError('Search failed', e)
    } finally {
      isSearching.value = false
    }
  }

  // ── Actions: Encryption ───────────────────────────────────
  async function fetchEncryptionStatus() {
    try {
      encryptionStatus.value = await invoke<EncryptionStatus>('get_encryption_status')
    } catch (e) {
      setError('Failed to get encryption status', e)
    }
  }

  async function generateKeypair(algorithm: string) {
    try {
      await invoke('generate_keypair', { algorithm })
      await listKeys()
      await fetchEncryptionStatus()
    } catch (e) {
      setError('Failed to generate keypair', e)
    }
  }

  async function listKeys() {
    try {
      encryptionKeys.value = await invoke<EncryptionKeyInfo[]>('list_keys')
    } catch (e) {
      setError('Failed to list keys', e)
    }
  }

  async function encryptFile(fileId: string, algorithm: string) {
    try {
      await invoke('encrypt_file', { fileId, algorithm })
      await fetchFiles()
      await fetchEncryptionStatus()
    } catch (e) {
      setError('Encryption failed', e)
    }
  }

  async function decryptFile(fileId: string) {
    try {
      await invoke('decrypt_file', { fileId })
      await fetchFiles()
      await fetchEncryptionStatus()
    } catch (e) {
      setError('Decryption failed', e)
    }
  }

  // ── Actions: Compression ──────────────────────────────────
  async function compressFile(fileId: string, layer: string) {
    try {
      const stats = await invoke<CompressionStats>('compress_file', { fileId, layer })
      compressionStats.value = stats
      await fetchFiles()
    } catch (e) {
      setError('Compression failed', e)
    }
  }

  async function decompressFile(fileId: string) {
    try {
      const stats = await invoke<CompressionStats>('decompress_file', { fileId })
      compressionStats.value = stats
      await fetchFiles()
    } catch (e) {
      setError('Decompression failed', e)
    }
  }

  // ── Actions: Collections ──────────────────────────────────
  async function fetchCollections() {
    try {
      collections.value = await invoke<Collection[]>('list_collections')
    } catch (e) {
      setError('Failed to fetch collections', e)
    }
  }

  async function createCollection(name: string, type: string, color: string, description?: string) {
    try {
      await invoke('create_collection', { name, collectionType: type, color, description })
      await fetchCollections()
    } catch (e) {
      setError('Failed to create collection', e)
    }
  }

  async function addToCollection(collectionId: string, fileId: string, note?: string) {
    try {
      await invoke('add_to_collection', { collectionId, fileId, note })
      await fetchCollections()
    } catch (e) {
      setError('Failed to add to collection', e)
    }
  }

  async function removeFromCollection(collectionId: string, fileId: string) {
    try {
      await invoke('remove_from_collection', { collectionId, fileId })
      await fetchCollections()
    } catch (e) {
      setError('Failed to remove from collection', e)
    }
  }

  // ── Actions: Face Groups ──────────────────────────────────
  async function fetchFaceGroups() {
    try {
      faceGroups.value = await invoke<FaceGroup[]>('list_face_groups')
    } catch (e) {
      setError('Failed to fetch face groups', e)
    }
  }

  async function detectFaces(fileId: string) {
    try {
      await invoke('detect_faces', { fileId })
      await fetchFaceGroups()
    } catch (e) {
      setError('Face detection failed', e)
    }
  }

  async function detectFacesBatch() {
    try {
      const result = await invoke<{ clustersCreated: number; totalFaces: number; noiseFaces: number; avgCohesion: number; strategyUsed: string }>('detect_faces_batch_cmd')
      await fetchFaceGroups()
      return result
    } catch (e) {
      setError('Batch face detection failed', e)
      return null
    }
  }

  async function reclusterFaces(strategy?: string) {
    try {
      const result = await invoke<{ clustersCreated: number; totalFaces: number; noiseFaces: number; avgCohesion: number; strategyUsed: string }>('recluster_faces', { strategy })
      await fetchFaceGroups()
      return result
    } catch (e) {
      setError('Re-clustering failed', e)
      return null
    }
  }

  async function renameFaceGroup(groupId: string, newName: string) {
    try {
      await invoke('rename_face_group', { groupId, newName })
      await fetchFaceGroups()
    } catch (e) {
      setError('Failed to rename face group', e)
    }
  }

  async function mergeFaceGroups(sourceGroupId: string, targetGroupId: string) {
    try {
      await invoke('merge_face_groups', { sourceGroupId, targetGroupId })
      await fetchFaceGroups()
    } catch (e) {
      setError('Failed to merge face groups', e)
    }
  }

  async function deleteFaceGroup(groupId: string) {
    try {
      await invoke('delete_face_group', { groupId })
      await fetchFaceGroups()
    } catch (e) {
      setError('Failed to delete face group', e)
    }
  }

  async function findSimilarFaces(groupId: string, threshold?: number) {
    try {
      return await invoke<FaceGroup[]>('find_similar_faces', { groupId, threshold })
    } catch (e) {
      setError('Failed to find similar faces', e)
      return []
    }
  }

  // ── Actions: Accounts ─────────────────────────────────────
  async function fetchAccounts() {
    try {
      accounts.value = await invoke<Account[]>('list_accounts')
      const active = accounts.value.find(a => a.isActive)
      if (active) activeAccountId.value = active.id
    } catch (e) {
      setError('Failed to fetch accounts', e)
    }
  }

  async function createAccount(name: string, type: string, path?: string, color?: string) {
    try {
      await invoke('create_account', { name, accountType: type, path, color })
      await fetchAccounts()
    } catch (e) {
      setError('Failed to create account', e)
    }
  }

  async function switchAccount(accountId: string) {
    try {
      await invoke('switch_account', { accountId })
      activeAccountId.value = accountId
      await fetchAccounts()
    } catch (e) {
      setError('Failed to switch account', e)
    }
  }

  // ── Actions: Map ──────────────────────────────────────────
  async function fetchGeoFiles() {
    try {
      const geoFiles = await invoke<Array<{ id: string; name: string; gpsLat?: number; gpsLon?: number }>>('get_geo_files')
      geoMarkers.value = geoFiles
        .filter(f => f.gpsLat != null && f.gpsLon != null)
        .map(f => ({
          fileId: f.id,
          fileName: f.name,
          lat: f.gpsLat!,
          lng: f.gpsLon!,
        }))
    } catch (e) {
      setError('Failed to fetch geo files', e)
    }
  }

  // ── Actions: Tree-sitter ──────────────────────────────────
  async function parseFileCode(filePath: string) {
    try {
      parseResult.value = await invoke<ParseResult>('parse_file', { filePath })
    } catch (e) {
      setError('Parse failed', e)
    }
  }

  // ── Actions: Loose Groups ─────────────────────────────────
  async function fetchLooseGroups() {
    try {
      looseGroups.value = await invoke<LooseGroup[]>('list_loose_groups')
    } catch (e) {
      setError('Failed to fetch loose groups', e)
    }
  }

  // ── Actions: Sync ──────────────────────────────────────
  async function fetchSyncConfigs() {
    try {
      syncConfigs.value = await invoke<SyncConfig[]>('list_sync_configs')
    } catch (e) {
      setError('Failed to fetch sync configs', e)
    }
  }

  async function createSyncConfig(config: Omit<SyncConfig, 'id' | 'createdAt' | 'updatedAt'>) {
    try {
      await invoke('create_sync_config', { config })
      await fetchSyncConfigs()
    } catch (e) {
      setError('Failed to create sync config', e)
    }
  }

  async function deleteSyncConfig(configId: string) {
    try {
      await invoke('delete_sync_config', { configId })
      await fetchSyncConfigs()
    } catch (e) {
      setError('Failed to delete sync config', e)
    }
  }

  async function startSync(configId: string, fileIds: string[]) {
    try {
      await invoke('start_sync', { configId, fileIds })
      await pollSyncProgress()
    } catch (e) {
      setError('Failed to start sync', e)
    }
  }

  async function pollSyncProgress() {
    try {
      syncProgress.value = await invoke<SyncProgress>('get_sync_progress')
      if (syncProgress.value &&
          syncProgress.value.status !== 'idle' &&
          syncProgress.value.status !== 'done' &&
          syncProgress.value.status !== 'error') {
        setTimeout(() => pollSyncProgress(), 1000)
      }
    } catch (e) {
      setError('Failed to get sync progress', e)
    }
  }

  async function getSyncProgress() {
    try {
      syncProgress.value = await invoke<SyncProgress>('get_sync_progress')
    } catch (e) {
      setError('Failed to get sync progress', e)
    }
  }

  async function testSyncConnection(config: SyncConfig) {
    try {
      return await invoke<boolean>('test_sync_connection', { config })
    } catch (e) {
      setError('Sync connection test failed', e)
      return false
    }
  }

  async function cancelSync() {
    try {
      await invoke('cancel_sync')
      syncProgress.value = null
    } catch (e) {
      setError('Failed to cancel sync', e)
    }
  }

  async function listRemoteFiles(config: SyncConfig, prefix: string) {
    try {
      return await invoke<RemoteFile[]>('list_remote_files', { config, prefix })
    } catch (e) {
      setError('Failed to list remote files', e)
      return []
    }
  }

  return {
    // State
    currentPath, currentPanel, viewMode, selectedFileId, sidebarSection, sidebarCollapsed,
    files, accounts, activeAccountId, collections, faceGroups, looseGroups,
    searchResults, geoMarkers, encryptionStatus, encryptionKeys,
    compressionStats, parseResult, syncConfigs, syncProgress,
    searchQuery, isSearching, isLoading, lastError, matrixRainEnabled,
    showEncryptionPanel, showCompressionPanel, commandPaletteOpen,
    // Computed
    selectedFile, activeAccount, encryptedFiles, compressedFiles,
    starredFiles, folders, currentFolderFiles,
    // Actions
    initialize, selectFile, toggleStar, clearError,
    fetchFiles, getFile, createFolder, deleteFile, renameFile, duplicateFileContext,
    searchFiles, fetchEncryptionStatus, generateKeypair, listKeys, encryptFile, decryptFile,
    compressFile, decompressFile, fetchCollections, createCollection, addToCollection, removeFromCollection,
    fetchFaceGroups, detectFaces, detectFacesBatch, reclusterFaces,
    renameFaceGroup, mergeFaceGroups, deleteFaceGroup, findSimilarFaces,
    fetchAccounts, createAccount, switchAccount, fetchGeoFiles,
    parseFileCode, fetchLooseGroups,
    fetchSyncConfigs, createSyncConfig, deleteSyncConfig, startSync,
    getSyncProgress, testSyncConnection, cancelSync, listRemoteFiles,
  }
})