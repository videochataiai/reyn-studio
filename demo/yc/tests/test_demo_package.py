from __future__ import annotations

import json
import re
import sys
import tempfile
import unittest
from pathlib import Path

YC = Path(__file__).resolve().parents[1]
ROOT = YC.parents[1]
SCRIPTS = YC / "scripts"
sys.path.insert(0, str(SCRIPTS))

import build_fixtures
from fixture_tools import validate_manifest
from validate_video import MAX_BYTES, validate_metrics


class FixtureTests(unittest.TestCase):
    def setUp(self):
        self.manifest_path = YC / "assets" / "fixture-manifest.json"
        self.manifest = json.loads(self.manifest_path.read_text(encoding="utf-8"))

    def test_hashes_triangle_counts_and_topology_match_manifest(self):
        validated = validate_manifest(self.manifest_path)
        self.assertEqual(len(validated), 3)
        by_role = {item["role"]: item for item in self.manifest["fixtures"]}
        self.assertTrue(by_role["primary"]["watertight"])
        self.assertTrue(by_role["fallback"]["watertight"])
        self.assertFalse(by_role["intentional_rejection"]["watertight"])
        self.assertEqual(by_role["intentional_rejection"]["boundary_edges"], 48)
        self.assertEqual(
            by_role["intentional_rejection"]["expected_diagnostics"][0]["code"],
            "mesh.open_boundary",
        )
        self.assertFalse(
            by_role["intentional_rejection"]["expected_diagnostics"][0]["waivable"]
        )

    def test_generation_is_byte_for_byte_deterministic(self):
        with tempfile.TemporaryDirectory() as directory:
            generated_manifest = build_fixtures.generate(Path(directory))
            generated = json.loads(generated_manifest.read_text(encoding="utf-8"))
            self.assertEqual(generated, self.manifest)
            for fixture in self.manifest["fixtures"]:
                expected = (YC / "assets" / fixture["file"]).read_bytes()
                actual = (Path(directory) / fixture["file"]).read_bytes()
                self.assertEqual(actual, expected)


class ScriptTests(unittest.TestCase):
    def test_documented_references_exist(self):
        combined = "\n".join(
            (YC / name).read_text(encoding="utf-8")
            for name in ("README.md", "SCRIPT.md", "SHOT_CHECKLIST.md")
        )
        for fixture in json.loads(
            (YC / "assets" / "fixture-manifest.json").read_text(encoding="utf-8")
        )["fixtures"]:
            self.assertIn(fixture["file"], combined)
        for relative in (
            "demo.sh",
            "captions.srt",
            "SCRIPT_FALLBACK.md",
            "REFERENCE_RUN_BLOCKER.md",
            "scripts/compress_demo.sh",
            "scripts/validate_video.py",
        ):
            self.assertTrue((YC / relative).is_file(), relative)

    def test_storyboard_and_captions_fit_duration_budget(self):
        storyboard = (YC / "SCRIPT.md").read_text(encoding="utf-8")
        declared = int(re.search(r"duration_seconds:\s*(\d+)", storyboard).group(1))
        self.assertEqual(declared, 160)
        ends = re.findall(r"## \d\d:\d\d[–-](\d\d):(\d\d)", storyboard)
        self.assertEqual(max(int(minutes) * 60 + int(seconds) for minutes, seconds in ends), 160)

        captions = (YC / "captions.srt").read_text(encoding="utf-8")
        self.assertIn("00:02:40,000", captions)
        self.assertNotIn("00:02:41", captions)

    def test_narration_avoids_unsupported_positive_claims(self):
        storyboard = (YC / "SCRIPT.md").read_text(encoding="utf-8")
        narration = " ".join(
            match.group(1)
            for match in re.finditer(
                r"\*\*Narration:\*\*\s*“(.*?)”", storyboard, flags=re.DOTALL
            )
        ).lower()
        narration = " ".join(narration.split())
        forbidden = (
            "commercially released",
            "production-accurate",
            "fully standalone",
            "signed and notarized",
            "qualified production model is loaded",
            "replaces cfd",
            "independently validated",
        )
        for claim in forbidden:
            self.assertNotIn(claim, narration)
        self.assertIn("qualified neural model is the next validation step", narration)
        self.assertEqual(narration.count("qualified neural model"), 1)
        self.assertIn("what is real today", narration)

    def test_no_project_or_solver_result_is_fabricated(self):
        self.assertFalse(list(YC.rglob("*.reynproj")))
        self.assertFalse(list(YC.rglob("*.reynmodel")))
        text = (YC / "README.md").read_text(encoding="utf-8")
        self.assertIn("No `.reynproj` template or recorded solver result is included.", text)

    def test_reference_blocker_and_original_fallback_are_explicit(self):
        blocker = (YC / "REFERENCE_RUN_BLOCKER.md").read_text(encoding="utf-8")
        for required in (
            "solver_reference",
            "model_prediction",
            "pressure_pa",
            "grid size 96 or above",
            "model: null",
            "No `.reynproj`",
        ):
            self.assertIn(required, blocker)
        fallback = (YC / "SCRIPT_FALLBACK.md").read_text(encoding="utf-8")
        self.assertIn("Show the honest model gate", fallback)
        self.assertIn("release gates are still work to finish", fallback)


class VideoLimitTests(unittest.TestCase):
    def test_duration_and_size_validation_logic(self):
        self.assertEqual(validate_metrics(160.0, 99_999_999), [])
        self.assertTrue(validate_metrics(149.999, 1))
        self.assertTrue(validate_metrics(170.001, 1))
        self.assertTrue(validate_metrics(160.0, MAX_BYTES + 1))
        self.assertTrue(validate_metrics(160.0, 0))

    def test_compressor_calls_hard_validator(self):
        script = (SCRIPTS / "compress_demo.sh").read_text(encoding="utf-8")
        self.assertIn("TARGET_BYTES=95000000", script)
        self.assertIn('validate_video.py" "$OUTPUT"', script)
        self.assertIn("150.0 <= duration <= 170.0", script)


if __name__ == "__main__":
    unittest.main()
