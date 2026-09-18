<div align="center">

# CC Switch Usage Plugin

### Add per-API-Key usage limits (budget) to CC Switch — enforced by the local proxy

[![Release](https://img.shields.io/github/v/release/Xhoryon/ccswitch-usage-plugin?color=blue&label=release)](https://github.com/Xhoryon/ccswitch-usage-plugin/releases)
[![Platform](https://img.shields.io/badge/platform-macOS%20Apple%20Silicon-lightgrey.svg)](https://github.com/Xhoryon/ccswitch-usage-plugin/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

[English](README.md) | [简体中文](README_ZH.md) | [繁體中文](README_ZH-TW.md) | [日本語](README_JA.md)

</div>

---

## What is this?

**CC Switch Usage Plugin** extends the open-source provider switcher
[CC Switch](https://github.com/farion1231/cc-switch) with a budget feature:
every API Key can get a **usage cap** — a money amount (USD / CNY) or a token
count. Once the cap is reached, the CC Switch local proxy **rejects new
requests** with a clear error instead of just tracking usage.

## Features

- **Money limit** — USD or CNY, converted with a configurable local
  USD→CNY exchange rate (default 7.2, no online rate API)
- **Token limit** — uses the same normalized token counting as the built-in
  Usage Dashboard, so both always agree
- **Real enforcement** — the proxy checks the budget _before_ forwarding; once
  the limit is reached, new requests fail fast with a structured
  `usage_limit_reached` error (HTTP 429). Requests that were already sent are
  allowed to finish, so a small overshoot on the final request is normal
- **Bound to the credential, not the provider name** — limits bind to a
  SHA-256 fingerprint of the API key; plaintext keys are never stored, and
  rotating a key automatically starts a fresh budget
- **Live status on every card** — a gauge icon with four states
  (off / active / warning ≥80% / exhausted), a config dialog with progress
  bar, remaining budget, and usage reset (statistics window moves forward,
  history is never deleted)
- **Honest by default** — if traffic bypasses the proxy or a model has no
  price, the UI says so explicitly instead of pretending the limit works
- **Languages** — English, 简体中文, 繁體中文, 日本語

## Install

1. Grab `CC.Switch_3.20.3_aarch64_usage-limit.dmg` from the
   [Releases](https://github.com/Xhoryon/ccswitch-usage-plugin/releases) page
   (macOS, Apple Silicon).
2. Mount the DMG and drag **CC Switch.app** into Applications.

## Quick start

1. Open CC Switch and switch to **Claude / Codex / Gemini / Grok Build**.
2. Hover a provider card and click the **gauge icon** (Usage Limit).
3. Flip the toggle on, pick **money** or **tokens**, enter a cap, save.
4. Enable the local proxy takeover for that app — the limit is now enforced.
5. "Reset usage" moves the statistics window forward; historical usage in the
   Usage Dashboard stays intact.

## Important boundaries

- Limits only apply to traffic that goes **through the CC Switch local
  proxy**. Direct connections cannot be observed or blocked (the UI tells you
  when enforcement is unavailable).
- Usage is **local observation** — this is not the provider's global quota
  across all your devices.
- If a model has no price configured, its cost counts as $0 **and** the dialog
  shows an explicit warning; you can add custom per-model pricing in CC Switch.

## Build from source

```bash
pnpm install
pnpm build        # requires a working Rust toolchain (stable)
```

Architecture notes and known limitations (in Chinese):
[docs/development_log.md](docs/development_log.md).

## Credits

This project is built on top of
[**CC Switch**](https://github.com/farion1231/cc-switch) by
[@farion1231](https://github.com/farion1231) (MIT) — an excellent All-in-One
manager for Claude Code, Codex, Gemini CLI and more. All credit for the
underlying switching/proxy/usage infrastructure goes to the upstream author
and contributors. Upstream website: [ccswitch.io](https://ccswitch.io).

The usage limit feature itself was developed in this repository.

## License

[MIT](LICENSE) © 2026 Jiayi Huang — includes the original CC Switch MIT
notice.
