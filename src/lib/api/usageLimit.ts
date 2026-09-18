import { invoke } from "@tauri-apps/api/core";
import type {
  UsageLimitConfig,
  UsageLimitCurrency,
  UsageLimitStatus,
} from "@/types/usageLimit";
import type { AppId } from "./types";

export const usageLimitApi = {
  getStatus: async (
    providerId: string,
    appId: AppId,
  ): Promise<UsageLimitStatus> => {
    return invoke("get_usage_limit_status", {
      providerId,
      appType: appId,
    });
  },

  save: async (
    providerId: string,
    appId: AppId,
    config: UsageLimitConfig,
  ): Promise<UsageLimitStatus> => {
    return invoke("save_usage_limit", {
      providerId,
      appType: appId,
      config,
    });
  },

  resetUsage: async (
    providerId: string,
    appId: AppId,
  ): Promise<UsageLimitStatus> => {
    return invoke("reset_usage_limit", { providerId, appType: appId });
  },

  /** 本地 USD → 指定币种汇率（未配置返回默认值；USD 返回 "1"） */
  getExchangeRate: async (currency: UsageLimitCurrency): Promise<string> => {
    return invoke("get_exchange_rate", { currency });
  },

  /** 保存本地 USD → 指定币种汇率（必须 > 0；USD 无需设置） */
  setExchangeRate: async (
    currency: UsageLimitCurrency,
    rate: string,
  ): Promise<void> => {
    return invoke("set_exchange_rate", { currency, rate });
  },
};

export type { UsageLimitCurrency };
