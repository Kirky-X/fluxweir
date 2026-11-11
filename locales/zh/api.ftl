# zh/api: api 桶（src/api）用户可见消息。zh 值 = 原中文保留或忠实翻译。

# ---- admin.rs ----
admin-vendor-reset = 厂商 { $id } 的密钥池黑名单与冷却状态已清空
admin-config-updated-immediate = 配置已更新；vendors/pricing/auth 立即生效
admin-config-updated-restart-required = 配置已更新；vendors/pricing/auth 立即生效；{ $sections } 重启后生效
admin-vendor-not-found = 厂商 { $id } 不存在
# ---- /admin/models 模型能力目录 CRUD ----
admin-model-upserted = 模型 { $model } 已登记至目录
admin-model-not-found = 模型 { $model } 不在目录中
admin-model-deleted = 模型 { $model } 已从目录移除
admin-vendor-updated = 厂商已更新
admin-global-model-updated = 全局模型配置已更新
admin-only-global-model-supported = 仅支持 'global' 模型配置
# ---- admin.rs /admin/cost 成本分账查询 ----
admin-cost-group-by-invalid = group_by（必须为 vendor|model）
admin-cost-date-format-invalid = { $name }（必须为 YYYY-MM-DD）
admin-masking-ledger-date-invalid = 日期（必须为 YYYY-MM-DD）
admin-masking-ledger-disabled = 掩码台账未启用（[masking].ledger.enabled 为 false）
admin-cost-usage-sink-unwired = usage sink 未接线；成本分账不可用
admin-token-stats-usage-sink-unwired = usage sink 未接线；token 统计不可用
admin-vendor-reset-unknown-vendor = 厂商 '{ $id }'（未知厂商）
# ---- admin.rs /admin/embedding-pins 嵌入 pin 管理 ----
admin-embedding-pins-unwired = 嵌入 pin 表未接线；端点不可用
admin-embedding-pin-reset = 嵌入 pin '{ $abstract }' 已清空；下一嵌入请求按权重重新粘着（可能换厂商）
admin-embedding-pin-reset-absent = 嵌入 pin '{ $abstract }' 无记录；无需清空

# ---- router_builder.rs ----
router-rate-limit-exceeded = 已超过速率限制
router-admission-rejected = 准入被拒绝（拥塞）
router-access-denied = 访问被拒绝
router-governor-init-failed = Governor 初始化失败
router-inbound-rate-limit-init-failed = 入站限流初始化失败
# 全局 tower 限流层内层错误：原文只进日志，响应体为通用 500 文案。
router-global-rate-limit-layer-error = 全局限流层错误：{ $error }
router-internal-server-error = 内部服务器错误

# ---- readiness.rs（就绪探针组件检查失败：details 只给固定码，原文进日志）----
api-readiness-db-probe-failed = 就绪探针 DB 探测失败：{ $error }
api-readiness-db-session-failed = 就绪探针 DB 会话获取失败：{ $error }
api-readiness-cache-probe-failed = 就绪探针缓存后端探测失败：{ $error }

# ---- api/mod.rs ----
api-log-truncated = ...[已截断，共 { $total_bytes } 字节]
api-amplification-diverging = 重试放大发散：r·f > 0.95（λ_eff 无界）
api-amplification-tightening = 失败率逼近临界 f_c；本请求 max_total_attempts 收紧为 1
api-vendor-failed-rerouting = vendor 失败；切换 fallback 重路由
api-vendor-failed-rerouting-pre-stream = vendor 在响应/流建立前失败；切换 fallback 重路由

