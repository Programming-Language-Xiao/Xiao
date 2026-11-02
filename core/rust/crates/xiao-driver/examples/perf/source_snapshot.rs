//! 将原生/VM测量限制到仓库标准库，并用运行开始时的摘要拒绝中途源码变化。

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

/// 本次测量固定使用的源码根和七模块摘要。
pub(super) struct Sources {
    root: PathBuf,
    digests: BTreeMap<&'static str, String>,
}

impl Sources {
    /// 不修改进程环境；加载器若被指向其他stdlib，报告生成前直接拒绝。
    pub(super) fn capture(repository: &Path) -> io::Result<Self> {
        Self::capture_with_root(repository, &xiao_standard_library::source_root())
    }

    /// 按明确加载根建立快照，供无全局环境修改的回归使用。
    fn capture_with_root(repository: &Path, loader_root: &Path) -> io::Result<Self> {
        let root = repository.join("stdlib").canonicalize()?;
        check_loader_root(&root, loader_root)?;
        let digests = ["math", "hex", "base64", "url", "csv", "fmt", "statistics"]
            .into_iter()
            .map(|module| Ok((module, digest(&root.join(format!("{module}.xiao")))?)))
            .collect::<io::Result<BTreeMap<_, _>>>()?;
        Ok(Self { root, digests })
    }

    /// 每项采样前后都检查，防止把缓存程序或旧源码的结果署上新摘要。
    pub(super) fn check_current(&self) -> io::Result<()> {
        self.check_with_root(&xiao_standard_library::source_root())
    }

    /// 检查实际加载根与所有模块字节都仍属于本次快照。
    fn check_with_root(&self, loader_root: &Path) -> io::Result<()> {
        check_loader_root(&self.root, loader_root)?;
        for (module, expected) in &self.digests {
            if digest(&self.root.join(format!("{module}.xiao")))? != *expected {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("测量中标准库源码发生变化：{module}"),
                ));
            }
        }
        Ok(())
    }

    /// 报告只使用开始时已固定的摘要，不在测量后重新认领当前磁盘版本。
    pub(super) fn product_digest(&self, module: &str) -> &str {
        &self.digests[module]
    }
}

/// 规范化路径后比较，允许指向同一目录的路径别名。
fn check_loader_root(expected: &Path, loader_root: &Path) -> io::Result<()> {
    let actual = loader_root.canonicalize()?;
    if actual != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "F10必须使用仓库stdlib；实际加载根={}，要求={}",
                actual.display(),
                expected.display()
            ),
        ));
    }
    Ok(())
}

/// 与报告和生产工具共用SHA-256字节摘要。
fn digest(path: &Path) -> io::Result<String> {
    Ok(xiao_artifacts::Digest256::of_bytes(&std::fs::read(path)?).to_string())
}

#[cfg(test)]
/// 用独立源码树复现覆盖和中途改写，不设置全局XIAO_STDLIB_ROOT。
mod tests {
    use super::Sources;

    #[test]
    /// 其他加载根即使内容相同也不接纳；任一模块被改写后不能继续署原摘要。
    fn loader_override_and_source_changes_cannot_mislabel_measurements() {
        let repository = super::super::run_directory::create("xiao-b1-source-test").unwrap();
        let root = repository.join("stdlib");
        let other = repository.join("other-stdlib");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&other).unwrap();
        let modules = ["math", "hex", "base64", "url", "csv", "fmt", "statistics"];
        let text = b"def identity(int value) -> int\n    return value\n";
        for module in modules {
            std::fs::write(root.join(format!("{module}.xiao")), text).unwrap();
            std::fs::write(other.join(format!("{module}.xiao")), text).unwrap();
        }
        assert!(Sources::capture_with_root(&repository, &other).is_err());
        assert!(Sources::capture(&repository).is_err());
        let sources = Sources::capture_with_root(&repository, &root).unwrap();
        sources.check_with_root(&root).unwrap();
        assert!(sources.check_current().is_err());
        assert_eq!(
            sources.product_digest("math"),
            xiao_artifacts::Digest256::of_bytes(text).to_string()
        );
        std::fs::write(
            root.join("math.xiao"),
            b"def identity(int value) -> int\n    return 0\n",
        )
        .unwrap();
        assert!(sources.check_with_root(&root).is_err());

        for module in modules {
            std::fs::remove_file(root.join(format!("{module}.xiao"))).unwrap();
            std::fs::remove_file(other.join(format!("{module}.xiao"))).unwrap();
        }
        std::fs::remove_dir(root).unwrap();
        std::fs::remove_dir(other).unwrap();
        std::fs::remove_dir(repository).unwrap();
    }
}
