// Cybermanju Drive — Pinia Store
import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { invoke } from '@/composables/useTauri'
import { useNotifications } from '@/composables/useNotifications'
import type {
  FileNode, Account, Collection, FaceGroup, LooseGroup,
  SearchResult, EncryptionStatus, EncryptionKeyInfo,
  CompressionStats, ParseResult, GeoMarker,
  ViewMode, PanelType, SidebarSection,
  SyncConfig, SyncProgress, SyncResult, RemoteFile,
  AuthResult, ModuleInfo, TrashItem, AuditEntry, FileVersion, User,
  DashboardStatus,
} from '@/types'
import { MODULE_METADATA } from '@/types'
import { setAuthToken } from '@/composables/useTauri'

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
  const searchPage = ref(0)
  const searchTotalResults = ref(0)
  const searchPageSize = 20
  const geoMarkers = ref<GeoMarker[]>([])

  // ── Encryption State ──────────────────────────────────────
  const encryptionStatus = ref<EncryptionStatus>({ isEncrypted: false })
  const encryptionKeys = ref<EncryptionKeyInfo[]>([])

  // ── Compression State ─────────────────────────────────────
  const compressionStats = ref<CompressionStats | null>(null)

  // ── Trash State ────────────────────────────────────────────
  const trashItems = ref<TrashItem[]>([])
  const showTrashPanel = ref(false)

  // ── Audit State ────────────────────────────────────────────
  const auditLog = ref<AuditEntry[]>([])

  // ── Versions State ─────────────────────────────────────────
  const fileVersions = ref<FileVersion[]>([])

  // ── Dashboard State ─────────────────────────────────────────
  const dashboardStatus = ref<DashboardStatus>({
    running: false,
    port: 3456,
    url: 'http://localhost:3456',
    activeConnections: 0,
  })

  // ── Sync State ────────────────────────────────────────────
  const syncConfigs = ref<SyncConfig[]>([])
  const syncProgress = ref<SyncProgress | null>(null)

  // ── Code Intelligence State ───────────────────────────────
  const parseResult = ref<ParseResult | null>(null)

  // ── Auth State ────────────────────────────────────────────
  const currentUser = ref<AuthResult | null>(null)
  const authToken = ref('')
  const isAuthenticated = computed(() => !!currentUser.value)
  const showLoginPopup = ref(false)

  // ── User Management State ──────────────────────────────────
  const users = ref<User[]>([])

  // ── Transition State ──────────────────────────────────────
  const previousPanel = ref<PanelType | null>(null)
  const isTransitioning = ref(false)

  // ── UI State ──────────────────────────────────────────────
  const searchQuery = ref('')
  const isSearching = ref(false)
  const isLoading = ref(false)
  const lastError = ref<string | null>(null)
  const matrixRainEnabled = ref(true)
  const showEncryptionPanel = ref(false)
  const showCompressionPanel = ref(false)
  const showPermissionsPanel = ref(false)
  const commandPaletteOpen = ref(false)
  const showShortcutsHelp = ref(false)
  const createFolderPromptOpen = ref(false)
  const autoRefreshInterval = ref(0)
  let autoRefreshTimer: ReturnType<typeof setInterval> | null = null
  const selectedFileIds = ref<string[]>([])
  const isMultiSelect = ref(false)

  // ── Module Helpers ─────────────────────────────────────────
  const currentModule = computed<ModuleInfo>(() =>
    MODULE_METADATA[(showEncryptionPanel.value ? 'encryption' : showCompressionPanel.value ? 'compression' : currentPanel.value) as PanelType] || MODULE_METADATA.files
  )

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

  const { notify } = useNotifications()

  function notifyError(msg: string, error: unknown) {
    const detail = error instanceof Error ? error.message : String(error)
    lastError.value = `${msg}: ${detail}`
    notify('error', lastError.value)
    console.error(lastError.value)
  }

  function notifySuccess(msg: string) {
    notify('success', msg)
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
      startAutoRefresh()
    } finally {
      isLoading.value = false
    }
  }

  function startAutoRefresh() {
    if (autoRefreshTimer) clearInterval(autoRefreshTimer)
    if (autoRefreshInterval.value > 0) {
      autoRefreshTimer = setInterval(async () => {
        await Promise.allSettled([
          fetchFiles(),
          fetchAccounts(),
          fetchCollections(),
          fetchFaceGroups(),
          fetchEncryptionStatus(),
          fetchSyncConfigs(),
          fetchTrashItems(),
        ])
      }, autoRefreshInterval.value * 1000)
    }
  }

  watch(autoRefreshInterval, () => startAutoRefresh())

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
      notifyError('Failed to fetch files', e)
    } finally {
      isLoading.value = false
    }
  }

  async function getFile(fileId: string) {
    try {
      return await invoke<FileNode>('get_file', { fileId })
    } catch (e) {
      notifyError('Failed to get file', e)
      return null
    }
  }

  async function createFolder(name: string, parentId: string) {
    try {
      await invoke('create_folder', { name, parentId })
      await fetchFiles()
    } catch (e) {
      notifyError('Failed to create folder', e)
    }
  }

  async function deleteFile(fileId: string) {
    try {
      await invoke('delete_file', { fileId })
      files.value = files.value.filter(f => f.id !== fileId)
      if (selectedFileId.value === fileId) selectedFileId.value = null
      selectedFileIds.value = selectedFileIds.value.filter(id => id !== fileId)
      notifySuccess(`File deleted`)
    } catch (e) {
      notifyError('Failed to delete file', e)
    }
  }

  async function renameFile(fileId: string, newName: string) {
    try {
      await invoke('rename_file', { fileId, newName })
      await fetchFiles()
      notifySuccess('File renamed')
    } catch (e) {
      notifyError('Failed to rename file', e)
    }
  }

  async function duplicateFileContext(fileId: string) {
    try {
      await invoke('duplicate_file_context', { fileId })
      await fetchFiles()
      notifySuccess('File context duplicated')
    } catch (e) {
      notifyError('Failed to duplicate file', e)
    }
  }

  // ── Actions: Search ───────────────────────────────────────
  async function searchFiles(query: string, page?: number) {
    if (!query.trim()) { searchResults.value = []; searchTotalResults.value = 0; return }
    isSearching.value = true
    clearError()
    try {
      const pg = page ?? 0
      const limit = searchPageSize
      const result = await invoke<{ results: SearchResult[]; total: number }>('search_files_paginated', { query, limit, offset: pg * limit })
      if (pg === 0) {
        searchResults.value = result.results
      } else {
        searchResults.value = [...searchResults.value, ...result.results]
      }
      searchTotalResults.value = result.total
      searchPage.value = pg
    } catch {
      try {
        const result = await invoke<SearchResult[]>('search_files', { query, limit: 50 })
        if (page && page > 0) {
          searchResults.value = [...searchResults.value, ...result]
        } else {
          searchResults.value = result
        }
        searchTotalResults.value = result.length
        searchPage.value = page ?? 0
      } catch (e) {
        notifyError('Search failed', e)
      }
    } finally {
      isSearching.value = false
    }
  }

  function loadMoreSearchResults() {
    if (searchQuery.value.trim()) {
      searchFiles(searchQuery.value, searchPage.value + 1)
    }
  }

  // ── Actions: Encryption ───────────────────────────────────
  async function fetchEncryptionStatus() {
    try {
      encryptionStatus.value = await invoke<EncryptionStatus>('get_encryption_status')
    } catch (e) {
      notifyError('Failed to get encryption status', e)
    }
  }

  async function generateKeypair(algorithm: string) {
    try {
      await invoke('generate_keypair', { algorithm })
      await listKeys()
      await fetchEncryptionStatus()
    } catch (e) {
      notifyError('Failed to generate keypair', e)
    }
  }

  async function listKeys() {
    try {
      encryptionKeys.value = await invoke<EncryptionKeyInfo[]>('list_keys')
    } catch (e) {
      notifyError('Failed to list keys', e)
    }
  }

  async function encryptFile(fileId: string, algorithm: string) {
    try {
      await invoke('encrypt_file', { fileId, algorithm })
      await fetchFiles()
      await fetchEncryptionStatus()
    } catch (e) {
      notifyError('Encryption failed', e)
    }
  }

  async function decryptFile(fileId: string) {
    try {
      await invoke('decrypt_file', { fileId })
      await fetchFiles()
      await fetchEncryptionStatus()
    } catch (e) {
      notifyError('Decryption failed', e)
    }
  }

  // ── Actions: Compression ──────────────────────────────────
  async function compressFile(fileId: string, layer: string) {
    try {
      const stats = await invoke<CompressionStats>('compress_file', { fileId, layer })
      compressionStats.value = stats
      await fetchFiles()
    } catch (e) {
      notifyError('Compression failed', e)
    }
  }

  async function decompressFile(fileId: string) {
    try {
      const stats = await invoke<CompressionStats>('decompress_file', { fileId })
      compressionStats.value = stats
      await fetchFiles()
    } catch (e) {
      notifyError('Decompression failed', e)
    }
  }

  // ── Actions: Collections ──────────────────────────────────
  async function fetchCollections() {
    try {
      collections.value = await invoke<Collection[]>('list_collections')
    } catch (e) {
      notifyError('Failed to fetch collections', e)
    }
  }

  async function createCollection(name: string, type: string, color: string, description?: string) {
    try {
      await invoke('create_collection', { name, collectionType: type, color, description })
      await fetchCollections()
    } catch (e) {
      notifyError('Failed to create collection', e)
    }
  }

  async function addToCollection(collectionId: string, fileId: string, note?: string) {
    try {
      await invoke('add_to_collection', { collectionId, fileId, note })
      await fetchCollections()
    } catch (e) {
      notifyError('Failed to add to collection', e)
    }
  }

  async function removeFromCollection(collectionId: string, fileId: string) {
    try {
      await invoke('remove_from_collection', { collectionId, fileId })
      await fetchCollections()
    } catch (e) {
      notifyError('Failed to remove from collection', e)
    }
  }

  // ── Actions: Face Groups ──────────────────────────────────
  async function fetchFaceGroups() {
    try {
      faceGroups.value = await invoke<FaceGroup[]>('list_face_groups')
    } catch (e) {
      notifyError('Failed to fetch face groups', e)
    }
  }

  async function detectFaces(fileId: string) {
    try {
      await invoke('detect_faces', { fileId })
      await fetchFaceGroups()
    } catch (e) {
      notifyError('Face detection failed', e)
    }
  }

  async function detectFacesBatch() {
    try {
      const result = await invoke<{ clustersCreated: number; totalFaces: number; noiseFaces: number; avgCohesion: number; strategyUsed: string }>('detect_faces_batch_cmd')
      await fetchFaceGroups()
      return result
    } catch (e) {
      notifyError('Batch face detection failed', e)
      return null
    }
  }

  async function reclusterFaces(strategy?: string) {
    try {
      const result = await invoke<{ clustersCreated: number; totalFaces: number; noiseFaces: number; avgCohesion: number; strategyUsed: string }>('recluster_faces', { strategy })
      await fetchFaceGroups()
      return result
    } catch (e) {
      notifyError('Re-clustering failed', e)
      return null
    }
  }

  async function renameFaceGroup(groupId: string, newName: string) {
    try {
      await invoke('rename_face_group', { groupId, newName })
      await fetchFaceGroups()
    } catch (e) {
      notifyError('Failed to rename face group', e)
    }
  }

  async function mergeFaceGroups(sourceGroupId: string, targetGroupId: string) {
    try {
      await invoke('merge_face_groups', { sourceGroupId, targetGroupId })
      await fetchFaceGroups()
    } catch (e) {
      notifyError('Failed to merge face groups', e)
    }
  }

  async function deleteFaceGroup(groupId: string) {
    try {
      await invoke('delete_face_group', { groupId })
      await fetchFaceGroups()
    } catch (e) {
      notifyError('Failed to delete face group', e)
    }
  }

  async function findSimilarFaces(groupId: string, threshold?: number) {
    try {
      return await invoke<FaceGroup[]>('find_similar_faces', { groupId, threshold })
    } catch (e) {
      notifyError('Failed to find similar faces', e)
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
      notifyError('Failed to fetch accounts', e)
    }
  }

  async function createAccount(name: string, type: string, path?: string, color?: string) {
    try {
      await invoke('create_account', { name, accountType: type, path, color })
      await fetchAccounts()
    } catch (e) {
      notifyError('Failed to create account', e)
    }
  }

  async function switchAccount(accountId: string) {
    try {
      await invoke('switch_account', { accountId })
      activeAccountId.value = accountId
      await fetchAccounts()
    } catch (e) {
      notifyError('Failed to switch account', e)
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
      notifyError('Failed to fetch geo files', e)
    }
  }

  // ── Actions: Tree-sitter ──────────────────────────────────
  async function parseFileCode(filePath: string) {
    try {
      parseResult.value = await invoke<ParseResult>('parse_file', { filePath })
    } catch (e) {
      notifyError('Parse failed', e)
    }
  }

  // ── Actions: Loose Groups ─────────────────────────────────
  async function fetchLooseGroups() {
    try {
      looseGroups.value = await invoke<LooseGroup[]>('list_loose_groups')
    } catch (e) {
      notifyError('Failed to fetch loose groups', e)
    }
  }

  // ── Actions: Sync ──────────────────────────────────────
  async function fetchSyncConfigs() {
    try {
      syncConfigs.value = await invoke<SyncConfig[]>('list_sync_configs')
    } catch (e) {
      notifyError('Failed to fetch sync configs', e)
    }
  }

  async function createSyncConfig(config: Omit<SyncConfig, 'id' | 'createdAt' | 'updatedAt'>) {
    try {
      await invoke('create_sync_config', { config })
      await fetchSyncConfigs()
    } catch (e) {
      notifyError('Failed to create sync config', e)
    }
  }

  async function deleteSyncConfig(configId: string) {
    try {
      await invoke('delete_sync_config', { configId })
      await fetchSyncConfigs()
    } catch (e) {
      notifyError('Failed to delete sync config', e)
    }
  }

  async function startSync(configId: string, fileIds: string[]) {
    try {
      await invoke('start_sync', { configId, fileIds })
      await pollSyncProgress()
    } catch (e) {
      notifyError('Failed to start sync', e)
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
      notifyError('Failed to get sync progress', e)
    }
  }

  async function getSyncProgress() {
    try {
      syncProgress.value = await invoke<SyncProgress>('get_sync_progress')
    } catch (e) {
      notifyError('Failed to get sync progress', e)
    }
  }

  async function testSyncConnection(config: SyncConfig) {
    try {
      return await invoke<boolean>('test_sync_connection', { config })
    } catch (e) {
      notifyError('Sync connection test failed', e)
      return false
    }
  }

  async function cancelSync() {
    try {
      await invoke('cancel_sync')
      syncProgress.value = null
    } catch (e) {
      notifyError('Failed to cancel sync', e)
    }
  }

  async function listRemoteFiles(config: SyncConfig, prefix: string) {
    try {
      return await invoke<RemoteFile[]>('list_remote_files', { config, prefix })
    } catch (e) {
      notifyError('Failed to list remote files', e)
      return []
    }
  }

  // ── Actions: Trash ─────────────────────────────────────────
  async function fetchTrashItems() {
    try {
      trashItems.value = await invoke<TrashItem[]>('list_trash')
    } catch (e) {
      notifyError('Failed to fetch trash', e)
    }
  }

  async function restoreTrashItem(fileId: string) {
    try {
      await invoke('restore_from_trash', { fileId })
      await fetchTrashItems()
      await fetchFiles()
      notifySuccess('File restored from trash')
    } catch (e) {
      notifyError('Failed to restore file', e)
    }
  }

  async function emptyTrash() {
    try {
      const count = await invoke<number>('empty_trash')
      trashItems.value = []
      notifySuccess(`Permanently deleted ${count} items`)
    } catch (e) {
      notifyError('Failed to empty trash', e)
    }
  }

  async function deleteFromTrash(fileId: string) {
    try {
      await invoke('delete_from_trash', { fileId })
      await fetchTrashItems()
      notifySuccess('File permanently deleted')
    } catch (e) {
      notifyError('Failed to delete from trash', e)
    }
  }

  // ── Actions: Audit Log ─────────────────────────────────────
  async function fetchAuditLog(limit?: number, entityType?: string) {
    try {
      auditLog.value = await invoke<AuditEntry[]>('get_audit_log', { limit, entityType })
    } catch (e) {
      notifyError('Failed to fetch audit log', e)
    }
  }

  // ── Actions: File Versions ─────────────────────────────────
  async function fetchFileVersions(fileId: string) {
    try {
      fileVersions.value = await invoke<FileVersion[]>('list_file_versions', { fileId })
    } catch (e) {
      notifyError('Failed to fetch file versions', e)
    }
  }

  async function createVersion(fileId: string) {
    try {
      await invoke('create_file_version', { fileId })
      await fetchFileVersions(fileId)
      notifySuccess('Version snapshot created')
    } catch (e) {
      notifyError('Failed to create version', e)
    }
  }

  async function revertToVersion(fileId: string, versionId: string) {
    try {
      await invoke('revert_file_version', { fileId, versionId })
      await fetchFileVersions(fileId)
      await fetchFiles()
      notifySuccess('File reverted to version')
    } catch (e) {
      notifyError('Failed to revert file version', e)
    }
  }

  async function snapshotAllVersions() {
    try {
      const count = await invoke<number>('snapshot_all_versions')
      notifySuccess(`Snapshotted ${count} files`)
    } catch (e) {
      notifyError('Failed to snapshot all versions', e)
    }
  }

  // ── Actions: Dashboard Status ───────────────────────────────
  async function fetchDashboardStatus() {
    try {
      dashboardStatus.value = await invoke<DashboardStatus>('dashboard_status')
    } catch (e) {
      notifyError('Failed to fetch dashboard status', e)
    }
  }

  async function startDashboard() {
    try {
      const result = await invoke<DashboardStatus>('start_dashboard')
      dashboardStatus.value = result
      notifySuccess('Dashboard started')
    } catch (e) {
      notifyError('Failed to start dashboard', e)
    }
  }

  async function stopDashboard() {
    try {
      await invoke<boolean>('stop_dashboard')
      dashboardStatus.value = { running: false, port: 3456, url: 'http://localhost:3456', activeConnections: 0 }
      notifySuccess('Dashboard stopped')
    } catch (e) {
      notifyError('Failed to stop dashboard', e)
    }
  }

  // ── Actions: User Management ───────────────────────────────
  async function fetchUsers() {
    try {
      users.value = await invoke<User[]>('list_users')
    } catch (e) {
      notifyError('Failed to fetch users', e)
    }
  }

  async function createUser(username: string, password: string, role: string) {
    try {
      await invoke('create_user', { username, password, role })
      await fetchUsers()
      notifySuccess(`User '${username}' created`)
    } catch (e) {
      notifyError('Failed to create user', e)
    }
  }

  async function deleteUser(userId: string) {
    try {
      await invoke('delete_user', { userId })
      await fetchUsers()
      notifySuccess('User deleted')
    } catch (e) {
      notifyError('Failed to delete user', e)
    }
  }

  async function updateUserRole(userId: string, role: string) {
    try {
      await invoke('update_user_role', { userId, role })
      await fetchUsers()
      notifySuccess('User role updated')
    } catch (e) {
      notifyError('Failed to update user role', e)
    }
  }

  // ── Actions: Batch Operations ──────────────────────────────
  async function batchDeleteFiles(fileIds: string[]) {
    try {
      const count = await invoke<number>('batch_delete', { fileIds })
      await fetchFiles()
      notifySuccess(`Batch deleted ${count} files`)
    } catch (e) {
      notifyError('Batch delete failed', e)
    }
  }

  async function batchEncryptFiles(fileIds: string[], algorithm: string) {
    try {
      const count = await invoke<number>('batch_encrypt', { fileIds, algorithm })
      await fetchFiles()
      notifySuccess(`Batch encrypted ${count} files`)
    } catch (e) {
      notifyError('Batch encrypt failed', e)
    }
  }

  async function batchCompressFiles(fileIds: string[], layer: string) {
    try {
      const count = await invoke<number>('batch_compress', { fileIds, layer })
      await fetchFiles()
      notifySuccess(`Batch compressed ${count} files`)
    } catch (e) {
      notifyError('Batch compress failed', e)
    }
  }

  // ── Actions: Share Links ────────────────────────────────────
  const shareLinks = ref<import('@/types').ShareLink[]>([])

  async function generateShareLink(fileId: string, expiresInHours?: number) {
    try {
      const result = await invoke<import('@/types').ShareLink>('generate_share_link', { fileId, expiresInHours })
      await fetchShareLinks()
      notifySuccess('Share link generated')
      return result
    } catch (e) {
      notifyError('Failed to generate share link', e)
      return null
    }
  }

  async function fetchShareLinks() {
    try {
      shareLinks.value = await invoke<import('@/types').ShareLink[]>('list_share_links')
    } catch (e) {
      notifyError('Failed to fetch share links', e)
    }
  }

  // ── Actions: URL Import ─────────────────────────────────────
  async function importFromUrl(url: string, parentPath: string) {
    try {
      const result = await invoke<FileNode>('import_from_url', { url, parentPath })
      await fetchFiles()
      notifySuccess('File imported from URL')
      return result
    } catch (e) {
      notifyError('Failed to import from URL', e)
      return null
    }
  }

  // ── Actions: Parent Index Rebuild ──────────────────────────
  async function rebuildParentIndex() {
    try {
      const count = await invoke<number>('rebuild_parent_index')
      await fetchFiles()
      notifySuccess(`Rebuilt parent index for ${count} files`)
    } catch (e) {
      notifyError('Failed to rebuild parent index', e)
    }
  }

  return {
    // State
    currentPath, currentPanel, viewMode, selectedFileId, sidebarSection, sidebarCollapsed,
    files, accounts, activeAccountId, collections, faceGroups, looseGroups,
    searchResults, geoMarkers, encryptionStatus, encryptionKeys,
    compressionStats, parseResult, syncConfigs, syncProgress,
    trashItems, showTrashPanel, auditLog, fileVersions, dashboardStatus, shareLinks,
    searchQuery, searchTotalResults, isSearching, isLoading, lastError, matrixRainEnabled,
    showEncryptionPanel, showCompressionPanel, showPermissionsPanel, commandPaletteOpen,
    showShortcutsHelp, createFolderPromptOpen, showLoginPopup,
    selectedFileIds, isMultiSelect, users, autoRefreshInterval,
    // Computed
    selectedFile, activeAccount, encryptedFiles, compressedFiles,
    starredFiles, folders, currentFolderFiles,
    // Actions
    initialize, selectFile, toggleStar, clearError,
    fetchFiles, getFile, createFolder, deleteFile, renameFile, duplicateFileContext,
    searchFiles, loadMoreSearchResults, fetchEncryptionStatus, generateKeypair, listKeys, encryptFile, decryptFile,
    compressFile, decompressFile, fetchCollections, createCollection, addToCollection, removeFromCollection,
    fetchFaceGroups, detectFaces, detectFacesBatch, reclusterFaces,
    renameFaceGroup, mergeFaceGroups, deleteFaceGroup, findSimilarFaces,
    fetchAccounts, createAccount, switchAccount, fetchGeoFiles,
    parseFileCode, fetchLooseGroups,
    fetchSyncConfigs, createSyncConfig, deleteSyncConfig, startSync,
    getSyncProgress, testSyncConnection, cancelSync, listRemoteFiles,
    // User Management
    fetchUsers, createUser, deleteUser, updateUserRole,
    // Trash
    fetchTrashItems, restoreTrashItem, emptyTrash, deleteFromTrash,
    // Audit
    fetchAuditLog,
    // Versions
    fetchFileVersions, createVersion, revertToVersion, snapshotAllVersions,
    // Batch
    batchDeleteFiles, batchEncryptFiles, batchCompressFiles,
    // Dashboard
    fetchDashboardStatus, startDashboard, stopDashboard,
    // Share Links
    generateShareLink, fetchShareLinks,
    // URL Import
    importFromUrl,
    // Utility
    rebuildParentIndex,
    notifySuccess,
  }
})