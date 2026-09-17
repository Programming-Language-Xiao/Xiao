//! Runtime 值的运算表：算术、比较、相等和显式转换。
//!
//! 本模块是 Runtime 唯一实现算子的地方；字节码 VM 和原生后端都经这里取值
//! 语义，不得各自维护一份矩阵。规则来源是 02 阶段的数值与转换契约：
//!
//! - **不做隐式宽度提升**。`int + sint` 这类跨宽度运算必须先由后端插入
//!   显式转换，Runtime 收到不匹配的宽度一律报错，避免不同执行路径给出
//!   不同结果。
//! - `/` 的静态结果类型是浮点，因此 Runtime 要求两侧都是同宽度浮点。
//! - `//` 和 `%` 只接受整数，除数为零使用稳定的除零错误身份。
//! - `lint` / `lfloat` 是尚无算术实现的高精度数值，参与数值运算时明确报错，
//!   不静默退化为字符串比较。

use xiao_syntax::ScalarType;

use super::RuntimeValue;
use crate::errors::{RuntimeError, RuntimeResult};

/// `str -> bool` 冻结接受的四个拼写。
const BOOLEAN_STRINGS: [&str; 4] = ["true", "True", "false", "False"];

/// 一组宽度一致、可以直接参与数值运算的操作数。
enum NumericPair {
    /// 两个 64 位整数。
    Int(i64, i64),
    /// 两个 32 位整数。
    Sint(i32, i32),
    /// 两个 64 位浮点。
    Float(f64, f64),
    /// 两个 32 位浮点。
    Sfloat(f32, f32),
}

impl NumericPair {
    /// 从两个值提取同宽度数值操作数；宽度不一致时给出可定位的错误。
    fn new(left: &RuntimeValue, right: &RuntimeValue) -> RuntimeResult<Self> {
        Ok(match (left, right) {
            (RuntimeValue::Int(left), RuntimeValue::Int(right)) => Self::Int(*left, *right),
            (RuntimeValue::Sint(left), RuntimeValue::Sint(right)) => Self::Sint(*left, *right),
            (RuntimeValue::Float(left), RuntimeValue::Float(right)) => Self::Float(*left, *right),
            (RuntimeValue::Sfloat(left), RuntimeValue::Sfloat(right)) => {
                Self::Sfloat(*left, *right)
            }
            _ => {
                return Err(RuntimeError::invalid_value(format!(
                    "{} 与 {} 的宽度不一致；后端必须先插入显式转换",
                    left.type_name(),
                    right.type_name()
                )));
            }
        })
    }
}

impl RuntimeValue {
    /// 对布尔值执行严格的 `bool ± 整数` 运算。
    pub fn bool_adjust(&self, amount: i128) -> RuntimeResult<Self> {
        let Self::Bool(value) = self else {
            return Err(RuntimeError::type_mismatch("bool", self.type_name()));
        };
        Ok(Self::Bool(boolean_parity(*value, amount)))
    }

    /// 执行加法，包括字符串拼接和布尔整数调整。
    pub fn add(&self, other: &Self) -> RuntimeResult<Self> {
        if let Self::Bool(value) = self {
            return Ok(Self::Bool(boolean_parity(
                *value,
                integer_value(other)
                    .ok_or_else(|| RuntimeError::type_mismatch("int", other.type_name()))?,
            )));
        }
        if let (Self::Str(left), Self::Str(right)) = (self, other) {
            let mut value = left.to_string()?;
            value.push_str(&right.to_string()?);
            return Self::new_string(value);
        }
        match NumericPair::new(self, other)? {
            NumericPair::Int(left, right) => checked_int64(left, right, i64::checked_add),
            NumericPair::Sint(left, right) => checked_int32(left, right, i32::checked_add),
            NumericPair::Float(left, right) => finite_float64(left + right),
            NumericPair::Sfloat(left, right) => finite_float32(left + right),
        }
    }

