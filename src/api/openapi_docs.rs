// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! OpenAPI 文档面：主 REST 端点经 sdforge inventory 机制注册进全局 spec；
//! 文档路由由 `[docs]`.enabled 开关挂载（默认 false = 404）——开关本身即
//! 访问门（打开后免认证免限流，spec 非敏感）

/// OpenAPI 版本号（全条目统一；升级大版本时改此处一处）。
const OPENAPI_VERSION: &str = "v1";

/// 提交简单条目（无路径参数）并登记描述翻译键。
///
/// `#[forge]` 宏不可用（手工 `inventory::submit!` 属 const 上下文），
/// 故直接构造 `OpenApiRouteInfo` 填入 `i18n_key`：spec 生成期经
/// `sdforge::i18n::translate_or_fallback` 按当前 locale 查表，未注册的键
/// 回落编译期英文 `description`（见 [`register_openapi_i18n`]）。
macro_rules! submit_route_i18n {
    ($path:expr, $method:expr, $summary:expr, $desc:expr, $tags:expr, $i18n_key:expr) => {
        sdforge::inventory::submit! {
            sdforge::openapi::OpenApiRouteInfo {
                path: $path,
                method: $method,
                summary: $summary,
                description: $desc,
                version: OPENAPI_VERSION,
                tags: $tags,
                path_params: &[],
                success_status: None,
                body_params: &[],
                response_type: None,
                response_schema: None,
                i18n_key: Some($i18n_key),
                deprecated: false,
                sunset: None,
                successor: None,
            }
        }
    };
}

/// 提交带 `{id}` 路径参数的条目并登记描述翻译键。
macro_rules! submit_route_with_id_i18n {
    ($path:expr, $method:expr, $summary:expr, $desc:expr, $tags:expr, $id_desc:expr, $i18n_key:expr) => {
        sdforge::inventory::submit! {
            sdforge::openapi::OpenApiRouteInfo {
                path: $path,
                method: $method,
                summary: $summary,
                description: $desc,
                version: OPENAPI_VERSION,
                tags: $tags,
                path_params: &[sdforge::openapi::OpenApiPathParam::new(
                    "id", $id_desc, true, "string", ""
                )],
                success_status: None,
                body_params: &[],
                response_type: None,
                response_schema: None,
                i18n_key: Some($i18n_key),
                deprecated: false,
                sunset: None,
                successor: None,
            }
        }
    };
}

/// 提交带 `{model_action}` 路径参数的条目（Gemini `<model>:<action>` 同段）。
macro_rules! submit_gemini_model_action_i18n {
    ($path:expr, $method:expr, $summary:expr, $desc:expr, $tags:expr, $i18n_key:expr) => {
        sdforge::inventory::submit! {
            sdforge::openapi::OpenApiRouteInfo {
                path: $path,
                method: $method,
                summary: $summary,
                description: $desc,
                version: OPENAPI_VERSION,
                tags: $tags,
                path_params: &[sdforge::openapi::OpenApiPathParam::new(
                    "model_action",
                    "Gemini model and action (`<model>:<action>`)",
                    true,
                    "string",
                    "",
                )],
                success_status: None,
                body_params: &[],
                response_type: None,
                response_schema: None,
                i18n_key: Some($i18n_key),
                deprecated: false,
                sunset: None,
                successor: None,
            }
        }
    };
}

/// OpenAPI 条目描述键已并入 `locales/{en,zh}/api.ftl`（单一真源），
/// 本函数幂等预热：触发 i18n 门面全量注册（含 OpenAPI 键族）并同步
/// sdforge active locale，保证 spec 生成期 `translate_or_fallback`
/// 查表命中（未注册键回落编译期英文 `description`）。
pub(crate) fn register_openapi_i18n() {
    crate::i18n::init();
}

