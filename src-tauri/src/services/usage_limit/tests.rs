//! 使用限额服务单元测试
//!
//! 覆盖需求 §38 的全部用例：限额关闭 / 金额低于·达到·超过 / Token 低于·达到·超过 /
//! Reset / 换 Key 不继承 / 汇率换算 / 未知定价 / 并发记账。测试统一使用
//! `sk-test-...` 假 key，绝不出现真实凭据。

use super::*;
use crate::database::Database;
use crate::provider::Provider;
use rust_decimal::Decimal;
use std::str::FromStr;

/// 构造一个带静态 API Key 的 Claude provider（settings_config 与代理
/// extract_auth 的读取路径一致：env.ANTHROPIC_AUTH_TOKEN）
fn claude_provider(id: &str, api_key: &str) -> Provider {
    let mut provider = Provider::with_id(
        id.to_string(),
        format!("Test Provider {id}"),
        serde_json::json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.test.local",
                "ANTHROPIC_AUTH_TOKEN": api_key,
            }
        }),
        None,
    );
    provider.category = Some("custom".to_string());
    provider
}

/// 写入一个已计价的请求日志行（模拟代理记账）
#[allow(clippy::too_many_arguments)]
fn insert_log(
    db: &Database,
    request_id: &str,
    provider_id: &str,
    fingerprint: &str,
    created_at: i64,
    input: i64,
    output: i64,
    total_cost: &str,
) {
    let conn = db.conn.lock().expect("test db lock");
    conn.execute(
        "INSERT INTO proxy_request_logs (
            request_id, provider_id, app_type, model, request_model, pricing_model,
            input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
            input_token_semantics, input_cost_usd, output_cost_usd,
            cache_read_cost_usd, cache_creation_cost_usd, total_cost_usd,
            latency_ms, status_code, created_at, data_source, credential_fingerprint
        ) VALUES (?1, ?2, 'claude', 'test-model', 'test-model', 'test-model',
            ?3, ?4, 0, 0, 2, '0', '0', '0', '0', ?5, 100, 200, ?6, 'proxy', ?7)",
        rusqlite::params![
            request_id,
            provider_id,
            input,
            output,
            total_cost,
            created_at,
            fingerprint
        ],
    )
    .unwrap();
}

fn limit_row(
    provider_id: &str,
    fingerprint: &str,
    enabled: bool,
    limit_type: &str,
    currency: Option<&str>,
    amount: &str,
) -> ApiKeyLimitRow {
    ApiKeyLimitRow {
        provider_id: provider_id.to_string(),
        app_type: "claude".to_string(),
        credential_fingerprint: fingerprint.to_string(),
        enabled,
        limit_type: limit_type.to_string(),
        currency: currency.map(str::to_string),
        limit_amount: amount.to_string(),
        usage_start_at: 1_000_000,
        reset_period: "never".to_string(),
        created_at: 1_000_000,
        updated_at: 1_000_000,
    }
}

const FP_A: &str = "aaaa"; // 旧 key 指纹（guard 直测用，无需 provider 行）
const FP_B: &str = "bbbb"; // 新 key 指纹

/// 建 provider 行并写入限额配置（api_key_limits 有 FK → providers，必须先建
/// provider）；返回该 key 的真实指纹。每个测试用独立的 memory DB，provider_id 可复用。
fn seed_limit(
    db: &Database,
    provider_id: &str,
    enabled: bool,
    limit_type: &str,
    currency: Option<&str>,
    amount: &str,
) -> String {
    let key = format!("sk-test-{provider_id}-secret-9876");
    db.save_provider("claude", &claude_provider(provider_id, &key))
        .unwrap();
    let fp = credential_fingerprint(&key);
    db.upsert_api_key_limit(&limit_row(
        provider_id,
        &fp,
        enabled,
        limit_type,
        currency,
        amount,
    ))
    .unwrap();
    fp
}

// ---------- 转发前判定（§38 Rust Unit Tests） ----------

#[test]
fn limit_disabled_allows_request() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", false, "token", None, "1000");
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 5_000, 5_000, "0.5");

    let result = db.check_budget_before_forward("p1", "P1", "claude", &fp);
    assert!(result.is_ok(), "限额关闭时必须放行");
}

#[test]
fn no_limit_config_allows_request() {
    let db = Database::memory().unwrap();
    assert!(db
        .check_budget_before_forward("p-none", "P", "claude", FP_A)
        .is_ok());
}

#[test]
fn money_below_limit_allows_request() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "10");
    // $9.99 < $10，快速通道应放行
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 100, 100, "9.99");

    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_ok());
}

#[test]
fn money_reached_blocks_request() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "10");
    // 精确 $10.00 → 达到限额，下一请求必须被拒
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 100, 100, "10.00");

    let err = db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .unwrap_err();
    assert!(
        err.message.contains("usage limit reached"),
        "{}",
        err.message
    );
    assert_eq!(err.detail["limitType"], "money");
    assert_eq!(err.detail["limit"], "$10");
}

#[test]
fn money_exceeded_blocks_request() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "1");
    // $1.02 > $1 → 拒绝（对齐集成测试 Case 2）
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 100, 100, "1.02");

    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_err());
}

#[test]
fn token_below_limit_allows_request() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "token", None, "1000000");
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 600_000, 300_000, "0.5");

    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_ok());
}

