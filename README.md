English | [中文](README_CN.md)

<div align="center">

<img src="docs/assets/fluxweir.png" alt="Fluxweir Logo" width="240">

[![Rust 2021](https://img.shields.io/badge/Rust-2021-edded?logo=rust&style=for-the-badge)](https://www.rust-lang.org/) [![Rustc 1.97.1+](https://img.shields.io/badge/Rustc-1.97.1+-orange.svg?style=for-the-badge)](https://www.rust-lang.org/) [![release](https://img.shields.io/github/v/release/Kirky-X/fluxweir?style=for-the-badge)](https://github.com/Kirky-X/fluxweir/releases) [![CI](https://img.shields.io/github/actions/workflow/status/Kirky-X/fluxweir/ci.yml?branch=main&label=CI&style=for-the-badge)](https://github.com/Kirky-X/fluxweir/actions/workflows/ci.yml) [![coverage](https://img.shields.io/codecov/c/github/Kirky-X/fluxweir?style=for-the-badge)](https://codecov.io/gh/Kirky-X/fluxweir) [![SQLite](https://img.shields.io/badge/SQLite-backend-003B57?logo=sqlite&logoColor=white&style=for-the-badge)](https://sqlite.org/) [![gRPC](https://img.shields.io/badge/gRPC-support-3C8CC9?logo=grpc&style=for-the-badge)](https://grpc.io/) [![Axum](https://img.shields.io/badge/Axum-REST-009688?style=for-the-badge)](https://docs.rs/axum) [![i18n](https://img.shields.io/badge/i18n-zh%20%7C%20en-2CA6E0?style=for-the-badge)](<>) [![Docker](https://img.shields.io/badge/Docker-ready-2496ED?logo=docker&logoColor=white&style=for-the-badge)](https://www.docker.com/) [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)

**Intelligent LLM routing gateway — unified multi-provider access · protocol conversion · resilient scheduling**

[✨ Features](#-features) • [🚀 Quick Start](#-quick-start) • [📚 Documentation](#-documentation) • [🐳 Docker Deployment](#-docker-deployment) • [🤝 Contributing](#-contributing)

</div>

---

<div align="center">

### 🎯 One endpoint, many providers

Clients call the gateway in either OpenAI or Anthropic format; the gateway handles auth, rate limiting, routing, protocol conversion and failover, abstracting multiple upstream LLM providers behind a single endpoint:

<table style="width:100%; border-collapse: collapse">
<tr>
<td align="center" width="25%">🔄<br><b>Protocol conversion</b><br><span style="color:#64748B">OpenAI ↔ Anthropic · zero-copy passthrough</span></td>
<td align="center" width="25%">🧭<br><b>Smart routing</b><br><span style="color:#64748B">6-factor weighting (optional quota factor) · cache-economy affinity · LinUCB</span></td>
<td align="center" width="25%">🛡️<br><b>Resilience</b><br><span style="color:#64748B">circuit breaker · retry · fallback chain</span></td>
<td align="center" width="25%">📊<br><b>Observability & cost</b><br><span style="color:#64748B">request logs · token pre-aggregation · cost tracking</span></td>
</tr>
</table>

</div>

---

## 📋 Table of Contents

<details open>
<summary>📑 Table of Contents</summary>

- [✨ Features](#-features)
- [🚀 Quick Start](#-quick-start)
- [⚙️ Configuration Overview](#️-configuration-overview)
- [📚 Documentation](#-documentation)
- [🏗️ Architecture](#️-architecture)
- [📚 As a Library Dependency](#-as-a-library-dependency)
- [🧪 Testing](#-testing)
- [⚡ Performance Design](#-performance-design)
- [🔒 Security](#-security)
- [🗺️ Roadmap](#️-roadmap)
- [🐳 Docker Deployment](#-docker-deployment)
- [🤝 Contributing](#-contributing)
- [📋 Changelog](#-changelog)
- [📄 License](#-license)
- [🙏 Acknowledgments](#-acknowledgments)
- [📞 Contact & Support](#-contact--support)

</details>

---

## ✨ Features

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔄 <b>Protocol conversion</b><br><span style="color:#64748B">Bidirectional OpenAI Chat / Embeddings ↔ Anthropic Messages conversion with SSE streaming on both sides; zero-copy passthrough when the inbound protocol matches the target vendor</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🧭 <b>Smart routing</b><br><span style="color:#64748B">6-factor weighted-random selection (model weight × queue × time window, × optional latency / error rate / quota budget) with temperature smoothing; optional Power-of-Two-Choices, LinUCB contextual bandit, cache-economy session affinity (<code>[routing.affinity]</code>: auto sticks by cache worth / turn window / persists across restarts, legacy default = zero regression), SLA virtual queue and LPS queueing lookahead; difficulty tiering (<code>[routing.difficulty]</code>, off by default): rule scoring fused with vecboost laya decision probabilities rewrites the target model into low/mid/high tiers by difficulty — small models for easy questions, large models for hard ones</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🛡️ <b>Resilience</b><br><span style="color:#64748B">limiteron circuit breaker (opens after 5 failures for 30s, slow-call threshold 60s avoids false trips), status-classified retries (429 honors Retry-After with spread, 5xx fixed-interval delay + spread), priority-driven fallback chain (total attempt budget defaults to 8)</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🚦 <b>Inbound rate limiting & admission</b><br><span style="color:#64748B">limiteron tower-layer inbound rate limiting (endpoint table + global layer), plus <code>[[rate_limit.rules]]</code> path rules (with no global rules configured, a shared token bucket is the fallback); congestion-aware priority admission (per-key tiers dedicated/guaranteed/elastic/spot)</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔐 <b>Secure auth &amp; key vault</b><br><span style="color:#64748B">API keys stored only as SHA256 digests with constant-time comparison; upstream credentials can persist in the encrypted vault (<code>POST/GET/DELETE /admin/vault/keys</code>, cocoon AEAD at-rest encryption, zero plaintext in responses/logs), seeded once from env and never re-entered — the master key comes from <code>FLUXWEIR_MASTER_KEY</code> or an auto-generated file (<code>~/.fluxweir/vault_master.key</code>, 0600); <b>losing it is unrecoverable, back it up</b></span></td>
<td width="50%" style="vertical-align:top; padding: 12px">⚡ <b>Response cache</b><br><span style="color:#64748B">Exact-match response cache (oxcache-backed) keyed by tenant + vendor + body fingerprint, tiered TTL multipliers with load linking, concurrent de-duplication (single-flight) carried by oxcache <code>get_or</code>, <code>X-No-Cache: 1</code> to force origin fetch</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">📊 <b>Request logs & pre-aggregation</b><br><span style="color:#64748B">Asynchronous batched writes to the configured backend (SQLite by default; db-postgres/db-mysql builds write to that backend — drop-counting under backpressure, never blocks responses), token usage UPSERT pre-aggregated by (vendor, model, API-key digest, date, hour)</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">💰 <b>Cost tracking</b><br><span style="color:#64748B">the four-tier unit prices in <code>[[model_catalog]]</code> are the costing source of truth (micro-USD atomic accumulation per vendor/model, hot-reloadable); <code>[[pricing]]</code> feeds only the routing SLA factor and the quota fallback tier for uncataloged models. There is also an offline cost-replay subcommand <code>fluxweir cost-replay</code> (<code>cost-replay</code> feature, not in <code>full</code>): replays <code>request_logs</code> history to price counterfactual affinity / cache_ttl scenarios — read-only, never writes the DB (the target DB must already exist; it errors otherwise)</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔥 <b>Hot config reload</b><br><span style="color:#64748B">Lock-free hot reads via confers DynamicField; transactional write path (validate → atomic persist → in-memory swap) with automatic snapshots in <code>config-backups/</code> (10 kept, secrets masked); <code>server</code>/<code>cors</code>/<code>rate_limit</code>/<code>database</code>/<code>grpc</code>/<code>global_model</code>/<code>cache.semantic</code>/<code>docs.enabled</code>/<code>routing.difficulty.enabled</code> and the <code>routing.difficulty.classifier</code> keys <code>model_path</code>/<code>engine</code>/<code>cache_ttl_secs</code> are reported explicitly in the <code>restart_required</code> response field (remaining restart-required surfaces such as vendor <code>concurrency</code> carry no such signal — restart manually)</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🌐 <b>Admin endpoints & observability</b><br><span style="color:#64748B">52 method endpoints with all features enabled (14 data-plane + 2 search/fetch + 35 admin + 1 health, 11 of them feature-gated; plus <code>/readyz</code>, <code>/metrics</code> ops exits and the <code>/console</code> UI): log query/search, token stats, cost breakdown query, global metrics (affinity hit rate, drop counts), vendor status & key-pool reset, route explain, config & model management, subscription/masking/catalog-sync (feature-gated), health check</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🗣️ <b>Internationalization</b><br><span style="color:#64748B">Fluent dual bundles (en/zh) covering all business messages and error copy, with a <code>FLUXWEIR_LANG</code> → system locale → en fallback chain</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🧰 <b>Production ready</b><br><span style="color:#64748B">Multi-stage Docker build with non-root runtime, image-level health check (<code>HEALTHCHECK</code>) plus docker-compose resource limits, GitHub Actions CI (fmt/clippy/test/coverage/audit) and automated release pipeline</span></td>
</tr>
</table>

---

## 🚀 Quick Start

### 📦 Requirements

- Rust 1.97.1+ (the MSRV declared by `rust-version` in `Cargo.toml`; edition 2021)
- SQLite: **statically embedded in every build** — `libsqlite3-sys` compiles the bundled C source and links it statically (`cargo tree` shows bundled; `ldd` on the artifact shows no dynamic sqlite dependency; identical for glibc and musl), so neither a system sqlite library nor `libsqlite3-dev` is required. TLS is pure rustls — no OpenSSL
- The 7 self-developed libraries (`confers`, `dbnexus`, `inklog`, `limiteron`, `oxcache`, `sdforge`, `trait-kit`) come from crates.io registry — cargo fetches them automatically, no sibling directory needed. The exception is the optional `difficulty-classifier` / `semantic-cache` features: they rely on the path dependency `vecboost = { path = "../vecboost" }` (`Cargo.toml`, following a local checkout while that library's surface is still evolving), so enabling them requires a vecboost checkout next to this repository; the default `full` does not include either feature, so default-caliber builds are unaffected

### 🔨 Build & run

<details>
<summary>📄 Code · bash</summary>

```bash
# Runtime config is config.toml — not committed (.gitignore lists it); start with
# cp config.example.toml config.toml (the example is the full in-repo reference;
# configure vendors and the env vars behind `api_key_env` before serving)
cp config.example.toml config.toml

# Reads config.toml from the working directory by default
cargo run --release

# Or specify a config file
cargo run --release -- /path/to/config.toml
```

</details>

### 💡 First requests

<details>
<summary>📄 Code · bash</summary>

```bash
# Health check (no auth; the registered path carries the sdforge version prefix)
curl http://127.0.0.1:8080/api/v1/health

# OpenAI-compatible chat completion
curl -X POST http://127.0.0.1:8080/v1/chat/completions \
  -H "Authorization: Bearer <your-api-key>" \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-4", "messages": [{"role": "user", "content": "Hello"}]}'
```

</details>

> Note: the bare `/health` path is not a registered route (it 404s) but is included in the auth and rate-limit whitelists. For the remaining entry points (Anthropic messages, model list, token counting) and every endpoint's parameters, responses and error codes, see the [📘 API Reference](docs/en/API_REFERENCE.md).

---

## ⚙️ Configuration Overview

Runtime configuration is a single `config.toml` (**not committed** — a clean checkout starts with `cp config.example.toml config.toml`; the in-repo example is the fully annotated reference carrying every section and field with its default and hot-reload availability). The 30 top-level sections' overview, per-field details, the hot-reload matrix and vendor/pricing config examples live in the [📖 User Guide](docs/en/USER_GUIDE.md).

---

## 📚 Documentation

<details>
<summary>📋 Table</summary>

| Document                                 | Description                                                                   |
| ------------------------------------ | ---------------------------------------------------------------------- |
| [📖 User Guide](docs/en/USER_GUIDE.md)    | End-to-end tutorial: configuration, deployment, operations                                       |
| [📘 API Reference](docs/en/API_REFERENCE.md) | All method endpoints (14 data-plane + 2 search/fetch + 35 admin + 1 health; 52 with all features enabled): parameters, responses, error codes |
| [🏗️ Architecture](docs/en/ARCHITECTURE.md)  | Design principles, routing decision system, resilience & hot reload                               |
| [🔒 Security](docs/en/SECURITY.md)      | Security design, supply-chain gates, vulnerability handling                                     |
| [❓ FAQ](docs/en/FAQ.md)                | Common questions and known limitations                                                     |
| [📋 Changelog](docs/en/CHANGELOG.md)     | Version history (Keep a Changelog format)                                  |
| [🤝 Contributing](docs/en/CONTRIBUTING.md)  | Environment setup, development commands, commit conventions                                           |
| [⚡ Performance Baselines](docs/en/PERFORMANCE.md)   | Criterion component-level benchmark figures (conversion / SSE / routing / affinity)                   |
| [🧭 Prompt Prefix Caching Guide](docs/en/PROMPT_CACHING_GUIDE.md) | The caller-side guide to maximizing vendor-side prefix-cache hits                   |
| [🤝 Code of Conduct](docs/en/CODE_OF_CONDUCT.md) | Community participation rules (Contributor Covenant 2.1)                |

</details>

---

## 🏗️ Architecture

The layered pipeline (middleware → API → routing decision → protocol conversion → vendor execution → async persistence), the routing decision system, resilience state machines, hot-reload transaction semantics, DI assembly and i18n design are documented in the [🏗️ Architecture guide](docs/en/ARCHITECTURE.md).

---

## 📚 As a Library Dependency

`fluxweir` is both an executable gateway and a Rust library (the default `full` feature includes the server components; the client SDK is provided by the `sdk` feature): embedded startup, Rust SDK usage examples and database-backend selection live in the [📖 User Guide · As a library dependency](docs/en/USER_GUIDE.md#-as-a-library-dependency).

---

## 🧪 Testing

The testing strategy, CI-consistent commands and the latest test baseline live in [🤝 Contributing · Testing conventions](docs/en/CONTRIBUTING.md#-testing-conventions).

---

## ⚡ Performance Design

The verifiable design points — zero-copy passthrough for same-protocol requests, lock-free hot reads via confers DynamicField, fully asynchronous logging and pre-aggregation — are documented in the [🏗️ Architecture guide](docs/en/ARCHITECTURE.md); Criterion component baselines (conversion / SSE frame parsing / routing decision paths / cache-economy affinity) live in [⚡ Performance Baselines](docs/en/PERFORMANCE.md).

---

## 🔒 Security

### 🛡️ Design

API keys stored as SHA256 digests with constant-time comparison, upstream credentials referenced by env var name only, error responses exposing fixed `error.type` codes only, secrets masked in config snapshots, layered CORS and inbound rate limiting (429), non-root Docker runtime. See the [🔒 Security guide](docs/en/SECURITY.md) for details.

### ⚠️ Subscription upstream compliance notice (subscription-vendor)

Using OAuth subscription accounts as server-side outbound upstreams may violate the vendor's terms of service; the compliance boundaries, single-instance constraint and observability surface to read before enabling live in [🔒 Security · Subscription upstream compliance notice](docs/en/SECURITY.md#️-subscription-upstream-compliance-notice-subscription-vendor).

### 🚨 Reporting a vulnerability

Please do not report security vulnerabilities through public issues. Use the GitHub [Security Advisories](https://github.com/Kirky-X/fluxweir/security/policy) private disclosure channel instead — the project commits to acknowledging within 48 hours and providing an initial assessment within 7 days; the full process is in [🔒 Security · Vulnerability reporting process](docs/en/SECURITY.md#-vulnerability-reporting-process).

---

## 🗺️ Roadmap

Completed items per area and their delivery calibers live in [📋 Changelog · Roadmap](docs/en/CHANGELOG.md#️-roadmap).

---

## 🐳 Docker Deployment

Image building (incl. the vecboost additional-context prerequisite), container running (incl. the single-file-mount EBUSY warning) and docker-compose deployment live in [📖 User Guide · Docker Deployment](docs/en/USER_GUIDE.md#-docker-deployment).

---

## 🤝 Contributing

For the detailed contribution process, development commands and commit conventions, see the [🤝 Contributing guide](docs/en/CONTRIBUTING.md).

---

## 📋 Changelog

The full version history lives in the [📋 Changelog](docs/en/CHANGELOG.md) (following the [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format, semantic versioning).

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).

---

## 🙏 Acknowledgments

The core-dependency list (7 self-developed sibling libraries) lives in the [🤝 Contributing guide](docs/en/CONTRIBUTING.md); thanks to the Rust community and all [contributors](https://github.com/Kirky-X/fluxweir/graphs/contributors).

---

## 📞 Contact & Support

Report issues and bugs on [Issues](https://github.com/Kirky-X/fluxweir/issues), ask questions and share ideas on [Discussions](https://github.com/Kirky-X/fluxweir/discussions), and browse the source on [GitHub](https://github.com/Kirky-X/fluxweir).

---

<sub>© 2026 Kirky.X. All rights reserved.</sub>
