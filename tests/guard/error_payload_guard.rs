// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 错误载荷锚点契约守卫（feature 无关）：
//! 1. 构造点禁止直接本地化——`GateError`/`VaultError`/`FluxweirError` 的载荷
//!    字段与返回它们的 helper 只能接收语言中性锚点（`"<ftl键>"` 或
//!    `"<ftl键>|<json>"`），本地化只允许发生在 `user_message()` / 消息轨道终点。
//! 2. 锚点键必须已登记进 FTL（动态键 `t_dynamic` 绕过 parity 守卫的调用点
//!    字面扫描，键存在性由本守卫兜底）。
//!
//! 边界声明：本守卫是**token 扫描绊线**，不是编译器级证明。别名导入
//! （`use t as tr`）、宏中转、变量间接（`let m = t("k"); message: m`）可绕过；
//! 权威门禁是审查流程与 `user_message` 的行为测试。
//!
//! 范围声明：`src/sdk/` 不在扫描域。SDK 是对外客户端库，`FluxweirSdkError`
//! 的 Display 就是终态用户消息（没有服务端 `user_message()` 双层轨道），
//! 构造点本地化是设计语义而非契约违规；服务端错误（GateError/VaultError/
//! FluxweirError）才有「载荷中性、展示单点」契约。

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// 去掉 `#[cfg(test)]` 起的测试模块（约定置于文件尾）：测试夹具里的
/// `vendor: "v-pool-test"` 等字面量不是锚点，不应触发键登记校验。
/// 若生产代码出现在测试模块之后，本裁剪会漏扫——由审查流程兜底。
fn strip_test_tail(src: &str) -> &str {
    match src.find("\n#[cfg(test)]") {
        Some(idx) => &src[..idx],
        None => src,
    }
}

fn collect_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Str(String),
    Punct(char),
}

#[derive(Debug, Clone)]
struct Token {
    tok: Tok,
    line: usize,
}

/// 词法切分：跳过行/块注释与字符串内容（字符串以 `Str` 记原文），
/// 保留标识符、标点与行号。`::` 记为两个 `:` 标点，便于路径跳读。
fn tokenize(src: &str) -> Vec<Token> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                if chars[i] == '\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 2;
        } else if c == '"' {
            i += 1;
            let mut lit = String::new();
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    lit.push(chars[i]);
                    i += 1;
                    if i < chars.len() {
                        lit.push(chars[i]);
                        i += 1;
                    }
                    continue;
                }
                if chars[i] == '\n' {
                    line += 1;
                }
                lit.push(chars[i]);
                i += 1;
            }
            i += 1;
            out.push(Token {
                tok: Tok::Str(lit),
                line,
            });
        } else if c.is_alphanumeric() || c == '_' {
            let mut ident = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                ident.push(chars[i]);
                i += 1;
            }
            out.push(Token {
                tok: Tok::Ident(ident),
                line,
            });
        } else {
            out.push(Token {
                tok: Tok::Punct(c),
                line,
            });
            i += 1;
        }
    }
    out
}

const PAYLOAD_FIELDS: &[&str] = &["field", "message", "source", "vendor", "kind"];
const ERROR_HELPERS: &[&str] = &["validation", "schema_error", "protocol_error"];
const LOCALIZERS: &[&str] = &["t", "t_args"];

fn ident(tokens: &[Token], idx: usize) -> Option<&str> {
    match tokens.get(idx).map(|t| &t.tok) {
        Some(Tok::Ident(name)) => Some(name.as_str()),
        _ => None,
    }
}

fn punct(tokens: &[Token], idx: usize) -> Option<char> {
    match tokens.get(idx).map(|t| &t.tok) {
        Some(Tok::Punct(c)) => Some(*c),
        _ => None,
    }
}

/// 跳读限定路径 `a::b::` 与 `crate::i18n::` 形态，返回定位到 `t`/`t_args`
/// 之后的索引（若命中并紧跟 `(`）。
fn localizer_call_after(tokens: &[Token], mut idx: usize) -> Option<usize> {
    loop {
        let name = ident(tokens, idx)?;
        if LOCALIZERS.contains(&name) && punct(tokens, idx + 1) == Some('(') {
            return Some(idx);
        }
        // 允许路径前缀：ident :: ident ...
        if punct(tokens, idx + 1) == Some(':') && punct(tokens, idx + 2) == Some(':') {
            idx += 3;
            continue;
        }
        return None;
    }
}

