//! 05-D `config.xiao` 声明式子集的公开规格测试。

use xiao_config::{
    ConfigValue, DUPLICATE_KEY_CODE, DUPLICATE_TABLE_CODE, INVALID_EXPORT_PATH_CODE,
    MISSING_REQUIRED_CODE, UNKNOWN_FIELD_CODE, UNKNOWN_TABLE_CODE, UNSUPPORTED_CONSTRUCT_CODE,
    parse_config, parse_config_project,
};
use xiao_source::SourceFile;

/// 返回各公开规格测试共用的最小项目身份配置。
fn project_prefix() -> &'static str {
    "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n"
}

#[test]
/// 配置树应保留递归数组、字典和布尔/数值字面量。
fn preserves_recursive_static_values() {
    let source = format!(
        "{}[runtime]\nvalues = [1, 2.5, true, {{ nested = [\"x\"] }}]\n",
        project_prefix()
    );
    let document = parse_config_project(&SourceFile::from_text(&source)).expect("配置应合法");
    let value = &document
        .table("runtime")
        .expect("runtime 表")
        .get("values")
        .expect("values 字段")
        .value;
    let ConfigValue::Array(values) = value else {
        panic!("应解析为数组");
    };
    assert_eq!(values.len(), 4);
    assert!(matches!(values[2], ConfigValue::Boolean(true)));
}

#[test]
/// 多行数组和表成员缩进不应改变静态解析结果。
fn accepts_multiline_arrays() {
    let source = format!(
        "{}[runtime]\n    values = [\n        \"a\",\n        2,\n    ]\n",
        project_prefix()
    );
    assert!(parse_config_project(&SourceFile::from_text(&source)).is_ok());
}

#[test]
/// 重复键和重复表头必须分别给出稳定诊断。
fn rejects_duplicate_keys_and_tables() {
    let duplicate_key = "[project]\nname = \"a\"\nname = \"b\"\nversion = \"1\"\n".to_owned();
    let errors = parse_config_project(&SourceFile::from_text(&duplicate_key)).expect_err("重复键");
    assert!(
        errors
            .iter()
            .any(|error| error.code() == DUPLICATE_KEY_CODE)
    );

    let duplicate_table = format!("{}[runtime]\na = 1\n[Runtime]\nb = 2\n", project_prefix());
    let errors =
        parse_config_project(&SourceFile::from_text(&duplicate_table)).expect_err("重复表");
    assert!(
        errors
            .iter()
            .any(|error| error.code() == DUPLICATE_TABLE_CODE)
    );
}

#[test]
/// 严格表字段和顶层表名不能静默接受拼写错误。
fn rejects_unknown_schema_nodes() {
    let source = format!("typo = true\n{}", project_prefix());
    let errors = parse_config_project(&SourceFile::from_text(&source)).expect_err("顶层赋值");
    assert!(
        errors
            .iter()
            .any(|error| error.code() == xiao_config::CONFIG_BOUNDARY_CODE)
    );

    let source = format!("{}[mystery]\nvalue = 1\n", project_prefix());
    let errors = parse_config_project(&SourceFile::from_text(&source)).expect_err("未知表");
    assert!(
        errors
            .iter()
            .any(|error| error.code() == UNKNOWN_TABLE_CODE)
    );

    let source = "[project]\nname = \"x\"\nversion = \"1\"\nextra = 1\n".to_owned();
    let errors = parse_config_project(&SourceFile::from_text(&source)).expect_err("未知字段");
    assert!(
        errors
            .iter()
            .any(|error| error.code() == UNKNOWN_FIELD_CODE)
    );
}

#[test]
/// 函数、导入和调用等可执行构造必须在配置读取阶段拒绝。
fn rejects_executable_constructs() {
    for body in [
        "def build()\n",
        "import package\n",
        "name = input(\"x\")\n",
        "name = \"x\" + \"y\"\n",
    ] {
        let source = format!("[project]\n{body}version = \"1\"\n");
        let errors = parse_config(&SourceFile::from_text(&source)).expect_err("可执行构造");
        assert!(
            errors
                .iter()
                .any(|error| error.code() == UNSUPPORTED_CONSTRUCT_CODE),
            "{body}"
        );
    }
}

#[test]
/// 项目入口要求身份字段，导出路径必须安全且以 `.xiao` 结尾。
fn validates_project_identity_and_exports() {
    let errors = parse_config_project(&SourceFile::from_text(
        "[exports]\napi = \"src/api.xiao\"\n",
    ))
    .expect_err("缺少项目表");
    assert!(
        errors
            .iter()
            .any(|error| error.code() == MISSING_REQUIRED_CODE)
    );

    let source = format!("{}[exports]\napi = \"../api.xiao\"\n", project_prefix());
    let errors = parse_config(&SourceFile::from_text(&source)).expect_err("路径穿越");
    assert!(
        errors
            .iter()
            .any(|error| error.code() == INVALID_EXPORT_PATH_CODE)
    );
}

