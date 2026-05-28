#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_TRIPLE="${TAURI_ENV_TARGET_TRIPLE:-$(rustc -vV | awk '/^host:/ { print $2 }')}"

if [[ -z "${TARGET_TRIPLE}" ]]; then
  echo "failed to determine Rust target triple" >&2
  exit 1
fi

cargo build --manifest-path "${ROOT_DIR}/Cargo.toml" -p localairouter-daemon --release

mkdir -p "${ROOT_DIR}/apps/desktop/src-tauri/binaries"
cp \
  "${ROOT_DIR}/target/release/localairouter-daemon" \
  "${ROOT_DIR}/apps/desktop/src-tauri/binaries/localairouter-daemon-${TARGET_TRIPLE}"

