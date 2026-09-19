<div align="center">

# CC Switch Usage Plugin

### 為 CC Switch 增加按 API Key 的使用限額（預算）——由本地代理強制執行

[![Release](https://img.shields.io/github/v/release/Xhoryon/cc-switch-usage-plugin?color=blue&label=release)](https://github.com/Xhoryon/cc-switch-usage-plugin/releases)
[![Platform](https://img.shields.io/badge/platform-macOS%20Apple%20Silicon-lightgrey.svg)](https://github.com/Xhoryon/cc-switch-usage-plugin/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

[English](README.md) | [简体中文](README_ZH.md) | 繁體中文 | [日本語](README_JA.md)

</div>

---

## 這是什麼？

**CC Switch Usage Plugin** 在開源供應商切換工具
[CC Switch](https://github.com/farion1231/cc-switch) 的基礎上增加了預算功能：
每個 API Key 都可以設定**使用上限**——金額（USD / CNY / EUR / JPY / GBP）
或 Token 數量。達到上限後，CC Switch 本地代理會**直接拒絕新的請求**並返回
明確錯誤，而不是只做統計。

## 功能特性

- **金額限額** — USD / CNY / EUR / JPY / GBP 五種主流貨幣；非美元幣種透過
  本地 USD→X 匯率換算，匯率可在對話框內自行修改（提供合理預設值，不連網）
- **重置週期** — 可保持純手動重置，也可讓用量在本地時區邊界自動重新統計：
  每小時 / 每天 / 每週（週一起）/ 每月（每月 1 日）。用滿的視窗到下個邊界
  自動恢復，無需手動干預
- **自訂視窗（v1.2.0 新增）** — 用量從打開限額的那一刻起算（絕不併入
  更早的歷史），並可按你自訂的每 N 小時 / 每 N 天自動重新統計
- **卡片用量徽標（v1.2.0 新增）** — 儀表板圖示旁的眼睛開關，一鍵在供應商
  卡片上直接顯示 `{百分比}` 與距下次重置的剩餘時間
- **Token 限額** — 與內建用量面板相同的標準化 Token 統計口徑，兩邊數字永遠一致
- **真正的強制執行** — 代理在**轉發前**檢查預算；達到限額後新請求快速失敗並返回結構化的
  `usage_limit_reached` 錯誤（HTTP 429）。已發出的請求允許正常完成，因此最後一個請求
  存在少量超出屬正常設計
- **綁定憑證而非供應商名稱** — 限額綁定到 API Key 的 SHA-256 指紋；絕不儲存明文 Key，
  更換 Key 後預算自動重新計算
- **每張卡片的即時狀態** — 儀表板圖示四態（關閉 / 正常 / 警告 ≥80% / 耗盡），
  設定對話框含進度條、剩餘額度與用量重置（統計視窗前移，歷史資料永不刪除）
- **誠實不誤導** — 流量未經過代理或模型缺少定價時，介面會明確提示，不假裝限額已生效
- **多語言** — 簡體中文、繁體中文、English、日本語

## 下載安裝

1. 從 [Releases](https://github.com/Xhoryon/cc-switch-usage-plugin/releases) 頁面下載
   `CC.Switch_3.20.3_aarch64_usage-limit.dmg`（macOS，Apple Silicon）。
2. 掛載 DMG，將 **CC Switch.app** 拖入「應用程式」。
3. 首次開啟：右鍵點擊應用並選擇**打開 → 打開**——本外掛未經 Apple 公證，
   macOS 會顯示一次性的「無法驗證開發者」提示。也可以在終端機執行一次
   `xattr -dr com.apple.quarantine "/Applications/CC Switch.app"`。

## 快速上手

1. 開啟 CC Switch，切換到 **Claude / Codex / Gemini / Grok Build**。
2. 游標停留在供應商卡片上，點擊**儀表板圖示**（使用限額）。
3. 打開開關，選擇**金額**或 **Token**，再選幣種與重置週期（金額模式），
   輸入上限並儲存。
4. 為該應用開啟本地路由接管後，限額開始強制執行。
5. 「重置使用量」只是把統計起點推進到當下，用量面板中的歷史資料完整保留。

## 重要邊界（請務必了解）

- 限額僅對**經過 CC Switch 本地代理的流量**生效；直連流量無法觀測也無法攔截
  （無法強制執行時介面會明確提示）。
- 用量是**本機觀測值**——它不是 Provider 帳戶在你所有裝置上的全域配額。
- 模型缺少定價時成本按 $0 計，**並且**對話框會顯式警示；可在 CC Switch 中設定
  自訂模型價格。

## 從原始碼建置

```bash
pnpm install
pnpm build        # 需要可用的 Rust 工具鏈（stable）
```

架構說明與已知限制（中文）：[docs/development_log.md](docs/development_log.md)。
使用限額知識庫提供四種語言版本：[簡體中文](docs/usage-limit-knowledge-base-zh.md) |
[English](docs/usage-limit-knowledge-base-en.md) |
[繁體中文](docs/usage-limit-knowledge-base-zh-TW.md) |
[日本語](docs/usage-limit-knowledge-base-ja.md)。

## 致謝

本專案基於 [@farion1231](https://github.com/farion1231) 的開源專案
[**CC Switch**](https://github.com/farion1231/cc-switch)（MIT）建置——一個優秀的
Claude Code / Codex / Gemini CLI 全能管理工具。底層的切換、代理與用量基礎設施
歸功於上游作者及所有貢獻者。原專案官網：[ccswitch.io](https://ccswitch.io)。

使用限額功能本身由本仓库開發。

## 授權條款

[MIT](LICENSE) © 2026 Jiayi Huang —— 含原 CC Switch 的 MIT 聲明。
