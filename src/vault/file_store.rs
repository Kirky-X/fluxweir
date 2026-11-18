// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 文件柜后端：整柜条目序列化后经 cocoon AEAD 落盘（SDK 无库形态保底）。
//!
//! `Cocoon` 的构造均强制 PBKDF2，而 master key 已是随机 32 字节，故沿用
//! `MiniCocoon::from_key` 的 `wrap`/`unwrap` 产 self-contained container，
//! 文件头自带 magic+版本行补自描述；写盘走临时文件 + rename 原子替换。

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::vault::crypto;
use crate::vault::{KeyStore, SecretKeyRecord, VaultError};

/// 文件头：magic + 格式版本，将来 cipher/结构演进由此行承载。
const VAULT_FILE_MAGIC: &[u8] = b"FLUXWEIR-VAULT1\n";

/// 默认柜文件：`~/.fluxweir/vault.bin`。
pub fn home_vault_path() -> Result<PathBuf, VaultError> {
    let home = std::env::home_dir().ok_or_else(|| VaultError::Storage {
        message: "vault-home-not-found".to_string(),
    })?;
    Ok(home.join(".fluxweir").join("vault.bin"))
}

/// 柜内条目：blob 为 `crypto::seal` 产出的 self-contained 密文。
/// version/superseded 两字段带 serde default——旧 vault.bin 条目缺字段
/// 落 version=1 / None，无需格式迁移。
#[derive(Serialize, Deserialize)]
struct StoredEntry {
    vendor_id: String,
    label: Option<String>,
    blob: Vec<u8>,
    mask: String,
    fingerprint: String,
    created_at_ms: i64,
    updated_at_ms: i64,
    #[serde(default = "default_entry_version")]
    version: u32,
    #[serde(default)]
    superseded_at_ms: Option<i64>,
}

fn default_entry_version() -> u32 {
    1
}

/// 文件柜：整柜常驻内存，每次变更全量落盘（条目量级：个位数到几十条）。
pub struct FileKeyStore {
    master_key: Vec<u8>,
    path: PathBuf,
    entries: Mutex<BTreeMap<String, StoredEntry>>,
}

impl FileKeyStore {
    /// 打开文件柜：文件存在则解密载入，不存在则以空柜启动（首次 put 时落盘）。
    pub fn open(master_key: Vec<u8>, path: impl Into<PathBuf>) -> Result<Self, VaultError> {
        let path = path.into();
        // 崩溃安全读取（magpie-absorb-r1）：校验=magic 前缀+解密

        // +JSON 解析三者全过；主文件损坏→隔离 .bad-<ts> 并回退 .bak；均坏→
        // 显式错误（绝不静默空柜——那等于丢账号密钥数据）。
        let validate = |raw: &[u8]| -> Option<BTreeMap<String, StoredEntry>> {
            let payload = raw.strip_prefix(VAULT_FILE_MAGIC)?;
            let bytes = crypto::open(&master_key, payload).ok()?;
            serde_json::from_str(bytes.expose_secret()).ok()
        };
        let entries = match crate::utils::state_file::read_with_recovery(&path, |raw| {
            validate(raw).is_some()
        }) {
            crate::utils::state_file::StateFileOutcome::Parsed(raw)
            | crate::utils::state_file::StateFileOutcome::RecoveredFromBak { bytes: raw, .. } => {
                validate(&raw).ok_or_else(|| VaultError::Storage {
                    message: "vault-file-magic-mismatch".to_string(),
                })?
            }
            crate::utils::state_file::StateFileOutcome::Unrecoverable { reason } => {
                return Err(VaultError::Storage {
                    message: format!(
                        "vault-state-unrecoverable|{}",
                        serde_json::json!({ "reason": reason })
                    ),
                });
            }
            // 首装形态（主/bak 均无）：空柜启动，首次 put 时落盘——历史行为
            crate::utils::state_file::StateFileOutcome::Missing => BTreeMap::new(),
        };
        Ok(Self {
            master_key,
            path,
            entries: Mutex::new(entries),
        })
    }

