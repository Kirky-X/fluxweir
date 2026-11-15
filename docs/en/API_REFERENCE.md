# Fluxweir REST API Reference

**Doc baseline**: v0.1.0 (per Cargo.toml)
**Base URL**: `http://0.0.0.0:8080` (default config; actual host/port per the `[server]` section)
**Content type**: `application/json` (streaming responses are `text/event-stream`)
**Content negotiation**: error message copy supports English and Chinese (server-side i18n following the `FLUXWEIR_LANG`/`LANG` environment variables; detection chain `FLUXWEIR_LANG` → `LC_ALL` → `LC_MESSAGES` → `LANG`)

---

## Table of Contents

- [Authentication](#authentication)
- [Common error response](#common-error-response)
- [Error code mapping](#error-code-mapping)
- [LLM inference endpoints](#llm-inference-endpoints)
  - [POST /v1/chat/completions](#post-v1chatcompletions)
  - [POST /v1/embeddings](#post-v1embeddings)
  - [POST /v1/rerank](#post-v1rerank)
  - [POST /v1/responses](#post-v1responses)
  - [GET /v1/responses/:id](#get-v1responsesid)
  - [DELETE /v1/responses/:id](#delete-v1responsesid)
  - [POST /v1/responses/:id/cancel](#post-v1responsesidcancel)
  - [POST /v1/responses/input_tokens](#post-v1responsesinput_tokens)
  - [Gemini native endpoints](#gemini-native-endpoints)
  - [POST /v1/messages](#post-v1messages)
  - [GET /v1/models](#get-v1models)
  - [POST /v1/messages/count_tokens](#post-v1messagescount_tokens)
- [Admin endpoints](#admin-endpoints)
  - [GET /admin/logs](#get-adminlogs)
  - [GET /admin/logs/search](#get-adminlogssearch)
  - [GET /admin/stats/tokens](#get-adminstatstokens)
  - [GET /admin/cost](#get-admincost)
  - [GET /admin/sessions](#get-adminsessions)
  - [GET /admin/context-stats](#get-admincontext-stats)
  - [GET /admin/metrics](#get-adminmetrics)
  - [GET /admin/quota/ledger](#get-adminquotaledger)
  - [GET /admin/vendor-budgets](#get-adminvendor-budgets)
  - [GET /admin/vendors](#get-adminvendors)
  - [GET /admin/vendors/config](#get-adminvendorsconfig)
  - [PUT /admin/vendors/:id](#put-adminvendorsid)
  - [GET /admin/vendors/:id/status](#get-adminvendorsidstatus)
  - [POST /admin/vendors/:id/reset](#post-adminvendorsidreset)
  - [GET /admin/embedding-pins](#get-adminembedding-pins)
  - [POST /admin/embedding-pins/:id/reset](#post-adminembedding-pinsidreset)
  - [GET /admin/config](#get-adminconfig)
  - [POST /admin/config](#post-adminconfig)
  - [GET /admin/models](#get-adminmodels)
  - [PUT /admin/models/:id](#put-adminmodelsid)
  - [DELETE /admin/models/:id](#delete-adminmodelsid)
  - [POST /admin/models/:id](#post-adminmodelsid)
  - [POST /admin/catalog/sync](#post-admincatalogsync)
  - [GET /admin/masking/stats](#get-adminmaskingstats)
  - [GET /admin/vault/keys](#get-adminvaultkeys)
  - [POST /admin/vault/keys](#post-adminvaultkeys)
  - [DELETE /admin/vault/keys/:id](#delete-adminvaultkeysid)
  - [GET /admin/masking/ledger](#get-adminmaskingledger)
  - [POST /v1/search](#post-v1search)
  - [POST /v1/fetch](#post-v1fetch)
  - [GET /admin/web-search/stats](#get-adminweb-searchstats)
  - [POST /admin/subscription/authorize](#post-adminsubscriptionauthorize)
  - [GET /admin/subscription/callback](#get-adminsubscriptioncallback)
  - [GET /admin/subscription/accounts](#get-adminsubscriptionaccounts)
  - [DELETE /admin/subscription/accounts/:id](#delete-adminsubscriptionaccountsid)
  - [POST /admin/route/explain](#post-adminrouteexplain)
  - [POST /admin/route/difficulty-explain](#post-adminroutedifficulty-explain)
- [Built-in console (console feature)](#built-in-console-console-feature)
- [Health checks](#health-checks)
  - [GET /api/v1/health](#get-apiv1health)
  - [GET /readyz](#get-readyz)
  - [GET /metrics](#get-metrics)
- [Rust SDK client](#rust-sdk-client)
- [Appendix: routing hint headers](#appendix-routing-hint-headers)

---

## Authentication

All endpoints require authentication except health checks and the root path.

| Item | Value |
|---|---|
| Auth method | Bearer Token (API key) |
| Header | `Authorization: Bearer <api_key>` |
| Fallback header | `x-api-key: <api_key>` |
| Universal fallback header | `x-goog-api-key: <api_key>` (effective on any path, Gemini SDK compatible; the `?key=<api_key>` query parameter is accepted only on `/v1beta/` paths) |
| Config location | the `[auth]` section of `config.toml` (`api_keys` stores SHA256 hex digests; plaintext never touches disk) |
| Switch | `auth.enabled` (`true` / `false`) |

The uniform auth-failure response:

```json
{
  "error": {
    "message": "Missing API key",
    "type": "authentication_error"
  }
}
```

HTTP status: `401 Unauthorized` (with an invalid key, `message` is `"Invalid API key"`).

> The auth middleware's allow paths are a code fact (the auth whitelist decision in `src/middleware/auth.rs`): ten entries allowed explicitly — `/health`, `/api/v1/health`, `/`, `/readyz`, `/metrics`, `/admin/subscription/callback` (the OAuth 302 callback, a bare GET), `/api-docs` and `/api-docs/*`, `/swagger-ui` and `/swagger-ui/*`. Of these, `/api/v1/health` is the health check's actual registered path (see [Health checks](#health-checks)); bare probe paths like `/health` have no registered route (requests 404) but stay in the whitelist (auth-free). `/healthz` and `/readyz` are mounted by sdforge **after all middleware** (`build_with_config`; Fluxweir's auth/rate-limit/timeout layers wrap only business routes, so the probes bypass them), **auth-free and rate-limit-free** (a keyless request returns 200 in practice); `/readyz` really runs DB and cache probes per request — internal-network exposure recommended; `/ready` is unregistered (404), surviving only in the rate-limit exemption table.

---

## Common error response

All non-2xx responses from Fluxweir handlers use the structure below (see the `IntoResponse` implementation in `src/error/mod.rs`); 404s for unregistered paths fall back to sdforge, shaped `{"code":"NOT_FOUND","message":"Route not found","trace_id":…}`.

```json
{
  "error": {
    "message": "<human-readable error description>",
    "type": "<error type code>"
  }
}
```

Field notes:

| Field | Type | Notes |
|---|---|---|
| `error.message` | string | detailed error description, following the i18n localization track (per-locale en/zh copy) |
| `error.type` | string | fixed error type code, never exposing internal enum structure |

---

## Error code mapping

The mapping below comes from the actual `FluxweirError::into_response` implementation in `src/error/mod.rs` (code governs).

| Error type (`error.type`) | HTTP status | Trigger |
|---|---|---|
| `validation_error` | 400 Bad Request | required request fields missing (e.g. `model`), or a model name with no usable candidates (incl. `model = "auto"` with `[global_model].enabled = false` closing the auto tier) |
| `protocol_error` | 502 Bad Gateway | protocol conversion failure, unexpected upstream responses, embeddings called against non-OpenAI vendors |
| `vendor_unavailable` | 503 Service Unavailable | vendor unavailable, network request failures |
| `timeout` | 504 Gateway Timeout | request timeout, `oneshot` channel receive failures |
| `rate_limited` | 429 Too Many Requests | vendor-side rate limiting |
| `all_vendors_throttled` | 429 Too Many Requests + `Retry-After` header | thrown when all candidate vendors remain rate-limited after fallback failover (requires `[routing].retry_after_hint`; with the default off, the original error semantics hold and this code is not produced); `Retry-After` takes the minimum of the candidates' remaining rate-limit window reset seconds |
| `config_error` | 500 Internal Server Error | config errors (e.g. the API key env var unset) |
| `database_error` | 500 Internal Server Error | database operation failures, IO errors |
| `routing_error` | 500 Internal Server Error | routing decision failures (no available vendor etc.) |
| `vault_error` | 404 / 409 / 500 | vault key missing (404), `vendor_id` unregistered (409), encryption/storage failures (500) (`vault` feature) |
| `masking_unavailable` | 503 Service Unavailable | masking engine fault, fail-closed refusal to send plaintext outbound (`masking` feature) |
| `masking_blocked` | 400 Bad Request | the masking Block action hit a sensitive value (`masking` feature) |
| `io_error` | 500 Internal Server Error | file/stream IO failures |

> **Rate limiting and admission (gateway-side, independent of the error codes above; tower implementation)**:
>
> - The inbound rate-limit middleware has two layers: the `[[rate_limit.rules]]` endpoint-rule table (`path` prefix matching, bucketed per tenant) + limiteron's tower global-rules layer (one `KeyedRateLimitLayer` per rule, bucketed per tenant digest; with no rules, a shared TokenBucket(100, 10/s) fallback).
> - Endpoint-rule over-limit → `429 Too Many Requests` + `Retry-After` header (reset seconds), body JSON, `error.message` copy per locale (zh `"已超过速率限制"` / en `"Rate limit exceeded"`).
> - Global-rule over-limit → `429 Too Many Requests` + `Retry-After` header (window remainder seconds), same copy; the legacy `Governor` path's header-less 429 and the `403` ban branch are deleted (rule bans are always None).
> - Rate-limit whitelist paths (skipping rate-limit checks; the ten entries of `is_quota_exempt_path`): `/health`, `/healthz`, `/ready`, `/api/v1/health`, `/readyz`, `/metrics`, `/api-docs` and `/api-docs/*`, `/swagger-ui` and `/swagger-ui/*`.
> - Every response (incl. 429) returns the **`X-Request-Id`** header: passing through the client's ID when present, otherwise gateway-generated (`req-` prefix); the same request's tracing span carries that ID.
> - **Failover observability headers** (`[resilience].expose_fallback_headers`, default `false`, ✅ hot-reloadable; when off, responses are byte-identical to the status quo): when on, successful responses carry `x-fluxweir-vendor` (the finally-hit vendor id), `x-fluxweir-model` (the served model id), `x-fluxweir-attempts` (total attempts) and `x-fluxweir-fallbacks` (the comma-separated attempt chain incl. the final hit; streaming writes at the response-header stage). Cache-hit early returns carry no headers (no actual attempts).
> - **The quota layer** (per-key token/cost fixed windows, `src/middleware/quota.rs`): active when `[quota].enabled` (default `false`)
>   is on and any of `[[auth.keys]]`'s `token_quota`/`cost_quota_millicents` is > 0; applies to all billed
>   data-plane traffic: `/v1/chat/completions`, `/v1/messages`, `/v1/rerank`, `/v1/responses` and Gemini
>   `POST /v1beta/models/{model}:action` (except `countTokens`). Entry pre-deducts (estimated input tokens×input price + `max_tokens`×output price,
>   prices from the `[[model_catalog]]` four tiers, with unregistered models falling back to `[[pricing]]`), refunding the difference per actual usage after the response/stream ends;
>   over-limit returns `429 Too Many Requests` + `Retry-After` (window remainder seconds), copy per locale (zh `"配额已耗尽，请稍后重试"`).
> - **Gateway-key cost budgets** (when the `[key_budgets]` section is configured): inbound key cost over-limit likewise returns `429` + `Retry-After` (converted from the budget window headroom); when the budget source is unreachable, `fail_open=false` (default) rejects with a warn and `fail_open=true` allows; a missing section = the feature entirely off, zero behavior change.
>   Non-streaming, converted streams and **passthrough streams** all refund; passthrough stream cuts, `event: error` terminations or usage-less runs (OpenAI without
>   `stream_options.include_usage`) keep the pre-deduction un-refunded; pre-deductions from entry rejections and upstream failures are never returned.
> - **Quota headroom response headers**: spec Out of Scope (X-RateLimit-\* forms; not this phase) — the surface is not provided; headroom observability goes through
>   `GET /admin/cost`'s aggregation-query side. Since quota sits outside rate_limit/admission, 429s rejected by the inner gates have already consumed this round's pre-deduction
>   (the no-refund-on-rejection caliber unchanged — it is simply no longer exposed via headers).
> - **The admission layer** (congestion-aware priority admission, `src/middleware/admission.rs`): with `[admission].enabled` (default `false`) on, when gateway concurrency occupancy (Σ in-flight / Σ concurrency caps) reaches `congestion_threshold` (default `0.9`), requests whose priority is below the smallest tier currently in flight are shed directly, returning `429 Too Many Requests` + `Retry-After: 1`, copy per locale (zh `"准入被拒绝（拥塞）"`). Request tiers (dedicated/guaranteed/elastic/spot) are declared per API key in the `[auth].keys` sub-table, with undeclared keys defaulting to `elastic`. Health-check paths are never shed.

---

## LLM inference endpoints

### POST /v1/chat/completions

An OpenAI Chat Completions compatible endpoint. Routing decisions follow the body's `model` field: when the target vendor's type is `openai`, the zero-copy passthrough path is taken; otherwise the request converts through `UnifiedRequest`.

**Auth**: Bearer Token required

**Headers**:

| Header | Required | Notes |
|---|---|---|
| `Authorization` | yes | `Bearer <api_key>`; with `[auth.oidc_rules]` configured, OIDC short-lived keys are also accepted (CI/automation exchanges via an OIDC provider, TTL default 1h, inheriting all parent-key restrictions; no rules = unchanged behavior) |
| `Content-Type` | yes | `application/json` |
| `X-Vendor` | no | routing hint: preferred vendor ID (ID matching case-insensitive) |
| `X-Model` | no | routing hint: preferred model ID (ID matching case-insensitive) |
| `X-Session-Id` | no | session affinity key; falls back to a `messages[0]` prefix hash when missing (see the [Appendix](#appendix-routing-hint-headers)) |
| `X-Weight` | no | request-level weight (finite positive numbers only); requires the server-side `[routing].allow_request_weight` switch |

**Request body** (`Json<Value>`, OpenAI compatible):

```json
{
  "model": "gpt-4",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello"}
  ],
  "temperature": 0.7,
  "max_tokens": 1024,
  "stream": false,
  "top_p": 1.0,
  "frequency_penalty": 0.0,
  "presence_penalty": 0.0,
  "stop": ["\n\n"],
  "user": "user-123",
  "tools": [],
  "tool_choice": "auto"
}
```

Required-field validation (missing → `validation_error`):

| Field | Type | Required | Notes |
|---|---|---|---|
| `model` | string | yes | model ID for routing decisions. `"auto"` routes the global model pool (the fallback after both abstract and direct routing miss); with `[global_model].enabled = false` that tier is closed and returns `validation_error` (400) (`X-Vendor`/`X-Model` hints are explicit targets, unaffected by the switch) |
| `messages` | array | yes | message list |
| `stream` | boolean | no | default `false`; `true` returns an SSE stream |

**Response (non-streaming, `stream: false`)**:

`Content-Type: application/json`, the standard OpenAI Chat Completions response structure:

```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1700000000,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 20,
    "completion_tokens": 10,
    "total_tokens": 30
  }
}
```

**Response (streaming, `stream: true`)**:

`Content-Type: text/event-stream`, with `Cache-Control: no-cache` and `Connection: keep-alive` added to the headers. Each chunk is standard OpenAI SSE format (`data: {...}\n\n`).

**Error codes**: `validation_error`(400) | `routing_error`(500) | `vendor_unavailable`(503) | `timeout`(504) | `protocol_error`(502) | `rate_limited`(429); with `[routing].retry_after_hint` on and all candidates rate-limited → `all_vendors_throttled`(429 + `Retry-After` header)

**curl example**:

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

Streaming example:

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
  }'
```

Vendor routing hint:

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -H "X-Vendor: mock-openai" \
  -d '{"model": "gpt-4", "messages": [{"role": "user", "content": "Hi"}]}'
```

---

### POST /v1/embeddings

An OpenAI Embeddings compatible endpoint. Supports only `type_ = "openai"` vendors; otherwise `protocol_error`.

**Auth**: Bearer Token required

**Request body** (`Json<Value>`):

```json
{
  "model": "text-embedding-3-small",
  "input": "The food was delicious and the waiter was friendly."
}
```

| Field | Type | Required | Notes |
|---|---|---|---|
| `model` | string | yes | model ID |
| `input` | string \| array | yes | the text or texts to embed (string arrays, integer token arrays and nested token arrays all accepted, the latter two passed to upstream verbatim; missing / empty array → 400 `validation_error`) |
| `dimensions` | number | no | a dimension-trimming parameter passed to upstream; requests carrying it do not participate in the gateway's dimension-gate records |

The body first converts through `embedding_request_to_openai`, then forwards to the upstream OpenAI-compatible API.

**Compatibility-group pinned routing**: embedding requests pin by "one abstract embedding model = one vector space = one compatibility group" — the group of the first weighted-selected vendor is remembered in the pin table, and later requests choose only within the group; without a declared `compat_group`, the group = one vendor (failures error explicitly, never silently switching vendors). See [USER_GUIDE embedding compatibility-group routing](./USER_GUIDE.md#embedding-compatibility-group-routing-post-v1embeddings).

**Response**:

The standard OpenAI Embeddings response structure:

```json
{
  "id": "embd-abc123",
  "object": "list",
  "created": 1700000000,
  "model": "text-embedding-3-small",
  "data": [
    {
      "index": 0,
      "object": "embedding",
      "embedding": [0.0023, -0.0091, 0.0152, "..."]
    }
  ],
  "usage": {
    "prompt_tokens": 12,
    "total_tokens": 12
  }
}
```

**Error codes**: `validation_error`(400) | `protocol_error`(502, incl. "Embeddings only supported for OpenAI vendors"; dimension drift from the same (vendor, model) is also 502, with the error body carrying vendor/model/old-new dimensions and reset guidance) | `vendor_unavailable`(503) | `config_error`(500, API key env unset)

**curl example**:

```bash
curl -X POST http://localhost:8080/v1/embeddings \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "text-embedding-3-small",
    "input": "The food was delicious."
  }'
```

---

### POST /v1/rerank

A Cohere-compatible reranking endpoint (non-streaming). Routes by the `model` field, allowing only candidates hitting "a `type_ = "rerank"` vendor + a `model_type = "rerank"` entry"; no embedding compat-group pins (rerank has no vector-space contract; vendors are interchangeable). The response's `model` echoes the actually-used upstream model name and `meta.vendor` the actual vendor, so score provenance is visible under multi-vendor routing.

**Auth**: Bearer Token required

**Request fields**

| Field | Type | Required | Notes |
|---|---|---|---|
| `model` | string | yes | routing abstract name (`abstract_id`, falling back to `model_id`) |
| `query` | string | yes | query text, ≤4096 chars |
| `documents` | array | yes | elements are strings or `{"text": "..."}`; 1–1000 rows, each ≤8192 chars |
| `top_n` | integer | no | default = `documents.len()`, value `1..=documents.len()` |
| `return_documents` | boolean | no | default `false`; `true` echoes document text in the response |
| `instruction` | string | no | passed through when the upstream supports it (Cohere form) |
| `stream` | — | no | unsupported; presence → 400 |

**Request example**

```bash
curl -X POST http://localhost:8080/v1/rerank \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "rerank-v3",
    "query": "什么是向量检索",
    "documents": ["向量检索是…", "重排序是…"],
    "top_n": 2
  }'
```

**Response example**

```json
{
  "id": "rerank-<uuid>",
  "model": "rerank-v3.5",
  "results": [
    {"index": 1, "relevance_score": 0.93},
    {"index": 0, "relevance_score": 0.12}
  ],
  "meta": {"vendor": "cohere-prod", "billed_documents": 2}
}
```

**Score caliber**: the default `none` passes upstream values through verbatim; `[routing].rerank_score_normalization = "sigmoid"` maps raw scores into (0,1), and `"minmax"` scales within the batch to [0,1] (equal-value batches all 0.5). Normalization changes only scores, never ordering or `index`; before filtering on absolute thresholds across vendors, confirm the vendor caliber — relative `top_n` ordering is unaffected.

**Routing hints**: `X-Vendor` must point to an enabled `type_ = "rerank"` vendor and `X-Model` must resolve to a `model_type = "rerank"` entry, otherwise 502 without sending; a legal explicit target keeps routing within the capability set.

**Session affinity**: with `X-Session-Id`, the request joins session affinity (`[routing].affinity_mode` hard direct selection / soft weight boost, within the eligible candidate set only); successful responses write back the actual vendor binding, failures write nothing.

**Errors**: validation failure / no available rerank candidate → 400; upstream non-2xx/timeout/connection failure → 503/504; illegal upstream response shape (`index` out of range or duplicated, non-finite scores) → 502.

**Metering**: included in `[quota]` pre-deduction (`query + documents` chars converted to tokens); per-request cost computes from `[[model_catalog]].rerank_price_per_1k_docs`, writing `request_logs.cost_usd` and, via breakdown pre-aggregation, `token_usage` (filed under `input_cost`, token columns always 0) — `/admin/cost`'s `input_cost`/`total_cost` covers rerank. Request logs store only summaries (`query_chars` / `documents` / `top_n` / `results` / `top_score`), never raw query or documents.

**Upstream config**: the vendor sets `type_ = "rerank"` and must declare `api_base` explicitly (no official default endpoint), with outbound fixed to `POST {api_base}/rerank` + `Authorization: Bearer`:

```toml
[[vendors]]
id = "cohere-prod"
type_ = "rerank"
api_key_env = "COHERE_API_KEY"  # pragma: allowlist secret (example env var name, not a credential)
api_base = "https://api.cohere.com/v1"   # Jina: https://api.jina.ai/v1; TEI: http://host:port

[[vendors.models]]
model_id = "rerank-v3.5"
abstract_id = "rerank-v3"
model_type = "rerank"
```

---

### POST /v1/messages

An Anthropic Messages API compatible endpoint. When the target vendor's type is `anthropic`, the zero-copy passthrough path is taken; otherwise the request converts through `UnifiedRequest`.

**Auth**: Bearer Token required

**Headers**:

| Header | Required | Notes |
|---|---|---|
| `Authorization` | yes | `Bearer <api_key>` |
| `Content-Type` | yes | `application/json` |
| `anthropic-version` | no | passed through to the upstream Anthropic API |
| `X-Vendor` | no | routing hint: preferred vendor ID (consumed only by the OpenAI-compatible endpoint; this endpoint ignores it) |
| `X-Model` | no | routing hint: preferred model ID (consumed only by the OpenAI-compatible endpoint; this endpoint ignores it) |
| `X-Session-Id` | no | session affinity key; falls back to a `messages[0]` prefix hash when missing |
| `X-Weight` | no | request-level weight (finite positive numbers only); requires `[routing].allow_request_weight` |

**Request body** (`Json<Value>`, Anthropic compatible):

```json
{
  "model": "claude-3-opus",
  "max_tokens": 1024,
  "messages": [
    {"role": "user", "content": "Hello, Claude"}
  ],
  "stream": false,
  "system": "You are a helpful assistant.",
  "temperature": 1.0,
  "top_p": 1.0
}
```

| Field | Type | Required | Notes |
|---|---|---|---|
| `model` | string | yes | model ID |
| `max_tokens` | integer | yes | max generated tokens |
| `messages` | array | yes | message list |
| `stream` | boolean | no | default `false` |

**Response (non-streaming)**:

`Content-Type: application/json`, the standard Anthropic Messages response structure:

```json
{
  "id": "msg_abc123",
  "type": "message",
  "role": "assistant",
  "model": "claude-3-opus",
  "content": [
    {"type": "text", "text": "Hello! How can I help you today?"}
  ],
  "stop_reason": "end_turn",
  "usage": {
    "input_tokens": 12,
    "output_tokens": 10
  }
}
```

**Response (streaming, `stream: true`)**:

`Content-Type: text/event-stream`, the standard Anthropic SSE event stream (`event: ...\ndata: {...}\n\n`).

**Error codes**: `validation_error`(400) | `routing_error`(500) | `vendor_unavailable`(503) | `timeout`(504) | `protocol_error`(502) | `rate_limited`(429); with `[routing].retry_after_hint` on and all candidates rate-limited → `all_vendors_throttled`(429 + `Retry-After` header)

**curl example**:

```bash
curl -X POST http://localhost:8080/v1/messages \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-opus",
    "max_tokens": 1024,
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

---

### POST /v1/responses

An OpenAI Responses API compatible endpoint (non-streaming + semantic SSE). The request converts to the unified chat IR by default, entering the existing weighted/affinity/circuit/quota routing pool — both OpenAI chat and Anthropic messages vendors can be hit.

**Auth**: Bearer Token required

**Request fields**

| Field | Type | Required | Notes |
|---|---|---|---|
| `model` | string | yes | routing abstract name (`abstract_id`, falling back to `model_id`) |
| `input` | string \| array | yes | a string or item array (message / function_call / function_call_output / reasoning / item_reference) |
| `instructions` | string | no | system instructions (IR `system_prompt`) |
| `max_output_tokens` | integer | no | 1–1,000,000; default decided by the upstream |
| `temperature` / `top_p` | number | no | sampling parameters |
| `stream` | boolean | no | `true` returns semantic SSE |
| `tools` | array | no | flat function tools (`{type,name,description,parameters,strict}`); built-in tools (`web_search`/`file_search`/`code_interpreter`/`image_generation`/`mcp`) are native-passthrough only |
| `tool_choice` | string \| object | no | `auto`/`none`/`required`/`{type:"function",name}` |
| `text.format` | object | no | structured output (`text`/`json_object`/`json_schema`) |
| `reasoning.effort` | string | no | `low`/`medium`/`high` |
| `parallel_tool_calls` | boolean | no | parallel tool calls |
| `store` | boolean | no | default `true`; `false` does not persist and the response cannot be referenced by `previous_response_id` |
| `previous_response_id` | string | no | the gateway replays that response's and its ancestors' input/output items |
| `conversation` | string | no | replays the session's full history (mutually exclusive with `previous_response_id`) |
| `metadata` | object | no | ≤16 entries, keys ≤64 / values ≤512 chars; echoed verbatim |
| `truncation` | string | no | with `auto`, coarsely drops the oldest items per the catalog `context_window` |
| `background` | boolean | no | executes in the gateway background; requires `store=true` and does not support `stream` |

**input item shapes**

- `{"role":"user","content":"..."}` or `{"role":"user","content":[{"type":"input_text","text":"..."},{"type":"input_image","image_url":"...","detail":"auto"}]}`
- `{"type":"function_call","call_id":"...","name":"...","arguments":"..."}`
- `{"type":"function_call_output","call_id":"...","output":"..."}`
- `{"type":"item_reference","id":"..."}` (resolved by id within the replayed history)

**Response shape (non-streaming)**

```json
{
  "id": "resp_...",
  "object": "response",
  "created_at": 1760000000,
  "status": "completed",
  "model": "gpt-4.1",
  "output": [
    {"type": "message", "id": "msg_...", "status": "completed", "role": "assistant",
     "content": [{"type": "output_text", "text": "...", "annotations": []}]},
    {"type": "function_call", "id": "fc_...", "call_id": "call_...", "name": "f", "arguments": "{}", "status": "completed"}
  ],
  "usage": {"input_tokens": 10, "output_tokens": 5, "total_tokens": 15,
            "input_tokens_details": {"cached_tokens": 2}},
  "store": true,
  "metadata": {}
}
```

`finish_reason=length` → `status: "incomplete"` + `incomplete_details.reason: "max_output_tokens"`; `content_filter` likewise.

**Streaming events (the `event:` name equals `data.type`; `sequence_number` increments monotonically from 0)**

| Event | Notes |
|---|---|
| `response.created` / `response.in_progress` | opening (carrying the in_progress response object) |
| `response.output_item.added` | a message / function_call item opens |
| `response.content_part.added` | a message's output_text part opens |
| `response.output_text.delta` / `.done` | text increments / closure |
| `response.function_call_arguments.delta` / `.done` | tool-argument increments / closure |
| `response.content_part.done` / `response.output_item.done` | part / item closure |
| `response.completed` / `response.incomplete` / `response.failed` | terminal events (exactly once, carrying the full response and usage) |

Upstream errors mid-stream → terminate after `response.failed`; no `[DONE]` is sent.

**Native passthrough**: built-in tools, `prompt` templates and `input_file`/audio inputs are allowed only to `api_style="responses"` vendors (`X-Vendor`/`X-Model` explicit-target mismatch → 502 without sending); no candidate → 400 with config guidance. The body is rewritten (`input` as the replayed items, `previous_response_id`/`conversation` stripped, upstream `store=false`) then posted verbatim to `{api_base}/responses`.

**Error codes**: `validation_error`(400) | `protocol_error`(502) | `vendor_unavailable`(503) | `timeout`(504) | `rate_limited`(429); `[responses].enabled=false` → 404.

**curl example**

```bash
curl -X POST http://localhost:8080/v1/responses \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4.1","input":"用一句话解释重排","max_output_tokens":128}'
```

---

### GET /v1/responses/:id

Retrieves a stored Responses object by id (tenant-isolated: only records written under the same API key digest are visible). Missing / unauthorized / expired → 404.

---

### DELETE /v1/responses/:id

Deletes a stored response (also aborting a running background task). Success returns `{"id":"...","object":"response.deleted","deleted":true}`; missing / unauthorized → 404.

---

### POST /v1/responses/:id/cancel

Cancels a background task: sets `status:"cancelled"`, aborts the task and returns the cancelled response object; non-background responses → 400.

---

### POST /v1/responses/input_tokens

An approximate input-token estimate (chars/4 + structural overhead, deterministic). Inputs `{model, input, instructions?, tools?}`, returning `{"object":"response.input_tokens","input_tokens":N}`; missing `model` or empty `input` → 400.

---

### Gemini native endpoints

Google's official SDK can point its base URL at the gateway root and use the Gemini native protocol directly. Inbound parses `model:action` within the same segment (`model` with or without the `models/` prefix).

| Method and path | Notes |
|---|---|
| `POST /v1beta/models/<model>:generateContent` | non-streaming conversation (text/multimodal/function calling/structured output/thinking) |
| `POST /v1beta/models/<model>:streamGenerateContent` | streaming conversation (SSE; `text/event-stream` regardless of `alt=sse`) |
| `POST /v1beta/models/<model>:countTokens` | local approximation (chars/4 + structural overhead, no upstream call) |
| `POST /v1beta/models/<model>:embedContent` | single embedding (converted to OpenAI embeddings shape, routed to OpenAI-type vendors) |
| `POST /v1beta/models/<model>:batchEmbedContents` | batch embeddings (as above; `outputDimensionality` must be consistent across the batch) |
| `GET /v1beta/models` | a Gemini-style list of routable models (`abstract_id` preferred, deduplicated, `supportedGenerationMethods`) |
| `GET /v1beta/models/<model>` | single-model metadata (unknown model 404) |

**Auth**: `x-goog-api-key` or `?key=` on `/v1beta/` paths (the query key is restricted to that prefix and never written to access logs). Auth failures return Gemini-shaped error bodies.

**Default routing**: requests convert to the unified chat IR, entering the existing weighted/affinity/difficulty/circuit/quota routing pool (OpenAI/Anthropic/Responses/Gemini vendors can all be hit). Gemini-specific fields:

- `safetySettings` / `cachedContent` / non-function tools (`googleSearch`, `codeExecution` etc.) → **native passthrough**: routing only to `type_ = "gemini"` vendors with the body posted verbatim upstream; no candidate → 400 with config guidance.
- `generationConfig.responseSchema` → IR `response_format` (json_schema); `thinkingConfig.thinkingBudget` (0=disabled, >0=budget, <0=dynamic) and `includeThoughts` → the IR thinking slot.
- `functionDeclarations` / `toolConfig.functionCallingConfig` → IR tools/tool_choice; `functionCall` gets a stable call ID in the IR, with `functionResponse` associated by `name`.
- `taskType` / `title` (embeddings) have no IR-equivalent slot and are dropped when converting to the OpenAI shape.

**Example (generateContent)**:

```bash
curl http://localhost:8080/v1beta/models/gemini-2.5-pro:generateContent \
  -H "x-goog-api-key: $FLUXWEIR_API_KEY" \
  -H "content-type: application/json" \
  -d '{
    "systemInstruction": {"parts": [{"text": "回答不超过 80 字"}]},
    "contents": [{"role": "user", "parts": [{"text": "解释重排与嵌入的区别"}]}],
    "generationConfig": {"maxOutputTokens": 256}
  }'
```

**Response shape**:

```json
{
  "candidates": [{
    "content": {"role": "model", "parts": [{"text": "…"}]},
    "finishReason": "STOP",
    "index": 0
  }],
  "usageMetadata": {
    "promptTokenCount": 12,
    "candidatesTokenCount": 34,
    "totalTokenCount": 46,
    "cachedContentTokenCount": 0
  },
  "modelVersion": "gemini-2.5-pro",
  "responseId": "resp_…"
}
```

**Streaming semantics**: each frame is one `data: <GenerateContentResponse>`, with no `event:` names and no `[DONE]`; function calls are delivered in full on the terminating frame; `usageMetadata` appears on the final frame.

**Limitations**:

- Live API (WebSocket), Files API, cachedContents CRUD and the Batches API are not supported.
- `?key=` is recommended only for Google SDK compatibility; prefer the `x-goog-api-key` header in production, keeping keys out of proxy access logs.
- Plain http(s) image URLs pass through as `fileData.fileUri`, but Gemini accepts only Files API / `gs://` URIs; the gateway does no server-side fetching (avoiding SSRF).

### GET /v1/models

An OpenAI model-list compatible endpoint. The model set = the `[[model_catalog]]` catalog ∪ vendors' configured models, deduplicated (only enabled models of enabled vendors; same id appears once, sorted by id). Vendor entries list the **routable public name**: models declaring `abstract_id` list `abstract_id`, undeclared ones list `model_id` — the `id`s in the list can be used verbatim in `/v1/chat/completions`, `/v1/messages` and the `X-Model` header; the qualified `vendor.model_id` form also routes (ID matching case-insensitive) but does not appear in the list (avoiding listing bare upstream names that would 400 on feedback). The response is always computed live and never enters the response cache (no `X-Cache` header).

**Auth**: Bearer Token required

**Query parameters**:

| Parameter | Type | Required | Notes |
|---|---|---|---|
| `task` | `chat` \| `embedding` \| `rerank` | no | filter by task type (case-insensitive; unfiltered by default; invalid values → 400 `validation_error`) |

**Response**:

```json
{
  "object": "list",
  "data": [
    {"id": "gpt-4", "object": "model", "owned_by": "openai"},
    {"id": "my-catalog-model", "object": "model", "owned_by": "fluxweir"}
  ]
}
```

| Field | Notes |
|---|---|
| `data[].id` | model ID |
| `data[].object` | always `"model"` |
| `data[].owned_by` | the claiming vendor's ID; catalog-exclusive entries are `"fluxweir"` |

**curl example**:

```bash
curl http://localhost:8080/v1/models \
  -H "Authorization: Bearer $API_KEY"
```

---

### POST /v1/messages/count_tokens

An Anthropic-compatible token counting endpoint. Returns an approximate token estimate of the input (chars/4 + per-message structural overhead; deterministic: the same input yields the same result twice; no tokenizer dependency).

**Auth**: Bearer Token required

**Request body** (`Json<Value>`):

```json
{
  "model": "gpt-4",
  "messages": [{"role": "user", "content": "Hello"}],
  "system": "You are helpful.",
  "tools": [{"type": "function", "function": {"name": "get_weather"}}}]
}
```

| Field | Type | Required | Notes |
|---|---|---|---|
| `model` | string | yes | model ID (existence-checked only, not registered-checked) |
| `messages` | array | yes | non-empty message list (`content` accepts a string or a content-part array) |
| `system` | string \| array | no | system prompt (counted in the estimate) |
| `tools` | array | no | tool schemas (serialized char counts counted in the estimate) |

**Response**:

```json
{"input_tokens": 23}
```

**Error codes**: `validation_error`(400, `model` missing/non-string, `messages` missing/non-array/empty)

**curl example**:

```bash
curl -X POST http://localhost:8080/v1/messages/count_tokens \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-4", "messages": [{"role": "user", "content": "Hello"}]}'
```

---

## Admin endpoints

All `/admin/*` endpoints require Bearer Token auth and are subject to the inbound rate-limit policy (in `config.toml`, `[[rate_limit.rules]]`'s `path` prefix rules can cover `/admin/*`; with no endpoint rules configured, the global layer falls back to a shared `TokenBucket(100, 10/s)`).

### GET /admin/logs

Queries request logs (the `request_logs` table) by condition, returned newest-first by timestamp.

**Auth**: Bearer Token required

**Query parameters**:

| Parameter | Type | Required | Default | Notes |
|---|---|---|---|---|
| `start_time` | integer (unix seconds) | no | - | start timestamp |
| `end_time` | integer (unix seconds) | no | - | end timestamp (a closed interval's upper bound, inclusive — rows with `timestamp = end_time` return) |
| `vendor` | string | no | - | exact match on the actual vendor ID |
| `model` | string | no | - | exact match on the actual model ID |
| `limit` | integer | no | `100` | max rows returned; `0` means unlimited (implementation clamps to a cap of 1000) |

**Response**: a `RequestLog` array

```json
[
  {
    "id": 1,
    "request_id": "550e8400-e29b-41d4-a716-446655440000",
    "timestamp": 1700000000,
    "endpoint": "/v1/chat/completions",
    "input_format": "openai",
    "abstract_model": "gpt-4",
    "messages": "[{\"role\":\"user\",\"content\":\"Hello\"}]",
    "actual_vendor": "mock-openai",
    "actual_model": "gpt-4",
    "routing_strategy": "Weighted",
    "status_code": 200,
    "response": "{...}",
    "error_message": null,
    "prompt_tokens": 1024,
    "completion_tokens": null,
    "total_tokens": null,
    "duration_ms": 523,
    "cost_usd": null,
    "upstream_key_fingerprint": "a1b2c3d4e5f60718",
    "session_key": null,
    "context_window_tokens": null,
    "cache_read_tokens": null,
    "cache_write_tokens": null
  }
]
```

> `cost_usd` persists as the `[[model_catalog]]` four-tier sum × catalog prices (rerank priced per 1k docs); unregistered prices write `0`, missing usage writes `null`. For breakdown aggregation see [GET /admin/cost](#get-admincost).
>
> `upstream_key_fingerprint`: the fingerprint of the upstream-pool key that **actually served this request** (recorded after fallback failover names the real hitter — failover attribution does not lie). `null` means rows written before migration V9, or requests that bypassed the key pool (legacy env keys / subscription-account credentials). Field-comment single source: `src/storage.rs`.

**Error codes**: `database_error`(500)

**curl example**:

```bash
curl "http://localhost:8080/admin/logs?vendor=mock-openai&limit=10" \
  -H "Authorization: Bearer $API_KEY"
```

---

### GET /admin/logs/search

Fuzzy-searches logs' `messages` and `error_message` fields by keyword (`LIKE %keyword%`).

**Auth**: Bearer Token required

**Query parameters**:

| Parameter | Type | Required | Default | Notes |
|---|---|---|---|---|
| `vendor` | string | no | `""` (empty) | the search keyword (the field name carries over from `vendor` but is actually the search keyword) |
| `limit` | integer | no | `100` | max rows returned |
| `start_time` / `end_time` / `model` | - | no | - | accepted but not part of the search logic |

> Note: this endpoint reuses the `LogQuery` struct, with the `vendor` field doubling as the search keyword.

**Response**: a `RequestLog` array (same shape as [GET /admin/logs](#get-adminlogs))

**Error codes**: `database_error`(500)

**curl example**:

```bash
curl "http://localhost:8080/admin/logs/search?vendor=Hello&limit=50" \
  -H "Authorization: Bearer $API_KEY"
```

---

### GET /admin/stats/tokens

Queries token usage statistics (the `token_usage` table, pre-aggregated by date and hour).

**Auth**: Bearer Token required

**Query parameters**:

| Parameter | Type | Required | Default | Notes |
|---|---|---|---|---|
| `vendor` | string | no | - | vendor filter; without it, everything is returned |
| `start_time` / `end_time` / `model` / `limit` | - | no | - | accepted but not part of the query logic |

**Response**: a `TokenUsage` array (sorted by `date DESC, hour DESC`; `prompt_tokens`=`input_tokens`, `completion_tokens`=`output_tokens`, `total_tokens`=the sum of the four classes, `cost_usd_sum`=the sum of the four costs — breakdown details in [GET /admin/cost](#get-admincost))

```json
[
  {
    "id": 1,
    "vendor": "mock-openai",
    "model": "gpt-4",
    "api_key_hash": "d6593c4e…",
    "date": "2026-07-14",
    "hour": 10,
    "request_count": 42,
    "total_tokens": 12000,
    "prompt_tokens": 8000,
    "completion_tokens": 4000,
    "cost_usd_sum": 0.36,
    "cache_read_tokens": 500,
    "cache_creation_tokens": 0,
    "input_cost": 0.24,
    "output_cost": 0.12,
    "cache_read_cost": 0.0,
    "cache_creation_cost": 0.0
  }
]
```

**Error codes**: `database_error`(500)

**curl example**:

```bash
curl "http://localhost:8080/admin/stats/tokens?vendor=mock-openai" \
  -H "Authorization: Bearer $API_KEY"
```

---

### GET /admin/cost

Cost-breakdown aggregation queries (the `token_usage` pre-aggregated table, with the four token classes and per-class costs summed per dimension). rerank request costs file under `input_cost` with token columns 0. Empty windows return an empty array, not an error.

**Auth**: Bearer Token required

**Query parameters**:

| Parameter | Type | Required | Notes |
|---|---|---|---|
| `group_by` | `vendor` \| `model` | yes | grouping dimension (invalid values 400) |
| `from` / `to` | `YYYY-MM-DD` | no | a closed date interval (format errors 400) |

**Request header** (optional filter):

| Header | Notes |
|---|---|
| `x-fluxweir-cost-key` | the requesting key verbatim (hashed server-side to filter that key's breakdown; empty/absent = no filter). Never passed as a query parameter, keeping keys out of access logs |

**Response**:

```json
{
  "group_by": "vendor",
  "rows": [
    {
      "vendor": "mock-openai",
      "model": null,
      "request_count": 42,
      "input_tokens": 8000,
      "output_tokens": 4000,
      "cache_read_tokens": 500,
      "cache_creation_tokens": 0,
      "input_cost": 0.24,
      "output_cost": 0.12,
      "cache_read_cost": 0.0,
      "cache_creation_cost": 0.0,
      "total_cost": 0.36
    }
  ]
}
```

**Error codes**: `validation_error`(400, invalid/missing `group_by`, bad date formats) | `database_error`(500)

**curl example**:

```bash
curl "http://localhost:8080/admin/cost?group_by=vendor&from=2026-10-01&to=2026-10-09" \
  -H "Authorization: Bearer $API_KEY" \
  -H "x-fluxweir-cost-key: $FILTER_KEY"
```

---

### GET /admin/sessions

Aggregates cost and tokens per session (available when `[usage_insights]` is on; 404 when off).

**Query parameters**

| Parameter | Required | Notes |
|:-----|:-----|:-----|
| `limit` | no | max sessions returned, default 50, cap 500 |
| `window_hours` | no | look-back hours, default 168 (7 days), cap 8760 |

**Response**

```json
{
  "limit": 20,
  "window_hours": 168,
  "rows": [
    {
      "session_key": "sess-abc",
      "request_count": 3,
      "prompt_tokens": 600,
      "completion_tokens": 30,
      "cache_read_tokens": 60,
      "cache_write_tokens": 6,
      "total_cost": 1.5,
      "first_at": 1761436800,
      "last_at": 1761436802,
      "title": "first message from user"
    }
  ]
}
```

> **Caliber**: only requests with `status_code = 200` count; rows with a `NULL` `session_key`
> are excluded (no session semantics). `title` takes the first 80 characters of the session's first
> `role = "user"` message. With `[usage_insights]` off, the endpoint returns 404 (`error.type = "not_found"`).

---

### GET /admin/context-stats

Aggregates context-window occupancy, cache-read share and the five-bucket distribution per model.

**Query parameters**: `window_hours` (as above, default 168).

**Response**

```json
{
  "window_hours": 168,
  "rows": [
    {
      "model": "claude-sonnet-4",
      "request_count": 5,
      "avg_occupancy": 0.64,
      "cache_read_share": 0.5,
      "bucket_counts": [1, 0, 1, 1, 2]
    }
  ]
}
```

> **Caliber**: occupancy = `(prompt + cache_write) / context_window` — `cache_read` is excluded
> to avoid double counting for OpenAI/Gemini (Inclusive: prompt already includes cache reads); Anthropic
> (Exclusive) is therefore a lower bound. `bucket_counts`' five buckets break at 20%/40%/60%/80%.
> `cache_read_share` can exceed 1 under Exclusive protocols. Models without a catalog window are excluded.

---

### GET /admin/metrics

Returns the global metrics snapshot (from `GlobalMetrics`: request counts, error rates, average latency, per-vendor breakdowns, session-affinity hit rate and traffic-share entropy), plus log-backpressure drop counters and response-cache statistics.

**Auth**: Bearer Token required

**Query parameters**: none

**Response**: a `MetricsSnapshot` (plus the `dropped_logs`, `secret_scan_dropped`, `affinity_persist_dropped`, `affinity_flush_failures` and `cache` fields)

```json
{
  "total_requests": 1000,
  "successful_requests": 950,
  "failed_requests": 50,
  "error_rate": 0.05,
  "avg_duration_ms": 320.5,
  "vendors": [
    {
      "vendor": "mock-openai",
      "requests": 700,
      "successes": 680,
      "failures": 20,
      "total_duration_ns": 224000000000,
      "last_request_at": 1700000000,
      "last_error": "Connection reset",
      "ewma_latency_ms": 320.0,
      "ewma_error_rate": 0.02,
      "ewma_samples": 700,
      "ewma_ttft_ms": 210.0,
      "ewma_e2e_ms": 318.0,
      "ewma_queue_ms": 2.5,
      "ttft_samples": 350,
      "e2e_samples": 690,
      "queue_samples": 690
    }
  ],
  "affinity_hits": 420,
  "affinity_misses": 580,
  "affinity_hit_rate": 0.42,
  "affinity_stick_total": 12,
  "affinity_drift_total": 3,
  "affinity_persist_drop_total": 0,
  "affinity_restored_count": 8,
  "embedding_pin_set_total": 0,
  "embedding_pin_reset_total": 0,
  "embedding_group_exhausted_total": 0,
  "embedding_dimension_mismatch_total": 0,
  "rerank_requests_total": 0,
  "rerank_documents_total": 0,
  "rerank_upstream_failures_total": 0,
  "responses_requests_total": 0,
  "responses_upstream_failures_total": 0,
  "responses_stored_total": 0,
  "responses_store_failures_total": 0,
  "responses_background_jobs_total": 0,
  "gemini_requests_total": 0,
  "gemini_upstream_failures_total": 0,
  "quota_window_refused_total": [],
  "quota_factor_depleted_total": [],
  "quota_ledger_flush_fail_total": 0,
  "catalog_sync_fail_total": [],
  "catalog_sync_added_total": 0,
  "subscription_refresh_fail_total": 0,
  "subscription_accounts_degraded": 0,
  "web_search_requests_total": [],
  "netguard_denied_total": [],
  "difficulty": {
    "fusion": 0,
    "rule_only": 0,
    "cache_hit": 0,
    "classifier_failed": 0,
    "features_degraded": 0,
    "breaker_opened": 0,
    "breaker_recovered": 0,
    "tier_low": 0,
    "tier_mid": 0,
    "tier_high": 0,
    "avg_latency_ms": 0.0
  },
  "top1_share": 0.7,
  "share_entropy": 0.88,
  "dropped_logs": 0,
  "secret_scan_dropped": 0,
  "affinity_persist_dropped": 0,
  "affinity_flush_failures": 0,
  "cache": {
    "l1_hits": 420,
    "l1_misses": 580,
    "l1_sets": 580,
    "l1_deletes": 12,
    "l2_hits": 0,
    "l2_misses": 0,
    "l2_sets": 0,
    "l2_deletes": 0,
    "total_operations": 1592,
    "l1_item_count": 300,
    "l1_capacity_used": 300,
    "prefetch_count": 0,
    "compression_count": 0,
    "compression_bytes_saved": 0,
    "timestamp": "2026-01-01T00:00:00Z"
  }
}
```

Top-level fields:

| Field | Type | Notes |
|---|---|---|
| `total_requests` | integer | total requests |
| `successful_requests` | integer | successful requests |
| `failed_requests` | integer | failed requests |
| `error_rate` | number | error rate |
| `avg_duration_ms` | number | average duration (ms) |
| `vendors` | array | per-vendor breakdowns |
| `affinity_hits` / `affinity_misses` | integer | session-affinity hits / misses |
| `affinity_hit_rate` | number | affinity hit rate ([0,1]; 0.0 with no samples) |
| `affinity_stick_total` | integer | cache-economy affinity stickiness-write total (`[routing.affinity]`: legacy successful settlements +1, auto worth-sticking +1, turn stuck +1; `off` writes nothing and counts nothing) |
| `affinity_drift_total` | integer | should-have-written-but-didn't total (auto with worth below `cache_worth_tokens`, the session returning to the routing pool) |
| `affinity_persist_drop_total` | integer | stickiness-persistence channel-full drop total (a settle-sampled approximation; `affinity_persist_dropped` is the channel-side exact reading) |
| `affinity_persist_dropped` | integer | the channel-side exact drop reading (an added field, matching the `dropped_logs` dual-exit convention) |
| `affinity_flush_failures` | integer | persistence-failed batch total (UPSERT batches dropped whole; listed separately from channel drops — different causes) |
| `affinity_restored_count` | integer | sticky entries refilled into the in-memory affinity table at startup |
| `embedding_pin_set_total` / `embedding_pin_reset_total` / `embedding_group_exhausted_total` / `embedding_dimension_mismatch_total` | integer | embedding compat-group pin writes / resets / in-group candidate exhaustion / dimension mismatches |
| `rerank_requests_total` / `rerank_documents_total` / `rerank_upstream_failures_total` | integer | rerank requests / documents / upstream failures |
| `responses_requests_total` / `responses_upstream_failures_total` / `responses_stored_total` / `responses_store_failures_total` / `responses_background_jobs_total` | integer | Responses API requests / upstream failures / stores / store failures / background jobs |
| `gemini_requests_total` / `gemini_upstream_failures_total` | integer | Gemini requests / upstream failures |
| `quota_window_refused_total` | array | quota-window rejection totals (by key digest, `Vec<(String, u64)>`) |
| `quota_factor_depleted_total` | array | quota-factor tier-exhaustion totals (`Vec<(String, u64)>`) |
| `quota_ledger_flush_fail_total` | integer | quota-ledger persistence failure total |
| `catalog_sync_fail_total` / `catalog_sync_added_total` | array \| integer | catalog-sync failure totals (by source) and added entries (`catalog-sync` feature) |
| `subscription_refresh_fail_total` / `subscription_accounts_degraded` | integer | subscription refresh failure totals / degraded accounts (`subscription-vendor` feature) |
| `web_search_requests_total` | array | web-search request counts (by source, `Vec<(String, u64)>`) |
| `netguard_denied_total` | array | netguard SSRF interception counts (by path, `Vec<(String, u64)>`) |
| `difficulty` | object | difficulty-tier snapshot: `fusion`/`rule_only`/`cache_hit`/`classifier_failed`/`features_degraded`/`breaker_opened`/`breaker_recovered`/`tier_low`/`tier_mid`/`tier_high` (all integer) and `avg_latency_ms` (number) |
| `top1_share` | number | the sliding-window top-1 vendor's traffic share ([0,1]) |
| `share_entropy` | number | the traffic share's Shannon entropy (normalized [0,1]) |
| `dropped_logs` | integer | log rows dropped when the log channel was full |
| `secret_scan_dropped` | integer | outbound strings intercepted by the secret gate |
| `cache` | object | response-cache statistics (oxcache's global recorder; see below) |

The `cache` object's fields:

| Field | Type | Notes |
|---|---|---|
| `l1_hits` / `l1_misses` | integer | L1 hits / misses (their ratio is the cache hit rate) |
| `l1_sets` / `l1_deletes` | integer | L1 writes / deletes |
| `l2_hits` / `l2_misses` / `l2_sets` / `l2_deletes` | integer | the L2 counterparts (this repo's single-layer memory backend keeps them 0) |
| `total_operations` | integer | total recorded operations |
| `l1_item_count` / `l1_capacity_used` | integer | L1's current entries / used capacity |
| `prefetch_count` / `compression_count` / `compression_bytes_saved` | integer | prefetch / compression counts and bytes saved (not enabled in this repo, always 0) |
| `timestamp` | string | snapshot time (RFC 3339) |

Each element of the `vendors` array:

| Field | Type | Notes |
|---|---|---|
| `vendor` | string | vendor ID |
| `cache_read_tokens_sum` / `cache_input_tokens_sum` | integer | cache-read / cache-input token totals |
| `requests` | integer | total requests |
| `successes` | integer | successful requests |
| `failures` | integer | failed requests |
| `total_duration_ns` | integer | cumulative duration (ns) |
| `last_request_at` | integer \| null | latest request unix timestamp |
| `last_error` | string \| null | latest error description |
| `ewma_latency_ms` | number | EWMA latency (ms, e2e caliber) |
| `ewma_error_rate` | number | EWMA error rate ([0,1]) |
| `ewma_samples` | integer | EWMA sample count (0 = no observations yet) |
| `ewma_ttft_ms` | number | streaming TTFT EWMA (ms, approximated from upstream response headers) |
| `ewma_e2e_ms` | number | non-streaming end-to-end EWMA (ms, incl. queueing) |
| `ewma_queue_ms` | number | the queueing segment's EWMA (ms) |
| `ttft_samples` / `e2e_samples` / `queue_samples` | integer | each lane's sample count |

**Error codes**: no business errors (only a possible rate-limit 429)

**curl example**:

```bash
curl http://localhost:8080/admin/metrics \
  -H "Authorization: Bearer $API_KEY"
```

---

### GET /admin/quota/ledger

Quota-ledger query: returns the current fixed windows' pre-deducted/actually-deducted balance surface per key digest (the ledger read side under `[quota]` or `[key_budgets]`; `__global__` / `__vendor__:{id}` are shared ledger keys).

**Auth**: Bearer Token required

**Response**: an array of ledger entries (key digest, window bounds, token/cost headroom etc.; the shape follows the `[quota]`/`[key_budgets]` configuration)

**Error codes**: no business errors (an empty collection when quota features are off)

---

### GET /admin/vendor-budgets

Vendor declarative budget query: the current-window consumption and headroom observability surface for each `[[vendor_budgets]]` entry.

**Auth**: Bearer Token required

**Response**: an array of budget entries (vendor, window, used/limit)

**Error codes**: no business errors (an empty collection when no budgets are declared)

---

### GET /admin/vendors

Returns all registered vendors' live status and queue lengths.

**Auth**: Bearer Token required

**Query parameters**: none

**Response**: a vendor-status array

```json
[
  {
    "id": "mock-openai",
    "status": "Available",
    "queue_length": 0,
    "key_pool": {
      "envs": ["MOCK_OPENAI_KEY"],
      "size": 1,
      "current_index": 0,
      "fingerprints": ["a1b2c3d4e5f60718"]
    }
  },
  {
    "id": "mock-anthropic",
    "status": "Busy",
    "queue_length": 3,
    "key_pool": null
  }
]
```

`status` is the `Debug` output of the `VendorStatus` enum; possible values: `Available` | `Busy` | `Throttled` | `Unavailable`.

`key_pool` is the vendor key pool's observability info (`KeyPoolInfo`: the env-name list / the count of successfully resolved keys / the current cursor index / each key's 16-char hex fingerprint — **never the key verbatim**); `null` when the vendor has no key-pool info.

Status is computed live: a vendor held by a rate-limit window shows `Throttled`; a circuit breaker (the `[resilience]` feature) Open and before its probe window shows `Unavailable` (probe traffic allowed once the window passes); an unknown vendor ID or `enabled = false` likewise shows `Unavailable`.

**Error codes**: no business errors

**curl example**:

```bash
curl http://localhost:8080/admin/vendors \
  -H "Authorization: Bearer $API_KEY"
```

---

### GET /admin/vendors/config

Returns all vendors' **raw configuration** (`Vec<VendorConfig>`, the config view). Complementary to [GET /admin/vendors](#get-adminvendors)'s status-observation view (live key-pool/queue info): this endpoint surfaces the config file's static fields (`priority`/`weight`/`timeout_ms`/`models` etc.) for console edit-page form backfill.

**Auth**: Bearer Token required

**Query parameters**: none

**Response**: a `VendorConfig` array (fields as in the `[[vendors]]` config section)

**Error codes**: no business errors

**curl example**:

```bash
curl http://localhost:8080/admin/vendors/config \
  -H "Authorization: Bearer $API_KEY"
```

---

### PUT /admin/vendors/:id

Atomically updates one vendor's config by id: read-modify-validate-persist inside the write lock (mutually exclusive with other `update_*` endpoints, no lost updates; validation failures do not persist). The body is a complete `VendorConfig` (whole-replace semantics, not an incremental patch).

**Auth**: Bearer Token required

**Path parameters**:

| Parameter | Type | Notes |
|---|---|---|
| `id` | string | vendor ID (must match an existing entry; unknown ids return 400 `validation_error` naming the id) |

**Request body**: a complete `VendorConfig` (fields as in the `[[vendors]]` config section; illegal values rejected by the existing config validation, leaving the old config untouched)

**Response**:

```json
{
  "success": true,
  "message": "厂商已更新"
}
```

**Error codes**:

| Status | Scenario |
|---|---|
| 400 | the id does not exist (`admin-vendor-not-found`) or config validation rejected |
| 4xx | request-body deserialization failures (wrong field types etc.) |

**curl example**:

```bash
curl -X PUT http://localhost:8080/admin/vendors/mock-openai \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"id":"mock-openai","type_":"openai","api_base":"http://localhost:8001/v1","priority":9,...}'
```

---

### GET /admin/vendors/:id/status

Returns the named vendor's live status and queue length.

**Auth**: Bearer Token required

**Path parameters**:

| Parameter | Type | Notes |
|---|---|---|
| `id` | string | vendor ID |

**Response**:

```json
{
  "id": "mock-openai",
  "status": "Available",
  "queue_length": 0,
  "key_pool": {
    "envs": ["MOCK_OPENAI_KEY"],
    "size": 1,
    "current_index": 0,
    "fingerprints": ["a1b2c3d4e5f60718"]
  }
}
```

With an unknown vendor ID, `status` returns `"Unavailable"`. Status semantics as in [GET /admin/vendors](#get-adminvendors); `key_pool` semantics as above.

**Error codes**: no business errors

**curl example**:

```bash
curl http://localhost:8080/admin/vendors/mock-openai/status \
  -H "Authorization: Bearer $API_KEY"
```

---

### POST /admin/vendors/:id/reset

Resets the vendor's key-pool blacklist (incl. cursor) and cooldown state (the key-pool three-state recovery channel; config hot reload trimming per key-set changes is the other). The rate limiter's state remains managed automatically by `limiteron` (token-bucket refill) — this endpoint does not touch the limiter.

**Auth**: Bearer Token required

**Path parameters**:

| Parameter | Type | Notes |
|---|---|---|
| `id` | string | vendor ID (unknown vendors return `validation_error`) |

**Request body**: none

**Response**:

```json
{
  "success": true,
  "message": "厂商 mock-openai 的密钥池黑名单与冷却状态已清空"
}
```

**Error codes**: `validation_error`(400, unknown vendor)

**curl example**:

```bash
curl -X POST http://localhost:8080/admin/vendors/mock-openai/reset \
  -H "Authorization: Bearer $API_KEY"
```

---

### GET /admin/embedding-pins

A snapshot of the embedding compat-group pin table (`abstract_id` ascending), observing the current state of `POST /v1/embeddings`' compat-group pinned routing.

**Auth**: Bearer Token required

**Request body**: none

**Response**:

```json
[
  { "abstract_id": "text-embedding-small", "group_key": "openai-embedding-v1" },
  { "abstract_id": "emb-other", "group_key": "vendor-b/::nomic-embed-text" }
]
```

`group_key` is the entry's declared `compat_group` (trimmed); without a declared group it is the singleton key `{vendor_id}/::{model_id}` (the safe default: interchangeable with no vendor). With the embedding pin table unwired (SDK minimal assembly), returns `config_error`.

**curl example**:

```bash
curl http://localhost:8080/admin/embedding-pins \
  -H "Authorization: Bearer $API_KEY"
```

---

### POST /admin/embedding-pins/:id/reset

Resets a single embedding pin (the recovery channel): clears that abstract model's stickiness record, so the next embedding request re-selects a vendor by weight and re-pins (possibly hitting a vendor previously excluded by the pin); it also **cascades away** all (vendor, model) dimension-gate baselines the abstract model currently resolves to — when the upstream model has truly changed, clearing only the pin without the baseline would make the very next request 502 again; this endpoint is the recovery exit the error message points to. `id` is the abstract model name (normalized the same way as the routing side: trim/lowercase), with any spelling form hitting the same pin; a non-existent abstract succeeds idempotently (`cleared: false`, not counted). After a successful config hot reload, all pins reset automatically (dimension baselines invalidated with them) — no manual call needed.

`embedding_pin_reset_total` is a dual-source counter: admin single-entry resets (+1 only on actual removal) and config-hot-reload clearing (by cleared entry count) share it.

**Auth**: Bearer Token required

**Path parameters**:

| Parameter | Type | Notes |
|---|---|---|
| `id` | string | the abstract model name (the pin key; idempotent success when absent) |

**Request body**: none

**Response**:

```json
{
  "success": true,
  "message": "嵌入 pin 'text-embedding-small' 已清空；下一嵌入请求按权重重新粘着（可能换厂商）",
  "cleared": true
}
```

**Error codes**: `config_error`(500, pin table unwired)

**curl example**:

```bash
curl -X POST http://localhost:8080/admin/embedding-pins/text-embedding-small/reset \
  -H "Authorization: Bearer $API_KEY"
```

---

### GET /admin/config

Returns the current runtime configuration (the full `Config` struct, 30 top-level fields/sections: `server`, `database`, `vendors`, `global_model`, `pricing`, `model_catalog`, `auth`, `cors`, `retry`, `routing`, `cache`, `rate_limit`, `resilience`, `admission`, `health`, `quota`, `vendor_budgets`, `key_budgets`, `responses`, `metrics`, `otlp`, `usage_insights`, `catalog_sync`, `subscription`, `subscription_accounts`, `web_search`, `masking`, `lexcompress`, `grpc`, `docs`; with an empty `model_catalog` or a missing `grpc` section, they are omitted in the response / serialized as `null`).

**Auth**: Bearer Token required

**Response**: `Config` (the example below is a representative excerpt; fields-heavy sections like `routing` list only common entries — the actual return governs)

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080
  },
  "database": {
    "path": "./data/llm_proxy.db",
    "pool_size": 10
  },
  "vendors": [
    {
      "id": "mock-openai",
      "name": "Mock OpenAI",
      "type_": "openai",
      "api_key_env": "MOCK_OPENAI_KEY",
      "api_base": "http://127.0.0.1:8001/v1",
      "enabled": true,
      "priority": 1,
      "models": [
        {
          "model_id": "gpt-3.5-turbo",
          "abstract_id": null,
          "weight": 1.0,
          "enabled": true
        }
      ],
      "rate_limit": {
        "type": "token",
        "max_tokens": 10000000,
        "window_hours": 24,
        "sliding": false
      },
      "concurrency": {
        "requests_per_second": 100,
        "max_concurrent_requests": null,
        "token_weighted": false,
        "token_ticket_min": 1,
        "token_ticket_max": 16384,
        "token_ticket_default": 4096
      },
      "timeout_ms": 60000,
      "stream_ttft_ms": 60000,
      "stream_idle_ms": 30000
    }
  ],
  "global_model": {
    "enabled": true,
    "strategy": "weighted"
  },
  "model_catalog": [
    {
      "model_id": "claude-sonnet-4-20250514",
      "context_window": 200000,
      "input_price": 3.0,
      "output_price": 15.0,
      "cache_read_price": 0.3,
      "cache_creation_price": 3.75,
      "currency": "usd",
      "max_output_tokens": 8192,
      "supports_tools": true,
      "supports_thinking": true
    }
  ],
  "pricing": [
    {
      "vendor_id": "mock-openai",
      "model_id": "gpt-3.5-turbo",
      "input_per_1k_tokens": 0.0005,
      "output_per_1k_tokens": 0.0015
    }
  ],
  "auth": {
    "enabled": true,
    "api_keys": ["<sha256-hex>"],
    "header": {
      "name": "Authorization",
      "prefix": "Bearer"
    },
    "keys": [
      {
        "hash": "<sha256-hex>",
        "tier": "elastic"
      }
    ]
  },
  "cors": {
    "enabled": true,
    "allowed_origins": ["*"],
    "allowed_methods": ["GET", "POST", "OPTIONS"],
    "allowed_headers": ["*"],
    "allow_credentials": false,
    "max_age": 86400
  },
  "retry": {
    "max_retries": 3,
    "retryable_status_codes": [429, 500, 502, 503],
    "initial_delay_ms": 1000,
    "factor": 2.0,
    "max_delay_ms": 16000,
    "jitter": 0.2,
    "spread_jitter": 1.0,
    "max_total_attempts": 8,
    "retry_budget_secs": 0,
    "amplification_gate": false
  },
  "routing": {
    "w_model": 0.4,
    "w_queue": 0.3,
    "w_window": 0.3,
    "w_latency": 0.0,
    "w_error": 0.0,
    "allow_request_weight": false,
    "retry_after_hint": false
  },
  "cache": {
    "enabled": false,
    "ttl_secs": 300
  },
  "rate_limit": {
    "rules": []
  },
  "resilience": {
    "cover_all_paths": false
  },
  "admission": {
    "enabled": false,
    "congestion_threshold": 0.9,
    "alpha_slo": 2.0,
    "alpha_burst": 1.0,
    "alpha_debt": 4.0
  },
  "grpc": null
}
```

`vendors[].rate_limit` is a tagged enum supporting three forms:

```json
{"type": "time_window", "start": "00:00", "end": "06:00"}
{"type": "interval", "hours": 24, "sliding": false}
{"type": "token", "max_tokens": 10000000, "window_hours": 24, "sliding": false}
```

> `[auth]`, `[cors]` and `[grpc]` serialize as `null` when missing from the config file; `pricing` defaults to the empty array `[]`; an empty `model_catalog` is omitted entirely (`skip_serializing_if`). Vendor entries with a multi-key pool include the `api_key_envs` array (env names only, omitted when empty).

**Error codes**: no business errors

**curl example**:

```bash
curl http://localhost:8080/admin/config \
  -H "Authorization: Bearer $API_KEY"
```

---

### POST /admin/config

Updates the runtime configuration wholesale (overwrite write).

**Auth**: Bearer Token required

**Request body**: the complete `Config` struct (same as the [GET /admin/config](#get-adminconfig) response, 30 top-level fields/sections)

**Response**:

```json
{
  "success": true,
  "message": "配置已更新；vendors/pricing/auth 立即生效",
  "restart_required": [],
  "applies_to_new_instances": []
}
```

| Field | Type | Notes |
|---|---|---|
| `success` | boolean | always `true` |
| `message` | string | update-result description (per locale) |
| `restart_required` | array | sections needing a restart to take effect |
| `applies_to_new_instances` | array | config keys effective only for newly created vendor instances (currently only `resilience.circuit_open_timeout_ms` — vendor instances are lazily created per id and cached process-wide, so breaker-window changes do not affect existing instances) |

> Effect semantics: when site-build/assembly-time-consumed sections change, `restart_required` contains the corresponding section names (`"server"` / `"cors"` / `"rate_limit"` / `"database"` / `"grpc"` / `"global_model"` / `"cache.semantic"` / `"docs.enabled"` / `"routing.difficulty.enabled"`), and `message` notes that these sections need a restart — `server` / `cors` / `rate_limit` are baked in at axum Router construction (layers compiled into the Router), `database` resolves connections at build time, `grpc` is baked at server construction, `cache.semantic`'s semantic layer and difficulty-tier routing assemble once in bootstrap, and `global_model` has no hot-read point; `routing.difficulty.classifier`'s `model_path` / `engine` / `cache_ttl_secs` key changes land as `"routing.difficulty.classifier.model_path"` / `"routing.difficulty.classifier.engine"` / `"routing.difficulty.classifier.cache_ttl_secs"` respectively (the classifier assembles once in bootstrap, staying lazy until restart); `resilience.circuit_open_timeout_ms` changes land in `applies_to_new_instances` (effective for newly created vendor instances); remaining sections (vendors/pricing/auth etc.) are hot-effective. vendor `concurrency` (consumed when vendor instances are constructed) and the `vendors` topology are not in the array — schedule restarts yourself.

**Error codes**: `config_error`(500, update failures / deserialization failures / disk config modified out-of-process refusing overwrite write-back — `config-disk-modified-refusing-write`; hot-reload or restart to apply the external edit first, then retry)

**curl example**:

```bash
curl -X POST http://localhost:8080/admin/config \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d @config.toml.json
```

---

### GET /admin/models

Returns the entries of the model capability catalog (the `[[model_catalog]]` section).

> **BREAKING**: the response shape changed from a `GlobalModelConfig` array (the old global_model singleton wrapper) to a `ModelCatalogEntry` array — global model policy moves to [POST /admin/models/:id](#post-adminmodelsid) (`id = "global"`) or [GET /admin/config](#get-adminconfig), no longer part of this list.

**Auth**: Bearer Token required

**Response**: a `ModelCatalogEntry` array (fifteen fields, aligned with agentstem's `ModelEntry`; pricing units = "currency-priced, per 1M tokens"; `cache_breakpoints` omitted entirely by default)

```json
[
  {
    "model_id": "claude-sonnet-4-20250514",
    "context_window": 200000,
    "input_price": 3.0,
    "output_price": 15.0,
    "cache_read_price": 0.3,
    "cache_creation_price": 3.75,
    "rerank_price_per_1k_docs": 0.0,
    "cache_creation_1h_price": 0.0,
    "long_context_input_price": 0.0,
    "long_context_threshold_tokens": 0,
    "currency": "usd",
    "max_output_tokens": 8192,
    "supports_tools": true,
    "supports_thinking": true
  }
]
```

| Field | Type | Notes |
|---|---|---|
| `model_id` | string | the full model name (the catalog's primary key, globally unique) |
| `context_window` | u64 | context window (tokens); registered entries must be > 0 |
| `input_price` / `output_price` | f64 | input/output unit prices (currency, per 1M tokens) |
| `cache_read_price` / `cache_creation_price` | f64 | cache read/write unit prices (same unit; 0 when independently unpriced) |
| `rerank_price_per_1k_docs` | f64 | rerank's per-1k-documents unit price (same unit; 0 = unconfigured) |
| `cache_creation_1h_price` | f64 | the 1h-tier cache-write unit price (same unit; 0.0 = unconfigured. When the upstream response carries the `usage.cache_creation.ephemeral_1h_input_tokens` split, 1h writes bill at this price and remaining writes at the base write price; missing split or unconfigured price → the whole thing at the base write price) |
| `long_context_input_price` / `long_context_threshold_tokens` | f64 / u64 | long-context tiered pricing: when input tokens exceed the threshold, the input leg switches to this price (0/absent = not enabled) |
| `currency` | string | `usd` \| `cny` (one currency shared by all four prices) |
| `max_output_tokens` | u64 \| null | max output tokens (null = unpublished by the vendor) |
| `supports_tools` / `supports_thinking` | bool | capability bits |
| `cache_breakpoints` | object | Anthropic's automatic cache-breakpoint injection policy (optional, serde default; default = the built-in — enabled / cap 4 / anchor order tools→system→last_user); when unconfigured the field is **omitted entirely** from the response (`skip_serializing_if`) |

The entry cap is 1024; capability queries for unregistered models fall back to per-flavor conservative defaults (openai 128k / anthropic 200k / ollama 4096 — see the `config::model_catalog` docs).

**Error codes**: no business errors

**curl example**:

```bash
curl http://localhost:8080/admin/models \
  -H "Authorization: Bearer $API_KEY"
```

---

### PUT /admin/models/:id

Upserts one catalog entry: an existing entry is replaced whole, a missing one appended. Atomic RMW (read-modify-validate-persist inside the write lock), with validation rules (uniqueness / non-zero window / prices finite non-negative (incl. the 5th tier `cache_creation_1h_price`) / the entry cap / `model_id` non-blank) sharing the source with config loading — both paths reject. Pricing surface: `long_context_input_price` + `long_context_threshold_tokens` enable long-context tiered pricing (the input leg switches wholesale once input tokens exceed the threshold, with the same decision condition as the routing factor); `cache_creation_1h_price` is the 1h-tier cache-write price tier (the protocol side already parses the `usage.cache_creation.ephemeral_1h_input_tokens` split: 1h writes bill at this price, the rest at the base write price; missing split or 0.0=unconfigured → the whole thing at the base write price).

**Auth**: Bearer Token required

**Path parameters**:

| Parameter | Type | Notes |
|---|---|---|
| `id` | string | the full model name (**the authoritative identifier** — the body's `model_id` is overwritten with it) |

**Request body**: a `ModelCatalogEntry` (same entry shape as [GET](#get-adminmodels); `model_id` defers to the path)

**Response**:

```json
{
  "success": true,
  "message": "model claude-sonnet-4-20250514 upserted into the catalog"
}
```

**Error codes**: `validation_error`(400, the reserved key `id == "global"` / validation failures) · `config_error`(500, persistence failures)

**curl example**:

```bash
curl -X PUT http://localhost:8080/admin/models/claude-sonnet-4-20250514 \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model_id":"claude-sonnet-4-20250514","context_window":200000,"input_price":3.0,"output_price":15.0,"cache_read_price":0.3,"cache_creation_price":3.75,"currency":"usd","max_output_tokens":8192,"supports_tools":true,"supports_thinking":true}'
```

---

### DELETE /admin/models/:id

Removes a catalog entry. Atomic RMW; a missing entry is explicitly rejected (silent success forbidden).

**Auth**: Bearer Token required

**Path parameters**:

| Parameter | Type | Notes |
|---|---|---|
| `id` | string | the full model name to remove |

**Response**:

```json
{
  "success": true,
  "message": "model claude-sonnet-4-20250514 removed from the catalog"
}
```

**Error codes**: `validation_error`(400, the entry missing or the reserved key `id == "global"`) · `config_error`(500, persistence failures)

**curl example**:

```bash
curl -X DELETE http://localhost:8080/admin/models/claude-sonnet-4-20250514 \
  -H "Authorization: Bearer $API_KEY"
```

---

### POST /admin/models/:id

Updates the global model policy section (`global_model`). Currently only `id == "global"` is supported; other IDs return `config_error`.

> For catalog-entry management use [PUT](#put-adminmodelsid) / [DELETE](#delete-adminmodelsid) `/admin/models/:id`; `"global"` is the policy section's reserved key, and PUT/DELETE on it return `validation_error`(400).

**Auth**: Bearer Token required

**Path parameters**:

| Parameter | Type | Notes |
|---|---|---|
| `id` | string | must be `"global"` |

**Request body**: `GlobalModelConfig`

```json
{
  "enabled": true,
  "strategy": "weighted"
}
```

`strategy` possible values: `weighted` | `fallback` | `priority` (a tier declaration; routing is currently decided uniformly by `[routing]` — `src/router/` does not branch on this field).

`enabled = false` closes the `model = "auto"` tier: the global pool is excluded from routing and auto requests return `validation_error`(400); explicit model names are unaffected. Writes take effect **immediately in the current process** (the switch hot-reads per request) and persist to disk in sync.

**Response**:

```json
{
  "success": true,
  "message": "Global model config updated"
}
```

**Error codes**: `config_error`(500; with `id != "global"`, returns `"Only 'global' model config is supported"`)

**curl example**:

```bash
curl -X POST http://localhost:8080/admin/models/global \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"enabled": true, "strategy": "weighted"}'
```

---

### POST /admin/catalog/sync

Automatic model-catalog sync (`catalog-sync` feature gated): fetches pricing/context-window mappings from an external catalog source (`[catalog_sync].source`, models.dev by default) into `[[model_catalog]]`, **add-only, never modify** (existing entries incl. user pricing are never overwritten); `dry_run = true` produces a diff without persisting. The `[catalog_sync].enabled` runtime hot-read gate sits inside the handler — off → 404.

**Auth**: Bearer Token required

**Request body**:

```json
{ "dry_run": false }
```

**Response**: a sync-result summary (fetch counts, added entries, skipped items and failure counters; failures fail-open with counters, never blocking)

**Error codes**: `config_error`(500, source unreachable); the feature uncompiled → 404; `[catalog_sync].enabled = false` → 404

---

### GET /admin/masking/stats

Outbound masking's 24h-window observability (`masking` feature gated): an in-memory aggregation of per-kind hit/action distributions (the detail fact source is [GET /admin/masking/ledger](#get-adminmaskingledger)).

**Auth**: Bearer Token required

**Response**: the `masking` statistics snapshot (24h window per-kind aggregation, Mask/Warn/Block counts, fail-closed rejection counts)

**Error codes**: no business errors; the feature uncompiled → 404; `[masking].enabled = false` → 404

---

### GET /admin/vault/keys

The key vault list (key-vault, `vault` feature gated): returns the vault's masked surface in `created_at` ascending order (zero plaintext — responses and logs never carry plaintext; `mask` exists for human verification only).

> **Feature gating**: the vault endpoint family mounts only when the `vault` feature is compiled in; without it the routes do not exist (404).

**Auth**: Bearer Token required

**Query parameters**:

| Parameter | Type | Required | Notes |
|---|---|---|---|
| `vendor_id` | string | no | filter by vendor; without it everything is returned |

**Response**:

```json
{
  "keys": [
    {
      "id": "vk_01H...",
      "vendor_id": "openai",
      "label": "primary",
      "mask": "sk-...abcd",
      "fingerprint": "a1b2c3d4e5f60718",
      "created_at_ms": 1754092800000,
      "updated_at_ms": 1754092800000
    }
  ]
}
```

| Field | Type | Notes |
|---|---|---|
| `id` | string | entry ID |
| `vendor_id` | string | owning vendor (uniquely constrained by `(vendor_id, fingerprint)`) |
| `label` | string \| null | a human label |
| `mask` | string | the masked form (for human verification, not plaintext) |
| `fingerprint` | string | the first 16 chars of the plaintext's SHA256 hex |
| `created_at_ms` / `updated_at_ms` | i64 | created/updated times (unix ms) |

**Error codes**: no business errors (an unwired vault returns 5xx `vendor_unavailable`-shaped errors)

**curl example**:

```bash
curl "http://localhost:8080/admin/vault/keys?vendor_id=openai" \
  -H "Authorization: Bearer $API_KEY"
```

---

### POST /admin/vault/keys

Enters a key (upsert by the same `vendor_id` + fingerprint); after writing, the vendor's inline key pool **re-injects immediately** (hot-effective; an emptied cabinet clears the pool, falling back to env).

**Auth**: Bearer Token required

**Request body**:

```json
{
  "vendor_id": "openai",
  "key": "sk-proj-...",
  "label": "primary"
}
```

| Field | Type | Required | Notes |
|---|---|---|---|
| `vendor_id` | string | yes | the target vendor (must be registered in `[[vendors]]`, otherwise `validation_error`) |
| `key` | string | yes | the plaintext key (in only, never out; blank strings rejected) |
| `label` | string | no | a human label |

**Response**: a `SecretKeyRecord` (the masked surface, same entry shape as [GET /admin/vault/keys](#get-adminvaultkeys))

**Error codes**: `validation_error`(400, a blank `key`) · `vault_error`(409, `vendor_id` unregistered; 404 key missing) · vault backend errors (5xx `vault_error`)

**curl example**:

```bash
curl -X POST http://localhost:8080/admin/vault/keys \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"vendor_id": "openai", "key": "sk-proj-...", "label": "primary"}'
```

---

### DELETE /admin/vault/keys/:id

Deletes a vault entry, syncing the inline key pool immediately (a non-empty cabinet re-injects from remaining entries; emptied → the pool clears and falls back to env).

**Auth**: Bearer Token required

**Path parameters**:

| Parameter | Type | Notes |
|---|---|---|
| `id` | string | entry ID |

**Response**:

```json
{
  "removed": true
}
```

| Field | Type | Notes |
|---|---|---|
| `removed` | bool | whether a deletion actually happened (idempotent — `false` when absent) |

**Error codes**: vault backend errors (5xx)

**curl example**:

```bash
curl -X DELETE http://localhost:8080/admin/vault/keys/vk_01H... \
  -H "Authorization: Bearer $API_KEY"
```

---

### GET /admin/masking/ledger

The masking audit-ledger query (feature `masking` + `[masking].ledger.enabled` dual gated; the ledger defaults off, 404 when off).
Returns the named day's (UTC day boundary) per-kind aggregation with paginated details — the ledger is the detail fact source while `/admin/masking/stats`'s 24h window is the in-memory aggregation; the two complement each other.

**Auth**: Bearer Token required

**Query parameters**:

| Parameter | Type | Required | Default | Notes |
|---|---|---|---|---|
| `date` | string | no | today (UTC) | ledger date (`YYYY-MM-DD`) |
| `page` | integer | no | `1` | detail page number (1-based) |
| `page_size` | integer | no | `100` | detail page size (1..=1000) |

**Response**:

```json
{
  "enabled": true,
  "date": "2026-11-03",
  "total": 2,
  "per_kind": {"api_key": 2},
  "page": 1,
  "page_size": 100,
  "entries": [
    {"ts": "2026-11-03T08:00:00.123Z", "kind": "api_key", "finder": "openai_sk",
     "placeholder": "{{FLUX_APIKEY_AB12CD34}}", "hint": {"first": "s", "len": 48}}
  ]
}
```

> The ledger records metadata only: `ts`/`kind`/`finder` (the inklog pattern name, `custom_rule_N` or `word`)/`placeholder`/`hint` (**first character** + byte length) — **values and the placeholder→value mapping never persist**. Daily files at `~/.fluxweir/mask_ledger/ledger-YYYYMMDD.jsonl`, with the day's first write triggering trimming (`retention_days` by day + `max_total_bytes` total, oldest deleted first).

**Error codes**: `validation_error`(400, invalid date); the ledger disabled → 404 (`masking_ledger_disabled`)

---

### POST /v1/search

Multi-source search injection (`web-search` feature gated): aggregates Tavily / Brave / Exa / SearXNG (multi-key rotation), with outbound passing netguard SSRF triage. The `[web_search].enabled` runtime hot-read gate sits inside the handler — off → 404.

**Auth**: Bearer Token required

**Request body**: the search query (query, max results, source selection etc. — see the `[web_search]` config surface)

**Response**: an aggregated results array (titles/links/snippets, annotated by source)

**Error codes**: `validation_error`(400); upstream failures → `vendor_unavailable`(503); the feature uncompiled or off → 404

---

### POST /v1/fetch

Guarded fetch (`web-search` feature gated): fetches a URL's body for context injection, with outbound passing netguard SSRF triage (private-net/redirect-follow policies consistent with `[web_search]`).

**Auth**: Bearer Token required

**Request body**: `{ "url": "https://…" }`

**Response**: the fetch result (body text and metadata)

**Error codes**: `validation_error`(400, invalid URL); netguard interception → 4xx (with the reason); upstream failures → `vendor_unavailable`(503); the feature uncompiled or off → 404

---

### GET /admin/web-search/stats

web-search observability (`web-search` feature gated): per-source request and interception counts.

**Auth**: Bearer Token required

**Response**: `{ "web_search_requests_total": {"tavily": 0}, "netguard_denied_total": {} }` (the same calibers as `/admin/metrics`' corresponding fields)

**Error codes**: no business errors; the feature uncompiled → 404

---

### POST /admin/subscription/authorize

Initiates subscription-account OAuth authorization (`subscription-vendor` feature gated, with `[subscription].enabled` the runtime hot-read gate — off → 404): the standard authorization-code + PKCE flow, with state in process memory (the single-instance constraint). See the compliance notice in the README's "Security" section.

**Auth**: Bearer Token required

**Response**: the authorization redirect URL (302 or JSON-carried, per implementation form)

**Error codes**: `validation_error`(400); the feature uncompiled or off → 404

---

### GET /admin/subscription/callback

The OAuth 302 callback endpoint (`subscription-vendor` feature): exchanges `code`/`state` for tokens; **exempt from the auth whitelist** (a bare GET callback — see Authentication), with state verification inside the handler.

**Response**: a 302 back to the front-end or an error page

**Error codes**: `validation_error`(400, state mismatch/expiry)

---

### GET /admin/subscription/accounts

The subscription-account list (`subscription-vendor` feature): the account health surface (degradation/cooldown states, tied to `subscription_refresh_fail_total`).

**Auth**: Bearer Token required

**Response**: an array of account entries (the masked surface, no token plaintext)

**Error codes**: no business errors; the feature uncompiled or off → 404

---

### DELETE /admin/subscription/accounts/:id

Revokes a subscription account (`subscription-vendor` feature): removes it from the credential pool and clears the vault's refresh token.

**Auth**: Bearer Token required

**Response**: `{ "success": true }`

**Error codes**: 404 (the account missing); the feature uncompiled or off → 404

---

### POST /admin/route/explain

Route explainability: per-candidate routing evaluations for a named model (purely read-only — no upstream requests, no changes to affinity/breaker/queue state). The candidate pool = all vendors × the request model (matched case-insensitively by `model_id`/`abstract_id`).

**Auth**: Bearer Token required

**Request body** (`Json<Value>`):

```json
{
  "model": "gpt-4",
  "session_id": "user-session-1"
}
```

| Field | Type | Required | Notes |
|---|---|---|---|
| `model` | string | yes | the model name (unknown models return `validation_error`; `"auto"` routes the global model pool, and with `[global_model].enabled = false` that tier is closed, likewise returning `validation_error`(400)) |
| `session_id` | string | no | session affinity key; skipped when absent |

**Response**: per-candidate `{vendor, model, decision, reason_code, detail}` (the winner additionally carries `factors`' 6-factor weight breakdown, `weight`/`base_weight` and the `selection` tier):

```json
{
  "model": "gpt-4",
  "candidates": [
    {
      "vendor": "openai", "model": "gpt-4",
      "decision": "eligible", "reason_code": "eligible",
      "detail": "候选合格（selection=weighted）",
      "factors": {"model": 0.4, "queue": 0.3, "window": 0.3, "latency": 0.0, "error": 0.0},
      "weight": 1.0, "base_weight": 1.0, "selection": "weighted",
      "quota": {"tie_break": false, "degrade_tier": null}
    },
    {
      "vendor": "anthropic", "model": "gpt-4",
      "decision": "excluded", "reason_code": "not_in_catalog",
      "detail": "vendor 'anthropic' 不提供模型 'gpt-4'"
    }
  ]
}
```

`reason_code`s are stable machine codes (never i18n'd): `disabled` (vendor/model not enabled) | `cooling_down` (breaker/cooldown active) | `throttled` (rate-limit window exhausted) | `not_in_catalog` (this vendor does not offer the model) | `weight_zero` (the synthesized weight is 0) | `affinity_bound_elsewhere` (session affinity bound elsewhere — exclusive in hard mode) | `queue_over_limit` (queue saturated) | `quota_depleted` (the vendor's budget exhausted, filtered out by quota-aware smart — excluded with the same caliber as production routing when other vendors still have budget) | `eligible` (the candidate qualifies). Priority: `not_in_catalog` → `disabled` → `throttled` → `cooling_down` → `affinity_bound_elsewhere` → `queue_over_limit` → `quota_depleted` → `weight_zero` → `eligible`.

**The `quota` observation block** (on the winner; the same snapshot single point as production routing, `quota_factor_scores`): `tie_break` = tie-breaking active (with a score gap < `[routing.quota_factor]`'s ε and remaining >10%, the earlier `reset_at` goes first, spending the about-to-expire allowance first); `degrade_tier` = the quota-clock degradation tier (remaining ≤90% → `"tier1"` (×0.9), ≤98% → `"tier2"` (×0.75), `null` = not degraded; threshold multipliers configurable, enabled by default).

**The `affinity` decision block** (R-affinity-004; the explain point is purely read-only — `peek` does not renew and changes no state):

```json
{
  "model": "gpt-4",
  "candidates": [],
  "affinity": { "decision": "hit", "vendor": "openai", "worth_tokens": 5000, "ttl_secs": 421 }
}
```

| Field | Type | Notes |
|:-----|:-----|:-----|
| `decision` | string | `hit` (a binding exists) / `miss` (the table is wired and a session key exists, but no binding) / `no_session` (the table is wired and the mode is not off, but the request carried no `session_id`) / `disabled` (affinity unwired — `[routing.affinity] mode = "off"` or the affinity table not assembled) |
| `vendor` | string | present only on `hit`: the bound vendor ID |
| `worth_tokens` | integer\|null | the last settle's stickiness-bookkeeping cache worth (and that bookkeeping's vendor matches the current binding); `null` with no bookkeeping (legacy mode / not settled / the bookkeeping row moved vendors / the bookkeeping table's bounded eviction) |
| `ttl_secs` | integer\|null | the **bound key**'s remaining lifetime seconds (same source as `peek`, second precision); `null` when the key is absent. Note it is this binding's remaining life in the affinity table, not the bookkeeping-row snapshot's remainder |

`no_session` and `disabled` must stay distinct: the former means "this request carried no session" (client-fixable), the latter "the server has affinity disabled" (an ops disposition) — conflating them sends debugging in exactly the wrong direction.

With tenant isolation (`affinity_tenant_scoped = true`) on, this block locates bindings via the routing-side's same-source `api_key_hash` (`anonymous` when auth is off), consistent with the candidates' `affinity_bound_elsewhere` exclusivity caliber.

**Error codes**: `validation_error`(400, `model` missing/non-string/unknown)

**curl example**:

```bash
curl -X POST http://localhost:8080/admin/route/explain \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-4", "session_id": "user-session-1"}'
```

---

### POST /admin/route/difficulty-explain

Difficulty-tier explain: for a named model (matched against `[routing.difficulty].groups`) and a request body (the feature-extraction input), returns the fused tiering result (purely read-only — writes nothing outside the decision cache, changes no breaker/affinity/queue state; classifier inference is consulted only when the breaker allows, and failures are not recorded). Unwired (`[routing.difficulty]` disabled) or `groups` unhit returns a Validation error with 404 semantics (the same caliber as [POST /admin/route/explain](#post-adminrouteexplain)'s unknown model).

**Auth**: Bearer Token required

**Request body** (`Json<Value>`):

```json
{
  "model": "auto",
  "body": {"messages": [{"role": "user", "content": "hello"}]}
}
```

| Field | Type | Required | Notes |
|---|---|---|---|
| `model` | string | yes | the request's model name (missing/non-string or an unhit `groups` → `validation_error` 400) |
| `body` | object | no | the full request body, the feature-extraction input; absent is treated as `null` |

**Response**: the fused tiering breakdown (`score_breakdown.formula` = `S = (w_clf·p + w_rule·r) / (w_clf + w_rule)`):

```json
{
  "model_input": "auto",
  "tier": "high",
  "rewritten_model": "claude-opus-4-20250514",
  "score": 0.82,
  "score_breakdown": {
    "w_clf": 0.5, "w_rule": 0.5, "p_clf": 0.9, "r_rule": 0.74,
    "formula": "S = (w_clf·p + w_rule·r) / (w_clf + w_rule)"
  },
  "thresholds": {"score_low": 0.3, "score_high": 0.7, "mid_band_policy": "strong", "mid_configured": true},
  "cached": false,
  "cached_decision": null,
  "classifier": {"wired": true, "breaker": "closed", "error": null},
  "explain_latency_ms": 3
}
```

| Response field | Notes |
|---|---|
| `model_input` | the echoed request model name |
| `tier` / `rewritten_model` | the hit tier (`low`/`mid`/`high`) and the rewritten abstract model name |
| `score` | the fused score S |
| `score_breakdown` | weights and per-term breakdown (`p_clf` = the classifier probability, `r_rule` = the rule score) |
| `thresholds` | tier thresholds and the mid-band policy (`mid_band_policy`, `mid_configured`) |
| `cached` / `cached_decision` | decision-cache hit status (read-only observation, no cache writes) |
| `classifier` | the classifier's wiring state, breaker state (`closed`/`half_open`/`open`) and error notes |
| `explain_latency_ms` | explain duration (ms) |

**Error codes**: `validation_error`(400, `model` missing/non-string; unwired or unhit `groups` returns the same code with 404 semantics)

**curl example**:

```bash
curl -X POST http://localhost:8080/admin/route/difficulty-explain \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model": "auto", "body": {"messages": [{"role": "user", "content": "hello"}]}}'
```

---

## Built-in console (console feature)

Built with `--features console`, the gateway embeds the admin console front-end (the Vite+React artifact embedded at compile time, no external file dependencies):

| Endpoint | Notes |
|---|---|
| `GET /console` | the console SPA shell (the login page takes the API key, held in memory only) |
| `GET /console/{path}` | static assets on exact hit; unmatched paths fall back to `index.html` (front-end routing) |

The static shell itself is auth-free (no key surface); data requests inside the console still go through `/admin/*` endpoints' Bearer auth. With the feature off (default), no routes are produced. Pages: dashboard (health+metrics), Vendors (config edit/status/reset), request logs (filter and paginate), route explain (incl. `quota.tie_break`/`degrade_tier`) and the model catalog.

> **Caching and negotiation**: responses carry a content-hash weak ETag (`If-None-Match` hits return 304), `X-Content-Type-Options: nosniff`, `CSP: frame-ancestors 'self'` and `X-Frame-Options: DENY`; content-hashed assets get `Cache-Control: immutable` and the SPA shell `no-cache`; with `Accept-Encoding: gzip`, negotiated gzip transport applies (console routes only — business/SSE streams never pass the compression layer).

---

## Health checks

### GET /api/v1/health

Returns the service's running status. No auth, and exempt from rate limiting.

> **Path note**: the registered path is `/api/v1/health` — the endpoint registers via sdforge's `#[forge(version = "v1", path = "/health")]` macro, which automatically prepends the version as a path prefix (see `src/api/health.rs`). The bare `/health` **has no registered route** (requests return 404) but stays in both the auth and rate-limit whitelists.

**Auth**: none

**Response**:

```json
{
  "status": "ok",
  "version": "0.1.0",
  "timestamp": "2026-07-14T10:30:00+00:00"
}
```

| Field | Type | Notes |
|---|---|---|
| `status` | string | always `"ok"` |
| `version` | string | from `CARGO_PKG_VERSION` |
| `timestamp` | string | the current UTC time in RFC 3339 |

**Error codes**: none

**curl example**:

```bash
curl http://localhost:8080/api/v1/health
```

### GET /readyz

The readiness probe: aggregates three real component checks — the DB pool, the `oxcache` backend and the vendor manager (read-only handles, no upstream calls). All green → `200 {"status":"ready"}`; any failure → `503 {"status":"unavailable"}`. **Auth**: none. **Rate limiting**: exempt. Internal-network scraping recommended (an unauthenticated endpoint).

### GET /metrics

The Prometheus text metrics exit (`Content-Type: text/plain; version=0.0.4`), RED dimensions (`sdforge_http_requests_total` / `sdforge_http_request_duration_seconds`, route labels normalized). **Auth**: none. **Rate limiting**: exempt. Internal-network scraping recommended.

### Documentation surface (requires `[docs].enabled = true`, off by default)

The OpenAPI documentation surface: `GET /api-docs/openapi.json` (main REST endpoints + admin + health paths) and Swagger UI (`/swagger-ui/`). Under the default config, both routes do not exist (404); with the switch on they are reachable, auth-free and rate-limit-free (the switch itself is the access gate).

```toml
[docs]
enabled = true
```

---

## Rust SDK client

The in-repo Rust SDK (`fluxweir::sdk`, the `sdk` feature, included in `full`; `src/sdk/`) covers chat (incl. streaming aggregation), embeddings, rerank, responses (incl. cancel / input_tokens), models and admin endpoints. Below is the parameter surface, error system, retry/timeouts and routing-header mapping after aligning with the official SDK.

### Client construction

```rust
use std::collections::BTreeMap;
use fluxweir::sdk::FluxweirClientBuilder;

let client = FluxweirClientBuilder::new()
    .base_url("http://localhost:8080")   // required
    .api_key("网关签发的 key")            // required (Authorization: Bearer)
    .default_headers(BTreeMap::new())    // client-level extra headers (optional)
    .max_retries(2)                      // overrides RetryPolicy::max_retries (optional)
    .build()?;
```

| Construction item | Default | Notes |
|---|---|---|
| `base_url` / `api_key` | required | missing → a `Config` error (`build()` rejects) |
| `default_headers` | empty | client-level extra headers; request-level `extra_headers` overrides same-named keys (case-insensitive) |
| `default_config` | `RequestConfig::default()` | client-level default parameters; merged per request (request-side non-None wins, `stream` always from the request side) |
| `timeout` | 600s | non-streaming (chat/embed) total timeout; does not apply to `chat_stream` |
| `connect_timeout` | 5s | connection establishment |
| `read_timeout` | 600s | read-stall guard (reset per read, protecting long SSE) |

> Worst-case latency caliber (the official SDK's trade-off): non-streaming worst-case hang ≈ `timeout` × (1 + `max_retries`) + backoff / `Retry-After` (capped at 120s per wait), about 30 minutes under default parameters; latency-sensitive callers should lower `timeout` / `max_retries` explicitly.

> Restricted headers: `authorization` / `host` / `content-length` within `extra_headers` / `default_headers` are ignored and debug-logged (auth and framework headers cannot be overridden).

### The error system

`FluxweirSdkError`'s six variants (the old `Auth` / `Server` variants are deleted, collapsed into `Api`; BREAKING — see the CHANGELOG):

| Variant | Trigger | Notes |
|---|---|---|
| `Network { source }` | connection/request errors | retryable |
| `Timeout { source }` | request timeout | retryable |
| `Api(ApiError)` | upstream non-2xx | carries `status` / `kind` / `message` / `code` / `param` / `request_id` / `retry_after` / `should_retry`; `is_retryable()` decides 408/409/429/5xx and `x-should-retry` |
| `Protocol { message }` | wire-shape mismatch or SDK-side parameter rejection | not retried |
| `Stream { message }` | in-stream errors (SSE error events etc.) | not retried |
| `Config { message }` | construction/URL errors | not retried |

```rust
use fluxweir::sdk::{ApiError, FluxweirSdkError};

match client.chat(&req).await {
    Ok(resp) => println!("{}", resp.content),
    Err(FluxweirSdkError::Api(ApiError { status: 401, message, .. })) => {
        eprintln!("认证失败：{message}");
    }
    Err(err) => return Err(err),
}
```

### Retry and timeouts

- The default `RetryPolicy` aligns with the official SDK: at most 2 retries, 500ms initial backoff, an 8s backoff cap, `Retry-After` hints truncated at 120s; per-request `RequestConfig::max_retries` overrides the count (builder and request levels both clamp to ≤16, over-limit values silently truncated and debug-logged);
- retry decisions: transport errors (timeout/connect/request) and `ApiError::is_retryable()` (408/409/429/5xx; explicit `x-should-retry` veto/allow takes precedence);
- delays: `retry-after` / `retry-after-ms` hints first (seconds or an HTTP-date, ignored on parse failure), defaulting to exponential backoff + jitter;
- streaming calls retry only at the response-header stage; `Stream` errors after the body stream is established are not retried.

### The error-message trust boundary

- `ApiError.message` and in-stream `Stream` error messages are **upstream verbatim** (truncated to 512 chars; error bodies read only the first 64 KiB), with control characters un-sanitized — callers must handle them before logging; never splice directly into structured logs;
- routing headers (`X-Vendor` / `X-Model` / `X-Session-Id` / `X-Batch` / `X-Weight`) can be set only via typed fields: same-named keys within `extra_headers` / `default_headers` are ignored and debug-logged (preventing multi-tenant apps' user input from smuggling routing headers).

### The parameter surface (`RequestConfig`)

- sampling: `temperature` / `top_p` / `top_k` / `max_tokens` / `max_completion_tokens` / `frequency_penalty` / `presence_penalty` / `n` / `seed` / `stop`;
- structured output and tools: `response_format` / `logit_bias` / `parallel_tool_calls` / `logprobs` / `top_logprobs` (tool definitions live in `ChatRequest.tools`);
- the official new parameters: `reasoning_effort` (`ReasoningEffort`: none/minimal/low/medium/high/xhigh/max), `verbosity` (`Verbosity`: low/medium/high), `service_tier`, `store`, `metadata` (≤16 keys), `safety_identifier`, `prompt_cache_key` / `prompt_cache_retention`, `stream_options` (`StreamOptions`: `include_usage` / `include_obfuscation`), `modalities` / `audio` / `prediction` / `web_search_options`;
- escape hatches: `extra_body` (appends body keys; merged with `vendor_extensions`, the latter winning, with `RESERVED_KEYS` conflicts rejected) and `extra_headers` (restricted headers excepted);
- per-request overrides: `max_retries` / `timeout`.

### chat and streaming

- `ChatResponse`: `choices: Vec<ChatChoice>` carries all multi-candidates (`n>1`); `content` / `reasoning_content` / `tool_calls` / `finish_reason` are first-choice convenience views (`content` keeps the old semantics);
- `ChatStream::finalize()`: aggregates fragments into a `ChatResponse` (content concatenated in arrival order; `tool_calls` accumulated by `index` slot with `id` / `name` from the first non-empty and `arguments` concatenated; `finish_reason` / `usage` from the last value-carrying frame);
- `ChatDelta` gains the `reasoning_content` (thinking increments) and `logprobs` (passthrough) slots;
- tools: `ChatTool` is an untagged enum — `Function { function }` and `Raw` (`type:"custom"` / unknown types preserved as the original object); `ChatToolChoice::AllowedTools { mode, tools }` corresponds to `{"type":"allowed_tools","allowed_tools":{...}}` (the mode vocabulary: auto/required).

### embeddings

- `EmbedInput` in four forms: `Text(String)` / `Strings(Vec<String>)` / `Tokens(Vec<u32>)` / `NestedTokens(Vec<Vec<u32>>)`;
- new `dimensions` / `encoding_format` (`EmbedEncodingFormat::Float` / `Base64`);
- `EmbedData.embedding` is an `EmbeddingValue`: `Floats(Vec<f32>)` or `Base64(String)` (untagged; base64 strings are not decoded on the SDK's behalf).

### responses: cancel and input_tokens

- `cancel_response(id)` (`POST /v1/responses/:id/cancel`): cancels a background response and returns the cancelled response object in full (non-background responses → 400);
- `input_tokens(&ResponsesInputTokensRequest { model, input })` (`POST /v1/responses/input_tokens`): returns `{"object":"response.input_tokens","input_tokens":N}`; empty `model` / `input` returns a `Protocol` error SDK-side without sending.

### Routing hint headers

`RequestConfig`'s header-field mapping (same source as the [Appendix](#appendix-routing-hint-headers); `vendor_hint` / `model_hint` are consumed only by the OpenAI-compatible endpoint):

| RequestConfig field | Header | Notes |
|---|---|---|
| `vendor_hint` | `X-Vendor` | target vendor (must match the gateway config) |
| `model_hint` | `X-Model` | target model |
| `session_id` | `X-Session-Id` | session affinity key |
| `batch` | `X-Batch` | set to `1` when `Some(true)` |
| `weight` | `X-Weight` | request-level weighting |
| `extra_headers` | any (restricted headers excepted) | request-level extra headers |

---

## Appendix: routing hint headers

Both LLM inference endpoints (`/v1/chat/completions` and `/v1/messages`) parse the routing hint headers (`RoutingHints::from_headers`, see `src/api/completions.rs` and `src/api/anthropic.rs`); `X-Vendor`/`X-Model` are consumed only by the OpenAI-compatible endpoint (`route_with_hints`), while `X-Session-Id`/`X-Weight` work on both:

| Header | Notes | Conditions |
|---|---|---|
| `X-Vendor` | preferred vendor ID (direct routing, without consulting the vendor's live status filter) | effective when non-empty |
| `X-Model` | preferred model ID | effective when non-empty |
| `X-Session-Id` | session affinity key — same-key requests stick to the same successful vendor | effective when non-empty; auto-fallback when missing (below). **Aliases configurable**: `[routing.affinity].session_headers` replaces the header source wholesale (e.g. absorbing agent-native headers like `X-Conversation-ID`; the first present, non-empty header in order wins, case-insensitive; default `x-session-id` only, zero-config behavior unchanged) |
| `X-Weight` | request-level weight (a float) | only **finite positive numbers** accepted (non-numeric / negative / NaN / Infinity all ignored); additionally requires the server-side `[routing].allow_request_weight` (default `false`, ignored when off) |

**The X-Session-Id fallback rule**: when a request carries no `X-Session-Id`, the SHA256 of the first 2KB of `serde_json::to_string(messages[0])` (UTF-8 boundary-safe truncation) is hex-encoded and its first 16 characters become the session key — conversations sharing a prefix automatically stick to the same vendor; without `messages` in the body there is no fallback key, and the request does not join affinity.

**X-Vendor/X-Model hint semantics**: with either hint header non-empty, hint routing applies (`route_with_hints`); otherwise the default routing decision (weighting/affinity etc.). Hints are "preferred" semantics only — a hint-target vendor's execution failure still enters failover fallback (when the resilience feature is on). **ID matching is case-insensitive** (ASCII folding): applying to `X-Vendor`/`X-Model`, the `model` field's bare name, the `vendor.model` qualified form and failover exclusion-set comparison alike.

---

## Related documents

[User Guide](./USER_GUIDE.md) · [Architecture](./ARCHITECTURE.md) · [FAQ](./FAQ.md) · [README](../../README.md)
