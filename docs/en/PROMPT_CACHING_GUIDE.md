# Prompt Prefix Caching Engineering Guide

> For callers integrating with Fluxweir: vendor-side prefix caching is a **byte-exact match** — one differing byte in the prefix means a full-price recompute. This guide describes how to maximize cache hits and reduce input cost (cached-input prices at major vendors are roughly 1/10–1/2 of full price).

## Rules

1. **Keep the system prompt byte-stable**: content, field order, and whitespace all feed the hash. Do not concatenate timestamps, request IDs, or random values.
2. **Put dynamic content last**: place the varying parts (user input, retrieval results) at the tail of the messages and the stable parts (instructions, few-shot examples, tool definitions) at the head.
3. **Keep multi-turn conversations append-only**: round N's messages must have round N-1's full messages as a prefix — append only, never modify.
4. **Keep tool definitions at the head and never reorder them**: the tools serialization order participates in the prefix.
5. **Pin a fixed session key per task flow**: `X-Session-Id` plus the gateway's prefix affinity (`[routing.prefix_affinity]`) keeps same-prefix requests stuck to one vendor.

## Gateway-side support

- `[routing.prefix_affinity].enabled = true` (default `false` — once enabled, same-prefix requests stick to ≤2 vendors with PoTC dual hashing, balancing load spread).
- `[routing] price_factor_mode = "cache_expected"` (default `nominal`; values `nominal` / `cache_expected`, unknown values fall back to `nominal` — routing automatically favors vendors with cache-friendly pricing).
- Hit observability: `cache_read_tokens_sum / cache_input_tokens_sum` on `/admin/metrics`.

## Measured results (2026-10-15, verified against real vendors)

- **Zhipu direct (glm-4.5-flash, free tier)**: of 1340 tokens in repeated same-prefix requests, 1330 hit the cache (**99.3%**); latency went 1136ms → 568-883ms; one cache eviction followed by automatic rebuild was observed in practice (hits dropped to zero then recovered). The response `usage.prompt_tokens_details.cached_tokens` is observable per request — the O2 observability chain covers it.
- **Agnes model measurements (2026-10-15)**: `agnes-2.5-flash` hits are a constant 17%; `agnes-3.0-flash` (512K tier) lists Cached input on its price sheet but the response usage **carries no cache fields at all** (verified 8 times) — hits are unobservable, effectively uncontrollable.
- **Aggregator-type vendors (e.g. Agnes apihub)**: responses carry `cached_tokens` but **repeated-prefix hits stay flat at 256/1497 (17%)** — the aggregator's internal second-level load balancing scatters requests across its own upstream instances, beyond the gateway's affinity control. **Cache-sensitive traffic should go straight to the vendor's official endpoint first**; use aggregators only as overflow/backup.
- For `/v4`-path vendors like Zhipu: configure `chat_path = "/chat/completions"` on `[[vendors]]` (skips the gateway's automatic `/v1` prefixing; when the field is absent the legacy rules apply — see the `[[vendors]]` section comments in `config.example.toml`), with no local forwarding shim needed.
