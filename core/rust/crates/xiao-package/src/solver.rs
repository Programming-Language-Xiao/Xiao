//! 11A-E3D：确定性依赖约束求解。

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostics::{
    SOURCE_AMBIGUOUS_CODE, SOURCE_INVALID_CODE, SOURCE_UNKNOWN_REFERENCE_CODE,
    VERSION_UNSATISFIED_CODE,
};
use crate::federation::{FederatedRecord, IndexPackage, SourceSnapshot};
use crate::selection::select_source;
use crate::source::{ConfiguredSource, SourceError};
use crate::version::{Version, VersionRequirement};

/// 项目或传递依赖要求的包名、版本和可选来源。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolveRequirement {
    /// 规范化包名。
    pub name: String,
    /// 原始版本约束文本。
    pub version: String,
    /// 源别名或源身份。
    pub source: Option<String>,
}

/// 求解结果中保留候选及其可审计的来源快照。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPackage {
    /// 完整的发布版本元数据。
    pub package: IndexPackage,
    /// 规范来源身份。
    pub source_id: String,
    /// 来源快照标识。
    pub snapshot_id: String,
    /// 来源快照摘要。
    pub snapshot_digest: String,
}

#[derive(Clone)]
struct Constraint {
    version: VersionRequirement,
    source: Option<String>,
}

#[derive(Clone, Default)]
struct State<'a> {
    constraints: BTreeMap<String, Vec<Constraint>>,
    selected: BTreeMap<String, &'a FederatedRecord>,
}

impl State<'_> {
    fn require(&mut self, requirement: &SolveRequirement) -> Result<(), SourceError> {
        self.constraints
            .entry(requirement.name.clone())
            .or_default()
            .push(Constraint {
                version: VersionRequirement::parse(&requirement.version)?,
                source: requirement.source.clone(),
            });
        Ok(())
    }
}

/// 从已验证索引求解依赖：先选来源，再按 SemVer 降序尝试候选并回溯。
///
/// 此入口尚无特性及 Xiao/Runtime 兼容性上下文；带目标、ABI 或兼容范围的候选
/// 在接口扩展前不会被选择，避免将条件包误当成通用包。
pub fn solve_dependencies(
    sources: &[ConfiguredSource],
    snapshots: &[SourceSnapshot],
    records: &[FederatedRecord],
    requirements: &[SolveRequirement],
    target_variant: Option<&str>,
) -> Result<BTreeMap<String, ResolvedPackage>, SourceError> {
    let mut state = State::default();
    for requirement in requirements {
        state.require(requirement)?;
    }
    for record in records {
        if record.key.name != record.package.name
            || record.key.version != record.package.version
            || record.key.variant != record.package.variant
        {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "联邦记录与包元数据的身份不一致",
            ));
        }
        Version::parse(&record.package.version)?;
        for dependency in &record.package.dependencies {
            VersionRequirement::parse(&dependency.version)?;
        }
    }
    let solved = search(sources, snapshots, records, target_variant, state)?
        .ok_or_else(|| unsatisfied("所有候选均无法满足依赖约束"))?;
    Ok(solved
        .selected
        .into_iter()
        .map(|(name, record)| {
            (
                name,
                ResolvedPackage {
                    package: record.package.clone(),
                    source_id: record.key.source_id.clone(),
                    snapshot_id: record.snapshot_id.clone(),
                    snapshot_digest: record.snapshot_digest.clone(),
                },
            )
        })
        .collect())
}

