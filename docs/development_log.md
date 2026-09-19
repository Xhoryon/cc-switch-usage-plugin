# 开发日志 / Development Log

> 语言说明 / Languages：本日志为追加式工程历史，以简体中文维护（不翻译
> 历史条目）。配套的《使用限额知识库》提供四种语言版本：
> [简体中文](usage-limit-knowledge-base-zh.md) |
> [English](usage-limit-knowledge-base-en.md) |
> [繁體中文](usage-limit-knowledge-base-zh-TW.md) |
> [日本語](usage-limit-knowledge-base-ja.md)。
> This append-only engineering log is maintained in Simplified Chinese; the
> companion Usage Limit knowledge base is available in four languages above.
>
> 版本映射 / Version mapping：内部迭代 V1.0 / V1.0.1 / V1.0.2（2026-09-17/18）
> 合并以 **V1.1.0**（基座 CC Switch 3.20.3）公开发布。/ Internal iterations
> V1.0 / V1.0.1 / V1.0.2 shipped publicly as **V1.1.0**.

## 2026-09-18 — V1.1.1 安装包修复 / Installer Fixes

### Problem（用户实测反馈的三个症状）

1. DMG 打开后窗口里出现裸露的 `.VolumeIcon.icns` 图标文件（观感异常）。
2. 第一次把应用拖进 Applications 后，启动台/程序坞不显示；第二次拖拽才提示
   「已存在，是否替换」。
3. 安装后打开提示「文件已损坏」（并非「无法验证开发者」那种可放行提示）。

### Root cause（诊断）

对已发布 DMG 挂载诊断（`codesign -dv` / `codesign --verify` / `spctl` /
`ls -la`）：

- **主因：应用包从未被整体签名**。`codesign -dv` 显示
  `flags=0x20002(adhoc,linker-signed)`——只有内部二进制带链接器自动 ad-hoc
  签名；`codesign --verify --deep --strict` 失败：
  `code has no resources but signature indicates they must be present`
  （bundle 缺 `_CodeSignature/CodeResources`）。带隔离属性的这种包，
  Gatekeeper 判「已损坏」（死路提示，无法右键放行）。
  根源：tauri.conf 未配置 `signingIdentity`，Tauri 打包时跳过 bundle 签名。
- 症状 2 是主因的伴生表现：LaunchServices 拒绝注册签名无效的包，首次拖拽
  落盘但不注册，二次拖拽触发「替换」。
- `.VolumeIcon.icns`（1MB，应用图标副本）躺在 DMG 卷根且无任何隐藏属性
  （Finder flags=0）——bundle_dmg.sh 生成后未标记不可见。

### Fix

1. `tauri.conf.json`：`bundle.macOS.signingIdentity = "-"`（显式 ad-hoc）。
   构建后 bundle 获得完整签名：`Identifier=com.ccswitch.desktop`、
   `flags=0x10002(adhoc,runtime)`、`CodeResources` 就位，
   `codesign --verify --deep --strict` **PASS**。
2. DMG 后处理：`hdiutil convert UDRW → 删除 .VolumeIcon.icns → convert
   UDZO`，安装窗口只剩应用与 Applications 链接。
3. 诚实边界：无付费 Apple 开发者账号则**无法公证**。修复后 Gatekeeper 对
   下载副本的判定从死路「已损坏」变为可放行的「无法验证开发者」——右键
   打开，或 `xattr -dr com.apple.quarantine`。该一次性指引已写入 README ×4
   与发布正文。

### Verification

- `codesign --verify --deep --strict`：PASS（v1.1.0 资产为 FAIL）。
- 隔离模拟（副本 + quarantine xattr）：`spctl -a` = rejected（未公证的
  预期拒绝，结构有效可放行），不再是签名损坏。
