use scap_rs::{
    capturer::{Area, Capturer, Options, Point, Size},
    frame::Frame,
};

fn main() {
    // 检查平台是否支持
    if !scap_rs::is_supported() {
        println!("当前平台不支持屏幕捕获");
        return;
    }

    // 检查是否有屏幕录制权限
    if !scap_rs::has_permission() {
        println!("未获得权限，正在请求权限...");
        if !scap_rs::request_permission() {
            println!("权限被拒绝");
            return;
        }
    }

    // 创建捕获选项
    let options = Options {
        fps: 60,
        show_cursor: true,
        show_highlight: true,
        excluded_targets: None,
        output_type: scap_rs::frame::FrameType::BGRAFrame,
        output_resolution: scap_rs::capturer::Resolution::_720p,
        crop_area: Some(Area {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: 500.0,
                height: 500.0,
            },
        }),
        ..Default::default()
    };

    // 创建捕获器
    let mut capturer = Capturer::build(options).unwrap_or_else(|err| {
        println!("创建捕获器时出错: {err}");
        std::process::exit(1);
    });

    // 开始捕获
    capturer.start_capture();
    println!("开始捕获屏幕...");

    // 捕获 10 帧
    for i in 0..10 {
        match capturer.get_next_frame() {
            Ok(frame) => match frame {
                Frame::BGRA(frame) => {
                    println!("帧 {}: {}x{}", i, frame.width, frame.height);
                }
                Frame::RGB(frame) => {
                    println!("帧 {}: {}x{}", i, frame.width, frame.height);
                }
                _ => {
                    println!("帧 {}: 其他格式", i);
                }
            },
            Err(e) => {
                println!("获取帧失败: {}", e);
                break;
            }
        }
    }

    // 停止捕获
    capturer.stop_capture();
    println!("捕获完成");
}
