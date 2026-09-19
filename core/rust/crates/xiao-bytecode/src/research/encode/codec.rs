//! 研究编码格式的底层字节读写原语。

use super::{EncodeError, MAX_COLLECTION, MAX_STRING, OperandWidth};

/// 写可选索引：`None` 写一个 0 字节，`Some` 先写 1 再按当前宽度写编号。
///
/// 不用「0 表示空」是因为 0 是合法编号——0 号常量、0 号函数（脚本入口）、
/// 0 号寄存器都存在，用 0 当哨兵会把一个真实引用变成「没有」。
pub(super) fn write_optional_index(
    writer: &mut Writer,
    value: Option<u32>,
) -> Result<(), EncodeError> {
    match value {
        Some(value) => {
            writer.byte(1);
            writer.index(value, "optional_index")?;
        }
        None => writer.byte(0),
    }
    Ok(())
}

/// 写可选字符串：`None` 写一个 0 字节，`Some` 先写 1 再写长度前缀文本。
///
/// 同样不能拿空串当哨兵：`Some("")`（比如无名的关键字实参、空的捕获类型名）与
/// `None`（根本没有这一项）在模型里是不同的值。
pub(super) fn write_optional_string(
    writer: &mut Writer,
    value: Option<&str>,
) -> Result<(), EncodeError> {
    match value {
        Some(value) => {
            writer.byte(1);
            writer.string(value)?;
        }
        None => writer.byte(0),
    }
    Ok(())
}

/// 写可选长度：`None` 写一个 0 字节，`Some` 先写 1 再写 uleb。
///
/// 与 [`write_optional_index`] 分开，是因为这里的载荷**固定用 uleb**，不跟随头部
/// 的操作数宽度：可选长度只出现在类型形状里（数组长度），而定宽策略只约束寄存器
/// 号与表索引。
pub(super) fn write_optional_usize(
    writer: &mut Writer,
    value: Option<usize>,
) -> Result<(), EncodeError> {
    match value {
        Some(value) => {
            writer.byte(1);
            writer.uleb(value as u64);
        }
        None => writer.byte(0),
    }
    Ok(())
}

/// 读一个布尔标志，只接受 0 和 1。
///
/// 其他任何值都报 [`EncodeError::InvalidEnum`]，不按「非零即真」处理：把 2 当作
/// `true` 会把一个已经损坏的字段静默吞掉，后面再想定位就无从下手。
pub(super) fn read_bool(reader: &mut Reader<'_>, field: &'static str) -> Result<bool, EncodeError> {
    match reader.byte(field)? {
        0 => Ok(false),
        1 => Ok(true),
        value => Err(EncodeError::InvalidEnum {
            field: field.to_owned(),
            value: value as u64,
        }),
    }
}

/// 编码写入器：累积字节串，并记住本次编码的操作数宽度。
///
/// 宽度存在写入器里而不是每个 [`Writer::index`] 调用点各传一次，是为了让一段
/// 字节里**不可能**混进两种宽度的编号——解码端只有头部一个宽度标签，混写就是
/// 不可解。
pub(super) struct Writer {
    /// 已写出的字节。
    pub(super) bytes: Vec<u8>,
    /// 本次编码使用的操作数宽度，写编号时生效。
    pub(super) width: OperandWidth,
}

impl Writer {
    /// 建一个空写入器，绑定本次编码的宽度。
    ///
    /// 是 `const`，方便在常量语境（比如块级写入器的初始化）里构造。
    pub(super) const fn new(width: OperandWidth) -> Self {
        Self {
            bytes: Vec::new(),
            width,
        }
    }

    /// 写一个原始字节（标签、布尔、opcode 都用它）。
    ///
    /// 不做长度或范围检查：能走到这里的值都已经由调用方决定了语义，检查放在
    /// 有字段名可用的一层（比如 [`Writer::index`]）才报得清楚。
    pub(super) fn byte(&mut self, value: u8) {
        self.bytes.push(value);
    }