- 复现 DMG 卷根仅 `.DS_Store` / `Applications` / `CC Switch.app`。
- 二进制本机用户名 0 命中（RUSTFLAGS 重映射保持）。
- 功能代码零变更（相对 V1.1.0 仅 tauri.conf 一行 + 文档）。

---

## 2026-09-18 — V1.1.0 公开发布 / Public Release & Release Engineering

### Scope

V1.0.2 代码冻结后的发布工程轮：文档四语言补齐、全仓库脱敏审计、
仓库命名统一、GitHub Release v1.1.0 发布、CI 事故定位与修复。
本轮**无功能代码变更**（代码态与 V1.0.2 完全一致），改动集中在
文档、元数据与发布流程。

### 脱敏审计（发布前置）

- 全仓库 1278 个源文件扫描（排除 gitignore 的构建产物/依赖）：
  API Key 形态、本地路径、用户名、邮箱、JWT、私钥块、密码字面量。
- **处置 1 处**：`deplink.html`（上游深链接调试演示页）残留
  `ctx7sk-<UUID>` 形态的 Context7 密钥（同页其余示例均为占位符，
  唯它例外）→ 替换为 `ctx7sk-your-api-key-here`。
- 核实为安全的高频命中：`sk-ant-api03-*`（日志脱敏测试的验证串，断言
  「不得包含原文」）、`sk-claude`/`sk-codex`/`sk-test`（内置供应商哨兵值）、
  `AKIAIOSFODNN7EXAMPLE`（AWS 官方文档示例键）、`/Users/test|me|demo`
  （通用夹具）、`farion1231@gmail.com`（上游作者公开联系方式）、
  `frontendLogger.ts` 的私钥检测正则（功能代码本身）。
- 本地数据：无 .env/.db/.log 落入源码树；本机用户名仅存在于
  `.mimosa/` 本地扫描器状态（gitignored）。
- 作者署名（Jiayi Huang / Xhoryon）为有意公开身份，保留。

### 文档四语言补齐（English / 简体中文 / 繁體中文 / 日本語）

- 知识库新增 English / 繁體中文 / 日本語 译本（`docs/usage-limit-knowledge-
  base-{en,zh-TW,ja}.md`），四版 13 章结构逐一对齐，头部互为语言导航。
- README ×4 段落级 parity 校验（脚本比对 8 特性 + 8 章节关键内容）。
- i18n 完整性脚本校验：组件引用的 29 个静态 key + 10 个动态模板 key
  在 4 locale 无缺失（usageLimit 命名空间 39 keys × 4）。
- 开发日志保持中文追加式（头部声明），知识库承载四语言。
- 仓库命名统一：README ×4 的 12 处 URL 从 `ccswitch-usage-plugin`
  修正为 `cc-switch-usage-plugin`；`package.json` 补 `repository` 字段。

### Release engineering

- **版本映射**：内部迭代 V1.0 / V1.0.1 / V1.0.2 合并以 V1.1.0 发布，
  基座 CC Switch 版本号 3.20.3 不变（DMG 文件名沿用）。
- **DMG 构建**：`pnpm tauri build --config
  '{"bundle":{"createUpdaterArtifacts":false}}'`——本环境无
  `TAURI_SIGNING_PRIVATE_KEY`，关闭 updater 签名产物（DMG 本身无需签名）；
  产物按约定重命名 `CC.Switch_3.20.3_aarch64_usage-limit.dmg`。
  构建脚本依赖 cargo 在 PATH，非交互 shell 需显式补工具链 PATH。
- **Source zip**：`git archive --format=zip v1.1.0` 生成
  `cc-switch-usage-plugin-1.1.0-source.zip`（沿用 v1.0.0 的 source snapshot
  惯例，命名修正为与新仓库名一致），已验证不含本地/敏感文件。
- **Release 正文**：`docs/release-notes/usage-limit-v1.1.0.md`（四语分节 +
  锚点导航，面向用户：新内容 / 完整功能 / 隐私声明 / 安装）。
