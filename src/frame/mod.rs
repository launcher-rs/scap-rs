/// YUV 帧格式结构体
/// 包含亮度和色度分量数据
#[derive(Debug, Clone)]
pub struct YUVFrame {
    pub display_time: u64,          // 显示时间戳（纳秒）
    pub width: i32,                 // 帧宽度（像素）
    pub height: i32,                // 帧高度（像素）
    pub luminance_bytes: Vec<u8>,   // 亮度分量数据（Y）
    pub luminance_stride: i32,      // 亮度分量步长
    pub chrominance_bytes: Vec<u8>, // 色度分量数据（UV）
    pub chrominance_stride: i32,    // 色度分量步长
}

/// RGB 帧格式结构体
/// 包含红、绿、蓝三通道数据
#[derive(Debug, Clone)]
pub struct RGBFrame {
    pub display_time: u64, // 显示时间戳（纳秒）
    pub width: i32,        // 帧宽度（像素）
    pub height: i32,       // 帧高度（像素）
    pub data: Vec<u8>,     // RGB 像素数据
}

/// RGB8 帧格式结构体（无数据字段）
#[derive(Debug, Clone)]
pub struct RGB8Frame {
    pub display_time: u64, // 显示时间戳（纳秒）
    pub width: i32,        // 帧宽度（像素）
    pub height: i32,       // 帧高度（像素）
}

/// RGBx 帧格式结构体（包含填充字节）
#[derive(Debug, Clone)]
pub struct RGBxFrame {
    pub display_time: u64, // 显示时间戳（纳秒）
    pub width: i32,        // 帧宽度（像素）
    pub height: i32,       // 帧高度（像素）
    pub data: Vec<u8>,     // RGBx 像素数据
}

/// XBGR 帧格式结构体
#[derive(Debug, Clone)]
pub struct XBGRFrame {
    pub display_time: u64, // 显示时间戳（纳秒）
    pub width: i32,        // 帧宽度（像素）
    pub height: i32,       // 帧高度（像素）
    pub data: Vec<u8>,     // XBGR 像素数据
}

/// BGRx 帧格式结构体
#[derive(Debug, Clone)]
pub struct BGRxFrame {
    pub display_time: u64, // 显示时间戳（纳秒）
    pub width: i32,        // 帧宽度（像素）
    pub height: i32,       // 帧高度（像素）
    pub data: Vec<u8>,     // BGRx 像素数据
}

/// BGR 帧格式结构体
#[derive(Debug, Clone)]
pub struct BGRFrame {
    pub display_time: u64, // 显示时间戳（纳秒）
    pub width: i32,        // 帧宽度（像素）
    pub height: i32,       // 帧高度（像素）
    pub data: Vec<u8>,     // BGR 像素数据
}

/// BGRA 帧格式结构体（包含 Alpha 通道）
#[derive(Debug, Clone)]
pub struct BGRAFrame {
    pub display_time: u64, // 显示时间戳（纳秒）
    pub width: i32,        // 帧宽度（像素）
    pub height: i32,       // 帧高度（像素）
    pub data: Vec<u8>,     // BGRA 像素数据
}

/// 帧类型枚举，指定输出帧的格式
#[derive(Debug, Clone, Copy, Default)]
pub enum FrameType {
    #[default]
    YUVFrame, // YUV 格式（推荐，性能最佳）
    BGR0,      // BGR0 格式（推荐，性能较好）
    RGB,       // RGB 格式（性能较慢，不推荐）
    BGRAFrame, // BGRA 格式（包含 Alpha 通道）
}

/// 统一的帧数据枚举，包含所有支持的帧格式
#[derive(Debug, Clone)]
pub enum Frame {
    YUVFrame(YUVFrame), // YUV 帧
    RGB(RGBFrame),      // RGB 帧
    RGBx(RGBxFrame),    // RGBx 帧
    XBGR(XBGRFrame),    // XBGR 帧
    BGRx(BGRxFrame),    // BGRx 帧
    BGR0(BGRFrame),     // BGR0 帧
    BGRA(BGRAFrame),    // BGRA 帧
}

