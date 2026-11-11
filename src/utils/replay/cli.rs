// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! `cost-replay` 子命令承载面（主 bin：`fluxweir cost-replay …`）。
//!
//! CLI 面走 sdforge 注册表（`CliCommandRegistration` + `inventory::submit!`
//! 编译期登记 + [`sdforge::cli::dispatch`] 分派，janvi 同款形态）；命令语义
//! 平移自旧独立 bin `fluxweir-replay`（src/bin 已删）：
//! 读既有 `request_logs` 历史，按时间窗重放 affinity/cache_ttl 场景的
//! counterfactual 成本，stdout 出 Markdown（或 `--json`）。纯只读：不开服务、
//! 不写库、不调模型。SQLite 目标以 `mode=ro` 打开且必须已存在——绝不新建库文件。
//!
//! 退出码契约（与旧 bin 一致，集成测试 `tests/replay/` 钉死）：缺参/参数非法
//! → 2（缺参由 clap 自带校验接管）；运行期失败 → 1；成功 / `--help` → 0。
//!
//! 双门控（`cost-replay` + `db-sqlite`）由父模块 `mod` 声明与本文件消费的
//! 引擎 API（`dbnexus`/`sea-orm` 经 `cost-replay` feature）共同继承；主 bin
//! 分派点见 `src/main.rs`。

use crate::config::loader::load_config_from_path;
use crate::database::redact_db_url;
use crate::i18n::{t, t_args};
use sdforge::cli::{
    CliArgInfo, CliArgType, CliBuilder, CliCommandRegistration, CliHandlerRegistration,
};
use sdforge::core::{HandlerArgs, HandlerFuture, HandlerState};
use sdforge::inventory;
use serde_json::Value;
use std::path::PathBuf;

use super::{build_report, load_rows, open_pool, ReplayReport, DEFAULT_MAX_ROWS};

/// 场景选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// legacy（无缓存收益）+ 单档 auto 对照。
    Affinity,
    /// {300,900,3600} 三档对比。
    CacheTtl,
    /// 全部（默认）。
    All,
}

/// 时间解析：unix 秒整数或 RFC3339 时间戳。
fn parse_time(raw: &str) -> Option<i64> {
    if let Ok(secs) = raw.parse::<i64>() {
        return Some(secs);
    }
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp())
}

/// 按选择保留场景（legacy 始终来自 affinity 组）。
fn select_scenarios(mut report: ReplayReport, scenario: Scenario) -> ReplayReport {
    report.scenarios.retain(|s| match scenario {
        Scenario::All => true,
        // affinity：legacy 对照（无缓存收益基准）+ 最大档 auto（保 warm 收益上限）
        Scenario::Affinity => s.name == "legacy" || s.name == "auto-3600s",
        // cache_ttl：仅三档 auto，无 legacy
        Scenario::CacheTtl => s.name.starts_with("auto-"),
    });
    report
}

/// 剥除 URL 查询串里既有的 `mode=` 参数（工具强制只读，避免重复/冲突取值）。
fn strip_mode_param(url: &str) -> String {
    let Some((base, query)) = url.split_once('?') else {
        return url.to_string();
    };
    let kept: Vec<&str> = query
        .split('&')
        .filter(|p| !p.is_empty() && !p.starts_with("mode="))
        .collect();
    if kept.is_empty() {
        base.to_string()
    } else {
        format!("{base}?{}", kept.join("&"))
    }
}