    /// 执行减法，包括布尔整数调整。
    pub fn subtract(&self, other: &Self) -> RuntimeResult<Self> {
        if let Self::Bool(value) = self {
            return Ok(Self::Bool(boolean_parity(
                *value,
                integer_value(other)
                    .ok_or_else(|| RuntimeError::type_mismatch("int", other.type_name()))?,
            )));
        }
        match NumericPair::new(self, other)? {
            NumericPair::Int(left, right) => checked_int64(left, right, i64::checked_sub),
            NumericPair::Sint(left, right) => checked_int32(left, right, i32::checked_sub),
            NumericPair::Float(left, right) => finite_float64(left - right),
            NumericPair::Sfloat(left, right) => finite_float32(left - right),
        }
    }

    /// 执行乘法。
    pub fn multiply(&self, other: &Self) -> RuntimeResult<Self> {
        match NumericPair::new(self, other)? {
            NumericPair::Int(left, right) => checked_int64(left, right, i64::checked_mul),
            NumericPair::Sint(left, right) => checked_int32(left, right, i32::checked_mul),
            NumericPair::Float(left, right) => finite_float64(left * right),
            NumericPair::Sfloat(left, right) => finite_float32(left * right),
        }
    }

    /// 执行 `/`；静态结果类型是浮点，因此只接受同宽度浮点操作数。
    pub fn divide(&self, other: &Self) -> RuntimeResult<Self> {
        match NumericPair::new(self, other)? {
            NumericPair::Float(left, right) => finite_float64(left / right),
            NumericPair::Sfloat(left, right) => finite_float32(left / right),
            NumericPair::Int(..) | NumericPair::Sint(..) => Err(RuntimeError::invalid_value(
                "/ 的结果类型是浮点；后端必须先把整数操作数显式转换为浮点",
            )),
        }
    }

    /// 执行整除 `//`，只接受同宽度整数；除数为零报告除零错误。
    pub fn floor_divide(&self, other: &Self) -> RuntimeResult<Self> {
        match NumericPair::new(self, other)? {
            NumericPair::Int(left, right) => Ok(Self::Int(floor_div_int64(left, right)?)),
            NumericPair::Sint(left, right) => Ok(Self::Sint(floor_div_int32(left, right)?)),
            NumericPair::Float(..) | NumericPair::Sfloat(..) => {
                Err(RuntimeError::invalid_value("// 只接受整数操作数"))
            }
        }
    }

    /// 执行取模 `%`，只接受同宽度整数；除数为零报告除零错误。
    ///
    /// 结果符号跟随除数，与 `//` 保持 `左 = 商 * 右 + 余` 的一致性。
    pub fn remainder(&self, other: &Self) -> RuntimeResult<Self> {
        match NumericPair::new(self, other)? {
            NumericPair::Int(left, right) => Ok(Self::Int(floor_rem_int64(left, right)?)),
            NumericPair::Sint(left, right) => Ok(Self::Sint(floor_rem_int32(left, right)?)),
            NumericPair::Float(..) | NumericPair::Sfloat(..) => {
                Err(RuntimeError::invalid_value("% 只接受整数操作数"))
            }
        }
    }

    /// 执行幂运算 `**`。
    ///
    /// 整数底数的指数必须非负：负指数会得到整数无法表示的分数结果，而静态层
    /// 已经把它标成整数，因此这里明确拒绝而不是静默返回浮点。
    pub fn power(&self, other: &Self) -> RuntimeResult<Self> {
        match NumericPair::new(self, other)? {
            NumericPair::Int(left, right) => {
                let exponent = non_negative_exponent(right)?;
                left.checked_pow(exponent)
                    .map(Self::Int)
                    .ok_or_else(|| RuntimeError::numeric_overflow("int 幂运算溢出"))
            }
            NumericPair::Sint(left, right) => {
                let exponent = non_negative_exponent(i64::from(right))?;
                left.checked_pow(exponent)
                    .map(Self::Sint)
                    .ok_or_else(|| RuntimeError::numeric_overflow("sint 幂运算溢出"))
            }
            NumericPair::Float(left, right) => finite_float64(left.powf(right)),
            NumericPair::Sfloat(left, right) => finite_float32(left.powf(right)),
        }
    }

