use std::sync::mpsc;

use anyhow::Result;

use super::Options;
use crate::{frame::Frame, Target};

/// macOS 平台引擎模块
#[cfg(target_os = "macos")]
pub mod mac;

/// Windows 平台引擎模块
#[cfg(target_os = "windows")]
mod win;

/// Linux/FreeBSD 平台引擎模块
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
mod linux;

/// 通道数据类型定义
/// macOS 使用 CMSampleBuffer，其他平台直接使用 Frame
#[cfg(target_os = "macos")]
pub type ChannelItem = (
    screencapturekit::cm_sample_buffer::CMSampleBuffer,
    screencapturekit::sc_output_handler::SCStreamOutputType,
);
#[cfg(not(target_os = "macos"))]
pub type ChannelItem = Frame;

/// 获取输出帧的尺寸
/// 根据捕获选项和平台特性计算最终输出尺寸
pub fn get_output_frame_size(options: &Options) -> [u32; 2] {
    #[cfg(target_os = "macos")]
    {
        mac::get_output_frame_size(options)
    }

    #[cfg(target_os = "windows")]
    {
        win::get_output_frame_size(options)
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        // TODO: 如何在 Linux 上计算输出尺寸？
        return [0, 0];
    }
}

/// 捕获引擎结构体
/// 封装不同平台的屏幕捕获实现
pub struct Engine {
    options: Options,                    // 捕获选项配置
    target: Option<Target>,              // 当前捕获目标

    /// macOS 平台的 SCStream 捕获流
    #[cfg(target_os = "macos")]
    mac: screencapturekit::sc_stream::SCStream,

    /// macOS 平台的错误标志
    #[cfg(target_os = "macos")]
    error_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,

    /// Windows 平台的 WCStream 捕获流
    #[cfg(target_os = "windows")]
    win: win::WCStream,

    /// Linux 平台的捕获器
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    linux: linux::LinuxCapturer,
}

impl Engine {
    /// 创建新的引擎实例
    /// 根据当前平台初始化对应的捕获器
    pub fn new(options: &Options, tx: mpsc::Sender<Result<ChannelItem>>) -> Result<Engine> {
        #[cfg(target_os = "macos")]
        {
            // 创建错误标志
            let error_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            // 创建 macOS 捕获器
            let (mac, target) = mac::create_capturer(options, tx, error_flag.clone());

            Ok(Engine {
                mac,
                error_flag,
                options: (*options).clone(),
                target: Some(target),
            })
        }

        #[cfg(target_os = "windows")]
        {
            // 创建 Windows 捕获器
            let (win, target) = win::create_capturer(options, tx);
            Ok(Engine {
                win,
                options: (*options).clone(),
                target: Some(target),
            })
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            // 创建 Linux 捕获器
            use linux::LinuxCapturerImpl;
            let linux = linux::create_capturer(&options, tx)?;
            let target = linux.imp.target().cloned();
            Ok(Engine {
                linux,
                options: (*options).clone(),
                target,
            })
        }
    }

    /// 启动屏幕捕获
    pub fn start(&mut self) {
        #[cfg(target_os = "macos")]
        {
            self.mac.start_capture().expect("启动捕获失败");
        }

        #[cfg(target_os = "windows")]
        {
            self.win.start_capture();
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            self.linux.imp.start_capture();
        }
    }

    /// 停止屏幕捕获
    pub fn stop(&mut self) {
        #[cfg(target_os = "macos")]
        {
            self.mac.stop_capture().expect("停止捕获失败");
        }

        #[cfg(target_os = "windows")]
        {
            self.win.stop_capture();
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            self.linux.imp.stop_capture();
        }
    }

    /// 获取输出帧尺寸
    pub fn get_output_frame_size(&mut self) -> [u32; 2] {
        get_output_frame_size(&self.options)
    }

    /// 处理通道数据并转换为帧
    /// macOS 需要额外的处理步骤
    pub fn process_channel_item(&self, data: ChannelItem) -> Option<Frame> {
        #[cfg(target_os = "macos")]
        {
            mac::process_sample_buffer(data.0, data.1, self.options.output_type)
        }
        #[cfg(not(target_os = "macos"))]
        Some(data)
    }

    /// 获取当前捕获目标
    pub fn target(&self) -> Option<&Target> {
        self.target.as_ref()
    }
}
