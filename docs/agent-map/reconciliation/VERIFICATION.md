# Reconciliation verification evidence

## Round 29 — original CI C08/C10 cleanup and workflow gates (covering SHA pending)

- [Evidence](hypothesis-ci-cleanup-gate.md)/[receipt](hypothesis-ci-cleanup-gate.json): six source fixtures target 242 research tests. Recursive uninstall cleanup has no reparse guard. The required `gate` job depends only on `changes` and `rust`; macOS and validate are separate.
- The workflow calls an external docs classifier and does not interpolate PR title/body into shell. No Actions run, branch-protection inspection, installer, or deletion was performed. C08/C10 remain hypotheses; exact archive repeat pending.

## Round 28 — original CI C06/C07 installer boundaries (exact covering SHA tested)

- [Evidence](hypothesis-installer-boundaries.md)/[receipt](hypothesis-installer-boundaries.json): six source fixtures target 236 research tests. Installer is per-user, exposes a directory page, and has no signature, ACL, or overwrite policy. `$INSTDIR` is interpolated inside the stop command without a NSIS quote escape.
- Exact **`bc08b3926e057d08b245cb83b15fb8daee9e7e4b`** archive passed **236 research +3 mocked Hermes, zero skips**, checkpoint issues empty (26/709), navigation ledger byte-identical SHA256 `b7cc3c4bb49cd36218e8c454c9a100b8796ef5c1a0cd7229965a9a17950764b2`. [Portable installer receipt](portable-recovery-installer-boundaries.json). C06/C07 remain hypotheses; no makensis, process stop, or deletion.

## Round 27 — original CI C04/C05 classification boundaries (exact covering SHA tested)

- [Evidence](hypothesis-gate-classification.md)/[receipt](hypothesis-gate-classification.json): six temporary Git/source fixtures target 230 research tests. Missing `origin/master` narrows the model to `HEAD~1`; that range classifies as docs while the full range sees the preceding Rust file and classifies as targeted.
- Exact **`386392e244067e61c6fc367b02e68606cf5efe31`** archive passed **230 research +3 mocked Hermes, zero skips**, checkpoint issues empty (26/709), navigation ledger byte-identical SHA256 `b7cc3c4bb49cd36218e8c454c9a100b8796ef5c1a0cd7229965a9a17950764b2`. [Portable gate receipt](portable-recovery-gate-classification.json). C04/C05 remain hypotheses; no PowerShell gate, hook, or Actions acceptance.

## Round 26 — confirmed persistence C01/C16 backup and recovery (exact covering SHA tested)

- [Evidence](hypothesis-catalog-backup-recovery.md)/[receipt](hypothesis-catalog-backup-recovery.json): six temporary SQLite/JSONL fixtures target 224 research tests. Raw main-file backup leaves no WAL sidecar. Recovery accepts start-only input but rejects stop, summary, and age past 12 hours.
- Exact **`1f2bb2154a47e2d2bc47f5d1fffa21c0c00c4a21`** archive passed **224 research +3 mocked Hermes, zero skips**, checkpoint issues empty (26/709), navigation ledger byte-identical SHA256 `b7cc3c4bb49cd36218e8c454c9a100b8796ef5c1a0cd7229965a9a17950764b2`. [Portable backup receipt](portable-recovery-catalog-backup-recovery.json). C01 and C16 were already confirmed; no owner migration or Rust scan.

## Round 25 — persistence C03/C17 retention and UTF-8 (exact covering SHA tested)

- [Evidence](hypothesis-journal-retention-utf8.md)/[receipt](hypothesis-journal-retention-utf8.json): six temporary-file/source fixtures target 218 research tests. Whole-file UTF-8 decoding fails before line parsing; the first valid physical line still parses alone.
- Exact **`3ad53ba7fc7c01ba2b7456cbeccec4aae0d0bb05`** archive passed **218 research +3 mocked Hermes, zero skips**, checkpoint issues empty (26/709), navigation ledger byte-identical SHA256 `b7cc3c4bb49cd36218e8c454c9a100b8796ef5c1a0cd7229965a9a17950764b2`. [Portable retention receipt](portable-recovery-journal-retention-utf8.json). C03 remains confirmed and C17 remains a hypothesis; no Rust retention/independent acceptance.

## Round 24 — original persistence C14/C15 schema boundaries (exact covering SHA tested)

- [Evidence](hypothesis-memory-migrations.md)/[receipt](hypothesis-memory-migrations.json): six temporary schema/source fixtures target 212 research tests. Candidate status accepts a non-contract value; one pending snapshot followed by two write/insert pairs mints two items, while rechecking pending blocks the second.
- Exact **`4719c51725cf6cf40e70b7f916edf5440864f3f0`** archive passed **212 research +3 mocked Hermes, zero skips**, checkpoint issues empty (26/709), navigation ledger byte-identical SHA256 `b7cc3c4bb49cd36218e8c454c9a100b8796ef5c1a0cd7229965a9a17950764b2`. [Portable memory receipt](portable-recovery-memory-migrations.json). C14/C15 remain hypotheses; no Rust runner/concurrency/independent acceptance.

## Round 23 — confirmed persistence C10/C12 side tables (exact covering SHA tested)

- [Evidence](hypothesis-catalog-side-tables.md)/[receipt](hypothesis-catalog-side-tables.json): six temporary schema/source fixtures target 206 research tests. Projection replacement cascades utterances but retains diarization and memory; hard delete removes diarization only. Index match handles four journal kinds, leaving other journal variants unprojected.
- Exact **`714d4f18235ba4e2f43b22e3abb4071b81691793`** archive passed **206 research +3 mocked Hermes, zero skips**, checkpoint issues empty (26/709), navigation ledger byte-identical SHA256 `b7cc3c4bb49cd36218e8c454c9a100b8796ef5c1a0cd7229965a9a17950764b2`. [Portable side-table receipt](portable-recovery-catalog-side-tables.json). Existing confirmed statuses remain unchanged; no rusqlite/owner-catalog/independent acceptance.

