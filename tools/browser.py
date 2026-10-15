"""Launch a real Firefox development host with a separate Quill profile."""
import argparse
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import time
import zipfile

from marionette import Marionette

ROOT = Path(__file__).resolve().parents[1]


def package_extension(destination):
    with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED) as archive:
        for path in sorted((ROOT / "browser/extension").iterdir()):
            if path.is_file():
                archive.write(path, path.name)
        for name in ["black icon.png", "white icon.png"]:
            archive.write(ROOT / "assets/branding" / name, name)
        parameters = json.loads((ROOT / "privacy/tracking-parameters.json").read_text())
        archive.writestr("tracking-parameters.js", f"const TRACKING_PARAMETERS = {json.dumps(parameters)};\n")


def prepare(profile, port):
    profile.mkdir(parents=True, exist_ok=True, mode=0o700)
    marker = profile / "quill-development-profile"
    if not marker.exists() and any(profile.iterdir()):
        raise RuntimeError("Refusing to modify an existing non-Quill profile")
    marker.touch()
    prefs = json.loads((ROOT / "browser/preferences.json").read_text())
    prefs.update({"marionette.port": port, "remote.prefs.recommended": False})
    (profile / "user.js").write_text("\n".join(f"user_pref({json.dumps(k)}, {json.dumps(v)});" for k, v in prefs.items()) + "\n")
    (profile / "chrome").mkdir(exist_ok=True)
    shutil.copyfile(ROOT / "browser/userChrome.css", profile / "chrome/userChrome.css")
    extension = profile / "quill.xpi"
    package_extension(extension)
    homepage = profile / "homepage"
    homepage.mkdir(exist_ok=True)
    with zipfile.ZipFile(extension) as archive:
        # Only extract our locally generated archive, never untrusted downloads.
        archive.extractall(homepage)
    return extension


def launch(binary, profile, headless=False):
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    extension = prepare(profile, port)
    args = [str(binary), "--no-remote", "--profile", str(profile), "--marionette", "--remote-allow-system-access"]
    if headless:
        args.append("--headless")
    args.append("about:blank")
    log = open(profile / "engine.log", "w")
    started = time.perf_counter()
    process = subprocess.Popen(args, stdout=log, stderr=log)
    log.close()
    client = None
    try:
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError("Firefox exited; inspect the profile's engine.log")
            try:
                client = Marionette(port)
                break
            except (ConnectionRefusedError, TimeoutError, OSError):
                time.sleep(0.1)
        if client is None:
            raise RuntimeError("Firefox automation did not start within 45 seconds")
        client.command("WebDriver:NewSession", {"capabilities": {"alwaysMatch": {"acceptInsecureCerts": False}}})
        extension_error = None
        try:
            client.command("Addon:Install", {"path": str(extension), "temporary": True, "allowPrivateBrowsing": False})
        except RuntimeError as error:
            extension_error = str(error)
        destination = (profile / "homepage/newtab.html").as_uri()
        if extension_error is None:
            # The new-tab button uses this URL; about:newtab remains Firefox's page.
            client.command("Marionette:SetContext", {"value": "chrome"})
            try:
                deadline = time.monotonic() + 10
                while time.monotonic() < deadline:
                    configured = client.command("WebDriver:ExecuteScript", {
                        "script": "return BROWSER_NEW_TAB_URL;", "args": []
                    })["value"]
                    if configured.startswith("moz-extension://"):
                        destination = configured
                        break
                    time.sleep(0.1)
                else:
                    extension_error = "Quill installed but its new-tab override did not become active"
            finally:
                client.command("Marionette:SetContext", {"value": "content"})
        client.command("WebDriver:Navigate", {"url": destination})
        return process, client, time.perf_counter() - started, extension_error
    except BaseException:
        if client:
            client.close()
        process.terminate()
        process.wait(timeout=15)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--firefox", required=True, type=Path)
    parser.add_argument("--profile", type=Path, default=ROOT / ".quill/browser-profile")
    args = parser.parse_args()
    if not args.firefox.is_file():
        parser.error("Firefox executable does not exist")
    args.profile = args.profile.resolve()
    args.profile.mkdir(parents=True, exist_ok=True, mode=0o700)
    # Store the lock outside the profile so first-run validation still rejects foreign profiles.
    lock_path = args.profile.with_name(args.profile.name + ".launcher-lock")
    try:
        lock_fd = os.open(lock_path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    except FileExistsError:
        parser.error("Launcher lock exists. Close the running host; remove a stale lock only after checking it is stopped.")
    os.close(lock_fd)
    try:
        return supervise(args)
    finally:
        lock_path.unlink(missing_ok=True)


def supervise(args):
    process, client, startup, extension_error = launch(args.firefox.resolve(), args.profile)
    # No other client should be allowed to attach to this development session.
    try:
        client.command("Marionette:AcceptConnections", {"value": False})
        client.command("WebDriver:DeleteSession")
    except BaseException:
        process.terminate()
        raise
    finally:
        client.close()
    print(f"Quill development host started in {startup:.3f}s (PID {process.pid}).")
    if extension_error:
        print("Extension unavailable: " + extension_error)
        print("Running homepage-only fallback. URL cleaning and local history search are unavailable.")
    print("Firefox engine services are not yet fully audited. Keep this host updated.")
    try:
        return process.wait()
    except KeyboardInterrupt:
        process.terminate()
        return process.wait(timeout=15)


if __name__ == "__main__":
    raise SystemExit(main())
