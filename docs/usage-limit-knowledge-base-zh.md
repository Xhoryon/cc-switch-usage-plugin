<!-- Language / 语言: [简体中文](usage-limit-knowledge-base-zh.md) | [English](usage-limit-knowledge-base-en.md) | [繁體中文](usage-limit-knowledge-base-zh-TW.md) | [日本語](usage-limit-knowledge-base-ja.md) -->

# 使用限额（Budget Limit）知识库

> 适用版本：**V1.1.1**（公开发布，2026-09-18；安装包修复版，功能与 V1.1.0
> 一致，基座 CC Switch 3.20.3）。各迭代明细见 `docs/development_log.md`
> 对应条目。本文是整合视图，面向后续维护者，回答「它是什么、为什么这样
> 设计、改哪里要小心什么」。

---

## 1. 功能概述

每个 Provider 的静态 API Key 可以单独设置一个最大使用量，达到后 **Local
Proxy 在转发前拒绝新请求**（真正的 enforcement，不是仅统计）。

| 能力 | 说明 |
| --- | --- |
| 限制方式 | 金额（USD / CNY / EUR / JPY / GBP）或 Token 数，同一时间二选一 |
| enforcement 位置 | Local Proxy `forward_with_retry_inner` 每个 provider attempt 转发前 |
| 统计来源 | 复用 Usage SSOT（`proxy_request_logs`），不建平行统计 |
| credential 绑定 | SHA-256 不可逆指纹，任何路径不落明文 API Key |
| 重置周期（V1.0.1） | 不重置 / 每小时 / 每天 / 每周 / 每月，本地时区边界懒滚动 |
| 多币种（V1.0.2） | 非 USD 币种按本地可调汇率换算（人工调节，不联网） |
| 手动重置 | `usage_start_at` 推进到当下，不删任何历史 Usage |

**核心边界（务必随功能一起传播的认知）**：

> Budget enforcement only applies to traffic routed through CC Switch Local
> Proxy. 未开启代理接管的流量观测不到、拦不住；UI 以
> `enforcement = proxy_disabled` 明示，不假装 Hard Limit 生效。
> 同一 Key 在其他设备/直连的用量不计入本机预算（usage is local observation）。

---

## 2. 总体架构

```
配置/展示链路
  Provider Card（ProviderActions 图标按钮）
    └─ UsageLimitButton（Gauge 图标，useUsageLimitStatus 自取状态）
         └─ UsageLimitDialog（开关 / 限制方式 / 币种 / 金额 / 重置周期 / 进度）
              └─ Tauri Command（get_usage_limit_status / save_usage_limit /
                 reset_usage_limit / get_exchange_rate / set_exchange_rate）
                   └─ Database impl（services/usage_limit.rs）
                        ├─ api_key_limits 表（配置 + 窗口起点 + 周期）
                        └─ proxy_request_logs 聚合（用量，SSOT 复用）

转发执行链路
  Client Request
    └─ CC Switch Local Proxy（forwarder.rs）
         └─ Budget Guard（per provider attempt，forward() 之前）
              ├─ 未启用 → 放行（零开销路径是一次 PK 查询）
              ├─ credential 指纹不一致 → 重绑 + 窗口重置为当下
              ├─ 周期边界已越过（V1.0.1）→ 懒滚动窗口起点
              └─ used >= limit → ProxyError::BudgetExhausted（429，不发上游）
                   └─ 响应完成后 Usage Accounting（log_with_calculation，
                      携带 credential_fingerprint 落 proxy_request_logs）
```

分层纪律：React → Tauri API → Command → Service（Database impl）→ DAO →
SQLite。React 不直接访问数据库；金额/Token 聚合只在 Rust 侧。

---

## 3. 数据模型

### 3.1 `api_key_limits`（V1.0 建，V1.0.1 加列）