    /// 写无符号 LEB128：每字节取低 7 位，最高位表示「后面还有」。
    ///
    /// 0 写成一个 `0x00`，即最小表示唯一——不写补零的冗余形式，否则同一份语义
    /// 会有多种字节表示，往返测试也就失去意义。
    pub(super) fn uleb(&mut self, mut value: u64) {
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            self.bytes.push(byte);
            if value == 0 {
                break;
            }
        }
    }

    /// 写有符号 LEB128：低 7 位加符号位，终止条件是「剩余位全 0（正）或全 1（负）」。
    ///
    /// 取 `i128` 而不是更窄的整数，是因为源码区间的增量可以横跨整个 `usize`
    /// 范围；按 `i64` 实现会在 64 位平台上截断极大的区间差。
    pub(super) fn sleb(&mut self, mut value: i128) {
        loop {
            let byte = (value as u8) & 0x7f;
            let sign = (byte & 0x40) != 0;
            value >>= 7;
            let done = (value == 0 && !sign) || (value == -1 && sign);
            self.bytes.push(if done { byte } else { byte | 0x80 });
            if done {
                break;
            }
        }
    }

    /// 按当前宽度写一个寄存器号或表索引。
    ///
    /// `FixedU16` 下超出 `u16` 范围直接报 [`EncodeError::IntegerOverflow`] 并带上
    /// `field`。这正是定宽操作数的契约：编号放不下时必须让编码失败，而不是截断
    /// 成一个指向别的表项的合法编号。
    pub(super) fn index(&mut self, value: u32, field: &str) -> Result<(), EncodeError> {
        match self.width {
            OperandWidth::Leb128 => {
                self.uleb(value as u64);
                Ok(())
            }
            OperandWidth::FixedU16 => {
                let value = u16::try_from(value).map_err(|_| EncodeError::IntegerOverflow {
                    field: field.to_owned(),
                    value: value as u64,
                })?;
                self.bytes.extend_from_slice(&value.to_le_bytes());
                Ok(())
            }
        }
    }

    /// 写长度前缀加 UTF-8 字节串。
    ///
    /// 长度是**字节数**不是字符数，所以中文、含 `\0` 的内容都能原样往返（`\0`
    /// 在源码字符串里是合法字符，不是终止符）。超过 [`MAX_STRING`] 拒绝。
    pub(super) fn string(&mut self, value: &str) -> Result<(), EncodeError> {
        let bytes = value.as_bytes();
        if bytes.len() as u64 > MAX_STRING {
            return Err(EncodeError::InvalidLength {
                field: "string".to_owned(),
                value: bytes.len() as u64,
            });
        }
        self.uleb(bytes.len() as u64);
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    /// 写一个集合数量或字节长度。
    ///
    /// 超过 [`MAX_COLLECTION`] 拒绝：写入方向也卡这个上限，是为了保证「能编出来
    /// 的字节一定能解回来」——解码端有同样的上限，写入端不卡就会产出自己读不了
    /// 的编码。
    pub(super) fn count(&mut self, count: usize, field: &str) -> Result<(), EncodeError> {
        if count as u64 > MAX_COLLECTION {
            return Err(EncodeError::InvalidLength {
                field: field.to_owned(),
                value: count as u64,
            });
        }
        self.uleb(count as u64);
        Ok(())
    }
}

/// 解码读取器：一个字节切片加当前消费位置。
///
/// **不保存操作数宽度**：宽度是头部的属性，在每次 [`Reader::index`] /
/// [`Reader::optional_index`] 调用点显式传入。这样同一个读取器既能读入口流
/// （头部自带宽度标签），也能读块字节这类「宽度由外层决定」的子串。
pub(super) struct Reader<'a> {
    /// 剩余待消费的字节。
    pub(super) bytes: &'a [u8],
    /// 已消费的字节数，同时是下一条指令的函数内 pc 基准。
    pub(super) offset: usize,
}

