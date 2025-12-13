# 002 config_resolution_plan

- [x] リポジトリルール・ドキュメント確認（AGENTS.md、README_mine.md、docs/logs 番号確認）。
- [x] 現状調査：設定読み込みとプロンプト解決の実装箇所を把握（`codex-rs/core/src/config/mod.rs`、`config_loader`、`custom_prompts.rs`）。
- [x] 設計案まとめ：リポジトリローカル`.codex/config.toml`優先ロジックと既存`CODEX_HOME`依存箇所の整理。
- [x] 実装タスク洗い出し：必要な関数追加・呼び出し元変更・ドキュメント更新の手順化。
- [x] テスト計画作成：新優先順位のユニットテスト/統合テストと既存プロンプトマージ挙動の回帰確認方法整理。

## 現状整理
- `find_codex_home()`は`CODEX_HOME`環境変数があればそれを正規化し、無ければ`~/.codex`を返す（存在確認なし）。【`codex-rs/core/src/config/mod.rs`付近】
- 各種設定読み込みは`Config::load_with_cli_overrides`→`load_config_layers_with_overrides`経由で`codex_home/config.toml`を基底レイヤーとして読む。リポジトリローカルの存在チェックはしていない。
- カスタムプロンプトは`prompt_search_roots`で「リポジトリ`.codex/prompts`→$CODEX_HOME/prompts」の順に探索し、`discover_prompts`で名前重複をリポジトリ優先で除外している。

## 設計方針
- **config.toml**: カレントディレクトリからGitルートを辿り、`.codex/config.toml`があればそれを採用。なければ従来通り`CODEX_HOME`/`~/.codex`を使用。マージはせず、選択したベースディレクトリを`Config`の`codex_home`として扱う。
- **prompts**: 既存ロジック（リポジトリ→ホームの順でマージ）を維持。ホーム側の解決は`find_codex_home()`を引き続き利用し、config.tomlの探索結果に依存させない。
- **影響範囲整理**: `find_codex_home()`を直接変更するとプロンプトやログ保存先がホーム以外に変わるため、設定ファイル用に別の解決関数を導入し、読み込み入口でのみリポジトリ優先を適用する。`Config`生成時にどのベースを使ったかを保持し、相対パス解決やmanaged_configとの整合性を担保する。

## 実装タスク（予定）
1) `config`モジュールに「設定ベースディレクトリ解決」用のヘルパーを追加（例：`resolve_config_base(cwd)`）。内部でGitルート探索（`resolve_root_git_project_for_trust`や`get_git_repo_root`）と`.codex/config.toml`存在チェックを行い、見つかった場合のみその`.codex`を返す。
2) `Config::load_with_cli_overrides`および関連の`load_config_*`入口で新ヘルパーを使用し、選択されたベースディレクトリを`codex_home`として以降の処理に渡す。`CODEX_HOME`環境変数はリポジトリに`.codex/config.toml`が無い場合のフォールバックとして利用する。
3) `find_codex_home()`を必要とする既存処理との整合確認。プロンプトやホームディレクトリ固有のパス計算はこれまで通り`find_codex_home()`を使い、設定用のベース解決と混同しないよう呼び分ける。
4) ドキュメント更新（`README_mine.md`や`docs/config.md`該当箇所など）で新たな優先順位と`.codex/config.toml`の扱いを明文化。

## テスト計画（予定）
- `codex-core`のユニットテストに「リポジトリ直下に`.git`と`.codex/config.toml`がある場合にホーム設定を無視する」ケースを追加。tempdirにGitルートを再現し、`Config::load_with_cli_overrides`がリポジトリ側ファイルを読むことを確認する。
- 上記が無い場合に`CODEX_HOME`/`~/.codex`が使用されるフォールバックを検証するテストを追加。
- プロンプト探索の回帰確認として、既存の`custom_prompts`系テストを実行し、リポジトリとホームのマージ順序が維持されることを確認。
- 変更後は影響範囲に応じて`cargo test -p codex-core`や関連クレートのスポットテストを実施。

## 追記: config_resolution_followup

- [x] ルール再確認（AGENTS.md、README_mine.md、ユーザー指示「docs/logsは002のみに統一」）。
- [x] 既存実装とレビュー指摘の再確認（config解決処理・テストの挙動）。
- [x] 修正方針策定（必要なコード修正とログ統合作業の段取り）。
- [x] 実装（config処理の修正、ドキュメント/ログの整理）。
- [x] テスト実行と結果確認。
- [x] ログ反映（002に統合、不要ファイル削除）と最終チェック。
