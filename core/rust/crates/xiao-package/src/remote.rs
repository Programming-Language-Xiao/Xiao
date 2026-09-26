//! 将静态远程需求、联邦求解结果和锁定正文汇入现有完整包图。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use xiao_config::{
    ConfigDocument, DependencyKind, RemoteDependencyDeclaration, parse_config_project,
    remote_dependency_declarations,
};
use xiao_source::{SourceFile, SourceSpan};

use crate::adapters::{GitReference, MultiSourceAdapter, PackageSourceAdapter};
use crate::cache::CacheStore;
use crate::diagnostics::{
    LOCKFILE_CONFIG_MISMATCH_CODE, SOURCE_INVALID_CODE, SYNC_LOCK_REQUIRED_CODE,
    TRUST_ARTIFACT_CODE, TRUST_SOURCE_CODE, VERSION_UNSATISFIED_CODE,
};
use crate::environment::fingerprint_config;
use crate::fastpath::SourceResolver;
use crate::fetch::import_artifact;
use crate::lockfile::{LockFile, LockedDependency, LockedPackage, LockedSourceSnapshot};
use crate::model::{PackageDependency, PackageEdge, PackageGraph, PackageIdentity, PackageNode};
use crate::solver::{SolveRequirement, solve_dependencies};
use crate::source::{ConfiguredSource, expand_source_lists, source_declarations};
use crate::source_lists;
use crate::sync::{PackageOperation, PackageSyncError, failure};

type Result<T> = std::result::Result<T, PackageSyncError>;

pub(crate) struct PreparedGraph {
    pub(crate) graph: PackageGraph,
    packages: BTreeMap<String, LockedPackage>,
    snapshots: BTreeMap<String, LockedSourceSnapshot>,
}

impl PreparedGraph {
    pub(crate) fn annotate_lockfile(&self, lock: &mut LockFile) -> Result<()> {
        lock.source_snapshots = self.snapshots.clone();
        for package in lock.packages.values_mut() {
            if let Some(remote) = self.packages.get(&package.name) {
                if remote.identity() != package.identity()
                    || remote.content_digest != package.content_digest
                {
                    return Err(failure(
                        TRUST_ARTIFACT_CODE,
                        "锁定包身份或目录摘要在导入时变化",
                    ));
                }
                package.source_artifact = remote.source_artifact.clone();
            }
        }
        lock.validate(Path::new("<memory>"))
            .map_err(|error| failure(error.code(), error))
    }
}

struct DirectRequirement {
    owner: PackageIdentity,
    dependency: RemoteDependencyDeclaration,
}

