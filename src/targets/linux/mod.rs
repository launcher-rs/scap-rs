/// Linux/FreeBSD 平台目标管理实现
/// 使用 X11 和 Wayland 协议枚举显示器和窗口

#[cfg(not(any(feature = "wayland", feature = "x11")))]
compile_error!("必须启用 'wayland' 或 'x11' 特性之一。");

#[cfg(feature = "x11")]
use std::ffi::{CStr, CString, NulError, c_char};

use super::{Display, Target};

#[cfg(feature = "x11")]
use anyhow::Context as _;
use anyhow::anyhow;

/// X11 相关导入
#[cfg(feature = "x11")]
use x11::xlib::{XFreeStringList, XGetTextProperty, XTextProperty, XmbTextPropertyToTextList};
/// XCB 相关导入（用于 X11 协议通信）
#[cfg(feature = "x11")]
use xcb::{
    Xid,
    randr::{GetCrtcInfo, GetOutputInfo, GetOutputPrimary, GetScreenResources},
    x::{self, GetPropertyReply, Screen},
};

/// 获取 X11 原子（Atom）
/// 用于获取 X11 属性的原子标识符
#[cfg(feature = "x11")]
fn get_atom(conn: &xcb::Connection, atom_name: &str) -> Result<x::Atom, xcb::Error> {
    let cookie = conn.send_request(&x::InternAtom {
        only_if_exists: true,
        name: atom_name.as_bytes(),
    });
    Ok(conn.wait_for_reply(cookie)?.atom())
}

/// 获取 X11 窗口属性
/// 用于读取窗口的各种属性信息
#[cfg(feature = "x11")]
fn get_property(
    conn: &xcb::Connection,
    win: x::Window,
    prop: x::Atom,
    typ: x::Atom,
    length: u32,
) -> Result<GetPropertyReply, xcb::Error> {
    let cookie = conn.send_request(&x::GetProperty {
        delete: false,
        window: win,
        property: prop,
        r#type: typ,
        long_offset: 0,
        long_length: length,
    });
    Ok(conn.wait_for_reply(cookie)?)
}

/// 解码复合文本（Compound Text）
/// X11 支持的多字节文本编码格式
#[cfg(feature = "x11")]
fn decode_compound_text(
    conn: &xcb::Connection,
    value: &[u8],
    client: &xcb::x::Window,
    ttype: xcb::x::Atom,
) -> Result<String, NulError> {
    let display = conn.get_raw_dpy();
    assert!(!display.is_null());

    // 创建 C 字符串
    let c_string = CString::new(value.to_vec())?;
    let mut text_prop = XTextProperty {
        value: std::ptr::null_mut(),
        encoding: 0,
        format: 0,
        nitems: 0,
    };

    // 获取文本属性
    let res = unsafe {
        XGetTextProperty(
            display,
            client.resource_id() as u64,
            &mut text_prop,
            x::ATOM_WM_NAME.resource_id() as u64,
        )
    };
    if res == 0 || text_prop.nitems == 0 {
        return Ok(String::from("n/a"));
    }

    // 转换文本编码
    let mut xname = XTextProperty {
        value: c_string.as_ptr() as *mut u8,
        encoding: ttype.resource_id() as u64,
        format: 8,
        nitems: text_prop.nitems,
    };
    let mut list: *mut *mut c_char = std::ptr::null_mut();
    let mut count: i32 = 0;
    let result = unsafe { XmbTextPropertyToTextList(display, &mut xname, &mut list, &mut count) };
    if result < 1 || list.is_null() || count < 1 {
        Ok(String::from("n/a"))
    } else {
        let title = unsafe { CStr::from_ptr(*list).to_string_lossy().into_owned() };
        unsafe { XFreeStringList(list) };
        Ok(title)
    }
}

