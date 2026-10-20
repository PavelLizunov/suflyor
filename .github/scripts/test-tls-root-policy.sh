#!/usr/bin/env bash
# Exercise the actual cargo-deny policy with registry-backed dependency fixtures.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
for version in 0.26.10 0.26.11 1.0.9 1.0.10; do
    fixture="$tmp/roots-$version"
    mkdir -p "$fixture/src"
    printf '[package]\nname = "tls-root-policy-fixture"\nversion = "0.0.0"\nedition = "2021"\n[dependencies]\nwebpki-roots = "=%s"\n' "$version" > "$fixture/Cargo.toml"
    printf '' > "$fixture/src/lib.rs"
    cargo generate-lockfile --manifest-path "$fixture/Cargo.toml"
    if cargo deny --manifest-path "$fixture/Cargo.toml" --config "$root/deny.toml" check bans > "$fixture/result.log" 2>&1; then
        result=allowed
    else
        result=denied
        if ! grep -q 'banned' "$fixture/result.log"; then
            cat "$fixture/result.log"
            exit 1
        fi
    fi
    case "$version" in
        0.26.11|1.0.10) expected=allowed ;;
        *) expected=denied ;;
    esac
    if [ "$result" != "$expected" ]; then
        cat "$fixture/result.log"
        echo "FAIL: webpki-roots $version was $result, expected $expected" >&2
        exit 1
    fi
    echo "PASS: webpki-roots $version is $result."
done
