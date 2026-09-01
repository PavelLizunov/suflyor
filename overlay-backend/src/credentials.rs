//! Windows Credential Manager storage for direct cloud-provider API keys.
//!
//! Legacy bridge/STT/Hermes secrets intentionally remain in the existing
//! config format for compatibility. New OpenAI and Anthropic keys never enter
//! `Config`, config exports, backups, or diagnostics.

#[cfg(windows)]
use anyhow::Context;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretSlot {
    OpenAi,
    Anthropic,
}

impl SecretSlot {
    #[must_use]
    pub const fn target(self) -> &'static str {
        match self {
            Self::OpenAi => "suflyor/ai/openai/v1",
            Self::Anthropic => "suflyor/ai/anthropic/v1",
        }
    }
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
pub fn write(slot: SecretSlot, secret: &str) -> Result<()> {
    use windows::core::PWSTR;
    use windows::Win32::Security::Credentials::{
        CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };

    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return delete(slot);
    }
    let mut target = wide(slot.target());
    let mut username = wide("suflyor");
    let mut blob = trimmed.as_bytes().to_vec();
    if blob.len() > 2_560 {
        blob.fill(0);
        anyhow::bail!("credential is too large");
    }
    let blob_len = u32::try_from(blob.len()).context("credential is too large")?;
    let credential = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(target.as_mut_ptr()),
        CredentialBlobSize: blob_len,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        UserName: PWSTR(username.as_mut_ptr()),
        ..Default::default()
    };
    let result = unsafe { CredWriteW(&credential, 0) };
    blob.fill(0);
    result.context("store protected credential")
}

#[cfg(windows)]
pub fn read(slot: SecretSlot) -> Result<Option<String>> {
    use windows::core::{HRESULT, PCWSTR};
    use windows::Win32::Foundation::ERROR_NOT_FOUND;
    use windows::Win32::Security::Credentials::{
        CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
    };

    let target = wide(slot.target());
    let mut raw: *mut CREDENTIALW = std::ptr::null_mut();
    let result = unsafe { CredReadW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None, &mut raw) };
    if let Err(error) = result {
        if error.code() == HRESULT::from_win32(ERROR_NOT_FOUND.0) {
            return Ok(None);
        }
        return Err(error).context("read protected credential");
    }
    if raw.is_null() {
        return Err(anyhow!("credential store returned no value"));
    }
    let credential = unsafe { &*raw };
    let bytes = if credential.CredentialBlob.is_null() || credential.CredentialBlobSize == 0 {
        &[][..]
    } else {
        unsafe {
            std::slice::from_raw_parts(
                credential.CredentialBlob,
                credential.CredentialBlobSize as usize,
            )
        }
    };
    let parsed = String::from_utf8(bytes.to_vec()).context("decode protected credential");
    unsafe { CredFree(raw.cast()) };
    parsed.map(Some)
}

#[cfg(windows)]
pub fn delete(slot: SecretSlot) -> Result<()> {
    use windows::core::{HRESULT, PCWSTR};
    use windows::Win32::Foundation::ERROR_NOT_FOUND;
    use windows::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};

    let target = wide(slot.target());
    match unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) } {
        Ok(()) => Ok(()),
        Err(error) if error.code() == HRESULT::from_win32(ERROR_NOT_FOUND.0) => Ok(()),
        Err(error) => Err(error).context("delete protected credential"),
    }
}

#[cfg(not(windows))]
mod posix_credentials {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    use std::path::{Path, PathBuf};

    fn credentials_path() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .ok_or_else(|| anyhow!("could not resolve user config dir"))?
            .join("suflyor");
        fs::create_dir_all(&dir)?;
        Ok(dir.join("credentials.json"))
    }

    fn read_map() -> HashMap<String, String> {
        let Ok(path) = credentials_path() else {
            return HashMap::new();
        };
        let Ok(bytes) = fs::read(path) else {
            return HashMap::new();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub(super) fn write_to_path(path: &Path, bytes: &[u8]) -> Result<()> {
        #[cfg(unix)]
        {
            use std::io::Write;

            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(path)?;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
            file.write_all(bytes)?;
        }
        #[cfg(not(unix))]
        fs::write(path, bytes)?;
        Ok(())
    }

    fn write_map(map: &HashMap<String, String>) -> Result<()> {
        let path = credentials_path()?;
        let bytes = serde_json::to_vec_pretty(map)?;
        write_to_path(&path, &bytes)
    }

    pub fn write(slot: SecretSlot, secret: &str) -> Result<()> {
        let trimmed = secret.trim();
        let mut map = read_map();
        if trimmed.is_empty() {
            map.remove(slot.target());
        } else {
            map.insert(slot.target().to_string(), trimmed.to_string());
        }
        write_map(&map)
    }

    pub fn read(slot: SecretSlot) -> Result<Option<String>> {
        let map = read_map();
        Ok(map.get(slot.target()).cloned())
    }

    pub fn delete(slot: SecretSlot) -> Result<()> {
        let mut map = read_map();
        map.remove(slot.target());
        write_map(&map)
    }
}

#[cfg(not(windows))]
pub use posix_credentials::*;

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn provider_slots_are_stable_and_distinct() {
        assert_eq!(SecretSlot::OpenAi.target(), "suflyor/ai/openai/v1");
        assert_eq!(SecretSlot::Anthropic.target(), "suflyor/ai/anthropic/v1");
        assert_ne!(SecretSlot::OpenAi.target(), SecretSlot::Anthropic.target());
    }

    #[cfg(unix)]
    #[test]
    fn posix_credentials_are_written_with_mode_0600() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("create temp directory");
        let path = temp.path().join("credentials.json");
        posix_credentials::write_to_path(&path, b"secret").expect("create credentials file");
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        posix_credentials::write_to_path(&path, b"updated").expect("update credentials file");
        assert_eq!(std::fs::read(&path).unwrap(), b"updated");
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