/// 扫描「载荷构造点直接本地化」违规，返回 `行号: 片段`。
fn scan_localized_payloads(src: &str) -> Vec<String> {
    let tokens = tokenize(src);
    let mut hits = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        // 形态 1：`field|message|source|vendor|kind: [path::]t(`
        if let Some(name) = ident(&tokens, i) {
            if PAYLOAD_FIELDS.contains(&name) && punct(&tokens, i + 1) == Some(':') {
                if let Some(call_idx) = localizer_call_after(&tokens, i + 2) {
                    hits.push(format!("{}: {}: t(...)", tokens[i].line, name));
                    i = call_idx;
                }
            }
            // 形态 2：`validation|schema_error|protocol_error ( [path::]t(`
            if ERROR_HELPERS.contains(&name) && punct(&tokens, i + 1) == Some('(') {
                if let Some(call_idx) = localizer_call_after(&tokens, i + 2) {
                    hits.push(format!("{}: {}（t(...)）", tokens[i].line, name));
                    i = call_idx;
                }
            }
            // 形态 3：`VaultError|HistoryError :: Variant ( [path::]t(`
            if (name == "VaultError" || name == "HistoryError")
                && punct(&tokens, i + 1) == Some(':')
                && punct(&tokens, i + 2) == Some(':')
                && ident(&tokens, i + 3).is_some()
                && punct(&tokens, i + 4) == Some('(')
            {
                if let Some(call_idx) = localizer_call_after(&tokens, i + 5) {
                    hits.push(format!("{}: {name}::…(t(...))", tokens[i].line));
                    i = call_idx;
                }
            }
        }
        i += 1;
    }
    hits
}

/// 锚点键（kebab-case）从 `Str` token 提取：截断 `|` 之后的 JSON 部分。
fn anchor_key(lit: &str) -> Option<&str> {
    let key = lit.split('|').next()?.trim();
    let is_kebab = key.contains('-')
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    is_kebab.then_some(key)
}

/// 扫描「锚点键未登记」违规：载荷字段赋字面锚点、helper 传字面锚点、
/// `format!("<键>|…")` 三种形态。
fn scan_unregistered_anchor_keys(src: &str) -> Vec<String> {
    let tokens = tokenize(src);
    let mut hits = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        let Tok::Str(lit) = &token.tok else {
            continue;
        };
        // 只考察紧邻的前文形态：`field: "…"` / `field: format!("…")` /
        // `validation("…")` / `validation(format!("…"))` /
        // `HistoryError::Invalid("…")`（错误载荷构造器）
        let prev = tokens.get(i.wrapping_sub(1)).map(|t| t.tok.clone());
        // 形态 0：`HistoryError::Invalid("…")`
        let is_history_invalid = matches!(prev, Some(Tok::Punct('(')))
            && ident(&tokens, i.saturating_sub(2))
                .is_some_and(|n| matches!(n, "Invalid" | "NotFound"))
            && punct(&tokens, i.saturating_sub(3)) == Some(':')
            && punct(&tokens, i.saturating_sub(4)) == Some(':')
            && ident(&tokens, i.saturating_sub(5)).is_some_and(|n| n == "HistoryError");
        // 形态 A：`field: "…"`（字面量直接赋值）
        // 形态 B：`field: format!("…")`
        // 形态 C：`validation("…")` / `validation(format!("…"))`
        let is_payload_literal = is_history_invalid
            || match prev {
                Some(Tok::Punct(':')) => {
                    ident(&tokens, i.saturating_sub(2)).is_some_and(|n| PAYLOAD_FIELDS.contains(&n))
                }
                Some(Tok::Punct('!')) => {
                    ident(&tokens, i.saturating_sub(2)).is_some_and(|n| n == "format")
                        && punct(&tokens, i.saturating_sub(3)) == Some('(')
                        && (ident(&tokens, i.saturating_sub(4))
                            .is_some_and(|n| PAYLOAD_FIELDS.contains(&n))
                            || ident(&tokens, i.saturating_sub(4))
                                .is_some_and(|n| ERROR_HELPERS.contains(&n)))
                }
                Some(Tok::Punct('(')) => {
                    // validation("…")
                    ident(&tokens, i.saturating_sub(2))
                    .is_some_and(|n| ERROR_HELPERS.contains(&n))
                    // field: format!("…") / validation(format!("…"))
                    || (punct(&tokens, i.saturating_sub(2)) == Some('!')
                        && ident(&tokens, i.saturating_sub(3))
                            .is_some_and(|n| n == "format"))
                }
                _ => false,
            };
        if !is_payload_literal {
            continue;
        }
        if let Some(key) = anchor_key(lit) {
            if !fluxweir::i18n::has_key(key) {
                hits.push(format!("{}: {key}", token.line));
            }
        }
    }
    hits
}