    fn persist(&self, entries: &BTreeMap<String, StoredEntry>) -> Result<(), VaultError> {
        let json = serde_json::to_vec(entries).map_err(|e| VaultError::Storage {
            message: format!(
                "vault-encode-entries-failed|{}",
                serde_json::json!({ "error": e.to_string() })
            ),
        })?;
        let container =
            crypto::cipher(&self.master_key)
                .wrap(&json)
                .map_err(|e| VaultError::Storage {
                    message: format!(
                        "vault-seal-file-failed|{}",
                        serde_json::json!({ "error": e.to_string() })
                    ),
                })?;
        // 全部 fs 操作走阻塞收口：vault 写盘在异步上下文调用（admin/key 注入），
        // 不让同步 IO 占住 async worker。
        crate::utils::blocking::run_blocking(|| -> Result<(), VaultError> {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| VaultError::Storage {
                    message: format!(
                        "vault-create-dir-failed|{}",
                        serde_json::json!({ "error": e.to_string() })
                    ),
                })?;
            }
            let tmp = self.path.with_extension("bin.tmp");
            {
                let mut file = std::fs::File::create(&tmp).map_err(|e| VaultError::Storage {
                    message: format!(
                        "vault-create-tmp-failed|{}",
                        serde_json::json!({ "error": e.to_string() })
                    ),
                })?;
                file.write_all(VAULT_FILE_MAGIC)
                    .and_then(|_| file.write_all(&container))
                    .map_err(|e| VaultError::Storage {
                        message: format!(
                            "vault-write-file-failed|{}",
                            serde_json::json!({ "error": e.to_string() })
                        ),
                    })?;
                file.sync_all().map_err(|e| VaultError::Storage {
                    message: format!(
                        "vault-sync-file-failed|{}",
                        serde_json::json!({ "error": e.to_string() })
                    ),
                })?;
            }
            std::fs::rename(&tmp, &self.path).map_err(|e| VaultError::Storage {
                message: format!(
                    "vault-replace-file-failed|{}",
                    serde_json::json!({ "error": e.to_string() })
                ),
            })?;
            // 成功落盘后回写 .bak（读侧崩溃回退源；失败仅警告不影响主链路）
            if let Ok(fresh) = std::fs::read(&self.path) {
                crate::utils::state_file::write_backup(&self.path, &fresh);
            }
            Ok(())
        })
    }

    fn record(id: &str, entry: &StoredEntry) -> SecretKeyRecord {
        SecretKeyRecord {
            id: id.to_string(),
            vendor_id: entry.vendor_id.clone(),
            label: entry.label.clone(),
            mask: entry.mask.clone(),
            fingerprint: entry.fingerprint.clone(),
            created_at_ms: entry.created_at_ms,
            updated_at_ms: entry.updated_at_ms,
            version: entry.version,
            superseded_at_ms: entry.superseded_at_ms,
        }
    }
}

#[async_trait::async_trait]
impl KeyStore for FileKeyStore {
    async fn put(
        &self,
        vendor_id: &str,
        label: Option<&str>,
        plaintext: &SecretString,
    ) -> Result<SecretKeyRecord, VaultError> {
        let mut entries = self.entries.lock().expect("vault lock poisoned");
        let fingerprint = crypto::fingerprint(plaintext.expose_secret());
        let now = crypto::now_ms();
        let existing = entries
            .iter_mut()
            .find(|(_, e)| e.vendor_id == vendor_id && e.fingerprint == fingerprint);
        let id = match existing {
            Some((id, entry)) => {
                entry.label = label.map(str::to_string);
                entry.updated_at_ms = now;
                id.clone()
            }
            None => {
                let id = crypto::new_id();
                let blob = crypto::seal(&self.master_key, plaintext)?;
                let mask = crypto::mask_key(plaintext.expose_secret());
                entries.insert(
                    id.clone(),
                    StoredEntry {
                        vendor_id: vendor_id.to_string(),
                        label: label.map(str::to_string),
                        blob,
                        mask,
                        fingerprint: fingerprint.clone(),
                        created_at_ms: now,
                        updated_at_ms: now,
                        version: 1,
                        superseded_at_ms: None,
                    },
                );
                id
            }
        };
        self.persist(&entries)?;
        let entry = &entries[&id];
        Ok(Self::record(&id, entry))
    }

    async fn list(&self, vendor_id: Option<&str>) -> Result<Vec<SecretKeyRecord>, VaultError> {
        let entries = self.entries.lock().expect("vault lock poisoned");
        let mut rows: Vec<(&String, &StoredEntry)> = entries
            .iter()
            .filter(|(_, e)| vendor_id.is_none_or(|v| e.vendor_id == v))
            .collect();
        rows.sort_by_key(|(_, e)| e.created_at_ms);
        Ok(rows
            .into_iter()
            .map(|(id, e)| Self::record(id, e))
            .collect())
    }

    async fn remove(&self, id: &str) -> Result<bool, VaultError> {
        let mut entries = self.entries.lock().expect("vault lock poisoned");
        let removed = entries.remove(id).is_some();
        if removed {
            self.persist(&entries)?;
        }
        Ok(removed)
    }

