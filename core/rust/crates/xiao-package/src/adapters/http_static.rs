//! 同步 HTTP 静态索引读取与有限的正文 Range 续传。

use std::io::Read;
use std::time::Duration;

use ureq::{Agent, http};

use super::{
    IndexSnapshot, PackageSourceAdapter, check_package_request, parse_package, parse_snapshot,
    verify_artifact,
};
use crate::credentials;
use crate::diagnostics::{SOURCE_INVALID_CODE, SOURCE_UNAVAILABLE_CODE};
use crate::federation::{ArtifactReference, IndexPackage};
use crate::source::{SourceDescriptor, SourceError};

const METADATA_LIMIT: usize = 4 * 1024 * 1024;
const ARTIFACT_LIMIT: u64 = 128 * 1024 * 1024;

/// 不跟随重定向、限制响应大小与超时的同步传输；3xx（含跨 host）均报告不可用。
#[derive(Clone, Debug)]
pub struct HttpStaticAdapter {
    agent: Agent,
}

impl Default for HttpStaticAdapter {
    fn default() -> Self {
        Self::with_timeout(Duration::from_secs(10))
    }
}

impl HttpStaticAdapter {
    pub(crate) fn source_list(&self, location: &str) -> Result<String, SourceError> {
        crate::source::source_id("static", location)?;
        let (base, name) = location
            .rsplit_once('/')
            .ok_or_else(|| SourceError::new(SOURCE_INVALID_CODE, "源列表地址必须包含文件名"))?;
        if name.is_empty() {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "源列表文件名不能为空",
            ));
        }
        self.get_text(base, name, None)
    }
    /// 使用无自动重定向的同步客户端。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 自托管服务可指定整个请求的超时时间；重定向仍固定禁用。
    #[must_use]
    pub fn with_timeout(timeout: Duration) -> Self {
        let config = Agent::config_builder()
            .max_redirects(0)
            .timeout_global(Some(timeout))
            .build();
        Self {
            agent: config.into(),
        }
    }

    pub(super) fn get_text(
        &self,
        base: &str,
        relative: &str,
        alias: Option<&str>,
    ) -> Result<String, SourceError> {
        let url = join_url(base, relative)?;
        let mut response = self.get(&url, None, alias)?;
        let bytes = read_limited(response.body_mut().as_reader(), METADATA_LIMIT, &url)?;
        String::from_utf8(bytes).map_err(|_| {
            SourceError::new(SOURCE_INVALID_CODE, format!("远程元数据不是 UTF-8：{url}"))
        })
    }

    pub(super) fn get_refs(&self, base: &str, alias: Option<&str>) -> Result<Vec<u8>, SourceError> {
        let url = format!("{}?service=git-upload-pack", join_url(base, "info/refs")?);
        let mut response = self.get(&url, None, alias)?;
        read_limited(response.body_mut().as_reader(), 1024 * 1024, &url)
    }

    fn get(
        &self,
        url: &str,
        start: Option<usize>,
        alias: Option<&str>,
    ) -> Result<http::Response<ureq::Body>, SourceError> {
        let credential = credentials::lookup(alias)?;
        if credential.is_some() {
            let uri: http::Uri = url.parse().map_err(|_| {
                SourceError::new(
                    crate::diagnostics::TRUST_CREDENTIALS_CODE,
                    "包源认证地址不安全",
                )
            })?;
            if uri.scheme_str() != Some("https")
                && !matches!(uri.host(), Some("127.0.0.1" | "localhost" | "[::1]"))
            {
                return Err(SourceError::new(
                    crate::diagnostics::TRUST_CREDENTIALS_CODE,
                    "凭据只允许 HTTPS 或本机测试源",
                ));
            }
        }
        let request = self.agent.get(url).header("Accept-Encoding", "identity");
        let request = if let Some(credential) = &credential {
            request.header("Authorization", &format!("Bearer {}", credential.as_str()))
        } else {
            request
        };
        let request = if let Some(start) = start {
            request.header("Range", &format!("bytes={start}-"))
        } else {
            request
        };
        let response = request.call().map_err(|error| {
            let reason = match error {
                ureq::Error::Http(_) => "HTTP 请求头无效",
                ureq::Error::StatusCode(_) => "HTTP 状态不成功",
                ureq::Error::Protocol(_) => "HTTP 响应协议错误",
                ureq::Error::Io(ref io) if io.kind() == std::io::ErrorKind::ConnectionReset => {
                    "网络连接已重置"
                }
                ureq::Error::Io(ref io) if io.kind() == std::io::ErrorKind::UnexpectedEof => {
                    "网络连接过早关闭"
                }
                ureq::Error::Io(ref io) if io.kind() == std::io::ErrorKind::ConnectionAborted => {
                    "网络连接被中止"
                }
                ureq::Error::Io(ref io) if io.kind() == std::io::ErrorKind::BrokenPipe => {
                    "网络连接管道中断"
                }
                ureq::Error::Io(_) => "网络连接中断",
                ureq::Error::Timeout(_) => "HTTP 请求超时",
                _ => "包源 HTTP 请求失败",
            };
            SourceError::new(SOURCE_UNAVAILABLE_CODE, reason)
        })?;
        if !matches!(
            (start, response.status().as_u16()),
            (None, 200) | (Some(_), 206)
        ) {
            return Err(SourceError::new(
                SOURCE_UNAVAILABLE_CODE,
                format!("{url} 返回 HTTP {}（重定向不跟随）", response.status()),
            ));
        }
        Ok(response)
    }

    pub(super) fn artifact(
        &self,
        base: &str,
        artifact: &ArtifactReference,
        alias: Option<&str>,
    ) -> Result<Vec<u8>, SourceError> {
        if !valid_relative(&artifact.location) || artifact.length > ARTIFACT_LIMIT {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "正文路径或长度不合法",
            ));
        }
        let url = join_url(base, &artifact.location)?;
        let mut bytes = Vec::new();
        for _attempt in 0..3 {
            let start = bytes.len();
            let mut response = self.get(&url, (start != 0).then_some(start), alias)?;
            if start == 0 && response.status().as_u16() != 200
                || start != 0 && response.status().as_u16() != 206
            {
                return Err(SourceError::new(
                    SOURCE_UNAVAILABLE_CODE,
                    "Range 响应状态不符",
                ));
            }
            let expected_range_length = if start == 0 {
                None
            } else {
                Some(
                    valid_content_range(&response, start as u64, artifact.length).ok_or_else(
                        || SourceError::new(SOURCE_UNAVAILABLE_CODE, "Range 响应范围不符"),
                    )?,
                )
            };
            let mut buffer = [0; 8192];
            let mut reader = response.body_mut().as_reader();
            let mut interrupted = false;
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        let next_size = bytes.len().saturating_add(count);
                        if next_size as u64 > artifact.length {
                            return Err(SourceError::new(
                                SOURCE_UNAVAILABLE_CODE,
                                "正文响应超出声明长度",
                            ));
                        }
                        if expected_range_length
                            .is_some_and(|length| (next_size - start) as u64 > length)
                        {
                            return Err(SourceError::new(
                                SOURCE_UNAVAILABLE_CODE,
                                "Range 响应正文超出声明范围",
                            ));
                        }
                        bytes.extend_from_slice(&buffer[..count]);
                    }
                    Err(_) => {
                        interrupted = true;
                        break;
                    }
                }
            }
            if !interrupted
                && expected_range_length
                    .is_some_and(|length| ((bytes.len() - start) as u64) < length)
            {
                return Err(SourceError::new(
                    SOURCE_UNAVAILABLE_CODE,
                    "Range 响应正文短于声明范围",
                ));
            }
            if bytes.len() as u64 == artifact.length {
                verify_artifact(artifact, &bytes)?;
                return Ok(bytes);
            }
            if bytes.len() == start {
                break;
            }
        }
        Err(SourceError::new(
            SOURCE_UNAVAILABLE_CODE,
            format!("正文 {url} 截断或续传失败"),
        ))
    }
}

