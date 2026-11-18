//! LLVM Pass 映射、开关和可证明 Runtime 裁剪报告（15A）。
//!
//! 这里不重造 13A 配置：所有级别、配置指纹和默认值都来自
//! [`xiao_optimizer::OptimizationConfig`]。本模块只把它映射到安全的 LLVM 参数，
//! 明确拒绝快速数学、未定义溢出回绕、LTO/PGO/目标特化等尚未冻结的能力。

use std::collections::BTreeMap;

use xiao_optimizer::{OptimizationConfig, OptimizationLevel};

use crate::target::TargetDescription;

/// LLVM 侧可控的 Pass 类别。
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LlvmPassKind {
    /// 函数内优化；由 `-O1` 至 `-O3` 的安全 LLVM 默认管线承载。
    Function,
    /// 跨模块优化；首版只建开关，默认关闭。
    CrossModule,
    /// 链接时优化；首版只建开关，默认关闭。
    LinkTime,
    /// 目标特化；首版只建开关，默认关闭。
    TargetSpecific,
}

/// LLVM 扩展 Pass 开关；首版只建立边界，默认全部关闭。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LlvmPassSwitches {
    /// 是否允许跨模块 Pass。
    pub cross_module: bool,
    /// 是否允许链接时 Pass。
    pub link_time: bool,
    /// 是否允许目标特化 Pass。
    pub target_specific: bool,
}

/// 15A 的 LLVM 优化计划。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlvmOptimizationPlan {
    /// 13A 规范化配置指纹。
    pub config_fingerprint: String,
    /// 规范化优化级别。
    pub level: OptimizationLevel,
    /// 目标指纹字段。
    pub target: String,
    /// LLVM 版本摘要；未探测时为稳定占位文本。
    pub llvm_version: String,
    /// 实际启用的 Pass 类别。
    pub enabled_passes: Vec<LlvmPassKind>,
    /// 扩展 Pass 开关。
    pub switches: LlvmPassSwitches,
    /// 传给 clang 的安全优化参数。
    pub compiler_flags: Vec<String>,
    /// LTO/PGO/目标特化开关的显式状态。
    pub disabled_extensions: BTreeMap<String, String>,
    /// 纳入构建产物的稳定指纹。
    pub fingerprint: String,
}

/// LLVM Pass 与 Runtime 裁剪的可解释报告。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlvmOptimizationReport {
    /// 优化计划。
    pub plan: LlvmOptimizationPlan,
    /// 可证明从 IR 效果摘要可达的 Runtime 组件。
    pub retained_runtime_components: Vec<String>,
    /// 没有进入调用图的 Runtime 组件。
    pub removed_runtime_components: Vec<String>,
    /// 每个组件的保留/删除理由。
    pub runtime_reasons: BTreeMap<String, String>,
}

impl LlvmOptimizationPlan {
    /// 从 13A 配置、目标和 LLVM 版本摘要建立安全计划。
    pub fn from_config(
        config: &OptimizationConfig,
        target: &TargetDescription,
        llvm_version: impl Into<String>,
    ) -> Result<Self, String> {
        Self::from_config_with_switches(config, target, llvm_version, LlvmPassSwitches::default())
    }

    /// 从 13A 配置和显式扩展开关建立计划；首版拒绝启用尚未冻结的扩展。
    pub fn from_config_with_switches(
        config: &OptimizationConfig,
        target: &TargetDescription,
        llvm_version: impl Into<String>,
        switches: LlvmPassSwitches,
    ) -> Result<Self, String> {
        let normalized = config
            .clone()
            .normalize()
            .map_err(|error| error.to_string())?;
        if switches.cross_module
            || switches.link_time
            || switches.target_specific
            || normalized.allow_lto
            || normalized.allow_cpu_specialization
            || normalized.experimental_passes.iter().any(|pass| {
                pass.contains("fast-math") || pass.contains("wrap") || pass.contains("vector")
            })
        {
            return Err("LLVM 15A 禁止快速数学、未定义回绕、LTO、向量化和目标特化".to_owned());
        }
        let level = normalized.level;
        let config_fingerprint = normalized
            .fingerprint()
            .map_err(|error| error.to_string())?;
        let mut compiler_flags = vec![format!("-O{}", level.as_u8())];
        // 这些参数明确禁止，不能因为调用方传了自定义 pass 名称就悄悄加入。
        compiler_flags.retain(|flag| !flag.contains("ffast-math") && !flag.contains("fwrapv"));
        let mut disabled_extensions = BTreeMap::new();
        disabled_extensions.insert("cross-module".to_owned(), "default-disabled".to_owned());
        disabled_extensions.insert("link-time".to_owned(), "default-disabled".to_owned());
        disabled_extensions.insert("target-specific".to_owned(), "default-disabled".to_owned());
        let llvm_version = llvm_version.into();
        let canonical = format!(
            "config={};level={};target={};llvm={};passes=function;extensions=cross-module:off,link-time:off,target-specific:off",
            config_fingerprint.as_str(),
            level.as_u8(),
            target.fingerprint_fields(),
            llvm_version
        );
        Ok(Self {
            config_fingerprint: config_fingerprint.as_str().to_owned(),
            level,
            target: target.fingerprint_fields(),
            llvm_version,
            enabled_passes: vec![LlvmPassKind::Function],
            switches,
            compiler_flags,
            disabled_extensions,
            fingerprint: format!("xiao-llvm-fnv1a64-{}", stable_hash(canonical.as_bytes())),
        })
    }