## Round 22 — original persistence C08/C09 projection (exact covering SHA tested)

- [Evidence](hypothesis-journal-projection.md)/[receipt](hypothesis-journal-projection.json): six temporary SQLite/source fixtures target 200 research tests. Finalized skip-set excludes crashed rows; missing stop heals after a later stop. Direct projection keeps `session_start.ai_model`; shipped backfill replaces it from turns or clears it when turnless.
- Exact **`1514c9844a33998581f8a9e4e8a6a030a30e44b3`** archive passed **200 research +3 mocked Hermes, zero skips**, checkpoint issues empty (26/709), navigation ledger byte-identical SHA256 `b7cc3c4bb49cd36218e8c454c9a100b8796ef5c1a0cd7229965a9a17950764b2`. [Portable projection receipt](portable-recovery-journal-projection.json). `reindex_default` calls backfill after indexing, but C08/C09 remain hypotheses; no Rust indexer/live race/independent acceptance.

## Round 21 — original persistence C05/C06 journal identity (exact covering SHA tested)

- [Evidence](hypothesis-journal-write-identity.md)/[receipt](hypothesis-journal-write-identity.json): six temporary JSONL/source fixtures target 194 research tests. Writer continues after an error and latches the first one; session open is append, not exclusive; suffix is one-second stamp plus low 24 bits.
- A torn JSON value consumes the next physical line, so later valid JSON survives but the tear is not two independent skips. Dual append handles preserve both dummy sessions without stable one-line interleaving. No owner journal, Rust writer, ENOSPC, or same-millisecond process race.
- Exact **`01f91c94a6152eeff9979dfb35f91fd34819f381`** archive passed **194 research +3 mocked Hermes, zero skips**, checkpoint issues empty (26/709), navigation ledger byte-identical SHA256 `b7cc3c4bb49cd36218e8c454c9a100b8796ef5c1a0cd7229965a9a17950764b2`. [Portable journal receipt](portable-recovery-journal-identity.json). Original C05/C06 remain hypotheses; no Rust writer/ENOSPC/process-race/independent acceptance.

## Round 20 — original persistence C02/C18 actual SQLite fixtures (exact covering SHA tested)

- [Evidence](hypothesis-sqlite-contention.md)/[engine/migration receipt](hypothesis-sqlite-contention.json): stdlib SQLite3.45.1, shipped six migrations, private dummy temp DBs. Six new engine/source fixtures target188 research total; no owner catalog/native rusqlite/UI/indexer/DB repair/SDK/Rust action.
- Actual engine: stale deferred read→write upgrade after other commit BUSY_SNAPSHOT517, write-first held lock BUSY5, WAL reader old snapshot doesn't block writer. Default autocheckpoint1000 pages; reader-pinned truncate busy, after rollback truncate0/0/0+zero WAL. 20ms fixture timeout explicitly not source2000ms; no benchmark/unbounded-WAL/product fault claim.
- C02 approve SELECT-before-write versus replace/delete write-first preconditions and C18 automatic checkpoint counterevidence preserved; original register hash/IDs/statuses unchanged39/75/5. Feature registry26/709/navigation unchanged (no interval-credit inflation).
- Exact **`e9612309c4936f1ebfafd91b03a898f6034b388c`** archive passed **188 research +3 mocked Hermes, zero skips**, frozen checkpoint issues empty (26/709), navigation validator zero errors and byte-identical ledger SHA256 `b7cc3c4bb49cd36218e8c454c9a100b8796ef5c1a0cd7229965a9a17950764b2`. [Portable SQLite receipt](portable-recovery-sqlite-contention.json); stdlib engine is not bundled rusqlite/native UI/independent/full-objective acceptance.


## Round 19 — audio route/settings/metrics/watchdog (exact covering SHA tested)

- [Contract](../features/audio-route-settings-and-watchdog.md) adds24 ranges over COM endpoint policy, Windows WASAPI retry/drop/nojoin, Settings clone-save-commit versus nonWindows path, macOS metrics/watchdog stop-intent/lifecycle. Registry26/709; no native device/COM/audio/config/Rust/UI/process restart action.
- Six source fixtures target182 research total. Names first-match/dedupe not stable endpoint ids; null default-id notifications dropped; retries fixed1s unbounded, event silence waits not restart. Settings persist candidate before in-memory commit. macOS successful-enqueue counter can stall on downstream full queue, never-flowed streams not expected; watchdog one-shot stop~5 ticks, no autorestart. Mutex metrics snapshots not atomic pair/RT callback/health-age synonym.
- Rust policy/Settings/static guard tests read not executed; original119 statuses39/75/5 unchanged. Navigation162 some pointers/136 none, union39,935 **not audited semantics**.
- Exact **`e5a633c973ad45ffd9609697a339b009213469b7`** archive passed **182 research +3 mocked Hermes, zero skips**, frozen checkpoint issues empty (26/709), navigation validator zero errors and byte-identical ledger SHA256 `b7cc3c4bb49cd36218e8c454c9a100b8796ef5c1a0cd7229965a9a17950764b2`. [Portable audio-route receipt](portable-recovery-audio-route.json); no device/config/COM/native/Rust/independent/full-objective acceptance.


## Round 18 — archive UI confirmation/snapshot/latch (exact covering SHA tested)

