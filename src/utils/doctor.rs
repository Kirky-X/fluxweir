// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 安全姿态体检引擎（security-doctor-cli）：静态检查编译面（feature 组合）
//! 与配置快照（[auth]/[key_budgets]/[masking] 等段），逐项输出三态报告。
//!
//! 边界（proposal Non-Goals）：不发起网络请求、不连数据库、不证明运行期
//! 行为（如主钥确实可解密既有密文）——只反映"这次构建 + 这份配置"的静态
//! 姿态；运行期探活属 readiness 面（src/api/readiness.rs）。
//!
//! 模块不设 feature 门（bin required-features = full，doctor 恒在 full
//! 构建内可用）；检查项内部用 `cfg!` 宏如实报告编译面。

use crate::config::Config;

/// 单项检查结论：Pass 通过 / Fail 失败（计入非零退出码）/ Warn 警示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
}

/// 单项检查结果：id 为稳定机器码（vault-encryption / quota-reserve / …），
/// detail 为英文规范串（对齐错误双轨惯例），含定位线索（段名/feature 名）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckItem {
    pub id: &'static str,
    pub status: CheckStatus,
    pub detail: String,
}

/// 渲染标记：✓ Pass / ! Warn / ✗ Fail。
fn glyph(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Pass => "✓",
        CheckStatus::Warn => "!",
        CheckStatus::Fail => "✗",
    }
}

/// 逐行渲染检查报告（stdout 口径）：`{glyph} {id}: {detail}`，行尾统一 \n。
pub fn render(items: &[CheckItem]) -> String {
    items
        .iter()
        .map(|item| format!("{} {}: {}\n", glyph(item.status), item.id, item.detail))
        .collect()
}

/// 聚合退出码：存在 Fail → 1，否则 0（Warn 不计入；与 cost-replay 0/1/2
/// 惯例一致，2 留给用法/装载错误，见 run_doctor）。
pub fn exit_code(items: &[CheckItem]) -> i32 {
    if items.iter().any(|item| item.status == CheckStatus::Fail) {
        1
    } else {
        0
    }
}

/// 主钥探测结论（分层与 vault::master_key 同口径：env 优先 →
/// FLUXWEIR_MASTER_KEY_FILE → 缺省 ~/.fluxweir/vault_master.key）。
enum MasterKeySource {
    Env,
    File(std::path::PathBuf),
    Missing,
}

/// 探测核心（env 值/文件路径 env/home 注入，便于测试隔离）：只判存在性，
/// 不读内容不验证可解密（doctor 是静态姿态检查，运行期验证属启动 fail-fast）。
fn classify_master_key(
    env_value: Option<&str>,
    env_file: Option<&str>,
    home: Option<&std::path::Path>,
) -> MasterKeySource {
    if env_value.is_some_and(|value| !value.trim().is_empty()) {
        return MasterKeySource::Env;
    }
    let path = env_file
        .filter(|value| !value.trim().is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| home.map(|home| home.join(".fluxweir").join("vault_master.key")));
    match path {
        Some(path) if path.exists() => MasterKeySource::File(path),
        _ => MasterKeySource::Missing,
    }
}

/// 生产探测：读真实 env 与 home。
fn master_key_source() -> MasterKeySource {
    classify_master_key(
        std::env::var("FLUXWEIR_MASTER_KEY").ok().as_deref(),
        std::env::var("FLUXWEIR_MASTER_KEY_FILE").ok().as_deref(),
        std::env::home_dir().as_deref(),
    )
}