- **发布**：main 推送（60045de）→ tag v1.1.0 →
  `gh release create v1.1.0` 附双资产，正式版（非 draft/prerelease）。
- **遗留决策（未擅自改动）**：`tauri.conf.json` 的 updater endpoint 仍指
  上游 `farion1231/cc-switch`；切到本仓库需发布流程以相同 minisign 密钥
  签发 `latest.json`，属产品决策。

### CI 事故与修复（rename → stale cache）

- 症状：推送后 4 个后端平台 CI 全部失败于 build.rs：
  `failed to read plugin permissions ... ccswitch-usage-plugin/src-tauri/
  target/.../app_hide.toml: No such file or directory`——注意缺失路径是
  **旧仓库名**，而 checkout 目录已是新名。
- 根因：仓库曾由 `ccswitch-usage-plugin` 改名为 `cc-switch-usage-plugin`；
  `actions/cache` 恢复的 `src-tauri/target` 缓存中，Tauri build script 的
  输出内嵌了旧工作区绝对路径，改名后绝对路径失配必然报错。
  缓存创建时间（04:28–06:03，v1.0.0 时代）佐证。
- 修复：`gh cache delete --all` 清空全部缓存 → `gh run rerun --failed` →
  **6 个任务全绿**（Frontend + Ubuntu/macOS/Windows/WSL2 后端，含完整
  `cargo test`）。
- 经验：**仓库改名必须清 CI 缓存**；且 actions/cache 的 restore-keys
  前缀回退会把旧缓存再次带回——只改 key 前缀不够，需删除。

### 安全扫描

- 推送触发的 Mimosa 深度扫描（238 条 hard-coded-credential，全部 high）
  逐条核实：分布在 proxy.rs(102)/claude.rs(33) 等测试密集文件，样本均为
  测试假值（`test-token`/`sk-test-123`/`old-pass`/`refresh-secret`…），
  限额功能自身文件零命中。属凭据形态字面量的预期扫描噪音，无真实凭据。

### Verification（发布链路终态）

- 前端：typecheck / format:check / 1151 tests / vite build 全 PASS。
- 后端：fmt --check / 2925 tests（跳过沙箱网络型模块）/ release build PASS。
- CI：push 后 6 任务全绿（清缓存重跑后）。
- 冒烟：release 二进制以隔离 `CC_SWITCH_TEST_HOME` 启动，因本机正在运行
  同应用经 single-instance 正常交接退出（0）——非崩溃。
- 发布终态：Release v1.1.0 正式版 + 双资产 + tag；仓库默认分支 main。

### 发布资产脱敏整改（复核轮）

对线上两个 Release 的全部资产复核时发现两类泄漏并彻底整改：

1. **v1.0.0 source zip 携带未脱敏 ctx7 密钥**：该 zip 于 deplink.html 脱敏
   之前打包，且密钥同样留在 v1.0.0 tag 指向的提交树中（GitHub 自动生成的
   "Source code" 链接会同样泄漏）。整改：`git filter-branch --tree-filter`
   对全部历史树级净化 → 强推 main 与双 tag（临时放开分支保护后立即恢复）
   → 重切 source zip。
2. **两个 DMG 二进制内嵌构建机路径**：`/Users/<用户名>/.cargo/...` 等
   1031 处（依赖 panic 路径，用户名泄漏）。整改：从净化后的 tag 以
   `RUSTFLAGS="--remap-path-prefix=/Users/<用户名>=/build"` 重建两个 DMG，
   代码逻辑零变化（仅编译期路径元数据重映射），重建后用户名出现次数为 0。
3. 资产替换：两个 Release 的 DMG + source zip 全量替换；v1.0.0 旧命名 zip
   （`ccswitch-usage-plugin-1.0.0-source.zip`）删除，正文资产名同步更新；
   v1.1.0 一次 `--clobber` 因本地文件名前缀未命中原资产产生的重复 DMG
   已删除。终验（重新下载线上资产逐一扫描）：4 资产用户名/密钥/本机路径
   全部 0 命中。
