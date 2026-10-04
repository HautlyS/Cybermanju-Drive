#!/usr/bin/env bash
# Android release signing for CI.
#
# `tauri android init` generates app/build.gradle.kts without a release
# signingConfig, so Gradle emits `*-release-unsigned.apk` and Android refuses
# to install it ("App not installed" / parse error). The keystore lives only
# in GitHub Actions secrets (src-tauri/gen/ is gitignored, so nothing
# signing-related is ever committed).
#
#   scripts/android-signing.sh prepare   # after `tauri android init`, before the build
#   scripts/android-signing.sh verify    # after the build
#
# REQUIRE_SIGNING=1 turns "signing unavailable" into an error — used by the
# release workflow, which must never publish an uninstallable APK.
set -euo pipefail

cd "$(dirname "$0")/.."

APP_DIR="src-tauri/gen/android/app"
GRADLE_FILE="$APP_DIR/build.gradle.kts"
# Written by `prepare` when it actually configured signing, so `verify` can
# tell "signing was expected but did not happen" from "signing unavailable".
EXPECTED_MARKER="src-tauri/gen/android/.signing-expected"
UNSIGNED_MARK="Android release signing (CI)"

require_signing() { [ "${REQUIRE_SIGNING:-0}" = "1" ]; }

prepare() {
  if [ ! -f "$GRADLE_FILE" ]; then
    echo "::error::$GRADLE_FILE not found — run \`tauri android init\` first" >&2
    exit 1
  fi

  if [ -z "${ANDROID_KEYSTORE_B64:-}" ]; then
    if require_signing; then
      echo "::error::Android keystore secrets are unavailable; refusing to publish an APK that cannot be installed" >&2
      exit 1
    fi
    echo "::warning::Android keystore secrets are unavailable; the APK will be unsigned" >&2
    return 0
  fi

  printf '%s' "$ANDROID_KEYSTORE_B64" | base64 -d > "$APP_DIR/cybermanju-release.keystore"
  # PKCS12 stores the key with the store password, so keyPassword mirrors it.
  cat > "$APP_DIR/keystore.properties" <<EOF
storeFile=$PWD/$APP_DIR/cybermanju-release.keystore
storePassword=${ANDROID_KEYSTORE_PASSWORD:-}
keyAlias=${ANDROID_KEY_ALIAS:-cybermanju-drive}
keyPassword=${ANDROID_KEY_PASSWORD:-${ANDROID_KEYSTORE_PASSWORD:-}}
EOF
  chmod 600 "$APP_DIR/cybermanju-release.keystore" "$APP_DIR/keystore.properties"
  touch "$EXPECTED_MARKER"

  # Idempotent: `tauri android init` regenerates the file on a fresh runner.
  if grep -q "$UNSIGNED_MARK" "$GRADLE_FILE"; then
    echo "release signing already configured"
    return 0
  fi

  cat >> "$GRADLE_FILE" <<'EOF'

// Android release signing (CI) — injected by scripts/android-signing.sh.
val androidReleaseKeystore = file("keystore.properties")
if (androidReleaseKeystore.exists()) {
    val androidReleaseProps = java.util.Properties().apply {
        androidReleaseKeystore.inputStream().use { load(it) }
    }
    android {
        signingConfigs {
            create("release") {
                storeFile = file(androidReleaseProps.getProperty("storeFile"))
                storePassword = androidReleaseProps.getProperty("storePassword")
                keyAlias = androidReleaseProps.getProperty("keyAlias")
                keyPassword = androidReleaseProps.getProperty("keyPassword")
            }
        }
        buildTypes {
            getByName("release") {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }
}
EOF
  echo "release signing configured"
}

verify() {
  local apk_dir="$APP_DIR/build/outputs/apk"
  # Signing was configured (or forced) if a signed APK must exist.
  local signing_expected=0
  if [ -f "$EXPECTED_MARKER" ] || require_signing; then
    signing_expected=1
  fi

  local apk
  apk="$(find "$apk_dir" -name '*.apk' ! -name '*-unsigned.apk' 2>/dev/null | head -1 || true)"

  if [ -z "$apk" ]; then
    if [ "$signing_expected" = "1" ]; then
      echo "::error::no signed APK produced under $apk_dir" >&2
      find "$apk_dir" -name '*.apk' 2>/dev/null || true
      exit 1
    fi
    echo "::warning::APK is unsigned (keystore secrets unavailable)" >&2
    return 0
  fi

  local apksigner
  apksigner="$(ls "${ANDROID_HOME:-/usr/local/lib/android/sdk}"/build-tools/*/apksigner 2>/dev/null | sort -V | tail -1 || true)"
  if [ -z "$apksigner" ]; then
    echo "::error::apksigner not found in \$ANDROID_HOME/build-tools" >&2
    exit 1
  fi

  "$apksigner" verify --verbose --print-certs "$apk"
  # The unsigned twin (if any) must not ship in the artifact.
  find "$apk_dir" -name '*-unsigned.apk' -delete

  # Gradle's `app-universal-release.apk` tells nobody which device it fits;
  # the build only contains arm64-v8a (ort-sys ships no other Android ABI).
  if command -v node > /dev/null; then
    local version friendly
    version="$(node -p "require('./package.json').version" 2>/dev/null || echo "")"
    if [ -n "$version" ]; then
      friendly="$(dirname "$apk")/Cybermanju-Drive-$version-arm64-v8a.apk"
      if [ "$apk" != "$friendly" ]; then
        mv "$apk" "$friendly"
        apk="$friendly"
      fi
    fi
  fi

  echo "verified signed APK: $apk"
}

case "${1:-}" in
  prepare) prepare ;;
  verify) verify ;;
  *)
    echo "usage: $0 {prepare|verify}" >&2
    exit 2
    ;;
esac
