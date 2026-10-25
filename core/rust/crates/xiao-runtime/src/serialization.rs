//! 22A 冻结的 XCON v1 长度前缀序列化。
//!
//! 编码器只处理静态 Send/Sync 已经允许的标量和容器；resource/dynamic tag 保留给格式
//! 但在 Runtime 编码阶段稳定拒绝。所有整数、长度和浮点位均使用大端表示。

use std::fmt;

/// XCON 魔数。
pub const XCON_MAGIC: [u8; 4] = *b"XCON";
/// XCON 当前版本。
pub const XCON_VERSION: u16 = 1;
/// 单值最大字节数。
pub const XCON_MAX_VALUE_BYTES: usize = 16 * 1024 * 1024;
/// 整帧最大字节数。
pub const XCON_MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;
/// 最大嵌套深度。
pub const XCON_MAX_DEPTH: usize = 64;
/// 最大节点数。
pub const XCON_MAX_NODES: usize = 1_000_000;

/// XCON v1 支持的值。
#[derive(Clone, Debug)]
pub enum XconValue {
    /// 空值。
    None,
    /// 64 位有符号整数。
    Int(i64),
    /// 32 位有符号整数。
    Sint(i32),
    /// 任意精度整数的规范文本。
    Lint(String),
    /// 64 位浮点。
    Float(f64),
    /// 32 位浮点。
    Sfloat(f32),
    /// 任意精度浮点的规范文本。
    Lfloat(String),
    /// 布尔值。
    Bool(bool),
    /// UTF-8 字符串。
    Str(String),
    /// 数组。
    Array(Vec<Self>),
    /// 元组。
    Tuple(Vec<Self>),
    /// 集合；编码时按节点字节序排序。
    Set(Vec<Self>),
    /// 字典；编码时按键/值节点字节序排序。
    Dict(Vec<(Self, Self)>),
}

impl PartialEq for XconValue {
    /// 按标签、位模式和递归成员比较 XCON 值。
    fn eq(&self, other: &Self) -> bool {
        use XconValue::*;
        match (self, other) {
            (None, None) => true,
            (Bool(a), Bool(b)) => a == b,
            (Int(a), Int(b)) => a == b,
            (Sint(a), Sint(b)) => a == b,
            (Lint(a), Lint(b)) | (Lfloat(a), Lfloat(b)) | (Str(a), Str(b)) => a == b,
            (Float(a), Float(b)) => a.to_bits() == b.to_bits(),
            (Sfloat(a), Sfloat(b)) => a.to_bits() == b.to_bits(),
            (Array(a), Array(b)) | (Tuple(a), Tuple(b)) | (Set(a), Set(b)) => a == b,
            (Dict(a), Dict(b)) => a == b,
            _ => false,
        }
    }
}

/// XCON 编解码错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XconError {
    /// 输入不是 XCON v1。
    InvalidHeader,
    /// 版本不支持。
    UnsupportedVersion(u16),
    /// 未知或保留 tag。
    UnsupportedTag(u8),
    /// 长度、深度或节点限制超出。
    LimitExceeded(&'static str),
    /// 节点载荷不完整或尾部有额外字节。
    Truncated,
    /// UTF-8 字符串无效。
    InvalidUtf8,
    /// bool 载荷不是 0/1。
    InvalidBoolean,
    /// 集合或字典存在重复键/成员。
    DuplicateMember,
}

impl XconError {
    /// 返回冻结的运行时错误码。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        "X06-RUNTIME-029"
    }
}

impl fmt::Display for XconError {
    /// 输出稳定的 XCON 错误文本。
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHeader => formatter.write_str("XCON 头无效"),
            Self::UnsupportedVersion(version) => write!(formatter, "XCON 版本 {version} 不支持"),
            Self::UnsupportedTag(tag) => write!(formatter, "XCON tag {tag} 不支持"),
            Self::LimitExceeded(limit) => write!(formatter, "XCON 超过 {limit} 限制"),
            Self::Truncated => formatter.write_str("XCON 节点数据不完整"),
            Self::InvalidUtf8 => formatter.write_str("XCON 字符串不是 UTF-8"),
            Self::InvalidBoolean => formatter.write_str("XCON bool 载荷不是 0/1"),
            Self::DuplicateMember => formatter.write_str("XCON 集合或字典存在重复成员"),
        }
    }
}

