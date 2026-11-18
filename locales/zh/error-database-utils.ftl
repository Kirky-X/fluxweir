# zh/error-database-utils: src/error.rs + src/{database,utils} 用户可见消息。
# error-*: FluxweirError 变体本地化消息（IntoResponse 展示轨道，Display 恒英文规范串）。
error-config = 配置错误：{ $message }
error-io = I/O 错误：{ $message }
error-config-storage-write = 存储写入失败：{ $error }
error-protocol = 协议转换错误：{ $source }
error-vendor-unavailable = 厂商不可用：{ $vendor }
error-timeout = 请求超时：{ $vendor }
error-rate-limited = 限流：{ $vendor }
error-database = 数据库错误：{ $message }
error-validation = 请求验证失败：{ $field }
error-routing = 路由错误：{ $message }
error-all-vendors-throttled = 全部候选厂商被限流：预计 { $retry_after_secs }s 后可重试
# error-* -source: From 转换时写入 Database 变体 message 字段的前缀。
# database-*: database.rs 初始化路径与 async_logger/pre_aggregator 运维消息。
database-connect = 连接数据库失败：{ $error }
database-session = 获取数据库会话失败：{ $error }
database-init-schema = 初始化 schema 失败：{ $error }
database-logger-shutdown = 日志写入器已关闭
database-logger-channel-closed = 异步日志写入器通道已关闭
database-logger-flush-failed = 写入请求日志失败（共 { $count } 条）：{ $error }
database-logger-secret-scan-dropped = 日志未通过出站密钥扫描已丢弃（累计 { $total } 条）：{ $error }
database-logger-console-secret-gate-dropped = console 日志未通过出站密钥门禁，已丢弃（不打印）
database-logger-join-failed = 等待异步日志写入器退出失败：{ $error }
database-preagg-flush-failed = token 聚合刷新失败：{ $error }
database-preagg-final-flush-failed = 停机前最终 token 聚合刷新失败：{ $error }
database-cost-usage-query-failed = 成本分账查询失败：{ $error }
# utils-*: metrics.rs 运维告警。
utils-metrics-concentration = 流量集中度告警：路由可能已收敛到单一厂商/模型
database-backend-missing = database.url 为空，path { $path } 的 sqlite 回退需要 db-sqlite feature：以 --features "full,db-sqlite" 重建，或在 [database] url 配置完整连接 URL
# repository.rs 缺列补列（V0.1 旧库迁移）。
database-repo-column-migrated = 迁移：已为表 { $table } 补齐缺失列 { $column }

# storage.rs 契约默认实现：数据面写前沿不承担 admin 查询，显式拒绝。
storage-log-query-unsupported = 此 RequestLogSink 实现为数据面写入前沿，不支持查询（admin 读取经存储后端实现）
storage-usage-stats-unsupported = 此 UsageSink 实现不提供统计查询（admin 读取经存储后端实现）

