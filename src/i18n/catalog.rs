// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 消息目录：`locales/{en,zh}/*.ftl` 经 `include_str!` 编译期内嵌，
//! 启动期行解析逐键注册进 `sdforge::i18n` 宿主注册表
//! （`register_translation`，janvi/CodeNexus 同款模式）；查找与占位符
//! 替换委托 sdforge（`translate_for`：当前 locale → en 回退 → 键名，
//! `{ $name }` 轻量替换，绝不 panic）。
//!
//! 内嵌保证部署产物无需携带磁盘 `locales/` 目录；新增 `.ftl` 文件必须
//! 登记进 `EMBEDDED_LOCALES`，否则守卫测试
//! `test_embedded_locales_cover_locales_dir` 失败。
//!
//! 行解析口径（与完整 Fluent 解析等价的前提，由守卫钉死）：
//! 本仓 FTL 为纯单行 `key = value` 形态——无 selector、无 term、无属性、
//! 无多行延续（`tests/i18n/i18n_parity_test.rs` 以 fluent-syntax 严格解析
//! 零 Junk 并逐键断言 en/zh 占位符集合一致，持续拦截非平凡语法混入）。
//!
//! locale 决策不依赖 sdforge 的 active locale：本模块每次查找显式传
//! [`super::locale::current_locale`]（FLUXWEIR_LANG 检测链 + 覆盖位），
//! 保证既有调用方语义与测试口径不变；sdforge active locale 仅在
//! [`init`]/`set_locale` 时做单向同步（框架自串如 HTTP 401/404 文案跟随）。

use std::collections::BTreeSet;
use std::sync::OnceLock;

use super::locale::current_locale;

/// 编译期内嵌的 locales 目录镜像：语言目录名 → [(文件名, 内容)]。
/// 文件按迁移桶组织：main → main.rs；api → src/api；vendor → src/vendor；
/// config-protocol-router-middleware → src/{config,protocol,router,middleware}；
/// error-database-utils → src/error.rs + src/{database,utils}。
const EMBEDDED_LOCALES: &[(&str, &[(&str, &str)])] = &[
    (
        "en",
        &[
            ("main.ftl", include_str!("../../locales/en/main.ftl")),
            ("api.ftl", include_str!("../../locales/en/api.ftl")),
            ("vendor.ftl", include_str!("../../locales/en/vendor.ftl")),
            (
                "config-protocol-router-middleware.ftl",
                include_str!("../../locales/en/config-protocol-router-middleware.ftl"),
            ),
            (
                "error-database-utils.ftl",
                include_str!("../../locales/en/error-database-utils.ftl"),
            ),
        ],
    ),
    (
        "zh",
        &[
            ("main.ftl", include_str!("../../locales/zh/main.ftl")),
            ("api.ftl", include_str!("../../locales/zh/api.ftl")),
            ("vendor.ftl", include_str!("../../locales/zh/vendor.ftl")),
            (
                "config-protocol-router-middleware.ftl",
                include_str!("../../locales/zh/config-protocol-router-middleware.ftl"),
            ),
            (
                "error-database-utils.ftl",
                include_str!("../../locales/zh/error-database-utils.ftl"),
            ),
        ],
    ),
];

/// 解析单源的 `key = value` 行 → `(key, value)`（值两端 trim）。
///
/// 跳过空行与 `#` 注释；无 `=` 或键为空的行丢弃。目录经核对无多行延续
/// 值（见模块文档），行解析与完整 Fluent 解析行为一致。
fn parse_entries(source: &'static str) -> Vec<(&'static str, &'static str)> {
    source
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') || !trimmed.contains('=') {
                return None;
            }
            let (raw_key, raw_value) = trimmed.split_once('=')?;
            let key = raw_key.trim();
            if key.is_empty() {
                return None;
            }
            Some((key, raw_value.trim()))
        })
        .collect()
}

/// 指定 locale（`en`/`zh`，未知值落 en）的全部目录条目（重复键后者覆盖）。
pub(crate) fn entries_for(locale: &str) -> Vec<(&'static str, &'static str)> {
    let files = EMBEDDED_LOCALES
        .iter()
        .find(|(lang, _)| *lang == locale)
        .map(|(_, files)| *files)
        .unwrap_or_else(|| {
            EMBEDDED_LOCALES
                .iter()
                .find(|(lang, _)| *lang == "en")
                .map(|(_, files)| *files)
                .expect("'en' must be registered in EMBEDDED_LOCALES")
        });
    files
        .iter()
        .flat_map(|(_, source)| parse_entries(source))
        .collect()
}

/// 指定 locale 的键集合（守卫与存在性断言用）。
pub(crate) fn keys_for(locale: &str) -> BTreeSet<String> {
    entries_for(locale)
        .into_iter()
        .map(|(key, _)| key.to_string())
        .collect()
}

/// 注册单次执行锁：进程内首次任一翻译调用或 [`init`] 触发全量注册。
static REGISTERED: OnceLock<()> = OnceLock::new();