// OpenAI 兼容面
submit_route_i18n!(
    "/v1/chat/completions",
    "POST",
    "OpenAI chat completions",
    "Unified chat completions (OpenAI wire format) across vendors.",
    &["openai"],
    "api-openapi-chat-completions-description"
);
submit_route_i18n!(
    "/v1/messages",
    "POST",
    "Anthropic messages",
    "Unified messages API (Anthropic wire format) across vendors.",
    &["anthropic"],
    "api-openapi-messages-description"
);
submit_route_i18n!(
    "/v1/embeddings",
    "POST",
    "OpenAI embeddings",
    "Unified embeddings API across vendors.",
    &["openai"],
    "api-openapi-embeddings-description"
);
submit_route_i18n!(
    "/v1/rerank",
    "POST",
    "Cohere rerank",
    "Unified rerank API (Cohere wire format) across vendors; non-streaming, documents capped at 1000 per request.",
    &["rerank"],
    "api-openapi-rerank-description"
);
submit_route_i18n!(
    "/v1/responses",
    "POST",
    "OpenAI Responses",
    "OpenAI Responses API (semantic SSE). Requests convert to the unified chat IR and route across all chat vendors; native-only capabilities (built-in tools, background, prompt templates, file/audio input) route to api_style=\"responses\" vendors. Supports previous_response_id/conversation replay via the gateway store.",
    &["openai", "responses"],
    "api-openapi-responses-description"
);
submit_route_i18n!(
    "/v1/responses/input_tokens",
    "POST",
    "Responses input tokens",
    "Approximate input token estimation for a Responses request (chars/4 + structure overhead, deterministic).",
    &["openai", "responses"],
    "api-openapi-responses-input-tokens-description"
);
submit_route_with_id_i18n!(
    "/v1/responses/{id}",
    "GET",
    "Retrieve response",
    "Retrieve a stored Responses object (tenant-scoped).",
    &["openai", "responses"],
    "Response ID",
    "api-openapi-responses-get-description"
);
submit_route_with_id_i18n!(
    "/v1/responses/{id}",
    "DELETE",
    "Delete response",
    "Delete a stored Responses object (tenant-scoped).",
    &["openai", "responses"],
    "Response ID",
    "api-openapi-responses-delete-description"
);
submit_route_with_id_i18n!(
    "/v1/responses/{id}/cancel",
    "POST",
    "Cancel response",
    "Cancel a background Responses job and return the cancelled response object.",
    &["openai", "responses"],
    "Response ID",
    "api-openapi-responses-cancel-description"
);
submit_gemini_model_action_i18n!(
    "/v1beta/models/{model_action}",
    "POST",
    "Gemini generateContent",
    "Gemini native API (`generateContent`, `streamGenerateContent`, `countTokens`, `embedContent`, `batchEmbedContents`). Requests convert to the unified chat IR and route across vendors; safetySettings/cachedContent/built-in tools route to type_=\"gemini\" vendors.",
    &["gemini"],
    "api-openapi-gemini-generate-description"
);
submit_gemini_model_action_i18n!(
    "/v1beta/models/{model_action}",
    "GET",
    "Gemini model metadata",
    "Return one Gemini model entry from the gateway catalog.",
    &["gemini"],
    "api-openapi-gemini-model-description"
);
submit_route_i18n!(
    "/v1beta/models",
    "GET",
    "List Gemini models",
    "List routable models in Gemini `models[]` shape (abstract_id preferred, deduplicated) with supportedGenerationMethods.",
    &["gemini"],
    "api-openapi-gemini-models-list-description"
);
submit_route_i18n!(
    "/v1/models",
    "GET",
    "OpenAI models list",
    "Union of model catalog and enabled vendor models, deduplicated; vendor entries list their routable public name (abstract_id, else model_id). Supports ?task=chat|embedding|rerank filtering. Never cached.",
    &["openai"],
    "api-openapi-models-list-description"
);
submit_route_i18n!(
    "/v1/messages/count_tokens",
    "POST",
    "Anthropic token count",
    "Approximate input token estimation (chars/4 + per-message overhead, deterministic).",
    &["anthropic"],
    "api-openapi-count-tokens-description"
);
submit_route_i18n!(
    "/admin/route/explain",
    "POST",
    "Route explanation",
    "Read-only per-candidate routing assessment (reason codes + weight breakdown).",
    &["admin"],
    "api-openapi-route-explain-description"
);
submit_route_i18n!(
    "/admin/cost",
    "GET",
    "Cost ledger query",
    "Aggregated token and cost ledger grouped by vendor or model, with optional time window and API key filter.",
    &["admin"],
    "api-openapi-cost-usage-description"
);
// 管理面
submit_route_i18n!(
    "/admin/logs",
    "GET",
    "Query request logs",
    "Paginated request log query with filters.",
    &["admin"],
    "api-openapi-admin-logs-description"
);
submit_route_i18n!(
    "/admin/logs/search",
    "GET",
    "Search request logs",
    "Keyword search over request logs.",
    &["admin"],
    "api-openapi-admin-logs-search-description"
);
submit_route_i18n!(
    "/admin/stats/tokens",
    "GET",
    "Token usage stats",
    "Aggregated token usage statistics.",
    &["admin"],
    "api-openapi-admin-tokens-stats-description"
);
submit_route_i18n!(
    "/admin/sessions",
    "GET",
    "Session aggregates",
    "Session-level aggregates: cost, tokens, request count, first/last timestamps and a title from the first user message. Returns 404 when [usage_insights] is disabled.",
    &["admin"],
    "api-openapi-admin-sessions-description"
);
submit_route_i18n!(
    "/admin/context-stats",
    "GET",
    "Context utilisation stats",
    "Per-model context-window occupancy, cache-read share and five-bucket distribution. Returns 404 when [usage_insights] is disabled.",
    &["admin"],
    "api-openapi-admin-context-stats-description"
);
submit_route_i18n!(
    "/admin/metrics",
    "GET",
    "Operational metrics",
    "Internal operational metrics snapshot (JSON).",
    &["admin"],
    "api-openapi-admin-metrics-description"
);
submit_route_i18n!(
    "/admin/quota/ledger",
    "GET",
    "Quota ledger",
    "Per-window quota usage snapshot (tokens/cost used, reset time, scope: tenant/global/vendor). Optional `key` query filters by tenant digest.",
    &["admin"],
    "api-openapi-admin-quota-ledger-description"
);
submit_route_i18n!(
    "/admin/vendor-budgets",
    "GET",
    "Vendor budgets",
    "Declared vendor budgets with live usage, pace consumption ratio and reset countdown.",
    &["admin"],
    "api-openapi-admin-vendor-budgets-description"
);
submit_route_i18n!(
    "/admin/vendors",
    "GET",
    "List vendors",
    "Vendor status and pool overview.",
    &["admin"],
    "api-openapi-admin-vendors-description"
);
submit_route_i18n!(
    "/admin/vendors/config",
    "GET",
    "List vendor configs",
    "Raw VendorConfig list (config view; complements the status view of /admin/vendors).",
    &["admin"],
    "api-openapi-admin-vendor-configs-description"
);
submit_route_i18n!(
    "/admin/vendors/{id}",
    "PUT",
    "Update vendor",
    "Atomically replace one vendor config by id (validated read-modify-write under the config write lock). Unknown id yields 400.",
    &["admin"],
    "api-openapi-admin-vendor-update-description"
);
submit_route_i18n!(
    "/admin/config",
    "GET",
    "Get runtime config",
    "Current runtime configuration snapshot.",
    &["admin"],
    "api-openapi-admin-config-get-description"
);
submit_route_i18n!(
    "/admin/config",
    "POST",
    "Update runtime config",
    "Hot-update runtime configuration (validated, atomic).",
    &["admin"],
    "api-openapi-admin-config-update-description"
);
submit_route_i18n!(
    "/admin/models",
    "GET",
    "List models",
    "Model catalog listing.",
    &["admin"],
    "api-openapi-admin-models-description"
);
submit_route_with_id_i18n!(
    "/admin/vendors/{id}/status",
    "GET",
    "Vendor status",
    "Per-vendor status detail.",
    &["admin"],
    "Vendor ID",
    "api-openapi-admin-vendor-status-description"
);
// 目录自动同步（catalog-sync feature 门控：路由本体同门控，spec 条目
// 仅在该 feature 下登记，避免文档面出现不可达端点）
#[cfg(feature = "catalog-sync")]
submit_route_i18n!(
    "/admin/catalog/sync",
    "POST",
    "Sync model catalog",
    "Fetch the external model catalog source (add-only merge) into [[model_catalog]]; dry_run returns the diff without persisting.",
    &["admin"],
    "api-openapi-admin-catalog-sync-description"
);
submit_route_with_id_i18n!(
    "/admin/vendors/{id}/reset",
    "POST",
    "Reset vendor",
    "Reset per-vendor breakers and quotas.",
    &["admin"],
    "Vendor ID",
    "api-openapi-admin-vendor-reset-description"
);
submit_route_with_id_i18n!(
    "/admin/models/{id}",
    "POST",
    "Update model entry",
    "Legacy global-strategy update path (kept for compatibility).",
    &["admin"],
    "Model ID",
    "api-openapi-admin-model-update-description"
);
submit_route_with_id_i18n!(
    "/admin/models/{id}",
    "PUT",
    "Upsert model entry",
    "Create or replace a catalog entry.",
    &["admin"],
    "Model ID",
    "api-openapi-admin-model-upsert-description"
);
submit_route_with_id_i18n!(
    "/admin/models/{id}",
    "DELETE",
    "Delete model entry",
    "Remove a catalog entry.",
    &["admin"],
    "Model ID",
    "api-openapi-admin-model-delete-description"
);
// 健康与基础设施
submit_route_i18n!(
    "/api/v1/health",
    "GET",
    "Liveness probe",
    "Forge-registered liveness endpoint.",
    &["health"],
    "api-openapi-health-description"
);
submit_route_i18n!(
    "/readyz",
    "GET",
    "Readiness probe",
    "Aggregated component readiness (DB, cache, vendors).",
    &["health"],
    "api-openapi-readyz-description"
);
submit_route_i18n!(
    "/metrics",
    "GET",
    "Prometheus metrics",
    "RED metrics in Prometheus text exposition format.",
    &["health"],
    "api-openapi-metrics-description"
);