#[test]
fn token_reached_blocks_request() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "token", None, "1000");
    // 正好 1000 tokens → 达到限额即拒绝（>= 语义）
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 600, 400, "0.5");

    let err = db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .unwrap_err();
    assert!(
        err.message.contains("token limit reached"),
        "{}",
        err.message
    );
    assert_eq!(err.detail["limitType"], "token");
}

#[test]
fn token_exceeded_blocks_request() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "token", None, "1000");
    // 1100 > 1000 → 拒绝（对齐集成测试 Case 1）
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 600, 500, "0.5");

    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_err());
}

#[test]
fn reset_moves_window_so_old_usage_not_counted() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "5");
    insert_log(&db, "r-old", "p1", &fp, 1_000_100, 100, 100, "5.10");
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_err());

    // 重置窗口：旧记录（created_at < 新起点）不再计入
    db.reset_api_key_limit_window("p1", "claude", 2_000_000)
        .unwrap();
    insert_log(&db, "r-new", "p1", &fp, 2_000_100, 100, 100, "0.10");

    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_ok());

    let status = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(status.used_money_usd, "0.10");
    assert_eq!(status.usage_start_at, Some(2_000_000));
}

#[test]
fn credential_change_does_not_inherit_old_usage() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "token", None, "1000");
    // 旧 key 已用 900
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 500, 400, "0.5");

    // 用户换成新 key（指纹 B）：guard 检测到轮换 → 重绑 + 窗口重置
    insert_log(&db, "r2", "p1", FP_B, 3_000_100, 300, 300, "0.2");
    db.check_budget_before_forward("p1", "P1", "claude", FP_B)
        .unwrap();

    let row = db.get_api_key_limit("p1", "claude").unwrap().unwrap();
    assert_eq!(row.credential_fingerprint, FP_B, "必须已重绑到新指纹");
    assert!(
        row.usage_start_at > 1_000_000,
        "统计窗口必须已重置（旧 key 用量不继承）"
    );

    // 新 key 用量 600 < 1000 → 放行
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", FP_B)
        .is_ok());
    // 重绑后窗口起点是「当下」：旧 key 的 900 与重绑前的 600 都不并入新窗口
    let fast = db
        .sum_credential_usage_fast("p1", "claude", FP_B, row.usage_start_at)
        .unwrap();
    assert_eq!(
        fast.normalized_total_tokens(),
        0,
        "旧 key / 窗口前用量不得计入新窗口"
    );

    // 新窗口内的请求正常计入
    insert_log(
        &db,
        "r3",
        "p1",
        FP_B,
        row.usage_start_at + 10,
        300,
        300,
        "0.2",
    );
    let fast = db
        .sum_credential_usage_fast("p1", "claude", FP_B, row.usage_start_at)
        .unwrap();
    assert_eq!(fast.normalized_total_tokens(), 600);
}

#[test]
fn currency_conversion_usd_cny_is_correct() {
    let db = Database::memory().unwrap();
    // 汇率默认 7.2：$10 限额 ≈ ¥72
    let rate = db.get_exchange_rate(LimitCurrency::Cny).unwrap();
    assert_eq!(rate, Decimal::from_str("7.2").unwrap());

    // CNY 限额 ¥7.2 = $1 上限；已用 $1.00 → 达到
    let fp = seed_limit(&db, "p1", true, "money", Some("CNY"), "7.2");
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 100, 100, "1.00");
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_err());

    // 自定义汇率可改
    db.set_exchange_rate(LimitCurrency::Cny, Decimal::from_str("7.5").unwrap())
        .unwrap();
    assert_eq!(
        db.get_exchange_rate(LimitCurrency::Cny).unwrap(),
        Decimal::from_str("7.5").unwrap()
    );

    // 非法汇率被拒绝
    assert!(
        db.set_exchange_rate(LimitCurrency::Cny, Decimal::ZERO)
            .is_err(),
        "汇率必须 > 0"
    );
}

#[test]
fn multi_currency_rates_are_independent_with_defaults() {
    let db = Database::memory().unwrap();

    // 各币种默认汇率合理，且互不干扰
    for (currency, default) in [
        (LimitCurrency::Usd, "1"),
        (LimitCurrency::Cny, "7.2"),
        (LimitCurrency::Eur, "0.92"),
        (LimitCurrency::Jpy, "150"),
        (LimitCurrency::Gbp, "0.79"),
    ] {
        assert_eq!(
            db.get_exchange_rate(currency).unwrap(),
            Decimal::from_str(default).unwrap(),
            "{currency:?} 默认汇率"
        );
    }

    // 修改 EUR 不影响 JPY / CNY
    db.set_exchange_rate(LimitCurrency::Eur, Decimal::from_str("0.85").unwrap())
        .unwrap();
    assert_eq!(
        db.get_exchange_rate(LimitCurrency::Eur).unwrap(),
        Decimal::from_str("0.85").unwrap()
    );
    assert_eq!(
        db.get_exchange_rate(LimitCurrency::Jpy).unwrap(),
        Decimal::from_str("150").unwrap()
    );
    assert_eq!(
        db.get_exchange_rate(LimitCurrency::Cny).unwrap(),
        Decimal::from_str("7.2").unwrap()
    );

    // USD 是内部基准币种：恒为 1，不可设置
    assert_eq!(
        db.get_exchange_rate(LimitCurrency::Usd).unwrap(),
        Decimal::ONE
    );
    assert!(db
        .set_exchange_rate(LimitCurrency::Usd, Decimal::from(2))
        .is_err());

    // 每个币种的汇率都可以独立设置且被拒绝非法值
    for currency in [
        LimitCurrency::Cny,
        LimitCurrency::Eur,
        LimitCurrency::Jpy,
        LimitCurrency::Gbp,
    ] {
        assert!(
            db.set_exchange_rate(currency, Decimal::from_str("-1").unwrap())
                .is_err(),
            "{currency:?} 负汇率必须拒绝"
        );
    }
}