    /// 用实际探测到的 LLVM 版本更新计划和指纹。
    #[must_use]
    pub fn with_llvm_version(mut self, llvm_version: impl Into<String>) -> Self {
        self.llvm_version = llvm_version.into();
        let canonical = format!(
            "config={};level={};target={};llvm={};passes=function;extensions=cross-module:off,link-time:off,target-specific:off",
            self.config_fingerprint,
            self.level.as_u8(),
            self.target,
            self.llvm_version
        );
        self.fingerprint = format!("xiao-llvm-fnv1a64-{}", stable_hash(canonical.as_bytes()));
        self
    }
}

impl LlvmOptimizationReport {
    /// 创建后端结构初始化所需的安全默认报告。
    #[must_use]
    pub fn empty(target: &TargetDescription, level: u8) -> Self {
        let level = level.min(3);
        let config = OptimizationConfig::baseline(target.fingerprint_fields())
            .with_level(OptimizationLevel::try_from(level).unwrap_or(OptimizationLevel::O0));
        let plan = Self::plan_or_fallback(&config, target);
        Self::from_plan(plan, &[])
    }

    fn plan_or_fallback(
        config: &OptimizationConfig,
        target: &TargetDescription,
    ) -> LlvmOptimizationPlan {
        LlvmOptimizationPlan::from_config(config, target, "unknown")
            .expect("默认 LLVM 优化计划必须合法")
    }

    /// 根据 IR 降低实际登记的 Runtime 组件建立可证明裁剪报告。
    #[must_use]
    pub fn from_plan(plan: LlvmOptimizationPlan, components: &[String]) -> Self {
        let mut retained = components.to_vec();
        retained.sort();
        retained.dedup();
        let all = ["value", "rc", "weak", "containers", "tables"];
        let removed = all
            .iter()
            .filter(|component| !retained.iter().any(|item| item == **component))
            .map(|component| (*component).to_owned())
            .collect::<Vec<_>>();
        let mut reasons = BTreeMap::new();
        for component in &retained {
            reasons.insert(
                component.clone(),
                "由 IR Runtime 效果摘要和生成的 ABI 调用图证明可达".to_owned(),
            );
        }
        for component in &removed {
            reasons.insert(
                component.clone(),
                "未出现在 IR 效果摘要的可达 Runtime 调用图中".to_owned(),
            );
        }
        Self {
            plan,
            retained_runtime_components: retained,
            removed_runtime_components: removed,
            runtime_reasons: reasons,
        }
    }

    /// 用实际工具链版本回填报告，保持 Runtime 裁剪事实不变。
    #[must_use]
    pub fn with_llvm_version(self, llvm_version: impl Into<String>) -> Self {
        let plan = self.plan.with_llvm_version(llvm_version);
        Self::from_plan(plan, &self.retained_runtime_components)
    }
}

fn stable_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::TargetDescription;

    #[test]
    fn plan_keeps_extensions_off_and_fingerprint_stable() {
        let config = OptimizationConfig::baseline("elf").with_level(OptimizationLevel::O2);
        let first = LlvmOptimizationPlan::from_config(
            &config,
            &TargetDescription::linux_x86_64(),
            "llvm-21",
        )
        .expect("LLVM 计划");
        let second = LlvmOptimizationPlan::from_config(
            &config,
            &TargetDescription::linux_x86_64(),
            "llvm-21",
        )
        .expect("LLVM 计划");
        assert_eq!(first, second);
        assert_eq!(first.compiler_flags, vec!["-O2"]);
        assert!(first.enabled_passes.contains(&LlvmPassKind::Function));
        assert!(!first.enabled_passes.contains(&LlvmPassKind::LinkTime));
    }

    #[test]
    fn runtime_report_explains_retained_and_removed_components() {
        let config = OptimizationConfig::baseline("elf");
        let plan = LlvmOptimizationPlan::from_config(
            &config,
            &TargetDescription::linux_x86_64(),
            "unknown",
        )
        .expect("LLVM 计划");
        let report = LlvmOptimizationReport::from_plan(plan, &["value".to_owned()]);
        assert_eq!(report.retained_runtime_components, vec!["value"]);
        assert!(
            report
                .removed_runtime_components
                .contains(&"containers".to_owned())
        );
        assert!(report.runtime_reasons.contains_key("value"));
    }

    #[test]
    fn extension_switches_are_explicitly_rejected_until_frozen() {
        let config = OptimizationConfig::baseline("elf");
        let error = LlvmOptimizationPlan::from_config_with_switches(
            &config,
            &TargetDescription::linux_x86_64(),
            "llvm-21",
            LlvmPassSwitches {
                link_time: true,
                ..LlvmPassSwitches::default()
            },
        )
        .expect_err("LTO/链接时开关当前不能启用");
        assert!(error.contains("禁止"));
    }
}
