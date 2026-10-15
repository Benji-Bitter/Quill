#!/usr/bin/env python3
"""Build the standalone macOS search app; local search and embedded system WebKit."""
import argparse
import plistlib
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--open", action="store_true")
    args = parser.parse_args()
    subprocess.run(["cargo", "build", "--release", "--locked", "-p", "quill-search"], cwd=ROOT, check=True)
    app = ROOT / "dist/Quill Search.app"
    binary = app / "Contents/MacOS"
    resources = app / "Contents/Resources"
    binary.mkdir(parents=True, exist_ok=True)
    resources.mkdir(parents=True, exist_ok=True)
    subprocess.run(["xcrun", "swiftc", "-O", "-parse-as-library", "-swift-version", "5",
                    *map(str, sorted((ROOT / "search/desktop/macos").glob("*.swift"))),
                    "-o", str(binary / "QuillSearch")], check=True)
    shutil.copy2(ROOT / "target/release/quill-search", binary / "quill-search")
    for name in ("black icon.png", "white icon.png"):
        shutil.copy2(ROOT / "assets/branding" / name, resources / name)
    with (app / "Contents/Info.plist").open("wb") as file:
        plistlib.dump({
            "CFBundleExecutable": "QuillSearch", "CFBundleIdentifier": "org.quill.search.local",
            "CFBundleName": "Quill Search", "CFBundleDisplayName": "Quill Search",
            "CFBundlePackageType": "APPL", "CFBundleShortVersionString": "0.1.0",
            "CFBundleVersion": "1", "LSMinimumSystemVersion": "14.0",
            "NSHighResolutionCapable": True,
        }, file)
    subprocess.run(["codesign", "--force", "--deep", "--sign", "-", str(app)], check=True)
    print(app)
    if args.open:
        subprocess.run(["open", str(app)], check=True)


if __name__ == "__main__":
    main()
