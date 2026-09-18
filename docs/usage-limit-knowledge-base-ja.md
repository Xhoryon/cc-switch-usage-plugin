<!-- Language / 语言: [简体中文](usage-limit-knowledge-base-zh.md) | [English](usage-limit-knowledge-base-en.md) | [繁體中文](usage-limit-knowledge-base-zh-TW.md) | [日本語](usage-limit-knowledge-base-ja.md) -->

# 使用制限（Budget Limit）ナレッジベース

> 適用バージョン：V1.0.2（2026-09-18）。V1.0 の基本機能は
> `docs/development_log.md` の 2026-09-17 エントリ、V1.0.1 のリセット周期と
> V1.0.2 の多通貨対応は 2026-09-18 のエントリを参照。本書は三者を統合した
> ビューであり、今後のメンテナに向けて「何か、なぜこの設計か、どこを変更する
> ときに注意すべきか」を答える。

---

## 1. 機能概要

各 Provider の静的 API Key には個別の使用上限を設定でき、上限に達すると
**ローカルプロキシが転送前に新しいリクエストを拒否します**（統計だけの
「見かけの制限」ではなく、本物の enforcement）。

| 能力 | 説明 |
| --- | --- |
| 制限方式 | 金額（USD / CNY / EUR / JPY / GBP）またはトークン数。同時に選べるのは片方 |
| enforcement 位置 | ローカルプロキシ `forward_with_retry_inner` の provider attempt ごとの転送前 |
| 集計ソース | Usage SSOT（`proxy_request_logs`）を再利用。平行する統計系は作らない |
| credential 結び付け | SHA-256 の不可逆フィンガープリント。平文 API Key はどの経路にも保存しない |
| リセット周期（V1.0.1） | なし / 每時 / 毎日 / 毎週 / 毎月。ローカル時刻の境界を遅延ロールオーバー |
| 多通貨（V1.0.2） | USD 以外はローカルの adjustable レートで換算（手動調整・オフライン） |
| 手動リセット | `usage_start_at` を現在に進めるのみ。履歴は一切削除しない |

**核心的境界（機能と一緒に必ず伝えること）**：

> Budget enforcement only applies to traffic routed through CC Switch Local
> Proxy. プロキシ接管が有効でないトラフィックは観測も遮断もできません。UI は
> `enforcement = proxy_disabled` でそれを明示し、ハードリミットが機能している
> よりをしません。同じキーが他デバイス/直連で使った使用量は本機の予算に
> 含まれません（usage is local observation）。

---

## 2. 全体アーキテクチャ

```
設定/表示系
  Provider Card（ProviderActions のアイコンボタン）
    └─ UsageLimitButton（Gauge アイコン、useUsageLimitStatus で状態を取得）
         └─ UsageLimitDialog（トグル / 制限方式 / 通貨 / 金額 / リセット周期 / 進捗）
              └─ Tauri Command（get_usage_limit_status / save_usage_limit /
                 reset_usage_limit / get_exchange_rate / set_exchange_rate）
                   └─ Database impl（services/usage_limit.rs）
                        ├─ api_key_limits テーブル（設定 + ウィンドウ起点 + 周期）
                        └─ proxy_request_logs 集計（使用量。SSOT を再利用）

転送実行系
  Client Request
    └─ CC Switch ローカルプロキシ（forwarder.rs）
         └─ Budget Guard（provider attempt ごと、forward() の前）
              ├─ 無効 → 通過（オーバーヘッドは PK ルックアップ 1 回）
              ├─ credential フィンガープリント不一致 → 再結び付け + ウィンドウを現在にリセット
              ├─ 周期境界を跨いだ（V1.0.1）→ ウィンドウ起点を遅延ロールオーバー
              └─ used >= limit → ProxyError::BudgetExhausted（429。上流へは送信しない）
                   └─ レスポンス完了後に Usage Accounting（log_with_calculation が
                      credential_fingerprint を proxy_request_logs に記録）
```

階層規律：React → Tauri API → Command → Service（Database impl）→ DAO →
SQLite。React が DB を直接触ることはない。金額/トークンの集計は Rust 側のみ。

