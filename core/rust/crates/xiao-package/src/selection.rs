//! 只限定源，不在此阶段执行源内版本求解。

use std::collections::BTreeSet;

use crate::diagnostics::{
    SOURCE_AMBIGUOUS_CODE, SOURCE_UNAVAILABLE_CODE, SOURCE_UNKNOWN_REFERENCE_CODE,
};
use crate::federation::{FederatedRecord, SnapshotStatus, SourceSnapshot};
use crate::source::{ConfiguredSource, SourceError};

/// 在有序源集合中挑选第一个含满足条件候选的源，并保留其全部匹配版本。
///
/// `matches` 是由求解器提供的版本、目标与锁定条件；未指定源时，不得跨源比版本。
pub fn select_source<'a>(
    sources: &[ConfiguredSource],
    snapshots: &[SourceSnapshot],
    records: &'a [FederatedRecord],
    name: &str,
    binding: Option<&str>,
    matches: impl Fn(&FederatedRecord) -> bool,
) -> Result<Vec<&'a FederatedRecord>, SourceError> {
    let mut ordered = sources.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|source| source.config_order);
    let mut orders = BTreeSet::new();
    for entry in &ordered {
        if !orders.insert(entry.config_order) {
            return Err(SourceError::new(
                SOURCE_AMBIGUOUS_CODE,
                format!("重复源优先级 {}", entry.config_order),
            ));
        }
    }
    let bound = if let Some(binding) = binding {
        let matching = ordered
            .iter()
            .copied()
            .filter(|entry| {
                entry.descriptor.source.alias.as_deref() == Some(binding)
                    || entry.descriptor.source.source_id == binding
            })
            .collect::<Vec<_>>();
        if matching.is_empty() {
            return Err(SourceError::new(
                SOURCE_UNKNOWN_REFERENCE_CODE,
                format!("未知源引用 {binding:?}"),
            ));
        }
        if matching.len() != 1 {
            return Err(SourceError::new(
                SOURCE_AMBIGUOUS_CODE,
                format!("源引用 {binding:?} 匹配多个端点"),
            ));
        }
        Some(matching[0].config_order)
    } else {
        None
    };
    for entry in ordered {
        if bound.is_some_and(|order| order != entry.config_order) {
            continue;
        }
        let matching_snapshots = snapshots
            .iter()
            .filter(|snapshot| {
                snapshot.config_order == entry.config_order
                    && snapshot.source_id == entry.descriptor.source.source_id
            })
            .collect::<Vec<_>>();
        if matching_snapshots.len() != 1 {
            return Err(SourceError::new(
                SOURCE_AMBIGUOUS_CODE,
                format!("源 {} 缺少唯一快照", entry.descriptor.source.source_id),
            ));
        }
        if matching_snapshots[0].status == SnapshotStatus::Unavailable {
            return Err(SourceError::new(
                SOURCE_UNAVAILABLE_CODE,
                format!("源 {} 无可验证快照", entry.descriptor.source.source_id),
            ));
        }
        let candidates = records
            .iter()
            .filter(|record| {
                record.key.config_order == entry.config_order
                    && record.key.source_id == entry.descriptor.source.source_id
                    && record.key.name == name
                    && matches(record)
            })
            .collect::<Vec<_>>();
        if !candidates.is_empty() {
            return Ok(candidates);
        }
    }
    Ok(Vec::new())
}
