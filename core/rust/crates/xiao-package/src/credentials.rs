//! 包源令牌只在传输请求现场读取；失败和调试输出都不包含机密文本。

#[cfg(unix)]
use std::collections::BTreeMap;
use std::env;
use std::fs;
#[cfg(unix)]
use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::diagnostics::TRUST_CREDENTIALS_CODE;
use crate::source::SourceError;

pub(crate) struct SecretToken(String);

impl std::fmt::Debug for SecretToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecretToken(<redacted>)")
    }
}

impl SecretToken {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

fn invalid() -> SourceError {
    SourceError::new(TRUST_CREDENTIALS_CODE, "包源凭据不可用或权限不安全")
}

fn token(value: String) -> Result<SecretToken, SourceError> {
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(invalid());
    }
    Ok(SecretToken(value))
}

fn env_name(alias: &str) -> Option<String> {
    alias
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        .then(|| format!("XIAO_SOURCE_TOKEN_{}", alias.to_ascii_uppercase()))
}

pub(crate) fn lookup(alias: Option<&str>) -> Result<Option<SecretToken>, SourceError> {
    let Some(alias) = alias else { return Ok(None) };
    if let Some(name) = env_name(alias) {
        match env::var(&name) {
            Ok(value) => return token(value).map(Some),
            Err(env::VarError::NotUnicode(_)) => return Err(invalid()),
            Err(env::VarError::NotPresent) => {}
        }
    }
    let home = env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
    home.map(PathBuf::from)
        .map(|path| from_file(alias, &path.join(".xiao/credentials")))
        .transpose()
        .map(Option::flatten)
}

fn from_file(_alias: &str, path: &Path) -> Result<Option<SecretToken>, SourceError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(invalid()),
    };
    if !metadata.file_type().is_file() {
        return Err(invalid());
    }
    #[cfg(unix)]
    {
        if metadata.permissions().mode() & 0o777 != 0o600 {
            return Err(invalid());
        }
    }
    #[cfg(not(unix))]
    {
        Err(invalid())
    }
    #[cfg(unix)]
    {
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|_| invalid())?;
        let opened = file.metadata().map_err(|_| invalid())?;
        if !opened.is_file() || opened.permissions().mode() & 0o777 != 0o600 {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        file.take(65_537)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid())?;
        if bytes.len() > 65_536 {
            return Err(invalid());
        }
        let values: BTreeMap<String, String> =
            serde_json::from_slice(&bytes).map_err(|_| invalid())?;
        values.get(_alias).cloned().map(token).transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_contains_token() {
        let secret = token("never-show-me".into()).unwrap();
        assert!(!format!("{secret:?}").contains("never-show-me"));
    }

    #[test]
    fn aliases_without_environment_names_remain_valid() {
        assert_eq!(env_name("mirror-prod"), None);
        assert_eq!(env_name("MIRROR_1"), None);
        assert_eq!(
            env_name("mirror_1"),
            Some("XIAO_SOURCE_TOKEN_MIRROR_1".into())
        );
    }

    #[cfg(unix)]
    #[test]
    fn fallback_requires_exact_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let path = env::temp_dir().join(format!("xiao-credentials-{}", std::process::id()));
        fs::write(&path, r#"{"private":"never-show-me"}"#).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            from_file("private", &path).unwrap_err().code,
            TRUST_CREDENTIALS_CODE
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            from_file("private", &path).unwrap().unwrap().as_str(),
            "never-show-me"
        );
        fs::remove_file(path).unwrap();
    }
}
