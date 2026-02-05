---
name: catchup-upstream
description: upstream（Codex CLI本家）のstable tag取り込み〜codez差分整備〜CI/リリースまでを一連で進める汎用手順
metadata:
  short-description: Upstream追従（stable tag）チェックリスト
  argument-hint: "upstream tag / codez suffix / 影響範囲（TUI|VSCode|App|Docs|Release）"
---

## 目的

- upstream の **stable tag（例: `rust-v0.94.0`）** を codez に取り込み、差分（仕様・ドキュメント・拡張）を破綻なく更新する
- CI（format / codespell / cargo-deny / release）まで通し、必要なら codez のリリースタグ発行まで終える

## 前提（このスキルが扱うリポジトリ）

- 対象リポジトリ: `~/workspace/codez`
- Rust実装: `codex-rs/`
- VSCode拡張: `vscode-extension/`（subtree）

## このスキルの“方針”（推測で埋めない）

- stable tagの取り込みを優先し、仕様追加/変更は **判断点として明示**する
- `README_codez.md` は codez固有の運用・差分の一次情報（一般向けREADMEに混ぜない）
- VSCode拡張は「**Upstream優先**（まずupstreamのUX/仕様に追随）」、codezは「決めたcodez仕様」に合わせる
- 不要仕様は入れない（例: `ephemeral thread` を採用しないならVSCode拡張でも対応しない）

## 優先度の決め方（機能単位）

原則: **壊れるものから直す**。機能は「upstreamの変更」→「codezカスタマイズ」→「VSCode拡張」の順で整合するように扱う。

### P0: 必須（取り込み直後に必ずやる）

- ビルド/CIが落ちる要因の解消（format / codespell / cargo-deny / release workflow）
- セキュリティ勧告（`RUSTSEC` 等）でCIが失敗しているものの解消
- 破壊的変更で既存の主要導線が壊れるもの（CLI起動、会話送信、ツール実行、thread一覧など）

### P1: 互換性（壊れないために追随）

- upstreamで追加/変更された **プロトコル/API/設定**（例: tool schema、config keys、thread filters、apps/mentionsの構造）
- 既存機能の名称整理・統一（例: 旧機能を廃止し、新機能に寄せる）

### P2: UX/機能追随（ユーザー価値が高い）

- upstreamの主要UI操作（キーバインド、モード切替、`/` コマンドなど）
- VSCode拡張での体験が upstream と大きく乖離して混乱するもの

### P3: 任意（判断して採用）

- 実験的/運用依存の機能（例: 特定のthread運用、環境依存の補助機能）
- codezの個別ニーズにしか効かないもの（README_codez.mdで運用理由が説明できる場合のみ）

## あなた（ユーザー）が判断する点（毎回、明示して決める）

このスキルは判断を代行しない。未確定のまま実装を確定させない。

- 採用/不採用:
  - upstream新機能を「codezでも採用するか」「VSCode拡張で追随するか」「対応しないか」
- 統一/廃止:
  - 既存のcodez独自機能（または旧仕様）を残すか、新しいupstream仕様へ統一するか
  - 例: `AskUserQuestion` のような旧I/Fを **完全廃止**して `request_user_input` に統一する、等
- 互換性の扱い:
  - 既存の設定ファイル/プロンプト/運用（`.codex/` 配下、`config.toml`、prompts等）をどう移行するか
- “やらない”範囲:
  - 例: `ephemeral thread` は不要、入力欄のフォーカス移動は不要、など
- リリース運用:
  - 既存タグの付け替え（force push）を許可するか、新タグを切るか
  - GitHub Releaseの既存アセットを上書きするか、作り直すか

## 入力（最初に確定する）

- 取り込む upstream stable tag: `rust-vX.Y.Z`（stableを前提。pre-releaseは別判断）
- codezバージョンのsuffix: `X.Y.Z-codez.N`
- 影響範囲（最低限どれか）:
  - `TUI` / `VSCode` / `App` / `Docs` / `Release`
- “やらない”方針（例: ephemeral threadは対応しない、など）

## 手順（チェックリスト）

### 1) Upstream取り込み（tag基準）

- `git fetch upstream --tags`
- `git merge upstream/rust-vX.Y.Z`（もしくはtagを指すcommitをマージ）
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
- upstreamの変更点（前回取り込みtag→今回tag）を列挙（機能単位）
- codez既存機能と重複/競合を棚卸し（機能単位）
- 「統一する/廃止する/拡張で吸収する/対応しない」を決め、理由と影響範囲を書く（上の優先度・判断点に沿う）

出力先:
- `README_codez.md`（codez向けの運用・差分の一次情報）
- VSCode拡張に挙動差分があるなら `vscode-extension/CHANGELOG.md` の `Unreleased`

### 4) VSCode拡張の追随（Upstream優先）

- upstreamで追加/変更されたUI操作やコマンド（例: `/apps`、`/personality`、モード切替）を反映
- codez側で“仕様として採用”したものだけ、codez固有の補足や既定値を整備
- 「対応しない」ものは明示（README_codez.md / CHANGELOGに残す）

実装の優先順位（VSCode拡張）:
- P0/P1: 送受信・thread操作・tool実行・API互換（壊れる/互換性）
- P2: モード切替、ショートカット、`/` コマンド等の操作統一（混乱低減）
- P3: 見た目・補助機能（必要性が明確なものだけ）

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