fn direct_requirements(
    graph: &PackageGraph,
    document: &ConfigDocument,
) -> Result<Vec<DirectRequirement>> {
    let mut result = Vec::new();
    for (identity, node) in &graph.nodes {
        let owned;
        let package_document = if graph.root.as_ref() == Some(identity) {
            document
        } else {
            let config = fs::read_to_string(node.root.join("config.xiao"))
                .map_err(|_| failure(SOURCE_INVALID_CODE, "本地包配置不可读取"))?;
            owned = parse_config_project(&SourceFile::from_text(&config))
                .map_err(|_| failure(SOURCE_INVALID_CODE, "本地包配置无效"))?;
            &owned
        };
        result.extend(
            remote_dependency_declarations(package_document)
                .into_iter()
                .map(|dependency| DirectRequirement {
                    owner: identity.clone(),
                    dependency,
                }),
        );
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare(
    document: &ConfigDocument,
    graph: PackageGraph,
    existing: Option<&LockFile>,
    operation: PackageOperation,
    cache: &CacheStore,
) -> Result<PreparedGraph> {
    let declarations = direct_requirements(&graph, document)?;
    let has_locked_remote = existing.is_some_and(|lock| {
        lock.packages
            .values()
            .any(|package| package.source_artifact.is_some())
    });
    if declarations.is_empty() && !has_locked_remote {
        return Ok(PreparedGraph {
            graph,
            packages: BTreeMap::new(),
            snapshots: BTreeMap::new(),
        });
    }
    let configured = source_declarations(document).map_err(source_failure)?;
    let imports = source_lists::load(&configured, cache).map_err(source_failure)?;
    let sources = expand_source_lists(&configured, &imports).map_err(source_failure)?;
    let must_match = matches!(
        operation,
        PackageOperation::Install | PackageOperation::Lock
    ) || matches!(
        operation,
        PackageOperation::Sync { locked: true, .. } | PackageOperation::Sync { frozen: true, .. }
    );
    if must_match && existing.is_none() && !matches!(operation, PackageOperation::Lock) {
        return Err(failure(
            SYNC_LOCK_REQUIRED_CODE,
            "锁文件缺失；请先执行 xiao sync",
        ));
    }
    if must_match
        && existing.is_some_and(|lock| lock.config_fingerprint != fingerprint_config(document))
    {
        return Err(failure(
            LOCKFILE_CONFIG_MISMATCH_CODE,
            "配置与现有锁文件不一致",
        ));
    }
    if !matches!(operation, PackageOperation::Update)
        && let Some(lock) = existing.filter(|lock| {
            lock.config_fingerprint == fingerprint_config(document) && has_locked_remote
        })
    {
        return restore_locked(graph, &declarations, lock, &sources, cache);
    }
    if must_match && existing.is_some() {
        return Err(failure(
            LOCKFILE_CONFIG_MISMATCH_CODE,
            "锁文件缺少远程解析结果",
        ));
    }
    if declarations.is_empty() {
        return Ok(PreparedGraph {
            graph,
            packages: BTreeMap::new(),
            snapshots: BTreeMap::new(),
        });
    }
    resolve_new(graph, &declarations, document, &sources, cache)
}

fn source_failure(error: crate::source::SourceError) -> PackageSyncError {
    failure(error.code, error.message)
}

fn restore_locked(
    graph: PackageGraph,
    declarations: &[DirectRequirement],
    lock: &LockFile,
    sources: &[ConfiguredSource],
    cache: &CacheStore,
) -> Result<PreparedGraph> {
    let adapter = MultiSourceAdapter::new();
    let mut packages = BTreeMap::new();
    for package in lock
        .packages
        .values()
        .filter(|package| package.source_artifact.is_some())
    {
        let source = sources
            .iter()
            .find(|item| item.descriptor.source.source_id == package.source.source_id)
            .ok_or_else(|| failure(TRUST_SOURCE_CODE, "锁定来源不在当前配置中"))?;
        let artifact = package.source_artifact.as_ref().expect("已过滤远程包");
        let object_path = cache
            .layout()
            .source_object_path(&package.content_digest)
            .map_err(|error| failure(error.code(), error))?;
        if fs::symlink_metadata(&object_path).is_ok() {
            cache
                .verify_source_object(&package.content_digest)
                .map_err(|error| failure(error.code(), error))?;
        } else {
            let pinned = lock
                .source_snapshots
                .get(&package.source.source_id)
                .ok_or_else(|| failure(TRUST_SOURCE_CODE, "锁定包没有来源快照"))?;
            let mut pinned_source = source.descriptor.clone();
            if pinned_source.kind == "git-index"
                && matches!(
                    pinned_source.git_reference.as_ref(),
                    None | Some(GitReference::Head | GitReference::Branch(_))
                )
            {
                pinned_source.git_reference = Some(
                    GitReference::from_config("rev", &pinned.snapshot_id)
                        .map_err(source_failure)?,
                );
                if !matches!(
                    pinned_source.git_reference.as_ref(),
                    Some(GitReference::Commit(_))
                ) {
                    return Err(failure(TRUST_SOURCE_CODE, "锁定 Git 快照不是不可变提交"));
                }
            }
            let snapshot = adapter
                .read_snapshot_pinned(&pinned_source, Some(pinned))
                .map_err(source_failure)?;
            if snapshot.manifest.snapshot_id != pinned.snapshot_id
                || snapshot.digest != pinned.snapshot_digest
            {
                return Err(failure(TRUST_SOURCE_CODE, "包源快照与锁定身份不符"));
            }
            let candidates = adapter
                .read_package(&source.descriptor, &snapshot, &package.name)
                .map_err(source_failure)?;
            if !candidates.iter().any(|candidate| {
                candidate.version == package.version
                    && candidate.variant == "any"
                    && &candidate.source_artifact == artifact
            }) {
                return Err(failure(TRUST_ARTIFACT_CODE, "索引产物声明与锁文件不符"));
            }
            import_artifact(
                &adapter,
                &source.descriptor,
                artifact,
                Some(&package.content_digest),
                cache,
            )
            .map_err(source_failure)?;
        }
        if packages
            .insert(package.name.clone(), package.clone())
            .is_some()
        {
            return Err(failure(TRUST_SOURCE_CODE, "远程包身份不唯一"));
        }
    }
    finish(
        graph,
        declarations,
        packages,
        lock.source_snapshots.clone(),
        cache,
    )
}

fn resolve_new(
    graph: PackageGraph,
    declarations: &[DirectRequirement],
    document: &ConfigDocument,
    sources: &[ConfiguredSource],
    cache: &CacheStore,
) -> Result<PreparedGraph> {
    let requirements = declarations
        .iter()
        .map(|item| {
            SolveRequirement::new(
                item.dependency.name.clone(),
                item.dependency.version.clone(),
                item.dependency.source.clone(),
            )
        })
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(source_failure)?;
    let mut requested = requirements
        .iter()
        .map(|item| item.name.clone())
        .collect::<BTreeSet<_>>();
    let resolver = SourceResolver::new(MultiSourceAdapter::new(), cache.clone());
    let resolution = loop {
        let names = requested.iter().cloned().collect::<Vec<_>>();
        let resolution = resolver
            .resolve(document, sources, None, &names, false)
            .map_err(source_failure)?;
        for record in &resolution.records {
            requested.extend(
                record
                    .package
                    .dependencies
                    .iter()
                    .map(|dep| dep.name.clone()),
            );
        }
        if requested.len() > 256 {
            return Err(failure(SOURCE_INVALID_CODE, "传递依赖查询超过安全上限"));
        }
        if requested.len() == names.len() {
            break resolution;
        }
    };
    let solved = solve_dependencies(
        sources,
        &resolution.snapshots,
        &resolution.records,
        &requirements,
        None,
    )
    .map_err(source_failure)?;
    let snapshots = resolution
        .snapshots
        .iter()
        .filter_map(|snapshot| {
            Some((
                snapshot.source_id.clone(),
                LockedSourceSnapshot {
                    snapshot_id: snapshot.snapshot_id.clone()?,
                    snapshot_digest: snapshot.snapshot_digest.clone()?,
                },
            ))
        })
        .collect();
    let mut packages = BTreeMap::new();
    for (name, resolved) in &solved {
        let source = sources
            .iter()
            .find(|item| item.descriptor.source.source_id == resolved.source_id)
            .ok_or_else(|| failure(TRUST_SOURCE_CODE, "求解后的来源身份丢失"))?;
        let artifact = &resolved.package.source_artifact;
        let object = import_artifact(
            resolver.adapter(),
            &source.descriptor,
            artifact,
            None,
            cache,
        )
        .map_err(source_failure)?;
        let dependencies = resolved
            .package
            .dependencies
            .iter()
            .map(|dep| {
                let target = solved
                    .get(&dep.name)
                    .ok_or_else(|| failure(VERSION_UNSATISFIED_CODE, "传递依赖未完成解析"))?;
                let target_source = sources
                    .iter()
                    .find(|item| item.descriptor.source.source_id == target.source_id)
                    .ok_or_else(|| failure(TRUST_SOURCE_CODE, "传递依赖来源丢失"))?;
                Ok((
                    dep.name.clone(),
                    LockedDependency {
                        kind: "dependencies".to_owned(),
                        version_constraint: Some(dep.version.clone()),
                        source_reference: dep.source.clone(),
                        config_path: String::new(),
                        target: PackageIdentity {
                            name: dep.name.clone(),
                            version: target.package.version.clone(),
                            source: target_source.descriptor.source.clone(),
                        },
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        packages.insert(
            name.clone(),
            LockedPackage {
                name: name.clone(),
                version: resolved.package.version.clone(),
                source: source.descriptor.source.clone(),
                content_digest: object.reference.digest,
                source_artifact: Some(artifact.clone()),
                dependencies,
                precompiled_variants: Vec::new(),
                target_conditions: Vec::new(),
            },
        );
    }
    finish(graph, declarations, packages, snapshots, cache)
}

fn finish(
    mut graph: PackageGraph,
    declarations: &[DirectRequirement],
    packages: BTreeMap<String, LockedPackage>,
    snapshots: BTreeMap<String, LockedSourceSnapshot>,
    cache: &CacheStore,
) -> Result<PreparedGraph> {
    for package in packages.values() {
        if graph
            .nodes
            .keys()
            .any(|identity| identity.name == package.name)
        {
            return Err(failure(VERSION_UNSATISFIED_CODE, "本地与远程包名冲突"));
        }
        let identity = package.identity();
        let root = cache
            .verify_source_object(&package.content_digest)
            .map_err(|error| failure(error.code(), error))?
            .path;
        let dependencies = package
            .dependencies
            .iter()
            .map(|(name, dependency)| {
                (
                    name.clone(),
                    PackageDependency {
                        name: name.clone(),
                        kind: DependencyKind::Runtime,
                        path: PathBuf::from(&dependency.config_path),
                        version: dependency.version_constraint.clone(),
                        source: dependency.source_reference.clone(),
                        span: SourceSpan::new(0, 0).expect("空区间有效"),
                        target: Some(dependency.target.clone()),
                    },
                )
            })
            .collect();
        let edges = package
            .dependencies
            .iter()
            .map(|(name, dependency)| PackageEdge {
                dependency: name.clone(),
                kind: DependencyKind::Runtime,
                target: dependency.target.clone(),
            })
            .collect();
        graph.nodes.insert(
            identity.clone(),
            PackageNode {
                identity: identity.clone(),
                root,
                dependencies,
            },
        );
        graph.edges.insert(identity, edges);
    }
    for item in declarations {
        let target = packages
            .get(&item.dependency.name)
            .ok_or_else(|| failure(LOCKFILE_CONFIG_MISMATCH_CODE, "锁文件没有配置声明的远程包"))?
            .identity();
        let owner = graph
            .nodes
            .get_mut(&item.owner)
            .ok_or_else(|| failure(SOURCE_INVALID_CODE, "声明远程依赖的本地包丢失"))?;
        if owner.dependencies.contains_key(&target.name) {
            return Err(failure(VERSION_UNSATISFIED_CODE, "本地与远程依赖重名"));
        }
        owner.dependencies.insert(
            target.name.clone(),
            PackageDependency {
                name: target.name.clone(),
                kind: item.dependency.kind,
                path: PathBuf::new(),
                version: Some(item.dependency.version.clone()),
                source: item.dependency.source.clone(),
                span: item.dependency.span,
                target: Some(target.clone()),
            },
        );
        graph
            .edges
            .entry(item.owner.clone())
            .or_default()
            .push(PackageEdge {
                dependency: target.name.clone(),
                kind: item.dependency.kind,
                target,
            });
    }
    let root = graph
        .root
        .clone()
        .ok_or_else(|| failure(SOURCE_INVALID_CODE, "包图缺少根包"))?;
    let mut visited = BTreeSet::new();
    let mut active = BTreeSet::new();
    let mut order = Vec::new();
    visit(&graph, &root, &mut visited, &mut active, &mut order)?;
    if order.len() != graph.nodes.len() {
        return Err(failure(
            LOCKFILE_CONFIG_MISMATCH_CODE,
            "锁定依赖图含不可达包",
        ));
    }
    graph.resolution_order = order;
    Ok(PreparedGraph {
        graph,
        packages,
        snapshots,
    })
}

fn visit(
    graph: &PackageGraph,
    identity: &PackageIdentity,
    visited: &mut BTreeSet<PackageIdentity>,
    active: &mut BTreeSet<PackageIdentity>,
    order: &mut Vec<PackageIdentity>,
) -> Result<()> {
    if visited.contains(identity) {
        return Ok(());
    }
    if !active.insert(identity.clone()) {
        return Err(failure(VERSION_UNSATISFIED_CODE, "传递依赖存在环"));
    }
    let edges = graph
        .edges
        .get(identity)
        .ok_or_else(|| failure(LOCKFILE_CONFIG_MISMATCH_CODE, "锁定依赖图缺少节点"))?;
    for edge in edges {
        if !graph.nodes.contains_key(&edge.target) {
            return Err(failure(LOCKFILE_CONFIG_MISMATCH_CODE, "锁定依赖目标不存在"));
        }
        visit(graph, &edge.target, visited, active, order)?;
    }
    active.remove(identity);
    visited.insert(identity.clone());
    order.push(identity.clone());
    Ok(())
}
