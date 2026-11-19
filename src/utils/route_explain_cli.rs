// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 路由预演 CLI（quota-forecast-and-route-explain-cli）：薄 HTTP 客户端，
//! 转发既有 `POST /admin/route/explain` 并把候选数组渲染为对齐文本表。
//! 不做本地路由计算（explain_route 需 ApiState 全装配）——无网关时显式报错。

use serde_json::Value;

const USAGE: &str = "usage: fluxweir route-explain --model <id> [--gateway <url>] [--admin-key <token>] [--session-id <id>]";

/// 解析后的 CLI 参数。
#[derive(Debug, PartialEq, Eq)]
pub struct ExplainArgs {
    pub model: String,
    pub gateway: String,
    pub admin_key: Option<String>,
    pub session_id: Option<String>,
}

/// 手写 flag 解析：--model 必填，--gateway 缺省 127.0.0.1:8080；未知 flag
/// 或缺 --model → Err（调用方打 stderr 用法 + 退出码 2）。
pub fn parse_args(args: Vec<String>) -> Result<ExplainArgs, String> {
    let mut model: Option<String> = None;
    let mut gateway = "http://127.0.0.1:8080".to_string();
    let mut admin_key = None;
    let mut session_id = None;
    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let slot = match flag.as_str() {
            "--model" => &mut model,
            "--gateway" => {
                gateway = iter.next().ok_or("--gateway requires a url")?;
                continue;
            }
            "--admin-key" => {
                admin_key = Some(iter.next().ok_or("--admin-key requires a token")?);
                continue;
            }
            "--session-id" => {
                session_id = Some(iter.next().ok_or("--session-id requires an id")?);
                continue;
            }
            other => return Err(format!("unknown flag: {other}")),
        };
        *slot = Some(iter.next().ok_or("--model requires a value")?);
    }
    let model = model.ok_or("--model is required")?;
    Ok(ExplainArgs {
        model,
        gateway,
        admin_key,
        session_id,
    })
}

/// 渲染候选数组为对齐文本表（vendor/decision/reason_code/weight 四列）；
/// eligible 项追加 factors.total 与 selection 行。
pub fn render_candidates(candidates: &[Value]) -> String {
    let header = format!(
        "{:<20} {:<10} {:<24} {:<10}",
        "VENDOR", "DECISION", "REASON_CODE", "WEIGHT"
    );
    let mut lines = vec![header];
    for c in candidates {
        let vendor = c.get("vendor").and_then(Value::as_str).unwrap_or("");
        let decision = c.get("decision").and_then(Value::as_str).unwrap_or("");
        let reason = c.get("reason_code").and_then(Value::as_str).unwrap_or("");
        let weight = c
            .get("weight")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        lines.push(format!(
            "{:<20} {:<10} {:<24} {:<10}",
            vendor, decision, reason, weight
        ));
        if decision == "eligible" {
            if let Some(factors) = c.get("factors") {
                let total = factor_total(factors);
                lines.push(format!("    factors total: {total}"));
            }
            if let Some(selection) = c.get("selection").and_then(Value::as_str) {
                lines.push(format!("    selection: {selection}"));
            }
        }
    }
    lines.join("\n")
}

/// factors 各分量求和（model/queue/window/latency/error 数值字段）。
fn factor_total(factors: &Value) -> f64 {
    ["model", "queue", "window", "latency", "error"]
        .iter()
        .map(|key| factors.get(*key).and_then(Value::as_f64).unwrap_or(0.0))
        .sum()
}