# vault (key-vault)：保密柜用户可见消息（Display 轨恒英文规范串，此处为展示轨）。
error-vault = 保密柜错误：{ $message }
error-vault-not-found = 保密柜密钥不存在：{ $id }
error-vault-unknown-vendor = 未知厂商：{ $vendor }
# vault 载荷键：经 error-vault = 保密柜错误：{ $message } 插值展示，展示轨本地化。
# file_store.rs
vault-home-not-found = 未找到主目录
vault-read-file-failed = 读取保密柜文件失败：{ $error }
vault-file-magic-mismatch = 保密柜文件魔数不匹配
vault-decode-entries-failed = 解析保密柜条目失败：{ $error }
vault-encode-entries-failed = 序列化保密柜条目失败：{ $error }
vault-seal-file-failed = 密封保密柜文件失败：{ $error }
vault-create-dir-failed = 创建保密柜目录失败：{ $error }
vault-create-tmp-failed = 创建保密柜临时文件失败：{ $error }
vault-write-file-failed = 写入保密柜文件失败：{ $error }
vault-sync-file-failed = 同步保密柜文件失败：{ $error }
vault-replace-file-failed = 替换保密柜文件失败：{ $error }
# crypto.rs
vault-seal-blob-failed = 密封失败：{ $error }
vault-open-blob-failed = 解密失败：{ $error } —— 当前主钥无法解密该条目（存入后 FLUXWEIR_MASTER_KEY / 主钥文件已变更，或 vault 柜格式 / KDF 域分隔常量变更；主钥变更后条目不可恢复，请重新录入厂商密钥）
vault-decrypted-not-utf8 = 解密结果不是 UTF-8：{ $error }
# db_store.rs
vault-db-access-failed = 访问保密柜数据库失败：{ $error }
# master_key.rs
vault-master-key-hex-len = { $env } 必须恰好为 { $expected } 个十六进制字符（32 字节），实际 { $actual } 个
vault-master-key-hex-decode-failed = 十六进制解码失败：{ $error }
vault-master-key-create-dir-failed = 创建密钥目录失败：{ $error }
vault-master-key-write-failed = 写入密钥文件失败：{ $error }
vault-master-key-restrict-failed = 收紧密钥文件权限失败：{ $error }
storage-affinity-sticks-unsupported = 该 AffinityStickSink 实现是数据面写入前沿，不支持读取（启动恢复走存储后端实现）
database-affinity-persist-flush-failed = 会话粘性批落库失败（丢弃 { $count } 条）：{ $error }
database-affinity-persist-channel-closed = 会话粘性落盘通道已关闭：{ $error }
database-affinity-sticks-read-failed = 会话粘性读取失败：{ $error }

# ---- masking: 出站掩码（feature masking；错误面 + config 校验面）----
masking-unavailable = 请求被拒绝：出站掩码不可用（fail-closed，绝不原文出站）
masking-blocked = 请求被拦截：{ $kind } 类敏感数据禁止出站
masking-pepper-degraded = 掩码 pepper 加载降级为进程内随机（{ $stage }：{ $detail }），占位符跨进程/重启确定性不成立
masking-ledger-io-failed = 掩码台账 IO 失败（{ $op }：{ $detail }），观测旁路降级不影响请求

# ---- cost-replay: 离线成本回放子命令（fluxweir cost-replay；用法/缺参/未知参数错误由 clap 接管，不再登记键）----
replay-cli-about = 离线成本回放：读 request_logs 历史重放反事实成本（只读；不开服务、不写库）
replay-cli-bad-time = 时间取值非法：{ $value }（需 unix 秒或 RFC3339）
replay-cli-bad-scenario = 场景取值非法：{ $value }（需 affinity|cache_ttl|all）
replay-cli-bad-max-rows = --max-rows 取值非法：{ $value }（需正整数）
replay-cli-bad-window = --to 必须大于或等于 --from
replay-cli-db-not-found = 回放数据库文件不存在：{ $path }（工具不建库，请核对 --db 路径）
replay-cli-config-load-failed = 加载 model_catalog 单价配置失败：{ $error }
replay-cli-db-open-failed = 打开回放数据库失败：{ $error }
replay-cli-load-failed = 加载 request_logs 回放数据失败：{ $error }

# ---- cost-replay: 回放引擎（库级错误）----
replay-window-too-large = 回放窗口有 { $rows } 行，超过 --max-rows 上限 { $limit }；请收窄 --from/--to 或提高 --max-rows
replay-row-decode-failed = 解码 request_logs 列 { $column } 失败：{ $error }
vault-state-unrecoverable = 保密柜文件与备份均不可用：{ $reason } —— 拒绝以空柜启动（数据保全优先），请从 .bad-* 隔离文件或外部备份排查恢复

util-fetch-body-not-utf8 = 响应正文不是合法 UTF-8

