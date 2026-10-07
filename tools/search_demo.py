"""Crawl a loopback fixture, reload the persisted index, and query real results."""
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading

ROOT = Path(__file__).resolve().parents[1]


class FixtureHandler(SimpleHTTPRequestHandler):
    requests = []

    def log_message(self, *_):
        pass

    def do_GET(self):
        self.requests.append(self.path)
        if self.path == "/redirect":
            self.send_response(302)
            self.send_header("Location", "https://example.com/must-not-fetch")
            self.end_headers()
        elif self.path == "/large.html":
            self.send_response(200)
            self.send_header("Content-Type", "text/html")
            self.send_header("Content-Length", str(4 * 1024 * 1024))
            self.end_headers()
        else:
            super().do_GET()


def run_demo(index_path=None):
    subprocess.run(["cargo", "build", "-p", "quill-search", "--locked"], cwd=ROOT, check=True)
    binary = ROOT / "target/debug" / ("quill-search.exe" if os.name == "nt" else "quill-search")
    handler = partial(FixtureHandler, directory=ROOT / "tests/fixtures/site")
    FixtureHandler.requests = []
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        with tempfile.TemporaryDirectory() as temporary:
            path = index_path or Path(temporary) / "index.json"
            path.parent.mkdir(parents=True, exist_ok=True)
            seed = f"http://127.0.0.1:{server.server_port}/"
            subprocess.run([str(binary), "crawl", str(path), seed], check=True)
            output = subprocess.check_output([str(binary), "query", str(path), "rust ownership"], text=True)
            results = json.loads(output)
            assert results[0]["title"] == "Rust ownership guide", results
            assert results[0]["url"] == seed + "rust.html", results
            assert "borrowing" in results[0]["snippet"], results
            persisted = json.loads(path.read_text())
            assert len(persisted["documents"]) == 3, persisted["documents"]
            assert "/private.html" not in FixtureHandler.requests
            assert "/not-followed" not in FixtureHandler.requests
            assert "tracking_secret" not in path.read_text()
            assert not json.loads(subprocess.check_output([str(binary), "query", str(path), "secretword"], text=True))
            print(output)
            print("Search acceptance passed: crawl, robots, canonicalization, deduplication, ranking, persistence, limits.")
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    run_demo(ROOT / ".quill/demo-index.json")
