// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

// 应用层错误处理：Database 等应用面变体定义在本模块，路由核心变体集（GateError）
// 在子模块 gate 并经 `Gate` 包装携带（`From<dbnexus::DbError>` 受孤儿规则约束须同 crate）。
// 实现（展示轨道 / HTTP 映射 / 来源转换）在子模块 app，本文件只保留类型与 re-export。
use thiserror::Error;

mod app;
pub(crate) mod gate;

pub use gate::GateError;

/// 错误双轨：`#[error]` Display 恒英文规范串（日志/错误链），HTTP 展示走 `user_message`
///（FTL，随 locale）；`Gate` 变体 `transparent` 透传英文规范串。
/// `Database.message` 存语言中性锚点（`"<ftl键>"` 或 `"<ftl键>|{json}"`），
/// 或来自底层库的原始英文串（走 `error-database` 通用模板回退），见 `app.rs`。
#[derive(Error, Debug)]
pub enum FluxweirError {
    #[error("Database error: {message}")]
    Database { message: String },

    #[error(transparent)]
    Gate(#[from] GateError),

    /// 密钥保密柜（key-vault）：Display 走 VaultError 英文规范串。
    #[cfg(feature = "vault")]
    #[error(transparent)]
    Vault(#[from] crate::vault::VaultError),
}
