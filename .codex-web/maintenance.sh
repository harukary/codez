#!/usr/bin/env bash
set -euo pipefail

if [ "${CODEX_WEB_DEBUG:-0}" = "1" ]; then
  set -x
fi

WORKSPACE="${WORKSPACE:-/workspace}"

if [ "${WORKSPACE:-}" = "/workspace" ]; then
  if [ -d "$(pwd)/.git" ] || [ -d "$(pwd)/codex-rs" ]; then
    WORKSPACE="$(pwd)"
  fi
fi

resolve_repo_root() {
  local workspace="$1"

  if [ -d "$workspace/.git" ]; then
    printf '%s\n' "$workspace"
    return 0
  fi

  if [ -d "$workspace/codex-mine/.git" ]; then
    printf '%s\n' "$workspace/codex-mine"
    return 0
  fi

  if git -C "$workspace" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git -C "$workspace" rev-parse --show-toplevel
    return 0
  fi

  local candidate=""
  for candidate in "$workspace"/*; do
    if [ -d "$candidate/.git" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  return 1
}

REPO_ROOT="$(resolve_repo_root "$WORKSPACE" || true)"
if [ -z "$REPO_ROOT" ]; then
  echo "ERROR: could not find git repository under $WORKSPACE" >&2
  ls -la "$WORKSPACE" >&2 || true
  exit 1
fi

CODEX_RS_DIR="$REPO_ROOT/codex-rs"
if [ ! -d "$CODEX_RS_DIR" ]; then
  echo "ERROR: codex-rs directory not found under repo." >&2
  exit 1
fi

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
fi

cd "$CODEX_RS_DIR"
cargo fetch

# 既存キャッシュを活かすため、デフォルトではビルドしない。
if [ "${WARM_BUILD:-0}" = "1" ]; then
  cargo build -p codex-cli
fi
