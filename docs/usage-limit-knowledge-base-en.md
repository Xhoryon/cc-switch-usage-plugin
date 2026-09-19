<!-- Language / 语言: [简体中文](usage-limit-knowledge-base-zh.md) | [English](usage-limit-knowledge-base-en.md) | [繁體中文](usage-limit-knowledge-base-zh-TW.md) | [日本語](usage-limit-knowledge-base-ja.md) -->

# Usage Limit (Budget Limit) Knowledge Base

> Applies to **V1.2.0** (public release, 2026-09-18; installer-fix release,
> functionality identical to V1.1.0, base CC Switch 3.20.3). Iteration details
> live in the matching `docs/development_log.md` entries. This document is an
> integrated view, written for future maintainers: what it is, why it is
> designed this way, and what to watch out for when changing it.

---

## 1. Overview

Every provider's static API key can get its own usage cap. Once the cap is
reached, the **local proxy rejects new requests before forwarding them**
(real enforcement, not just accounting).

| Capability | Details |
| --- | --- |
| Limit type | Money (USD / CNY / EUR / JPY / GBP) or tokens; one at a time |
| Enforcement point | Local Proxy `forward_with_retry_inner`, before each provider attempt forwards |
| Usage source | Reuses the Usage SSOT (`proxy_request_logs`); no parallel accounting |
| Credential binding | Irreversible SHA-256 fingerprint; plaintext API keys are never stored anywhere |
| Reset schedule (V1.0.1) | Never / hourly / daily / weekly / monthly, lazy rollover on local-time boundaries |
| Multi-currency (V1.0.2) | Non-USD currencies convert through a locally adjustable rate (manual, offline) |
| Manual reset | Advances `usage_start_at` to now; no history is ever deleted |

**Core boundary (keep this in sync with the feature everywhere)**:

> Budget enforcement only applies to traffic routed through CC Switch Local
> Proxy. Traffic that bypasses the proxy takeover cannot be observed or
> blocked; the UI says so via `enforcement = proxy_disabled` instead of
> pretending the hard limit works. Usage of the same key on other devices or
> via direct connections is invisible to this budget (usage is local
> observation).

---

## 2. Architecture

```
Config / display path
  Provider Card (ProviderActions icon button)
    └─ UsageLimitButton (gauge icon, fetches status via useUsageLimitStatus)
         └─ UsageLimitDialog (toggle / limit type / currency / amount /
            reset schedule / progress)
              └─ Tauri Command (get_usage_limit_status / save_usage_limit /
                 reset_usage_limit / get_exchange_rate / set_exchange_rate)
                   └─ Database impl (services/usage_limit.rs)
                        ├─ api_key_limits table (config + window start + period)
                        └─ proxy_request_logs aggregation (usage, SSOT reuse)

Enforcement path
  Client Request
    └─ CC Switch Local Proxy (forwarder.rs)
         └─ Budget Guard (per provider attempt, before forward())
              ├─ disabled → allow (zero-cost path is one PK lookup)
              ├─ credential fingerprint mismatch → rebind + reset window to now
              ├─ period boundary crossed (V1.0.1) → lazy window rollover
              └─ used >= limit → ProxyError::BudgetExhausted (429, no upstream call)
                   └─ after the response completes: Usage Accounting
                      (log_with_calculation stores credential_fingerprint
                      into proxy_request_logs)
```

Layering discipline: React → Tauri API → Command → Service (Database impl) →
DAO → SQLite. React never touches the database; money/token aggregation lives
only on the Rust side.

---

## 3. Data model

### 3.1 `api_key_limits` (created in V1.0, extended in V1.0.1)

