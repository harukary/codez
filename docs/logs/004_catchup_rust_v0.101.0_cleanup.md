# rust-v0.101.0 catch-up: ローカル整理メモ

日付: 2026-02-13
ブランチ: `local/merge-cleanup-v0.101.0`
基準マージ: `8a80ff9d1` (`Merge rust-v0.101.0 into codez`)

## 今回のローカル適用（確定）

- upstream `rust-v0.101.0` を取り込み
- `turn/steer` は upstream 仕様をそのまま採用
- `codex-rs/Cargo.toml` の workspace version を `0.101.0-codez.0` に更新
- `codex-rs/core/models.json` に `gpt-5.3-codex-spark` を追加
- モデル優先順を codez 側の表示要件に合わせて調整
- `README_codez.md` に Upstream マージ履歴を追記（`2026-02-13 / 8a80ff9d1`）

## 差分の内訳（マージコミット `8a80ff9d1`）

- 変更ファイル数: 50
- 主な変更領域:
  - `codex-rs/*`（50中46ファイル）
  - ルート設定系（`.bazelrc`, `AGENTS.md`, `defs.bzl`）
  - リリース関連スクリプト（`scripts/stage_npm_packages.py`）

## upstreamに対する codez 独自差分（主要）

- `codex-rs/Cargo.toml`
  - `version: 0.101.0` -> `0.101.0-codez.0`
- `codex-rs/core/models.json`
  - codez独自の model instruction / messages 体系を維持
  - `gpt-5.3-codex-spark` を追加し、priorityを再配置
- `AGENTS.md`
  - codez運用ルール（README_codez.mdへの集約、vscode subtree運用等）を維持

## 未実施（要確認）

- テスト完了確認（今回は未実施）
  - `cargo test -p codex-core list_models`
  - `cargo test -p codex-app-server model_list`
- 上記が通るまで、モデル順変更の副作用は未確定

## 次にやること（ローカル）

- 失敗テストが出た場合のみ最小修正
- 必要なら `models.json` の priority と visibility を再調整
- 追加修正が固まったらコミットを分割
