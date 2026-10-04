#!/usr/bin/env bash
# AGENT-4 item 15 — version single-source check.
#
# package.json is the source of truth. Every other file that carries a
# version, and (for releases) the git tag, must agree with it.
#
#   scripts/check-version.sh                 # plain consistency check (CI)
#   scripts/check-version.sh --tag v0.1.0    # release check, tag included
set -euo pipefail

cd "$(dirname "$0")/.."

tag=""
while [ $# -gt 0 ]; do
  case "$1" in
    --tag) tag="${2:-}"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

# First `version = "…"` line of a Cargo.toml is the package version.
toml_version() { sed -n 's/^version = "\(.*\)"$/\1/p' "$1" | head -1; }

fail=0
check() {
  local label="$1" actual="$2"
  if [ "$actual" = "$version" ]; then
    printf 'ok   %-34s %s\n' "$label" "$actual"
  else
    printf 'FAIL %-34s %s (expected %s, from package.json)\n' "$label" "${actual:-<missing>}" "$version"
    fail=1
  fi
}

version=$(node -e "process.stdout.write(require('./package.json').version)")

echo "source of truth: package.json = $version"

check "package-lock.json" "$(node -e "process.stdout.write(require('./package-lock.json').version)")"
check "src-tauri/tauri.conf.json" "$(node -e "process.stdout.write(require('./src-tauri/tauri.conf.json').version)")"
check "src-tauri/Cargo.toml" "$(toml_version src-tauri/Cargo.toml)"
check "crates/web/Cargo.toml (status ep.)" "$(toml_version crates/web/Cargo.toml)"
check "docker/server/Cargo.toml" "$(toml_version docker/server/Cargo.toml)"
check "docker-compose.yml x-casaos" "$(sed -n 's/^  version: "\(.*\)"$/\1/p' docker-compose.yml | head -1)"
check "aur/PKGBUILD pkgver" "$(sed -n 's/^pkgver=\(.*\)$/\1/p' aur/PKGBUILD | head -1)"
check "README.md" "$(sed -n 's/^\*\*Version:\*\* \([^ ]*\).*$/\1/p' README.md | head -1)"

for manifest in crates/*/Cargo.toml; do
  check "$manifest" "$(toml_version "$manifest")"
done

if [ -n "$tag" ]; then
  check "git tag ($tag)" "${tag#v}"
fi

if [ "$fail" -ne 0 ]; then
  echo "version mismatch — see FAIL lines above" >&2
  exit 1
fi
echo "all versions agree"
