#!/usr/bin/env bash
# Verify the gate (ci.yml) and cargo-deny validate (security.yml) shell logic.
# Also check the fast/full command text and exercise ci.yml's actual selector.
# Gate/deny functions mirror the `run:` block of the corresponding job.
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

# ---- actual workflow full selector (no Actions runner or network needed) ----
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CI="$ROOT/.github/workflows/ci.yml"
selector="$(awk '
  /          full=false/ {copy=1}
  copy && /^$/ {exit}
  copy {sub(/^          /, ""); print}
' "$CI")"
output="$(mktemp)"
trap 'rm -f "$output"' EXIT
selects() {
  local expected_full="$1" expected_rust="$2"
  : > "$output"
  (cd "$ROOT"; EVENT_NAME="$3" REF="$4" FULL_REQUEST="$5" \
    BASE_SHA=HEAD HEAD_SHA=HEAD GITHUB_OUTPUT="$output" \
    bash -e -c "$selector") || return 1
  grep -qx "full=$expected_full" "$output" && grep -qx "rust=$expected_rust" "$output"
}
t 'selector: PR fast' pass selects false false pull_request refs/pull/1/merge false
t 'selector: ordinary push fast' pass selects false false push refs/heads/master false
t 'selector: stable tag full' pass selects true true push refs/tags/v12.34.56 ''
t 'selector: RC fast' pass selects false true push refs/tags/v1.2.3-rc.1 ''
t 'selector: malformed tag fast' pass selects false true push refs/tags/v1.2.3extra ''
t 'selector: manual default fast' pass selects false true workflow_dispatch refs/heads/master false
t 'selector: manual full' pass selects true true workflow_dispatch refs/heads/master true
t 'selector: manual tag still fast' pass selects false true workflow_dispatch refs/tags/v1.2.3 false

# Native targeted code starts after the full-tier early exit.
native="$ROOT/scripts/git-gate-native.ps1"
targeted="$(awk '/^foreach \(\$crate in \$affectedCrates\)/ {copy=1} copy' "$native")"
t 'targeted: no clippy' fail grep -qi clippy <<< "$targeted"
t 'targeted: no all-targets' fail grep -q -- --all-targets <<< "$targeted"
t 'targeted: overlay compile' pass grep -Fq 'check --locked --manifest-path $manifest --bin overlay-host' <<< "$targeted"
t 'targeted: unit tests only' pass grep -Fq 'test --locked --manifest-path $manifest --lib --bin overlay-host' <<< "$targeted"
t 'targeted: one shared guard list' pass test "$(grep -Fc 'codex_copy_guard' "$native")" -eq 1
t 'targeted: other crates run their tests' pass grep -Eq '^[[:space:]]*& \$cargo test --manifest-path \$manifest[[:space:]]*$' <<< "$targeted"
t 'CI: three full-only clippy steps' pass test "$(awk '/- name: .* clippy -D warnings/ {getline; if ($0 ~ /if: needs.changes.outputs.full == '\''true'\''/) n++} END {print n+0}' "$CI")" -eq 3
t 'CI: fast Slint unit tests' pass grep -Fq 'cargo test --locked --manifest-path slint-experiment/Cargo.toml --lib --bin overlay-host' "$CI"
t 'CI: full Slint tests guarded' pass test "$(awk '/- name: slint test \(full suite/ {getline; if ($0 ~ /if: needs.changes.outputs.full == '\''true'\''/) n++} END {print n+0}' "$CI")" -eq 1
t 'CI: stable tag trigger' pass grep -Fq "tags: ['v[0-9]+.[0-9]+.[0-9]+']" "$CI"
# Assert load-bearing gate wiring, not merely its presence.
t 'CI: gate needs unchanged' pass grep -Fq '    needs: [changes, rust]' "$CI"
t 'CI: always condition unchanged' pass grep -Fq '    if: always()' "$CI"
macos="$ROOT/scripts/git-gate-macos.sh"
t 'macOS: full flag' pass grep -Fq -- '--full) full=true' "$macos"
t 'macOS: all five clippy calls guarded' pass test "$(awk '/if \$full; then/ {guard=1} /^fi$/ {guard=0} /^ *cargo clippy/ {if (!guard) bad=1; n++} END {print (!bad && n==5)}' "$macos")" -eq 1
t 'macOS: combined unit tests' pass grep -Fq 'cargo test --locked --lib --bin overlay-host' "$macos"

[ "$fail" -eq 0 ] || { echo "logic tests FAILED"; exit 1; }
echo "All gate/cargo-deny logic tests passed."
