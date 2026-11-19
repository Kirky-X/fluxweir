[English](README.md) | 中文

<div align="center">

<img src="docs/assets/fluxweir.png" alt="Fluxweir Logo" width="240">

[![Rust 2021](https://img.shields.io/badge/Rust-2021-edded?logo=rust&style=for-the-badge)](https://www.rust-lang.org/) [![Rustc 1.97.1+](https://img.shields.io/badge/Rustc-1.97.1+-orange.svg?style=for-the-badge)](https://www.rust-lang.org/) [![release](https://img.shields.io/github/v/release/Kirky-X/fluxweir?style=for-the-badge)](https://github.com/Kirky-X/fluxweir/releases) [![CI](https://img.shields.io/github/actions/workflow/status/Kirky-X/fluxweir/ci.yml?branch=main&label=CI&style=for-the-badge)](https://github.com/Kirky-X/fluxweir/actions/workflows/ci.yml) [![coverage](https://img.shields.io/codecov/c/github/Kirky-X/fluxweir?style=for-the-badge)](https://codecov.io/gh/Kirky-X/fluxweir) [![SQLite](https://img.shields.io/badge/SQLite-backend-003B57?logo=sqlite&logoColor=white&style=for-the-badge)](https://sqlite.org/) [![gRPC](https://img.shields.io/badge/gRPC-support-3C8CC9?logo=grpc&style=for-the-badge)](https://grpc.io/) [![Axum](https://img.shields.io/badge/Axum-REST-009688?style=for-the-badge)](https://docs.rs/axum) [![i18n](https://img.shields.io/badge/i18n-zh%20%7C%20en-2CA6E0?style=for-the-badge)](<>) [![Docker](https://img.shields.io/badge/Docker-ready-2496ED?logo=docker&logoColor=white&style=for-the-badge)](https://www.docker.com/) [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)

**智能 LLM 路由网关 —— 多厂商统一接入 · 协议互转 · 弹性调度**

[✨ 功能特性](#-功能特性) • [🚀 快速开始](#-快速开始) • [📚 文档](#-文档) • [🐳 Docker 部署](#-docker-部署) • [🤝 参与贡献](#-参与贡献)

</div>

---

<div align="center">

### 🎯 一个接入点，多家上游

客户端用 OpenAI 或 Anthropic 任一格式请求，网关完成认证、限流、路由、协议转换与容错，把多个上游 LLM 供应商抽象为单一端点：

<table style="width:100%; border-collapse: collapse">
<tr>
<td align="center" width="25%">🔄<br><b>协议互转</b><br><span style="color:#64748B">OpenAI ↔ Anthropic · 零拷贝透传</span></td>
<td align="center" width="25%">🧭<br><b>智能路由</b><br><span style="color:#64748B">6 因子加权（第 6 项 quota 可选）· 缓存经济学亲和 · LinUCB</span></td>
<td align="center" width="25%">🛡️<br><b>弹性容错</b><br><span style="color:#64748B">熔断 · 重试 · Fallback 链</span></td>
<td align="center" width="25%">📊<br><b>可观测与成本</b><br><span style="color:#64748B">请求日志 · Token 预聚合 · 成本追踪</span></td>
</tr>
</table>

</div>

---

## 📋 目录

<details open>
<summary>📑 目录</summary>

- [✨ 功能特性](#-功能特性)
- [🚀 快速开始](#-快速开始)
- [⚙️ 配置概览](#️-配置概览)
- [📚 文档](#-文档)
- [🏗️ 架构](#️-架构)
- [📚 作为库依赖](#-作为库依赖)
- [🧪 测试](#-测试)
- [⚡ 性能设计](#-性能设计)
- [🔒 安全](#-安全)
- [🗺️ 开发路线图](#️-开发路线图)
- [🐳 Docker 部署](#-docker-部署)
- [🤝 参与贡献](#-参与贡献)
- [📋 更新日志](#-更新日志)
- [📄 许可证](#-许可证)
- [🙏 致谢](#-致谢)
- [📞 联系与支持](#-联系与支持)

</details>

---

## ✨ 功能特性

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔄 <b>协议互转</b><br><span style="color:#64748B">OpenAI Chat / Embeddings ↔ Anthropic Messages 双向转换，SSE 流式两端同支持；入站协议与目标厂商一致时零拷贝透传</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🧭 <b>智能路由</b><br><span style="color:#64748B">模型权重 × 队列 × 时间窗口（× 可选延迟 / 错误率 / 配额预算）6 因子加权随机，温度平滑；可选 Power-of-Two-Choices、LinUCB 上下文老虎机、缓存经济学会话亲和（<code>[routing.affinity]</code>：auto 按缓存价值粘 / turn 会话窗口粘 / 落盘重启恢复，legacy 默认零回归）、SLA 虚拟队列与 LPS 排队论前瞻；难度定档（<code>[routing.difficulty]</code>，默认关）：规则计分 + vecboost laya 决策概率融合，按难度切 low/mid/high 三档改写目标模型——简单问题小模型、困难问题大模型</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🛡️ <b>弹性容错</b><br><span style="color:#64748B">limiteron 熔断器（5 次失败开 30s，慢调用 60s 阈值防误判）、按状态码分类重试（429 尊重 Retry-After 展宽、5xx 固定间隔 + 展宽）、priority 驱动的 Fallback 换路链（总尝试预算默认 8）</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🚦 <b>入站限流与准入</b><br><span style="color:#64748B">limiteron tower 层入站限流（端点表 + 全局层）+ <code>[[rate_limit.rules]]</code> 路径规则（未配置全局规则时，共享令牌桶兜底）；拥塞感知优先级准入（按 API Key 档位 dedicated/guaranteed/elastic/spot 分级 shed）</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔐 <b>安全认证与密钥保密柜</b><br><span style="color:#64748B">API Key 仅存 SHA256 摘要、常量时间比较；上游密钥支持保密柜持久化（<code>POST/GET/DELETE /admin/vault/keys</code>，cocoon AEAD 加密入库、响应与日志零明文），首装由 env 引导吸收、之后免二次输入——主钥经 <code>FLUXWEIR_MASTER_KEY</code> 或自动生成文件（<code>~/.fluxweir/vault_master.key</code>，0600）分层提供，<b>主钥丢失不可恢复，请备份</b></span></td>
<td width="50%" style="vertical-align:top; padding: 12px">⚡ <b>响应缓存</b><br><span style="color:#64748B">精确响应缓存（oxcache 存储），租户 + 厂商 + 请求体指纹键，TTL 分级乘数与负载联动，并发去重（single-flight）由 oxcache <code>get_or</code> 承载，<code>X-No-Cache: 1</code> 强制回源</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">📊 <b>请求日志与预聚合</b><br><span style="color:#64748B">异步批量写库（默认 SQLite，db-postgres/db-mysql 构建下写对应后端；满载丢弃计数不阻塞响应），Token 用量按 (厂商, 模型, api_key 摘要, 日期, 小时) 预聚合 UPSERT</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">💰 <b>成本追踪</b><br><span style="color:#64748B"><code>[[model_catalog]]</code> 的四档单价为计价正源（微美元原子累积按厂商/模型分账，热更新立即生效）；<code>[[pricing]]</code> 仅用于路由 SLA 因子与未登记目录模型的配额回退档。另有离线成本回放子命令 <code>fluxweir cost-replay</code>（<code>cost-replay</code> feature，不进 <code>full</code>）：读 <code>request_logs</code> 历史重放 affinity/cache_ttl 场景的 counterfactual 成本，只读不写库（目标库须已存在，不存在则报错）</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔥 <b>配置热更新</b><br><span style="color:#64748B">基于 confers DynamicField 的无锁热读；写路径事务语义（校验 → 原子落盘 → 切换内存态），自动快照备份到 <code>config-backups/</code>（保留 10 份、密钥打码）；<code>server</code>/<code>cors</code>/<code>rate_limit</code>/<code>database</code>/<code>grpc</code>/<code>global_model</code>/<code>cache.semantic</code>/<code>docs.enabled</code>/<code>routing.difficulty.enabled</code> 及 <code>routing.difficulty.classifier</code> 的 <code>model_path</code>/<code>engine</code>/<code>cache_ttl_secs</code> 变更会在响应 <code>restart_required</code> 中显式提示（vendor <code>concurrency</code> 等其余需重启面无此字段，需自行重启）</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🌐 <b>管理端点与可观测</b><br><span style="color:#64748B">52 个方法端点（feature 全开口径：数据面 14 + 搜索/抓取 2 + 管理面 35 + 健康 1，其中 11 个随 feature 门控；另有 <code>/readyz</code>、<code>/metrics</code> 运维出口与 <code>/console</code> 控制台）：日志查询/搜索、Token 统计、成本分账查询、全局指标（含亲和命中率、丢弃计数）、厂商状态与密钥池重置、路由解释、配置与模型管理、订阅/掩码/目录同步（feature 门控）、健康检查</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🗣️ <b>国际化</b><br><span style="color:#64748B">Fluent 双语束（en/zh）覆盖全部业务消息与错误文案，<code>FLUXWEIR_LANG</code> → 系统环境 → 回退 en 检测链</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🧰 <b>生产就绪</b><br><span style="color:#64748B">Docker 多阶段构建 + 非 root 运行、镜像层健康检查（<code>HEALTHCHECK</code>）与 docker-compose 资源限制、GitHub Actions CI（fmt/clippy/test/coverage/audit）与自动发布流水线</span></td>
</tr>
</table>

---

## 🚀 快速开始

### 📦 环境要求

- Rust 1.97.1+（MSRV，由 `Cargo.toml` 的 `rust-version` 声明；edition 2021）
- SQLite：**全构建静态内嵌**——`libsqlite3-sys` 的 bundled 特性以 C 源现场编译并静态链接（`cargo tree` 显示 bundled、产物 `ldd` 无 sqlite 动态依赖；glibc 与 musl 同口径），无需系统 sqlite 库或 `libsqlite3-dev`；TLS 为纯 rustls，不依赖 OpenSSL
- 自研 7 库经 crates.io registry 集成（`confers`、`dbnexus`、`inklog`、`limiteron`、`oxcache`、`sdforge`、`trait-kit`），由 cargo 自动拉取，无需置于同级目录。例外是可选 feature `difficulty-classifier` / `semantic-cache`：二者依赖 path 依赖 `vecboost = { path = "../vecboost" }`（`Cargo.toml`，库面快速演进期跟随本地 checkout），启用时需先把 vecboost 检出到仓库同级目录；默认 `full` 不含这两个 feature，按默认口径构建不受影响

### 🔨 编译运行

<details>
<summary>📄 代码 · bash</summary>

```bash
# 运行时配置为 config.toml——该文件不入库（.gitignore 已列），先 cp config.example.toml config.toml 起步（示例为随仓全量参考；起服前按需配置厂商与 `api_key_env` 指向的环境变量）
cp config.example.toml config.toml

# 默认读取工作目录 config.toml
cargo run --release

# 指定配置文件
cargo run --release -- /path/to/config.toml
```

</details>

### 💡 首次请求

<details>
<summary>📄 代码 · bash</summary>

```bash
# 健康检查（无需认证；实际注册路径带 sdforge 版本前缀）
curl http://127.0.0.1:8080/api/v1/health

# OpenAI 兼容聊天补全
curl -X POST http://127.0.0.1:8080/v1/chat/completions \
  -H "Authorization: Bearer <your-api-key>" \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-4", "messages": [{"role": "user", "content": "Hello"}]}'
```

</details>

> 提示：裸 `/health` 未注册路由（会 404），但已加入认证与限流白名单；Anthropic 消息、模型列表、Token 计数等其余端点的参数、响应与错误码见 [📘 API 参考](docs/cn/API_REFERENCE.md)。

### 🩺 安全姿态体检（`fluxweir doctor`）

`fluxweir doctor` 是静态姿态检查：一条命令报告当前构建的 feature 编译面 + 配置快照——不发起网络请求、不连数据库、不启动服务器。

<details>
<summary>📄 代码 · bash</summary>

```bash
# 全量报告（安全 + 基础项）；--config 缺省 config.toml
cargo run -p fluxweir --features full,db-sqlite -- doctor --config config.example.toml

# 只输出安全项
cargo run -p fluxweir --features full,db-sqlite -- doctor --security --config config.example.toml
```

</details>

退出码：`0` 无 Fail 项 · `1` 存在 Fail 项 · `2` 用法错误或配置装载失败（只走 stderr，不产报告）。

| 检查项 id | 判定 |
|---|---|
| `vault-encryption` | `vault` feature 关 → Fail；开时探测主钥来源——env `FLUXWEIR_MASTER_KEY` 优先，其次主钥文件（`FLUXWEIR_MASTER_KEY_FILE` 或 `~/.fluxweir/vault_master.key`）；双缺 → Fail（容器重建会重新生成主钥，柜内密文全部不可解） |
| `vault-backend` | `database` feature 开 → DbKeyStore；vault 开而 database 关 → Warn（FileKeyStore 回落，SDK 嵌入形态）；vault 关 → 跳过 |
| `quota-reserve` | `[key_budgets].rules` 非空 → Pass（N 条规则）；空/缺段 → Warn（配额预留整体跳过） |
| `quota-fail-open` | `fail_open=true` 的规则逐条 Warn（key 摘要前 8 字符，预算源不可用时请求放行）；全 fail-closed → 单条 Pass |
| `auth-http` | `[auth].enabled` → Pass（SHA256 摘要）；关 → Fail（网关接受未认证请求） |
| `auth-grpc` | `grpc` feature 未编入 → Warn；编入且认证开 → Pass；编入而认证关 → Fail |
| `masking` | feature `masking` + `[masking].enabled` 双开 → Pass；其余 → Warn（合规敏感，默认不进产物） |
| `logging` | 仅全量模式：`logging` feature → Pass，否则 Warn |

示例输出（全量模式，`full,db-sqlite` 构建，主钥来自 env——`--security` 不含最后一行）：

```
✓ vault-encryption: master key: env
✓ vault-backend: DbKeyStore
! quota-reserve: quota reservation skipped (no [key_budgets].rules)
✓ quota-fail-open: all rules fail-closed
✓ auth-http: SHA256 digest
! auth-grpc: not compiled in (feature grpc)
! masking: disabled — compliance-sensitive feature ships off (needs feature masking + [masking].enabled)
✓ logging: compiled in (feature logging)
```

doctor 是静态检查：反映本次构建的 feature 编译面与这份配置快照，不证明运行期行为（如主钥确实可解密既有密文）。doctor 子命令在 `full` 构建内恒可用（bin 的 `required-features`）；库形态自选 feature 组合的用户按上表自查。

### 🧭 路由预演（`fluxweir route-explain`）

`fluxweir route-explain` 转发到运行中网关的 `POST /admin/route/explain`，把每个候选打印为对齐文本表。它不做本地路由计算——网关必须已启动。

| 参数 | 含义 |
|---|---|
| `--model <id>` | 必填。要解释的模型 id。 |
| `--gateway <url>` | 网关基址；缺省 `http://127.0.0.1:8080`。 |
| `--admin-key <token>` | 可选 `x-api-key` 头值（admin 认证）。 |
| `--session-id <id>` | 可选会话 id，用于带亲和的预演。 |

```bash
cargo run -p fluxweir --features full,db-sqlite -- route-explain \
  --model gpt-4o --gateway http://127.0.0.1:8080 --admin-key "$ADMIN_KEY"
```

示例输出：

```
VENDOR               DECISION   REASON_CODE              WEIGHT
vendor-a             eligible   ok                       1.5
    factors total: 3.5
    selection: primary
vendor-b             excluded   no_budget                0
```

退出码：`0` 成功 · `1` 网关不可达 / HTTP 错误（stderr 提示"start the gateway or pass --gateway"）· `2` 用法错误。

### 📡 审计事件三通道

安全审计事件（vault reveal、quota fail-open、quota attribution）以 target=`fluxweir::audit` 发出，抵达三个通道：

- **内存环形缓冲**——由 admin 端点 `GET /admin/quota/failopen` 与 `GET /admin/quota/attribution` 读取（有界、最新在前、零明文密钥）。
- **Tracing → 日志管道**——结构化事件经 `tracing-log` 桥并入统一 inklog 总落库（`logging` feature）。
- **stdout 渲染**——CLI/doctor 输出以文本呈现姿态与诊断。

`tests/audit_bus` 钉住 tracing→总线路径：发出一条 reveal 与一条 fail-open 事件后，可捕获两条 `fluxweir::audit` 事件，且 `kind` 与 key 字段完整。

---

## ⚙️ 配置概览

运行时配置为单个 `config.toml`（**不入库**，干净检出先 `cp config.example.toml config.toml` 起步——随仓示例为每段每字段带默认值与热更新可用性标注的全量参考）；30 个顶层段总览、各段字段明细、热更新矩阵与厂商/计价配置示例见 [📖 用户指南](docs/cn/USER_GUIDE.md)。

---

## 📚 文档

<details>
<summary>📋 表格</summary>

| 文档                                 | 说明                                                                   |
| ------------------------------------ | ---------------------------------------------------------------------- |
| [📖 用户指南](docs/cn/USER_GUIDE.md)    | 从配置、部署到运维的完整使用教程                                       |
| [📘 API 参考](docs/cn/API_REFERENCE.md) | 全部方法端点（数据面 14 + 搜索/抓取 2 + 管理面 35 + 健康 1，feature 全开口径共 52 个）的参数、响应与错误码 |
| [🏗️ 架构文档](docs/cn/ARCHITECTURE.md)  | 设计原则、路由决策体系、弹性与热更新机制                               |
| [🔒 安全文档](docs/cn/SECURITY.md)      | 安全设计、供应链门禁与漏洞处理流程                                     |
| [❓ FAQ](docs/cn/FAQ.md)                | 常见问题与已知边界                                                     |
| [📋 更新日志](docs/cn/CHANGELOG.md)     | 版本变更记录（Keep a Changelog 格式）                                  |
| [🤝 贡献指南](docs/cn/CONTRIBUTING.md)  | 环境搭建、开发命令与提交规范                                           |
| [⚡ 性能基线](docs/cn/PERFORMANCE.md)   | Criterion 组件级基准数据（转换 / SSE / 路由 / 亲和）                   |
| [🧭 Prompt 前缀缓存指南](docs/cn/PROMPT_CACHING_GUIDE.md) | 最大化厂商端前缀缓存命中的调用方规范                   |
| [🤝 行为准则](docs/cn/CODE_OF_CONDUCT.md) | 社区参与准则（Contributor Covenant 2.1）                |

</details>

---

## 🏗️ 架构

分层管线（中间件 → API → 路由决策 → 协议转换 → 厂商执行 → 异步落库）、路由决策体系、弹性状态机、配置热更新事务语义、DI 组装与 i18n 架构的完整说明见 [🏗️ 架构文档](docs/cn/ARCHITECTURE.md)。

---

## 📚 作为库依赖

`fluxweir` 同时是可执行网关与 Rust 库（默认 `full` feature 已含服务器组件，客户端 SDK 由 `sdk` feature 提供）：嵌入式起服、Rust SDK 使用示例与数据库后端选择见 [📖 用户指南 · 作为库依赖](docs/cn/USER_GUIDE.md#-作为库依赖)。

---

## 🧪 测试

测试策略、与 CI 一致的运行命令与最新测试基线见 [🤝 贡献指南 · 测试约定](docs/cn/CONTRIBUTING.md#-测试约定)。

---

## ⚡ 性能设计

同协议零拷贝透传、confers DynamicField 无锁热读、日志与预聚合全异步等可复核设计要点见 [🏗️ 架构文档](docs/cn/ARCHITECTURE.md)；Criterion 组件级基线数据（转换 / SSE 帧解析 / 路由决策路径 / 缓存经济学亲和）见 [⚡ 性能基线](docs/cn/PERFORMANCE.md)。

---

## 🔒 安全

### 🛡️ 安全设计

API Key 仅以 SHA256 摘要存储并常量时间比较、上游密钥只存环境变量名、错误响应仅暴露固定 `error.type` 代码、配置快照自动打码密钥、CORS 与入站限流（429）分层防护、Docker 非 root 运行。逐项机制见 [🔒 安全文档](docs/cn/SECURITY.md)。

### ⚠️ 订阅上游合规警示（subscription-vendor）

以 OAuth 订阅账号作服务端出站上游可能违反厂商服务条款；启用前必读的合规边界、单实例约束与观测面说明见 [🔒 安全文档 · 订阅上游合规警示](docs/cn/SECURITY.md#️-订阅上游合规警示subscription-vendor)。

### 🚨 报告安全漏洞

请勿通过公开 issue 报告安全漏洞。请使用 GitHub [Security Advisories](https://github.com/Kirky-X/fluxweir/security/policy) 私密披露通道提交报告，项目承诺 48 小时内确认、7 天内给出初步评估；完整流程见 [🔒 安全文档 · 漏洞报告流程](docs/cn/SECURITY.md#-漏洞报告流程)。

---

## 🗺️ 开发路线图

各方向已完成条目与交付口径见 [📋 更新日志 · 开发路线图](docs/cn/CHANGELOG.md#️-开发路线图)。

---

## 🐳 Docker 部署

镜像构建（含 vecboost 附加上下文前置）、容器运行（含单文件挂载 EBUSY 警示）与 docker-compose 部署见 [📖 用户指南 · Docker 部署](docs/cn/USER_GUIDE.md#-docker-部署)。

---

## 🤝 参与贡献

详细的贡献流程、开发命令与提交规范请参阅 [🤝 贡献指南](docs/cn/CONTRIBUTING.md)。

---

## 📋 更新日志

完整版本历史见 [📋 更新日志](docs/cn/CHANGELOG.md)（遵循 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) 格式，语义化版本）。

---

## 📄 许可证

本项目采用 [MIT License](LICENSE)。

---

## 🙏 致谢

核心依赖（7 个自研兄弟库）清单见 [🤝 贡献指南](docs/cn/CONTRIBUTING.md)；感谢 Rust 社区与所有[贡献者](https://github.com/Kirky-X/fluxweir/graphs/contributors)。

---

## 📞 联系与支持

报告问题和 Bug 见 [Issues](https://github.com/Kirky-X/fluxweir/issues)，提问和分享想法见 [讨论区](https://github.com/Kirky-X/fluxweir/discussions)，查看源代码见 [GitHub](https://github.com/Kirky-X/fluxweir)。

---

<sub>© 2026 Kirky.X. 保留所有权利。</sub>
