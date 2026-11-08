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
            GateError::Config { message } => {
                t_args("error-config", &[("message", message.clone())])
            }
            GateError::Io { message } => t_args("error-io", &[("message", message.clone())]),
            GateError::Protocol { source, .. } => {
                t_args("error-protocol", &[("source", source.clone())])
            }
            GateError::VendorUnavailable { vendor } => {
                t_args("error-vendor-unavailable", &[("vendor", vendor.clone())])
            }
            GateError::VendorCircuitOpen { vendor } => {
                t_args("error-vendor-circuit-open", &[("vendor", vendor.clone())])
            }
            GateError::Timeout { vendor } => t_args("error-timeout", &[("vendor", vendor.clone())]),
            GateError::RateLimited { vendor } => {
                t_args("error-rate-limited", &[("vendor", vendor.clone())])
            }
            GateError::Validation { field } => {
                // 路由层 Validation 以「锚点:取值」形态携带机器可读信息
                //（field 恒英文、无本地化句子）；本地化在此单层完成，
                // 未识别锚点走通用模板回退
                if let Some(model) = field.strip_prefix("model-not-found:") {
                    t_args(
                        "error-validation-model-not-found",
                        &[("model", model.to_string())],
                    )
                } else if let Some(model) = field.strip_prefix("global-model-disabled:") {
                    t_args(
                        "error-validation-global-model-disabled",
                        &[("model", model.to_string())],
                    )
                } else if let Some(vendor) = field.strip_prefix("vendor-unavailable:") {
                    t_args(
                        "error-validation-vendor-unavailable",
                        &[("vendor", vendor.to_string())],
                    )
                } else {
                    t_args("error-validation", &[("field", field.clone())])
                }
            }
            GateError::Routing { message } => {
                t_args("error-routing", &[("message", message.clone())])
            }
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
    fn from(_e: tokio::sync::oneshot::error::RecvError) -> Self {
        GateError::Timeout {
            vendor: "unknown".to_string(),
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
            field: "model-not-found:GLM-4.5".to_string(),
        };
        assert_eq!(
            not_found.to_string(),
            "Request validation failed: model-not-found:GLM-4.5",
            "Display 必须恒英文规范串，不得混入本地化句子"
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
            !message.contains("model-not-found:"),
            "本地化消息不得携带机器锚点前缀: {message}"
        );

        let vendor = GateError::Validation {
            field: "vendor-unavailable:openai".to_string(),
        };
        let vendor_message = vendor.user_message();
        assert_ne!(vendor_message, "error-validation-vendor-unavailable");
        assert!(
            vendor_message.contains("openai"),
            "占位符 $vendor 必须被替换: {vendor_message}"
        );
        assert!(
            !vendor_message.contains("vendor-unavailable:"),
            "本地化消息不得携带机器锚点前缀: {vendor_message}"
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
            GateError::Timeout { vendor } => {
                assert_eq!(
                    vendor, "unknown",
                    "From<RecvError> should set vendor to 'unknown'"
                );
            }
            _ => panic!("From<RecvError> should produce Timeout variant"),
        }
    }
}
