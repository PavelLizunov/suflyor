#!/usr/bin/env bash
# Build an isolated Hyprland 0.56.2 with the adapted PR #16061 capture path.
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cache=${SUFLYOR_HYPRLAND_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/suflyor-hyprland-capture}
patch_file="$script_dir/hyprland-capture/0001-clean-monitor-capture.patch"
patch_id=$(sha256sum "$patch_file" | cut -c1-12)
prefix=${SUFLYOR_HYPRLAND_PREFIX:-$HOME/.local/lib/suflyor-hyprland/0.56.2-$patch_id}
source_dir="$cache/source-$patch_id"
build_dir="$cache/build-$patch_id"

fetch() {
  local name=$1 url=$2 digest=$3
  local path="$cache/downloads/$name"
  if ! printf '%s  %s\n' "$digest" "$path" | sha256sum --check --status 2>/dev/null; then
    curl --fail --location --connect-timeout 20 --max-time 300 --output "$path.part" "$url"
    printf '%s  %s\n' "$digest" "$path.part" | sha256sum --check --status
    mv -- "$path.part" "$path"
  fi
}

mkdir -p "$cache/downloads" "$cache/deps" "$cache/glaze"
fetch hyprland.tar.gz \
  https://codeload.github.com/hyprwm/Hyprland/tar.gz/efb50993780079460b0cbed1363e2166a2de1d9f \
  5b05ce7ba5296fa46e666852bf108ab5af121bf577f8043787e2d2c252d8f109
fetch udis86.tar.gz \
  https://codeload.github.com/canihavesomecoffee/udis86/tar.gz/5336633af70f3917760a6d441ff02d93477b0c86 \
  2f459fafa19535ef1be417bf7614e4f92475e5787d7ba76df20287cc29d1b5a9
fetch hyprland-protocols.tar.gz \
  https://codeload.github.com/hyprwm/hyprland-protocols/tar.gz/bd153e76f751f150a09328dbdeb5e4fab9d23622 \
  f31112800ec83ec529d0d35699a3d20fa0ed3cbb97d344146340815fec5ac055
fetch glaze.tar.gz \
  https://codeload.github.com/stephenberry/glaze/tar.gz/refs/tags/v7.2.0 \
  17dba19ae63ae48f94994f00d49d5cb3c8f1306db1046c534c4828662490b7d4
fetch hyprwayland-scanner.pkg.tar.zst \
  https://rc-mirror.omarchy.org/extra/os/x86_64/hyprwayland-scanner-0.4.6-1-x86_64.pkg.tar.zst \
  3d9da39c19f9febb0a1b3af9d88dc9a1466568f595228ffc88ffc51907e73d3a

if [[ ! -e "$source_dir/.suflyor-patched" ]]; then
  mkdir -p "$source_dir"
  tar -xf "$cache/downloads/hyprland.tar.gz" -C "$source_dir" --strip-components=1
  tar -xf "$cache/downloads/udis86.tar.gz" -C "$source_dir/subprojects/udis86" --strip-components=1
  tar -xf "$cache/downloads/hyprland-protocols.tar.gz" -C "$source_dir/subprojects/hyprland-protocols" --strip-components=1
  patch --directory="$source_dir" --strip=1 --batch < "$patch_file"
  touch "$source_dir/.suflyor-patched"
fi

tar -xf "$cache/downloads/glaze.tar.gz" -C "$cache/glaze" --strip-components=1
tar -xf "$cache/downloads/hyprwayland-scanner.pkg.tar.zst" -C "$cache/deps"
export PATH="$cache/deps/usr/bin:$PATH"
export GIT_COMMIT_HASH=efb50993780079460b0cbed1363e2166a2de1d9f
export GIT_BRANCH=suflyor-capture-pr16061 GIT_TAG=v0.56.2 GIT_DIRTY=dirty GIT_COMMITS=7661
export GIT_COMMIT_MESSAGE="Suflyor clean capture: PR 16061, patch $patch_id"
export GIT_COMMIT_DATE=2026-08-05
cmake -S "$source_dir" -B "$build_dir" -G 'Unix Makefiles' \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$prefix" \
  -DCMAKE_PREFIX_PATH="$cache/deps/usr" -DFETCHCONTENT_SOURCE_DIR_GLAZE="$cache/glaze" \
  -DCMAKE_C_COMPILER=/usr/bin/gcc -DCMAKE_CXX_COMPILER=/usr/bin/g++ \
  -DBUILD_TESTING=OFF -DWITH_TESTS=OFF -DNO_HYPRPM=ON \
  -DCMAKE_CXX_FLAGS_RELEASE='-O2 -DNDEBUG'
cmake --build "$build_dir" --target Hyprland --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-1}"
install -Dm755 "$build_dir/Hyprland" "$prefix/bin/Hyprland"
install -Dm644 "$patch_file" "$prefix/clean-monitor-capture.patch"
install -Dm644 "$source_dir/LICENSE" "$prefix/LICENSE.hyprland"
printf 'Installed, not started: %s\n' "$prefix/bin/Hyprland"
printf 'Use scripts/hyprland-capture/session.sh enable %q to select it for the next login.\n' "$prefix/bin/Hyprland"
