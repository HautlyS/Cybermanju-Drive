<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'

const emit = defineEmits<{ (e: 'open-app'): void }>()

const bootLines = [
  'CYBERMANJU DRIVE v0.0.1',
  'Copyright (c) 2026 Cybermanju Systems',
  'All Rights Reserved.',
  '',
  'Initializing post-quantum crypto engine... ML-KEM-1024 [OK]',
  'Mounting redb database... cybermanju.db [OK]',
  'Loading Tantivy search index... tantivy_index [OK]',
  'Calibrating triple-layer compressor... LZ4+ZSTD+BROTLI [OK]',
  'Warming up ONNX face detection model... [OK]',
  'Establishing cloud sync backends... [OK]',
  '',
  '>>> WELCOME TO CYBERMANJU DRIVE <<<',
  '',
  '  "Google Drive is like a digital attic — you keep throwing',
  '   stuff in there and praying you never have to find it again."',
  '',
  '  "Dropbox thought folders were revolutionary.',
  '   We thought encryption that survives a quantum apocalypse',
  '   might be slightly more important."',
  '',
  '  "The cloud is just someone else\'s computer.',
  '   This one has ML-KEM-1024 and ML-DSA-87 signatures.',
  '   Good luck, NSA."',
  '',
  'System ready. Type HELP for available commands.',
]

