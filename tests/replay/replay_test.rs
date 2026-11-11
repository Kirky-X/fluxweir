// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! cost-replay 集成测试：CLI 子进程跑 fixture 库断言退出码与报告关键字段，
//! 以及确定性回归（同输入两跑 byte-equal，无随机无时钟）。
//!
//! 子命令形态（原独立 bin `fluxweir-replay` 折叠进主 bin）：子进程跑主
//! bin `fluxweir cost-replay …`（sdforge CliBuilder + inventory + dispatch）。
//!
//! 覆盖：
//! - 退出码：缺参 → 2（clap 校验）；正常 / `--help` → 0；运行期失败（库不存在、超限）→ 1。
//! - 报告关键字段：窗口、baseline、场景行、假设披露。
//! - `--json` 机器可读且与 Markdown 数值一致。
//! - `--scenario` 过滤（含 affinity 的 legacy 对照）。
//! - 只读契约：`--db` 指向不存在的路径必须报错且**不新建**库文件。

#![cfg(all(feature = "cost-replay", feature = "db-sqlite"))]

use fluxweir::database::{Database, RequestLog};
use std::path::PathBuf;
use std::process::Command;

/// 写一条 fixture 请求日志。
#[allow(clippy::too_many_arguments)]
fn log(
    request_id: &str,
    ts: i64,
    session: Option<&str>,
    vendor: &str,
    model: &str,
    prompt: i32,
    completion: i32,
    cache_write: i64,
    cost: f64,
) -> RequestLog {
    RequestLog {
        id: None,
        request_id: request_id.to_string(),
        timestamp: ts,
        endpoint: "/v1/chat/completions".to_string(),
        input_format: "openai".to_string(),
        abstract_model: None,
        messages: "[]".to_string(),
        actual_vendor: vendor.to_string(),
        actual_model: model.to_string(),
        routing_strategy: "weighted".to_string(),
        status_code: Some(200),
        response: None,
        error_message: None,
        prompt_tokens: Some(prompt),
        completion_tokens: Some(completion),
        total_tokens: Some(prompt + completion),
        upstream_key_fingerprint: None,
        duration_ms: Some(1),
        cost_usd: Some(cost),
        session_key: session.map(str::to_string),
        context_window_tokens: None,
        cache_read_tokens: Some(0),
        cache_write_tokens: Some(cache_write),
    }
}

/// fixture 库 + 目录配置：返回 (db 文件路径, db url, config 路径, tempdir 守卫)。
async fn fixture() -> (PathBuf, String, PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("replay.db");
    let url = format!("sqlite://{}?mode=rwc", db_path.display());

    let db = Database::new(&url, 1).await.expect("fixture db");
    for l in [
        log(
            "r1",
            1_000,
            Some("s1"),
            "v1",
            "claude-sonnet-4",
            1_000_000,
            0,
            0,
            3.0,
        ),
        log(
            "r2",
            1_100,
            Some("s1"),
            "v1",
            "claude-sonnet-4",
            1_000_000,
            0,
            0,
            3.0,
        ),
        log(
            "r3",
            1_200,
            None,
            "v2",
            "claude-sonnet-4",
            1_000_000,
            0,
            0,
            3.0,
        ),
    ] {
        db.repository().log_request(&l).await.expect("seed");
    }

    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
vendors = []

[server]
host = "127.0.0.1"
port = 18099

[database]
path = ":memory:"

[[model_catalog]]
model_id = "claude-sonnet-4"
context_window = 200000
input_price = 3.0
output_price = 15.0
cache_read_price = 0.3
cache_creation_price = 3.75
"#,
    )
    .expect("write config");

    (db_path, url, config_path, dir)
}

/// 主 bin 路径：cost-replay 已折叠为其子命令（双门控编入，required-features
/// full 由 default 携带，本测试的 required-features 保证两个门控全开）。
fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_fluxweir")
}

