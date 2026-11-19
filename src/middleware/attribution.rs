// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 配额归因与 fail_open 审计（quota-attribution-and-failopen-audit）：
//! ① fail_open 放行事件——预算源不可用且规则 fail_open=true 时请求放行，
//! 每次放行落一条事件（内存环形缓冲 + `tracing::info!`
//! target=`fluxweir::audit` 字段化双写，经 LogAdapter 并入落库管线）；
//! ② 配额全链路归因——预留→路由→协议→结算的结构化证据（Stage 2 扩展）。
//!
//! 与 `vault::audit` 同模式：缓冲读侧返回 `Arc` 克隆，零拷贝争用。

use std::sync::{Arc, OnceLock};

use crate::utils::audit_buffer::AuditRingBuffer;

/// fail_open 触发面（non_exhaustive：预留后续变体——Exceeded 降级等）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FailOpenReason {
    /// 预算源不可用（DB 断连/初始化未完成等）——本规则 fail_open=true 故放行。
    BudgetSourceUnavailable,
}

impl FailOpenReason {
    /// 恒英文小写串（错误双轨惯例；tracing 字段与 admin 查询共用口径）。
    pub fn as_str(self) -> &'static str {
        match self {
            FailOpenReason::BudgetSourceUnavailable => "budget_source_unavailable",
        }
    }
}

/// 单次 fail_open 放行事件（零明文：key_hash 为 SHA256 hex 摘要）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct QuotaFailOpenEvent {
    /// 网关键摘要（与 [auth] 同口径，非明文）。
    pub key_hash: String,
    pub reason: FailOpenReason,
    /// 放行时的预扣估算值（millicents——即便未实际预留，审计面保留量级）。
    pub consumed_estimate_millicents: u64,
    pub timestamp_ms: i64,
}

/// 生产缓冲容量（与 reveal 审计同预算口径）。
const FAIL_OPEN_BUFFER_CAPACITY: usize = 4096;

static FAIL_OPEN: OnceLock<AuditRingBuffer<QuotaFailOpenEvent>> = OnceLock::new();

fn fail_open_buffer() -> &'static AuditRingBuffer<QuotaFailOpenEvent> {
    FAIL_OPEN.get_or_init(|| AuditRingBuffer::new(FAIL_OPEN_BUFFER_CAPACITY))
}

/// 双写：内存 buffer + tracing 字段化事件（kind=quota_fail_open）。best-effort
/// ——记录动作不可失败（buffer 满淘汰属正常语义）。
pub fn record_fail_open(event: QuotaFailOpenEvent) {
    tracing::info!(
        target: "fluxweir::audit",
        kind = "quota_fail_open",
        key_hash = %event.key_hash,
        reason = event.reason.as_str(),
        consumed_estimate_millicents = event.consumed_estimate_millicents,
        timestamp_ms = event.timestamp_ms,
        "quota fail-open admitted the request"
    );
    fail_open_buffer().push(event);
}

/// 读侧快照：最新在前，最多 `limit` 条（admin 查询面消费）。
pub fn fail_open_snapshot(limit: usize) -> Vec<Arc<QuotaFailOpenEvent>> {
    fail_open_buffer().snapshot(limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// record 后 snapshot 可读回同构记录（字段逐项一致，最新在前）。
    #[test]
    fn record_fail_open_reads_back_same_event() {
        let event = QuotaFailOpenEvent {
            key_hash: "a1b2c3d4e5f60718".to_string(),
            reason: FailOpenReason::BudgetSourceUnavailable,
            consumed_estimate_millicents: 4_200,
            timestamp_ms: 1_700_000_000_000,
        };
        record_fail_open(event.clone());
        let snapshot = fail_open_snapshot(8);
        let read_back = snapshot
            .iter()
            .find(|evt| evt.timestamp_ms == 1_700_000_000_000)
            .expect("记录可读回");
        assert_eq!(**read_back, event);
        assert_eq!(read_back.reason.as_str(), "budget_source_unavailable");
    }
}

// ---- Stage 2：配额全链路归因（预留→路由→协议→结算的结构化证据） ----

use crate::protocol::Protocol;

/// 结算路由快照：选中 vendor/model（+路由档位，未透传则 None）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RouteSnapshot {
    pub selected_vendor: String,
    pub selected_model: String,
    /// CandidateAssessment.selection（router 内部档位名）；未透传 = None。
    pub selection_tier: Option<&'static str>,
}