    async fn reveal(
        &self,
        id: &str,
        reason: crate::vault::RevealReason,
    ) -> Result<SecretString, VaultError> {
        let entries = self.entries.lock().expect("vault lock poisoned");
        let entry = entries
            .get(id)
            .ok_or_else(|| VaultError::NotFound(id.to_string()))?;
        let plaintext = crypto::open(&self.master_key, &entry.blob)?;
        // best-effort 双写（与 DbKeyStore 同语义）；key_version 恒 1 直到
        // StoredEntry 版本字段落地（T008）。
        crate::vault::record_reveal(crate::vault::RevealAuditRecord {
            key_id: id.to_string(),
            vendor_id: entry.vendor_id.clone(),
            key_version: 1,
            fingerprint: entry.fingerprint.clone(),
            reason,
            timestamp_ms: crypto::now_ms(),
        });
        Ok(plaintext)
    }

    async fn rotate(
        &self,
        vendor_id: &str,
        label: Option<&str>,
        plaintext: &SecretString,
    ) -> Result<SecretKeyRecord, VaultError> {
        // 内存态同步换：max(version)+1 新代插入 + 旧 active 全部置 superseded，
        // 一次 persist 落盘（整柜序列化，原子 rename 替换）。
        let mut entries = self.entries.lock().expect("vault lock poisoned");
        let fingerprint = crypto::fingerprint(plaintext.expose_secret());
        let now = crypto::now_ms();
        let max_version = entries
            .values()
            .filter(|e| e.vendor_id == vendor_id)
            .map(|e| e.version)
            .max()
            .unwrap_or(0);
        for entry in entries.values_mut() {
            if entry.vendor_id == vendor_id && entry.superseded_at_ms.is_none() {
                entry.superseded_at_ms = Some(now);
                entry.updated_at_ms = now;
            }
        }
        let id = crypto::new_id();
        let blob = crypto::seal(&self.master_key, plaintext)?;
        let mask = crypto::mask_key(plaintext.expose_secret());
        entries.insert(
            id.clone(),
            StoredEntry {
                vendor_id: vendor_id.to_string(),
                label: label.map(str::to_string),
                blob,
                mask,
                fingerprint: fingerprint.clone(),
                created_at_ms: now,
                updated_at_ms: now,
                version: max_version + 1,
                superseded_at_ms: None,
            },
        );
        self.persist(&entries)?;
        Ok(Self::record(&id, &entries[&id]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn open_store(dir: &Path) -> FileKeyStore {
        FileKeyStore::open(vec![7u8; 32], dir.join("vault.bin")).expect("open store")
    }

    async fn put_key(
        store: &FileKeyStore,
        vendor: &str,
        label: Option<&str>,
        key: &str,
    ) -> SecretKeyRecord {
        store
            .put(vendor, label, &SecretString::new(key.into()))
            .await
            .expect("put")
    }

    #[tokio::test]
    async fn put_list_remove_reveal_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = open_store(dir.path());
        let record = put_key(&store, "openai", Some("prod"), "sk-live-abcdef123").await;
        assert_eq!(record.mask, "sk-***f123");
        assert_eq!(record.vendor_id, "openai");
        let listed = store.list(Some("openai")).await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, record.id);
        assert_eq!(listed[0].mask, "sk-***f123");
        let revealed = store
            .reveal(&record.id, crate::vault::RevealReason::BootstrapAssembly)
            .await
            .expect("reveal");
        assert_eq!(revealed.expose_secret(), "sk-live-abcdef123");
        assert!(store.remove(&record.id).await.expect("remove"));
        assert!(matches!(
            store
                .reveal(&record.id, crate::vault::RevealReason::BootstrapAssembly)
                .await,
            Err(VaultError::NotFound(_))
        ));
        assert!(!store.remove(&record.id).await.expect("remove again"));
    }

    /// 端到端故障注入（吸收审计）：主文件损坏 → 重新打开必须经.bak
    /// 恢复而非报错/空柜——单测五路径之外验证接线处的真实文件行为
    #[tokio::test]
    async fn corrupted_primary_recovers_from_bak_on_reopen() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        let master = vec![7u8; 32];
        let store = FileKeyStore::open(master.clone(), &path).expect("open");
        let record = put_key(&store, "openai", None, "sk-live-abcdef123").await;
        drop(store);

        std::fs::write(&path, b"corrupted-not-a-vault").expect("corrupt primary");
        let reopened = FileKeyStore::open(master, &path).expect("reopen must recover");
        let listed = reopened.list(Some("openai")).await.expect("list");
        assert_eq!(listed.len(), 1, ".bak 恢复后条目仍在");
        assert_eq!(listed[0].id, record.id);
        assert_eq!(
            reopened
                .reveal(&record.id, crate::vault::RevealReason::BootstrapAssembly)
                .await
                .expect("reveal")
                .expose_secret(),
            "sk-live-abcdef123"
        );
    }

