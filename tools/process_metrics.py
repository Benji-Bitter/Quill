"""Sample a process tree using ps on macOS/Linux; no extra runtime dependency."""
import subprocess
import sys
import time


def cpu_seconds(value):
    days, _, value = value.rpartition("-")
    parts = [float(part) for part in value.split(":")]
    seconds = sum(number * 60 ** i for i, number in enumerate(reversed(parts)))
    return seconds + (int(days) * 86400 if days else 0)


def snapshot(root):
    if sys.platform == "win32":
        raise RuntimeError("Process sampling currently supports macOS/Linux only")
    output = subprocess.check_output(["ps", "-axo", "pid=,ppid=,rss=,time="], text=True)
    rows = {}
    for line in output.splitlines():
        pid, parent, rss, cpu = line.split()
        rows[int(pid)] = (int(parent), int(rss), cpu_seconds(cpu))
    selected = {root}
    while True:
        children = {pid for pid, row in rows.items() if row[0] in selected}
        if children <= selected:
            break
        selected |= children
    return {pid: rows[pid] for pid in selected if pid in rows}


def measure_idle(root, seconds=5):
    before = snapshot(root)
    start = time.monotonic()
    time.sleep(seconds)
    after = snapshot(root)
    elapsed = time.monotonic() - start
    # Ignore exited processes rather than presenting a misleading negative delta.
    cpu = sum(max(0, row[2] - before.get(pid, (0, 0, 0))[2]) for pid, row in after.items())
    return {"sample_seconds": elapsed, "processes": len(after),
            "tree_rss_kib": sum(row[1] for row in after.values()),
            "cpu_seconds_during_sample": cpu, "cpu_percent_one_core": cpu / elapsed * 100,
            "note": "RSS double-counts shared pages; exited processes are omitted; automation host, not release performance."}
