// 本程序是一个测试应用程序，用于演示屏幕捕获库的使用方法
// 参考 `lib.rs` 获取库源代码

use std::process;
use scap_rs::{
    capturer::{Area, Capturer, Options, Point, Size},
    frame::Frame,
};

fn main() {
    // 检查当前平台是否支持屏幕捕获
    if !scap_rs::is_supported() {
        println!("❌ 当前平台不支持屏幕捕获");
        return;
    }

    // 检查是否有屏幕捕获权限
    // 如果没有权限，则请求用户授权
    if !scap_rs::has_permission() {
        println!("❌ 未获得权限，正在请求权限...");
        if !scap_rs::request_permission() {
            println!("❌ 权限被拒绝");
            return;
        }
    }

    // // 获取所有可捕获的目标（显示器、窗口等）
    // let targets = scap_rs::get_all_targets();

    // 创建捕获选项配置
    let options = Options {
        fps: 60,                    // 帧率：60 FPS
        show_cursor: true,          // 显示鼠标光标
        show_highlight: true,       // 显示高亮效果
        excluded_targets: None,     // 不排除任何目标
        output_type: scap_rs::frame::FrameType::BGRAFrame,  // 输出帧格式：BGRA
        output_resolution: scap_rs::capturer::Resolution::_720p,  // 输出分辨率：720p
        crop_area: Some(Area {      // 裁剪区域：500x500 像素
            origin: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: 500.0,
                height: 500.0,
            },
        }),
        ..Default::default()        // 其他选项使用默认值
    };

    // 使用配置选项创建屏幕捕获器
    let mut recorder = Capturer::build(options).unwrap_or_else(|err| {
        println!("创建捕获器时出错: {err}");
        process::exit(1);
    });

    // 开始屏幕捕获
    recorder.start_capture();

    // 捕获 100 帧图像
    let mut start_time: u64 = 0;
    for i in 0..100 {
        // 获取下一帧图像
        let frame = recorder.get_next_frame().expect("获取帧失败");

        // 根据帧类型进行处理和显示
        match frame {
            Frame::YUVFrame(frame) => {
                println!(
                    "收到 YUV 帧 {}，宽度 {}，高度 {}，显示时间 {}",
                    i, frame.width, frame.height, frame.display_time
                );
            }
            Frame::BGR0(frame) => {
                println!(
                    "收到 BGR0 帧，宽度 {}，高度 {}",
                    frame.width, frame.height
                );
            }
            Frame::RGB(frame) => {
                if start_time == 0 {
                    start_time = frame.display_time;
                }
                println!(
                    "收到 RGB 帧 {}，宽度 {}，高度 {}，相对时间 {}",
                    i,
                    frame.width,
                    frame.height,
                    frame.display_time - start_time
                );
            }
            Frame::RGBx(frame) => {
                println!(
                    "收到 RGBx 帧，宽度 {}，高度 {}",
                    frame.width, frame.height
                );
            }
            Frame::XBGR(frame) => {
                println!(
                    "收到 XBGR 帧，宽度 {}，高度 {}",
                    frame.width, frame.height
                );
            }
            Frame::BGRx(frame) => {
                println!(
                    "收到 BGRx 帧，宽度 {}，高度 {}",
                    frame.width, frame.height
                );
            }
            Frame::BGRA(frame) => {
                if start_time == 0 {
                    start_time = frame.display_time;
                }
                println!(
                    "收到 BGRA 帧 {}，宽度 {}，高度 {}，相对时间 {}",
                    i,
                    frame.width,
                    frame.height,
                    frame.display_time - start_time
                );
            }
        }
    }

    // 停止屏幕捕获
    recorder.stop_capture();
}
