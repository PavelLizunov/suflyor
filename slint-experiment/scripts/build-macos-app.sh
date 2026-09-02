#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "build-macos-app.sh must run on macOS" >&2
  exit 1
fi

crate_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$crate_root/target}"
mkdir -p "$target_dir"
target_dir="$(cd "$target_dir" && pwd -P)"
if [[ "$target_dir" == "/" ]]; then
  echo "refusing to package into the filesystem root" >&2
  exit 1
fi
export CARGO_TARGET_DIR="$target_dir"
app_dir="$target_dir/Suflyor.app"
if [[ "$(dirname "$app_dir")" != "$target_dir" || "$(basename "$app_dir")" != "Suflyor.app" ]]; then
  echo "refusing to replace an unexpected app path: $app_dir" >&2
  exit 1
fi
contents_dir="$app_dir/Contents"
macos_dir="$contents_dir/MacOS"
resources_dir="$contents_dir/Resources"
third_party_notices_dir="$resources_dir/ThirdPartyNotices"
frameworks_dir="$contents_dir/Frameworks"
binary="$target_dir/release/overlay-host"
sidecar_binary="$target_dir/release/suflyor-tts"
tera_binary="$target_dir/release/suflyor-teratts"
mlx_root="$crate_root/../suflyor-mlx"
sign_identity="${SUFLYOR_MACOS_SIGN_IDENTITY:--}"
if [[ "$sign_identity" != "-" ]]; then
  if [[ ! "$sign_identity" =~ ^[[:xdigit:]]{40}$ ]]; then
    echo "SUFLYOR_MACOS_SIGN_IDENTITY must be the exact 40-hex certificate SHA-1" >&2
    exit 1
  fi
  if ! security find-identity -v -p codesigning \
    | awk -v wanted="$sign_identity" '
        toupper($2) == toupper(wanted) { found = 1 }
        END { exit(found ? 0 : 1) }
      '
  then
    echo "requested macOS code-signing identity is unavailable" >&2
    exit 1
  fi
  echo "using an explicit stable macOS code-signing identity" >&2
else
  echo "using ad-hoc macOS signing; rebuilt apps may require fresh permissions" >&2
fi
codesign_args=(--force --sign "$sign_identity" --options runtime)
if [[ "$sign_identity" != "-" ]]; then
  codesign_args+=(--timestamp)
fi

export CARGO_INCREMENTAL=0
export CARGO_BUILD_JOBS=2
cargo build --locked --release --bin overlay-host \
  --manifest-path "$crate_root/Cargo.toml"

cargo build --locked --release --manifest-path "$crate_root/../suflyor-tts/Cargo.toml"
cargo build --locked --release --manifest-path "$crate_root/../suflyor-teratts/Cargo.toml"
if [[ ! -f "$mlx_root/Package.resolved" ]]; then
  echo "required MLX Package.resolved is missing" >&2
  exit 1
fi
swift build --package-path "$mlx_root" -c release --disable-automatic-resolution --jobs 2
mlx_bin_dir="$(swift build --package-path "$mlx_root" -c release \
  --disable-automatic-resolution --jobs 2 --show-bin-path)"
mlx_binary="$mlx_bin_dir/suflyor-mlx"
mlx_metallib="$($mlx_root/Scripts/build-metallib.sh release)"

for executable in "$binary" "$sidecar_binary" "$tera_binary" "$mlx_binary"; do
  if [[ ! -x "$executable" ]]; then
    echo "required executable missing after build: $executable" >&2
    exit 1
  fi
done
if [[ "$mlx_metallib" != "$mlx_bin_dir/mlx.metallib" || ! -s "$mlx_metallib" ]]; then
  echo "required MLX Metal library missing after build" >&2
  exit 1
fi
if [[ "$(lipo -archs "$mlx_binary")" != "arm64" ]]; then
  echo "suflyor-mlx must be a thin arm64 executable" >&2
  exit 1
fi

plutil -lint "$crate_root/macos/Info.plist"
plutil -lint "$crate_root/macos/entitlements.plist"

