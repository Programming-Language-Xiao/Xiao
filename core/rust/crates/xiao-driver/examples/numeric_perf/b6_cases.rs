//! B6 的完整调用语料与独立预期值。

use super::{b1_cases, cases};
use serde::Serialize;
use std::{fs, path::Path};
use xiao_artifacts::Digest256;

/// 固定的测量单元及其实际源程序。
pub struct Load {
    /// 源程序对应的冻结目录记录。
    pub spec: Spec,
    /// 每个后端实际编译的 Xiao 源程序。
    pub source: String,
}

/// 在采样前固定并逐项对账的负载。
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Spec {
    /// 数值、cmath 或兼容回归测试组。
    pub suite: &'static str,
    /// 同一输入族的稳定标识。
    pub case: String,
    /// 输入族内的规模或参考向量标识。
    pub profile: String,
    /// 以 dimension 为单位的输入规模。
    pub size: usize,
    /// 输入规模的计量单位。
    pub dimension: String,
    /// 在同输入族中与本项比较的半规模。
    pub double_of: Option<String>,
    /// 适用的翻倍时间比上限。
    pub ratio_limit: Option<u8>,
    /// 是否独立核对最大合法负载。
    pub maximum: bool,
    /// 已登记输入的 UTF-8 字节数。
    pub input_bytes: Option<usize>,
    /// 已登记黄金输出的 UTF-8 字节数。
    pub output_bytes: Option<usize>,
    /// 独立黄金的来源标识或实际字节摘要。
    pub reference: Option<String>,
    /// 实际生成程序的原始 UTF-8 摘要。
    pub source_sha256: String,
}

impl Load {
    /// 绑定即将交给编译器的原始源码。
    fn new(mut spec: Spec, source: String) -> Self {
        spec.source_sha256 = Digest256::of_bytes(source.as_bytes()).to_string();
        Self { spec, source }
    }
}

/// 按输入族定义规模，按具体操作追加比较与黄金约束。
fn spec(suite: &'static str, case: &str, profile: &str, size: usize, dimension: &str) -> Spec {
    Spec {
        suite,
        case: case.into(),
        profile: profile.into(),
        size,
        dimension: dimension.into(),
        double_of: None,
        ratio_limit: None,
        maximum: profile == "maximum",
        input_bytes: None,
        output_bytes: None,
        reference: None,
        source_sha256: String::new(),
    }
}

/// 转义示例输出里的换行和引号，使其可作 Xiao 字符串字面量。
fn literal(value: &str) -> String {
    serde_json::to_string(value).unwrap()
}

/// 将源语句移入函数或批次循环，同时保留原有缩进。
fn indent(body: &str) -> String {
    body.lines().map(|line| format!("    {line}\n")).collect()
}

/// 单次调用和计时区间外的输入准备、结果校验。
struct Call<'a> {
    prelude: &'a str,
    parameters: &'a str,
    args: &'a str,
    result: &'a str,
    initial: &'a str,
    body: &'a str,
    prepare: &'a str,
    check: &'a str,
}