```sql
CREATE TABLE api_key_limits (
    provider_id            TEXT NOT NULL,
    app_type               TEXT NOT NULL,
    credential_fingerprint TEXT NOT NULL,      -- SHA-256 hex, never a plaintext key
    enabled                INTEGER NOT NULL DEFAULT 0,
    limit_type             TEXT NOT NULL CHECK (limit_type IN ('money', 'token')),
    currency               TEXT,                    -- NULL for token; no CHECK since V1.0.2
    limit_amount           TEXT NOT NULL,      -- decimal string (money) / positive integer string (token)
    usage_start_at         INTEGER NOT NULL,   -- budget window start (unix seconds)
    reset_period           TEXT NOT NULL DEFAULT 'never',  -- V1.0.1
    window_length          INTEGER,            -- V1.2.0: custom window length
    window_unit            TEXT,               -- V1.2.0: hours | days
    created_at             INTEGER NOT NULL,
    updated_at             INTEGER NOT NULL,
    PRIMARY KEY (provider_id, app_type),
    FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type)
        ON DELETE CASCADE                       -- cascade cleanup when provider is deleted
);
```

- At most one config row per provider (the PK).
- `limit_amount` is a string so SQLite REAL floating-point error can never
  enter the limit comparison.
- `reset_period` deliberately has **no CHECK**: the ALTER ADD COLUMN migration
  and the fresh-install DDL behave identically; the value domain is validated
  on the save path, and unknown stored values fall back to `never` with a
  warning.
- The `currency` CHECK was **removed** in the v22 migration (V1.0.2
  multi-currency): SQLite cannot modify an existing CHECK, so the migration is
  the standard table rebuild (create new → copy → drop old → rename;
  idempotency is detected via whether `sqlite_master.sql` still contains
  `CHECK (currency`). The value domain is validated by `LimitCurrency::parse`
  on the save path; unknown stored values fall back to USD on read. **Future
  currency additions need no migration.**
- This table **never stores cumulative usage** (no double SSOT); usage is
  always aggregated live from `proxy_request_logs`.

### 3.2 Usage SSOT aggregation predicate

All budget accounting shares the same WHERE clause:

```sql
WHERE provider_id = ? AND app_type = ?
  AND credential_fingerprint = ?
  AND created_at >= usage_start_at   -- effective window start (see §4.3)
  AND data_source = 'proxy'          -- session-sync / direct rows are excluded
```

- **Tokens**: normalized total = fresh input + output + cache_creation +
  cache_read (`services/sql_helpers::fresh_input_sql` — the same rule as the
  Usage Dashboard; `input_token_semantics` distinguishes cache semantics).
- **Money**: sums the stored `total_cost_usd` decimal strings. Two-stage
  pipeline:
  - Fast path: `SUM(CAST(... AS REAL))` clearly below the threshold
    (relative margin, see §4.2) → O(1) allow;
  - Exact path: near or above the threshold, re-sums the raw strings row by
    row with `rust_decimal`, avoiding 0.1+0.2 ≠ 0.3 float errors.

### 3.3 Settings keys

| Key | Meaning | Default |
| --- | --- | --- |
| `usage_limit_usd_cny_rate` | Local USD→CNY rate (reuses the V1.0 key, upgrades keep it) | `7.2` |
| `usage_limit_usd_eur_rate` | Local USD→EUR rate | `0.92` |
| `usage_limit_usd_jpy_rate` | Local USD→JPY rate | `150` |
| `usage_limit_usd_gbp_rate` | Local USD→GBP rate | `0.79` |

Corrupt stored rates fall back to that currency's default with a warning;
rates must be > 0 and < 1,000,000; USD is the internal pricing base — always
1 and not settable. Rates are edited in the dialog and persisted on save
(manual adjustment, no online API).

---

## 4. Core mechanics

### 4.1 Credential fingerprint and rotation

- `credential_fingerprint(api_key)` = lowercase SHA-256 hex (64 chars),
  irreversible.
- Resolution reuses the same `adapter.extract_auth` path as proxy forwarding,
  so the stored fingerprint and the forwarded one always come from the same
  source.
- Display is always masked: `mask_credential` → `sk-****ABCD` (≤8 chars →
  `****`).
- OAuth providers (codex_oauth / xai_oauth / managed account / copilot) have
  no static key → `unsupported_credential`; per-key limits are not offered.
- **Key rotation**: when the guard or a save detects a fingerprint mismatch,
  the config rebinds to the new fingerprint and the window resets to now. The
  old key's usage never carries over. The read path (status) detects rotation
  and reports usage for the current credential but **never writes back**
  (side-effect free).

### 4.2 Decision semantics

- Tokens: `used_normalized_total >= limit` → BLOCK (integer comparison, no
  precision concerns).
