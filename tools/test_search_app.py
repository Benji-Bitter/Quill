#!/usr/bin/env python3
"""Exercise the native app model against the real Rust executable."""
import subprocess
import threading
from functools import partial
from http.server import ThreadingHTTPServer
from search_demo import FixtureHandler
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
output = ROOT / ".quill/search-app-smoke"
output.parent.mkdir(exist_ok=True)
subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "5",
                str(ROOT / "search/desktop/macos/SearchModel.swift"),
                str(ROOT / "tests/search_app_smoke.swift"), "-o", str(output)], check=True)
server = ThreadingHTTPServer(("127.0.0.1", 0), partial(FixtureHandler, directory=ROOT / "tests/fixtures/site"))
thread = threading.Thread(target=server.serve_forever, daemon=True)
thread.start()
try:
    subprocess.run([str(output), str(ROOT / "target/release/quill-search"),
                    str(ROOT / ".quill/demo-index.json"), "rust ownership",
                    f"http://127.0.0.1:{server.server_port}/"], check=True)
finally:
    server.shutdown()
    server.server_close()
    thread.join()
