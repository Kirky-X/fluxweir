// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Fluxweir i18n 基座：sdforge::i18n 宿主注册表（en/zh）+ 语言检测链。
//!
//! - **检测链**：`FLUXWEIR_LANG` → `LC_ALL` → `LC_MESSAGES` → `LANG`
//!   → sys-locale → `"en"`；`zh*` 归一 `zh`，`C`/`POSIX`/未知语言继续走链。
//! - **消息目录**：`locales/{en,zh}/*.ftl` 编译期内嵌 → 启动期行解析注册
//!   进 `sdforge::i18n` 宿主注册表（janvi/CodeNexus 同款）；查找走
//!   `translate_for`（当前 locale → en 回退 → 键名），占位符 `{ $name }`
//!   由 sdforge 模板替换。locale 决策权留在本仓检测链（显式传参，
//!   不依赖 sdforge active locale）。
//! - **结果域**：仅 `{en, zh}`，一切回退终结 en，默认英文。
//!
//! # 示例
//!
//! ```rust,ignore
//! use fluxweir::i18n;
//!
//! i18n::init();                          // 可选预热（注册 + 单向同步 sdforge active locale）
//! i18n::t("vendor-upstream-stream-interrupted");
//! i18n::t_args("admin-vendor-not-found", &[("id", "openai".into())]);
//! i18n::set_locale("zh-CN").unwrap();    // 覆盖位优先于检测链
//! ```
//!
//! 缺键不 panic：回退链终结于 en 注册表，仍缺则原样返回键名——因此新增用户可见
//! 串必须先登记 en 键（zh 同步补齐），键位对齐与引用存在性由
//! `tests/i18n/i18n_parity_test.rs` 守卫（按调用点字面形态扫描，键必须是字面量；
//! `examples/` 不在扫描域，引入 i18n 时须同步扩域）。
//!
//! # 键化边界（调用点形态约定）
//!
//! 错误载荷（`GateError::Validation.field`、`GateError::Config.message`、
//! `FluxweirError::Database.message` 等）**只承载语言中性锚点**，不得在构造点
//! 直接调用 t / t_args：
//! - **锚点形态**：`"<ftl键>"` 或 `"<ftl键>|<JSON 对象>"`——锚名即已登记的
//!   FTL 键，参数为 `serde_json` 对象（如
//!   `error-validation-model-not-found|{"model":"gpt-4"}`）；`Display` 因此
//!   恒为英文规范串；
//! - **本地化单点**：仅在 `user_message()`（`GateError`/`FluxweirError`）经
//!   `localize_anchor` → `t_dynamic` 完成；动态键绕过 parity 守卫的字面
//!   扫描，键存在性由 `tests/guard/error_payload_guard.rs` 的锚点守卫
//!   （`has_key`）兜底；
//! - **回退**：未登记键/裸字段路径（`"model"`）/原始英文串经通用模板
//!   （`error-validation`/`error-config`/`error-database`）单层包裹——合法
//!   回退，不算违规；
//! - **响应构造点**（无错误变体的直接 HTTP 错误体，如中间件 401/403、SSE
//!   错误帧、web-search 聚合错误体）允许就地 `t()` / `localize_anchor()`——
//!   该处即消息轨道终点，不属于错误载荷；
//! - **范围**：契约约束服务端错误类型（`GateError`/`VaultError`/
//!   `FluxweirError`）。`src/sdk/` 是对外客户端库，`FluxweirSdkError` 的
//!   Display 就是终态用户消息（无双层轨道），构造点本地化是设计语义。
//!
//! 构造点纪律由 `tests/guard/error_payload_guard.rs` 两个 token 级守卫承载
//!（禁止载荷字段/错误 helper 就地 `t()`/`t_args()`；锚点键必须在 en 束登记）。
//! 守卫是绊线而非证明：别名导入、宏中转、变量间接可绕过，审查流程兜底。
//!
//! # 日志面分层
//!
//! `warn`/`info`/`error` 级日志（运维可见）走 t()/t_args()（与 vendor
//! manager 先例一致）；`debug` 级协议调试锚**保持英文**——调试锚靠 grep
//! 固定短语定位，本地化会让诊断关键词随 locale 漂移，收益不抵成本。

pub mod catalog;
pub mod locale;

use thiserror::Error;

/// i18n 模块错误。
#[derive(Debug, Error)]
pub enum I18nError {
    /// 语言串非法或不受支持（结果域仅 `{en, zh}`）。
    #[error("invalid or unsupported locale '{input}' (supported: en, zh)")]
    InvalidLocale {
        /// 无法归一化的原始输入。
        input: String,
    },
}

pub use catalog::{has_key, init, t, t_args, t_dynamic};
pub use locale::{current_locale, detected_locale};

/// 设置全局 locale 覆盖位，并单向同步 sdforge active locale。
///
/// sdforge 的框架自串（HTTP 401/404 等按 active locale 渲染的文案）跟随
/// 本仓语言决策；本仓 [`t`]/[`t_args`] 的查找不依赖该同步（显式传
/// `current_locale`），同步失败语义不受影响（覆盖位已生效）。
pub fn set_locale(locale: &str) -> Result<(), I18nError> {
    locale::set_locale(locale)?;
    sdforge::i18n::set_locale(&locale::current_locale().to_string());
    Ok(())
}

/// 清除覆盖位，回退到检测 locale（sdforge active locale 同步跟随）。
pub fn clear_locale_override() {
    locale::clear_locale_override();
    sdforge::i18n::set_locale(&locale::current_locale().to_string());
}