---

## 3. データモデル

### 3.1 `api_key_limits`（V1.0 で作成、V1.0.1 で列追加）

```sql
CREATE TABLE api_key_limits (
    provider_id            TEXT NOT NULL,
    app_type               TEXT NOT NULL,
    credential_fingerprint TEXT NOT NULL,      -- SHA-256 hex。平文キーは保存しない
    enabled                INTEGER NOT NULL DEFAULT 0,
    limit_type             TEXT NOT NULL CHECK (limit_type IN ('money', 'token')),
    currency               TEXT,                    -- token モードは NULL。V1.0.2 から CHECK なし
    limit_amount           TEXT NOT NULL,      -- 金額は十進文字列 / トークンは正整數文字列
    usage_start_at         INTEGER NOT NULL,   -- 集計ウィンドウ起点（unix 秒）
    reset_period           TEXT NOT NULL DEFAULT 'never',  -- V1.0.1
    created_at             INTEGER NOT NULL,
    updated_at             INTEGER NOT NULL,
    PRIMARY KEY (provider_id, app_type),
    FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type)
        ON DELETE CASCADE                       -- Provider 削除時にカスケード削除
);
```

- Provider ごとに最大 1 行（PK が保証）。
- `limit_amount` は文字列。SQLite REAL の浮動小数誤差が限額判定に入らないようにする。
- `reset_period` は意図的に**CHECK なし**：ALTER ADD COLUMN の移行パスと新規
  作成 DDL の挙動を完全に一致させるため。値域は保存経路で検証し、読み取り側の
  不明値は `never` にフォールバックして警告。
- `currency` の CHECK は v22 マイグレーションで**削除**（V1.0.2 多通貨）：
  SQLite は既存 CHECK を変更できないため、標準的なテーブル再構築
  （新テーブル作成 → コピー → 旧テーブル削除 → リネーム。冪等性は
  `sqlite_master.sql` がまだ `CHECK (currency` を含むかで判定）で対応。値域は
  保存経路の `LimitCurrency::parse` が検証し、読み取り側の不明値は USD に
  フォールバック。**今後の通貨追加にマイグレーションは不要**。
- 本テーブルは**累計使用量を冗長保存しない**（二重 SSOT の回避）。使用量は
  常に `proxy_request_logs` から即時集計する。

### 3.2 使用量 SSOT の集計述語

すべての予算集計は同じ WHERE を共有する：

```sql
WHERE provider_id = ? AND app_type = ?
  AND credential_fingerprint = ?
  AND created_at >= usage_start_at   -- 有効ウィンドウ起点（§4.3 参照）
  AND data_source = 'proxy'          -- セッション同期行/直連行は予算に含めない
```

- **トークン**：normalized total = fresh input + output + cache_creation +
  cache_read（`services/sql_helpers::fresh_input_sql`。Usage Dashboard と同じ
  規則。`input_token_semantics` がキャッシュの意味論を区別）。
- **金額**：保存済みの `total_cost_usd` 十進文字列を合算。二段構え：
  - 高速経路：`SUM(CAST(... AS REAL))` が閾値より十分低ければ（相対マージン、
    §4.2 参照）O(1) で通過；
  - 高精度経路：閾値付近・超過時に元の文字列を `rust_decimal` で行単位に
    再加算し、0.1+0.2 ≠ 0.3 の浮動小数誤差を回避。

### 3.3 settings キー

| キー | 意味 | デフォルト |
| --- | --- | --- |
| `usage_limit_usd_cny_rate` | ローカル USD→CNY レート（V1.0 の旧キーを継続。アップグレードで失われない） | `7.2` |
| `usage_limit_usd_eur_rate` | ローカル USD→EUR レート | `0.92` |
| `usage_limit_usd_jpy_rate` | ローカル USD→JPY レート | `150` |
| `usage_limit_usd_gbp_rate` | ローカル USD→GBP レート | `0.79` |