```sql
CREATE TABLE api_key_limits (
    provider_id            TEXT NOT NULL,
    app_type               TEXT NOT NULL,
    credential_fingerprint TEXT NOT NULL,      -- SHA-256 hex，绝不存明文 key
    enabled                INTEGER NOT NULL DEFAULT 0,
    limit_type             TEXT NOT NULL CHECK (limit_type IN ('money', 'token')),
    currency               TEXT,                    -- token 模式为 NULL；V1.0.2 起无 CHECK
    limit_amount           TEXT NOT NULL,      -- 金额十进制字符串 / token 正整数字符串
    usage_start_at         INTEGER NOT NULL,   -- 统计窗口起点（unix 秒）
    reset_period           TEXT NOT NULL DEFAULT 'never',  -- V1.0.1
    created_at             INTEGER NOT NULL,
    updated_at             INTEGER NOT NULL,
    PRIMARY KEY (provider_id, app_type),
    FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type)
        ON DELETE CASCADE                       -- Provider 删除级联清理
);
```

- 每个 Provider 至多一条配置（PK 决定）。
- `limit_amount` 用字符串避免 SQLite REAL 浮点误差进入限额判断。
- `reset_period` 刻意**无 CHECK**：迁移路径（ALTER ADD COLUMN）与全新建表
  行为完全一致；值域由保存路径校验，读取侧未知值回退 `never` 并告警。
- `currency` 的 CHECK 在 v22 迁移中**移除**（V1.0.2 多币种）：SQLite 无法修改
  既有 CHECK，迁移为标准表重建（建新表→拷贝→删旧→改名，幂等性靠
  `sqlite_master.sql` 是否仍含 `CHECK (currency` 判断）；值域由
  `LimitCurrency::parse` 在保存路径校验、读取侧未知值回退 USD。**后续扩展
  币种不再需要迁移**。
- 本表**不冗余存储累计用量**（避免双 SSOT），用量始终从
  `proxy_request_logs` 实时聚合。

### 3.2 用量 SSOT 聚合谓词

所有预算统计共用同一 WHERE：

```sql
WHERE provider_id = ? AND app_type = ?
  AND credential_fingerprint = ?
  AND created_at >= usage_start_at   -- 有效窗口起点（见 §4.3）
  AND data_source = 'proxy'          -- 会话同步行/直连行不参与预算
```

- **Token**：normalized total = fresh input + output + cache_creation +
  cache_read（`services/sql_helpers::fresh_input_sql`，与 Usage Dashboard
  同一规则；`input_token_semantics` 区分缓存语义）。
- **金额**：累加已落库的 `total_cost_usd` 十进制字符串。两级通道：
  - 快速通道：`SUM(CAST(... AS REAL))` 明显低于阈值（margin 1e-6）→ O(1) 放行；
  - 精确通道：贴近/超过阈值时取原始字符串逐行 `rust_decimal` 求和复核，
    规避 0.1+0.2 ≠ 0.3 的浮点误判。

### 3.3 settings 键

| 键 | 含义 | 默认 |
| --- | --- | --- |
| `usage_limit_usd_cny_rate` | 本地 USD→CNY 汇率（沿用 V1.0 旧键，升级不丢） | `7.2` |
| `usage_limit_usd_eur_rate` | 本地 USD→EUR 汇率 | `0.92` |
| `usage_limit_usd_jpy_rate` | 本地 USD→JPY 汇率 | `150` |
| `usage_limit_usd_gbp_rate` | 本地 USD→GBP 汇率 | `0.79` |

非法存量汇率回退该币种默认并告警；汇率必须 > 0 且 < 1,000,000；USD 是内部
计价基准，恒为 1 且不可设置。前端在 Dialog 内修改并随保存持久化（人工调节，
不联网获取）。

---

## 4. 核心机制

### 4.1 credential 指纹与轮换

- `credential_fingerprint(api_key)` = SHA-256 小写 hex（64 位），不可逆。
- 解析复用代理转发同款 `adapter.extract_auth`，保证「配置绑定的指纹」与
  「实际转发的指纹」同源。
