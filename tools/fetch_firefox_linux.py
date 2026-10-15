"""Fetch a checksum-verified Mozilla development host for Linux CI.

Uses Mozilla HTTPS metadata, not Quill update trust. Never a production updater.
"""
import hashlib
import json
from pathlib import Path
import platform
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def fetch():
    if platform.system() != "Linux" or platform.machine() not in {"x86_64", "aarch64"}:
        raise RuntimeError("This helper supports Linux x86-64/aarch64 only")
    version = json.loads((ROOT / "browser/engine-reference.json").read_text())["version"]
    architecture = "linux-x86_64" if platform.machine() == "x86_64" else "linux-aarch64"
    relative = f"{architecture}/en-US/firefox-{version}.tar.xz"
    base = f"https://archive.mozilla.org/pub/firefox/releases/{version}/"
    with urllib.request.urlopen(base + "SHA256SUMS", timeout=30) as response:
        lines = response.read(2 * 1024 * 1024).decode().splitlines()
    expected = next(line.split()[0] for line in lines if line.split(maxsplit=1)[-1].strip() == relative)
    destination = ROOT / ".quill/ci-engine"
    destination.mkdir(parents=True, exist_ok=True)
    archive = destination / "firefox.tar.xz"
    digest = hashlib.sha256()
    with urllib.request.urlopen(base + relative, timeout=30) as response, archive.open("wb") as file:
        size = 0
        while chunk := response.read(1024 * 1024):
            size += len(chunk)
            if size > 300 * 1024 * 1024:
                raise RuntimeError("Engine archive exceeds download limit")
            digest.update(chunk)
            file.write(chunk)
    if digest.hexdigest() != expected:
        archive.unlink()
        raise RuntimeError("Mozilla archive checksum mismatch")
    with tarfile.open(archive) as source:
        source.extractall(destination, filter="data")
    print(destination / "firefox/firefox")


if __name__ == "__main__":
    fetch()