不正な保存値はその通貨のデフォルトにフォールバックして警告。レートは
0 < rate < 1,000,000。USD は内部の基準通貨で常に 1、設定不可。レートは
ダイアログで編集し、保存時に永続化される（手動調整・オンライン取得なし）。

---

## 4. 中核メカニズム

### 4.1 credential フィンガープリントとローテーション

- `credential_fingerprint(api_key)` = SHA-256 小文字 hex（64 文字）。不可逆。
- 解析はプロキシ転送と同じ `adapter.extract_auth` を再利用し、「設定に保存した
  フィンガープリント」と「実際に転送するフィンガープリント」が同じ経路から
  得られることを保証。
- 表示は常にマスク：`mask_credential` → `sk-****ABCD`（8 文字以下は `****`）。
- OAuth 系 Provider（codex_oauth / xai_oauth / managed account / copilot）は
  静的キーが無い → `unsupported_credential`。キー単位の制限は提供しない。
- **キー交換**：guard / 保存時にフィンガープリント不一致を検出 → 新フィンガー
  プリントに再結び付け + ウィンドウ起点を現在にリセット。旧キーの使用量が
  新キーに引き継がれることはない。読み取り系（status）はローテーションを
  検出すると「現在の credential」で集計するが**書き戻さない**（副作用なし）。

### 4.2 判定セマンティクス

- トークン：`used_normalized_total >= limit` → BLOCK（整數比較。精度問題なし）。
- 金額：`limit_in_usd = limit_amount / rate` で USD ドメインに折り返して
  `Decimal` 比較。表示は `used_in_currency = used_usd × rate`。`rate` は
  USD→制限通貨のレート（V1.0.2：5 通貨それぞれ独立。USD は常に 1。単一コード
  パス）。
- 高速経路のマージンは `max(1e-6, limit_in_usd × 1e-12)`——大きな上限では
  絶対値 1e-6 USD では f64 の累積誤差をカバーできない（レビュー P2-3）。
- BLOCK は 429 を返す（§6 参照）。上流へのトラフィックはゼロ。failover
  チェーンで `BudgetExhausted` は NonRetryable であり、他 Provider の試行を
  消費しない。
- DB の読み取り/集計失敗 → fail-open（warn + 通過）。サーキットブレーカーの
  縮退と同じ発想。判定失敗で全トラフィックを止めてはいけない。
- **Overshoot は仕様内**：転送を許可したリクエストは完了させ、usage は
  レスポンス後に記録される。最後のリクエストで僅かに超える可能性がある。
  進行中の SSE/ストリームを中断することは禁止。

### 4.3 リセット周期と遅延ロールオーバー（V1.0.1）

`ResetPeriod::current_period_start(now)` は**ローカルタイムゾーン**で現在の
周期境界を返す：

| 周期 | 境界 | 例（2026-09-18 金 15:27） |
| --- | --- | --- |
| never | なし | — |
| hourly | ローカルの整時 | 15:00 |
| daily | ローカルの零時 | 09-18 00:00 |
| weekly | ローカルの月曜零時（ISO 週） | 09-14 00:00 |
| monthly | ローカルの 1 日零時 | 09-01 00:00 |

有効ウィンドウ起点 = `max(usage_start_at, 現在の周期境界)`。適用点は 3 つ：

1. **guard（書き込み経路）**：先にロールオーバー、後に判定。ロールオーバーは
   1 行の UPDATE で永続化（best-effort。失敗時は警告のみで判定は新しい起点を
   使う）。制限にかかったウィンドウは境界を跨いだ瞬間、同じ guard 呼び出し内で
   解除される。
2. **status（読み取り経路）**：同じ規則で即時計算し `resetPeriod` /
   `nextResetAt`（次の境界）を返すが**書き戻さない**——読み取りは副作用なし。
3. **save（書き込み経路）**：周期の省略時は `never`。不正値は保存を拒否。
   元のウィンドウが新しい周期境界より前なら（例：never→daily でウィンドウが
   昨日）即座に境界へ整列。既に現周期内なら**巻き戻さない**（現周期の集計を
   保守的に維持）。

