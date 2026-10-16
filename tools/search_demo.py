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
    robots_redirect = None

    def log_message(self, *_):
        pass

    def do_GET(self):
        self.requests.append(self.path)
        if self.path == "/robots.txt" and self.robots_redirect:
            self.send_response(302)
            self.send_header("Location", self.robots_redirect)
            self.end_headers()
        elif self.path == "/robots-policy.txt":
            data = (ROOT / "tests/fixtures/site/robots.txt").read_bytes()
            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
        elif self.path == "/redirect":
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
            FixtureHandler.robots_redirect = "/robots-policy.txt"
            FixtureHandler.requests = []
            subprocess.run([str(binary), "crawl", str(Path(temporary) / "redirect-index.json"), seed], check=True, capture_output=True)
            assert "/robots-policy.txt" in FixtureHandler.requests
            assert "/private.html" not in FixtureHandler.requests
            FixtureHandler.robots_redirect = "https://example.com/robots.txt"
            rejected = subprocess.run([str(binary), "crawl", str(Path(temporary) / "rejected-index.json"), seed], capture_output=True, text=True)
            assert rejected.returncode != 0 and "leaves the seed origin" in rejected.stderr
            FixtureHandler.robots_redirect = None
            print(output)
            print("Search acceptance passed: crawl, robots, canonicalization, deduplication, ranking, persistence, limits.")
    finally:
        FixtureHandler.robots_redirect = None
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    run_demo(ROOT / ".quill/demo-index.json")
