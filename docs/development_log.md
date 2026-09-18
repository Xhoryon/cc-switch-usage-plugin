# 开发日志

## 2026-09-17 — API Key 使用限额 / Budget Limit

### Feature

为每个 API Key 增加可选的使用限额（Budget Limit）：

- 两种限制方式（同一时间一种）：金额（USD / CNY）或 Token 数量
- 达到限额后，**Local Proxy 在转发前拒绝新请求**（真正的 enforcement，不是仅统计）
- Provider Card 增加限额图标（OFF / ACTIVE / WARNING >= 80% / EXHAUSTED 四态）
- Dialog 内配置：开关、限制方式、币种、最大金额 / Token、当前已用、剩余额度、
  进度条、重置使用量
- i18n：zh / zh-TW / en / ja 全量补齐

### Architecture decision

```
Provider Card
  └─ UsageLimitButton（图标，自取状态）
       └─ UsageLimitDialog
            └─ Tauri Command（get_usage_limit_status / save_usage_limit /
               reset_usage_limit / get_usd_cny_rate / set_usd_cny_rate）
                 └─ Database impl（services/usage_limit.rs）
                      ├─ api_key_limits 表（配置）
                      └─ proxy_request_logs 聚合（用量，SSOT 复用）

Client Request
  └─ CC Switch Local Proxy
       └─ Budget Guard（forward_with_retry_inner 每 provider attempt，转发前）
            ├─ 未启用 → 放行
            ├─ credential 指纹与配置不一致 → 重绑 + 窗口重置
            └─ 达到限额 → ProxyError::BudgetExhausted（429，不发上游）
                 └─ 响应完成后 Usage Accounting（log_with_calculation，
                    携带 credential_fingerprint 落 proxy_request_logs）
```

关键决策：

1. **用量不冗余存储**（避免双 SSOT）：`api_key_limits` 只存配置与统计窗口起点
   `usage_start_at`；用量始终从 `proxy_request_logs` 按
   `(provider_id, app_type, credential_fingerprint, created_at >= usage_start_at,
   data_source = 'proxy')` 实时聚合。Token 数用与 Usage Dashboard 完全一致的
   normalized total 规则（`fresh_input_sql` + output + cache_creation +
   cache_read）；金额直接累加已落库的 `total_cost_usd` 十进制字符串。
2. **credential 身份**：SHA-256 不可逆指纹（`credential_fingerprint`）。任何新表、
   日志、Usage Record 都不保存明文 API Key；日志展示仅 `sk-****ABCD` 遮蔽形式。
   复用代理转发同款 `adapter.extract_auth` 路径解析 Key，保证「配置绑定的指纹」
   与「实际转发的指纹」一致。OAuth 类 Provider（codex_oauth / xai_oauth /
   managed account / copilot）无静态 Key → 状态标记 unsupported_credential，
   不允许设置按 Key 限额。
3. **Key 轮换**：guard / 保存时发现指纹与配置行不一致 → 自动重绑到新指纹并把
   `usage_start_at` 重置为当前时间。旧 Key 的历史 Usage 不并入新 Key。
4. **金额精度**：SQLite 内成本为十进制字符串。判定采用两级通道——REAL 快速预判
   （margin 1e-6，远离阈值零开销放行）+ 贴近阈值时逐行 `rust_decimal` 精确求和
   复核，避免 9.999999999 式浮点误判。CNY 限额通过本地可配置汇率
   （settings 键 `usage_limit_usd_cny_rate`，默认 7.2）换算，无在线汇率依赖。
5. **enforcement 位置**：`forward_with_retry_inner` 的 per-provider attempt 循环内、
   `forward()` 调用前。failover 链上每个 Provider 各查各的 Key 预算；
   `BudgetExhausted` 在 `categorize_proxy_error` 中落入 NonRetryable，不会继续
   消耗其他 Provider 的尝试名额。DB 读取失败时 fail-open（warn + 放行），
   与熔断器降级思路一致。