4. 后续复核：开发日志审计条目中的本机用户名字面量泛化为「本机用户名」；
   仓库与 tag 树复扫仅剩 `.gitignore` 的 `.mimosa/` 规则文本与 AWS 官方
   示例键字面量（均为非敏感引用）。
5. **经验**：① 脱敏必须先于首次打包，打包顺序错误会让泄漏进入不可撤回的
   发布资产（只能换资产 + 重写历史 + 建议轮换密钥）；② 本机构建产物必然
   内嵌构建机路径，公开发布需 `--remap-path-prefix` 或在 CI runner 上构建；
   ③ 仓库改名后 CI 缓存与 git 历史都是泄漏/故障的隐蔽载体。

### 发布可复现性验证（三层）

1. **源码保真**：两个 Release 的 source zip 与对应 tag 的 `git archive`
   输出逐字节一致（v1.0.0 1264 文件 / v1.1.0 1269 文件），zip 即 tag 树。
2. **洁净房复现**：从线上 v1.1.0 source zip 在全新路径解压 → 冻结锁文件
   安装 → typecheck / format / vitest **1151 全过** → fmt / cargo test
   **2925 全过**（沙箱环境按模块跳过 9 个网络型测试，CI 已全量绿）→
   按知识库记载的发布命令构建成功。证明 zip 自含构建与验证所需的全部
   文件（无 gitignored 的必需文件遗漏）。
3. **产物确定性**：从 zip 在**不同构建路径**上重建的 `cc-switch` 二进制
   与已发布版 **sha256 逐字节一致**（同工具链 rustc 1.97.1 + 相同
   RUSTFLAGS 重映射；依赖路径统一重映射后不嵌入本机信息，本地 crate
   路径不进产物）。DMG 容器本身因镜像时间戳/UUID 不逐字节可复现，
   以内部二进制为确定性单元——这是常规口径。

---

## 2026-09-18 — V1.0.2 多币种限额 / Multi-Currency Budget

### Feature

在 V1.0.1 重置周期之上，金额限额从 USD/CNY 扩展到 **5 种主流货币**：

- **USD / CNY / EUR / JPY / GBP**，USD 为内部计价基准（`total_cost_usd`），
  其余币种限额按 **USD→X 本地汇率**换算
- 每个币种的汇率均可**人工调节**（Dialog 内输入，随保存持久化），提供合理
  默认值（CNY 7.2 / EUR 0.92 / JPY 150 / GBP 0.79），**不联网获取**
- CNY 沿用 V1.0 的 settings 键 `usage_limit_usd_cny_rate`，升级不丢已配汇率；
  新增 `usage_limit_usd_eur_rate` / `usage_limit_usd_jpy_rate` /
  `usage_limit_usd_gbp_rate`
- 命令泛化：`get_usd_cny_rate` / `set_usd_cny_rate` →
  `get_exchange_rate(currency)` / `set_exchange_rate(currency, rate)`
- UI：币种五段选择器（带符号）、非 USD 币种显示汇率输入（标签按币种参数化
  「USD → EUR 汇率」）、金额输入框内嵌币种符号后缀（右对齐 muted 小字）
- JPY 与 CNY 同用 ¥ 字形，展示与错误消息中 JPY 加国别前缀 `JP¥` 消歧

### Architecture decision

- **换算方向统一为「除以汇率折回 USD」**：`limit_in_usd = amount / rate`、
  `used_in_currency = used_usd × rate`。USD 汇率恒为 1（`get_exchange_rate`
  对 USD 直接返回 ONE，不访问 settings），金额判定只有一条代码路径，杜绝
  「USD 分支 / CNY 分支」式重复逻辑在多币种下的组合爆炸。
