# Original config C05/C07: bounded credential-file evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_credential_file_hypotheses.py) use temporary dummy files and frozen source. They do not read owner credentials or execute Rust. C05/C07 remain hypotheses.

## C05 — temporary open follows a symlink

[Directory permissions](<../../../overlay-backend/src/credentials.rs#L124-L138>) check metadata and then chmod; the selected function has no `O_NOFOLLOW` or `fchmod`. [write_to_path](<../../../overlay-backend/src/credentials.rs#L158-L194>) uses `credentials.json.tmp`, creates and truncates it, then renames it. A temporary symlink is followed by the Python model of that regular open.

## C07 — invalid JSON becomes an empty map

[read_map](<../../../overlay-backend/src/credentials.rs#L148-L156>) returns an empty map for missing, unreadable, or invalid JSON. The fixture starts with `{bad`, adds one dummy slot, and writes only that slot. There is no lock or merge in the selected source.

## Limits

No owner credential file, Rust filesystem race, Windows credential API, or secret material was used. Original statuses and 39/75/5 remain unchanged.
