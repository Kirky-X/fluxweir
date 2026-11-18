// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 保密柜启动装配：env 吸收（幂等不覆盖）与 inline key pool 注入。
//!
//! 吸收语义：env 有值且柜内无同 (vendor_id, fingerprint) 条目才入库——
//! API 录入的新 key 永不被重启时的旧 env 值覆盖；env 只是首装引导源。

use secrecy::{ExposeSecret, SecretString};

use crate::config::VendorConfig;
use crate::vault::crypto;
use crate::vault::KeyStore;
use crate::vault::RevealReason;
use crate::vault::VaultError;
use crate::vendor::VendorManager;

/// env 值探测函数型（生产走 `std::env::var`，测试注入避免并行 set_var 竞争）。
pub(crate) type EnvLookup<'a> = dyn Fn(&str) -> Option<String> + Send + Sync + 'a;

/// 对每个 vendor 的合并 env 名逐个探测吸收，返回新入库条数。
pub(crate) async fn absorb_env_keys(
    store: &dyn KeyStore,
    vendors: &[VendorConfig],
    lookup: &EnvLookup<'_>,
) -> Result<usize, VaultError> {
    let mut absorbed = 0;
    for vendor in vendors {
        for name in crate::vendor::key_pool::merged_key_env_names(vendor) {
            let Some(value) = lookup(&name) else {
                continue;
            };
            if value.trim().is_empty() {
                continue;
            }
            let plaintext = SecretString::new(value.into());
            let fingerprint = crypto::fingerprint(plaintext.expose_secret());
            let present = store
                .list(Some(&vendor.id))
                .await?
                .iter()
                .any(|r| r.fingerprint == fingerprint);
            if present {
                continue;
            }
            // label 记录来源 env 名（人工核对"这条 key 从哪来"）
            store.put(&vendor.id, Some(&name), &plaintext).await?;
            absorbed += 1;
        }
    }
    Ok(absorbed)
}