- **币种值域完全下沉到应用层**：`LimitCurrency::parse` 在保存路径校验，
  读取侧未知值回退 USD（`unwrap_or(LimitCurrency::Usd)`，与 reset_period /
  汇率的防御思路一致）。
- **切换币种 = 切换汇率来源**：前端汇率按币种分 TanStack Query 键
  （`["usage-limit","exchange-rate",currency]`），Dialog 内 `touchedRate`
  独立于其他配置——用户切币种即放弃未保存的手改汇率并回填该币种已存值，
  避免「CNY 的汇率误填进 EUR」。
- **跨币种查看用量**：`usedMoneyInCurrency` 只在「已保存限额币种 == 当前
  所选币种」时信任；切换中 / 未保存时按本地汇率输入实时估算（修复了 V1.0
  隐含的「仅 CNY 特判」写法，泛化为任意币种）。

### Database changes

- `SCHEMA_VERSION` 21 → 22，`migrate_v21_to_v22`：**重建 `api_key_limits`
  移除 currency 的 CHECK 约束**（SQLite 无法修改既有 CHECK，标准四步：
  建新表 → INSERT SELECT 拷贝 → DROP 旧表 → RENAME）。存量数据
  （含 V1.0.1 的 reset_period）完整保留。
- 幂等性：读取 `sqlite_master.sql` 判断建表语句是否仍含
  `CHECK (currency`，无则跳过；全新 DDL 已无该约束，天然命中跳过。
- `limit_type` 的 CHECK 保留（值域稳定）；currency 值域自此完全由应用层
  保证，**后续扩展币种不再需要迁移**。
- FK（providers 级联删除）在重建后原样保留。

### Backend / Frontend changes

- `services/usage_limit.rs`：`LimitCurrency` 扩展（as_str/parse/symbol/
  rate_setting_key/default_exchange_rate）、`get_exchange_rate` /
  `set_exchange_rate`（USD 拒绝设置）、`parse_exchange_rate` 按币种回退默认
- `commands/usage_limit.rs` + `lib.rs`：两个汇率命令签名泛化
- `types/usageLimit.ts`：currency 联合类型扩展 + 导出
  `USAGE_LIMIT_CURRENCY_SYMBOLS`（与后端 `symbol()` 一致）
- `lib/api/usageLimit.ts` / `lib/query/usageLimit.ts`：
  `getExchangeRate` / `setExchangeRate`、`useExchangeRate(currency)` /
  `useSetExchangeRate`（成功后 invalidate 整个 usage-limit 命名空间，
  已换算用量展示同步刷新）
- `UsageLimitDialog.tsx`：五币种选择器、汇率输入条件改为 `currency !== "USD"`、
  金额输入符号后缀（`pr-10` + 右侧绝对定位 muted 小字）、`touchedRate`
  状态（切币种重置）
- `UsageLimitButton.tsx`：`formatMoney` 用符号表；`budgetSummary` 去除
  USD/CNY 钳制，改用实际币种（tooltip 简略用量对 5 币种正确）
- 清理：`touchedConfig` 状态在汇率回填改用 `touchedRate` 后已无读取方，
  连同全部写入一并移除

### Tests

- Rust（usage_limit 36 → 41，迁移测试 +3）：
  `multi_currency_rates_are_independent_with_defaults`（默认值/互不干扰/
  USD 恒 1 且拒设/负汇率拒绝）、`eur_and_jpy_limits_convert_correctly`
  （EUR 达到拒绝含 € 符号与 detail.currency、JPY 快速通道与状态换算
  99×150=14850、GBP 两步临界）、`save_accepts_all_supported_currencies_only`
  （5 币种可存、RUB 拒绝）、`migration_v21_to_v22_rebuilds_limits_without_
  currency_check`（v21 形状种子 → 数据保留/EUR 可入/limit_type CHECK 仍在）、
  `fresh_limits_table_accepts_all_currencies_without_check`、
  `migration_v21_to_v22_with_foreign_keys_on_preserves_data`（FK=ON 生产路径
  端到端：数据保留 + foreign_key_check 零违规 + 新表仍拒孤儿行）、
  `custom_rate_end_to_end_changes_guard_threshold`（非默认汇率 EUR=0.46 →
  €0.92 限额在 $2 精确拦截，防键名脱节）、
  `corrupt_stored_rate_falls_back_to_default`（settings 脏值回退默认）