#[test]
fn eur_and_jpy_limits_convert_correctly() {
    // EUR：€0.92 / 0.92 = $1 上限；已用 $1.00 → 达到
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("EUR"), "0.92");
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 100, 100, "1.00");
    let err = db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .unwrap_err();
    assert!(err.message.contains("€"), "{}", err.message);
    assert_eq!(err.detail["currency"], "EUR");

    // JPY：JP¥15000 / 150 = $100 上限；已用 $99（快速通道范围内）→ 放行，
    // 状态里的已用金额按 JPY 展示（99 × 150 = 14850）
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p2", true, "money", Some("JPY"), "15000");
    insert_log(&db, "r1", "p2", &fp, 1_000_100, 100, 100, "99");
    assert!(db
        .check_budget_before_forward("p2", "P2", "claude", &fp)
        .is_ok());
    let status = db.get_budget_status("p2", "claude").unwrap();
    assert_eq!(status.used_money_in_currency.as_deref(), Some("14850"));
    assert_eq!(status.currency.as_deref(), Some("JPY"));

    // GBP：£0.79 / 0.79 = $1 上限；已用 $0.99 → 放行；再加 $0.02 → 拒绝
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p3", true, "money", Some("GBP"), "0.79");
    insert_log(&db, "r1", "p3", &fp, 1_000_100, 100, 100, "0.99");
    assert!(db
        .check_budget_before_forward("p3", "P3", "claude", &fp)
        .is_ok());
    insert_log(&db, "r2", "p3", &fp, 1_000_200, 100, 100, "0.02");
    assert!(db
        .check_budget_before_forward("p3", "P3", "claude", &fp)
        .is_err());
}

#[test]
fn save_accepts_all_supported_currencies_only() {
    let db = Database::memory().unwrap();
    db.save_provider("claude", &claude_provider("p1", "sk-test-multi-currency"))
        .unwrap();

    for currency in ["USD", "CNY", "EUR", "JPY", "GBP"] {
        let config = UsageLimitConfig {
            enabled: true,
            limit_type: "money".to_string(),
            currency: Some(currency.to_string()),
            limit_amount: Some("10".to_string()),
            reset_period: None,
        };
        let status = db.save_budget_config("p1", "claude", &config).unwrap();
        assert_eq!(status.currency.as_deref(), Some(currency), "{currency}");
    }

    // 币种 CHECK 已移除，但应用层仍拒绝未知币种
    let config = UsageLimitConfig {
        enabled: true,
        limit_type: "money".to_string(),
        currency: Some("RUB".to_string()),
        limit_amount: Some("10".to_string()),
        reset_period: None,
    };
    assert!(db.save_budget_config("p1", "claude", &config).is_err());
}

#[test]
fn custom_rate_end_to_end_changes_guard_threshold() {
    // 审查 P1-3：非默认汇率必须真正影响 guard 阈值——
    // 把 USD→EUR 汇率改为 0.46 后，€0.92 限额的 USD 阈值变为 $2：
    // 已用 $1.9 → 放行；累计 $2.0 → 达到拦截
    let db = Database::memory().unwrap();
    db.set_exchange_rate(LimitCurrency::Eur, Decimal::from_str("0.46").unwrap())
        .unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("EUR"), "0.92");
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 100, 100, "1.90");
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_ok());

    insert_log(&db, "r2", "p1", &fp, 1_000_200, 100, 100, "0.10");
    let err = db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .unwrap_err();
    assert_eq!(err.detail["currency"], "EUR");
    assert!(err.message.contains("€"), "{}", err.message);

    // 键名脱节防护：写 EUR 键不得影响 JPY 的默认判定路径
    assert_eq!(
        db.get_exchange_rate(LimitCurrency::Jpy).unwrap(),
        Decimal::from_str("150").unwrap()
    );
}

#[test]
fn corrupt_stored_rate_falls_back_to_default() {
    // 审查 P2-7：存量脏汇率（settings 被手工改坏）按该币种默认值回退，不拒绝服务
    let db = Database::memory().unwrap();
    db.set_exchange_rate(LimitCurrency::Eur, Decimal::from_str("0.92").unwrap())
        .unwrap();
    db.set_setting(
        LimitCurrency::Eur.rate_setting_key().unwrap(),
        "not-a-number",
    )
    .unwrap();
    assert_eq!(
        db.get_exchange_rate(LimitCurrency::Eur).unwrap(),
        Decimal::from_str("0.92").unwrap()
    );

    // 空字符串与非正值同样回退
    for garbage in ["", "0", "-3"] {
        db.set_setting(LimitCurrency::Jpy.rate_setting_key().unwrap(), garbage)
            .unwrap();
        assert_eq!(
            db.get_exchange_rate(LimitCurrency::Jpy).unwrap(),
            Decimal::from_str("150").unwrap(),
            "垃圾值 {garbage} 必须回退默认"
        );
    }
}