/// 获取所有 X11 目标
/// 包括所有窗口和显示器
#[cfg(feature = "x11")]
fn get_x11_targets() -> Result<Vec<Target>, xcb::Error> {
    // 连接到 X11 服务器
    let (conn, _screen_num) =
        xcb::Connection::connect_with_xlib_display_and_extensions(&[xcb::Extension::RandR], &[])?;
    let setup = conn.get_setup();
    let screens = setup.roots();

    // 获取窗口管理器客户端列表原子
    let wm_client_list = get_atom(&conn, "_NET_CLIENT_LIST")?;
    assert!(wm_client_list != x::ATOM_NONE, "不支持 EWMH");

    // 获取相关原子
    let atom_net_wm_name = get_atom(&conn, "_NET_WM_NAME")?;
    let atom_text = get_atom(&conn, "TEXT")?;
    let atom_utf8_string = get_atom(&conn, "UTF8_STRING")?;
    let atom_compound_text = get_atom(&conn, "COMPOUND_TEXT")?;

    let mut targets = Vec::new();
    for screen in screens {
        // 获取窗口列表
        let window_list = get_property(&conn, screen.root(), wm_client_list, x::ATOM_NONE, 100)?;

        // 遍历所有窗口
        for client in window_list.value::<x::Window>() {
            // 尝试获取 _NET_WM_NAME 属性
            let cr = get_property(&conn, *client, atom_net_wm_name, x::ATOM_STRING, 4096)?;
            if !cr.value::<x::Atom>().is_empty() {
                targets.push(Target::Window(crate::targets::Window {
                    id: 0,
                    title: String::from_utf8(cr.value().to_vec())
                        .map_err(|_| xcb::Error::Connection(xcb::ConnError::ClosedParseErr))?,
                    raw_handle: *client,
                }));
                continue;
            }

            // 尝试获取 WM_NAME 属性
            let reply = get_property(&conn, *client, x::ATOM_WM_NAME, x::ATOM_ANY, 4096)?;
            let value: &[u8] = reply.value();
            if !value.is_empty() {
                let ttype = reply.r#type();
                // 根据文本类型解码窗口标题
                let title =
                    if ttype == x::ATOM_STRING || ttype == atom_utf8_string || ttype == atom_text {
                        String::from_utf8(reply.value().to_vec()).unwrap_or(String::from("n/a"))
                    } else if ttype == atom_compound_text {
                        decode_compound_text(&conn, value, client, ttype)
                            .map_err(|_| xcb::Error::Connection(xcb::ConnError::ClosedParseErr))?
                    } else {
                        String::from_utf8(reply.value().to_vec()).unwrap_or(String::from("n/a"))
                    };

                targets.push(Target::Window(crate::targets::Window {
                    id: 0,
                    title,
                    raw_handle: *client,
                }));
                continue;
            }
            // 无法获取标题时使用默认值
            targets.push(Target::Window(crate::targets::Window {
                id: 0,
                title: String::from("n/a"),
                raw_handle: *client,
            }));
        }

        // 获取屏幕资源（显示器信息）
        let resources = conn.send_request(&GetScreenResources {
            window: screen.root(),
        });
        let resources = conn.wait_for_reply(resources)?;

        // 遍历所有输出（显示器）
        for output in resources.outputs() {
            let info = conn.send_request(&GetOutputInfo {
                output: *output,
                config_timestamp: 0,
            });
            let info = conn.wait_for_reply(info)?;

            // 只处理已连接的显示器
            if info.connection() == xcb::randr::Connection::Connected {
                let crtc = info.crtc();
                let crtc_info = conn.send_request(&GetCrtcInfo {
                    crtc,
                    config_timestamp: 0,
                });
                let crtc_info = conn.wait_for_reply(crtc_info)?;
                let title = String::from_utf8(info.name().to_vec()).unwrap_or(String::from("n/a"));
                targets.push(Target::Display(crate::targets::Display {
                    id: crtc.resource_id(),
                    title,
                    width: crtc_info.width(),
                    height: crtc_info.height(),
                    x_offset: crtc_info.x(),
                    y_offset: crtc_info.y(),
                    raw_handle: screen.root(),
                }));
            }
        }
    }

    Ok(targets)
}

