<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { isWebMode } from '@/composables/useTauri'

const emit = defineEmits<{
  (e: 'open-app'): void
}>()

const activePlatform = ref<string | null>(null)
const activeSetupStep = ref<string>('windows')
const isDocker = ref(false)

const platforms = [
  {
    id: 'windows',
    name: 'Windows',
    icon: '🪟',
    color: '#0078D4',
    version: '0.1.0',
    size: '~45 MB',
    format: '.msi installer',
    downloadUrl: '#windows-setup',
    description: 'Native desktop app with full Tauri integration. File system access, encryption, compression, face detection.',
    requirements: ['Windows 10/11', '8 GB RAM recommended', '500 MB disk space'],
    steps: [
      'Download the .msi installer from the latest release',
      'Run the installer and follow the setup wizard',
      'Launch Cybermanju Drive from Start Menu or Desktop',
      'The app will create a local database on first run',
      'Navigate to Sync to connect cloud storage backends',
    ],
  },
  {
    id: 'android',
    name: 'Android',
    icon: '📱',
    color: '#3DDC84',
    version: '0.1.0',
    size: '~25 MB',
    format: '.apk file',
    downloadUrl: '#android-setup',
    description: 'Mobile access to your encrypted file vault. Browse, preview, and manage files on the go.',
    requirements: ['Android 8.0+', '4 GB RAM recommended', '200 MB free storage'],
    steps: [
      'Download the .apk from the latest release',
      'Enable "Install from unknown sources" in Settings > Security',
      'Open the .apk file and tap Install',
      'Grant storage permissions when prompted',
      'Connect to your desktop instance via the Dashboard URL',
    ],
  },
  {
    id: 'docker',
    name: 'Docker',
    icon: '🐳',
    color: '#2496ED',
    version: '0.1.0',
    size: '~120 MB image',
    format: 'Docker image',
    downloadUrl: '#docker-setup',
    description: 'Self-hosted server with web dashboard. Perfect for NAS, ZimaOS, or headless servers.',
    requirements: ['Docker 20.10+', '2 GB RAM minimum', 'Persistent volume for data'],
    steps: [
      'Pull the image: docker pull cybermanju/drive:latest',
      'Run with: docker run -d -p 3456:3456 -v /path/to/data:/data cybermanju/drive:latest',
      'Or use Docker Compose (see below)',
      'Open http://localhost:3456 in your browser',
      'Create an admin account on first visit',
    ],
  },
  {
    id: 'web',
    name: 'Web App',
    icon: '🌐',
    color: '#FF6B2B',
    version: '0.1.0',
    size: 'Browser only',
    format: 'Progressive Web App',
    downloadUrl: '#web-setup',
    description: 'Full-featured web interface. Works in any modern browser. Connects to Docker or desktop instance.',
    requirements: ['Chrome 90+, Firefox 90+, Safari 15+, Edge 90+', 'JavaScript enabled', 'Network access to backend'],
    steps: [
      'This page IS the web app — you are already here!',
      'For full features, connect to a running backend (Docker or Desktop)',
      'Click "Launch App" above to enter the file manager',
      'Navigate to Sync to configure cloud storage backends',
      'Add your accounts for GitHub, Google Drive, or Google Photos',
    ],
  },
]

const dockerComposeYaml = `version: '3.8'
services:
  cybermanju-drive:
    image: cybermanju/drive:latest
    container_name: cybermanju-drive
    ports:
      - "3456:3456"
    volumes:
      - ./cybermanju-data:/DATA
    environment:
      - RUST_LOG=info
      - ADMIN_PASSWORD=changeme
    restart: unless-stopped
    # ZimaOS Casaos metadata
    labels:
      - "casaos.enable=true"
      - "casaos.name=Cybermanju Drive"
      - "casaos.description=Quantum-resistant encrypted file manager"
      - "casaos.icon=https://raw.githubusercontent.com/hautlythird211/Cybermanju-Drive/main/public/tauri.svg"
      - "casaos.port=3456"
      - "casaos.category=file-manager"`