- Money: `limit_in_usd = limit_amount / rate` converts the cap back into USD
  for a same-domain `Decimal` comparison; display uses
  `used_in_currency = used_usd × rate`. `rate` is the USD→limit-currency rate
  (V1.0.2: five independent rates; USD is always 1; one code path).
- The fast-path margin is `max(1e-6, limit_in_usd × 1e-12)` — an absolute
  1e-6 USD alone would not cover f64 accumulation error for large caps
  (audit P2-3).
- BLOCK returns 429 (see §6) with zero upstream traffic; in the failover
  chain `BudgetExhausted` is NonRetryable and does not consume other
  providers' attempts.
- DB read/aggregation failure → fail-open (warn + allow), same philosophy as
  the circuit breaker's degradation; a decision failure must never block all
  traffic.
- **Overshoot is by design**: an already-forwarded request is allowed to
  finish; usage lands after the response, so the final request may exceed the
  cap slightly. Never interrupt an in-flight SSE stream.

### 4.3 Reset schedule and lazy rollover (V1.0.1, reworked in V1.2.0)

**Window start (since V1.2.0)**: on first creation, key rotation, **off→on transition**, or **changing the reset configuration while enabled**, `usage_start_at` is set to "now" — counting starts when the option is turned on; earlier usage is never pulled in. The save path no longer back-aligns to calendar boundaries (the V1.0.1 daily back-alignment behavior was removed).

`ResetPeriod`：
| Period | Rollover | Example (enabled Fri 2026-09-18 15:27) |
| --- | --- | --- |
| never | none, manual reset only | — |
| hourly | local top-of-hour boundary | starts 15:27, resets 16:00 |
| daily | local midnight boundary | starts 15:27, resets next 00:00 |
| weekly | local Monday midnight (ISO week) | until next Monday |
| monthly | local midnight on the 1st | until the 1st of next month |
| **custom** (V1.2.0) | **anchor + whole multiples of the window** (N hours/days) | starts 15:27, rolls every N hours/days |

- Calendar periods: effective start = `max(usage_start_at, current boundary)`
  (stays at the enable moment until the next boundary, then rolls).
- Custom: effective start = `anchor + floor((now-anchor)/window) × window`;
  the length lives in `window_length` (1–10000) + `window_unit`
  (hours/days), only allowed for custom.
- Unified entry points `effective_window_start(row, now)` /
  `next_window_reset(...)`.

Three application points:
1. **guard (write path)**: rollover first, decide second; the rollover is
   persisted with a single UPDATE (best-effort — a failure only warns, the
   decision still uses the new start). A spent window recovers within the
   same guard call once the boundary is crossed.
2. **status (read path)**: computes the same effective start and returns
   `resetPeriod` / `windowLength` / `windowUnit` / `nextResetAt` but **never
   writes back** — reads stay side-effect free.
3. **save (write path, V1.2.0)**: a missing period means `never`; invalid
   values are rejected. The window start follows the four cases above
   (off→on and reset-config changes → now); no back-alignment to calendar
   boundaries. Custom length/unit are strictly validated (1–10000,
   hours/days); non-custom configs must not carry window fields.


### 4.4 Concurrency model

Decision reads and usage writes are serialized under `Database.conn`'s
mutex — no lost updates from read→write races (unit test: 8 threads × 25
concurrent inserts aggregate exactly to the total). Real usage is only known
after a response, so concurrent requests can still overshoot slightly — by
design; forwarding concurrency is never sacrificed for theoretical zero
overshoot. Period rollovers write under the same mutex, so there is no extra
race surface.

### 4.5 Unknown pricing (money-mode reliability)

For requests in the window with token usage but `total_cost_usd = '0'`, each
`pricing_model` is checked against the pricing table: **priced with zero
cost** (free model) → fine; **unpriced and not a placeholder** → counted in
`unpricedRequestCount` / `unpricedModels`, and the UI shows a warning banner
pointing at custom pricing. Unpriced requests aggregate as 0 cost, but the UI
**never pretends the hard limit precisely covers them**.

---

## 5. State machine and UI

`BudgetStatus.state` (computed by the backend; the frontend only renders):

