//! 跨平台、高性能、高质量的屏幕录制库
//!
//! 本库支持 macOS、Windows 和 Linux 平台，提供屏幕捕获、帧处理等功能。
//! 主要功能包括：
//! - 屏幕捕获（支持全屏和窗口捕获）
//! - 多种帧格式支持（YUV、RGB、BGRA 等）
//! - 跨平台兼容性
//! - 权限管理

/// 屏幕捕获模块，提供捕获器和相关配置
pub mod capturer;

/// 帧数据模块，定义各种帧格式和转换函数
pub mod frame;

/// 目标管理模块，处理捕获目标（显示器、窗口）的枚举和管理
mod targets;

/// 工具函数模块，提供权限检查、平台支持检测等辅助功能
mod utils;

// 重新导出常用类型和函数，方便用户使用
pub use targets::Target; // 目标类型枚举
pub use targets::get_all_targets; // 获取所有可捕获目标
pub use targets::{Display, Window}; // 显示器和窗口类型
pub use utils::has_permission; // 检查是否有屏幕捕获权限
pub use utils::is_supported; // 检查当前平台是否支持
pub use utils::request_permission; // 请求屏幕捕获权限

/// macOS 平台特定的引擎模块
#[cfg(target_os = "macos")]
pub mod engine {
    pub use crate::capturer::engine::mac;
}