/// 输入准备、一次预热及每轮黄金检查均在计时标记外执行。
fn program(call: Call<'_>) -> String {
    let Call {
        prelude,
        parameters,
        args,
        result,
        initial,
        body,
        prepare,
        check,
    } = call;
    let loop_parameters = if parameters.is_empty() {
        String::new()
    } else {
        format!("{parameters}, ")
    };
    let loop_args = if args.is_empty() {
        String::new()
    } else {
        format!("{args}, ")
    };
    let names = parameters
        .split(',')
        .filter_map(|declaration| declaration.split_whitespace().last())
        .collect::<Vec<_>>()
        .join(", ");
    let body = indent(body);
    let validation = indent(check);
    let declaration = if initial == "[]" {
        String::new()
    } else {
        format!("{result} ")
    };
    format!(
        "{prelude}\
def __b6_repeat() -> int
    value = 0
    for digit in input()
        point = int(digit)
        if point < 48 or point > 57
            raise Error(code = \"B6_CONFIGURATION\")
        value = value * 10 + point - 48
    if value < 0 or value > 16777216
        raise Error(code = \"B6_CONFIGURATION\")
    return value
def __b6_call({parameters}) -> {result}
{body}def __b6_batch({loop_parameters}int repeats) -> {result}
    {declaration}actual = {initial}
    iteration = 0
    while iteration < repeats
        actual = __b6_call({names})
        iteration = iteration + 1
    return actual
selected = input()
profile = input()
print(0)
{prepare}
actual = __b6_call({args})
{check}
print(1)
repeats = __b6_repeat()
while repeats != 0
    print(2)
    actual = __b6_batch({loop_args}repeats)
    print(3)
{validation}    repeats = __b6_repeat()
print(4)
"
    )
}

/// 复用 A6 运算负载，追加 B2 的显式精度除法。
fn numeric(root: &Path, loads: &mut Vec<Load>) {
    for case in cases::catalog(root) {
        let source = cases::source(std::slice::from_ref(&case));
        for profile in case.profiles {
            let mut cell = spec(
                "numeric",
                &case.id,
                &profile.label,
                profile.size,
                &case.dimension,
            );
            cell.maximum = profile.maximum;
            if profile.label == "2n" {
                cell.double_of = Some("n".to_owned());
                cell.ratio_limit = Some(if case.complexity == "linear" { 3 } else { 5 });
            }
            loads.push(Load::new(cell, source.clone()));
        }
    }
    for (profile, size) in [("n", 4096), ("2n", 8192), ("maximum", 32768)] {
        // B = 3A + 1，A/B 与 1/3 相差 1/(3B)，舍入至 p 位得到 p 个 3。
        let left = format!("{}2", "3".repeat(size - 1));
        let right = format!("{}7", "9".repeat(size - 1));
        let expected = format!("0.{}", "3".repeat(size));
        let prepare = format!("left = lfloat(\"{left}\")\nright = lfloat(\"{right}\")\n");
        let mut cell = spec(
            "numeric",
            "lfloat.divide-precision",
            profile,
            size,
            "系数位数及有效数字",
        );
        cell.reference = Some("(B-1)/(3B) half_even".into());
        if profile == "2n" {
            cell.double_of = Some("n".to_owned());
            cell.ratio_limit = Some(5);
        }
        loads.push(Load::new(
            cell,
            program(Call {
                prelude: "",
                parameters: "lfloat left, lfloat right",
                args: "left, right",
                result: "lfloat",
                initial: "lfloat(0)",
                body: &format!("return lfloat_divide(left, right, {size}, \"half_even\")"),
                prepare: &prepare,
                check: &format!("print(str(actual) == \"{expected}\")"),
            }),
        ));
    }
    for (name, precision, error) in [("minimum", 1, false), ("invalid", 32769, true)] {
        let body = if error {
            format!(
                "try\n    lfloat_divide(left, right, {precision}, \"half_even\")\ncatch failure as Error\n    return error_code(failure)\nreturn \"unexpected\""
            )
        } else {
            format!("return str(lfloat_divide(left, right, {precision}, \"half_even\"))")
        };
        loads.push(Load::new(
            spec(
                "numeric",
                "lfloat.divide-precision",
                name,
                precision,
                "有效数字",
            ),
            program(Call {
                prelude: "",
                parameters: "lfloat left, lfloat right",
                args: "left, right",
                result: "str",
                initial: "\"\"",
                body: &body,
                prepare: "left = lfloat(1)\nright = lfloat(3)\n",
                check: if error {
                    "print(actual == \"X06-RUNTIME-012\")"
                } else {
                    "print(actual == \"0.3\")"
                },
            }),
        ));
    }
}

/// 正式 F-9 向量中可由语言源码精确构造的输入。
const CMATH: &[(&str, &[&str])] = &[
    ("sqrt", &["v00290", "v00001", "v00602", "v00607"]),
    ("floor", &["v01509", "v01816"]),
    ("ceil", &["v03315", "v03030"]),
    ("round", &["v03937", "v04244"]),
    ("trunc", &["v05743", "v05458"]),
    ("abs", &["v06957", "v06672"]),
    ("min", &["v14568", "v14577"]),
    ("max", &["v15368", "v15377"]),
    ("pow", &["v16544", "v16177", "v16181"]),
    ("sin", &["v07551", "v07886"]),
    ("cos", &["v08765", "v09100"]),
    ("tan", &["v09979", "v10314"]),
    ("exp", &["v11193", "v12136", "v11268"]),
    ("ln", &["v12430", "v12742", "v12140"]),
    ("log10", &["v13644", "v13956", "v13354"]),
];

/// 读取参考文件的 binary64 十六进制位模式。
fn bits(raw: &str) -> u64 {
    u64::from_str_radix(raw, 16).unwrap()
}

/// 使用能往返到原位模式的十进制字面量构造有限 Float。
fn float_expression(value: u64) -> String {
    let number = f64::from_bits(value);
    assert!(number.is_finite());
    format!("float({number:e})")
}

/// 逐位核对精确项和零，其余非零结果核对 F-9 的有符号 4 ULP 区间。
fn cmath_check(name: &str, expected: u64) -> String {
    let value = f64::from_bits(expected);
    if value == 0.0
        || matches!(
            name,
            "sqrt" | "floor" | "ceil" | "round" | "trunc" | "abs" | "min" | "max"
        )
    {
        return format!("print(actual == {})", float_expression(expected));
    }
    let magnitude = expected & !(1_u64 << 63);
    let sign = expected ^ magnitude;
    let near = float_expression(sign | magnitude.saturating_sub(4).max(1));
    let far = float_expression(sign | (magnitude + 4).min(f64::MAX.to_bits()));
    let (low, high) = if sign == 0 { (near, far) } else { (far, near) };
    format!("print(actual >= {low} and actual <= {high})")
}

/// 为两个官方别名和共享浮点幂生成可执行的参考向量程序。
fn cmath(root: &Path, loads: &mut Vec<Load>) {
    let reference =
        fs::read_to_string(root.join("tests/fixtures/docs/cmath/reference-vectors.tsv")).unwrap();
    let vectors: std::collections::BTreeMap<_, _> = reference
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            (fields[0], fields)
        })
        .collect();
    for &(name, ids) in CMATH {
        let mut aliases = vec!["std", "stdlib"];
        if name == "pow" {
            aliases.push("operator");
        }
        for alias in aliases {
            let prelude = if alias == "operator" {
                String::new()
            } else {
                format!("from {alias}.cmath import {name}\n")
            };
            for &id in ids {
                let row = &vectors[id];
                assert_eq!(row[1], name);
                let (parameters, args, prepare) = if row[3] == "-" {
                    (
                        "float left",
                        "left",
                        format!("left = {}\n", float_expression(bits(row[2]))),
                    )
                } else {
                    (
                        "float left, float right",
                        "left, right",
                        format!(
                            "left = {}\nright = {}\n",
                            float_expression(bits(row[2])),
                            float_expression(bits(row[3]))
                        ),
                    )
                };
                let expression = if alias == "operator" {
                    "left ** right".into()
                } else {
                    format!("{name}({args})")
                };
                let (result, initial, body, check) = if row[4] == "X06-RUNTIME-009" {
                    (
                        "str",
                        "\"\"",
                        format!(
                            "try\n    {expression}\ncatch failure as Error\n    return error_code(failure)\nreturn \"unexpected\""
                        ),
                        "print(actual == \"X06-RUNTIME-009\")".to_owned(),
                    )
                } else {
                    (
                        "float",
                        "float(0)",
                        format!("return {expression}"),
                        cmath_check(name, bits(row[4])),
                    )
                };
                let mut cell = spec("cmath", &format!("{alias}.cmath.{name}"), id, 1, "binary64");
                cell.reference = Some(id.into());
                loads.push(Load::new(
                    cell,
                    program(Call {
                        prelude: &prelude,
                        parameters,
                        args,
                        result,
                        initial,
                        body: &body,
                        prepare: &prepare,
                        check: &check,
                    }),
                ));
            }
        }
    }
}