#[test]
fn payload_localization_scanner_catches_and_passes_representative_forms() {
    let catch = [
        r#"let e = GateError::Validation { field: t("k") };"#,
        "let e = GateError::Validation { field: crate::i18n::t_args(\"k\", &[]) };",
        "let e = validation(t(\"k\"));",
        "let e = validation(crate::i18n::t_args(\"k\", &[]));",
        "let e = VaultError::Crypto(t_args(\"k\", &[(\"x\", y)]));",
        "let e = GateError::Protocol {\n    source: t_args(\n        \"k\",\n        &[],\n    ),\n};",
    ];
    for src in catch {
        assert!(
            !scan_localized_payloads(src).is_empty(),
            "必须抓到违规形态: {src}"
        );
    }

    let pass = [
        r#"let e = GateError::Validation { field: "k".to_string() };"#,
        "let e = GateError::Config { message: format!(\"k|{}\", serde_json::json!({})) };",
        "let e = validation(format!(\"k|{}\", serde_json::json!({ \"x\": y })));",
        "let e = VaultError::Crypto(\"k\".to_string());",
        "let e = HistoryError::Invalid(\"k|{}\");",
        "let e = HistoryError::NotFound(\"k\".to_string());",
        "tracing::warn!(message = %error, \"log line\");",
        "let log = t(\"log-key\");",
    ];
    for src in pass {
        assert!(
            scan_localized_payloads(src).is_empty(),
            "合法形态不得误报: {src} -> {:?}",
            scan_localized_payloads(src)
        );
    }
}

#[test]
fn anchor_key_scanner_catches_unregistered_and_accepts_registered() {
    let unregistered = [
        r#"let e = GateError::Config { message: "no-such-key|{}".to_string() };"#,
        "let e = GateError::Config { message: format!(\"no-such-key|{}\", serde_json::json!({})) };",
        "let e = validation(\"no-such-key|{}\");",
        "let e = validation(format!(\"no-such-key|{}\", serde_json::json!({})));",
    ];
    for src in unregistered {
        assert!(
            !scan_unregistered_anchor_keys(src).is_empty(),
            "必须抓到未登记键: {src}"
        );
    }
    let registered = [
        r#"let e = GateError::Config { message: "config-vendor-id-empty".to_string() };"#,
        "let e = GateError::Config { message: format!(\"config-vendor-id-empty|{}\", serde_json::json!({})) };",
        "let e = validation(\"model\");",
    ];
    for src in registered {
        assert!(
            scan_unregistered_anchor_keys(src).is_empty(),
            "已登记键/裸字段路径不得误报: {src} -> {:?}",
            scan_unregistered_anchor_keys(src)
        );
    }
}

#[test]
fn error_payload_construction_never_localizes_directly() {
    let root = repo_root().join("src");
    let mut files = collect_rs_files(&root);
    assert!(!files.is_empty(), "扫描面为空说明路径失效：{root:?}");
    // i18n 是本地化实现本体；sdk 是客户端库（Display 即终态消息，无双层轨道）
    files.retain(|p| {
        !p.ends_with("i18n/catalog.rs")
            && !p.ends_with("i18n/locale.rs")
            && !p.starts_with(repo_root().join("src/sdk"))
    });

    let mut offenders = Vec::new();
    for file in &files {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let text = strip_test_tail(&text);
        for hit in scan_localized_payloads(text) {
            let rel = file.strip_prefix(repo_root()).unwrap_or(file);
            offenders.push(format!("{}:{hit}", rel.display()));
        }
    }
    assert!(
        offenders.is_empty(),
        "错误载荷构造点不得直接本地化（改为语言中性锚点）: {offenders:#?}"
    );
}

#[test]
fn error_anchor_keys_are_registered() {
    let root = repo_root().join("src");
    let files = collect_rs_files(&root);
    assert!(!files.is_empty(), "扫描面为空说明路径失效：{root:?}");

    let mut missing = Vec::new();
    for file in &files {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let text = strip_test_tail(&text);
        for hit in scan_unregistered_anchor_keys(text) {
            let rel = file.strip_prefix(repo_root()).unwrap_or(file);
            missing.push(format!("{}:{hit}", rel.display()));
        }
    }
    assert!(
        missing.is_empty(),
        "锚点键未登记进 FTL（须 en/zh 至少一侧存在）: {missing:#?}"
    );
}