/// 协议跳：入站协议 → 出站协议（透传时单协议）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct ProtocolHop {
    /// settle 调用点所在 endpoint 静态确定。
    pub inbound: Protocol,
    /// vendor_type 映射（openai/anthropic/gemini → 同名协议）。
    pub outbound: Protocol,
}

/// 预留痕迹（reserve 中间件 Admitted 分支写入请求 extensions）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct ReservationTrace {
    pub est_millicents: u64,
    pub reserved_at_ms: i64,
}

/// 结算记录（settle 三钩结算完成后组装）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SettlementRecord {
    pub actual_millicents: u64,
    pub settled_at_ms: i64,
    /// 租户摘要（SHA256 hex，与 [auth] 同口径）。
    pub key_hash: String,
}

/// 单请求配额归因链。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct QuotaAttribution {
    pub request_id: String,
    pub route: RouteSnapshot,
    pub protocol_hop: ProtocolHop,
    /// extensions 不可达（如部分流式路径）→ None（不伪造）。
    pub reservation: Option<ReservationTrace>,
    pub settlement: Option<SettlementRecord>,
    pub finalized_at_ms: i64,
}

/// 归因缓冲容量（≈4096 × ~400B ≈ 1.6MB 常驻，变更规格预算）。
const ATTRIBUTION_BUFFER_CAPACITY: usize = 4096;

static ATTRIBUTION: OnceLock<AuditRingBuffer<QuotaAttribution>> = OnceLock::new();

fn attribution_buffer() -> &'static AuditRingBuffer<QuotaAttribution> {
    ATTRIBUTION.get_or_init(|| AuditRingBuffer::new(ATTRIBUTION_BUFFER_CAPACITY))
}

/// 双写：内存 buffer + tracing 字段化事件（kind=quota_attribution）。
/// best-effort——记录动作不可失败。
pub fn record_attribution(attribution: QuotaAttribution) {
    tracing::info!(
        target: "fluxweir::audit",
        kind = "quota_attribution",
        request_id = %attribution.request_id,
        selected_vendor = %attribution.route.selected_vendor,
        selected_model = %attribution.route.selected_model,
        inbound = ?attribution.protocol_hop.inbound,
        outbound = ?attribution.protocol_hop.outbound,
        est_millicents = attribution.reservation.as_ref().map(|trace| trace.est_millicents),
        actual_millicents = attribution.settlement.as_ref().map(|record| record.actual_millicents),
        finalized_at_ms = attribution.finalized_at_ms,
        "quota attribution recorded"
    );
    attribution_buffer().push(attribution);
}

/// 读侧快照：最新在前，最多 `limit` 条；`request_id` 给定时精确过滤。
pub fn attribution_snapshot(limit: usize, request_id: Option<&str>) -> Vec<Arc<QuotaAttribution>> {
    attribution_buffer()
        .snapshot(limit)
        .into_iter()
        .filter(|record| request_id.is_none_or(|id| record.request_id == id))
        .collect()
}

/// vendor_type → 出站协议（与 config.example.toml `type_` 取值同口径）。
/// 未知值回落到 OpenAI 兼容面（rerank 不在结算面）。
pub fn protocol_of_vendor_type(vendor_type: &str) -> Protocol {
    match vendor_type {
        "anthropic" => Protocol::Anthropic,
        "gemini" => Protocol::Gemini,
        _ => Protocol::OpenAI,
    }
}

/// 结算钩收尾：组装五段归因并落缓冲（T009）。`selection_tier` 未透传恒 None；
/// `reservation` 从请求 task_local 读（不可达 → None，不伪造）；`settlement`
/// 恒 Some（本函数只在真实结算完成后调用）。
#[allow(clippy::too_many_arguments)]
pub fn finish_attribution(
    vendor_id: &str,
    model: &str,
    inbound: Protocol,
    vendor_type: &str,
    reservation: Option<ReservationTrace>,
    actual_millicents: u64,
    key_hash: &str,
) {
    let now_ms = chrono::Utc::now().timestamp_millis();
    record_attribution(QuotaAttribution {
        request_id: current_request_id(),
        route: RouteSnapshot {
            selected_vendor: vendor_id.to_string(),
            selected_model: model.to_string(),
            selection_tier: None,
        },
        protocol_hop: ProtocolHop {
            inbound,
            outbound: protocol_of_vendor_type(vendor_type),
        },
        reservation,
        settlement: Some(SettlementRecord {
            actual_millicents,
            settled_at_ms: now_ms,
            key_hash: key_hash.to_string(),
        }),
        finalized_at_ms: now_ms,
    });
}

