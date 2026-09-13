//! Xiao 源码位置与 UTF-8 处理的基础类型。
//!
//! 本 crate 只负责保存经过 UTF-8 校验的源码、计算稳定的位置和区间，
//! 不包含 Token、解析器或任何执行语义。字节偏移始终对应原始输入，
//! 行号与列号则提供面向诊断的逻辑坐标。

use std::fmt::{self, Display, Formatter};

/// 源文件不是合法 UTF-8 时使用的稳定诊断编号。
pub const INVALID_UTF8_CODE: &str = "X01-SOURCE-001";

/// 位置偏移超出源码范围时使用的稳定诊断编号。
pub const OFFSET_OUT_OF_BOUNDS_CODE: &str = "X01-SOURCE-002";

/// 偏移不在 UTF-8 字符边界时使用的稳定诊断编号。
pub const NOT_CHAR_BOUNDARY_CODE: &str = "X01-SOURCE-003";

/// 源码区间无效时使用的稳定诊断编号。
pub const INVALID_SPAN_CODE: &str = "X01-SOURCE-004";

/// Xiao 源码中的一基位置。
///
/// `offset` 是原始 UTF-8 字节偏移；`line` 和 `column` 都从 1 开始。
/// 列号按 Unicode 标量值计数，因此一个中文字符占一列。Tab 在词法器
/// 的缩进判断中会展开为四个空格，但位置模型仍将原始 Tab 记为一个
/// Unicode 标量，以便位置可以无损映射回输入字节。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourcePosition {
    /// 原始 UTF-8 字节偏移。
    pub offset: usize,
    /// 一基行号。
    pub line: usize,
    /// 一基 Unicode 标量列号。
    pub column: usize,
}

impl SourcePosition {
    /// 创建一个位置值。
    #[must_use]
    pub const fn new(offset: usize, line: usize, column: usize) -> Self {
        Self {
            offset,
            line,
            column,
        }
    }
}

/// Xiao 源码中的半开字节区间 `[start, end)`。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceSpan {
    start: usize,
    end: usize,
}

impl SourceSpan {
    /// 创建一个区间；当 `start > end` 时返回 `None`。
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Option<Self> {
        if start > end {
            None
        } else {
            Some(Self { start, end })
        }
    }

    /// 返回区间起始字节偏移。
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    /// 返回区间结束字节偏移（不包含）。
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }

    /// 返回区间包含的字节长度。
    #[must_use]
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    /// 判断区间是否为空。
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// 源码读取或位置查询失败的原因。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceError {
    /// 输入不是合法的 UTF-8。
    InvalidUtf8 {
        /// 第一个无效字节的偏移。
        valid_up_to: usize,
        /// 无效序列长度；无法确定时为空。
        error_len: Option<usize>,
    },
    /// 字节偏移超过源码长度。
    OffsetOutOfBounds {
        /// 请求的偏移。
        offset: usize,
        /// 源码字节长度。
        length: usize,
    },
    /// 偏移落在 UTF-8 多字节字符的中间。
    NotCharBoundary {
        /// 不合法的偏移。
        offset: usize,
    },
    /// 区间不是合法的源码边界。
    InvalidSpan {
        /// 区间起点。
        start: usize,
        /// 区间终点。
        end: usize,
    },
}

impl SourceError {
    /// 返回错误对应的稳定机器编号。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidUtf8 { .. } => INVALID_UTF8_CODE,
            Self::OffsetOutOfBounds { .. } => OFFSET_OUT_OF_BOUNDS_CODE,
            Self::NotCharBoundary { .. } => NOT_CHAR_BOUNDARY_CODE,
            Self::InvalidSpan { .. } => INVALID_SPAN_CODE,
        }
    }
}

impl Display for SourceError {
    /// 将源码错误格式化为稳定、便于调试的文本。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8 {
                valid_up_to,
                error_len,
            } => write!(
                formatter,
                "invalid UTF-8 at byte {valid_up_to} (length: {error_len:?})"
            ),
            Self::OffsetOutOfBounds { offset, length } => {
                write!(
                    formatter,
                    "byte offset {offset} is outside source length {length}"
                )
            }
            Self::NotCharBoundary { offset } => {
                write!(
                    formatter,
                    "byte offset {offset} is not a UTF-8 character boundary"
                )
            }
            Self::InvalidSpan { start, end } => {
                write!(formatter, "invalid source span [{start}, {end})")
            }
        }
    }
}

