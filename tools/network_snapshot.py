"""Inspect current browser-tree sockets, not a packet capture or zero-traffic proof."""
import argparse
import shutil
import subprocess
from process_metrics import snapshot


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pid", type=int, required=True)
    args = parser.parse_args()
    if not shutil.which("lsof"):
        parser.error("lsof is required (macOS/Linux)")
    pids = snapshot(args.pid)
    if args.pid not in pids:
        parser.error("process is not running")
    result = subprocess.run(["lsof", "-nP", "-a", "-p", ",".join(map(str, sorted(pids))), "-i"], text=True, capture_output=True)
    if result.returncode not in (0, 1):
        raise RuntimeError(result.stderr)
    print(result.stdout or "No sockets visible in this snapshot. Short-lived and OS-delegated traffic may be missed.")
    if result.stderr:
        print(result.stderr)


if __name__ == "__main__":
    main()
