# Credentials and managed child lifetime: bounded source contract

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Source/caller inspection plus backend SDK candidate census; **no credential values/files, Credential Manager API, process spawn/kill, models, SDK/native tests executed**. Original `wave2_worker3_tts-C03` / `wave2_worker4_local_ai-C10` remain hypotheses; append counterevidence/preconditions, not reproduced platform failures.

## Direct provider credentials versus legacy config

[Slots](<../../../overlay-backend/src/credentials.rs#L11-L25>) map OpenAI/Anthropic to distinct targets; legacy bridge/Groq/vision/Whisper remain config/export secret-bearing. [Settings key callbacks](<../../../slint-experiment/src/bin/overlay_host/settings_ai.rs#L567-L616>) write credentials API, generic localized error/status, no Config mutation/save for direct keys. Empty input deletes. [Config endpoint resolver](<../../../overlay-backend/src/config.rs#L769-L781>) reads protected slot into ephemeral endpoint bearer; [read failure seam](<../../../overlay-backend/src/config.rs#L1145-L1151>) maps missing/error to empty, not an error-preserving credential readiness state. [Settings status](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L1633-L1646>) similarly collapses read failure/missing to not-set. This does not encrypt legacy config/exports or imply all API failures surfaced.

## Windows blob ownership

[Write](<../../../overlay-backend/src/credentials.rs#L32-L63>) trims, blank→delete, constructs UTF8 blob, CredWrite generic LOCAL_MACHINE persistence; zeroes temporary byte Vec **after call even Err**, then maps category context. Original caller strings/additional copies aren't zeroized, no global memory scrub claim. [Read](<../../../overlay-backend/src/credentials.rs#L66-L99>) CredRead error returns None without distinguishing not-found/access/other API failures; validates returned pointer/size, copies blob as String UTF8 Result, calls CredFree once **before** returning parsed Result. UTF8 decoding failure still frees allocation; allocation panic/hostile pointers not tested. [Delete](<../../../overlay-backend/src/credentials.rs#L101-L113>) not-found is success, other failures Err. No native credential accessibility/roaming/security acceptance.

## Unix plaintext file and lost-update/durability boundaries

[File store](<../../../overlay-backend/src/credentials.rs#L115-L196>) uses product data root/credentials.json; HashMap serialized pretty JSON, **plaintext, not Keychain/encrypted**. Existing file parse errors become empty map via unwrap_or_default; subsequent write could replace malformed prior data. Directory mode 0700, temp/file 0600, write+flush (no sync_all), remove own temp on write Err, rename atomic-replace intent. Parent fsync/symlink-hardening/concurrency lock not shown; same `.tmp` path shared across writes. No corruption/concurrent-write/symlink/disk-crash reproduction; filesystem fault preconditions unresolved.

[Unix read/write/delete](<../../../overlay-backend/src/credentials.rs#L198-L225>) blank→delete, read returns cloned slot, modifying one slot merges current map but no synchronized transaction. Inline Rust mode tests exist [source](<../../../overlay-backend/src/credentials.rs#L238-L281>), **not run here**. Never inspect real store or include private key values in research.

## Hidden spawning and best-effort Windows lifetime job

[Shared no-window](<../../../overlay-backend/src/download.rs#L120-L130>) adds CREATE_NO_WINDOW Windows, no-op POSIX. [Managed hidden spawn](<../../../overlay-backend/src/local_ai.rs#L3062-L3092>) wraps Command, child stdin null, spawns then Windows attach; returns Child regardless attach result.

[Lifetime JobObject](<../../../overlay-backend/src/local_ai.rs#L3094-L3146>) uses process-wide OnceLock<usize>, CreateJobObject→KILL_ON_JOB_CLOSE limit→AssignProcess. Failure cached zero for creation/limit so retries don't occur; SetInformation failure leaves created handle unclosed in reviewed branch; successful handle intentionally process-lifetime (OS closes on parent exit). Assignment failure only logs and caller receives no Result. Therefore attach **exists**, but not universal guaranteed parent-death cleanup. Nested job/restrictions/inheritance/launch-before-attach race and OS force-kill behavior untested. This is precise C10 precondition evidence, not runtime orphan proof.

[Explicit stop](<../../../overlay-backend/src/local_ai.rs#L1063-L1101>) iterates owned servers, on Windows calls taskkill tree helper; on failure child.kill, all platforms child.wait ignoring error. [Tree helper](<../../../overlay-backend/src/local_ai.rs#L1172-L1178>) taskkill /PID /T /F. Separate owner-root listener sweep prevents arbitrary unmanaged listener kill by intent; full PID-reuse/root-validation/native-process acceptance outside slice.

## TTS EOF/handle-drop distinction and Nemotron guard

[TTS spawn](<../../../overlay-backend/src/tts.rs#L1581-L1603>) no-window+stdin/out pipes, stderr null, Windows assign present. [Sidecar ensure](<../../../overlay-backend/src/tts.rs#L839-L940>) try_wait determines alive; old stdin/process handles dropped on restart, carries voice/rate/speed state. [Broken-pipe write](<../../../overlay-backend/src/tts.rs#L946-L965>) drops stdin/proc without explicit kill/wait; no Sidecar Drop impl in selected source. std::process::Child handle drop does not itself guarantee kill; EOF/JobObject/worker protocol are distinct cleanup mechanisms, not all OSes equivalent.

[Piper stdin](<../../../suflyor-tts/src/main.rs#L136-L150>) EOF sends Message::Shutdown; [Tera stdin](<../../../suflyor-teratts/src/main.rs#L648-L663>) also sends Shutdown; [Tera worker](<../../../suflyor-teratts/src/main.rs#L499-L516>) breaks and closes active controller. This **counterevidence** narrows C03: ordinary stdin-close has explicit protocol shutdown, not proof every parent kill/drain/channel situation or every Child is reaped. No POSIX pdeathsig/kqueue parent watch found in these entrypoint paths; scheduler/thread/model cancellation behavior remains native-untested.

[Nemotron](<../../../overlay-backend/src/nemotron_diar.rs#L19-L26>) is diarization, not STT. ChildGuard Drop kills+waits; [spawn](<../../../overlay-backend/src/nemotron_diar.rs#L44-L107>) no-window+Windows job attach, timeout/cancel normal polling return triggers guard cleanup, stdout/stderr suppressed, RTTM parsed only on success. Early file/parse errors not process crash acceptance. Native .wav/model execution not performed.

## SDK name scope and hypothesis evidence

Five backend files yield 28 SDK import-name records/17 direct-call syntax candidates. [Inventory/reproduction](<../native/backend-sdk-name-edges.md>) inherits unresolved cfg/type/lexical/callback limits; not entire backend audio/credentials/installer Windows graph. Six source assertions validate selected ownership/caller patterns, not storage/write/child lifecycle implementation execution. Original 39/75/5 totals unchanged; independent/native permission/concurrency/force-kill/driver acceptance open.