#[test]
fn cny_limit_uses_converted_threshold_in_fast_path() {
    let db = Database::memory().unwrap();
    // ¥14.4 / 7.2 = $2 上限；已用 $1.9（快速通道范围内）→ 放行
    let fp = seed_limit(&db, "p1", true, "money", Some("CNY"), "14.4");
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 100, 100, "1.90");
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_ok());

    // 再加 $0.2 → $2.1 > $2 → 拒绝（精确复核通道）
    insert_log(&db, "r2", "p1", &fp, 1_000_200, 100, 100, "0.20");
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_err());
}

#[test]
fn money_float_precision_is_decimal_safe() {
    let db = Database::memory().unwrap();
    // 经典浮点陷阱：0.1+0.2 != 0.3。十进制求和必须精确判满。
    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "0.3");
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 1, 1, "0.1");
    insert_log(&db, "r2", "p1", &fp, 1_000_200, 1, 1, "0.2");

    let err = db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .unwrap_err();
    assert!(err.message.contains("limit reached"));
}

// ---------- 未知定价 ----------

#[test]
fn unknown_pricing_is_surfaced_not_silently_zero() {
    // 需求 §10：缺价模型的用量不能按 $0 假装正常——状态必须暴露 unpriced 信息
    let db = Database::memory().unwrap();
    let key = "sk-test-pricing-key";
    let fp = &credential_fingerprint(key);
    db.save_provider("claude", &claude_provider("p1", key))
        .unwrap();
    db.upsert_api_key_limit(&limit_row("p1", fp, true, "money", Some("USD"), "10"))
        .unwrap();

    // 已计价行（先给 test-model 种子定价）+ 未计价行（unknown-x）
    {
        let conn = db.conn.lock().expect("test db lock");
        conn.execute(
            "INSERT INTO model_pricing (model_id, display_name, input_cost_per_million, output_cost_per_million)
             VALUES ('test-model', 'Test Model', '3.0', '15.0')",
            [],
        )
        .unwrap();
    }
    insert_log(&db, "r1", "p1", fp, 1_000_100, 10_000, 10_000, "0.5");
    {
        let conn = db.conn.lock().expect("test db lock");
        conn.execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model, request_model, pricing_model,
                input_tokens, output_tokens, input_token_semantics,
                total_cost_usd, latency_ms, status_code, created_at, data_source, credential_fingerprint
             ) VALUES ('r2', 'p1', 'claude', 'unknown-x', 'unknown-x', 'unknown-x',
                5000, 5000, 2, '0', 10, 200, ?2, 'proxy', ?1)",
            rusqlite::params![fp, 1_000_200],
        )
        .unwrap();
    }

    let status = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(status.unpriced_request_count, 1);
    assert_eq!(status.unpriced_models, vec!["unknown-x".to_string()]);
    // 已计价部分继续有效；未计价行不得悄悄按 0 假装全貌
    assert_eq!(status.used_money_usd, "0.5");
}

// ---------- 并发与数据源边界 ----------

