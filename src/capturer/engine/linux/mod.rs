/// Linux/FreeBSD 平台屏幕捕获实现
/// 支持 Wayland 和 X11 显示服务器协议

#[cfg(not(any(feature = "wayland", feature = "x11")))]
compile_error!("必须启用 'wayland' 或 'x11' 特性之一。");

use std::{env, sync::mpsc};

use anyhow::{Result, anyhow};

#[cfg(feature = "x11")]
use x11::X11Capturer;

use crate::{Target, capturer::Options, frame::Frame};

/// 错误处理模块
mod error;

/// Wayland 捕获器模块
#[cfg(feature = "wayland")]
mod wayland;

/// X11 捕获器模块
#[cfg(feature = "x11")]
mod x11;

/// Wayland 捕获器类型导入
#[cfg(feature = "wayland")]
use wayland::WaylandCapturer;

/// Linux 捕获器实现 trait
/// 定义捕获器的基本接口
pub trait LinuxCapturerImpl {
    /// 开始屏幕捕获
    fn start_capture(&mut self);

    /// 停止屏幕捕获
    fn stop_capture(&mut self);

    /// 获取当前捕获目标
    /// 默认返回 None
    fn target(&self) -> Option<&Target> {
        None
    }
}

/// Linux 捕获器结构体
/// 封装具体的捕获器实现（Wayland 或 X11）
pub struct LinuxCapturer {
    pub imp: Box<dyn LinuxCapturerImpl>, // 具体实现（Wayland 或 X11）
}

/// 帧数据发送通道类型别名
type Type = mpsc::Sender<Result<Frame>>;

impl LinuxCapturer {
    /// 创建新的 Linux 捕获器
    /// 根据环境变量自动检测显示服务器类型
    pub fn new(options: &Options, tx: Type) -> Result<Self> {
        // 优先尝试 Wayland
        #[cfg(feature = "wayland")]
        if env::var("WAYLAND_DISPLAY").is_ok() {
            log::debug!("创建新的 Wayland 屏幕捕获器");
            return Ok(Self {
                imp: Box::new(WaylandCapturer::new(options, tx)?),
            });
        }

        // 然后尝试 X11
        #[cfg(feature = "x11")]
        if env::var("DISPLAY").is_ok() {
            log::debug!("创建新的 X11 屏幕捕获器");
            return Ok(Self {
                imp: Box::new(X11Capturer::new(options, tx)?),
            });
        }

        // 根据启用的特性返回相应的错误信息
        #[cfg(all(feature = "wayland", feature = "x11"))]
        let error_msg = "不支持的平台：无法检测到 Wayland 或 X11 显示器";
        #[cfg(all(not(feature = "wayland"), feature = "x11"))]
        let error_msg =
            "不支持的平台：无法检测到 X11 显示器。请启用 'wayland' 特性以支持 Wayland。";
        #[cfg(all(feature = "wayland", not(feature = "x11")))]
        let error_msg = "不支持的平台：无法检测到 Wayland 显示器。请启用 'x11' 特性以支持 X11。";

        Err(anyhow!(error_msg))
    }
}

/// 创建 Linux 捕获器
/// 根据选项和通道创建捕获器实例
pub fn create_capturer(
    options: &Options,
    tx: mpsc::Sender<Result<Frame>>,
) -> Result<LinuxCapturer> {
    LinuxCapturer::new(options, tx)
}