/// 在新快照上重测的 20 个 B1 产品入口。
const B1: &[&str] = &[
    "math.abs",
    "math.min",
    "math.max",
    "math.clamp",
    "math.gcd",
    "math.pow",
    "statistics.sum",
    "statistics.min",
    "statistics.max",
    "fmt.format_int",
    "fmt.pad_left",
    "fmt.pad_right",
    "hex.encode",
    "hex.decode",
    "base64.encode",
    "base64.decode",
    "url.encode_unicode",
    "url.decode_unicode",
    "csv.parse_ascii",
    "csv.format_ascii",
];

/// 复用原 B1 产品负载，并对全转义 URL 额外绑定输入和黄金摘要。
fn b1(loads: &mut Vec<Load>) {
    assert!(B1.iter().all(|id| b1_cases::IDS.contains(id)));
    for &id in B1 {
        let profiles = if id == "url.decode_unicode" {
            vec![
                ("4096", Some(4096)),
                ("8192", Some(8192)),
                ("16384", Some(16384)),
                ("maximum", None),
            ]
        } else {
            vec![("maximum", None)]
        };
        for (profile, size) in profiles {
            let work = b1_cases::workload(id, size);
            let mut cell = spec(
                "compat",
                &format!("b1.{id}"),
                profile,
                work.input_bytes_or_items,
                "B1 输入规模",
            );
            if id == "url.decode_unicode" {
                cell.dimension = "输入 UTF-8 字节".into();
                cell.input_bytes = Some(work.input_bytes_or_items);
                cell.output_bytes = Some(work.output_bytes_or_items);
                let n = work.input_bytes_or_items;
                let input = "%F0%9F%9A%80".repeat(n / 12) + &"x".repeat(n % 12);
                let golden = "🚀".repeat(n / 12) + &"x".repeat(n % 12);
                assert_eq!(golden.len(), work.output_bytes_or_items);
                cell.reference = Some(format!(
                    "input-sha256={} stdout-value-sha256={}",
                    Digest256::of_bytes(input.as_bytes()),
                    Digest256::of_bytes(golden.as_bytes())
                ));
                cell.double_of = match profile {
                    "8192" => Some("4096".into()),
                    "16384" => Some("8192".into()),
                    _ => None,
                };
                cell.ratio_limit = cell.double_of.as_ref().map(|_| 3);
            }
            loads.push(Load::new(
                cell,
                program(Call {
                    prelude: &format!("from std.{} import {}\n", work.module, work.function),
                    parameters: work.signature,
                    args: &work.arguments,
                    result: work.result,
                    initial: work.initial,
                    body: &format!("return {}", work.call),
                    prepare: &work.prepare,
                    check: &work.check,
                }),
            ));
        }
    }
}

