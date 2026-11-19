// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 审计事件统一总线集成测试（quota-forecast-and-route-explain-cli）：
//! reveal / fail_open 两类事件以 target=`fluxweir::audit` 发 tracing 事件，
//! 经 tracing-log LogTracer 桥并入 inklog 统一总线；本测试用 tracing 测试
//! subscriber 钉住该路径不断裂（字段化 kind / key 非空）。
#![cfg(feature = "logging")]

use std::sync::{Arc, Mutex};
use tracing_subscriber::prelude::*;

/// 捕获到的审计事件字段。
#[derive(Debug, Clone, Default)]
struct Captured {
    kind: String,
    key_id: String,
    key_hash: String,
    target: String,
}

struct FieldVisitor(Captured);
impl tracing::field::Visit for FieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "kind" => self.0.kind = value.to_string(),
            "key_id" => self.0.key_id = value.to_string(),
            "key_hash" => self.0.key_hash = value.to_string(),
            _ => {}
        }
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let text = format!("{value:?}");
        match field.name() {
            "kind" => self.0.kind = text.trim_matches('"').to_string(),
            "key_id" => self.0.key_id = text.trim_matches('"').to_string(),
            "key_hash" => self.0.key_hash = text.trim_matches('"').to_string(),
            _ => {}
        }
    }
}

struct CaptureLayer(Arc<Mutex<Vec<Captured>>>);
impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CaptureLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if event.metadata().target() != "fluxweir::audit" {
            return;
        }
        let mut visitor = FieldVisitor(Captured::default());
        event.record(&mut visitor);
        let mut captured = visitor.0;
        captured.target = event.metadata().target().to_string();
        self.0.lock().expect("not poisoned").push(captured);
    }
}

#[test]
fn audit_events_reach_unified_bus() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry().with(CaptureLayer(captured.clone()));

    tracing::subscriber::with_default(subscriber, || {
        fluxweir::vault::record_reveal(fluxweir::vault::RevealAuditRecord {
            key_id: "key-bus-probe".to_string(),
            vendor_id: "v".to_string(),
            key_version: 1,
            fingerprint: "abc".to_string(),
            reason: fluxweir::vault::RevealReason::BootstrapAssembly,
            timestamp_ms: 1_700_000_000_000,
        });
        fluxweir::middleware::attribution::record_fail_open(
            fluxweir::middleware::attribution::QuotaFailOpenEvent {
                key_hash: "hash-bus-probe".to_string(),
                reason: fluxweir::middleware::attribution::FailOpenReason::BudgetSourceUnavailable,
                consumed_estimate_millicents: 100,
                timestamp_ms: 1_700_000_000_000,
            },
        );
    });

    let events = captured.lock().expect("not poisoned");
    assert_eq!(events.len(), 2, "两条审计事件均可捕获");
    assert!(events.iter().all(|e| e.target == "fluxweir::audit"));
    let reveal = events
        .iter()
        .find(|e| e.kind == "reveal")
        .expect("reveal 事件");
    assert_eq!(reveal.key_id, "key-bus-probe");
    let fail = events
        .iter()
        .find(|e| e.kind == "quota_fail_open")
        .expect("fail_open 事件");
    assert_eq!(fail.key_hash, "hash-bus-probe");
}