/// 生产侧 env 探测。
pub(crate) fn std_env_lookup(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

/// 把柜内某 vendor 的 active 条目解密注入 inline key pool（热生效；
/// superseded 旧代不入池——active-only）；柜内无条目时返回 false（不触碰
/// 既有 pool，env 兜底语义保留）。
pub async fn inject_vendor_keys(
    store: &dyn KeyStore,
    manager: &VendorManager,
    vendor_id: &str,
    reason: RevealReason,
) -> Result<bool, VaultError> {
    let records = store.list(Some(vendor_id)).await?;
    let active: Vec<&crate::vault::SecretKeyRecord> = records
        .iter()
        .filter(|record| record.superseded_at_ms.is_none())
        .collect();
    if active.is_empty() {
        return Ok(false);
    }
    let mut keys = Vec::with_capacity(active.len());
    for record in &active {
        keys.push(
            store
                .reveal(&record.id, reason)
                .await?
                .expose_secret()
                .to_string(),
        );
    }
    manager.set_inline_key_pool(vendor_id, crate::vendor::PoolSnapshot::from_keys(keys));
    Ok(true)
}

/// 轮换过渡注入（双活窗口）：active ∪ superseded 未过 grace 的条目全部入
/// 池——旧代密钥若仍被厂商接受则与新版并存（与 api_key_envs 多 key 池
/// 同语义：401/403 自动切换），grace 过期后旧版不再注入。柜内无可用条目
/// （柜空或全部过窗）返回 false 并清池（env 兜底）。
pub async fn inject_vendor_keys_with_grace(
    store: &dyn KeyStore,
    manager: &VendorManager,
    vendor_id: &str,
    grace_secs: u64,
    reason: RevealReason,
) -> Result<bool, VaultError> {
    let records = store.list(Some(vendor_id)).await?;
    if records.is_empty() {
        return Ok(false);
    }
    let now_ms = crypto::now_ms() as i128;
    let grace_ms = grace_secs as i128 * 1000;
    let eligible: Vec<&crate::vault::SecretKeyRecord> = records
        .iter()
        .filter(|record| match record.superseded_at_ms {
            None => true,
            Some(superseded_at) => (now_ms - superseded_at as i128) < grace_ms,
        })
        .collect();
    if eligible.is_empty() {
        // 全部旧代已过窗：清池回退 env（不清池会让 vendor 挂在死键上）
        manager.clear_inline_key_pool(vendor_id);
        return Ok(false);
    }
    let mut keys = Vec::with_capacity(eligible.len());
    for record in &eligible {
        keys.push(
            store
                .reveal(&record.id, reason)
                .await?
                .expose_secret()
                .to_string(),
        );
    }
    manager.set_inline_key_pool(vendor_id, crate::vendor::PoolSnapshot::from_keys(keys));
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::FileKeyStore;
    use std::sync::Arc;

    fn vendor(id: &str, envs: &[&str]) -> VendorConfig {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": id,
            "type_": "openai",
            "api_key_env": "",
            "api_key_envs": envs,
            "rate_limit": { "type": "interval", "hours": 1 },
            "concurrency": {},
        }))
        .expect("vendor config fragment")
    }

    fn open_store(dir: &std::path::Path) -> FileKeyStore {
        FileKeyStore::open(vec![7u8; 32], dir.join("vault.bin")).expect("open store")
    }

    fn lookup_of<'a>(map: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + use<'a> {
        move |name: &str| {
            map.iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.to_string())
        }
    }

    #[tokio::test]
    async fn absorb_is_idempotent_across_restarts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = open_store(dir.path());
        let vendors = [vendor("openai", &["TEST_VAULT_KEY_A"])];
        let lookup = lookup_of(&[("TEST_VAULT_KEY_A", "sk-absorb-once-123456")]);
        let first = absorb_env_keys(&store, &vendors, &lookup)
            .await
            .expect("absorb 1");
        assert_eq!(first, 1);
        // 二次启动（env 不变）不新增
        let second = absorb_env_keys(&store, &vendors, &lookup)
            .await
            .expect("absorb 2");
        assert_eq!(second, 0);
        assert_eq!(store.list(Some("openai")).await.expect("list").len(), 1);
    }

    #[tokio::test]
    async fn api_stored_key_wins_over_stale_env() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = open_store(dir.path());
        // API 先录入（label 不同于 env 名）
        let api_record = store
            .put(
                "openai",
                Some("admin-api"),
                &SecretString::new("sk-same-value-9876".into()),
            )
            .await
            .expect("api put");
        let vendors = [vendor("openai", &["TEST_VAULT_KEY_B"])];
        let lookup = lookup_of(&[("TEST_VAULT_KEY_B", "sk-same-value-9876")]);
        let absorbed = absorb_env_keys(&store, &vendors, &lookup)
            .await
            .expect("absorb");
        assert_eq!(absorbed, 0, "同指纹 key 已在柜内，env 不得覆盖");
        let listed = store.list(Some("openai")).await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].label.as_deref(), Some("admin-api"));
        assert_eq!(listed[0].id, api_record.id);
    }

    /// T011：双活窗口——grace 内 superseded 旧代与 active 新代同时入池；
    /// grace 外（0 秒）旧代不进池。
    #[tokio::test]
    async fn grace_window_injects_superseded_within_window_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = open_store(dir.path());
        let _gen1 = store
            .put(
                "openai",
                Some("gen-1"),
                &SecretString::new("sk-grace-gen1-000001".into()),
            )
            .await
            .expect("put gen-1");
        let _gen2 = store
            .rotate(
                "openai",
                Some("gen-2"),
                &SecretString::new("sk-grace-gen2-000002".into()),
            )
            .await
            .expect("rotate gen-2");

        let config = Arc::new(crate::config::ConfigManager::new(
            common_test_config_with_vendor(),
        ));
        let manager = crate::vendor::VendorManager::new(config);

        // grace 86400s：新旧两代都在池内（指纹比对）
        let injected = inject_vendor_keys_with_grace(
            &store,
            &manager,
            "openai",
            86400,
            RevealReason::AdminRotation,
        )
        .await
        .expect("grace inject");
        assert!(injected);
        let info = manager.key_pool_info("openai").expect("pool info");
        assert_eq!(info.size, 2, "双活窗口两代并存");
        // 池指纹为进程内随机哈希（key_pool::fingerprint_of），与柜内 SHA256
        // 指纹不同源——用同函数对明文现算比对
        let fp1 = crate::vendor::key_pool::fingerprint_of("sk-grace-gen1-000001");
        let fp2 = crate::vendor::key_pool::fingerprint_of("sk-grace-gen2-000002");
        assert!(info.fingerprints.contains(&fp1), "{:?}", info.fingerprints);
        assert!(info.fingerprints.contains(&fp2), "{:?}", info.fingerprints);

        // grace 0s：旧代立刻过窗，仅新代在池内
        let injected = inject_vendor_keys_with_grace(
            &store,
            &manager,
            "openai",
            0,
            RevealReason::AdminRotation,
        )
        .await
        .expect("grace-0 inject");
        assert!(injected, "active 新代仍在");
        let info = manager.key_pool_info("openai").expect("pool info");
        assert_eq!(info.size, 1, "过窗旧代不入池");
        let fp1 = crate::vendor::key_pool::fingerprint_of("sk-grace-gen1-000001");
        let fp2 = crate::vendor::key_pool::fingerprint_of("sk-grace-gen2-000002");
        assert!(info.fingerprints.contains(&fp2));
        assert!(!info.fingerprints.contains(&fp1));

        // active-only 注入（inject_vendor_keys）同样只入新代
        let injected =
            inject_vendor_keys(&store, &manager, "openai", RevealReason::BootstrapAssembly)
                .await
                .expect("active-only inject");
        assert!(injected);
        let info = manager.key_pool_info("openai").expect("pool info");
        assert_eq!(info.size, 1);
        let fp2 = crate::vendor::key_pool::fingerprint_of("sk-grace-gen2-000002");
        assert!(info.fingerprints.contains(&fp2));
    }

    /// T011：柜空仍 Ok(false)（env 兜底保留）。
    #[tokio::test]
    async fn grace_inject_empty_vault_returns_false() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = open_store(dir.path());
        let config = Arc::new(crate::config::ConfigManager::new(
            common_test_config_with_vendor(),
        ));
        let manager = crate::vendor::VendorManager::new(config);
        let injected = inject_vendor_keys_with_grace(
            &store,
            &manager,
            "openai",
            86400,
            RevealReason::BootstrapAssembly,
        )
        .await
        .expect("empty vault inject");
        assert!(!injected, "柜空 Ok(false) 保留 env 兜底");
    }

    fn common_test_config_with_vendor() -> crate::config::Config {
        serde_json::from_value(serde_json::json!({
            "server": {"host": "127.0.0.1", "port": 8080},
            "database": {"path": "test.db", "pool_size": 1},
            "vendors": [{
                "id": "openai",
                "name": "openai",
                "type_": "openai",
                "api_key_env": "",
                "api_key_envs": [],
                "rate_limit": { "type": "interval", "hours": 1 },
                "concurrency": {},
            }],
        }))
        .expect("test config")
    }

    #[tokio::test]
    async fn empty_or_missing_env_values_are_skipped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = open_store(dir.path());
        let vendors = [
            vendor("openai", &["TEST_VAULT_MISSING", "TEST_VAULT_EMPTY"]),
            vendor("anthropic", &["TEST_VAULT_OK"]),
        ];
        let lookup = lookup_of(&[
            ("TEST_VAULT_EMPTY", "  "),
            ("TEST_VAULT_OK", "sk-anthropic-real-01"),
        ]);
        let absorbed = absorb_env_keys(&store, &vendors, &lookup)
            .await
            .expect("absorb");
        assert_eq!(absorbed, 1);
        assert!(store.list(Some("openai")).await.expect("list").is_empty());
        assert_eq!(store.list(Some("anthropic")).await.expect("list").len(), 1);
    }
}