/// CLI 入口：解析 → 转发 → 渲染。返回进程退出码（0 成功 / 1 失败 / 2 用法错误）。
pub async fn run_route_explain(args: Vec<String>) -> i32 {
    let args = match parse_args(args) {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("route-explain: {msg}\n{USAGE}");
            return 2;
        }
    };
    let client = match reqwest::Client::builder().build() {
        Ok(c) => c,
        Err(_) => return 1,
    };
    let url = format!("{}/admin/route/explain", args.gateway.trim_end_matches('/'));
    let mut body = serde_json::json!({ "model": args.model });
    if let Some(sid) = &args.session_id {
        body["session_id"] = Value::String(sid.clone());
    }
    let mut req = client.post(&url).json(&body);
    if let Some(key) = &args.admin_key {
        req = req.header("x-api-key", key);
    }
    let response = match req.send().await {
        Ok(r) => r,
        Err(_) => {
            eprintln!("route-explain: cannot reach gateway; start the gateway or pass --gateway");
            return 1;
        }
    };
    let status = response.status();
    if !status.is_success() {
        eprintln!("route-explain: gateway returned HTTP {status}");
        return 1;
    }
    let payload: Value = match response.json().await {
        Ok(v) => v,
        Err(_) => {
            eprintln!("route-explain: invalid response body");
            return 1;
        }
    };
    let candidates = payload
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    println!("{}", render_candidates(&candidates));
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_requires_model() {
        assert_eq!(
            parse_args(vec!["--gateway".into(), "http://x".into()]).unwrap_err(),
            "--model is required"
        );
    }

    #[test]
    fn parse_args_unknown_flag_is_error() {
        assert_eq!(
            parse_args(vec!["--model".into(), "m".into(), "--bogus".into()]).unwrap_err(),
            "unknown flag: --bogus"
        );
    }

    #[test]
    fn parse_args_defaults_gateway() {
        let a = parse_args(vec!["--model".into(), "m".into()]).expect("parse");
        assert_eq!(a.model, "m");
        assert_eq!(a.gateway, "http://127.0.0.1:8080");
        assert_eq!(a.admin_key, None);
        assert_eq!(a.session_id, None);
    }

    #[test]
    fn parse_args_all_flags() {
        let a = parse_args(vec![
            "--model".into(),
            "m".into(),
            "--gateway".into(),
            "http://gw:9000".into(),
            "--admin-key".into(),
            "k".into(),
            "--session-id".into(),
            "s".into(),
        ])
        .expect("parse");
        assert_eq!(a.model, "m");
        assert_eq!(a.gateway, "http://gw:9000");
        assert_eq!(a.admin_key.as_deref(), Some("k"));
        assert_eq!(a.session_id.as_deref(), Some("s"));
    }

    #[test]
    fn render_candidates_table_and_eligible_extras() {
        let candidates = vec![
            serde_json::json!({
                "vendor": "v1", "decision": "eligible", "reason_code": "ok",
                "weight": 1.5,
                "factors": {"model": 1.0, "queue": 2.0, "window": 0.0, "latency": 0.5, "error": 0.0},
                "selection": "primary"
            }),
            serde_json::json!({
                "vendor": "v2", "decision": "excluded", "reason_code": "no_budget", "weight": 0.0
            }),
        ];
        let rendered = render_candidates(&candidates);
        assert!(rendered.contains("VENDOR"));
        assert!(rendered.contains("v1"));
        assert!(rendered.contains("eligible"));
        assert!(rendered.contains("factors total: 3.5"));
        assert!(rendered.contains("selection: primary"));
        assert!(rendered.contains("v2"));
        assert!(rendered.contains("no_budget"));
        assert!(!rendered
            .lines()
            .any(|l| l.contains("v2") && l.contains("factors total")));
    }

    /// 本地 axum 测试服务器断言请求体与 x-api-key 头。
    #[tokio::test]
    async fn forward_sends_body_and_header() {
        use std::sync::{Arc, Mutex};
        type Captured = Option<(Value, Option<String>)>;
        let captured: Arc<Mutex<Captured>> = Arc::new(Mutex::new(None));
        let cap = captured.clone();
        let app = axum::Router::new().route(
            "/admin/route/explain",
            axum::routing::post(
                move |headers: axum::http::HeaderMap, axum::Json(body): axum::Json<Value>| {
                    let cap = cap.clone();
                    async move {
                        let key = headers
                            .get("x-api-key")
                            .and_then(|v| v.to_str().ok())
                            .map(str::to_string);
                        *cap.lock().unwrap() = Some((body, key));
                        axum::Json(serde_json::json!({"candidates": []}))
                    }
                },
            ),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let code = run_route_explain(vec![
            "--model".into(),
            "m1".into(),
            "--gateway".into(),
            format!("http://{addr}"),
            "--admin-key".into(),
            "secret-key".into(),
            "--session-id".into(),
            "s1".into(),
        ])
        .await;
        assert_eq!(code, 0);
        let (body, key) = captured.lock().unwrap().clone().expect("请求已发送");
        assert_eq!(body["model"], "m1");
        assert_eq!(body["session_id"], "s1");
        assert_eq!(key.as_deref(), Some("secret-key"));
    }

    /// 无网关（连接失败）→ 退出码 1。绑定本地端口后立刻释放，
    /// 保证该端口无监听，连接必然失败。
    #[tokio::test]
    async fn connection_failure_returns_exit_one() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let code = run_route_explain(vec![
            "--model".into(),
            "m".into(),
            "--gateway".into(),
            format!("http://{addr}"),
        ])
        .await;
        assert_eq!(code, 1, "无网关须退出码 1");
    }
}
