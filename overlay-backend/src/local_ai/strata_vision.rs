//! Optional CPU image encoder for the original Qwen3.8-Flash-Next Strata model.
//! Download and activation are deliberately separate operations.

use super::*;

pub const STRATA_VISION_SIZE: u64 = 907_543_008;
const FILE: &str = "mmproj-Qwen3.8-Flash-Next-BF16.gguf";
const SHA256: &str = "b1a82259702816a5330d7bd7607cd9676b11780e79ff7348c21103ff3ce49bd0";
const URL: &str = "https://huggingface.co/ISTA-DASLab/Qwen3.8-Flash-Next-GSQ-RCO-GGUF/resolve/ed59f92082b1e93c0e96d60a8b11aab089b52f09/mmproj-Qwen3.8-Flash-Next-BF16.gguf";

struct Target {
    config: PathBuf,
    json: serde_json::Value,
    helper: PathBuf,
    model: PathBuf,
    projector: PathBuf,
}

fn config_path(dir: &Path, cfg: &serde_json::Value, value: &str) -> PathBuf {
    let cwd = PathBuf::from(cfg["cwd"].as_str().unwrap_or("."));
    let cwd = if cwd.is_absolute() {
        cwd
    } else {
        dir.join(cwd)
    };
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}

fn target_at(dir: &Path, config: &Path) -> Option<Target> {
    let json = parse_json_file(&std::fs::read_to_string(config).ok()?);
    let name = json["model_name"].as_str()?;
    // These are the original family's supported quantizations in the pinned
    // setup. In particular Coder and Qwen3.6 cannot use this image projector.
    let quant = name.strip_prefix("qwen3.8-flash-next-")?;
    if !matches!(quant, "q2_0" | "iq2_xs" | "iq3_xxs" | "iq3_s" | "iq4_xs") {
        return None;
    }
    let args = json["args"].as_array()?;
    let native = args.windows(2).find(|a| a[0] == "--native")?[1].as_str()?;
    let model = config_path(dir, &json, native);
    let engine = strata_engine_path(dir, &json)?;
    let helper = engine.parent()?.join(if cfg!(windows) {
        "strata-vision.exe"
    } else {
        "strata-vision"
    });
    if !model.is_file() || !helper.is_file() || !strata_python(dir).is_file() {
        return None;
    }
    let projector = model.parent()?.join(FILE);
    Some(Target {
        config: config.to_owned(),
        json,
        helper,
        model,
        projector,
    })
}

fn target(root: &Path) -> Option<Target> {
    let (dir, config) = active_location(root);
    target_at(&dir, &config)
}

#[must_use]
pub fn strata_vision_installable(root: &Path) -> bool {
    target(root).is_some()
}

#[must_use]
pub fn strata_vision_downloaded(root: &Path) -> bool {
    target(root).is_some_and(|t| super::super::file_len(&t.projector) == STRATA_VISION_SIZE)
}

fn configured_at(dir: &Path, target: &Target) -> bool {
    let vision = &target.json["vision"];
    ["exe", "model", "mmproj"].into_iter().all(|key| {
        vision[key]
            .as_str()
            .is_some_and(|value| config_path(dir, &target.json, value).is_file())
    })
}

/// Stat-only readiness for Settings. Activation verifies the pinned bytes and
/// sends a real image before declaring the route enabled.
#[must_use]
pub fn strata_vision_available(root: &Path) -> bool {
    let (dir, config) = active_location(root);
    target_at(&dir, &config).is_some_and(|t| configured_at(&dir, &t))
}

/// Download only: no run-config edits, restarts or route changes.
pub fn download_strata_vision(
    root: &Path,
    cancel: &AtomicBool,
    on: &dyn Fn(Progress),
) -> Result<()> {
    let t = target(root)
        .context("a compatible installed Strata model and image helper are required")?;
    super::super::bail_if_cancelled(cancel)?;
    if pinned_file_matches(&t.projector, STRATA_VISION_SIZE, SHA256) {
        return Ok(());
    }
    ensure_disk_space(
        t.projector.parent().context("model folder")?,
        STRATA_VISION_SIZE,
        on,
    )?;
    curl_resumable(
        URL,
        &t.projector,
        STRATA_VISION_SIZE,
        "Strata Vision",
        cancel,
        on,
    )?;
    super::super::verify_sha256(&t.projector, SHA256, "Strata Vision")
}

fn with_cpu_encoder(t: &Target) -> serde_json::Value {
    let mut json = t.json.clone();
    json["vision"] = serde_json::json!({
        "exe": t.helper,
        "model": t.model,
        "mmproj": t.projector,
        "gpu": false,
        "max_tokens": 1024,
        "threads": std::thread::available_parallelism().map_or(2, |n| n.get().min(4)),
    });
    // Write the flag explicitly for adopted versions predating server.py's
    // automatic injection. A CPU encoder needs no extra VRAM reserve.
    if let Some(args) = json["args"].as_array_mut() {
        if !args.iter().any(|a| a == "--vision") {
            args.push(serde_json::json!("--vision"));
        }
    }
    json
}

fn replace_config(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("suflyor-vision.tmp");
    std::fs::write(&tmp, bytes).context("write Strata Vision configuration")?;
    std::fs::rename(&tmp, path).context("replace Strata Vision configuration")
}

fn verify_image(cancel: &AtomicBool) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let request = crate::ai::complete(
            STRATA_BASE_URL, "", "strata",
            crate::vision::build_vision_request(crate::vision::SYNTHETIC_TEST_IMAGE_DATA_URL, "Reply with: ok"),
            16,
        );
        tokio::pin!(request);
        let deadline = tokio::time::sleep(Duration::from_secs(120));
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                result = &mut request => {
                    if result?.trim().is_empty() { bail!("Strata returned an empty image answer"); }
                    return Ok(());
                }
                _ = &mut deadline => bail!("Strata image verification timed out"),
                _ = tokio::time::sleep(Duration::from_millis(100)) => super::super::bail_if_cancelled(cancel)?,
            }
        }
    })
}

