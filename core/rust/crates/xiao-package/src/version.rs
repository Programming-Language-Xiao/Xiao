//! 11A-E3D 冻结的版本与约束解释；不依赖第三方约束语法。

use std::cmp::Ordering;

use crate::diagnostics::VERSION_INVALID_CODE;
use crate::source::SourceError;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Core {
    major: String,
    minor: String,
    patch: String,
}

impl Core {
    fn precedence(&self, other: &Self) -> Ordering {
        compare_decimal(&self.major, &other.major)
            .then_with(|| compare_decimal(&self.minor, &other.minor))
            .then_with(|| compare_decimal(&self.patch, &other.patch))
    }

    fn version(&self) -> Version {
        Version {
            core: self.clone(),
            prerelease: Vec::new(),
            build: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Identifier {
    Number(String),
    Text(String),
}

impl Identifier {
    fn precedence(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => compare_decimal(left, right),
            (Self::Number(_), Self::Text(_)) => Ordering::Less,
            (Self::Text(_), Self::Number(_)) => Ordering::Greater,
            (Self::Text(left), Self::Text(right)) => left.cmp(right),
        }
    }
}

/// 严格的 SemVer 2.0.0 版本；构建元数据保留身份但不参与优先级。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Version {
    core: Core,
    prerelease: Vec<Identifier>,
    build: Vec<Identifier>,
}

impl Version {
    /// 解析完整三段版本，拒绝前导零和非法预发布/构建标识符。
    pub fn parse(text: &str) -> Result<Self, SourceError> {
        let (without_build, build) = text
            .split_once('+')
            .map_or((text, None), |(head, tail)| (head, Some(tail)));
        let build = build
            .map(|identifiers| validate_identifiers(identifiers, false))
            .transpose()?
            .unwrap_or_default();
        let (numbers, prerelease) = without_build
            .split_once('-')
            .map_or((without_build, None), |(head, tail)| (head, Some(tail)));
        let mut parts = numbers.split('.');
        let core = Core {
            major: parse_number(parts.next().unwrap_or_default())?,
            minor: parse_number(parts.next().unwrap_or_default())?,
            patch: parse_number(parts.next().unwrap_or_default())?,
        };
        if parts.next().is_some() {
            return Err(invalid("版本号必须恰好包含三段"));
        }
        let prerelease = if let Some(prerelease) = prerelease {
            validate_identifiers(prerelease, true)?
        } else {
            Vec::new()
        };
        Ok(Self {
            core,
            prerelease,
            build,
        })
    }

    /// 比较 SemVer 优先级；构建元数据不会改变比较结果。
    #[must_use]
    pub fn precedence(&self, other: &Self) -> Ordering {
        self.core.precedence(&other.core).then_with(|| {
            if self.prerelease.is_empty() {
                return if other.prerelease.is_empty() {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                };
            }
            if other.prerelease.is_empty() {
                return Ordering::Less;
            }
            for (left, right) in self.prerelease.iter().zip(&other.prerelease) {
                let order = left.precedence(right);
                if order != Ordering::Equal {
                    return order;
                }
            }
            self.prerelease.len().cmp(&other.prerelease.len())
        })
    }

