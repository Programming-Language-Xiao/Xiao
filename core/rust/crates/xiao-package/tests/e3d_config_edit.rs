//! 保留格式依赖编辑与原子写回的契约回归。

use std::collections::BTreeMap;
use std::fs;

use xiao_config::{DependencyKind, parse_config_project};
use xiao_package::{DependencyEdit, edit_dependency, write_config_edit};
use xiao_source::SourceFile;

fn edit(original: &str, action: DependencyEdit) -> Result<String, xiao_package::PackageSyncError> {
    let document = parse_config_project(&SourceFile::from_text(original)).unwrap();
    edit_dependency(&document, original, &action)
}

fn add(name: &str, path: &str, kind: DependencyKind) -> DependencyEdit {
    DependencyEdit::Add {
        name: name.to_owned(),
        kind,
        fields: BTreeMap::from([("path".to_owned(), path.to_owned())]),
    }
}

fn remove(name: &str, kind: DependencyKind) -> DependencyEdit {
    DependencyEdit::Remove {
        name: name.to_owned(),
        kind,
    }
}

#[test]
fn preserves_order_comments_quotes_and_crlf() {
    let original = "# 总注释\r\n[project]\r\nname = 'app'\r\nversion = '1'\r\n\r\n[dependencies]\r\n# 第一个包\r\nold = { path = '../old' } # 行尾说明\r\n\r\n# 保留空行\r\n[toolchain]\r\n";
    let inserted = edit(original, add("new-pkg", "../new", DependencyKind::Runtime)).unwrap();
    assert!(inserted.contains("old = { path = '../old' } # 行尾说明\r\n\"new-pkg\" = { path = \"../new\" }\r\n\r\n# 保留空行"));
    assert!(inserted.starts_with("# 总注释\r\n[project]\r\nname = 'app'"));
    let removed = edit(&inserted, remove("old", DependencyKind::Runtime)).unwrap();
    assert!(removed.contains("# 第一个包\r\n # 行尾说明\r\n\"new-pkg\""));
    assert!(!removed.contains("old ="));
    let restored = edit(&removed, remove("new-pkg", DependencyKind::Runtime)).unwrap();
    assert!(restored.contains("# 保留空行\r\n[toolchain]"));
}

#[test]
fn new_table_and_conflicting_edits() {
    let original = "[project]\nname = \"app\"\nversion = \"1\"";
    let inserted = edit(original, add("lib", "../lib", DependencyKind::Development)).unwrap();
    assert!(inserted.ends_with("\n\n[devdependencies]\n\"lib\" = { path = \"../lib\" }\n"));
    assert!(
        edit(
            &inserted,
            add("lib", "../other", DependencyKind::Development)
        )
        .is_err()
    );
    assert!(edit(original, remove("lib", DependencyKind::Runtime)).is_err());
    assert!(
        edit(
            original,
            add("NOT-VALID", "../lib", DependencyKind::Runtime)
        )
        .is_err()
    );
}

#[test]
fn new_table_respects_crlf_even_without_final_newline() {
    let original = "[project]\r\nname = \"app\"\r\nversion = \"1\"";
    let inserted = edit(original, add("lib", "../lib", DependencyKind::Runtime)).unwrap();
    assert!(inserted.ends_with("\r\n\r\n[dependencies]\r\n\"lib\" = { path = \"../lib\" }\r\n"));
    assert!(!inserted.replace("\r\n", "").contains('\n'));
}

#[test]
fn mismatched_unicode_span_returns_error_instead_of_panicking() {
    let original =
        "[project]\nname = \"app\"\nversion = \"1\"\n[dependencies]\nlib = { path = \"../lib\" }\n";
    let document = parse_config_project(&SourceFile::from_text(original)).unwrap();
    let start = document
        .table("dependencies")
        .unwrap()
        .get("lib")
        .unwrap()
        .span
        .start();
    let mismatched = format!("{}é{}", &original[..start - 1], &original[start + 1..]);
    assert_eq!(mismatched.len(), original.len());
    assert!(
        edit_dependency(
            &document,
            &mismatched,
            &remove("lib", DependencyKind::Runtime)
        )
        .is_err()
    );
}

#[test]
fn invalid_constraint_and_path_are_not_committed() {
    let original = "[project]\nname = \"app\"\nversion = \"1\"\n";
    let action = DependencyEdit::Add {
        name: "lib".to_owned(),
        kind: DependencyKind::Runtime,
        fields: BTreeMap::from([
            ("path".to_owned(), "../lib".to_owned()),
            ("version".to_owned(), "not-a-version".to_owned()),
        ]),
    };
    assert!(edit(original, action).is_err());
    assert!(edit(original, add("lib", "C:/outside", DependencyKind::Runtime)).is_err());
}

#[test]
fn stale_original_preserves_file() {
    let folder = std::env::temp_dir().join(format!(
        "xiao-e3d-edit-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("config.xiao");
    let original = "[project]\nname = \"app\"\nversion = \"1\"\n";
    fs::write(&path, original).unwrap();
    let edited = edit(original, add("lib", "../lib", DependencyKind::Runtime)).unwrap();
    fs::write(&path, "changed").unwrap();
    assert!(write_config_edit(&path, original, &edited).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "changed");
    fs::write(&path, original).unwrap();
    write_config_edit(&path, original, &edited).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), edited);
    fs::remove_dir_all(folder).unwrap();
}
