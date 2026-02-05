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

## 基本原則（推測で埋めない / 変更を隠さない）

- stable tagの取り込みを優先し、仕様追加/変更は **判断点として明示**する
- `README_codez.md` は codez固有の運用・差分の一次情報（一般向けREADMEに混ぜない）
- VSCode拡張は「**Upstream優先**（まずupstreamのUX/仕様に追随）」、codezは「決めたcodez仕様」に合わせる
- 不要仕様は入れない（例: `ephemeral thread` を採用しないならVSCode拡張でも対応しない）
- “とりあえず動く”フォールバックで誤魔化さない（問題は露呈させ、判断が必要なら止める）
- 「実装したこと」と「提案/推測」を混ぜない（根拠と未確定点を分離する）

## 変更対応の決め方（全upstream変更に対して、codez / VSCode拡張の方針を“決定”する）

このスキルの中心は「Upstreamの差分を一覧化し、各項目ごとに **codez / VSCode拡張としてどう扱うかを決定する**」こと。

### まず作る成果物: 「Upstream変更→対応方針」リスト

- 形式は `templates/UPSTREAM_CHANGE_DECISION.md` の1項目を、変更点の数だけ積み上げる（1変更=1決定）
- 目的は「実装の前に決定を固定し、後から検証できる状態にする」

### “決定”に含める内容（各変更項目で必須）

- upstream側の変更の要約（機能名・CLI/設定/APIなどの分類）
- 影響（どこが壊れる/変わるか、ユーザー導線、互換性、データ/設定移行）
- codezの方針（採用 / 不採用 / 代替 / 遅延）
- VSCode拡張の方針（追随 / 部分対応 / 未対応）
- 移行方針（既存設定・既存スレッド・既存ワークフロー）
- 検証（成功判定・テスト観点・CI観点）
- ドキュメント更新先（`README_codez.md` / `vscode-extension/CHANGELOG.md` など）

## 決定までの流れ（推奨の順序）

このフローは「優先度で殴る」ではなく、「判断可能な粒度で決定する」ための順序。

### 0) スコープ確定（ここが曖昧だと決定がブレる）

- 対象 upstream 範囲（例: `rust-vA.B.C` → `rust-vX.Y.Z`）
- 対象コンポーネント（`codex-rs` / `vscode-extension` / `docs` / `release`）
- “やらない”方針（例: ephemeral threadは採用しない）

### 1) Upstream変更の棚卸し（リスト化）

- 方法は固定しないが、最低限は「前回取り込みtag→今回tagの差分」で拾う
- 重要: 変更は「コミット列挙」ではなく、「機能/仕様単位」に正規化して1項目=1決定にする

### 2) 変更ごとに「決定シート」を作る（未決定を露呈させる）

- `templates/UPSTREAM_CHANGE_DECISION.md` を複製して、変更項目ごとに作成する
- ここで「不明点」「判断が必要な点」を質問として列挙し、**あなた（ユーザー）が決める**
- 実装は“決定後”に始める（決定なしに突っ走らない）

### 3) あなた（ユーザー）が決める点（毎回、必ず明示して確定）

このスキルは判断を代行しない。未確定のまま実装を確定させない。

- 採用範囲:
  - upstream新機能を「codezでも採用する/しない」
  - VSCode拡張で「追随する/部分対応/未対応」
- 互換性:
  - 既存設定（`.codex/` 配下、`config.toml`、prompts等）を維持するか、移行を入れるか（移行するなら手順を用意するか）
  - 破壊的変更がある場合に、既存利用者へどう見せるか（エラーで露呈させる/段階移行など）
- 整理統合:
  - 既存のcodez独自機能（または旧仕様）を残すか、upstream仕様へ統一するか
  - 例: 旧I/F（`AskUserQuestion` 等）を廃止し、upstream I/F（`request_user_input` 等）へ統一する、など
- “やらない”範囲:
  - 例: ephemeral threadは採用しない、入力欄のフォーカス移動は不要、など
- リリース運用:
  - 既存タグの付け替え（force push）を許可するか、新タグを切るか
  - GitHub Releaseの既存アセットを上書きするか、作り直すか

### 4) 実装（決定に従う）→ 検証 → ドキュメント反映

- 実装は「決定シート」の方針・受け入れ条件に従う
- 検証は「成功判定」を満たすまで
- ドキュメントは差分が残る場所にのみ追記する（`README_codez.md` / `vscode-extension/CHANGELOG.md`）

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
- 「統一する/廃止する/拡張で吸収する/対応しない」を決め、理由と影響範囲を書く（本スキルの決定フローに沿う）

出力先:
- `README_codez.md`（codez向けの運用・差分の一次情報）
- VSCode拡張に挙動差分があるなら `vscode-extension/CHANGELOG.md` の `Unreleased`

### 4) VSCode拡張の追随（Upstream優先）

- upstreamで追加/変更されたUI操作やコマンド（例: `/apps`、`/personality`、モード切替）を反映
- codez側で“仕様として採用”したものだけ、codez固有の補足や既定値を整備
- 「対応しない」ものは明示（README_codez.md / CHANGELOGに残す）

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
