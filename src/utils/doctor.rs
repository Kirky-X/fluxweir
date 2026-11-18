// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 安全姿态体检引擎（security-doctor-cli）：静态检查编译面（feature 组合）
//! 与配置快照（[auth]/[key_budgets]/[masking] 等段），逐项输出三态报告。
//!
//! 边界（proposal Non-Goals）：不发起网络请求、不连数据库、不证明运行期
//! 行为（如主钥确实可解密既有密文）——只反映"这次构建 + 这份配置"的静态
//! 姿态；运行期探活属 readiness 面（src/api/readiness.rs）。
//!
//! 模块不设 feature 门（bin required-features = full，doctor 恒在 full
//! 构建内可用）；检查项内部用 `cfg!` 宏如实报告编译面。

/// 单项检查结论：Pass 通过 / Fail 失败（计入非零退出码）/ Warn 警示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
}

/// 单项检查结果：id 为稳定机器码（vault-encryption / quota-reserve / …），
/// detail 为英文规范串（对齐错误双轨惯例），含定位线索（段名/feature 名）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckItem {
    pub id: &'static str,
    pub status: CheckStatus,
    pub detail: String,
}

/// 渲染标记：✓ Pass / ! Warn / ✗ Fail。
fn glyph(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Pass => "✓",
        CheckStatus::Warn => "!",
        CheckStatus::Fail => "✗",
    }
}

/// 逐行渲染检查报告（stdout 口径）：`{glyph} {id}: {detail}`，行尾统一 \n。
pub fn render(items: &[CheckItem]) -> String {
    items
        .iter()
        .map(|item| format!("{} {}: {}\n", glyph(item.status), item.id, item.detail))
        .collect()
}

/// 聚合退出码：存在 Fail → 1，否则 0（Warn 不计入；与 cost-replay 0/1/2
/// 惯例一致，2 留给用法/装载错误，见 run_doctor）。
pub fn exit_code(items: &[CheckItem]) -> i32 {
    if items.iter().any(|item| item.status == CheckStatus::Fail) {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &'static str, status: CheckStatus, detail: &str) -> CheckItem {
        CheckItem {
            id,
            status,
            detail: detail.to_string(),
        }
    }

    /// 三态渲染：标记字符、id、detail 按固定口径逐行输出。
    #[test]
    fn render_marks_three_statuses() {
        let items = [
            item("vault-encryption", CheckStatus::Pass, "master key: env"),
            item(
                "quota-fail-open",
                CheckStatus::Warn,
                "requests pass when budget source unavailable",
            ),
            item(
                "auth-grpc",
                CheckStatus::Fail,
                "not compiled in (feature grpc)",
            ),
        ];
        assert_eq!(
            render(&items),
            "✓ vault-encryption: master key: env\n\
             ! quota-fail-open: requests pass when budget source unavailable\n\
             ✗ auth-grpc: not compiled in (feature grpc)\n"
        );
    }

    /// 空项集渲染为空串（零行为）。
    #[test]
    fn render_empty_items_is_empty() {
        assert_eq!(render(&[]), "");
    }

    /// 退出码聚合：无 Fail（含全 Warn）→ 0；任一 Fail → 1。
    #[test]
    fn exit_code_aggregates_fail_only() {
        assert_eq!(exit_code(&[]), 0);
        assert_eq!(exit_code(&[item("a", CheckStatus::Pass, "ok")]), 0);
        assert_eq!(
            exit_code(&[
                item("a", CheckStatus::Pass, "ok"),
                item("b", CheckStatus::Warn, "careful"),
            ]),
            0
        );
        assert_eq!(
            exit_code(&[
                item("a", CheckStatus::Warn, "careful"),
                item("b", CheckStatus::Fail, "broken"),
                item("c", CheckStatus::Pass, "ok"),
            ]),
            1
        );
    }
}
