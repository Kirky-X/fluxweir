// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Gemini 协议转换（入站 `generateContent`/`streamGenerateContent` 与出站
//! `type_ = "gemini"` 共用）。

pub mod convert_in;
pub mod convert_out;
pub mod schema;
pub mod stream;
pub mod types;

use crate::error::GateError;
use crate::protocol::{Protocol, ProtocolConverter, StreamChunk, UnifiedRequest, UnifiedResponse};
use serde_json::{json, Value};

pub use stream::GeminiStreamEncoder;
pub use types::{normalize_model, GeminiAction};

/// `ProtocolConverter` 的 Gemini 实现。
///
/// 有状态流式编码（工具片段缓冲/终止帧去重）由 [`GeminiStreamEncoder`]
/// 承担；trait 的 `stream_chunk_to_external` 仅提供单 chunk 的无状态投影。
#[derive(Debug, Default)]
pub struct GeminiConverter;

impl GeminiConverter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ProtocolConverter for GeminiConverter {
    fn input_protocol(&self) -> Protocol {
        Protocol::Gemini
    }

    fn output_protocol(&self) -> Protocol {
        Protocol::Gemini
    }

    fn to_unified(&self, body: Value) -> Result<UnifiedRequest, GateError> {
        let model = body
            .get("model")
            .and_then(Value::as_str)
            .ok_or(GateError::Validation {
                field: "api-gemini-model-required".to_string(),
            })?;
        let stream = body.get("stream").and_then(Value::as_bool).unwrap_or(false);
        convert_in::to_unified_request(&body, model, stream)
    }

    fn request_to_external(&self, request: UnifiedRequest) -> Result<Value, GateError> {
        convert_out::request_to_gemini(&request)
    }

    fn response_to_external(&self, response: UnifiedResponse) -> Result<Value, GateError> {
        Ok(convert_out::response_to_gemini(&response))
    }

    fn parse_response(&self, response: Value) -> Result<UnifiedResponse, GateError> {
        convert_out::parse_gemini_response(&response, "gemini")
    }

    fn stream_chunk_to_external(&self, chunk: StreamChunk) -> Result<Value, GateError> {
        Ok(match chunk {
            StreamChunk::Content {
                delta,
                finish_reason,
                ..
            } => {
                let parts = message_parts(&delta);
                let mut candidate =
                    json!({"content": {"role": "model", "parts": parts}, "index": 0});
                if let Some(reason) = finish_reason {
                    candidate["finishReason"] = json!(ir_finish_to_gemini(&reason));
                }
                json!({"candidates": [candidate]})
            }
            StreamChunk::ToolCall {
                index,
                id: _,
                name,
                arguments,
            } => {
                let args = arguments
                    .and_then(|arguments| serde_json::from_str::<Value>(&arguments).ok())
                    .filter(Value::is_object)
                    .unwrap_or_else(|| json!({}));
                json!({"candidates": [{
                    "content": {"role": "model", "parts": [{
                        "functionCall": {"name": name.unwrap_or_default(), "args": args}
                    }]},
                    "index": index,
                }]})
            }
            StreamChunk::Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens,
                cache_read_tokens,
                finish_reason,
                ..
            } => {
                let mut body = json!({"usageMetadata": {
                    "promptTokenCount": prompt_tokens,
                    "candidatesTokenCount": completion_tokens,
                    "totalTokenCount": total_tokens,
                }});
                if cache_read_tokens > 0 {
                    body["usageMetadata"]["cachedContentTokenCount"] = json!(cache_read_tokens);
                }
                if let Some(reason) = finish_reason {
                    body["candidates"] = json!([{
                        "content": {"role": "model", "parts": []},
                        "finishReason": ir_finish_to_gemini(&reason),
                        "index": 0,
                    }]);
                }
                body
            }
            StreamChunk::Heartbeat => json!({}),
        })
    }

    fn parse_stream_frame(&self, data: &str) -> Option<StreamChunk> {
        stream::parse_stream_frame(data)
    }
}