impl<'a> Reader<'a> {
    /// 从字节切片从头开始读。
    ///
    /// 适合整段入口流：头部自己带着宽度标签，后续每次读取再显式传入宽度，因此
    /// 这里不需要挑一个默认宽度。
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self::with_width(bytes, OperandWidth::Leb128)
    }

    /// 带宽度标注的构造入口。
    ///
    /// 当前实现**刻意忽略该参数**：读取器不持有宽度，宽度一律在读取点传入。
    /// 保留这个入口是为了让「这段子串是按哪种宽度写的」在构造处写清楚，读代码的
    /// 人不必回头去追头部。
    pub(super) fn with_width(bytes: &'a [u8], _width: OperandWidth) -> Self {
        Self { bytes, offset: 0 }
    }

    /// 判断是否已消费到末尾。
    ///
    /// 整个编码流读完后必须为空，否则 [`decode_inner`] 会报 `TrailingBytes`。
    pub(super) fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }

    /// 返回尚未消费的字节数。
    ///
    /// 用于 `TrailingBytes` 诊断，也被 [`Reader::count`] 当作「这个长度有没有可能
    /// 装得下」的粗筛上界。
    pub(super) fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    /// 读一个字节，越界报 [`EncodeError::UnexpectedEof`]，并带上 `context` 字段名，
    /// 让截断的位置可定位。
    pub(super) fn byte(&mut self, context: &'static str) -> Result<u8, EncodeError> {
        let value = *self
            .bytes
            .get(self.offset)
            .ok_or_else(|| EncodeError::UnexpectedEof {
                context: context.to_owned(),
            })?;
        self.offset += 1;
        Ok(value)
    }

    /// 读走恰好 `length` 个字节并返回借用切片。
    ///
    /// `offset + length` 用 `checked_add`：长度是从输入里读出来的，可以大到让
    /// `usize` 回绕；回绕后的上界会落在切片内，于是「越界」变成一次成功的读取。
    /// 越界统一报 [`EncodeError::UnexpectedEof`]，与 `byte` 保持一致。
    pub(super) fn take_exact(
        &mut self,
        length: usize,
        context: &'static str,
    ) -> Result<&'a [u8], EncodeError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| EncodeError::InvalidLength {
                field: context.to_owned(),
                value: length as u64,
            })?;
        if end > self.bytes.len() {
            return Err(EncodeError::UnexpectedEof {
                context: context.to_owned(),
            });
        }
        let result = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(result)
    }

    /// 读定宽 `N` 字节并转成数组，供整数/浮点按小端还原。
    ///
    /// `N` 由调用点从常量给出（整数 8/4、定宽操作数 2），所以长度不足只可能是
    /// 输入被截断，报 [`EncodeError::UnexpectedEof`]。
    pub(super) fn fixed<const N: usize>(
        &mut self,
        context: &'static str,
    ) -> Result<[u8; N], EncodeError> {
        self.take_exact(N, context)?
            .try_into()
            .map_err(|_| EncodeError::UnexpectedEof {
                context: context.to_owned(),
            })
    }

    /// 读无符号 LEB128，上限 10 字节（`u64` 的宽度）。
    ///
    /// 每步都检查位移：位移到 64 位以外、或最后一个可用字节带多余高位时报
    /// [`EncodeError::IntegerOverflow`]。不查的话多出来的位会被 `<<` 静默丢弃，
    /// 读出一个比实际小的数——而它多半会被当成合法的表索引或长度用下去。
    pub(super) fn uleb(&mut self, context: &'static str) -> Result<u64, EncodeError> {
        let mut value = 0_u64;
        let mut shift = 0_u32;
        for _ in 0..10 {
            let byte = self.byte(context)?;
            let part = (byte & 0x7f) as u64;
            if shift >= 64 || (shift == 63 && part > 1) {
                return Err(EncodeError::IntegerOverflow {
                    field: context.to_owned(),
                    value: u64::MAX,
                });
            }
            value |= part << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
        }
        Err(EncodeError::IntegerOverflow {
            field: context.to_owned(),
            value: u64::MAX,
        })
    }

    /// 读有符号 LEB128，按 `i128` 累积，最多 19 字节。
    ///
    /// 第 19 字节只剩 2 个有效位（`shift == 126`），此时还要检查它的其余位是不是
    /// 合法的符号扩展（正数只能剩 `0x00`/`0x01`，负数只能剩 `0x7e`/`0x7f`），
    /// 否则报 [`EncodeError::IntegerOverflow`]：不做这一步，一段超宽的编码会被
    /// 截成一个「看起来正常」的小区间。
    ///
    /// 用 `i128` 而不是 `i64`，是因为源码区间增量要能覆盖整个 `usize` 范围。
    pub(super) fn sleb(&mut self, context: &'static str) -> Result<i128, EncodeError> {
        let mut bits = 0_u128;
        let mut shift = 0_u32;
        for index in 0..19 {
            let byte = self.byte(context)?;
            let payload = byte & 0x7f;
            if shift == 126 {
                let valid = if byte & 0x40 == 0 {
                    payload <= 1
                } else {
                    payload >= 0x7e
                };
                if !valid {
                    return Err(EncodeError::IntegerOverflow {
                        field: context.to_owned(),
                        value: u64::MAX,
                    });
                }
                bits |= u128::from(payload & 0x03) << shift;
            } else if shift < 126 {
                bits |= u128::from(payload) << shift;
            } else {
                return Err(EncodeError::IntegerOverflow {
                    field: context.to_owned(),
                    value: u64::MAX,
                });
            }
            shift += 7;
            if byte & 0x80 == 0 {
                if byte & 0x40 != 0 && shift < 128 {
                    bits |= (!0_u128) << shift;
                }
                return Ok(bits as i128);
            }
            if index == 18 {
                break;
            }
        }
        Err(EncodeError::IntegerOverflow {
            field: context.to_owned(),
            value: u64::MAX,
        })
    }

    /// 读 uleb 并收窄到 `u32`（作用域号、各类版本号用）。
    ///
    /// 放不进 `u32` 时报 [`EncodeError::IntegerOverflow`]，而不是截断取低 32 位。
    pub(super) fn u32_uleb(&mut self, context: &'static str) -> Result<u32, EncodeError> {
        u32::try_from(self.uleb(context)?).map_err(|_| EncodeError::IntegerOverflow {
            field: context.to_owned(),
            value: u64::MAX,
        })
    }

    /// 读 uleb 并收窄到 `usize`。
    ///
    /// 用于 [`super::lower::TacReleaseAction::order`] 这类「宽度跟宿主走」的字段，
    /// 与 [`Reader::u32_uleb`] 分开写是为了让收窄目标在调用点一眼可辨。
    pub(super) fn usize_uleb(&mut self, context: &'static str) -> Result<usize, EncodeError> {
        usize::try_from(self.uleb(context)?).map_err(|_| EncodeError::IntegerOverflow {
            field: context.to_owned(),
            value: u64::MAX,
        })
    }

    /// 读一个集合数量，三重校验后才交给调用方去预留容量。
    ///
    /// 1. 不超过 [`MAX_COLLECTION`]；
    /// 2. 能收窄到 `usize`；
    /// 3. 不超过剩余字节数加一——每个条目至少要占一个字节，超过这个上界说明长度
    ///    是伪造的。
    ///
    /// 第 3 条是防「几字节输入骗出巨大分配」的主要手段：前两条都只是定值上限，
    /// 只有拿剩余长度当上界才真正和输入规模挂钩。
    pub(super) fn count(&mut self, context: &'static str) -> Result<usize, EncodeError> {
        let value = self.uleb(context)?;
        if value > MAX_COLLECTION {
            return Err(EncodeError::InvalidLength {
                field: context.to_owned(),
                value,
            });
        }
        let count = usize::try_from(value).map_err(|_| EncodeError::InvalidLength {
            field: context.to_owned(),
            value,
        })?;
        if count > self.remaining() + 1 {
            return Err(EncodeError::InvalidLength {
                field: context.to_owned(),
                value,
            });
        }
        Ok(count)
    }

    /// 读长度前缀加 UTF-8 字节串。
    ///
    /// 长度先卡 [`MAX_STRING`] 再收窄成 `usize` 才取字节。非 UTF-8 报
    /// [`EncodeError::InvalidFormat`] 而不是替换成 U+FFFD：静默替换会改掉标识符
    /// 和字符串常量的内容，而调用方拿到的仍是一个「成功」的结果。
    pub(super) fn string(&mut self, context: &'static str) -> Result<String, EncodeError> {
        let length = self.uleb(context)?;
        if length > MAX_STRING {
            return Err(EncodeError::InvalidLength {
                field: context.to_owned(),
                value: length,
            });
        }
        let bytes = self.take_exact(
            usize::try_from(length).map_err(|_| EncodeError::InvalidLength {
                field: context.to_owned(),
                value: length,
            })?,
            context,
        )?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| EncodeError::InvalidFormat(format!("{context} 不是合法 UTF-8")))
    }

    /// 读可选字符串，标志字节只接受 0/1（其他值报 [`EncodeError::InvalidEnum`]）。
    ///
    /// 与 [`write_optional_string`] 对称，`None` 和 `Some("")` 保持可区分。
    pub(super) fn optional_string(
        &mut self,
        context: &'static str,
    ) -> Result<Option<String>, EncodeError> {
        match self.byte(context)? {
            0 => Ok(None),
            1 => Ok(Some(self.string(context)?)),
            value => Err(EncodeError::InvalidEnum {
                field: context.to_owned(),
                value: value as u64,
            }),
        }
    }

    /// 按给定宽度读一个寄存器号或表索引。
    ///
    /// 这是「两种操作数宽度」在解码侧唯一的分叉点：`Leb128` 走 uleb，`FixedU16`
    /// 读小端两字节。宽度由调用方传入而不是从 `self` 取，见 [`Reader`] 的说明。
    pub(super) fn index(
        &mut self,
        width: OperandWidth,
        context: &'static str,
    ) -> Result<u32, EncodeError> {
        match width {
            OperandWidth::Leb128 => self.u32_uleb(context),
            OperandWidth::FixedU16 => Ok(u16::from_le_bytes(self.fixed::<2>(context)?) as u32),
        }
    }

    /// 读可选编号：标志字节只接受 0/1（其他值报 [`EncodeError::InvalidEnum`]），
    /// 随后按给定宽度读编号。
    ///
    /// 与 [`write_optional_index`] 对称，`None` 与编号 0 保持可区分。
    pub(super) fn optional_index(
        &mut self,
        width: OperandWidth,
        context: &'static str,
    ) -> Result<Option<u32>, EncodeError> {
        match self.byte(context)? {
            0 => Ok(None),
            1 => Ok(Some(self.index(width, context)?)),
            value => Err(EncodeError::InvalidEnum {
                field: context.to_owned(),
                value: value as u64,
            }),
        }
    }

    /// 读可选长度（固定用 uleb）。
    ///
    /// 与 [`Reader::optional_index`] 分开，因为可选长度不参与定宽操作数策略；
    /// 目前唯一的用处是数组形状里的长度。
    pub(super) fn optional_usize(
        &mut self,
        context: &'static str,
    ) -> Result<Option<usize>, EncodeError> {
        match self.byte(context)? {
            0 => Ok(None),
            1 => Ok(Some(self.usize_uleb(context)?)),
            value => Err(EncodeError::InvalidEnum {
                field: context.to_owned(),
                value: value as u64,
            }),
        }
    }
}

/// 定宽模式下检查一个编号能否放进 `u16`。
///
/// `validate_references` 会把本次编码的真实宽度传进来，因此 `handler.binding`
/// 在写出前就能得到字段级错误；其余操作数仍由 [`Writer::index`] 在写出时兜底。
pub(super) fn check_index_width(
    value: u64,
    field: &str,
    width: OperandWidth,
) -> Result<(), EncodeError> {
    if matches!(width, OperandWidth::FixedU16) && value > u16::MAX as u64 {
        return Err(EncodeError::IntegerOverflow {
            field: field.to_owned(),
            value,
        });
    }
    Ok(())
}
