//! Git smart HTTP v1 的受限 pkt-line refs 广告解析，不请求仓库对象。

use std::collections::BTreeMap;

use crate::diagnostics::{SOURCE_INVALID_CODE, SOURCE_UNSUPPORTED_VERSION_CODE};
use crate::source::SourceError;

/// Git 索引显式引用；无引用的稀疏索引源只发现服务器广告的 HEAD。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GitReference {
    /// 广告的默认 HEAD。
    Head,
    /// 可前进的分支。
    Branch(String),
    /// 不允许被改写的标签。
    Tag(String),
    /// 直接钉住完整提交哈希。
    Commit(String),
}

/// 已解析为不可变对象名的 Git 引用。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedGitRef {
    /// 服务端广告的实际 Git 提交哈希。
    pub commit: String,
    /// 用于日志和诊断的引用名。
    pub name: String,
}

impl GitReference {
    /// 配置中的 rev 可以是完整提交哈希或标签名；其余两者始终为具名引用。
    pub fn from_config(kind: &str, value: &str) -> Result<Self, SourceError> {
        if !valid_ref_name(value) {
            return Err(invalid("Git 引用名称不合法"));
        }
        match kind {
            "branch" => Ok(Self::Branch(value.to_owned())),
            "tag" => Ok(Self::Tag(value.to_owned())),
            "rev" if valid_hash(value) => Ok(Self::Commit(value.to_owned())),
            "rev" => Ok(Self::Tag(value.to_owned())),
            _ => Err(invalid("Git 引用类型不受支持")),
        }
    }
}

/// 从完整服务端广告读取单个不可变提交；拒绝不支持的版本和畸形行。
pub fn parse_advertised_refs(
    input: &[u8],
    reference: &GitReference,
) -> Result<ResolvedGitRef, SourceError> {
    if input.len() > 1024 * 1024 {
        return Err(invalid("Git refs 广告过长"));
    }
    let mut offset = 0;
    if next_line(input, &mut offset)? != Some(b"# service=git-upload-pack\n".as_slice())
        || next_line(input, &mut offset)?.is_some()
    {
        return Err(invalid("Git refs 缺少 smart HTTP 服务头或 flush"));
    }
    let mut refs = BTreeMap::new();
    let mut first = true;
    let mut terminated = false;
    while offset < input.len() {
        let Some(line) = next_line(input, &mut offset)? else {
            terminated = true;
            break;
        };
        let text = std::str::from_utf8(line).map_err(|_| invalid("Git refs 含非 UTF-8 行"))?;
        if first && text.starts_with("version ") {
            if text != "version 1\n" {
                return Err(SourceError::new(
                    SOURCE_UNSUPPORTED_VERSION_CODE,
                    "Git refs 协议版本不受支持",
                ));
            }
            first = false;
            continue;
        }
        first = false;
        let clean = text
            .strip_suffix('\n')
            .ok_or_else(|| invalid("Git refs 行缺少换行"))?;
        let advertised = clean.split_once('\0').map_or(clean, |(before, _)| before);
        let (commit, name) = advertised
            .split_once(' ')
            .ok_or_else(|| invalid("Git refs 行缺少引用名"))?;
        if !valid_hash(commit)
            || name.is_empty()
            || name
                .bytes()
                .any(|byte| byte.is_ascii_control() || byte == b' ')
        {
            return Err(invalid("Git refs 哈希或名称不合法"));
        }
        if refs.insert(name.to_owned(), commit.to_owned()).is_some() {
            return Err(invalid("Git refs 广告含重复引用"));
        }
    }
    if !terminated || offset != input.len() || refs.is_empty() {
        return Err(invalid("Git refs 广告未正常结束或没有引用"));
    }
    let (name, commit) = match reference {
        GitReference::Head => ("HEAD".to_owned(), refs.get("HEAD")),
        GitReference::Branch(branch) => {
            if !valid_ref_name(branch) {
                return Err(invalid("分支名称不合法"));
            }
            let name = format!("refs/heads/{branch}");
            (name.clone(), refs.get(&name))
        }
        GitReference::Tag(tag) => {
            if !valid_ref_name(tag) {
                return Err(invalid("标签名称不合法"));
            }
            let name = format!("refs/tags/{tag}");
            (
                name.clone(),
                refs.get(&format!("{name}^{{}}"))
                    .or_else(|| refs.get(&name)),
            )
        }
        GitReference::Commit(commit) => {
            if !valid_hash(commit) {
                return Err(invalid("提交哈希不合法"));
            }
            return Ok(ResolvedGitRef {
                commit: commit.clone(),
                name: commit.clone(),
            });
        }
    };
    let commit = commit.ok_or_else(|| invalid(format!("Git refs 未广告引用 {name}")))?;
    Ok(ResolvedGitRef {
        commit: commit.clone(),
        name,
    })
}

fn next_line<'a>(input: &'a [u8], offset: &mut usize) -> Result<Option<&'a [u8]>, SourceError> {
    let header = input
        .get(*offset..*offset + 4)
        .ok_or_else(|| invalid("Git pkt-line 长度头截断"))?;
    let text = std::str::from_utf8(header).map_err(|_| invalid("Git pkt-line 长度非 ASCII"))?;
    let length =
        usize::from_str_radix(text, 16).map_err(|_| invalid("Git pkt-line 长度非十六进制"))?;
    *offset += 4;
    if length == 0 {
        return Ok(None);
    }
    if !(5..=65520).contains(&length) {
        return Err(invalid("Git pkt-line 长度不合法"));
    }
    let line = input
        .get(*offset..*offset + length - 4)
        .ok_or_else(|| invalid("Git pkt-line 正文截断"))?;
    *offset += length - 4;
    Ok(Some(line))
}

fn valid_hash(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_ref_name(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.contains("..")
        && !value.contains("@{")
        && value.split('/').all(|part| {
            !part.is_empty()
                && !part.starts_with('.')
                && !part.ends_with('.')
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        })
}

fn invalid(message: impl Into<String>) -> SourceError {
    SourceError::new(SOURCE_INVALID_CODE, message)
}