    /// 执行 `<`。
    pub fn less(&self, other: &Self) -> RuntimeResult<bool> {
        self.compare(other, |value| value < 0.0, |left, right| left < right)
    }

    /// 执行 `<=`。
    pub fn less_equal(&self, other: &Self) -> RuntimeResult<bool> {
        self.compare(other, |value| value <= 0.0, |left, right| left <= right)
    }

    /// 执行 `>`。
    pub fn greater(&self, other: &Self) -> RuntimeResult<bool> {
        self.compare(other, |value| value > 0.0, |left, right| left > right)
    }

    /// 执行 `>=`。
    pub fn greater_equal(&self, other: &Self) -> RuntimeResult<bool> {
        self.compare(other, |value| value >= 0.0, |left, right| left >= right)
    }

    /// 执行语言级相等比较。
    ///
    /// 语义与 [`PartialEq`] 实现完全一致；提供具名入口是为了让后端不会误用
    /// Rust 侧的其他相等语义，并保证 `str` 内容比较零拷贝。
    #[must_use]
    pub fn equals(&self, other: &Self) -> bool {
        self == other
    }

    /// 执行语言级不等比较。
    #[must_use]
    pub fn not_equals(&self, other: &Self) -> bool {
        self != other
    }

    /// 执行显式标量转换。
    ///
    /// 只实现 02 阶段转换矩阵接受的组合；`lint` / `lfloat` 的算术与转换仍是
    /// 登记在案的债项，这里明确报错而不是给出近似结果。
    pub fn convert_to(&self, target: ScalarType) -> RuntimeResult<Self> {
        let source = self.scalar_type().ok_or_else(|| {
            RuntimeError::invalid_value(format!("{} 不是标量，不能转换", self.type_name()))
        })?;
        if source == target {
            return Ok(self.clone());
        }
        match (source, target) {
            (ScalarType::Str, ScalarType::Bool) => self.convert_str_to_bool(),
            (ScalarType::Bool, ScalarType::Str) => {
                Self::new_string(if self.as_bool() == Some(true) {
                    "true"
                } else {
                    "false"
                })
            }
            (ScalarType::Sint, ScalarType::Int) => Ok(Self::Int(i64::from(self.sint_value()?))),
            (ScalarType::Int, ScalarType::Sint) => i32::try_from(self.int_value()?)
                .map(Self::Sint)
                .map_err(|_| range_error("sint")),
            (ScalarType::Sfloat, ScalarType::Float) => {
                Ok(Self::Float(f64::from(self.sfloat_value()?)))
            }
            (ScalarType::Sint, ScalarType::Float) => Ok(Self::Float(f64::from(self.sint_value()?))),
            (ScalarType::Sint, ScalarType::Sfloat) => Ok(Self::Sfloat(self.sint_value()? as f32)),
            (ScalarType::Int, ScalarType::Float) => Ok(Self::Float(self.int_value()? as f64)),
            (ScalarType::Int, ScalarType::Sfloat) => Ok(Self::Sfloat(self.int_value()? as f32)),
            (ScalarType::Float, ScalarType::Sfloat) => {
                let narrowed = self.float_value()? as f32;
                if narrowed.is_finite() {
                    Ok(Self::Sfloat(narrowed))
                } else {
                    Err(range_error("sfloat"))
                }
            }
            (ScalarType::Float, ScalarType::Int) => {
                narrow_to_int64(self.float_value()?).map(Self::Int)
            }
            (ScalarType::Sfloat, ScalarType::Int) => {
                narrow_to_int64(f64::from(self.sfloat_value()?)).map(Self::Int)
            }
            (ScalarType::Float, ScalarType::Sint) => {
                narrow_to_int32(self.float_value()?).map(Self::Sint)
            }
            (ScalarType::Sfloat, ScalarType::Sint) => {
                narrow_to_int32(f64::from(self.sfloat_value()?)).map(Self::Sint)
            }
            (ScalarType::Lint | ScalarType::Lfloat, _)
            | (_, ScalarType::Lint | ScalarType::Lfloat) => {
                Err(RuntimeError::invalid_value(format!(
                    "{} 与 {} 之间的转换尚未实现",
                    source.as_str(),
                    target.as_str()
                )))
            }
            _ => Err(RuntimeError::invalid_value(format!(
                "不支持把 {} 转换为 {}",
                source.as_str(),
                target.as_str()
            ))),
        }
    }