- [Contract](../features/archive-ui-confirmations-and-latches.md) adds24 ranges over Slint row/search/rename/modal/progress keyboard and host async list/sync query/index-confirm/global guard. Registry25/685. Corrected old task-owned archive contract list cap100→actual300, no source mutation.
- Six new fixtures target176 research total. Pure row-index refresh model shows potential confirm title/current idx mismatch, not native wrong delete repro; query resets rename only, async initial replacement not modal-generation-gated. Process guard prevents double work but fresh window UI busy false; progress status rendered only while busy, post-completion visibility unclear. Captured active/language/recording/cloud flags versus later job Config documented.
- No user DB/journal/audio/rename/delete/retranscription/provider/native/Rust action. Original119 statuses39/75/5 unchanged; navigation156 some refs/142 none, union38,545 **not audited semantics**.
- Exact **`8426a6b04f4b2643dfd5103cd76effad5115c724`** archive passed **176 research +3 mocked Hermes, zero skips**, frozen checkpoint issues empty (25/685), navigation validator zero errors and byte-identical ledger SHA256 `b55baa48437c26427271f0da974c63bc961cf76019c23cb35317c15fcd343d3b`. [Portable archive UI receipt](portable-recovery-archive-ui.json); no DB/destructive/native/live UI/independent/full-objective acceptance.


## Round 17 — overlay/tile state and stream terminal preconditions (exact covering SHA tested)

- [Contract](../features/overlay-tile-state-and-stream-terminals.md) adds26 ranges: actual Slint busy/reset/action bindings, bar pulse, shared-generation check/slot/closure, PTT fixed sink, close/registry/cap and backend EOF. Registry24/661; no Slint UI/native/Rust/provider execution.
- Six new fixtures target170 research total: source order plus **pure** model old gate-pass→new generation/slot→old handler wrong-slot candidate. Not actual Rust scheduling/race repro; original tile-C01 remains hypothesis. C03 EOF does not synthesize terminal, producer prerequisite still open; explicit terminal/install reset/missing history/MLX clear counterevidence, PTT error no busy clear documented. Original39/75/5 unchanged.
- Navigation155 some refs/143 none, union37,881 line pointers **not audited semantics**. Default properties/comments not evidence every callback wired/live/UI accepted.
- Exact **`e152dab4dc19fd4430b1e5d1fa57143a72d90961`** archive passed **170 research +3 mocked Hermes, zero skips**, frozen checkpoint issues empty (24/661), navigation validator zero errors and byte-identical ledger SHA256 `44ce61e5d9891e430a1c9d0aa9f4d48e46e3c718014efd8540b172d70a5963e8`. [Portable overlay/tile receipt](portable-recovery-overlay-tile.json); source/model not Slint/native/Rust race/independent/full-objective acceptance.


## Round 16 — Tera graph/text/cancellation source semantics (exact covering SHA tested)

- [Contract](../features/tera-graph-text-and-cancellation.md) adds 31 ranges: runtime marker/size vs installer digest, graph output name/shape/window, normalizer/tag/indexer/NPY, duration/work bounds, generation/controller and Rust mock/helper test intent. Registry23/635, no model/ORT/Rust/audio/licence/native execution.
- Six source fixtures; target local research164. Explicit boundaries: SynthOutput accumulates vocoder chunks before worker event; generation checked before job/discards late result, no active ORT abort. Single long word escapes120char target; duration latent alloc and NPY product unchecked; nested spans/normalization preconditions source-only/unreproduced. Runtime size isn't rehash.
- Navigation updated153 some precise refs/145 none, union36,707 lines **not semantic audited lines**. Original119 statuses39/75/5 unchanged, independent/native/full semantics absent.
- Exact **`8a9508ee90eb0a32ac99b30d445951d1890de096`** archive passed **164 research +3 mocked Hermes, zero skips**, frozen checkpoint issues empty (23/635), navigation validator zero errors and byte-identical ledger SHA256 `0a114f90a5af5983efeccc8d2bd143dbc1c2812028b5e0430b9ad0434187e8a0`. [Portable Tera receipt](portable-recovery-tera.json); no ORT/model/Rust/native/independent/full-objective acceptance.


## Round 15 — WSOLA streaming/playback source and test intent (exact covering SHA tested)

- [Contract](../features/wsola-streaming-and-playback.md) adds 27 precise source ranges over WSOLA geometry/correlation/output/error/allocation, stream tails, four transport callers and separate transcript adapter; registry 22/604. No Rust/audio/listening/benchmark/model/SDK execution.
- Narrowed assertions: process_into_no_grow gates main output storage, not FFT/correlation vector/plan growth; actual live wrappers allocate Vec output/chunk tails. Rust ratio1 speech test proves repeat-output determinism/length, not output==input bit identity. Transport error raw-fresh fallback differs from transcript empty-chunk fallback; macOS callback mutex/queue not lock-free acceptance.
- Six new source tests; total local research target 158, runtime-dependent full repeat pending. Navigation updated 146 some precise references/152 none, line union35,233 remains **not audited semantics**. Original 39/75/5 statuses unchanged.
- Exact **`2b630b649b93a510d80b9b52727e73ce08adea16`** archive passed **158 research +3 mocked Hermes, zero skips**, frozen checkpoint issues empty (22/604), navigation validator zero errors and deterministic byte-identical ledger SHA256 `4f02e231aeb8f7aac8a90811349d0a0a0e9d26c4661521492ad22539d4c6fa1e`. [Portable WSOLA receipt](portable-recovery-wsola.json); Rust DSP tests/native/quality/independent/full-objective acceptance not established.


## Round 14 — evidence integrity/navigation gaps (exact covering SHA tested)

- Read-only exact historical commit receipt check: 14 receipts/11 artifact hash+size checks, zero mismatches, four absent legacy boundary flags explicitly retained not accepted. No historical tests rerun by integrity utility; [report](portable-receipt-integrity.json).
- [Navigation ledger](NAVIGATION-GAPS.md): 298 selected syntax files/120,858 source lines, 137 with some precise registered ranges/161 none, 33,524 reference-line union, 26 unranged/directory pointers zero credit. **No semantically reviewed-line number/percentage established**. Input source/registry hashes and intervals verified.
- Eight new ledger/receipt bookkeeping fixtures; local research total 152, parser/runtime-dependent full suite verification pending. No production/native/secret/process action, original 39/75/5 statuses unchanged; ledger prioritizes missing real source chains rather than broadening ranges to inflate coverage.
- Exact **`b8bb920063162585e62a4598cf20719d84660147`** archive passed **152 research +3 mocked Hermes, zero skips**, checkpoint issues empty (21/577), navigation validator zero errors and deterministic byte-identical ledger SHA256 `a2fe17fba52909bd90a0b47f52320a06f608ee74590abc9fa804507b5c79e675`. [Portable gap receipt](portable-recovery-navigation-gaps.json). Historical Git receipt check performed in checkout (archive lacks Git objects); no semantic/independent/native/full acceptance.


