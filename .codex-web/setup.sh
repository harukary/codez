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

echo "REPO_ROOT=$REPO_ROOT"

# codex-rs の場所（あなたの repo だとここで確実に当たる）
CODEX_RS_DIR="$REPO_ROOT/codex-rs"

if [ ! -d "$CODEX_RS_DIR" ]; then
  echo "ERROR: codex-rs directory not found under repo."
  find "$REPO_ROOT" -maxdepth 2 -type d -print | sed 's|^|  |'
  exit 1
fi

echo "CODEX_RS_DIR=$CODEX_RS_DIR"

# Rust ツール（rustup を入れた直後は cargo を見失うことがあるので保険で PATH/ENV を読む）
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: cargo not found. Install Rust toolchain (rustup) first." >&2
  exit 1
fi

if command -v rustup >/dev/null 2>&1; then
  rustup component add rustfmt clippy >/dev/null 2>&1 || true
fi

# `just` は任意（欲しい場合だけインストール）
if [ "${INSTALL_JUST:-0}" = "1" ] && ! command -v just >/dev/null 2>&1; then
  cargo install --locked just
fi

cd "$CODEX_RS_DIR"
cargo fetch

# 初回ビルド時間を短縮するため、デフォルトではビルド/チェックしない。
# 必要なら WARM_BUILD=1 で `codex` バイナリだけビルドしてキャッシュを温める。
if [ "${WARM_BUILD:-0}" = "1" ]; then
  cargo build -p codex-cli
fi

# agent 側にも PATH を効かせたいなら ~/.bashrc（重複追記は避ける）
if [ -f "$HOME/.bashrc" ]; then
  if ! grep -Fq 'export PATH="$HOME/.cargo/bin:$PATH"' "$HOME/.bashrc" >/dev/null 2>&1; then
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$HOME/.bashrc"
  fi
else
  echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$HOME/.bashrc"
fi
