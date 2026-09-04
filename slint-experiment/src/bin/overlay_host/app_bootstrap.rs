//! Host application preflight and bootstrap helpers.

use overlay_backend::paths::DataMigration;
use std::time::Duration;

pub struct AppBootstrap {
    pub is_relaunch: bool,
    pub _singleton: Option<slint_replay::native::lifecycle::SingletonGuard>,
    pub tokio_rt: tokio::runtime::Runtime,
    pub first_run: bool,
}

pub fn run_preflight() -> Result<Option<AppBootstrap>, slint::PlatformError> {
    let data_migration = overlay_backend::paths::migrate_data_root();
    slint_replay::logging::init();
    match &data_migration {
        DataMigration::Migrated => {
            log::info!("[overlay-host] data dir migrated: overlay-mvp -> suflyor");
        }
        DataMigration::Failed(e) => {
            log::warn!("[overlay-host] data dir migration failed (staying on overlay-mvp): {e}");
        }
        _ => {}
    }

    let is_relaunch = std::env::args().any(|a| a == "--relaunch");
    let wait_ms = if is_relaunch { 8_000 } else { 0 };
    let singleton = match slint_replay::native::lifecycle::acquire_singleton(wait_ms) {
        Ok(g) => {
            if is_relaunch {
                log::info!("[overlay-host] relaunch: parent exited, singleton acquired");
            }
            Some(g)
        }
        Err(e) => {
            log::warn!("[overlay-host] another instance is already running ({e}); exiting.");
            return Ok(None);
        }
    };

    match overlay_backend::recorder::recordings_dir()
        .and_then(|root| overlay_backend::recorder::repair_unfinalized_in(&root, Duration::ZERO))
    {
        Ok(0) => {}
        Ok(count) => log::info!("[overlay-host] repaired {count} crash-truncated recording(s)"),
        Err(error) => log::warn!("[overlay-host] startup recording repair failed: {error:#}"),
    }

    let tokio_rt = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("[overlay-host] tokio runtime init failed: {e}. AI calls disabled.");
            return Err(slint::PlatformError::Other(format!("tokio init: {e}")));
        }
    };

    let first_run = overlay_backend::config::config_path()
        .map(|p| !p.exists())
        .unwrap_or(false);

    Ok(Some(AppBootstrap {
        is_relaunch,
        _singleton: singleton,
        tokio_rt,
        first_run,
    }))
}
