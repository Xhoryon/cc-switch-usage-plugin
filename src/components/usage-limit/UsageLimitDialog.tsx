import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { AlertTriangle, Gauge, RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import type { Provider } from "@/types";
import type { AppId } from "@/lib/api";
import {
  useExchangeRate,
  useResetUsageLimit,
  useSaveUsageLimit,
  useSetExchangeRate,
} from "@/lib/query/usageLimit";
import {
  budgetSummary,
  formatMoney,
  formatTokensExact,
} from "./UsageLimitButton";
import {
  USAGE_LIMIT_CURRENCY_SYMBOLS,
  type UsageLimitConfig,
  type UsageLimitCurrency,
  type UsageLimitResetPeriod,
  type UsageLimitState,
  type UsageLimitStatus,
  type UsageLimitType,
  type UsageLimitWindowUnit,
} from "@/types/usageLimit";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { extractErrorMessage } from "@/utils/errorUtils";

interface UsageLimitDialogProps {
  provider: Provider;
  appId: AppId;
  isOpen: boolean;
  status: UsageLimitStatus | undefined;
  onClose: () => void;
}

/**
 * 使用限额 Dialog（需求 §4-§6、§23-§26）。
 *
 * - 默认 OFF：只展示标题、说明与开关，保持简洁
 * - 开关打开后在同一 Dialog 内展开完整配置（不弹二级 Dialog）
 * - 重置使用量走确认框：usage_start_at 前移，不删历史
 *
 * 视觉遵循 CC Switch 现有氛围：glass 磨砂面板 + bg-muted 分段控件 +
 * 半透明 amber 警示条（与应用内设置行 / ProviderForm 面板同款）。
 */
export function UsageLimitDialog({
  provider,
  appId,
  isOpen,
  status,
  onClose,
}: UsageLimitDialogProps) {
  const { t, i18n } = useTranslation();

  const [enabled, setEnabled] = useState(false);
  const [limitType, setLimitType] = useState<UsageLimitType>("money");
  const [currency, setCurrency] = useState<UsageLimitCurrency>("USD");
  const [amountInput, setAmountInput] = useState("");
  const [resetPeriod, setResetPeriod] =
    useState<UsageLimitResetPeriod>("never");
  const [windowLengthInput, setWindowLengthInput] = useState("");
  const [windowUnit, setWindowUnit] = useState<UsageLimitWindowUnit>("hours");
  const [rateInput, setRateInput] = useState("");
  const [touchedRate, setTouchedRate] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [confirmResetOpen, setConfirmResetOpen] = useState(false);

  // 汇率按币种分查询；切换币种后重新加载对应汇率
  const { data: savedRate } = useExchangeRate(currency);
  const saveMutation = useSaveUsageLimit(provider.id, appId);
  const resetMutation = useResetUsageLimit(provider.id, appId);
  const setRateMutation = useSetExchangeRate();

  // 打开时用已保存配置初始化（关闭→重开保留原配置，§24）。
  // 仅在「isOpen 由关变开」后的首次初始化及随后的配置到达时执行；
  // 打开期间的后台 refetch（refetchOnMount / 记账事件）不得打断用户编辑。
  const wasOpenRef = useRef(false);
  useEffect(() => {
    if (!isOpen) {
      wasOpenRef.current = false;
      return;
    }
    if (wasOpenRef.current) {
      return;
    }
    wasOpenRef.current = true;
    setEnabled(status?.enabled ?? false);
    setLimitType(status?.limitType ?? "money");
    setCurrency(status?.currency ?? "USD");
    setAmountInput(status?.limitAmount ?? "");
    setResetPeriod(status?.resetPeriod ?? "never");
    setWindowLengthInput(
      status?.windowLength != null ? String(status.windowLength) : "",
    );
    setWindowUnit(status?.windowUnit === "days" ? "days" : "hours");
    setTouchedRate(false);
    setRateInput("");
    setValidationError(null);
  }, [
    isOpen,
    status?.enabled,
    status?.limitType,
    status?.currency,
    status?.limitAmount,
    status?.resetPeriod,
  ]);

  // 未手动编辑过汇率时，用当前币种的已保存（或默认）汇率回填。
  // 依赖必须包含 currency：切换币种的瞬间会清空输入框（见 onClick），
  // 新币种汇率（即使与旧值相同）返回后由本 effect 回填；未返回前输入框
  // 保持为空，保存被校验拦截——杜绝「上一币种汇率被持久化为当前币种」
  // 的竞态（审查 P1-1）。
  useEffect(() => {
    if (savedRate && !touchedRate) {
      setRateInput(savedRate);
    }
  }, [currency, savedRate, touchedRate]);

  const state: UsageLimitState = status?.state ?? "off";
  const percent = useMemo(() => {
    const raw = Number(status?.percentUsed);
    return Number.isFinite(raw) ? raw : 0;
  }, [status?.percentUsed]);

  // 启用行的实时副标题：与卡片图标 tooltip 同一套文案
  const enableRowSubtitle = useMemo(() => {
    if (!status || !status.enabled) {
      return t("usageLimit.tooltipOff");
    }
    if (state === "exhausted") {
      return t("usageLimit.tooltipExhausted");
    }
    const summary = budgetSummary(status);
    return summary
      ? `${summary.used} / ${summary.limit}`
      : t("usageLimit.title");
  }, [status, state, t]);

  // 当前输入币种下的已用金额：
  // - USD 直接用 USD 原值
  // - 已保存限额币种与所选一致时，用后端换算值
  // - 其余（切换中 / 尚未保存）按本地汇率输入估算
  const usedMoneyInInputCurrency = useMemo(() => {
    if (!status) {
      return 0;
    }
    if (currency === "USD") {
      return Number(status.usedMoneyUsd);
    }
    if (status.currency === currency && status.usedMoneyInCurrency != null) {
      const converted = Number(status.usedMoneyInCurrency);
      if (Number.isFinite(converted)) {
        return converted;
      }
    }
    const rate = Number(rateInput) || 0;
    return Number(status.usedMoneyUsd) * rate;
  }, [status, currency, rateInput]);

  const limitValue = useMemo(() => Number(amountInput), [amountInput]);
  const hasValidAmount = useMemo(() => {
    if (limitType === "token") {
      return /^\d+$/.test(amountInput.trim()) && Number(amountInput.trim()) > 0;
    }
    return Number.isFinite(limitValue) && limitValue > 0;
  }, [limitType, amountInput, limitValue]);

  const usedValue =
    limitType === "token"
      ? (status?.usedTokens ?? 0)
      : usedMoneyInInputCurrency;
  const remainingValue = hasValidAmount ? limitValue - usedValue : 0;

  const showEnforcementWarning =
    enabled && status && status.enforcement !== "active";
  const showUnpricedWarning =
    enabled && limitType === "money" && (status?.unpricedRequestCount ?? 0) > 0;

  const handleSave = () => {
    const trimmed = amountInput.trim().replace(/,/g, "");
    // 前端预校验（后端仍会权威校验）：非法输入不保存
    if (limitType === "money") {
      const value = Number(trimmed);
      if (!trimmed || !Number.isFinite(value) || value <= 0) {
        setValidationError(t("usageLimit.errorInvalidAmount"));
        return;
      }
    } else if (!/^\d+$/.test(trimmed) || Number(trimmed) <= 0) {
      setValidationError(t("usageLimit.errorInvalidTokens"));
      return;
    }
    if (resetPeriod === "custom") {
      const len = Number(windowLengthInput.trim());
      if (
        !windowLengthInput.trim() ||
        !Number.isInteger(len) ||
        len < 1 ||
        len > 10000
      ) {
        setValidationError(t("usageLimit.errorInvalidWindow"));
        return;
      }
    }
    if (limitType === "money" && currency !== "USD") {
      const rate = Number(rateInput.trim());
      // 上限与后端 set_exchange_rate 一致（1,000,000）：
      // 前端先拦住，避免「config 已保存、汇率被后端拒绝」的半保存状态
      if (
        !rateInput.trim() ||
        !Number.isFinite(rate) ||
        rate <= 0 ||
        rate > 1_000_000
      ) {
        setValidationError(t("usageLimit.errorInvalidRate"));
        return;
      }
    }

    setValidationError(null);
    const config: UsageLimitConfig = {
      enabled,
      limitType,
      currency: limitType === "money" ? currency : null,
      limitAmount: trimmed,
      resetPeriod,
      windowLength:
        resetPeriod === "custom" ? Number(windowLengthInput.trim()) : null,
      windowUnit: resetPeriod === "custom" ? windowUnit : null,
    };
    saveMutation.mutate(config, {
      onSuccess: () => {
        // 非 USD 币种顺带持久化本地汇率（人工调节，不联网获取）
        if (limitType === "money" && currency !== "USD") {
          const rate = rateInput.trim();
          if (rate && Number(rate) > 0) {
            setRateMutation.mutate(
              { currency, rate },
              {
                onError: (error) => {
                  toast.error(extractErrorMessage(error));
                },
              },
            );
          }
        }
        toast.success(t("usageLimit.saved"));
        onClose();
      },
      onError: (error) => {
        toast.error(extractErrorMessage(error));
      },
    });
  };

  const handleReset = () => {
    resetMutation.mutate(undefined, {
      onSuccess: () => {
        toast.success(t("usageLimit.resetDone"));
        setConfirmResetOpen(false);
      },
      onError: (error) => {
        toast.error(extractErrorMessage(error));
        setConfirmResetOpen(false);
      },
    });
  };

  const toggleType = (type: UsageLimitType) => {
    setLimitType(type);
    setValidationError(null);
  };

  const hasSavedConfig = Boolean(status?.limitAmount);
  const canReset = Boolean(status?.enabled || hasSavedConfig);

  return (
    <>
      {/* 非模态：遮罩不拦截指针事件，窗口保持可拖动、背景可交互（V1.2.0） */}
      <Dialog
        open={isOpen}
        onOpenChange={(open) => !open && onClose()}
        modal={false}
      >
        <DialogContent
          className="sm:max-w-md"
          overlayClassName="pointer-events-none"
          data-testid="usage-limit-dialog"
        >
          <DialogHeader>
            <DialogTitle>
              {t("usageLimit.title")}
              <span className="ml-2 text-sm font-normal text-muted-foreground">
                {provider.name}
              </span>
            </DialogTitle>
            <DialogDescription>{t("usageLimit.description")}</DialogDescription>
          </DialogHeader>

          {/* 内容区可滚动：小窗口下配置过长时不再被裁切（V1.2.0） */}
          <div
            className="min-h-0 flex-1 overflow-y-auto"
            data-testid="usage-limit-scroll"
          >
            {/* 启用开关：设置页同款图标卡片行 */}
            <div className="glass flex items-center gap-3 rounded-xl border border-white/10 p-3.5">
              <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-primary/10">
                <Gauge className="h-4 w-4 text-primary" />
              </div>
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium leading-tight">
                  {t("usageLimit.enable")}
                </p>
                <p className="mt-0.5 truncate text-xs text-muted-foreground">
                  {enableRowSubtitle}
                </p>
              </div>
              <Switch
                checked={enabled}
                onCheckedChange={(checked) => {
                  setEnabled(checked);
                  setValidationError(null);
                }}
                aria-label={t("usageLimit.enable")}
                data-testid="usage-limit-toggle"
              />
            </div>

            {enabled && (
              <div className="glass space-y-4 rounded-xl border border-white/10 p-4">
                {/* 限制方式 */}
                <div className="space-y-2">
                  <span className="text-sm font-medium">
                    {t("usageLimit.limitType")}
                  </span>
                  <div className="grid grid-cols-2 gap-1 rounded-lg bg-muted p-1">
                    <SegmentOption
                      active={limitType === "money"}
                      label={t("usageLimit.money")}
                      onClick={() => toggleType("money")}
                    />
                    <SegmentOption
                      active={limitType === "token"}
                      label={t("usageLimit.tokens")}
                      onClick={() => toggleType("token")}
                    />
                  </div>
                </div>

                {limitType === "money" && (
                  <>
                    {/* 币种（V1.0.2：5 种主流货币，USD 为内部计价基准） */}
                    <div className="space-y-2">
                      <span className="text-sm font-medium">
                        {t("usageLimit.currency")}
                      </span>
                      <div
                        className="grid grid-cols-5 gap-1 rounded-lg bg-muted p-1"
                        data-testid="usage-limit-currency"
                      >
                        {(
                          [
                            ["USD", "usd"],
                            ["CNY", "cny"],
                            ["EUR", "eur"],
                            ["JPY", "jpy"],
                            ["GBP", "gbp"],
                          ] as const
                        ).map(([value, labelKey]) => (
                          <SegmentOption
                            key={value}
                            active={currency === value}
                            label={t(`usageLimit.${labelKey}`)}
                            onClick={() => {
                              setCurrency(value);
                              // 立刻清空旧币种汇率：新币种汇率返回前输入框为空，
                              // 保存会被校验拦截，杜绝错误汇率被持久化（审查 P1-1）
                              setRateInput("");
                              setTouchedRate(false);
                              setValidationError(null);
                            }}
                          />
                        ))}
                      </div>
                    </div>

                    {/* 非 USD 本地汇率（人工调节，不依赖在线汇率 API） */}
                    {currency !== "USD" && (
                      <div className="space-y-1.5">
                        <label
                          htmlFor="usage-limit-rate"
                          className="text-sm font-medium"
                        >
                          {t("usageLimit.exchangeRate", { currency })}
                        </label>
                        <Input
                          id="usage-limit-rate"
                          data-testid="usage-limit-rate"
                          value={rateInput}
                          onChange={(e) => {
                            setRateInput(e.target.value);
                            setTouchedRate(true);
                          }}
                          placeholder={
                            currency === "JPY"
                              ? "150"
                              : currency === "CNY"
                                ? "7.20"
                                : "0.90"
                          }
                          inputMode="decimal"
                          className="h-9"
                        />
                        <p className="text-xs text-muted-foreground">
                          {t("usageLimit.exchangeRateHint")}
                        </p>
                      </div>
                    )}

                    {/* 最大金额（带币种符号后缀） */}
                    <div className="space-y-1.5">
                      <label
                        htmlFor="usage-limit-amount"
                        className="text-sm font-medium"
                      >
                        {t("usageLimit.maxAmount")}
                      </label>
                      <div className="relative">
                        <Input
                          id="usage-limit-amount"
                          data-testid="usage-limit-amount"
                          value={amountInput}
                          onChange={(e) => {
                            setAmountInput(e.target.value);
                            setValidationError(null);
                          }}
                          placeholder="100.00"
                          inputMode="decimal"
                          className="h-9 pr-10"
                        />
                        <span
                          data-testid="usage-limit-currency-symbol"
                          className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-xs tabular-nums text-muted-foreground"
                        >
                          {USAGE_LIMIT_CURRENCY_SYMBOLS[currency]}
                        </span>
                      </div>
                    </div>
                  </>
                )}

                {limitType === "token" && (
                  <div className="space-y-1.5">
                    <label
                      htmlFor="usage-limit-amount"
                      className="text-sm font-medium"
                    >
                      {t("usageLimit.maxTokens")}
                    </label>
                    <Input
                      id="usage-limit-amount"
                      data-testid="usage-limit-amount"
                      value={amountInput}
                      onChange={(e) => {
                        setAmountInput(e.target.value);
                        setValidationError(null);
                      }}
                      placeholder="5,000,000"
                      inputMode="numeric"
                      className="h-9"
                    />
                  </div>
                )}

                {/* 重置周期（V1.0.1）：本地时区周期边界自动重置窗口 */}
                <div className="space-y-2">
                  <span className="text-sm font-medium">
                    {t("usageLimit.resetPeriod")}
                  </span>
                  <div
                    className="grid grid-cols-3 gap-1 rounded-lg bg-muted p-1"
                    data-testid="usage-limit-reset-period"
                  >
                    {(
                      [
                        ["never", "resetNever"],
                        ["hourly", "resetHourly"],
                        ["daily", "resetDaily"],
                        ["weekly", "resetWeekly"],
                        ["monthly", "resetMonthly"],
                        ["custom", "resetCustom"],
                      ] as const
                    ).map(([value, labelKey]) => (
                      <SegmentOption
                        key={value}
                        active={resetPeriod === value}
                        label={t(`usageLimit.${labelKey}`)}
                        onClick={() => {
                          setResetPeriod(value);
                          setValidationError(null);
                        }}
                      />
                    ))}
                  </div>
                  {resetPeriod === "custom" && (
                    <div className="space-y-1.5">
                      <label
                        htmlFor="usage-limit-window-length"
                        className="text-sm font-medium"
                      >
                        {t("usageLimit.windowLength")}
                      </label>
                      <div className="flex gap-2">
                        <Input
                          id="usage-limit-window-length"
                          data-testid="usage-limit-window-length"
                          value={windowLengthInput}
                          onChange={(e) => {
                            setWindowLengthInput(e.target.value);
                            setValidationError(null);
                          }}
                          placeholder="6"
                          inputMode="numeric"
                          className="h-9"
                        />
                        <div className="grid flex-1 grid-cols-2 gap-1 rounded-lg bg-muted p-1">
                          <SegmentOption
                            active={windowUnit === "hours"}
                            label={t("usageLimit.hours")}
                            onClick={() => setWindowUnit("hours")}
                          />
                          <SegmentOption
                            active={windowUnit === "days"}
                            label={t("usageLimit.days")}
                            onClick={() => setWindowUnit("days")}
                          />
                        </div>
                      </div>
                    </div>
                  )}
                  {resetPeriod === "never" ? (
                    <p className="text-xs text-muted-foreground">
                      {t("usageLimit.resetPeriodHint")}
                    </p>
                  ) : (
                    // 下次重置时间仅在所选周期与已保存配置一致时展示，
                    // 避免「正在切换、尚未保存」时展示过期边界
                    status?.resetPeriod === resetPeriod &&
                    status.nextResetAt != null && (
                      <p
                        className="text-xs text-muted-foreground"
                        data-testid="usage-limit-next-reset"
                      >
                        {t("usageLimit.nextResetAt", {
                          time: formatResetTime(
                            status.nextResetAt,
                            i18n.language,
                          ),
                        })}
                      </p>
                    )
                  )}
                </div>

                {/* enforcement 边界提示（§20 / §21） */}
                {showEnforcementWarning && (
                  <WarningBanner>
                    {status.enforcement === "unsupported_credential"
                      ? t("usageLimit.enforcementUnsupported")
                      : t("usageLimit.enforcementProxyDisabled")}
                  </WarningBanner>
                )}

                {/* 未知定价提示（§10） */}
                {showUnpricedWarning && (
                  <WarningBanner>
                    {t("usageLimit.pricingUnavailable", {
                      count: status?.unpricedRequestCount ?? 0,
                      models: (status?.unpricedModels ?? []).join(", "),
                    })}
                  </WarningBanner>
                )}

                {/* 用量与进度（§26：百分比不 clamp，进度条视觉封顶 100%） */}
                <div className="space-y-2.5 rounded-lg border border-border/60 p-3">
                  <UsageRow
                    label={t("usageLimit.used")}
                    value={
                      limitType === "token"
                        ? `${formatTokensExact(status?.usedTokens ?? 0)} tokens`
                        : formatMoney(
                            usedMoneyInInputCurrency.toString(),
                            currency,
                          )
                    }
                  />
                  {hasValidAmount && (
                    <UsageRow
                      label={t("usageLimit.remaining")}
                      value={
                        limitType === "token"
                          ? `${formatTokensExact(remainingValue)} tokens`
                          : formatMoney(remainingValue.toString(), currency)
                      }
                    />
                  )}
                  <div className="pt-0.5">
                    <ProgressBar percent={percent} state={state} />
                    {hasValidAmount && (
                      <p className="mt-1.5 text-right text-xs tabular-nums text-muted-foreground">
                        {percent.toFixed(1)}%
                      </p>
                    )}
                  </div>
                </div>
              </div>
            )}

            {validationError && (
              <p
                className="text-sm text-red-600 dark:text-red-400"
                role="alert"
                data-testid="usage-limit-error"
              >
                {validationError}
              </p>
            )}
          </div>

          <DialogFooter className="gap-2 sm:gap-0">
            {canReset && (
              <Button
                variant="ghost"
                className="mr-auto text-muted-foreground hover:text-red-600 dark:hover:text-red-400"
                onClick={() => setConfirmResetOpen(true)}
                data-testid="usage-limit-reset"
              >
                <RotateCcw className="mr-1 h-4 w-4" />
                {t("usageLimit.reset")}
              </Button>
            )}
            <Button variant="outline" onClick={onClose}>
              {t("common.cancel")}
            </Button>
            <Button
              onClick={handleSave}
              disabled={saveMutation.isPending}
              data-testid="usage-limit-save"
            >
              {saveMutation.isPending ? t("common.saving") : t("common.save")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ConfirmDialog
        isOpen={confirmResetOpen}
        title={t("usageLimit.resetConfirmTitle")}
        message={t("usageLimit.resetConfirmMessage")}
        confirmText={t("usageLimit.reset")}
        cancelText={t("common.cancel")}
        variant="info"
        zIndex="nested"
        pending={resetMutation.isPending}
        onConfirm={() => handleReset()}
        onCancel={() => setConfirmResetOpen(false)}
      />
    </>
  );
}

/** 下次重置时间的本地化展示（与 Usage 页 request 时间同款 locale 推导） */
function formatResetTime(unixSeconds: number, language: string): string {
  const locale =
    language === "zh"
      ? "zh-CN"
      : language === "zh-TW"
        ? "zh-TW"
        : language === "ja"
          ? "ja-JP"
          : "en-US";
  return new Date(unixSeconds * 1000).toLocaleString(locale);
}

/** bg-muted 轨道分段选择（与设置页 pill 风格一致） */
function SegmentOption({
  active,
  label,
  onClick,
}: {
  active: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <Button
      type="button"
      size="sm"
      variant="ghost"
      onClick={onClick}
      aria-pressed={active}
      className={cn(
        "h-8 font-medium",
        active && "bg-background shadow-sm hover:bg-background",
        !active && "text-muted-foreground hover:text-foreground",
      )}
    >
      {label}
    </Button>
  );
}

function UsageRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between text-sm">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-medium tabular-nums">{value}</span>
    </div>
  );
}

/** 进度条：视觉最多填满 100%，颜色随状态变化 */
function ProgressBar({
  percent,
  state,
}: {
  percent: number;
  state: UsageLimitState;
}) {
  const clamped = Math.max(0, Math.min(100, percent));
  return (
    <div
      className="h-2 w-full overflow-hidden rounded-full bg-muted"
      data-testid="usage-limit-progress"
    >
      <div
        className={cn(
          "h-full rounded-full transition-all",
          state === "exhausted" && "bg-red-500",
          state === "warning" && "bg-amber-500",
          (state === "active" || state === "off") && "bg-emerald-500",
        )}
        style={{ width: `${clamped}%` }}
      />
    </div>
  );
}

/** 半透明警示条（与应用 amber 提示同款色调，深浅色自适应） */
function WarningBanner({ children }: { children: ReactNode }) {
  return (
    <div
      className="flex items-start gap-2 rounded-md border border-amber-500/30 bg-amber-500/10 p-2.5 text-xs leading-relaxed text-amber-600 dark:text-amber-400"
      data-testid="usage-limit-warning"
    >
      <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-500 dark:text-amber-400" />
      <span>{children}</span>
    </div>
  );
}
