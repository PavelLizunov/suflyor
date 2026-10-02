# Original config C01/C02: bounded credentials path data root poisoning and mask_host evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_config_credentials_path_mask_host_confirmed.py) inspect frozen POSIX credentials path creation and URL host masking logic. They do not write live user credentials or access network services. C01 and C02 remain confirmed mechanisms.

## C01 — credentials path directory creation and legacy data root orphaning

In `credentials.rs`, [credentials_path](<../../../overlay-backend/src/credentials.rs#L140-L146>) resolves the user config directory, joins the `"suflyor"` brand directory, and unconditionally calls `ensure_dir_permissions(&dir)` which executes `fs::create_dir_all(dir)`.
In `paths.rs`, [data_root_in](<../../../overlay-backend/src/paths.rs#L31-L37>) checks:
`if !brand.exists() && legacy.exists() { legacy } else { brand }`.
If an older installation has existing session journals, databases, and configuration under `overlay-mvp/`, calling `credentials_path()` creates an empty `suflyor/` directory, causing `data_root()` to permanently select the newly created brand path and silently bypass the legacy directory.

## C02 — `mask_host` authority masking vs query/fragment/path retention

In `config.rs`, [mask_host](<../../../overlay-backend/src/config.rs#L1868-L1928>) splits a URL into scheme, authority, and path/delimiter components.
While it strips userinfo (username and password) and replaces the hostname with `***`, it preserves the port suffix and concatenates the remainder verbatim via `format!("{scheme}***{port}{path}")`.
As a result, sensitive query parameters (e.g. `?api_key=...`), URL fragments (`#...`), and path segments are preserved unredacted in diagnostic outputs.

## Limits

No user configuration directories were mutated outside temporary test folders, and no live diagnostic logs were transmitted. Original statuses in `candidates.json` remain `confirmed`.
