# CC Switch Usage Plugin v1.2.0

<div align="center">

**English** | [简体中文](#简体中文) | [繁體中文](#繁體中文) | [日本語](#日本語)

</div>

## English

Budget windows redesigned around **when you turn the limit on**, plus
custom rolling windows and on-card usage badges.

### Fixed

- **Counting now starts when you enable the limit** — re-enabling a limit
  (or changing its reset schedule) restarts the statistics window at that
  moment; usage from before is never pulled in. Previously a daily schedule
  could include everything that had been used earlier the same day.
- **Dialog clipped on small windows** — the config dialog is now scrollable,
  so nothing is cut off when the CC Switch window is small.
- **CC Switch could not be dragged while the dialog was open** — the dialog
  is now non-modal: the window stays draggable (and the background stays
  clickable) while it is open.

### New

- **Custom windows** — beyond the calendar schedules (hourly / daily /
  weekly / monthly), pick **Custom** and define your own rolling window of
  N hours or N days. The window starts the moment you save and restarts
  every N hours/days.
- **Card usage badge** — a new eye toggle next to the gauge icon shows the
  usage right on the provider card: `{percent}%` (colored by state) plus
  the time until the next reset (e.g. `⏱2d4h`). The preference is
  remembered per provider.

**First launch (from v1.1.1):** the plugin is not notarized — right-click
the app → Open → Open once, or run
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"`.

**Based on** [farion1231/cc-switch](https://github.com/farion1231/cc-switch) (MIT). MIT © 2026 Jiayi Huang.

## 简体中文

预算窗口围绕「打开限额的那一刻」重新设计，新增自定义滚动窗口与卡片用量徽标。

### 修复

- **统计从打开限额的那一刻起算** —— 重新启用限额（或更改其重置周期）会把
  统计窗口重置到当下；此前的用量绝不并入。此前「每天」周期可能把当天早些
  时候的用量一并计入。
- **小窗口下计费对话框被裁切** —— 配置对话框现在可以滚动，小窗口下不再
  有内容被裁掉。
- **打开计费窗口时无法拖动 CC Switch** —— 对话框改为非模态：窗口保持
  可拖动（背景也可点击）。

### 新增

- **自定义窗口** —— 在日历周期（每小时 / 每天 / 每周 / 每月）之外，可选择
  **自定义**并定义 N 小时或 N 天的滚动窗口；窗口从保存那一刻开始，每
  N 小时/天自动重置。
- **卡片用量徽标** —— 仪表盘图标旁新增眼睛开关，直接在供应商卡片上显示
  用量：`{百分比}`（按状态着色）+ 距下次重置时间（如 `⏱2d4h`）。偏好按
  供应商记忆。

**首次打开（v1.1.1 起）：** 本插件未经公证——右键应用 → 打开 → 打开一次，
或执行 `xattr -dr com.apple.quarantine "/Applications/CC Switch.app"`。

**基于** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。

## 繁體中文

預算視窗圍繞「打開限額的那一刻」重新設計，新增自訂滾動視窗與卡片用量徽標。

### 修復

- **統計從打開限額的那一刻起算** —— 重新啟用限額（或更改其重置週期）會把
  統計視窗重置到當下；此前的用量絕不併入。此前「每天」週期可能把當天早些
  時候的用量一併計入。
- **小視窗下計費對話框被裁切** —— 設定對話框現在可以捲動，小視窗下不再
  有內容被裁掉。
- **開啟計費視窗時無法拖動 CC Switch** —— 對話框改為非模態：視窗保持
  可拖動（背景也可點擊）。

### 新增

- **自訂視窗** —— 在日曆週期（每小時 / 每天 / 每週 / 每月）之外，可選擇
  **自訂**並定義 N 小時或 N 天的滾動視窗；視窗從儲存那一刻開始，每
  N 小時/天自動重置。
- **卡片用量徽標** —— 儀表板圖示旁新增眼睛開關，直接在供應商卡片上顯示
  用量：`{百分比}`（按狀態著色）+ 距下次重置時間（如 `⏱2d4h`）。偏好按
  供應商記憶。

**首次開啟（v1.1.1 起）：** 本外掛未經公證——右鍵應用 → 打開 → 打開一次，
或執行 `xattr -dr com.apple.quarantine "/Applications/CC Switch.app"`。

**基於** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。

## 日本語

予算ウィンドウを「制限を有効にした瞬間」を基準に再設計し、カスタム
ローリングウィンドウとカード使用量バッジを追加しました。

### 修正

- **集計は有効化した瞬間から開始** — 制限を再有効化（またはリセット周期の
  変更）すると、集計ウィンドウはその瞬間にリセットされ、それ以前の使用量は
  決して取り込まれません。以前の「毎日」周期は当日の早い時間の使用量まで
  含めてしまうことがありました。
- **小さいウィンドウで設定ダイアログが切れる** — 設定ダイアログは
  スクロール可能になり、CC Switch のウィンドウが小さくても切り取られません。
- **ダイアログを開くと CC Switch をドラッグできない** — ダイアログは
  モーダルなしに変更され、開いたままでもウィンドウのドラッグ（および
  背景のクリック）が可能です。

### 新機能

- **カスタムウィンドウ** — 日曆周期（每時 / 毎日 / 毎週 / 毎月）に加え、
  **カスタム**を選んで N 時間 / N 日のローリングウィンドウを定義できます。
  ウィンドウは保存した瞬間から始まり、N 時間/日ごとにリセットされます。
- **カード使用量バッジ** — メーターアイコンの横に目のトグルを追加。
  プロバイダカードに直接使用量を表示：`{百分比}`（状態色）+ 次回リセット
  までの時間（例：`⏱2d4h`）。設定はプロバイダごとに記憶されます。

**初回起動（v1.1.1 から）：** 本プラグインは公証されていません——アプリを
右クリック →「開く」→「開く」を 1 回、または
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"` を実行。

**ベース：** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。
