//! 11A-E3D：多源 SemVer 选择与回溯的独立规格测试。

use xiao_package::{
    ArtifactReference, ConfiguredSource, IndexDependency, IndexPackage, SOURCE_AMBIGUOUS_CODE,
    SnapshotStatus, SolveRequirement, SourceDescriptor, SourceSnapshot, VERSION_UNSATISFIED_CODE,
    federate, solve_dependencies,
};

fn source(alias: &str, order: usize) -> ConfiguredSource {
    ConfiguredSource {
        descriptor: SourceDescriptor::new(
            "static",
            &format!("https://{alias}.example"),
            Some(alias),
            None,
            1,
        )
        .unwrap(),
        config_order: order,
        imported_from: None,
    }
}

fn package(name: &str, version: &str, dependencies: &[(&str, &str, Option<&str>)]) -> IndexPackage {
    IndexPackage {
        name: name.to_owned(),
        version: version.to_owned(),
        variant: "any".to_owned(),
        withdrawn: false,
        dependencies: dependencies
            .iter()
            .map(|(name, version, source)| IndexDependency {
                name: (*name).to_owned(),
                version: (*version).to_owned(),
                source: source.map(str::to_owned),
            })
            .collect(),
        features: Vec::new(),
        target: None,
        abi: None,
        xiao_range: None,
        runtime_range: None,
        source_artifact: ArtifactReference {
            location: format!("objects/{name}/{version}"),
            length: 1,
            digest: "a".repeat(64),
        },
        binary_artifacts: Vec::new(),
    }
}

fn snapshot(source: &ConfiguredSource, candidates: Vec<IndexPackage>) -> SourceSnapshot {
    let queried_packages = candidates
        .iter()
        .map(|package| package.name.clone())
        .collect();
    SourceSnapshot {
        config_order: source.config_order,
        source_id: source.descriptor.source.source_id.clone(),
        snapshot_id: Some("snapshot".to_owned()),
        snapshot_digest: Some("b".repeat(64)),
        status: SnapshotStatus::Fresh,
        candidates,
        queried_packages,
    }
}

fn requirement(name: &str, version: &str, source: Option<&str>) -> SolveRequirement {
    SolveRequirement {
        name: name.to_owned(),
        version: version.to_owned(),
        source: source.map(str::to_owned),
    }
}

#[test]
fn first_source_wins_before_any_cross_source_version_comparison() {
    let first = source("first", 0);
    let second = source("second", 1);
    let sources = [first.clone(), second.clone()];
    let snapshots = [
        snapshot(&first, vec![package("lib", "1.2.0", &[])]),
        snapshot(&second, vec![package("lib", "9.0.0", &[])]),
    ];
    let records = federate(&snapshots).unwrap();
    let first_result = solve_dependencies(
        &sources,
        &snapshots,
        &records,
        &[requirement("lib", "*", None)],
        None,
    )
    .unwrap();
    assert_eq!(first_result["lib"].package.version, "1.2.0");
    assert_eq!(
        first_result["lib"].source_id,
        first.descriptor.source.source_id
    );
    let fallback = solve_dependencies(
        &sources,
        &snapshots,
        &records,
        &[requirement("lib", ">=2", None)],
        None,
    )
    .unwrap();
    assert_eq!(fallback["lib"].package.version, "9.0.0");
    let second_result = solve_dependencies(
        &sources,
        &snapshots,
        &records,
        &[requirement("lib", "1", Some("second"))],
        None,
    );
    assert_eq!(second_result.unwrap_err().code, VERSION_UNSATISFIED_CODE);
    let second_result = solve_dependencies(
        &sources,
        &snapshots,
        &records,
        &[requirement("lib", "9", Some("second"))],
        None,
    )
    .unwrap();
    assert_eq!(second_result["lib"].package.version, "9.0.0");
}

