# Fluxweir Performance Baselines

> First-round Criterion component-level baselines (conversion / SSE frame parsing / routing decision paths / cache-economy affinity), established alongside the `roadmap-gap-closure` and `cache-economy-affinity` changes.
> Figures are transcribed from actual `cargo bench` output (mean = midpoint of the criterion point estimate; throughput = elements per iteration / time);
> for regression comparisons, re-run on the same machine, same profile and same feature configuration before comparing against this page.

## Measurement caliber

| Item | Value |
|:---|:---|
| Machine | AMD Ryzen 9 9950X 16-Core (16 logical cores), WSL2 (kernel `6.6.87.2-microsoft-standard-WSL2`) |
| Toolchain | rustc 1.97.1 / cargo 1.97.1 |
| Code version | HEAD `bc5cc3b` (conversion/SSE/routing groups) / `1f3089b` + this fix batch (affinity group); the code keeps evolving after the baseline was established (current HEAD `fbd6a3c`, 2026-11-04) — the figures' representativeness decays with evolution, re-run before comparing |
| Benchmark framework | criterion 0.8 (dev-dependency; default features off, keeping only `cargo_bench_support`, single-threaded statistics) |
| Build profile | `cargo bench` (bench profile inherits release: `opt-level=3` / `lto=true` / `codegen-units=1`) |
| Re-run commands | `cargo bench --bench conversion` / `--bench sse` / `--bench routing` / `--bench affinity` (each bench uses `harness = false`) |
| Fixture base | `benches/common/mod.rs` (hand-written test-only constructors, no network or external services) |

## Conversion

| Benchmark | mean | Throughput | Covered path |
|:-----|-----:|-----:|:---------|
| `openai_normalize` | 275.98 ns | 3.62 Melem/s | OpenAI streaming chunk steady-state normalization (id/created rewritten per frame) |
| `anthropic_request_to_external_with_cache_policy` | 5.5102 µs | 181.48 Kelem/s | Multi-turn + system + tools outbound encoding + cache breakpoint injection |
| `anthropic_encode_chunk_tool_use_sequence` | 3.3679 µs | 1.19 Melem/s | tool_use fragment sequence (first fragment → argument shards → termination) stream frame encoding |
| `inject_cache_control` | 3.4939 µs | 286.22 Kelem/s | Pure injection function over three anchors (tools / system / last_user) of a long multi-turn request body |

## SSE frame parsing

| Benchmark | mean | Throughput | Covered path |
|:-----|-----:|-----:|:---------|
| `openai_parse_sse_frame_to_chunk` | 1.6132 µs | 3.10 Melem/s | Tail 5 frames (data: prefix + `[DONE]` terminator) |
| `anthropic_parse_sse_frame_to_chunk` | 1.4827 µs | 4.72 Melem/s | Tail 7 frames (event:/data: prefixes + message_stop closure) |
| `restore_off_passthrough` | — (`masking` feature-gated group, not recorded in round one) | | Streaming return path passed through verbatim with masking off |
| `restore_on_placeholder_stream` | — (`masking` feature-gated group, not recorded in round one) | | Streaming placeholder restoration with masking on |

> The passthrough path is proxied at component level by frame parsing: when the inbound protocol matches the upstream, traffic goes through zero-copy passthrough and the streaming memory hotspot is per-frame parsing. `benches/sse.rs` also contains a `masking` feature-gated `sse_masking` group (`restore_off_passthrough` / `restore_on_placeholder_stream`), compiled and run with `--features masking`.

## Routing decision

| Benchmark | mean | Throughput | Covered path |
|:-----|-----:|-----:|:---------|
| `route_weighted` | 3.4751 µs | 287.76 Kelem/s | Weighted decision over a fixed 5-vendor multi-weight vendor table (all optional layers off; includes the tokio `block_on` entry overhead) |

## Cache-economy affinity

Measured with `cargo bench --bench affinity -- --warm-up-time 0.5 --measurement-time 2 --sample-size 30`:

| Benchmark | mean | Throughput | Covered path |
|:-----|-----:|-----:|:---------|
| `decide_auto` | 5.2446 ns | 190.67 Melem/s | Pure decision: mode dispatch + threshold comparison (zero IO, zero locks) |
| `worth_silent_cache` | 758.65 ps | 1318.13 Melem/s | Three-tier saturating addition for silent-cache worth |
| `settle_auto_stick` | 888.75 ns | 1.13 Melem/s | Full settle path (read-lock snapshot + decision + oxcache set + stickiness bookkeeping) |
| `settle_auto_below_threshold` | 80.382 ns | 12.44 Melem/s | Full settle path (below threshold: read-lock + decision, no writes) |
| `settle_auto_with_sink` | 891.46 ns | 1.12 Melem/s | Full settle path with a persistence channel (`try_send` + drop-delta reconciliation) |

> Conclusion: pure decisions are at the 5 ns scale, so the fixed per-response settle cost on the hot path is negligible; the main cost sits in affinity-table writes (oxcache set, ~0.8 µs) and stickiness bookkeeping. The with-sink increment over without-sink is within noise (891 ns vs 889 ns) — the channel is `try_send`, dropping when full, never blocking the response path. `below_threshold` only decides and counts drift (80 ns), meaning "not worth sticking" sessions are nearly free.

## Boundaries and notes

- **Component-level caliber**: no full axum server is started and no mock upstream is built; HTTP-level end-to-end throughput / latency stress tests are planned separately.
- **Fixture maintenance**: `benches/common` is coupled to production struct fields and must be maintained in sync as protocol structures evolve.
- **CI gating**: round one is manually recorded only; benches do not run in CI (time and resource trade-off); CI bench smoke and baseline regression alerting are registered as Deferred and will be re-evaluated once this page accumulates two or more rounds of baselines.
- **Cross-machine limitation**: figures reflect this machine and the WSL2 caliber; cross-machine comparisons must note the environmental differences.
