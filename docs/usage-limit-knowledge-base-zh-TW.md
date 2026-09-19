<!-- Language / 语言: [简体中文](usage-limit-knowledge-base-zh.md) | [English](usage-limit-knowledge-base-en.md) | [繁體中文](usage-limit-knowledge-base-zh-TW.md) | [日本語](usage-limit-knowledge-base-ja.md) -->

# 使用限額（Budget Limit）知識庫

> 適用版本：**V1.2.0**（公開發布，2026-09-18；安裝包修復版，功能與 V1.1.0
> 一致，基座 CC Switch 3.20.3）。各迭代明細見 `docs/development_log.md`
> 對應條目。本文是整合視圖，面向後續維護者，回答「它是什麼、為什麼這樣
> 設計、改哪裡要小心什麼」。

---

## 1. 功能概述

每個 Provider 的靜態 API Key 可以單獨設定一個最大使用量，達到後 **Local
Proxy 在轉發前拒絕新請求**（真正的 enforcement，不是僅統計）。

| 能力 | 說明 |
| --- | --- |
| 限制方式 | 金額（USD / CNY / EUR / JPY / GBP）或 Token 數，同一時間二選一 |
| enforcement 位置 | Local Proxy `forward_with_retry_inner` 每個 provider attempt 轉發前 |
| 統計來源 | 重複使用 Usage SSOT（`proxy_request_logs`），不建平行統計 |
| credential 綁定 | SHA-256 不可逆指紋，任何路徑不落明文 API Key |
| 重置週期（V1.0.1） | 不重置 / 每小時 / 每天 / 每週 / 每月，本地時區邊界懶滾動 |
| 多幣種（V1.0.2） | 非 USD 幣種按本地可調匯率換算（人工調節，不連網） |
| 手動重置 | `usage_start_at` 推進到當下，不刪任何歷史 Usage |

**核心邊界（務必隨功能一起傳播的認知）**：

> Budget enforcement only applies to traffic routed through CC Switch Local
> Proxy. 未開啟代理接管的流量觀測不到、攔不住；UI 以
> `enforcement = proxy_disabled` 明示，不假裝 Hard Limit 生效。
> 同一 Key 在其他裝置/直連的用量不計入本機預算（usage is local observation）。

---

## 2. 總體架構

```
設定/展示鏈路
  Provider Card（ProviderActions 圖示按鈕）
    └─ UsageLimitButton（Gauge 圖示，useUsageLimitStatus 自取狀態）
         └─ UsageLimitDialog（開關 / 限制方式 / 幣種 / 金額 / 重置週期 / 進度）
              └─ Tauri Command（get_usage_limit_status / save_usage_limit /
                 reset_usage_limit / get_exchange_rate / set_exchange_rate）
                   └─ Database impl（services/usage_limit.rs）
                        ├─ api_key_limits 表（設定 + 視窗起點 + 週期）
                        └─ proxy_request_logs 聚合（用量，SSOT 重複使用）

轉發執行鏈路
  Client Request
    └─ CC Switch Local Proxy（forwarder.rs）
         └─ Budget Guard（per provider attempt，forward() 之前）
              ├─ 未啟用 → 放行（零開銷路徑是一次 PK 查詢）
              ├─ credential 指紋不一致 → 重綁 + 視窗重置為當下
              ├─ 週期邊界已越過（V1.0.1）→ 懶滾動視窗起點
              └─ used >= limit → ProxyError::BudgetExhausted（429，不發上游）
                   └─ 回應完成後 Usage Accounting（log_with_calculation，
                      攜帶 credential_fingerprint 落 proxy_request_logs）
```

分層紀律：React → Tauri API → Command → Service（Database impl）→ DAO →
SQLite。React 不直接存取資料庫；金額/Token 聚合只在 Rust 側。

---

## 3. 資料模型

### 3.1 `api_key_limits`（V1.0 建立，V1.0.1 加欄位）