#[cfg(test)]
mod tests {
    use super::*;

    /// 描述翻译键与 inventory 条目对齐，且 en/zh 两档在 sdforge 注册表可解析：
    /// 注册缺失或键名手误时 spec 静默回落编译期英文，翻译不生效——在此钉住。
    #[test]
    fn openapi_descriptions_resolve_for_en_and_zh() {
        register_openapi_i18n();
        // (method, path, key)：同路径多方法的条目按 method 精确匹配
        for (method, path, key) in [
            ("GET", "/v1/models", "api-openapi-models-list-description"),
            (
                "POST",
                "/v1/messages/count_tokens",
                "api-openapi-count-tokens-description",
            ),
            (
                "POST",
                "/admin/route/explain",
                "api-openapi-route-explain-description",
            ),
            ("GET", "/admin/cost", "api-openapi-cost-usage-description"),
            (
                "POST",
                "/v1/chat/completions",
                "api-openapi-chat-completions-description",
            ),
            ("POST", "/v1/messages", "api-openapi-messages-description"),
            (
                "POST",
                "/v1/embeddings",
                "api-openapi-embeddings-description",
            ),
            ("POST", "/v1/rerank", "api-openapi-rerank-description"),
            ("POST", "/v1/responses", "api-openapi-responses-description"),
            (
                "POST",
                "/v1/responses/input_tokens",
                "api-openapi-responses-input-tokens-description",
            ),
            (
                "GET",
                "/v1/responses/{id}",
                "api-openapi-responses-get-description",
            ),
            (
                "DELETE",
                "/v1/responses/{id}",
                "api-openapi-responses-delete-description",
            ),
            (
                "POST",
                "/v1/responses/{id}/cancel",
                "api-openapi-responses-cancel-description",
            ),
            (
                "POST",
                "/v1beta/models/{model_action}",
                "api-openapi-gemini-generate-description",
            ),
            (
                "GET",
                "/v1beta/models/{model_action}",
                "api-openapi-gemini-model-description",
            ),
            (
                "GET",
                "/v1beta/models",
                "api-openapi-gemini-models-list-description",
            ),
            ("GET", "/admin/logs", "api-openapi-admin-logs-description"),
            (
                "GET",
                "/admin/logs/search",
                "api-openapi-admin-logs-search-description",
            ),
            (
                "GET",
                "/admin/stats/tokens",
                "api-openapi-admin-tokens-stats-description",
            ),
            (
                "GET",
                "/admin/sessions",
                "api-openapi-admin-sessions-description",
            ),
            (
                "GET",
                "/admin/context-stats",
                "api-openapi-admin-context-stats-description",
            ),
            (
                "GET",
                "/admin/metrics",
                "api-openapi-admin-metrics-description",
            ),
            (
                "GET",
                "/admin/quota/ledger",
                "api-openapi-admin-quota-ledger-description",
            ),
            (
                "GET",
                "/admin/vendor-budgets",
                "api-openapi-admin-vendor-budgets-description",
            ),
            (
                "GET",
                "/admin/vendors",
                "api-openapi-admin-vendors-description",
            ),
            (
                "GET",
                "/admin/vendors/config",
                "api-openapi-admin-vendor-configs-description",
            ),
            (
                "PUT",
                "/admin/vendors/{id}",
                "api-openapi-admin-vendor-update-description",
            ),
            (
                "GET",
                "/admin/config",
                "api-openapi-admin-config-get-description",
            ),
            (
                "POST",
                "/admin/config",
                "api-openapi-admin-config-update-description",
            ),
            (
                "GET",
                "/admin/models",
                "api-openapi-admin-models-description",
            ),
            (
                "GET",
                "/admin/vendors/{id}/status",
                "api-openapi-admin-vendor-status-description",
            ),
            (
                "POST",
                "/admin/vendors/{id}/reset",
                "api-openapi-admin-vendor-reset-description",
            ),
            (
                "POST",
                "/admin/models/{id}",
                "api-openapi-admin-model-update-description",
            ),
            (
                "PUT",
                "/admin/models/{id}",
                "api-openapi-admin-model-upsert-description",
            ),
            (
                "DELETE",
                "/admin/models/{id}",
                "api-openapi-admin-model-delete-description",
            ),
            ("GET", "/api/v1/health", "api-openapi-health-description"),
            ("GET", "/readyz", "api-openapi-readyz-description"),
            ("GET", "/metrics", "api-openapi-metrics-description"),
        ] {
            let route = sdforge::inventory::iter::<sdforge::openapi::OpenApiRouteInfo>()
                .into_iter()
                .find(|route| route.path == path && route.method == method)
                .unwrap_or_else(|| panic!("{method} {path} 必须登记进 OpenAPI inventory"));
            assert_eq!(
                route.i18n_key,
                Some(key),
                "{method} {path} 的描述翻译键失配"
            );
            for locale in ["en", "zh"] {
                assert_ne!(
                    sdforge::i18n::translate_for(locale, key, &[]),
                    key,
                    "{locale}/{key} 未注册——spec 将回落英文并静默丢失翻译"
                );
            }
        }
        // catalog-sync 条目按 feature 门控登记（关闭时不在 inventory 中）
        #[cfg(feature = "catalog-sync")]
        {
            let key = "api-openapi-admin-catalog-sync-description";
            let route = sdforge::inventory::iter::<sdforge::openapi::OpenApiRouteInfo>()
                .into_iter()
                .find(|route| route.path == "/admin/catalog/sync" && route.method == "POST")
                .expect("catalog-sync feature 下条目不登记");
            assert_eq!(route.i18n_key, Some(key));
            for locale in ["en", "zh"] {
                assert_ne!(sdforge::i18n::translate_for(locale, key, &[]), key);
            }
        }
    }
}
