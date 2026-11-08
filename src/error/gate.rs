// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 路由核心错误：从原 `FluxweirError` 拆出的核心变体集（冻结）。
//! `Database` 等应用面变体留应用面（受孤儿规则约束）；应用错误经 `From<GateError>` 包装。
//! 错误双轨：`#[error]` Display 恒英文规范串，HTTP 展示走 [`GateError::user_message`]
//!（FTL，随 locale）；状态码/错误码/Retry-After 以固有方法暴露，由应用面 IntoResponse 消费。

use thiserror::Error;

use crate::i18n::{t, t_args};

/// 路由核心错误变体集（冻结）。
#[derive(Error, Debug)]
pub enum GateError {
    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("I/O error: {message}")]
    Io { message: String },

    #[error("Protocol conversion error: {source}")]
    Protocol {
        source: String,
        #[source]
        inner: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Vendor unavailable: {vendor}")]
    VendorUnavailable { vendor: String },

    /// 熔断器 Open 拒绝（`from_circuit_call` 对 `CircuitCallError::Open` 的
    /// 还原形态）：`vendor` 为纯厂商 id——结构化判别不靠字符串标记，
    /// fallback 触发器与 `is_circuit_open_error` 按变体匹配。
    #[error("Vendor circuit open: {vendor}")]
    VendorCircuitOpen { vendor: String },

    #[error("Request timeout: {vendor}")]
    Timeout { vendor: String },

    #[error("Rate limited: {vendor}")]
    RateLimited { vendor: String },

    #[error("Request validation failed: {field}")]
    Validation { field: String },

    #[error("Routing error: {message}")]
    Routing { message: String },

    /// `[routing].retry_after_hint` 开启时抛出：fallback 耗尽且全部候选因限流被排除，
    /// 映射 429 + Retry-After；开关关闭时永不产生本变体。
    #[error("All candidate vendors rate limited: retry after {retry_after_secs}s")]
    AllVendorsThrottled { retry_after_secs: u64 },

    /// 出站掩码不可用（feature `masking` + `[masking]`.enabled；检测/掩码/
    /// 还原任一步 Err 或 panic 的 fail-closed 折叠）：503 拒发，绝不原文出站。
    #[error("Masking unavailable: request refused (fail-closed)")]
    MaskingUnavailable,

    /// 出站掩码 `Block` 动作命中：400 拒绝，`kind` 为触发拦截的种类串。
    #[error("Masking blocked: {kind}")]
    MaskingBlocked { kind: String },
}

/// 载荷锚点本地化：`"<ftl-key>"` 或 `"<ftl-key>|{json-object}"`。
/// 锚名恒英文机器标识且必须是已登记的 FTL 键（键在 en/zh 至少一侧存在）；
/// 未登记或 JSON 损坏 → None（调用方走通用模板回退）。本地化只在
/// `user_message` 单点发生——`Display` 因此恒为语言中性串。
pub(crate) fn localize_anchor(payload: &str) -> Option<String> {
    use crate::i18n::{has_key, t_dynamic};
    match payload.split_once('|') {
        None => has_key(payload).then(|| t_dynamic(payload, &[])),
        Some((key, json)) => {
            if !has_key(key) {
                return None;
            }
            let obj: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(json).ok()?;
            let owned: Vec<(String, String)> = obj
                .into_iter()
                .map(|(k, v)| {
                    let value = v
                        .as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| v.to_string());
                    (k, value)
                })
                .collect();
            let refs: Vec<(&str, String)> =
                owned.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            Some(t_dynamic(key, &refs))
        }
    }
}

impl GateError {
    /// 固定客户端状态映射（u16：本 crate 不引 http 类型，应用层转 `StatusCode`）。
    pub fn status_code(&self) -> u16 {
        match self {
            GateError::Validation { .. } => 400,
            GateError::VendorUnavailable { .. } => 503,
            GateError::VendorCircuitOpen { .. } => 503,
            GateError::Timeout { .. } => 504,
            GateError::RateLimited { .. } => 429,
            GateError::Config { .. } => 500,
            GateError::Io { .. } => 500,
            GateError::Protocol { .. } => 502,
            GateError::Routing { .. } => 500,
            GateError::AllVendorsThrottled { .. } => 429,
            GateError::MaskingUnavailable => 503,
            GateError::MaskingBlocked { .. } => 400,
        }
    }