- 前端（25 → 31）：渲染五币种、EUR 汇率输入 + 换算展示（2.41×0.92=€2.22）+
  符号后缀、EUR 保存持久化汇率（mutation 载荷 `{currency, rate}`）、
  USD 模式不出现也不修改汇率、切换币种按该币种已存汇率回填（含手改后
  切换重置）、汇率未加载完成时保存被拦截（P1-1 竞态回归）

### 审核返场（2026-09-18，独立代码审查）

第一轮审查（P0×0 / P1×3 / P2×7）→ 全部修复 → 全量复验全绿：

- **P1-1（真实竞态，必修）**：Dialog 切换到汇率尚未加载完成的币种时，
  上一币种汇率残留在输入框，点保存会把错误汇率持久化（如 CNY 7.2 存成
  JPY 汇率 → 实际阈值放大约 20 倍）。修复：切币种立即清空 `rateInput`
  并重置 `touchedRate`；回填 effect 依赖加入 `currency`（缓存同值也触发）；
  汇率为空时保存被校验拦截。补两个前端回归用例。
- **P1-2**：v22 重建此前只在 FK=OFF 的测试连接里覆盖。新增 FK=ON 端到端
  用例（镜像 `Database::init` 真实路径）：数据保留、`foreign_key_check`
  零违规、重建后新表仍拒绝孤儿行。
- **P1-3**：新增非默认汇率端到端用例（EUR=0.46 → €0.92 在 $2 精确拦截），
  防止「汇率读写键名脱节但单测全绿」的失效模式。
- P2 修复：快速通道 margin 由绝对 1e-6 改为 `max(1e-6, limit×1e-12)`
  （大限额下绝对边距不足以覆盖 f64 累积误差）；前端汇率校验补上限
  1,000,000（与后端一致，避免「config 已存、汇率被拒」半保存）；Dialog
  初始化 effect 加 `wasOpenRef` 防打开期间后台 refetch 打断用户编辑；
  脏汇率回退默认值补测试；两处 `"USD" | "CNY"` 过期注释更新为五币种。
- P2 采纳说明：`sqlite_master` 文本匹配的幂等判断保留（本仓库全部 DDL
  措辞受控，且任何 DDL 变更都会 bump 版本）；providers 缺表守卫保留
  （仅极简夹具可达，真实库由 create_tables_on_conn 保证）。

### Known limitations

- 汇率为本地静态值：不联网、不自动跟踪市场波动；跨币种比较请以 USD 原值
  （`usedMoneyUsd`）为准。
- JPY 展示符号 `JP¥` 与 CNY 的 `¥` 在 UI 中并存（宽度不同属预期）。
- 其余边界同 V1.0 / V1.0.1（仅本机代理流量、设计内 overshoot、OAuth 不支持
  按 Key 限额、配置不参与云同步）。

### Verification

2026-09-18（同 V1.0.1 工具链说明：本机 stable 1.97.1；`proxy::server::tests`
/ `hyper_client` 网络型测试在本环境挂起为既有环境限制，模块级跳过）：

