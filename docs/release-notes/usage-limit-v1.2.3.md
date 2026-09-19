# CC Switch Usage Plugin v1.2.3

<div align="center">

**English** | [简体中文](#简体中文) | [繁體中文](#繁體中文) | [日本語](#日本語)

</div>

## English

The definitive fix for the usage-limit dialog being pushed out of short
windows — with the actual root cause proven this time. No functional changes.

### Fixed

- **Dialog pushed out of short windows (root cause found and eliminated).**
  The previous two attempts adjusted the dialog's `max-height`, but the real
  culprit was elsewhere: a centering transform (`translate-y-[-50%]`) from
  the shared dialog component survived every override (a tailwind-merge
  deduplication blind spot for negative arbitrary values) and shifted the
  dialog up by half its height — cancelling both earlier fixes regardless of
  how the height was constrained. The dialog is now positioned with
  **top/bottom anchoring via inline styles**: it is physically pinned between
  2.5rem from the window top and 0.75rem from the bottom, so its height is
  always exactly the window minus 52px and it cannot overflow by construction.
- **The enable card and the config card sat too close together** — spacing
  between them has been increased.

Everything else is identical to v1.2.2 (custom windows, enable-time
counting, five currencies, reset schedules, card usage badge, hover hints,
icon tooltips).

**First launch:** right-click the app → Open → Open once (the plugin is not
notarized), or run
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"`.

**Based on** [farion1231/cc-switch](https://github.com/farion1231/cc-switch) (MIT). MIT © 2026 Jiayi Huang.

## 简体中文

彻底修复矮窗口下计费对话框被顶出的问题——这次找到了真正的根因。功能无任何变更。

### 修复

- **对话框被顶出短窗口（根因实锤并消除）。** 此前两次修复都在调整对话框的
  `max-height`，但真正的元凶在别处：共享对话框组件的居中平移
  （`translate-y-[-50%]`）在每次覆盖后都幸存了下来（tailwind-merge 对负值
  任意值不去重的盲区），把对话框向上平移了半高——无论高度如何约束都会被
  抵消。对话框现在改用**内联样式的 top/bottom 双锚定定位**：物理上被夹在
  距窗口顶部 2.5rem、距底部 0.75rem 之间，高度恒等于窗口减 52px，
  结构上不可能溢出。
- **启用卡与配置卡两块卡片挨得太近** —— 已拉开间距。

其余功能与 v1.2.2 完全一致（自定义窗口、开启时起算、五种货币、重置周期、
卡片用量徽标、悬停提示、图标 Tooltip）。

**首次打开：** 右键应用 → 打开 → 打开一次（本插件未经公证），或执行
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"`。

**基于** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。

## 繁體中文

徹底修復矮視窗下計費對話框被頂出的問題——這次找到了真正的根因。功能無任何變更。

### 修復

- **對話框被頂出短視窗（根因實錘並消除）。** 此前兩次修復都在調整對話框的
  `max-height`，但真正的元兇在別處：共享對話框元件的置中平移
  （`translate-y-[-50%]`）在每次覆蓋後都倖存了下來（tailwind-merge 對負值
  任意值不去重的盲區），把對話框向上平移了半高——無論高度如何約束都會被
  抵消。對話框現在改用**內聯樣式的 top/bottom 雙錨定定位**：物理上被夾在
  距視窗頂部 2.5rem、距底部 0.75rem 之間，高度恆等於視窗減 52px，
  結構上不可能溢出。
- **啟用卡與設定卡兩塊卡片挨得太近** —— 已拉開間距。

其餘功能與 v1.2.2 完全一致（自訂視窗、開啟時起算、五種貨幣、重置週期、
卡片用量徽標、懸停提示、圖示 Tooltip）。

**首次開啟：** 右鍵應用 → 打開 → 打開一次（本外掛未經公證），或執行
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"`。

**基於** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。

## 日本語

低いウィンドウで使用制限ダイアログが押し出される問題を根本修正——今回こそ
真の根因を特定しました。機能の変更はありません。

### 修正

- **ダイアログが低いウィンドウから押し出される（根因を特定し解消）。**
  これまでの 2 回の修正はダイアログの `max-height` を調整していましたが、
  真の原因は別の場所にありました：共有ダイアログコンポーネントの中央寄せ
  transform（`translate-y-[-50%]`）が、tailwind-merge が負の任意値を
  デデュープしない盲区のため、毎回の上書き後も生き残り、ダイアログを半分の
  高さだけ上へ移動させていました——高さの制約方法にかかわらず相殺されて
  いました。ダイアログは now **インライン style による top/bottom
  アンカー配置**に変更：ウィンドウ上部から 2.5rem、下部から 0.75rem の間に
  物理的に固定され、高さは常にウィンドウ − 52px となり、構造的にはみ出し
  不可能です。
- **有効化カードと設定カードが近すぎた** — 間隔を広げました。

その他の機能は v1.2.2 と同一です（カスタムウィンドウ、有効化時点からの
集計、5 通貨、リセット周期、カード使用量バッジ、ホバー説明、アイコン
Tooltip）。

**初回起動：** アプリを右クリック →「開く」→「開く」を 1 回（本プラグインは
公証されていません）、または
`xattr -dr com.apple.quarantine "/Applications/CC Switch.app"` を実行。

**ベース：** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。