// 当前请求的预留痕迹：reserve 中间件在 Admitted 分支的 `next.run` 作用域内
// 写入；settle 钩同任务读回。无作用域（如直接单测 settle）→ None，不伪造。
tokio::task_local! {
    static RESERVATION: ReservationTrace;
}

/// 读当前请求预留痕迹（settle 钩用；不可得返回 None）。
pub fn current_reservation() -> Option<ReservationTrace> {
    RESERVATION.try_with(|trace| *trace).ok()
}

/// 在预留痕迹作用域内跑请求后续链（reserve 中间件调用）。
pub async fn with_reservation<F: std::future::Future>(
    trace: ReservationTrace,
    fut: F,
) -> F::Output {
    RESERVATION.scope(trace, fut).await
}

/// 当前请求 id：sdforge context 中间件（最外层）已挂 task_local；无上下文
/// （如直接单测）回落到稳定占位串，归因记录不缺 request_id 字段。
pub fn current_request_id() -> String {
    sdforge::context::current()
        .map(|ctx| ctx.request_id().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod attribution_tests {
    use super::*;

    fn sample_attribution(request_id: &str) -> QuotaAttribution {
        QuotaAttribution {
            request_id: request_id.to_string(),
            route: RouteSnapshot {
                selected_vendor: "vendor-a".to_string(),
                selected_model: "model-a".to_string(),
                selection_tier: None,
            },
            protocol_hop: ProtocolHop {
                inbound: Protocol::OpenAI,
                outbound: Protocol::Anthropic,
            },
            reservation: Some(ReservationTrace {
                est_millicents: 4_200,
                reserved_at_ms: 1_700_000_000_000,
            }),
            settlement: Some(SettlementRecord {
                actual_millicents: 3_900,
                settled_at_ms: 1_700_000_000_500,
                key_hash: "a1b2c3d4".to_string(),
            }),
            finalized_at_ms: 1_700_000_000_500,
        }
    }

    /// 两条不同 request_id 后按 id 精确过滤只剩一条（T005 验收）。
    #[test]
    fn attribution_snapshot_filters_by_request_id() {
        record_attribution(sample_attribution("req-t005-alpha"));
        record_attribution(sample_attribution("req-t005-beta"));

        let filtered = attribution_snapshot(64, Some("req-t005-alpha"));
        assert_eq!(filtered.len(), 1, "按 id 过滤只剩一条");
        assert_eq!(filtered[0].request_id, "req-t005-alpha");

        let all = attribution_snapshot(64, None);
        assert!(
            all.iter().any(|a| a.request_id == "req-t005-alpha"),
            "无过滤时含 alpha"
        );
        assert!(
            all.iter().any(|a| a.request_id == "req-t005-beta"),
            "无过滤时含 beta"
        );
    }

    /// vendor_type 四取值映射：anthropic/gemini 直连，openai 与未知值兜底 OpenAI（T008）。
    #[test]
    fn protocol_of_vendor_type_maps_known_and_unknown() {
        assert_eq!(protocol_of_vendor_type("openai"), Protocol::OpenAI);
        assert_eq!(protocol_of_vendor_type("anthropic"), Protocol::Anthropic);
        assert_eq!(protocol_of_vendor_type("gemini"), Protocol::Gemini);
        // 未知值（含 rerank/空串）回落 OpenAI 兼容面
        assert_eq!(protocol_of_vendor_type("rerank"), Protocol::OpenAI);
        assert_eq!(protocol_of_vendor_type(""), Protocol::OpenAI);
    }

    /// 预留痕迹 task_local：作用域内可读回，作用域外为 None（T009 支撑）。
    #[tokio::test]
    async fn reservation_task_local_scoped() {
        let trace = ReservationTrace {
            est_millicents: 100,
            reserved_at_ms: 200,
        };
        assert!(current_reservation().is_none(), "作用域外无痕迹");
        let inside = with_reservation(trace, async { current_reservation() }).await;
        assert_eq!(inside, Some(trace));
        assert!(current_reservation().is_none(), "作用域结束即失效");
    }
}
