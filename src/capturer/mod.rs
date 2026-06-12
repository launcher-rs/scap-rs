/// 屏幕捕获引擎模块，提供底层捕获实现
pub mod engine;

use std::{error::Error, sync::mpsc};

use anyhow::anyhow;

use engine::ChannelItem;

use crate::{
    frame::{Frame, FrameType},
    has_permission, is_supported,
    targets::Target,
};

/// 重新导出输出帧尺寸计算函数
pub use engine::get_output_frame_size;

/// 输出分辨率枚举，支持多种标准分辨率
#[derive(Debug, Clone, Copy, Default)]
pub enum Resolution {
    _480p,  // 480p 分辨率
    _720p,  // 720p 高清分辨率
    _1080p, // 1080p 全高清分辨率
    _1440p, // 1440p 2K 分辨率
    _2160p, // 2160p 4K 分辨率
    _4320p, // 4320p 8K 分辨率

    #[default]
    Captured, // 使用原始捕获分辨率
}

impl Resolution {
    /// 根据宽高比计算实际输出分辨率
    /// 返回 [宽度, 高度] 的像素值
    fn value(&self, aspect_ratio: f32) -> [u32; 2] {
        match *self {
            Resolution::_480p => [640, (640_f32 / aspect_ratio).floor() as u32],
            Resolution::_720p => [1280, (1280_f32 / aspect_ratio).floor() as u32],
            Resolution::_1080p => [1920, (1920_f32 / aspect_ratio).floor() as u32],
            Resolution::_1440p => [2560, (2560_f32 / aspect_ratio).floor() as u32],
            Resolution::_2160p => [3840, (3840_f32 / aspect_ratio).floor() as u32],
            Resolution::_4320p => [7680, (7680_f32 / aspect_ratio).floor() as u32],
            Resolution::Captured => {
                panic!("不应在 Captured 分辨率类型上调用 .value 方法")
            }
        }
    }
}

/// 二维坐标点结构体
#[derive(Debug, Default, Clone)]
pub struct Point {
    pub x: f64, // X 坐标
    pub y: f64, // Y 坐标
}

/// 尺寸结构体，表示宽度和高度
#[derive(Debug, Default, Clone)]
pub struct Size {
    pub width: f64,  // 宽度
    pub height: f64, // 高度
}

/// 区域结构体，由原点和尺寸定义
#[derive(Debug, Default, Clone)]
pub struct Area {
    pub origin: Point, // 区域原点
    pub size: Size,    // 区域尺寸
}

/// 屏幕捕获选项配置
/// 包含帧率、目标、裁剪区域、输出格式等参数
#[derive(Debug, Default, Clone)]
pub struct Options {
    pub fps: u32,                              // 帧率（每秒帧数）
    pub show_cursor: bool,                     // 是否显示鼠标光标
    pub show_highlight: bool,                  // 是否显示高亮效果
    pub target: Option<Target>,                // 捕获目标（显示器或窗口）
    pub crop_area: Option<Area>,               // 裁剪区域（None 表示全屏）
    pub output_type: FrameType,                // 输出帧格式
    pub output_resolution: Resolution,         // 输出分辨率
    pub excluded_targets: Option<Vec<Target>>, // 排除的目标（仅 macOS 支持）
}

/// 屏幕捕获器主类
/// 负责管理屏幕捕获的生命周期
pub struct Capturer {
    engine: engine::Engine,                          // 底层捕获引擎
    rx: mpsc::Receiver<anyhow::Result<ChannelItem>>, // 帧数据接收通道
}

/// 捕获器构建错误类型
#[derive(Debug)]
pub enum CapturerBuildError {
    NotSupported,         // 平台不支持
    PermissionNotGranted, // 权限未授予
}

/// 实现 Display trait 用于错误信息显示
impl std::fmt::Display for CapturerBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapturerBuildError::NotSupported => write!(f, "屏幕捕获不支持"),
            CapturerBuildError::PermissionNotGranted => {
                write!(f, "未获得屏幕捕获权限")
            }
        }
    }
}

/// 实现 Error trait
impl Error for CapturerBuildError {}

impl Capturer {
    /// 创建新的捕获器实例（已弃用）
    /// 请使用 `build` 方法替代
    #[deprecated(since = "0.0.6", note = "请使用 `build` 方法创建新的捕获器实例。")]
    pub fn new(options: Options) -> anyhow::Result<Capturer> {
        let (tx, rx) = mpsc::channel();
        let engine = engine::Engine::new(&options, tx)?;

        Ok(Capturer { engine, rx })
    }

    /// 使用指定选项构建新的捕捕器实例
    /// 会检查平台支持和权限
    pub fn build(options: Options) -> anyhow::Result<Capturer> {
        // 检查平台是否支持屏幕捕获
        if !is_supported() {
            return Err(anyhow!(CapturerBuildError::NotSupported));
        }

        // 检查是否有屏幕捕获权限
        if !has_permission() {
            return Err(anyhow!(CapturerBuildError::PermissionNotGranted));
        }

        // 创建帧数据传输通道
        let (tx, rx) = mpsc::channel();
        // 创建底层引擎实例
        let engine = engine::Engine::new(&options, tx)?;

        Ok(Capturer { engine, rx })
    }

    // TODO: 防止重复启动捕获
    /// 开始屏幕捕获
    pub fn start_capture(&mut self) {
        self.engine.start();
    }

    /// 停止屏幕捕获
    pub fn stop_capture(&mut self) {
        self.engine.stop();
    }

    /// 获取下一帧捕获的图像
    /// 循环接收帧数据直到获得有效帧
    pub fn get_next_frame(&self) -> anyhow::Result<Frame> {
        loop {
            // 从通道接收帧数据
            let res = self.rx.recv()??;

            // 处理通道数据并返回帧
            if let Some(frame) = self.engine.process_channel_item(res) {
                return Ok(frame);
            }
        }
    }

    /// 获取输出帧的尺寸
    /// 返回 [宽度, 高度] 的像素值
    pub fn get_output_frame_size(&mut self) -> [u32; 2] {
        self.engine.get_output_frame_size()
    }

    /// 获取原始捕获器引用
    pub fn raw(&self) -> RawCapturer<'_> {
        RawCapturer { capturer: self }
    }

    /// 获取当前捕获目标
    pub fn target(&self) -> Option<&Target> {
        self.engine.target()
    }
}

/// 原始捕获器包装结构体
pub struct RawCapturer<'a> {
    #[allow(dead_code)] // used on macOS via cfg
    capturer: &'a Capturer, // 捕获器引用
}
