#!/usr/bin/env bash
# Classify a base..head diff as docs-only or not.
#
# Usage: bash is-docs-only.sh BASE_SHA HEAD_SHA
# Prints "true" (docs-only) or "false" (full CI) to stdout.
# Exits 1 on any error — callers must fail the job (fail-closed).
#
# docs-only ⟺ every changed path has suffix .md OR lives below docs/.
# --no-renames: a rename shows both old and new names, so renaming a source
# file to .md still counts as a code change (conservative).
#
# NUL-safe: uses git diff -z and iterates NUL-delimited paths so filenames
# containing tabs or newlines are classified correctly.  No newline splitting,
# no eval.  git diff exit status is checked before iteration (fail-closed).
#
# This script is the SINGLE SOURCE OF TRUTH for the classification pattern.
# Both ci.yml and security.yml call it.  The regression test in
# test-docs-only-detection.sh exercises it end-to-end.
set -euo pipefail

BASE_SHA="${1:?usage: is-docs-only.sh BASE_SHA HEAD_SHA}"
HEAD_SHA="${2:?usage: is-docs-only.sh BASE_SHA HEAD_SHA}"

# Fail closed on invalid refs.
git rev-parse --verify --quiet "$BASE_SHA" >/dev/null 2>&1 ||
  { echo "is-docs-only: invalid BASE_SHA: $BASE_SHA" >&2; exit 1; }
git rev-parse --verify --quiet "$HEAD_SHA" >/dev/null 2>&1 ||
  { echo "is-docs-only: invalid HEAD_SHA: $HEAD_SHA" >&2; exit 1; }

# NUL-delimited diff into a temp file so we can check git's exit status
# before iterating (fail-closed on git errors).
tmpfile="$(mktemp)"
trap 'rm -f "$tmpfile"' EXIT

if ! git diff --name-only -z --no-renames "$BASE_SHA" "$HEAD_SHA" > "$tmpfile"; then
  echo "is-docs-only: git diff failed" >&2
  exit 1
fi

# Iterate NUL-delimited paths.  Each exact path is checked:
#   *.md suffix   → docs-only OK
#   docs/* prefix → docs-only OK
#   anything else → full CI
while IFS= read -r -d '' path; do
  case "$path" in
    *.md | docs/*) ;;
    *) echo "false"; exit 0 ;;
  esac
done < "$tmpfile"

# Empty diff or all paths matched → docs-only.
echo "true"
