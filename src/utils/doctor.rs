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

/// vault-encryption（初版，Stage 2 T005 补主钥来源探测）：编译面 feature
/// `vault` 关 → Fail；开 → Pass（detail 由 T005 细化为主钥来源）。
fn check_vault_encryption() -> CheckItem {
    if cfg!(feature = "vault") {
        CheckItem {
            id: "vault-encryption",
            status: CheckStatus::Pass,
            detail: "vault feature compiled in".to_string(),
        }
    } else {
        CheckItem {
            id: "vault-encryption",
            status: CheckStatus::Fail,
            detail: "not compiled in (feature vault) — upstream keys fall back to env".to_string(),
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
    // 安全组（Stage 2 逐项扩充：vault-backend/quota-reserve/quota-fail-open/
    // auth-grpc/masking）
    #[allow(unused_mut)] // 基础组项在 Stage 2 按模式追加
    let mut items = vec![check_vault_encryption(), check_auth_http(config)];
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
    /// [auth].enabled=true → 全 Pass → 退出码 0（本测试环境编译面恒含
    /// vault：单测跑在 dev/default 组合，vault 进 full）。
    #[test]
    fn run_doctor_aggregates_auth_state() {
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
    }

    /// 非法 flag 组合 → 退出码 2：未知 flag、--config 缺值。
    #[test]
    fn run_doctor_from_args_rejects_invalid_flags() {
        assert_eq!(run_doctor_from_args(args(&["--bogus"])), 2);
        assert_eq!(run_doctor_from_args(args(&["--config"])), 2);
        assert_eq!(run_doctor_from_args(args(&["--security", "--oops"])), 2);
    }
}