    /// 本地化用户消息（展示轨道）：随 locale 查 FTL 目录，写入响应体 error.message。
    pub fn user_message(&self) -> String {
        match self {
            GateError::Config { message } => localize_anchor(message)
                .unwrap_or_else(|| t_args("error-config", &[("message", message.clone())])),
            GateError::Io { message } => localize_anchor(message)
                .unwrap_or_else(|| t_args("error-io", &[("message", message.clone())])),
            GateError::Protocol { source, .. } => localize_anchor(source)
                .unwrap_or_else(|| t_args("error-protocol", &[("source", source.clone())])),
            // vendor 维度承载两类值：纯厂商 id（走模板插值）或语言中性锚点
            // （执行层以锚点描述具体原因）。锚点命中时直接作为用户消息，
            // 避免把键名/JSON 当厂商名插进模板。
            GateError::VendorUnavailable { vendor } => {
                localize_anchor(vendor).unwrap_or_else(|| {
                    t_args("error-vendor-unavailable", &[("vendor", vendor.clone())])
                })
            }
            GateError::VendorCircuitOpen { vendor } => {
                localize_anchor(vendor).unwrap_or_else(|| {
                    t_args("error-vendor-circuit-open", &[("vendor", vendor.clone())])
                })
            }
            GateError::Timeout { vendor } => localize_anchor(vendor)
                .unwrap_or_else(|| t_args("error-timeout", &[("vendor", vendor.clone())])),
            GateError::RateLimited { vendor } => localize_anchor(vendor)
                .unwrap_or_else(|| t_args("error-rate-limited", &[("vendor", vendor.clone())])),
            GateError::Validation { field } => localize_anchor(field)
                .unwrap_or_else(|| t_args("error-validation", &[("field", field.clone())])),
            GateError::Routing { message } => localize_anchor(message)
                .unwrap_or_else(|| t_args("error-routing", &[("message", message.clone())])),
            GateError::AllVendorsThrottled { retry_after_secs } => t_args(
                "error-all-vendors-throttled",
                &[("retry_after_secs", retry_after_secs.to_string())],
            ),
            GateError::MaskingUnavailable => t("masking-unavailable"),
            GateError::MaskingBlocked { kind } => {
                t_args("masking-blocked", &[("kind", kind.clone())])
            }
        }
    }

