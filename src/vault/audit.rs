// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! reveal 审计（vault-reveal-audit-and-key-rotation）：唯一明文出口的全量
//! 可追溯——每次 `KeyStore::reveal` 解密成功即落一条记录（内存环形缓冲 +
//! `tracing::info!` target=`fluxweir::audit` 字段化双写，经 LogAdapter 并入
//! 落库管线）。记录零明文：fingerprint 即柜内条目指纹（SHA256 hex 前 16
//! 字符），key_id/vendor_id 为标识符非密钥材料。

use std::sync::{Arc, OnceLock};

use crate::utils::audit_buffer::AuditRingBuffer;

/// reveal 触发面：启动装配 / admin 轮换重注 / inline 池重注。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RevealReason {
    BootstrapAssembly,
    AdminRotation,
    InlinePoolReinjection,
}

impl RevealReason {
    /// 恒英文小写串（错误双轨惯例；tracing 字段与 admin 查询共用同一口径）。
    pub fn as_str(self) -> &'static str {
        match self {
            RevealReason::BootstrapAssembly => "bootstrap_assembly",
            RevealReason::AdminRotation => "admin_rotation",
            RevealReason::InlinePoolReinjection => "inline_pool_reinjection",
        }
    }
}

/// 单次 reveal 审计记录（零明文；Serialize 供 admin 查询面直接出 JSON）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RevealAuditRecord {
    pub key_id: String,
    pub vendor_id: String,
    /// 记录时条目版本（Stage 2 版本化后自 store 行读出；此前恒 1）。
    pub key_version: u32,
    /// 明文 SHA256 hex 前 16 字符（与 SecretKeyRecord.fingerprint 同源）。
    pub fingerprint: String,
    pub reason: RevealReason,
    pub timestamp_ms: i64,
}

/// 生产缓冲容量（≈4096 × 250B ≈ 1MB 常驻，容量预算见变更规格）。
const REVEAL_BUFFER_CAPACITY: usize = 4096;

static REVEAL_AUDIT: OnceLock<AuditRingBuffer<RevealAuditRecord>> = OnceLock::new();

fn reveal_buffer() -> &'static AuditRingBuffer<RevealAuditRecord> {
    REVEAL_AUDIT.get_or_init(|| AuditRingBuffer::new(REVEAL_BUFFER_CAPACITY))
}

/// 双写：内存 buffer + tracing 字段化事件（kind=reveal）。best-effort——
/// 记录动作不可失败（buffer 满淘汰属正常语义，tracing 无订阅者静默丢弃）。
pub fn record_reveal(rec: RevealAuditRecord) {
    tracing::info!(
        target: "fluxweir::audit",
        kind = "reveal",
        key_id = %rec.key_id,
        vendor_id = %rec.vendor_id,
        key_version = rec.key_version,
        fingerprint = %rec.fingerprint,
        reason = rec.reason.as_str(),
        timestamp_ms = rec.timestamp_ms,
        "vault key revealed"
    );
    reveal_buffer().push(rec);
}

/// 读侧快照：最新在前，最多 `limit` 条（admin 查询面消费）。
pub fn reveal_snapshot(limit: usize) -> Vec<Arc<RevealAuditRecord>> {
    reveal_buffer().snapshot(limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(key_id: &str, reason: RevealReason) -> RevealAuditRecord {
        RevealAuditRecord {
            key_id: key_id.to_string(),
            vendor_id: "openai".to_string(),
            key_version: 1,
            fingerprint: "a1b2c3d4e5f60718".to_string(),
            reason,
            timestamp_ms: 1_700_000_000_000,
        }
    }

    /// record 后 snapshot 可读回同构记录（字段逐项一致，最新在前）。
    #[test]
    fn record_then_snapshot_reads_back_same_records() {
        let first = sample("key-alpha", RevealReason::BootstrapAssembly);
        let second = sample("key-beta", RevealReason::AdminRotation);
        record_reveal(first.clone());
        record_reveal(second.clone());
        let snapshot = reveal_snapshot(8);
        let read_back: Vec<&RevealAuditRecord> = snapshot
            .iter()
            .take_while(|rec| rec.key_id == "key-beta" || rec.key_id == "key-alpha")
            .map(Arc::as_ref)
            .collect();
        assert!(
            read_back.len() >= 2,
            "两条记录必须可读回（并发测试共享全局 buffer，take_while 容忍更早残留）"
        );
        assert_eq!(*read_back[0], second, "最新在前");
        assert_eq!(*read_back[1], first);
    }

    /// reason 串口径：恒英文小写下划线形态。
    #[test]
    fn reveal_reason_strings_are_stable() {
        assert_eq!(
            RevealReason::BootstrapAssembly.as_str(),
            "bootstrap_assembly"
        );
        assert_eq!(RevealReason::AdminRotation.as_str(), "admin_rotation");
        assert_eq!(
            RevealReason::InlinePoolReinjection.as_str(),
            "inline_pool_reinjection"
        );
    }
}
