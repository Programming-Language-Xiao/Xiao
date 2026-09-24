//! 不可执行配置树的数据模型。
//!
//! 模型只保存已经解码的字面量和源码区间，不包含 Xiao `Statement`、表达式或
//! 可调用对象。表和键使用确定性的 `BTreeMap`，便于 CLI、包管理器和缓存生成
//! 得到稳定结果。

use std::collections::BTreeMap;

use xiao_source::SourceSpan;

/// 配置文档的根节点。
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigDocument {
    /// 按规范化表名保存的配置表。
    pub tables: BTreeMap<String, ConfigTable>,
    /// 覆盖整个输入文件的源码区间。
    pub span: SourceSpan,
}

impl ConfigDocument {
    /// 创建一份配置文档。
    #[must_use]
    pub fn new(tables: BTreeMap<String, ConfigTable>, span: SourceSpan) -> Self {
        Self { tables, span }
    }

    /// 按大小写规范化后的表名读取表。
    #[must_use]
    pub fn table(&self, name: &str) -> Option<&ConfigTable> {
        self.tables.get(&name.to_ascii_lowercase())
    }

    /// 返回所有表的确定性迭代器。
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ConfigTable)> {
        self.tables.iter()
    }

    /// 返回文档源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// 返回不含源码区间的版本化类型化指纹输入。
    ///
    /// 该编码使用明确的类型标签、元素数量和字节长度边界，不是 RFC 8785/JCS。
    /// 表、键和字典均依赖 `BTreeMap` 的确定性顺序；源码位置不会进入编码。
    #[must_use]
    pub fn canonical_fingerprint_input(&self) -> Vec<u8> {
        let mut output = Vec::new();
        output.extend_from_slice(b"xiao-config-fingerprint-v1\0");
        append_len(&mut output, self.tables.len());
        for (table_key, table) in &self.tables {
            append_text(&mut output, table_key);
            append_text(&mut output, &table.name);
            append_len(&mut output, table.entries.len());
            for (entry_key, entry) in &table.entries {
                append_text(&mut output, entry_key);
                append_text(&mut output, &entry.key);
                append_value(&mut output, &entry.value);
            }
        }
        output
    }
}

/// 一个顶层配置表。
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigTable {
    /// 规范化后的表名。
    pub name: String,
    /// 按规范化键名保存的条目。
    pub entries: BTreeMap<String, ConfigEntry>,
    /// 覆盖表头及其成员的源码区间。
    pub span: SourceSpan,
}

impl ConfigTable {
    /// 创建一份配置表。
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        entries: BTreeMap<String, ConfigEntry>,
        span: SourceSpan,
    ) -> Self {
        Self {
            name: name.into(),
            entries,
            span,
        }
    }

    /// 按键名读取配置条目。
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&ConfigEntry> {
        self.entries.get(key)
    }

    /// 返回表成员的确定性迭代器。
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ConfigEntry)> {
        self.entries.iter()
    }

    /// 返回表源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// 一个键值配置条目。
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigEntry {
    /// 规范化后的键名。
    pub key: String,
    /// 不可执行的配置值。
    pub value: ConfigValue,
    /// 从键起点到值末尾的源码区间。
    pub span: SourceSpan,
}

impl ConfigEntry {
    /// 创建一份配置条目。
    #[must_use]
    pub fn new(key: impl Into<String>, value: ConfigValue, span: SourceSpan) -> Self {
        Self {
            key: key.into(),
            value,
            span,
        }
    }
}

/// 配置值的静态种类。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConfigValueKind {
    /// 字符串字面量。
    String,
    /// 整数字面量。
    Integer,
    /// 浮点字面量。
    Float,
    /// 布尔字面量。
    Boolean,
    /// 数组字面量。
    Array,
    /// 字典表字面量。
    Dictionary,
}

impl ConfigValueKind {
    /// 返回稳定的 Xiao 类型名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::String => "str",
            Self::Integer => "int",
            Self::Float => "float",
            Self::Boolean => "bool",
            Self::Array => "array",
            Self::Dictionary => "dict",
        }
    }
}

/// 配置允许的不可执行值。
#[derive(Clone, Debug, PartialEq)]
pub enum ConfigValue {
    /// UTF-8 字符串值。
    String(String),
    /// 任意精度范围内可被 `i128` 表示的整数值。
    Integer(i128),
    /// 有限 IEEE-754 双精度浮点值。
    Float(f64),
    /// 布尔值。
    Boolean(bool),
    /// 递归数组值。
    Array(Vec<ConfigValue>),
    /// 递归字典表值；键已经解码为字符串。
    Dictionary(BTreeMap<String, ConfigValue>),
}

impl ConfigValue {
    /// 返回值的静态种类。
    #[must_use]
    pub const fn kind(&self) -> ConfigValueKind {
        match self {
            Self::String(_) => ConfigValueKind::String,
            Self::Integer(_) => ConfigValueKind::Integer,
            Self::Float(_) => ConfigValueKind::Float,
            Self::Boolean(_) => ConfigValueKind::Boolean,
            Self::Array(_) => ConfigValueKind::Array,
            Self::Dictionary(_) => ConfigValueKind::Dictionary,
        }
    }

    /// 尝试读取字符串引用。
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    /// 尝试读取整数值。
    #[must_use]
    pub const fn as_integer(&self) -> Option<i128> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// 尝试读取浮点值。
    #[must_use]
    pub const fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(value) => Some(*value),
            _ => None,
        }
    }

    /// 尝试读取布尔值。
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    /// 尝试读取数组引用。
    #[must_use]
    pub fn as_array(&self) -> Option<&[ConfigValue]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    /// 尝试读取字典引用。
    #[must_use]
    pub fn as_dictionary(&self) -> Option<&BTreeMap<String, ConfigValue>> {
        match self {
            Self::Dictionary(values) => Some(values),
            _ => None,
        }
    }
}

/// 当前已通过配置校验的规范化文档类型。
pub type NormalizedConfig = ConfigDocument;

/// 以固定宽度大端整数追加长度边界。
fn append_len(output: &mut Vec<u8>, length: usize) {
    output.extend_from_slice(
        &u64::try_from(length)
            .expect("配置树长度必须能表示为 64 位无符号整数")
            .to_be_bytes(),
    );
}

/// 追加带长度边界的 UTF-8 文本。
fn append_text(output: &mut Vec<u8>, text: &str) {
    append_len(output, text.len());
    output.extend_from_slice(text.as_bytes());
}

/// 追加带类型标签的静态配置值。
fn append_value(output: &mut Vec<u8>, value: &ConfigValue) {
    match value {
        ConfigValue::String(value) => {
            output.push(1);
            append_text(output, value);
        }
        ConfigValue::Integer(value) => {
            output.push(2);
            output.extend_from_slice(&value.to_be_bytes());
        }
        ConfigValue::Float(value) => {
            output.push(3);
            output.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        ConfigValue::Boolean(value) => {
            output.push(4);
            output.push(u8::from(*value));
        }
        ConfigValue::Array(values) => {
            output.push(5);
            append_len(output, values.len());
            for value in values {
                append_value(output, value);
            }
        }
        ConfigValue::Dictionary(values) => {
            output.push(6);
            append_len(output, values.len());
            for (key, value) in values {
                append_text(output, key);
                append_value(output, value);
            }
        }
    }
}
