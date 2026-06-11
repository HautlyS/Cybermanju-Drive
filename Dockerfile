# ═══════════════════════════════════════════════════════════════════════
# Cybermanju Drive — Multi-stage Docker Build
#
# Stage 1: Build Vue 3 frontend (Node.js)
# Stage 2: Build standalone Rust web server (no Tauri/GTK deps)
# Stage 3: Minimal Alpine runtime
# ═══════════════════════════════════════════════════════════════════════

# ─── Stage 1: Frontend Build ──────────────────────────────────────────
FROM node:20-alpine AS frontend-builder

WORKDIR /app

# Install dependencies first (layer caching)
COPY package.json package-lock.json* ./
RUN npm install --frozen-lockfile 2>/dev/null || npm install

# Copy frontend source and build for web deployment (no Tauri)
COPY index.html ./
COPY tsconfig.json tsconfig.node.json env.d.ts ./
COPY vite.config.wasm.ts ./
COPY public/ ./public/
COPY src/ ./src/

# DOCKER_BUILD=true tells vite.config.wasm.ts to use base: "/" instead of
# the GitHub Pages prefix "/cybermanju-drive/"
RUN DOCKER_BUILD=true npm run build:wasm

# ─── Stage 2: Rust Backend Build ─────────────────────────────────────
FROM rust:1.85-alpine AS backend-builder

# musl-dev is required for linking on Alpine
RUN apk add --no-cache musl-dev pkgconf

WORKDIR /build

# Copy the standalone server project definition
COPY docker/server/Cargo.toml ./Cargo.toml
COPY docker/server/src/main.rs ./src/main.rs

# Create a stub to pre-cache dependency compilation (this file is
# replaced with the real module in the next COPY layer)
RUN echo "pub struct WebDashboard;" > src/web_dashboard.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -f src/web_dashboard.rs

# Now copy the actual web_dashboard module from the Tauri crate
COPY src-tauri/src/web_dashboard/mod.rs ./src/web_dashboard.rs

# Make handle_request public so main.rs can route API requests to it
RUN sed -i 's/^fn handle_request(/pub fn handle_request(/' src/web_dashboard.rs

# Build the release binary
# Touch the source to invalidate the cache placeholder
RUN touch src/web_dashboard.rs && cargo build --release

# ─── Stage 3: Minimal Runtime ────────────────────────────────────────
FROM alpine:3.21 AS runtime

# ca-certificates for HTTPS, wget for healthcheck
RUN apk add --no-cache ca-certificates wget

# Create non-root user for security
RUN addgroup -S cybermanju && adduser -S cybermanju -G cybermanju

WORKDIR /app

# Copy the compiled Rust binary from stage 2
COPY --from=backend-builder /build/target/release/cybermanju-drive-server ./

# Copy the compiled Vue frontend from stage 1
COPY --from=frontend-builder /app/dist-wasm ./static

# Create data directory for database and config
RUN mkdir -p /data && chown -R cybermanju:cybermanju /app /data

# Switch to non-root user
USER cybermanju

# Expose the web dashboard port
EXPOSE 3456

# Environment variables (can be overridden in docker-compose.yml)
ENV RUST_LOG=info
ENV PORT=3456
ENV DB_PATH=/data/cybermanju.db
ENV STATIC_DIR=/app/static
ENV TZ=UTC

# Volume mount point for persistent data
VOLUME ["/data"]

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=10s --retries=3 --start-period=15s \
    CMD wget --spider -q http://localhost:3456/api/health || exit 1

# Start the server
CMD ["./cybermanju-drive-server"]