/// 获取所有可捕获的目标
/// 根据环境变量检测显示服务器类型
pub fn get_all_targets() -> anyhow::Result<Vec<Target>> {
    // Wayland 需要用户交互选择目标
    #[cfg(feature = "wayland")]
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        // Wayland 下目标在 Recorder 实例化时选择
        return Ok(Vec::new());
    }

    // X11 可以直接枚举目标
    #[cfg(feature = "x11")]
    if std::env::var("DISPLAY").is_ok() {
        return Ok(get_x11_targets()?);
    }

    // 根据启用的特性返回相应的错误信息
    #[cfg(all(feature = "wayland", feature = "x11"))]
    let error_msg = "不支持的平台：无法检测到 Wayland 或 X11 显示器";
    #[cfg(all(not(feature = "wayland"), feature = "x11"))]
    let error_msg = "不支持的平台：无法检测到 X11 显示器。请启用 'wayland' 特性以支持 Wayland。";
    #[cfg(all(feature = "wayland", not(feature = "x11")))]
    let error_msg = "不支持的平台：无法检测到 Wayland 显示器。请启用 'x11' 特性以支持 X11。";

    Err(anyhow!(error_msg))
}

/// 获取默认 X11 显示器
/// 返回主显示器的详细信息
#[cfg(feature = "x11")]
pub(crate) fn get_default_x_display(
    conn: &xcb::Connection,
    screen: &Screen,
) -> Result<Display, xcb::Error> {
    // 获取主输出
    let primary_display_cookie = conn.send_request(&GetOutputPrimary {
        window: screen.root(),
    });
    let primary_display = conn.wait_for_reply(primary_display_cookie)?;

    // 获取输出信息
    let info_cookie = conn.send_request(&GetOutputInfo {
        output: primary_display.output(),
        config_timestamp: 0,
    });
    let info = conn.wait_for_reply(info_cookie)?;

    // 获取 CRTC 信息
    let crtc = info.crtc();
    let crtc_info_cookie = conn.send_request(&GetCrtcInfo {
        crtc,
        config_timestamp: 0,
    });
    let crtc_info = conn.wait_for_reply(crtc_info_cookie)?;

    Ok(Display {
        id: crtc.resource_id(),
        title: String::from_utf8(info.name().to_vec()).unwrap_or(String::from("default")),
        width: crtc_info.width(),
        height: crtc_info.height(),
        x_offset: crtc_info.x(),
        y_offset: crtc_info.y(),
        raw_handle: screen.root(),
    })
}

/// 获取主显示器信息
/// 返回系统主显示器的详细信息
pub fn get_main_display() -> anyhow::Result<Display> {
    // Wayland 下暂不支持获取主显示器
    #[cfg(feature = "wayland")]
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return Err(anyhow!("Wayland 下暂不支持获取主显示器。"));
    }

    // X11 下获取默认显示器
    #[cfg(feature = "x11")]
    if std::env::var("DISPLAY").is_ok() {
        let (conn, screen_num) =
            xcb::Connection::connect_with_extensions(None, &[xcb::Extension::RandR], &[]).unwrap();
        let setup = conn.get_setup();
        let screen = setup.roots().nth(screen_num as usize).unwrap();
        return get_default_x_display(&conn, screen).context("获取主 X11 显示器失败");
    }

    // 根据启用的特性返回相应的错误信息
    #[cfg(all(feature = "wayland", feature = "x11"))]
    let error_msg = "不支持的平台：无法检测到 Wayland 或 X11 显示器";
    #[cfg(all(not(feature = "wayland"), feature = "x11"))]
    let error_msg = "不支持的平台：无法检测到 X11 显示器。请启用 'wayland' 特性以支持 Wayland。";
    #[cfg(all(feature = "wayland", not(feature = "x11")))]
    let error_msg = "不支持的平台：无法检测到 Wayland 显示器。请启用 'x11' 特性以支持 X11。";

    Err(anyhow!(error_msg))
}

/// 获取目标的像素尺寸
/// 窗口返回 (0, 0)（待实现），显示器返回实际尺寸
pub fn get_target_dimensions(target: &Target) -> (u64, u64) {
    match target {
        Target::Window(_w) => (0, 0), // TODO: 实现窗口尺寸获取
        Target::Display(d) => (d.width as u64, d.height as u64),
    }
}
