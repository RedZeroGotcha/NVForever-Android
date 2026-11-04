#!/usr/bin/env python3
"""Apply the M6 controller overlay to a fresh NVForever v0.5.1 source tree.

The Android fork's current main uses a different Bevy API than the published M6
APK. Keep this release-specific source overlay separate from current PC code.
"""
from __future__ import annotations
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import tarfile
import urllib.request
import zipfile

HERE = Path(__file__).resolve().parent
M6_URL = "https://github.com/RedZeroGotcha/NVForever-Android/releases/download/test_play/NVForever-android-m6.zip"
M6_SHA256 = "d4e5e2508868819e13521849fd3baa4724cd1b6beeeba87b0ad344600f9f6744"
WINIT_URL = "https://static.crates.io/crates/winit/winit-0.30.13.crate"
WINIT_SHA256 = "a6755fa58a9f8350bd1e472d4c3fcc25f824ec358933bba33306d0b63df5978d"


def archive_bytes(path: Path | None, url: str, expected: str) -> bytes:
    if path is None:
        request = urllib.request.Request(url, headers={"User-Agent": "NVForever-M6-controller-overlay"})
        with urllib.request.urlopen(request, timeout=60) as response:
            data = response.read()
    else:
        data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != expected:
        raise ValueError(f"Archive SHA-256 mismatch: {path or url}")
    return data


def member_path(root: Path, name: str) -> Path:
    relative = PurePosixPath(name)
    if relative.is_absolute() or ".." in relative.parts or "\\" in name or ":" in name:
        raise ValueError(f"Unsafe archive member: {name}")
    target = root.joinpath(*relative.parts)
    if not target.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"Archive member leaves destination: {name}")
    return target


def validate_base(root: Path) -> None:
    for name, expected in json.loads((HERE / "base-files.json").read_text()).items():
        file = member_path(root, name)
        if not file.is_file():
            raise ValueError(f"Missing v0.5.1 base file: {name}")
        actual = hashlib.sha256(file.read_bytes().replace(b"\r\n", b"\n")).hexdigest()
        if actual != expected:
            raise ValueError(f"Expected a fresh v0.5.1 tree; {name} differs. Current main is not supported.")
    for name in ["android", "engine/nvf-android", "engine/nvf-android-input", "engine/vendor/winit"]:
        if (root / name).exists():
            raise ValueError(f"Refusing to replace existing Android sources: {name}")


def apply(root: Path, m6: bytes, winit: bytes) -> None:
    validate_base(root)
    # Validate every member before the first write, including archive link types.
    with zipfile.ZipFile(io.BytesIO(m6)) as archive:
        for info in archive.infolist():
            member_path(root, info.filename)
            if (info.external_attr >> 16) & 0o170000 == 0o120000:
                raise ValueError(f"Archive symlink is not supported: {info.filename}")
        with tarfile.open(fileobj=io.BytesIO(winit), mode="r:gz") as crate:
            members = []
            for info in crate.getmembers():
                parts = PurePosixPath(info.name).parts
                if not parts or parts[0] != "winit-0.30.13" or not (info.isfile() or info.isdir()):
                    raise ValueError(f"Unexpected winit member: {info.name}")
                relative = str(PurePosixPath(*parts[1:]))
                target = member_path(root / "engine/vendor/winit", relative)
                members.append((info, target))
            for info in archive.infolist():
                target = member_path(root, info.filename)
                if info.is_dir():
                    target.mkdir(parents=True, exist_ok=True)
                else:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(archive.read(info))
            for info, target in members:
                if info.isdir():
                    target.mkdir(parents=True, exist_ok=True)
                else:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    source = crate.extractfile(info)
                    assert source is not None
                    target.write_bytes(source.read())
    patch = HERE / "controls.patch"
    subprocess.run(["git", "apply", "--no-index", "--check", str(patch)], cwd=root, check=True)
    subprocess.run(["git", "apply", "--no-index", str(patch)], cwd=root, check=True)
    shutil.copytree(HERE / "overlay", root, dirs_exist_ok=True)
    patch = HERE / "winit-0.30.13.patch"
    subprocess.run(["git", "apply", "--no-index", "--directory=engine/vendor/winit", "--check", str(patch)], cwd=root, check=True)
    subprocess.run(["git", "apply", "--no-index", "--directory=engine/vendor/winit", str(patch)], cwd=root, check=True)
    validate_result(root)
    print(f"Applied and verified M6 console-controller overlay to {root}")


def validate_result(root: Path) -> None:
    # A successful git apply from a subdirectory can skip every path. Check the
    # reconstructed files as well as the pinned inputs, so that cannot pass CI.
    for name, expected in json.loads((HERE / "result-files.json").read_text()).items():
        file = member_path(root, name)
        if not file.is_file() or hashlib.sha256(file.read_bytes()).hexdigest() != expected:
            raise ValueError(f"Reconstructed source differs: {name}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=Path, help="Fresh NVForever v0.5.1 source tree, not current main")
    parser.add_argument("--m6-archive", type=Path, help="Optional cached original M6 release ZIP")
    parser.add_argument("--winit-archive", type=Path, help="Optional cached winit 0.30.13 crate")
    args = parser.parse_args()
    root = args.destination.resolve()
    validate_base(root)
    m6 = archive_bytes(args.m6_archive, M6_URL, M6_SHA256)
    winit = archive_bytes(args.winit_archive, WINIT_URL, WINIT_SHA256)
    apply(root, m6, winit)


if __name__ == "__main__":
    main()
