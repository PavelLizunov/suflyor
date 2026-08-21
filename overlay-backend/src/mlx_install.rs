//! Pinned, on-demand MLX model installer for the macOS sidecar.

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

#[cfg(any(target_os = "macos", test))]
use std::path::Component;
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
#[cfg(any(target_os = "macos", test))]
use std::sync::atomic::Ordering;
#[cfg(target_os = "macos")]
use std::time::Duration;

pub const DEFAULT_TEXT_MODEL: &str = "LiquidAI/LFM2.5-8B-A1B-MLX-4bit";
pub const DEFAULT_VISION_MODEL: &str = "mlx-community/Qwen3.5-2B-4bit";
const MARKER: &str = ".suflyor-mlx-ready-v1";

#[cfg(any(target_os = "macos", test))]
type DownloadFn = dyn Fn(&str, &Path, &AtomicBool, &dyn Fn(u64)) -> Result<()>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelRole {
    Text,
    Vision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogFile {
    pub path: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogModel {
    pub id: &'static str,
    pub revision: &'static str,
    pub supports_images: bool,
    pub license: &'static str,
    pub files: &'static [CatalogFile],
}

const TEXT_FILES: &[CatalogFile] = &[
    CatalogFile {
        path: "LICENSE",
        size: 10_574,
        sha256: "4d28ca14dedc0b3d0fcc2b3339f0e79931faa33874f3d24f522183a8fc70068c",
    },
    CatalogFile {
        path: "chat_template.jinja",
        size: 4_674,
        sha256: "f434e8c96e6c0a63a022a3ad0a299bb94e58aa90e3c9ebe65034f8e8c6188aa9",
    },
    CatalogFile {
        path: "config.json",
        size: 1_858,
        sha256: "d84a36881f9cb75105ad4810945668d5a2f6911dce85011d181af928cb955e59",
    },
    CatalogFile {
        path: "generation_config.json",
        size: 230,
        sha256: "9583face86b498637e45a76535052166a59586f1bdde9b98c8074b728f9f6b42",
    },
    CatalogFile {
        path: "model.safetensors",
        size: 4_834_029_941,
        sha256: "3cc15631acc1894b3584ac11fb4122beee50b604e1a0be575da686cee87aa3a4",
    },
    CatalogFile {
        path: "model.safetensors.index.json",
        size: 40_148,
        sha256: "c6453667bbe2ef15b95e93d69b091e1eaa853796ce9efde88f9c92133ae92d2b",
    },
    CatalogFile {
        path: "tokenizer.json",
        size: 17_905_598,
        sha256: "695be7802a0e4b8a81048f0ff5ebb7fc811a0ba5a6be63dbb24deb5a81096f41",
    },
    CatalogFile {
        path: "tokenizer_config.json",
        size: 315,
        sha256: "b53c3caceb7fa0de424e1aa67b3cc5008cd2f8b8d2426a8fa221ef3cf1a949d5",
    },
];

const VISION_FILES: &[CatalogFile] = &[
    CatalogFile {
        path: "chat_template.jinja",
        size: 7_755,
        sha256: "273d8e0e683b885071fb17e08d71e5f2a5ddfb5309756181681de4f5a1822d80",
    },
    CatalogFile {
        path: "config.json",
        size: 3_113,
        sha256: "beb7fc5a6e0405fe332821cf1a8ef7b69bb390a8c8933171647de5579debf949",
    },
    CatalogFile {
        path: "model.safetensors",
        size: 1_722_271_785,
        sha256: "713fe7e5d3c3965f7106b0d0ee17615f7869c23c8d327996df8c1196fbcf07d5",
    },
    CatalogFile {
        path: "model.safetensors.index.json",
        size: 81_722,
        sha256: "8294c05cca7d53a6c33e3db2b379539bd296d054e0b689711b16b6ac93c7e49d",
    },
    CatalogFile {
        path: "preprocessor_config.json",
        size: 390,
        sha256: "27225450ac9c6529872ee1924fcb0962ff5634834f817040f444118116f4e516",
    },
    CatalogFile {
        path: "processor_config.json",
        size: 1_300,
        sha256: "14932921ca485d458a04dafd8069fbb0a4505622a48208d19ed247115801385b",
    },
    CatalogFile {
        path: "tokenizer.json",
        size: 19_989_343,
        sha256: "87a7830d63fcf43bf241c3c5242e96e62dd3fdc29224ca26fed8ea333db72de4",
    },
    CatalogFile {
        path: "tokenizer_config.json",
        size: 1_139,
        sha256: "e98f1901ac6f0adff67b1d540bfa0c36ac1a0cf59eb72ed78146ef89aafa1182",
    },
    CatalogFile {
        path: "video_preprocessor_config.json",
        size: 385,
        sha256: "7768af27c1fafa9cc9011c1dc20067e03f8915e03b63504550e11d5066986d13",
    },
    CatalogFile {
        path: "vocab.json",
        size: 6_722_759,
        sha256: "ce99b4cb2983d118806ce0a8b777a35b093e2000a503ebde25853284c9dfa003",
    },
];

pub const CATALOG: &[CatalogModel] = &[
    CatalogModel {
        id: DEFAULT_TEXT_MODEL,
        revision: "2e92b640a63d47ad4dcf81a19a366b902356b3bc",
        supports_images: false,
        license: "LFM Open License v1.0",
        files: TEXT_FILES,
    },
    CatalogModel {
        id: DEFAULT_VISION_MODEL,
        revision: "674aaa7240b91e8012fcad5d791b7dfe5ba90207",
        supports_images: true,
        license: "Apache-2.0",
        files: VISION_FILES,
    },
];

#[must_use]
pub fn catalog_for_role(role: ModelRole) -> Vec<&'static CatalogModel> {
    CATALOG
        .iter()
        .filter(|model| match role {
            ModelRole::Text => !model.supports_images,
            ModelRole::Vision => model.supports_images,
        })
        .collect()
}

#[must_use]
pub fn catalog_model(id: &str) -> Option<&'static CatalogModel> {
    CATALOG.iter().find(|model| model.id == id)
}