const syncBackends = [
  {
    id: 'github',
    name: 'GitHub',
    icon: '📦',
    color: '#333',
    description: 'Store encrypted files as GitHub repository releases or in a private repo. Free tier: 1 GB.',
    setup: [
      'Create a GitHub Personal Access Token (Settings > Developer settings > Tokens)',
      'Select "repo" scope for private repos, "public_repo" for public',
      'In the app, go to Sync > Add Backend > GitHub',
      'Enter your GitHub username and the PAT token',
      'Enter the repository name (will be created if it doesn\'t exist)',
      'Click "Test Connection" then save',
    ],
  },
  {
    id: 'gdrive',
    name: 'Google Drive',
    icon: '📂',
    color: '#4285F4',
    description: 'Sync to Google Drive with full folder support. Free tier: 15 GB.',
    setup: [
      'Go to Google Cloud Console (console.cloud.google.com)',
      'Create a new project and enable the Drive API',
      'Create OAuth 2.0 credentials (Desktop app type)',
      'Copy the Client ID and Client Secret',
      'In the app, go to Sync > Add Backend > Google Drive',
      'Enter the Client ID and Secret, then authorize in the browser',
      'Select a folder or let the app create one',
    ],
  },
  {
    id: 'gphotos',
    name: 'Google Photos',
    icon: '🖼️',
    color: '#EA4335',
    description: 'Back up photos and videos to Google Photos. Great for camera roll backup.',
    setup: [
      'Same Google Cloud project as Google Drive setup',
      'Enable the Photos Library API in the project',
      'Create OAuth 2.0 credentials (Desktop app type)',
      'In the app, go to Sync > Add Backend > Google Photos',
      'Enter credentials and authorize',
      'Configure upload quality and album naming',
    ],
  },
  {
    id: 'local',
    name: 'Local / NAS',
    icon: '💾',
    color: '#6B7280',
    description: 'Sync to a local directory or mounted network share. Ideal for NAS devices.',
    setup: [
      'Ensure the target directory is accessible (local path or mounted share)',
      'In the app, go to Sync > Add Backend > Local',
      'Enter the directory path (e.g., /mnt/nas/backups)',
      'Set permissions if needed (the app needs read/write access)',
      'Optionally enable compression before sync',
      'Click "Test Connection" then save',
    ],
  },
  {
    id: 'telegram',
    name: 'Telegram',
    icon: '✈️',
    color: '#0088CC',
    description: 'Use Telegram as a file storage backend via a bot. Free tier: 2 GB per file.',
    setup: [
      'Open Telegram and search for @BotFather',
      'Create a new bot with /newbot and get the bot token',
      'Create a channel or group and add the bot as admin',
      'Get the chat ID (forward a message to @userinfobot)',
      'In the app, go to Sync > Add Backend > Telegram',
      'Enter the bot token and chat ID',
      'Click "Test Connection" then save',
    ],
  },
]

const features = [
  { icon: '🔐', title: 'Post-Quantum Encryption', desc: 'ML-KEM-1024 + ML-DSA signatures. NIST FIPS 203/204 compliant. Future-proof against quantum attacks.' },
  { icon: '📦', title: 'Triple Compression', desc: 'LZ4 → ZSTD → Brotli pipeline. Up to 90% size reduction with configurable algorithms.' },
  { icon: '🔍', title: 'Full-Text Search', desc: 'Tantivy-powered BM25 ranking with fuzzy matching, faceted search, and real-time indexing.' },
  { icon: '👤', title: 'Face Detection', desc: 'ONNX Runtime face detection with similarity clustering. Auto-group photos by person.' },
  { icon: '🌐', title: 'Cloud Sync', desc: 'Multi-backend sync: GitHub, Google Drive, Google Photos, Telegram, Local/NAS.' },
  { icon: '👥', title: 'Multi-User', desc: 'JWT authentication with role-based access control. Admin, editor, and viewer roles.' },
  { icon: '🗺️', title: 'GPS Map View', desc: 'MapLibre GL integration. View photos on an interactive map with EXIF GPS data.' },
  { icon: '💻', title: 'Code Intelligence', desc: 'Tree-sitter parsing for 50+ languages. AST navigation and code structure analysis.' },
]