/// Call under the local-AI lifecycle lock. Never stops an external server.
/// Existing user Vision blocks are kept byte-for-byte, not replaced by defaults.
pub fn enable_strata_vision(
    root: &Path,
    cancel: &AtomicBool,
    on: &dyn Fn(Progress),
    commit: &dyn Fn() -> Result<()>,
) -> Result<()> {
    if crate::deep_lock::deep_lock_active() {
        bail!(crate::deep_lock::BLOCKED_ERROR);
    }
    let (dir, config) = active_location(root);
    let t = target_at(&dir, &config).context("compatible Strata installation missing")?;
    super::super::bail_if_cancelled(cancel)?;
    let owned = strata_running();
    let same = STRATA_CHILD_CONFIG
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .as_ref()
        == Some(&config);
    if (owned && !same) || (!owned && strata_reachable()) {
        bail!("Strata is managed by another process or installation");
    }
    // An already configured, running encoder needs only the image check.
    if configured_at(&dir, &t) && owned {
        verify_image(cancel)?;
        super::super::bail_if_cancelled(cancel)?;
        return commit();
    }
    let original = std::fs::read(&t.config)?;
    let changed = !configured_at(&dir, &t);
    if changed {
        // Preserve an incomplete user configuration rather than silently
        // replacing its GPU selection, projector or custom settings.
        if !t.json["vision"].is_null() {
            bail!("the existing Strata Vision configuration needs repair");
        }
        if super::super::file_len(&t.projector) != STRATA_VISION_SIZE {
            bail!("download and verify the Strata Vision encoder first");
        }
        super::super::verify_sha256(&t.projector, SHA256, "Strata Vision")?;
        if std::fs::read(&t.config)? != original {
            bail!("the Strata configuration changed during verification");
        }
        replace_config(
            &t.config,
            &serde_json::to_vec_pretty(&with_cpu_encoder(&t))?,
        )?;
    }
    stop_strata();
    let result = (|| {
        let outcome = start_strata(root, cancel, on);
        if outcome != StrataStart::Ready {
            bail!("Strata Vision start failed: {outcome:?}");
        }
        super::super::bail_if_cancelled(cancel)?;
        on(Progress::Step("Checking Strata image input".to_string()));
        verify_image(cancel)?;
        super::super::bail_if_cancelled(cancel)?;
        commit()
    })();
    if let Err(error) = result {
        stop_strata();
        if changed {
            replace_config(&t.config, &original)
                .context("restore previous Strata configuration")?;
        }
        if owned {
            on(Progress::Step(
                "Restoring the previous Strata server".to_string(),
            ));
            let restored = start_strata(root, &AtomicBool::new(false), on);
            if restored != StrataStart::Ready {
                bail!("{error:#}; previous Strata server could not restart: {restored:?}");
            }
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn fixture(dir: &Path) -> PathBuf {
        std::fs::create_dir_all(dir.join("engine")).unwrap();
        std::fs::create_dir_all(strata_python(dir).parent().unwrap()).unwrap();
        std::fs::write(strata_python(dir), []).unwrap();
        std::fs::write(dir.join("model.gguf"), b"synthetic").unwrap();
        std::fs::write(
            dir.join("engine").join(if cfg!(windows) {
                "strata-vision.exe"
            } else {
                "strata-vision"
            }),
            [],
        )
        .unwrap();
        let path = dir.join("strata-iq2_xs.json");
        std::fs::write(&path, serde_json::to_vec(&serde_json::json!({
            "exe":"engine/strata.exe", "cwd":".", "model_name":"qwen3.8-flash-next-iq2_xs",
            "args":["--native", "model.gguf", "--max-context", "8192"], "sampling":{"temperature":0.7}
        })).unwrap()).unwrap();
        path
    }

    #[test]
    fn cpu_encoder_preserves_main_model_args_and_custom_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = fixture(dir.path());
        let t = target_at(dir.path(), &path).unwrap();
        let before = std::fs::read(&path).unwrap();
        let updated = with_cpu_encoder(&t);
        assert_eq!(updated["sampling"], t.json["sampling"]);
        assert_eq!(updated["vision"]["gpu"], false);
        assert_eq!(updated["vision"]["max_tokens"], 1024);
        assert_eq!(
            updated["vision"]["model"],
            t.model.to_string_lossy().as_ref()
        );
        assert!(!updated["args"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a == "--vram-reserve-mib"));
        assert_eq!(
            updated["args"].as_array().unwrap().len(),
            t.json["args"].as_array().unwrap().len() + 1
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        replace_config(&path, &serde_json::to_vec(&updated).unwrap()).unwrap();
        replace_config(&path, &before).unwrap();
        assert_eq!(std::fs::read(path).unwrap(), before);
    }

    #[test]
    fn projector_is_not_attached_to_other_model_families_or_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = fixture(dir.path());
        let original = parse_json_file(&std::fs::read_to_string(&path).unwrap());
        for name in [
            "qwen3.6-35b-a3b",
            "qwen3.8-flash-next-coder-iq2_xs",
            "qwen3.5-0.8b",
            "swift-1.5-iq2_xs",
            "qwen3.8-flash-next-unsloth-ud-iq4_xs",
        ] {
            let mut json = original.clone();
            json["model_name"] = name.into();
            std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
            assert!(target_at(dir.path(), &path).is_none(), "{name}");
        }
        std::fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
        std::fs::remove_file(dir.path().join("model.gguf")).unwrap();
        assert!(target_at(dir.path(), &path).is_none());
    }
}
