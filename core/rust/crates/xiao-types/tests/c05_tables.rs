//! 05-C 表类型、成员访问和构造生命周期契约测试。

use xiao_source::SourceFile;
use xiao_syntax::Parser;
use xiao_types::{
    ScalarType, TABLE_CONSTRUCTOR_CODE, TABLE_INITIALIZER_CODE, TABLE_LIFECYCLE_CODE,
    TABLE_VISIBILITY_CODE, TableValueKind, Type, TypeChecker,
};

/// 解析并检查一段表语法测试源码。
fn check(source_text: &str) -> xiao_types::TypeCheckResult {
    let source = SourceFile::from_text(source_text);
    let parsed = Parser::new(&source).parse();
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    TypeChecker::check(&source, parsed.program.as_ref().expect("program"))
}

#[test]
/// 表签名应登记字段、方法和单例/构造目标形态。
fn registers_table_signature() {
    let result = check(
        "[Config]\n    str host = \"localhost\"\n    def label(self) -> str\n        return self.host\n",
    );
    assert!(
        result.is_success(),
        "diagnostics: {:?}",
        result.diagnostics()
    );
    let signature = result.table_signatures().get("Config").expect("Config");
    assert!(!signature.is_instantiable());
    assert_eq!(
        signature.member("ascii:host").expect("host").ty,
        Type::scalar(ScalarType::Str)
    );
    assert!(signature.member("ascii:label").expect("label").is_method());
    assert_eq!(
        result.binding("Config").expect("table binding").scheme.ty,
        Type::Table(xiao_types::TableType::singleton("Config"))
    );
}

#[test]
/// `[[Table]]` 允许构造，并把 `new` 的结果标记为实例类型。
fn constructs_instance_table() {
    let result =
        check("[[User]]\n    def init(self, int id) -> none\n        return\nuser = new User(1)\n");
    assert!(
        result.is_success(),
        "diagnostics: {:?}",
        result.diagnostics()
    );
    assert_eq!(
        result.binding("user").expect("user").scheme.ty,
        Type::Table(xiao_types::TableType::instance("User"))
    );
    assert_eq!(
        result
            .table_signatures()
            .get("User")
            .expect("User")
            .declaration_kind,
        xiao_syntax::TableKind::Instance
    );
    assert!(TableValueKind::Constructor.is_constructor());
}

#[test]
/// 单例表不可用 `new`，缺少 init 的实例表也不接受构造参数。
fn diagnoses_constructor_contracts() {
    let singleton = check("[Config]\n    value = 1\nconfig = new Config()\n");
    assert!(
        singleton
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == TABLE_CONSTRUCTOR_CODE)
    );
    let no_init = check("[[User]]\n    value = 1\nuser = new User(1)\n");
    assert!(
        no_init
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == TABLE_CONSTRUCTOR_CODE)
    );
}

#[test]
/// 下划线成员默认私有，表外访问必须被拒绝。
fn diagnoses_private_member_access() {
    let result = check("[Config]\n    _secret = 1\nvalue = Config._secret\n");
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == TABLE_VISIBILITY_CODE),
        "diagnostics: {:?}",
        result.diagnostics()
    );
}

#[test]
/// init/drop 必须有 self，drop 不能接收额外参数，且生命周期方法返回 none。
fn diagnoses_lifecycle_contracts() {
    let result = check(
        "[[User]]\n    def init(int id) -> int\n        return id\n    def drop(self, int extra) -> int\n        return extra\n",
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == TABLE_LIFECYCLE_CODE)
    );
}

#[test]
/// 表字段初始化器不得调用动态输入或普通函数。
fn diagnoses_dynamic_field_initializer() {
    let result = check("[Config]\n    value = input(\"host\")\n");
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == TABLE_INITIALIZER_CODE)
    );
}
