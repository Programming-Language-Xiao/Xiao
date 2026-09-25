//! E3B 本地源驱动的确定性并行读取与离线快照快速路径。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::thread;

use xiao_config::ConfigDocument;

use crate::adapters::PackageSourceAdapter;
use crate::cache::{CacheError, CacheStore};
use crate::diagnostics::{SOURCE_CACHE_IO_CODE, SOURCE_INVALID_CODE};
use crate::environment::fingerprint_config;
use crate::federation::{
    FederatedRecord, SnapshotStatus, SourceSnapshot, federate, source_list_fingerprint,
    valid_package_name,
};
use crate::federation_cache::{FederationCache, FederationIndex, MetadataCache};
use crate::lockfile::LockFile;
use crate::selection::select_source;
use crate::snapshot_store::SnapshotStore;
use crate::source::{ConfiguredSource, SourceError};

/// 源列表由上游控制；同时最多打开八个目录或传输端点，而不是每源开线程。
pub const MAX_PARALLEL_SOURCES: usize = 8;

enum ReadOutcome {
    Fresh(crate::adapters::IndexSnapshot, SourceSnapshot),
    Cached(SourceSnapshot, u64),
    Failed(Option<SourceError>),
}

/// 一个来源的本次读取结果及可展示的新鲜度，不参与选包。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceStatusReport {
    /// 源最终配置优先级。
    pub config_order: usize,
    /// 规范源身份。
    pub source_id: String,
    /// 本次读取、沿用缓存或不可用。
    pub status: SnapshotStatus,
    /// 已验证快照摘要；不可用时为空。
    pub snapshot_digest: Option<String>,
    /// 成功读到快照时的毫秒时间戳，仅供展示。
    pub observed_at_ms: Option<u64>,
    /// 来源失败的原始诊断；基础设施错误直接返回而非降级。
    pub problem: Option<SourceError>,
}

/// 供后续单一求解器消费的完整有序源视图。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceResolution {
    /// 是否在未经本次源读取的情况下复用了完整锁文件和缓存。
    pub fast_path: bool,
    /// 与最终配置顺序一致的快照状态。
    pub snapshots: Vec<SourceSnapshot>,
    /// 已保留来源维度的完整候选版本。
    pub records: Vec<FederatedRecord>,
    /// 每个源的摘要、新鲜度和失败原因。
    pub reports: Vec<SourceStatusReport>,
}

/// 只使用同步适配器，传输方式不会改变包选择语义。
pub struct SourceResolver<A> {
    adapter: A,
    cache: CacheStore,
    snapshots: SnapshotStore,
    federation: FederationCache,
    metadata: MetadataCache,
}

impl<A: PackageSourceAdapter + Sync> SourceResolver<A> {
    /// 使用同一缓存布局初始化四类独立对象。
    #[must_use]
    pub fn new(adapter: A, cache: CacheStore) -> Self {
        let layout = cache.layout().clone();
        Self {
            adapter,
            cache,
            snapshots: SnapshotStore::new(layout.clone()),
            federation: FederationCache::new(layout.clone()),
            metadata: MetadataCache::new(layout),
        }
    }

