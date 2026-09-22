//! 独立诊断窗口的跨进程消息与渲染契约。
//!
//! Runtime 只发送结构化事件；本模块不执行用户代码，也不把终端 API 暴露给语言
//! 语义层。消息使用与 X0-A 相同的八字节大端长度前缀，方便 Runtime 和窗口进程
//! 在三种宿主上共享边界检查。

use std::collections::BTreeMap;
use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

/// 当前独立诊断通道版本。
pub const DIAGNOSTIC_PROTOCOL_VERSION: u16 = 1;
/// 单帧诊断负载的最大字节数。
pub const MAX_DIAGNOSTIC_FRAME_BYTES: usize = 4 * 1024 * 1024;

/// 诊断启动阶段的稳定错误码。
pub const DIAGNOSTIC_START_CODE: &str = "X11-DIAGNOSTIC-START-001";
/// 诊断通道通信失败的稳定错误码（运行中仅写入日志，不改变退出码）。
pub const DIAGNOSTIC_CHANNEL_CODE: &str = "X11-DIAGNOSTIC-CHANNEL-001";

/// 写入原生产物旁边的调试激活元数据。
///
/// 该文件是发布构建的持久激活位载体；普通构建不会生成它。X0-E 负责把这份
/// 元数据随原生产物分发，独立 Runtime 读取后按同一启动协议开窗。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiagnosticActivation {
    /// 激活元数据格式版本。
    pub format_version: u16,
    /// 是否强制启动诊断窗口。
    pub enabled: bool,
    /// 源码映射文件或内嵌映射摘要。
    pub source_map: Option<String>,
    /// 诊断元数据版本。
    pub metadata_version: u16,
    /// Runtime 钩子能力是否可用。
    pub hooks: bool,
}

/// 从原生可执行文件路径派生激活元数据路径。
#[must_use]
pub fn activation_path(executable: impl AsRef<std::path::Path>) -> std::path::PathBuf {
    let mut path = executable.as_ref().to_path_buf();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if extension.is_empty() {
        path.set_extension("xiao-debug.json");
    } else {
        path.set_extension(format!("{extension}.xiao-debug.json"));
    }
    path
}

/// 将激活位写到原生产物旁边。
pub fn write_activation(
    executable: impl AsRef<std::path::Path>,
    activation: &DiagnosticActivation,
) -> io::Result<std::path::PathBuf> {
    let path = activation_path(executable);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(activation)
        .map_err(|error| io::Error::other(error.to_string()))?;
    std::fs::write(&path, [bytes.as_slice(), b"\n"].concat())?;
    Ok(path)
}

/// 读取原生产物旁的调试激活位；缺少文件表示普通构建。
pub fn read_activation(
    executable: impl AsRef<std::path::Path>,
) -> io::Result<Option<DiagnosticActivation>> {
    let path = activation_path(executable);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(path)?;
    let activation = serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    Ok(Some(activation))
}

/// 一条跨进程诊断事件。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiagnosticEvent {
    /// 从会话建立起计算的单调纳秒时间。
    pub monotonic_ns: u128,
    /// `trace`、`debug`、`info`、`warn` 或 `error`。
    pub level: String,
    /// 稳定事件类型名。
    pub event_type: String,
    /// 逻辑模块身份。
    pub module: Option<String>,
    /// 源码路径或内存来源名。
    pub source: Option<String>,
    /// 节点或函数标识。
    pub node: Option<String>,
    /// 函数标识（若事件带有函数上下文）。
    pub function: Option<String>,
    /// 关联的结构化错误编号。
    pub error_id: Option<u64>,
    /// 不改变语义的事件负载。
    pub payload: BTreeMap<String, Value>,
}

/// 诊断窗口最后一行显示的聚合指标。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiagnosticMetrics {
    /// 程序运行时间（毫秒）。
    pub elapsed_ms: u128,
    /// 当前内存占用；Runtime 尚未提供采样时为零。
    pub current_memory_bytes: u64,
    /// 内存峰值；Runtime 尚未提供采样时为零。
    pub peak_memory_bytes: u64,
    /// 全部结构化运行时错误事件数量。
    pub error_count: u64,
    /// 预断点/断点钩子命中次数。
    pub breakpoint_hits: u64,
    /// 钩子触发次数。
    pub hook_count: u64,
}