#[test]
/// 相对路径可以使用 `./`，规范化结果会移除冗余段。
fn normalizes_relative_export_path() {
    let source = format!(
        "{}[exports]\napi = \"./src\\\\api.xiao\"\n",
        project_prefix()
    );
    let document = parse_config_project(&SourceFile::from_text(&source)).expect("路径应合法");
    let path = document
        .table("exports")
        .expect("exports")
        .get("api")
        .expect("api")
        .value
        .as_str();
    assert_eq!(path, Some("src/api.xiao"));
}

#[test]
/// 常见损坏配置必须可恢复并返回诊断，而不是让读取器崩溃。
fn malformed_inputs_do_not_panic() {
    for source in [
        "[",
        "[project",
        "[project]\nname =",
        "[project]\nname = [1, 2\n",
        "[project]\nname = { key = 1\n",
        "[project]\nname = \"x\"\nversion = \"1\"\n    [nested]\n",
    ] {
        let result = std::panic::catch_unwind(|| parse_config(&SourceFile::from_text(source)));
        assert!(result.is_ok(), "读取损坏配置不应 panic: {source:?}");
        assert!(result.expect("catch_unwind 已确认成功").is_err());
    }
}

#[test]
/// 配置指纹输入只依赖规范化值和确定性顺序，不包含源码区间。
fn canonical_fingerprint_input_is_stable_and_span_free() {
    use std::collections::BTreeMap;

    use xiao_config::{ConfigDocument, ConfigEntry, ConfigTable, ConfigValue};
    use xiao_source::SourceSpan;

    let first_span = SourceSpan::new(1, 2).expect("valid span");
    let second_span = SourceSpan::new(100, 200).expect("valid span");
    let mut entries = BTreeMap::new();
    entries.insert(
        "name".to_owned(),
        ConfigEntry::new("name", ConfigValue::String("demo".to_owned()), first_span),
    );
    let mut tables = BTreeMap::new();
    tables.insert(
        "project".to_owned(),
        ConfigTable::new("project", entries, first_span),
    );
    let first = ConfigDocument::new(tables, first_span);

    let mut second_entries = BTreeMap::new();
    second_entries.insert(
        "name".to_owned(),
        ConfigEntry::new("name", ConfigValue::String("demo".to_owned()), second_span),
    );
    let mut second_tables = BTreeMap::new();
    second_tables.insert(
        "project".to_owned(),
        ConfigTable::new("project", second_entries, second_span),
    );
    let second = ConfigDocument::new(second_tables, second_span);

    assert_eq!(
        first.canonical_fingerprint_input(),
        second.canonical_fingerprint_input()
    );
}

#[test]
/// 配置指纹输入必须保留键顺序语义和每种静态值的类型标签。
fn canonical_fingerprint_input_distinguishes_order_and_value_types() {
    use std::collections::BTreeMap;

    use xiao_config::{ConfigDocument, ConfigEntry, ConfigTable, ConfigValue};
    use xiao_source::SourceSpan;

    let span = SourceSpan::new(1, 2).expect("valid span");
    let mut first_entries = BTreeMap::new();
    first_entries.insert(
        "alpha".to_owned(),
        ConfigEntry::new("alpha", ConfigValue::Integer(1), span),
    );
    first_entries.insert(
        "beta".to_owned(),
        ConfigEntry::new("beta", ConfigValue::String("1".to_owned()), span),
    );
    let mut first_tables = BTreeMap::new();
    first_tables.insert(
        "project".to_owned(),
        ConfigTable::new("project", first_entries, span),
    );
    let first = ConfigDocument::new(first_tables, span);

    let mut second_entries = BTreeMap::new();
    second_entries.insert(
        "alpha".to_owned(),
        ConfigEntry::new("alpha", ConfigValue::String("1".to_owned()), span),
    );
    second_entries.insert(
        "beta".to_owned(),
        ConfigEntry::new("beta", ConfigValue::Integer(1), span),
    );
    let mut second_tables = BTreeMap::new();
    second_tables.insert(
        "project".to_owned(),
        ConfigTable::new("project", second_entries, span),
    );
    let second = ConfigDocument::new(second_tables, span);

    assert_ne!(
        first.canonical_fingerprint_input(),
        second.canonical_fingerprint_input()
    );
}
