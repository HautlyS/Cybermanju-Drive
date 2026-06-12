<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { isWebMode } from '@/composables/useTauri'

const emit = defineEmits<{ (e: 'open-app'): void }>()

const activePlatform = ref<string | null>(null)
const activeSetupStep = ref<string>('windows')

const platforms = [
  { id: 'windows', name: 'WINDOWS', icon: '[W]', version: '0.1.0', size: '~45 MB', format: '.MSI', description: 'NATIVE DESKTOP APP WITH FULL TAURI INTEGRATION.', requirements: ['WINDOWS 10/11', '8 GB RAM', '500 MB DISK'], steps: ['DOWNLOAD .MSI INSTALLER', 'RUN INSTALLER', 'LAUNCH APP'] },
  { id: 'android', name: 'ANDROID', icon: '[A]', version: '0.1.0', size: '~25 MB', format: '.APK', description: 'MOBILE ACCESS TO ENCRYPTED FILE VAULT.', requirements: ['ANDROID 8.0+', '4 GB RAM', '200 MB FREE'], steps: ['DOWNLOAD .APK', 'ENABLE UNKNOWN SOURCES', 'INSTALL APK'] },
  { id: 'docker', name: 'DOCKER', icon: '[D]', version: '0.1.0', size: '~120 MB', format: 'IMAGE', description: 'SELF-HOSTED SERVER WITH WEB DASHBOARD.', requirements: ['DOCKER 20.10+', '2 GB RAM', 'PERSISTENT VOLUME'], steps: ['PULL IMAGE', 'RUN CONTAINER', 'OPEN HTTP://LOCALHOST:3456'] },
  { id: 'web', name: 'WEB APP', icon: '[W]', version: '0.1.0', size: 'BROWSER', format: 'PWA', description: 'FULL-FEATURED WEB INTERFACE.', requirements: ['CHROME 90+', 'FIREFOX 90+', 'JAVASCRIPT ENABLED'], steps: ['THIS PAGE IS THE APP', 'CLICK LAUNCH APP BELOW', 'NAVIGATE FILE MANAGER'] },
]

const features = [
  { icon: '[#]', title: 'POST-QUANTUM ENCRYPTION', desc: 'ML-KEM-1024 + ML-DSA SIGNATURES. NIST FIPS 203/204.' },
  { icon: '[$]', title: 'TRIPLE COMPRESSION', desc: 'LZ4 - ZSTD - BROTLI PIPELINE. UP TO 90% REDUCTION.' },
  { icon: '[@]', title: 'FULL-TEXT SEARCH', desc: 'TANTIVY BM25 RANKING WITH FUZZY MATCHING.' },
  { icon: '[+]', title: 'FACE DETECTION', desc: 'ONNX RUNTIME FACE DETECTION WITH CLUSTERING.' },
  { icon: '[~]', title: 'CLOUD SYNC', desc: 'MULTI-BACKEND SYNC: GITHUB, GDRIVE, TELEGRAM.' },
  { icon: '[!]', title: 'MULTI-USER', desc: 'JWT AUTH WITH ROLE-BASED ACCESS CONTROL.' },
  { icon: '[@]', title: 'GPS MAP VIEW', desc: 'MAPLIBRE GL INTEGRATION. PHOTOS ON MAP.' },
  { icon: '[T]', title: 'CODE INTELLIGENCE', desc: 'TREE-SITTER PARSING FOR 50+ LANGUAGES.' },
]
</script>

