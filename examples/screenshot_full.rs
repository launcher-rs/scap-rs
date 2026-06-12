use scap_rs::{
    capturer::{Capturer, Options},
    frame::Frame,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // 检查平台是否支持
    if !scap_rs::is_supported() {
        println!("当前平台不支持屏幕捕获");
        return;
    }

    // 检查权限
    if !scap_rs::has_permission() {
        println!("未获得权限，正在请求权限...");
        if !scap_rs::request_permission() {
            println!("权限被拒绝");
            return;
        }
    }

    // 创建捕获选项 - 全屏捕获，不设置 crop_area
    let options = Options {
        fps: 1, // 截图只需 1 帧
        show_cursor: true,
        show_highlight: false,
        excluded_targets: None,
        output_type: scap_rs::frame::FrameType::BGRAFrame,
        output_resolution: scap_rs::capturer::Resolution::Captured, // 原始分辨率
        crop_area: None,                                            // None 表示全屏
        ..Default::default()
    };

    // 创建捕获器
    let mut capturer = Capturer::build(options).unwrap_or_else(|err| {
        println!("创建捕获器时出错: {err}");
        std::process::exit(1);
    });

    // 开始捕获
    capturer.start_capture();
    println!("正在截取全屏...");

    // 获取一帧
    match capturer.get_next_frame() {
        Ok(frame) => {
            match frame {
                Frame::BGRA(bgra) => {
                    // 生成文件名（使用时间戳）
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let filename = format!("screenshot_full_{}.png", timestamp);

                    // BGRA 转换为 RGBA 用于保存
                    let mut rgba_data = Vec::with_capacity(bgra.data.len());
                    for chunk in bgra.data.chunks_exact(4) {
                        rgba_data.push(chunk[2]); // R
                        rgba_data.push(chunk[1]); // G
                        rgba_data.push(chunk[0]); // B
                        rgba_data.push(chunk[3]); // A
                    }

                    // 保存为 PNG
                    match image::save_buffer(
                        &filename,
                        &rgba_data,
                        bgra.width as u32,
                        bgra.height as u32,
                        image::ColorType::Rgba8,
                    ) {
                        Ok(()) => println!("全屏截图已保存: {}", filename),
                        Err(e) => println!("保存截图失败: {}", e),
                    }
                }
                Frame::RGB(rgb) => {
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let filename = format!("screenshot_full_{}.png", timestamp);

                    match image::save_buffer(
                        &filename,
                        &rgb.data,
                        rgb.width as u32,
                        rgb.height as u32,
                        image::ColorType::Rgb8,
                    ) {
                        Ok(()) => println!("全屏截图已保存: {}", filename),
                        Err(e) => println!("保存截图失败: {}", e),
                    }
                }
                _ => {
                    println!("不支持的帧格式");
                }
            }
        }
        Err(e) => {
            println!("获取帧失败: {}", e);
        }
    }

    // 停止捕获
    capturer.stop_capture();
}