```sql
CREATE TABLE api_key_limits (
    provider_id            TEXT NOT NULL,
    app_type               TEXT NOT NULL,
    credential_fingerprint TEXT NOT NULL,      -- SHA-256 hex，絕不存明文 key
    enabled                INTEGER NOT NULL DEFAULT 0,
    limit_type             TEXT NOT NULL CHECK (limit_type IN ('money', 'token')),
    currency               TEXT,                    -- token 模式為 NULL；V1.0.2 起無 CHECK
    limit_amount           TEXT NOT NULL,      -- 金額十進位字串 / token 正整數字串
    usage_start_at         INTEGER NOT NULL,   -- 統計視窗起點（unix 秒）
    reset_period           TEXT NOT NULL DEFAULT 'never',  -- V1.0.1
    window_length          INTEGER,            -- V1.2.0: custom window length
    window_unit            TEXT,               -- V1.2.0: hours | days
    created_at             INTEGER NOT NULL,
    updated_at             INTEGER NOT NULL,
    PRIMARY KEY (provider_id, app_type),
    FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type)
        ON DELETE CASCADE                       -- Provider 刪除級聯清理
);
```

- 每個 Provider 至多一條設定（PK 決定）。
- `limit_amount` 用字串避免 SQLite REAL 浮點誤差進入限額判斷。
- `reset_period` 刻意**無 CHECK**：遷移路徑（ALTER ADD COLUMN）與全新建表
  行為完全一致；值域由儲存路徑校驗，讀取側未知值回退 `never` 並告警。
- `currency` 的 CHECK 在 v22 遷移中**移除**（V1.0.2 多幣種）：SQLite 無法修改
  既有 CHECK，遷移為標準表重建（建新表→拷貝→刪舊→改名，冪等性靠
  `sqlite_master.sql` 是否仍含 `CHECK (currency` 判斷）；值域由
  `LimitCurrency::parse` 在儲存路徑校驗、讀取側未知值回退 USD。**後續擴展
  幣種不再需要遷移**。
- 本表**不冗餘儲存累計用量**（避免雙 SSOT），用量始終從
  `proxy_request_logs` 即時聚合。

### 3.2 用量 SSOT 聚合謂詞

所有預算統計共用同一 WHERE：

```sql
WHERE provider_id = ? AND app_type = ?
  AND credential_fingerprint = ?
  AND created_at >= usage_start_at   -- 有效視窗起點（見 §4.3）
  AND data_source = 'proxy'          -- 會話同步列/直連列不參與預算
```

- **Token**：normalized total = fresh input + output + cache_creation +
  cache_read（`services/sql_helpers::fresh_input_sql`，與 Usage Dashboard
  同一規則；`input_token_semantics` 區分快取語義）。
- **金額**：累加已落庫的 `total_cost_usd` 十進位字串。兩級通道：
  - 快速通道：`SUM(CAST(... AS REAL))` 明顯低於閾值（相對邊距，見 §4.2）→
    O(1) 放行；
  - 精確通道：貼近/超過閾值時取原始字串逐列 `rust_decimal` 求和複核，
    避免 0.1+0.2 ≠ 0.3 的浮點誤判。

### 3.3 settings 鍵

| 鍵 | 含義 | 預設 |
| --- | --- | --- |
| `usage_limit_usd_cny_rate` | 本地 USD→CNY 匯率（沿用 V1.0 舊鍵，升級不丟） | `7.2` |
| `usage_limit_usd_eur_rate` | 本地 USD→EUR 匯率 | `0.92` |
| `usage_limit_usd_jpy_rate` | 本地 USD→JPY 匯率 | `150` |
| `usage_limit_usd_gbp_rate` | 本地 USD→GBP 匯率 | `0.79` |

非法存量匯率回退該幣種預設並告警；匯率必須 > 0 且 < 1,000,000；USD 是內部
計價基準，恆為 1 且不可設定。前端在 Dialog 內修改並隨儲存持久化（人工調節，
不連網取得）。

---

## 4. 核心機制

### 4.1 credential 指紋與輪換

- `credential_fingerprint(api_key)` = SHA-256 小寫 hex（64 位），不可逆。
- 解析重複使用代理轉發同款 `adapter.extract_auth`，保證「設定綁定的指紋」與
  「實際轉發的指紋」同源。
