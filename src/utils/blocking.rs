// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 同步阻塞收口：在异步上下文里执行短小但阻塞的 fs/CPU 片段。
//!
//! 多线程 runtime 上走 [`tokio::task::block_in_place`]（把当前 worker 的
//! 队列让给其它任务，不占住 executor）；current-thread runtime 或纯同步
//! 上下文直接执行（这些场景没有可让出的 worker）。

/// 在异步上下文安全执行阻塞闭包。闭包内不得再 `await`。
pub fn run_blocking<T>(f: impl FnOnce() -> T) -> T {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(f)
        }
        _ => f(),
    }
}

#[cfg(test)]
mod tests {
    use super::run_blocking;

    #[test]
    fn executes_in_sync_context() {
        assert_eq!(run_blocking(|| 2 + 2), 4);
    }

    #[tokio::test]
    async fn executes_on_current_thread_runtime() {
        // current-thread runtime 不走 block_in_place（后者会 panic）
        assert_eq!(run_blocking(|| 3 * 3), 9);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn executes_on_multi_thread_runtime() {
        assert_eq!(run_blocking(|| "ok".to_string()), "ok");
    }
}