const activePlatformData = computed(() => platforms.find(p => p.id === activePlatform.value))
const activeSetupData = computed(() => syncBackends.find(b => b.id === activeSetupStep.value))

function copyDockerCompose() {
  navigator.clipboard.writeText(dockerComposeYaml)
}
</script>

<template>
  <div class="landing-page">
    <!-- Hero Section -->
    <section class="hero">
      <div class="hero-bg">
        <div class="matrix-overlay"></div>
      </div>
      <div class="hero-content">
        <div class="hero-badge">
          <span class="badge-dot"></span>
          Post-Quantum Secure
        </div>
        <h1 class="hero-title">
          <span class="title-cyber">Cybermanju</span>
          <span class="title-drive">Drive</span>
        </h1>
        <p class="hero-subtitle">
          Quantum-resistant encrypted file manager with AI face detection,
          triple compression, full-text search, and multi-cloud sync.
        </p>
        <div class="hero-actions">
          <button class="btn-primary" @click="emit('open-app')">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>
            Launch App
          </button>
          <a href="#platforms" class="btn-secondary">
            Download for Your Platform
          </a>
        </div>
        <div class="hero-stats">
          <div class="stat">
            <span class="stat-value">ML-KEM-1024</span>
            <span class="stat-label">Encryption</span>
          </div>
          <div class="stat-divider"></div>
          <div class="stat">
            <span class="stat-value">NIST L5</span>
            <span class="stat-label">Security Level</span>
          </div>
          <div class="stat-divider"></div>
          <div class="stat">
            <span class="stat-value">5 Backends</span>
            <span class="stat-label">Cloud Sync</span>
          </div>
          <div class="stat-divider"></div>
          <div class="stat">
            <span class="stat-value">11 Tables</span>
            <span class="stat-label">Database</span>
          </div>
        </div>
      </div>
    </section>

    <!-- Platform Downloads -->
    <section id="platforms" class="section">
      <div class="section-header">
        <h2 class="section-title">Download & Install</h2>
        <p class="section-desc">Choose your platform. All builds are produced by CI from the same source code.</p>
      </div>

      <div class="platform-grid">
        <div
          v-for="platform in platforms"
          :key="platform.id"
          class="platform-card"
          :class="{ active: activePlatform === platform.id }"
          @click="activePlatform = activePlatform === platform.id ? null : platform.id"
        >
          <div class="platform-header">
            <span class="platform-icon">{{ platform.icon }}</span>
            <div class="platform-info">
              <h3 class="platform-name">{{ platform.name }}</h3>
              <span class="platform-version">v{{ platform.version }}</span>
            </div>
            <span class="platform-format">{{ platform.format }}</span>
          </div>
          <p class="platform-desc">{{ platform.description }}</p>
          <div class="platform-meta">
            <span class="meta-item">{{ platform.size }}</span>
          </div>

          <!-- Expanded setup steps -->
          <div v-if="activePlatform === platform.id" class="platform-steps">
            <h4 class="steps-title">Installation Steps</h4>
            <ol class="steps-list">
              <li v-for="(step, i) in platform.steps" :key="i" class="step-item">
                <span class="step-num">{{ i + 1 }}</span>
                <span class="step-text">{{ step }}</span>
              </li>
            </ol>
            <div class="requirements">
              <h5>Requirements</h5>
              <ul>
                <li v-for="req in platform.requirements" :key="req">{{ req }}</li>
              </ul>
            </div>

            <!-- Docker Compose special section -->
            <div v-if="platform.id === 'docker'" class="docker-compose-section">
              <h5>Docker Compose (Recommended)</h5>
              <div class="code-block">
                <div class="code-header">
                  <span>docker-compose.yml</span>
                  <button class="copy-btn" @click.stop="copyDockerCompose()">Copy</button>
                </div>
                <pre><code>{{ dockerComposeYaml }}</code></pre>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Features Grid -->
    <section class="section features-section">
      <div class="section-header">
        <h2 class="section-title">Features</h2>
        <p class="section-desc">Built with Rust, Vue 3, and post-quantum cryptography.</p>
      </div>

      <div class="features-grid">
        <div v-for="feature in features" :key="feature.title" class="feature-card">
          <span class="feature-icon">{{ feature.icon }}</span>
          <h3 class="feature-title">{{ feature.title }}</h3>
          <p class="feature-desc">{{ feature.desc }}</p>
        </div>
      </div>
    </section>

    <!-- Sync Setup Guide -->
    <section id="sync" class="section sync-section">
      <div class="section-header">
        <h2 class="section-title">Cloud Sync Setup</h2>
        <p class="section-desc">Connect your preferred cloud storage backend. All data is encrypted before upload.</p>
      </div>

      <div class="sync-layout">
        <!-- Backend selector -->
        <div class="sync-sidebar">
          <button
            v-for="backend in syncBackends"
            :key="backend.id"
            class="sync-tab"
            :class="{ active: activeSetupStep === backend.id }"
            @click="activeSetupStep = backend.id"
          >
            <span class="tab-icon">{{ backend.icon }}</span>
            <span class="tab-name">{{ backend.name }}</span>
          </button>
        </div>

        <!-- Backend details -->
        <div v-if="activeSetupData" class="sync-detail">
          <div class="sync-detail-header">
            <span class="detail-icon" :style="{ color: activeSetupData.color }">{{ activeSetupData.icon }}</span>
            <div>
              <h3 class="detail-name">{{ activeSetupData.name }}</h3>
              <p class="detail-desc">{{ activeSetupData.description }}</p>
            </div>
          </div>

          <div class="setup-steps">
            <h4>Setup Instructions</h4>
            <ol class="setup-list">
              <li v-for="(step, i) in activeSetupData.setup" :key="i" class="setup-item">
                <span class="setup-num">{{ i + 1 }}</span>
                <span class="setup-text">{{ step }}</span>
              </li>
            </ol>
          </div>
        </div>
      </div>
    </section>

    <!-- Architecture Overview -->
    <section class="section arch-section">
      <div class="section-header">
        <h2 class="section-title">Architecture</h2>
        <p class="section-desc">Three runtime modes. One codebase.</p>
      </div>

      <div class="arch-grid">
        <div class="arch-card">
          <div class="arch-icon">🖥️</div>
          <h3>Tauri Desktop</h3>
          <p>Native desktop app with full system access. File system, encryption, compression, face detection — all local.</p>
          <div class="arch-tech">
            <span>Rust</span>
            <span>Tauri v2</span>
            <span>Vue 3</span>
          </div>
        </div>
        <div class="arch-card">
          <div class="arch-icon">🐳</div>
          <h3>Docker Server</h3>
          <p>Self-hosted HTTP server with web dashboard. Perfect for NAS, ZimaOS, or headless deployments.</p>
          <div class="arch-tech">
            <span>Rust</span>
            <span>redb</span>
            <span>REST API</span>
          </div>
        </div>
        <div class="arch-card">
          <div class="arch-icon">🌐</div>
          <h3>GitHub Pages</h3>
          <p>Static WASM build deployed to GitHub Pages. Connects to any backend instance via REST API.</p>
          <div class="arch-tech">
            <span>Vue 3</span>
            <span>Vite</span>
            <span>TypeScript</span>
          </div>
        </div>
      </div>
    </section>

    <!-- API Reference Quick Links -->
    <section class="section api-section">
      <div class="section-header">
        <h2 class="section-title">API & Documentation</h2>
      </div>

      <div class="api-grid">
        <a href="https://github.com/hautlythird211/Cybermanju-Drive/blob/main/README.md" target="_blank" class="api-card">
          <span class="api-icon">📖</span>
          <span class="api-name">README</span>
          <span class="api-desc">Full project documentation</span>
        </a>
        <a href="https://github.com/hautlythird211/Cybermanju-Drive/blob/main/ARCHITECTURE.md" target="_blank" class="api-card">
          <span class="api-icon">🏗️</span>
          <span class="api-name">Architecture</span>
          <span class="api-desc">System design & data flow</span>
        </a>
        <a href="https://github.com/hautlythird211/Cybermanju-Drive" target="_blank" class="api-card">
          <span class="api-icon">💻</span>
          <span class="api-name">Source Code</span>
          <span class="api-desc">GitHub repository</span>
        </a>
        <a href="#sync" class="api-card">
          <span class="api-icon">🔗</span>
          <span class="api-name">REST API</span>
          <span class="api-desc">20+ endpoints documented</span>
        </a>
      </div>
    </section>

    <!-- Footer -->
    <footer class="landing-footer">
      <div class="footer-content">
        <div class="footer-brand">
          <span class="footer-logo">⬡</span>
          <span>Cybermanju Drive</span>
        </div>
        <p class="footer-tagline">Quantum-resistant. Open source. Built with Rust.</p>
        <div class="footer-links">
          <a href="https://github.com/hautlythird211/Cybermanju-Drive" target="_blank">GitHub</a>
          <a href="https://github.com/hautlythird211/Cybermanju-Drive/blob/main/LICENSE" target="_blank">License</a>
          <a href="#platforms">Downloads</a>
          <a href="#sync">Sync Setup</a>
        </div>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.landing-page {
  min-height: 100vh;
  background: var(--bg-primary, #0a0a0a);
  color: var(--text-primary, #e0e0e0);
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  overflow-y: auto;
}

/* Hero */
.hero {
  position: relative;
  padding: 80px 40px 60px;
  text-align: center;
  overflow: hidden;
}

.hero-bg {
  position: absolute;
  inset: 0;
  background: linear-gradient(135deg, #0a1628 0%, #0d0d0d 50%, #1a0a28 100%);
}

.matrix-overlay {
  position: absolute;
  inset: 0;
  background: repeating-linear-gradient(
    0deg,
    transparent,
    transparent 2px,
    rgba(0, 255, 65, 0.03) 2px,
    rgba(0, 255, 65, 0.03) 4px
  );
}

.hero-content {
  position: relative;
  max-width: 800px;
  margin: 0 auto;
}

.hero-badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 16px;
  border: 1px solid rgba(0, 255, 65, 0.3);
  border-radius: 20px;
  font-size: 12px;
  font-family: monospace;
  color: #00ff41;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin-bottom: 24px;
  background: rgba(0, 255, 65, 0.05);
}

.badge-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #00ff41;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.hero-title {
  font-size: 56px;
  font-weight: 800;
  line-height: 1.1;
  margin: 0 0 20px;
}

.title-cyber {
  background: linear-gradient(135deg, #00ff41, #00d4ff);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.title-drive {
  color: #fff;
  margin-left: 12px;
}

.hero-subtitle {
  font-size: 18px;
  color: #888;
  line-height: 1.6;
  max-width: 600px;
  margin: 0 auto 32px;
}

.hero-actions {
  display: flex;
  gap: 16px;
  justify-content: center;
  margin-bottom: 48px;
}

.btn-primary {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 14px 28px;
  background: linear-gradient(135deg, #00ff41, #00cc33);
  color: #000;
  border: none;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.2s;
  text-decoration: none;
}

.btn-primary:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(0, 255, 65, 0.3);
}

.btn-secondary {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 14px 28px;
  background: transparent;
  color: #fff;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 8px;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  text-decoration: none;
}

.btn-secondary:hover {
  border-color: rgba(255, 255, 255, 0.4);
  background: rgba(255, 255, 255, 0.05);
}

.hero-stats {
  display: flex;
  justify-content: center;
  gap: 32px;
  flex-wrap: wrap;
}

.stat {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.stat-value {
  font-family: monospace;
  font-size: 14px;
  font-weight: 700;
  color: #00ff41;
}

.stat-label {
  font-size: 12px;
  color: #666;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.stat-divider {
  width: 1px;
  height: 32px;
  background: rgba(255, 255, 255, 0.1);
}

/* Sections */
.section {
  padding: 60px 40px;
  max-width: 1200px;
  margin: 0 auto;
}

.section-header {
  text-align: center;
  margin-bottom: 40px;
}

.section-title {
  font-size: 32px;
  font-weight: 700;
  margin: 0 0 8px;
}

.section-desc {
  font-size: 16px;
  color: #888;
  margin: 0;
}

/* Platform Cards */
.platform-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 20px;
}

.platform-card {
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  padding: 24px;
  cursor: pointer;
  transition: all 0.2s;
  background: rgba(255, 255, 255, 0.02);
}

.platform-card:hover {
  border-color: rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.04);
}

.platform-card.active {
  border-color: #00ff41;
  background: rgba(0, 255, 65, 0.05);
}

.platform-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.platform-icon {
  font-size: 32px;
}

.platform-info {
  flex: 1;
}

.platform-name {
  font-size: 18px;
  font-weight: 700;
  margin: 0;
}

.platform-version {
  font-size: 12px;
  color: #666;
  font-family: monospace;
}

.platform-format {
  font-size: 11px;
  color: #00ff41;
  font-family: monospace;
  padding: 2px 8px;
  border: 1px solid rgba(0, 255, 65, 0.3);
  border-radius: 4px;
}

.platform-desc {
  font-size: 14px;
  color: #aaa;
  line-height: 1.5;
  margin: 0 0 12px;
}

.platform-meta {
  display: flex;
  gap: 12px;
}

.meta-item {
  font-size: 12px;
  color: #666;
  font-family: monospace;
}

.platform-steps {
  margin-top: 20px;
  padding-top: 20px;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
}

.steps-title {
  font-size: 14px;
  font-weight: 600;
  color: #00ff41;
  margin: 0 0 12px;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.steps-list {
  list-style: none;
  padding: 0;
  margin: 0 0 16px;
}

.step-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 8px 0;
}

.step-num {
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: rgba(0, 255, 65, 0.1);
  color: #00ff41;
  font-size: 11px;
  font-weight: 700;
  flex-shrink: 0;
}

.step-text {
  font-size: 13px;
  color: #ccc;
  line-height: 1.4;
}

.requirements h5 {
  font-size: 12px;
  color: #666;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin: 0 0 8px;
}

.requirements ul {
  list-style: none;
  padding: 0;
  margin: 0;
}

.requirements li {
  font-size: 12px;
  color: #888;
  padding: 2px 0;
}

.requirements li::before {
  content: '• ';
  color: #00ff41;
}

/* Docker Compose */
.docker-compose-section {
  margin-top: 16px;
}

.docker-compose-section h5 {
  font-size: 12px;
  color: #666;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin: 0 0 8px;
}

.code-block {
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  overflow: hidden;
}

.code-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: rgba(255, 255, 255, 0.05);
  font-size: 12px;
  color: #888;
  font-family: monospace;
}