| state | condition | icon | progress bar |
| --- | --- | --- | --- |
| off | disabled or no config | muted gray | — |
| active | enabled and < 80% | emerald | emerald |
| warning | ≥ 80% and < 100% | amber | amber |
| exhausted | ≥ 100% | red | red (value is not clamped; the bar visually caps at 100%) |

- `percent_used` is not clamped (may show 104.2%).
- Enforcement is three-valued: `active` / `proxy_disabled` (the app's proxy
  takeover is off; a warning banner is shown) / `unsupported_credential`
  (OAuth; configuration is refused).
- Dialog layout: when off, only the toggle row (concise); when on, everything
  expands inside the same dialog (no second dialog): limit type →
  currency/rate (money) → token input → window input → **reset schedule six-segment
  selector (3×2 grid, adds custom since V1.2.0)** → custom window length
  input with hours/days segments → enforcement/unknown-pricing warnings → enforcement/unknown-pricing warnings → used,
  remaining, progress → [Reset usage] [Cancel] [Save].
- "Next reset: <local time>" is shown only while the selected schedule equals
  the **saved** one, avoiding a stale boundary while switching.
- **Card usage badge (V1.2.0)**: a display toggle (Eye/EyeOff, persisted
  per provider in localStorage — UI preference only) next to the gauge icon;
  when on, the card shows `{percent}%` (state-colored) + `⏱{time to next
  reset}` (`formatDurationUntil`: 38m / 5h12m / 2d4h); hidden automatically
  when the limit is off or no percent exists.
- **Hover hints (V1.2.1)**: every button in the card icon row (including the
  eye = badge toggle and the gauge = limit settings) explains itself via a
  Radix Tooltip on hover (`TipButton` in ProviderActions, span-wrapped for
  disabled states); replaces the unreliable native `title` in WKWebView.
- **Anchored dialog positioning (V1.2.3 — the final fix after two failed
  max-h rounds)**: DialogContent uses **inline style** `position:fixed;
  top:2.5rem; bottom:0.75rem; left:50%; transform:translateX(-50%);
  maxHeight:none` — top/bottom double anchoring pins the box to
  window-height − 52px, making it mathematically impossible to overflow the
  window. **Lessons**: ① tailwind-merge does NOT dedupe negative arbitrary
  values — the shared `translate-y-[-50%]` survived alongside the override
  class and won the CSS cascade, cancelling every max-h fix (the common root
  of both the V1.2.1 dvh failure and the V1.2.2 vh failure); ② inline styles
  are the only reliable way to override component-library positioning.
  Custom-window row input (`min-w-0 flex-1`) plus hours/days segments
  (`w-28 shrink-0`) no longer overflow in narrow dialogs.
- **Nested radii (V1.2.2)**: the scroll container got `px-4 py-1` so the
  glass cards (rounded-xl) sit inset from the outer frame (rounded-lg) —
  the 6px⊃12px radius difference no longer reads as "rounded inside, straight
  line outside".
- **Scroll container `space-y-4` (V1.2.3)**: separates the enable card and
  the config card (user feedback: too close together).
- Live refresh: `refetchOnMount: "always"` plus `useUsageLimitEventBridge`
  listening for `usage-log-recorded` to invalidate the `usage-limit`
  namespace (cards and the dialog update right after accounting); mutations
  invalidate their queries on success. No `window.location.reload()`, no
  high-frequency polling.

---

## 6. Error response (429)

`ProxyError::BudgetExhausted { message, detail }` →
`StatusCode::TOO_MANY_REQUESTS` with a body following the existing
`{"error": {...}}` convention:

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

Money/token values in `message` are human-readable (`$10.18` / `5,013,241`);
exact values live in the detail fields and the status query. Never contains
an API key (only the provider name and usage numbers).

---

## 7. API inventory

### Tauri Commands (`commands/usage_limit.rs`)

| Command | Signature notes |
| --- | --- |
| `get_usage_limit_status(provider_id, app_type)` | → `BudgetStatus` |
| `save_usage_limit(provider_id, app_type, config)` | config includes `resetPeriod`; → latest `BudgetStatus` after saving |
| `reset_usage_limit(provider_id, app_type)` | advances the window to now; → latest status |
| `get_exchange_rate(currency)` / `set_exchange_rate(currency, rate)` | local USD→X rate read/write (USD returns "1" / refuses to be set) |

