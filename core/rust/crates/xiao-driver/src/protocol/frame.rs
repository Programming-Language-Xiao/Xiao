//! 长度前缀 JSON 帧编解码。

use std::fmt::{self, Display, Formatter};
use std::io::{self, Read, Write};

use serde::{Serialize, de::DeserializeOwned};

/// 长度字段的固定字节数。
pub const FRAME_LENGTH_BYTES: usize = 8;
/// 单帧 JSON 负载的最大字节数。
pub const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
/// 帧或协议错误的稳定编号。
pub const FRAME_ERROR_CODE: &str = "X11-PROTOCOL-001";

/// 帧解码错误。
#[derive(Debug)]
pub enum FrameError {
    /// 底层 I/O 错误。
    Io(io::Error),
    /// 长度字段超过协议上限。
    LengthTooLarge {
        /// 收到的长度。
        length: u64,
        /// 允许的最大长度。
        maximum: usize,
    },
    /// 长度字段只读到一部分。
    TruncatedLength {
        /// 已读取字节数。
        read: usize,
    },
    /// JSON 负载只读到一部分。
    TruncatedPayload {
        /// 声明的负载长度。
        expected: usize,
        /// 实际读取字节数。
        read: usize,
    },
    /// 负载不是 UTF-8。
    InvalidUtf8,
    /// JSON 语法或形状非法。
    InvalidJson(String),
}

impl FrameError {
    /// 返回稳定错误码。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        FRAME_ERROR_CODE
    }
}

impl Display for FrameError {
    /// 将帧错误渲染为调试摘要；调用方仍应读取 [`FrameError::code`]。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "帧 I/O 失败：{error}"),
            Self::LengthTooLarge { length, maximum } => {
                write!(formatter, "帧长度 {length} 超过上限 {maximum}")
            }
            Self::TruncatedLength { read } => {
                write!(
                    formatter,
                    "长度字段被截断：已读取 {read}/{FRAME_LENGTH_BYTES} 字节"
                )
            }
            Self::TruncatedPayload { expected, read } => {
                write!(formatter, "JSON 负载被截断：需要 {expected}，实际 {read}")
            }
            Self::InvalidUtf8 => formatter.write_str("JSON 负载不是 UTF-8"),
            Self::InvalidJson(error) => write!(formatter, "JSON 负载无效：{error}"),
        }
    }
}

impl std::error::Error for FrameError {}

impl From<io::Error> for FrameError {
    /// 将底层流错误归入帧错误。
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// 将 JSON 值编码为一帧，长度只包含 JSON 字节。
pub fn encode_frame<T: Serialize>(value: &T) -> Result<Vec<u8>, FrameError> {
    let payload =
        serde_json::to_vec(value).map_err(|error| FrameError::InvalidJson(error.to_string()))?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(FrameError::LengthTooLarge {
            length: payload.len() as u64,
            maximum: MAX_FRAME_BYTES,
        });
    }
    let length = u64::try_from(payload.len()).map_err(|_| FrameError::LengthTooLarge {
        length: u64::MAX,
        maximum: MAX_FRAME_BYTES,
    })?;
    let mut frame = Vec::with_capacity(FRAME_LENGTH_BYTES + payload.len());
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

/// 从一帧 JSON 负载解码消息。
pub fn decode_frame<T: DeserializeOwned>(payload: &[u8]) -> Result<T, FrameError> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err(FrameError::LengthTooLarge {
            length: payload.len() as u64,
            maximum: MAX_FRAME_BYTES,
        });
    }
    std::str::from_utf8(payload).map_err(|_| FrameError::InvalidUtf8)?;
    serde_json::from_slice(payload).map_err(|error| FrameError::InvalidJson(error.to_string()))
}

/// 从流中读取一帧；干净 EOF 返回 `Ok(None)`。
pub fn read_frame<R: Read>(reader: &mut R) -> Result<Option<Vec<u8>>, FrameError> {
    let mut length_bytes = [0_u8; FRAME_LENGTH_BYTES];
    let mut read = 0;
    while read < FRAME_LENGTH_BYTES {
        match reader.read(&mut length_bytes[read..])? {
            0 if read == 0 => return Ok(None),
            0 => return Err(FrameError::TruncatedLength { read }),
            amount => read += amount,
        }
    }
    let length = u64::from_be_bytes(length_bytes);
    if length > MAX_FRAME_BYTES as u64 {
        return Err(FrameError::LengthTooLarge {
            length,
            maximum: MAX_FRAME_BYTES,
        });
    }
    let expected = usize::try_from(length).map_err(|_| FrameError::LengthTooLarge {
        length,
        maximum: MAX_FRAME_BYTES,
    })?;
    let mut payload = vec![0_u8; expected];
    let mut received = 0;
    while received < expected {
        match reader.read(&mut payload[received..])? {
            0 => {
                return Err(FrameError::TruncatedPayload {
                    expected,
                    read: received,
                });
            }
            amount => received += amount,
        }
    }
    Ok(Some(payload))
}

/// 将 JSON 值编码并写入流，然后刷新输出。
pub fn write_frame<W: Write, T: Serialize>(writer: &mut W, value: &T) -> Result<(), FrameError> {
    let frame = encode_frame(value)?;
    writer.write_all(&frame)?;
    writer.flush()?;
    Ok(())
}
