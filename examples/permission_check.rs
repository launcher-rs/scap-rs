fn main() {
    println!("=== 屏幕捕获权限检查 ===\n");

    // 检查平台支持
    println!("1. 检查平台支持...");
    if scap_rs::is_supported() {
        println!("   ✓ 当前平台支持屏幕捕获\n");
    } else {
        println!("   ✗ 当前平台不支持屏幕捕获\n");
        return;
    }

    // 检查权限状态
    println!("2. 检查权限状态...");
    if scap_rs::has_permission() {
        println!("   ✓ 已获得屏幕捕获权限\n");
    } else {
        println!("   ✗ 未获得屏幕捕获权限\n");

        // 请求权限
        println!("3. 正在请求权限...");
        if scap_rs::request_permission() {
            println!("   ✓ 权限请求成功\n");
        } else {
            println!("   ✗ 权限请求失败或被拒绝\n");
            println!("请在系统设置中手动授予屏幕录制权限:");
            println!("  - macOS: 系统设置 > 隐私与安全性 > 屏幕录制");
            println!("  - Windows: 设置 > 隐私 > 屏幕截图");
            println!("  - Linux: 根据桌面环境不同，权限设置位置也不同");
            return;
        }
    }

    // 获取可捕获目标数量
    println!("4. 获取可捕获目标...");
    let targets = scap_rs::get_all_targets().unwrap_or_else(|err| {
        println!("   获取目标列表失败: {}", err);
        std::process::exit(1);
    });
    println!("   找到 {} 个可捕获目标\n", targets.len());

    // 列出部分目标
    let display_count = targets
        .iter()
        .filter(|t| matches!(t, scap_rs::Target::Display(_)))
        .count();
    let window_count = targets
        .iter()
        .filter(|t| matches!(t, scap_rs::Target::Window(_)))
        .count();
    println!("   显示器: {} 个", display_count);
    println!("   窗口: {} 个", window_count);

    println!("\n=== 权限检查完成 ===");
    println!("现在可以运行 basic_capture 示例进行屏幕捕获");
}