    /// 比较两个同宽度数值或两个字符串。
    ///
    /// `from_sign` 接收 `self` 相对 `other` 的序符号（负、零、正），因此整数
    /// 和浮点可以共用同一套关系判断；整数不使用相减，避免极值相减溢出。
    fn compare(
        &self,
        other: &Self,
        from_sign: impl Fn(f64) -> bool,
        from_floats: impl Fn(f64, f64) -> bool,
    ) -> RuntimeResult<bool> {
        if let (Self::Str(left), Self::Str(right)) = (self, other) {
            return left
                .with_str(|left| right.with_str(|right| left.cmp(right)))
                .and_then(|ordering| ordering)
                .map(|ordering| from_sign(f64::from(ordering as i8)))
                .map_err(|_| RuntimeError::invalid_value("已释放的字符串句柄不能比较"));
        }
        Ok(match NumericPair::new(self, other)? {
            NumericPair::Int(left, right) => from_sign(f64::from(left.cmp(&right) as i8)),
            NumericPair::Sint(left, right) => from_sign(f64::from(left.cmp(&right) as i8)),
            NumericPair::Float(left, right) => from_floats(left, right),
            NumericPair::Sfloat(left, right) => from_floats(f64::from(left), f64::from(right)),
        })
    }

    /// 按冻结的四个拼写把 `str` 转换为 `bool`。
    fn convert_str_to_bool(&self) -> RuntimeResult<Self> {
        let Self::Str(handle) = self else {
            return Err(RuntimeError::type_mismatch("str", self.type_name()));
        };
        let text = handle.with_str(|value| value.to_owned())?;
        if !BOOLEAN_STRINGS.contains(&text.as_str()) {
            return Err(RuntimeError::invalid_value(format!(
                "只有 true/True/false/False 可以转换为 bool，实际是 {text:?}"
            )));
        }
        Ok(Self::Bool(text.eq_ignore_ascii_case("true")))
    }

    /// 读取 `sint` 载荷。
    fn sint_value(&self) -> RuntimeResult<i32> {
        match self {
            Self::Sint(value) => Ok(*value),
            _ => Err(RuntimeError::type_mismatch("sint", self.type_name())),
        }
    }

    /// 读取 `int` 载荷。
    fn int_value(&self) -> RuntimeResult<i64> {
        match self {
            Self::Int(value) => Ok(*value),
            _ => Err(RuntimeError::type_mismatch("int", self.type_name())),
        }
    }

    /// 读取 `sfloat` 载荷。
    fn sfloat_value(&self) -> RuntimeResult<f32> {
        match self {
            Self::Sfloat(value) => Ok(*value),
            _ => Err(RuntimeError::type_mismatch("sfloat", self.type_name())),
        }
    }

    /// 读取 `float` 载荷。
    fn float_value(&self) -> RuntimeResult<f64> {
        match self {
            Self::Float(value) => Ok(*value),
            _ => Err(RuntimeError::type_mismatch("float", self.type_name())),
        }
    }
}