# ---- api/completions.rs / api/anthropic.rs ----
api-log-push-rejected = 请求日志推送被拒：{ $error }
api-usage-record-rejected = Token 用量记账被拒：{ $error }
api-embeddings-openai-only = embeddings 仅支持 OpenAI 类型厂商
api-rerank-capability-mismatch = 重排请求命中的厂商或模型不具备 rerank 能力：请检查 X-Vendor/X-Model 点名，或为条目声明 model_type = "rerank"
api-embeddings-dimension-mismatch = 厂商 { $vendor }/{ $model } 的嵌入维度漂移：首次响应 { $old } 维，本次 { $new } 维——不同向量空间的向量不得混用；若上游模型已变更，可 POST /admin/embedding-pins/{ $abstract }/reset 重置 pin

# ---- api/response_cache.rs ----
api-response-cache-leader-not-cached = 响应缓存：leader 未缓存，follower 自愈回源
api-response-cache-library-fault = 响应缓存：库侧故障，直接派发

# ---- api/semantic_cache.rs ----
# BgeEmbedder::load 的失败载荷：经 bootstrap 的 %error 字段落到日志面。
api-semantic-cache-embedder-load-failed = 语义缓存嵌入器加载失败：{ $error }
# BgeEmbedder::embed 的失败载荷：消费点（lookup/store）静默计数丢弃，无用户可见面。
api-semantic-cache-embed-failed = 语义缓存嵌入失败：{ $error }

# ---- api/count_tokens.rs / api/route_explain.rs ----
api-count-tokens-model-invalid = model（缺失或非字符串）
api-count-tokens-messages-invalid = messages（缺失或非数组）
api-count-tokens-messages-empty = messages（不得为空）
api-route-explain-model-invalid = model（缺失或非字符串）
api-route-explain-model-unknown = model '{ $model }'（未知模型）

# ---- 不变量 panic 文案（api/{anthropic,completions}.rs 最终路由 / api/route_explain.rs 入选者字段）----
api-final-route-missing = leader/自愈/未参与路径闭包必产出最终路由
api-explain-no-factors = 入选者必有分解
api-explain-no-weight = 入选者必有权重
api-explain-no-base = 入选者必有合成基准
api-explain-no-selection = 入选者必有档位

# ---- api/health.rs（#[forge] description，经 sdforge::i18n::register_translation 运行时注册） ----
api-health-description = 健康检查端点

# ---- api/openapi_docs.rs（OpenAPI description，经 sdforge::i18n::register_translation 运行时注册） ----
api-openapi-models-list-description = 模型目录与已启用厂商模型的并集，去重；厂商条目列可路由的公开名（abstract_id，缺省 model_id）；不缓存
api-openapi-count-tokens-description = 近似输入 token 估算（字符数/4 + 每条消息结构开销，确定性）
api-openapi-route-explain-description = 只读的逐候选路由评估（原因码 + 权重分解）
api-openapi-cost-usage-description = 按厂商或模型聚合的 token 与成本分账，支持可选时间窗与 API key 过滤
api-responses-disabled = Responses 端点已禁用（[responses].enabled = false）
api-responses-store-failed = Responses 有状态存储写入失败（响应不受影响）：{ $error }
api-responses-purge-failed = Responses 过期行清理失败：{ $error }
api-responses-store-too-large = Responses 存储被拒：单列 JSON 超过 max_response_bytes 上限
api-responses-not-found = Responses 记录不存在或已过期：{ $id }
api-responses-store-unavailable = Responses 有状态存储未接线（[responses] 运行时缺失）
api-responses-native-hint-vendor = X-Vendor "{ $vendor }" 不具备 Responses 原生能力：请声明 type_ = "openai" 且 api_style = "responses"
api-responses-native-hint-model = X-Model "{ $model }" 无 Responses 原生厂商承载：请检查厂商 api_style 与启用状态
api-responses-orphans-failed = Responses 孤儿后台任务已置 failed：{ $count } 条
api-responses-orphan-cleanup-failed = Responses 孤儿任务清理失败（启动继续）：{ $error }

