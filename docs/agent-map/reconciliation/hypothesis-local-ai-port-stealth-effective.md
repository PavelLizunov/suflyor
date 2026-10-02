# Original local_ai C01 / window C11: bounded port reachability and effective stealth evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_local_ai_port_stealth_effective_confirmed.py) inspect frozen local AI port checking and window stealth state management logic. They do not run network probes, spawn local server processes, or manipulate display affinity. C01 and C11 remain confirmed mechanisms.

## C01 — `is_reachable` curl probe without `--fail` and port squatting vulnerability

In `overlay-backend/src/local_ai.rs`:
- In [is_reachable](<../../../overlay-backend/src/local_ai.rs#L912-L919>), reachability executes `curl -s -o /dev/null --max-time 2 <url>` and checks `o.status.success()`. Because `-f` (`--fail`) is omitted, curl returns exit status 0 on HTTP 404, 503, or foreign server responses;
- In [ensure_llama_serving](<../../../overlay-backend/src/local_ai.rs#L1417-L1426>), if `llama_reachable()` is true, it immediately returns `(ModelSwitch::Switched, Vec::new())` with no child handles, no process inspection, and no alias verification;
- In [ensure_servers_for_route](<../../../overlay-backend/src/local_ai.rs#L1958-L2005>), if `:8080/models` is reachable, the launch of `llama-server` is skipped completely.

## C11 — `stealth_supported` static return and `STEALTH_EFFECTIVE` bar-only scope

In `slint-experiment/src/win32.rs`:
- [stealth_supported](<../../../slint-experiment/src/win32.rs#L329-L331>) returns a compile-time `true` constant on Windows without querying the OS build number or verifying that the display driver supports `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` (introduced in Windows 10 build 2004).
In `slint-experiment/src/bin/overlay_host/window_lifecycle.rs`:
- [STEALTH_EFFECTIVE](<../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L78-L100>) records the verified capture-exclusion state of the overlay bar only;
- As explicitly documented in the module comments, per-window exclusion failures for tiles, settings, and other auxiliary windows are logged but not aggregated into `STEALTH_EFFECTIVE`, meaning tiles can remain visible to screen recorders even when `global_stealth_effective()` reads `true`.

## Limits

No rogue local HTTP servers were bound to port 8080, and no screen recordings were executed across multi-window setups. Original statuses in `candidates.json` remain `confirmed`.
