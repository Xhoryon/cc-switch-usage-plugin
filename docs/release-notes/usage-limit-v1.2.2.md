# CC Switch Usage Plugin v1.2.2

<div align="center">

**English** | [简体中文](#简体中文) | [繁體中文](#繁體中文) | [日本語](#日本語)

</div>

## English

Definitive fix for the usage-limit dialog layout on short windows, plus a
visual polish pass. No functional changes.

### Fixed

- **Dialog title pushed out of the window — for real this time.** The v1.2.1
  height constraint used the `dvh` CSS unit, which older WKWebViews (macOS
  12/13) do not parse — the whole constraint silently vanished and the
  dialog rendered at full content height, pushing its title out of view.
  The constraint now uses universally supported `vh`
  (`calc(100vh - 1rem)`), so the title and action buttons stay visible at
  any window size and the middle section scrolls.
- **"Rounded inside, straight line outside"** — the glass panels inside the
  dialog (12px corner radius) used to sit flush against the dialog frame
  (6px radius), which read as a broken nested-radius layout. The panels are
  now inset within the frame for a balanced look.

Functionality is otherwise identical to v1.2.1 (custom windows,
enable-time counting, five currencies, reset schedules, card usage badge,
hover hints).

**First launch:** right-click the app → Open → Open once (the plugin is not
notarized), or run
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"`.

**Based on** [farion1231/cc-switch](https://github.com/farion1231/cc-switch) (MIT). MIT © 2026 Jiayi Huang.

## 简体中文

彻底修复矮窗口下计费对话框的布局问题，并做了一次视觉打磨。功能无任何变更。

### 修复

- **对话框标题被顶出窗口——这次是真正的修复。** v1.2.1 的高度约束使用了
  `dvh` CSS 单位，旧版 WKWebView（macOS 12/13）无法解析——整条约束静默
  失效，对话框按全部内容高度渲染，标题被顶出视野。约束现在改用所有
  环境都支持的 `vh`（`calc(100vh - 1rem)`）：任意窗口尺寸下标题与操作
  按钮恒定可见，中间区域滚动。
- **「里面是圆角、外面是一条直线」** —— 对话框内的玻璃面板（12px 圆角）
  此前紧贴对话框外框（6px 圆角），嵌套圆角失衡、观感割裂。现在面板
  相对外框内缩，视觉平衡。

其余功能与 v1.2.1 完全一致（自定义窗口、开启时起算、五种货币、重置周期、
卡片用量徽标、悬停提示）。

**首次打开：** 右键应用 → 打开 → 打开一次（本插件未经公证），或执行
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"`。

**基于** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。

## 繁體中文

徹底修復矮視窗下計費對話框的佈局問題，並做了一次視覺打磨。功能無任何變更。

### 修復

- **對話框標題被頂出視窗——這次是真正的修復。** v1.2.1 的高度約束使用了
  `dvh` CSS 單位，舊版 WKWebView（macOS 12/13）無法解析——整條約束靜默
  失效，對話框按全部內容高度渲染，標題被頂出視野。約束現在改用所有
  環境都支援的 `vh`（`calc(100vh - 1rem)`）：任意視窗尺寸下標題與操作
  按鈕恆定可見，中間區域捲動。
- **「裡面是圓角、外面是一條直線」** —— 對話框內的玻璃面板（12px 圓角）
  此前緊貼對話框外框（6px 圓角），嵌套圓角失衡、觀感割裂。現在面板
  相對外框內縮，視覺平衡。

其餘功能與 v1.2.1 完全一致（自訂視窗、開啟時起算、五種貨幣、重置週期、
卡片用量徽標、懸停提示）。

**首次開啟：** 右鍵應用 → 打開 → 打開一次（本外掛未經公證），或執行
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"`。

**基於** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。

## 日本語

低いウィンドウでの使用制限ダイアログのレイアウトを根本修正し、視覚的な
磨きを加えました。機能の変更はありません。

### 修正

- **ダイアログのタイトルが窓外に出る問題——今回こそ完全修正。** v1.2.1 の
  高さ制約は `dvh` CSS 単位を使用していましたが、旧 WKWebView（macOS
  12/13）はこれを解析できず、制約全体が静かに無効化され、ダイアログは
  全内容高で描画されてタイトルが窓外に出ていました。制約は全環境で
  サポートされる `vh`（`calc(100vh - 1rem)`）に変更：どのウィンドウ
  サイズでもタイトルと操作ボタンが常に見え、中央部はスクロールします。
- **「内は角丸、外は直線」** — ダイアログ内のガラスパネル（12px 角丸）が
  外枠（6px 角丸）に接して配置され、ネストされた角丸が崩れて見えていました。
  パネルは外枠から内縮され、視覚的にバランスの取れた見た目になりました。

その他の機能は v1.2.1 と同一です（カスタムウィンドウ、有効化時点からの
集計、5 通貨、リセット周期、カード使用量バッジ、ホバー説明）。

**初回起動：** アプリを右クリック →「開く」→「開く」を 1 回（本プラグインは
公証されていません）、または
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"` を実行。

**ベース：** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。