/// Runtime 与独立窗口之间的消息。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DiagnosticMessage {
    /// 窗口进程连接后的首条握手。
    Hello {
        /// 通道版本。
        protocol_version: u16,
        /// Runtime 生成的一次性令牌。
        token: String,
        /// 窗口进程版本文本。
        renderer: String,
    },
    /// Runtime 确认窗口已经可接收事件。
    Ready {
        /// 会话编号。
        session_id: String,
    },
    /// 一条事件。
    Event {
        /// 结构化事件。
        event: DiagnosticEvent,
    },
    /// 程序结束时的最终指标快照。
    Final {
        /// 最终指标。
        metrics: DiagnosticMetrics,
    },
    /// Runtime 主动关闭通道。
    Close {
        /// 关闭原因，供窗口显示。
        reason: String,
    },
}

/// 诊断帧错误。
#[derive(Debug)]
pub enum DiagnosticFrameError {
    /// 底层 I/O 错误。
    Io(io::Error),
    /// 长度超过上限。
    LengthTooLarge {
        /// 收到的长度。
        length: u64,
        /// 允许的最大长度。
        maximum: usize,
    },
    /// 长度字段不完整。
    TruncatedLength {
        /// 已读取的长度字节数。
        read: usize,
    },
    /// 负载不完整。
    TruncatedPayload {
        /// 声明的负载长度。
        expected: usize,
        /// 实际读取的负载长度。
        read: usize,
    },
    /// JSON 编码失败。
    Json(String),
}

impl std::fmt::Display for DiagnosticFrameError {
    /// 渲染稳定的开发者错误摘要。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "诊断通道 I/O 失败：{error}"),
            Self::LengthTooLarge { length, maximum } => {
                write!(formatter, "诊断帧长度 {length} 超过上限 {maximum}")
            }
            Self::TruncatedLength { read } => {
                write!(formatter, "诊断帧长度字段截断（已读 {read} 字节）")
            }
            Self::TruncatedPayload { expected, read } => {
                write!(
                    formatter,
                    "诊断帧负载截断（需要 {expected}，已读 {read} 字节）"
                )
            }
            Self::Json(error) => write!(formatter, "诊断帧 JSON 无效：{error}"),
        }
    }
}

impl std::error::Error for DiagnosticFrameError {}

