//! Engine-only updates through Strata's signed component updater.
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use super::{
    active_location, parse_json_file, spawn_owned, start_strata, stop_strata,
    strata_engine_path, strata_python, strata_reachable, strata_running,
    terminate_child_tree, Progress, StrataStart,
};

#[must_use]
pub fn strata_update_available(root: &Path) -> bool {
    let (dir, _) = active_location(root);
    dir.join("strata.py").is_file() && dir.join("strata_cli/update_cmd.py").is_file()
}

#[must_use]
pub fn strata_engine_version(root: &Path) -> Option<String> {
    let (dir, config) = active_location(root);
    let cfg = parse_json_file(&std::fs::read_to_string(config).ok()?);
    let engine = strata_engine_path(&dir, &cfg)?;
    let manifest = parse_json_file(
        &std::fs::read_to_string(engine.parent()?.join("engine-component.json")).ok()?,
    );
    manifest["version"].as_str().map(str::to_owned)
}

pub struct StrataUpdate {
    pub version: String,
    pub update_available: bool,
}

/// Callers serialize engine work with the shared lifecycle lock. No model
/// installer is called, and the official signature and test-start checks stay on.
pub fn update_strata_engine(
    root: &Path,
    check_only: bool,
    cancel: &AtomicBool,
    on: &dyn Fn(Progress),
) -> Result<StrataUpdate> {
    if !strata_update_available(root) {
        bail!("This Strata installation has no component updater");
    }
    if cancel.load(Ordering::Relaxed) {
        bail!("Strata update cancelled");
    }
    let owned = !check_only && strata_running();
    if !check_only && !owned && strata_reachable() {
        bail!("Stop the externally started Strata before updating its engine");
    }
    if owned {
        on(Progress::Step("Stopping Strata server for update".into()));
        stop_strata();
    }
    let (dir, config) = active_location(root);
    let result = run_updater(&dir, &config, check_only, cancel, on);
    // Restore a previously running server after success, failure or cancellation.
    // A lock activated during the update still takes precedence over restart.
    if owned && !crate::deep_lock::deep_lock_active() {
        let restart = start_strata(root, &AtomicBool::new(false), on);
        if restart != StrataStart::Ready {
            log::warn!("Strata update: restart failed: {restart:?}");
            bail!("Strata engine update finished but the server could not restart");
        }
    }
    result
}

fn stop_updater(mut child: Child) {
    #[cfg(unix)]
    {
        // Python's KeyboardInterrupt runs the official updater's finally block,
        // which stops its isolated test server before removing the staging tree.
        unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT); }
        let deadline = Instant::now() + Duration::from_secs(65);
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                Err(_) => break,
            }
        }
        unsafe { libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL); }
    }
    terminate_child_tree(child);
}

fn run_updater(
    dir: &Path,
    config: &Path,
    check_only: bool,
    cancel: &AtomicBool,
    on: &dyn Fn(Progress),
) -> Result<StrataUpdate> {
    on(Progress::Step(if check_only {
        "Checking Strata engine updates"
    } else {
        "Updating Strata engine"
    }.into()));
    let output = dir.join(format!("suflyor-update-{}.json", std::process::id()));
    let mut cmd = Command::new(strata_python(dir));
    cmd.arg(dir.join("strata.py"))
        .args(["update", "--no-control", "--model-config"])
        .arg(config)
        .args(["--output", "json"]);
    if check_only { cmd.arg("--check"); }
    cmd.current_dir(dir)
        .env("PYTHONUNBUFFERED", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .stdin(Stdio::null())
        .stdout(std::fs::File::create(&output).context("updater output")?)
        .stderr(std::fs::File::create(dir.join("suflyor-update.log")).context("updater log")?);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    crate::download::no_window(&mut cmd);
    let mut child = spawn_owned(&mut cmd)?;
    let started = Instant::now();
    let budget = Duration::from_secs(if check_only { 60 } else { 900 });
    let result = loop {
        if cancel.load(Ordering::Relaxed) || started.elapsed() > budget {
            stop_updater(child);
            break Err(anyhow::anyhow!("Strata update cancelled or timed out"));
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let json = std::fs::read_to_string(&output)
                    .map(|text| parse_json_file(&text))
                    .unwrap_or_default();
                break parse_report(status.success(), &json, check_only);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(error) => {
                stop_updater(child);
                break Err(error.into());
            }
        }
    };
    let _ = std::fs::remove_file(output);
    result
}

fn parse_report(success: bool, report: &serde_json::Value, check: bool) -> Result<StrataUpdate> {
    if !success || report["ok"].as_bool() != Some(true) {
        log::warn!("Strata updater failed: {}", report["error"]);
        bail!("Strata engine update failed; see suflyor-update.log");
    }
    let engine = if check {
        &report["engine"]
    } else {
        report["results"].as_array()
            .and_then(|rows| rows.iter().find(|row| row["component"] == "engine"))
            .context("Strata updater returned no engine result")?
    };
    let status = engine["status"].as_str().unwrap_or_default();
    if !matches!(status, "available" | "install" | "current" | "updated") {
        bail!("Strata update has no compatible engine: {status}");
    }
    let version = if check {
        engine.pointer(if status == "current" { "/installed/version" } else { "/offered/version" })
    } else {
        engine.pointer("/installed_after/version").or_else(|| engine.pointer("/installed_before/version"))
    }.and_then(serde_json::Value::as_str).context("Strata engine version missing")?;
    Ok(StrataUpdate {
        version: version.into(),
        update_available: matches!(status, "available" | "install"),
    })
}