impl PackageSourceAdapter for HttpStaticAdapter {
    fn supports_partial_reads(&self) -> bool {
        true
    }

    fn read_snapshot(&self, source: &SourceDescriptor) -> Result<IndexSnapshot, SourceError> {
        require_kind(source)?;
        parse_snapshot(
            source,
            &self.get_text(
                &source.location,
                "snapshot.json",
                source.source.alias.as_deref(),
            )?,
        )
    }

    fn read_package(
        &self,
        source: &SourceDescriptor,
        snapshot: &IndexSnapshot,
        name: &str,
    ) -> Result<Vec<IndexPackage>, SourceError> {
        require_kind(source)?;
        check_package_request(source, snapshot, name)?;
        if !snapshot.manifest.shards.contains_key(name) {
            return Ok(Vec::new());
        }
        parse_package(
            source,
            snapshot,
            name,
            &self.get_text(
                &source.location,
                &format!("index/{name}.json"),
                source.source.alias.as_deref(),
            )?,
        )
    }

    fn read_artifact(
        &self,
        source: &SourceDescriptor,
        artifact: &ArtifactReference,
    ) -> Result<Vec<u8>, SourceError> {
        require_kind(source)?;
        self.artifact(&source.location, artifact, source.source.alias.as_deref())
    }
}

