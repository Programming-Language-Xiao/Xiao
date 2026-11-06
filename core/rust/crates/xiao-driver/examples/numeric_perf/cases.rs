//! 独立黄金目录与完整语言测量程序；不内嵌另一套数值实现。

use serde::{Deserialize, Serialize};

/// 一个固定输入族的规模。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Profile {
    /// 同输入族的n、2n或maximum规模标签。
    pub label: String,
    /// 翻倍比较使用的实际输入维度。
    pub size: usize,
    /// 是否为该路径的最大合法输入。
    pub maximum: bool,
    /// 左实参的完整Xiao初始化表达式。
    pub left: String,
    /// 右实参的完整Xiao初始化表达式。
    pub right: String,
    /// 独立参考生成的结果初始化表达式。
    pub expected: String,
}

/// 同一路径的小规模、翻倍规模及最大合法输入。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Case {
    /// 固定负载标识，进入报告和进程配置。
    pub id: String,
    /// 两个输入实参的Xiao数值类型。
    pub family: String,
    /// 完整语言调用的结果类型。
    pub result: String,
    /// 默认调用体的实际被测表达式。
    pub expression: String,
    /// 需要控制流或承载时使用的完整函数体。
    pub body: Option<String>,
    /// 冻结的linear或superlinear翻倍预算类别。
    pub complexity: String,
    /// 规模数值的含义，防止跨输入族比较。
    pub dimension: String,
    /// 同一路径的固定规模与独立黄金。
    pub profiles: Vec<Profile>,
}

#[derive(Deserialize)]
/// 固定JSON目录的顶层容器，只接受其实际负载集合。
struct Catalog {
    /// 提交在仓库中的负载和规模。
    cases: Vec<Case>,
}

/// 读取提交在仓库中的独立输入和黄金，不从被测运行结果生成期望值。
pub fn catalog(repository: &std::path::Path) -> Vec<Case> {
    let raw = std::fs::read(repository.join("tests/fixtures/20o-b2-a6/catalog.json")).unwrap();
    serde_json::from_slice::<Catalog>(&raw).unwrap().cases
}

/// 数值初值只用于循环结果槽，不能改变测量表达式。
fn initial(result: &str) -> &'static str {
    match result {
        "lint" => "lint(0)",
        "lfloat" => "lfloat(0)",
        "float" => "float(0)",
        "sfloat" => "sfloat(0)",
        "str" => "\"\"",
        "bool" => "false",
        "int" => "0",
        _ => panic!("未登记的A6结果类型"),
    }
}

/// 一个可复用的完整程序；每个测量子进程只选择一项/一规模，输入和预热在时钟外。
pub fn source(cases: &[Case]) -> String {
    let mut result = String::from(
        "def __a6_repeat() -> int\n    value = 0\n    for digit in input()\n        point = int(digit)\n        if point < 48 or point > 57\n            raise Error(code = \"A6_CONFIGURATION\")\n        value = value * 10 + point - 48\n    if value < 0 or value > 16777216\n        raise Error(code = \"A6_CONFIGURATION\")\n    return value\n",
    );
    if cases.iter().any(|case| case.id.ends_with("xcon")) {
        result.insert_str(0,"from std.concurrent import current_runtime, spawn, join, pool_create, submit, wait, shutdown\ndef __a6_echo(lfloat value) -> lfloat\n    return value\n");
    }
    for (index, case) in cases.iter().enumerate() {
        let family = &case.family;
        let returned = &case.result;
        result.push_str(&format!(
            "def __a6_call_{index}({family} left, {family} right) -> {returned}\n"
        ));
        let body = case
            .body
            .clone()
            .unwrap_or_else(|| format!("return {}", case.expression));
        for line in body.lines() {
            result.push_str(&format!("    {line}\n"));
        }
        let initialize = if case.id.ends_with("xcon") {
            "    runtime_initialized = current_runtime()\n"
        } else {
            ""
        };
        result.push_str(&format!(
            "def __a6_batch_{index}({family} left, {family} right, int repeats) -> {returned}\n    actual = {}\n    iteration = 0\n    while iteration < repeats\n        actual = __a6_call_{index}(left, right)\n        iteration = iteration + 1\n    return actual\ndef __a6_run_{index}(str profile)\n{initialize}    print(0)\n    {family} left\n    {family} right\n    {returned} expected\n",
            initial(returned)
        ));
        for (profile_index, profile) in case.profiles.iter().enumerate() {
            let keyword = if profile_index == 0 { "if" } else { "elif" };
            result.push_str(&format!(
                "    {keyword} profile == {}\n        left = {}\n        right = {}\n        expected = {}\n",
                serde_json::to_string(&profile.label).unwrap(), profile.left, profile.right, profile.expected
            ));
        }
        result.push_str(&format!(
            "    else\n        raise Error(code = \"A6_PROFILE\")\n    warmed = __a6_call_{index}(left, right)\n    print(str(warmed) == str(expected))\n    print(1)\n    repeats = __a6_repeat()\n    while repeats != 0\n        print(2)\n        actual = __a6_batch_{index}(left, right, repeats)\n        print(3)\n        print(str(actual) == str(expected))\n        repeats = __a6_repeat()\n    print(4)\n"
        ));
    }
    result.push_str("selected = input()\nprofile = input()\n");
    for (index, case) in cases.iter().enumerate() {
        let keyword = if index == 0 { "if" } else { "elif" };
        result.push_str(&format!(
            "{keyword} selected == {}\n    __a6_run_{index}(profile)\n",
            serde_json::to_string(&case.id).unwrap()
        ));
    }
    result.push_str("else\n    raise Error(code = \"A6_CASE\")\n");
    result
}