## Round 13 — credentials/backend child ownership (exact covering SHA tested)

- Five frozen backend SDK sources yield 28 imports/17 name-call candidates, no symbol/cfg/type resolution; host inventory unchanged. [Backend census](../native/backend-sdk-name-edges.md)/[storage and child contract](../features/credentials-and-managed-process-ownership.md); registry 21/577 ranges.
- Six new source assertions; research total 144. Direct keys via credential slot/callback rather than Config; Windows temporary write blob zeroed/CredFree after UTF8 Result, Unix plaintext mode0700/0600/temp-write+flush/no sync_all/rename. No live secrets/read/write tests. Storage concurrency/durability/API errors remain unaccepted.
- JobObject attach exists but unit-return/best-effort, zero failure cached; limit failure handle not explicitly closed. TTS broken-pipe drops Child without explicit wait/kill; Piper+Tera EOF Shutdown counterevidence, Nemotron ChildGuard kill/wait. Original TTS-C03/local_ai-C10 source hypotheses unchanged, no forced-parent-exit/native process repro.
- Exact **`6853a54cad6f86e9397c7e72de0b75fb98775f2a`** archive passed **144 research +3 mocked Hermes, zero skips**, frozen checkpoint issues empty (21 contracts/577 ranges), backend inventory validator zero errors and byte-identical generation SHA256 `0dfe1cc03ed6c03fe5a6f2fc8529f6e56876d47f737a7a70db8609927d6cf709`. [Portable receipt](portable-recovery-backend-sdk.json). Host SDK unchanged; no production/global/secret/process/model changes/native independent acceptance, 39/75/5 retained.


## Round 12 — selected Windows adapter ownership/SDK candidates (exact covering SHA tested)

- Four frozen adapter files yield 180 SDK import records/121 direct/qualified call syntax candidates, every symbol unresolved; not all SDK/backend/cfg/indirect graph. [Census](../native/windows-sdk-name-edges.md)/[ownership contract](../features/windows-capture-tray-and-sdk-ownership.md); 20 contracts/553 source ranges.
- Seven new fixtures plus previous suites: local research total 138; no Win32/API/input/clipboard/window/capture/Cargo/SDK/gate execution. Source checks verify GDI deselect/ordinary cleanup, partial scanline acceptance, hide/restore before error, WDA readback, tray TLS/lifetime/install failures. Native fault/UI/cfg/type/independent acceptance open.
- Coordinator corrected draft thread-marker assumption: Windows TrayHandle has HWND/TLS UI-thread contract, no macOS-style Rc PhantomData marker. GDI zero/nonzero lines check is not full-height guarantee; Result cleanup doesn't protect allocation panic paths. No original status promotion.
- Exact **`48dbd31238b0c7c4ca123ffbc8e6e30c4f1091f0`** archive passed **138 research +3 mocked Hermes, zero skips**, frozen checkpoint issues empty (20 contracts/553 references), SDK-name validator errors empty; inventory regenerated **byte-identically**, SHA256 `e869041507a34d705a44f189ad8f6d7f1889891ca63e83a237811cc1e06a0e06`. [Portable receipt](portable-recovery-windows-sdk.json); original claims 39/75/5, no native/independent/type graph acceptance.


## Round 11 — Config/UI name edges and selected save chains (exact covering SHA tested)

- Frozen 175 Rust inputs/schema hash validated; 2,951 unresolved name candidates (1,232 Config members/1,719 Slint method names); 176 UI matches ambiguous across components. No type/cfg/alias graph proof. [Name inventory](../schema/config-ui-name-edges.md), [four manual chains](../features/ui-setting-consumers-and-save-boundaries.md).
- Seven new fixtures; local research suite now 131 tests, supplied parser/runtime/PO dependencies required for zero skips. Config memory mutation precedes save for scheme/opacity; return before globals/live on error, no rollback. Monitor runtime applied before save; language live selection before memory/save. Source order only, no fault/native UI acceptance.
- Zero selected member candidate for auto_export_on_quit; targeted Rust source search found declaration/default only. Not universal dead-state proof or deletion recommendation. Original claim counts unchanged; registry 19 contracts/535 ranges.
- Exact **`5b27000cd2252c5e41957fe1d6adf629958fa7de`** archive passed **131 research +3 mocked Hermes tests, zero skips**; frozen checkpoint issues empty (19 contracts/535 ranges), name validator errors empty and complete name inventory regenerated **byte-identically**, SHA256 `11b670eda3635708daae5d6f9297c0675c6ebf11c3c590a61bab1bd8555bd26d`. [Portable receipt](portable-recovery-config-ui-edges.json); native/type/independent/full semantics open.


## Round 10 — proportional CI hypothesis fixtures (exact covering SHA tested)