/// 以 `fluxweir cost-replay <args…>` 形态构造子进程命令。
fn replay_cmd() -> Command {
    let mut cmd = Command::new(bin());
    cmd.arg("cost-replay");
    cmd
}

/// 正常路径：退出码 0，报告含窗口/基线/场景/假设关键字段。
#[tokio::test]
async fn cli_reports_replay_scenarios() {
    let (_path, url, config, _dir) = fixture().await;
    let out = replay_cmd()
        .args([
            "--db",
            &url,
            "--from",
            "0",
            "--to",
            "99999",
            "--config",
            config.to_str().unwrap(),
        ])
        .output()
        .expect("spawn replay bin");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("# Cost replay report"), "{stdout}");
    assert!(stdout.contains("window: 0..99999 (3 requests)"), "{stdout}");
    assert!(stdout.contains("| baseline |"), "{stdout}");
    assert!(stdout.contains("| legacy |"), "{stdout}");
    assert!(stdout.contains("| auto-300s |"), "{stdout}");
    assert!(stdout.contains("| auto-900s |"), "{stdout}");
    assert!(stdout.contains("| auto-3600s |"), "{stdout}");
    assert!(stdout.contains("## assumptions"), "{stdout}");
    assert!(
        stdout.contains("NULL-session rows have no cache benefit"),
        "{stdout}"
    );
    // 计价值源可追溯
    assert!(
        stdout.contains("pricing source: ") && stdout.contains("config.toml"),
        "{stdout}"
    );
    // r2 在 s1 会话内、同 vendor、间隔 100 < TTL → auto 命中一行
    // total = r1 miss 3.0 + r2 hit 0.3 + r3(NULL) miss 3.0 = 6.3 USD
    assert!(stdout.contains("| auto-3600s | 6.300000 |"), "{stdout}");
}

/// 缺参：退出码 2（clap required 校验接管，报错指向缺失 flag）。
#[tokio::test]
async fn cli_missing_args_exit_code_2() {
    let out = replay_cmd()
        .env("FLUXWEIR_LANG", "en")
        .arg("--from")
        .arg("0")
        .output()
        .expect("spawn replay bin");
    assert_eq!(out.status.code(), Some(2), "缺 --db 必须退出码 2");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--db") && stderr.contains("required"),
        "clap 报错须指向缺失的 --db: {stderr}"
    );
}

/// `--help` 不是用法错误：stdout + 退出码 0。
#[tokio::test]
async fn cli_help_exits_zero_on_stdout() {
    let out = replay_cmd()
        .env("FLUXWEIR_LANG", "en")
        .arg("--help")
        .output()
        .expect("spawn replay bin");
    assert_eq!(out.status.code(), Some(0), "--help 必须退出码 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Usage:"), "用法须打到 stdout: {stdout}");
    assert!(
        stdout.contains("cost-replay"),
        "帮助须展示 cost-replay 子命令: {stdout}"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).is_empty(),
        "--help 不得写 stderr"
    );
}

/// `--json` 机器可读，且与 Markdown 数值同源。
#[tokio::test]
async fn cli_json_matches_markdown() {
    let (_path, url, config, _dir) = fixture().await;
    let json_out = replay_cmd()
        .args([
            "--db",
            &url,
            "--from",
            "0",
            "--to",
            "99999",
            "--config",
            config.to_str().unwrap(),
            // sdforge CLI 面无 boolean flag：--json 为带值参数（janvi 同款形态）
            "--json",
            "true",
        ])
        .output()
        .expect("spawn");
    assert_eq!(json_out.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&json_out.stdout).expect("valid json");
    assert_eq!(json["window"]["requests"], 3);
    // baseline = 3 × 3.0 USD = 9.0 USD = 9_000_000 µ$
    assert_eq!(json["baseline"]["total_micro_usd"], 9_000_000);
    assert_eq!(json["assumptions"].as_array().unwrap().len(), 5);
}