#[test]
fn concurrent_accounting_does_not_lose_writes() {
    // 并发语义验证：N 个线程同时写同一 credential 的用量行，
    // 全部写完后聚合值必须等于写入总量（DB Mutex 下无丢失写）
    let db = std::sync::Arc::new(Database::memory().unwrap());
    let fp = seed_limit(&db, "p1", true, "token", None, "1000000000");

    let handles: Vec<_> = (0..8)
        .map(|worker| {
            let db = db.clone();
            let fp = fp.clone();
            std::thread::spawn(move || {
                for i in 0..25 {
                    insert_log(
                        &db,
                        &format!("r-{worker}-{i}"),
                        "p1",
                        &fp,
                        1_000_100 + worker * 1000 + i,
                        1_000,
                        1_000,
                        "0.01",
                    );
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }

    let fast = db
        .sum_credential_usage_fast("p1", "claude", &fp, 1_000_000)
        .unwrap();
    assert_eq!(fast.normalized_total_tokens(), 8 * 25 * 2_000);
    assert!((fast.total_cost_usd_real - 8.0 * 25.0 * 0.01).abs() < 1e-9);
}

#[test]
fn session_log_rows_are_excluded_from_budget() {
    // 会话日志同步行为 data_source != 'proxy'：不是本地代理的观测，
    // 不能计入本机预算（usage is local observation）
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "1");
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 100, 100, "0.99");
    {
        let conn = db.conn.lock().expect("test db lock");
        conn.execute(
            "UPDATE proxy_request_logs SET data_source = 'session_log' WHERE request_id = 'r1'",
            [],
        )
        .unwrap();
    }
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_ok());
}

// ---------- 输入验证 ----------

#[test]
fn validate_money_amounts() {
    assert_eq!(
        validate_limit_amount(LimitType::Money, "100").unwrap(),
        "100"
    );
    assert_eq!(
        validate_limit_amount(LimitType::Money, " 100.50 ").unwrap(),
        "100.5"
    );
    assert_eq!(
        validate_limit_amount(LimitType::Money, "0.01").unwrap(),
        "0.01"
    );

    for bad in ["", "0", "-5", "abc", "1e3", "NaN", "1,000", "Infinity"] {
        assert!(
            validate_limit_amount(LimitType::Money, bad).is_err(),
            "非法金额 {bad} 必须被拒绝"
        );
    }
}

#[test]
fn validate_token_amounts() {
    assert_eq!(
        validate_limit_amount(LimitType::Token, "5000000").unwrap(),
        "5000000"
    );

    for bad in [
        "",
        "0",
        "-1",
        "3.5",
        "1e6",
        "1_000",
        "5,000,000",
        "abc",
        "18446744073709551616", // u64 溢出
    ] {
        assert!(
            validate_limit_amount(LimitType::Token, bad).is_err(),
            "非法 token 数 {bad} 必须被拒绝"
        );
    }
}

// ---------- 指纹与遮蔽 ----------

#[test]
fn fingerprint_is_irreversible_and_stable() {
    let fp1 = credential_fingerprint("sk-test-abcdef123456");
    let fp2 = credential_fingerprint("sk-test-abcdef123456");
    let fp3 = credential_fingerprint("sk-test-different-key");

    assert_eq!(fp1, fp2, "同一 key 指纹必须一致");
    assert_ne!(fp1, fp3, "不同 key 指纹必须不同");
    assert_eq!(fp1.len(), 64, "SHA-256 hex 长度");
    // 指纹不能包含原始 key
    assert!(!fp1.contains("sk-test"));
}

#[test]
fn mask_credential_never_leaks_full_key() {
    assert_eq!(mask_credential("sk-ant-abc123def456"), "sk-****f456");
    assert_eq!(mask_credential("short"), "****");
    assert_eq!(mask_credential(""), "****");
    let masked = mask_credential("sk-1234567890");
    assert!(!masked.contains("1234567890"), "遮蔽后不得包含完整 key");
}

// ---------- 保存语义 ----------

#[test]
fn save_validates_and_persists_config() {
    let db = Database::memory().unwrap();
    db.save_provider("claude", &claude_provider("p1", "sk-test-save-key"))
        .unwrap();

    // Token 模式带币种 → 拒绝
    let config = UsageLimitConfig {
        enabled: true,
        limit_type: "token".to_string(),
        currency: Some("USD".to_string()),
        limit_amount: Some("1000".to_string()),
        reset_period: None,
    };
    assert!(db.save_budget_config("p1", "claude", &config).is_err());

    // 金额模式缺币种 → 拒绝
    let config = UsageLimitConfig {
        enabled: true,
        limit_type: "money".to_string(),
        currency: None,
        limit_amount: Some("1000".to_string()),
        reset_period: None,
    };
    assert!(db.save_budget_config("p1", "claude", &config).is_err());

    // 非法金额 → 拒绝，异常数据不进 SQLite
    let config = UsageLimitConfig {
        enabled: true,
        limit_type: "token".to_string(),
        currency: None,
        limit_amount: Some("-5".to_string()),
        reset_period: None,
    };
    assert!(db.save_budget_config("p1", "claude", &config).is_err());
    assert!(db.get_api_key_limit("p1", "claude").unwrap().is_none());

    // 合法保存（CNY）
    let config = UsageLimitConfig {
        enabled: true,
        limit_type: "money".to_string(),
        currency: Some("CNY".to_string()),
        limit_amount: Some("50".to_string()),
        reset_period: None,
    };
    let status = db.save_budget_config("p1", "claude", &config).unwrap();
    assert!(status.enabled);
    assert_eq!(status.limit_type.as_deref(), Some("money"));
    assert_eq!(status.currency.as_deref(), Some("CNY"));
    assert_eq!(status.limit_amount.as_deref(), Some("50"));
    assert_eq!(status.enforcement, EnforcementSupport::ProxyDisabled);

    // 关闭（enabled=false）保留配置
    let config = UsageLimitConfig {
        enabled: false,
        limit_type: "money".to_string(),
        currency: Some("CNY".to_string()),
        limit_amount: Some("50".to_string()),
        reset_period: None,
    };
    let status = db.save_budget_config("p1", "claude", &config).unwrap();
    assert!(!status.enabled);
    assert_eq!(status.limit_amount.as_deref(), Some("50"), "关闭不能删配置");
    assert_eq!(status.state, "off");

    // 换 key → 保存时重绑新指纹 + 重置窗口
    let old_row = db.get_api_key_limit("p1", "claude").unwrap().unwrap();
    db.save_provider("claude", &claude_provider("p1", "sk-test-rotated-key-9999"))
        .unwrap();
    let config = UsageLimitConfig {
        enabled: true,
        limit_type: "money".to_string(),
        currency: Some("CNY".to_string()),
        limit_amount: Some("50".to_string()),
        reset_period: None,
    };
    db.save_budget_config("p1", "claude", &config).unwrap();
    let new_row = db.get_api_key_limit("p1", "claude").unwrap().unwrap();
    assert_ne!(
        old_row.credential_fingerprint, new_row.credential_fingerprint,
        "保存时必须绑定当前 key 的指纹"
    );
    assert!(
        new_row.usage_start_at >= old_row.usage_start_at,
        "换 key 后统计窗口重置"
    );

    // OAuth provider（无静态 key）→ 拒绝
    let mut oauth_provider = claude_provider("p2", "sk-test-oauth");
    oauth_provider.settings_config = serde_json::json!({});
    oauth_provider.meta = Some(crate::provider::ProviderMeta {
        provider_type: Some("codex_oauth".to_string()),
        ..Default::default()
    });
    db.save_provider("claude", &oauth_provider).unwrap();
    let config = UsageLimitConfig {
        enabled: true,
        limit_type: "token".to_string(),
        currency: None,
        limit_amount: Some("1000".to_string()),
        reset_period: None,
    };
    assert!(db.save_budget_config("p2", "claude", &config).is_err());
}

// ---------- 状态机 ----------

#[test]
fn status_states_off_active_warning_exhausted() {
    let db = Database::memory().unwrap();
    db.save_provider("claude", &claude_provider("p1", "sk-test-state-base-0000"))
        .unwrap();

    // 无配置 → off
    let status = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(status.state, "off");

    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "10");

    // 24% → active
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 10, 10, "2.40");
    let status = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(status.state, "active");
    assert_eq!(
        status
            .percent_used
            .as_deref()
            .map(|p| Decimal::from_str(p).unwrap()),
        Some(Decimal::from(24))
    );

    // 85% → warning
    insert_log(&db, "r2", "p1", &fp, 1_000_200, 10, 10, "6.10");
    let status = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(status.state, "warning");

    // 104% → exhausted（数值不 clamp，进度条由前端视觉封顶）
    insert_log(&db, "r3", "p1", &fp, 1_000_300, 10, 10, "1.90");
    let status = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(status.state, "exhausted");
    assert_eq!(
        status
            .percent_used
            .as_deref()
            .map(|p| Decimal::from_str(p).unwrap()),
        Some(Decimal::from(104))
    );

    // 关闭 → off，但用量字段照常返回（同一指纹，仅开关翻转）
    seed_limit(&db, "p1", false, "money", Some("USD"), "10");
    let status = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(status.state, "off");
    assert_eq!(status.used_money_usd, "10.40");
}

