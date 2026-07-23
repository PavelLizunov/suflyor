#!/usr/bin/env bash
# End-to-end regression test for .github/scripts/is-docs-only.sh.
# Creates a throwaway git repo, commits specific file sets, and verifies
# the classifier output for each.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLASSIFIER="$SCRIPT_DIR/is-docs-only.sh"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
cd "$tmpdir"

git init -q
git config user.email "ci@test"
git config user.name "ci"
git commit -q --allow-empty -m "base"
BASE="$(git rev-parse HEAD)"

fail=0

check() {
  local expect="$1" desc="$2"; shift 2
  for f in "$@"; do
    mkdir -p "$(dirname "$f")"
    echo x > "$f"
  done
  git add -A
  git commit -q -m "$desc"
  local head got
  head="$(git rev-parse HEAD)"
  if ! got="$(bash "$CLASSIFIER" "$BASE" "$head" 2>&1)"; then
    echo "FAIL: $desc — classifier exited non-zero: $got"
    fail=1
  elif [ "$got" != "$expect" ]; then
    echo "FAIL: $desc — expected=$expect got=$got"
    fail=1
  else
    echo "ok:   $desc -> $got"
  fi
  git reset -q --hard "$BASE"
}

# ---- docs-only ----
check true  "single .md"            README.md
check true  "docs/ markdown"        docs/goal-foo.md
check true  "docs/ non-markdown"    docs/retest.html
check true  "docs/ nested"          docs/reference-shots/icon.png
check true  "crate README"          slint-experiment/README.md
check true  "multiple docs"         README.md docs/a.md docs/b.png

# ---- full CI ----
check false "rust source"           overlay-backend/src/lib.rs
check false "cargo manifest"        Cargo.toml
check false "cargo lock"            slint-experiment/Cargo.lock
check false "slint UI"              slint-experiment/ui/main.slint
check false "workflow file"         .github/workflows/ci.yml
check false "script"                scripts/ci.ps1
check false "deny.toml"             deny.toml
check false "gitleaks config"       .gitleaks.toml
check false "mixed docs+code"       docs/goal.md src/main.rs
check false "mixed md+cargo"        README.md Cargo.lock

# ---- NUL-safety: tab in filename ----
tab="$(printf '\t')"
check true  "docs/ tab in name"     "docs/foo${tab}bar.md"
check false "src/ tab in name"      "src/foo${tab}bar.rs"

# ---- NUL-safety: newline in filename (Linux/ext4; NTFS forbids \n) ----
nl="$(printf '\nx')"; nl="${nl%x}"
if mkdir -p docs && touch "docs/a${nl}b.md" 2>/dev/null; then
  rm -f "docs/a${nl}b.md"
  check true  "docs/ newline in name"  "docs/a${nl}b.md"
  check false "src/ newline in name"   "src/a${nl}b.rs"
else
  echo "skip: newline-in-filename (filesystem restriction)"
fi

# ---- rename: --no-renames shows old+new → conservative full CI ----
mkdir -p src docs
echo x > src/old.rs
git add -A
git commit -q -m "add src/old.rs"
RENAME_BASE="$(git rev-parse HEAD)"
git mv src/old.rs docs/new.md
git commit -q -m "rename src/old.rs -> docs/new.md"
RENAME_HEAD="$(git rev-parse HEAD)"
got="$(bash "$CLASSIFIER" "$RENAME_BASE" "$RENAME_HEAD" 2>&1)" || {
  echo "FAIL: rename — classifier exited non-zero: $got"; fail=1; got=""; }
if [ -n "$got" ] && [ "$got" != "false" ]; then
  echo "FAIL: rename — expected=false got=$got"
  fail=1
elif [ -n "$got" ]; then
  echo "ok:   rename src/old.rs -> docs/new.md -> false (conservative)"
fi
git reset -q --hard "$BASE"

# ---- empty diff (base..base) ----
got="$(bash "$CLASSIFIER" "$BASE" "$BASE")"
if [ "$got" != "true" ]; then
  echo "FAIL: empty diff — expected=true got=$got"
  fail=1
else
  echo "ok:   empty diff -> true"
fi

# ---- invalid SHA must fail closed ----
if bash "$CLASSIFIER" "not-a-sha" "$BASE" >/dev/null 2>&1; then
  echo "FAIL: invalid BASE_SHA — classifier should have exited non-zero"
  fail=1
else
  echo "ok:   invalid BASE_SHA -> exit 1"
fi

[ "$fail" -eq 0 ] || { echo "is-docs-only.sh tests FAILED"; exit 1; }
echo "All is-docs-only.sh tests passed."