### Frontend query keys (`lib/query/usageLimit.ts`)

```
["usage-limit", "status", providerId, appType]
["usage-limit", "exchange-rate", currency]
```

Hooks: `useUsageLimitStatus` (refetchOnMount always), `useSaveUsageLimit`,
`useResetUsageLimit`, `useExchangeRate(currency)`, `useSetExchangeRate`
(invalidates the whole usage-limit namespace on success).

### `BudgetStatus` fields (camelCase serde)

`providerId` `appType` `enabled` `limitType` `currency` `limitAmount`
`usageStartAt` (effective window start) **`resetPeriod` `nextResetAt`
(V1.0.1)** `usedMoneyUsd` `usedMoneyInCurrency` `usedTokens` `percentUsed`
`state` `unpricedRequestCount` `unpricedModels` `enforcement`
`maskedCredential`.

---

## 8. i18n (usageLimit namespace, 4 locales × 39 keys)

V1.0.1 added 8 keys; V1.0.2 added 3 currency labels (eur/jpy/gbp) and
parameterized the rate copy (`exchangeRate: "USD → {{currency}} rate"`,
currency-neutral hint); the remaining 28 are from V1.0:

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

Maintenance rule: new keys must be added to all 4 locales in the same change
(no CI or test enforces locale completeness — review is the gate); hardcoded
Chinese in components is forbidden.

---

## 9. Test index

| Layer | Location | Coverage |
| --- | --- | --- |
| Rust unit (41) | `services/usage_limit/tests.rs` | disabled/below/reached/exceeded, reset, key rotation, rates, precision, unknown pricing, concurrency, validation, fingerprint/masking, save semantics, state machine, enforcement; V1.0.1: boundary calendar, strictly-future next, parse domain, four-period rollover recovery, never regression, save validation/schedule alignment, corrupt-data fallback, manual reset coexistence, fresh-DDL default; V1.0.2: five default rates independent, EUR/JPY/GBP conversion decisions, five currencies accepted & unknown rejected, custom-rate end-to-end threshold, FK=ON migration end-to-end, corrupt stored rate fallback |
| Proxy integration | `proxy/forwarder.rs` tests | Case 1 token 1100/1000 BLOCK, Case 2 money $1.02 no upstream call, Case 4 unconfigured/disabled behavior unchanged, 429 mapping; V1.0.1 `budget_recovers_after_period_boundary_rollover` (rollover allow + persistence + control block) |
| Accounting path | `proxy/usage/logger.rs` tests | fingerprinted usage stored after streaming → guard blocks immediately |
| Migrations | `database/tests.rs` | v0→…→v22 chained migrations, column defaults, v22 rebuild preserves data / accepts EUR / keeps limit_type CHECK, fresh table has no currency CHECK |
| Frontend (31) | `tests/components/UsageLimitDialog.test.tsx` | OFF default, expand, CNY/USD switch, tokens, invalid input, progress, exhausted unclamped, reset confirmation, API errors; V1.0.1: schedule selector rendering, never default + hint, save carries schedule, next-reset shown/hidden, reopening keeps schedule; V1.0.2: five currencies rendered, EUR rate input + conversion + € suffix, EUR save persists rate, USD never touches rates, switching refills that currency's rate, saving blocked while rate not loaded (P1-1 race regression) |

Test discipline: fake keys are always `sk-test-...`; schedule tests build
expected values with the same local-timezone helpers
(`Local.with_ymd_and_hms`) instead of hardcoded UTC timestamps.

Environment note: `proxy::server::tests` / `proxy::hyper_client::tests` are
network-bound integration tests (real sockets/upstream) and hang in an
offline sandbox — a pre-existing environment limitation, unrelated to the
limit feature; the budget decision path's integration coverage lives in
`proxy::forwarder::tests` (network-free) and runs in full.

---

## 10. FAQ / troubleshooting

**Q: The UI shows 100% but requests are not blocked?**
Check `enforcement` first: `proxy_disabled` = the app's proxy takeover is off
so traffic bypasses CC Switch (a V1 boundary, not a bug);
`unsupported_credential` = OAuth without a static key. Both show a warning
banner in the UI.

