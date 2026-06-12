/// Windows 平台屏幕捕获实现
/// 使用 Windows Graphics Capture API 进行屏幕捕获

use crate::{
    capturer::{Area, Options, Point, Resolution, Size},
    frame::{BGRAFrame, Frame, FrameType, RGBxFrame},
    targets::{self, Target},
};
use std::cmp;
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};
use windows_capture::{
    capture::{CaptureControl, Context, GraphicsCaptureApiHandler},
    frame::Frame as WCFrame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor as WCMonitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings as WCSettings,
    },
    window::Window as WCWindow,
};

/// Windows 捕获器结构体
/// 实现 GraphicsCaptureApiHandler trait 处理帧数据
#[derive(Debug)]
struct Capturer {
    pub tx: mpsc::Sender<anyhow::Result<Frame>>,  // 帧数据发送通道
    pub crop: Option<Area>,                       // 裁剪区域
}

/// 捕获设置枚举，支持窗口或显示器捕获
#[derive(Clone)]
enum Settings {
    Window(WCSettings<FlagStruct, WCWindow>),    // 窗口捕获设置
    Display(WCSettings<FlagStruct, WCMonitor>),  // 显示器捕获设置
}

/// Windows 捕获流结构体
/// 管理捕获会话的生命周期
pub struct WCStream {
    settings: Settings,  // 捕获设置
    capture_control: Option<CaptureControl<Capturer, Box<dyn std::error::Error + Send + Sync>>>,  // 捕获控制句柄
}

/// 实现 GraphicsCaptureApiHandler trait
/// 处理帧到达和流关闭事件
impl GraphicsCaptureApiHandler for Capturer {
    type Flags = FlagStruct;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    /// 创建新的捕获器实例
    fn new(context: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            tx: context.flags.tx,
            crop: context.flags.crop,
        })
    }

    /// 帧到达回调函数
    /// 处理捕获的帧数据并发送到通道
    fn on_frame_arrived(
        &mut self,
        frame: &mut WCFrame,
        _: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        // 获取帧的颜色格式
        let color_format = frame.color_format();

        // 根据是否裁剪获取帧数据
        let (width, height, data) = match &self.crop {
            Some(cropped_area) => {
                // 计算裁剪区域坐标
                let start_x = cropped_area.origin.x as u32;
                let start_y = cropped_area.origin.y as u32;
                let end_x = (cropped_area.origin.x + cropped_area.size.width) as u32;
                let end_y = (cropped_area.origin.y + cropped_area.size.height) as u32;

                // 裁剪帧缓冲区
                let cropped_buffer = frame
                    .buffer_crop(start_x, start_y, end_x, end_y)
                    .expect("裁剪缓冲区失败");

                // 获取原始帧数据
                let mut nopadding_buffer = Vec::new();
                let raw_frame_buffer = cropped_buffer.as_nopadding_buffer(&mut nopadding_buffer);

                (
                    cropped_area.size.width as i32,
                    cropped_area.size.height as i32,
                    raw_frame_buffer.to_vec(),
                )
            }
            None => {
                // 获取完整帧数据
                let width = frame.width() as i32;
                let height = frame.height() as i32;
                let mut frame_buffer = frame.buffer().unwrap();
                let raw_frame_buffer = frame_buffer.as_raw_buffer();
                (width, height, raw_frame_buffer.to_vec())
            }
        };

        // 获取当前时间戳
        let display_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("获取当前时间失败")
            .as_nanos() as u64;

        // 根据颜色格式创建对应的帧对象
        let frame = match color_format {
            ColorFormat::Rgba16F => return Err("Rgba16F 格式暂不支持".into()),
            ColorFormat::Rgba8 => Frame::RGBx(RGBxFrame {
                display_time,
                width,
                height,
                data,
            }),
            ColorFormat::Bgra8 => Frame::BGRA(BGRAFrame {
                display_time,
                width,
                height,
                data,
            }),
        };

        // 发送帧数据到通道
        Ok(self.tx.send(Ok(frame))?)
    }

    /// 流关闭回调函数
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        log::debug!("屏幕捕获流已关闭");
        Ok(())
    }
}

impl WCStream {
    /// 启动屏幕捕获
    pub fn start_capture(&mut self) {
        let cc = match &self.settings {
            Settings::Display(st) => Capturer::start_free_threaded(st.to_owned()).unwrap(),
            Settings::Window(st) => Capturer::start_free_threaded(st.to_owned()).unwrap(),
        };

        self.capture_control = Some(cc)
    }