- 展示一律遮蔽：`mask_credential` → `sk-****ABCD`（≤8 字元 → `****`）。
- OAuth 類 Provider（codex_oauth / xai_oauth / managed account / copilot）
  無靜態 Key → `unsupported_credential`，不允許按 Key 限額。
- **換 Key**：guard / 儲存發現指紋不一致 → 重綁新指紋 + 視窗起點重置為
  當下。舊 Key 的歷史用量不併入新 Key；讀介面（status）偵測到輪換時按
  「目前 credential」統計但**不回寫**（無副作用）。

### 4.2 判定語義

- Token：`used_normalized_total >= limit` → BLOCK（整數比較，無精度問題）。
- 金額：`limit_in_usd = limit_amount / rate` 折回 USD 與成本同域 `Decimal`
  比較；展示側 `used_in_currency = used_usd × rate`。`rate` 為 USD→限額幣種
  匯率（V1.0.2：5 幣種各自獨立，USD 恆 1，單一路徑）。
- 快速通道邊距為 `max(1e-6, limit_in_usd × 1e-12)`——大限額下絕對 1e-6
  USD 不足以覆蓋 f64 累積誤差（審查 P2-3）。
- BLOCK 回傳 429（見 §6），上游零流量；failover 鏈中
  `BudgetExhausted` 屬 NonRetryable，不消耗其他 Provider 的嘗試名額。
- DB 讀取/聚合失敗 → fail-open（warn + 放行），與熔斷器降級同思路；
  判定失敗絕不能把所有請求攔死。
- **Overshoot 是設計內行為**：已允許發出的請求允許完成，usage 在回應
  結束後落庫，最後一個請求可能小幅超出。禁止中斷進行中的 SSE/串流。

### 4.3 重置週期與懶滾動（V1.0.1 引入，V1.2.0 重做語義）

**視窗起點（V1.2.0 起）**：首次建立、換 Key、**關→開**、**啟用狀態下改變重置設定**時，`usage_start_at` 一律設為「當下」——打開選項後開始計算，開啟前的歷史用量不併入。儲存路徑不再做日曆回對齊（V1.0.1 的 daily 回對齊行為已移除）。

`ResetPeriod`：
| 週期 | 滾動方式 | 例（2026-09-18 週五 15:27 啟用） |
| --- | --- | --- |
| never | 不滾動，僅手動重置 | — |
| hourly | 本地整點邊界 | 15:00 起算，16:00 重置 |
| daily | 本地零點邊界 | 15:00 起算，次日 00:00 重置 |
| weekly | 本地週一零點（ISO 週） | 起算至下週一 |
| monthly | 本地 1 日零點 | 起算至下月 1 日 |
| **custom**（V1.2.0） | **錨點 + 整數倍窗長**（N 小時/天） | 15:27 起算，每 N 小時/天滾動 |

- 日曆週期有效起點 = `max(usage_start_at, 目前日曆邊界)`（啟用後未到下個
  邊界時保持啟用時刻，跨邊界後滾動）。
- custom 有效起點 = `錨點 + floor((now-錨點)/窗長) × 窗長`；窗長存於
  `window_length`（1~10000）+ `window_unit`（hours/days），僅 custom 可攜帶。
- 統一入口 `effective_window_start(row, now)` / `next_window_reset(...)`。

三個應用點:
1. **guard（寫路徑）**：先滾動後判定；滾動以一列 UPDATE 持久化
   （best-effort，失敗僅告警，判定仍按新起點）。被限額的視窗跨過邊界後
   在同一次 guard 呼叫內恢復放行。
2. **status（讀路徑）**：同樣即時計算並回傳 `resetPeriod` / `windowLength` /
   `windowUnit` / `nextResetAt`，但**不回寫**——讀介面保持無副作用。
3. **save（寫路徑，V1.2.0）**：週期值缺省視為 `never`，非法值拒絕入庫；
   視窗起點按上方四種情形設定（關→開與重置設定變更 → 當下），不再回對齊
   日曆邊界。custom 的長度/單位強校驗（1~10000、hours/days），非 custom
   不得攜帶視窗欄位。


