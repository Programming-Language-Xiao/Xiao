//! 归档输出与构建输入、受管理缓存之间的路径隔离。

use std::path::{Component, Path, PathBuf};

/// 在创建缓存或提交产物之前检查输出，不允许归档替换编译输入或缓存内容。
pub(crate) fn validate_archive_output(
    output: &Path,
    inputs: &[PathBuf],
    cache_root: &Path,
) -> Result<(), String> {
    let destination = resolve_destination(output).map_err(|error| error.to_string())?;
    let cache = resolve_destination(cache_root).map_err(|error| error.to_string())?;
    if destination.starts_with(cache) {
        return Err("归档输出不能位于受管理的全局缓存中".to_owned());
    }
    for input in inputs {
        let source = resolve_destination(input).map_err(|error| error.to_string())?;
        if destination == source {
            return Err(format!("归档输出不能覆盖构建输入：{}", input.display()));
        }
    }
    Ok(())
}

/// 逐段解析现有符号链接并保留未创建的尾部，正确处理符号链接之后的 `..`。
fn resolve_destination(path: &Path) -> std::io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let mut resolved = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => resolved.push(component),
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            Component::Normal(segment) => {
                resolved.push(segment);
                if resolved.try_exists()? {
                    resolved = resolved.canonicalize()?;
                }
            }
        }
    }
    Ok(resolved)
}
