"""Exercise Quill's embedded WebKit against local pages and optional live HTTPS."""
import subprocess
import uuid
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class Pages(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_GET(self):
        title = "Two" if self.path == "/two" else "One"
        body = f'<title>{title}</title><body data-load="{uuid.uuid4()}"><p>Real website content</p><a href="/two" target="_blank">Next</a>'.encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/html")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

binary = ROOT / ".quill/browser-view-smoke"
binary.parent.mkdir(exist_ok=True)
sources = [p for p in (ROOT / "search/desktop/macos").glob("*.swift") if p.name != "QuillSearch.swift"]
subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "5", *map(str, sources),
                str(ROOT / "tests/browser_view_smoke.swift"), "-o", str(binary)], check=True)
server = ThreadingHTTPServer(("127.0.0.1", 0), Pages)
thread = threading.Thread(target=server.serve_forever, daemon=True)
thread.start()
try:
    subprocess.run([str(binary), f"http://127.0.0.1:{server.server_port}",
                    f"https://127.0.0.1:{server.server_port}/unavailable", *sys.argv[1:]], check=True, timeout=120)
finally:
    server.shutdown()
    server.server_close()
    thread.join()
