// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! doctor CLI 集成测试：主 bin 子进程跑 `fluxweir doctor …` 断言退出码
//! 与报告行（tests/ 既有 CLI 子进程模式，同 tests/replay/replay_test.rs）。
//!
//! 覆盖（security-doctor-cli T004 验收标准）：
//! - `--config config.example.toml`：退出码 ∈ {0,1}（静态检查不出网络
//!   副作用），stdout 含 `vault-encryption` 与 `auth-http` 检查行；
//! - `--config` 指向不存在文件：退出码 2（配置装载失败语义）。

// 主 bin required-features = full：非 full 组合下 bin 不编译，本测试同门
#![cfg(feature = "full")]

use std::process::Command;

fn doctor(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fluxweir"))
        .arg("doctor")
        .args(args)
        .output()
        .expect("spawn fluxweir doctor")
}

/// 示例配置：装载成功 + 报告含安全组锚点行。
#[test]
fn doctor_on_example_config_reports_security_items() {
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/config.example.toml");
    let out = doctor(&["--config", example]);
    let stdout = String::from_utf8(out.stdout).expect("utf-8 stdout");
    assert!(
        out.status.code().is_some_and(|code| code == 0 || code == 1),
        "exit code {:?} not in {{0,1}}; stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("vault-encryption"),
        "stdout missing vault-encryption line:\n{stdout}"
    );
    assert!(
        stdout.contains("auth-http"),
        "stdout missing auth-http line:\n{stdout}"
    );
}

/// 配置装载失败：stderr + 退出码 2，不产报告。
#[test]
fn doctor_with_missing_config_exits_two() {
    let out = doctor(&["--config", "/nonexistent/fluxweir-doctor-no-such.toml"]);
    assert_eq!(out.status.code(), Some(2));
    let stdout = String::from_utf8(out.stdout).expect("utf-8 stdout");
    assert!(
        !stdout.contains("vault-encryption"),
        "no report on load failure"
    );
}
