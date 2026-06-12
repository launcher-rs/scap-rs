/// macOS 平台屏幕捕获实现
/// 使用 ScreenCaptureKit 框架进行屏幕捕获

use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::{cmp, sync::Arc};

use pixelformat::get_pts_in_nanoseconds;
use screencapturekit::{
    cm_sample_buffer::CMSampleBuffer,
    sc_content_filter::{InitParams, SCContentFilter},
    sc_error_handler::StreamErrorHandler,
    sc_output_handler::{SCStreamOutputType, StreamOutput},
    sc_shareable_content::SCShareableContent,
    sc_stream::SCStream,
    sc_stream_configuration::{PixelFormat, SCStreamConfiguration},
    sc_types::SCFrameStatus,
};
use screencapturekit_sys::os_types::base::{CMTime, CMTimeScale};
use screencapturekit_sys::os_types::geometry::{CGPoint, CGRect, CGSize};

use crate::frame::{Frame, FrameType};
use crate::targets::Target;
use crate::{
    capturer::{Area, Options, Point, Resolution, Size},
    frame::BGRAFrame,
    targets,
};

use super::ChannelItem;

/// Apple 系统相关模块
mod apple_sys;
/// 像素缓冲区模块
mod pixel_buffer;
/// 像素格式转换模块
mod pixelformat;

/// 重新导出 PixelBuffer 类型
pub use pixel_buffer::PixelBuffer;

/// 错误处理器结构体
/// 实现 StreamErrorHandler trait 处理流错误
struct ErrorHandler {
    error_flag: Arc<AtomicBool>,  // 错误标志原子变量
}

