// Linux/FreeBSD 平台工具函数实现
// 注意：这些函数目前是简单的占位符实现

// TODO: 实现实际的平台支持检测
/// 检查当前系统是否支持屏幕捕获
/// 目前始终返回 true
pub fn is_supported() -> bool {
    true
    // false
}

// TODO: 实现实际的权限检查
/// 检查是否有屏幕捕获权限
/// 目前始终返回 true
#[allow(dead_code)]
pub fn has_permission() -> bool {
    true
    // false
}