    pub(crate) fn is_prerelease(&self) -> bool {
        !self.prerelease.is_empty()
    }
}

#[derive(Clone, Debug)]
struct PartialVersion {
    floor: Version,
    ceiling: Option<Version>,
}

impl PartialVersion {
    fn parse(text: &str) -> Result<Self, SourceError> {
        let parts = text.split('.').collect::<Vec<_>>();
        if parts.is_empty() || parts.len() > 3 || parts[0] == "*" && parts.len() != 1 {
            return Err(invalid("部分版本格式不合法"));
        }
        if parts == ["*"] {
            return Ok(Self {
                floor: Version::parse("0.0.0")?,
                ceiling: None,
            });
        }
        let major = parse_number(parts[0])?;
        let minor = match parts.get(1) {
            Some(&"*") | None => None,
            Some(value) => Some(parse_number(value)?),
        };
        let patch = match parts.get(2) {
            Some(&"*") | None => None,
            Some(value) => Some(parse_number(value)?),
        };
        if (minor.is_none() && parts.len() == 3) || (patch.is_some() && minor.is_none()) {
            return Err(invalid("通配符只能位于版本末尾"));
        }
        let ceiling = if patch.is_some() {
            None
        } else if let Some(minor) = &minor {
            Some(Version {
                core: Core {
                    major: major.clone(),
                    minor: increment(minor),
                    patch: "0".to_owned(),
                },
                prerelease: Vec::new(),
                build: Vec::new(),
            })
        } else {
            Some(Version {
                core: Core {
                    major: increment(&major),
                    minor: "0".to_owned(),
                    patch: "0".to_owned(),
                },
                prerelease: Vec::new(),
                build: Vec::new(),
            })
        };
        Ok(Self {
            floor: Version {
                core: Core {
                    major,
                    minor: minor.unwrap_or_else(|| "0".to_owned()),
                    patch: patch.unwrap_or_else(|| "0".to_owned()),
                },
                prerelease: Vec::new(),
                build: Vec::new(),
            },
            ceiling,
        })
    }
}

#[derive(Clone, Copy, Debug)]
enum Operator {
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Equal,
}

#[derive(Clone, Debug)]
struct Predicate {
    operator: Operator,
    version: Version,
}

impl Predicate {
    fn matches(&self, version: &Version) -> bool {
        let order = version.precedence(&self.version);
        match self.operator {
            Operator::Greater => order == Ordering::Greater,
            Operator::GreaterEqual => order != Ordering::Less,
            Operator::Less => order == Ordering::Less,
            Operator::LessEqual => order != Ordering::Greater,
            Operator::Equal => order == Ordering::Equal,
        }
    }
}

/// 冻结的逗号合取约束；部分版本严格按末位通配区间展开。
#[derive(Clone, Debug)]
pub struct VersionRequirement {
    predicates: Vec<Predicate>,
    prerelease_cores: Vec<Core>,
}

impl VersionRequirement {
    /// 解析精确、插入符、波浪号、比较符、部分版本和通配符。
    pub fn parse(text: &str) -> Result<Self, SourceError> {
        if text.trim().is_empty() {
            return Err(invalid("版本约束不能为空"));
        }
        let mut result = Self {
            predicates: Vec::new(),
            prerelease_cores: Vec::new(),
        };
        for term in text.split(',') {
            let term = term.trim();
            if term.is_empty() || term.contains(char::is_whitespace) {
                return Err(invalid("版本约束项格式不合法"));
            }
            let (operator, value) = if let Some(value) = term.strip_prefix(">=") {
                (Some(Operator::GreaterEqual), value)
            } else if let Some(value) = term.strip_prefix("<=") {
                (Some(Operator::LessEqual), value)
            } else if let Some(value) = term.strip_prefix('>') {
                (Some(Operator::Greater), value)
            } else if let Some(value) = term.strip_prefix('<') {
                (Some(Operator::Less), value)
            } else if let Some(value) = term.strip_prefix('=') {
                (Some(Operator::Equal), value)
            } else {
                (None, term)
            };
            if let Some(value) = value.strip_prefix('^') {
                if operator.is_some() {
                    return Err(invalid("比较符不能与插入符组合"));
                }
                let version = Version::parse(value)?;
                result.track_prerelease(&version);
                let ceiling = next_compatible(&version);
                result.push(Operator::GreaterEqual, version);
                result.push(Operator::Less, ceiling);
            } else if let Some(value) = value.strip_prefix('~') {
                if operator.is_some() {
                    return Err(invalid("比较符不能与波浪号组合"));
                }
                let version = Version::parse(value)?;
                result.track_prerelease(&version);
                let ceiling = Version {
                    core: Core {
                        major: version.core.major.clone(),
                        minor: increment(&version.core.minor),
                        patch: "0".to_owned(),
                    },
                    prerelease: Vec::new(),
                    build: Vec::new(),
                };
                result.push(Operator::GreaterEqual, version);
                result.push(Operator::Less, ceiling);
            } else if value.contains(['-', '+']) {
                let version = Version::parse(value)?;
                result.track_prerelease(&version);
                result.push(operator.unwrap_or(Operator::Equal), version);
            } else {
                let partial = PartialVersion::parse(value)?;
                match (operator.unwrap_or(Operator::Equal), partial.ceiling) {
                    (Operator::Equal, Some(ceiling)) => {
                        result.push(Operator::GreaterEqual, partial.floor);
                        result.push(Operator::Less, ceiling);
                    }
                    (Operator::Greater, Some(ceiling)) => {
                        result.push(Operator::GreaterEqual, ceiling)
                    }
                    (Operator::LessEqual, Some(ceiling)) => result.push(Operator::Less, ceiling),
                    (Operator::Less, Some(_)) => result.push(Operator::Less, partial.floor),
                    (Operator::GreaterEqual, Some(_)) => {
                        result.push(Operator::GreaterEqual, partial.floor);
                    }
                    (Operator::Equal, None) if value == "*" => {}
                    (Operator::Greater, None) if value == "*" => {
                        return Err(invalid("通配根不支持比较符"));
                    }
                    (Operator::Less, None) | (Operator::LessEqual, None) if value == "*" => {
                        return Err(invalid("通配根不支持比较符"));
                    }
                    (Operator::GreaterEqual, None) if value == "*" => {
                        return Err(invalid("通配根不支持比较符"));
                    }
                    (operator, None) => result.push(operator, partial.floor),
                }
            }
        }
        Ok(result)
    }