6. **并发**：判定读与用量写都在 `Database` 的 Mutex 下串行；真实用量在响应结束
   后才落库，同时进行的多个请求仍可能产生少量 overshoot——属于设计内行为
   （limit enforcement applies before new requests; an already-running request
   is allowed to finish）。

### Database changes

- `SCHEMA_VERSION` 19 → 20，`migrate_v19_to_v20`（幂等，向后兼容，自动迁移）：
  - 新表 `api_key_limits`：PK (provider_id, app_type)，列
    credential_fingerprint / enabled / limit_type(money|token) /
    currency(USD|CNY) / limit_amount（十进制或整数字符串）/ usage_start_at /
    created_at / updated_at；FK → providers ON DELETE CASCADE
    （Provider 删除时限额配置级联清理）
  - `proxy_request_logs` 增加 `credential_fingerprint TEXT` 列（历史行 / 会话
    同步行 / OAuth 行为 NULL，不参与预算）
  - 新索引 `idx_request_logs_budget (provider_id, app_type,
    credential_fingerprint, created_at)`
- 启动时预迁移备份逻辑照常生效

### Backend changes

- `database/dao/usage_limit.rs`（新）：配置 CRUD、重置窗口、凭证重绑、
  用量快速聚合、成本字符串精确取数、零成本计价模型清单
- `services/usage_limit.rs`（新）：指纹 / 遮蔽、credential 解析、
  `check_budget_before_forward`（Budget Guard）、`get_budget_status`、
  `save_budget_config`（含输入验证）、`reset_budget_usage`、USD→CNY 汇率读写、
  `validate_limit_amount`
- `commands/usage_limit.rs`（新）+ lib.rs 注册 5 个命令
- `proxy/error.rs`：`BudgetExhausted { message, detail }` → 429 +
  `{"error": {"type": "usage_limit_reached", "provider", "limitType", "currency",
  "used", "limit", ...}}`（与既有 `{"error": {...}}` 约定一致）
- `proxy/forwarder.rs`：Budget Guard 接入 failover 循环；`ForwardResult` 增加
  `credential_fingerprint`
- `proxy/handlers.rs` / `response_processor.rs`：指纹随 ctx 透传至记账
- `proxy/usage/logger.rs`：`RequestLog.credential_fingerprint` 落库

### Proxy enforcement

- **检查时机**：每个 provider attempt 转发之前（在选择/熔断放行之后）。
- **BLOCK 时机**：`used >= limit`（token 为 normalized total 整数比较；金额为
  Decimal 比较）。被拒请求返回 429 + 结构化错误体，上游零流量。
- **Streaming**：已允许发出的请求正常完成（SSE / tool call / reasoning 不受
  影响）；usage 在流结束的最终 chunk 中解析后记账。
- **Overshoot**：最后一个请求可能小幅超出（响应期间无法预知最终用量）；UI
  明示"新请求被阻止，进行中的请求允许完成"。

### Frontend changes

- `types/usageLimit.ts` / `lib/api/usageLimit.ts` / `lib/query/usageLimit.ts`（新）：
  类型、invoke 封装、TanStack Query keys + useUsageLimitStatus /
  useSaveUsageLimit / useResetUsageLimit / useUsdCnyRate / useSetUsdCnyRate
  （mutation 成功后 invalidate 对应 query，无 window.location.reload）
- `components/usage-limit/UsageLimitButton.tsx`（新）：Gauge 图标四态
  （muted / emerald / amber / red），Tooltip 展示 `$2.41 / $10` 或 `1.2M / 5M`
- `components/usage-limit/UsageLimitDialog.tsx`（新）：shadcn Dialog；默认 OFF
  只显示开关；开启后同 Dialog 内展开配置；CNY 汇率本地输入；重置使用量带确认框；
  未知定价 / enforcement 不可用显式警示
- `ProviderActions.tsx`：新增使用限额图标按钮（仅代理类应用渲染）
- `ProviderCard.tsx`：托管 UsageLimitDialog
- `hooks/useUsageEventBridge.ts`：usage-log-recorded 事件同步 invalidate
  usage-limit 命名空间（请求完成后卡片/Dialog 状态即时刷新）
