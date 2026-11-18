// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 进程内有界审计环形缓冲（vault-reveal-audit-and-key-rotation）：满则
//! 淘汰最旧，读侧 snapshot 返回 `Arc` 克隆（零拷贝争用）。纯 std 实现，
//! 不设 feature 门（审计面恒可用）；消费方见 `vault::audit` 与
//! `middleware::attribution`。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// 有界环形缓冲：push 满则 pop_front（淘汰最旧）；snapshot 最新在前。
pub struct AuditRingBuffer<T> {
    inner: Mutex<VecDeque<Arc<T>>>,
    capacity: usize,
}

impl<T> AuditRingBuffer<T> {
    /// 生产构造：容量为 0 视为 1（最小可用；审计面不接受零容量静默丢全量）。
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(VecDeque::new()),
            capacity: capacity.max(1),
        }
    }

    /// 追加一条；满则先淘汰最旧（锁内 O(1) 摊销）。
    pub fn push(&self, item: T) {
        let mut queue = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if queue.len() >= self.capacity {
            queue.pop_front();
        }
        queue.push_back(Arc::new(item));
    }

    /// 读侧快照：最新在前，最多 `limit` 条（`Arc` 克隆，零拷贝争用）。
    /// T001 Red 桩：未实现 limit 截断与最新在前序（T002 Green 落地）。
    pub fn snapshot(&self, _limit: usize) -> Vec<Arc<T>> {
        let queue = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        queue.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 容量 3 连 push 5 条：淘汰最旧 2 条，snapshot 回最新 3 条且最新在前。
    #[test]
    fn push_beyond_capacity_evicts_oldest() {
        let buffer: AuditRingBuffer<u32> = AuditRingBuffer::new(3);
        for value in 1..=5 {
            buffer.push(value);
        }
        let snapshot = buffer.snapshot(usize::MAX);
        assert_eq!(
            snapshot.iter().map(|v| **v).collect::<Vec<_>>(),
            vec![5, 4, 3],
            "最新在前，且只保留容量内最新的 3 条"
        );
    }

    /// snapshot limit 截断：limit=2 只回最新 2 条。
    #[test]
    fn snapshot_limit_caps_result() {
        let buffer: AuditRingBuffer<&str> = AuditRingBuffer::new(8);
        for value in ["a", "b", "c", "d"] {
            buffer.push(value);
        }
        let snapshot = buffer.snapshot(2);
        assert_eq!(
            snapshot.iter().map(|v| **v).collect::<Vec<_>>(),
            vec!["d", "c"],
            "limit=2 只回最新 2 条（最新在前）"
        );
    }
}
