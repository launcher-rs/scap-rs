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

## 跨平台检查

**问题背景**：Windows 下 `cargo check` 只编译 `#[cfg(target_os = "windows")]` 和通用代码，`#[cfg(target_os = "macos")]` 和 `#[cfg(target_os = "linux")]` 中的代码不会被编译。合并上游 PR 后容易把 Linux/macOS 代码弄坏。

### 本地跨目标检查

安装目标：
```bash
rustup target add x86_64-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin
```

跨平台检查（只检查 Rust 语法和类型，不需要真机）：
```bash
# 检查 Linux 代码
cargo check --target x86_64-unknown-linux-gnu

# 检查 macOS Intel 代码
cargo check --target x86_64-apple-darwin

# 检查 macOS ARM 代码
cargo check --target aarch64-apple-darwin
```

**注意**：由于平台绑定的原生依赖（windows-rs、objc2、tree-sitter 的 C 库、psm/stacker），跨目标检查在 Windows 上几乎不可行——缺少对应平台的 C 编译器（如 x86_64-linux-gnu-gcc）。真正的跨平台验证依赖 CI（GitHub Actions 矩阵构建）。

### Feature 组合检查

使用 cargo-hack 验证所有 feature 组合：
```bash
cargo hack check --each-feature --workspace --ignore-private
```

### GitHub Actions CI

项目配置了 CI（`.github/workflows/ci.yml`），在 push/PR 时自动执行：

| 任务 | 平台 | 内容 |
|------|------|------|
| check | windows-latest / ubuntu-latest / macos-latest | `cargo check --workspace` + `cargo test --workspace` + `cargo clippy` |
| feature-check | ubuntu-latest | `cargo hack check --each-feature`（排除已知故障的 scap/screen-capture feature） |
| lint | ubuntu-latest | `cargo fmt --all --check` |

CI 通过矩阵策略在三个平台分别运行，确保跨平台兼容性。

## 依赖版本

| 依赖 | 版本 |
|------|------|
| windows-capture | 2.0 |
| windows | 0.62.2 |
| screencapturekit | 0.2.8 |
| screencapturekit-sys | 0.2.8 |
| cocoa | 0.26.1 |
| xcb | 1.7.0 |
| pipewire | 0.10.0 |
| dbus | 0.9.11 |

## 测试

无测试套件。这是平台依赖库，测试需要主机操作系统上的实际屏幕捕获权限。