/// 实现 StreamErrorHandler trait
impl StreamErrorHandler for ErrorHandler {
    fn on_error(&self) {
        log::error!("屏幕捕获错误发生");
        self.error_flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// macOS 捕获器结构体
/// 实现 StreamOutput trait 处理输出数据
pub struct Capturer {
    pub tx: mpsc::Sender<anyhow::Result<ChannelItem>>,  // 帧数据发送通道
}

impl Capturer {
    /// 创建新的捕获器实例
    pub fn new(tx: mpsc::Sender<anyhow::Result<ChannelItem>>) -> Self {
        Capturer { tx }
    }
}

/// 实现 StreamOutput trait 处理输出的样本缓冲区
impl StreamOutput for Capturer {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, of_type: SCStreamOutputType) {
        self.tx.send(Ok((sample, of_type))).unwrap_or(());
    }
}

/// 创建 macOS 捕获器
/// 配置 ScreenCaptureKit 流并返回捕获流和目标
pub fn create_capturer(
    options: &Options,
    tx: mpsc::Sender<anyhow::Result<ChannelItem>>,
    error_flag: Arc<AtomicBool>,
) -> (SCStream, Target) {
    // 如果未指定目标，捕获主显示器
    let target = options
        .target
        .clone()
        .unwrap_or_else(|| Target::Display(targets::get_main_display().unwrap()));

    // 获取可共享内容（显示器和窗口列表）
    let sc_shareable_content = SCShareableContent::current();

    // 根据目标类型创建内容过滤器参数
    let params = match &target {
        Target::Window(window) => {
            // 从窗口 ID 获取 SCWindow 对象
            let sc_window = sc_shareable_content
                .windows
                .into_iter()
                .find(|sc_win| sc_win.window_id == window.id)
                .unwrap();

            // 返回桌面独立窗口参数
            // https://developer.apple.com/documentation/screencapturekit/sccontentfilter/3919804-init
            InitParams::DesktopIndependentWindow(sc_window)
        }
        Target::Display(display) => {
            // 从显示器 ID 获取 SCDisplay 对象
            let sc_display = sc_shareable_content
                .displays
                .into_iter()
                .find(|sc_dis| sc_dis.display_id == display.id)
                .unwrap();

            // 根据是否排除目标创建参数
            match &options.excluded_targets {
                None => InitParams::Display(sc_display),
                Some(excluded_targets) => {
                    // 过滤出需要排除的窗口
                    let excluded_windows = sc_shareable_content
                        .windows
                        .into_iter()
                        .filter(|window| {
                            excluded_targets
                                .iter()
                                .any(|excluded_target| match excluded_target {
                                    Target::Window(excluded_window) => {
                                        excluded_window.id == window.window_id
                                    }
                                    _ => false,
                                })
                        })
                        .collect();

                    InitParams::DisplayExcludingWindows(sc_display, excluded_windows)
                }
            }
        }
    };

    // 创建内容过滤器
    let filter = SCContentFilter::new(params);

    // 获取裁剪区域
    let crop_area = get_crop_area(options);

    // 设置源区域矩形
    let source_rect = CGRect {
        origin: CGPoint {
            x: crop_area.origin.x,
            y: crop_area.origin.y,
        },
        size: CGSize {
            width: crop_area.size.width,
            height: crop_area.size.height,
        },
    };

    // 根据输出类型选择像素格式
    let pixel_format = match options.output_type {
        FrameType::YUVFrame => PixelFormat::YCbCr420v,
        FrameType::BGR0 => PixelFormat::ARGB8888,
        FrameType::RGB => PixelFormat::ARGB8888,
        FrameType::BGRAFrame => PixelFormat::ARGB8888,
    };

    // 获取输出帧尺寸
    let [width, height] = get_output_frame_size(options);

    // 创建流配置
    let stream_config = SCStreamConfiguration {
        width,
        height,
        source_rect,
        pixel_format,
        shows_cursor: options.show_cursor,
        minimum_frame_interval: CMTime {
            value: 1,
            timescale: options.fps as CMTimeScale,
            epoch: 0,
            flags: 1,
        },
        ..Default::default()
    };

    // 创建 SCStream 并添加输出
    let mut stream = SCStream::new(filter, stream_config, ErrorHandler { error_flag });
    stream.add_output(Capturer::new(tx), SCStreamOutputType::Screen);

    (stream, target)
}

/// 获取输出帧尺寸
/// 根据目标缩放因子和分辨率选项计算最终输出尺寸
pub fn get_output_frame_size(options: &Options) -> [u32; 2] {
    // 获取捕获目标
    let target = options
        .target
        .clone()
        .unwrap_or_else(|| Target::Display(targets::get_main_display().unwrap()));

    // 获取缩放因子（DPI）
    let scale_factor = targets::get_scale_factor(&target);
    // 获取裁剪区域
    let source_rect = get_crop_area(options);

    // 计算输出尺寸，需要乘以缩放因子
    let mut output_width = (source_rect.size.width as u32) * (scale_factor as u32);
    let mut output_height = (source_rect.size.height as u32) * (scale_factor as u32);

    // 应用分辨率限制
    match options.output_resolution {
        Resolution::Captured => {}  // 使用原始分辨率
        _ => {
            let [resolved_width, resolved_height] = options
                .output_resolution
                .value((source_rect.size.width as f32) / (source_rect.size.height as f32));
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

/// 获取裁剪区域
/// 根据选项和目标尺寸计算实际裁剪区域
pub fn get_crop_area(options: &Options) -> Area {
    // 获取捕获目标
    let target = options
        .target
        .clone()
        .unwrap_or_else(|| Target::Display(targets::get_main_display().unwrap()));

    // 获取目标尺寸
    let (width, height) = targets::get_target_dimensions(&target);

    // 如果指定了裁剪区域，则调整为偶数尺寸
    options
        .crop_area
        .as_ref()
        .map(|val| {
            let input_width = val.size.width + (val.size.width % 2.0);
            let input_height = val.size.height + (val.size.height % 2.0);

            Area {
                origin: Point {
                    x: val.origin.x,
                    y: val.origin.y,
                },
                size: Size {
                    width: input_width,
                    height: input_height,
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

/// 处理样本缓冲区
/// 将 CMSampleBuffer 转换为 Frame 对象
pub fn process_sample_buffer(
    sample: CMSampleBuffer,
    of_type: SCStreamOutputType,
    output_type: FrameType,
) -> Option<Frame> {
    // 只处理屏幕输出类型
    if let SCStreamOutputType::Screen = of_type {
        let frame_status = &sample.frame_status;

        match frame_status {
            // 帧完成或开始状态，创建对应的帧对象
            SCFrameStatus::Complete | SCFrameStatus::Started => unsafe {
                return Some(match output_type {
                    FrameType::YUVFrame => {
                        let yuvframe = pixelformat::create_yuv_frame(sample).unwrap();
                        Frame::YUVFrame(yuvframe)
                    }
                    FrameType::RGB => {
                        let rgbframe = pixelformat::create_rgb_frame(sample).unwrap();
                        Frame::RGB(rgbframe)
                    }
                    FrameType::BGR0 => {
                        let bgrframe = pixelformat::create_bgr_frame(sample).unwrap();
                        Frame::BGR0(bgrframe)
                    }
                    FrameType::BGRAFrame => {
                        let bgraframe = pixelformat::create_bgra_frame(sample).unwrap();
                        Frame::BGRA(bgraframe)
                    }
                });
            },
            // 空闲状态，返回空帧
            SCFrameStatus::Idle => {
                if let FrameType::BGRAFrame = output_type {
                    return Some(Frame::BGRA(BGRAFrame {
                        display_time: get_pts_in_nanoseconds(&sample),
                        width: 0,
                        height: 0,
                        data: vec![],
                    }));
                }
            }
            _ => {}
        }
    }

    None
}
