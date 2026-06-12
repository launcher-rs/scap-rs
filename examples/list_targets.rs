use scap_rs::Target;

fn main() {
    // 检查平台是否支持
    if !scap_rs::is_supported() {
        println!("当前平台不支持屏幕捕获");
        return;
    }

    // 检查权限
    if !scap_rs::has_permission() {
        println!("未获得屏幕捕获权限");
        println!("请先运行 permission_check 示例获取权限");
        return;
    }

    // 获取所有可捕获目标
    let targets = scap_rs::get_all_targets().unwrap_or_else(|err| {
        println!("获取目标列表失败: {}", err);
        std::process::exit(1);
    });

    println!("找到 {} 个可捕获目标:\n", targets.len());

    for (i, target) in targets.iter().enumerate() {
        match target {
            Target::Display(display) => {
                println!("{}: 显示器 - {}", i + 1, display.title);
                println!("   ID: {:?}", display.id);
            }
            Target::Window(window) => {
                println!("{}: 窗口 - {}", i + 1, window.title);
                println!("   ID: {:?}", window.id);
            }
        }
        println!();
    }

    // 示例：选择特定目标进行捕获
    if let Some(target) = targets.first() {
        println!("示例：使用第一个目标进行捕获");
        match target {
            Target::Display(display) => {
                println!("将捕获显示器: {}", display.title);
            }
            Target::Window(window) => {
                println!("将捕获窗口: {}", window.title);
            }
        }
    }
}
