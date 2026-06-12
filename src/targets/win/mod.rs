/// Windows 平台目标管理实现
/// 使用 Windows API 枚举显示器和窗口
use super::{Display, Target};
use anyhow::{Context as _, Result};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI};
use windows::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::HMONITOR,
};
use windows_capture::{
    monitor::{Error as MonitorError, Monitor},
    window::Window,
};

/// 获取所有可捕获的目标
/// 包括所有显示器和窗口
pub fn get_all_targets() -> Result<Vec<Target>> {
    let mut targets: Vec<Target> = Vec::new();

    // 枚举所有显示器
    let displays = Monitor::enumerate().context("枚举显示器失败")?;
    for display in displays {
        let id = display.as_raw_hmonitor() as u32;
        let title = monitor_title(&display).context("获取显示器名称失败")?;

        let target = Target::Display(super::Display {
            id,
            title,
            raw_handle: HMONITOR(display.as_raw_hmonitor()),
            width: display.width()? as u16,
            height: display.height()? as u16,
        });
        targets.push(target);
    }

    // 枚举所有窗口
    let windows = Window::enumerate().context("枚举窗口失败")?;
    for window in windows {
        let id = window.as_raw_hwnd() as u32;
        let title = window
            .title()
            .context("未找到窗口标题")?
            .to_string();

        let target = Target::Window(super::Window {
            id,
            title,
            raw_handle: HWND(window.as_raw_hwnd()),
        });
        targets.push(target);
    }

    Ok(targets)
}

/// 获取主显示器信息
/// 返回系统主显示器的详细信息
pub fn get_main_display() -> Result<Display> {
    let display = Monitor::primary().context("获取主显示器失败")?;
    let id = display.as_raw_hmonitor() as u32;

    Ok(Display {
        id,
        title: monitor_title(&display).context("获取显示器名称失败")?,
        raw_handle: HMONITOR(display.as_raw_hmonitor()),
        width: display.width()? as u16,
        height: display.height()? as u16,
    })
}

/// 获取显示器标题
/// 尝试多种方式获取显示器名称
fn monitor_title(monitor: &Monitor) -> Result<String, MonitorError> {
    monitor
        .name()
        .or_else(|_| monitor.device_string())
        .or_else(|_| monitor.device_name())
}

/// 获取目标的缩放因子
/// 基于 DPI 计算缩放比例
/// 参考：https://github.com/tauri-apps/tao/blob/ab792dbd6c5f0a708c818b20eaff1d9a7534c7c1/src/platform_impl/windows/dpi.rs#L50
pub fn get_scale_factor(target: &Target) -> f64 {
    const BASE_DPI: u32 = 96;  // 基准 DPI

    let mut dpi_x = 0;
    let mut dpi_y = 0;

    // 根据目标类型获取 DPI
    let dpi = match target {
        Target::Window(window) => unsafe { GetDpiForWindow(window.raw_handle) },
        Target::Display(display) => unsafe {
            if GetDpiForMonitor(
                display.raw_handle,
                MDT_EFFECTIVE_DPI,
                &mut dpi_x,
                &mut dpi_y,
            )
            .is_ok()
            {
                dpi_x
            } else {
                BASE_DPI  // 获取失败时使用基准 DPI
            }
        },
    };

    // 计算缩放因子
    let scale_factor = dpi as f64 / BASE_DPI as f64;
    scale_factor as f64
}

/// 获取目标的像素尺寸
/// 窗口返回客户端区域大小，显示器返回屏幕大小
pub fn get_target_dimensions(target: &Target) -> (u64, u64) {
    match target {
        Target::Window(window) => unsafe {
            let hwnd = window.raw_handle;

            // 获取窗口矩形区域
            let mut rect = RECT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;

            (width as u64, height as u64)
        },
        Target::Display(display) => {
            // 获取显示器对象
            let monitor = Monitor::from_raw_hmonitor(display.raw_handle.0);

            (
                monitor.width().unwrap() as u64,
                monitor.height().unwrap() as u64,
            )
        }
    }
}
