"""Exercise the real Gecko host; optional HTTPS check makes an explicit request."""
import argparse
import base64
from functools import partial
from http.server import ThreadingHTTPServer
import json
from pathlib import Path
import statistics
import socket
import tempfile
import threading
import time

from browser import ROOT, launch
from search_demo import FixtureHandler
from process_metrics import measure_idle


def execute(client, script):
    return client.command("WebDriver:ExecuteScript", {"script": script, "args": [], "newSandbox": True, "sandbox": "default"})["value"]


def wait_title(client, expected):
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        title = client.command("WebDriver:GetTitle")["value"]
        if title == expected:
            return
        time.sleep(0.05)
    raise AssertionError(f"Expected {expected!r}, got {title!r} at {client.command('WebDriver:GetCurrentURL')}")


def run(binary, output, https=False, require_extension=False):
    output.mkdir(parents=True, exist_ok=True)
    server = ThreadingHTTPServer(("127.0.0.1", 0), partial(FixtureHandler, directory=ROOT / "tests/fixtures/site"))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        with tempfile.TemporaryDirectory(prefix="quill-browser-") as temporary:
            process, client, startup, extension_error = launch(binary, Path(temporary), headless=True)
            try:
                if require_extension and extension_error:
                    raise RuntimeError(extension_error)
                print("Browser started; checking homepage", flush=True)
                title = client.command("WebDriver:GetTitle")["value"]
                if title != "Quill":
                    raise AssertionError(f"New-tab extension did not load: {title}")
                assert execute(client, "return document.querySelector('img').naturalWidth > 0")
                screenshot = client.command("WebDriver:TakeScreenshot", {"full": False})["value"]
                (output / "newtab.png").write_bytes(base64.b64decode(screenshot))
                homepage = client.command("WebDriver:GetCurrentURL")["value"]
                client.command("Marionette:SetContext", {"value": "chrome"})
                chrome = client.command("WebDriver:TakeScreenshot", {"full": False})["value"]
                (output / "browser-chrome.png").write_bytes(base64.b64decode(chrome))
                client.command("Marionette:SetContext", {"value": "content"})
                time.sleep(15)
                idle = measure_idle(process.pid)
                idle["settling_seconds"] = 15
                seed = f"http://127.0.0.1:{server.server_port}"
                client.command("WebDriver:Navigate", {"url": seed + "/rust.html?utm_source=smoke&keep=1"})
                current = client.command("WebDriver:GetCurrentURL")["value"]
                assert current == seed + ("/rust.html?keep=1" if extension_error is None else "/rust.html?utm_source=smoke&keep=1"), current
                client.command("WebDriver:Navigate", {"url": seed + "/garden.html"})
                client.command("WebDriver:Back")
                assert client.command("WebDriver:GetTitle")["value"] == "Rust ownership guide"
                client.command("WebDriver:Forward")
                assert client.command("WebDriver:GetTitle")["value"] == "Growing a garden"
                client.command("WebDriver:Refresh")
                print("Navigation and history passed; checking tabs", flush=True)
                samples = []
                for sample in range(5):
                    print(f"Tab sample {sample + 1}", flush=True)
                    started = time.perf_counter()
                    before = set(client.command("WebDriver:GetWindowHandles"))
                    client.command("WebDriver:NewWindow", {"type": "tab", "focus": False})
                    # Some engine builds return the old handle from NewWindow.
                    created = set(client.command("WebDriver:GetWindowHandles")) - before
                    assert len(created) == 1, created
                    handle = created.pop()
                    client.command("WebDriver:SwitchToWindow", {"handle": handle, "focus": False})
                    client.command("WebDriver:Navigate", {"url": homepage})
                    wait_title(client, "Quill")
                    samples.append((time.perf_counter() - started) * 1000)
                    remaining = client.command("WebDriver:CloseWindow")
                    client.command("WebDriver:SwitchToWindow", {"handle": remaining[0], "focus": False})
                if https:
                    print("Checking HTTPS", flush=True)
                    client.command("WebDriver:Navigate", {"url": "https://example.com/"})
                    assert client.command("WebDriver:GetTitle")["value"] == "Example Domain"
                port = client.socket.getpeername()[1]
                client.command("Marionette:AcceptConnections", {"value": False})
                try:
                    probe = socket.create_connection(("127.0.0.1", port), timeout=1)
                except ConnectionRefusedError:
                    pass
                else:
                    probe.close()
                    raise AssertionError("Automation listener still accepts new connections")
                measurements = {"mode": "headless; automation round trips included", "startup_to_newtab_seconds": startup, "newtab_samples_ms": samples, "newtab_median_ms": statistics.median(samples), "https_checked": https, "idle": idle, "extension_error": extension_error}
                (output / "browser.json").write_text(json.dumps(measurements, indent=2) + "\n")
                print(json.dumps(measurements, indent=2))
                print("Browser smoke passed: branding, navigation, back/forward, reload, tabs. See extension_error for URL-cleaning coverage.")
            finally:
                try:
                    client.command("Marionette:Quit", {"flags": ["eForceQuit"]})
                except (RuntimeError, OSError):
                    process.terminate()
                client.close()
                process.wait(timeout=15)
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--firefox", type=Path, required=True)
    parser.add_argument("--https", action="store_true")
    parser.add_argument("--require-extension", action="store_true")
    parser.add_argument("--output", type=Path, default=ROOT / ".quill/measurements")
    args = parser.parse_args()
    run(args.firefox.resolve(), args.output, args.https, args.require_extension)
