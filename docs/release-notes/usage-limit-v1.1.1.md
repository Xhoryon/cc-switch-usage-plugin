# CC Switch Usage Plugin v1.1.1

<div align="center">

**English** | [简体中文](#简体中文) | [繁體中文](#繁體中文) | [日本語](#日本語)

</div>

## English

Installer fixes — no functional changes to the usage-limit feature.

### Fixed

- **"App is damaged" on first launch** — previous installers shipped the app
  bundle without a complete code signature; macOS Gatekeeper therefore treated
  downloaded copies as damaged. The bundle is now properly (ad-hoc) signed, so
  it launches normally after you approve it once.
- **Drag-to-Applications not registering** (app missing from Launchpad/Dock
  after the first drag, or a "replace?" prompt on a second drag) — same root
  cause as above; with a valid signature the first drag registers normally.
- **Stray `.VolumeIcon.icns` file visible in the DMG window** — removed; the
  installer window now shows only the app and the Applications link.

### First launch note (one time)

The plugin is **not notarized** (that requires a paid Apple Developer
account). On first launch macOS may say *""CC Switch" cannot be opened
because the developer cannot be verified"*. This is expected and safe to
approve — either **right-click the app → Open → Open**, or run once in
Terminal:

```bash
xattr -dr com.apple.quarantine "/Applications/CC Switch.app"
```

Everything else works exactly as in v1.1.0 (five-currency money limits,
token limits, reset schedules, proxy enforcement).

**Based on** [farion1231/cc-switch](https://github.com/farion1231/cc-switch) (MIT). MIT © 2026 Jiayi Huang.

## 简体中文

安装包修复——使用限额功能本身无任何变更。

### 修复

- **首次打开提示「文件已损坏」** —— 之前的安装包中应用缺少完整的代码签名，
  macOS Gatekeeper 会把下载副本判为损坏。现在应用包已完整签名（ad-hoc），
  批准一次后即可正常打开。
- **第一次拖入 Applications 不生效**（启动台/程序坞不显示，第二次拖拽才提示
  「已存在，是否替换」）—— 与上一条同源；签名有效后首次拖拽即可正常注册。
- **DMG 窗口出现杂散的 `.VolumeIcon.icns` 文件** —— 已移除；安装窗口现在
  只显示应用与 Applications 链接。

### 首次打开说明（仅需一次）

本插件**未经 Apple 公证**（公证需要付费开发者账号）。首次打开时 macOS 可能
提示 *"无法打开"CC Switch"，因为无法验证开发者"*。这是预期行为、可以安全
放行——**右键点击应用 → 打开 → 打开**，或在终端执行一次：

```bash
xattr -dr com.apple.quarantine "/Applications/CC Switch.app"
```

其余功能与 v1.1.0 完全一致（五币种金额限额、Token 限额、重置周期、代理强制执行）。

**基于** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。

## 繁體中文

安裝包修復——使用限額功能本身無任何變更。

### 修復

- **首次開啟提示「檔案已損毀」** —— 之前的安裝包中應用缺少完整的程式碼簽章，
  macOS Gatekeeper 會把下載副本判定為損毀。現在應用套件已完整簽章（ad-hoc），
  批准一次後即可正常開啟。
- **第一次拖入 Applications 不生效**（啟動台/程式塢不顯示，第二次拖拽才提示
  「已存在，是否取代」）—— 與上一條同源；簽章有效後首次拖拽即可正常註冊。
- **DMG 視窗出現雜散的 `.VolumeIcon.icns` 檔案** —— 已移除；安裝視窗現在
  只顯示應用與 Applications 連結。

### 首次開啟說明（僅需一次）

本外掛**未經 Apple 公證**（公證需要付費開發者帳號）。首次開啟時 macOS 可能
提示 *"無法打開"CC Switch"，因為無法驗證開發者"*。這是預期行為、可以安全
放行——**右鍵點擊應用 → 打開 → 打開**，或在終端機執行一次：

```bash
xattr -dr com.apple.quarantine "/Applications/CC Switch.app"
```

其餘功能與 v1.1.0 完全一致（五幣種金額限額、Token 限額、重置週期、代理強制執行）。

**基於** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。

## 日本語

インストーラ修正——使用制限機能自体の変更はありません。

### 修正

- **初回起動で「ファイルが破損しています」** —— 以前のインストーラはアプリ
  バンドルに完全なコード署名がなく、macOS Gatekeeper がダウンロード副本を
  破損扱いしていました。バンドルは現在正しく（ad-hoc）署名されており、一度
  承認すれば正常に起動します。
- **Applications への初回ドラッグが反映されない**（起動パッド/ドックに表示
  されず、2 回目のドラッグで「既に存在します」と出る）—— 上記と同根因。
  署名が有効なら初回ドラッグで正常に登録されます。
- **DMG ウィンドウに迷子の `.VolumeIcon.icns` が表示される** —— 削除しました。
  インストーラウィンドウにはアプリと Applications リンクのみが表示されます。

### 初回起動について（1 回のみ）

本プラグインは **Apple の公証を受けていません**（公証には有料の開発者
アカウントが必要です）。初回起動時に *「"CC Switch" を検証できないため
開けません」* と表示されることがあります。これは想定内で、安全に許可でき
ます——**アプリを右クリック → 開く → 開く**、またはターミナルで 1 回：

```bash
xattr -dr com.apple.quarantine "/Applications/CC Switch.app"
```

その他の機能は v1.1.0 と完全に同じです（5 通貨の金額制限、トークン制限、
リセット周期、プロキシ強制执行）。

**ベース：** [farion1231/cc-switch](https://github.com/farion1231/cc-switch)（MIT）。MIT © 2026 Jiayi Huang。
