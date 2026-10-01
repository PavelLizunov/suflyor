# Hermes bridge/plugin and profile preparation: source-linked contract

**Evidence:** principal source inspected at `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`; three existing mocked Python limit tests ran locally. No server bind, Hermes plugin installation/API/profile turn, private transcript read or native Settings test executed.

## Two directions and actual bind scope

[Settings wiring](../../../slint-experiment/src/bin/overlay_host/settings_hermes.rs#L1-L117) distinguishes Hermes → Suflyor artifact bridge and Suflyor → Hermes OpenAI-compatible profile-prep API. Bridge is off by default/token required; apply stops current handle then starts matching configured state. Return label and config save are heterogeneous/best effort, not confirmed persisted-state transaction.

[bridge::start](../../../overlay-backend/src/bridge.rs#L79-L168) defaults loopback but allows **configured non-loopback bind host**. Old module header promising strict loopback-only conflicts with implementation and [integration README](../../../integrations/hermes-plugin/README.md#L38-L53). Remote binding exposes bearer-gated private text/profiles on that interface; no TLS/transport encryption is implemented by tiny_http here. Trusted-network routing/firewall needs explicit platform/infrastructure acceptance, not auto-configured by this research.

[Token generation](../../../overlay-backend/src/bridge.rs#L97-L111) uses 16 OS-random bytes with time-based fallback if RNG fails. This fallback is not cryptographically equivalent entropy. [Authentication](../../../overlay-backend/src/bridge.rs#L33-L54) hashes supplied/expected header before fixed-size XOR comparison; [request shell](../../../overlay-backend/src/bridge.rs#L171-L224) authenticates before reading <=256KiB body and returns generic errors. Byte cap does not bound slow-reader time/serial accept-loop denial of service; stop joins request thread and can wait on current handling beyond 300ms poll tick.

## Artifact API and mutations

[dispatch](../../../overlay-backend/src/bridge.rs#L293-L535) implements nine tool-facing surfaces:

| Route | Actual response/side effect |
| --- | --- |
| GET health | Version/ok (still store open occurs before dispatch) |
| GET sessions | Newest catalog rows, limit capped100; list query loads all first |
| GET sessions/id | Catalog transcript, char cap up to500k; text only |
| GET sessions/id/summary | Latest catalog AI turn with purpose summary, **not** conspect file |
| GET search | FTS hits capped50; invalid FTS query returns generic400 |
| GET memory | All active default-profile approved items; no candidate auto-exposure |
| POST memory/suggest | Capped text candidate queued pending; not approved |
| GET profiles | Profile names/context and active name |
| POST profiles | In-memory upsert/optional active profile, save flag returned |

[Config-save shell](../../../overlay-backend/src/bridge.rs#L210-L224) logs save failure but still responds with dispatch success; profile upsert success is not proof persisted disk state. Summary from catalog can lag or omit a saved conspect/re-STT recap; bridge API and Archive saved-summary view use different stores. Authenticated GET health still opens `Store::open`, which can create catalog/run migrations; “read-mostly” is not a guarantee that every GET is filesystem-read-only. No audio/pixel routes implemented in this reviewed dispatch, but authorized text/profiles are still sensitive.

## Plugin client and safe limits

[Python client](../../../integrations/hermes-plugin/suflyor/__init__.py#L24-L65) reads URL/token from env at call time, adds bearer, bypasses proxies and uses 15s urllib timeout, returning generic bridge errors for common HTTP/network failures. Configured URL is not restricted to loopback in client; token can be sent to chosen endpoint. Whole-response read lacks separate byte cap; `_h_transcript` has weaker int conversion than the robust list limits.

[List limit normalizer](../../../integrations/hermes-plugin/suflyor/__init__.py#L72-L80) defaults10/clamps1..50 and rejects bool/non-numeric; [handlers](../../../integrations/hermes-plugin/suflyor/__init__.py#L86-L195) adapt transcripts, summaries, memory/profiles. [plugin declaration](../../../integrations/hermes-plugin/suflyor/plugin.yaml) lists nine tools; registration failure/Hermes API compatibility requires actual plugin host. Existing [test_limits](../../../integrations/hermes-plugin/tests/test_limits.py) passed three tests with request mocked (nine input cases each), no actual network/user data accessed.

## Embedded installer and profile-prep

[hermes_install](../../../overlay-backend/src/hermes_install.rs#L22-L89) embeds plugin files into production and installs plugin/env/config by user Settings action. [Home/permission/write](../../../overlay-backend/src/hermes_install.rs#L91-L165) resolves Hermes home, restricts POSIX secret files and uses temp+rename; `.bak` can retain prior credentials and path ownership/symlink/linebreak risks need scope-specific tests. [dotenv merge](../../../overlay-backend/src/hermes_install.rs#L419-L449) replaces only known assignment lines and preserves other text; not a full dotenv/YAML parser. Unsupported YAML shapes return manual hint rather than silent general reconstruction.

[Settings API test/prep](../../../slint-experiment/src/bin/overlay_host/settings_hermes.rs#L280-L417) calls configured Hermes API with hardcoded model label `hermes-agent` off-thread, longer completion budget and profile prompt. Profile result is upserted/made active and live meeting_context set; save result ignored before “готово”. This is profile-preparation path, not a special `ai_provider="hermes"` branch in Config. Users can configure generic bridge endpoint for live answers separately.

## Remaining acceptance

[Bridge inline tests](../../../overlay-backend/src/bridge.rs#L546-L750), [installer tests](../../../overlay-backend/src/hermes_install.rs#L622-L873) and [backend plugin smoke](../../../overlay-backend/tests/hermes_plugin_smoke.rs) were not Cargo-executed. Exact-SHA native/network acceptance requires invalid/blank token, bind defaults/non-loopback, slow/body/response bounds, store-unavailable health, summary store consistency, suggestion approval-only, persistence failure before success, conservative config upgrade, plugin register and safe secret/screenshots. No configured service/provider/credentials changed here.
