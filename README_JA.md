<div align="center">

# CC Switch Usage Plugin

### CC Switch に API Key ごとの使用制限（バジェット）を追加 — ローカルプロキシが強制します

[![Release](https://img.shields.io/github/v/release/Xhoryon/cc-switch-usage-plugin?color=blue&label=release)](https://github.com/Xhoryon/cc-switch-usage-plugin/releases)
[![Platform](https://img.shields.io/badge/platform-macOS%20Apple%20Silicon-lightgrey.svg)](https://github.com/Xhoryon/cc-switch-usage-plugin/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

[English](README.md) | [简体中文](README_ZH.md) | [繁體中文](README_ZH-TW.md) | 日本語

</div>

---

## これは何？

**CC Switch Usage Plugin** は、オープンソースのプロバイダ切り替えツール
[CC Switch](https://github.com/farion1231/cc-switch) にバジェット機能を追加したものです。
すべての API Key に**使用上限** — 金額（USD / CNY / EUR / JPY / GBP）または
トークン数 — を設定でき、上限に達すると CC Switch のローカルプロキシが新しい
リクエストを明確なエラーで**拒否します**。統計だけの「見かけの制限」ではありません。

## 機能

- **金額制限** — USD / CNY / EUR / JPY / GBP の 5 通貨に対応。USD 以外の通貨は
  ローカルの USD→X 為替レートで換算し、レートはダイアログ内で自由に編集できます
  （妥当なデフォルト値付き、オンライン API 不要）
- **リセット周期** — 手動リセットのみにするか、ローカル時刻の境界で使用量を自動的に
  再集計するかを選択できます：毎時 / 毎日 / 毎週（月曜開始）/ 毎月（1 日開始）。
  使い切ったウィンドウは次の境界で自動的に回復します
- **カスタムウィンドウ（v1.2.0 新規）** — 集計は制限を有効にした瞬間から
  開始（それ以前の使用量は決して含まれず）、N 時間 / N 日ごとの自動リセット
  を自由に設定できます
- **カード使用量バッジ（v1.2.0 新規）** — メーターアイコンの横の目のトグル
  で、プロバイダカードに `{百分比}` と次回リセットまでの残り時間を直接表示
- **トークン制限** — 内蔵の使用量ダッシュボードと同じ正規化トークン計算を使用するため、
  両者の数値は常に一致します
- **本物の強制執行** — プロキシは転送**前**に予算をチェックします。上限に達すると、
  新しいリクエストは構造化された `usage_limit_reached` エラー（HTTP 429）で即座に失敗。
  すでに送信されたリクエストは最後まで完了するため、最後のリクエストで多少の超過が
  発生するのは正常な設計です
- **プロバイダ名ではなくクレデンシャルに紐付け** — 制限は API Key の SHA-256
  フィンガープリントに紐付き、平文のキーは一切保存されません。キーをローテーションすると
  予算は自動的に新規開始します
- **カードごとのライブステータス** — メーターアイコンの 4 状態
  （オフ / 通常 / 警告 ≥80% / 枯渇）、進捗バー・残額・使用量リセット付きの設定ダイアログ
  （統計ウィンドウを進めるだけで、履歴は削除されません）
- **誤解を招かない設計** — トラフィックがプロキシを経由しない場合やモデルの価格が
  未登録の場合、UI が明示します。制限が機能していないふりはしません
- **対応言語** — 简体中文、繁體中文、English、日本語

## インストール

1. [Releases](https://github.com/Xhoryon/cc-switch-usage-plugin/releases) ページから
   `CC.Switch_3.20.3_aarch64_usage-limit.dmg`（macOS、Apple Silicon）をダウンロード。
2. DMG をマウントし、**CC Switch.app** をアプリケーションにドラッグします。
3. 初回起動：アプリを右クリックして**「開く」→「開く」**を選択してください。
   本プラグインは公証されていないため、macOS が 1 回だけ「開発者を検証でき
   ません」と表示します。ターミナルで
   `xattr -dr com.apple.quarantine "/Applications/CC Switch.app"` を 1 回
   実行しても良いです。

## クイックスタート

1. CC Switch を開き、**Claude / Codex / Gemini / Grok Build** に切り替えます。
2. プロバイダカードにカーソルを合わせ、**メーターアイコン**（使用制限）をクリック。
3. トグルをオンにし、**金額**または**トークン**を選択、通貨とリセット周期を
   決めて（金額モード）上限を入力して保存。
4. 該当アプリのローカルルート接管を有効にすると、制限の強制が始まります。
5. 「使用量をリセット」は統計の起点を現在に進めるだけで、ダッシュボードの履歴は
   完全に保持されます。

## 重要な境界線（必ずご確認ください）

- 制限は **CC Switch ローカルプロキシを経由するトラフィック**にのみ適用されます。
  直接接続は観測も遮断もできません（強制できない場合は UI が明示します）。
- 使用量は**ローカルでの観測値**です。プロバイダアカウントの全デバイス合計の
  グローバル割り当てではありません。
- 価格が未登録のモデルのコストは $0 として計上され、**かつ**ダイアログに明示的な
  警告が表示されます。CC Switch でカスタムモデル価格を設定できます。

## ソースからビルド

```bash
pnpm install
pnpm build        # 利用可能な Rust ツールチェーン（stable）が必要です
```

アーキテクチャと既知の制限（中国語）：[docs/development_log.md](docs/development_log.md)。
使用制限ナレッジベースは 4 言語で提供：[简体中文](docs/usage-limit-knowledge-base-zh.md) |
[English](docs/usage-limit-knowledge-base-en.md) |
[繁體中文](docs/usage-limit-knowledge-base-zh-TW.md) |
[日本語](docs/usage-limit-knowledge-base-ja.md)。

## クレジット

本プロジェクトは [@farion1231](https://github.com/farion1231) によるオープンソース
プロジェクト [**CC Switch**](https://github.com/farion1231/cc-switch)（MIT）を
ベースに構築されています — Claude Code / Codex / Gemini CLI などのための優れた
オールインワンマネージャーです。切り替え・プロキシ・使用量の基盤インフラの功績は
上流の作者とコントリビュータに帰属します。公式サイト：[ccswitch.io](https://ccswitch.io)。

使用制限機能自体はこのリポジトリで開発されました。

## ライセンス

[MIT](LICENSE) © 2026 Jiayi Huang — オリジナル CC Switch の MIT 表示を含みます。