    /// 停止屏幕捕获
    pub fn stop_capture(&mut self) {
        let capture_control = self.capture_control.take().unwrap();
        let _ = capture_control.stop();
    }
}

/// 标志结构体，包含通道和裁剪区域
#[derive(Clone, Debug)]
struct FlagStruct {
    pub tx: mpsc::Sender<anyhow::Result<Frame>>,  // 帧数据发送通道
    pub crop: Option<Area>,                       // 裁剪区域
}

/// 创建 Windows 捕获器
/// 根据选项配置创建捕获流和目标
pub fn create_capturer(options: &Options, tx: mpsc::Sender<anyhow::Result<Frame>>) -> (WCStream, Target) {
    // 获取捕获目标，默认使用主显示器
    let target = options.target.clone().unwrap_or_else(|| {
        Target::Display(targets::get_main_display().expect("获取主显示器失败"))
    });

    // 根据输出类型选择颜色格式
    let color_format = match options.output_type {
        FrameType::BGRAFrame => ColorFormat::Bgra8,
        _ => ColorFormat::Rgba8,
    };

    // 设置鼠标光标捕获选项
    let show_cursor = match options.show_cursor {
        true => CursorCaptureSettings::WithCursor,
        false => CursorCaptureSettings::WithoutCursor,
    };

    // 根据目标类型创建捕获设置
    let settings = match target.clone() {
        Target::Display(display) => Settings::Display(WCSettings::new(
            WCMonitor::from_raw_hmonitor(display.raw_handle.0),
            show_cursor,
            DrawBorderSettings::Default,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Default,
            DirtyRegionSettings::Default,
            color_format,
            FlagStruct {
                tx,
                crop: Some(get_crop_area(options)),
            },
        )),
        Target::Window(window) => Settings::Window(WCSettings::new(
            WCWindow::from_raw_hwnd(window.raw_handle.0),
            show_cursor,
            DrawBorderSettings::Default,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Default,
            DirtyRegionSettings::Default,
            color_format,
            FlagStruct {
                tx,
                crop: Some(get_crop_area(options)),
            },
        )),
    };

    (WCStream {
        settings,
        capture_control: None,
    }, target)
}

/// 获取输出帧尺寸
/// 根据裁剪区域和分辨率选项计算最终输出尺寸
pub fn get_output_frame_size(options: &Options) -> [u32; 2] {
    let crop_area = get_crop_area(options);

    let mut output_width = (crop_area.size.width) as u32;
    let mut output_height = (crop_area.size.height) as u32;

    // 应用分辨率限制
    match options.output_resolution {
        Resolution::Captured => {}  // 使用原始分辨率
        _ => {
            let [resolved_width, resolved_height] = options
                .output_resolution
                .value((crop_area.size.width as f32) / (crop_area.size.height as f32));
            // 取较小值以确保不超过目标分辨率
            output_width = cmp::min(output_width, resolved_width);
            output_height = cmp::min(output_height, resolved_height);
        }
    }

    // 确保尺寸为偶数
    output_width -= output_width % 2;
    output_height -= output_height % 2;

    [output_width, output_height]
}

/// 获取绝对值坐标（考虑缩放因子）
fn get_absolute_value(value: f64, scale_factor: f64) -> f64 {
    let value = (value * scale_factor).floor();
    value + value % 2.0
}

/// 获取裁剪区域
/// 根据选项和目标尺寸计算实际裁剪区域
pub fn get_crop_area(options: &Options) -> Area {
    // 获取捕获目标
    let target = options.target.clone().unwrap_or_else(|| {
        Target::Display(targets::get_main_display().expect("获取主显示器失败"))
    });

    // 获取目标尺寸
    let (width, height) = targets::get_target_dimensions(&target);

    // 获取缩放因子
    let scale_factor = targets::get_scale_factor(&target);

    // 如果指定了裁剪区域，则应用缩放
    options
        .crop_area
        .as_ref()
        .map(|val| {
            Area {
                origin: Point {
                    x: get_absolute_value(val.origin.x, scale_factor),
                    y: get_absolute_value(val.origin.y, scale_factor),
                },
                size: Size {
                    width: get_absolute_value(val.size.width, scale_factor),
                    height: get_absolute_value(val.size.height, scale_factor),
                },
            }
        })
        // 否则使用完整目标区域
        .unwrap_or_else(|| Area {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: width as f64,
                height: height as f64,
            },
        })
}