rm -rf -- "$app_dir"
mkdir -p "$macos_dir" "$resources_dir" "$third_party_notices_dir" "$frameworks_dir"
cp "$crate_root/macos/Info.plist" "$contents_dir/Info.plist"
install -m 755 "$binary" "$macos_dir/overlay-host"
install -m 755 "$sidecar_binary" "$macos_dir/suflyor-tts"
install -m 755 "$tera_binary" "$macos_dir/suflyor-teratts"
install -m 755 "$mlx_binary" "$macos_dir/suflyor-mlx"
install -m 644 "$mlx_metallib" "$macos_dir/mlx.metallib"
if ! otool -l "$macos_dir/suflyor-mlx" | grep -q '@executable_path/../Frameworks'; then
  install_name_tool -add_rpath '@executable_path/../Frameworks' "$macos_dir/suflyor-mlx"
fi
xcrun swift-stdlib-tool --copy --platform macosx \
  --scan-executable "$macos_dir/suflyor-mlx" \
  --destination "$frameworks_dir"
while IFS= read -r bundle; do
  cp -R "$bundle" "$resources_dir/"
done < <(find "$mlx_bin_dir" -maxdepth 1 -type d -name '*.bundle' -print)
mlx_swift_checkout="$mlx_root/.build/checkouts/mlx-swift"
for license in \
  "$mlx_swift_checkout/LICENSE" \
  "$mlx_swift_checkout/Source/Cmlx/mlx/LICENSE" \
  "$mlx_swift_checkout/Source/Cmlx/metal-cpp/LICENSE.txt"
do
  if [[ ! -s "$license" ]]; then
    echo "required MLX third-party license is missing: $license" >&2
    exit 1
  fi
done
install -m 644 "$mlx_swift_checkout/LICENSE" \
  "$third_party_notices_dir/MLX-SWIFT-LICENSE"
install -m 644 "$mlx_swift_checkout/Source/Cmlx/mlx/LICENSE" \
  "$third_party_notices_dir/MLX-LICENSE"
install -m 644 "$mlx_swift_checkout/Source/Cmlx/metal-cpp/LICENSE.txt" \
  "$third_party_notices_dir/METAL-CPP-LICENSE"
bad_mlx_deps="$(otool -L "$macos_dir/suflyor-mlx" | tail -n +2 | awk '{print $1}' \
  | grep -Ev '^(@rpath/|@loader_path/|@executable_path/|/usr/lib/|/System/Library/)' || true)"
if [[ -n "$bad_mlx_deps" ]]; then
  echo "suflyor-mlx has non-bundle dependencies" >&2
  exit 1
fi
dot_clean -m "$app_dir" 2>/dev/null || true
while IFS= read -r library; do
  if [[ " $(lipo -archs "$library") " != *" arm64 "* ]]; then
    echo "bundled Swift runtime library does not contain arm64" >&2
    exit 1
  fi
done < <(find "$frameworks_dir" -type f -name '*.dylib' ! -name '._*' -print)