- Original C04/C05/C06/C12 identity/status preserved; [portable evidence](hypothesis-ci-portable.md)/[structured receipts](hypothesis-ci-portable.json) link eight frozen source ranges and original register hash. Eight new research tests: actual temporary Git diffs/quoting/ref/rename and real NUL-safe Bash classifier plus labelled native source Python model; not PowerShell gate/Windows/branch-policy execution.
- C04 single-commit fallback omits prior Rust fixture; C05 quotes/core.quotePath-dependent Unicode lose model prefix, ASCII spaces do not. GitHub NUL-safe classifier rejects code/mixed/renamed fixtures, classifies docs tabs/newlines correctly, invalid refs fail. Native user-visible/exploit preconditions remain unresolved. C06/C12 source wiring and fail-closed counterevidence recorded, external required checks uninspected.
- **124 research +3 mocked Hermes tests passed locally, zero skips**. Existing Bash docs classifier **24 cases** and gate/deny logic **15 cases** all pass. No repo-source/workflow/gate production changes, original 39/75/5 counts unchanged.
- Exact **`6cbeaf060b6256941e201c1bb383c4b326518401`** archive passed **124 research +3 mocked Hermes, zero skips**, plus **24 classifier/15 gate-deny** existing Bash cases; all syntax/schema/native validators and frozen checkpoint returned zero issues. [Portable CI hypothesis receipt](portable-recovery-ci-hypotheses.json). Portable mechanism/source-model is not native PowerShell/Windows/Actions/branch-policy/independent/full-objective acceptance.


## Round 9 — canonical PowerShell parser-only route (exact covering SHA tested)

- Official portable Linux-x64 PowerShell 7.4.13 asset SHA256 matched GitHub digest; MIT/third-party licences inspected, complete 623-file manifest hash-checked before runtime. Task-owned ParseInput helper only, NoProfile/NonInteractive/telemetry+update opt-out; no SDK/global install/Add-Type/dot-source/repo script/gate execution. [Provenance](powershell-parser-provenance.json)/[research](powershell-parser-research.md).
- All 24 PS frozen files parsed without canonical errors. Prior [seven Tree-sitter ERROR receipts](powershell-tree-sitter-errors.json) preserved as grammar limitations, not source findings. Initial BOM helper mismatch caught via ParseFile countercheck and fixed in helper/raw-offset mapping, source untouched; BOM fixture protects parity.
- 25 PS names/lines retained, canonical AST schema replaces prior PS CST; **16,230 non-PS nodes unchanged**, total remains 16,255. All-language syntax status now 298 successes, zero parse/process/unsupported selected files; exclusions still 22+520. Not all-project review/Windows compatibility/native acceptance.
- **116 local research tests passed, zero skips**, seven new canonical fixtures (1MB/native args, nested/class/enum, never-executed input/here-strings, UTF16/UTF8/CRLF/BOM, errors/runtime-hash absence). Saved ranges/hash validators pending covering-SHA seal; original 39/75/5 counts unchanged.
- Exact **`0d1f9e214d418e251e6d8bcd65eae72ab9e5ad95`** Git archive: **116 research +3 mocked Hermes tests passed, zero skips**; syntax/schema/native validators errors empty, frozen checkpoint issues empty (18 contracts/510 ranges). Full syntax regeneration reproduced both committed artifacts **byte-identically** (16,255 nodes, 298 syntax-success files). [Portable canonical-PS receipt](portable-recovery-powershell.json) records hashes. Parsers/runtime separate ignored inputs, not installer/app execution; semantic/native/Windows compatibility/independent acceptance open.


## Round 8 — prebuilt NSIS WASM navigation (exact covering SHA tested)

- npm search supplied ready `tree-sitter-nsis 0.4.1` WASM and `web-tree-sitter 0.25.10` CJS/WASM; exact registry integrity/file hashes/licences recorded in [NSIS provenance](nsis-parser-provenance.json)/[decision](nsis-parser-research.md). Existing Node 22.23.2, Python binding remains 0.25.2. No npm install/scripts/build/SDK/NSIS compiler/installer run.
- Native WASM isolated per-file, hash checks before require/load, grammar ABI15; installer parsed without errors. 12 nodes (six definitions/two sections/four labels), previous 16,243 nodes unchanged. Current syntax: 291 successes/seven PowerShell parse errors/no unsupported selected languages, 16,255 nodes; cfg/macros/includes/installer behavior unresolved.
- Six fixtures added: Unicode UTF16→UTF8 spans, nested scopes/non-declaration comments/strings, macro/section/preprocessor/variable syntax, missing-end error, runtime absence/hash mismatch fail-closed. **109 local research tests passed, zero skips**, saved syntax validator errors empty. No semantic/native/independent acceptance or original-Grok status changes.
- Exact **`ce14c313570225ba5d93d69df37f4832f9646355`** Git archive: **109 research +3 mocked Hermes tests passed, zero skips**; syntax/schema/native validators errors empty, checkpoint issues empty. Full syntax regeneration reproduced both committed artifacts **byte-identically** (16,255 nodes). [Portable NSIS receipt](portable-recovery-nsis.json) preserves hashes; parsers/WASM supplied separately. No installer/macro/include/native/independent/full-objective acceptance.


## Round 7 — native FFI census/ownership (exact covering SHA tested)

- Seven production macOS C/Objective-C bridges/175 Rust input sources frozen-hashed: 37 nonstatic C ABI exports, 37 matching Rust foreign declarations, 57 direct CST calls; all paired by name, no ABI/cfg/linker/type/reachability proof. [Inventory](../native/README.md), [ownership/caller contract](../features/native-macos-ffi-and-ownership.md).
- Eight new parser fixtures and six source ownership assertions; **103 research tests passed locally, zero skips** with unchanged pinned parser/PO environment. Native source-hash/range/name/acceptance validator errors empty; contract registry 18/510 ranges; original 39/75/5 classifications unchanged.
- Draft assumptions corrected by source/fixtures before seal: Rust validates screenshot size before slice/copy; OCR null success returns empty; system tap exclusions empty (not self-excluding); native safe-release failures deliberately retain state; mic-start recv has no timeout, unlike asynchronous system startup.
- Source-only concerns: screenshot late callback retains image after 5s timeout without inspected cancellation, synchronous host screenshot may block UI, system Pending detach/native state retention; none natively reproduced or elevated to original-Grok bugs. Platform build/permissions/clipboard/audio/HAL/capture/ABI/model tests **not run**, independent review unavailable/not accepted.
- Exact **`acbc312a06b79062ec270b4f4f676198e6117a5e`** Git archive: **103 research +3 mocked Hermes tests passed, zero skips**; frozen checkpoint issues empty (18 contracts/510 ranges), syntax/schema/native validators errors empty. Full native census generation reproduced committed inventory byte-identically, SHA256 `8e1ee145f1024022d804465accf16a001d92b6a59621561ff2bce1a555338a1e`; [portable native receipt](portable-recovery-native.json). Parsers supplied separately, no native app/ABI/independent/whole-objective acceptance.