/// vault-encryption：编译面 feature `vault` 关 → Fail；开时探测主钥来源
/// （env → 文件 → 双缺 Fail，双缺时容器重建即失配——config.example.toml
/// 主钥注释锚点同语义）。
fn check_vault_encryption() -> CheckItem {
    if !cfg!(feature = "vault") {
        return CheckItem {
            id: "vault-encryption",
            status: CheckStatus::Fail,
            detail: "not compiled in (feature vault) — upstream keys fall back to env".to_string(),
        };
    }
    match master_key_source() {
        MasterKeySource::Env => CheckItem {
            id: "vault-encryption",
            status: CheckStatus::Pass,
            detail: "master key: env".to_string(),
        },
        MasterKeySource::File(path) => CheckItem {
            id: "vault-encryption",
            status: CheckStatus::Pass,
            detail: format!("master key: file {}", path.display()),
        },
        MasterKeySource::Missing => CheckItem {
            id: "vault-encryption",
            status: CheckStatus::Fail,
            detail: "master key missing: set FLUXWEIR_MASTER_KEY or mount the key file \
                     (auto-generated on first boot; a container rebuild loses it and the \
                     vault becomes undecryptable — see config.example.toml master key notes)"
                .to_string(),
        },
    }
}

/// vault-backend：柜后端形态。database feature 开 → DbKeyStore（admin 面
/// 持久化）；vault 开而 database 关 → FileKeyStore 回落（SDK 嵌入形态）；
/// vault 关 → 跳过（依赖 vault-encryption 的 Fail 已覆盖）。
fn check_vault_backend() -> CheckItem {
    if !cfg!(feature = "vault") {
        return CheckItem {
            id: "vault-backend",
            status: CheckStatus::Pass,
            detail: "skipped: vault disabled".to_string(),
        };
    }
    if cfg!(feature = "database") {
        CheckItem {
            id: "vault-backend",
            status: CheckStatus::Pass,
            detail: "DbKeyStore".to_string(),
        }
    } else {
        CheckItem {
            id: "vault-backend",
            status: CheckStatus::Warn,
            detail: "FileKeyStore fallback (SDK-embed form)".to_string(),
        }
    }
}

/// auth-http（初版，Stage 2 T009 细化 detail）：`[auth]` 缺段或
/// enabled=false → Fail（网关裸奔）；启用 → Pass。None 语义与
/// `ApiAuthState::new(false)` 同口径（缺段 = 认证禁用）。
fn check_auth_http(config: &Config) -> CheckItem {
    if config.auth.as_ref().is_some_and(|auth| auth.enabled) {
        CheckItem {
            id: "auth-http",
            status: CheckStatus::Pass,
            detail: "SHA256 digest".to_string(),
        }
    } else {
        CheckItem {
            id: "auth-http",
            status: CheckStatus::Fail,
            detail: "gateway accepts unauthenticated requests".to_string(),
        }
    }
}

/// 收集检查项：`--security` 时只含安全组（vault-*/quota-*/auth-*/masking），
/// 全量模式另加基础组（logging 等）。项顺序稳定（机器可 diff）。
fn collect_items(config: &Config, security_only: bool) -> Vec<CheckItem> {
    // 安全组（Stage 2 逐项扩充：quota-reserve/quota-fail-open/auth-grpc/masking）
    #[allow(unused_mut)] // 基础组项在 Stage 2 按模式追加
    let mut items = vec![
        check_vault_encryption(),
        check_vault_backend(),
        check_auth_http(config),
    ];
    // 基础组：仅全量模式
    if !security_only {
        // Stage 2 T010 接入 logging
    }
    items
}

/// 装载配置并跑全部检查；返回进程退出码（0=无 Fail，1=存在 Fail，
/// 2=配置装载失败）。装载失败只走 stderr，不产检查报告。
pub fn run_doctor(config_path: &str, security_only: bool) -> i32 {
    let config =
        match crate::config::loader::load_config_from_path(std::path::Path::new(config_path)) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("doctor: config load failed: {err}");
                return 2;
            }
        };
    let items = collect_items(&config, security_only);
    print!("{}", render(&items));
    exit_code(&items)
}

/// doctor 用法行（未知 flag / 缺参数时打 stderr）。
const DOCTOR_USAGE: &str =
    "usage: fluxweir doctor [--security] [--config <path>] (default config: config.toml)";