### 4.4 並行模型

判定讀與用量寫都在 `Database.conn` 的 Mutex 下序列化，無 read→write 競態
丟寫（單測：8 執行緒 × 25 列並行寫後聚合 == 寫入總量）。真實用量在回應後
才可知，同時進行的多個請求仍可能少量 overshoot——設計內，不為理論零
overshoot 破壞轉發並行。週期滾動寫在同一 Mutex 下，無額外競態面。

### 4.5 未知定價（金額模式可靠性）

視窗內「有 token 用量但 `total_cost_usd = '0'`」的請求，逐 `pricing_model`
核對模型定價表：**有定價但成本 0**（免費模型）→ 正常；**無定價且非佔位
模型** → 計入 `unpricedRequestCount` / `unpricedModels`，UI 顯示警示條並
引導設定 custom pricing。缺價請求成本按 0 聚合，但**絕不假裝 Hard Limit
精確覆蓋了它們**（UI 明示）。

---

## 5. 狀態機與 UI

`BudgetStatus.state`（後端計算，前端只渲染）：

| state | 條件 | 圖示 | 進度條 |
| --- | --- | --- | --- |
| off | 未啟用或無設定 | muted 灰 | — |
| active | 已啟用且 < 80% | emerald | emerald |
| warning | ≥ 80% 且 < 100% | amber | amber |
| exhausted | ≥ 100% | red | red（數值不 clamp，進度條視覺封頂 100%） |

- `percent_used` 不 clamp（可顯示 104.2%）。
- enforcement 三態：`active` / `proxy_disabled`（該應用代理接管未開啟，
  顯示警示條）/ `unsupported_credential`（OAuth，禁止設定）。
- Dialog 結構：關閉態只有開關列（簡潔）；開啟後同 Dialog 展開（不彈二級
  視窗）：限制方式 → 幣種/匯率（money）/ Token 輸入 → **重置週期六段選擇器
  （3×2，V1.2.0 起含自訂）**→ 自訂視窗長度輸入 + 小時/天分段 → enforcement/未知定價警示 → 用量/剩餘/進度 →
  [重置使用量] [取消] [儲存]。
- 「下次重置：<本地時間>」僅在所選週期與**已儲存**設定一致時展示，
  避免正在切換時顯示過期邊界。
- **卡片用量徽標（V1.2.0）**：儀表板圖示旁新增顯示開關（Eye/EyeOff，按
  provider 存 localStorage——僅 UI 偏好）；開啟後顯示 `{percent}%`
  （狀態著色）+ `⏱{距下次重置}`；限額關閉或百分比缺失時自動隱藏。
- **懸停提示（V1.2.1）**：卡片圖示列全部按鈕（含眼睛=徽標開關、儀表板=
  限額設定）以 Radix Tooltip 懸停說明（ProviderActions `TipButton`，
  disabled 態外包 span）；取代 WKWebView 下不可靠的原生 `title`。
- **對話框錨定定位（V1.2.1/V1.2.2 兩輪 max-h 修復失敗後的 V1.2.3 終案）**：
  DialogContent 用**內聯 style** `position:fixed; top:2.5rem; bottom:0.75rem;
  left:50%; transform:translateX(-50%); maxHeight:none` —— top/bottom 雙錨定
  使盒子高度恆等於「視窗高 − 52px」，數學上不可能超出視窗。**教訓**：
  ① tailwind-merge 不去重負值任意值——共享元件的 `translate-y-[-50%]` 與
  覆蓋類共存後按 CSS 級聯勝出，任何 max-h 修復都被平移抵消（V1.2.1 dvh
  失效 + V1.2.2 vh 仍無效的共同根因）；② 內聯樣式優先級最高，是覆蓋
  元件庫定位的唯一可靠手段。自訂視窗列輸入 `min-w-0 flex-1`、小時/天
  分段 `w-28 shrink-0`，窄窗不橫向裁切。