## Round 6 — config/UI/translation/assets inventory (exact covering SHA tested)

- Frozen baseline unchanged; task-owned docs/parser inventory only, no production/UI/catalog/asset modification. [Schema inventory/reproduction/limits](../schema/README.md), [seventeenth bounded contract](../features/config-ui-translation-and-assets.md), [PO-reader provenance](schema-parser-provenance.json).
- 83 hash-verified inputs: 84 Config fields/default-source ranges (not evaluated/literal endpoints), 2,440 Slint declaration/resource nodes, 740 PO source entries preserving duplicate before Babel merge, 58 asset hashes/XML metadata. 23 UI files transitive root-import reachable; static references present. Existing original Grok 39/75/5 classifications unchanged.
- All current 854 translation occurrences have catalog membership; five match duplicated `Installing…` with different source translations. No compiler duplicate precedence or translation quality accepted. Catalog 739 unique keys, 104 unreferenced by direct selected literals are not automatically dead.
- Reused installed Babel 2.10.3 PO-reader source exactly matches pinned upstream wheel SHA256; licences and isolated offline hash-locked Babel/pytz setup tested. Python 3.12 only; `cgi` deprecation warning means Python 3.13 is not accepted. Native parsers unchanged at binding 0.25.2/hash-gated Slint.
- Ten new inventory fixtures, total **89 local research tests** (including anti-false-membership/default/privacy/baseline/range guards). Frozen range/hash validator errors empty; contract registry 17/473 ranges; no native/app/compiler/independent acceptance.
- Exact **`f4dd4191ec4920f6b8d3d0b0c66ba6d24f0d13ac`** Git archive: **89 research +3 mocked Hermes tests passed, zero skips**, frozen checkpoint issues empty (17 contracts/473 ranges), syntax/schema validators errors empty. Full schema generation reproduced committed inventory **byte-identically**, SHA256 `fe09bf75064de1c7be50613f2a9de6781105cc1af1df7c8a22db3c7bdc8ef482`; [portable schema receipt](portable-recovery-schema.json). Parsers/PO packages supplied separately, not bundled in Git. No native/compiler/translation-quality/independent/full-project acceptance.


## Startup/health contract extension — exact covering SHA tested

- New sixteenth bounded feature contract: [startup/wizard/health/diagnostics](../features/startup-wizard-health-and-diagnostics.md), 41 exact principal-source ranges; registry now 451 ranges. No production behavior changed, no original Grok classifications changed.
- Eight new source-seam assertions; complete local suite **79 tests passed, zero skips** with pinned grammars. Frozen checkpoint validator: 16 contracts, no issues; syntax artifacts unchanged from exact `a8c75253` parser receipt.
- Coordinator checks corrected draft assumptions before registration: actual wizard finish/cancel clears slot (no completion flag/save), an existing wizard re-focuses without resetting, selecting mode saves config, report/log paths include credential-prefix redaction, readiness UI details are not report-redacted. These source tests are not independent semantic review or native privacy/timing acceptance.
- Exact commit **`a9e2a8c4424552e629c9247ec3aa151dbf58b7d2`** tested from Git archive: **79 research +3 mocked Hermes tests, zero skips**, frozen checkpoint issues empty (16 contracts/451 source ranges), saved syntax validator errors empty (16,243 nodes). See [startup portable receipt](portable-recovery-startup-health.json). Native/compiler/independent acceptance not run; config/UI schemas/native callers/remaining error paths/hypotheses remain open.


## Round 5 — polyglot parser checkpoint (exact covering SHA tested)

- Entry checkout `7909617a`; unchanged handoff 62 research tests passed with pinned original parsers and saved 13,961-node validation.
- Task-owned changes only: frozen syntax index/parser helper/fixtures/hash pins/provenance and living research docs. No production source/dependency/UI/script behavior changes.
- Extended index: 840 paths; 290 syntax successes, seven PowerShell parse-error files with partial nodes, one unsupported NSIS, 22 exclusions, 520 nonselected; 16,243 navigation nodes. Rust/Python 13,961-node subtotal preserved. Source-range/hash/count/parent validation returned zero errors; frozen 119-claim checkpoint validator issues empty, original 39/75/5 counts unchanged.
- Current local test suite: **71 passed, zero skips**, all supplied grammars pinned. 23 syntax/index fixtures plus previous 48 recovery/SQL/source tests. Offline hash-locked install of five added wheels exercised in a new ignored target. Slint shared-library archive and library SHA256 verified before loading; ABI 15/binding 0.25.2 smoke/fixtures/full-source parse passed.
- Missing anonymous CST delimiter tokens now explicitly traversed in Rust/polyglot parsers; tests revealed and corrected node-kind/name assumptions before accepted generation. PowerShell ERROR spans not suppressed or mislabeled as source bugs.
- [Parser decision/provenance/limits](parser-polyglot-research.md), [wheel/library metadata](parser-polyglot-provenance.json), [current syntax scope](../syntax/README.md).
- Native PowerShell parser, Cargo/Swift builds, live UI/model/audio/installer behavior, full semantic/caller coverage and independent review **not run/not accepted**. Permitted tools currently cannot select explicit Gemini/Opus except workflow, which the owner has not explicitly requested in this session; no inherited-model substitution. Configured web-search HTTP 402 did not block direct upstream/registry inspection.
- Exact commit **`a8c752534ccba081aecc99040e4f03f54c65f47e`** tested from a tracked-only Git archive: **71 research +3 mocked Hermes tests passed, zero skips**, syntax validator errors empty and frozen checkpoint issues empty. Full parser generation from that archive reproduced both committed syntax artifacts **byte-identically** (16,243 nodes; hashes in [round-5 portable receipt](portable-recovery-round5.json)). Original 13,961 Rust/Python records compare unchanged to `7909617a`. Parsers supplied separately from ignored hash-pinned targets, not in archive. This is bounded portable research evidence, not semantic/native/independent acceptance or goal completion.


