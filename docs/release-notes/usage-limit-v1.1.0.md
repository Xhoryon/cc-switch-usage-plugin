# CC Switch Usage Plugin v1.1.0

<div align="center">

**English** | [简体中文](#简体中文) | [繁體中文](#繁體中文) | [日本語](#日本語)

</div>

## English

Per-API-Key **usage limits** for CC Switch, enforced by the local proxy — now with **five currencies** and **automatic reset schedules**.

### What's new since v1.0.0

- **Money limits in 5 currencies** — USD, CNY, EUR, JPY, GBP. Every non-USD currency converts with a locally configurable USD→X exchange rate that you can edit right in the dialog (sensible defaults, no online rate API). Your previously configured CNY rate carries over.
- **Reset schedules** — keep the budget manual-only, or let usage restart automatically at local-time boundaries: hourly / daily / weekly (Mondays) / monthly (on the 1st). A spent window recovers by itself at the next boundary — no manual reset needed.

### The full picture

Every API Key can get its own usage cap — a money amount or a token count. Once the cap is reached, the CC Switch local proxy **rejects new requests before forwarding them** with a structured `usage_limit_reached` error (HTTP 429); already-sent requests finish normally. Limits bind to a SHA-256 fingerprint of the key (plaintext keys are never stored), with a four-state gauge icon on every card, live progress with warning states, and usage reset that never deletes history. Token counting matches the built-in Usage Dashboard exactly.

**Privacy:** everything is computed and stored locally — no telemetry, no online rate API, no account access.

**Install:** download the DMG below (macOS Apple Silicon), drag CC Switch.app to Applications, open a provider card's gauge icon to configure. Limits apply only to traffic routed through the CC Switch local proxy.

**Based on** [farion1231/cc-switch](https://github.com/farion1231/cc-switch) (MIT). MIT © 2026 Jiayi Huang.

## 简体中文

为 CC Switch 的每个 API Key 提供**使用限额**，由本地代理强制执行——现已支持**五种货币**与**自动重置周期**。

### 自 v1.0.0 以来的新内容

- **金额限额支持 5 种货币** — USD / CNY / EUR / JPY / GBP。非美元币种通过本地 USD→X 汇率换算，汇率可直接在对话框内修改（提供合理默认值，不联网获取）。此前配置的 CNY 汇率自动保留。
- **重置周期** — 可保持纯手动重置，也可让用量在本地时区边界自动重新统计：每小时 / 每天 / 每周（周一起）/ 每月（每月 1 日）。用满的窗口到下个边界自动恢复，无需手动干预。

### 完整功能

每个 API Key 都可以设置独立的使用上限——金额或 Token 数量。达到上限后，CC Switch 本地代理会在**转发前直接拒绝新请求**并返回结构化的 `usage_limit_reached` 错误（HTTP 429），已发出的请求正常完成。限额绑定到 API Key 的 SHA-256 指纹（不存明文），每张卡片提供四态仪表盘图标、实时进度与告警状态，用量重置永不删除历史。Token 统计口径与内置用量面板完全一致。

**隐私：** 一切计算与存储都在本机——无遥测、无在线汇率请求、不访问任何账户。

**安装：** 下载下方 DMG（macOS Apple Silicon），将 CC Switch.app 拖入应用程序，打开供应商卡片的仪表盘图标即可配置。限额仅对经过 CC Switch 本地代理的流量生效。

**基于** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。

## 繁體中文

為 CC Switch 的每個 API Key 提供**使用限額**，由本地代理強制執行——現已支援**五種貨幣**與**自動重置週期**。

### 自 v1.0.0 以來的新內容

- **金額限額支援 5 種貨幣** — USD / CNY / EUR / JPY / GBP。非美元幣種透過本地 USD→X 匯率換算，匯率可直接在對話框內修改（提供合理預設值，不連網取得）。此前設定的 CNY 匯率自動保留。
- **重置週期** — 可保持純手動重置，也可讓用量在本地時區邊界自動重新統計：每小時 / 每天 / 每週（週一起）/ 每月（每月 1 日）。用滿的視窗到下個邊界自動恢復，無需手動干預。

### 完整功能

每個 API Key 都可以設定獨立的使用上限——金額或 Token 數量。達到上限後，CC Switch 本地代理會在**轉發前直接拒絕新請求**並返回結構化的 `usage_limit_reached` 錯誤（HTTP 429），已發出的請求正常完成。限額綁定到 API Key 的 SHA-256 指紋（不存明文），每張卡片提供四態儀表板圖示、即時進度與警告狀態，用量重置永不刪除歷史。Token 統計口徑與內建用量面板完全一致。

**隱私：** 一切計算與儲存都在本機——無遙測、無線上匯率請求、不存取任何帳戶。

**安裝：** 下載下方 DMG（macOS Apple Silicon），將 CC Switch.app 拖入應用程式，開啟供應商卡片的儀表板圖示即可設定。限額僅對經過 CC Switch 本地代理的流量生效。

**基於** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。

## 日本語

CC Switch の各 API Key に**使用制限**を提供し、ローカルプロキシが強制します——今回は**5 通貨**と**自動リセット周期**に対応。

### v1.0.0 からの新機能

- **金額制限が 5 通貨に対応** — USD / CNY / EUR / JPY / GBP。USD 以外はローカルの USD→X 為替レートで換算し、レートはダイアログ内で直接編集できます（妥当な既定値付き、オンライン API 不要）。以前設定した CNY レートはそのまま引き継がれます。
- **リセット周期** — 手動リセットのみにするか、ローカル時刻の境界で使用量を自動的に再集計するかを選択：每時 / 毎日 / 毎週（月曜開始）/ 毎月（1 日開始）。使い切ったウィンドウは次の境界で自動回復し、手動操作は不要です。

### 機能全体

各 API Key に独立の使用上限——金額またはトークン数——を設定できます。上限に達すると、CC Switch ローカルプロキシは**転送前に新しいリクエストを拒否**し、構造化された `usage_limit_reached` エラー（HTTP 429）を返します。送信済みのリクエストは最後まで完了します。制限は API Key の SHA-256 フィンガープリントに紐付き（平文は保存しない）、カードごとに 4 状態のメーターアイコン、ライブ進捗と警告状態、履歴を壊さない使用量リセットを提供します。トークン計数は内蔵の使用量ダッシュボードと完全に一致します。

**プライバシー：** 計算も保存もすべてローカル——テレメトリなし、オンラインレート取得なし、アカウントアクセスなし。

**インストール：** 下の DMG をダウンロード（macOS Apple Silicon）し、CC Switch.app をアプリケーションにドラッグ、プロバイダカードのメーターアイコンから設定します。制限は CC Switch ローカルプロキシを経由するトラフィックにのみ適用されます。

**ベース：** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。

---

## Assets

- `CC.Switch_3.20.3_aarch64_usage-limit.dmg` — macOS（Apple Silicon）installer
- `cc-switch-usage-plugin-1.1.0-source.zip` — full source snapshot（与 tag 源码一致 / identical to the tagged source）