    /// 固定的错误类型代码字符串（不暴露内部枚举结构）
    pub fn error_code(&self) -> &'static str {
        match self {
            GateError::Config { .. } => "config_error",
            GateError::Io { .. } => "io_error",
            GateError::Protocol { .. } => "protocol_error",
            GateError::VendorUnavailable { .. } => "vendor_unavailable",
            GateError::VendorCircuitOpen { .. } => "vendor_circuit_open",
            GateError::Timeout { .. } => "timeout",
            GateError::RateLimited { .. } => "rate_limited",
            GateError::Validation { .. } => "validation_error",
            GateError::Routing { .. } => "routing_error",
            GateError::AllVendorsThrottled { .. } => "all_vendors_throttled",
            GateError::MaskingUnavailable => "masking_unavailable",
            GateError::MaskingBlocked { .. } => "masking_blocked",
        }
    }

    /// AllVendorsThrottled 携带的 Retry-After 头秒数，其余变体 None。
    pub fn retry_after_secs(&self) -> Option<u64> {
        match self {
            GateError::AllVendorsThrottled { retry_after_secs } => Some(*retry_after_secs),
            _ => None,
        }
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for GateError {
    /// oneshot 通道关闭（发送端提前 drop）是进程内同步失败，不是上游超时：
    /// 归 `Io`（500），避免把内部错误误标成可重试的 vendor 超时。
    fn from(_e: tokio::sync::oneshot::error::RecvError) -> Self {
        GateError::Io {
            message: "oneshot channel closed before response".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_anchors_localize_without_nested_sentences() {
        // field 存语言中性锚点：Display 恒英文规范串，user_message 单层本地化
        let not_found = GateError::Validation {
            field: format!(
                "error-validation-model-not-found|{}",
                serde_json::json!({"model": "GLM-4.5"})
            ),
        };
        assert!(
            not_found
                .to_string()
                .contains("error-validation-model-not-found"),
            "Display 必须携带语言中性锚点（英文键名）: {not_found}"
        );
        assert!(
            !not_found.to_string().contains('模'),
            "Display 不得混入本地化句子: {not_found}"
        );
        let message = not_found.user_message();
        assert_ne!(
            message, "error-validation-model-not-found",
            "键 'error-validation-model-not-found' 未注册进 FTL 目录"
        );
        assert!(
            message.contains("GLM-4.5"),
            "占位符 $model 必须被替换: {message}"
        );
        assert!(
            !message.contains("error-validation-model-not-found"),
            "本地化消息不得携带机器锚点: {message}"
        );

        let vendor = GateError::Validation {
            field: format!(
                "error-validation-vendor-unavailable|{}",
                serde_json::json!({"vendor": "openai"})
            ),
        };
        let vendor_message = vendor.user_message();
        assert_ne!(vendor_message, "error-validation-vendor-unavailable");
        assert!(
            vendor_message.contains("openai"),
            "占位符 $vendor 必须被替换: {vendor_message}"
        );
        assert!(
            !vendor_message.contains("error-validation-vendor-unavailable"),
            "本地化消息不得携带机器锚点: {vendor_message}"
        );

        // 未识别锚点原样走通用模板（既有行为）
        let generic = GateError::Validation {
            field: "model".to_string(),
        }
        .user_message();
        assert!(
            !generic.is_empty(),
            "通用 field 路径必须产出非空消息: {generic}"
        );
    }

    #[test]
    fn test_user_message_uses_ftl_catalog() {
        // 键必须在 FTL 目录注册（缺键时 t_args 回退键名本身），占位符必须被实参替换
        let message = GateError::Config {
            message: "bad key".to_string(),
        }
        .user_message();
        assert_ne!(
            message, "error-config",
            "键 'error-config' 未注册进 FTL 目录"
        );
        assert!(
            message.contains("bad key"),
            "占位符 $message 必须被替换: {message}"
        );

        let throttled = GateError::AllVendorsThrottled {
            retry_after_secs: 7,
        }
        .user_message();
        assert_ne!(
            throttled, "error-all-vendors-throttled",
            "键 'error-all-vendors-throttled' 未注册进 FTL 目录"
        );
        assert!(
            throttled.contains('7'),
            "占位符 $retry_after_secs 必须被替换: {throttled}"
        );
    }

    #[test]
    fn test_status_code_table() {
        let cases: Vec<(GateError, u16)> = vec![
            (
                GateError::Validation {
                    field: "model".into(),
                },
                400,
            ),
            (
                GateError::VendorUnavailable {
                    vendor: "openai".into(),
                },
                503,
            ),
            (
                GateError::Timeout {
                    vendor: "openai".into(),
                },
                504,
            ),
            (
                GateError::RateLimited {
                    vendor: "openai".into(),
                },
                429,
            ),
            (
                GateError::Config {
                    message: "x".into(),
                },
                500,
            ),
            (
                GateError::Protocol {
                    source: "openai".into(),
                    inner: None,
                },
                502,
            ),
            (
                GateError::Routing {
                    message: "x".into(),
                },
                500,
            ),
            (
                GateError::AllVendorsThrottled {
                    retry_after_secs: 1,
                },
                429,
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.status_code(), expected, "{err:?}");
        }
    }

    #[test]
    fn test_error_code_values() {
        assert_eq!(
            GateError::Validation {
                field: "model".into()
            }
            .error_code(),
            "validation_error"
        );
        assert_eq!(
            GateError::AllVendorsThrottled {
                retry_after_secs: 1
            }
            .error_code(),
            "all_vendors_throttled"
        );
    }

    #[test]
    fn test_retry_after_secs_only_for_all_throttled() {
        assert_eq!(
            GateError::AllVendorsThrottled {
                retry_after_secs: 30
            }
            .retry_after_secs(),
            Some(30)
        );
        assert_eq!(
            GateError::RateLimited {
                vendor: "openai".into()
            }
            .retry_after_secs(),
            None
        );
    }

    #[tokio::test]
    async fn test_error_from_recv_error() {
        // 构造 RecvError：发送端被丢弃后，接收端 await 会返回 RecvError
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        drop(tx);
        let recv_err = rx.await.unwrap_err();

        let err: GateError = recv_err.into();
        match err {
            GateError::Io { message } => {
                assert!(
                    message.contains("oneshot"),
                    "From<RecvError> 必须标明通道关闭: {message}"
                );
            }
            _ => panic!("From<RecvError> 应产生 Io 变体（内部同步失败，非上游超时）"),
        }
    }
}