impl std::error::Error for XconError {}

/// 编码一帧 XCON v1。
pub fn encode_xcon(value: &XconValue) -> Result<Vec<u8>, XconError> {
    let mut output = Vec::new();
    let mut nodes = 0;
    encode_node(value, 0, &mut nodes, &mut output)?;
    if output.len() > XCON_MAX_FRAME_BYTES {
        return Err(XconError::LimitExceeded("整帧大小"));
    }
    Ok(output)
}

/// 解码一帧 XCON v1，并拒绝尾部垃圾字节。
pub fn decode_xcon(bytes: &[u8]) -> Result<XconValue, XconError> {
    if bytes.len() > XCON_MAX_FRAME_BYTES {
        return Err(XconError::LimitExceeded("整帧大小"));
    }
    let mut decoder = Decoder {
        bytes,
        cursor: 0,
        nodes: 0,
    };
    let value = decoder.decode_node(0)?;
    if decoder.cursor != bytes.len() {
        return Err(XconError::InvalidHeader);
    }
    Ok(value)
}

/// 编码单个递归节点。
fn encode_node(
    value: &XconValue,
    depth: usize,
    nodes: &mut usize,
    output: &mut Vec<u8>,
) -> Result<(), XconError> {
    if depth > XCON_MAX_DEPTH {
        return Err(XconError::LimitExceeded("嵌套深度"));
    }
    *nodes = nodes.saturating_add(1);
    if *nodes > XCON_MAX_NODES {
        return Err(XconError::LimitExceeded("节点数"));
    }
    let mut payload = Vec::new();
    let tag = match value {
        XconValue::None => 0,
        XconValue::Int(value) => {
            payload.extend_from_slice(&value.to_be_bytes());
            1
        }
        XconValue::Sint(value) => {
            payload.extend_from_slice(&value.to_be_bytes());
            2
        }
        XconValue::Lint(value) => {
            payload.extend_from_slice(value.as_bytes());
            3
        }
        XconValue::Float(value) => {
            payload.extend_from_slice(&value.to_bits().to_be_bytes());
            4
        }
        XconValue::Sfloat(value) => {
            payload.extend_from_slice(&value.to_bits().to_be_bytes());
            5
        }
        XconValue::Lfloat(value) => {
            payload.extend_from_slice(value.as_bytes());
            6
        }
        XconValue::Bool(value) => {
            payload.push(u8::from(*value));
            7
        }
        XconValue::Str(value) => {
            payload.extend_from_slice(value.as_bytes());
            8
        }
        XconValue::Array(values) => {
            encode_children(values, depth, nodes, &mut payload)?;
            9
        }
        XconValue::Tuple(values) => {
            encode_children(values, depth, nodes, &mut payload)?;
            10
        }
        XconValue::Set(values) => {
            let mut encoded = values
                .iter()
                .map(|value| encode_node_bytes(value, depth + 1, nodes))
                .collect::<Result<Vec<_>, _>>()?;
            encoded.sort();
            encoded.dedup();
            if encoded.len() != values.len() {
                return Err(XconError::DuplicateMember);
            }
            put_u32(&mut payload, encoded.len())?;
            for child in encoded {
                payload.extend_from_slice(&child);
            }
            11
        }
        XconValue::Dict(entries) => {
            let mut encoded = entries
                .iter()
                .map(|(key, value)| {
                    let key = encode_node_bytes(key, depth + 1, nodes)?;
                    let value = encode_node_bytes(value, depth + 1, nodes)?;
                    Ok((key, value))
                })
                .collect::<Result<Vec<_>, XconError>>()?;
            encoded.sort();
            if encoded.windows(2).any(|pair| pair[0].0 == pair[1].0) {
                return Err(XconError::DuplicateMember);
            }
            put_u32(&mut payload, encoded.len())?;
            for (key, value) in encoded {
                payload.extend_from_slice(&key);
                payload.extend_from_slice(&value);
            }
            12
        }
    };
    if payload.len() > XCON_MAX_VALUE_BYTES || payload.len() > u32::MAX as usize {
        return Err(XconError::LimitExceeded("单值大小"));
    }
    output.extend_from_slice(&XCON_MAGIC);
    output.extend_from_slice(&XCON_VERSION.to_be_bytes());
    output.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    output.push(tag);
    output.push(0);
    output.extend_from_slice(&payload);
    Ok(())
}