遅延ロールオーバー vs タイマー：アプリ停止中に跨いだ境界も次回起動後に通常
通り効く。並行タイマー処理なし。判定は元々 DB Mutex 下で直列化されており、
ロールオーバーは 1 行の UPDATE に過ぎない。

DST 対応（`usage_rollup::compute_local_midnight_cutoff` と同じ方式）：
牆時計の曖昧時刻（fall-back の重複時刻）は早い方を採用。存在しない牆時計
時刻（spring-forward の gap）は 1 時間後に再試行。最終的に UTC 解釈へ
フォールバック。`next_period_start` は結果が厳密に now より未来であることを
要求し、そうでなければ None を返す（「次のリセット」の表示は控える）。

手動リセットと周期は独立：手動リセットはウィンドウを現在へ進めて即ゼロに
する。次の周期境界では通常通りロールオーバーする。リセットは
`proxy_request_logs` の履歴行を**決して削除しない**。

### 4.4 並行モデル

判定の読み取りと使用量の書き込みは `Database.conn` の Mutex 下で直列化され、
read→write 競合による書き込み欠落がない（単体テスト：8 スレッド × 25 行の
並行書き込み後、集計値 == 書き込み総量）。実際の使用量はレスポンス後にしか
分からないため、同時リクエストでも僅かな overshoot は起こり得る——仕様内。
理論上のゼロ overshoot のために転送の並行性を犠牲にしない。周期ロール
オーバーの書き込みも同じ Mutex 下にあり、追加の競合面はない。

### 4.5 未知の価格（金額モードの信頼性）

ウィンドウ内の「トークン使用量があるが `total_cost_usd = '0'`」のリクエストを
`pricing_model` ごとに価格テーブルと突き合わせる：**価格登録ありでコスト 0**
（無料モデル）→ 正常。**未登録かつプレースホルダでない** →
`unpricedRequestCount` / `unpricedModels` に計上し、UI に警告バナーを表示して
custom pricing へ誘導。未登録のリクエストはコスト 0 で集計されるが、ハード
リミットがそれらを正確にカバーすると**見せかけることは決してしない**
（UI が明示）。

---

## 5. ステートマシンと UI

`BudgetStatus.state`（バックエンドが計算し、フロントエンドは描画のみ）：

| state | 条件 | アイコン | 進捗バー |
| --- | --- | --- | --- |
| off | 無効または設定なし | muted グレー | — |
| active | 有効かつ < 80% | emerald | emerald |
| warning | ≥ 80% かつ < 100% | amber | amber |
| exhausted | ≥ 100% | red | red（数値は clamp しない。バーの見た目は 100% で封頂） |

- `percent_used` は clamp しない（104.2% を表示し得る）。
- enforcement は 3 値：`active` / `proxy_disabled`（そのアプリのプロキシ接管が
  無効。警告バナー表示）/ `unsupported_credential`（OAuth。設定を拒否）。
- Dialog 構成：オフ時はトグル行のみ（簡潔）。オンで同じ Dialog 内に展開
  （二重ダイアログにしない）：制限方式 → 通貨/レート（money）/ トークン入力 →
  **リセット周期の 5 セグメント選択（V1.0.1）** → enforcement/未知価格の警告 →
  使用量/残額/進捗 → [使用量をリセット] [キャンセル] [保存]。
- 「次のリセット：<ローカル時刻>」は選択中の周期が**保存済み**設定と一致する
  場合のみ表示。切替中に古い境界を表示しない。
- 即時更新：`refetchOnMount: "always"` + `useUsageLimitEventBridge` が
  `usage-log-recorded` イベントで `usage-limit` 名前空間を invalidate
  （記帳後にカード/Dialog が即時更新）。mutation 成功時は対応 query を
  invalidate。`window.location.reload()` や高頻度ポーリングはしない。

---

## 6. エラーレスポンス（429）

`ProxyError::BudgetExhausted { message, detail }` →
`StatusCode::TOO_MANY_REQUESTS` + 既存の `{"error": {...}}` 規約に沿った本文：

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

