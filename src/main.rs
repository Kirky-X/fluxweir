// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 薄启动入口 — 装配链已下沉 `fluxweir::server`（库化承载）。此处仅解析
//! config_path 参数，缺省 "config.toml" 的语义由 `server::run` 内部承担；
//! 首参数为 `cost-replay` 时改走离线成本回放子命令（sdforge CLI 面）。

// 装配链 async 块在 mysql 组合下 trait 求解深度超默认 128 层（sqlx 类型层级更深）
#![recursion_limit = "256"]

#[tokio::main]
async fn main() -> Result<(), fluxweir::error::FluxweirError> {
    // cost-replay 子命令（原独立 bin fluxweir-replay 折叠而来）：离线只读
    // 回放，双门控（cost-replay + db-sqlite）下才编入，见 utils::replay::cli。
    #[cfg(all(feature = "cost-replay", feature = "db-sqlite"))]
    if std::env::args().nth(1).as_deref() == Some("cost-replay") {
        fluxweir::utils::replay::cli::main_entry().await;
    }
    // 门控关闭的构建收到 cost-replay：显式报错，而不是把子命令名当配置路径
    // 静默吞掉（服务端会报"找不到配置文件"，误导排查方向）。
    #[cfg(not(all(feature = "cost-replay", feature = "db-sqlite")))]
    if std::env::args().nth(1).as_deref() == Some("cost-replay") {
        eprintln!("cost-replay is not built in: rebuild with --features cost-replay,db-sqlite");
        std::process::exit(2);
    }
    // 仅读 CLI 配置路径，不进 exec/网络面；扫描器 INFO 按误报行内豁免
    let config_path = std::env::args().nth(1); // nosemgrep
    fluxweir::server::run(config_path).await
}
