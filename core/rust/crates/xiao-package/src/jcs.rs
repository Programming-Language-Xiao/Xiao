//! RFC 8785 的 JSON 规范化；重复键及非 I-JSON 整数在解析时拒绝。

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use sha2::{Digest, Sha256};

use crate::diagnostics::SOURCE_INVALID_CODE;
use crate::source::SourceError;

/// I-JSON 安全整数上限，超出此范围的整数必须由源端编码为字符串。
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// 解析后保留数字及有序容器语义的内部 JSON 值。
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
}

/// 强制逐层检查重复键和整数范围的解析种子。
struct StrictJson;

impl<'de> DeserializeSeed<'de> for StrictJson {
    type Value = JsonValue;

    /// 将整棵 JSON 树交给严格访问器解析。
    fn deserialize<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(JsonVisitor)
    }
}

/// 对数组、字典及标量实施 I-JSON 约束。
struct JsonVisitor;

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = JsonValue;

    /// 描述当前 JSON 值的可接受形状。
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("不含重复键、非有限数值或不安全整数的 JSON")
    }

    /// 保存空值。
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(JsonValue::Null)
    }
    /// 保存布尔字面量。
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(JsonValue::Bool(value))
    }
    /// 保存借用字符串为独立值。
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(JsonValue::String(value.to_owned()))
    }
    /// 保存已分配的字符串值。
    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(JsonValue::String(value))
    }
    /// 拒绝绝对值超出安全范围的负整数。
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        if value.unsigned_abs() > MAX_SAFE_INTEGER {
            return Err(E::custom("不安全整数必须用字符串"));
        }
        Ok(JsonValue::Number(value as f64))
    }
    /// 拒绝超出安全范围的非负整数。
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        if value > MAX_SAFE_INTEGER {
            return Err(E::custom("不安全整数必须用字符串"));
        }
        Ok(JsonValue::Number(value as f64))
    }
    /// 只接受有限 IEEE-754 双精度浮点值。
    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        if !value.is_finite() {
            return Err(E::custom("非有限 JSON 数值"));
        }
        Ok(JsonValue::Number(value))
    }
    /// 递归读取数组元素。
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(StrictJson)? {
            values.push(value);
        }
        Ok(JsonValue::Array(values))
    }
    /// 递归读取对象，同时禁止同一对象内的重复键。
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!("重复 JSON 键 {key:?}")));
            }
            let value = map.next_value_seed(StrictJson)?;
            values.insert(key, value);
        }
        Ok(JsonValue::Object(values))
    }
}

/// 把输入解析为无重复键的 I-JSON 并输出 RFC 8785 规范 UTF-8 字节。
pub fn canonicalize_json(input: &str) -> Result<Vec<u8>, SourceError> {
    reject_unsafe_integer_tokens(input)?;
    let mut deserializer = serde_json::Deserializer::from_str(input);
    let value = StrictJson
        .deserialize(&mut deserializer)
        .map_err(|error| SourceError::new(SOURCE_INVALID_CODE, error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| SourceError::new(SOURCE_INVALID_CODE, error.to_string()))?;
    let mut output = String::new();
    write_value(&value, &mut output);
    Ok(output.into_bytes())
}

/// 计算规范 JSON 字节的 SHA-256（小写十六进制）。
pub fn jcs_digest(input: &str) -> Result<String, SourceError> {
    let bytes = canonicalize_json(input)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

/// 按 UTF-16 键序及 ECMAScript 数值格式写出规范值。
fn write_value(value: &JsonValue, output: &mut String) {
    match value {
        JsonValue::Null => output.push_str("null"),
        JsonValue::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        JsonValue::Number(value) => {
            if *value == 0.0 {
                output.push('0');
            } else if (1e-6..1e21).contains(&value.abs()) {
                output.push_str(&value.to_string());
            } else {
                let encoded = serde_json::to_string(value).expect("有限 f64 可序列化");
                let encoded = encoded.trim_end_matches(".0");
                output.push_str(encoded);
            }
        }
        JsonValue::String(value) => {
            output.push_str(&serde_json::to_string(value).expect("字符串可序列化"))
        }
        JsonValue::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_value(value, output);
            }
            output.push(']');
        }
        JsonValue::Object(values) => {
            output.push('{');
            let mut entries = values.iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.encode_utf16().cmp(right.encode_utf16()));
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&serde_json::to_string(key).expect("键可序列化"));
                output.push(':');
                write_value(value, output);
            }
            output.push('}');
        }
    }
}

/// 在 JSON 转为浮点数前拦截长度超出可表示范围的整数字面量。
fn reject_unsafe_integer_tokens(input: &str) -> Result<(), SourceError> {
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index += 2;
                } else if bytes[index] == b'"' {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
            continue;
        }
        if bytes[index] == b'-' || bytes[index].is_ascii_digit() {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_digit()
                    || matches!(bytes[index], b'.' | b'e' | b'E' | b'+' | b'-'))
            {
                index += 1;
            }
            let token = &input[start..index];
            if !token.contains(['.', 'e', 'E'])
                && token.parse::<i128>().map_or(true, |value| {
                    value.unsigned_abs() > u128::from(MAX_SAFE_INTEGER)
                })
            {
                return Err(SourceError::new(
                    SOURCE_INVALID_CODE,
                    "不安全整数必须用字符串",
                ));
            }
        } else {
            index += 1;
        }
    }
    Ok(())
}