/// CLI 入口：解析 `--security` 与 `--config <path>`（缺省 "config.toml"，与
/// main.rs 服务器路径缺省同语义）后调 run_doctor。未知 flag / `--config`
/// 缺值 → stderr 用法行 + 退出码 2（与 cost-replay 用法错误惯例一致）。
pub fn run_doctor_from_args<I>(args: I) -> i32
where
    I: Iterator<Item = String>,
{
    let mut security_only = false;
    let mut config_path = String::from("config.toml");
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--security" => security_only = true,
            "--config" => match args.next() {
                Some(path) => config_path = path,
                None => {
                    eprintln!("doctor: --config requires a path\n{DOCTOR_USAGE}");
                    return 2;
                }
            },
            other => {
                eprintln!("doctor: unknown flag: {other}\n{DOCTOR_USAGE}");
                return 2;
            }
        }
    }
    run_doctor(&config_path, security_only)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// FLUXWEIR_MASTER_KEY* env 互斥锁：探测类测试与依赖退出码的测试
    /// 共享同一进程 env，必须串行改读。
    static MASTER_KEY_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn item(id: &'static str, status: CheckStatus, detail: &str) -> CheckItem {
        CheckItem {
            id,
            status,
            detail: detail.to_string(),
        }
    }

    /// 三态渲染：标记字符、id、detail 按固定口径逐行输出。
    #[test]
    fn render_marks_three_statuses() {
        let items = [
            item("vault-encryption", CheckStatus::Pass, "master key: env"),
            item(
                "quota-fail-open",
                CheckStatus::Warn,
                "requests pass when budget source unavailable",
            ),
            item(
                "auth-grpc",
                CheckStatus::Fail,
                "not compiled in (feature grpc)",
            ),
        ];
        assert_eq!(
            render(&items),
            "✓ vault-encryption: master key: env\n\
             ! quota-fail-open: requests pass when budget source unavailable\n\
             ✗ auth-grpc: not compiled in (feature grpc)\n"
        );
    }

    /// 空项集渲染为空串（零行为）。
    #[test]
    fn render_empty_items_is_empty() {
        assert_eq!(render(&[]), "");
    }

    /// 退出码聚合：无 Fail（含全 Warn）→ 0；任一 Fail → 1。
    #[test]
    fn exit_code_aggregates_fail_only() {
        assert_eq!(exit_code(&[]), 0);
        assert_eq!(exit_code(&[item("a", CheckStatus::Pass, "ok")]), 0);
        assert_eq!(
            exit_code(&[
                item("a", CheckStatus::Pass, "ok"),
                item("b", CheckStatus::Warn, "careful"),
            ]),
            0
        );
        assert_eq!(
            exit_code(&[
                item("a", CheckStatus::Warn, "careful"),
                item("b", CheckStatus::Fail, "broken"),
                item("c", CheckStatus::Pass, "ok"),
            ]),
            1
        );
    }

    /// 最小可解析配置骨架（server/database 全字段缺省，validate 可过）。
    fn write_min_config(extra: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("config.toml"),
            format!("vendors = []\n[server]\n[database]\n{extra}"),
        )
        .expect("write config");
        dir
    }

    /// run_doctor：配置装载失败（路径不存在）→ stderr + 退出码 2。
    #[test]
    fn run_doctor_missing_config_returns_two() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("no-such-config.toml");
        assert_eq!(run_doctor(missing.to_str().expect("utf-8 path"), false), 2);
    }

    /// run_doctor：缺 [auth] 段 = 认证禁用 → auth-http Fail → 退出码 1；
    /// [auth].enabled=true + 主钥 env 注入 → 退出码 0（vault 编译面在
    /// dev/default 组合恒开；主钥经 env 强制 Pass，消除机器态漂移）。
    #[test]
    fn run_doctor_aggregates_auth_state() {
        let _guard = MASTER_KEY_ENV_LOCK.lock().expect("env lock");
        std::env::set_var("FLUXWEIR_MASTER_KEY", "unit-test-master-key");
        let no_auth = write_min_config("");
        assert_eq!(
            run_doctor(
                no_auth.path().join("config.toml").to_str().expect("path"),
                false
            ),
            1
        );
        let with_auth = write_min_config("[auth]\nenabled = true\n");
        assert_eq!(
            run_doctor(
                with_auth.path().join("config.toml").to_str().expect("path"),
                false
            ),
            if cfg!(feature = "vault") { 0 } else { 1 }
        );
        std::env::remove_var("FLUXWEIR_MASTER_KEY");
    }

    /// collect_items：security_only 只裁基础组不裁安全组；安全组两项恒在。
    #[test]
    fn collect_items_keeps_security_group_in_both_modes() {
        let config: Config = toml::from_str("vendors = []\n[server]\n[database]\n").expect("parse");
        for security_only in [true, false] {
            let ids: Vec<&str> = collect_items(&config, security_only)
                .into_iter()
                .map(|item| item.id)
                .collect();
            assert!(ids.contains(&"vault-encryption"), "{ids:?}");
            assert!(ids.contains(&"auth-http"), "{ids:?}");
        }
    }

    fn args<'a>(
        list: &'a [&'a str],
    ) -> std::iter::Map<std::slice::Iter<'a, &'a str>, fn(&&'a str) -> String> {
        list.iter().map(|s| s.to_string())
    }

    /// 合法 flag 组合：缺省路径 config.toml + 全量模式；--security + --config
    /// 覆盖。断言以"装载结果"区分（缺省路径在测试 CWD 必装载失败 → 2，
    /// 显式指向 fixture → 0/1），用法错误恒 2。
    #[test]
    fn run_doctor_from_args_parses_valid_flags() {
        let _guard = MASTER_KEY_ENV_LOCK.lock().expect("env lock");
        std::env::set_var("FLUXWEIR_MASTER_KEY", "unit-test-master-key");
        let dir = write_min_config("[auth]\nenabled = true\n");
        let path = dir.path().join("config.toml");
        let expected = if cfg!(feature = "vault") { 0 } else { 1 };
        // 两 flag 任意顺序 + 等价形态
        assert_eq!(
            run_doctor_from_args(args(&[
                "--config",
                path.to_str().expect("path"),
                "--security"
            ])),
            expected
        );
        assert_eq!(
            run_doctor_from_args(args(&[
                "--security",
                "--config",
                path.to_str().expect("path"),
            ])),
            expected
        );
        std::env::remove_var("FLUXWEIR_MASTER_KEY");
    }

    /// T005 三分支（注入式探测核心，env 值/文件路径/home 全受控）：
    /// env 优先；文件存在（env 路径或 home 缺省路径）次之；双缺 Missing。
    #[test]
    fn master_key_probe_three_branches() {
        let dir = tempfile::tempdir().expect("tempdir");
        let key_file = dir.path().join("vault_master.key");
        std::fs::write(&key_file, "hex-key").expect("seed key file");
        let home = std::path::Path::new("/home/doctor-probe");

        // env 命中（文件也存在时 env 仍优先）
        assert!(matches!(
            classify_master_key(Some("64-hex"), Some(key_file.to_str().unwrap()), Some(home)),
            MasterKeySource::Env
        ));
        // 文件命中：FLUXWEIR_MASTER_KEY_FILE 指向存在的文件
        assert!(matches!(
            classify_master_key(None, Some(key_file.to_str().unwrap()), Some(home)),
            MasterKeySource::File(ref path) if path == &key_file
        ));
        // 文件命中：env 文件缺省回落 home 拼接路径（临时目录充当 home，
        // 种子文件按缺省布局 home/.fluxweir/vault_master.key 写入）
        let home_dir = tempfile::tempdir().expect("tempdir");
        let default_layout = home_dir.path().join(".fluxweir").join("vault_master.key");
        std::fs::create_dir_all(default_layout.parent().expect("parent")).expect("mkdir");
        std::fs::write(&default_layout, "hex-key").expect("seed default key file");
        assert!(matches!(
            classify_master_key(None, None, Some(home_dir.path())),
            MasterKeySource::File(ref path) if path == &default_layout
        ));
        // 双缺：env 无、文件路径指向不存在位置
        assert!(matches!(
            classify_master_key(None, Some("/nonexistent/doctor-probe.key"), Some(home)),
            MasterKeySource::Missing
        ));
        // 空/空白 env 值视同未设置（与 vault::master_key 同口径）
        assert!(matches!(
            classify_master_key(Some("   "), Some(key_file.to_str().unwrap()), Some(home)),
            MasterKeySource::File(_)
        ));
    }

    /// check_vault_encryption：vault 关 → Fail；开 → Pass(detail 标注 env)；
    /// 主钥双缺 → Fail（detail 含容器重建失配提示与配置文档锚点）。
    #[test]
    fn vault_encryption_item_follows_master_key_presence() {
        let _guard = MASTER_KEY_ENV_LOCK.lock().expect("env lock");
        if !cfg!(feature = "vault") {
            let item = check_vault_encryption();
            assert_eq!(item.status, CheckStatus::Fail);
            return;
        }
        // env 分支
        std::env::set_var("FLUXWEIR_MASTER_KEY", "64-hex-unit-test");
        let item = check_vault_encryption();
        assert_eq!(item.status, CheckStatus::Pass);
        assert_eq!(item.detail, "master key: env");
        std::env::remove_var("FLUXWEIR_MASTER_KEY");
        // 文件分支：FLUXWEIR_MASTER_KEY_FILE 指向临时主钥文件
        let dir = tempfile::tempdir().expect("tempdir");
        let key_file = dir.path().join("vault_master.key");
        std::fs::write(&key_file, "hex-key").expect("seed key file");
        std::env::set_var("FLUXWEIR_MASTER_KEY_FILE", key_file.to_str().unwrap());
        let item = check_vault_encryption();
        assert_eq!(item.status, CheckStatus::Pass);
        assert_eq!(
            item.detail,
            format!("master key: file {}", key_file.display())
        );
        // 双缺分支：env 清空、文件路径指向不存在位置
        std::env::set_var("FLUXWEIR_MASTER_KEY_FILE", "/nonexistent/doctor-probe.key");
        let item = check_vault_encryption();
        assert_eq!(item.status, CheckStatus::Fail);
        assert!(item.detail.contains("container rebuild"), "{}", item.detail);
        assert!(
            item.detail.contains("config.example.toml"),
            "{}",
            item.detail
        );
        std::env::remove_var("FLUXWEIR_MASTER_KEY_FILE");
    }

    /// 非法 flag 组合 → 退出码 2：未知 flag、--config 缺值。
    #[test]
    fn run_doctor_from_args_rejects_invalid_flags() {
        assert_eq!(run_doctor_from_args(args(&["--bogus"])), 2);
        assert_eq!(run_doctor_from_args(args(&["--config"])), 2);
        assert_eq!(run_doctor_from_args(args(&["--security", "--oops"])), 2);
    }

    /// T006：vault-backend 与编译面一致；项集内紧随 vault-encryption。
    #[test]
    fn vault_backend_matches_compile_face() {
        let check = check_vault_backend();
        if !cfg!(feature = "vault") {
            assert_eq!(check.status, CheckStatus::Pass);
            assert_eq!(check.detail, "skipped: vault disabled");
        } else if cfg!(feature = "database") {
            assert_eq!(check.status, CheckStatus::Pass);
            assert_eq!(check.detail, "DbKeyStore");
        } else {
            assert_eq!(check.status, CheckStatus::Warn);
            assert_eq!(check.detail, "FileKeyStore fallback (SDK-embed form)");
        }
        let config: Config = toml::from_str("vendors = []\n[server]\n[database]\n").expect("parse");
        let ids: Vec<&str> = collect_items(&config, true)
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(ids[0], "vault-encryption");
        assert_eq!(ids[1], "vault-backend");
    }
}
