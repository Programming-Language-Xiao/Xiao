//! 环境包命名空间的纯静态检查。

use std::collections::{BTreeMap, BTreeSet};

use xiao_source::SourceFile;
use xiao_syntax::parse;
use xiao_types::{ExternalNamespaces, TypeChecker, UNDEFINED_NAME_CODE};

fn check(text: &str) -> xiao_types::TypeCheckResult {
    let source = SourceFile::from_text(text);
    let program = parse(&source).program.expect("valid program");
    let namespaces = ExternalNamespaces {
        members: BTreeMap::from([
            ("lib".to_owned(), BTreeSet::from(["api".to_owned()])),
            ("lib.api".to_owned(), BTreeSet::from(["value".to_owned()])),
        ]),
    };
    TypeChecker::new(&source)
        .with_external_namespaces(namespaces)
        .check_program(&program)
}

#[test]
fn resolves_nested_package_members_without_running_code() {
    let result = check("value = lib.api.value\n");
    assert!(result.is_success(), "{:?}", result.diagnostics());
}

#[test]
fn rejects_unknown_package_member_using_existing_diagnostic() {
    let result = check("value = lib.api.missing\n");
    assert_eq!(result.diagnostics()[0].code(), UNDEFINED_NAME_CODE);
}