impl From<io::Error> for DiagnosticFrameError {
    /// 将底层 I/O 错误包装为诊断帧错误。
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// 编码一条带长度边界的诊断帧。
pub fn encode_frame<T: Serialize>(value: &T) -> Result<Vec<u8>, DiagnosticFrameError> {
    let payload =
        serde_json::to_vec(value).map_err(|error| DiagnosticFrameError::Json(error.to_string()))?;
    if payload.len() > MAX_DIAGNOSTIC_FRAME_BYTES {
        return Err(DiagnosticFrameError::LengthTooLarge {
            length: payload.len() as u64,
            maximum: MAX_DIAGNOSTIC_FRAME_BYTES,
        });
    }
    let mut frame = Vec::with_capacity(8 + payload.len());
    frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

/// 从流中读取一条诊断帧；干净 EOF 返回 `Ok(None)`。
pub fn read_frame<R: Read>(reader: &mut R) -> Result<Option<Vec<u8>>, DiagnosticFrameError> {
    let mut length_bytes = [0_u8; 8];
    let mut read = 0;
    while read < length_bytes.len() {
        match reader.read(&mut length_bytes[read..])? {
            0 if read == 0 => return Ok(None),
            0 => return Err(DiagnosticFrameError::TruncatedLength { read }),
            amount => read += amount,
        }
    }
    let length = u64::from_be_bytes(length_bytes);
    if length > MAX_DIAGNOSTIC_FRAME_BYTES as u64 {
        return Err(DiagnosticFrameError::LengthTooLarge {
            length,
            maximum: MAX_DIAGNOSTIC_FRAME_BYTES,
        });
    }
    let expected = length as usize;
    let mut payload = vec![0_u8; expected];
    let mut received = 0;
    while received < expected {
        match reader.read(&mut payload[received..])? {
            0 => {
                return Err(DiagnosticFrameError::TruncatedPayload {
                    expected,
                    read: received,
                });
            }
            amount => received += amount,
        }
    }
    Ok(Some(payload))
}

/// 读取并反序列化一条诊断消息。
pub fn read_message<R: Read>(
    reader: &mut R,
) -> Result<Option<DiagnosticMessage>, DiagnosticFrameError> {
    let Some(payload) = read_frame(reader)? else {
        return Ok(None);
    };
    serde_json::from_slice(&payload)
        .map(Some)
        .map_err(|error| DiagnosticFrameError::Json(error.to_string()))
}

/// 编码并写出一条诊断消息。
pub fn write_message<W: Write>(
    writer: &mut W,
    message: &DiagnosticMessage,
) -> Result<(), DiagnosticFrameError> {
    let frame = encode_frame(message)?;
    writer.write_all(&frame)?;
    writer.flush()?;
    Ok(())
}

/// 把事件类型映射到默认日志等级。
#[must_use]
pub fn default_level(event_type: &str) -> &'static str {
    if event_type.contains("fatal") {
        "fatal"
    } else if event_type.contains("error") {
        "error"
    } else if event_type == "metrics" {
        "debug"
    } else {
        "info"
    }
}

/// 把任意 JSON 值转换为结构化负载地图。
#[must_use]
pub fn object_payload(value: Value) -> BTreeMap<String, Value> {
    value
        .as_object()
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// 读取 JSON 帧的通用辅助函数，供契约测试使用。
pub fn decode_payload<T: DeserializeOwned>(payload: &[u8]) -> Result<T, DiagnosticFrameError> {
    serde_json::from_slice(payload).map_err(|error| DiagnosticFrameError::Json(error.to_string()))
}

#[cfg(test)]
/// 覆盖诊断帧和激活位边界。
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    /// 普通诊断消息经过长度帧后保持完全一致。
    fn diagnostic_frame_round_trips() {
        let message = DiagnosticMessage::Ready {
            session_id: "s1".to_owned(),
        };
        let frame = encode_frame(&message).expect("encode");
        let decoded = read_message(&mut Cursor::new(frame))
            .expect("read")
            .expect("message");
        assert_eq!(decoded, message);
    }

    #[test]
    /// 读取长度时先检查上限，不按恶意长度分配内存。
    fn oversized_frame_is_rejected_before_allocation() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&((MAX_DIAGNOSTIC_FRAME_BYTES as u64) + 1).to_be_bytes());
        let error = read_frame(&mut Cursor::new(bytes)).expect_err("must reject");
        assert!(matches!(error, DiagnosticFrameError::LengthTooLarge { .. }));
    }

    #[test]
    /// 致命事件使用独立的 fatal 等级，便于等级过滤和终端高亮。
    fn fatal_events_use_fatal_level() {
        assert_eq!(default_level("fatal_raised"), "fatal");
        assert_eq!(default_level("error_raised"), "error");
    }

    #[test]
    /// 激活位可持久化读取，删除后恢复普通构建语义。
    fn activation_is_persistent_and_absent_for_release_shape() {
        let directory = std::env::temp_dir().join(format!(
            "xiao-diagnostic-activation-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).expect("temp directory");
        let executable = directory.join(if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        });
        std::fs::write(&executable, b"binary").expect("executable placeholder");
        assert_eq!(read_activation(&executable).expect("read"), None);
        let activation = DiagnosticActivation {
            format_version: 1,
            enabled: true,
            source_map: Some("<embedded>".to_owned()),
            metadata_version: 1,
            hooks: true,
        };
        let path = write_activation(&executable, &activation).expect("write");
        assert_eq!(
            read_activation(&executable).expect("read"),
            Some(activation)
        );
        std::fs::remove_file(path).expect("remove");
        std::fs::remove_file(executable).expect("remove executable");
        std::fs::remove_dir(directory).expect("remove directory");
    }
}