fn safe_key(model: &CatalogModel) -> String {
    let mut key = String::with_capacity(model.id.len() + 1);
    for character in model.id.chars() {
        match character {
            '/' => key.push_str("--"),
            character if character.is_ascii_alphanumeric() || "._-".contains(character) => {
                key.push(character);
            }
            _ => key.push('-'),
        }
    }
    if matches!(key.as_str(), "" | "." | "..") {
        key.insert_str(0, "model-");
    }
    key
}

#[cfg(any(target_os = "macos", test))]
fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('/')
        && !path.contains('\\')
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

#[must_use]
pub fn snapshot_path_in(data_root: &Path, model: &CatalogModel) -> PathBuf {
    data_root
        .join("models")
        .join("mlx")
        .join(safe_key(model))
        .join(model.revision)
}

#[must_use]
pub fn installed_snapshot(id: &str) -> Option<PathBuf> {
    let model = catalog_model(id)?;
    let path = snapshot_path_in(&crate::paths::data_root()?, model);
    check_snapshot_verified(model, &path).then_some(path)
}

pub fn install(id: &str, cancel: &AtomicBool, progress: &dyn Fn(u64, u64)) -> Result<PathBuf> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (id, cancel, progress);
        bail!("MLX installation is unavailable on this platform");
    }
    #[cfg(target_os = "macos")]
    {
        let root = crate::paths::data_root().context("managed data directory unavailable")?;
        install_in(&root, id, cancel, progress, &curl_download)
    }
}

#[cfg(target_os = "macos")]
fn install_in(
    data_root: &Path,
    id: &str,
    cancel: &AtomicBool,
    progress: &dyn Fn(u64, u64),
    download: &DownloadFn,
) -> Result<PathBuf> {
    let model = catalog_model(id).context("unsupported MLX model")?;
    install_model_in(data_root, model, cancel, progress, download)
}