- 展示一律遮蔽：`mask_credential` → `sk-****ABCD`（≤8 字符 → `****`）。
- OAuth 类 Provider（codex_oauth / xai_oauth / managed account / copilot）
  无静态 Key → `unsupported_credential`，不允许按 Key 限额。
- **换 Key**：guard / 保存发现指纹不一致 → 重绑新指纹 + 窗口起点重置为
  当下。旧 Key 的历史用量不并入新 Key；读接口（status）检测到轮换时按
  「当前 credential」统计但**不回写**（无副作用）。

### 4.2 判定语义

- Token：`used_normalized_total >= limit` → BLOCK（整数比较，无精度问题）。
- 金额：`limit_in_usd = limit_amount / rate` 折回 USD 与成本同域 `Decimal`
  比较；展示侧 `used_in_currency = used_usd × rate`。`rate` 为 USD→限额币种
  汇率（V1.0.2：5 币种各自独立，USD 恒 1，单一代码路径）。
- BLOCK 返回 429（见 §6），上游零流量；failover 链中
  `BudgetExhausted` 属 NonRetryable，不消耗其他 Provider 的尝试名额。
- DB 读取/聚合失败 → fail-open（warn + 放行），与熔断器降级同思路；
  判定失败绝不能把所有请求拦死。
- **Overshoot 是设计内行为**：已允许发出的请求允许完成，usage 在响应
  结束后落库，最后一个请求可能小幅超出。禁止中断进行中的 SSE/流。

### 4.3 重置周期与懒滚动（V1.0.1）

`ResetPeriod::current_period_start(now)` 按**本地时区**给出当前周期边界：

| 周期 | 边界 | 例（2026-09-18 周五 15:27） |
| --- | --- | --- |
| never | 无 | — |
| hourly | 本地整点 | 15:00 |
| daily | 本地零点 | 09-18 00:00 |
| weekly | 本地周一零点（ISO 周） | 09-14 00:00 |
| monthly | 本地 1 日零点 | 09-01 00:00 |

有效窗口起点 = `max(usage_start_at, 当前周期边界)`，三个应用点：

1. **guard（写路径）**：先滚动后判定；滚动以一行 UPDATE 持久化
   （best-effort，失败仅告警，判定仍按新起点）。被限额的窗口跨过边界后
   在同一次 guard 调用内恢复放行。
2. **status（读路径）**：同样即时计算并返回 `resetPeriod` / `nextResetAt`
   （下一次边界），但**不回写**——读接口保持无副作用。
3. **save（写路径）**：周期值缺省视为 `never`，非法值拒绝入库；若原窗口
   早于新周期边界（如 never→daily 且窗口在昨天）立即对齐到边界；原窗口
   已在本周期内则**不回退**（保守保留本期已统计用量）。

懒滚动 vs 定时器：应用关闭期间跨过的边界在下次启动后照常生效；无并发
定时任务；判定本来就在 DB Mutex 下串行，滚动只是一行 UPDATE。

DST 处理（与 `usage_rollup::compute_local_midnight_cutoff` 同款）：
墙钟歧义（fall-back 重复小时）取较早映射；不存在的墙钟时刻（spring-forward
gap）顺延一小时重试；最终 UTC 解释兜底。`next_period_start` 额外要求结果
严格晚于 now，否则返回 None（宁可不展示「下次重置」）。

手动重置与周期正交：手动重置把窗口推进到当下立即清零；下个周期边界仍
照常滚动。重置**永不删除** `proxy_request_logs` 历史行。

### 4.4 并发模型

判定读与用量写都在 `Database.conn` 的 Mutex 下串行，无 read→write 竞态
丢写（单测：8 线程 × 25 行并发写后聚合 == 写入总量）。真实用量在响应后
才可知，同时进行的多个请求仍可能少量 overshoot——设计内，不为理论零
overshoot 破坏转发并发。周期滚动写在同一 Mutex 下，无额外竞态面。

### 4.5 未知定价（金额模式可靠性）

