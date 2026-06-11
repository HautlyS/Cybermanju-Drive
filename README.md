# Cybermanju Drive

> Quantum-resistant encrypted file manager with AI face grouping, triple-layer compression, code intelligence, GPS map view, web dashboard, and multi-user access control.

**Version:** 0.1.0  
**Identifier:** `com.cybermanju.drive`  
**License:** MIT

---

## Features

### Core File Management
- **Virtual file system** with folders, tagging, and metadata
- **BLAKE3 content hashing** for deduplication and integrity verification
- **MIME type detection** via the `infer` crate (magic bytes, not extensions)
- **File previews** with Lanczos3 thumbnail generation (512px max, PNG)
- **EXIF GPS extraction** from images via `kamadak-exif`
- **Collections** — curated groups: highlights, best moments, custom albums
- **Loose groups** — ad-hoc user-defined file groupings

### Post-Quantum Cryptography
- **NIST FIPS 203** — ML-KEM-1024 (Kyber) lattice-based key encapsulation, Level 5
- **NIST FIPS 204** — ML-DSA-65 (Dilithium-5) lattice-based digital signatures, Level 5
- **NIST FIPS 205** — SLH-DSA-128f (SPHINCS+) hash-based signatures, Level 1
- **Hybrid mode** — ML-KEM + X25519 for defense-in-depth transitional security
- **ChaCha20Poly1305** AEAD symmetric encryption with 96-bit CSPRNG nonces
- **BLAKE3 integrity verification** on every encrypt/decrypt cycle
- **Argon2id** password hashing for user authentication

### Triple-Layer Compression
- **LZ4** (lz4_flex) — ~400 MB/s ultra-fast, real-time previews
- **Zstandard** (level 15) — balanced ratio/speed
- **Brotli** (level 11) — maximum compression ratio for archival
- Cascading pipeline: LZ4 → ZSTD → Brotli (or any single layer)
- Per-layer stats reporting with compression ratios and timing

### Full-Text Search
- **Tantivy** search engine with BM25 ranking
- Indexed fields: filename, content text, tags
- Query support: terms, phrases, boolean operators, wildcards, fuzzy matching
- Faceted filtering by file type, encryption status, GPS data
- Real autocomplete from Tantivy term dictionary

### AI Face Detection & Clustering
- Face embedding extraction (128-dim vectors, BLAKE3-seeded deterministic pipeline)
- Cosine distance computation for similarity measurement
- DBSCAN-like clustering via connected components (Union-Find)
- Automatic person grouping with centroid embeddings

### Code Intelligence
- **tree-sitter** integration with language detection for 50+ file extensions
- Heuristic symbol extraction: functions, classes, structs, traits, interfaces
- Language-aware keyword sets for Rust, Python, Go, TypeScript, Java, C/C++, Ruby, Swift, and more
- Structured AST output with symbol names, kinds, and line ranges

### Multi-User Access Control
- Role-based access control: `admin`, `user`, `viewer`
- Per-file permissions: `read`, `write`, `admin`
- Argon2id password hashing with cryptographically secure salts
- JWT-like session tokens (UUID v4)

### Cloud Sync
- **Local** — filesystem copy to any local directory
- **GitHub** — Contents API + Releases for large files (up to 2GB)
- **Google Drive** — Drive API v3 with full CRUD
- **Google Photos** — optimized media upload
- Configurable pipeline: compress → preview → upload → link → delete raw
- Real-time progress with ETA estimation and cancellation support

### GPS Map View
- EXIF GPS coordinate extraction from photos
- Interactive map display via MapLibre GL
- Per-file geo-marker clustering

### Web Dashboard
- Embedded HTTP/1.1 server on port 3456 (no external HTTP dependency)
- REST API mirroring all Tauri IPC commands
- Works in Docker containers and ZimaOS NAS devices
- Browser access from any device on the network

---