    /// 默认排除预发布，只允许同一合取中显式提及相同核心版本的预发布。
    #[must_use]
    pub fn matches(&self, version: &Version) -> bool {
        (!version.is_prerelease() || self.permits_prerelease(version))
            && self.matches_precedence(version)
    }

    pub(crate) fn permits_prerelease(&self, version: &Version) -> bool {
        self.prerelease_cores
            .iter()
            .any(|core| core == &version.core)
    }

    pub(crate) fn matches_precedence(&self, version: &Version) -> bool {
        self.predicates
            .iter()
            .all(|predicate| predicate.matches(version))
    }

    fn push(&mut self, operator: Operator, version: Version) {
        self.predicates.push(Predicate { operator, version });
    }

    fn track_prerelease(&mut self, version: &Version) {
        if !version.prerelease.is_empty() {
            self.prerelease_cores.push(version.core.clone());
        }
    }
}

fn next_compatible(version: &Version) -> Version {
    let core = &version.core;
    let next = if core.major != "0" {
        Core {
            major: increment(&core.major),
            minor: "0".to_owned(),
            patch: "0".to_owned(),
        }
    } else if core.minor != "0" {
        Core {
            major: "0".to_owned(),
            minor: increment(&core.minor),
            patch: "0".to_owned(),
        }
    } else {
        Core {
            major: "0".to_owned(),
            minor: "0".to_owned(),
            patch: increment(&core.patch),
        }
    };
    next.version()
}

fn parse_number(text: &str) -> Result<String, SourceError> {
    if text.is_empty()
        || text.len() > 1 && text.starts_with('0')
        || !text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid("版本数字不能为空、含前导零或非 ASCII 数字"));
    }
    Ok(text.to_owned())
}

fn validate_identifiers(text: &str, prerelease: bool) -> Result<Vec<Identifier>, SourceError> {
    text.split('.')
        .map(|part| {
            if part.is_empty()
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            {
                return Err(invalid("版本标识符为空或包含非法字符"));
            }
            if part.bytes().all(|byte| byte.is_ascii_digit()) {
                if prerelease && part.len() > 1 && part.starts_with('0') {
                    return Err(invalid("数字预发布标识符不能含前导零"));
                }
                Ok(Identifier::Number(part.to_owned()))
            } else {
                Ok(Identifier::Text(part.to_owned()))
            }
        })
        .collect()
}

fn compare_decimal(left: &str, right: &str) -> Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn increment(text: &str) -> String {
    let mut digits = text.as_bytes().to_vec();
    for digit in digits.iter_mut().rev() {
        if *digit < b'9' {
            *digit += 1;
            return String::from_utf8(digits).expect("ASCII 数字的递增结果仍为 UTF-8");
        }
        *digit = b'0';
    }
    format!(
        "1{}",
        String::from_utf8(digits).expect("ASCII 数字的递增结果仍为 UTF-8")
    )
}

fn invalid(message: &str) -> SourceError {
    SourceError::new(VERSION_INVALID_CODE, message)
}