fn search<'a>(
    sources: &[ConfiguredSource],
    snapshots: &[SourceSnapshot],
    records: &'a [FederatedRecord],
    target_variant: Option<&str>,
    state: State<'a>,
) -> Result<Option<State<'a>>, SourceError> {
    for (name, constraints) in &state.constraints {
        if let Some(record) = state.selected.get(name) {
            let binding = match source_binding(sources, constraints) {
                Err(error) if error.code == VERSION_UNSATISFIED_CODE => return Ok(None),
                result => result?,
            };
            if binding
                .as_ref()
                .is_some_and(|source_id| source_id != &record.key.source_id)
                || !satisfies(record, constraints, target_variant)
            {
                return Ok(None);
            }
        }
    }
    let Some(name) = state
        .constraints
        .keys()
        .find(|name| !state.selected.contains_key(*name))
    else {
        return Ok((!has_cycle(&state.selected)).then_some(state));
    };
    let constraints = &state.constraints[name];
    let binding = match source_binding(sources, constraints) {
        Err(error) if error.code == VERSION_UNSATISFIED_CODE => return Ok(None),
        result => result?,
    };
    let mut available = sources.to_vec();
    loop {
        let matching = select_source(
            &available,
            snapshots,
            records,
            name,
            binding.as_deref(),
            |record| satisfies(record, constraints, target_variant),
        )?;
        if matching.is_empty() {
            return Ok(None);
        }
        let chosen_order = matching[0].key.config_order;
        let mut ranked = matching
            .into_iter()
            .map(|record| Ok((Version::parse(&record.package.version)?, record)))
            .collect::<Result<Vec<_>, SourceError>>()?;
        ranked.sort_by(|(left, _), (right, _)| right.precedence(left));
        let mut index = 0;
        while index < ranked.len() {
            let mut end = index + 1;
            while end < ranked.len()
                && ranked[index].0.precedence(&ranked[end].0) == Ordering::Equal
            {
                end += 1;
            }
            let mut solution = None;
            for (_, record) in &ranked[index..end] {
                let mut branch = state.clone();
                branch.selected.insert(name.clone(), record);
                for dependency in &record.package.dependencies {
                    branch.require(&SolveRequirement {
                        name: dependency.name.clone(),
                        version: dependency.version.clone(),
                        source: dependency.source.clone(),
                    })?;
                }
                if let Some(next) = search(sources, snapshots, records, target_variant, branch)? {
                    if solution.is_some() {
                        return Err(SourceError::new(
                            SOURCE_AMBIGUOUS_CODE,
                            format!("包 {name:?} 存在多个同优先级可行候选"),
                        ));
                    }
                    solution = Some(next);
                }
            }
            if solution.is_some() {
                return Ok(solution);
            }
            index = end;
        }
        if binding.is_some() {
            return Ok(None);
        }
        available.retain(|source| source.config_order != chosen_order);
    }
}

fn satisfies(
    record: &FederatedRecord,
    constraints: &[Constraint],
    target_variant: Option<&str>,
) -> bool {
    if record.package.withdrawn
        || !record.package.features.is_empty()
        || record.package.target.is_some()
        || record.package.abi.is_some()
        || record.package.xiao_range.is_some()
        || record.package.runtime_range.is_some()
        || !matches!(target_variant, Some(target) if record.package.variant == target)
            && record.package.variant != "any"
    {
        return false;
    }
    let Ok(version) = Version::parse(&record.package.version) else {
        return false;
    };
    (!version.is_prerelease()
        || constraints
            .iter()
            .any(|constraint| constraint.version.permits_prerelease(&version)))
        && constraints
            .iter()
            .all(|constraint| constraint.version.matches_precedence(&version))
}

fn source_binding(
    sources: &[ConfiguredSource],
    constraints: &[Constraint],
) -> Result<Option<String>, SourceError> {
    let mut selected = None;
    for binding in constraints.iter().filter_map(|item| item.source.as_deref()) {
        let matches = sources
            .iter()
            .filter(|source| {
                source.descriptor.source.alias.as_deref() == Some(binding)
                    || source.descriptor.source.source_id == binding
            })
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return Err(SourceError::new(
                SOURCE_UNKNOWN_REFERENCE_CODE,
                "依赖引用的包源不存在",
            ));
        }
        if matches.len() != 1 {
            return Err(SourceError::new(
                SOURCE_AMBIGUOUS_CODE,
                "依赖包源引用不唯一",
            ));
        }
        let source_id = &matches[0].descriptor.source.source_id;
        if selected
            .as_ref()
            .is_some_and(|previous| previous != source_id)
        {
            return Err(unsatisfied("同一包的约束指定了不同来源"));
        }
        selected = Some(source_id.clone());
    }
    Ok(selected)
}

fn unsatisfied(message: &str) -> SourceError {
    SourceError::new(VERSION_UNSATISFIED_CODE, message)
}

fn has_cycle(selected: &BTreeMap<String, &FederatedRecord>) -> bool {
    fn visit(
        name: &str,
        selected: &BTreeMap<String, &FederatedRecord>,
        active: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if active.contains(name) {
            return true;
        }
        if visited.contains(name) {
            return false;
        }
        active.insert(name.to_owned());
        if selected.get(name).is_some_and(|record| {
            record
                .package
                .dependencies
                .iter()
                .any(|dependency| visit(&dependency.name, selected, active, visited))
        }) {
            return true;
        }
        active.remove(name);
        visited.insert(name.to_owned());
        false
    }

    let mut active = BTreeSet::new();
    let mut visited = BTreeSet::new();
    selected
        .keys()
        .any(|name| visit(name, selected, &mut active, &mut visited))
}