- i18n：4 个 locale 增加 `usageLimit` 命名空间（30 keys × 4）

### 验收轮修复（2026-09-17 交接验收轮）

1. **v20 迁移列防御**：全量 `cargo test` 发现 3 个旧版迁移测试失败——
   `migrate_v19_to_v20` 建 `idx_request_logs_budget` 前只检查了表存在，
   极简旧库（迁移测试手工构造的 `proxy_request_logs`）缺 `provider_id`/
   `app_type` 列导致 `no such column`。已按
   `create_request_logs_usage_indexes_if_supported` 的既有模式补列存在性
   检查（真实库两列由 `create_tables_on_conn` 保证，纯防御）。
2. **主界面限额状态实时刷新**：`useUsageEventBridge` 原本只挂 UsageDashboard，
   主界面卡片/Dialog 在请求记账后不刷新（不满足"Request 完成后状态必须更新"）。
   新增轻量 `useUsageLimitEventBridge`（只 invalidate usage-limit 命名空间）
   挂到 `ProviderList`，一个监听覆盖全部卡片。

### Tests

- Rust 单元测试（`services/usage_limit/tests.rs`，覆盖需求 §38 全部条目）：
  限额关闭 / 无配置 / 金额低于·达到·超过 / Token 低于·达到·超过 / Reset 旧用量
  不计入 / 换 Key 不继承 / USD↔CNY 换算 / 浮点精度（0.1+0.2）/ 未知定价显式暴露 /
  会话日志行不计入 / 并发记账无丢失写 / 输入验证 / 指纹不可逆 / 遮蔽不泄漏 /
  保存语义（校验、关闭保留、换 Key 重绑、OAuth 拒绝）/ 状态机
  （off-active-warning-exhausted）/ enforcement 开关联动 / reset 命令
- Proxy 集成测试（`proxy/forwarder.rs` tests）：Case 1（token 1100/1000 → 下一
  请求 BLOCK 于转发前）、Case 2（money $1.02 → 不发 upstream）、Case 4（未配置 /
  已关闭 → 绝不出现 BudgetExhausted）、429 状态映射
- 记账链路测试（`proxy/usage/logger.rs`）：流式请求完成后 usage 带指纹落库 →
  Budget Guard 随即按 SSOT 拦截（Case 3 记账 + Case 1 前半）
- 前端测试（`tests/components/UsageLimitDialog.test.tsx`，覆盖需求 §40）：
  OFF 默认 / Toggle 展开无二级 Dialog / Money CNY↔USD 切换 / Token 输入 /
  非法输入不保存 / progress used-remaining-percent / exhausted 不 clamp /
  Reset 确认后调用 mutation / API 错误可理解提示 / 图标四态与 tooltip /
  非代理应用不渲染 / 格式化工具

### Known limitations

- **Budget enforcement only applies to traffic routed through CC Switch Local
  Proxy.** 直连（未开启代理接管）的流量 CC Switch 观测不到，也无法拦截；此时
  UI 明确显示 enforcement 不可用（proxy_disabled），不假装 Hard Limit 生效。
- 本机观测边界：同一个 API Key 在其他设备 / 其他程序直连产生的用量不计入本机
  预算（usage is local observation，不是 Provider 账户的全局配额）。
- 金额模式依赖模型定价：缺失定价的请求成本记为 0，UI 会显示
  unpriced 警示并引导配置模型价格；金额判定按已计价成本进行。
- 已允许发出的请求允许完成，故存在设计内的少量 overshoot（见 Proxy enforcement）。
- OAuth 类 Provider（ChatGPT / Grok / Copilot 登录）没有静态 API Key，V1 不支持
  按 Key 限额（UI 明示 unsupported credential）。
- 限额配置不参与 Cloud Sync / 导入导出（V1：usage is local observation；配置行
  为本机决策，跟随设备），不会产生跨设备重复统计。

### Verification

见工作日志最新条目（typecheck / format:check / test:unit / cargo fmt / clippy /
test / build 逐项结果）。