/// 编码节点并返回独立字节，用于集合/字典确定性排序。
fn encode_node_bytes(
    value: &XconValue,
    depth: usize,
    nodes: &mut usize,
) -> Result<Vec<u8>, XconError> {
    let mut bytes = Vec::new();
    encode_node(value, depth, nodes, &mut bytes)?;
    Ok(bytes)
}

/// 编码容器子节点序列。
fn encode_children(
    values: &[XconValue],
    depth: usize,
    nodes: &mut usize,
    output: &mut Vec<u8>,
) -> Result<(), XconError> {
    put_u32(output, values.len())?;
    for value in values {
        encode_node(value, depth + 1, nodes, output)?;
    }
    Ok(())
}

/// 写入大端 u32 容器计数。
fn put_u32(output: &mut Vec<u8>, value: usize) -> Result<(), XconError> {
    if value > u32::MAX as usize {
        return Err(XconError::LimitExceeded("容器元素数"));
    }
    output.extend_from_slice(&(value as u32).to_be_bytes());
    Ok(())
}

/// XCON 递归解码器状态。
struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
    nodes: usize,
}

impl<'a> Decoder<'a> {
    /// 解码一个节点并推进游标。
    fn decode_node(&mut self, depth: usize) -> Result<XconValue, XconError> {
        if depth > XCON_MAX_DEPTH {
            return Err(XconError::LimitExceeded("嵌套深度"));
        }
        self.nodes = self.nodes.saturating_add(1);
        if self.nodes > XCON_MAX_NODES {
            return Err(XconError::LimitExceeded("节点数"));
        }
        let magic = self.take(4)?;
        if magic != XCON_MAGIC {
            return Err(XconError::InvalidHeader);
        }
        let version = u16::from_be_bytes(self.take(2)?.try_into().expect("版本宽度"));
        if version != XCON_VERSION {
            return Err(XconError::UnsupportedVersion(version));
        }
        let length = u32::from_be_bytes(self.take(4)?.try_into().expect("长度宽度")) as usize;
        if length > XCON_MAX_VALUE_BYTES {
            return Err(XconError::LimitExceeded("单值大小"));
        }
        let tag = self.take(1)?[0];
        if self.take(1)?[0] != 0 {
            return Err(XconError::InvalidHeader);
        }
        let payload = self.take(length)?.to_vec();
        decode_payload(tag, &payload, depth, self)
    }

    /// 读取固定长度输入。
    fn take(&mut self, length: usize) -> Result<&'a [u8], XconError> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(XconError::Truncated)?;
        let slice = self
            .bytes
            .get(self.cursor..end)
            .ok_or(XconError::Truncated)?;
        self.cursor = end;
        Ok(slice)
    }
}

