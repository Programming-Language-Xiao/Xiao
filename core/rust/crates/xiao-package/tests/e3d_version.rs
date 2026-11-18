//! 11A-E3D：冻结的 SemVer 优先级、通配展开和预发布准入向量。

use std::cmp::Ordering;

use xiao_package::{VERSION_INVALID_CODE, Version, VersionRequirement};

#[test]
fn all_frozen_requirement_forms_and_boundaries() {
    for (requirement, candidate, expected) in [
        ("1.2.3", "1.2.3", true),
        ("1.2.3", "1.2.4", false),
        ("^1.2.3", "1.9.9", true),
        ("^1.2.3", "2.0.0", false),
        ("^0.2.3", "0.2.99", true),
        ("^0.2.3", "0.3.0", false),
        ("^0.0.3", "0.0.4", false),
        ("~1.2.3", "1.2.99", true),
        ("~1.2.3", "1.3.0", false),
        (">=1.2, <2", "1.9.3", true),
        (">=1.2, <2", "2.0.0", false),
        ("1", "1.0.0", true),
        ("1", "1.99.0", true),
        ("1", "2.0.0", false),
        ("1.2", "1.2.99", true),
        ("1.2", "1.1.99", false),
        ("1.2", "1.3.0", false),
        ("=1.2", "1.2.45", true),
        ("=1.2", "1.3.0", false),
        ("1.*", "1.6.0", true),
        ("1.2.*", "1.2.0", true),
        ("1.2.*", "1.3.0", false),
        (">1.2", "1.2.99", false),
        (">1.2", "1.3.0", true),
        (">=1.2", "1.2.0", true),
        ("<2", "1.99.99", true),
        ("<=1.2", "1.2.99", true),
        ("<=1.2", "1.3.0", false),
        ("<2", "2.0.0", false),
        ("1.2.3", "1.2.3+build.7", true),
        ("1.2.3+build.1", "1.2.3+build.2", true),
        ("1.2", "1.2.4-alpha.1", false),
        ("*", "9.1.0-rc.1", false),
        (">=1.2.3-alpha.1, <2.0.0", "1.2.3-alpha.2", true),
        (">=1.2.3-alpha.1, <2.0.0", "1.2.4-alpha.2", false),
        ("^1.2.3-alpha.1", "1.2.3-alpha.2", true),
        ("^1.2.3-alpha.1", "1.2.4-alpha.1", false),
    ] {
        let requirement = VersionRequirement::parse(requirement).unwrap();
        let actual = requirement.matches(&Version::parse(candidate).unwrap());
        assert_eq!(actual, expected, "{requirement:?} / {candidate}");
    }
}

#[test]
fn precedence_ignores_build_and_compares_prerelease_identifiers() {
    for (lower, higher) in [
        ("1.0.0-alpha", "1.0.0-alpha.1"),
        ("1.0.0-alpha.1", "1.0.0-alpha.beta"),
        ("1.0.0-beta", "1.0.0-beta.2"),
        ("1.0.0-beta.2", "1.0.0-beta.11"),
        ("1.0.0-rc.1", "1.0.0"),
        ("2.2.99", "2.3.0"),
        (
            "999999999999999999999999999999.1.1",
            "1000000000000000000000000000000.0.0",
        ),
    ] {
        assert_eq!(
            Version::parse(lower)
                .unwrap()
                .precedence(&Version::parse(higher).unwrap()),
            Ordering::Less,
            "{lower} / {higher}"
        );
    }
    assert_eq!(
        Version::parse("1.2.3+first")
            .unwrap()
            .precedence(&Version::parse("1.2.3+second").unwrap()),
        Ordering::Equal
    );
    assert_ne!(
        Version::parse("1.2.3+first").unwrap(),
        Version::parse("1.2.3+second").unwrap()
    );
}

#[test]
fn malformed_versions_and_constraints_are_rejected() {
    for text in [
        "1",
        "1.2",
        "01.2.3",
        "1.02.3",
        "1.2.03",
        "1.2.3-alpha..1",
        "1.2.3-alpha.01",
        "1.2.3+",
        "1.2.3+a..b",
        "1.2.3-!",
        "1.2.3+构建",
    ] {
        assert_eq!(
            Version::parse(text).unwrap_err().code,
            VERSION_INVALID_CODE,
            "{text}"
        );
    }
    for text in [
        "",
        "^",
        "^1",
        "~1.2",
        "1..2",
        "1.*.3",
        "01",
        "1.02",
        ">=1.2,,<2",
        ">=1.2 <2",
        "1 || 2",
        "1.2.3 - 2.0.0",
        ">*",
        "1.2.3-alpha.01",
    ] {
        assert_eq!(
            VersionRequirement::parse(text).unwrap_err().code,
            VERSION_INVALID_CODE,
            "{text}"
        );
    }
}
