/// macOS 平台工具函数模块
#[cfg(target_os = "macos")]
mod mac;

/// Windows 平台工具函数模块
#[cfg(target_os = "windows")]
mod win;

/// Linux/FreeBSD 平台工具函数模块
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
mod linux;

/// 检查当前进程是否有屏幕捕获权限
/// macOS 需要明确授权，Windows/Linux 默认返回 true
pub fn has_permission() -> bool {
    #[cfg(target_os = "macos")]
    return mac::has_permission();

    #[cfg(target_os = "windows")]
    return true;  // Windows 默认有权限

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    return true;  // Linux/FreeBSD 默认有权限
}

/// 请求用户授予屏幕捕获权限
/// macOS 会弹出系统授权对话框
pub fn request_permission() -> bool {
    #[cfg(target_os = "macos")]
    return mac::request_permission();

    // Windows 假设已授权
    #[cfg(target_os = "windows")]
    return true;

    // TODO: 检查 Linux 是否有权限系统
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    return true;
}

/// 检查当前系统是否支持屏幕捕获
/// 不同平台有不同的支持级别和要求
pub fn is_supported() -> bool {
    #[cfg(target_os = "macos")]
    return mac::is_supported();

    #[cfg(target_os = "windows")]
    return win::is_supported();

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    return linux::is_supported();
}