- **嵌套圓角（V1.2.2）**：捲動容器 `px-4 py-1` 讓玻璃卡（rounded-xl）
  與外框（rounded-lg）之間留出內縮——外 6px ⊃ 內 12px 的半徑差不再
  產生「內圓角外直線」的觀感。
- **捲動容器 `space-y-4`（V1.2.3）**：啟用卡與設定卡兩塊玻璃卡間距拉開
  （使用者回饋「挨得太近」）。
- 即時重新整理：`refetchOnMount: "always"` + `useUsageLimitEventBridge` 監聽
  `usage-log-recorded` 事件 invalidate `usage-limit` 命名空間（請求記帳後
  卡片/Dialog 即時更新），mutation 成功後 invalidate 對應 query；無
  `window.location.reload()`、無高頻輪詢。

---

## 6. 錯誤回應（429）

`ProxyError::BudgetExhausted { message, detail }` →
`StatusCode::TOO_MANY_REQUESTS` + 與既有 `{"error": {...}}` 約定一致的結構：

```json
{
  "error": {
    "message": "CC Switch usage limit reached. Provider: X. Used: $10.18 / Limit: $10",
    "type": "usage_limit_reached",
    "provider": "X",
    "limitType": "money",
    "currency": "USD",
    "used": "$10.18",
    "limit": "$10"
  }
}
```

message 中的金額/Token 為人類可讀格式（`$10.18` / `5,013,241`）；精確值以
detail 欄位與 status 查詢為準。絕不包含 API Key（只有 Provider 名稱與用量）。

---

## 7. API 清單

### Tauri Commands（`commands/usage_limit.rs`）

| 命令 | 簽名要點 |
| --- | --- |
| `get_usage_limit_status(provider_id, app_type)` | → `BudgetStatus` |
| `save_usage_limit(provider_id, app_type, config)` | config 含 `resetPeriod`；→ 儲存後最新 `BudgetStatus` |
| `reset_usage_limit(provider_id, app_type)` | 視窗推進到當下；→ 最新狀態 |
| `get_exchange_rate(currency)` / `set_exchange_rate(currency, rate)` | 本地 USD→X 匯率讀寫（USD 回傳 "1" / 拒絕設定） |

### 前端 Query Keys（`lib/query/usageLimit.ts`）

```
["usage-limit", "status", providerId, appType]
["usage-limit", "exchange-rate", currency]
```

Hooks：`useUsageLimitStatus`（refetchOnMount always）、`useSaveUsageLimit`、
`useResetUsageLimit`、`useExchangeRate(currency)`、`useSetExchangeRate`
（成功後 invalidate 整個 usage-limit 命名空間）。

### `BudgetStatus` 欄位（camelCase serde）

`providerId` `appType` `enabled` `limitType` `currency` `limitAmount`
`usageStartAt`（有效視窗起點）**`resetPeriod` `nextResetAt`（V1.0.1）**
`usedMoneyUsd` `usedMoneyInCurrency` `usedTokens` `percentUsed` `state`
`unpricedRequestCount` `unpricedModels` `enforcement` `maskedCredential`。

---

## 8. i18n（usageLimit 命名空間，4 locale × 39 keys）

V1.0.1 新增 8 個、V1.0.2 新增 3 個幣種標籤（eur/jpy/gbp）並參數化匯率文案
（`exchangeRate: "USD → {{currency}} rate"`，hint 幣種中性化）；其餘 28 個
為 V1.0：

| key | zh | en |
| --- | --- | --- |
| `resetPeriod` | 重置週期 | Reset schedule |
| `resetNever` | 不重置 | Off |
| `resetHourly` | 每小時 | Hourly |
| `resetDaily` | 每天 | Daily |
| `resetWeekly` | 每週 | Weekly |
| `resetMonthly` | 每月 | Monthly |
| `resetPeriodHint` | 選擇週期後…自動重新統計 | With a schedule selected… |
| `nextResetAt` | 下次重置：{{time}} | Next reset: {{time}} |

