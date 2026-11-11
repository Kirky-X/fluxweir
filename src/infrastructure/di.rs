// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Dependency Injection container built on trait-kit's `AsyncKit`.
//!
//! Assembles the four infrastructure modules into a single `AsyncKit<AsyncReady>`
//! at startup. The kit resolves modules in topological order and provides
//! type-erased capabilities (`CacheBackend`, `ConnectionPool`, `Governor`,
//! `Database`) that downstream code can retrieve via `kit.require::<M>`.
//!
//! # Kit-面范式归属（纯组合豁免申报）
//!
//! 本文件是**纯组合根（composition root）**：自有模块为零——`ModuleMeta` /
//! builder 范式实现全部在各模块提供方仓的 `integrations::kit`（`oxcache`
//! `dbnexus` `inklog` `limiteron` 四仓，各自承担 kit 面接口统一），此处只做
//! `register`/`set_config`/`build`/`require` 编排、错误锚点映射与
//! `verify_kit` 自检。故本仓不设 `kit` feature、不重复实现 `ModuleMeta`
//! （trait-kit 依赖由 `database`/`infrastructure` feature 承载，rc.13）。
//! 范式要求的测试三面已由本文件测试承载：build 快乐路径
//! （`test_build_kit_resolves_all_four_modules`）、`require::<M>()`
//! （多处）、错误分支（`test_verify_kit_returns_err_for_empty_kit`）。
//!
//! # DI Chains
//!
//! 1. `oxcache` → `dbnexus` → `inklog` (cache backend → pool → log storage)
//! 2. `limiteron` (standalone governor)

use std::sync::Arc;

use dbnexus::integrations::kit::DbNexusModule;
use inklog::integrations::kit::InklogModule;
use limiteron::integrations::kit::LimiteronModule;
use oxcache::integrations::kit::{OxcacheConfig, OxcacheModule};
use trait_kit::prelude::*;

use crate::error::FluxweirError;
use crate::error::GateError;

/// Build a minimal valid `FlowControlConfig` (1 rule matching all IPs with a
/// permissive token bucket). `FlowControlConfig::default` has an empty
/// rules vec which fails `validate`.
fn minimal_flow_config() -> limiteron::config::FlowControlConfig {
    use limiteron::config::{
        Action, ActionConfig, FlowControlConfig, LimiterConfig, Matcher, Rule,
    };
    let mut config = FlowControlConfig::default();
    config.rules.push(Rule {
        id: "default".to_string(),
        name: "default rule".to_string(),
        priority: 0,
        matchers: vec![Matcher::Ip {
            ip_ranges: vec!["0.0.0.0/0".to_string()],
        }],
        limiters: vec![LimiterConfig::TokenBucket {
            capacity: 100,
            refill_rate: 10,
        }],
        action: ActionConfig {
            on_exceed: Action::Reject,
            ban: None,
        },
    });
    config
}

