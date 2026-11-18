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