#[test]
fn enforcement_reflects_proxy_toggle() {
    let db = Database::memory().unwrap();
    db.save_provider("claude", &claude_provider("p1", "sk-test-enforcement-key"))
        .unwrap();

    let status = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(status.enforcement, EnforcementSupport::ProxyDisabled);

    db.set_proxy_flags_sync("claude", true, false).unwrap();
    let status = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(status.enforcement, EnforcementSupport::Active);

    // masked_credential 绝不包含完整 key
    let masked = status.masked_credential.unwrap();
    assert!(!masked.contains("sk-test-enforcement-key"));
    assert!(masked.ends_with("****") || masked.contains("****"));
}

#[test]
fn reset_command_moves_window() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "token", None, "100000");
    insert_log(&db, "r1", "p1", &fp, 1_000_100, 50_000, 50_000, "0.5");

    let before = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(before.used_tokens, 100_000);

    db.reset_budget_usage("p1", "claude").unwrap();

    let after = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(after.used_tokens, 0, "重置后窗口内用量为 0");
    assert!(after.usage_start_at.unwrap() >= before.usage_start_at.unwrap());
}

// ---------- 重置周期（V1.0.1） ----------

use crate::services::usage_limit::ResetPeriod;

/// 覆盖周期字段的 limit 行（金额模式 USD，窗口起点与限额可指定）
fn limit_row_with_period(
    provider_id: &str,
    fingerprint: &str,
    period: &str,
    usage_start_at: i64,
    amount: &str,
) -> ApiKeyLimitRow {
    ApiKeyLimitRow {
        reset_period: period.to_string(),
        usage_start_at,
        ..limit_row(provider_id, fingerprint, true, "money", Some("USD"), amount)
    }
}

fn local_ts(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> i64 {
    use chrono::TimeZone;
    chrono::Local
        .with_ymd_and_hms(year, month, day, hour, min, sec)
        .single()
        .expect("测试用的本地墙钟时刻应存在")
        .timestamp()
}

#[test]
fn period_boundaries_follow_local_calendar() {
    // 2026-09-18 15:27:08 是周五：整点 / 当天零点 / 本周一零点 / 本月 1 日零点
    let now = local_ts(2026, 9, 18, 15, 27, 8);

    assert_eq!(ResetPeriod::Never.current_period_start(now), None);
    assert_eq!(ResetPeriod::Never.next_period_start(now), None);

    assert_eq!(
        ResetPeriod::Hourly.current_period_start(now),
        Some(local_ts(2026, 9, 18, 15, 0, 0))
    );
    assert_eq!(
        ResetPeriod::Daily.current_period_start(now),
        Some(local_ts(2026, 9, 18, 0, 0, 0))
    );
    // ISO 周：周一为一周起点 → 2026-09-14（周一）
    assert_eq!(
        ResetPeriod::Weekly.current_period_start(now),
        Some(local_ts(2026, 9, 14, 0, 0, 0))
    );
    assert_eq!(
        ResetPeriod::Monthly.current_period_start(now),
        Some(local_ts(2026, 9, 1, 0, 0, 0))
    );
}

#[test]
fn next_period_start_is_strictly_future() {
    // 恰好踩在边界上：当前周期刚开启，下一次重置必须是下一个边界
    let midnight = local_ts(2026, 9, 18, 0, 0, 0);
    assert_eq!(
        ResetPeriod::Daily.next_period_start(midnight),
        Some(local_ts(2026, 9, 19, 0, 0, 0))
    );
    assert_eq!(
        ResetPeriod::Hourly.next_period_start(midnight),
        Some(local_ts(2026, 9, 18, 1, 0, 0))
    );
    // 月末跨年：12 月的下一个周期边界是次年 1 月 1 日
    let dec = local_ts(2026, 12, 31, 23, 0, 0);
    assert_eq!(
        ResetPeriod::Monthly.next_period_start(dec),
        Some(local_ts(2027, 1, 1, 0, 0, 0))
    );
    // 周边界：周日 23 点的下一个重置是下周一零点
    let sunday = local_ts(2026, 9, 20, 23, 0, 0);
    assert_eq!(
        ResetPeriod::Weekly.next_period_start(sunday),
        Some(local_ts(2026, 9, 21, 0, 0, 0))
    );
}

#[test]
fn parse_reset_period_accepts_known_values_only() {
    for (raw, expected) in [
        ("never", ResetPeriod::Never),
        ("hourly", ResetPeriod::Hourly),
        ("daily", ResetPeriod::Daily),
        ("weekly", ResetPeriod::Weekly),
        ("monthly", ResetPeriod::Monthly),
    ] {
        assert_eq!(ResetPeriod::parse(raw), Some(expected));
    }
    assert_eq!(ResetPeriod::parse("yearly"), None);
    assert_eq!(ResetPeriod::parse(""), None);
    assert_eq!(ResetPeriod::parse("DAILY"), None, "值域区分大小写");
}

#[test]
fn hourly_period_rolls_window_and_recovers_budget() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "5");
    // 窗口压回固定历史时间（远早于当前小时边界）：$10 早已超额
    db.upsert_api_key_limit(&limit_row_with_period("p1", &fp, "hourly", 1_000_000, "5"))
        .unwrap();
    insert_log(&db, "r-old", "p1", &fp, 1_000_100, 10, 10, "10.00");

    // guard 先滚动到当前整点再判定：旧周期 $10 出窗 → 放行
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_ok());
    let row = db.get_api_key_limit("p1", "claude").unwrap().unwrap();
    let now = chrono::Utc::now().timestamp();
    let hour_start = ResetPeriod::Hourly.current_period_start(now).unwrap();
    assert_eq!(row.usage_start_at, hour_start, "窗口必须已滚动到当前整点");

    // 当前整点内的新用量正常计入
    insert_log(&db, "r-new", "p1", &fp, hour_start + 10, 10, 10, "6.00");
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_err());
}