#[test]
fn backtracks_when_new_transitive_constraint_conflicts_with_prior_choice() {
    let first = source("first", 0);
    let sources = [first.clone()];
    let snapshots = [snapshot(
        &first,
        vec![
            package("app", "2.0.0", &[("lib", "1", None)]),
            package("app", "1.0.0", &[("lib", "2", None)]),
            package("lib", "1.5.0", &[]),
            package("lib", "2.3.0", &[]),
        ],
    )];
    let records = federate(&snapshots).unwrap();
    let input = [requirement("app", "*", None), requirement("lib", "2", None)];
    let solve = || solve_dependencies(&sources, &snapshots, &records, &input, None).unwrap();
    assert_eq!(solve(), solve());
    assert_eq!(solve()["app"].package.version, "1.0.0");
    assert_eq!(solve()["lib"].package.version, "2.3.0");
}

#[test]
fn missing_solution_and_equal_precedence_are_distinct_errors() {
    let first = source("first", 0);
    let sources = [first.clone()];
    let snapshots = [snapshot(
        &first,
        vec![
            package("lib", "1.0.0+first", &[]),
            package("lib", "1.0.0+second", &[]),
        ],
    )];
    let records = federate(&snapshots).unwrap();
    assert_eq!(
        solve_dependencies(
            &sources,
            &snapshots,
            &records,
            &[requirement("lib", "1", None)],
            None,
        )
        .unwrap_err()
        .code,
        SOURCE_AMBIGUOUS_CODE
    );
    assert_eq!(
        solve_dependencies(
            &sources,
            &snapshots,
            &records,
            &[requirement("lib", "2", None)],
            None,
        )
        .unwrap_err()
        .code,
        VERSION_UNSATISFIED_CODE
    );
}

#[test]
fn separate_constraints_share_prerelease_admission() {
    let first = source("first", 0);
    let sources = [first.clone()];
    let snapshots = [snapshot(&first, vec![package("lib", "1.2.3-alpha.2", &[])])];
    let records = federate(&snapshots).unwrap();
    let result = solve_dependencies(
        &sources,
        &snapshots,
        &records,
        &[
            requirement("lib", ">=1.2.3-alpha.1", None),
            requirement("lib", "<2", None),
        ],
        None,
    )
    .unwrap();
    assert_eq!(result["lib"].package.version, "1.2.3-alpha.2");
}

#[test]
fn conflicting_transitive_sources_can_backtrack() {
    let first = source("first", 0);
    let second = source("second", 1);
    let sources = [first.clone(), second.clone()];
    let snapshots = [
        snapshot(
            &first,
            vec![
                package("app", "2.0.0", &[("lib", "1", Some("second"))]),
                package("app", "1.0.0", &[("lib", "1", Some("first"))]),
                package("lib", "1.0.0", &[]),
            ],
        ),
        snapshot(&second, vec![package("lib", "1.0.0", &[])]),
    ];
    let records = federate(&snapshots).unwrap();
    let result = solve_dependencies(
        &sources,
        &snapshots,
        &records,
        &[
            requirement("app", "*", None),
            requirement("lib", "1", Some("first")),
        ],
        None,
    )
    .unwrap();
    assert_eq!(result["app"].package.version, "1.0.0");
    assert_eq!(result["lib"].source_id, first.descriptor.source.source_id);
}

#[test]
fn cyclic_highest_candidate_backtracks_to_acyclic_release() {
    let first = source("first", 0);
    let sources = [first.clone()];
    let snapshots = [snapshot(
        &first,
        vec![
            package("app", "2.0.0", &[("helper", "1", None)]),
            package("app", "1.0.0", &[]),
            package("helper", "1.0.0", &[("app", "*", None)]),
        ],
    )];
    let records = federate(&snapshots).unwrap();
    let result = solve_dependencies(
        &sources,
        &snapshots,
        &records,
        &[requirement("app", "*", None)],
        None,
    )
    .unwrap();
    assert_eq!(result["app"].package.version, "1.0.0");
    assert!(!result.contains_key("helper"));
}