/// `--scenario cache_ttl` 只保留三档 auto，不含 legacy。
#[tokio::test]
async fn cli_scenario_filter() {
    let (_path, url, config, _dir) = fixture().await;
    let out = replay_cmd()
        .args([
            "--db",
            &url,
            "--from",
            "0",
            "--to",
            "99999",
            "--config",
            config.to_str().unwrap(),
            "--scenario",
            "cache_ttl",
        ])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("| legacy |"), "{stdout}");
    assert!(stdout.contains("| auto-300s |"), "{stdout}");
}

/// `--scenario affinity` 保留 legacy 对照 + 最大档 auto（文档承诺的行为）。
#[tokio::test]
async fn cli_scenario_affinity_keeps_legacy() {
    let (_path, url, config, _dir) = fixture().await;
    let out = replay_cmd()
        .args([
            "--db",
            &url,
            "--from",
            "0",
            "--to",
            "99999",
            "--config",
            config.to_str().unwrap(),
            "--scenario",
            "affinity",
        ])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("| legacy |"),
        "affinity 必须含 legacy 对照: {stdout}"
    );
    assert!(stdout.contains("| auto-3600s |"), "{stdout}");
    assert!(!stdout.contains("| auto-300s |"), "{stdout}");
    assert!(!stdout.contains("| auto-900s |"), "{stdout}");
}

/// `--db` 裸路径（无 scheme）走只读解析：指向已存在的库可读。
#[tokio::test]
async fn cli_accepts_bare_path_db() {
    let (db_path, _url, config, _dir) = fixture().await;
    let out = replay_cmd()
        .args([
            "--db",
            db_path.to_str().unwrap(),
            "--from",
            "0",
            "--to",
            "99999",
            "--config",
            config.to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("| baseline |"),
        "裸路径必须能读同一库"
    );
}

/// 只读契约：`--db` 指向不存在的路径 → 退出码 1，且**不新建**库文件。
#[tokio::test]
async fn cli_missing_db_file_fails_without_creating() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("does-not-exist.db");
    let config = dir.path().join("config.toml");
    std::fs::write(&config, "vendors = []\n").expect("write config");

    let out = replay_cmd()
        .env("FLUXWEIR_LANG", "en")
        .args([
            "--db",
            missing.to_str().unwrap(),
            "--from",
            "0",
            "--to",
            "99999",
            "--config",
            config.to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1), "库不存在必须退出码 1");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("does not exist"),
        "报错须说明库文件不存在: {stderr}"
    );
    assert!(
        !missing.exists(),
        "只读工具绝不新建库文件：{}",
        missing.display()
    );
}

/// `--max-rows` 超限 fail-fast：退出码 1，报错含实际行数。
#[tokio::test]
async fn cli_max_rows_fails_fast() {
    let (_path, url, config, _dir) = fixture().await;
    let out = replay_cmd()
        .args([
            "--db",
            &url,
            "--from",
            "0",
            "--to",
            "99999",
            "--config",
            config.to_str().unwrap(),
            "--max-rows",
            "1",
        ])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1), "超限必须退出码 1");
    let stderr = String::from_utf8_lossy(&out.stderr);
    // 文案随 locale 本地化：断言语言中性锚点与参数（--max-rows / 实际行数）
    assert!(
        stderr.contains("--max-rows"),
        "报错须说明超限参数: {stderr}"
    );
    assert!(stderr.contains('2'), "报错须带实际行数: {stderr}");
}

/// 确定性：同输入两跑 stdout byte-equal。
#[tokio::test]
async fn cli_output_is_deterministic() {
    let (_path, url, config, _dir) = fixture().await;
    let run = || {
        replay_cmd()
            .args([
                "--db",
                &url,
                "--from",
                "0",
                "--to",
                "99999",
                "--config",
                config.to_str().unwrap(),
            ])
            .output()
            .expect("spawn")
    };
    let a = run();
    let b = run();
    assert_eq!(a.status.code(), Some(0));
    assert_eq!(a.stdout, b.stdout, "同输入两跑必须 byte-equal");
}
