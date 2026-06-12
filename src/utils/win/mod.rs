/// Windows 平台支持检测模块
/// 使用 Graphics Capture API 检查平台支持
use windows_capture::graphics_capture_api::GraphicsCaptureApi;

/// 检查当前系统是否支持屏幕捕获
/// 需要 Windows 10 版本 1903 或更高版本
pub fn is_supported() -> bool {
    GraphicsCaptureApi::is_supported().expect("检查支持状态失败")
}