.copy-btn {
  padding: 2px 8px;
  background: rgba(0, 255, 65, 0.1);
  border: 1px solid rgba(0, 255, 65, 0.3);
  border-radius: 4px;
  color: #00ff41;
  font-size: 11px;
  cursor: pointer;
  font-family: monospace;
}

.copy-btn:hover {
  background: rgba(0, 255, 65, 0.2);
}

.code-block pre {
  margin: 0;
  padding: 12px;
  overflow-x: auto;
}

.code-block code {
  font-family: 'Fira Code', monospace;
  font-size: 12px;
  color: #ccc;
  line-height: 1.5;
}

/* Features */
.features-section {
  background: rgba(255, 255, 255, 0.02);
  border-radius: 16px;
  margin: 0 40px;
  max-width: 1120px;
}

.features-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 20px;
}

.feature-card {
  padding: 24px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 12px;
  background: rgba(0, 0, 0, 0.3);
}

.feature-icon {
  font-size: 28px;
  display: block;
  margin-bottom: 12px;
}

.feature-title {
  font-size: 16px;
  font-weight: 700;
  margin: 0 0 8px;
}

.feature-desc {
  font-size: 13px;
  color: #888;
  line-height: 1.5;
  margin: 0;
}

/* Sync Section */
.sync-layout {
  display: grid;
  grid-template-columns: 200px 1fr;
  gap: 24px;
}

