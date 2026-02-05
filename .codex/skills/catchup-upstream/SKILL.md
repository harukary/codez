---
name: catchup-upstream
description: upstream（openai/codex）のstable tag取り込み〜codez差分整備〜CI/リリースまでを一連で進める手順
metadata:
  short-description: Upstream追従（stable tag）チェックリスト
  argument-hint: "rust-vX.Y.Z / codez.N / 影響範囲（TUI|VSCode|App|Docs）"
---

## 目的

- `openai/codex` の **stable tag（例: `rust-v0.94.0`）** を codez に取り込み、差分（仕様・ドキュメント・拡張）を破綻なく更新する
- CI（format / codespell / cargo-deny / release）まで通し、必要なら codez のリリースタグ発行まで終える

## 前提（このスキルが扱うリポジトリ）

- 対象リポジトリ: `~/workspace/codez`
- Rust実装: `codex-rs/`
- VSCode拡張: `vscode-extension/`（subtree）

## このスキルの“方針”（推測で埋めない）

- stable tagの取り込みを優先し、仕様追加は **判断点として明示**する
- `README_codez.md` は codez固有の運用・差分の一次情報（一般向けREADMEに混ぜない）
- VSCode拡張は「**Upstream優先**（まずupstreamのUX/仕様に追随）」、codezは「決めたcodez仕様」に合わせる
- 不要仕様は入れない（例: `ephemeral thread` は不要ならVSCode拡張で対応しない）

## 入力（最初に確定する）

- 取り込む upstream stable tag: `rust-vX.Y.Z`
- codezバージョンのsuffix: `X.Y.Z-codez.N`
- 影響範囲（最低限どれか）:
  - `TUI` / `VSCode` / `App` / `Docs` / `Release`
- “やらない”方針（例: ephemeral threadは対応しない、など）

## 手順（チェックリスト）

### 1) Upstream取り込み（tag基準）

- `git fetch upstream --tags`
- `git merge upstream/rust-vX.Y.Z`（もしくは tag を指すcommitをマージ）
- 競合が出たら「差分の意図」を先に言語化してから解消する（場当たりで直さない）

### 2) codezバージョン更新（Rust）

- `codex-rs/Cargo.toml` の `[workspace.package].version` を `X.Y.Z-codez.N` へ
- Rust変更後:
  - `cd codex-rs && just fmt`
  - 変更範囲に応じてテスト
    - 例: `cargo test -p codex-tui`
    - core/common/protocolを触った場合は `cargo test --all-features`（※実行前に合意を取る運用でも可）

### 3) 仕様差分の言語化（必須）

やること（順序固定）:
- upstreamの変更点（0.89→0.94など）を列挙
- codez既存機能と重複/競合を棚卸し
- 「統一する/廃止する/拡張で吸収する/対応しない」を決め、理由と影響範囲を書く

出力先:
- `README_codez.md`（codez向けの運用・差分の一次情報）
- VSCode拡張に挙動差分があるなら `vscode-extension/CHANGELOG.md` の `Unreleased`

### 4) VSCode拡張の追随（Upstream優先）

- upstreamで追加/変更されたUI操作やコマンド（例: `/apps`、`/personality`、モード切替）を反映
- codez側で“仕様として採用”したものだけ、codez固有の補足や既定値を整備
- 「対応しない」ものは明示（例: ephemeral threadは不要なのでVSCode拡張で未対応）

### 5) リポジトリ全体の整形・静的チェック（CI落ち潰し）

- Prettier:
  - `pnpm run format`（CIの `prettier --check` と同等に通す）
- codespell:
  - CIログに出た箇所を **修正**（原則）し、やむを得ない固有名詞だけ `.codespellignore` へ
- cargo-deny:
  - `codex-rs/deny.toml` の advisory/sources/bans を確認
  - `RUSTSEC` が出たら、可能なら `cargo update -p <crate>` で解消して `Cargo.lock` を更新

### 6) リリース（タグ→Artifacts→GitHub Release）

タグ運用（判断が必要）:
- 既存タグを付け替える（force push）か、新タグを切るかを決める
  - 既存タグ付け替えは破壊的操作なので、事前に合意を取る

Workflow（詰まりポイント）:
- `mine-release` / codezリリースで `error[E0463]: can't find crate for 'core'` が出る場合は、
  - macOS `x86_64-apple-darwin` を `macos-13` でビルドする（クロス回避）
  - muslは `Zig + .github/scripts/install-musl-build-tools.sh` を使う
  - `.github/workflows/mine-release.yml` を `rust-release.yml` の手順に寄せる

## よくあるCIエラーと対処

### Prettierが落ちる（`pnpm run format`）

- 対処: `pnpm run format` を通し、差分が出たファイルをコミットする

### codespellが落ちる

- 対処: 指摘箇所を修正（例: `configRes` のような誤判定は単語選定を変える）
- どうしても必要なら `.codespellignore` へ追加（最小限）

### cargo-denyで `vulnerability`（例: bytes）

- 対処: `cargo update -p bytes` などでロックファイルを更新し、`RUSTSEC` を解消する

### release buildで `can't find crate for 'core'`

- 対処: ターゲットstd未導入/runner不整合が典型。
  - runner見直し（macOS x86_64はintel runner）
  - toolchain actionを `dtolnay/rust-toolchain@1.93` に寄せ、`targets:` を明示

## 完了条件（このスキルのゴール）

- upstream stable tagが取り込み済み
- `README_codez.md` / `vscode-extension/CHANGELOG.md` が必要な範囲で更新済み
- `pnpm run format` が通る
- codespell / cargo-deny /（必要なら）Rustテストが通る
- リリースする場合は、タグpush〜Artifacts作成までCIが成功する