    /// 完整命中时不碰源；否则分批读取并仅在全部返回后合并。
    pub fn resolve(
        &self,
        document: &ConfigDocument,
        sources: &[ConfiguredSource],
        lockfile: Option<&LockFile>,
        names: &[String],
        offline: bool,
    ) -> Result<SourceResolution, SourceError> {
        let mut ordered = sources.to_vec();
        ordered.sort_by_key(|source| source.config_order);
        if ordered
            .iter()
            .enumerate()
            .any(|(order, source)| source.config_order != order)
        {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "源优先级必须连续且唯一",
            ));
        }
        let fingerprint = fingerprint_config(document);
        let usable_lockfile = lockfile.filter(|lockfile| {
            lockfile.config_fingerprint == fingerprint
                && lockfile.validate(std::path::Path::new("<memory>")).is_ok()
        });
        let mut requested = names.iter().cloned().collect::<BTreeSet<_>>();
        if let Some(lockfile) = usable_lockfile {
            requested.extend(
                lockfile
                    .packages
                    .values()
                    .filter(|package| {
                        ordered.iter().any(|source| {
                            source.descriptor.source.source_id == package.source.source_id
                        })
                    })
                    .map(|package| package.name.clone()),
            );
        }
        if requested.iter().any(|name| !valid_package_name(name)) {
            return Err(SourceError::new(SOURCE_INVALID_CODE, "查询包名不合法"));
        }
        let source_fingerprint = source_list_fingerprint(&ordered);
        if let Some(lockfile) = usable_lockfile {
            if let Some(cached) =
                self.try_fast_path(&fingerprint, &source_fingerprint, &ordered, lockfile, names)?
            {
                return Ok(cached);
            }
        }
        let previous = self.federation.read(&fingerprint, &source_fingerprint)?;
        let requested = requested.into_iter().collect::<Vec<_>>();
        let can_reuse = usable_lockfile.is_some();
        let mut completed = Vec::with_capacity(ordered.len());
        for batch in ordered.chunks(MAX_PARALLEL_SOURCES) {
            let results = thread::scope(|scope| {
                let handles = batch
                    .iter()
                    .map(|source| {
                        scope.spawn(|| {
                            if can_reuse {
                                if let Some(existing) = previous.as_ref().and_then(|index| {
                                    index.snapshots.get(source.config_order).filter(|snapshot| {
                                        snapshot.source_id == source.descriptor.source.source_id
                                            && snapshot.status != SnapshotStatus::Unavailable
                                            && requested.iter().all(|name| {
                                                snapshot.queried_packages.contains(name)
                                            })
                                    })
                                }) {
                                    if let Some(current) = self
                                        .snapshots
                                        .current(&source.descriptor.source.source_id)?
                                    {
                                        if existing.snapshot_id.as_deref()
                                            == Some(&current.index.manifest.snapshot_id)
                                            && existing.snapshot_digest.as_deref()
                                                == Some(&current.index.digest)
                                        {
                                            let mut reused = existing.clone();
                                            reused.status = SnapshotStatus::Cached;
                                            return Ok(ReadOutcome::Cached(
                                                reused,
                                                current.observed_at_ms,
                                            ));
                                        }
                                    }
                                }
                            }
                            if offline {
                                return Ok(ReadOutcome::Failed(None));
                            }
                            Ok(
                                match self.adapter.read_snapshot(&source.descriptor).and_then(
                                    |index| {
                                        let mut snapshot = SourceSnapshot::from_index(
                                            source.config_order,
                                            SnapshotStatus::Fresh,
                                            &index,
                                        )?;
                                        for name in &requested {
                                            snapshot.query_package(
                                                &self.adapter,
                                                &source.descriptor,
                                                &index,
                                                name,
                                            )?;
                                        }
                                        Ok((index, snapshot))
                                    },
                                ) {
                                    Ok((index, snapshot)) => ReadOutcome::Fresh(index, snapshot),
                                    Err(error) => ReadOutcome::Failed(Some(error)),
                                },
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                handles
                    .into_iter()
                    .map(|handle| handle.join())
                    .collect::<Vec<_>>()
            });
            for result in results {
                completed.push(result.map_err(|_| {
                    SourceError::new(SOURCE_CACHE_IO_CODE, "包源读取工作线程意外退出")
                })??);
            }
        }
        let mut snapshots = Vec::with_capacity(ordered.len());
        let mut reports = Vec::with_capacity(ordered.len());
        for (source, result) in ordered.iter().zip(completed) {
            let (snapshot, observed_at_ms, problem) = match result {
                ReadOutcome::Fresh(index, snapshot) => {
                    for package in &snapshot.candidates {
                        self.metadata.store(package)?;
                    }
                    let stored = self.snapshots.save(&index)?;
                    (snapshot, Some(stored.observed_at_ms), None)
                }
                ReadOutcome::Cached(snapshot, observed_at_ms) => {
                    (snapshot, Some(observed_at_ms), None)
                }
                ReadOutcome::Failed(problem) => {
                    let previous_snapshot = previous.as_ref().and_then(|index| {
                        index.snapshots.get(source.config_order).filter(|snapshot| {
                            snapshot.source_id == source.descriptor.source.source_id
                        })
                    });
                    let (cached, observed_at_ms) =
                        self.fallback(source, &requested, previous_snapshot)?;
                    (cached, observed_at_ms, problem)
                }
            };
            reports.push(report(source, &snapshot, observed_at_ms, problem));
            snapshots.push(snapshot);
        }
        let records = federate(&snapshots)?;
        for name in &requested {
            let binding = usable_lockfile.and_then(|lockfile| explicit_source(lockfile, name));
            select_source(
                &ordered,
                &snapshots,
                &records,
                name,
                binding.as_deref(),
                |_| true,
            )?;
        }
        self.federation.store(&FederationIndex {
            config_fingerprint: fingerprint,
            sources: source_fingerprint,
            snapshots: snapshots.clone(),
            records: records.clone(),
        })?;
        Ok(SourceResolution {
            fast_path: false,
            snapshots,
            records,
            reports,
        })
    }

    fn fallback(
        &self,
        source: &ConfiguredSource,
        names: &[String],
        previous: Option<&SourceSnapshot>,
    ) -> Result<(SourceSnapshot, Option<u64>), SourceError> {
        let source_id = &source.descriptor.source.source_id;
        if let Some(current) = self.snapshots.current(source_id)? {
            let mut snapshot = SourceSnapshot::from_index(
                source.config_order,
                SnapshotStatus::Cached,
                &current.index,
            )?;
            if let Some(previous) = previous.filter(|previous| {
                previous.snapshot_id == snapshot.snapshot_id
                    && previous.snapshot_digest == snapshot.snapshot_digest
                    && previous.status != SnapshotStatus::Unavailable
            }) {
                snapshot.candidates = previous.candidates.clone();
                snapshot.queried_packages = previous.queried_packages.clone();
            }
            for name in names {
                if !current.index.manifest.shards.contains_key(name) {
                    snapshot.queried_packages.insert(name.clone());
                }
            }
            return Ok((snapshot, Some(current.observed_at_ms)));
        }
        Ok((
            SourceSnapshot {
                config_order: source.config_order,
                source_id: source_id.clone(),
                snapshot_id: None,
                snapshot_digest: None,
                status: SnapshotStatus::Unavailable,
                candidates: Vec::new(),
                queried_packages: BTreeSet::new(),
            },
            None,
        ))
    }

    fn try_fast_path(
        &self,
        config_fingerprint: &str,
        source_fingerprint: &crate::federation::SourceListFingerprint,
        sources: &[ConfiguredSource],
        lockfile: &LockFile,
        names: &[String],
    ) -> Result<Option<SourceResolution>, SourceError> {
        if lockfile.validate(std::path::Path::new("<memory>")).is_err()
            || lockfile.config_fingerprint != config_fingerprint
        {
            return Ok(None);
        }
        if names.iter().any(|name| {
            !lockfile
                .packages
                .values()
                .any(|package| package.name == *name)
        }) {
            return Ok(None);
        }
        for package in lockfile.packages.values() {
            match self.cache.verify_source_object(&package.content_digest) {
                Ok(_) => (),
                Err(CacheError::ObjectCorrupt {
                    quarantine: Some(_),
                    ..
                }) => return Ok(None),
                Err(CacheError::Write {
                    ref path,
                    operation: "读取缓存对象",
                    ..
                }) if fs::symlink_metadata(path)
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
                {
                    return Ok(None);
                }
                Err(error) => {
                    return Err(SourceError::new(SOURCE_CACHE_IO_CODE, error.to_string()));
                }
            }
        }
        let Some(mut cached) = self
            .federation
            .read(config_fingerprint, source_fingerprint)?
        else {
            return Ok(None);
        };
        let required = required_sources(lockfile, sources);
        for (order, names) in required {
            let Some(current) = self
                .snapshots
                .current(&sources[order].descriptor.source.source_id)?
            else {
                return Ok(None);
            };
            let snapshot = &cached.snapshots[order];
            if snapshot.snapshot_id.as_deref() != Some(&current.index.manifest.snapshot_id)
                || snapshot.snapshot_digest.as_deref() != Some(&current.index.digest)
                || names
                    .iter()
                    .any(|name| !snapshot.queried_packages.contains(name))
                || snapshot.status == SnapshotStatus::Unavailable
            {
                return Ok(None);
            }
        }
        for package in lockfile.packages.values() {
            if !sources
                .iter()
                .any(|source| source.descriptor.source.source_id == package.source.source_id)
            {
                continue;
            }
            let binding = explicit_source(lockfile, &package.name);
            let Ok(selected) = select_source(
                sources,
                &cached.snapshots,
                &cached.records,
                &package.name,
                binding.as_deref(),
                |_| true,
            ) else {
                return Ok(None);
            };
            if selected
                .first()
                .is_none_or(|record| record.key.source_id != package.source.source_id)
                || !selected
                    .iter()
                    .any(|record| record.key.version == package.version)
            {
                return Ok(None);
            }
        }
        let mut reports = Vec::new();
        for (source, snapshot) in sources.iter().zip(&mut cached.snapshots) {
            if snapshot.status != SnapshotStatus::Unavailable {
                snapshot.status = SnapshotStatus::Cached;
            }
            let observed_at_ms = self
                .snapshots
                .current(&source.descriptor.source.source_id)?
                .map(|current| current.observed_at_ms);
            reports.push(report(source, snapshot, observed_at_ms, None));
        }
        cached.records = federate(&cached.snapshots)?;
        Ok(Some(SourceResolution {
            fast_path: true,
            snapshots: cached.snapshots,
            records: cached.records,
            reports,
        }))
    }
}

fn required_sources(
    lockfile: &LockFile,
    sources: &[ConfiguredSource],
) -> BTreeMap<usize, BTreeSet<String>> {
    let mut required = BTreeMap::<usize, BTreeSet<String>>::new();
    for package in lockfile.packages.values() {
        let Some(selected) = sources
            .iter()
            .find(|source| source.descriptor.source.source_id == package.source.source_id)
        else {
            continue;
        };
        let references = lockfile.packages.values().flat_map(|owner| {
            owner
                .dependencies
                .values()
                .filter(move |dependency| dependency.target == package.identity())
        });
        let needs_priority = references.count() == 0
            || lockfile.packages.values().any(|owner| {
                owner.dependencies.values().any(|dependency| {
                    dependency.target == package.identity() && dependency.source_reference.is_none()
                })
            });
        for source in sources {
            if source.config_order == selected.config_order
                || (needs_priority && source.config_order < selected.config_order)
            {
                required
                    .entry(source.config_order)
                    .or_default()
                    .insert(package.name.clone());
            }
        }
    }
    required
}

fn explicit_source(lockfile: &LockFile, name: &str) -> Option<String> {
    let mut bound = None;
    for reference in lockfile
        .packages
        .values()
        .flat_map(|package| package.dependencies.values())
        .filter(|dependency| dependency.target.name == name)
    {
        reference.source_reference.as_ref()?;
        if let Some(existing) = &bound {
            if existing != &reference.target.source.source_id {
                return None;
            }
        }
        bound = Some(reference.target.source.source_id.clone());
    }
    bound
}

fn report(
    source: &ConfiguredSource,
    snapshot: &SourceSnapshot,
    observed_at_ms: Option<u64>,
    problem: Option<SourceError>,
) -> SourceStatusReport {
    SourceStatusReport {
        config_order: source.config_order,
        source_id: source.descriptor.source.source_id.clone(),
        status: snapshot.status,
        snapshot_digest: snapshot.snapshot_digest.clone(),
        observed_at_ms,
        problem,
    }
}