/// 把 `--db` 解析为**只读**连接 URL。
///
/// - 非 SQLite 的 `scheme://` URL 原样保留（各后端无统一只读开关）；
/// - SQLite（裸路径或 `sqlite:` URL）：目标文件**必须已存在**（不存在即报错，
///   绝不借 `mode=rwc` 静默新建空库），并统一强制 `mode=ro`；
/// - `sqlite::memory:` 原样保留（无文件可建，回放将得空结果）。
fn resolve_db_url(db: &str) -> Result<String, String> {
    if !db.starts_with("sqlite:") && db.contains("://") {
        return Ok(db.to_string());
    }
    let path_part = db
        .strip_prefix("sqlite://")
        .or_else(|| db.strip_prefix("sqlite:"))
        .unwrap_or(db);
    let file = path_part.split('?').next().unwrap_or(path_part);
    if file == ":memory:" {
        return Ok(db.to_string());
    }
    if !std::path::Path::new(file).exists() {
        return Err(t_args("replay-cli-db-not-found", &[("path", db.into())]));
    }
    let url = if db.starts_with("sqlite:") {
        db.to_string()
    } else {
        format!("sqlite://{db}")
    };
    let url = strip_mode_param(&url);
    let sep = if url.contains('?') { '&' } else { '?' };
    Ok(format!("{url}{sep}mode=ro"))
}

/// 错误串里的连接 URL 脱敏：dbnexus 报错会回显完整连接串（含 userinfo 口令），
/// 必须以 [`redact_db_url`] 形态落 stderr，避免口令进 CI 日志 / 终端回滚缓冲。
fn redact_error(message: &str, url: &str) -> String {
    message.replace(url, &redact_db_url(url))
}

/// 错误展示本地化：`FluxweirError` 的 Display 是英文规范串（Database 变体
/// 可能带 `key|{json}` 锚点）。CLI 是消息轨道终点，这里就地把锚点渲染成
/// 当前 locale 的 FTL 文案；非锚点内容原样返回。
fn localized_error(err: &crate::error::FluxweirError) -> String {
    let raw = err.to_string();
    let payload = raw.strip_prefix("Database error: ").unwrap_or(&raw);
    let Some((key, json)) = payload.split_once('|') else {
        return raw;
    };
    let Ok(obj) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(json) else {
        return raw;
    };
    let args: Vec<(&str, String)> = obj
        .iter()
        .map(|(k, v)| {
            let value = v
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| v.to_string());
            (k.as_str(), value)
        })
        .collect();
    crate::i18n::t_dynamic(key, &args)
}

/// 回放执行（参数已过 clap 校验）：只读解析 → 装载 → 回放 → 渲染。
///
/// 错误串全部经 i18n 本地化后返回（调用方决定退出码语义）。
async fn run(args: &HandlerArgs) -> Result<String, String> {
    let db = args["db"].as_str();
    let from_raw = args["from"].as_str();
    let to_raw = args["to"].as_str();
    let from = parse_time(from_raw)
        .ok_or_else(|| t_args("replay-cli-bad-time", &[("value", from_raw.into())]))?;
    let to = parse_time(to_raw)
        .ok_or_else(|| t_args("replay-cli-bad-time", &[("value", to_raw.into())]))?;
    if to < from {
        return Err(t("replay-cli-bad-window"));
    }
    let scenario = match args.get("scenario").map(String::as_str) {
        None | Some("all") => Scenario::All,
        Some("affinity") => Scenario::Affinity,
        Some("cache_ttl") => Scenario::CacheTtl,
        Some(other) => {
            return Err(t_args(
                "replay-cli-bad-scenario",
                &[("value", other.into())],
            ));
        }
    };
    let max_rows = match args.get("max-rows").map(String::as_str) {
        None => DEFAULT_MAX_ROWS,
        Some(raw) => raw
            .parse::<u64>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(|| t_args("replay-cli-bad-max-rows", &[("value", raw.into())]))?,
    };
    let config = args
        .get("config")
        .cloned()
        .unwrap_or_else(|| "config.toml".to_string());
    let json = args.get("json").map(String::as_str) == Some("true");

    // 先做只读 URL 解析：库文件不存在即 fail-fast（与配置是否合法无关）。
    let url = resolve_db_url(db)?;

    // 目录单价正源：既有 config.toml 的 [[model_catalog]]（只读解析）。
    let catalog = load_config_from_path(&PathBuf::from(&config))
        .map_err(|e| t_args("replay-cli-config-load-failed", &[("error", e.to_string())]))?
        .model_catalog;

    let pool = open_pool(&url).await.map_err(|e| {
        t_args(
            "replay-cli-db-open-failed",
            &[("error", redact_error(&e.to_string(), &url))],
        )
    })?;
    let rows = load_rows(&pool, from, to, max_rows).await.map_err(|e| {
        t_args(
            "replay-cli-load-failed",
            &[("error", redact_error(&localized_error(&e), &url))],
        )
    })?;

    let report = select_scenarios(build_report(&rows, &catalog, from, to, &config), scenario);
    Ok(if json {
        report.render_json()
    } else {
        report.render_markdown()
    })
}

