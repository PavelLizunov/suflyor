#!/usr/bin/env bash
# Verify the gate (ci.yml) and cargo-deny validate (security.yml) shell logic.
# Each function mirrors the `run:` block of the corresponding job.
set -euo pipefail

# ci.yml → gate job
gate() {
  local CHANGES_RESULT="$1" RUST_NEEDED="$2" RUST_RESULT="$3"
  test "$CHANGES_RESULT" = "success" || return 1
  case "$RUST_NEEDED" in true|false) ;; *) return 1 ;; esac
  test "$RUST_NEEDED" != "true" || test "$RUST_RESULT" = "success"
}

# security.yml → cargo-deny "Validate path selector" step
cargo_deny_validate() {
  local CHANGES_RESULT="$1" DOCS_ONLY="$2"
  test "$CHANGES_RESULT" = "success" || return 1
  case "$DOCS_ONLY" in true|false) ;; *) return 1 ;; esac
}

fail=0
t() {
  local desc="$1" expect="$2"; shift 2
  local got
  if "$@"; then got=pass; else got=fail; fi
  if [ "$got" != "$expect" ]; then
    echo "FAIL: $desc (got=$got want=$expect)"
    fail=1
  else
    echo "ok:   $desc -> $got"
  fi
}

# ---- ci.yml gate ----
t 'gate: docs-only, rust skipped'      pass  gate success false skipped
t 'gate: code, rust success'           pass  gate success true  success
t 'gate: code, rust failure'           fail  gate success true  failure
t 'gate: code, rust cancelled'         fail  gate success true  cancelled
t 'gate: changes crashed'             fail  gate failure ''    skipped
t 'gate: changes cancelled'           fail  gate cancelled ''  ''
t 'gate: code, rust skipped (bug)'    fail  gate success true  skipped
t 'gate: invalid rust value (empty)'  fail  gate success ''    skipped
t 'gate: invalid rust value (junk)'   fail  gate success maybe success

# ---- security.yml cargo-deny validate ----
t 'deny: docs-only'                   pass  cargo_deny_validate success true
t 'deny: code'                        pass  cargo_deny_validate success false
t 'deny: changes crashed'             fail  cargo_deny_validate failure true
t 'deny: changes cancelled'           fail  cargo_deny_validate cancelled false
t 'deny: invalid docs_only (empty)'   fail  cargo_deny_validate success ''
t 'deny: invalid docs_only (junk)'    fail  cargo_deny_validate success maybe

[ "$fail" -eq 0 ] || { echo "logic tests FAILED"; exit 1; }
echo "All gate/cargo-deny logic tests passed."