verify_bundle_dependency() {
  local owner="$1"
  local dependency="$2"
  case "$dependency" in
    /usr/lib/*|/System/Library/*)
      return 0
      ;;
    @rpath/*)
      local relative="${dependency#@rpath/}"
      if [[ -e "$frameworks_dir/$relative" || -e "$frameworks_dir/$(basename "$relative")" ]]; then
        return 0
      fi
      ;;
    @loader_path/*)
      if [[ -e "$(dirname "$owner")/${dependency#@loader_path/}" ]]; then
        return 0
      fi
      ;;
    @executable_path/*)
      if [[ -e "$macos_dir/${dependency#@executable_path/}" ]]; then
        return 0
      fi
      ;;
  esac
  echo "unresolved bundled dependency for $(basename "$owner"): $dependency" >&2
  return 1
}

while IFS= read -r owner; do
  while IFS= read -r dependency; do
    verify_bundle_dependency "$owner" "$dependency"
  done < <(otool -L "$owner" | tail -n +2 | awk '{print $1}')
done < <(printf '%s\n' "$macos_dir/suflyor-mlx"; find "$frameworks_dir" -type f -name '*.dylib' -print)

# Package AppIcon.icns
icon_source="$crate_root/assets/icon-source.png"
if [[ ! -f "$icon_source" ]]; then
  icon_source="$crate_root/assets/icon.png"
fi
if [[ ! -f "$icon_source" ]]; then
  echo "required app icon source is missing" >&2
  exit 1
fi
tmp_parent="$(mktemp -d)"
trap 'rm -rf -- "$tmp_parent"' EXIT
iconset_dir="$tmp_parent/AppIcon.iconset"
mkdir -p "$iconset_dir"
sips -z 16 16     "$icon_source" --out "$iconset_dir/icon_16x16.png" >/dev/null
sips -z 32 32     "$icon_source" --out "$iconset_dir/icon_16x16@2x.png" >/dev/null
sips -z 32 32     "$icon_source" --out "$iconset_dir/icon_32x32.png" >/dev/null
sips -z 64 64     "$icon_source" --out "$iconset_dir/icon_32x32@2x.png" >/dev/null
sips -z 128 128   "$icon_source" --out "$iconset_dir/icon_128x128.png" >/dev/null
sips -z 256 256   "$icon_source" --out "$iconset_dir/icon_128x128@2x.png" >/dev/null
sips -z 256 256   "$icon_source" --out "$iconset_dir/icon_256x256.png" >/dev/null
sips -z 512 512   "$icon_source" --out "$iconset_dir/icon_256x256@2x.png" >/dev/null
sips -z 512 512   "$icon_source" --out "$iconset_dir/icon_512x512.png" >/dev/null
sips -z 1024 1024 "$icon_source" --out "$iconset_dir/icon_512x512@2x.png" >/dev/null
iconutil -c icns "$iconset_dir" -o "$resources_dir/AppIcon.icns"
if [[ ! -s "$resources_dir/AppIcon.icns" ]]; then
  echo "AppIcon.icns was not created" >&2
  exit 1
fi
rm -rf -- "$tmp_parent"
trap - EXIT

# Local packaging defaults to ad-hoc. An explicit stable Keychain identity keeps
# the designated requirement stable for personal upgrade/TCC testing. Public
# distribution still requires owner-authorized Developer ID + notarization.
while IFS= read -r library; do
  codesign "${codesign_args[@]}" "$library"
done < <(find "$frameworks_dir" -type f -name '*.dylib' ! -name '._*' -print)
while IFS= read -r framework; do
  codesign "${codesign_args[@]}" "$framework"
done < <(find "$frameworks_dir" -depth -type d -name '*.framework' ! -name '._*' -print)
codesign "${codesign_args[@]}" "$macos_dir/mlx.metallib"
codesign "${codesign_args[@]}" "$macos_dir/suflyor-mlx"
codesign "${codesign_args[@]}" "$macos_dir/suflyor-tts"
codesign "${codesign_args[@]}" "$macos_dir/suflyor-teratts"
codesign "${codesign_args[@]}" \
  --entitlements "$crate_root/macos/entitlements.plist" \
  "$app_dir"
codesign --verify --strict --verbose=2 "$macos_dir/suflyor-tts"
codesign --verify --strict --verbose=2 "$macos_dir/suflyor-teratts"
codesign --verify --strict --verbose=2 "$macos_dir/suflyor-mlx"
codesign --verify --strict --verbose=2 "$macos_dir/mlx.metallib"
codesign --verify --deep --strict --verbose=2 "$app_dir"
plutil -lint "$contents_dir/Info.plist"
if "$macos_dir/suflyor-mlx" </dev/null >/dev/null 2>&1; then
  echo "suflyor-mlx accepted an empty startup command" >&2
  exit 1
else
  mlx_smoke_status=$?
fi
if [[ "$mlx_smoke_status" -ne 1 ]]; then
  echo "suflyor-mlx packaged launch smoke failed" >&2
  exit 1
fi

echo "$app_dir"