const commands: Record<string, { output: string[]; description: string }> = {
  help: {
    description: 'Show available commands',
    output: [
      'AVAILABLE COMMANDS:',
      '',
      '  HELP        Show this help message',
      '  ABOUT       About Cybermanju Drive',
      '  FEATURES    List all features',
      '  PLATFORMS   Show supported platforms',
      '  DOWNLOADS   Download links for all platforms',
      '  WHY         Why not just use Google Drive?',
      '  LICENSE     License information',
      '  CLEAR       Clear the terminal',
      '  LAUNCH      Launch the file manager app',
      '  REBOOT      Reboot the system (replay boot sequence)',
      '',
    ],
  },
  about: {
    description: 'About Cybermanju Drive',
    output: [
      'CYBERMANJU DRIVE v0.0.1',
      '',
      'A quantum-resistant encrypted file manager built with:',
      '  - Rust + Tauri v2 for native desktop',
      '  - Vue 3 + Pinia + Vite for the frontend',
      '  - ML-KEM-1024 (FIPS 203) post-quantum key encapsulation',
      '  - ML-DSA-87 (FIPS 204) post-quantum signatures',
      '  - X25519 hybrid classical+PQC key exchange',
      '  - Triple-layer compression (LZ4 + Zstd + Brotli)',
      '  - Tantivy full-text BM25 search',
      '  - ONNX Runtime face detection + clustering',
      '  - Tree-sitter code intelligence (50+ languages)',
      '  - Multi-cloud sync (GitHub, Google Drive, Telegram)',
      '  - JWT-authenticated REST API + Web Dashboard',
      '',
      'Repository: https://github.com/hautlythird211/Cybermanju-Drive',
      '',
    ],
  },
  features: {
    description: 'List all features',
    output: [
      'FEATURES:',
      '',
      '  [#] Post-Quantum Encryption   ML-KEM-1024 + ML-DSA. NIST FIPS 203/204.',
      '  [$] Triple Compression        LZ4 -> Zstd -> Brotli pipeline. Up to 90% reduction.',
      '  [@] Full-Text Search          Tantivy BM25 ranking with fuzzy matching.',
      '  [+] Face Detection            ONNX Runtime with clustering.',
      '  [~] Cloud Sync                Multi-backend: GitHub, GDrive, Telegram.',
      '  [!] Multi-User                JWT auth with role-based access control.',
      '  [@] GPS Map View              MapLibre GL integration with EXIF extraction.',
      '  [T] Code Intelligence         Tree-sitter parsing for 50+ languages.',
      '  [%] File Versioning           Snapshot and revert file history.',
      '  [&] Share Links               Token-based expiring file sharing.',
      '  [*] Collections               Curate files into themed collections.',
      '  [=] Trash + Audit             Soft delete with full audit trail.',
      '',
    ],
  },
  platforms: {
    description: 'Show supported platforms',
    output: [
      'SUPPORTED PLATFORMS:',
      '',
      '  WINDOWS 10/11     .MSI/.EXE installer     Tauri v2 native',
      '  macOS 12+         .DMG                    Tauri v2 native',
      '  Linux (deb)       .DEB package            Debian/Ubuntu',
      '  Linux (rpm)       .RPM package            Fedora/RHEL',
      '  Linux (Flatpak)   .FLATPAK bundle         Universal Linux',
      '  Linux (Arch)      AUR/PKGBUILD            Arch/CachyOS',
      '  Android 8+        .APK                    Tauri mobile',
      '  Web/WASM          Static site             GitHub Pages',
      '  Docker            Container               Self-hosted server',
      '',
    ],
  },
  downloads: {
    description: 'Download links for all platforms',
    output: [
      'DOWNLOAD LINKS:',
      '',
      '  GitHub Releases:',
      '    https://github.com/hautlythird211/Cybermanju-Drive/releases',
      '',
      '  Web App (GitHub Pages):',
      '    https://hautlythird211.github.io/Cybermanju-Drive/',
      '',
      '  Docker:',
      '    docker pull hautlythird211/cybermanju-drive:latest',
      '    docker run -d -p 3456:3456 -v /data:/data cybermanju-drive',
      '',
      '  AUR (Arch Linux):',
      '    yay -S cybermanju-drive',
      '',
      '  Build from source:',
      '    git clone https://github.com/hautlythird211/Cybermanju-Drive',
      '    cd Cybermanju-Drive && npm install && npm run tauri build',
      '',
    ],
  },
  why: {
    description: 'Why not just use Google Drive?',
    output: [
      'WHY NOT GOOGLE DRIVE:',
      '',
      '  "Google Drive is like a digital attic — you keep throwing',
      '   stuff in there and praying you never have to find it again."',
      '',
      '  "Google Drive reads your files. We just store them.',
      '   The difference is subtle but important."',
      '',
      '  "Google Drive: 15 GB free, then they own your soul.',
      '   Cybermanju Drive: unlimited, and your files are yours."',
      '',
      '  "Google Drive: encrypted at rest (they have the keys).',
      '   Cybermanju Drive: encrypted with ML-KEM-1024 (only you have the keys).',
      '   One of these survives a quantum computer. The other is Google."',
      '',
      '  "Google Drive: AI that reads your docs to sell you ads.',
      '   Cybermanju Drive: AI that detects faces in your photos.',
      '   We know which one is creepier."',
      '',
      '  "Dropbox: \'We\'re not just a sync service.\'',
      '   Also Dropbox: charges you $10/mo for 2 TB of sync.',
      '   Cybermanju Drive: free, open source, and has ML-DSA signatures.',
      '   Your move, Dropbox."',
      '',
    ],
  },
  license: {
    description: 'License information',
    output: [
      'LICENSE:',
      '',
      '  Cybermanju Drive is open source software.',
      '  Licensed under the MIT License.',
      '',
      '  Copyright (c) 2026 Cybermanju Systems',
      '',
      '  Permission is hereby granted, free of charge, to any person',
      '  obtaining a copy of this software... wait, you actually read',
      '  licenses? Based.',
      '',
      '  TL;DR: Do whatever you want. We are not responsible if your',
      '  files get quantum-computed into oblivion.',
      '',
    ],
  },
  clear: {
    description: 'Clear the terminal',
    output: [],
  },
  launch: {
    description: 'Launch the file manager app',
    output: [
      'LAUNCHING FILE MANAGER...',
      '',
      '  Actually, you could have just clicked the button.',
      '  But I respect the commitment to the bit.',
      '',
      '  Opening the app now. Enjoy your quantum-safe files.',
      '',
    ],
  },
  reboot: {
    description: 'Reboot the system',
    output: [],
  },
}

const visibleLines = ref<string[]>([])
const bootProgress = ref(0)
const booting = ref(true)
const userInput = ref('')
const terminalOutput = ref<string[]>([])
const showCursor = ref(true)
const cursorInterval = ref<ReturnType<typeof setInterval> | null>(null)
const inputEnabled = ref(false)
const terminalScrollRef = ref<HTMLDivElement | null>(null)
const commandHistory = ref<string[]>([])
const historyIndex = ref(-1)