.sync-sidebar {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.sync-tab {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 8px;
  color: #888;
  cursor: pointer;
  transition: all 0.2s;
  text-align: left;
  font-size: 14px;
}

.sync-tab:hover {
  background: rgba(255, 255, 255, 0.05);
  color: #fff;
}

.sync-tab.active {
  background: rgba(0, 255, 65, 0.08);
  border-color: rgba(0, 255, 65, 0.2);
  color: #00ff41;
}

.tab-icon {
  font-size: 18px;
}

.sync-detail {
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  padding: 24px;
  background: rgba(255, 255, 255, 0.02);
}

.sync-detail-header {
  display: flex;
  align-items: flex-start;
  gap: 16px;
  margin-bottom: 24px;
}

.detail-icon {
  font-size: 32px;
}

.detail-name {
  font-size: 20px;
  font-weight: 700;
  margin: 0 0 4px;
}

.detail-desc {
  font-size: 14px;
  color: #888;
  margin: 0;
}

.setup-steps h4 {
  font-size: 14px;
  font-weight: 600;
  color: #00ff41;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin: 0 0 16px;
}

.setup-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.setup-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 10px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.setup-item:last-child {
  border-bottom: none;
}

.setup-num {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: rgba(0, 255, 65, 0.1);
  color: #00ff41;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
}

.setup-text {
  font-size: 14px;
  color: #ccc;
  line-height: 1.5;
}

/* Architecture */
.arch-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 20px;
}