窗口内「有 token 用量但 `total_cost_usd = '0'`」的请求，逐 `pricing_model`
核对模型定价表：**有定价但成本 0**（免费模型）→ 正常；**无定价且非占位
模型** → 计入 `unpricedRequestCount` / `unpricedModels`，UI 显示警示条并
引导配置 custom pricing。缺价请求成本按 0 聚合，但**绝不假装 Hard Limit
精确覆盖了它们**（UI 明示）。

---

## 5. 状态机与 UI

`BudgetStatus.state`（后端计算，前端只渲染）：

| state | 条件 | 图标 | 进度条 |
| --- | --- | --- | --- |
| off | 未启用或无配置 | muted 灰 | — |
| active | 已启用且 < 80% | emerald | emerald |
| warning | ≥ 80% 且 < 100% | amber | amber |
| exhausted | ≥ 100% | red | red（数值不 clamp，进度条视觉封顶 100%） |

- `percent_used` 不 clamp（可显示 104.2%）。
- enforcement 三态：`active` / `proxy_disabled`（该应用代理接管未开启，
  显示警示条）/ `unsupported_credential`（OAuth，禁止配置）。
- Dialog 结构：关闭态只有开关行（简洁）；开启后同 Dialog 展开（不弹二级
  窗）：限制方式 → 币种/汇率（money）/ Token 输入 → **重置周期五段选择器
  （V1.0.1）** → enforcement/未知定价警示 → 用量/剩余/进度 →
  [重置使用量] [取消] [保存]。
- 「下次重置：<本地时间>」仅在所选周期与**已保存**配置一致时展示，
  避免正在切换时显示过期边界。
- 实时刷新：`refetchOnMount: "always"` + `useUsageLimitEventBridge` 监听
  `usage-log-recorded` 事件 invalidate `usage-limit` 命名空间（请求记账后
  卡片/Dialog 即时更新），mutation 成功后 invalidate 对应 query；无
  `window.location.reload()`、无高频轮询。

---

## 6. 错误响应（429）

`ProxyError::BudgetExhausted { message, detail }` →
`StatusCode::TOO_MANY_REQUESTS` + 与既有 `{"error": {...}}` 约定一致的结构体：

```json
{
  "error": {
    "message": "CC Switch usage limit reached. Provider: X. Used: $10.18 / Limit: $10",
    "type": "usage_limit_reached",
    "provider": "X",
    "limitType": "money",          // money | token
    "currency": "USD",             // token 模式为 ""
    "used": "$10.18",
    "limit": "$10"
  }
}
```

message 中的金额/Token 为人类可读格式（`$10.18` / `5,013,241`）；精确值以
detail 字段与 status 查询为准。绝不包含 API Key（只有 Provider 名称与用量）。

---

## 7. API 清单

### Tauri Commands（`commands/usage_limit.rs`）

| 命令 | 签名要点 |
| --- | --- |
| `get_usage_limit_status(provider_id, app_type)` | → `BudgetStatus` |
| `save_usage_limit(provider_id, app_type, config)` | config 含 `resetPeriod`；→ 保存后最新 `BudgetStatus` |
| `reset_usage_limit(provider_id, app_type)` | 窗口推进到当下；→ 最新状态 |
| `get_exchange_rate(currency)` / `set_exchange_rate(currency, rate)` | 本地 USD→X 汇率读写（USD 返回 "1" / 拒绝设置） |

### 前端 Query Keys（`lib/query/usageLimit.ts`）

```
["usage-limit", "status", providerId, appType]
["usage-limit", "exchange-rate", currency]
```

Hooks：`useUsageLimitStatus`（refetchOnMount always）、`useSaveUsageLimit`、
`useResetUsageLimit`、`useExchangeRate(currency)`、`useSetExchangeRate`
（成功后 invalidate 整个 usage-limit 命名空间）。

### `BudgetStatus` 字段（camelCase serde）

`providerId` `appType` `enabled` `limitType` `currency` `limitAmount`
`usageStartAt`（有效窗口起点）**`resetPeriod` `nextResetAt`（V1.0.1）**
`usedMoneyUsd` `usedMoneyInCurrency` `usedTokens` `percentUsed` `state`
`unpricedRequestCount` `unpricedModels` `enforcement` `maskedCredential`。