#[test]
fn daily_period_excludes_previous_day_usage() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "5");
    let now = chrono::Utc::now().timestamp();
    let today_start = ResetPeriod::Daily.current_period_start(now).unwrap();

    // 窗口压回昨天，昨天用 $4.90、今天零点后用 $0.20
    db.upsert_api_key_limit(&limit_row_with_period(
        "p1",
        &fp,
        "daily",
        today_start - 3_600,
        "5",
    ))
    .unwrap();
    insert_log(&db, "r-yday", "p1", &fp, today_start - 60, 10, 10, "4.90");
    insert_log(&db, "r-today", "p1", &fp, today_start + 60, 10, 10, "0.20");

    // 滚动后只计今天的 $0.20 < $5 → 放行，且窗口持久化到今天零点
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_ok());
    let row = db.get_api_key_limit("p1", "claude").unwrap().unwrap();
    assert_eq!(row.usage_start_at, today_start);

    let status = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(status.used_money_usd, "0.20");
    assert_eq!(status.usage_start_at, Some(today_start));
}

#[test]
fn weekly_and_monthly_periods_align_to_boundaries() {
    let now = chrono::Utc::now().timestamp();

    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "5");
    db.upsert_api_key_limit(&limit_row_with_period("p1", &fp, "weekly", 1_000_000, "5"))
        .unwrap();
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_ok());
    let row = db.get_api_key_limit("p1", "claude").unwrap().unwrap();
    assert_eq!(
        row.usage_start_at,
        ResetPeriod::Weekly.current_period_start(now).unwrap()
    );

    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p2", true, "money", Some("USD"), "5");
    db.upsert_api_key_limit(&limit_row_with_period("p2", &fp, "monthly", 1_000_000, "5"))
        .unwrap();
    assert!(db
        .check_budget_before_forward("p2", "P2", "claude", &fp)
        .is_ok());
    let row = db.get_api_key_limit("p2", "claude").unwrap().unwrap();
    assert_eq!(
        row.usage_start_at,
        ResetPeriod::Monthly.current_period_start(now).unwrap()
    );
}

#[test]
fn never_period_keeps_window_until_manual_reset() {
    // 回归：never（V1.0 默认）语义不变——窗口压在过去时，历史超额持续拦截，
    // 不因日历边界自动放行
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "5");
    db.upsert_api_key_limit(&limit_row_with_period("p1", &fp, "never", 1_000_000, "5"))
        .unwrap();
    insert_log(&db, "r-old", "p1", &fp, 1_000_100, 10, 10, "10.00");
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_err());

    // 手动重置立即恢复（窗口推进到当下，历史行全部出窗）
    db.reset_budget_usage("p1", "claude").unwrap();
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_ok());
}