fn message_parts(content: &crate::protocol::MessageContent) -> Vec<Value> {
    use crate::protocol::MessageContent;
    match content {
        MessageContent::Text(text) => vec![json!({"text": text})],
        MessageContent::MultiModal(blocks) => blocks
            .iter()
            .map(crate::protocol::gemini::convert_out::block_to_part)
            .collect(),
        MessageContent::ToolCalls(calls) => calls
            .iter()
            .map(|call| {
                json!({"functionCall": {
                    "name": call.function.name,
                    "args": serde_json::from_str::<Value>(&call.function.arguments)
                        .ok()
                        .filter(Value::is_object)
                        .unwrap_or_else(|| json!({"raw": call.function.arguments})),
                }})
            })
            .collect(),
        MessageContent::TextWithToolCalls {
            text, tool_calls, ..
        } => {
            let mut parts = vec![json!({"text": text})];
            parts.extend(tool_calls.iter().map(|call| {
                json!({"functionCall": {
                    "name": call.function.name,
                    "args": serde_json::from_str::<Value>(&call.function.arguments)
                        .ok()
                        .filter(Value::is_object)
                        .unwrap_or_else(|| json!({"raw": call.function.arguments})),
                }})
            }));
            parts
        }
        MessageContent::ToolResult(result) => vec![json!({"functionResponse": {
            "name": result.function_name,
            "response": serde_json::from_str::<Value>(&result.content)
                .ok()
                .filter(Value::is_object)
                .unwrap_or_else(|| json!({"result": result.content})),
        }})],
        // Gemini 无思考 part：丢弃并 debug 显性化（归 gemini-api 功能面）
        MessageContent::Reasoning { .. } => {
            crate::protocol::drop_unsupported(
                crate::protocol::Protocol::Gemini,
                "message.reasoning",
                "no gemini reasoning part",
            );
            Vec::new()
        }
    }
}

