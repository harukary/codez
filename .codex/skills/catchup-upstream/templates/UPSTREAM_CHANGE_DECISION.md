---
id: CHG-000
upstream_range: "rust-vA.B.C -> rust-vX.Y.Z"
area:
  - codex-rs
  - vscode-extension
type:
  - command
  - ui
  - config
  - api
  - behavior
status: draft # draft | decided | implemented | verified
---

# Upstream変更: <短い名前>

## 1. 観察事実（upstreamで何が変わったか）

- 変更内容:
- upstreamの根拠:
  - 該当PR/コミット/リリースノート:
  - 仕様/挙動の要点（箇条書き）:

## 2. 影響（何がどう変わるか）

- 影響範囲（ユーザー導線）:
- 互換性（既存設定/既存thread/既存運用）:
- 破壊的変更の有無:
- セキュリティ/コンプライアンス観点（あれば）:

## 3. 対応方針（ここを“決定”する）

### 3.1 codez（Rust/CLI/TUI/周辺ツール）

- 方針: 採用 / 不採用 / 代替 / 遅延
- 理由（制約・コスト・安全性・整合性）:
- 移行方針（必要なら）:

### 3.2 VSCode拡張

- 方針: 追随 / 部分対応 / 未対応
- 理由（upstream整合・UX・実装制約）:
- 代替導線（未対応なら、どう見せる/どう説明する）:

## 4. ユーザーが決める質問（未確定点）

- Q1:
- Q2:
- Q3:

## 5. 受け入れ条件（成功判定）

- CLI/TUI:
- VSCode:
- 互換性（既存設定で壊れない/壊れるならエラーで露呈させる）:
- ドキュメント更新:
  - `README_codez.md`:
  - `vscode-extension/CHANGELOG.md`:

## 6. 実装メモ（決定後に埋める）

- 変更箇所:
- テスト/検証コマンド:
- リスクとロールバック:

