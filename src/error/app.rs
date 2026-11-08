// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! `FluxweirError` 的应用面实现（展示轨道、HTTP 映射、来源转换）。
//! 类型定义在上级 `mod.rs`；本文件承载 impl，mod.rs 只保留类型与 re-export。

#[cfg(feature = "web-server")]
use axum::response::IntoResponse;
#[cfg(feature = "web-server")]
use serde_json::json;

use super::{FluxweirError, GateError};

#[cfg(feature = "web-server")]
use crate::i18n::t_args;

#[cfg(feature = "web-server")]
impl IntoResponse for FluxweirError {
    fn into_response(self) -> axum::response::Response {
        // AllVendorsThrottled 额外携带 Retry-After 响应头
        let retry_after_secs = match &self {
            FluxweirError::Gate(e) => e.retry_after_secs(),
            FluxweirError::Database { .. } => None,
            #[cfg(feature = "vault")]
            FluxweirError::Vault(_) => None,
        };
        let status = match &self {
            FluxweirError::Database { .. } => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            FluxweirError::Gate(e) => axum::http::StatusCode::from_u16(e.status_code())
                .expect("GateError::status_code must yield a valid HTTP status code"),
            #[cfg(feature = "vault")]
            FluxweirError::Vault(e) => match e {
                crate::vault::VaultError::NotFound(_) => axum::http::StatusCode::NOT_FOUND,
                crate::vault::VaultError::UnknownVendor(_) => axum::http::StatusCode::CONFLICT,
                _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            },
        };

        let body = json!({
            "error": {
                "message": self.user_message(),
                "type": self.error_code(),
            }
        });

        let mut response = (status, axum::Json(body)).into_response();
        if let Some(secs) = retry_after_secs {
            // u64 的十进制表示恒为可见 ASCII，`HeaderValue: From<u64>` 不会失败；
            // 旧实现的字符串 parse Err 分支不可达，已删除。
            response
                .headers_mut()
                .insert(axum::http::header::RETRY_AFTER, secs.into());
        }
        response
    }
}

#[cfg(feature = "web-server")]
impl FluxweirError {
    /// 本地化用户消息（展示轨道）：随 locale 查 FTL 目录。
    /// Database.message 为语言中性锚点（`"<ftl键>"` 或 `"<ftl键>|{json}"`，
    /// 锚点本身即英文规范串）——锚点命中时单层本地化，未识别内容走通用包裹。
    pub(crate) fn user_message(&self) -> String {
        match self {
            FluxweirError::Database { message } => super::gate::localize_anchor(message)
                .unwrap_or_else(|| t_args("error-database", &[("message", message.clone())])),
            FluxweirError::Gate(e) => e.user_message(),
            #[cfg(feature = "vault")]
            FluxweirError::Vault(e) => match e {
                crate::vault::VaultError::NotFound(id) => {
                    t_args("error-vault-not-found", &[("id", id.clone())])
                }
                crate::vault::VaultError::UnknownVendor(vendor) => {
                    t_args("error-vault-unknown-vendor", &[("vendor", vendor.clone())])
                }
                // 载荷本身是语言中性锚点（Display 带 "Xxx error: " 前缀，
                // 直接 to_string() 会让锚点键失效）——按变体取原始载荷。
                crate::vault::VaultError::MasterKey(message)
                | crate::vault::VaultError::Crypto(message) => {
                    super::gate::localize_anchor(message)
                        .unwrap_or_else(|| t_args("error-vault", &[("message", message.clone())]))
                }
                crate::vault::VaultError::Storage { message } => {
                    super::gate::localize_anchor(message)
                        .unwrap_or_else(|| t_args("error-vault", &[("message", message.clone())]))
                }
            },
        }
    }

    /// 固定的错误类型代码字符串（不暴露内部枚举结构）
    fn error_code(&self) -> &'static str {
        match self {
            FluxweirError::Database { .. } => "database_error",
            FluxweirError::Gate(e) => e.error_code(),
            #[cfg(feature = "vault")]
            FluxweirError::Vault(_) => "vault_error",
        }
    }
}

#[cfg(feature = "database")]
impl From<dbnexus::DbError> for FluxweirError {
    fn from(e: dbnexus::DbError) -> Self {
        // 原始错误文本（英文来源串）：Display 恒英文，本地化只在 user_message
        FluxweirError::Database {
            message: e.to_string(),
        }
    }
}

impl From<std::io::Error> for FluxweirError {
    fn from(e: std::io::Error) -> Self {
        // IO 错误归 GateError::Io 变体（错误码/展示双层一致），不混入 Database
        FluxweirError::Gate(GateError::Io {
            message: e.to_string(),
        })
    }
}

#[cfg(feature = "sdforge-integration")]
impl From<sdforge::config::ConfigError> for FluxweirError {
    fn from(e: sdforge::config::ConfigError) -> Self {
        FluxweirError::Gate(GateError::Config {
            message: e.to_string(),
        })
    }
}

#[cfg(all(test, feature = "web-server"))]
mod tests {
    use super::super::GateError;
    use super::FluxweirError;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    async fn extract_body(response: axum::response::Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("failed to read body");
        serde_json::from_slice(&bytes).expect("body is not valid JSON")
    }

