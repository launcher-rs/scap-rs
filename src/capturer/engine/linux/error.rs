/// Linux 屏幕捕获错误类型定义
/// 实现各种错误类型的转换
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    sync::PoisonError,
};

/// PipeWire 序列化错误类型（仅在启用 Wayland 特性时）
#[cfg(feature = "wayland")]
use pipewire::spa::pod::serialize::GenError;

/// Linux 屏幕捕获错误结构体
/// 封装各种可能的错误信息
#[derive(Debug)]
pub struct LinCapError {
    msg: String, // 错误消息
}

/// 实现 Error trait
impl Error for LinCapError {}

impl LinCapError {
    /// 创建新的错误实例
    pub fn new(msg: String) -> Self {
        Self { msg }
    }
}

/// 实现 Display trait 用于错误信息显示
impl Display for LinCapError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

/// 从 PipeWire 错误转换
#[cfg(feature = "wayland")]
impl From<pipewire::Error> for LinCapError {
    fn from(e: pipewire::Error) -> Self {
        Self::new(e.to_string())
    }
}

/// 从通道发送错误转换
impl From<std::sync::mpsc::SendError<bool>> for LinCapError {
    fn from(e: std::sync::mpsc::SendError<bool>) -> Self {
        Self::new(e.to_string())
    }
}

/// 从 PipeWire 序列化错误转换
#[cfg(feature = "wayland")]
impl From<GenError> for LinCapError {
    fn from(e: GenError) -> Self {
        Self::new(e.to_string())
    }
}

/// 从 D-Bus 错误转换
#[cfg(feature = "wayland")]
impl From<dbus::Error> for LinCapError {
    fn from(e: dbus::Error) -> Self {
        Self::new(e.to_string())
    }
}

/// 从锁中毒错误转换
impl<T> From<PoisonError<T>> for LinCapError {
    fn from(e: PoisonError<T>) -> Self {
        Self::new(e.to_string())
    }
}
