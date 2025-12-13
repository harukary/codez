# 003 execpolicy_dotenv_fallback

 - [x] ルール・ドキュメント確認（AGENTS.md、README_mine.md、関連仕様）。
 - [x] 現状調査：execpolicy/dotenvの読み込み経路とconfig解決の影響箇所を把握。
 - [x] 修正方針策定：リポジトリ`.codex/config.toml`有無に関わらずホーム設定をフォールバックする手順を決める。
 - [x] 実装：execpolicy/dotenvの解決ロジックを修正しテストを追加。
 - [x] テスト実行：必要なフォーカステストと`just fmt`/`just fix`を実施。
 - [x] 最終確認：変更内容とログの反映、番号整合性チェック。