# ---- api/gemini.rs（Gemini 原生入站） ----
api-gemini-action-required = Gemini 请求路径必须包含 ":<action>"（generateContent / streamGenerateContent / countTokens / embedContent / batchEmbedContents）
api-gemini-action-unknown = 未知的 Gemini action "{ $action }"
api-gemini-model-required = Gemini 请求缺少 model（路径 /models/{"{"}model{"}"}:{"{"}action{"}"}）
api-gemini-model-mismatch = Gemini body 中的 model "{ $body }" 与路径 model "{ $path }" 不一致
api-gemini-contents-invalid = Gemini "contents" 必须是非空数组
api-gemini-parts-invalid = Gemini content 的 "parts" 必须是非空数组
api-gemini-part-invalid = Gemini content part 形状非法
api-gemini-part-unsupported = 网关不支持该 Gemini content part 类型
api-gemini-tool-name-required = Gemini function declaration/functionCall 缺少 "name"
api-gemini-tool-unsupported = 不含 functionDeclarations 的 Gemini tools 仅支持原生直通
api-gemini-function-response-name-missing = Gemini functionResponse 缺少 name 且无法从历史工具调用反查
api-gemini-response-invalid = Gemini 上游响应缺少 candidates/output
api-gemini-embed-content-invalid = Gemini embedContent 需要 "content.parts[].text"
api-gemini-embed-requests-invalid = Gemini batchEmbedContents 需要非空 "requests" 数组
api-gemini-embed-dimensions-conflict = Gemini batchEmbedContents 各请求的 outputDimensionality 必须一致
api-gemini-schema-ref-cycle = Gemini responseSchema 的 $ref 存在环：{ $detail }
api-gemini-schema-ref-depth = Gemini responseSchema 的 $ref 超过最大展开深度：{ $detail }
api-gemini-schema-ref-missing = Gemini responseSchema 的 $ref 目标不存在：{ $detail }
api-gemini-schema-ref-unsupported = Gemini responseSchema 的 $ref 必须指向 #/$defs 或 #/definitions：{ $detail }
api-gemini-native-vendor-mismatch = X-Vendor "{ $vendor }" 不是 Gemini 原生厂商
router-gemini-no-native-vendor = 模型 "{ $model }" 无 Gemini 原生厂商：请声明 type_ = "gemini" 并启用厂商/模型
middleware-gemini-unauthorized = Gemini 请求缺少有效的 API key

usage-insights-disabled = 会话/上下文统计已关闭；启用 [usage_insights] 后可用

# ---- api/admin.rs（订阅账号管理面，subscription-vendor feature）----
subscription-admin-disabled = 订阅账号已关闭；启用 [subscription] 与 subscription-vendor feature 后可用
subscription-client-id-missing = 未配置 [subscription].client_id：发起授权需要 OAuth 应用 client id
subscription-account-unknown = 订阅账号「{ $account }」未在池中登记
subscription-account-unknown-vendor = vendor「{ $vendor }」与任何 [[vendors]] id 都对不上

# ---- api/web_search.rs（搜索注入与守护抓取，web-search feature）----
web-search-disabled = 搜索与抓取端点已关闭；启用 [web_search] 后可用
netguard-blocked = 出站抓取被拒绝：目标解析到受保护网段（{ $tag }）
fetch-unsupported-media = 出站抓取仅支持 text/html、text/plain、text/markdown 或 application/json 响应
fetch-body-too-large = 出站抓取响应超过 1 MiB 上限
search-all-providers-failed = 全部搜索源失败：{ $summary }
search-no-providers = 未配置搜索源；请添加 [[web_search.providers]] 条目
config-web-search-unknown-kind = 未知的搜索源类型 "{ $kind }"（合法值 tavily、brave、exa、searxng）
config-web-search-missing-base-url = SearXNG 源必须显式配置 base_url
config-web-search-missing-key = 搜索源 "{ $kind }" 至少需要一个 key 来源（api_key_env、api_key_envs 或 api_key_vault_ref）
config-otlp-endpoint-invalid = OTLP 端点不是合法 URL：{ $value }
config-otlp-endpoint-scheme = OTLP 端点 scheme 必须是 http 或 https，实际 "{ $value }"
config-otlp-endpoint-userinfo = OTLP 端点不得内嵌凭证（userinfo）