維護約定：新增 key 必須 4 個 locale 同步補齊（CI/測試均無 locale 完整性
兜底，靠評審把關）；元件內禁止硬編碼中文。

---

## 9. 測試索引

| 層 | 位置 | 覆蓋 |
| --- | --- | --- |
| Rust 單測（41） | `services/usage_limit/tests.rs` | 關閉/低於/達到/超過、Reset、換 Key、匯率、精度、未知定價、並行、驗證、指紋/遮蔽、儲存語義、狀態機、enforcement；V1.0.1：邊界日曆、next 嚴格未來、parse 值域、四週期滾動恢復、never 回歸、儲存校驗/切換對齊、髒資料兜底、手動重置共存、DDL 預設值；V1.0.2：五幣種預設匯率獨立、EUR/JPY/GBP 換算判定、5 幣種可存與未知幣種拒絕、非預設匯率端到端攔截、FK=ON 遷移端到端、髒匯率回退預設 |
| Proxy 整合 | `proxy/forwarder.rs` tests | Case 1 token 1100/1000 BLOCK、Case 2 money $1.02 不發上游、Case 4 未設定/關閉不改行為、429 映射；V1.0.1 `budget_recovers_after_period_boundary_rollover`（滾動放行 + 持久化 + 對照攔截） |
| 記帳鏈路 | `proxy/usage/logger.rs` tests | 串流完成後帶指紋落庫 → guard 隨即攔截 |
| 遷移 | `database/tests.rs` | v0→…→v22 鏈式遷移、欄位預設值對齊、v22 重建保留資料/EUR 可入/limit_type CHECK 仍在、全新表無 currency CHECK |
| 前端（31） | `tests/components/UsageLimitDialog.test.tsx` | OFF 預設、展開、CNY/USD 切換、Token、非法輸入、進度、exhausted 不 clamp、Reset 確認、API 錯誤；V1.0.1：週期選擇器渲染、預設 never+hint、儲存攜帶週期、下次重置展示/隱藏、重開保留週期；V1.0.2：五幣種渲染、EUR 匯率輸入+換算+€ 後綴、EUR 儲存持久化匯率、USD 不涉及匯率、切換幣種按該幣種匯率回填、匯率未載入時儲存被攔截（P1-1 競態回歸） |

測試紀律：假 key 統一 `sk-test-...`；週期測試的期望值由同一套本地時區
建構（`Local.with_ymd_and_hms`），不寫死 UTC 時間戳。

環境註記：`proxy::server::tests` / `proxy::hyper_client::tests` 為網路型
整合測試（真實 socket/上游），在無外網的沙箱環境會掛起——屬既有環境
限制，與限額功能無關；預算判定路徑的整合覆蓋在
`proxy::forwarder::tests`（無網路依賴），可獨立全量執行。

---

## 10. 常見問題排查（FAQ）

**Q：UI 顯示已 100% 但請求沒被攔？**
先看 `enforcement`：`proxy_disabled` = 該應用代理接管未開啟，流量不經過
CC Switch（V1 明確邊界，不是 bug）；`unsupported_credential` = OAuth 無
靜態 Key。兩者 UI 都有警示條。

**Q：金額模式和 Usage Dashboard 對不上？**
兩者同源（`proxy_request_logs.total_cost_usd`）。若 Budget 只統計視窗內
（`created_at >= 有效視窗起點`）而 Dashboard 統計更大範圍，數值自然不同；
範圍一致時必定一致。出現缺口優先查：① 有無 `data_source != 'proxy'` 的列
被算進來（不應）；② 視窗起點是否剛被週期滾動。

**Q：設定了每天重置，為什麼當天早些的用量還在統計裡？**
儲存路徑是保守語義：原視窗已在本週期內不回退（保留本期已統計用量），
下個零點起嚴格按天。想要立即清零請點「重置使用量」。

**Q：跨過零點後請求立即恢復了嗎？**
是。guard 先滾動後判定，邊界一過舊用量即出窗（整合測試
`budget_recovers_after_period_boundary_rollover` 覆蓋）。