/// 按冻结的奇偶规则调整布尔值。
fn boolean_parity(value: bool, amount: i128) -> bool {
    if amount.unsigned_abs() % 2 == 0 {
        value
    } else {
        !value
    }
}

/// 提取可参与布尔调整的整数值。
fn integer_value(value: &RuntimeValue) -> Option<i128> {
    match value {
        RuntimeValue::Int(value) => Some(i128::from(*value)),
        RuntimeValue::Sint(value) => Some(i128::from(*value)),
        RuntimeValue::Lint(value) => value.parse().ok(),
        _ => None,
    }
}

/// 应用一个可能溢出的 64 位整数运算。
fn checked_int64(
    left: i64,
    right: i64,
    op: fn(i64, i64) -> Option<i64>,
) -> RuntimeResult<RuntimeValue> {
    op(left, right)
        .map(RuntimeValue::Int)
        .ok_or_else(|| RuntimeError::numeric_overflow("int 运算溢出"))
}

/// 应用一个可能溢出的 32 位整数运算。
fn checked_int32(
    left: i32,
    right: i32,
    op: fn(i32, i32) -> Option<i32>,
) -> RuntimeResult<RuntimeValue> {
    op(left, right)
        .map(RuntimeValue::Sint)
        .ok_or_else(|| RuntimeError::numeric_overflow("sint 运算溢出"))
}

/// 拒绝非有限的 64 位浮点结果。
fn finite_float64(value: f64) -> RuntimeResult<RuntimeValue> {
    value
        .is_finite()
        .then_some(RuntimeValue::Float(value))
        .ok_or_else(|| RuntimeError::numeric_overflow("float 运算产生非有限值"))
}

/// 拒绝非有限的 32 位浮点结果。
fn finite_float32(value: f32) -> RuntimeResult<RuntimeValue> {
    value
        .is_finite()
        .then_some(RuntimeValue::Sfloat(value))
        .ok_or_else(|| RuntimeError::numeric_overflow("sfloat 运算产生非有限值"))
}

/// 计算向下取整的 64 位整数除法；除数为零使用稳定除零错误。
fn floor_div_int64(left: i64, right: i64) -> RuntimeResult<i64> {
    if right == 0 {
        return Err(RuntimeError::division_by_zero("//"));
    }
    let truncated = left
        .checked_div(right)
        .ok_or_else(|| RuntimeError::numeric_overflow("int 整除溢出"))?;
    let remainder = left % right;
    if remainder != 0 && (remainder < 0) != (right < 0) {
        truncated
            .checked_sub(1)
            .ok_or_else(|| RuntimeError::numeric_overflow("int 整除溢出"))
    } else {
        Ok(truncated)
    }
}

/// 计算向下取整的 32 位整数除法；除数为零使用稳定除零错误。
fn floor_div_int32(left: i32, right: i32) -> RuntimeResult<i32> {
    if right == 0 {
        return Err(RuntimeError::division_by_zero("//"));
    }
    let truncated = left
        .checked_div(right)
        .ok_or_else(|| RuntimeError::numeric_overflow("sint 整除溢出"))?;
    let remainder = left % right;
    if remainder != 0 && (remainder < 0) != (right < 0) {
        truncated
            .checked_sub(1)
            .ok_or_else(|| RuntimeError::numeric_overflow("sint 整除溢出"))
    } else {
        Ok(truncated)
    }
}

/// 计算与向下取整除法配套的 64 位余数，结果符号跟随除数。
fn floor_rem_int64(left: i64, right: i64) -> RuntimeResult<i64> {
    let quotient = floor_div_int64(left, right)?;
    quotient
        .checked_mul(right)
        .and_then(|product| left.checked_sub(product))
        .ok_or_else(|| RuntimeError::numeric_overflow("int 取模溢出"))
}

