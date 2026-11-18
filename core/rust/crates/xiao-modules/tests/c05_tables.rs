//! 05-C 表在模块发现和词法作用域中的最小闭环测试。

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use xiao_modules::{ModuleName, ModuleSymbolKind, analyze_project};

/// 为并行运行的临时项目夹具生成进程内唯一后缀。
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// 管理一个测试期间创建并在销毁时清理的临时 Xiao 项目。
struct TempProject(PathBuf);

impl TempProject {
    /// 创建一个隔离的临时项目目录。
    fn new() -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("xiao-c05-modules-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).expect("create fixture");
        Self(path)
    }

    /// 写入一个测试用 Xiao 源文件。
    fn write(&self, name: &str, source: &str) {
        fs::write(self.0.join(name), source).expect("write fixture");
    }
}

impl Drop for TempProject {
    /// 清理测试目录。
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
/// 表应作为可导出的模块符号，方法体应进入独立词法作用域。
fn discovers_table_symbol_and_method_scope() {
    let project = TempProject::new();
    project.write(
        "main.xiao",
        "[Config]\n    value = 1\n    def get(self) -> int\n        return self.value\n",
    );
    let result = analyze_project(&project.0);
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    let module = result
        .modules
        .get(&ModuleName::new(vec!["main".to_owned()]))
        .expect("main module");
    assert_eq!(
        module.symbols.get("Config").expect("Config").kind,
        ModuleSymbolKind::Table
    );
}
