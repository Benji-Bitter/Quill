import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from browser import package_extension, prepare


class BrowserTools(unittest.TestCase):
    def test_refuses_existing_profile(self):
        with tempfile.TemporaryDirectory() as path:
            profile = Path(path)
            (profile / "prefs.js").write_text("user data")
            with self.assertRaises(RuntimeError):
                prepare(profile, 2828)
            self.assertEqual((profile / "prefs.js").read_text(), "user data")

    def test_package_preserves_branding_and_shared_rules(self):
        with tempfile.TemporaryDirectory() as path:
            extension = prepare(Path(path), 2828)
            with zipfile.ZipFile(extension) as archive:
                for name in ["black icon.png", "white icon.png"]:
                    self.assertEqual(archive.read(name), (ROOT / "assets/branding" / name).read_bytes())
                manifest = json.loads(archive.read("manifest.json"))
                self.assertEqual(manifest["incognito"], "not_allowed")
                self.assertIn("utm_source", archive.read("tracking-parameters.js").decode())
            prefs = (Path(path) / "user.js").read_text()
            self.assertIn('"datareporting.policy.dataSubmissionEnabled", false', prefs)
            self.assertNotIn("security.sandbox.content.level", prefs)

    def test_quill_modules_have_no_network_dependency_except_crawler(self):
        # Source architecture check, not a claim about engine network behavior.
        for crate in ["privacy", "vault", "updater", "search/index"]:
            manifest = (ROOT / crate / "Cargo.toml").read_text()
            self.assertNotIn("reqwest", manifest)
            self.assertNotIn("libp2p", manifest)


if __name__ == "__main__":
    unittest.main()