message 内の金額/トークンは人間可読形式（`$10.18` / `5,013,241`）。正確な値は
detail フィールドと status クエリを正とする。API Key は決して含まない
（Provider 名と使用量のみ）。

---

## 7. API 一覧

### Tauri Commands（`commands/usage_limit.rs`）

| コマンド | シグネチャの要点 |
| --- | --- |
| `get_usage_limit_status(provider_id, app_type)` | → `BudgetStatus` |
| `save_usage_limit(provider_id, app_type, config)` | config は `resetPeriod` を含む。→ 保存後の最新 `BudgetStatus` |
| `reset_usage_limit(provider_id, app_type)` | ウィンドウを現在へ進める。→ 最新状態 |
| `get_exchange_rate(currency)` / `set_exchange_rate(currency, rate)` | ローカル USD→X レートの読み書き（USD は "1" を返す / 設定を拒否） |

### フロントエンド Query Keys（`lib/query/usageLimit.ts`）

```
["usage-limit", "status", providerId, appType]
["usage-limit", "exchange-rate", currency]
```

Hooks：`useUsageLimitStatus`（refetchOnMount always）、`useSaveUsageLimit`、
`useResetUsageLimit`、`useExchangeRate(currency)`、`useSetExchangeRate`
（成功時 usage-limit 名前空間全体を invalidate）。

### `BudgetStatus` フィールド（camelCase serde）

`providerId` `appType` `enabled` `limitType` `currency` `limitAmount`
`usageStartAt`（有効ウィンドウ起点）**`resetPeriod` `nextResetAt`（V1.0.1）**
`usedMoneyUsd` `usedMoneyInCurrency` `usedTokens` `percentUsed` `state`
`unpricedRequestCount` `unpricedModels` `enforcement` `maskedCredential`。

---

## 8. i18n（usageLimit 名前空間、4 locale × 39 keys）

V1.0.1 で 8 個、V1.0.2 で通貨ラベル 3 個（eur/jpy/gbp）を追加し、レート文言を
パラメータ化（`exchangeRate: "USD → {{currency}} レート"`。ヒントは通貨中立）。
残り 28 個は V1.0 から：

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

保守ルール：新しい key は 4 locale すべてに同じ変更で追加すること（CI/テスト
に locale 完全性の保証はなく、レビューが門番）。コンポーネントへの中国語
ハードコーディングは禁止。

---

## 9. テスト索引

| 層 | 位置 | カバレッジ |
| --- | --- | --- |
| Rust 単体（41） | `services/usage_limit/tests.rs` | 無効/未達/到達/超過、リセット、キー交換、レート、精度、未知価格、並行、検証、フィンガープリント/マスキング、保存セマンティクス、ステートマシン、enforcement。V1.0.1：境界カレンダー、next の厳密な未来、parse 値域、4 周期のロールオーバー回復、never リグレッション、保存検証/切替整列、破損データフォールバック、手動リセット共存、新規 DDL の既定値。V1.0.2：5 通貨の既定レート独立性、EUR/JPY/GBP 換算判定、5 通貨の保存可と未知通貨の拒否、非既定レートのエンドツーエンド遮断、FK=ON マイグレーション、破損レートのフォールバック |
| プロキシ統合 | `proxy/forwarder.rs` tests | Case 1 token 1100/1000 BLOCK、Case 2 money $1.02 で上流送信なし、Case 4 未設定/無効で挙動不変、429 マッピング。V1.0.1 `budget_recovers_after_period_boundary_rollover`（ロールオーバーで通過 + 永続化 + 対照で遮断） |
| 記帳系 | `proxy/usage/logger.rs` tests | ストリーミング完了後にフィンガープリント付きで記録 → guard が即遮断 |
| マイグレーション | `database/tests.rs` | v0→…→v22 連鎖マイグレーション、列既定値の整列、v22 再構築でデータ保持/EUR 投入可/limit_type CHECK 維持、新規テーブルに currency CHECK なし |
| フロントエンド（31） | `tests/components/UsageLimitDialog.test.tsx` | OFF 既定、展開、CNY/USD 切替、トークン、不正入力、進捗、exhausted 非 clamp、リセット確認、API エラー。V1.0.1：周期セレクタ描画、never 既定+ヒント、保存に周期を含む、次回リセットの表示/非表示、再オープンで周期保持。V1.0.2：5 通貨描画、EUR レート入力+換算+€ 接尾辞、EUR 保存でレート永続化、USD はレートに触れない、通貨切替でその通貨のレートを再取得、レート未ロード時は保存を遮断（P1-1 競合リグレッション） |