#[cfg(any(target_os = "macos", test))]
fn install_model_in(
    data_root: &Path,
    model: &CatalogModel,
    cancel: &AtomicBool,
    progress: &dyn Fn(u64, u64),
    download: &DownloadFn,
) -> Result<PathBuf> {
    validate_catalog_model(model)?;
    let final_dir = snapshot_path_in(data_root, model);
    if check_snapshot_verified(model, &final_dir) {
        return Ok(final_dir);
    }
    std::fs::create_dir_all(data_root).context("prepare managed data directory")?;
    reject_symlink(data_root)?;
    let models_root = prepare_child_dir(data_root, "models")?;
    let mlx_root = prepare_child_dir(&models_root, "mlx")?;
    let model_root = prepare_child_dir(&mlx_root, &safe_key(model))?;
    let staging = model_root.join(format!("{}.staging", model.revision));
    if path_present(&final_dir) {
        recover_invalid_snapshot(&model_root, model, &final_dir, &staging)?;
    }
    let staging_name = staging
        .file_name()
        .and_then(|name| name.to_str())
        .context("invalid MLX staging directory")?
        .to_string();
    let staging = prepare_child_dir(&model_root, &staging_name)?;

    let remaining = remaining_bytes(model, &staging)?;
    ensure_disk_space(data_root, remaining)?;
    let total = model.files.iter().try_fold(0_u64, |sum, f| {
        sum.checked_add(f.size).context("MLX catalog size overflow")
    })?;
    let mut done: u64 = model
        .files
        .iter()
        .filter(|file| file_matches(&staging.join(file.path), file))
        .map(|file| file.size)
        .sum();
    progress(
        done.saturating_add(first_partial_bytes(model, &staging)),
        total,
    );
    for file in model.files {
        if cancel.load(Ordering::Acquire) {
            bail!("MLX installation cancelled");
        }
        let target = staging.join(file.path);
        let partial = staging.join(format!("{}.part", file.path));
        if file_matches(&target, file) {
            reject_symlink_if_present(&partial)?;
            if path_present(&partial) {
                remove_exact_file(&staging, &partial)?;
            }
            continue;
        }
        reject_symlink_if_present(&target)?;
        reject_symlink_if_present(&partial)?;
        if path_present(&target) {
            remove_exact_file(&staging, &target)?;
        }
        if let Some(parent) = partial.parent() {
            std::fs::create_dir_all(parent).context("prepare MLX file directory")?;
        }
        if partial
            .metadata()
            .map(|m| m.len() >= file.size)
            .unwrap_or(false)
        {
            if file_matches(&partial, file) {
                std::fs::rename(&partial, &target).context("publish verified MLX file")?;
                done = done.saturating_add(file.size);
                progress(done, total);
                continue;
            }
            remove_exact_file(&staging, &partial)?;
        }
        let url = format!(
            "https://huggingface.co/{}/resolve/{}/{}",
            model.id, model.revision, file.path
        );
        let report_partial = |partial_done: u64| {
            progress(done.saturating_add(partial_done.min(file.size)), total);
        };
        download(&url, &partial, cancel, &report_partial)?;
        if !file_matches(&partial, file) {
            remove_exact_file(&staging, &partial)?;
            bail!("MLX model verification failed");
        }
        std::fs::rename(&partial, &target).context("publish verified MLX file")?;
        done = done.saturating_add(file.size);
        progress(done, total);
    }
    let marker = staging.join(MARKER);
    reject_symlink_if_present(&marker)?;
    if path_present(&marker) {
        remove_exact_file(&staging, &marker)?;
    }
    std::fs::write(&marker, format!("{}\n{}\n", model.id, model.revision))
        .context("write MLX publish marker")?;
    if path_present(&final_dir) {
        bail!("MLX snapshot publish conflict");
    }
    std::fs::rename(&staging, &final_dir).context("publish MLX snapshot")?;
    Ok(final_dir)
}

#[cfg(target_os = "macos")]
fn curl_download(
    url: &str,
    partial: &Path,
    cancel: &AtomicBool,
    progress: &dyn Fn(u64),
) -> Result<()> {
    let mut child = Command::new("/usr/bin/curl")
        .args([
            "-L",
            "--fail",
            "--retry",
            "10",
            "--retry-all-errors",
            "-C",
            "-",
            "-o",
        ])
        .arg(partial)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("start MLX download")?;
    loop {
        if cancel.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            bail!("MLX installation cancelled");
        }
        if let Some(status) = child.try_wait().context("monitor MLX download")? {
            if status.success() {
                progress(partial.metadata().map_or(0, |metadata| metadata.len()));
                return Ok(());
            }
            bail!("MLX download failed");
        }
        progress(partial.metadata().map_or(0, |metadata| metadata.len()));
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn file_matches(path: &Path, file: &CatalogFile) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_file() && m.len() == file.size)
        && hash_file(path).is_ok_and(|hash| hash == file.sha256)
}