/// 把内嵌目录两语言的条目逐键注册进 sdforge 宿主注册表，单次执行。
///
/// 幂等：重复调用无副作用（[`OnceLock`] 门）。sdforge 注册表面向进程全局，
/// 测试进程与其他单测共享同一注册表——不可注册测试专用键。
pub(crate) fn ensure_registered() {
    REGISTERED.get_or_init(|| {
        for (locale, files) in EMBEDDED_LOCALES {
            for (_file_name, source) in *files {
                for (key, value) in parse_entries(source) {
                    sdforge::i18n::register_translation(locale, key, value);
                }
            }
        }
    });
}

/// 翻译：当前 locale（检测链 + 覆盖位）→ en 回退 → 键名本身。
///
/// 显式传 locale 给 [`sdforge::i18n::translate_for`]，不经 sdforge 的
/// active locale——FLUXWEIR_LANG 优先级与覆盖位语义由本仓
/// [`super::locale`] 独占决定。
fn translate(key: &str, args: &[(&str, String)]) -> String {
    ensure_registered();
    sdforge::i18n::translate_for(current_locale().language.as_str(), key, args)
}

/// 翻译无参消息。缺键返回键名本身，不 panic。
///
/// 键必须传字面量：键位守卫按调用点字面形态扫描源码，动态键会静默漏检。
pub fn t(key: &str) -> String {
    translate(key, &[])
}

/// 翻译带 `{ $name }` 占位符的消息。缺键返回键名本身，不 panic。
///
/// 实参名必须与 FTL 占位符逐字一致（en/zh 占位符对齐由守卫测试断言）；
/// 键必须传字面量，理由同 [`t`]。缺实参的占位符按 sdforge 模板替换
/// 语义原样保留 `{ $name }` 文本（不报错回退）。
pub fn t_args(key: &str, args: &[(&str, String)]) -> String {
    translate(key, args)
}

/// 动态键本地化 — 仅供错误载荷锚点路由（键名来自语言中性锚点，
/// 非字面调用点）。锚点键的存在性由 `error::app` 的契约守卫静态校验。
pub fn t_dynamic(key: &str, args: &[(&str, String)]) -> String {
    translate(key, args)
}

/// 键是否已登记（锚点路由判定用：未登记 → 通用模板回退）。
///
/// 以 **en 目录**为准：`translate` 的回退链终点是 en，仅 zh 侧登记的键在
/// en 客户端会原样吐出键名（等于泄漏键名），因此按未登记处理。
/// 纯静态行解析判定，无需触发注册。
pub fn has_key(key: &str) -> bool {
    keys_for("en").contains(key)
}