/// Construct a minimal in-memory `DbConfig` for the DI kit (tests only).
#[cfg(test)]
fn memory_db_config() -> dbnexus::foundation::config::DbConfig {
    dbnexus::foundation::config::DbConfig {
        url: "sqlite::memory:".to_string(),
        pool_config: dbnexus::foundation::config::PoolConfig {
            max_connections: 5,
            min_connections: 1,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Build the `AsyncKit` DI container with all four infrastructure modules,
/// using an externally-provided database configuration.
///
/// Registers `OxcacheModule`, `LimiteronModule`, `DbNexusModule`, `InklogModule`
/// and resolves them in topological order. The returned `AsyncKit<AsyncReady>`
/// provides capabilities via `require::<M>`.
///
/// `db_config` should point to the production database (e.g. `sqlite:./data/llm_proxy.db`).
/// Cache config uses `OxcacheConfig::default` (in-memory).
/// Flow config uses a permissive default (100 capacity / 10 refill rate, all IPs).
///
/// # Errors
///
/// Returns `FluxweirError::Config` if module registration or the topological
/// build fails (missing dependency, cycle, or a module `build` callback
/// returning `Err`).
pub async fn build_kit_with_config(
    db_config: dbnexus::foundation::config::DbConfig,
) -> Result<AsyncKit<AsyncReady>, FluxweirError> {
    let mut kit = AsyncKit::new();

    kit.set_config(OxcacheConfig::default());
    kit.set_config(minimal_flow_config());
    kit.set_config(db_config);

    kit.register::<OxcacheModule>().map_err(|e| {
        FluxweirError::from(GateError::Config {
            message: format!(
                "config-di-module-failed|{}",
                serde_json::json!({ "operation": "register", "module": "OxcacheModule", "error": e.to_string() })
            ),
        })
    })?;
    kit.register::<LimiteronModule>().map_err(|e| {
        FluxweirError::from(GateError::Config {
            message: format!(
                "config-di-module-failed|{}",
                serde_json::json!({ "operation": "register", "module": "LimiteronModule", "error": e.to_string() })
            ),
        })
    })?;
    kit.register::<DbNexusModule>().map_err(|e| {
        FluxweirError::from(GateError::Config {
            message: format!(
                "config-di-module-failed|{}",
                serde_json::json!({ "operation": "register", "module": "DbNexusModule", "error": e.to_string() })
            ),
        })
    })?;
    kit.register::<InklogModule>().map_err(|e| {
        FluxweirError::from(GateError::Config {
            message: format!(
                "config-di-module-failed|{}",
                serde_json::json!({ "operation": "register", "module": "InklogModule", "error": e.to_string() })
            ),
        })
    })?;

    kit.build().await.map_err(|e| {
        FluxweirError::from(GateError::Config {
            message: format!(
                "config-di-module-failed|{}",
                serde_json::json!({ "operation": "build", "module": "kit", "error": e.to_string() })
            ),
        })
    })
}

/// Test-only convenience: builds kit with an in-memory database and permissive
/// defaults. Production code should call `build_kit_with_config` with a real
/// `DbConfig` sourced from `config.toml`.
#[cfg(test)]
pub async fn build_kit() -> Result<AsyncKit<AsyncReady>, FluxweirError> {
    build_kit_with_config(memory_db_config()).await
}

/// Verify that all four module capabilities resolve successfully.
///
/// Call after `build_kit` to confirm the topological sort produced every
/// expected capability. Returns `Ok()` if all resolve, or the first
/// failing module's error.
///
/// # Errors
///
/// Returns `FluxweirError::Config` if any capability is missing.
pub fn verify_kit(kit: &AsyncKit<AsyncReady>) -> Result<(), FluxweirError> {
    let _cache: Arc<dyn oxcache::backend::CacheBackend + Send + Sync> =
        kit.require::<OxcacheModule>().map_err(|e| {
            FluxweirError::from(GateError::Config {
                message: format!(
                "config-di-module-failed|{}",
                serde_json::json!({ "operation": "require", "module": "OxcacheModule", "error": e.to_string() })
            ),
            })
        })?;
    let _governor: Arc<limiteron::Governor> = kit.require::<LimiteronModule>().map_err(|e| {
        FluxweirError::from(GateError::Config {
            message: format!(
                "config-di-module-failed|{}",
                serde_json::json!({ "operation": "require", "module": "LimiteronModule", "error": e.to_string() })
            ),
        })
    })?;
    let _pool: Arc<dyn dbnexus::database::pool::ConnectionPool + Send + Sync> =
        kit.require::<DbNexusModule>().map_err(|e| {
            FluxweirError::from(GateError::Config {
                message: format!(
                "config-di-module-failed|{}",
                serde_json::json!({ "operation": "require", "module": "DbNexusModule", "error": e.to_string() })
            ),
            })
        })?;
    let _provider: Arc<dyn inklog::Database + Send + Sync> =
        kit.require::<InklogModule>().map_err(|e| {
            FluxweirError::from(GateError::Config {
                message: format!(
                "config-di-module-failed|{}",
                serde_json::json!({ "operation": "require", "module": "InklogModule", "error": e.to_string() })
            ),
            })
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_build_kit_resolves_all_four_modules() {
        let kit = build_kit().await.expect("DI kit should build");

        // DI chain 1: oxcache → dbnexus → inklog
        let cache: Arc<dyn oxcache::backend::CacheBackend + Send + Sync> = kit
            .require::<OxcacheModule>()
            .expect("OxcacheModule capability");
        let _ = cache;

        let pool: Arc<dyn dbnexus::database::pool::ConnectionPool + Send + Sync> = kit
            .require::<DbNexusModule>()
            .expect("DbNexusModule capability");
        let _ = pool;

        let provider: Arc<dyn inklog::Database + Send + Sync> = kit
            .require::<InklogModule>()
            .expect("InklogModule capability");
        let _ = provider;

        // DI chain 2: limiteron (standalone)
        let governor: Arc<limiteron::Governor> = kit
            .require::<LimiteronModule>()
            .expect("LimiteronModule capability");
        let _ = governor;
    }

    #[tokio::test]
    async fn test_kit_contains_all_modules() {
        let kit = build_kit().await.expect("DI kit should build");

        assert!(kit.contains::<OxcacheModule>(), "contains OxcacheModule");
        assert!(
            kit.contains::<LimiteronModule>(),
            "contains LimiteronModule"
        );
        assert!(kit.contains::<DbNexusModule>(), "contains DbNexusModule");
        assert!(kit.contains::<InklogModule>(), "contains InklogModule");
    }

    #[tokio::test]
    async fn test_kit_contains_oxcache_config() {
        let kit = build_kit().await.expect("DI kit should build");

        assert!(
            kit.contains_config::<OxcacheConfig>(),
            "contains_config OxcacheConfig"
        );
    }

    #[tokio::test]
    async fn test_di_chain_inklog_executes_ddl() {
        // Proves the full oxcache → dbnexus → inklog chain: the ConnectionPool
        // from DbNexusModule (which itself depended on CacheBackend from
        // OxcacheModule) is successfully injected into InklogModule's
        // `Database` adapter (DbNexusAdapter). inklog 0.3 rc.5 的
        // `Database` trait 只暴露 `insert_batch` / `is_healthy`（无旧
        // `LogDbProvider::execute_log`），故改用 `is_healthy` 驱动一次
        // get_session → 连接获取，验证注入的连接池确实可用。
        let kit = build_kit().await.expect("DI kit should build");

        let provider: Arc<dyn inklog::Database + Send + Sync> = kit
            .require::<InklogModule>()
            .expect("InklogModule capability");

        assert!(
            provider.is_healthy().await,
            "injected ConnectionPool should be usable through inklog Database adapter"
        );
    }

    #[tokio::test]
    async fn test_optional_returns_some_for_registered() {
        let kit = build_kit().await.expect("DI kit should build");

        assert!(kit.optional::<OxcacheModule>().is_some());
        assert!(kit.optional::<DbNexusModule>().is_some());
        assert!(kit.optional::<InklogModule>().is_some());
        assert!(kit.optional::<LimiteronModule>().is_some());
    }

    #[tokio::test]
    async fn test_verify_kit_returns_err_for_empty_kit() {
        // An empty kit (no modules registered) should fail verification —
        // verify_kit requires all four capabilities to resolve.
        let kit = AsyncKit::new()
            .build()
            .await
            .expect("empty kit should build");
        let result = verify_kit(&kit);
        assert!(
            result.is_err(),
            "verify_kit must return Err when capabilities are missing"
        );
    }
}