---

## 8. i18n（usageLimit 命名空间，4 locale × 39 keys）

V1.0.1 新增 8 个、V1.0.2 新增 3 个币种标签（eur/jpy/gbp）并参数化汇率文案
（`exchangeRate: "USD → {{currency}} 汇率"`，hint 币种中性化）；其余 28 个
为 V1.0：

| key | zh | en |
| --- | --- | --- |
| `resetPeriod` | 重置周期 | Reset schedule |
| `resetNever` | 不重置 | Off |
| `resetHourly` | 每小时 | Hourly |
| `resetDaily` | 每天 | Daily |
| `resetWeekly` | 每周 | Weekly |
| `resetMonthly` | 每月 | Monthly |
| `resetPeriodHint` | 选择周期后…自动重新统计 | With a schedule selected… |
| `nextResetAt` | 下次重置：{{time}} | Next reset: {{time}} |

维护约定：新增 key 必须 4 个 locale 同步补齐（CI/测试均无 locale 完整性
兜底，靠评审把关）；组件内禁止硬编码中文。

---

## 9. 测试索引

| 层 | 位置 | 覆盖 |
| --- | --- | --- |
| Rust 单测（41） | `services/usage_limit/tests.rs` | 关闭/低于/达到/超过、Reset、换 Key、汇率、精度、未知定价、并发、验证、指纹/遮蔽、保存语义、状态机、enforcement；V1.0.1：边界日历、next 严格未来、parse 值域、四周期滚动恢复、never 回归、保存校验/切换对齐、脏数据兜底、手动重置共存、DDL 默认值；V1.0.2：五币种默认汇率独立、EUR/JPY/GBP 换算判定、5 币种可存与未知币种拒绝、非默认汇率端到端拦截、FK=ON 迁移端到端、脏汇率回退默认 |
| Proxy 集成 | `proxy/forwarder.rs` tests | Case 1 token 1100/1000 BLOCK、Case 2 money $1.02 不发上游、Case 4 未配置/关闭不改行为、429 映射；V1.0.1 `budget_recovers_after_period_boundary_rollover`（滚动放行 + 持久化 + 对照拦截） |
| 记账链路 | `proxy/usage/logger.rs` tests | 流式完成后带指纹落库 → guard 随即拦截 |
| 迁移 | `database/tests.rs` | v0→…→v22 链式迁移、列默认值对齐、v22 重建保留数据/EUR 可入/limit_type CHECK 仍在、全新表无 currency CHECK |
| 前端（31） | `tests/components/UsageLimitDialog.test.tsx` | OFF 默认、展开、CNY/USD 切换、Token、非法输入、进度、exhausted 不 clamp、Reset 确认、API 错误；V1.0.1：周期选择器渲染、默认 never+hint、保存携带周期、下次重置展示/隐藏、重开保留周期；V1.0.2：五币种渲染、EUR 汇率输入+换算+€ 后缀、EUR 保存持久化汇率、USD 不涉及汇率、切换币种按该币种汇率回填、汇率未加载时保存被拦截（P1-1 竞态回归） |

测试纪律：假 key 统一 `sk-test-...`；周期测试的期望值由同一套本地时区
构造（`Local.with_ymd_and_hms`），不写死 UTC 时间戳。

环境注记：`proxy::server::tests` / `proxy::hyper_client::tests` 为网络型
集成测试（真实 socket/上游），在无外网的沙箱环境会挂起——属既有环境
限制，与限额功能无关；预算判定路径的集成覆盖在
`proxy::forwarder::tests`（无网络依赖），可独立全量运行。

---

## 10. 常见问题排查（FAQ）

**Q：UI 显示已 100% 但请求没被拦？**
先看 `enforcement`：`proxy_disabled` = 该应用代理接管未开启，流量不经过
CC Switch（V1 明确边界，不是 bug）；`unsupported_credential` = OAuth 无
静态 Key。两者 UI 都有警示条。

