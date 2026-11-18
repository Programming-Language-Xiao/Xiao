//! 只读取入口可达模块的导入边，不扫描完整索引中不可达的模块体。

use std::collections::{BTreeMap, BTreeSet};

use xiao_ir::{IrModule, IrProgram, IrStatement, IrStatementKind};

/// 从入口导入出发计算模块闭包；模块索引仅用于查找，每个可达模块体至多访问一次。
///
/// 函数、表和控制流中的语法导入均保守纳入所属模块的边；不据运行时分支条件删边。
/// `std.` 使用既有规范身份，项目模块也可作为通往标准库依赖的中间节点。
pub(super) fn reachable_modules(program: &IrProgram) -> BTreeSet<String> {
    let index = program
        .modules
        .iter()
        .map(|module| (module.name.as_str(), module))
        .collect::<BTreeMap<_, _>>();
    let mut pending = Vec::new();
    collect_imports(&program.body, &index, &mut pending);
    let mut visited = BTreeSet::new();
    while let Some(identity) = pending.pop() {
        if visited.insert(identity.clone())
            && let Some(module) = index.get(identity.as_str())
        {
            collect_imports(&module.body, &index, &mut pending);
        }
    }
    visited
}

/// 收集单个已可达语句树的直接导入；从命名空间选取文件模块时加入对应子模块。
fn collect_imports(
    statements: &[IrStatement],
    index: &BTreeMap<&str, &IrModule>,
    pending: &mut Vec<String>,
) {
    for statement in statements {
        match &statement.kind {
            IrStatementKind::Import { items, .. } => {
                for item in items {
                    let identity = xiao_standard_library::canonical_module_identity(&item.module)
                        .unwrap_or_else(|| item.module.clone());
                    if let Some(selected) = &item.selected
                        && index
                            .get(identity.as_str())
                            .is_none_or(|module| module.kind != "file")
                    {
                        let child = format!("{identity}.{}", selected.text);
                        if index.contains_key(child.as_str()) {
                            pending.push(child);
                        }
                    }
                    pending.push(identity);
                }
            }
            IrStatementKind::Table { body, .. }
            | IrStatementKind::Function { body, .. }
            | IrStatementKind::For { body, .. }
            | IrStatementKind::While { body, .. } => collect_imports(body, index, pending),
            IrStatementKind::If {
                body,
                elif_branches,
                else_body,
                ..
            } => {
                collect_imports(body, index, pending);
                for branch in elif_branches {
                    collect_imports(&branch.body, index, pending);
                }
                if let Some(body) = else_body {
                    collect_imports(body, index, pending);
                }
            }
            IrStatementKind::Try {
                body,
                catches,
                finally_body,
            } => {
                collect_imports(body, index, pending);
                for catch in catches {
                    collect_imports(&catch.body, index, pending);
                }
                if let Some(body) = finally_body {
                    collect_imports(body, index, pending);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xiao_ir::{IrEntryMode, IrImportItem, IrName, IrSpan};

    #[test]
    /// 命名空间选择导入只展开所选子模块，不把同级未选模块的依赖纳入闭包。
    fn namespace_selection_does_not_expand_unselected_siblings() {
        let span = IrSpan::new(0, 1);
        let make_import = |module: &str, selected: Option<&str>| IrStatement {
            kind: IrStatementKind::Import {
                description: String::new(),
                items: vec![IrImportItem {
                    module: module.to_owned(),
                    selected: selected.map(|text| IrName {
                        text: text.to_owned(),
                        backticked: false,
                        span,
                    }),
                    binding: IrName {
                        text: "chosen".to_owned(),
                        backticked: false,
                        span,
                    },
                    binding_module: String::new(),
                }],
            },
            span,
            leading_docs: Vec::new(),
        };
        let make_module = |name: &str, body| IrModule {
            name: name.to_owned(),
            kind: "file".to_owned(),
            path: name.to_owned(),
            body,
            has_concurrency: false,
            symbols: Vec::new(),
        };
        let mut program = IrProgram::new(
            IrEntryMode::Script,
            vec![make_import("std.group", Some("chosen"))],
            span,
        );
        program.modules = vec![
            make_module("stdlib.group.chosen", vec![make_import("std.text", None)]),
            make_module("stdlib.group.unused", vec![make_import("std.cmath", None)]),
        ];
        assert_eq!(
            reachable_modules(&program),
            BTreeSet::from([
                "stdlib.group".to_owned(),
                "stdlib.group.chosen".to_owned(),
                "stdlib.text".to_owned(),
            ])
        );
    }
}