/// 校验三条完整示例仍来自正式教程，并保留已提交的预期输出。
fn a1(root: &Path, loads: &mut Vec<Load>) {
    let directory = root.join("tests/fixtures/docs/numeric");
    let tutorial =
        fs::read_to_string(root.join("docs/UseDocs/language/basics/types/conversions.md"))
            .unwrap()
            .replace("\r\n", "\n");
    for name in ["a1_identity", "a1_admission_error", "a1_pending_finally"] {
        let file = directory.join(format!("{name}.xiao"));
        let source = fs::read_to_string(&file).unwrap().replace("\r\n", "\n");
        let golden = fs::read_to_string(file.with_extension("stdout"))
            .unwrap()
            .replace("\r\n", "\n");
        let fence = char::from(96).to_string().repeat(3);
        assert!(tutorial.contains(&format!("{fence}xiao\n{source}{fence}")));
        // 保留整段示例和输出格式，只把输出写入内存以便在计时后核对。
        let mut body = String::from("str observed = \"\"\n");
        for line in source.lines() {
            let statement = line.trim_start();
            if let Some(value) = statement
                .strip_prefix("print(")
                .and_then(|part| part.strip_suffix(')'))
            {
                let prefix = &line[..line.len() - statement.len()];
                body.push_str(&format!(
                    "{prefix}observed = observed + str({value}) + \"\\n\"\n"
                ));
            } else {
                body.push_str(line);
                body.push('\n');
            }
        }
        body.push_str("return observed");
        let mut cell = spec(
            "compat",
            &format!("a1.{name}"),
            "complete",
            1,
            "完整教程示例",
        );
        cell.reference = Some(format!(
            "{}:{}",
            file.file_name().unwrap().to_string_lossy(),
            Digest256::of_bytes(golden.as_bytes())
        ));
        loads.push(Load::new(
            cell,
            program(Call {
                prelude: "",
                parameters: "",
                args: "",
                result: "str",
                initial: "\"\"",
                body: &body,
                prepare: "",
                check: &format!("print(actual == {})", literal(&golden)),
            }),
        ));
    }
}

/// 由固定的 A6、F-9 和 B1/A1 夹具生成 B6 清单，采样前验证源程序摘要。
pub fn catalog(root: &Path) -> Vec<Load> {
    let mut loads = Vec::new();
    numeric(root, &mut loads);
    cmath(root, &mut loads);
    b1(&mut loads);
    a1(root, &mut loads);
    loads
}