fn hash_file(path: &Path) -> Result<String> {
    let mut input = File::open(path).context("read MLX model file")?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 1024 * 1024];
    loop {
        let read = input.read(&mut buf).context("verify MLX model file")?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(crate::download::hex(&hasher.finalize()))
}

fn check_snapshot_fast(model: &CatalogModel, path: &Path) -> bool {
    let marker = path.join(MARKER);
    std::fs::symlink_metadata(&marker).is_ok_and(|metadata| metadata.file_type().is_file())
        && std::fs::read_to_string(marker)
            .is_ok_and(|value| value == format!("{}\n{}\n", model.id, model.revision))
        && model.files.iter().all(|file| {
            std::fs::symlink_metadata(path.join(file.path))
                .is_ok_and(|metadata| metadata.file_type().is_file() && metadata.len() == file.size)
        })
}

fn check_snapshot_verified(model: &CatalogModel, path: &Path) -> bool {
    check_snapshot_fast(model, path)
        && model
            .files
            .iter()
            .all(|file| file_matches(&path.join(file.path), file))
}

#[cfg(any(target_os = "macos", test))]
fn path_present(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

#[cfg(any(target_os = "macos", test))]
fn remaining_bytes(model: &CatalogModel, staging: &Path) -> Result<u64> {
    model.files.iter().try_fold(0_u64, |sum, file| {
        let target = staging.join(file.path);
        let present = if file_matches(&target, file) {
            file.size
        } else {
            partial_bytes(&staging.join(format!("{}.part", file.path)), file)
        };
        sum.checked_add(file.size - present)
            .context("MLX catalog size overflow")
    })
}

#[cfg(any(target_os = "macos", test))]
fn partial_bytes(path: &Path, file: &CatalogFile) -> u64 {
    std::fs::symlink_metadata(path)
        .ok()
        .filter(|metadata| metadata.file_type().is_file())
        .map_or(0, |metadata| {
            let length = metadata.len();
            if length < file.size || (length == file.size && file_matches(path, file)) {
                length
            } else {
                0
            }
        })
}

#[cfg(any(target_os = "macos", test))]
fn first_partial_bytes(model: &CatalogModel, staging: &Path) -> u64 {
    model
        .files
        .iter()
        .find_map(|file| {
            if file_matches(&staging.join(file.path), file) {
                return None;
            }
            let bytes = partial_bytes(&staging.join(format!("{}.part", file.path)), file);
            (bytes > 0).then_some(bytes)
        })
        .unwrap_or(0)
}

#[cfg(any(target_os = "macos", test))]
fn required_with_headroom(bytes: u64) -> Option<u64> {
    bytes.checked_add(bytes.checked_div(10)?)
}

#[cfg(any(target_os = "macos", test))]
fn parse_df_available(output: &str) -> Option<u64> {
    let blocks = output
        .lines()
        .rev()
        .find_map(|line| line.split_whitespace().nth(3)?.parse::<u64>().ok())?;
    blocks.checked_mul(1024)
}

#[cfg(any(target_os = "macos", test))]
fn ensure_disk_space(path: &Path, remaining: u64) -> Result<()> {
    let required = required_with_headroom(remaining).context("MLX disk requirement overflow")?;
    if required == 0 {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/bin/df")
            .args(["-Pk"])
            .arg(path)
            .output()
            .context("check MLX disk space")?;
        let free = output
            .status
            .success()
            .then(|| parse_df_available(&String::from_utf8_lossy(&output.stdout)))
            .flatten()
            .context("check MLX disk space")?;
        if free < required {
            bail!("not enough free space for MLX model");
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Ok(())
    }
}

#[cfg(any(target_os = "macos", test))]
fn validate_catalog_model(model: &CatalogModel) -> Result<()> {
    if model.revision.len() != 40 || !model.revision.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("invalid MLX catalog revision");
    }
    let mut seen = std::collections::BTreeSet::new();
    for file in model.files {
        if !safe_relative(file.path)
            || file.size == 0
            || file.sha256.len() != 64
            || !file.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || !seen.insert(file.path)
        {
            bail!("invalid MLX catalog file");
        }
    }
    Ok(())
}

#[cfg(any(target_os = "macos", test))]
fn reject_symlink(path: &Path) -> Result<()> {
    if std::fs::symlink_metadata(path)
        .context("inspect MLX directory")?
        .file_type()
        .is_symlink()
    {
        bail!("unsafe MLX model directory");
    }
    Ok(())
}
#[cfg(any(target_os = "macos", test))]
fn reject_symlink_if_present(path: &Path) -> Result<()> {
    if path_present(path) {
        reject_symlink(path)?;
    }
    Ok(())
}

#[cfg(any(target_os = "macos", test))]
fn prepare_child_dir(parent: &Path, name: &str) -> Result<PathBuf> {
    if !safe_relative(name) {
        bail!("unsafe MLX model directory");
    }
    reject_symlink(parent)?;
    let child = parent.join(name);
    reject_symlink_if_present(&child)?;
    if !path_present(&child) {
        std::fs::create_dir(&child).context("prepare MLX model directory")?;
    }
    if !std::fs::symlink_metadata(&child)
        .context("inspect MLX directory")?
        .file_type()
        .is_dir()
    {
        bail!("unsafe MLX model directory");
    }
    let canonical_parent = std::fs::canonicalize(parent).context("inspect MLX directory")?;
    let canonical_child = std::fs::canonicalize(&child).context("inspect MLX directory")?;
    if canonical_child.parent() != Some(canonical_parent.as_path()) {
        bail!("unsafe MLX model directory");
    }
    Ok(child)
}

#[cfg(any(target_os = "macos", test))]
fn recover_invalid_snapshot(
    model_root: &Path,
    model: &CatalogModel,
    final_dir: &Path,
    staging: &Path,
) -> Result<()> {
    reject_symlink_if_present(final_dir)?;
    if !path_present(staging) {
        std::fs::rename(final_dir, staging).context("recover incomplete MLX snapshot")?;
        return Ok(());
    }
    reject_symlink_if_present(staging)?;
    for file in model.files {
        let source = final_dir.join(file.path);
        let target = staging.join(file.path);
        if file_matches(&source, file) && !file_matches(&target, file) {
            remove_exact_file_if_present(staging, &target)?;
            std::fs::rename(&source, &target).context("recover verified MLX file")?;
        }
        remove_exact_file_if_present(final_dir, &source)?;
        remove_exact_file_if_present(final_dir, &final_dir.join(format!("{}.part", file.path)))?;
    }
    remove_exact_file_if_present(final_dir, &final_dir.join(MARKER))?;
    if final_dir.parent() != Some(model_root) {
        bail!("unsafe MLX cleanup target");
    }
    std::fs::remove_dir(final_dir).context("remove invalid MLX snapshot")
}

#[cfg(any(target_os = "macos", test))]
fn remove_exact_file_if_present(root: &Path, file: &Path) -> Result<()> {
    if path_present(file) {
        remove_exact_file(root, file)?;
    }
    Ok(())
}
#[cfg(any(target_os = "macos", test))]
fn remove_exact_file(root: &Path, file: &Path) -> Result<()> {
    if !file.starts_with(root)
        || file.components().any(|c| matches!(c, Component::ParentDir))
        || std::fs::symlink_metadata(file).is_ok_and(|m| !m.file_type().is_file())
    {
        bail!("unsafe MLX cleanup target");
    }
    std::fs::remove_file(file).context("remove invalid MLX file")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn catalog_is_exact_role_filtered_and_valid() {
        assert_eq!(CATALOG.len(), 2);
        assert_eq!(
            catalog_for_role(ModelRole::Text)
                .iter()
                .map(|m| m.id)
                .collect::<Vec<_>>(),
            [DEFAULT_TEXT_MODEL]
        );
        assert_eq!(
            catalog_for_role(ModelRole::Vision)
                .iter()
                .map(|m| m.id)
                .collect::<Vec<_>>(),
            [DEFAULT_VISION_MODEL]
        );
        assert!(!catalog_model(DEFAULT_TEXT_MODEL).unwrap().supports_images);
        assert!(catalog_model(DEFAULT_VISION_MODEL).unwrap().supports_images);
        assert_eq!(
            CATALOG[0].revision,
            "2e92b640a63d47ad4dcf81a19a366b902356b3bc"
        );
        assert_eq!(CATALOG[0].license, "LFM Open License v1.0");
        assert_eq!(CATALOG[0].files.len(), 8);
        assert_eq!(
            CATALOG[1].revision,
            "674aaa7240b91e8012fcad5d791b7dfe5ba90207"
        );
        assert_eq!(CATALOG[1].license, "Apache-2.0");
        assert_eq!(CATALOG[1].files.len(), 10);
        for model in CATALOG {
            validate_catalog_model(model).unwrap();
            assert_eq!(
                model
                    .files
                    .iter()
                    .map(|file| file.sha256)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                model.files.len(),
                "catalog hashes must not be duplicated"
            );
        }
        assert_eq!(
            CATALOG[0].files.iter().map(|f| f.size).sum::<u64>(),
            4_851_993_338
        );
        assert_eq!(
            CATALOG[1].files.iter().map(|f| f.size).sum::<u64>(),
            1_749_079_691
        );
    }

    #[test]
    fn paths_are_fixed_below_managed_root() {
        let root = Path::new("managed");
        let path = snapshot_path_in(root, &CATALOG[0]);
        assert!(path.starts_with(root.join("models/mlx")));
        assert!(!path.to_string_lossy().contains(".."));
        let hostile = CatalogModel {
            id: "..\\outside",
            revision: CATALOG[0].revision,
            supports_images: false,
            license: "test",
            files: &[],
        };
        assert!(snapshot_path_in(root, &hostile).starts_with(root.join("models/mlx")));
        for bad in ["", "../x", "/absolute", "a/../../b", "a\\b"] {
            assert!(!safe_relative(bad), "{bad}");
        }
    }

    #[test]
    fn disk_math_parser_and_overflow_fail_closed() {
        assert_eq!(required_with_headroom(100), Some(110));
        assert_eq!(required_with_headroom(u64::MAX), None);
        assert_eq!(parse_df_available("Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/disk 10 2 8 20% /\n"), Some(8192));
        assert_eq!(parse_df_available("broken"), None);

        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("config.json.part"), b"oversized").unwrap();
        let model = CatalogModel {
            id: "test/model",
            revision: "cccccccccccccccccccccccccccccccccccccccc",
            supports_images: false,
            license: "test",
            files: &[CatalogFile {
                path: "config.json",
                size: 3,
                sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            }],
        };
        assert_eq!(remaining_bytes(&model, root.path()).unwrap(), 3);
    }

    #[test]
    fn tiny_install_resumes_verifies_and_publishes() {
        static FILES: &[CatalogFile] = &[CatalogFile {
            path: "config.json",
            size: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        }];
        let model = CatalogModel {
            id: "test/model",
            revision: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            supports_images: false,
            license: "test",
            files: FILES,
        };
        let root = tempfile::tempdir().unwrap();
        let staging = snapshot_path_in(root.path(), &model)
            .with_file_name(format!("{}.staging", model.revision));
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(staging.join("config.json.part"), b"a").unwrap();
        assert_eq!(remaining_bytes(&model, &staging).unwrap(), 2);
        let cancel = AtomicBool::new(false);
        let download =
            |_url: &str, path: &Path, _cancel: &AtomicBool, progress: &dyn Fn(u64)| -> Result<()> {
                assert_eq!(std::fs::read(path)?, b"a");
                std::fs::write(path, b"abc")?;
                progress(3);
                Ok(())
            };
        // Use the same engine with an injectable one-entry model.
        let final_dir =
            install_model_in(root.path(), &model, &cancel, &|_, _| {}, &download).unwrap();
        assert!(final_dir.join(MARKER).is_file());
        assert_eq!(
            std::fs::read(final_dir.join("config.json")).unwrap(),
            b"abc"
        );
    }

    #[test]
    fn cancel_preserves_partial_and_bad_download_removes_only_bad_file() {
        static FILES: &[CatalogFile] = &[CatalogFile {
            path: "config.json",
            size: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        }];
        let model = CatalogModel {
            id: "test/model",
            revision: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            supports_images: false,
            license: "test",
            files: FILES,
        };
        let root = tempfile::tempdir().unwrap();
        let staging = snapshot_path_in(root.path(), &model)
            .with_file_name(format!("{}.staging", model.revision));
        std::fs::create_dir_all(&staging).unwrap();
        let partial = staging.join("config.json.part");
        std::fs::write(&partial, b"a").unwrap();
        let cancel = AtomicBool::new(true);
        assert!(install_model_in(
            root.path(),
            &model,
            &cancel,
            &|_, _| {},
            &|_, _, _, _| unreachable!()
        )
        .is_err());
        assert_eq!(std::fs::read(&partial).unwrap(), b"a");

        cancel.store(false, Ordering::Release);
        let canary = staging.join("keep.txt");
        std::fs::write(&canary, b"keep").unwrap();
        assert!(install_model_in(
            root.path(),
            &model,
            &cancel,
            &|_, _| {},
            &|_, path, _, progress| {
                std::fs::write(path, b"bad")?;
                progress(3);
                Ok(())
            }
        )
        .is_err());
        assert!(!partial.exists());
        assert_eq!(std::fs::read(canary).unwrap(), b"keep");
    }

    #[test]
    fn complete_corrupt_partial_is_restarted_and_fast_marker_is_exact() {
        static FILES: &[CatalogFile] = &[CatalogFile {
            path: "config.json",
            size: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        }];
        let model = CatalogModel {
            id: "test/model",
            revision: "dddddddddddddddddddddddddddddddddddddddd",
            supports_images: false,
            license: "test",
            files: FILES,
        };
        let root = tempfile::tempdir().unwrap();
        let staging = snapshot_path_in(root.path(), &model)
            .with_file_name(format!("{}.staging", model.revision));
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(staging.join("config.json.part"), b"bad").unwrap();
        assert_eq!(remaining_bytes(&model, &staging).unwrap(), 3);
        let cancel = AtomicBool::new(false);
        let final_dir = install_model_in(
            root.path(),
            &model,
            &cancel,
            &|_, _| {},
            &|_, path, _, progress| {
                assert!(!path.exists(), "corrupt complete partial was removed");
                std::fs::write(path, b"abc")?;
                progress(3);
                Ok(())
            },
        )
        .unwrap();
        assert!(check_snapshot_fast(&model, &final_dir));
        assert!(check_snapshot_verified(&model, &final_dir));
        std::fs::write(final_dir.join("config.json"), b"abd").unwrap();
        assert!(check_snapshot_fast(&model, &final_dir));
        assert!(!check_snapshot_verified(&model, &final_dir));
        std::fs::write(final_dir.join(MARKER), b"wrong\nmarker\n").unwrap();
        assert!(!check_snapshot_fast(&model, &final_dir));
    }

    #[test]
    fn invalid_final_recovers_verified_files_without_broken_snapshots() {
        static FILES: &[CatalogFile] = &[CatalogFile {
            path: "config.json",
            size: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        }];
        let model = CatalogModel {
            id: "test/model",
            revision: "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            supports_images: false,
            license: "test",
            files: FILES,
        };
        let root = tempfile::tempdir().unwrap();
        let final_dir = snapshot_path_in(root.path(), &model);
        let staging = final_dir.with_file_name(format!("{}.staging", model.revision));
        std::fs::create_dir_all(&final_dir).unwrap();
        std::fs::write(final_dir.join("config.json"), b"abc").unwrap();
        std::fs::write(final_dir.join(MARKER), b"wrong\nmarker\n").unwrap();
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(staging.join("config.json"), b"bad").unwrap();

        let result = install_model_in(
            root.path(),
            &model,
            &AtomicBool::new(false),
            &|_, _| {},
            &|_, _, _, _| unreachable!("verified final file should be recovered"),
        )
        .unwrap();
        assert!(check_snapshot_verified(&model, &result));
        assert_eq!(std::fs::read(result.join("config.json")).unwrap(), b"abc");
        assert!(std::fs::read_dir(result.parent().unwrap())
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".broken-")));
    }
}