**Q: Money mode disagrees with the Usage Dashboard?**
They share one source (`proxy_request_logs.total_cost_usd`). The budget only
counts the window (`created_at >= effective window start`) while the
dashboard may cover a wider range — with identical ranges the numbers always
match. If not: ① check whether rows with `data_source != 'proxy'` slipped in
(they must not); ② check whether the window start was just rolled over.

**Q: I chose daily resets — why is earlier-today usage still counted?**
The save path is conservative: a window already inside the current period is
not rewound (this period's accounting is kept) and strict per-day accounting
starts at the next midnight. To zero out immediately, click "Reset usage".

**Q: Do requests recover right after the boundary?**
Yes. The guard rolls over before deciding; once the boundary passes, old
usage leaves the window (covered by
`budget_recovers_after_period_boundary_rollover`).

**Q: Does the limit survive an API key change?**
The config stays (one row per provider); the fingerprint rebinds and the
window resets, so the new key starts from 0.

**Q: Why does the rate input go blank when I switch currencies?**
By design (audit fix P1-1): the previous currency's rate must not linger
(otherwise 7.2 for CNY could be saved as the JPY rate). The selected
currency's saved rate refills automatically — or its default on first use —
and until then saving is blocked by validation, so a wrong rate can never be
persisted.

**Q: macOS says "cannot verify developer" or "damaged" on first launch?**
- Since v1.1.1 the app bundle carries a full ad-hoc signature, so the
  "damaged" warning is gone; because the plugin is not notarized, the first
  launch shows a one-time "cannot verify developer" prompt — **right-click
  the app → Open → Open**, or run `xattr -dr com.apple.quarantine
  "/Applications/CC Switch.app"` once.
- Installers from v1.1.0 and earlier lack the bundle signature (they trigger
  the "damaged" warning); always use v1.1.1 or later.

**Q: How do I cut a new release?**
1. `pnpm tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}'`
   (skips updater artifacts when no signing key is present; the build needs
   cargo on PATH);
2. rename the DMG to `CC.Switch_<base version>_aarch64_usage-limit.dmg`;
3. `git archive --format=zip <tag>` for the source snapshot (named after the
   repo);
4. `gh release create <tag> --notes-file <4-language body>` with both assets.
The body template lives in `docs/release-notes/usage-limit-v1.1.0.md`.

**Q: Why did CI break after the repository was renamed?**
The `src-tauri/target` cache restored by `actions/cache` contains Tauri build
script outputs with absolute paths of the old workspace; after a rename they
always mismatch ("failed to read plugin permissions"). Fix:
`gh cache delete --all` and rerun — the restore-keys prefix fallback would
bring the poisoned cache back, so deleting is mandatory.

**Q: Why does `api_key_limits.reset_period` have no CHECK constraint?**
So the ALTER ADD COLUMN migration and the fresh DDL behave identically; the
value domain is validated on save and unknown stored values fall back to
never (`parse_reset_period` warns).

**Q: Is the limit config cloud-synced / exported?**
No (V1 decision: usage is local observation; the config is a local decision),
avoiding cross-device double counting. When a provider is deleted, the config
row is cleaned up via FK CASCADE.

---

## 11. Known limitations

1. **Only traffic through this machine's local proxy** — not the provider
   account's global quota; other devices / direct usage is invisible.
2. Money mode depends on model pricing; unpriced requests aggregate as 0 with
   an explicit warning (never silently treated as the whole picture).
3. Allowed requests finish → overshoot by design.
4. OAuth providers cannot have per-key limits.
5. Period boundaries follow the device's current timezone; "daily" follows
   the new timezone's midnight after travel.
6. DST switch days carry ±1h wall-clock ambiguity (see §4.3); never
   directional.
7. Config and usage are excluded from Cloud Sync / import-export.
8. Rates are static local values: manual, offline, no market tracking;
   cross-currency comparisons should use the USD source
   (`usedMoneyUsd`).
9. JPY displays as `JP¥` alongside CNY's `¥` (country prefix for
   disambiguation).

---

## 12. File inventory