    #[tokio::test]
    async fn entries_survive_reopen() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        let record = {
            let store = FileKeyStore::open(vec![7u8; 32], &path).expect("open");
            put_key(&store, "anthropic", None, "sk-ant-xyz98765432").await
        };
        let reopened = FileKeyStore::open(vec![7u8; 32], &path).expect("reopen");
        assert_eq!(
            reopened.list(Some("anthropic")).await.expect("list").len(),
            1
        );
        assert_eq!(
            reopened
                .reveal(&record.id, crate::vault::RevealReason::BootstrapAssembly)
                .await
                .expect("reveal")
                .expose_secret(),
            "sk-ant-xyz98765432"
        );
    }

    #[tokio::test]
    async fn same_fingerprint_upserts_instead_of_duplicating() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = open_store(dir.path());
        let first = put_key(&store, "openai", None, "sk-same-key-123456").await;
        let second = put_key(&store, "openai", Some("renamed"), "sk-same-key-123456").await;
        assert_eq!(first.id, second.id);
        assert_eq!(second.label.as_deref(), Some("renamed"));
        assert_eq!(store.list(Some("openai")).await.expect("list").len(), 1);
    }

    /// T010：文件柜轮换——version 递增、旧代 superseded、重开后状态保持。
    #[tokio::test]
    async fn rotate_bumps_version_and_persists() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        let master = vec![7u8; 32];
        let store = FileKeyStore::open(master.clone(), &path).expect("open");
        put_key(&store, "openai", Some("gen-1"), "sk-file-rotate-g1-001").await;
        let gen2 = store
            .rotate(
                "openai",
                Some("gen-2"),
                &SecretString::new("sk-file-rotate-g2-002".into()),
            )
            .await
            .expect("rotate");
        assert_eq!(gen2.version, 2);
        assert_eq!(gen2.superseded_at_ms, None);
        let listed = store.list(Some("openai")).await.expect("list");
        assert_eq!(listed.len(), 2);
        assert_eq!(
            listed
                .iter()
                .filter(|r| r.superseded_at_ms.is_none())
                .count(),
            1,
            "仅新代 active"
        );
        // 重开（文件柜持久性）：代数与 superseded 状态保持
        drop(store);
        let reopened = FileKeyStore::open(master, &path).expect("reopen");
        let listed = reopened
            .list(Some("openai"))
            .await
            .expect("list after reopen");
        assert_eq!(listed.len(), 2);
        assert_eq!(
            listed
                .iter()
                .filter(|r| r.superseded_at_ms.is_none())
                .count(),
            1
        );
        assert_eq!(
            listed
                .iter()
                .find(|r| r.superseded_at_ms.is_none())
                .expect("active")
                .version,
            2
        );
    }

    /// T008：旧 vault.bin 条目 JSON（无 version/superseded 字段）反序列化
    /// → version==1 / superseded==None。
    #[test]
    fn legacy_entry_json_defaults_to_active_version_one() {
        let legacy = serde_json::json!({
            "vendor_id": "openai",
            "label": null,
            "blob": [1, 2, 3],
            "mask": "sk-***bbbb",
            "fingerprint": "fedcba9876543210",
            "created_at_ms": 1,
            "updated_at_ms": 1
        });
        let entry: StoredEntry = serde_json::from_value(legacy).expect("旧条目可反序列化");
        assert_eq!(entry.version, 1);
        assert_eq!(entry.superseded_at_ms, None);
        let record = FileKeyStore::record("legacy-id", &entry);
        assert_eq!(record.version, 1);
        assert_eq!(record.superseded_at_ms, None);
    }

    /// T005：reveal 成功后审计缓冲含该条记录，字段零明文。
    #[tokio::test]
    async fn reveal_records_audit_entry_without_plaintext() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = open_store(dir.path());
        let record = put_key(&store, "openai", Some("audit"), "sk-file-audit-321").await;
        store
            .reveal(&record.id, crate::vault::RevealReason::BootstrapAssembly)
            .await
            .expect("reveal");
        let snapshot = crate::vault::reveal_snapshot(8);
        let audited = snapshot
            .iter()
            .find(|rec| rec.key_id == record.id && rec.reason.as_str() == "bootstrap_assembly")
            .expect("审计缓冲含该条记录");
        assert_eq!(audited.vendor_id, "openai");
        assert_eq!(audited.fingerprint.len(), 16);
        assert!(
            audited.fingerprint.chars().all(|c| c.is_ascii_hexdigit()),
            "fingerprint 为 16 hex 字符"
        );
        assert!(!audited.fingerprint.contains("sk-file-audit"));
    }

    #[test]
    fn magic_mismatch_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        std::fs::write(&path, b"GARBAGE-DATA").expect("write garbage");
        let err = FileKeyStore::open(vec![7u8; 32], path);
        assert!(matches!(err, Err(VaultError::Storage { .. })));
    }
}