/// 根据 tag 解码节点载荷。
fn decode_payload(
    tag: u8,
    payload: &[u8],
    depth: usize,
    decoder: &mut Decoder<'_>,
) -> Result<XconValue, XconError> {
    match tag {
        0 if payload.is_empty() => Ok(XconValue::None),
        1 if payload.len() == 8 => Ok(XconValue::Int(i64::from_be_bytes(
            payload.try_into().unwrap(),
        ))),
        2 if payload.len() == 4 => Ok(XconValue::Sint(i32::from_be_bytes(
            payload.try_into().unwrap(),
        ))),
        3 => Ok(XconValue::Lint(
            String::from_utf8(payload.to_vec()).map_err(|_| XconError::InvalidUtf8)?,
        )),
        4 if payload.len() == 8 => Ok(XconValue::Float(f64::from_bits(u64::from_be_bytes(
            payload.try_into().unwrap(),
        )))),
        5 if payload.len() == 4 => Ok(XconValue::Sfloat(f32::from_bits(u32::from_be_bytes(
            payload.try_into().unwrap(),
        )))),
        6 => Ok(XconValue::Lfloat(
            String::from_utf8(payload.to_vec()).map_err(|_| XconError::InvalidUtf8)?,
        )),
        7 if payload.len() == 1 => match payload[0] {
            0 => Ok(XconValue::Bool(false)),
            1 => Ok(XconValue::Bool(true)),
            _ => Err(XconError::InvalidBoolean),
        },
        8 => Ok(XconValue::Str(
            String::from_utf8(payload.to_vec()).map_err(|_| XconError::InvalidUtf8)?,
        )),
        9..=11 => {
            let (count, mut cursor) = read_count(payload)?;
            let mut values = Vec::with_capacity(count.min(1024));
            for _ in 0..count {
                let start = cursor;
                let mut child = Decoder {
                    bytes: &payload[cursor..],
                    cursor: 0,
                    nodes: decoder.nodes,
                };
                let value = child.decode_node(depth + 1)?;
                cursor = cursor
                    .checked_add(child.cursor)
                    .ok_or(XconError::Truncated)?;
                decoder.nodes = child.nodes;
                if cursor <= start {
                    return Err(XconError::Truncated);
                }
                values.push(value);
            }
            if cursor != payload.len() {
                return Err(XconError::Truncated);
            }
            match tag {
                9 => Ok(XconValue::Array(values)),
                10 => Ok(XconValue::Tuple(values)),
                _ => Ok(XconValue::Set(values)),
            }
        }
        12 => {
            let (count, mut cursor) = read_count(payload)?;
            let mut entries = Vec::with_capacity(count.min(1024));
            for _ in 0..count {
                let mut key_decoder = Decoder {
                    bytes: &payload[cursor..],
                    cursor: 0,
                    nodes: decoder.nodes,
                };
                let key = key_decoder.decode_node(depth + 1)?;
                cursor = cursor
                    .checked_add(key_decoder.cursor)
                    .ok_or(XconError::Truncated)?;
                decoder.nodes = key_decoder.nodes;
                let mut value_decoder = Decoder {
                    bytes: &payload[cursor..],
                    cursor: 0,
                    nodes: decoder.nodes,
                };
                let value = value_decoder.decode_node(depth + 1)?;
                cursor = cursor
                    .checked_add(value_decoder.cursor)
                    .ok_or(XconError::Truncated)?;
                decoder.nodes = value_decoder.nodes;
                entries.push((key, value));
            }
            if cursor != payload.len() {
                return Err(XconError::Truncated);
            }
            Ok(XconValue::Dict(entries))
        }
        13 | 14 => Err(XconError::UnsupportedTag(tag)),
        _ => Err(XconError::UnsupportedTag(tag)),
    }
}

/// 读取容器载荷中的大端元素计数。
fn read_count(payload: &[u8]) -> Result<(usize, usize), XconError> {
    let count = payload
        .get(..4)
        .ok_or(XconError::Truncated)
        .map(|bytes| u32::from_be_bytes(bytes.try_into().expect("元素数宽度")) as usize)?;
    Ok((count, 4))
}

#[cfg(test)]
/// XCON 编解码、确定性和拒绝路径回归。
mod tests {
    use super::*;

    #[test]
    /// 标量、容器和嵌套字典可往返。
    fn xcon_round_trips_scalars_and_nested_values() {
        let value = XconValue::Dict(vec![
            (
                XconValue::Str("a".to_owned()),
                XconValue::Array(vec![XconValue::Bool(true), XconValue::Sint(-3)]),
            ),
            (XconValue::Str("b".to_owned()), XconValue::Int(2)),
        ]);
        let encoded = encode_xcon(&value).expect("XCON 编码");
        assert_eq!(decode_xcon(&encoded).expect("XCON 解码"), value);
        assert_eq!(&encoded[..4], b"XCON");
        assert_eq!(u16::from_be_bytes(encoded[4..6].try_into().unwrap()), 1);
    }

    #[test]
    /// 集合和字典编码按节点字节稳定排序。
    fn xcon_set_and_dict_encoding_is_deterministic() {
        let left = XconValue::Set(vec![XconValue::Int(2), XconValue::Int(1)]);
        let right = XconValue::Set(vec![XconValue::Int(1), XconValue::Int(2)]);
        assert_eq!(encode_xcon(&left), encode_xcon(&right));
    }

    #[test]
    /// 魔数和保留 tag 错误稳定。
    fn xcon_rejects_reserved_tags_and_bad_headers() {
        let value = XconValue::None;
        let mut encoded = encode_xcon(&value).expect("编码");
        encoded[0] = b"Y"[0];
        assert_eq!(decode_xcon(&encoded), Err(XconError::InvalidHeader));
        let mut reserved = encode_xcon(&value).expect("编码");
        reserved[10] = 13;
        assert_eq!(decode_xcon(&reserved), Err(XconError::UnsupportedTag(13)));
    }
}