.arch-card {
  padding: 28px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.02);
}

.arch-icon {
  font-size: 36px;
  margin-bottom: 16px;
}

.arch-card h3 {
  font-size: 18px;
  font-weight: 700;
  margin: 0 0 8px;
}

.arch-card p {
  font-size: 14px;
  color: #888;
  line-height: 1.5;
  margin: 0 0 16px;
}

.arch-tech {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.arch-tech span {
  font-size: 11px;
  font-family: monospace;
  padding: 3px 8px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 4px;
  color: #aaa;
}

/* API Grid */
.api-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}

.api-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 24px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  text-decoration: none;
  color: inherit;
  transition: all 0.2s;
}

.api-card:hover {
  border-color: rgba(0, 255, 65, 0.3);
  background: rgba(0, 255, 65, 0.05);
}

.api-icon {
  font-size: 28px;
}

.api-name {
  font-size: 16px;
  font-weight: 700;
}

.api-desc {
  font-size: 12px;
  color: #666;
  text-align: center;
}

/* Footer */
.landing-footer {
  padding: 40px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  text-align: center;
}

.footer-content {
  max-width: 600px;
  margin: 0 auto;
}

.footer-brand {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 18px;
  font-weight: 700;
  margin-bottom: 8px;
}

.footer-logo {
  color: #00ff41;
  font-size: 24px;
}

.footer-tagline {
  font-size: 14px;
  color: #666;
  margin: 0 0 20px;
}

.footer-links {
  display: flex;
  gap: 24px;
  justify-content: center;
}

.footer-links a {
  font-size: 13px;
  color: #888;
  text-decoration: none;
  transition: color 0.2s;
}

.footer-links a:hover {
  color: #00ff41;
}

/* Responsive */
@media (max-width: 768px) {
  .hero { padding: 40px 20px; }
  .hero-title { font-size: 36px; }
  .hero-actions { flex-direction: column; align-items: center; }
  .section { padding: 40px 20px; }
  .features-section { margin: 0 20px; }
  .sync-layout { grid-template-columns: 1fr; }
  .arch-grid { grid-template-columns: 1fr; }
  .hero-stats { gap: 16px; }
  .stat-divider { display: none; }
}
</style>