/// 帧数据引用枚举，用于零拷贝访问帧数据
pub enum FrameData<'a> {
    NV12(&'a YUVFrame), // NV12 格式 YUV 帧引用
    BGR0(&'a [u8]),     // BGR0 格式数据引用
}

/// 移除图像数据的 Alpha 通道
/// 将 4 字节的 BGRA 数据转换为 3 字节的 BGR 数据
pub fn remove_alpha_channel(frame_data: Vec<u8>) -> Vec<u8> {
    let width = frame_data.len();
    let width_without_alpha = (width / 4) * 3;

    let mut data: Vec<u8> = vec![0; width_without_alpha];

    // 每 4 字节复制前 3 字节，跳过 Alpha 通道
    for (src, dst) in frame_data.chunks_exact(4).zip(data.chunks_exact_mut(3)) {
        dst[0] = src[0];
        dst[1] = src[1];
        dst[2] = src[2];
    }

    data
}

/// 将 BGRA 格式转换为 RGB 格式
/// 调整颜色通道顺序：BGRA -> RGB
pub fn convert_bgra_to_rgb(frame_data: Vec<u8>) -> Vec<u8> {
    let width = frame_data.len();
    let width_without_alpha = (width / 4) * 3;

    let mut data: Vec<u8> = vec![0; width_without_alpha];

    // 转换颜色通道顺序：B,G,R,A -> R,G,B
    for (src, dst) in frame_data.chunks_exact(4).zip(data.chunks_exact_mut(3)) {
        dst[0] = src[2]; // R
        dst[1] = src[1]; // G
        dst[2] = src[0]; // B
    }

    data
}

/// 裁剪图像数据
/// 从原始图像中提取指定区域的像素数据
pub fn get_cropped_data(data: Vec<u8>, cur_width: i32, height: i32, width: i32) -> Vec<u8> {
    // 检查数据长度是否匹配
    if data.len() as i32 != height * cur_width * 4 {
        data
    } else {
        // 分配裁剪后的数据缓冲区
        let mut cropped_data: Vec<u8> = vec![0; (4 * height * width).try_into().unwrap()];
        let mut cropped_data_index = 0;

        // 逐行复制指定宽度的像素数据
        for (i, item) in data.iter().enumerate() {
            let x = i as i32 % (cur_width * 4);
            if x < (width * 4) {
                cropped_data[cropped_data_index] = *item;
                cropped_data_index += 1;
            }
        }
        cropped_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试移除 Alpha 通道功能
    #[test]
    fn test_remove_alpha_channel() {
        assert_eq!(remove_alpha_channel(vec![1, 2, 3, 0]), vec![1, 2, 3]);
        assert_eq!(
            remove_alpha_channel(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            vec![1, 2, 3, 5, 6, 7]
        );
    }

    /// 测试 BGRA 到 RGB 转换功能
    #[test]
    fn test_convert_bgra_to_rgb() {
        assert_eq!(convert_bgra_to_rgb(vec![1, 2, 3, 0]), vec![3, 2, 1]);
        assert_eq!(
            convert_bgra_to_rgb(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            vec![3, 2, 1, 7, 6, 5]
        );
    }

    /// 辅助宏：创建 RGBA 像素数据
    macro_rules! rgba {
        ($n:expr) => {
            &mut vec![$n, $n, $n, $n]
        };
    }

    /// 测试图像裁剪功能
    #[test]
    pub fn test_get_cropped_data() {
        // 创建 3x3 的测试图像数据
        let mut data: Vec<u8> = Vec::new();
        for i in 1..=9 {
            data.append(rgba!(i));
        }
        // 预期裁剪后的 2x2 图像数据
        let mut expected: Vec<u8> = Vec::new();
        expected.append(rgba!(1));
        expected.append(rgba!(2));
        expected.append(rgba!(4));
        expected.append(rgba!(5));
        expected.append(rgba!(7));
        expected.append(rgba!(8));
        assert_eq!(get_cropped_data(data, 3, 3, 2), expected)
    }
}