| File | Responsibility |
| --- | --- |
| `src-tauri/src/services/usage_limit.rs` | Domain service: fingerprint/masking, credential resolution, ResetPeriod and boundaries, guard (`check_budget_before_forward`), status, save (validation), exchange rates |
| `src-tauri/src/services/usage_limit/tests.rs` | Domain unit tests (41) |
| `src-tauri/src/database/dao/usage_limit.rs` | DAO: config CRUD, window reset/rebind, SSOT aggregation |
| `src-tauri/src/database/schema.rs` | `api_key_limits` DDL + `migrate_v20_to_v21` / `migrate_v21_to_v22` + dispatch |
| `src-tauri/src/database/mod.rs` | `SCHEMA_VERSION = 22` |
| `src-tauri/src/commands/usage_limit.rs` | 5 Tauri commands |
| `src-tauri/src/proxy/forwarder.rs` | Budget Guard wiring (per attempt) |
| `src-tauri/src/proxy/error.rs` | `BudgetExhausted` → structured 429 body |
| `src-tauri/src/proxy/usage/logger.rs` | Accounting stores `credential_fingerprint` |
| `src/types/usageLimit.ts` | Types (incl. `UsageLimitResetPeriod`) |
| `src/lib/api/usageLimit.ts` | invoke wrappers |
| `src/lib/query/usageLimit.ts` | Query keys + hooks |
| `src/components/usage-limit/UsageLimitButton.tsx` | Card icon, four states + compact usage |
| `src/components/usage-limit/UsageLimitDialog.tsx` | Dialog (config/progress/schedule/next reset) |
| `src/hooks/useUsageEventBridge.ts` | Accounting event → invalidate (main UI refresh) |
| `src/i18n/locales/{zh,zh-TW,en,ja}.json` | `usageLimit` namespace × 39 keys |
| `tests/components/UsageLimitDialog.test.tsx` | Frontend tests (31) |
| `docs/release-notes/usage-limit-v1.1.{0,1}.md` | Per-version 4-language release bodies (mirrored on the GitHub Releases) |
| `docs/usage-limit-knowledge-base-{zh,en,zh-TW,ja}.md` | The four language editions of this knowledge base |

---

## 13. Version history

| Version | Date | Content |
| --- | --- | --- |
| V1.0 | 2026-09-17 | Money/token limits, proxy enforcement, fingerprint binding, manual reset, four-state icon, i18n, end-to-end tests |
| V1.0.1 | 2026-09-18 | Reset schedule (never/hourly/daily/weekly/monthly, local-time lazy rollover), "next reset" display, schema v21, boundary-recovery integration test |
| V1.0.2 | 2026-09-18 | Money limits extended to 5 currencies (USD/CNY/EUR/JPY/GBP) with per-currency manual rates, generalized rate commands, schema v22 (currency CHECK removed), currency-symbol suffix and other UI polish |
| **V1.1.0** | 2026-09-18 | **Public release**: everything from V1.0 + V1.0.1 + V1.0.2 shipped as one version (base CC Switch 3.20.3) — the first public release after v1.0.0 |
| **V1.1.1** | 2026-09-18 | **Installer fixes**: proper ad-hoc bundle signature (fixes "damaged" warning and first-drag registration), stray `.VolumeIcon.icns` removed from the DMG, first-launch approval guidance added to README/notes. Functionality identical to V1.1.0 |
| **V1.2.0** | 2026-09-19 | Window semantics reworked (off→on and reset-config changes start from now; calendar back-alignment removed) + custom rolling windows (N hours/days, schema v23) + dialog scroll/drag fixes + card usage badge |
| **V1.2.1** | 2026-09-19 | UI polish: dialog height constraint (title no longer pushed out), custom-window row overflow, hover hints across the card icon row |
| **V1.2.2** | 2026-09-19 | Dialog layout final fix: max-h switched to vh (dvh failing on old WKWebView was why V1.2.1 didn't work) + glass card inset for nested radii |
| **V1.2.3** | 2026-09-19 | Dialog switched to inline top/bottom anchored positioning (root cause proven: tailwind-merge keeps the negative translate-y, cancelling max-h fixes), enable/config cards spaced apart |