# ---- api/admin.rs（catalog-sync：目录自动同步管理面） ----
catalog-sync-disabled = 目录同步已关闭；启用 [catalog_sync].enabled（并以 --features catalog-sync 构建）后可用
catalog-sync-in-flight = 已有一次目录同步在进行中；请等其完成后再试
catalog-sync-failed = 目录同步失败：{ $reason }
config-catalog-sync-timeout-zero = [catalog_sync].timeout_secs 必须至少为 1
config-catalog-sync-source-url-invalid = [catalog_sync].source_url 不是合法 URL：{ $value }
config-catalog-sync-source-url-scheme = [catalog_sync].source_url 必须使用 HTTPS（loopback 上的 http 放行），实际 "{ $value }"
config-catalog-sync-source-url-userinfo = [catalog_sync].source_url 不得内嵌凭证（userinfo）
config-catalog-sync-alias-blank = [catalog_sync.alias] 表键值不得为空白
catalog-sync-parse-failed = 目录源无法按 JSON 解析：{ $error }
catalog-sync-source-skipped = 目录源条目已跳过（缺定价或上下文窗口）
main-catalog-sync-started = 目录同步周期任务已启动（间隔 { $interval_secs } 秒）
main-catalog-sync-synced = 目录同步完成：新增 { $added } 条
main-catalog-sync-failed = 目录同步失败（fail-open，目录保持原样）：{ $error }

# Responses 端点校验与历史解析（错误经 error-validation 模板的 { $field } 槽位插值）
api-responses-background-native-only = background: 原生能力请求不支持后台模式
api-responses-background-disabled = background: 已被 [responses].background_enabled 禁用
api-responses-background-store-required = background 需要 store=true
api-responses-background-store-missing = background: 状态存储未接线
api-responses-background-throttled = 后台响应并发数已达上限，请稍后重试。
api-responses-input-required = input: 不能为空
api-responses-cancel-background-only = id: 只有后台响应可以取消
api-responses-history-mutually-exclusive = previous_response_id/conversation: 互斥
api-responses-history-cycle = previous_response_id: 在 { $id } 处检测到循环
api-responses-history-depth = previous_response_id: 链深超过 100
api-gemini-model-not-found = 未找到 Gemini 模型: { $model }

# web-search 端点（SearchError 消息经 search-all-providers-failed 的 { $summary } 聚合返回）
search-upstream-bad-status = 上游状态码非法
search-key-pool-empty = key 池为空
search-key-unusable = 无可用 key
search-key-env-missing = 环境变量 { $name } 未设置
search-vault-unwired = vault 未接线
search-vault-disabled = vault 特性未启用

# 订阅 OAuth 用户面（Display 规范串仅作 detail，外层语义本地化）
admin-subscription-oauth = 订阅 OAuth 端点错误（状态码 { $status }）
admin-subscription-token-invalid = 订阅 token 响应非法（缺少字段 { $field }）
admin-subscription-transport = 订阅传输错误：{ $detail }
admin-subscription-degraded = 订阅账号处于降级冷却期，请稍后重试
subscription-vault-unwired = vault 存储未接线
api-responses-history-lookup-failed = 响应历史查询失败
api-responses-store-op-failed = 响应存储操作失败，详情见服务端日志

# ---- api/responses.rs（responses 存储运维日志，原文经 error 结构化字段承载）----
api-log-responses-store-insert-failed = 响应存储插入失败
api-log-responses-store-op-failed = 响应存储操作失败
api-log-responses-history-lookup-failed = 响应历史查询失败
api-log-responses-conversation-lookup-failed = 响应会话查询失败

