import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { usageLimitApi } from "@/lib/api/usageLimit";
import type { AppId } from "@/lib/api/types";
import type { UsageLimitConfig, UsageLimitCurrency } from "@/types/usageLimit";

/** Query keys（独立命名空间，避免与 Usage Dashboard 的轮询互相牵连） */
export const usageLimitKeys = {
  all: ["usage-limit"] as const,
  status: (providerId: string, appType: string) =>
    [...usageLimitKeys.all, "status", providerId, appType] as const,
  /** 本地 USD → 币种汇率（按币种分键） */
  exchangeRate: (currency: UsageLimitCurrency) =>
    [...usageLimitKeys.all, "exchange-rate", currency] as const,
};

/**
 * 读取某个 Provider 的限额状态。
 *
 * - `enabled` 由调用方控制（只对可走 Local Proxy 的应用启用）
 * - `refetchOnMount: "always"`：Dialog / 卡片重新挂载时拿到最新用量，
 *   请求完成后的刷新由 usage-log-recorded 事件的 invalidate 兜底
 */
export function useUsageLimitStatus(providerId: string, appType: AppId) {
  return useQuery({
    queryKey: usageLimitKeys.status(providerId, appType),
    queryFn: () => usageLimitApi.getStatus(providerId, appType),
    enabled: Boolean(providerId) && Boolean(appType),
    refetchOnMount: "always",
  });
}

/** 本地 USD → 指定币种汇率（未配置返回默认值；USD 返回 "1"） */
export function useExchangeRate(currency: UsageLimitCurrency) {
  return useQuery({
    queryKey: usageLimitKeys.exchangeRate(currency),
    queryFn: () => usageLimitApi.getExchangeRate(currency),
    staleTime: 60_000,
  });
}

function useInvalidateUsageLimit(providerId: string, appType: AppId) {
  const queryClient = useQueryClient();
  return () => {
    void queryClient.invalidateQueries({
      queryKey: usageLimitKeys.status(providerId, appType),
    });
  };
}

/** 保存限额配置（新建 / 修改 / 启停） */
export function useSaveUsageLimit(providerId: string, appType: AppId) {
  const invalidate = useInvalidateUsageLimit(providerId, appType);
  return useMutation({
    mutationFn: (config: UsageLimitConfig) =>
      usageLimitApi.save(providerId, appType, config),
    onSuccess: invalidate,
  });
}

/** 重置使用量：usage_start_at 推进到当前时间，不删除历史 Usage */
export function useResetUsageLimit(providerId: string, appType: AppId) {
  const invalidate = useInvalidateUsageLimit(providerId, appType);
  return useMutation({
    mutationFn: () => usageLimitApi.resetUsage(providerId, appType),
    onSuccess: invalidate,
  });
}

/** 保存本地汇率（USD → 指定币种） */
export function useSetExchangeRate() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      currency,
      rate,
    }: {
      currency: UsageLimitCurrency;
      rate: string;
    }) => usageLimitApi.setExchangeRate(currency, rate),
    onSuccess: () => {
      // 已换算的非 USD 用量展示依赖汇率，全部币种 + 状态一并刷新
      void queryClient.invalidateQueries({ queryKey: usageLimitKeys.all });
    },
  });
}