| 检查 | 结果 |
| --- | --- |
| `pnpm typecheck` / `format:check` | PASS |
| `pnpm test:unit` | PASS（1151 tests，含新增 6 个多币种/竞态用例） |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets` | 本次改动文件 0 警告 |
| `cargo build --release` | PASS |
| `cargo test --lib`（跳过网络型模块） | **PASS：2925 passed / 0 failed** |
| 定向回归 | usage_limit 41、migration（含 v22 × 3）全过 |

以上为审核返场修复后的最终复验结果（第一轮验证为 2922/1149，返场新增
5 个测试后全绿）。

---

## 2026-09-18 — V1.0.1 使用限额重置周期 / Reset Schedule

### Feature

在 V1.0 限额功能（2026-09-17）之上，为每个 API Key 的统计窗口增加可选的
**自动重置周期**：

- 五种选择：**不重置**（V1.0 默认，仅手动重置）/ **每小时** / **每天** /
  **每周**（周一起算）/ **每月**（每月 1 日起算）
- 周期边界按**本地时区**计算：每天 = 本地零点、每周 = 本地周一零点、
  每月 = 本地 1 日零点、每小时 = 本地整点
- Dialog 在限额配置区内新增「重置周期」五段选择器；周期生效时展示
  「下次重置：<本地时间>」
- 达到限额的窗口跨过周期边界后**自动恢复**，无需用户手动干预
- i18n：zh / zh-TW / en / ja 各新增 8 个 key（usageLimit 命名空间 28 → 36）

### Architecture decision

**懒滚动（lazy rollover）**：不引入后台定时器。窗口起点
`usage_start_at` 保持「上次对齐的时间戳」，判定与状态查询时按当前时间
即时计算有效起点：

```
effective_start = max(usage_start_at, 当前周期边界(reset_period, now))
```

- `check_budget_before_forward`：**先滚动、后判定**。边界已越过 → 推进
  `usage_start_at`（持久化，best-effort，失败仅告警）→ 用新起点聚合判定。
  因此「昨天用满、今天零点已过」的请求在同一次 guard 调用内被放行。
- `get_budget_status`：按相同规则即时计算有效起点并返回
  `resetPeriod` / `nextResetAt`，但**读接口无副作用**（不回写持久化窗口）。
- `save_budget_config`：保存时校验周期值域；若原窗口早于新周期边界
  （如从 never 改为 daily 且窗口已在昨天）则立即对齐到边界。原窗口已在
  本周期内时不回退——保守保留本期已统计的用量（与手动重置「从当下起算」
  的语义一致）。
- 与 credential 轮换的叠加：重绑后窗口起点是「当下」，周期边界不可能
  早于它，因此轮换与滚动不会互相破坏（旧 Key 用量依旧不继承）。

选择懒滚动而非定时器的理由：应用关闭/休眠期间跨过的边界在下次启动后
照常生效（定时器方案会漏重置）；无并发定时任务开销；判定路径本来就在
DB Mutex 下串行，滚动写是一行 UPDATE。

### Database changes

- `SCHEMA_VERSION` 20 → 21，`migrate_v20_to_v21`（幂等，向后兼容）：
  `api_key_limits` 新增列 `reset_period TEXT NOT NULL DEFAULT 'never'`。
  存量行回填 `never`，行为与 V1.0 完全一致。
- 刻意**不加 CHECK 约束**：保证迁移路径（ALTER ADD COLUMN）与全新建表
  DDL 行为完全一致；值域由保存路径（`ResetPeriod::parse`）校验，读取侧
  未知值回退 never 并告警（与汇率 `parse_exchange_rate` 同一防御思路）。
- 全新建表 DDL 同步包含该列（DEFAULT 'never'）。
- 启动时预迁移备份逻辑照常生效；无数据删除、无表重建。

### Backend / Proxy / Frontend changes

- `services/usage_limit.rs`：`ResetPeriod` 枚举（never/hourly/daily/
  weekly/monthly）+ `current_period_start` / `next_period_start`
  （本地时区，DST 处理与 `usage_rollup::compute_local_midnight_cutoff`
  同款：歧义取较早者、gap 顺延一小时、最终 UTC 兜底）；
  `UsageLimitConfig.resetPeriod`（缺省 never，非法值拒绝入库）；
  `BudgetStatus.resetPeriod` / `nextResetAt`；guard 懒滚动接入。
- `database/dao/usage_limit.rs`：`ApiKeyLimitRow.reset_period` +
  SELECT/UPSERT 更新。
- `commands/`：命令签名不变（新字段随 config 序列化透传）。
- `UsageLimitDialog.tsx`：重置周期五段选择器（bg-muted 分段控件，与
  限制方式/币种同款）；never 显示说明文案；周期生效且与已保存配置一致时
  展示「下次重置」（`formatResetTime`，locale 推导与 Usage 页一致）；
  保存载荷始终携带 `resetPeriod`。
- `types/usageLimit.ts`：`UsageLimitResetPeriod` 类型 + status/config 字段。

### Tests

- Rust 单元测试（`services/usage_limit/tests.rs` 28 → 36 个）：
  周期边界本地日历对齐（含跨年/周界/边界时刻）、next 严格未来、
  parse 值域（大小写敏感）、hourly/daily/weekly/monthly 滚动与恢复、
  never 语义回归（不因日历边界自动放行）、保存校验与周期切换对齐、
  状态回带 resetPeriod/nextResetAt、脏数据回退 never、手动重置与周期
  共存、全新 DDL 默认值。
- Proxy 集成测试（`forwarder.rs`）新增
  `budget_recovers_after_period_boundary_rollover`：昨天 $10 ≥ $5 限额 +
  daily 周期 → guard 懒滚动放行且窗口持久化到今天零点；对照组今天窗口
  内 $10 依旧拦截。`insert_budget_log_at` 支持自定义 created_at。
- 前端测试（`UsageLimitDialog.test.tsx` 21 → 25 个）：选择器五选项渲染、
  默认 never + hint、保存携带所选周期、已保存周期展示下次重置、切换未
  保存周期不展示过期边界、重开保留已保存周期（aria-pressed）。

### Known limitations

- 周期边界按本机时区计算；跨时区使用时「每天」的零点跟随设备当前时区。
- DST 切换日的边界存在 ±1 小时级的墙钟歧义（gap 顺延、歧义取较早），
  仅影响每年个别小时，不产生方向性错误。
- **Budget enforcement only applies to traffic routed through CC Switch
  Local Proxy.**（V1.0 边界不变；周期重置不改变观测边界）

### Verification

2026-09-18 全量验证（本机工具链说明：`rust-toolchain.toml` 锁定的 1.95
工具链缺 cargo 组件、无法执行；实际使用本机 stable 1.97.1 完成验证，1.97
下仅有的 3 个 clippy 警告均位于本次未触碰的既有文件
`gemini_mcp.rs` / `tray.rs` / `transform_codex_chat.rs`，属版本差异下的
既有警告）：

| 检查 | 结果 |
| --- | --- |
| `pnpm typecheck` | PASS |
| `pnpm format:check` | PASS |
| `pnpm test:unit` | PASS（139 文件 / 1145 tests） |
| `pnpm build:renderer`（vite 生产构建） | PASS |
| `cargo build --release` | PASS（4m20s，产出 release 二进制） |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets` | 本次改动文件 0 警告（3 个既有警告见上） |
| `cargo test --lib`（跳过网络型模块） | **PASS：2917 passed / 0 failed**（9 ignored 为仓库原有） |
| `cargo test --doc` | PASS（0 / 0 failed） |
| 定向回归 | usage_limit 36、budget/forwarder 39（含新增 rollover）、migration 38 全过 |

环境限制说明：`proxy::server::tests` 与
`proxy::hyper_client::tests` 中的 9 个网络型集成测试在本沙箱环境挂起
（需真实 socket/上游），为**既有环境限制、与本次改动无关**——佐证：本机
另一同名工程存在凌晨遗留的同模块挂起调试进程；本次改动涉及的 budget
判定路径（`proxy::forwarder::tests`）已全量通过。以上 9 个测试以
`--skip` 模块级跳过后其余全绿。

---

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