**Q：金额模式和 Usage Dashboard 对不上？**
两者同源（`proxy_request_logs.total_cost_usd`）。若 Budget 只统计窗口内
（`created_at >= 有效窗口起点`）而 Dashboard 统计更大范围，数值自然不同；
范围一致时必定一致。出现缺口优先查：① 有无 `data_source != 'proxy'` 的行
被算进来（不应）；② 窗口起点是否刚被周期滚动。

**Q：设置了每天重置，为什么当天早些的用量还在统计里？**
保存路径是保守语义：原窗口已在本周期内不回退（保留本期已统计用量），
下个零点起严格按天。想要立即清零请点「重置使用量」。

**Q：跨过零点后请求立即恢复了吗？**
是。guard 先滚动后判定，边界一过旧用量即出窗（集成测试
`budget_recovers_after_period_boundary_rollover` 覆盖）。

**Q：换 API Key 后限额还在吗？**
配置保留（每 Provider 一条），指纹重绑 + 窗口重置，新 Key 从 0 起算。

**Q：切换币种时汇率输入框为什么变空了？**
设计行为（审查 P1-1 修复）：旧币种汇率不得残留（否则可能把 CNY 的 7.2 存成
JPY 汇率）。该币种已保存的汇率会自动回填；首次使用该币种时回填默认值；
返回前保存会被校验拦截，不会落库错误汇率。

**Q：首次打开提示「无法验证开发者」或「文件已损坏」？**
- v1.1.1 起：应用包已完整 ad-hoc 签名，不会再报「已损坏」；因未经 Apple
  公证，首次打开会显示一次性的「无法验证开发者」——**右键点击应用 → 打开
  → 打开**放行，或终端执行 `xattr -dr com.apple.quarantine
  "/Applications/CC Switch.app"`。
- v1.1.0 及更早的安装包存在签名缺失（会报「已损坏」），请一律使用 v1.1.1
  及以后的安装包。

**Q：如何发布新版本？**
1. `pnpm tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}'`
   （无更新签名私钥时关闭 updater 产物；构建依赖 cargo 在 PATH 中）；
2. DMG 按约定重命名 `CC.Switch_<基座版本>_aarch64_usage-limit.dmg`；
3. `git archive --format=zip <tag>` 生成源码快照（命名随仓库）；
4. `gh release create <tag> --notes-file <四语正文>` 附双资产。
正文模板见 `docs/release-notes/usage-limit-v1.1.0.md`。

**Q：仓库改名后 CI 为什么会挂？**
`actions/cache` 恢复的 `src-tauri/target` 缓存中，Tauri build script 输出内嵌
旧工作区绝对路径，改名后必然失配（报 "failed to read plugin permissions"）。
修复：`gh cache delete --all` 后重跑——restore-keys 前缀回退会把旧缓存带回，
只改 key 前缀无效，必须删除。

**Q：为什么 `api_key_limits.reset_period` 没有 CHECK 约束？**
让 ALTER ADD COLUMN 迁移路径与全新 DDL 行为完全一致；值域由保存路径
校验，读取侧未知值回退 never（`parse_reset_period` 告警兜底）。

**Q：限额配置会云同步 / 导出吗？**
不会（V1 决策：usage is local observation，配置行为本机决策），避免跨
设备重复统计。Provider 删除时配置行经 FK CASCADE 清理。

---

## 11. 已知限制

1. **只管经过本机 Local Proxy 的流量**——不是 Provider 账户的全局配额；
   多设备/直连用量不可见。
2. 金额模式依赖模型定价；缺价请求按 0 聚合并显式警示（不悄悄按 0 假装
   全貌）。