// ---- sdforge CLI 注册（编译期 inventory 登记，无运行时注册调用）----

/// 参数面（与旧 bin 逐一对应；`--help` 展示描述，缺省逻辑在 handler）。
static COST_REPLAY_ARGS: &[CliArgInfo] = &[
    CliArgInfo::new(
        "db",
        "database to replay: sqlite path/url (opened read-only; must already exist) or backend url",
        CliArgType::Body,
        true,
        None,
    ),
    CliArgInfo::new(
        "from",
        "window start: unix seconds or RFC3339",
        CliArgType::Body,
        true,
        None,
    ),
    CliArgInfo::new(
        "to",
        "window end: unix seconds or RFC3339",
        CliArgType::Body,
        true,
        None,
    ),
    CliArgInfo::new(
        "config",
        "config path for [[model_catalog]] prices (default: config.toml)",
        CliArgType::Body,
        false,
        None,
    ),
    CliArgInfo::new(
        "scenario",
        "scenario filter: affinity | cache_ttl | all (default: all)",
        CliArgType::Body,
        false,
        None,
    ),
    CliArgInfo::new(
        "max-rows",
        "load cap, fail fast when exceeded (default: 1000000)",
        CliArgType::Body,
        false,
        None,
    ),
    // sdforge CLI 面无 boolean flag 形态（CliArgType 只有 Path/Body/State），
    // 布尔开关按 janvi 先例注册为带值 Body + default（`--json true` 显式取值）。
    CliArgInfo::new(
        "json",
        "emit machine-readable JSON instead of markdown: true | false (default: false)",
        CliArgType::Body,
        false,
        Some("false"),
    ),
];

inventory::submit! {
    CliCommandRegistration::new(
        "cost-replay",
        "1",
        "Offline cost replay over request_logs history (read-only, never starts the server)",
        "handle_cost_replay_cmd",
    )
    .with_i18n_key(Some("replay-cli-about"))
    .with_args(COST_REPLAY_ARGS)
}

inventory::submit! {
    CliHandlerRegistration { name: "cost-replay", handler: handle_cost_replay_cmd }
}

/// dispatch 入口：state 无注入面（cost-replay 的全部输入来自 CLI 参数）。
fn handle_cost_replay_cmd(args: HandlerArgs, _state: HandlerState) -> HandlerFuture {
    Box::pin(async move {
        match run(&args).await {
            Ok(output) => {
                // 输出已在此打印（Markdown/JSON 原文），Null 哨兵让上层跳过渲染
                println!("{output}");
                Ok(Value::Null)
            }
            Err(msg) => {
                eprintln!("{msg}");
                // 运行期失败退出码 1（与旧 bin 契约一致；用法类错误已由 clap 以 2 接管）
                std::process::exit(1);
            }
        }
    })
}

/// 主 bin 分派入口：`argv[1] == "cost-replay"` 时由 `src/main.rs` 进入。
///
/// clap 树根名 `fluxweir`、版本取本包 `CARGO_PKG_VERSION`（`CliBuilder::build`
/// 写死 sdforge 自身版本，此处链式覆盖——janvi main.rs 同款注释）。clap 自带
/// `--help`（stdout + 0）与用法错误（stderr + 2）；dispatch 内 handler 处理
/// 运行期失败（exit 1）。
pub async fn main_entry() -> ! {
    let matches = CliBuilder::new()
        .with_name("fluxweir")
        .build()
        .version(env!("CARGO_PKG_VERSION"))
        .get_matches();
    match sdforge::cli::dispatch(&matches, None).await {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