/// 计算与向下取整除法配套的 32 位余数，结果符号跟随除数。
fn floor_rem_int32(left: i32, right: i32) -> RuntimeResult<i32> {
    let quotient = floor_div_int32(left, right)?;
    quotient
        .checked_mul(right)
        .and_then(|product| left.checked_sub(product))
        .ok_or_else(|| RuntimeError::numeric_overflow("sint 取模溢出"))
}

/// 把整数指数转换为 `checked_pow` 接受的宽度。
fn non_negative_exponent(exponent: i64) -> RuntimeResult<u32> {
    if exponent < 0 {
        return Err(RuntimeError::invalid_value(
            "整数幂的指数不能为负；负指数会得到整数无法表示的结果",
        ));
    }
    u32::try_from(exponent).map_err(|_| RuntimeError::numeric_overflow("整数幂的指数过大"))
}

/// 按向零截断的规则把浮点转换为 `int`；非有限或超出范围时报错。
fn narrow_to_int64(value: f64) -> RuntimeResult<i64> {
    let truncated = value.trunc();
    let upper = 2_f64.powi(63);
    if !truncated.is_finite() || truncated < -upper || truncated >= upper {
        return Err(range_error("int"));
    }
    Ok(truncated as i64)
}

/// 按向零截断的规则把浮点转换为 `sint`；非有限或超出范围时报错。
fn narrow_to_int32(value: f64) -> RuntimeResult<i32> {
    i32::try_from(narrow_to_int64(value)?).map_err(|_| range_error("sint"))
}

/// 创建数值超出目标宽度范围的错误。
fn range_error(name: &str) -> RuntimeError {
    RuntimeError::numeric_overflow(format!("转换结果超出 {name} 的表示范围"))
}

#[cfg(test)]
/// 算术、比较、相等和显式转换的回归测试。
mod tests {
    use super::RuntimeValue;
    use crate::errors::{DIVISION_BY_ZERO_CODE, NUMERIC_OVERFLOW_CODE, RuntimeError};
    use xiao_syntax::ScalarType;

    /// 构造 `int` 值。
    fn int(value: i64) -> RuntimeValue {
        RuntimeValue::Int(value)
    }

    /// 构造 `str` 值。
    fn text(value: &str) -> RuntimeValue {
        RuntimeValue::new_string(value).expect("字符串应分配")
    }

    /// 提取错误码，便于断言稳定身份而不是文案。
    fn code(error: RuntimeError) -> String {
        error.code().to_owned()
    }

    #[test]
    /// 验证乘除整除取模幂的基本结果。
    fn supports_arithmetic_operators() {
        assert_eq!(int(6).multiply(&int(7)), Ok(int(42)));
        assert_eq!(int(7).floor_divide(&int(2)), Ok(int(3)));
        assert_eq!(int(7).remainder(&int(2)), Ok(int(1)));
        assert_eq!(int(2).power(&int(10)), Ok(int(1024)));
        assert_eq!(
            RuntimeValue::Float(7.0).divide(&RuntimeValue::Float(2.0)),
            Ok(RuntimeValue::Float(3.5))
        );
    }

    #[test]
    /// 验证整除与取模按向下取整配对，负数结果符号跟随除数。
    fn floor_division_pairs_with_remainder() {
        assert_eq!(int(-7).floor_divide(&int(2)), Ok(int(-4)));
        assert_eq!(int(-7).remainder(&int(2)), Ok(int(1)));
        assert_eq!(int(7).floor_divide(&int(-2)), Ok(int(-4)));
        assert_eq!(int(7).remainder(&int(-2)), Ok(int(-1)));
    }

    #[test]
    /// 验证除零使用稳定的除零错误身份，而不是泛化的值错误。
    fn division_by_zero_has_stable_identity() {
        assert_eq!(
            code(int(1).floor_divide(&int(0)).expect_err("除零应报错")),
            DIVISION_BY_ZERO_CODE
        );
        assert_eq!(
            code(int(1).remainder(&int(0)).expect_err("除零应报错")),
            DIVISION_BY_ZERO_CODE
        );
    }

