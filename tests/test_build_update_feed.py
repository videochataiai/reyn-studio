import base64
import copy
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import build_update_feed as feed  # noqa: E402

try:
    from cryptography.hazmat.primitives import serialization
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
except ImportError:
    serialization = None
    Ed25519PrivateKey = None


TEST_SEED = bytes(range(32))
ALLOWED_HOSTS = ("updates.reynflow.test",)


@unittest.skipIf(Ed25519PrivateKey is None, "cryptography is not installed")
class UpdateFeedTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.macos = self.root / "Reyn-Studio-0.3.0-build.7-arm64.app.zip"
        self.windows = self.root / "Reyn-Studio-0.3.0-windows-x64.zip"
        self.macos.write_bytes(b"deterministic macOS archive\n")
        self.windows.write_bytes(b"deterministic Windows archive\n")
        private_key = Ed25519PrivateKey.from_private_bytes(TEST_SEED)
        public_key = private_key.public_key().public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        )
        self.public_key_b64 = base64.b64encode(public_key).decode("ascii")

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def make_feed(self) -> dict:
        return feed.build_feed(
            version="0.3.0",
            release_sequence=300,
            published=1_800_000_000,
            expires=1_800_604_800,
            minimum_updater_version="0.2.0",
            channel="stable",
            changelog_url="https://updates.reynflow.test/changelog/0.3.0",
            key_id="reyn-update-2026-01",
            macos_path=self.macos,
            macos_minimum_os="14.0",
            macos_url=f"https://updates.reynflow.test/downloads/{self.macos.name}",
            windows_path=self.windows,
            windows_minimum_os="11",
            windows_url=f"https://updates.reynflow.test/downloads/{self.windows.name}",
            developer_id_signed=True,
            notarized=True,
            authenticode_signed=True,
            allowed_hosts=ALLOWED_HOSTS,
        )

    def test_build_uses_actual_package_size_and_hash(self) -> None:
        document = self.make_feed()
        self.assertEqual(
            [artifact["platform"] for artifact in document["artifacts"]],
            ["macos-arm64", "windows-x64"],
        )
        macos = document["artifacts"][0]
        windows = document["artifacts"][1]
        self.assertEqual(macos["bytes"], len(self.macos.read_bytes()))
        self.assertEqual(
            macos["sha256"], hashlib.sha256(self.macos.read_bytes()).hexdigest()
        )
        self.assertTrue(macos["developer_id_signed"])
        self.assertTrue(macos["notarized"])
        self.assertFalse(macos["authenticode_signed"])
        self.assertEqual(windows["bytes"], len(self.windows.read_bytes()))
        self.assertTrue(windows["authenticode_signed"])
        self.assertFalse(windows["developer_id_signed"])
        self.assertFalse(windows["notarized"])

    def test_canonical_json_is_deterministic_and_whitespace_free(self) -> None:
        document = self.make_feed()
        first = feed.canonical_json_bytes(document)
        second = feed.canonical_json_bytes(dict(reversed(list(document.items()))))
        self.assertEqual(first, second)
        self.assertFalse(first.endswith(b"\n"))
        self.assertNotIn(b": ", first)
        self.assertEqual(feed.parse_canonical_json(first, description="feed"), document)

    def test_sign_and_verify_round_trip_and_tamper_rejection(self) -> None:
        document = self.make_feed()
        raw = feed.canonical_json_bytes(document)
        signature = feed.sign_feed(
            raw, key_id=document["key_id"], private_seed=TEST_SEED
        )
        self.assertEqual(
            feed.verify_feed(
                raw,
                signature,
                expected_public_key_b64=self.public_key_b64,
                allowed_hosts=ALLOWED_HOSTS,
            ),
            document,
        )
        tampered = raw.replace(b'"release_sequence":300', b'"release_sequence":301')
        with self.assertRaisesRegex(ValueError, "signature verification failed"):
            feed.verify_feed(
                tampered,
                signature,
                expected_public_key_b64=self.public_key_b64,
                allowed_hosts=ALLOWED_HOSTS,
            )

    def test_verify_rejects_noncanonical_json_even_with_valid_signature(self) -> None:
        document = self.make_feed()
        noncanonical = json.dumps(document, indent=2, sort_keys=True).encode()
        signature = feed.sign_feed(
            noncanonical, key_id=document["key_id"], private_seed=TEST_SEED
        )
        with self.assertRaisesRegex(ValueError, "not canonical JSON"):
            feed.verify_feed(
                noncanonical,
                signature,
                expected_public_key_b64=self.public_key_b64,
                allowed_hosts=ALLOWED_HOSTS,
            )

    def test_validation_rejects_unknown_and_missing_fields_at_every_level(self) -> None:
        document = self.make_feed()
        document["unexpected"] = True
        with self.assertRaisesRegex(ValueError, "unknown fields: unexpected"):
            feed.validate_feed(document, allowed_hosts=ALLOWED_HOSTS)

        document = self.make_feed()
        document["artifacts"][0]["unexpected"] = True
        with self.assertRaisesRegex(ValueError, "unknown fields: unexpected"):
            feed.validate_feed(document, allowed_hosts=ALLOWED_HOSTS)

        document = self.make_feed()
        del document["artifacts"][0]["bytes"]
        with self.assertRaisesRegex(ValueError, "missing fields: bytes"):
            feed.validate_feed(document, allowed_hosts=ALLOWED_HOSTS)

    def test_validation_rejects_invalid_version_sequence_expiry_and_hash(self) -> None:
        mutations = (
            ("version", "v0.3.0", "canonical major.minor.patch"),
            ("release_sequence", 0, "positive integer"),
            ("expires", 1_800_000_000, "greater than published"),
        )
        for field_name, value, message in mutations:
            with self.subTest(field=field_name):
                document = self.make_feed()
                document[field_name] = value
                with self.assertRaisesRegex(ValueError, message):
                    feed.validate_feed(document, allowed_hosts=ALLOWED_HOSTS)

        document = self.make_feed()
        document["artifacts"][0]["sha256"] = "A" * 64
        with self.assertRaisesRegex(ValueError, "lowercase SHA-256"):
            feed.validate_feed(document, allowed_hosts=ALLOWED_HOSTS)

    def test_previous_feed_enforces_monotonic_sequence_and_publication(self) -> None:
        previous = self.make_feed()
        current = self.make_feed()
        current["release_sequence"] = 301
        current["published"] += 1
        current["expires"] += 1
        feed.validate_release_progression(
            current, previous, allowed_hosts=ALLOWED_HOSTS
        )

        current["release_sequence"] = 300
        with self.assertRaisesRegex(ValueError, "greater than the previous"):
            feed.validate_release_progression(
                current, previous, allowed_hosts=ALLOWED_HOSTS
            )
        current["release_sequence"] = 301
        current["published"] = previous["published"]
        with self.assertRaisesRegex(ValueError, "previous feed timestamp"):
            feed.validate_release_progression(
                current, previous, allowed_hosts=ALLOWED_HOSTS
            )

    def test_validation_rejects_non_https_and_unapproved_urls(self) -> None:
        for url in (
            "http://updates.reynflow.test/changelog/0.3.0",
            "https://evil.example/changelog/0.3.0",
            "https://updates.reynflow.test/changelog/0.3.0?redirect=evil",
        ):
            with self.subTest(url=url):
                document = self.make_feed()
                document["changelog_url"] = url
                with self.assertRaisesRegex(ValueError, "approved host"):
                    feed.validate_feed(document, allowed_hosts=ALLOWED_HOSTS)

    def test_validation_rejects_wrong_archive_filename_and_url(self) -> None:
        document = self.make_feed()
        document["artifacts"][0]["archive_name"] = (
            "Reyn-Studio-0.2.0-build.7-arm64.app.zip"
        )
        with self.assertRaisesRegex(ValueError, "invalid for 0.3.0"):
            feed.validate_feed(document, allowed_hosts=ALLOWED_HOSTS)

        document = self.make_feed()
        document["artifacts"][1]["url"] = (
            "https://updates.reynflow.test/downloads/not-the-archive.zip"
        )
        with self.assertRaisesRegex(ValueError, "must end with archive_name"):
            feed.validate_feed(document, allowed_hosts=ALLOWED_HOSTS)

    def test_validation_requires_exact_platform_records(self) -> None:
        document = self.make_feed()
        document["artifacts"][1] = copy.deepcopy(document["artifacts"][0])
        with self.assertRaisesRegex(ValueError, "exactly macos-arm64 and windows-x64"):
            feed.validate_feed(document, allowed_hosts=ALLOWED_HOSTS)

    def test_missing_package_is_rejected(self) -> None:
        self.macos.unlink()
        with self.assertRaisesRegex(ValueError, "does not exist"):
            self.make_feed()

    def test_signing_seed_sources_are_strict(self) -> None:
        encoded = base64.b64encode(TEST_SEED).decode("ascii")
        self.assertEqual(
            feed.load_private_seed(None, {"REYN_UPDATE_SIGNING_KEY_B64": encoded}),
            TEST_SEED,
        )
        key_file = self.root / "seed"
        key_file.write_bytes(TEST_SEED)
        self.assertEqual(feed.load_private_seed(key_file, {}), TEST_SEED)
        with self.assertRaisesRegex(ValueError, "not both"):
            feed.load_private_seed(
                key_file, {"REYN_UPDATE_SIGNING_KEY_B64": encoded}
            )
        with self.assertRaisesRegex(ValueError, "missing signing seed"):
            feed.load_private_seed(None, {})
        with self.assertRaisesRegex(ValueError, "exactly 32 bytes"):
            feed.load_private_seed(
                None,
                {"REYN_UPDATE_SIGNING_KEY_B64": base64.b64encode(b"short").decode()},
            )

    def test_signature_document_rejects_wrong_key_and_unknown_fields(self) -> None:
        document = self.make_feed()
        raw = feed.canonical_json_bytes(document)
        signature_raw = feed.sign_feed(
            raw, key_id=document["key_id"], private_seed=TEST_SEED
        )
        signature = json.loads(signature_raw)
        signature["key_id"] = "another-key"
        with self.assertRaisesRegex(ValueError, "key_id does not match"):
            feed.verify_feed(
                raw,
                feed.canonical_json_bytes(signature),
                expected_public_key_b64=self.public_key_b64,
                allowed_hosts=ALLOWED_HOSTS,
            )
        signature["key_id"] = document["key_id"]
        signature["unknown"] = 1
        with self.assertRaisesRegex(ValueError, "unknown fields"):
            feed.verify_feed(
                raw,
                feed.canonical_json_bytes(signature),
                expected_public_key_b64=self.public_key_b64,
                allowed_hosts=ALLOWED_HOSTS,
            )

    def test_cli_writes_atomically_named_files_and_verify_mode_accepts_them(self) -> None:
        output = self.root / "feed-output"
        env = os.environ.copy()
        env["REYN_UPDATE_SIGNING_KEY_B64"] = base64.b64encode(TEST_SEED).decode()
        command = [
            sys.executable,
            str(ROOT / "scripts/build_update_feed.py"),
            "build",
            "--version",
            "0.3.0",
            "--release-sequence",
            "300",
            "--published",
            "1800000000",
            "--expires",
            "1800604800",
            "--minimum-updater-version",
            "0.2.0",
            "--channel",
            "stable",
            "--changelog-url",
            "https://updates.reynflow.test/changelog/0.3.0",
            "--key-id",
            "reyn-update-2026-01",
            "--macos-package",
            str(self.macos),
            "--macos-minimum-os",
            "14.0",
            "--macos-url",
            f"https://updates.reynflow.test/downloads/{self.macos.name}",
            "--windows-package",
            str(self.windows),
            "--windows-minimum-os",
            "11",
            "--windows-url",
            f"https://updates.reynflow.test/downloads/{self.windows.name}",
            "--developer-id-signed",
            "--notarized",
            "--authenticode-signed",
            "--output-dir",
            str(output),
            "--allowed-host",
            "updates.reynflow.test",
        ]
        subprocess.run(command, check=True, capture_output=True, text=True, env=env)
        self.assertTrue((output / "latest.json").is_file())
        self.assertTrue((output / "latest.sig").is_file())
        self.assertEqual(
            list(output.glob(".latest.*")),
            [],
            "atomic temporary files must not remain",
        )
        subprocess.run(
            [
                sys.executable,
                str(ROOT / "scripts/build_update_feed.py"),
                "verify",
                "--feed",
                str(output / "latest.json"),
                "--signature",
                str(output / "latest.sig"),
                "--public-key-b64",
                self.public_key_b64,
                "--allowed-host",
                "updates.reynflow.test",
            ],
            check=True,
            capture_output=True,
            text=True,
        )

    def test_atomic_write_preserves_existing_file_if_replace_fails(self) -> None:
        destination = self.root / "latest.json"
        destination.write_bytes(b"old")
        with patch.object(feed.os, "replace", side_effect=OSError("simulated")):
            with self.assertRaises(OSError):
                feed.atomic_write(destination, b"new")
        self.assertEqual(destination.read_bytes(), b"old")
        self.assertEqual(list(self.root.glob(".latest.json.*")), [])


if __name__ == "__main__":
    unittest.main()
