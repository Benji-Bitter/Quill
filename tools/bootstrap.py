"""Build Quill components, then launch the supplied Gecko development host."""
import argparse
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--firefox", type=Path, required=True)
    args = parser.parse_args()
    if not args.firefox.is_file():
        parser.error("Supply an installed Firefox executable")
    subprocess.run(["cargo", "build", "--workspace", "--release", "--locked"], cwd=ROOT, check=True)
    raise SystemExit(subprocess.call([sys.executable, str(ROOT / "tools/browser.py"), "--firefox", str(args.firefox.resolve())]))
