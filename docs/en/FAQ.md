# Fluxweir FAQ

This page collects common questions and real quirks from using Fluxweir (the Rust LLM routing gateway), each answer ending with the code location. For systematic how-to instructions see the [User Guide](./USER_GUIDE.md); for the project overview see the [README](../../README.md).

## 📋 Table of Contents

- [Routing & endpoints](#-routing-endpoints)
- [Configuration & hot reload](#️-configuration-hot-reload)
- [Auth & rate limiting](#-auth-rate-limiting)
- [Cost & statistics](#-cost-statistics)
- [Resilience & routing behavior](#️-resilience-routing-behavior)

---

## 🌐 Routing & endpoints

### ❓ Why does `GET /health` return 404?

Because the **registered health-check path is `/api/v1/health`**. In `src/api/health.rs`, the `#[forge(version = "v1", path = "/health")]` macro automatically prepends the `/api/v1` version prefix before registering.

The bare `/health` path appears in the **rate-limit** whitelist (the ten entries of `is_quota_exempt_path`: `/health`, `/healthz`, `/ready`, `/api/v1/health`, `/readyz`, `/metrics`, `/api-docs` and `/api-docs/*`, `/swagger-ui` and `/swagger-ui/*` — see the rate-limit exemption table in `src/api/router_builder.rs`). The auth side keeps a **separate, independent whitelist** (`/health`, `/api/v1/health`, `/`, `/readyz`, `/metrics`, `/admin/subscription/callback`, plus the two documentation groups — ten entries in total, see the auth whitelist decision in `src/middleware/auth.rs`); the two are independent and share no semantics. Exceptions are `/healthz` and `/readyz` — mounted by sdforge after the auth layer, **auth-free and rate-limit-free** (a keyless request returns 200 in practice); `/ready` is unregistered (404) and survives only in the rate-limit exemption table. A whitelist entry only guarantees "not intercepted by middleware", not "a handler exists" — and the bare `/health` has **no route registered on it at all**. Please use:

```bash
curl http://127.0.0.1:8080/api/v1/health
```

> The Docker health check now has a **single definition at the image layer** (`HEALTHCHECK`; compose no longer declares its own) and targets this path (it was previously misconfigured as the bare `/health`, with two inconsistent `start-period` values, now fixed) — see [User Guide · Docker Deployment](./USER_GUIDE.md#-docker-deployment).

**Code locations**: `src/api/health.rs` (forge macro registration), `src/api/router_builder.rs` (rate-limit exemption table)

### ❓ How do I target a specific vendor or model?

Use request headers (`X-Vendor` / `X-Model` are consumed only by the OpenAI-compatible endpoint `/v1/chat/completions` — `/v1/messages` keeps its no-hints routing semantics and ignores both headers; `X-Session-Id` / `X-Weight` are supported by both endpoints):

| Header | Behavior |
|:-------|:-----|
| `X-Vendor: <vendor_id>` | `/v1/chat/completions` only: go straight to the named vendor; **a missing vendor is a Validation error**, never a silent fallback to default routing |
| `X-Model: <abstract_id>` | `/v1/chat/completions` only: prefer routing by abstract model (case-insensitive); this request is an explicit target and does not join session affinity |
| `X-Session-Id: <key>` | Session affinity key (both endpoints); when missing, falls back to truncated hex of `sha256(first 2KB of messages[0])` |
| `X-Weight: <positive number>` | Request-level weight (both endpoints); effective only with `[routing].allow_request_weight = true` (ignored by default) |

**Code locations**: `src/api/routing_hints.rs` (header extraction), `src/api/completions.rs` (when `hints.has_hints()`, goes through `route_with_hints`; an unmatched X-Vendor errors out), `src/api/anthropic.rs` (keeps the no-hints semantics via `route_weighted`)

---

## ⚙️ Configuration & hot reload

### ❓ Why did some settings not take effect after `POST /admin/config`?

Because each section takes effect differently. `server` / `cors` are baked into the axum Router by sdforge at router-build time, `rate_limit` is consumed while the Router is constructed, vendor `concurrency` is consumed when vendor instances are constructed, `database` builds its pool at startup, and `cache.semantic`'s semantic layer is assembled once during bootstrap — these sections **require a process restart**; the remaining sections (`auth` / `retry` / `routing` / `cache` (exact cache) / `resilience` / `admission` / `quota` / `health` / `pricing` / `global_model` / `lexcompress` / `masking` / `usage_insights` / `responses` / `model_catalog` / `key_budgets` / `vendor_budgets`) are hot-read per request and take effect on the next request. Among them, `global_model`'s `enabled` is hot-read per request at routing decision time via `ConfigManager::is_global_model_enabled` — effective on the next request after the switch, though the response still carries the conservative `restart_required` hint (file-side hot reload of this section is rejected by the structural gate, so the hint follows the most conservative caliber).

The `restart_required` array in the response lists the sections carrying conservative restart hints: `server` / `cors` / `rate_limit` / `database` / `grpc` / `otlp` / `global_model` / `cache.semantic` / `docs.enabled` / `routing.difficulty.enabled`, plus the `routing.difficulty.classifier` keys `model_path` / `engine` / `cache_ttl_secs` (the classifier is assembled once during bootstrap). Beyond the array, two restart-required surfaces remain for you to schedule: vendor `concurrency` (consumed when vendor instances are constructed) and the `vendors` topology.

**Code location**: `src/api/admin.rs` (restart_required assembly in update_config)

### ❓ Did a custom section disappear after config.toml was written back?

`persist` uses **full-file write-back semantics**: every `POST /admin/config` persists the entire `Config` struct back to config.toml via `toml::to_string_pretty`. All sections must be defined in the `Config` struct (the `[resilience]` section was added to the struct precisely for this); **unknown sections not present in the struct are silently dropped during TOML deserialization** and vanish from disk after the first hot-reload write-back.

Do not put custom sections Fluxweir does not know into config.toml; to extend, add fields to the `Config` struct in `src/config/mod.rs` first.

**Code location**: `src/config/mod.rs` (the "persist full write-back" hard-constraint note in the `Config` struct field docs)

### ❓ What is the semantics of the `/admin/logs` `end_time` filter?

Fixed. All four links of the `/admin/logs?end_time=...` parameter chain work: the API layer receives and forwards it (`src/api/admin.rs`) → `LogFilter.end_time` (`src/storage.rs`) → the SQL branch of `Repository::query_logs` (`AND timestamp <= ?`, passed as a bound parameter) → database-level filtering. The time unit is seconds as i64 (consistent with `request_logs.timestamp` storage).

The filter semantics are a **closed interval** `[start_time, end_time]`, both boundaries inclusive — rows with `timestamp = end_time` are returned, symmetric with the existing `start_time >=` branch.

**Code location**: `src/database/repository.rs` (`query_logs`; the `end_time` branch directly follows the `start_time` branch)

### ❓ Why is the `vendor` parameter of `/admin/logs/search` a search keyword?

Historical design. The endpoint reuses the `LogQuery` query struct (whose `vendor` field name carries over from `/admin/logs`), but the semantics are entirely different: `search_logs` takes `params.vendor` and uses it directly as a **full-text keyword**, doing `LIKE %keyword%` matching against the `messages` and `error_message` columns — unrelated to vendor filtering.

```bash
# Search logs whose message or error message contains "timeout"
curl "http://127.0.0.1:8080/admin/logs/search?vendor=timeout"
```

**Code location**: `src/api/admin.rs` (`search_logs`'s `let keyword = params.vendor.unwrap_or_default()`)

### ❓ How do I switch the message language?

Set the environment variable **`FLUXWEIR_LANG=en|zh`** (project-level override, highest priority). When unset, the detection chain applies: `LC_ALL` → `LC_MESSAGES` → `LANG` → system locale (sys-locale), falling back to `en` when nothing is recognizable. `zh_CN` / `zh_TW` and other zh variants all map to `zh`.

**Code location**: `src/i18n/locale.rs` (`detect_from` detection chain)

---

## 🔑 Auth & rate limiting

### ❓ Which auth headers are supported?

Two, consumed in order:

| Priority | Header | Form |
|:------:|:-------|:-----|
| 1 | `Authorization` | `Bearer <key>` (prefix stripped) |
| 2 | `x-api-key` | `<key>` verbatim |

`Authorization: Bearer` is tried first, then `x-api-key`. The key is SHA256-digested and compared in constant time against `[auth].api_keys`; keys not declared in `[[auth.keys]]` default to the `elastic` tier. When the whole `[auth]` section is absent, auth is disabled (allow all).

**Code location**: `src/utils/tenant.rs` (`extract_api_key`; `src/middleware/auth.rs` re-exports the same implementation)

### ❓ What does `POST /admin/vendors/:id/reset` do?

It clears the runtime state of that vendor's key pool — the key-pool blacklist, rotation cursor and cooldown state (the WP6 key-pool three-state). Rate-limit tokens refill automatically at their rate and need no manual reset; this endpoint targets the **error-classification-driven key blacklist and cooldown** (after 401/402/403 and billing-form errors blacklist a key, use this to restore it). Outbound retries/failover are unaffected. An unknown vendor explicitly returns `validation_error` (400); on success it returns `success: true` with the message that the key-pool blacklist and cooldown state were cleared.

**Code location**: `src/api/admin.rs` (`reset_vendor` → `VendorManager::reset_key_pool_state`, `src/vendor/manager.rs`)

### ❓ How do I force-bypass the response cache?

Send **`X-No-Cache: 1`** (compatibility alias `X-Skip-Cache`) with the request: it neither reads nor writes the cache and forces an origin fetch. Note the prerequisite: `[cache].enabled` defaults to `false`, so the cache is off to begin with; the header is only needed for per-request exemption after you enable it explicitly.

**Code location**: `src/api/response_cache.rs` (`X-No-Cache` detection)

---

## 💰 Cost & statistics

### ❓ Why is the cost statistic always 0?

Two independent causes — check them one by one:

1. **Pricing miss**: cost matches `[[model_catalog]]`'s four-tier unit prices exactly by `(vendor_id, model_id)` — the catalog is the costing source of truth (`estimate_cost_from_catalog`); a model missing from the catalog simply returns 0 (without blocking the response). `[[pricing]]` only feeds the routing SLA factor and the quota fallback tier for uncataloged models — it is not the costing source.
2. **Where to look**: once a non-zero cost is computed, it is written into the four cost columns of the `token_usage` table by `(vendor, model, api_key_hash, date, hour)` (`input_cost`/`output_cost`/`cache_read_cost`/`cache_creation_cost`) and can be aggregated via `GET /admin/cost` grouped by vendor/model, surviving restarts; per-request logs `request_logs.cost_usd` are persisted (four-tier sum × catalog prices, written by all five data-plane entries); the in-process `CostTracker` accumulates in memory and restores from `token_usage` at startup per `[metrics].cost_restore_window_hours` (default 24h) — **it no longer resets on restart**. If `GET /admin/cost` is still 0, go back to item 1 and check `[[pricing]]` / `[[model_catalog]]` pricing hits.

**Code locations**: `src/vendor/cost.rs` (`estimate_cost_from_catalog` returns `None` on a catalog miss) + caller mapping (`src/api/mod.rs` cost breakdown, `src/middleware/quota.rs` quota); the `[[pricing]]` path (`CostCalculator::calculate`, `src/vendor/cost.rs`) has zero production callers and is used by tests only

---

## 🛡️ Resilience & routing behavior

### ❓ When does the circuit breaker open / recover?

The per-vendor circuit breaker (limiteron CircuitBreaker) state machine:

| State | Entry condition / behavior |
|:-----|:----------------|
| Closed (normal) | **5 consecutive failures** → Open |
| Open | lasts **30 seconds**, during which requests are fast-rejected and failover is triggered |
| Half-open (probing) | after 30 seconds a probe request is let through; **3 consecutive successes** → back to Closed |

Slow-call misfire protection: the slow-call threshold is **60 seconds at a 100% ratio** — successful LLM calls routinely exceed the generic default (500ms); without raising it, the slow-call ratio trips easily under mixed traffic and causes circuit breaks unrelated to real failures; at 60s / 1.0 the slow-call path goes dormant under mixed traffic and breaks are driven by real failures only.

**Code location**: `src/vendor/manager.rs` (`gate_circuit_config`: `CircuitBreakerConfig::new(5, 3, 30s)` + the slow-call pair)

---

## 🎯 More questions?

| Document | Description |
|:-----|:-----|
| [User Guide](./USER_GUIDE.md) | Complete path from build to operations |
| [API Reference](./API_REFERENCE.md) | Endpoints and request / response formats |
| [Architecture](./ARCHITECTURE.md) | Internal mechanisms and module design |
| [README](../../README.md) | Project overview |
