/// Wayland 屏幕捕获实现
/// 使用 PipeWire 和 D-Bus 进行屏幕捕获
use std::{
    mem::size_of,
    sync::{
        atomic::{AtomicBool, AtomicU8},
        mpsc::{RecvError, SendError, Sender, SyncSender, sync_channel},
    },
    thread::JoinHandle,
    time::Duration,
};

use anyhow::{Context as _, Result, anyhow};
use pipewire as pw;
use pw::{
    context::ContextRc,
    loop_::Timeout,
    main_loop::MainLoopRc,
    properties::properties,
    spa::{
        self,
        param::{
            ParamType,
            format::{FormatProperties, MediaSubtype, MediaType},
            video::VideoFormat,
        },
        pod::{Pod, Property},
        sys::{
            SPA_META_Header, SPA_PARAM_META_size, SPA_PARAM_META_type, spa_buffer, spa_meta_header,
        },
        utils::{Direction, SpaTypes},
    },
    stream::{Stream, StreamRc, StreamState},
};

use crate::{
    capturer::Options,
    frame::{BGRxFrame, Frame, RGBFrame, RGBxFrame, XBGRFrame},
};

use self::portal::ScreenCastPortal;

use super::LinuxCapturerImpl;

/// Portal 通信模块
mod portal;

// TODO: 使用 Arc<> 移动到 Wayland 捕获器
/// 捕获器状态：0=停止，1=运行，2=停止中
static CAPTURER_STATE: AtomicU8 = AtomicU8::new(0);
/// 流状态错误标志
static STREAM_STATE_CHANGED_TO_ERROR: AtomicBool = AtomicBool::new(false);

/// 用户数据结构体，用于传递给 PipeWire 回调
#[derive(Clone)]
struct ListenerUserData {
    pub tx: Sender<Result<Frame>>,               // 帧数据发送通道
    pub format: spa::param::video::VideoInfoRaw, // 视频格式信息
}

/// 参数变更回调函数
/// 处理 PipeWire 流的格式参数变更
fn param_changed_callback(
    _stream: &Stream,
    user_data: &mut ListenerUserData,
    id: u32,
    param: Option<&Pod>,
) {
    let Some(param) = param else {
        return;
    };
    if id != pw::spa::param::ParamType::Format.as_raw() {
        return;
    }

    // 解析格式参数
    let (media_type, media_subtype) = match pw::spa::param::format_utils::parse_format(param) {
        Ok(v) => v,
        Err(_) => return,
    };

    // 只处理视频类型
    if media_type != MediaType::Video || media_subtype != MediaSubtype::Raw {
        return;
    }

    // 解析视频格式
    user_data
        .format
        .parse(param)
        // TODO: 告知库用户错误
        .expect("解析格式参数失败");
}

