import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from package_windows import (  # noqa: E402
    EXPECTED_ACCESS_ENDPOINT,
    EXPECTED_PRIVACY_VERSION,
    EXPECTED_TERMS_VERSION,
    preview_access_contract,
)
from windows_packaging import (  # noqa: E402
    ENGINE_RESOURCES,
    RESEARCH_RESOURCES,
    authenticode_sign,
    deterministic_zip,
    inventory,
    is_windows_reparse_point,
    loader_probe,
    locked_python_packages,
    normalize_architecture,
    normalize_python_dependency_metadata,
    prepare_runtime_manifest,
    python_dependency_metadata,
    runtime_probe,
    rust_dependency_metadata,
    safe_copy_tree,
    safe_files,
    validate_stage,
    write_json,
)


class WindowsPackagingTests(unittest.TestCase):
    def make_stage(self, root: Path) -> Path:
        stage = root / "Reyn-Studio-0.1.1-windows-x64"
        required = (
            "Reyn Studio.exe",
            "ReynStudio.ico",
            "LICENSE",
            "NOTICE",
            "ReynPython/python.exe",
            "ReynPython/runtime-manifest.cjson",
            "ReynPython/runtime-sbom.cdx.json",
            "ReynPython/THIRD_PARTY_NOTICES.html",
            "resources/engine/reyn_engine.py",
            "resources/engine/model_bundle.py",
            "resources/docs/PRD.md",
            "resources/docs/MODEL_BUNDLE_PROVENANCE.md",
            "THIRD_PARTY_NOTICES.md",
            "SBOM.spdx.json",
            "dependency-closure.json",
        )
        for relative in required:
            path = stage / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(relative.encode("utf-8"))
        for name in RESEARCH_RESOURCES:
            path = stage / "resources/research" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("# fixture\n", encoding="utf-8")
        packages = [
            {
                "ecosystem": "python",
                "name": "CPython",
                "normalized_name": "cpython",
                "version": "3.14.6",
                "license": "PSF-2.0",
                "source": "https://www.python.org/",
            }
        ]
        closure = {
            "schema": "com.reyn.dependency-closure/1",
            "target": "windows-x86_64",
            "cargo_lock_sha256": "a" * 64,
            "python_lock_sha256": "b" * 64,
            "packages": packages,
        }
        write_json(stage / "dependency-closure.json", closure)
        write_json(
            stage / "SBOM.spdx.json",
            {
                "spdxVersion": "SPDX-2.3",
                "packages": [
                    {
                        "name": "CPython",
                        "versionInfo": "3.14.6",
                        "licenseDeclared": "PSF-2.0",
                        "downloadLocation": "https://www.python.org/",
                    }
                ],
            },
        )
        write_json(
            stage / "ReynPython/runtime-sbom.cdx.json",
            {
                "bomFormat": "CycloneDX",
                "components": [{"name": "CPython", "version": "3.14.6"}],
            },
        )
        (stage / "THIRD_PARTY_NOTICES.md").write_text(
            "# Notices\n\n## CPython 3.14.6 (python)\n", encoding="utf-8"
        )
        write_json(
            stage / "release-manifest.json",
            {
                "platform": "windows",
                "architecture": "x86_64",
                "cuda_supported": False,
                "windows_verified": False,
                "preview_access": {
                    "schema": "com.reyn.studio.preview-access/1",
                    "required": True,
                    "endpoint": EXPECTED_ACCESS_ENDPOINT,
                    "terms_version": EXPECTED_TERMS_VERSION,
                    "privacy_version": EXPECTED_PRIVACY_VERSION,
                },
                "cargo_lock_sha256": "a" * 64,
                "python_lock_sha256": "b" * 64,
                "model_loader_probe": {
                    "bundle_schema": "com.reyn.inference-model-bundle/1",
                    "loader_error": "signature.missing",
                    "loader_origin": "engine",
                    "model_card_status": "invalid",
                    "import_ok": False,
                    "production_tuf_root_pinned": False,
                },
            },
        )
        write_json(
            stage / "resource-inventory.json",
            inventory(
                stage,
                excluded={"release-manifest.json", "resource-inventory.json"},
            ),
        )
        return stage

    def test_portable_stage_inventory_is_complete_and_truthful(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = self.make_stage(Path(directory))
            self.assertEqual(validate_stage(stage), [])
            manifest = json.loads(
                (stage / "release-manifest.json").read_text(encoding="utf-8")
            )
            self.assertFalse(manifest["cuda_supported"])
            self.assertFalse(manifest["windows_verified"])

    def test_packager_verifies_the_compiled_access_contract(self):
        expected = {
            "schema": "com.reyn.studio.preview-access/1",
            "required": True,
            "endpoint": EXPECTED_ACCESS_ENDPOINT,
            "terms_version": EXPECTED_TERMS_VERSION,
            "privacy_version": EXPECTED_PRIVACY_VERSION,
        }
        completed = type(
            "Completed",
            (),
            {"stdout": json.dumps(expected) + "\n"},
        )()
        binary = Path("Reyn Studio.exe")
        with patch("package_windows.subprocess.run", return_value=completed) as run:
            self.assertEqual(preview_access_contract(binary), expected)
        run.assert_called_once_with(
            [str(binary), "--print-access-contract"],
            check=True,
            capture_output=True,
            text=True,
            timeout=15,
        )

        invalid = dict(expected, required=False)
        completed.stdout = json.dumps(invalid)
        with patch("package_windows.subprocess.run", return_value=completed):
            with self.assertRaisesRegex(ValueError, "required YC preview access"):
                preview_access_contract(binary)

    def test_locked_dependency_inputs_are_complete_and_licensed(self):
        locked = locked_python_packages(
            ROOT / "packaging/windows/python-runtime.lock"
        )
        self.assertEqual(locked["torch"], "2.13.0+cpu")
        self.assertEqual(locked["numpy"], "2.5.1")
        self.assertEqual(locked["cryptography"], "49.0.0")
        self.assertEqual(locked["safetensors"], "0.8.0")
        self.assertEqual(locked["securesystemslib"], "1.4.0")
        self.assertEqual(locked["tuf"], "6.0.0")
        rust = rust_dependency_metadata(ROOT)
        self.assertGreater(len(rust), 20)
        self.assertTrue(all(row["license"] and row["source"] for row in rust))

    def test_python_dependency_metadata_requires_source_and_is_deterministic(self):
        rows = [
            {
                "name": "Torch",
                "version": "2.13.0+cpu",
                "license": "BSD-3-Clause",
                "source": "https://github.com/pytorch/pytorch",
            },
            {
                "name": "NumPy",
                "version": "2.5.1",
                "license": "BSD-3-Clause",
                "source": "https://github.com/numpy/numpy",
            },
        ]
        expected = normalize_python_dependency_metadata(rows)
        self.assertEqual(expected, normalize_python_dependency_metadata(list(reversed(rows))))
        with tempfile.TemporaryDirectory() as directory:
            runtime = Path(directory)
            (runtime / "python.exe").write_bytes(b"python")
            completed = type(
                "Completed",
                (),
                {"stdout": json.dumps(list(reversed(rows))) + "\n"},
            )()
            with patch("windows_packaging.subprocess.run", return_value=completed):
                self.assertEqual(python_dependency_metadata(runtime), expected)
        for source in (None, "", "   ", "NOASSERTION", "unknown"):
            invalid = [dict(rows[0], source=source)]
            with self.subTest(source=source):
                with self.assertRaisesRegex(ValueError, "no source metadata"):
                    normalize_python_dependency_metadata(invalid)

    def test_deterministic_zip_repeats_exact_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            stage = self.make_stage(root)
            first = root / "first.zip"
            second = root / "second.zip"
            deterministic_zip(stage, first, 315532800)
            deterministic_zip(stage, second, 315532800)
            self.assertEqual(first.read_bytes(), second.read_bytes())

    def test_windows_reparse_attribute_is_detected_without_windows(self):
        self.assertEqual(normalize_architecture("AMD64"), "x86_64")
        self.assertEqual(normalize_architecture("x64"), "x86_64")
        metadata = type("Metadata", (), {"st_file_attributes": 0x400})()
        self.assertTrue(
            is_windows_reparse_point(
                Path("unused"),
                lstat=lambda _path: metadata,
            )
        )

    def test_safe_tree_rejects_injected_reparse_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            junction = root / "junction"
            junction.mkdir()
            (junction / "payload.bin").write_bytes(b"payload")
            with self.assertRaisesRegex(ValueError, "reparse point"):
                safe_files(
                    root,
                    reparse_checker=lambda path: path.name == "junction",
                )
            destination = root.parent / f"{root.name}-copy"
            with self.assertRaisesRegex(ValueError, "reparse point"):
                safe_copy_tree(
                    root,
                    destination,
                    reparse_checker=lambda path: path.name == "junction",
                )
            self.assertFalse(destination.exists())
            archive = root.parent / f"{root.name}.zip"
            with self.assertRaisesRegex(ValueError, "reparse point"):
                deterministic_zip(
                    root,
                    archive,
                    315532800,
                    reparse_checker=lambda path: path.name == "junction",
                )
            self.assertFalse(archive.exists())
            errors = validate_stage(
                root,
                reparse_checker=lambda path: path.name == "junction",
            )
            self.assertTrue(errors[0].startswith("unsafe package tree:"))

    @unittest.skipUnless(os.name == "nt", "requires a real Windows junction")
    def test_real_windows_junction_is_rejected_before_packaging_outputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source with spaces"
            outside = root / "outside Résultats"
            source.mkdir()
            outside.mkdir()
            (outside / "payload.bin").write_bytes(b"outside")
            junction = source / "linked Résultats"
            subprocess.run(
                ["cmd.exe", "/d", "/c", "mklink", "/J", str(junction), str(outside)],
                check=True,
                capture_output=True,
                text=True,
            )
            destination = root / "copied output"
            archive = root / "archive output.zip"
            try:
                with self.assertRaisesRegex(ValueError, "reparse point"):
                    safe_copy_tree(source, destination)
                self.assertFalse(destination.exists())
                with self.assertRaisesRegex(ValueError, "reparse point"):
                    inventory(source)
                with self.assertRaisesRegex(ValueError, "reparse point"):
                    deterministic_zip(source, archive, 315532800)
                self.assertFalse(archive.exists())
            finally:
                if junction.exists():
                    os.rmdir(junction)

    def test_safe_tree_rejects_symlink_and_unicode_paths_zip_cleanly(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source with spaces"
            source.mkdir()
            unicode_file = source / "résultat.bin"
            unicode_file.write_bytes(b"payload")
            archive = root / "unicode.zip"
            deterministic_zip(source, archive, 315532800)
            with zipfile.ZipFile(archive) as packaged:
                self.assertIn(
                    f"{source.name}/résultat.bin",
                    packaged.namelist(),
                )
            link = source / "outside-link"
            try:
                link.symlink_to(root / "outside")
            except (NotImplementedError, OSError):
                self.skipTest("symlinks are unavailable")
            with self.assertRaisesRegex(ValueError, "link or reparse point"):
                safe_files(source)

    def test_runtime_probe_accepts_x86_64_aliases_and_rejects_unknown_machine(self):
        with tempfile.TemporaryDirectory() as directory:
            runtime = Path(directory)
            (runtime / "python.exe").write_bytes(b"python")
            payload = {
                "python": "3.14.6",
                "cryptography": "49.0.0",
                "numpy": "2.5.1",
                "safetensors": "0.8.0",
                "securesystemslib": "1.4.0",
                "torch": "2.13.0",
                "tuf": "6.0.0",
                "platform": "win32",
                "machine": "x86_64",
                "cuda": False,
            }
            completed = type(
                "Completed",
                (),
                {"stdout": json.dumps(payload) + "\n"},
            )()
            for machine in ("x86_64", "AMD64", "x64"):
                payload["machine"] = machine
                completed.stdout = json.dumps(payload) + "\n"
                with self.subTest(machine=machine):
                    with patch(
                        "windows_packaging.subprocess.run",
                        return_value=completed,
                    ):
                        self.assertEqual(runtime_probe(runtime)["machine"], "x86_64")
            payload["machine"] = "i686"
            completed.stdout = json.dumps(payload) + "\n"
            with patch("windows_packaging.subprocess.run", return_value=completed):
                with self.assertRaisesRegex(ValueError, "unknown runtime architecture"):
                    runtime_probe(runtime)

    def test_runtime_manifest_has_content_identity_and_cpu_inventory(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory)
            runtime = stage / "ReynPython"
            runtime.mkdir()
            (runtime / "python.exe").write_bytes(b"python")
            (stage / "SBOM.spdx.json").write_text("{}", encoding="utf-8")
            (stage / "THIRD_PARTY_NOTICES.md").write_text(
                "Runtime notices", encoding="utf-8"
            )
            for prefix, names in (
                ("engine", ENGINE_RESOURCES),
                ("research", RESEARCH_RESOURCES),
            ):
                for name in names:
                    path = stage / "resources" / prefix / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text(f"# {name}\n", encoding="utf-8")
            prepare_runtime_manifest(
                stage,
                "fixture-revision",
                {"python": "3.14.6", "numpy": "2.5.1", "torch": "2.13.0"},
                {
                    "packages": [
                        {
                            "ecosystem": "python",
                            "name": "CPython",
                            "normalized_name": "cpython",
                            "version": "3.14.6",
                            "license": "PSF-2.0",
                            "source": "https://www.python.org/",
                        }
                    ]
                },
            )
            manifest = json.loads(
                (runtime / "runtime-manifest.cjson").read_text(encoding="utf-8")
            )
            identity = dict(manifest)
            identity.pop("runtime_id")
            canonical = json.dumps(
                identity, sort_keys=True, separators=(",", ":")
            ).encode("utf-8")
            self.assertEqual(
                manifest["runtime_id"],
                "sha256:" + hashlib.sha256(canonical).hexdigest(),
            )
            self.assertNotIn("minimum_macos", manifest)
            self.assertIn(
                "runtime-sbom.cdx.json",
                [row["path"] for row in manifest["files"]],
            )

    def test_loader_probe_exercises_model_card_and_import_rejection(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory)
            python = stage / "ReynPython/python.exe"
            python.parent.mkdir(parents=True)
            python.write_bytes(b"python")
            completed = type(
                "Completed",
                (),
                {
                    "stdout": json.dumps(
                        {
                            "bundle_schema": "com.reyn.inference-model-bundle/1",
                            "loader_error": "signature.missing",
                            "loader_origin": "engine",
                            "model_card_status": "invalid",
                            "import_ok": False,
                            "production_tuf_root_pinned": False,
                        }
                    )
                    + "\n"
                },
            )()
            with patch(
                "windows_packaging.subprocess.run", return_value=completed
            ) as run:
                result = loader_probe(stage)
            self.assertEqual(result["model_card_status"], "invalid")
            command = run.call_args.args[0]
            self.assertIn("model_bundle.load_model_bundle", command[3])
            self.assertIn("runtime.checkpoint_card", command[3])
            self.assertIn("runtime.import_model", command[3])

    def test_authenticode_request_fails_closed_without_password(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable = root / "Reyn Studio.exe"
            pfx = root / "certificate.pfx"
            executable.write_bytes(b"exe")
            pfx.write_bytes(b"pfx")
            with patch.dict(os.environ, {}, clear=True):
                with self.assertRaisesRegex(
                    RuntimeError, "REYN_AUTHENTICODE_PFX_PASSWORD is not set"
                ):
                    authenticode_sign(
                        executable,
                        pfx,
                        "REYN_AUTHENTICODE_PFX_PASSWORD",
                        "https://timestamp.example",
                    )

    def test_validator_rejects_cuda_or_verified_claims(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = self.make_stage(Path(directory))
            manifest_path = stage / "release-manifest.json"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["cuda_supported"] = True
            manifest["windows_verified"] = True
            manifest["preview_access"]["required"] = False
            write_json(manifest_path, manifest)
            errors = validate_stage(stage)
            self.assertIn(
                "release manifest must state cuda_supported=false",
                errors,
            )
            self.assertIn(
                "local packaging must not claim Windows verification",
                errors,
            )
            self.assertIn(
                "release manifest must record the exact YC preview access contract",
                errors,
            )

    def test_validator_rejects_sbom_closure_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = self.make_stage(Path(directory))
            write_json(
                stage / "SBOM.spdx.json",
                {"spdxVersion": "SPDX-2.3", "packages": []},
            )
            errors = validate_stage(stage)
            self.assertIn(
                "SBOM package closure does not match dependency-closure.json",
                errors,
            )

    def test_validator_rejects_missing_blank_or_noassertion_source(self):
        for source in (None, "", "NOASSERTION"):
            with self.subTest(source=source), tempfile.TemporaryDirectory() as directory:
                stage = self.make_stage(Path(directory))
                closure_path = stage / "dependency-closure.json"
                closure = json.loads(closure_path.read_text(encoding="utf-8"))
                sbom_path = stage / "SBOM.spdx.json"
                sbom = json.loads(sbom_path.read_text(encoding="utf-8"))
                if source is None:
                    closure["packages"][0].pop("source")
                    sbom["packages"][0].pop("downloadLocation")
                else:
                    closure["packages"][0]["source"] = source
                    sbom["packages"][0]["downloadLocation"] = source
                write_json(closure_path, closure)
                write_json(sbom_path, sbom)
                errors = validate_stage(stage)
                self.assertIn(
                    "dependency closure package CPython 3.14.6 has no source metadata",
                    errors,
                )


if __name__ == "__main__":
    unittest.main()