function typeBootSequence() {
  if (bootProgress.value >= bootLines.length) {
    booting.value = false
    inputEnabled.value = true
    if (cursorInterval.value) clearInterval(cursorInterval.value)
    cursorInterval.value = setInterval(() => {
      showCursor.value = !showCursor.value
    }, 530)
    return
  }

  const line = bootLines[bootProgress.value]
  if (line === '') {
    visibleLines.value.push('')
    bootProgress.value++
    setTimeout(typeBootSequence, 80)
    return
  }

  let charIndex = 0
  function typeChar() {
    if (charIndex === 0) {
      visibleLines.value.push('')
    }
    if (charIndex < line.length) {
      const currentLine = visibleLines.value.pop() || ''
      visibleLines.value.push(currentLine + line[charIndex])
      charIndex++
      const delay = line[charIndex - 1] === ' ' ? 8 : 12 + Math.random() * 20
      setTimeout(typeChar, delay)
    } else {
      bootProgress.value++
      setTimeout(typeBootSequence, 100)
    }
  }
  typeChar()
}

function processCommand() {
  const cmd = userInput.value.trim().toLowerCase()
  userInput.value = ''

  if (!cmd) return

  commandHistory.value.push(cmd)
  historyIndex.value = -1

  terminalOutput.value.push(`> ${cmd}`)

  if (cmd === 'clear') {
    terminalOutput.value = []
    visibleLines.value = []
    return
  }

  if (cmd === 'reboot') {
    terminalOutput.value = []
    visibleLines.value = []
    bootProgress.value = 0
    booting.value = true
    inputEnabled.value = false
    setTimeout(typeBootSequence, 300)
    return
  }

  if (cmd === 'launch') {
    terminalOutput.value.push(...(commands.launch?.output || []))
    setTimeout(() => emit('open-app'), 1500)
    return
  }

  const command = commands[cmd]
  if (command) {
    terminalOutput.value.push(...command.output)
  } else {
    terminalOutput.value.push(
      `Unknown command: ${cmd}`,
      `Type HELP for available commands.`,
    )
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (!inputEnabled.value) return

  if (e.key === 'Enter') {
    processCommand()
    return
  }

  if (e.key === 'ArrowUp') {
    e.preventDefault()
    if (commandHistory.value.length > 0) {
      if (historyIndex.value === -1) {
        historyIndex.value = commandHistory.value.length - 1
      } else if (historyIndex.value > 0) {
        historyIndex.value--
      }
      userInput.value = commandHistory.value[historyIndex.value]
    }
    return
  }

  if (e.key === 'ArrowDown') {
    e.preventDefault()
    if (historyIndex.value >= 0 && historyIndex.value < commandHistory.value.length - 1) {
      historyIndex.value++
      userInput.value = commandHistory.value[historyIndex.value]
    } else {
      historyIndex.value = -1
      userInput.value = ''
    }
    return
  }
}

onMounted(() => {
  setTimeout(typeBootSequence, 300)
})

onUnmounted(() => {
  if (cursorInterval.value) clearInterval(cursorInterval.value)
})
</script>

<template>
  <div class="terminal-crt" @keydown="handleKeydown" tabindex="0">
    <div class="crt-bezel">
      <div class="crt-screen">
        <div class="scanlines"></div>
        <div class="screen-content" ref="terminalScrollRef">
          <div class="boot-text" v-if="visibleLines.length > 0 && booting">
            <div v-for="(line, i) in visibleLines" :key="'boot-' + i" class="boot-line">{{ line }}</div>
          </div>

          <div class="terminal-body">
            <div v-for="(line, i) in terminalOutput" :key="'out-' + i" class="out-line">{{ line }}</div>

            <div v-if="!booting" class="prompt-line">
              <span class="prompt-sign">></span>
              <span class="prompt-text">{{ userInput }}</span>
              <span class="cursor" :class="{ blink: showCursor }">&#9608;</span>
            </div>
          </div>

          <div v-if="!booting && terminalOutput.length === 0 && visibleLines.length === 0" class="help-hint">
            <div class="hint-line">SYSTEM READY. TYPE <span class="hint-cmd">HELP</span> FOR COMMANDS.</div>
            <div class="hint-line" style="margin-top: 4px;">
              <button class="launch-btn" @click="emit('open-app')">[ LAUNCH APP ]</button>
            </div>
          </div>
        </div>

        <div class="screen-status">
          <span class="status-led"></span>
          <span class="status-text">ONLINE</span>
          <span class="status-sep">|</span>
          <span class="status-text">ML-KEM-1024</span>
          <span class="status-sep">|</span>
          <span class="status-text">v0.0.1</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.terminal-crt {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #0a0a0a;
  font-family: 'Courier New', 'Fira Code', monospace;
  outline: none;
  overflow: hidden;
}

.crt-bezel {
  width: 92vw;
  max-width: 900px;
  height: 85vh;
  max-height: 640px;
  background: #1a1a1a;
  border: 4px solid #333;
  border-radius: 12px;
  padding: 20px;
  box-shadow:
    0 0 40px rgba(0, 255, 65, 0.05),
    inset 0 0 60px rgba(0, 0, 0, 0.5),
    0 20px 60px rgba(0, 0, 0, 0.8);
}

.crt-screen {
  position: relative;
  width: 100%;
  height: 100%;
  background: #0d0d0d;
  border-radius: 4px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  box-shadow: inset 0 0 30px rgba(0, 0, 0, 0.8);
}

.scanlines {
  position: absolute;
  inset: 0;
  pointer-events: none;
  z-index: 10;
  background: repeating-linear-gradient(
    0deg,
    transparent 0px,
    transparent 2px,
    rgba(0, 255, 65, 0.03) 2px,
    rgba(0, 255, 65, 0.03) 4px
  );
}

.screen-content {
  flex: 1;
  padding: 20px 24px;
  overflow-y: auto;
  overflow-x: hidden;
  color: #00ff41;
  font-size: 13px;
  line-height: 1.5;
  text-shadow: 0 0 4px rgba(0, 255, 65, 0.3);
  position: relative;
  z-index: 5;
}

.screen-content::-webkit-scrollbar {
  width: 4px;
}
.screen-content::-webkit-scrollbar-track {
  background: #0d0d0d;
}
.screen-content::-webkit-scrollbar-thumb {
  background: #00ff41;
  opacity: 0.5;
}

.boot-line {
  white-space: pre-wrap;
  word-break: break-word;
  min-height: 1.2em;
}

.out-line {
  white-space: pre-wrap;
  word-break: break-word;
  line-height: 1.4;
}

.terminal-body {
  margin-top: 4px;
}

.prompt-line {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 4px;
}

.prompt-sign {
  color: #00ff41;
  font-weight: 700;
  opacity: 0.7;
}

.prompt-text {
  color: #00ff41;
}

.cursor {
  color: #00ff41;
  font-size: 12px;
  line-height: 1;
  animation: blink 530ms step-end infinite;
}

.cursor.blink {
  animation: none;
}

@keyframes blink {
  50% { opacity: 0; }
}

.screen-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 24px;
  border-top: 1px solid rgba(0, 255, 65, 0.2);
  background: rgba(0, 0, 0, 0.4);
  position: relative;
  z-index: 5;
}

