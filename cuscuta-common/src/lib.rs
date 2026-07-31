//! cuscuta的通用组件
//!
//! # 速览
//! cuscuta-common定义了一些通用的函数、数据模型和一些实用函数，包括各种API，数据库的通用业务操作，定时操作的简易注册和管理，和一些全局变量的线程安全的便捷操作
//!
//! # 局限
//! 很明显，cuscuta并没有做数据层兼容，即cuscuta目前**强绑定于**`Redis`和`PostgreSQL`，这将在未来必要时重构

/// api相关
pub mod api;

/// 通用数据结构相关
pub mod data;

/// 数据库相关
pub mod db;

/// 便捷变量操作相关
pub mod quick_fetch;

/// 定时操作相关
pub mod scheduled_job;
