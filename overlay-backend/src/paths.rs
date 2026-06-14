//! Single source of truth for the per-user data directory + its one-time rename
//! from the legacy Tauri product name to the brand name.
//!
//! The app shipped for a long time writing its data (`config.json`, `sessions/`,
//! `recordings/`, `catalog.sqlite`, `overlay-host.log`) under
//! `%APPDATA%\overlay-mvp\` — the OLD Tauri product name, kept for back-compat
//! after the Slint rewrite. The brand is now "suflyor", so the dir name was
//! confusing/undiscoverable (a user hunting "suflyor" in %APPDATA% found
//! nothing). [`migrate_data_root`] renames it ONCE, at startup, atomically and
//! fail-safe; [`data_root`] is the resolver every data path goes through —
//! preferring the brand dir but falling back to the legacy one if the rename
//! hasn't happened (or couldn't), so a pre-migration or unmigratable install
//! never loses data.

use std::path::{Path, PathBuf};

/// Legacy Tauri product name — the old `%APPDATA%\overlay-mvp\` data dir.
const LEGACY_DIR: &str = "overlay-mvp";
/// Brand name — the new `%APPDATA%\suflyor\` data dir.
const BRAND_DIR: &str = "suflyor";

/// The per-user data root, e.g. `%APPDATA%\suflyor\`. Prefers the brand dir; only
/// falls back to the legacy `overlay-mvp\` when the brand dir is absent AND the
/// legacy one exists (a pre-migration / unmigratable install). Never points at
/// both. `None` only if the OS config dir can't be resolved.
#[must_use]
pub fn data_root() -> Option<PathBuf> {
    dirs::config_dir().map(|base| data_root_in(&base))
}

/// Pure resolver (test seam): given the platform config base, pick the data dir.
fn data_root_in(base: &Path) -> PathBuf {
    let brand = base.join(BRAND_DIR);
    // Brand wins if it exists (post-migration / fresh install). Use legacy ONLY
    // when the brand dir is absent and the legacy one is present.
    if !brand.exists() && base.join(LEGACY_DIR).exists() {
        base.join(LEGACY_DIR)
    } else {
        brand
    }
}

/// Outcome of [`migrate_data_root`], for one-line logging at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataMigration {
    /// Renamed `overlay-mvp` → `suflyor` this launch.
    Migrated,
    /// The brand dir already existed — nothing to do (prior migration / fresh).
    AlreadyDone,
    /// No legacy dir to move (fresh install / new user).
    FreshInstall,
    /// The OS config dir couldn't be resolved.
    NoConfigDir,
    /// The rename failed; the legacy dir is intact and still used, so no data is
    /// lost — retries next launch. Carries a short IO-error reason for the log
    /// (an error string, never file contents/secrets).
    Failed(String),
}

/// One-time, atomic, fail-safe rename of the data dir from the legacy
/// `overlay-mvp` name to the brand `suflyor`. Call ONCE at the very start of
/// `main()`, BEFORE logging or config touch the data dir.
///
/// - No-op when already migrated (`suflyor` exists) or a fresh install (no
///   `overlay-mvp`). NEVER writes to both dirs.
/// - `std::fs::rename` is atomic on the same volume (both live under
///   `%APPDATA%`), so there is no partial/torn state — it fully succeeds or
///   fully fails.
/// - On failure (e.g. a file in the dir is held open by another instance) the
///   legacy dir is left untouched and [`data_root`] keeps resolving to it, so no
///   data is ever lost; the migration simply retries on the next launch.
pub fn migrate_data_root() -> DataMigration {
    match dirs::config_dir() {
        Some(base) => migrate_in(&base),
        None => DataMigration::NoConfigDir,
    }
}

/// Pure migration step (test seam): operate on an explicit base dir.
fn migrate_in(base: &Path) -> DataMigration {
    let brand = base.join(BRAND_DIR);
    let legacy = base.join(LEGACY_DIR);
    if brand.exists() {
        return DataMigration::AlreadyDone; // never write both
    }
    if !legacy.exists() {
        return DataMigration::FreshInstall;
    }
    match std::fs::rename(&legacy, &brand) {
        Ok(()) => DataMigration::Migrated,
        Err(e) => DataMigration::Failed(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn data_root_in_prefers_brand_then_legacy_then_brand() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        // Neither exists yet → brand (a fresh install writes there).
        assert_eq!(data_root_in(base), base.join("suflyor"));
        // Only legacy exists → fall back to legacy (pre-migration).
        std::fs::create_dir_all(base.join("overlay-mvp")).unwrap();
        assert_eq!(data_root_in(base), base.join("overlay-mvp"));
        // Brand exists → brand wins even if legacy still lingers.
        std::fs::create_dir_all(base.join("suflyor")).unwrap();
        assert_eq!(data_root_in(base), base.join("suflyor"));
    }

    #[test]
    fn migrate_in_renames_legacy_to_brand_once_and_moves_contents() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        std::fs::create_dir_all(base.join("overlay-mvp")).unwrap();
        std::fs::write(base.join("overlay-mvp").join("config.json"), b"{}").unwrap();

        assert_eq!(migrate_in(base), DataMigration::Migrated);
        assert!(!base.join("overlay-mvp").exists(), "legacy dir is gone");
        assert!(
            base.join("suflyor").join("config.json").exists(),
            "data moved into the brand dir"
        );

        // Second call: brand exists → no-op, never recreates legacy.
        assert_eq!(migrate_in(base), DataMigration::AlreadyDone);
        assert!(!base.join("overlay-mvp").exists());
    }

    #[test]
    fn migrate_in_fresh_install_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(migrate_in(tmp.path()), DataMigration::FreshInstall);
        assert!(!tmp.path().join("suflyor").exists());
    }

    #[test]
    fn migrate_in_never_clobbers_when_brand_already_present() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        std::fs::create_dir_all(base.join("overlay-mvp")).unwrap();
        std::fs::create_dir_all(base.join("suflyor")).unwrap();
        // Brand present → AlreadyDone; legacy left as-is (we never merge/clobber).
        assert_eq!(migrate_in(base), DataMigration::AlreadyDone);
        assert!(base.join("overlay-mvp").exists());
        assert!(base.join("suflyor").exists());
    }
}
