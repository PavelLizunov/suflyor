#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "build-metallib.sh must run on macOS" >&2
  exit 1
fi

package_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
configuration="${1:-release}"
if [[ "$configuration" != "debug" && "$configuration" != "release" ]]; then
  echo "configuration must be debug or release" >&2
  exit 1
fi

mlx_checkout="$package_root/.build/checkouts/mlx-swift"
expected_mlx_swift_revision="0bb916c67f4b9e5c682cbe02a42c701c93ab5021"
if [[ ! -d "$mlx_checkout/.git" ]]; then
  echo "resolved mlx-swift checkout is missing; run the locked Swift build first" >&2
  exit 1
fi
if [[ "$(git -C "$mlx_checkout" rev-parse HEAD)" != "$expected_mlx_swift_revision" ]]; then
  echo "mlx-swift checkout does not match the audited Package.resolved revision" >&2
  exit 1
fi
component="$(xcodebuild -showComponent MetalToolchain -json 2>/dev/null || true)"
if ! grep -q '"status" : "installed"' <<<"$component" \
  || ! xcrun -f metal >/dev/null 2>&1 \
  || ! xcrun -f metallib >/dev/null 2>&1; then
  echo "Xcode Metal Toolchain is missing; install it with xcodebuild -downloadComponent MetalToolchain" >&2
  exit 1
fi

metal_root="$mlx_checkout/Source/Cmlx/mlx-generated/metal"
sources=(
  arg_reduce.metal
  conv.metal
  gemv.metal
  layer_norm.metal
  random.metal
  rms_norm.metal
  rope.metal
  scaled_dot_product_attention.metal
  steel/attn/kernels/steel_attention.metal
)
if [[ "${#sources[@]}" -ne 9 ]]; then
  echo "the audited MLX Metal source set must contain exactly nine entrypoints" >&2
  exit 1
fi
for source in "${sources[@]}"; do
  if [[ ! -f "$metal_root/$source" ]]; then
    echo "required pinned MLX Metal source is missing: $source" >&2
    exit 1
  fi
done

bin_dir="$(swift build --package-path "$package_root" -c "$configuration" \
  --disable-automatic-resolution --show-bin-path)"
mkdir -p "$bin_dir"
tmp_parent="$(mktemp -d)"
trap 'rm -rf -- "$tmp_parent"' EXIT

airs=()
for source in "${sources[@]}"; do
  name="$(basename "$source" .metal)"
  air="$tmp_parent/$name.air"
  xcrun -sdk macosx metal \
    -x metal \
    -Wall \
    -Wextra \
    -fno-fast-math \
    -Wno-c++17-extensions \
    -Wno-c++20-extensions \
    -mmacosx-version-min=14.2 \
    -c "$metal_root/$source" \
    -I"$metal_root" \
    -o "$air"
  airs+=("$air")
done

xcrun -sdk macosx metallib "${airs[@]}" \
  -o "$tmp_parent/mlx.metallib"
if [[ ! -s "$tmp_parent/mlx.metallib" ]]; then
  echo "MLX Metal library was not created" >&2
  exit 1
fi
xcrun metallib --app-store-validate "$tmp_parent/mlx.metallib" >/dev/null
if ! file "$tmp_parent/mlx.metallib" | grep -q 'MetalLib executable'; then
  echo "MLX Metal output is not a valid Metal library" >&2
  exit 1
fi

# MLX searches beside the executable first, before its SwiftPM bundle paths.
staged="$bin_dir/.mlx.metallib.$$"
install -m 644 "$tmp_parent/mlx.metallib" "$staged"
mv -f -- "$staged" "$bin_dir/mlx.metallib"
echo "$bin_dir/mlx.metallib"
