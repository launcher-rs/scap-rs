# AGENTS.md - scap-rs

## 项目概述

跨平台屏幕捕获库，基于原生操作系统 API 实现。使用 ScreenCaptureKit (macOS)、Windows.Graphics.Capture (Windows)、PipeWire/XCB (Linux)。


## 构建命令

```bash
cargo build                    # 构建库 + 二进制文件
cargo build --examples         # 构建所有示例
cargo run --example basic_capture   # 运行指定示例
cargo run --example screenshot_full   # 全屏截图并保存
cargo run --example screenshot_crop   # 指定区域截图并保存
```

**平台特定构建**：许多依赖项受平台限制。Linux 上需要安装 `pipewire` 和 `xcb` 开发库。

**特性**：`wayland` 和 `x11` 是仅限 Linux 的特性，默认启用。如不针对 Linux，使用 `--no-default-features` 禁用。

## 代码结构

```
src/
├── lib.rs          # 库入口，重新导出公共 API
├── main.rs         # 测试/演示二进制文件
├── capturer/       # 核心捕获逻辑
│   ├── mod.rs      # Capturer 结构体、Options、Resolution、Area 类型
│   └── engine/     # 平台特定实现
│       ├── win/
│       ├── mac/
│       └── linux/
├── frame/          # 帧类型 (BGRA, RGB, YUV 等)
├── targets/        # 显示器/窗口枚举
│   ├── win/
│   ├── mac/
│   └── linux/
└── utils/          # 权限检查、平台支持检测
```

## 关键 API 模式

1. **先检查支持**：`scap_rs::is_supported()`
2. **检查/请求权限**：`scap_rs::has_permission()`、`scap_rs::request_permission()`
3. **使用 `Capturer::build(options)`** - `new()` 方法已弃用
4. **帧循环**：`start_capture()` → 循环 `get_next_frame()` → `stop_capture()`

## 测试

无测试套件。这是平台依赖库，测试需要主机操作系统上的实际屏幕捕获权限。
