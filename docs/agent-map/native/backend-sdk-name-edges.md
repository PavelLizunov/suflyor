# Backend credential/process SDK candidates

[Machine inventory](backend-sdk-name-edges.json) reuses selected Windows Rust CST import/call reader with **five frozen files**: credentials, local_ai, tts, nemotron_diar, download. **28 SDK imports /17 direct call syntax candidates**, every row unresolved_symbol false. Qualified/native constructor/cfg/name collisions aren't compiler graph/ABI/runtime evidence. Other backend audio/device/installer APIs, aliases/function pointers/macros/test reachability not covered.

[Manual contract](../features/credentials-and-managed-process-ownership.md) maps protected versus plaintext credential storage and different attach/kill/wait/EOF cleanup source paths; original C03/C10 statuses unchanged. No native secret/child/SDK/API operation executed. Host [four-file SDK census](windows-sdk-name-edges.md) remains separate and byte-identical after reader selection extension.

```bash
PYTHONPATH=.campaign-state/parser-site-stable python3 -B \
  docs/agent-map/operations/windows_sdk_edges.py --selection backend --output docs/agent-map/native/backend-sdk-name-edges.json
python3 -B docs/agent-map/operations/windows_sdk_edges.py \
  --selection backend --output docs/agent-map/native/backend-sdk-name-edges.json --validate
```

Read artifact before regeneration under file-observation policy. Existing binding 0.25.2/Rust grammar 0.24.2, isolated 60s worker; no new dependencies or SDK. Six added source fixtures inspect CredFree/zeroing/plaintext secure modes/temp flush/rename, direct-key Config boundary, JobObject best-effort/unit-return/process-lifetime handle, TTS no-window/drop without wait and distinct Nemotron ChildGuard. None executes actual Rust storage/process behavior or proves all caller/alias/OS semantics.
