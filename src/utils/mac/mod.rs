/// macOS 平台工具函数实现
/// 处理权限检查和平台支持检测
use core_graphics_helmer_fork::access::ScreenCaptureAccess;
use sysinfo::System;

/// 检查是否有屏幕捕获权限
/// 使用 ScreenCaptureAccess 进行预检查
pub fn has_permission() -> bool {
    ScreenCaptureAccess.preflight()
}

/// 请求屏幕捕获权限
/// 弹出系统授权对话框
pub fn request_permission() -> bool {
    ScreenCaptureAccess.request()
}

/// 检查当前系统是否支持屏幕捕获
/// 需要 macOS 12.3 或更高版本
pub fn is_supported() -> bool {
    // 获取当前 macOS 版本
    let os_version = System::os_version()
        .expect("获取 macOS 版本失败")
        .as_bytes()
        .to_vec();

    // 最低支持版本
    let min_version: Vec<u8> = "12.3\n".as_bytes().to_vec();

    // 比较版本号
    os_version >= min_version
}
