//! 静态浮点比较的共享回归语料；VM与原生使用完全相同的函数形参。

/// 一条带人工期望值的语言比较，不从被测LLVM推导黄金。
pub struct Case {
    /// 稳定的矩阵用例名称。
    pub name: String,
    /// 函数定义及其比较表达式。
    pub prefix: String,
    /// 调用比较函数的表达式。
    pub expression: String,
    /// 依据冻结语言语义确定的布尔黄金值。
    pub expected: bool,
    /// 应在LLVM中出现的按位比较宽度；排序用例没有此项。
    pub bits: Option<&'static str>,
}

impl Case {
    /// 打印入口强制动态降低；纯表达式入口保留局部静态比较。
    pub fn source(&self, printed: bool) -> String {
        let expression = if printed {
            format!("print({})", self.expression)
        } else {
            self.expression.clone()
        };
        format!("{}{expression}\n", self.prefix)
    }
}

/// 对一组输入同时构造相等与不等，期望值必须互补。
fn pair(
    cases: &mut Vec<Case>,
    label: &str,
    types: (&str, &str),
    values: (&str, &str),
    equal: bool,
    bits: Option<&'static str>,
) {
    for (operator, suffix, expected) in [("==", "eq", equal), ("!=", "ne", !equal)] {
        cases.push(Case {
            name: format!("{label}-{suffix}"),
            prefix: format!(
                "def compare({} left, {} right) -> bool\n    return left {operator} right\n",
                types.0, types.1
            ),
            expression: format!("compare({}, {})", values.0, values.1),
            expected,
            bits,
        });
    }
}

/// 正负零、普通有限值、提升舍入、整数/布尔和原始pow复现共用一张表。
pub fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for (ty, bits) in [("float", "i64"), ("sfloat", "i32")] {
        for (label, left, right, equal) in [
            ("opposite-zero", "0.0", "-0.0", false),
            ("reversed-zero", "-0.0", "0.0", false),
            ("positive-zero", "0.0", "0.0", true),
            ("negative-zero", "-0.0", "-0.0", true),
            ("same", "1.5", "1.5", true),
            ("different", "1.5", "2.0", false),
            ("negative-same", "-1.5", "-1.5", true),
        ] {
            let value = |text: &str| {
                if ty == "sfloat" {
                    format!("sfloat({text})")
                } else {
                    text.to_owned()
                }
            };
            pair(
                &mut cases,
                &format!("{ty}-{label}"),
                (ty, ty),
                (&value(left), &value(right)),
                equal,
                Some(bits),
            );
        }
    }
    for (label, types, values, equal, bits) in [
        ("int-float", ("int", "float"), ("1", "1.0"), true, "i64"),
        (
            "rounded-int",
            ("int", "float"),
            ("9007199254740993", "9007199254740992.0"),
            true,
            "i64",
        ),
        (
            "sint-sfloat",
            ("sint", "sfloat"),
            ("sint(1)", "sfloat(1.0)"),
            true,
            "i32",
        ),
        (
            "sfloat-float",
            ("sfloat", "float"),
            ("sfloat(1.5)", "1.5"),
            true,
            "i64",
        ),
        (
            "float-sfloat",
            ("float", "sfloat"),
            ("1.5", "sfloat(1.5)"),
            true,
            "i64",
        ),
        (
            "int-sfloat",
            ("int", "sfloat"),
            ("16777217", "sfloat(16777216.0)"),
            false,
            "i64",
        ),
        (
            "int-negative-zero",
            ("int", "float"),
            ("0", "-0.0"),
            false,
            "i64",
        ),
        (
            "sint-negative-zero",
            ("sint", "sfloat"),
            ("sint(0)", "sfloat(-0.0)"),
            false,
            "i32",
        ),
    ] {
        pair(&mut cases, label, types, values, equal, Some(bits));
    }
    for (label, ty, left, right, equal) in [
        ("integer-same", "int", "1", "1", true),
        ("integer-different", "int", "1", "2", false),
        ("boolean-same", "bool", "true", "true", true),
        ("boolean-different", "bool", "true", "false", false),
    ] {
        pair(&mut cases, label, (ty, ty), (left, right), equal, None);
    }
    for (operator, suffix, expected) in [("==", "eq", true), ("!=", "ne", false)] {
        cases.push(Case {
            name: format!("original-pow-{suffix}"),
            prefix: "def probe(float a, float b) -> float\n    return a ** b\n".to_owned(),
            expression: format!("probe(2.0, 3.0) {operator} 8.0"),
            expected,
            bits: Some("i64"),
        });
    }
    for ty in ["float", "sfloat"] {
        for (operator, suffix, expected) in [
            ("<", "lt", false),
            ("<=", "le", true),
            (">", "gt", false),
            (">=", "ge", true),
        ] {
            cases.push(Case {
                name: format!("{ty}-zero-order-{suffix}"),
                prefix: format!(
                    "def compare({ty} left, {ty} right) -> bool\n    return left {operator} right\n"
                ),
                expression: if ty == "float" {
                    "compare(0.0, -0.0)".to_owned()
                } else {
                    "compare(sfloat(0.0), sfloat(-0.0))".to_owned()
                },
                expected,
                bits: None,
            });
        }
    }
    cases
}
