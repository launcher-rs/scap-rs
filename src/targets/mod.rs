/// macOS 平台目标管理模块
#[cfg(target_os = "macos")]
mod mac;

/// Windows 平台目标管理模块
#[cfg(target_os = "windows")]
mod win;

use anyhow::Result;

/// Linux/FreeBSD 平台目标管理模块
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(crate) mod linux;

/// 窗口结构体，表示一个可捕获的窗口
#[derive(Debug, Clone)]
pub struct Window {
    pub id: u32,       // 窗口 ID
    pub title: String, // 窗口标题

    /// Windows 平台原生窗口句柄
    #[cfg(target_os = "windows")]
    pub raw_handle: windows::Win32::Foundation::HWND,

    /// macOS 平台原生窗口 ID
    #[cfg(target_os = "macos")]
    pub raw_handle: core_graphics_helmer_fork::window::CGWindowID,

    /// Linux 平台 X11 窗口句柄
    #[cfg(all(any(target_os = "linux", target_os = "freebsd"), feature = "x11"))]
    pub raw_handle: xcb::x::Window,
}

/// 显示器结构体，表示一个可捕获的显示器
#[derive(Debug, Clone)]
pub struct Display {
    pub id: u32,       // 显示器 ID
    pub title: String, // 显示器名称

    /// Windows 平台原生显示器句柄
    #[cfg(target_os = "windows")]
    pub raw_handle: windows::Win32::Graphics::Gdi::HMONITOR,

    /// macOS 平台原生显示器对象
    #[cfg(target_os = "macos")]
    pub raw_handle: core_graphics_helmer_fork::display::CGDisplay,

    /// Linux 平台 X11 显示器句柄
    #[cfg(all(any(target_os = "linux", target_os = "freebsd"), feature = "x11"))]
    pub raw_handle: xcb::x::Window,

    /// 显示器宽度（像素）
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "windows"))]
    pub width: u16,

    /// 显示器高度（像素）
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "windows"))]
    pub height: u16,

    /// 显示器 X 偏移量
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub x_offset: i16,

    /// 显示器 Y 偏移量
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub y_offset: i16,
}

/// 捕获目标枚举，可以是窗口或显示器
#[derive(Debug, Clone)]
pub enum Target {
    Window(Window),   // 窗口目标
    Display(Display), // 显示器目标
}

// 为 Windows 平台的 Display 和 Window 实现 Send 和 Sync trait
// 因为 HWND 和 HMONITOR 都是线程安全的
#[cfg(target_os = "windows")]
unsafe impl Send for Display {}
#[cfg(target_os = "windows")]
unsafe impl Sync for Display {}

#[cfg(target_os = "windows")]
unsafe impl Send for Window {}
#[cfg(target_os = "windows")]
unsafe impl Sync for Window {}

/// 获取所有可捕获的目标列表
/// 包括所有显示器和窗口
pub fn get_all_targets() -> Result<Vec<Target>> {
    #[cfg(target_os = "macos")]
    return mac::get_all_targets();

    #[cfg(target_os = "windows")]
    return win::get_all_targets();

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    return linux::get_all_targets();
}

/// 获取目标的缩放因子
/// 用于处理高 DPI 显示器
#[allow(unused_variables)]
pub fn get_scale_factor(target: &Target) -> f64 {
    #[cfg(target_os = "macos")]
    return mac::get_scale_factor(target);

    #[cfg(target_os = "windows")]
    return win::get_scale_factor(target);

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    return 1.0;
}

/// 获取主显示器信息
/// 返回系统主显示器的详细信息
pub fn get_main_display() -> Result<Display> {
    #[cfg(target_os = "macos")]
    return mac::get_main_display();

    #[cfg(target_os = "windows")]
    return win::get_main_display();

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    return linux::get_main_display();
}

/// 获取目标的像素尺寸
/// 返回 (宽度, 高度) 的元组
pub fn get_target_dimensions(target: &Target) -> (u64, u64) {
    #[cfg(target_os = "macos")]
    return mac::get_target_dimensions(target);

    #[cfg(target_os = "windows")]
    return win::get_target_dimensions(target);

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    return linux::get_target_dimensions(target);
}