    #[tokio::test]
    async fn test_error_validation_to_response() {
        let response = FluxweirError::from(GateError::Validation {
            field: "model".to_string(),
        })
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_error_vendor_unavailable_to_response() {
        let response = FluxweirError::from(GateError::VendorUnavailable {
            vendor: "openai".to_string(),
        })
        .into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_error_timeout_to_response() {
        let response = FluxweirError::from(GateError::Timeout {
            vendor: "openai".to_string(),
        })
        .into_response();
        assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
    }

    #[tokio::test]
    async fn test_error_rate_limited_to_response() {
        let response = FluxweirError::from(GateError::RateLimited {
            vendor: "anthropic".to_string(),
        })
        .into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_error_config_to_response() {
        let response = FluxweirError::from(GateError::Config {
            message: "bad config".to_string(),
        })
        .into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_error_protocol_to_response() {
        let response = FluxweirError::from(GateError::Protocol {
            source: "openai".to_string(),
            inner: None,
        })
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn test_error_database_to_response() {
        let response = FluxweirError::Database {
            message: "connection lost".to_string(),
        }
        .into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_error_routing_to_response() {
        let response = FluxweirError::from(GateError::Routing {
            message: "no vendor".to_string(),
        })
        .into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_error_all_vendors_throttled_to_response_with_retry_after() {
        // AllVendorsThrottled → 429 + Retry-After 头：秒数原样带出 + 固定错误代码
        let response = FluxweirError::from(GateError::AllVendorsThrottled {
            retry_after_secs: 30,
        })
        .into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after = response
            .headers()
            .get(axum::http::header::RETRY_AFTER)
            .expect("Retry-After header must be present")
            .to_str()
            .expect("Retry-After must be visible ASCII");
        assert_eq!(retry_after, "30");

        let body = extract_body(response).await;
        assert_eq!(body["error"]["type"], "all_vendors_throttled");
        assert!(
            body["error"]["message"]
                .as_str()
                .expect("message")
                .contains("30"),
            "message 应携带重试秒数"
        );
    }

    #[tokio::test]
    async fn test_error_response_body_structure() {
        let err = FluxweirError::from(GateError::Validation {
            field: "model".to_string(),
        });
        // error.message 走本地化轨道：必须等于 user_message 的输出
        let expected_message = err.user_message();
        let response = err.into_response();
        let body = extract_body(response).await;

        assert!(
            body["error"].is_object(),
            "response body must contain an 'error' object"
        );
        assert_eq!(body["error"]["message"], expected_message);
        let type_str = body["error"]["type"]
            .as_str()
            .expect("error.type must be a string");
        assert_eq!(
            type_str, "validation_error",
            "error.type should be the fixed error code, got: {}",
            type_str
        );
    }

    #[tokio::test]
    async fn test_error_from_dbnexus_error() {
        let db_err = dbnexus::DbError::Config("table missing".to_string());
        let err: FluxweirError = db_err.into();
        match err {
            FluxweirError::Database { message } => {
                assert!(
                    message.contains("table missing"),
                    "message should contain the original error text: {}",
                    message
                );
                // 语言中性：构造时不本地化（Display 恒英文契约），本地化只在 user_message
                assert!(
                    !message.contains("数据库"),
                    "From<DbError> 不得存储本地化文本: {}",
                    message
                );
            }
            _ => panic!("From<DbError> should produce Database variant"),
        }
    }

    #[tokio::test]
    async fn test_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file gone");
        let err: FluxweirError = io_err.into();
        match err {
            FluxweirError::Gate(GateError::Io { message }) => {
                assert!(
                    message.contains("file gone"),
                    "message should contain the original error text: {}",
                    message
                );
            }
            _ => panic!("From<io::Error> should produce Gate(Io) variant"),
        }
    }

    #[tokio::test]
    async fn database_anchor_localizes_single_layer() {
        // 锚点命中：message 语言中性，user_message 单层本地化且不含双层前缀
        let err = FluxweirError::Database {
            message: "database-logger-shutdown".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Database error: database-logger-shutdown",
            "Display 必须恒英文规范串"
        );
        let message = err.user_message();
        assert_ne!(message, "database-logger-shutdown", "键未注册");
        assert!(
            !message.contains("Database error:"),
            "锚点命中时不得叠加通用前缀: {message}"
        );

        let with_detail = FluxweirError::Database {
            message: format!(
                "database-connect|{}",
                serde_json::json!({"error": "pool timeout"})
            ),
        };
        let detail_message = with_detail.user_message();
        assert!(
            detail_message.contains("pool timeout"),
            "锚点明细必须带入: {detail_message}"
        );
        assert!(
            !detail_message.contains("database-connect"),
            "本地化后不得残留锚点: {detail_message}"
        );
    }

    /// Vault Storage 载荷是锚点：展示轨道必须解引用，不得把键名/JSON 带出站。
    #[test]
    #[cfg(feature = "vault")]
    fn vault_storage_anchor_localizes_without_key_leak() {
        let err = FluxweirError::Vault(crate::vault::VaultError::Storage {
            message: "vault-db-access-failed|{\"error\":\"db down\"}".to_string(),
        });
        let message = err.user_message();
        assert!(
            !message.contains("vault-db-access-failed"),
            "不得泄漏锚点键名: {message}"
        );
        assert!(!message.contains('|'), "不得泄漏锚点分隔符: {message}");
        assert!(
            message.contains("db down"),
            "锚点参数必须带入本地化文案: {message}"
        );
    }

    #[tokio::test]
    #[cfg(feature = "sdforge-integration")]
    async fn test_error_from_config_error() {
        let cfg_err = sdforge::config::ConfigError::FileNotFound {
            path: "/etc/config.toml".to_string(),
        };
        let err: FluxweirError = cfg_err.into();
        match err {
            FluxweirError::Gate(GateError::Config { message }) => {
                assert!(
                    message.contains("/etc/config.toml"),
                    "message should contain the original path: {}",
                    message
                );
                assert!(
                    message.contains("File not found"),
                    "message should contain the ConfigError text: {}",
                    message
                );
            }
            _ => panic!("From<ConfigError> should produce Gate(Config) variant"),
        }
    }
}
