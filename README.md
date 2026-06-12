## scap-rs

一个基于原生操作系统 API 实现高性能屏幕捕获的 Rust 库！

1. macOS: [ScreenCaptureKit](https://developer.apple.com/documentation/screencapturekit)
2. Windows: [Windows.Graphics.Capture](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture?view=winrt-22621)
3. Linux: [Pipewire](https://pipewire.org)

---

## 特性

1. 跨平台支持 Windows、macOS 和 Linux！
2. 检查平台支持和录制权限。
3. 查询可捕获目标列表（显示器和窗口）。
4. 排除特定目标不被录制。


## 使用方法

```rust
use scap_rs::{
    capturer::{Point, Area, Size, Capturer, Options},
    frame::Frame,
};

fn main() {
    // 检查平台是否支持
    if !scap_rs::is_supported() {
        println!("❌ 平台不支持");
        return;
    }

    // 检查是否有屏幕录制权限
    // 如果没有，则请求权限
    if !scap_rs::has_permission() {
        println!("❌ 未授予权限，正在请求权限...");
        if !scap_rs::request_permission() {
            println!("❌ 权限被拒绝");
            return;
        }
    }

    // 获取录制目标
    let targets = scap_rs::get_all_targets();
    println!("目标: {:?}", targets);

    // 所有显示器和窗口都是目标
    // 你可以过滤并选择需要录制的目标

    // 创建选项
    let options = Options {
        fps: 60,
        target: None, // None 表示捕获主显示器
        show_cursor: true,
        show_highlight: true,
        excluded_targets: None,
        output_type: scap_rs::frame::FrameType::BGRAFrame,
        output_resolution: scap_rs::capturer::Resolution::_720p,
        source_rect: Some(Area {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: 2000.0,
                height: 1000.0,
            },
        }),
        ..Default::default()
    };

    // 创建录制器
    let mut capturer = Capturer::new(options);

    // 开始录制
    capturer.start_capture();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    // 停止录制
    capturer.stop_capture();
}
```

## 许可证

本仓库代码基于 MIT 许可证开源，但可能依赖其他不同许可证的依赖项。请查阅相关文档了解具体条款。