/// 状态变更回调函数
/// 处理 PipeWire 流的状态变更事件
fn state_changed_callback(
    _stream: &Stream,
    _user_data: &mut ListenerUserData,
    _old: StreamState,
    new: StreamState,
) {
    if let StreamState::Error(e) = new {
        log::debug!("PipeWire: 状态变更为错误({e})");
        STREAM_STATE_CHANGED_TO_ERROR.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// 获取缓冲区时间戳
/// 从 PipeWire 缓冲区元数据中提取 PTS（显示时间戳）
unsafe fn get_timestamp(buffer: *mut spa_buffer) -> i64 {
    unsafe {
        let n_metas = (*buffer).n_metas;
        if n_metas > 0 {
            let mut meta_ptr = (*buffer).metas;
            let metas_end = (*buffer).metas.wrapping_add(n_metas as usize);
            // 遍历元数据查找头部信息
            while meta_ptr != metas_end {
                if (*meta_ptr).type_ == SPA_META_Header {
                    let meta_header: &mut spa_meta_header =
                        &mut *((*meta_ptr).data as *mut spa_meta_header);
                    return meta_header.pts;
                }
                meta_ptr = meta_ptr.wrapping_add(1);
            }
            0
        } else {
            0
        }
    }
}

/// 处理回调函数
/// PipeWire 流的数据处理回调
fn process_callback(stream: &Stream, user_data: &mut ListenerUserData) {
    let buffer = unsafe { stream.dequeue_raw_buffer() };
    let frame_result = match process_callback_impl(buffer, user_data) {
        Ok(None) => None,
        Ok(Some(frame)) => Some(Ok(frame)),
        Err(err) => Some(Err(err)),
    };
    // 发送帧数据
    if let Some(frame_result) = frame_result {
        match user_data.tx.send(frame_result) {
            Ok(()) => {}
            Err(SendError(_)) => {
                log::debug!("帧接收器已释放")
            }
        }
    }
    // 重新排队缓冲区
    unsafe { stream.queue_raw_buffer(buffer) };
}

/// 处理回调实现
/// 从 PipeWire 缓冲区中提取帧数据
fn process_callback_impl(
    buffer: *mut pipewire::sys::pw_buffer,
    user_data: &mut ListenerUserData,
) -> Result<Option<Frame>> {
    if buffer.is_null() {
        return Err(anyhow!("Wayland 屏幕捕获缓冲区不足"));
    }
    let buffer = unsafe { (*buffer).buffer };
    if buffer.is_null() {
        // TODO: 与原始代码行为保持一致
        log::error!("Wayland 屏幕捕获中缓冲区指针意外为空");
        return Ok(None);
    }

    // 获取时间戳
    let timestamp = unsafe { get_timestamp(buffer) };

    let n_datas = unsafe { (*buffer).n_datas };
    if n_datas < 1 {
        return Ok(None);
    }

    // 获取帧尺寸和数据
    let frame_size = user_data.format.size();
    let frame_data: Vec<u8> = unsafe {
        std::slice::from_raw_parts(
            (*(*buffer).datas).data as *mut u8,
            (*(*buffer).datas).maxsize as usize,
        )
        .to_vec()
    };

    // 根据视频格式创建对应的帧对象
    match user_data.format.format() {
        VideoFormat::RGBx => Ok(Some(Frame::RGBx(RGBxFrame {
            display_time: timestamp as u64,
            width: frame_size.width as i32,
            height: frame_size.height as i32,
            data: frame_data,
        }))),
        VideoFormat::RGB => Ok(Some(Frame::RGB(RGBFrame {
            display_time: timestamp as u64,
            width: frame_size.width as i32,
            height: frame_size.height as i32,
            data: frame_data,
        }))),
        VideoFormat::xBGR => Ok(Some(Frame::XBGR(XBGRFrame {
            display_time: timestamp as u64,
            width: frame_size.width as i32,
            height: frame_size.height as i32,
            data: frame_data,
        }))),
        VideoFormat::BGRx => Ok(Some(Frame::BGRx(BGRxFrame {
            display_time: timestamp as u64,
            width: frame_size.width as i32,
            height: frame_size.height as i32,
            data: frame_data,
        }))),
        _ => Err(anyhow!("收到不支持的帧格式")),
    }
}

/// 启动 PipeWire 捕获器
/// 配置 PipeWire 流并返回主循环
fn start_pipewire_capturer(
    options: Options,
    tx: Sender<Result<Frame>>,
    stream_id: u32,
) -> Result<MainLoopRc> {
    // 初始化 PipeWire
    pw::init();

    // 创建主循环、上下文和核心连接
    let mainloop = MainLoopRc::new(None)?;
    let context = ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;

    let user_data = ListenerUserData {
        tx,
        format: Default::default(),
    };

    // 创建 PipeWire 流
    let stream = StreamRc::new(
        core.clone(),
        "scap",
        properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Screen",
        },
    )?;

    // 添加监听器
    let _listener = stream
        .add_local_listener_with_user_data(user_data.clone())
        .state_changed(state_changed_callback)
        .param_changed(param_changed_callback)
        .process(process_callback)
        .register()?;

    // 创建格式参数对象
    let obj = pw::spa::pod::object!(
        pw::spa::utils::SpaTypes::ObjectParamFormat,
        pw::spa::param::ParamType::EnumFormat,
        pw::spa::pod::property!(FormatProperties::MediaType, Id, MediaType::Video),
        pw::spa::pod::property!(FormatProperties::MediaSubtype, Id, MediaSubtype::Raw),
        pw::spa::pod::property!(
            FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            pw::spa::param::video::VideoFormat::RGB,
            pw::spa::param::video::VideoFormat::RGBA,
            pw::spa::param::video::VideoFormat::RGBx,
            pw::spa::param::video::VideoFormat::BGRx,
        ),
        pw::spa::pod::property!(
            FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            pw::spa::utils::Rectangle {
                // 默认尺寸
                width: 128,
                height: 128,
            },
            pw::spa::utils::Rectangle {
                // 最小尺寸
                width: 1,
                height: 1,
            },
            pw::spa::utils::Rectangle {
                // 最大尺寸
                width: 4096,
                height: 4096,
            }
        ),
        pw::spa::pod::property!(
            FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            pw::spa::utils::Fraction {
                num: options.fps,
                denom: 1
            },
            pw::spa::utils::Fraction { num: 0, denom: 1 },
            pw::spa::utils::Fraction {
                num: 1000,
                denom: 1
            }
        ),
    );

    // 创建元数据对象
    let metas_obj = pw::spa::pod::object!(
        SpaTypes::ObjectParamMeta,
        ParamType::Meta,
        Property::new(
            SPA_PARAM_META_type,
            pw::spa::pod::Value::Id(pw::spa::utils::Id(SPA_META_Header))
        ),
        Property::new(
            SPA_PARAM_META_size,
            pw::spa::pod::Value::Int(size_of::<pw::spa::sys::spa_meta_header>() as i32)
        ),
    );

    // 序列化参数
    let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )?
    .0
    .into_inner();
    let metas_values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(metas_obj),
    )?
    .0
    .into_inner();

    let mut params = [
        pw::spa::pod::Pod::from_bytes(&values).context("屏幕捕获 'values' 参数空间不足")?,
        pw::spa::pod::Pod::from_bytes(&metas_values)
            .context("屏幕捕获 'metas_values' 参数空间不足")?,
    ];

    // 连接流
    stream.connect(
        Direction::Input,
        Some(stream_id),
        pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
        &mut params,
    )?;

    Ok(mainloop)
}