fn ir_finish_to_gemini(raw: &str) -> &'static str {
    match raw {
        "length" => "MAX_TOKENS",
        "content_filter" => "SAFETY",
        "tool_calls" | "stop" | "function_call" => "STOP",
        _ => "OTHER",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_parts_cover_all_content_variants() {
        use crate::protocol::{MessageContent, ToolCall, ToolCallFunction, ToolResult};

        // 纯文本
        let parts = super::message_parts(&MessageContent::Text("hi".into()));
        assert_eq!(parts, vec![json!({"text": "hi"})]);

        // 工具调用：合法 JSON 对象 → 结构化 args；非对象 → raw 回退
        let call = |args: &str| {
            MessageContent::ToolCalls(vec![ToolCall {
                id: "c1".into(),
                call_type: "function".into(),
                function: ToolCallFunction {
                    name: "f".into(),
                    arguments: args.into(),
                },
            }])
        };
        let parts = super::message_parts(&call("{\"a\":1}"));
        assert_eq!(parts[0]["functionCall"]["args"], json!({"a": 1}));
        let parts = super::message_parts(&call("not-json"));
        assert_eq!(parts[0]["functionCall"]["args"], json!({"raw": "not-json"}));

        // 文本 + 工具调用：文本 part 在前
        let parts = super::message_parts(&MessageContent::TextWithToolCalls {
            text: "think".into(),
            tool_calls: match call("[1,2]") {
                MessageContent::ToolCalls(calls) => calls,
                _ => unreachable!(),
            },
            reasoning: None,
        });
        assert_eq!(parts[0]["text"], "think");
        assert_eq!(parts[1]["functionCall"]["args"], json!({"raw": "[1,2]"}));

        // 工具结果：对象 → response；非对象 → result 回退
        let result = |content: &str| {
            MessageContent::ToolResult(ToolResult {
                tool_call_id: "c1".into(),
                function_name: "f".into(),
                content: content.into(),
                is_error: false,
            })
        };
        let parts = super::message_parts(&result("{\"ok\":true}"));
        assert_eq!(
            parts[0]["functionResponse"]["response"],
            json!({"ok": true})
        );
        let parts = super::message_parts(&result("plain"));
        assert_eq!(
            parts[0]["functionResponse"]["response"],
            json!({"result": "plain"})
        );

        // reasoning 无 Gemini part：丢弃（debug 显性化）
        let parts = super::message_parts(&MessageContent::Reasoning {
            text: "r".into(),
            signature: None,
        });
        assert!(parts.is_empty());
    }

    #[test]
    fn ir_finish_to_gemini_maps_known_and_unknown() {
        assert_eq!(super::ir_finish_to_gemini("length"), "MAX_TOKENS");
        assert_eq!(super::ir_finish_to_gemini("content_filter"), "SAFETY");
        assert_eq!(super::ir_finish_to_gemini("tool_calls"), "STOP");
        assert_eq!(super::ir_finish_to_gemini("weird"), "OTHER");
    }

    #[test]
    fn protocols_are_gemini() {
        let converter = GeminiConverter::new();
        assert_eq!(converter.input_protocol(), Protocol::Gemini);
        assert_eq!(converter.output_protocol(), Protocol::Gemini);
        assert!(!converter.can_passthrough(Protocol::OpenAI));
        assert!(converter.can_passthrough(Protocol::Gemini));
    }

    #[test]
    fn trait_roundtrip_request_and_response() {
        let converter = GeminiConverter::new();
        let body = json!({
            "model": "gemini-2.5-pro",
            "contents": [{"role": "user", "parts": [{"text": "hi"}]}],
            "generationConfig": {"maxOutputTokens": 8}
        });
        let request = converter.to_unified(body).unwrap();
        assert_eq!(request.model, "gemini-2.5-pro");
        let external = converter.request_to_external(request).unwrap();
        assert_eq!(external["contents"][0]["parts"][0]["text"], "hi");
        assert_eq!(external["generationConfig"]["maxOutputTokens"], 8);
        assert!(external.get("model").is_none(), "model 只出现在 URL");

        let upstream = json!({
            "candidates": [{"content": {"parts": [{"text": "hello"}]}, "finishReason": "STOP"}],
            "usageMetadata": {"promptTokenCount": 1, "candidatesTokenCount": 2, "totalTokenCount": 3},
            "modelVersion": "gemini-2.5-pro"
        });
        let unified = converter.parse_response(upstream).unwrap();
        let back = converter.response_to_external(unified).unwrap();
        assert_eq!(
            back["candidates"][0]["content"]["parts"][0]["text"],
            "hello"
        );
        assert_eq!(back["candidates"][0]["finishReason"], "STOP");
    }

    #[test]
    fn stream_chunk_projection() {
        let converter = GeminiConverter::new();
        let text = converter
            .stream_chunk_to_external(StreamChunk::Content {
                index: 0,
                delta: crate::protocol::MessageContent::Text("hi".into()),
                finish_reason: None,
                logprobs: None,
            })
            .unwrap();
        assert_eq!(text["candidates"][0]["content"]["parts"][0]["text"], "hi");

        let usage = converter
            .stream_chunk_to_external(StreamChunk::Usage {
                prompt_tokens: 1,
                completion_tokens: 2,
                total_tokens: 3,
                cache_creation_1h_tokens: 0,
                cache_read_tokens: 0,
                cache_creation_tokens: 0,
                finish_reason: Some("length".into()),
            })
            .unwrap();
        assert_eq!(usage["usageMetadata"]["promptTokenCount"], 1);
        assert_eq!(usage["candidates"][0]["finishReason"], "MAX_TOKENS");
    }

    #[test]
    fn missing_model_is_validation_error() {
        let converter = GeminiConverter::new();
        assert!(converter.to_unified(json!({"contents": []})).is_err());
    }
}