テスト規律：偽キーは常に `sk-test-...`。周期テストの期待値は同じローカル
タイムゾーンのヘルパー（`Local.with_ymd_and_hms`）で構築し、UTC タイムスタンプ
をハードコードしない。

環境注記：`proxy::server::tests` / `proxy::hyper_client::tests` はネットワーク
依存の統合テスト（実ソケット/上流）で、オフラインのサンドボックスでは停止する
——既存の環境制限であり、制限機能とは無関係。予算判定経路の統合カバレッジは
`proxy::forwarder::tests`（ネットワーク非依存）にあり、単独で全量実行できる。

---

## 10. FAQ / トラブルシュート

**Q：UI は 100% なのにリクエストが遮断されない？**
まず `enforcement` を確認：`proxy_disabled` = そのアプリのプロキシ接管が無効で
トラフィックが CC Switch を経由していない（V1 の明確な境界。バグではない）。
`unsupported_credential` = 静的キーのない OAuth。いずれも UI に警告バナーが
出る。

**Q：金額モードが Usage Dashboard と合わない？**
両者は同ソース（`proxy_request_logs.total_cost_usd`）。予算はウィンドウ内のみ
（`created_at >= 有効ウィンドウ起点`）を集計し、ダッシュボードはより広い範囲を
見るため範囲が違えば数値も違う。範囲が同じなら必ず一致する。食い違い時は
① `data_source != 'proxy'` の行が混入していないか（混入してはならない）、
② ウィンドウ起点が周期ロールオーバーされた直後か、を確認。

**Q：毎日リセットを設定したのに、当日の早い時間の使用量が計上されたまま？**
保存経路は保守的セマンティクス：元のウィンドウが現周期内なら巻き戻さず
（現周期の集計を維持）、次の零時から厳密に日単位。即時にゼロにするには
「使用量をリセット」を実行。

**Q：零点を跨いだらリクエストはすぐ復活する？**
はい。guard は先にロールオーバーしてから判定するため、境界を跨いだ瞬間に
旧使用量はウィンドウ外になる（
`budget_recovers_after_period_boundary_rollover` がカバー）。

**Q：API Key を交換しても制限は残る？**
設定は残る（Provider ごと 1 行）。フィンガープリントを再結び付けしウィンドウを
リセットするため、新しいキーは 0 から開始。

**Q：通貨を切り替えるとレート入力が空になるのはなぜ？**
仕様（レビュー修正 P1-1）：前の通貨のレートを残してはならない（CNY の 7.2 を
JPY のレートとして保存しかねない）。選択した通貨の保存済みレートが自動で
再取得され、初使用時は既定値が入る。取得が完了するまで保存は検証で遮断され、
誤ったレートが保存されることはない。

**Q：`api_key_limits.reset_period` に CHECK 制約がないのはなぜ？**
ALTER ADD COLUMN の移行パスと新規 DDL の挙動を完全に一致させるため。値域は
保存経路で検証し、読み取り側の不明値は never にフォールバック
（`parse_reset_period` が警告）。

**Q：制限設定はクラウド同期 / エクスポートされる？**
されない（V1 の決定：usage is local observation。設定はローカルの決定）。
デバイス間の二重計上を避ける。Provider 削除時は FK CASCADE で設定行も削除。

---

## 11. 既知の制限

1. **本機のローカルプロキシを経由するトラフィックのみ**——Provider アカウントの
   グローバル割り当てではない。他デバイス/直連の使用量は見えない。
