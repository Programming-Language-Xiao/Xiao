//! 类型化 IR 的稳定 JSON 快照接口。
//!
//! 快照使用显式 IR 结构的声明顺序序列化；输入模型中的集合、映射和控制流
//! 已在降低阶段按确定性顺序排列。解析快照时会检查格式版本，避免把未来
//! 版本静默解释成旧结构。

use std::fmt::{self, Display, Formatter};

use crate::{IR_VERSION, IrProgram};

/// 快照格式版本。
pub const IR_SNAPSHOT_VERSION: u32 = 1;

/// 快照编码或解码错误。
#[derive(Debug)]
pub enum SnapshotError {
    /// JSON 编码失败。
    Encode(String),
    /// JSON 解码失败。
    Decode(String),
    /// 文档不是当前支持的 IR 版本。
    UnsupportedVersion(u32),
}

impl Display for SnapshotError {
    /// 输出稳定错误文本。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(message) => write!(formatter, "IR 快照编码失败: {message}"),
            Self::Decode(message) => write!(formatter, "IR 快照解码失败: {message}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "不支持的 IR 快照版本: {version}")
            }
        }
    }
}

impl std::error::Error for SnapshotError {}

/// 将 IR 编码为稳定紧凑 JSON。
pub fn to_json(program: &IrProgram) -> Result<String, SnapshotError> {
    if program.version != IR_VERSION {
        return Err(SnapshotError::UnsupportedVersion(program.version));
    }
    serde_json::to_string(program).map_err(|error| SnapshotError::Encode(error.to_string()))
}

/// 从 JSON 读取并检查 IR 快照版本。
pub fn from_json(input: &str) -> Result<IrProgram, SnapshotError> {
    let program: IrProgram =
        serde_json::from_str(input).map_err(|error| SnapshotError::Decode(error.to_string()))?;
    if program.version != IR_VERSION {
        return Err(SnapshotError::UnsupportedVersion(program.version));
    }
    Ok(program)
}