#[test]
fn save_persists_and_validates_reset_period() {
    let db = Database::memory().unwrap();
    db.save_provider("claude", &claude_provider("p1", "sk-test-period-save-key"))
        .unwrap();

    // 缺省 → never
    let config = UsageLimitConfig {
        enabled: true,
        limit_type: "money".to_string(),
        currency: Some("USD".to_string()),
        limit_amount: Some("10".to_string()),
        reset_period: None,
    };
    db.save_budget_config("p1", "claude", &config).unwrap();
    let row = db.get_api_key_limit("p1", "claude").unwrap().unwrap();
    assert_eq!(row.reset_period, "never");

    // 合法周期入库，状态回带 next_reset_at
    let config = UsageLimitConfig {
        enabled: true,
        limit_type: "money".to_string(),
        currency: Some("USD".to_string()),
        limit_amount: Some("10".to_string()),
        reset_period: Some("weekly".to_string()),
    };
    let status = db.save_budget_config("p1", "claude", &config).unwrap();
    assert_eq!(status.reset_period, "weekly");
    assert!(status.next_reset_at.is_some());
    let row = db.get_api_key_limit("p1", "claude").unwrap().unwrap();
    assert_eq!(row.reset_period, "weekly");

    // 非法周期拒绝入库，原配置不被破坏
    let config = UsageLimitConfig {
        enabled: true,
        limit_type: "money".to_string(),
        currency: Some("USD".to_string()),
        limit_amount: Some("10".to_string()),
        reset_period: Some("yearly".to_string()),
    };
    assert!(db.save_budget_config("p1", "claude", &config).is_err());
    let row = db.get_api_key_limit("p1", "claude").unwrap().unwrap();
    assert_eq!(row.reset_period, "weekly", "非法保存不得覆盖原配置");

    // 改周期时窗口对齐到新周期边界：weekly → daily 即从今天零点起统计
    let config = UsageLimitConfig {
        enabled: true,
        limit_type: "money".to_string(),
        currency: Some("USD".to_string()),
        limit_amount: Some("10".to_string()),
        reset_period: Some("daily".to_string()),
    };
    db.save_budget_config("p1", "claude", &config).unwrap();
    let row = db.get_api_key_limit("p1", "claude").unwrap().unwrap();
    let now = chrono::Utc::now().timestamp();
    // 保守语义：原窗口已在本周期内（晚于今天零点）则不回退，保持已统计的
    // 本期用量；窗口早于边界时才对齐到边界（见各周期滚动用例）
    let today_start = ResetPeriod::Daily.current_period_start(now).unwrap();
    assert!(row.usage_start_at >= today_start);
    assert_eq!(row.reset_period, "daily");
}

#[test]
fn status_reports_period_and_next_reset() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "5");
    let now = chrono::Utc::now().timestamp();
    let today_start = ResetPeriod::Daily.current_period_start(now).unwrap();

    db.upsert_api_key_limit(&limit_row_with_period(
        "p1",
        &fp,
        "daily",
        today_start - 3_600,
        "5",
    ))
    .unwrap();
    insert_log(&db, "r-yday", "p1", &fp, today_start - 60, 10, 10, "4.90");
    insert_log(&db, "r-today", "p1", &fp, today_start + 60, 10, 10, "1.00");

    let status = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(status.reset_period, "daily");
    // 有效窗口 = 今天零点（懒滚动，读接口无副作用）
    assert_eq!(status.usage_start_at, Some(today_start));
    // 只计今天窗口内的 $1.00；昨天的 $4.90 出窗
    assert_eq!(status.used_money_usd, "1.00");
    assert_eq!(status.state, "active");
    // 下次重置 = 明天零点
    assert_eq!(
        status.next_reset_at,
        Some(ResetPeriod::Daily.next_period_start(now).unwrap())
    );
}

#[test]
fn unknown_stored_period_is_treated_as_never() {
    // 历史脏数据 / 手工改库：未知周期值按 never 兜底（告警），服务不拒绝
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "5");
    db.upsert_api_key_limit(&limit_row_with_period("p1", &fp, "bogus", 1_000_000, "5"))
        .unwrap();
    insert_log(&db, "r-old", "p1", &fp, 1_000_100, 10, 10, "10.00");

    // never 语义：旧超额持续拦截，不自动放行
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_err());

    let status = db.get_budget_status("p1", "claude").unwrap();
    assert_eq!(status.reset_period, "never");
    assert_eq!(status.next_reset_at, None);
}

#[test]
fn manual_reset_coexists_with_period_rollover() {
    let db = Database::memory().unwrap();
    let fp = seed_limit(&db, "p1", true, "money", Some("USD"), "5");
    let now = chrono::Utc::now().timestamp();
    let today_start = ResetPeriod::Daily.current_period_start(now).unwrap();

    // 窗口已在今天零点：今天窗口内 $10 → 超额拦截
    db.upsert_api_key_limit(&limit_row_with_period("p1", &fp, "daily", today_start, "5"))
        .unwrap();
    insert_log(&db, "r1", "p1", &fp, today_start + 60, 10, 10, "10.00");
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_err());

    // 手动重置：窗口推进到当下，窗口内的历史行全部出窗 → 恢复；
    // daily 周期继续有效（下个零点仍会滚动）
    db.reset_budget_usage("p1", "claude").unwrap();
    assert!(db
        .check_budget_before_forward("p1", "P1", "claude", &fp)
        .is_ok());
    let row = db.get_api_key_limit("p1", "claude").unwrap().unwrap();
    assert_eq!(row.reset_period, "daily");
    assert!(row.usage_start_at >= today_start + 60);
}

#[test]
fn fresh_schema_has_reset_period_default() {
    // 全新建表路径：新插入行不指定 reset_period 时默认 never
    let db = Database::memory().unwrap();
    db.save_provider("claude", &claude_provider("p-fresh", "sk-test-fresh-key"))
        .unwrap();
    let conn = db.conn.lock().expect("test db lock");
    conn.execute(
        "INSERT INTO api_key_limits (
            provider_id, app_type, credential_fingerprint, enabled, limit_type,
            currency, limit_amount, usage_start_at, created_at, updated_at
         ) VALUES ('p-fresh', 'claude', 'fp-fresh', 1, 'token', NULL, '1000', 0, 0, 0)",
        [],
    )
    .unwrap();
    let period: String = conn
        .query_row(
            "SELECT reset_period FROM api_key_limits WHERE provider_id = 'p-fresh'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(period, "never");
}