## Tech Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| Desktop Framework | Tauri | v2 |
| Backend Language | Rust | 2021 edition |
| Frontend Framework | Vue 3 (Composition API) | ^3.5.13 |
| State Management | Pinia | ^3.0.2 |
| Type System | TypeScript | ^5.8.3 |
| Build Tool | Vite | ^6.3.5 |
| Icons | Lucide Vue | ^0.525.0 |
| Maps | MapLibre GL | ^5.4.0 |
| Database | redb | 2.x |
| Full-Text Search | Tantivy | 0.22 |
| Compression | lz4_flex + zstd + brotli | 0.11 / 0.13 / 7 |
| Encryption | ChaCha20Poly1305 + rustpq | 0.10 / 0.2 |
| Password Hashing | argon2 | 0.5 |
| Hashing | BLAKE3 | 1 |
| Content ID | UUID v4 | 1 |
| ML Inference | ort (ONNX Runtime) | 2.0.0-rc.12 |
| Code Parsing | tree-sitter | 0.24 |
| EXIF | kamadak-exif | 0.5 |
| Image Processing | image | 0.25 |
| MIME Detection | infer + mime_guess | 0.16 / 2 |

---

## Quick Start

### Prerequisites

- **Node.js** 20+
- **Rust** 1.85+ (via [rustup](https://rustup.rs/))
- **Platform dependencies:**
  - **Linux:** `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libsoup-3.0-dev`, `libjavascriptcoregtk-4.1-dev`
  - **macOS:** Xcode Command Line Tools
  - **Windows:** WebView2 Runtime (usually pre-installed)

### Install & Run

```bash
# Clone the repository
git clone https://github.com/cybermanju/cybermanju-drive.git
cd cybermanju-drive

# Install frontend dependencies
npm install

# Run in development mode (Tauri desktop + Vite HMR)
npm run tauri:dev
```

The app will open a native window. The web dashboard simultaneously starts on `http://localhost:3456`.

---

## Build Instructions

### Desktop App

```bash
# Type-check frontend
npm run typecheck

# Check Rust code
npm run rust:check

# Lint Rust code
npm run rust:clippy

# Build production desktop app
npm run tauri:build

# Build debug desktop app (faster, larger)
npm run tauri:build:debug
```

Output installers are in `src-tauri/target/release/bundle/`.

### Docker Image

```bash
# Build the multi-stage Docker image
docker build -t cybermanju-drive:latest .

# Run with Docker Compose
docker compose up -d

# Access at http://localhost:3456
```

The container runs as a non-root user with persistent data in `/data`.

### WASM / GitHub Pages

```bash
# Build frontend for web deployment
npm run build:wasm
```

Output is in `dist-wasm/`. The CI pipeline automatically deploys this to GitHub Pages on push to `main`.

---

## ZimaOS Installation

Cybermanju Drive is packaged as a ZimaOS App Store application with full metadata:

1. **Add the app** to your ZimaOS instance via the App Store, or deploy manually with:
   ```bash
   docker compose up -d
   ```
2. The container maps persistent data to `/DATA/AppData/cybermanju-drive/config`
3. Access the web dashboard at `http://<your-nas-ip>:3456`
4. Supported architectures: **amd64** and **arm64**

### ZimaOS Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Rust log level |
| `PORT` | `3456` | Web dashboard port |
| `DB_PATH` | `/data/cybermanju.db` | Database file path |
| `STATIC_DIR` | `/app/static` | Vue frontend static files |
| `TZ` | `UTC` | Timezone |

---

## API Documentation

### Tauri IPC Commands

The desktop app exposes 40+ Tauri IPC commands registered in `src-tauri/src/lib.rs`. See [ARCHITECTURE.md](./ARCHITECTURE.md) for the full data flow documentation.

### Web Dashboard REST API

When running as a Docker container or with the embedded web dashboard, all data is accessible via REST:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/health` | GET | Health check |
| `/api/files` | GET | List all files |
| `/api/files/{id}` | GET | Get file by ID |
| `/api/files/{id}` | DELETE | Delete a file |
| `/api/accounts` | GET | List storage accounts |
| `/api/collections` | GET | List collections |
| `/api/collection-items` | GET | List collection items |
| `/api/face-groups` | GET | List face groups |
| `/api/loose-groups` | GET | List loose groups |
| `/api/encryption/status` | GET | Encryption engine status |
| `/api/encryption/keys` | GET | List encryption keys |
| `/api/geo-files` | GET | Files with GPS coordinates |
| `/api/search?q={term}` | GET | Search files |
| `/api/locations` | GET | List locations |
| `/api/users` | GET | List users |
| `/api/users/register` | POST | Register user |
| `/api/users/login` | POST | Login |
| `/api/permissions/{fileId}` | GET | Get file permissions |
| `/api/permissions` | POST | Grant permission |
| `/api/permissions/verify` | POST | Verify access |

Full REST API documentation with request/response schemas is in [ARCHITECTURE.md](./ARCHITECTURE.md#10-web-dashboard-rest-api).

---

## Project Structure

```
cybermanju-drive/
├── .github/workflows/
│   └── ci.yml                          # Rust check, Docker build, WASM build, Pages deploy
├── docker/
│   └── server/
│       ├── Cargo.toml                  # Standalone server dependencies
│       └── src/main.rs                 # Docker entrypoint (web_dashboard + static files)
├── public/
│   └── tauri.svg                       # App icon
├── scripts/
│   ├── build-all.sh                    # Build desktop + Docker + WASM
│   ├── build-wasm.sh                   # Build WASM for web
│   ├── check-all.sh                    # Run all checks
│   └── dev.sh                          # Development helper
├── src/
│   ├── assets/main.css                 # Global styles
│   ├── components/
│   │   ├── FileGrid.vue                # Main file browser (grid/list/masonry)
│   │   ├── TopBar.vue                  # Search bar, view toggles, user menu
│   │   ├── Sidebar.vue                 # Navigation, tree, locations, collections
│   │   ├── StatusBar.vue               # Bottom status bar
│   │   ├── FilePreview.vue             # File preview panel
│   │   ├── DashboardOverlay.vue        # Startup/loading overlay
│   │   ├── SyncPanel.vue               # Cloud sync configuration and progress
│   │   ├── CompressionPanel.vue        # Compression stats and controls
│   │   ├── EncryptionPanel.vue         # PQC key management and encryption
│   │   ├── FaceGroupingPanel.vue       # Face detection and person clusters
│   │   ├── MapView.vue                 # GPS map with geo-markers
│   │   ├── CollectionsPanel.vue        # Collection management
│   │   ├── CodeIntelligencePanel.vue   # Source code symbol extraction
│   │   ├── WebDashboardPanel.vue       # Dashboard status and endpoints
│   │   ├── UserManagementPanel.vue     # User/role/permission management
│   │   ├── MatrixRain.vue              # Matrix digital rain animation
│   │   └── PrayerFlags.vue             # Animated prayer flag decorations
│   ├── composables/
│   │   └── useTauri.ts                 # Dual-mode IPC/REST composable
│   ├── stores/
│   │   └── app.ts                      # Pinia store with all state and actions
│   ├── types/
│   │   └── index.ts                    # TypeScript type definitions + design tokens
│   ├── App.vue                         # Root component
│   └── main.ts                         # Vue app entry point
├── src-tauri/
│   ├── build.rs                        # Tauri build script
│   ├── Cargo.toml                      # Rust dependencies
│   ├── Cargo.lock                      # Locked dependency versions
│   ├── capabilities/default.json       # Tauri v2 capability permissions
│   ├── icons/icon.svg                  # App icon
│   ├── tauri.conf.json                 # Tauri configuration
│   └── src/
│       ├── main.rs                     # Tauri process entry point
│       ├── lib.rs                      # Core library: module registration, AppState, Tauri builder
│       ├── commands/
│       │   ├── mod.rs                  # Command module exports
│       │   ├── files.rs                # File CRUD, folders, loose groups
│       │   ├── accounts.rs             # Storage account management
│       │   ├── collections.rs          # Collection CRUD
│       │   ├── compression.rs          # Compress/decompress commands
│       │   ├── encryption.rs           # Encrypt/decrypt, keypair generation
│       │   ├── faces.rs                # Face detection trigger and listing
│       │   ├── map.rs                  # GPS file listing, EXIF extraction
│       │   ├── search.rs               # Search and autocomplete commands
│       │   ├── sync.rs                 # Sync config CRUD, start/cancel/progress
│       │   ├── users.rs                # Register, authenticate, RBAC permissions
│       │   ├── dashboard.rs            # Web dashboard start/stop/status
│       │   └── import.rs               # File import, directory scan, index rebuild
│       ├── db/
│       │   ├── mod.rs                  # redb wrapper, 11 table definitions
│       │   └── schema.rs               # All Rust structs (FileNode, User, etc.)
│       ├── compression/
│       │   ├── mod.rs                  # Module exports
│       │   └── triple.rs               # TripleCompressor: LZ4 → ZSTD → Brotli
│       ├── crypto/
│       │   ├── mod.rs                  # Module exports
│       │   └── pqc.rs                  # PQC engine, ChaCha20Poly1305, ML-DSA sign/verify
│       ├── faces/
│       │   └── mod.rs                  # Face embeddings, cosine distance, DBSCAN clustering
│       ├── preview/
│       │   └── mod.rs                  # Lanczos3 thumbnail, metadata extraction
│       ├── search/
│       │   ├── mod.rs                  # Module exports
│       │   └── tantivy_index.rs        # Tantivy schema, indexing, BM25 search, autocomplete
│       ├── sync/
│       │   ├── mod.rs                  # Module exports
│       │   ├── models.rs               # SyncConfig, SyncProgress, SyncResult, StorageBackend trait
│       │   ├── backends.rs             # Local, GitHub, Google Drive, Google Photos implementations
│       │   └── pipeline.rs             # SyncPipeline: scan → compress → preview → upload → link → clean
│       ├── tree_sitter/
│       │   └── mod.rs                  # Language detection, heuristic symbol extraction
│       └── web_dashboard/
│           └── mod.rs                  # Embedded HTTP server, REST API router, handlers
├── Dockerfile                          # Multi-stage Docker build (Node → Rust → Alpine)
├── docker-compose.yml                  # Docker Compose with ZimaOS x-casaos metadata
├── index.html                          # Vite HTML entry point
├── package.json                        # Node.js dependencies and scripts
├── tsconfig.json                       # TypeScript configuration
├── tsconfig.node.json                  # TypeScript config for Vite/Node
├── vite.config.ts                      # Vite config for Tauri desktop
├── vite.config.wasm.ts                 # Vite config for WASM/GitHub Pages
├── env.d.ts                            # Vite environment type declarations
├── ARCHITECTURE.md                     # Detailed architecture documentation
└── README.md                           # This file
```

---

## Scripts

| Command | Description |
|---------|-------------|
| `npm run dev` | Start Vite dev server (frontend only) |
| `npm run tauri:dev` | Start Tauri desktop app with HMR |
| `npm run build` | Type-check + build frontend |
| `npm run tauri:build` | Build production desktop installer |
| `npm run build:wasm` | Build frontend for web/GitHub Pages |
| `npm run typecheck` | TypeScript type checking |
| `npm run rust:check` | Cargo check |
| `npm run rust:clippy` | Cargo clippy lints |
| `npm run rust:fmt` | Check Rust formatting |
| `npm run rust:test` | Run Rust tests |
| `npm run check:all` | TypeScript + cargo check + clippy |
| `npm run clean` | Remove build artifacts and database |
| `npm run db:reset` | Remove database and search index |

---

## License

MIT