impl std::error::Error for SourceError {}

/// 经过 UTF-8 校验并建立行索引的不可变源码。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile {
    text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    /// 从已经拥有的 UTF-8 字符串建立源码对象。
    #[must_use]
    pub fn new(text: String) -> Self {
        let line_starts = compute_line_starts(&text);
        Self { text, line_starts }
    }

    /// 从 UTF-8 字节建立源码对象。
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self, SourceError> {
        std::str::from_utf8(bytes.as_ref())
            .map(Self::from_text)
            .map_err(|error| SourceError::InvalidUtf8 {
                valid_up_to: error.valid_up_to(),
                error_len: error.error_len(),
            })
    }

    /// 从字符串切片建立源码对象。
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        Self::new(text.to_owned())
    }

    /// 返回完整源码文本。
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// 返回源码的原始 UTF-8 字节视图。
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.text.as_bytes()
    }

    /// 返回源码字节长度。
    #[must_use]
    pub fn len_bytes(&self) -> usize {
        self.text.len()
    }

    /// 判断源码是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// 返回逻辑行数；空文件也包含一行空行。
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// 将字节偏移转换为一基行列位置。
    pub fn position_at(&self, offset: usize) -> Result<SourcePosition, SourceError> {
        if offset > self.text.len() {
            return Err(SourceError::OffsetOutOfBounds {
                offset,
                length: self.text.len(),
            });
        }
        if !self.text.is_char_boundary(offset) {
            return Err(SourceError::NotCharBoundary { offset });
        }
        let line_index = self
            .line_starts
            .partition_point(|&line_start| line_start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line_index];
        let column = scalar_column(&self.text, line_start, offset);
        Ok(SourcePosition::new(offset, line_index + 1, column))
    }

    /// 验证并创建一个属于本源码的半开区间。
    pub fn span(&self, start: usize, end: usize) -> Result<SourceSpan, SourceError> {
        if start > end || end > self.text.len() {
            return Err(SourceError::InvalidSpan { start, end });
        }
        if !self.text.is_char_boundary(start) {
            return Err(SourceError::NotCharBoundary { offset: start });
        }
        if !self.text.is_char_boundary(end) {
            return Err(SourceError::NotCharBoundary { offset: end });
        }
        // `start <= end` 且两端都经过边界检查，因此这里必然成功。
        Ok(SourceSpan::new(start, end).expect("validated source span ordering"))
    }

    /// 读取一个已经验证的源码区间。
    #[must_use]
    pub fn slice(&self, span: SourceSpan) -> &str {
        &self.text[span.start..span.end]
    }

    /// 返回指定逻辑行的起始字节偏移（行号从 1 开始）。
    pub fn line_start(&self, line: usize) -> Option<usize> {
        self.line_starts.get(line.checked_sub(1)?).copied()
    }
}

impl std::str::FromStr for SourceFile {
    type Err = std::convert::Infallible;

    /// 从 UTF-8 文本解析源码对象；字符串切片本身已经保证合法 UTF-8。
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_text(text))
    }
}

/// 面向词法器和后续编辑器的源码游标。
pub struct SourceCursor<'source> {
    source: &'source SourceFile,
    offset: usize,
}

impl<'source> SourceCursor<'source> {
    /// 创建位于源码开头的游标。
    #[must_use]
    pub fn new(source: &'source SourceFile) -> Self {
        Self { source, offset: 0 }
    }

    /// 返回当前字节偏移。
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// 返回当前源码位置。
    pub fn position(&self) -> Result<SourcePosition, SourceError> {
        self.source.position_at(self.offset)
    }

    /// 判断游标是否到达源码末尾。
    #[must_use]
    pub fn is_eof(&self) -> bool {
        self.offset >= self.source.len_bytes()
    }

    /// 查看当前 Unicode 标量但不移动游标。
    #[must_use]
    pub fn peek(&self) -> Option<char> {
        self.source.text()[self.offset..].chars().next()
    }

    /// 消费并返回当前 Unicode 标量。
    pub fn bump(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.offset += character.len_utf8();
        Some(character)
    }

    /// 返回从 `start` 到当前位置的源码区间。
    pub fn span_from(&self, start: usize) -> Result<SourceSpan, SourceError> {
        self.source.span(start, self.offset)
    }
}

