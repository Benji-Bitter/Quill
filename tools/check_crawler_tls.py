"""Verify that the crawler rejects an untrusted local TLS certificate."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import os
import shutil
import ssl
import subprocess
import tempfile
import threading

ROOT = Path(__file__).resolve().parents[1]

class Handler(BaseHTTPRequestHandler):
    requests = 0

    def do_GET(self):
        Handler.requests += 1
        self.send_response(404)
        self.end_headers()

    def log_message(self, *_):
        pass


def main():
    openssl = shutil.which("openssl")
    if not openssl:
        print("TLS rejection check skipped: OpenSSL test-certificate tool unavailable")
        return
    binary = ROOT / "target/debug" / ("quill-search.exe" if os.name == "nt" else "quill-search")
    with tempfile.TemporaryDirectory() as temporary:
        directory = Path(temporary)
        certificate, key = directory / "certificate.pem", directory / "key.pem"
        subprocess.run([openssl, "req", "-x509", "-newkey", "rsa:2048", "-nodes",
                        "-keyout", str(key), "-out", str(certificate), "-days", "1",
                        "-subj", "/CN=localhost"], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(certificate, key)
        server.socket = context.wrap_socket(server.socket, server_side=True)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            result = subprocess.run([str(binary), "crawl", str(directory / "index.json"),
                                     f"https://localhost:{server.server_port}/"], capture_output=True, text=True, timeout=30)
            assert result.returncode != 0, result.stdout
            assert "InvalidCertificate" in result.stderr, result.stderr
            assert Handler.requests == 0, "Untrusted TLS connection reached the HTTP handler"
            assert not (directory / "index.json").exists()
            print("Crawler rejected the untrusted certificate before sending an HTTP request")
        finally:
            server.shutdown()
            server.server_close()
            thread.join()

if __name__ == "__main__":
    main()