fn require_kind(source: &SourceDescriptor) -> Result<(), SourceError> {
    if !matches!(source.kind.as_str(), "static" | "registry") {
        return Err(SourceError::new(
            SOURCE_INVALID_CODE,
            "HTTP 适配器只接受静态源或注册表源",
        ));
    }
    Ok(())
}

pub(super) fn valid_relative(path: &str) -> bool {
    !path.is_empty()
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        })
}

pub(super) fn join_url(base: &str, relative: &str) -> Result<String, SourceError> {
    let uri: http::Uri = base
        .parse()
        .map_err(|_| SourceError::new(SOURCE_INVALID_CODE, "HTTP 源地址不合法"))?;
    if !matches!(uri.scheme_str(), Some("http" | "https"))
        || uri.authority().is_none()
        || uri
            .authority()
            .is_some_and(|part| part.as_str().contains('@'))
        || base.contains(['?', '#', '\\', '%'])
        || uri.path().split('/').any(|part| matches!(part, "." | ".."))
        || !valid_relative(relative)
    {
        return Err(SourceError::new(
            SOURCE_INVALID_CODE,
            "HTTP 源地址或相对路径不安全",
        ));
    }
    Ok(format!("{}/{}", base.trim_end_matches('/'), relative))
}

fn read_limited(reader: impl Read, limit: usize, url: &str) -> Result<Vec<u8>, SourceError> {
    let mut bytes = Vec::new();
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            SourceError::new(SOURCE_UNAVAILABLE_CODE, format!("读取 {url} 失败：{error}"))
        })?;
    if bytes.len() > limit {
        return Err(SourceError::new(
            SOURCE_INVALID_CODE,
            format!("{url} 超出元数据大小限制"),
        ));
    }
    Ok(bytes)
}

fn valid_content_range(
    response: &http::Response<ureq::Body>,
    start: u64,
    length: u64,
) -> Option<u64> {
    let range = response
        .headers()
        .get("content-range")
        .and_then(|value| value.to_str().ok())?;
    let (begin, total) = range
        .strip_prefix("bytes ")
        .and_then(|text| text.split_once('/'))?;
    let (begin, end) = begin.split_once('-')?;
    let end = end.parse::<u64>().ok()?;
    (begin.parse::<u64>() == Ok(start)
        && total.parse::<u64>() == Ok(length)
        && end >= start
        && end < length)
        .then_some(end - start + 1)
}
