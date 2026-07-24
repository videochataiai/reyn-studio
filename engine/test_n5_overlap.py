import tempfile
import unittest
from pathlib import Path

import numpy as np

from engine.n5_overlap import (
    ALGORITHM,
    CANDIDATE_SCHEMA,
    OVERLAP_SCHEMA,
    analyze_field_trajectory_overlap,
)


class FieldTrajectoryOverlapTests(unittest.TestCase):
    def artifact(self, path, initial, trajectories, *, complete=True):
        np.savez(
            path,
            schema=np.array(CANDIDATE_SCHEMA),
            candidate_set_name=np.array("fixture training split"),
            candidate_set_complete=np.array(complete),
            representation=np.array("full velocity tensor [component,y,x]"),
            candidate_ids=np.array(["train-000", "train-001"]),
            initial_conditions=np.asarray(initial, dtype=np.float32),
            trajectory_candidate_ids=np.array(["train-000", "train-001"]),
            trajectories=np.asarray(trajectories, dtype=np.float32),
            generation_manifest_json=np.array(
                '{"dataset_seed":7,"split":"training","generator_version":"fixture-v1"}'
            ),
        )

    def test_missing_training_artifact_is_explicit_unknown(self):
        initial = np.zeros((1, 2, 4, 4), dtype=np.float32)
        result = analyze_field_trajectory_overlap(
            initial,
            candidate_artifact="/missing/training-candidates.npz",
            query_ids=["benchmark-70000"],
        )

        self.assertEqual(result["schema"], OVERLAP_SCHEMA)
        self.assertEqual(result["status"], "UNKNOWN")
        self.assertEqual(result["algorithm"]["id"], ALGORITHM)
        self.assertEqual(result["representation"]["name"], "UNKNOWN")
        self.assertEqual(result["candidate_set"]["status"], "UNAVAILABLE")
        self.assertIsNone(result["candidate_set"]["artifact_sha256"])
        self.assertEqual(
            result["checks"]["initial_condition"]["nearest_matches"], []
        )
        self.assertIn("UNKNOWN", result["proposition"])
        self.assertEqual(
            result["reproducible_inputs"]["initial_conditions"]["shape"],
            [1, 2, 4, 4],
        )

    def test_exact_field_and_trajectory_matches_are_flagged_and_named(self):
        initial = np.stack([
            np.zeros((2, 4, 4), dtype=np.float32),
            np.ones((2, 4, 4), dtype=np.float32),
        ])
        trajectories = np.stack([initial * 0.5, initial * 1.5], axis=1)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candidates.npz"
            self.artifact(path, initial, trajectories)
            result = analyze_field_trajectory_overlap(
                initial[1:2],
                trajectories[1:2],
                candidate_artifact=path,
                query_ids=["benchmark-70000"],
                initial_threshold=1e-8,
                trajectory_threshold=1e-8,
                nearest_count=2,
            )

        self.assertEqual(result["status"], "FLAGGED")
        self.assertTrue(result["candidate_set"]["declared_complete"])
        self.assertEqual(result["candidate_set"]["initial_condition_candidates"], 2)
        self.assertEqual(
            result["candidate_set"]["generation_manifest"]["dataset_seed"], 7
        )
        self.assertEqual(
            result["checks"]["initial_condition"]["nearest_matches"][0]["matches"][0],
            {
                "candidate_id": "train-001",
                "distance": 0.0,
                "at_or_below_threshold": True,
            },
        )
        self.assertEqual(
            result["checks"]["trajectory"]["nearest_matches"][0]["matches"][0][
                "candidate_id"
            ],
            "train-001",
        )
        self.assertEqual(len(result["candidate_set"]["artifact_sha256"]), 64)

    def test_clean_requires_complete_candidate_set_and_both_checks(self):
        candidates = np.ones((2, 2, 4, 4), dtype=np.float32)
        candidate_trajectories = np.ones((2, 3, 2, 4, 4), dtype=np.float32)
        query = -np.ones((1, 2, 4, 4), dtype=np.float32)
        query_trajectory = -np.ones((1, 3, 2, 4, 4), dtype=np.float32)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candidates.npz"
            self.artifact(path, candidates, candidate_trajectories)
            clean = analyze_field_trajectory_overlap(
                query,
                query_trajectory,
                candidate_artifact=path,
                initial_threshold=0.1,
                trajectory_threshold=0.1,
            )
            self.artifact(
                path,
                candidates,
                candidate_trajectories,
                complete=False,
            )
            incomplete = analyze_field_trajectory_overlap(
                query,
                query_trajectory,
                candidate_artifact=path,
                initial_threshold=0.1,
                trajectory_threshold=0.1,
            )

        self.assertEqual(clean["status"], "CLEAN")
        self.assertEqual(clean["checks"]["initial_condition"]["status"], "CLEAN")
        self.assertEqual(clean["checks"]["trajectory"]["status"], "CLEAN")
        self.assertIn("complete checked training candidate set", clean["proposition"])
        self.assertEqual(incomplete["status"], "UNKNOWN")
        self.assertEqual(
            incomplete["checks"]["initial_condition"]["status"], "UNKNOWN"
        )
        self.assertIn("completeness", incomplete["warnings"][0])

    def test_missing_query_trajectory_prevents_overall_clean(self):
        candidates = np.ones((2, 2, 4, 4), dtype=np.float32)
        candidate_trajectories = np.ones((2, 3, 2, 4, 4), dtype=np.float32)
        query = -np.ones((1, 2, 4, 4), dtype=np.float32)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candidates.npz"
            self.artifact(path, candidates, candidate_trajectories)
            result = analyze_field_trajectory_overlap(
                query,
                candidate_artifact=path,
                initial_threshold=0.1,
                trajectory_threshold=0.1,
            )

        self.assertEqual(result["checks"]["initial_condition"]["status"], "CLEAN")
        self.assertEqual(result["checks"]["trajectory"]["status"], "UNKNOWN")
        self.assertEqual(result["status"], "UNKNOWN")


if __name__ == "__main__":
    unittest.main()