/// 预热：全量注册进 sdforge 注册表 + 把检测链结果单向同步为 sdforge
/// active locale（框架自串如 HTTP 401/404 文案跟随本仓语言决策）。
/// 可选调用——[`t`]/[`t_args`] 首次调用本就会完成注册。
pub fn init() {
    ensure_registered();
    let _ = super::locale::detected_locale();
    sdforge::i18n::set_locale(&current_locale().to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_en_zh_key_sets_identical() {
        assert_eq!(EMBEDDED_LOCALES.len(), 2, "仅支持 en/zh 双语言");
        let (en, zh) = (&EMBEDDED_LOCALES[0], &EMBEDDED_LOCALES[1]);
        assert_eq!(en.0, "en");
        assert_eq!(zh.0, "zh");
        assert_eq!(en.1.len(), zh.1.len(), "en/zh 文件对数量必须一致");
        for ((en_name, _), (zh_name, _)) in en.1.iter().zip(zh.1.iter()) {
            assert_eq!(en_name, zh_name, "en/zh 文件必须同名成对");
        }
        let en_keys = keys_for("en");
        let zh_keys = keys_for("zh");
        assert!(!en_keys.is_empty(), "en 目录不能为空");
        assert_eq!(en_keys, zh_keys, "en/zh 键集合不一致");
    }

    /// 内嵌清单与磁盘目录双向一致：新增 .ftl 漏登记 EMBEDDED_LOCALES 时
    /// 文件不会被打进二进制，运行时对该文件整族键缺键——在此钉死。
    #[test]
    fn embedded_manifest_matches_on_disk() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales/en");
        let on_disk: std::collections::BTreeSet<String> = std::fs::read_dir(&dir)
            .expect("locales/en 目录必须存在")
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "ftl"))
            .filter_map(|e| {
                e.path()
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
            })
            .collect();
        let embedded: std::collections::BTreeSet<String> = EMBEDDED_LOCALES
            .iter()
            .flat_map(|(_, files)| {
                files
                    .iter()
                    .map(|(name, _)| name.strip_suffix(".ftl").unwrap_or(name).to_string())
            })
            .collect();
        assert_eq!(
            embedded, on_disk,
            "EMBEDDED_LOCALES 与 locales/en/*.ftl 失配：新增目录文件必须同步嵌入"
        );
    }

    #[test]
    fn test_no_duplicate_keys_within_locale() {
        for (locale, files) in EMBEDDED_LOCALES {
            let mut seen = std::collections::HashSet::new();
            for (file_name, source) in *files {
                for key in parse_entries(source).into_iter().map(|(k, _)| k) {
                    assert!(
                        seen.insert(key),
                        "{locale}/{file_name}: 键 '{key}' 重复定义"
                    );
                }
            }
        }
    }

    #[test]
    fn test_embedded_locales_cover_locales_dir() {
        // 编译期登记守卫：locales/ 下每个语言目录的每个 .ftl 都必须
        // 出现在 EMBEDDED_LOCALES，否则新文件会在产物中静默缺译
        let locales_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
        let mut disk_dirs: Vec<String> = std::fs::read_dir(&locales_dir)
            .expect("locales dir exists")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        disk_dirs.sort();
        let mut registered_dirs: Vec<&str> =
            EMBEDDED_LOCALES.iter().map(|(locale, _)| *locale).collect();
        registered_dirs.sort();
        assert_eq!(
            disk_dirs, registered_dirs,
            "EMBEDDED_LOCALES 必须覆盖 locales/ 全部语言目录"
        );

        for (locale, files) in EMBEDDED_LOCALES {
            let dir = locales_dir.join(locale);
            let mut disk_stems: Vec<String> = std::fs::read_dir(&dir)
                .unwrap_or_else(|e| panic!("locale dir '{locale}' readable: {e}"))
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("ftl"))
                .filter_map(|entry| {
                    entry
                        .path()
                        .file_stem()
                        .and_then(|s| s.to_str().map(str::to_string))
                })
                .collect();
            disk_stems.sort();
            let mut registered_stems: Vec<&str> = files
                .iter()
                .map(|(name, _)| name.strip_suffix(".ftl").unwrap())
                .collect();
            registered_stems.sort();
            assert_eq!(
                disk_stems, registered_stems,
                "locale '{locale}': 磁盘 .ftl 与 EMBEDDED_LOCALES 登记不一致"
            );
        }
    }

    #[test]
    fn test_unknown_language_falls_back_to_en_catalog() {
        ensure_registered();
        // sdforge translate_for：未知 locale → en 注册表回退（不建第三目录）
        let en_value = entries_for("en")
            .into_iter()
            .find(|(key, _)| *key == "main-config-loaded")
            .map(|(_, v)| v)
            .expect("'main-config-loaded' must exist in the en catalog");
        assert_eq!(
            sdforge::i18n::translate_for("ar", "main-config-loaded", &[]),
            en_value,
            "未知语言必须回退 en 目录"
        );
        let zh_value = entries_for("zh")
            .into_iter()
            .find(|(key, _)| *key == "main-config-loaded")
            .map(|(_, v)| v)
            .expect("'main-config-loaded' must exist in the zh catalog");
        assert_eq!(
            sdforge::i18n::translate_for("zh", "main-config-loaded", &[]),
            zh_value,
            "zh 必须命中 zh 目录"
        );
    }

    #[test]
    fn test_missing_key_returns_key_name_without_panic() {
        // 缺键路径：translate_for 回退链终点返回键名本身
        assert_eq!(t("no-such-key"), "no-such-key");
        assert_eq!(
            t_args("no-such-key", &[("vendor", String::from("openai"))]),
            "no-such-key"
        );
    }

    #[test]
    fn test_placeholder_substitution_via_registered_catalog() {
        ensure_registered();
        // 注册表面（宿主注册表条目）的 { $name } 占位符替换：
        // 显式 locale 保证断言不受进程环境影响
        let out = sdforge::i18n::translate_for(
            "en",
            "main-logger-init-failed",
            &[("error", String::from("boom"))],
        );
        assert!(out.contains("boom"), "实参必须替换占位符: {out}");
        assert!(!out.contains("{ $error }"), "占位符不得残留: {out}");
        // 缺实参：占位符原样保留（sdforge 模板语义，不报错回退）
        let untouched = sdforge::i18n::translate_for("en", "main-logger-init-failed", &[]);
        assert!(
            untouched.contains("{ $error }"),
            "缺实参时占位符应原样保留: {untouched}"
        );
    }

    #[test]
    fn test_translations_available_after_init() {
        init();
        assert!(
            REGISTERED.get().is_some(),
            "init 后注册表必须已就绪（OnceLock 已走过）"
        );
        assert!(has_key("main-config-loaded"));
        assert!(!has_key("no-such-key"));
    }

    #[test]
    fn test_entries_for_unknown_locale_falls_back_to_en() {
        // 未知 locale 直查：entries_for 的 unwrap_or_else 回落 en 目录。
        // 生产调用方只传 en/zh，该防御面由测试钉住（键集合与 en 完全一致）。
        let fr_keys: std::collections::BTreeSet<String> = entries_for("fr")
            .into_iter()
            .map(|(k, _)| k.to_string())
            .collect();
        assert_eq!(fr_keys, keys_for("en"), "未知 locale 必须整目录回落 en");
    }

    #[test]
    fn test_parse_entries_skips_empty_key_lines() {
        // 空键行（"= orphan"）丢弃：键为空的行不能进目录，否则占位一个
        // 无法查找的空键条目
        let parsed = parse_entries("# comment\n\nplain line\n= orphan\n valid = v\n");
        assert_eq!(parsed, vec![("valid", "v")]);
    }
}