## Reviewed source and changed scope

- Source baseline: `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`.
- Task branch: `codex/research-reconciliation`.
- Change scope: research/docs metadata and dependency-free checkpoint/test helper under docs. No production Rust/Slint/installer/CI source changed.
- First GitHub checkpoint: `c3051441b8071f83981b079604197dda4d7f5417` (remote branch SHA verified after push).

## Executed local checks

| Check | Observed result | Boundary |
| --- | --- | --- |
| Exact candidate register | 119 unique original IDs with original redacted claims | Source triage, not exhaustive project review |
| Candidate source references | All start/end ranges exist and fall within actual file lines | Range validity does not independently prove claim truth |
| Input continuity | All frozen source paths and 14 raw Grok hashes unchanged | Historical pre-snapshot source coherence remains unestablished |
| JSON/JSONL decode | Every published research JSON/JSONL decoded | Record semantics/symbol completeness not compiler-verified |
| Python syntax | Checkpoint/test files parsed successfully | Not native Suflyor application compilation |
| Navigation | Main corrected README/model/recovery/topology links resolve | Historical generated review links not all audited |
| Recovery/SQL tests | `python3 -B -m unittest discover -s docs/agent-map/operations -p 'test_*.py' -v`: 48 tests, OK (22 provenance/recovery + 7 SQLite fixtures + 19 source-seam checks); existing mocked Hermes suite: 3 tests OK | Helper and Python SQLite mechanics only; no native application or independent host-crash restart |
| Checkpoint/recover commands | Completed; null/misaligned lane files labeled `unaccepted_proposal` | No automatic redispatch or independent acceptance |
| Actual SQL migrations | Python SQLite 3.45.1 in-memory: migrations loaded; triggered FTS row deleted by session_id; no session FK on memory/diarization | Not native bundled rusqlite/Cargo tests or full DB durability |
| Git whitespace | `git diff --check`, staged check and first checkpoint `git show --check`: clean | Docs gate, not behavioral/native gate |
| Release observation | GitHub API/tag queries: latest observed RC `v0.38.1-rc.3` predates Nemotron introduction; task branch contains new source | No new publication or installer run |

The initial SQL fixture used a nonexistent `utterances.seq` column and failed before the check; the fixture was corrected against the real migration and rerun successfully. Two model-correction scripts initially used wrong constant names and failed before publishing corrected model inventory; final pins were copied from exact source. These ordinary failed checks are not hidden as passing attempts.

## Continuation round 1 evidence

- Three bounded feature contracts were added for live transcription, speaker diarization and session lifecycle/storage, with 83 unique linked source ranges/file references after spotcheck corrections. Link/range validation passed; semantic/native coverage remains partial.
- Canonical coordinator records are now checked separately from unaccepted worker proposals; receipt mismatches, missing counterevidence and out-of-range source references fail validation.
- A real `git archive 927003f2` extraction (no raw Grok reports and no prior campaign DB) initially failed source continuity on four vendored license/notice files because Git normalized CRLF to LF. Investigation proved exact CRLF-to-LF equality and preserved both hashes; all 34 verified source text-form differences are recorded for cross-platform checkout compatibility.
- Repeating tracked-only recovery with the updated helper and portability record passed: 119 canonical records recovered, zero worker proposals accepted, no automatic dispatch. The precommit experiment used the updated working helper. A subsequent [exact covering-SHA experiment](portable-recovery-evidence.json) at `059a04b1b90017fab032c25fc93d68b2be1f39eb` used only Git-archived files and passed all 24 tests plus verify/checkpoint/recover, without raw reports or prior DB. No file overlays were used.
- Normal session Start/Stop and recovery callbacks spawn work on Tokio runtime. This corrected the historical blanket UI-blocking classification; final event-loop shutdown has a synchronous stop but is a different path.
- Bounded Gemini spotcheck `workflow-3` completed with issues. Coordinator confirmed and fixed managed Whisper Turbo identity, capture-watchdog vs meeting-ending hint and diarization run/persist/poll links. Two quoted guarantees were absent from current documents and were not accepted as existing errors. Successful diarization replacement of manual names was additionally verified against SQL shape; native speaker rerun remains unexecuted.
- Selected historical O1–O8 corrections are recorded separately (11 items); no blanket historical review acceptance or full all-line claim.

## Continuation round 2 evidence

- Six additional bounded contracts cover AI/vision/provider, managed local lifecycle, read-aloud/OCR, personal memory, portable config/Settings and hotkeys/window/capture. Nine contracts now index 277 source references; links/ranges checked, not an all-function coverage percentage.
- Direct source recheck corrected TTS Windows JobObject absence, confirms init actually calls warm, AI channel=64/shared permit=2, stream timeout=120s/completion=180s, and current OCR in-memory stdin/stdout with dimension guards. Several older prose/comment assumptions were not propagated.
- Seven SQL fixtures use actual migrations for FTS/curated tables, rerun name replacement, approval V2-field omission, source restore and active/default-profile query. Seven source-seam checks protect the exact enum/hotkey/AI/TTS/OCR/normalizer declarations without claiming compilation or behavior.
- First source-seam run failed on two incorrect delimiter substrings (permit select form and end-of-spawn helper marker); tests were rebased onto inspected actual source and rerun. These are fixture failures, not application fixes.
- Bounded Gemini workflow-4 completed with issues. Receipt records accepted queue/provenance/fallback caveats and rejected stale/misquoted rows; hypothetical Codex HTTP summary consequence is not established because its exclusive flag requires local managed prep.
- [Parser research](parser-research.md) records installed-tool absence and live upstream alternatives. Verdict Compose (stdlib AST/TOML + pinned syntax grammars); no parser package/toolchain installed and no accurate AST index claimed.
- [Exact round-2 covering-SHA receipt](portable-recovery-round2.json) at `8cd96a8bcbf3a68c9af8bde43bfa842fd34457a6` passed all 34 archived tests and verify/checkpoint/recover: nine contracts, 119 canonical records, zero rejected-proposal promotions, no raw reports/prior DB/dependency install or overlays.