3. 已允许的请求允许完成 → 设计内 overshoot。
4. OAuth Provider 不支持按 Key 限额。
5. 周期边界按设备当前时区；跨时区移动后「每天」跟随新时区零点。
6. DST 切换日边界有 ±1 小时级墙钟歧义（见 §4.3），不产生方向性错误。
7. 限额配置与用量不参与 Cloud Sync / 导入导出。
8. 汇率为本地静态值：人工调节、不联网、不跟踪市场波动；跨币种比较以 USD
   原值（`usedMoneyUsd`）为准。
9. JPY 展示符号 `JP¥` 与 CNY 的 `¥` 并存（JPY 加国别前缀消歧）。

---

## 12. 文件清单

| 文件 | 职责 |
| --- | --- |
| `src-tauri/src/services/usage_limit.rs` | 域服务：指纹/遮蔽、credential 解析、ResetPeriod 与周期边界、guard（`check_budget_before_forward`）、status、save（验证）、汇率 |
| `src-tauri/src/services/usage_limit/tests.rs` | 域单测（41） |
| `src-tauri/src/database/dao/usage_limit.rs` | DAO：配置 CRUD、窗口重置/重绑、SSOT 聚合 |
| `src-tauri/src/database/schema.rs` | `api_key_limits` DDL + `migrate_v20_to_v21` / `migrate_v21_to_v22` + 分发 |
| `src-tauri/src/database/mod.rs` | `SCHEMA_VERSION = 22` |
| `src-tauri/src/commands/usage_limit.rs` | 5 个 Tauri 命令 |
| `src-tauri/src/proxy/forwarder.rs` | Budget Guard 接入（per attempt） |
| `src-tauri/src/proxy/error.rs` | `BudgetExhausted` → 429 结构化错误体 |
| `src-tauri/src/proxy/usage/logger.rs` | 记账落库携带 `credential_fingerprint` |
| `src/types/usageLimit.ts` | 类型（含 `UsageLimitResetPeriod`） |
| `src/lib/api/usageLimit.ts` | invoke 封装 |
| `src/lib/query/usageLimit.ts` | Query keys + hooks |
| `src/components/usage-limit/UsageLimitButton.tsx` | 卡片图标四态 + 简略用量 |
| `src/components/usage-limit/UsageLimitDialog.tsx` | Dialog（配置/进度/重置周期/下次重置） |
| `src/hooks/useUsageEventBridge.ts` | 记账事件 → invalidate（主界面刷新） |
| `src/i18n/locales/{zh,zh-TW,en,ja}.json` | `usageLimit` 命名空间 × 39 keys |
| `tests/components/UsageLimitDialog.test.tsx` | 前端测试（31） |
| `docs/release-notes/usage-limit-v1.1.{0,1}.md` | 各版本四语发布正文（GitHub Release 与仓库各存一份） |
| `docs/usage-limit-knowledge-base-{zh,en,zh-TW,ja}.md` | 本知识库的四语言版本 |

---

## 13. 版本历史

| 版本 | 日期 | 内容 |
| --- | --- | --- |
| V1.0 | 2026-09-17 | 金额/Token 限额、Proxy enforcement、指纹绑定、手动重置、四态图标、i18n、全链路测试 |
| V1.0.1 | 2026-09-18 | 重置周期（never/hourly/daily/weekly/monthly，本地时区懒滚动）、「下次重置」展示、schema v21、周期恢复集成测试 |
| V1.0.2 | 2026-09-18 | 金额限额扩展至 5 币种（USD/CNY/EUR/JPY/GBP）+ 各币种人工汇率、汇率命令泛化、schema v22（移除 currency CHECK）、币种符号后缀等 UI 打磨 |
| **V1.1.0** | 2026-09-18 | **公开发布**：V1.0 + V1.0.1 + V1.0.2 的全部内容作为一个版本发布（基座 CC Switch 3.20.3），即 v1.0.0 之后的第一个公开版本 |
| **V1.1.1** | 2026-09-18 | **安装包修复**：应用包完整 ad-hoc 签名（修复「文件已损坏」与首次拖拽不注册）、DMG 移除杂散 `.VolumeIcon.icns`、README/发布说明补充首次打开放行指引。功能与 V1.1.0 一致 |