2. 金額モードはモデル価格に依存。未登録のリクエストは 0 で集計し、明示的に
   警告する（黙って 0 として全容を偽装しない）。
3. 許可済みリクエストは完了する → 仕様内の overshoot。
4. OAuth Provider はキー単位の制限に対応しない。
5. 周期境界はデバイスの現在タイムゾーンに従う。移動後は「毎日」が新しい
   タイムゾーンの零時に追随。
6. DST 切替日は ±1 時間程度の牆時計の曖昧さがある（§4.3 参照）。方向性の
   誤りにはならない。
7. 設定と使用量はクラウド同期 / インポート・エクスポートの対象外。
8. レートはローカルの静的値：手動調整・オフライン・市場追従なし。通貨間比較は
   USD 原値（`usedMoneyUsd`）を基準に。
9. JPY の表示記号は `JP¥` で、CNY の `¥` と共存（国プレフィックスで曖昧さを回避）。

---

## 12. ファイル一覧

| ファイル | 責務 |
| --- | --- |
| `src-tauri/src/services/usage_limit.rs` | ドメインサービス：フィンガープリント/マスキング、credential 解析、ResetPeriod と周期境界、guard（`check_budget_before_forward`）、status、save（検証）、レート |
| `src-tauri/src/services/usage_limit/tests.rs` | ドメイン単体テスト（41） |
| `src-tauri/src/database/dao/usage_limit.rs` | DAO：設定 CRUD、ウィンドウリセット/再結び付け、SSOT 集計 |
| `src-tauri/src/database/schema.rs` | `api_key_limits` DDL + `migrate_v20_to_v21` / `migrate_v21_to_v22` + ディスパッチ |
| `src-tauri/src/database/mod.rs` | `SCHEMA_VERSION = 22` |
| `src-tauri/src/commands/usage_limit.rs` | 5 個の Tauri コマンド |
| `src-tauri/src/proxy/forwarder.rs` | Budget Guard の組み込み（attempt ごと） |
| `src-tauri/src/proxy/error.rs` | `BudgetExhausted` → 429 構造化エラー本文 |
| `src-tauri/src/proxy/usage/logger.rs` | `credential_fingerprint` 付きで記帳を保存 |
| `src/types/usageLimit.ts` | 型定義（`UsageLimitResetPeriod` 含む） |
| `src/lib/api/usageLimit.ts` | invoke ラッパー |
| `src/lib/query/usageLimit.ts` | Query keys + hooks |
| `src/components/usage-limit/UsageLimitButton.tsx` | カードアイコン 4 状態 + 簡略使用量 |
| `src/components/usage-limit/UsageLimitDialog.tsx` | Dialog（設定/進捗/リセット周期/次回リセット） |
| `src/hooks/useUsageEventBridge.ts` | 記帳イベント → invalidate（メイン UI の更新） |
| `src/i18n/locales/{zh,zh-TW,en,ja}.json` | `usageLimit` 名前空間 × 39 keys |
| `tests/components/UsageLimitDialog.test.tsx` | フロントエンドテスト（31） |

---

## 13. バージョン履歴

| バージョン | 日付 | 内容 |
| --- | --- | --- |
| V1.0 | 2026-09-17 | 金額/トークン制限、プロキシ enforcement、フィンガープリント結び付け、手動リセット、4 状態アイコン、i18n、全経路テスト |
| V1.0.1 | 2026-09-18 | リセット周期（never/hourly/daily/weekly/monthly、ローカル時刻の遅延ロールオーバー）、「次のリセット」表示、schema v21、周期回復の統合テスト |
| V1.0.2 | 2026-09-18 | 金額制限を 5 通貨（USD/CNY/EUR/JPY/GBP）に拡張 + 通貨別の手動レート、レートコマンドの汎化、schema v22（currency CHECK 削除）、通貨記号接尾辞などの UI 磨き |
| **V1.1.0** | 2026-09-18 | **公開リリース**：V1.0 + V1.0.1 + V1.0.2 の全内容を 1 バージョンとしてリリース（ベース CC Switch 3.20.3）。v1.0.0 次の最初の公開版 |