## Continuation round 3 evidence

- Six additional principal contracts map KB/reference, archive/playback/re-STT, summary/conspect/coaching, updater/build/release, MLX owned sidecar/install and Hermes bridge/plugin/profile-prep. Fifteen bounded contracts are now indexed; not every source/caller/native branch is accepted.
- Source checks corrected empty-query KB palette, UTF8-byte reference budget, synchronous archive delete/partial cleanup, force=false audio summary wrapper, transcript-only recap cache and best-effort conspect save. Numeric live WPM/filler pill is not established by current coaching style source/UI search.
- Updater draft erroneously claimed final redirect validation. Actual code allowlists original URL, follows default redirects and hashes bytes before write, with no `.url()`/redirect policy. Contract corrected; hypothetical arbitrary executable remains unproven because digest check exists.
- MLX contract distinguishes fast marker runtime load vs full file hash at install, exact owned-child readiness and serialized inference vs unbounded waiters. Hermes contract distinguishes configured nonloopback bind from stale loopback-only header, catalog summary vs conspect and save failure vs response success.
- Source-seam fixtures initially failed on guessed identifiers/substring shapes, then reread actual implementation and reran; all 48 research tests passed (22 recovery/provenance +7 SQL +19 source seams). Three existing plugin limit tests ran with requests mocked, no network/user data.
- workflow-5 bounded Gemini retrospective spotcheck settled with null result and no file output; receipt retains no reason/acceptance. Coordinator source review is not independent model acceptance.
- [Round-3 exact covering-SHA receipt](portable-recovery-round3.json) at `3188e6b05f36273bd32faebec3ab64dad0301afe` reran 48 research + three existing mocked Hermes tests and verify/checkpoint/recover from tracked-only Git archive: 15 contracts, 119 records, zero failed-proposal promotions, no overlays/raw reports/prior DB/network/native builds.

## Continuation round 4 / context handoff

- Pinned prebuilt MIT parser wheels installed only under ignored research target, no production/global/DSH runtime dependency change. First binding 0.26.0 passed fixtures but SIGSEGV on large traversal; per-file attempt reported 89 native failures. Failed receipt preserved; not accepted as an index.
- Alternate binding 0.25.2 + Rust grammar0.24.2 parsed all selected frozen Rust/Python files in subprocess isolation: 224 files (220 Rust/four Python), 13,961 nodes/declarations including 7,692 macro invocations, zero observed parse/native failures. 74 unsupported,22 protected/vendor excluded,520 nonselected remain explicit.
- Syntax validator checks exact source SHA, UTF8 byte/line ranges, signature prefixes, parents and counts with no semantic acceptance. It passed for all saved output; Python stdlib version3.12 and exact wheel provenance preserved.
- Full pinned suite passed 62 research checks plus three existing mocked Hermes tests. Syntax absence can skip Rust fixtures and is not parser acceptance. [Exact covering-commit receipt](portable-recovery-round4.json) at `3bfc15ae5ff7fdc405f46055ab1d60a8c943b3f8` passed all62 research+3 mocked tests, saved syntax validation and checkpoint verify/recover. Parser wheels supplied from isolated pinned site, not shipped Git archive; no raw/prior state/overlays.
- User requested context transfer and commit of completed work. [Continuation prompt](CONTINUE-PROMPT.md) records scope/next tasks; whole goal remains incomplete and must not be marked complete because of this handoff.

## Model dispatch outcome

- `workflow-1`: four Opus final results null with partial, topic-misaligned JSON; one Gemini summary returned, then corrected against source. No blanket Opus acceptance.
- `workflow-2`: eight Opus final results null; no exact-slice files produced; reason not supplied. No quota/exhaustion/approval diagnosis inferred.
- All jobs collected; none left running by this reconciliation pass. No Astra or new Grok calls.

## Source findings beyond the original Grok register

[Additional source findings](additional-source-findings.json) preserve three follow-up contracts without changing the 119 original IDs: successful diarization rerun replaces prior manual speaker names, meeting-ending is a visual hint rather than automatic stop, and candidate approval omits V2 source/entity/normalization metadata from the minted item. The replacement mechanism is reproduced in the actual shipped schema through Python SQLite; native analysis/identity remapping is not tested. [Selected historical corrections](historical-review-corrections.json) cover 14 selected historical/code-summary assertions, not every historical sentence.

## Remaining acceptance

- 39 confirmed source mechanisms, 75 hypotheses, 5 rejected original claims after caller-level follow-up. Source mechanisms do not imply 39 reproduced bugs. Normal Start/Stop dispatch counterevidence downgraded the earlier blanket UI-blocking claim.
- Native build/tests/live UI/stealth/driver latency/fault/security reproductions: **not run**.
- Full all-language symbols, feature contracts, line semantics and independent acceptance: **not complete**.
- DSH crash-restart supervisor: **not implemented**; durable checkpoint/portable continuation tested only.

The evidence-overstatement incident is recorded as `INC-1381` in the local Trajectory ledger. The ledger is operational evidence outside this repository; no private session log was copied into public research.