impl std::fmt::Debug for SourceCursor<'_> {
    /// 输出便于调试的游标状态。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SourceCursor")
            .field("offset", &self.offset)
            .field("is_eof", &self.is_eof())
            .finish()
    }
}

/// 计算一行内的 Unicode 标量列号，并将 CRLF 的两个字节视为一个换行。
fn scalar_column(text: &str, line_start: usize, offset: usize) -> usize {
    let mut column = 1;
    let mut cursor = line_start;
    while cursor < offset {
        if text.as_bytes().get(cursor) == Some(&b'\r')
            && text.as_bytes().get(cursor + 1) == Some(&b'\n')
        {
            break;
        }
        let character = text[cursor..]
            .chars()
            .next()
            .expect("位置必须位于有效 UTF-8 边界");
        cursor += character.len_utf8();
        column += 1;
    }
    column
}

/// 计算每个逻辑行的起始字节偏移。
fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    let bytes = text.as_bytes();
    let mut offset = 0;
    while offset < bytes.len() {
        match bytes[offset] {
            b'\r' if bytes.get(offset + 1) == Some(&b'\n') => {
                offset += 2;
                starts.push(offset);
            }
            b'\n' => {
                offset += 1;
                starts.push(offset);
            }
            _ => {
                let character = text[offset..]
                    .chars()
                    .next()
                    .expect("offset remains inside valid UTF-8 text");
                offset += character.len_utf8();
            }
        }
    }
    starts
}

#[cfg(test)]
/// 覆盖源码位置、UTF-8 校验和游标边界的单元测试。
mod tests {
    use super::{SourceError, SourceFile, SourcePosition, SourceSpan};

    #[test]
    /// 确认多字节字符同时保留字节偏移和标量列号。
    fn tracks_utf8_bytes_and_scalar_columns() {
        let source = SourceFile::from_text("a\n星崽");
        assert_eq!(source.len_bytes(), 8);
        assert_eq!(
            source.position_at(2).expect("位置应有效"),
            SourcePosition::new(2, 2, 1)
        );
        assert_eq!(
            source.position_at(5).expect("位置应有效"),
            SourcePosition::new(5, 2, 2)
        );
    }

    #[test]
    /// 确认 CRLF 只产生一个逻辑行边界。
    fn treats_crlf_as_one_logical_line_break() {
        let source = SourceFile::from_text("a\r\nb");
        assert_eq!(source.line_count(), 2);
        assert_eq!(source.line_start(2), Some(3));
        assert_eq!(
            source.position_at(2).expect("CRLF 内部位置应有效").column,
            2
        );
        assert_eq!(source.position_at(3).expect("位置应有效").line, 2);
    }

    #[test]
    /// 确认尾部换行会留下可定位的空逻辑行。
    fn preserves_trailing_empty_line_after_newline() {
        let source = SourceFile::from_text("a\n");
        assert_eq!(source.line_count(), 2);
        assert_eq!(source.position_at(2).expect("EOF 位置应有效").line, 2);
        assert_eq!(source.position_at(2).expect("EOF 位置应有效").column, 1);
    }

    #[test]
    /// 确认非法 UTF-8 和多字节字符内部偏移都会被拒绝。
    fn rejects_invalid_utf8_and_bad_boundaries() {
        let error =
            SourceFile::from_bytes(vec![0xf0, 0x28, 0x8c, 0xbc]).expect_err("应拒绝非法 UTF-8");
        assert!(matches!(error, SourceError::InvalidUtf8 { .. }));
        assert_eq!(error.code(), super::INVALID_UTF8_CODE);

        let source = SourceFile::from_text("星");
        assert!(matches!(
            source.position_at(1),
            Err(SourceError::NotCharBoundary { offset: 1 })
        ));
    }

    #[test]
    /// 确认区间验证和游标消费都能稳定前进。
    fn validates_spans_and_cursor_progress() {
        let source = SourceFile::from_text("ab");
        let span = source.span(0, 1).expect("区间应有效");
        assert_eq!(span, SourceSpan::new(0, 1).expect("区间应有效"));
        assert_eq!(source.slice(span), "a");
        assert!(source.span(2, 1).is_err());

        let mut cursor = super::SourceCursor::new(&source);
        assert_eq!(cursor.peek(), Some('a'));
        assert_eq!(cursor.bump(), Some('a'));
        assert_eq!(cursor.offset(), 1);
        assert_eq!(cursor.bump(), Some('b'));
        assert!(cursor.is_eof());
    }
}
