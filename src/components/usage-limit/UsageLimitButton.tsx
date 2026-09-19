import { memo, useCallback, useState } from "react";
import { Eye, EyeOff, Gauge, Timer } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { useUsageLimitStatus } from "@/lib/query/usageLimit";
import { isProxyAppId } from "@/config/appConfig";
import type { AppId } from "@/lib/api";
import type {
  UsageLimitCurrency,
  UsageLimitState,
  UsageLimitStatus,
} from "@/types/usageLimit";
import { USAGE_LIMIT_CURRENCY_SYMBOLS } from "@/types/usageLimit";

/** Token 友好显示：532 / 12.4K / 1.25M / 18.2M */
export function formatTokensCompact(tokens: number): string {
  if (tokens < 1000) {
    return String(tokens);
  }
  if (tokens < 1_000_000) {
    return `${(tokens / 1000).toFixed(1).replace(/\.0$/, "")}K`;
  }
  if (tokens < 10_000_000) {
    return `${(tokens / 1_000_000).toFixed(2).replace(/\.?0+$/, "")}M`;
  }
  return `${(tokens / 1_000_000).toFixed(1).replace(/\.0$/, "")}M`;
}

/** Token 精确显示：1,245,821 */
export function formatTokensExact(tokens: number): string {
  return tokens.toLocaleString("en-US");
}

/** 金额显示：$12.34 / ¥88.50 / €9.99（符号表与后端 LimitCurrency::symbol 一致） */
export function formatMoney(amount: string, currency: UsageLimitCurrency) {
  const symbol = USAGE_LIMIT_CURRENCY_SYMBOLS[currency] ?? "$";
  const value = Number(amount);
  if (!Number.isFinite(value)) {
    return `${symbol}${amount}`;
  }
  return `${symbol}${value.toFixed(2)}`;
}

interface BudgetSummary {
  used: string;
  limit: string;
}

/** 图标 Tooltip 里的简略用量（$2.41 / $10 或 1.2M / 5M） */
export function budgetSummary(
  status: UsageLimitStatus,
): BudgetSummary | undefined {
  if (!status.enabled || !status.limitType || !status.limitAmount) {
    return undefined;
  }
  if (status.limitType === "money") {
    const currency: UsageLimitCurrency = status.currency ?? "USD";
    const used =
      currency === "USD"
        ? status.usedMoneyUsd
        : (status.usedMoneyInCurrency ?? status.usedMoneyUsd);
    return {
      used: formatMoney(used, currency),
      limit: formatMoney(status.limitAmount, currency),
    };
  }
  return {
    used: formatTokensCompact(Number(status.usedTokens)),
    limit: formatTokensCompact(Number(status.limitAmount)),
  };
}

interface UsageLimitButtonProps {
  providerId: string;
  appId: AppId;
  onOpen: () => void;
}

/**
 * Provider Card 上的使用限额图标。
 *
 * 四种状态（需求 §3.2）：
 * - OFF：未开启限额，普通灰色
 * - ACTIVE：已开启、用量正常
 * - WARNING：>= 80% 且 < 100%
 * - EXHAUSTED：>= 100%，明显警示
 *
 * 仅对可走 Local Proxy 的应用显示（claude / codex / gemini / grokbuild）；
 * 非代理应用没有强制执行能力，不渲染。外层组件先做应用类型闸门，
 * 内层组件才挂查询 hook（保证 hook 顺序稳定）。
 */
function UsageLimitButtonImpl({
  providerId,
  appId,
  onOpen,
}: UsageLimitButtonProps) {
  if (!isProxyAppId(appId)) {
    return null;
  }
  return (
    <UsageLimitButtonInner
      providerId={providerId}
      appId={appId}
      onOpen={onOpen}
    />
  );
}

