#!/usr/bin/env python3
"""Install the pinned, optional CUDA 13 speech runtime into a build directory.

Usage: python3 scripts/setup-linux-speech-cuda.py BUILD_DIR
The installer copies BUILD_DIR/speech-gpu beside the host and speech sidecars.
Model weights are not part of this runtime.
"""
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import zipfile

SHERPA = (
    "sherpa-cuda13.tar.bz2", 229362945,
    "dde7732e0649dbe0702266f4b57d9b2ac1136f879057bed6754b984752ca7a35",
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.13.5/"
    "sherpa-onnx-v1.13.5-cuda-13.x-cudnn-9.x-onnxruntime1.27.1-linux-x64-gpu.tar.bz2",
)
WHEELS = [
    ("cudnn.whl", 351301951, "6349bc8769369a91611c5e2ce5c2e510e61848c245099c31e870d2cdce0ab90d",
     "https://files.pythonhosted.org/packages/d5/bb/0170724b08affdd6187d550b28d121ffb2f93f162b8ea3b63c545e1fcde4/nvidia_cudnn_cu13-9.16.0.29-py3-none-manylinux_2_27_x86_64.whl"),
    ("cublas.whl", 420882912, "0cf238ffdbe46c00cb9aa98e7bca745c223a5ca5a53e686ac8b3e5a08fe80d6c",
     "https://files.pythonhosted.org/packages/28/d4/30e630055d8e0f7058cc0d39e562c965d692f4735537d3f02f45d579a280/nvidia_cublas-13.0.2.14-py3-none-manylinux_2_27_x86_64.whl"),
    ("cudart.whl", 2243632, "7f82250d7782aa23b6cfe765ecc7db554bd3c2870c43f3d1821f1d18aebf0548",
     "https://files.pythonhosted.org/packages/2e/24/d1558f3b68b1d26e706813b1d10aa1d785e4698c425af8db8edc3dced472/nvidia_cuda_runtime-13.0.96-py3-none-manylinux2014_x86_64.manylinux_2_17_x86_64.whl"),
    ("cufft.whl", 214085489, "6c44f692dce8fd5ffd3e3df134b6cdb9c2f72d99cf40b62c32dde45eea9ddad3",
     "https://files.pythonhosted.org/packages/a8/2f/7b57e29836ea8714f81e9898409196f47d772d5ddedddf1592eadb8ab743/nvidia_cufft-12.0.0.61-py3-none-manylinux2014_x86_64.manylinux_2_17_x86_64.whl"),
    ("curand.whl", 59544258, "1aee33a5da6e1db083fe2b90082def8915f30f3248d5896bcec36a579d941bfc",
     "https://files.pythonhosted.org/packages/a5/9f/be0a41ca4a4917abf5cb9ae0daff1a6060cc5de950aec0396de9f3b52bc5/nvidia_curand-10.4.0.35-py3-none-manylinux_2_27_x86_64.whl"),
    ("nvrtc.whl", 90215200, "ad9b6d2ead2435f11cbb6868809d2adeeee302e9bb94bcf0539c7a40d80e8575",
     "https://files.pythonhosted.org/packages/c3/68/483a78f5e8f31b08fb1bb671559968c0ca3a065ac7acabfc7cee55214fd6/nvidia_cuda_nvrtc-13.0.88-py3-none-manylinux2010_x86_64.manylinux_2_12_x86_64.whl"),
    ("nvjitlink.whl", 40713933, "13a74f429e23b921c1109976abefacc69835f2f433ebd323d3946e11d804e47b",
     "https://files.pythonhosted.org/packages/56/7a/123e033aaff487c77107195fa5a2b8686795ca537935a24efae476c41f05/nvidia_nvjitlink-13.0.88-py3-none-manylinux2010_x86_64.manylinux_2_12_x86_64.whl"),
]


def verified(path, size, sha):
    if not path.is_file() or path.stat().st_size != size:
        return False
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest() == sha


def download(cache, item):
    name, size, sha, url = item
    path = cache / name
    if verified(path, size, sha):
        return path
    partial = path.with_suffix(".download")
    print("Downloading", name, flush=True)
    subprocess.run([
        "curl", "--fail", "--location", "--continue-at", "-",
        "--connect-timeout", "15", "--max-time", "600",
        "--speed-limit", "1024", "--speed-time", "30", "--silent", "--show-error",
        "--output", str(partial), url,
    ], check=True)
    if not verified(partial, size, sha):
        partial.unlink(missing_ok=True)
        raise RuntimeError(f"Runtime download failed integrity validation: {name}")
    partial.replace(path)
    return path


def main():
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    build = Path(sys.argv[1]).resolve()
    build.mkdir(parents=True, exist_ok=True)
    cache = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "suflyor-install/gpu-runtime"
    cache.mkdir(parents=True, exist_ok=True)
    out = build / "speech-gpu"
    out.mkdir(exist_ok=True)
    # Copy only library basenames; archive paths and symlinks are never followed.
    with tarfile.open(download(cache, SHERPA), "r:bz2") as archive:
        for entry in archive:
            name = Path(entry.name).name
            if entry.isfile() and "/lib/" in entry.name and ".so" in name:
                with archive.extractfile(entry) as source, (out / name).open("wb") as dest:
                    shutil.copyfileobj(source, dest)
    with ThreadPoolExecutor(max_workers=3) as workers:
        wheels = list(workers.map(lambda item: download(cache, item), WHEELS))
    for wheel in wheels:
        with zipfile.ZipFile(wheel) as archive:
            for entry in archive.infolist():
                name = Path(entry.filename).name
                if "/lib/" in entry.filename and ".so" in name and not entry.is_dir():
                    with archive.open(entry) as source, (out / name).open("wb") as dest:
                        shutil.copyfileobj(source, dest)
    patchelf = shutil.which("patchelf") or str(cache / "patchelf")
    if not Path(patchelf).is_file():
        raise RuntimeError("Install patchelf before staging the CUDA runtime")
    for path in out.iterdir():
        if ".so" in path.name:
            subprocess.run([patchelf, "--set-rpath", "$ORIGIN", str(path)], check=True)
            soname = subprocess.check_output([patchelf, "--print-soname", str(path)], text=True).strip()
            if soname and Path(soname).name == soname and soname != path.name:
                alias = out / soname
                if not alias.exists():
                    alias.symlink_to(path.name)
    # The shared runtime exports both CPU and CUDA providers. Each process loads
    # exactly one ORT instance; no ONNX runtimes are merged into the shell.
    for name in ("suflyor-tts", "suflyor-teratts", "overlay-host"):
        binary = build / name
        if binary.is_file():
            subprocess.run([patchelf, "--set-rpath", "$ORIGIN/speech-gpu", str(binary)], check=True)
    manifest = {"sherpa": "1.13.5", "onnxruntime": "1.27.1", "cuda": "13.0", "cudnn": "9.16.0.29"}
    (out / "runtime.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print("Speech runtime installed:", out, flush=True)


if __name__ == "__main__":
    main()