<template>
  <div class="landing-page">
    <section class="hero">
      <div class="hero-content">
        <h1 class="hero-title">
          CYBERMANJU<span class="title-sub">DRIVE</span>
        </h1>
        <p class="hero-subtitle">
          QUANTUM-RESISTANT ENCRYPTED FILE MANAGER WITH AI FACE DETECTION,<br/>
          TRIPLE COMPRESSION, FULL-TEXT SEARCH, AND MULTI-CLOUD SYNC.
        </p>
        <div class="hero-actions">
          <button class="btn-primary" @click="emit('open-app')">[LAUNCH APP]</button>
          <a href="#platforms" class="btn-secondary">[DOWNLOAD]</a>
        </div>
        <div class="hero-stats">
          <div class="stat"><span class="stat-value">ML-KEM-1024</span><span class="stat-label">ENCRYPTION</span></div>
          <div class="stat-divider" />
          <div class="stat"><span class="stat-value">NIST L5</span><span class="stat-label">SECURITY</span></div>
          <div class="stat-divider" />
          <div class="stat"><span class="stat-value">5 BACKENDS</span><span class="stat-label">CLOUD SYNC</span></div>
          <div class="stat-divider" />
          <div class="stat"><span class="stat-value">11 TABLES</span><span class="stat-label">DATABASE</span></div>
        </div>
      </div>
    </section>

    <section id="platforms" class="section">
      <h2 class="section-title">DOWNLOAD & INSTALL</h2>
      <div class="platform-grid">
        <div v-for="platform in platforms" :key="platform.id" class="platform-card" :class="{ active: activePlatform === platform.id }" @click="activePlatform = activePlatform === platform.id ? null : platform.id">
          <div class="platform-header">
            <span class="platform-icon">{{ platform.icon }}</span>
            <div class="platform-info">
              <h3 class="platform-name">{{ platform.name }}</h3>
              <span class="platform-version">V{{ platform.version }}</span>
            </div>
            <span class="platform-format">{{ platform.format }}</span>
          </div>
          <p class="platform-desc">{{ platform.description }}</p>
          <div v-if="activePlatform === platform.id" class="platform-steps">
            <div v-for="(step, i) in platform.steps" :key="i" class="step-item">
              <span class="step-num">{{ i + 1 }}</span>
              <span class="step-text">{{ step }}</span>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="section features-section">
      <h2 class="section-title">FEATURES</h2>
      <div class="features-grid">
        <div v-for="feature in features" :key="feature.title" class="feature-card">
          <span class="feature-icon">{{ feature.icon }}</span>
          <h3 class="feature-title">{{ feature.title }}</h3>
          <p class="feature-desc">{{ feature.desc }}</p>
        </div>
      </div>
    </section>

    <section class="section arch-section">
      <h2 class="section-title">ARCHITECTURE</h2>
      <div class="arch-grid">
        <div class="arch-card">
          <div class="arch-icon">[D]</div>
          <h3>TAURI DESKTOP</h3>
          <p>NATIVE DESKTOP APP WITH FULL SYSTEM ACCESS.</p>
          <div class="arch-tech"><span>RUST</span><span>TAURI V2</span><span>VUE 3</span></div>
        </div>
        <div class="arch-card">
          <div class="arch-icon">[C]</div>
          <h3>DOCKER SERVER</h3>
          <p>SELF-HOSTED HTTP SERVER WITH WEB DASHBOARD.</p>
          <div class="arch-tech"><span>RUST</span><span>REDB</span><span>REST API</span></div>
        </div>
        <div class="arch-card">
          <div class="arch-icon">[W]</div>
          <h3>WEB APP</h3>
          <p>STATIC BUILD DEPLOYED TO GITHUB PAGES.</p>
          <div class="arch-tech"><span>VUE 3</span><span>VITE</span><span>TS</span></div>
        </div>
      </div>
    </section>

    <section class="section api-section">
      <h2 class="section-title">API & DOCS</h2>
      <div class="api-grid">
        <a href="https://github.com/hautlythird211/Cybermanju-Drive/blob/main/README.md" target="_blank" class="api-card">[DOC] README</a>
        <a href="https://github.com/hautlythird211/Cybermanju-Drive/blob/main/ARCHITECTURE.md" target="_blank" class="api-card">[ARC] ARCHITECTURE</a>
        <a href="https://github.com/hautlythird211/Cybermanju-Drive" target="_blank" class="api-card">[GH] SOURCE CODE</a>
      </div>
    </section>

    <footer class="landing-footer">
      <div class="footer-content">
        <p>CYBERMANJU DRIVE - QUANTUM-RESISTANT. OPEN SOURCE. BUILT WITH RUST.</p>
        <div class="footer-links">
          <a href="https://github.com/hautlythird211/Cybermanju-Drive" target="_blank">GITHUB</a>
          <a href="#platforms">DOWNLOADS</a>
        </div>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.landing-page {
  min-height: 100vh;
  background: #000;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  overflow-y: auto;
}

.hero {
  padding: 60px 30px 40px;
  text-align: center;
  border-bottom: 2px solid #FFFFFF;
}

.hero-content {
  max-width: 700px;
  margin: 0 auto;
}

.hero-title {
  font-size: 42px;
  font-weight: 800;
  letter-spacing: 3px;
  margin: 0 0 16px;
  color: #FFFFFF;
}

.title-sub {
  margin-left: 8px;
  opacity: 0.5;
}

.hero-subtitle {
  font-size: 13px;
  color: rgba(255,255,255,0.7);
  line-height: 1.6;
  margin: 0 auto 28px;
  max-width: 550px;
}

.hero-actions {
  display: flex;
  gap: 12px;
  justify-content: center;
  margin-bottom: 36px;
}