error-vendor-circuit-open = 厂商「{ $vendor }」不可用：熔断器已打开

# SDK 进程内库面（FluxweirSdkError 的 message；面向集成方，文案随进程 locale）
sdk-url-invalid = URL 非法: { $error }
sdk-base-url-required = base_url 必填
sdk-api-key-required = api_key 必填
sdk-missing-choices = 响应缺少 choices 数组
sdk-empty-choices = 响应 choices 数组为空
sdk-invalid-tool-call = tool_call 非法: { $error }
sdk-transport-not-cloneable = 请求 builder 不可克隆（流式请求体？）
sdk-admin-action-failed = 管理操作失败: { $detail }
sdk-json-parse = JSON 解析错误: { $error }
sdk-sse-decode = SSE 解码错误: { $error }
sdk-input-tokens-model-required = input_tokens 需要非空 `model`
sdk-input-tokens-input-required = input_tokens 需要非空 `input`
sdk-extension-key-conflict = { $source } 键 `{ $key }` 与类型化请求字段冲突
sdk-invalid-json-response = 响应体 JSON 非法
sdk-base-url-not-base = base_url 不能作为基址
sdk-stream-error = 流错误
sdk-error-network = 网络错误：{ $error }
sdk-error-timeout = 请求超时：{ $error }
sdk-error-api = API 错误（状态 { $status }）：{ $message }
sdk-error-protocol = 协议错误：{ $message }
sdk-error-stream = 流错误：{ $message }
sdk-error-config = 配置错误：{ $message }
console-assets-missing = 控制台静态资源缺失：请在 web/console 运行 `npm run build`（或在 npm 可用时重新构建）

# ---- router/state/stream/grpc: warn/info 级运维日志（走 t()/t_args()）----
logs-affinity-non-utf8 = 亲和值不是合法 UTF-8，按未命中处理
logs-state-recovered-from-bak = 状态文件已从 .bak 恢复，主文件已隔离
logs-state-primary-missing-recovered = 主文件缺失，已从 .bak 恢复
logs-state-bak-write-failed = 状态 .bak 写入失败
logs-stream-usage-update-failed = 流式用量日志更新失败
logs-grpc-non-loopback = gRPC 监听器绑定到非回环地址，请确认网络暴露是有意为之

# ---- cost-replay: CLI 报表（Markdown/JSON 双渲染）----
replay-report-title = # 成本回放报告
replay-report-window = 时间窗：{ $from }..{ $to }（{ $requests } 个请求）
replay-report-pricing-source = 计价来源：{ $source }
replay-report-summary-header = | 场景 | 总额 (USD) | 相对基线差异 (USD) | 会话数 | 请求数 | 缓存命中 | 命中率 | 未计价 |
replay-report-vendor-header = | 场景 | 厂商 | USD |
replay-report-model-header = | 场景 | 模型 | USD |
replay-report-by-vendor = ## 按厂商
replay-report-by-model = ## 按模型
replay-report-assumptions = ## 假设
replay-report-baseline-recomputed = 基线口径：{ $rows } 行没有可用的日志成本，已按当前目录单价重算。
replay-report-uncataloged = { $rows } 行引用的模型不在 [[model_catalog]] 中，已回退到日志成本。
replay-assumption-1 = session 为 NULL 的行没有缓存收益（每行自成一个会话）。
replay-assumption-2 = 上游缓存行为以 TTL 窗口近似（TTL 内同会话 + 同厂商视为命中）。
replay-assumption-3 = 单价取当前 [[model_catalog]]，历史单价不可还原。
replay-assumption-4 = 缓存命中时 prompt token 按 cache_read_price 计价，该行 cache_write_tokens 再按 cache_creation_price 计费；未命中时 prompt token 仅按 input_price 计价（不收 cache_write 费用）。
replay-assumption-5 = 日志中的 cache_read_tokens 列不参与计价，反事实结果仅由目录单价驱动。
