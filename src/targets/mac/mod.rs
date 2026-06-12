/// macOS 平台目标管理实现
/// 使用 Cocoa 和 Core Graphics API 枚举显示器和窗口

#![allow(unexpected_cfgs)]

use anyhow::{Context as _, Result};
use cocoa::appkit::{NSApp, NSScreen};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSRect, NSString, NSUInteger};
use core_graphics_helmer_fork::display::{CGDirectDisplayID, CGDisplay, CGMainDisplayID};
use core_graphics_helmer_fork::window::CGWindowID;
use objc::{msg_send, sel, sel_impl};
use screencapturekit::sc_shareable_content::SCShareableContent;

use super::{Display, Target};

/// 获取显示器名称
/// 通过 NSScreen API 获取显示器的本地化名称
fn get_display_name(display_id: CGDirectDisplayID) -> String {
    unsafe {
        // 获取所有屏幕
        let screens: id = NSScreen::screens(nil);
        let count: u64 = msg_send![screens, count];

        // 遍历屏幕查找匹配的显示器
        for i in 0..count {
            let screen: id = msg_send![screens, objectAtIndex: i];
            let device_description: id = msg_send![screen, deviceDescription];
            let display_id_number: id = msg_send![device_description, objectForKey: NSString::alloc(nil).init_str("NSScreenNumber")];
            let display_id_number: u32 = msg_send![display_id_number, unsignedIntValue];

            // 找到匹配的显示器，返回其名称
            if display_id_number == display_id {
                let localized_name: id = msg_send![screen, localizedName];
                let name: *const i8 = msg_send![localized_name, UTF8String];
                return std::ffi::CStr::from_ptr(name)
                    .to_string_lossy()
                    .into_owned();
            }
        }

        // 未找到则返回未知显示器
        format!("未知显示器 {}", display_id)
    }
}

/// 获取所有可捕获的目标
/// 包括所有显示器和窗口
pub fn get_all_targets() -> Result<Vec<Target>> {
    let mut targets: Vec<Target> = Vec::new();

    // 获取可共享内容（显示器和窗口）
    let content = SCShareableContent::current();

    // 添加显示器到目标列表
    for display in content.displays {
        let id: CGDirectDisplayID = display.display_id;
        let raw_handle = CGDisplay::new(id);
        let title = get_display_name(id);

        let target = Target::Display(super::Display {
            id,
            title,
            raw_handle,
        });

        targets.push(target);
    }

    // 添加窗口到目标列表（跳过没有标题的窗口）
    for window in content.windows {
        if window.title.is_some() {
            let id = window.window_id;
            let title = window.title.context("未找到窗口标题")?;
            let raw_handle: CGWindowID = id;

            let target = Target::Window(super::Window {
                id,
                title,
                raw_handle,
            });
            targets.push(target);
        }
    }

    Ok(targets)
}

/// 获取主显示器信息
/// 返回系统主显示器的详细信息
pub fn get_main_display() -> Result<Display> {
    let id = unsafe { CGMainDisplayID() };
    let title = get_display_name(id);

    Ok(Display {
        id,
        title,
        raw_handle: CGDisplay::new(id),
    })
}

/// 获取目标的缩放因子
/// 窗口使用 backingScaleFactor，显示器使用像素/点比例
pub fn get_scale_factor(target: &Target) -> f64 {
    match target {
        Target::Window(window) => unsafe {
            let cg_win_id = window.raw_handle;
            let ns_app: id = NSApp();
            let ns_window: id = msg_send![ns_app, windowWithWindowNumber: cg_win_id as NSUInteger];
            let scale_factor: f64 = msg_send![ns_window, backingScaleFactor];
            scale_factor
        },
        Target::Display(display) => {
            // 计算像素宽度与点宽度的比例
            let mode = display.raw_handle.display_mode().unwrap();
            (mode.pixel_width() / mode.width()) as f64
        }
    }
}

/// 获取目标的像素尺寸
/// 窗口返回框架大小，显示器返回屏幕分辨率
pub fn get_target_dimensions(target: &Target) -> (u64, u64) {
    match target {
        Target::Window(window) => unsafe {
            let cg_win_id = window.raw_handle;
            let ns_app: id = NSApp();
            let ns_window: id = msg_send![ns_app, windowWithWindowNumber: cg_win_id as NSUInteger];
            let frame: NSRect = msg_send![ns_window, frame];
            (frame.size.width as u64, frame.size.height as u64)
        },
        Target::Display(display) => {
            // 获取显示器显示模式
            let mode = display.raw_handle.display_mode().unwrap();
            (mode.width(), mode.height())
        }
    }
}