# ---- OpenAPI 条目描述（sdforge translate_or_fallback 消费；原 openapi_docs.rs 手工注册迁入，单一真源）----
api-openapi-chat-completions-description = 跨厂商统一聊天补全（OpenAI wire 形态）
api-openapi-messages-description = 跨厂商统一 messages API（Anthropic wire 形态）
api-openapi-embeddings-description = 跨厂商统一 embeddings API
api-openapi-rerank-description = 跨厂商统一重排 API（Cohere wire 形态）；非流式，单请求 documents 上限 1000 条
api-openapi-responses-description = OpenAI Responses API（语义化 SSE）。请求转换为统一聊天 IR 并跨全部聊天厂商路由；原生能力（内置工具/后台/prompt 模板/文件与音频输入）路由到 api_style="responses" 厂商。经网关存储支持 previous_response_id/conversation 回放
api-openapi-responses-input-tokens-description = Responses 请求的近似输入 token 估算（字符/4 + 结构开销，确定性）
api-openapi-responses-get-description = 按租户隔离检索已存 Responses 对象
api-openapi-responses-delete-description = 按租户隔离删除已存 Responses 对象
api-openapi-responses-cancel-description = 取消后台 Responses 任务并返回 cancelled 响应对象
api-openapi-gemini-generate-description = Gemini 原生 API（generateContent、streamGenerateContent、countTokens、embedContent、batchEmbedContents）。请求转换为统一聊天 IR 并跨厂商路由；safetySettings/cachedContent/内置工具路由到 type_="gemini" 厂商
api-openapi-gemini-model-description = 从网关目录返回单个 Gemini 模型条目
api-openapi-gemini-models-list-description = 以 Gemini models[] 形态列出可路由模型（abstract_id 优先、去重）并附 supportedGenerationMethods
api-openapi-admin-logs-description = 带过滤条件的分页请求日志查询
api-openapi-admin-logs-search-description = 请求日志关键词搜索
api-openapi-admin-tokens-stats-description = 聚合 token 用量统计
api-openapi-admin-sessions-description = 会话级聚合：成本、token、请求数、首末时间戳与首条 user 消息标题。[usage_insights] 关闭时返回 404
api-openapi-admin-context-stats-description = 逐模型上下文窗口占用率、缓存读取占比与五档分布。[usage_insights] 关闭时返回 404
api-openapi-admin-metrics-description = 内部运维指标快照（JSON）
api-openapi-admin-quota-ledger-description = 逐窗口配额用量快照（已用 token/成本、重置时间、作用域：租户/全局/厂商）。可选 key 查询按租户摘要过滤
api-openapi-admin-vendor-budgets-description = 已声明厂商预算及实时用量、消耗进度比与重置倒计时
api-openapi-admin-vendors-description = 厂商状态与池概览
api-openapi-admin-vendor-configs-description = 原始 VendorConfig 列表（配置视图；与 /admin/vendors 状态视图互补）
api-openapi-admin-vendor-update-description = 按 id 原子替换单个厂商配置（配置写锁下校验读改写）。未知 id 返回 400
api-openapi-admin-config-get-description = 当前运行时配置快照
api-openapi-admin-config-update-description = 热更新运行时配置（校验后原子生效）
api-openapi-admin-models-description = 模型目录列表
api-openapi-admin-vendor-status-description = 单个厂商的状态明细
api-openapi-admin-catalog-sync-description = 拉取外部模型目录源（仅新增合并）进 [[model_catalog]]；dry_run 返回差异不落库
api-openapi-admin-vendor-reset-description = 重置单厂商熔断器与配额
api-openapi-admin-model-update-description = 旧版全局策略更新路径（兼容保留）
api-openapi-admin-model-upsert-description = 创建或替换目录条目
api-openapi-admin-model-delete-description = 删除目录条目
api-openapi-health-description = forge 注册的存活探针端点
api-openapi-readyz-description = 聚合组件就绪度（DB、缓存、厂商）
api-openapi-metrics-description = Prometheus 文本暴露格式的 RED 指标
