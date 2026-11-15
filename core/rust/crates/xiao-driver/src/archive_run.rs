//! 官方标准库由统一前端提供，应用模块只从验证后的归档索引读取。

use crate::module_loader::DriverModuleLoader;

/// 使用工具链固定官方加载器运行归档；不读取已删除的应用源码或普通包环境。
pub fn run_archive(
    bytes: &[u8],
    options: xiao_xar::XarRunOptions,
) -> Result<xiao_vm::RunOutcome, xiao_xar::XarRunError> {
    run_archive_with_control(bytes, options, None)
}

/// 与源码运行共用官方加载器，同时保留归档的取消、版本和能力预检。
pub fn run_archive_with_control(
    bytes: &[u8],
    options: xiao_xar::XarRunOptions,
    cancellation: Option<xiao_vm::CancellationSource>,
) -> Result<xiao_vm::RunOutcome, xiao_xar::XarRunError> {
    let official = DriverModuleLoader::new(None, None);
    xiao_xar::run_archive_with_module_loader(bytes, options, cancellation, Some(&official))
}