.btn-primary {
  padding: 10px 24px;
  background: #FFFFFF;
  color: #000;
  border: 2px solid #000;
  font-family: 'Courier New', monospace;
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
  text-decoration: none;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.btn-primary:hover {
  background: #000;
  color: #FFFFFF;
  border-color: #FFFFFF;
}

.btn-secondary {
  padding: 10px 24px;
  background: #000;
  color: #FFFFFF;
  border: 2px solid #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
  text-decoration: none;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.btn-secondary:hover {
  background: #FFFFFF;
  color: #000;
}

.hero-stats {
  display: flex;
  justify-content: center;
  gap: 24px;
  flex-wrap: wrap;
}

.stat {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.stat-value {
  font-size: 12px;
  font-weight: 700;
  color: #FFFFFF;
}

.stat-label {
  font-size: 10px;
  color: rgba(255,255,255,0.5);
  letter-spacing: 1px;
}

.stat-divider {
  width: 1px;
  height: 28px;
  background: rgba(255,255,255,0.3);
}

.section {
  padding: 40px 30px;
  max-width: 1000px;
  margin: 0 auto;
  border-bottom: 2px solid rgba(255,255,255,0.1);
}

.section-title {
  font-size: 18px;
  font-weight: 800;
  letter-spacing: 2px;
  margin: 0 0 24px;
  text-align: center;
}

.platform-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 16px;
}

.platform-card {
  border: 2px solid #FFFFFF;
  padding: 16px;
  cursor: pointer;
  background: #000;
}

.platform-card:hover,
.platform-card.active {
  background: #FFFFFF;
  color: #000;
}

.platform-card:hover .platform-desc,
.platform-card:hover .platform-version,
.platform-card:hover .platform-format,
.platform-card.active .platform-desc,
.platform-card.active .platform-version,
.platform-card.active .platform-format {
  color: #000;
}

.platform-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}

.platform-icon {
  font-size: 20px;
}

.platform-info {
  flex: 1;
}

.platform-name {
  font-size: 14px;
  font-weight: 700;
  margin: 0;
}

.platform-version {
  font-size: 10px;
  color: rgba(255,255,255,0.5);
}

.platform-format {
  font-size: 9px;
  border: 1px solid #FFFFFF;
  padding: 1px 6px;
}

.platform-desc {
  font-size: 11px;
  color: rgba(255,255,255,0.7);
  line-height: 1.4;
  margin: 0;
}

.platform-steps {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 2px solid rgba(255,255,255,0.3);
}

.step-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 4px 0;
}

.step-num {
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px solid #000;
  font-size: 9px;
  font-weight: 700;
  flex-shrink: 0;
  background: #FFFFFF;
  color: #000;
}

.platform-card:hover .step-num,
.platform-card.active .step-num {
  background: #000;
  color: #FFFFFF;
  border-color: #FFFFFF;
}

.step-text {
  font-size: 11px;
  line-height: 1.3;
}

.features-section {
  max-width: 880px;
}

.features-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}

.feature-card {
  padding: 16px;
  border: 2px solid #FFFFFF;
  background: #000;
}

.feature-icon {
  font-size: 18px;
  display: block;
  margin-bottom: 8px;
}

.feature-title {
  font-size: 12px;
  font-weight: 700;
  margin: 0 0 6px;
}

.feature-desc {
  font-size: 10px;
  color: rgba(255,255,255,0.6);
  line-height: 1.4;
  margin: 0;
}

.arch-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
}

.arch-card {
  padding: 20px;
  border: 2px solid #FFFFFF;
  background: #000;
}

.arch-icon {
  font-size: 22px;
  margin-bottom: 10px;
}

.arch-card h3 {
  font-size: 13px;
  font-weight: 700;
  margin: 0 0 6px;
}

.arch-card p {
  font-size: 11px;
  color: rgba(255,255,255,0.6);
  line-height: 1.4;
  margin: 0 0 12px;
}

.arch-tech {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.arch-tech span {
  font-size: 9px;
  border: 1px solid #FFFFFF;
  padding: 1px 6px;
  color: rgba(255,255,255,0.7);
}

.api-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
}

.api-card {
  display: block;
  padding: 14px;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  text-decoration: none;
  text-align: center;
  font-size: 12px;
  font-weight: 700;
}

.api-card:hover {
  background: #FFFFFF;
  color: #000;
}

.landing-footer {
  padding: 30px;
  text-align: center;
}

.footer-content p {
  font-size: 11px;
  color: rgba(255,255,255,0.5);
  margin: 0 0 12px;
}

.footer-links {
  display: flex;
  gap: 20px;
  justify-content: center;
}

.footer-links a {
  font-size: 11px;
  color: rgba(255,255,255,0.7);
  text-decoration: underline;
}
</style>
