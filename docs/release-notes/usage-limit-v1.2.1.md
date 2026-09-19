# CC Switch Usage Plugin v1.2.1

<div align="center">

**English** | [简体中文](#简体中文) | [繁體中文](#繁體中文) | [日本語](#日本語)

</div>

## English

UI polish for the usage-limit feature — no functional changes.

### Fixed

- **Dialog title pushed out of the window on short windows** — the config
  dialog now has an explicit height constraint (`calc(100dvh - 1rem)`) with
  the scrollable content area inside it, so the title and the action buttons
  stay visible at any window size.
- **Custom window row overflowed in narrow dialogs** — the hours/days
  segment control no longer gets clipped next to the length input.
- **No hover hints on the provider card icon row** — every action icon
  (edit, duplicate, connectivity check, plan usage, usage-limit gauge,
  terminal, delete) now explains itself in a tooltip on hover. The **eye
  icon** is the card-usage badge toggle introduced in v1.2.0: click it to
  show `{percent}%` and the time to the next reset right on the provider
  card; its tooltip explains this too.

Functionality is otherwise identical to v1.2.0 (custom windows,
enable-time counting, five currencies, reset schedules).

**First launch:** right-click the app → Open → Open once (the plugin is not
notarized), or run
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"`.

**Based on** [farion1231/cc-switch](https://github.com/farion1231/cc-switch) (MIT). MIT © 2026 Jiayi Huang.

## 简体中文

使用限额功能的界面细节修复——功能无任何变更。

### 修复

- **矮窗口下对话框标题被顶出窗口** —— 配置对话框现在有明确的高度约束
  （`calc(100dvh - 1rem)`），滚动内容区在其内部生效，任意窗口尺寸下标题
  与底部按钮都保持可见。
- **窄对话框中自定义窗口行溢出** —— 小时/天分段控件不再被裁切。
- **供应商卡片图标行没有悬停提示** —— 每个操作图标（编辑、复制、连通
  检测、套餐用量、限额仪表盘、终端、删除）悬停即显示功能说明。**眼睛
  图标**是 v1.2.0 引入的卡片用量徽标开关：点击即可在供应商卡片上显示
  `{百分比}` 与距下次重置时间；悬停提示中也有说明。

其余功能与 v1.2.0 完全一致（自定义窗口、开启时起算、五种货币、重置周期）。

**首次打开：** 右键应用 → 打开 → 打开一次（本插件未经公证），或执行
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"`。

**基于** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。

## 繁體中文

使用限額功能的介面細節修復——功能無任何變更。

### 修復

- **矮視窗下對話框標題被頂出視窗** —— 設定對話框現在有明確的高度約束
  （`calc(100dvh - 1rem)`），捲動內容區在其內部生效，任意視窗尺寸下標題
  與底部按鈕都保持可見。
- **窄對話框中自訂視窗列溢出** —— 小時/天分段控制項不再被裁切。
- **供應商卡片圖示列沒有懸停提示** —— 每個操作圖示（編輯、複製、連通
  檢測、套餐用量、限額儀表板、終端機、刪除）懸停即顯示功能說明。**眼睛
  圖示**是 v1.2.0 引入的卡片用量徽標開關：點擊即可在供應商卡片上顯示
  `{百分比}` 與距下次重置時間；懸停提示中也有說明。

其餘功能與 v1.2.0 完全一致（自訂視窗、開啟時起算、五種貨幣、重置週期）。

**首次開啟：** 右鍵應用 → 打開 → 打開一次（本外掛未經公證），或執行
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"`。

**基於** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。

## 日本語

使用制限機能の UI 修正——機能自体の変更はありません。

### 修正

- **低いウィンドウでダイアログのタイトルが窓外に出る** — 設定ダイアログに
  明示的な高さ制約（`calc(100dvh - 1rem)`）を付け、スクロール領域をその
  内側で機能させました。どのウィンドウサイズでもタイトルと操作ボタンが
  常に見えます。
- **狭いダイアログでカスタム窗長列が溢れる** — 時間/日セグメントが長さ
  入力の横で切れなくなりました。
- **カードのアイコン列にホバー説明がない** — 各操作アイコン（編集、複製、
  接続チェック、プラン使用量、制限メーター、ターミナル、削除）にホバーで
  説明が表示されます。**目のアイコン**は v1.2.0 のカード使用量バッジの
  トグルです：クリックするとカードに `{百分比}` と次回リセットまでの時間を
  表示します。ホバー説明にも記載されています。

その他の機能は v1.2.0 と同一です（カスタムウィンドウ、有効化時点からの
集計、5 通貨、リセット周期）。

**初回起動：** アプリを右クリック →「開く」→「開く」を 1 回（本プラグインは
公証されていません）、または
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"` を実行。

**ベース：** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。