**Q：換 API Key 後限額還在嗎？**
設定保留（每 Provider 一條），指紋重綁 + 視窗重置，新 Key 從 0 起算。

**Q：切換幣種時匯率輸入框為什麼變空了？**
設計行為（審查 P1-1 修復）：舊幣種匯率不得殘留（否則可能把 CNY 的 7.2 存成
JPY 匯率）。該幣種已儲存的匯率會自動回填；首次使用該幣種時回填預設值；
回傳前儲存會被校驗攔截，不會落庫錯誤匯率。

**Q：首次開啟提示「無法驗證開發者」或「檔案已損毀」？**
- v1.1.1 起：應用套件已完整 ad-hoc 簽章，不會再報「已損毀」；因未經 Apple
  公證，首次開啟會顯示一次性的「無法驗證開發者」——**右鍵點擊應用 → 打開
  → 打開**放行，或終端機執行 `xattr -dr com.apple.quarantine
  "/Applications/CC Switch.app"`。
- v1.1.0 及更早的安裝包存在簽章缺失（會報「已損毀」），請一律使用 v1.1.1
  及以後的安裝包。

**Q：如何發布新版本？**
1. `pnpm tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}'`
   （無更新簽章私鑰時關閉 updater 產物；建置依賴 cargo 在 PATH 中）；
2. DMG 按約定重新命名 `CC.Switch_<基座版本>_aarch64_usage-limit.dmg`；
3. `git archive --format=zip <tag>` 產生源碼快照（命名隨倉庫）；
4. `gh release create <tag> --notes-file <四語正文>` 附雙資產。
正文範本見 `docs/release-notes/usage-limit-v1.1.0.md`。

**Q：倉庫改名後 CI 為什麼會掛？**
`actions/cache` 恢復的 `src-tauri/target` 快取中，Tauri build script 輸出內嵌
舊工作區絕對路徑，改名後必然失配（報 "failed to read plugin permissions"）。
修復：`gh cache delete --all` 後重跑——restore-keys 前綴回退會把舊快取帶回，
只改 key 前綴無效，必須刪除。

**Q：為什麼 `api_key_limits.reset_period` 沒有 CHECK 約束？**
讓 ALTER ADD COLUMN 遷移路徑與全新 DDL 行為完全一致；值域由儲存路徑
校驗，讀取側未知值回退 never（`parse_reset_period` 告警兜底）。

**Q：限額設定會雲同步 / 匯出嗎？**
不會（V1 決策：usage is local observation，設定行為本機決策），避免跨
裝置重複統計。Provider 刪除時設定列經 FK CASCADE 清理。

---

## 11. 已知限制

1. **只管經過本機 Local Proxy 的流量**——不是 Provider 帳戶的全域配額；
   多裝置/直連用量不可見。
2. 金額模式依賴模型定價；缺價請求按 0 聚合並顯式警示（不悄悄按 0 假裝
   全貌）。
3. 已允許的請求允許完成 → 設計內 overshoot。
4. OAuth Provider 不支援按 Key 限額。
5. 週期邊界按裝置目前時區；跨時區移動後「每天」跟隨新時區零點。
6. DST 切換日邊界有 ±1 小時級牆鐘歧義（見 §4.3），不產生方向性錯誤。
7. 限額設定與用量不參與 Cloud Sync / 匯入匯出。
8. 匯率為本地靜態值：人工調節、不連網、不追蹤市場波動；跨幣種比較以 USD
   原值（`usedMoneyUsd`）為準。
9. JPY 展示符號 `JP¥` 與 CNY 的 `¥` 並存（JPY 加國別前綴消歧）。

---

## 12. 檔案清單