function UsageLimitButtonInner({
  providerId,
  appId,
  onOpen,
}: UsageLimitButtonProps) {
  const { t } = useTranslation();
  const { data: status } = useUsageLimitStatus(providerId, appId);

  const state: UsageLimitState = status?.state ?? "off";
  const summary = status ? budgetSummary(status) : undefined;

  // 卡片徽标显示开关（V1.2.0）：按 provider 持久化在 localStorage
  const storageKey = badgeStorageKey(appId, providerId);
  const [showBadge, setShowBadge] = useState(() => {
    try {
      return window.localStorage.getItem(storageKey) === "1";
    } catch {
      return false;
    }
  });
  const toggleBadge = useCallback(() => {
    setShowBadge((prev) => {
      const next = !prev;
      try {
        window.localStorage.setItem(storageKey, next ? "1" : "0");
      } catch {
        // localStorage 不可用时仅内存态生效
      }
      return next;
    });
  }, [storageKey]);

  const percent =
    status?.enabled && status.percentUsed != null
      ? Number(status.percentUsed)
      : null;
  const badgeVisible = showBadge && percent != null && Number.isFinite(percent);

  let title = t("usageLimit.title");
  if (status) {
    switch (state) {
      case "off":
        title = t("usageLimit.tooltipOff", { defaultValue: "使用限额：关闭" });
        break;
      case "exhausted":
        title = t("usageLimit.tooltipExhausted", {
          defaultValue: "已达到使用限额",
        });
        break;
      case "warning":
      case "active":
        title = summary
          ? `${summary.used} / ${summary.limit}`
          : t("usageLimit.title");
        break;
    }
  }

  return (
    <TooltipProvider delayDuration={250}>
      <div className="flex items-center gap-1.5">
        {badgeVisible && (
          <span
            className="flex items-center gap-1 text-xs tabular-nums"
            data-testid="usage-limit-card-badge"
          >
            <span
              className={cn(
                "font-medium",
                state === "exhausted" && "text-red-600 dark:text-red-400",
                state === "warning" && "text-amber-600 dark:text-amber-400",
                (state === "active" || state === "off") &&
                  "text-emerald-600 dark:text-emerald-400",
              )}
            >
              {percent!.toFixed(0)}%
            </span>
            {status?.nextResetAt != null && (
              <span className="flex items-center gap-0.5 text-muted-foreground">
                <Timer className="h-3 w-3" aria-hidden="true" />
                {formatDurationUntil(status.nextResetAt)}
              </span>
            )}
          </span>
        )}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              size="icon"
              variant="ghost"
              onClick={toggleBadge}
              aria-pressed={showBadge}
              aria-label={
                showBadge
                  ? t("usageLimit.hideFromCard")
                  : t("usageLimit.showOnCard")
              }
              className="h-8 w-8 p-1 text-muted-foreground hover:text-foreground"
            >
              {showBadge ? (
                <Eye className="h-4 w-4" />
              ) : (
                <EyeOff className="h-4 w-4" />
              )}
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {showBadge
              ? t("usageLimit.hideFromCard")
              : t("usageLimit.showOnCard")}
          </TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              size="icon"
              variant="ghost"
              onClick={onOpen}
              aria-label={t("usageLimit.title")}
              className={cn(
                "h-8 w-8 p-1",
                state === "off" &&
                  "text-muted-foreground hover:text-foreground",
                state === "active" && "text-emerald-600 dark:text-emerald-400",
                state === "warning" && "text-amber-600 dark:text-amber-400",
                state === "exhausted" &&
                  "text-red-600 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300",
              )}
            >
              <Gauge className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent className="flex items-center gap-1.5">
            <span
              aria-hidden="true"
              className={cn(
                "h-1.5 w-1.5 rounded-full",
                state === "off" && "bg-muted-foreground",
                state === "active" && "bg-emerald-400",
                state === "warning" && "bg-amber-400",
                state === "exhausted" && "bg-red-400",
              )}
            />
            {title}
          </TooltipContent>
        </Tooltip>
      </div>
    </TooltipProvider>
  );
}

/** 距下次重置的紧凑时长：38m / 5h12m / 2d4h（<1m 显示 <1m） */
export function formatDurationUntil(
  targetEpochSeconds: number,
  nowEpochSeconds: number = Date.now() / 1000,
): string {
  const secs = Math.max(0, Math.floor(targetEpochSeconds - nowEpochSeconds));
  if (secs < 60) return "<1m";
  const m = Math.floor((secs % 3600) / 60);
  const h = Math.floor((secs % 86400) / 3600);
  const d = Math.floor(secs / 86400);
  if (d > 0) return `${d}d${h}h`;
  if (h > 0) return m > 0 ? `${h}h${m}m` : `${h}h`;
  return `${m}m`;
}

const badgeStorageKey = (appId: string, providerId: string) =>
  `ccswitch.usageBadge.${appId}.${providerId}`;

export const UsageLimitButton = memo(UsageLimitButtonImpl);
