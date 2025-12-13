# Codex Web Container Configuration

## Description

This repo includes a **copy/paste template** for configuring a Codex Web Environment. Simply having these files in the repository does **not** apply anything automatically.

- In Codex Web, configure `Container Image`, `Setup Script`, and `Maintenance Script` in your Environment settings
- For the `Setup Script` / `Maintenance Script` fields, paste `@./.codex-web/setup.sh` / `@./.codex-web/maintenance.sh` exactly as shown below
- `@./...` tells Codex Web to execute a script from this repository, so the referenced files must be committed

## Container Image
[openai/codex-universal](https://github.com/openai/codex-universal)

## Setup Script
The setup script is run after creating new containers, after the repo is cloned.
Network access is always enabled for this step.

@./.codex-web/setup.sh

環境変数（任意）:

- `WARM_BUILD=1`: `cargo build -p codex-cli` を実行してビルドキャッシュを温める（デフォルトはビルドしない）
- `INSTALL_JUST=1`: `just` をインストールする（デフォルトはインストールしない）
- `CODEX_WEB_DEBUG=1`: `set -x` でデバッグログを出す

## Maintenance Script
The maintenance script is run in containers that were resumed from the cache, after checking out the branch.
Network access is always enabled for this step.

@./.codex-web/maintenance.sh

環境変数（任意）:

- `WARM_BUILD=1`: `cargo build -p codex-cli` を実行してビルドキャッシュを温める（デフォルトはビルドしない）
- `CODEX_WEB_DEBUG=1`: `set -x` でデバッグログを出す