    #[test]
    /// 验证整数溢出和非有限浮点结果都归入数值溢出。
    fn overflow_uses_stable_identity() {
        assert_eq!(
            code(int(i64::MAX).multiply(&int(2)).expect_err("应溢出")),
            NUMERIC_OVERFLOW_CODE
        );
        assert_eq!(
            code(int(i64::MIN).floor_divide(&int(-1)).expect_err("应溢出")),
            NUMERIC_OVERFLOW_CODE
        );
    }

    #[test]
    /// 验证跨宽度运算被拒绝，后端必须先插入显式转换。
    fn rejects_cross_width_operands() {
        let error = int(1).add(&RuntimeValue::Sint(1)).expect_err("应拒绝");
        assert!(error.message().contains("宽度不一致"));
    }

    #[test]
    /// 验证 `/` 的结果类型是浮点，整数操作数必须先显式转换。
    fn division_requires_float_operands() {
        let error = int(1).divide(&int(2)).expect_err("整数除法应被拒绝");
        assert!(error.message().contains("显式转换"));
    }

    #[test]
    /// 验证整数幂拒绝负指数而不是静默返回浮点。
    fn integer_power_rejects_negative_exponent() {
        assert!(int(2).power(&int(-1)).is_err());
    }

    #[test]
    /// 验证数值与字符串的比较关系。
    fn compares_numbers_and_strings() {
        assert_eq!(int(1).less(&int(2)), Ok(true));
        assert_eq!(int(2).less_equal(&int(2)), Ok(true));
        assert_eq!(int(3).greater(&int(2)), Ok(true));
        assert_eq!(int(3).greater_equal(&int(4)), Ok(false));
        assert_eq!(int(i64::MIN).less(&int(i64::MAX)), Ok(true));
        assert_eq!(text("apple").less(&text("banana")), Ok(true));
    }

    #[test]
    /// 验证字符串相等按内容比较，且不依赖句柄身份。
    fn string_equality_compares_content() {
        assert!(text("same").equals(&text("same")));
        assert!(!text("same").equals(&text("other")));
        assert!(text("same").not_equals(&text("other")));
        assert!(!text("same").equals(&int(1)));
    }

    #[test]
    /// 验证 `bool -> str` 与四个冻结拼写的 `str -> bool`。
    fn converts_between_bool_and_str() {
        assert_eq!(
            RuntimeValue::Bool(true).convert_to(ScalarType::Str),
            Ok(text("true"))
        );
        assert_eq!(
            text("False").convert_to(ScalarType::Bool),
            Ok(RuntimeValue::Bool(false))
        );
        assert!(text("yes").convert_to(ScalarType::Bool).is_err());
    }

    #[test]
    /// 验证数值加宽与窄化，窄化越界时报数值溢出。
    fn converts_numeric_widths_with_range_check() {
        assert_eq!(
            RuntimeValue::Sint(7).convert_to(ScalarType::Int),
            Ok(int(7))
        );
        assert_eq!(
            RuntimeValue::Float(3.9).convert_to(ScalarType::Int),
            Ok(int(3))
        );
        assert_eq!(
            RuntimeValue::Float(-3.9).convert_to(ScalarType::Int),
            Ok(int(-3))
        );
        assert_eq!(
            code(
                RuntimeValue::Int(i64::MAX)
                    .convert_to(ScalarType::Sint)
                    .expect_err("应越界")
            ),
            NUMERIC_OVERFLOW_CODE
        );
    }

    #[test]
    /// 验证 `lint` / `lfloat` 参与的转换明确报错，不给出近似结果。
    fn high_precision_numerics_are_explicitly_unsupported() {
        let error = RuntimeValue::Lint("1".to_owned())
            .convert_to(ScalarType::Int)
            .expect_err("应报错");
        assert!(error.message().contains("尚未实现"));
    }
}