/// PipeWire 捕获器线程函数
/// 运行 PipeWire 主循环并处理帧数据
fn pipewire_capturer(
    options: Options,
    tx: Sender<Result<Frame>>,
    ready_sender: &SyncSender<Result<()>>,
    stream_id: u32,
) {
    // 启动 PipeWire 捕获器
    let mainloop = match start_pipewire_capturer(options, tx, stream_id) {
        Ok(mainloop) => {
            ready_sender.send(Ok(())).ok();
            mainloop
        }
        Err(err) => {
            ready_sender.send(Err(err)).ok();
            return;
        }
    };

    // 等待捕获器启动
    while CAPTURER_STATE.load(std::sync::atomic::Ordering::Relaxed) == 0 {
        std::thread::sleep(Duration::from_millis(10));
    }

    let pw_loop = mainloop.loop_();

    // 用户调用 Capturer::start() 后启动主循环
    while CAPTURER_STATE.load(std::sync::atomic::Ordering::Relaxed) == 1
        && // 如果流状态变更为错误，退出循环
          !STREAM_STATE_CHANGED_TO_ERROR.load(std::sync::atomic::Ordering::Relaxed)
    {
        pw_loop.iterate(Timeout::Finite(Duration::from_millis(100)));
    }
}

/// Wayland 捕获器结构体
/// 管理 PipeWire 流和 D-Bus 连接
pub struct WaylandCapturer {
    capturer_join_handle: Option<JoinHandle<()>>, // 捕获器线程句柄
    // PipeWire 流在连接释放时会被删除，因此需要保持连接
    _connection: dbus::blocking::Connection, // D-Bus 连接
}

impl WaylandCapturer {
    // TODO: 错误处理
    /// 创建新的 Wayland 捕获器
    /// 通过 D-Bus 与 ScreenCast Portal 通信创建流
    pub fn new(options: &Options, tx: Sender<Result<Frame>>) -> Result<Self> {
        // 创建 D-Bus 会话连接
        let connection =
            dbus::blocking::Connection::new_session().context("创建 D-Bus 连接失败")?;

        // 创建屏幕投射流
        let stream_id = ScreenCastPortal::new(&connection)
            .show_cursor(options.show_cursor)
            .context("不支持的屏幕捕获光标显示模式")?
            .create_stream()
            .context("获取屏幕捕获流失败")?
            .pw_node_id();

        // TODO: 修复这个 hack
        let options = options.clone();
        let (ready_sender, ready_recv) = sync_channel(1);
        // 启动 PipeWire 捕获器线程
        let capturer_join_handle =
            std::thread::spawn(move || pipewire_capturer(options, tx, &ready_sender, stream_id));

        // 等待捕获器准备就绪
        match ready_recv.recv() {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                return Err(anyhow!(err));
            }
            Err(RecvError) => {
                return Err(anyhow!("Wayland 屏幕捕获 bug: 流意外释放"));
            }
        }

        Ok(Self {
            capturer_join_handle: Some(capturer_join_handle),
            _connection: connection,
        })
    }
}

/// 实现 LinuxCapturerImpl trait
impl LinuxCapturerImpl for WaylandCapturer {
    /// 开始屏幕捕获
    fn start_capture(&mut self) {
        CAPTURER_STATE.store(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// 停止屏幕捕获
    fn stop_capture(&mut self) {
        CAPTURER_STATE.store(2, std::sync::atomic::Ordering::Relaxed);
        // 等待捕获器线程结束
        if let Some(handle) = self.capturer_join_handle.take() {
            match handle.join() {
                Ok(()) => {}
                Err(err) => log::error!("连接 Wayland 屏幕捕获线程失败: {err:?}"),
            }
        }
        // 重置状态
        CAPTURER_STATE.store(0, std::sync::atomic::Ordering::Relaxed);
        STREAM_STATE_CHANGED_TO_ERROR.store(false, std::sync::atomic::Ordering::Relaxed);
    }
}