.status-led {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #00ff41;
  box-shadow: 0 0 6px #00ff41;
  animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.status-text {
  color: rgba(0, 255, 65, 0.6);
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.status-sep {
  color: rgba(0, 255, 65, 0.2);
  font-size: 10px;
}

.help-hint {
  margin-top: 8px;
}

.hint-line {
  color: #00ff41;
  font-size: 13px;
  opacity: 0.8;
}

.hint-cmd {
  font-weight: 700;
  opacity: 1;
  text-shadow: 0 0 8px rgba(0, 255, 65, 0.5);
}

.launch-btn {
  background: transparent;
  border: 1px solid #00ff41;
  color: #00ff41;
  font-family: 'Courier New', monospace;
  font-size: 13px;
  font-weight: 700;
  padding: 8px 20px;
  cursor: pointer;
  text-shadow: 0 0 4px rgba(0, 255, 65, 0.3);
  margin-top: 8px;
}

.launch-btn:hover {
  background: #00ff41;
  color: #000;
  text-shadow: none;
}

@media (max-width: 768px) {
  .crt-bezel {
    width: 98vw;
    height: 92vh;
    max-height: none;
    padding: 10px;
    border-width: 2px;
    border-radius: 6px;
  }

  .screen-content {
    padding: 12px 14px;
    font-size: 11px;
  }

  .screen-status {
    padding: 4px 14px;
  }
}
</style>