| 檔案 | 職責 |
| --- | --- |
| `src-tauri/src/services/usage_limit.rs` | 領域服務：指紋/遮蔽、credential 解析、ResetPeriod 與週期邊界、guard（`check_budget_before_forward`）、status、save（驗證）、匯率 |
| `src-tauri/src/services/usage_limit/tests.rs` | 領域單測（41） |
| `src-tauri/src/database/dao/usage_limit.rs` | DAO：設定 CRUD、視窗重置/重綁、SSOT 聚合 |
| `src-tauri/src/database/schema.rs` | `api_key_limits` DDL + `migrate_v20_to_v21` / `migrate_v21_to_v22` + 分發 |
| `src-tauri/src/database/mod.rs` | `SCHEMA_VERSION = 22` |
| `src-tauri/src/commands/usage_limit.rs` | 5 個 Tauri 命令 |
| `src-tauri/src/proxy/forwarder.rs` | Budget Guard 接入（per attempt） |
| `src-tauri/src/proxy/error.rs` | `BudgetExhausted` → 429 結構化錯誤體 |
| `src-tauri/src/proxy/usage/logger.rs` | 記帳落庫攜帶 `credential_fingerprint` |
| `src/types/usageLimit.ts` | 類型（含 `UsageLimitResetPeriod`） |
| `src/lib/api/usageLimit.ts` | invoke 封裝 |
| `src/lib/query/usageLimit.ts` | Query keys + hooks |
| `src/components/usage-limit/UsageLimitButton.tsx` | 卡片圖示四態 + 簡略用量 |
| `src/components/usage-limit/UsageLimitDialog.tsx` | Dialog（設定/進度/重置週期/下次重置） |
| `src/hooks/useUsageEventBridge.ts` | 記帳事件 → invalidate（主介面重新整理） |
| `src/i18n/locales/{zh,zh-TW,en,ja}.json` | `usageLimit` 命名空間 × 39 keys |
| `tests/components/UsageLimitDialog.test.tsx` | 前端測試（31） |
| `docs/release-notes/usage-limit-v1.1.{0,1}.md` | 各版本四語發布正文（GitHub Release 與倉庫各存一份） |
| `docs/usage-limit-knowledge-base-{zh,en,zh-TW,ja}.md` | 本知識庫的四語言版本 |

---

## 13. 版本歷史

| 版本 | 日期 | 內容 |
| --- | --- | --- |
| V1.0 | 2026-09-17 | 金額/Token 限額、Proxy enforcement、指紋綁定、手動重置、四態圖示、i18n、全鏈路測試 |
| V1.0.1 | 2026-09-18 | 重置週期（never/hourly/daily/weekly/monthly，本地時區懶滾動）、「下次重置」展示、schema v21、週期恢復整合測試 |
| V1.0.2 | 2026-09-18 | 金額限額擴展至 5 幣種（USD/CNY/EUR/JPY/GBP）+ 各幣種人工匯率、匯率命令泛化、schema v22（移除 currency CHECK）、幣種符號後綴等 UI 打磨 |
| **V1.1.0** | 2026-09-18 | **公開發布**：V1.0 + V1.0.1 + V1.0.2 的全部內容作為一個版本發布（基座 CC Switch 3.20.3），即 v1.0.0 之後的第一個公開版本 |
| **V1.1.1** | 2026-09-18 | **安裝包修復**：應用套件完整 ad-hoc 簽章（修復「檔案已損毀」與首次拖拽不註冊）、DMG 移除雜散 `.VolumeIcon.icns`、README/發布說明補充首次開啟放行指引。功能與 V1.1.0 一致 |
| **V1.2.0** | 2026-09-19 | 視窗語義重做（關→開/重置設定變更從當下起算，移除日曆回對齊）+ 自訂滾動視窗（N 小時/天，schema v23）+ 對話框滾動/拖動修復 + 卡片用量徽標 |
| **V1.2.1** | 2026-09-19 | 介面細節修復：對話框高度約束（標題不再頂出）、自訂視窗列溢出、卡片圖示列懸停提示 |
| **V1.2.2** | 2026-09-19 | 對話框布局終修：max-h 改用 vh（dvh 在舊 WKWebView 失效是 V1.2.1 無效的根因）+ 玻璃卡內縮解決嵌套圓角 |
| **V1.2.3** | 2026-09-19 | 對話框改 top/bottom 內聯錨定定位（實錘根因：tailwind-merge 不去重負值 translate-y，抵消 max-h 修復），啟用卡與設定卡間距拉開 |
