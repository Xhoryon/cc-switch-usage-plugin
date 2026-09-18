// API Key 使用限额（Budget Limit）类型定义
//
// 与后端 `services/usage_limit.rs` 的 serde 输出一一对应（camelCase）。

/** 限额模式：同一时间只有一种 */
export type UsageLimitType = "money" | "token";

/**
 * 金额模式币种（V1.0.2 起支持 5 种主流货币）。
 * 数据库内部成本统一为 USD，其他币种通过本地可配置汇率换算（人工调节，不联网）。
 */
export type UsageLimitCurrency = "USD" | "CNY" | "EUR" | "JPY" | "GBP";

/** 各币种展示符号（与后端 LimitCurrency::symbol 一致；JPY 加国别前缀消歧） */
export const USAGE_LIMIT_CURRENCY_SYMBOLS: Record<UsageLimitCurrency, string> =
  {
    USD: "$",
    CNY: "¥",
    EUR: "€",
    JPY: "JP¥",
    GBP: "£",
  };

/**
 * 统计窗口重置周期（V1.0.1）。
 * never = 仅手动重置；其余在本地时区周期边界（整点/零点/周一零点/每月 1 日零点）
 * 自动重置——后端为懒滚动实现，应用未运行期间跨过的边界在下一次启动时照常生效。
 */
export type UsageLimitResetPeriod =
  | "never"
  | "hourly"
  | "daily"
  | "weekly"
  | "monthly";

/** enforcement 能力：Proxy 是否真的在该 Provider 的请求路径上 */
export type UsageLimitEnforcement =
  | "active" // Proxy 已接管该应用，限额可强制执行
  | "proxy_disabled" // 该应用 Proxy 接管未开启，流量不经过 CC Switch
  | "unsupported_credential"; // OAuth 类 Provider，无静态 Key

/** 卡片图标 / Dialog 共用的状态聚合 */
export interface UsageLimitStatus {
  providerId: string;
  appType: string;
  enabled: boolean;
  limitType?: UsageLimitType | null;
  currency?: UsageLimitCurrency | null;
  limitAmount?: string | null;
  /** 预算统计窗口起点（unix 秒；周期重置下为对齐周期边界后的有效起点） */
  usageStartAt?: number | null;
  /** 重置周期（V1.0.1），缺省视为 never */
  resetPeriod?: UsageLimitResetPeriod | null;
  /** 下一次周期重置时间（unix 秒）；never 时为 null */
  nextResetAt?: number | null;
  /** 窗口内已用金额（USD 原值，十进制字符串） */
  usedMoneyUsd: string;
  /** 限额币种下的已用金额（CNY 限额时已按汇率换算） */
  usedMoneyInCurrency?: string | null;
  /** 窗口内已用 token（与 Usage Dashboard 一致的 normalized total） */
  usedTokens: number;
  /** 使用百分比字符串（十进制，不 clamp，可超过 100） */
  percentUsed?: string | null;
  /** "off" | "active" | "warning" | "exhausted" */
  state: UsageLimitState;
  /** 窗口内「有 token 用量但当前无定价」的请求数（金额模式可靠性提示） */
  unpricedRequestCount: number;
  /** 上述请求涉及的模型列表（引导配置 custom pricing） */
  unpricedModels: string[];
  enforcement: UsageLimitEnforcement;
  /** 当前 credential 遮蔽展示（如 sk-****ABCD），无静态 key 时为 null */
  maskedCredential?: string | null;
}

export type UsageLimitState = "off" | "active" | "warning" | "exhausted";

/** 保存/开关提交载荷 */
export interface UsageLimitConfig {
  enabled: boolean;
  limitType: UsageLimitType;
  /** token 模式下为 null */
  currency?: UsageLimitCurrency | null;
  /** 金额（十进制字符串）或 token 数（正整数字符串） */
  limitAmount?: string | null;
  /** 重置周期（V1.0.1），始终显式提交 */
  resetPeriod: UsageLimitResetPeriod;
}
