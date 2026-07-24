import unittest
from pathlib import Path
import sys
import tempfile

import numpy as np

from engine.reyn_engine import (
    Engine,
    analyze_checkpoint_provenance,
    benchmark_cell_evidence,
    classify_benchmark_seed,
    divergence_rms,
    engineering_surface_loads,
    radial_energy_spectrum,
)

try:
    import torch
except ImportError:  # The lightweight system-Python suite remains usable.
    torch = None


class ProvenanceTests(unittest.TestCase):
    def test_reserved_streams_are_distinct_and_flagged(self):
        checkpoint = {
            "epoch": 40,
            "checkpoint_role": "fixed_final",
            "best_metric": "val_multi_horizon_wake_rel_l2",
            "source_fingerprint": {"digest": "source-a"},
            "train_args": {"seed": 7, "dataset": "mixed", "epochs": 40},
        }
        result = analyze_checkpoint_provenance(
            checkpoint, [7, 10007, 50007, 70000]
        )

        self.assertEqual(result["verdict"], "flagged")
        self.assertEqual(result["overlap_count"], 3)
        self.assertEqual(
            [record["stream"] for record in result["benchmark_seeds"]],
            ["training", "mixed_fork", "validation_selection", "fresh_test"],
        )
        self.assertEqual(result["selection_stream"], "fixed_epoch")
        self.assertEqual(result["final_epoch_status"], "fixed_final")

    def test_fresh_fixed_final_checkpoint_is_clean(self):
        checkpoint = {
            "epoch": 20,
            "checkpoint_role": "fixed_final",
            "best_metric": "val_multi_horizon_rel_l2",
            "source_fingerprint": {"digest": "source-b"},
            "train_args": {"seed": 0, "dataset": "standard", "epochs": 20},
        }
        result = analyze_checkpoint_provenance(
            checkpoint, [70000, 70001, 70002]
        )

        self.assertEqual(result["verdict"], "clean")
        self.assertEqual(result["overlap_count"], 0)
        self.assertTrue(all(
            record["stream"] == "fresh_test"
            for record in result["benchmark_seeds"]
        ))
        self.assertFalse(result["mixed_fork_used"])

    def test_legacy_metadata_remains_unknown(self):
        checkpoint = {
            "epoch": 30,
            "best_metric": "val_multi_horizon_rel_l2",
            "train_args": {"seed": 0, "dataset": "standard", "epochs": 30},
        }
        result = analyze_checkpoint_provenance(checkpoint, [70000])

        self.assertEqual(result["verdict"], "unknown")
        self.assertIn("checkpoint role absent", result["legacy_unknown"])
        self.assertIn("source fingerprint absent", result["legacy_unknown"])
        self.assertEqual(
            classify_benchmark_seed(50000, 0), "validation_selection"
        )


class SpectralEvidenceTests(unittest.TestCase):
    def test_radial_spectrum_finds_known_mode(self):
        n = 32
        x = 2.0 * np.pi * np.arange(n) / n
        field = np.zeros((2, n, n), dtype=np.float32)
        field[0] = np.sin(3.0 * x)[:, None]

        k, energy = radial_energy_spectrum(field)

        self.assertEqual(float(k[np.argmax(energy)]), 3.0)
        self.assertGreater(float(energy.max()), 0.0)

    def test_divergence_measure_distinguishes_solenoidal_field(self):
        n = 32
        x = 2.0 * np.pi * np.arange(n) / n
        divergence_free = np.zeros((2, n, n), dtype=np.float32)
        divergence_free[0] = np.sin(x)[:, None]
        divergence_free[1] = -np.sin(x)[None, :]
        divergent = np.zeros_like(divergence_free)
        divergent[0] = np.sin(x)[None, :]

        self.assertLess(divergence_rms(divergence_free), 1e-6)
        self.assertGreater(divergence_rms(divergent), 0.5)

    def test_cell_evidence_matches_scaled_prediction(self):
        n = 32
        x = 2.0 * np.pi * np.arange(n) / n
        truth = np.zeros((2, n, n), dtype=np.float32)
        truth[0] = np.sin(2.0 * x)[None, :]
        truth[1] = -np.sin(2.0 * x)[:, None]
        prediction = 0.5 * truth
        initial = np.zeros_like(truth)

        evidence = benchmark_cell_evidence(prediction, truth, initial)

        self.assertAlmostEqual(evidence["rel_l2"], 0.5, places=6)
        self.assertAlmostEqual(evidence["persist_rel_l2"], 1.0, places=6)
        self.assertAlmostEqual(evidence["improvement_ratio"], 2.0, places=6)
        self.assertAlmostEqual(evidence["spectrum_rel_l2"], 0.75, places=6)
        self.assertEqual(evidence["error_map"].shape, (n, n))
        self.assertTrue(np.all(np.isfinite(evidence["error_map"])))


class EngineeringSurfaceLoadTests(unittest.TestCase):
    def synthetic_cube(self, n=32):
        mask = np.zeros((n, n, n), dtype=np.float32)
        mask[n // 3: 2 * n // 3, n // 3: 2 * n // 3, n // 3: 2 * n // 3] = 1.0
        velocity = np.zeros((3, n, n, n), dtype=np.float32)
        return velocity, mask

    def test_constant_pressure_closed_surface_has_negligible_net_force(self):
        velocity, mask = self.synthetic_cube()
        pressure = np.ones_like(mask) * 0.3
        result = engineering_surface_loads(
            velocity,
            pressure,
            mask,
            reynolds=150.0,
            char_len_solver=0.6,
            reference_length_m=0.1,
            velocity_mps=12.0,
            density_kg_m3=1.225,
            reference_pressure_pa=101325.0,
        )
        self.assertLess(np.linalg.norm(result["force_coefficients"]), 1e-5)
        self.assertTrue(np.allclose(result["cp"][mask > 0], 0.6))
        expected_pressure = 101325.0 + 0.3 * 1.225 * 12.0**2
        self.assertTrue(
            np.allclose(result["pressure_pa"][mask > 0], expected_pressure)
        )
        self.assertGreater(result["surface_area_m2"], 0.0)
        self.assertEqual(result["method"], "diffuse_interface_traction.v1")
        self.assertGreaterEqual(result["wake_deficit_peak"], 0.0)
        self.assertGreaterEqual(result["wake_deficit_mean"], 0.0)

    def test_pressure_gradient_produces_drag_and_physical_scaling(self):
        velocity, mask = self.synthetic_cube()
        x = np.linspace(0.5, -0.5, mask.shape[0], dtype=np.float32)
        pressure = np.broadcast_to(x[:, None, None], mask.shape).copy()
        result = engineering_surface_loads(
            velocity,
            pressure,
            mask,
            reynolds=150.0,
            char_len_solver=0.6,
            reference_length_m=0.2,
            velocity_mps=20.0,
            density_kg_m3=1.0,
            reference_pressure_pa=100000.0,
        )
        self.assertGreater(abs(result["force_coefficients"][0]), 1e-3)
        expected = (
            result["force_coefficients"][0]
            * 0.5
            * 1.0
            * 20.0**2
            * 0.2**2
        )
        self.assertAlmostEqual(result["force_newtons"][0], expected, places=5)
        self.assertEqual(result["traction_pa"].shape, velocity.shape)

    def test_malformed_or_nonphysical_contract_is_rejected(self):
        velocity, mask = self.synthetic_cube(16)
        with self.assertRaisesRegex(ValueError, "positive"):
            engineering_surface_loads(
                velocity,
                np.zeros_like(mask),
                mask,
                reynolds=0.0,
                char_len_solver=0.6,
                reference_length_m=1.0,
                velocity_mps=1.0,
                density_kg_m3=1.0,
                reference_pressure_pa=0.0,
            )
        invalid_pressure = np.zeros_like(mask)
        invalid_pressure[0, 0, 0] = np.nan
        with self.assertRaisesRegex(ValueError, "finite"):
            engineering_surface_loads(
                velocity,
                invalid_pressure,
                mask,
                reynolds=150.0,
                char_len_solver=0.6,
                reference_length_m=1.0,
                velocity_mps=1.0,
                density_kg_m3=1.0,
                reference_pressure_pa=101325.0,
            )


@unittest.skipIf(torch is None, "torch is available in the Reyn research environment")
class ModelLibraryTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.engine = Engine.__new__(Engine)
        self.engine.research_dir = str(self.root)
        self.engine.torch = torch
        self.engine.device = torch.device("cpu")
        self.engine.cache = {}
        self.engine.traj2d = {}
        self.engine.cad_cache = {}
        self.engine._probe_checkpoint_compatibility = lambda _path: None

    def tearDown(self):
        self.directory.cleanup()

    def checkpoint(
        self,
        path,
        *,
        source=True,
        in_channels=3,
        out_channels=2,
        param_dim=0,
    ):
        checkpoint = {
            "model_config": {
                "in_channels": in_channels,
                "out_channels": out_channels,
                "param_dim": param_dim,
            },
            "model_state_dict": {"trunk.weight": torch.ones(2, 2, 3, 3)},
            "train_args": {
                "grid_size": 128,
                "max_steps": 64,
                "epochs": 40,
                "dt": 0.01,
                "stride": 4,
                "warmup_steps": 8,
                "nu": 0.01,
                "scenario": "obstacle",
            },
            "epoch": 40,
            "checkpoint_role": "fixed_final",
            "limitations": ["Static 2D geometry only"],
            "benchmark_reports": [{"sha256": "a" * 64}],
        }
        if source:
            checkpoint["source_fingerprint"] = {"digest": "source-test"}
        torch.save(checkpoint, path)

    def test_import_validates_copies_and_deletes_only_managed_models(self):
        source = self.root / "outside.pth"
        self.checkpoint(source)

        result = self.engine.import_model(source)
        imported = result["imported"]
        cards = result["models"]

        self.assertTrue(result["ok"])
        self.assertTrue(result["validation"]["accepted"])
        self.assertEqual(imported["status"], "clean")
        self.assertTrue(imported["managed"])
        self.assertTrue(imported["id"].startswith("reyn_models/"))
        self.assertEqual(len(imported["checkpoint_sha256"]), 64)
        self.assertEqual(imported["limitations"], ["Static 2D geometry only"])
        self.assertEqual(imported["benchmark_report_hashes"], ["a" * 64])
        self.assertEqual(len(cards), 2)
        remaining = self.engine.delete_model(imported["id"])
        self.assertEqual([card["id"] for card in remaining], ["outside.pth"])
        with self.assertRaisesRegex(ValueError, "only checkpoints imported"):
            self.engine.delete_model("outside.pth")

    def test_invalid_checkpoint_is_rejected_with_a_clear_reason(self):
        source = self.root / "notes.pth"
        torch.save({"not": "a model"}, source)

        card = self.engine.checkpoint_card(source)

        self.assertEqual(card["status"], "invalid")
        self.assertIn("missing required", card["status_detail"])
        result = self.engine.import_model(source)
        self.assertFalse(result["ok"])
        self.assertFalse(result["validation"]["accepted"])
        self.assertIn(
            "checkpoint.missing_field",
            {issue["code"] for issue in result["validation"]["issues"]},
        )
        self.assertFalse(self.engine.managed_model_dir.exists())

    def test_incompatible_checkpoint_returns_structured_contract_rejection(self):
        source = self.root / "unsupported.pth"
        self.checkpoint(source, in_channels=4, out_channels=2, param_dim=0)

        result = self.engine.import_model(source)

        self.assertFalse(result["ok"])
        self.assertIn(
            "contract.unsupported_channels",
            {issue["code"] for issue in result["validation"]["issues"]},
        )
        self.assertFalse(self.engine.managed_model_dir.exists())

    def test_state_dictionary_load_failure_is_structured_and_not_copied(self):
        source = self.root / "shape-mismatch.pth"
        self.checkpoint(source)

        def fail_probe(_path):
            raise RuntimeError("state dictionary shape mismatch")

        self.engine._probe_checkpoint_compatibility = fail_probe
        result = self.engine.import_model(source)

        self.assertFalse(result["ok"])
        self.assertIn(
            "checkpoint.load_incompatible",
            {issue["code"] for issue in result["validation"]["issues"]},
        )
        self.assertFalse(self.engine.managed_model_dir.exists())

    def test_legacy_checkpoint_remains_amber_not_verified(self):
        source = self.root / "legacy.pth"
        self.checkpoint(source, source=False)

        card = self.engine.checkpoint_card(source)

        self.assertEqual(card["status"], "review")
        self.assertIn("source fingerprint absent", card["status_detail"])
        self.assertIn("source_fingerprint", card["unknown_fields"])


@unittest.skipIf(torch is None, "torch is available in the Reyn research environment")
class ModelContractTests(unittest.TestCase):
    def test_fixed_body_v2_packs_sponge_and_viscosity(self):
        research_dir = Path(__file__).resolve().parents[2] / "reyn-research"
        sys.path.insert(0, str(research_dir))
        from flow_contract import fixed_body_v2_metadata

        class RecordingModel:
            def __init__(self):
                self.packed = None
                self.params = None

            def __call__(self, packed, dt, params=None):
                self.packed = packed
                self.params = params
                return packed[:, :2]

        model = RecordingModel()
        engine = Engine.__new__(Engine)
        engine.torch = torch
        engine.device = torch.device("cpu")
        n = 8
        state = torch.randn(1, 2, n, n)
        mask = torch.zeros(1, 1, n, n)
        mask[:, :, 2:4, 2:4] = 1.0
        sponge = torch.full((1, 1, n, n), 2.5)
        context = {
            "nu": torch.tensor([[0.004]]),
            "sponge": sponge,
            "u_inf": torch.tensor([[1.0, 0.0]]),
            "eta": torch.tensor([[1e-3]]),
        }
        info = {
            "model": model,
            "cfg": {
                "in_channels": 4,
                "out_channels": 2,
                "param_dim": 1,
            },
            "ta": {"dt": 0.01, "stride": 4},
            "physics_spec": fixed_body_v2_metadata(),
        }

        result = engine._run_model_2d(info, state, mask, context, horizon=3)

        self.assertEqual(tuple(result.shape), (1, 2, n, n))
        self.assertEqual(tuple(model.packed.shape), (1, 4, n, n))
        self.assertTrue(torch.equal(model.packed[:, 2:3], mask))
        self.assertTrue(torch.allclose(model.packed[:, 3:4], sponge / 5.0))
        self.assertEqual(tuple(model.params.shape), (1, 1))
        self.assertTrue(torch.isfinite(model.params).all())

    def test_physics_checkpoint_benchmark_round_trip(self):
        research_dir = Path(__file__).resolve().parents[2] / "reyn-research"
        sys.path.insert(0, str(research_dir))
        from flow_contract import fixed_body_v2_metadata
        from time_moe_operator import DirectFlowMap

        config = {
            "in_channels": 4,
            "out_channels": 2,
            "width": 8,
            "trunk_depth": 1,
            "time_dim": 8,
            "dt_scale": 0.01,
            "param_dim": 1,
        }
        model = DirectFlowMap(**config)
        train_args = {
            "dataset": "standard",
            "epochs": 1,
            "seed": 0,
            "grid_size": 16,
            "dt": 0.01,
            "stride": 1,
            "warmup_steps": 1,
            "max_steps": 2,
        }
        checkpoint = {
            "model_config": config,
            "model_state_dict": model.state_dict(),
            "train_args": train_args,
            "epoch": 1,
            "checkpoint_role": "fixed_final",
            "source_fingerprint": {"digest": "test-source"},
            "physics_spec": fixed_body_v2_metadata(),
        }
        engine = Engine.__new__(Engine)
        engine.research_dir = str(research_dir)
        engine.torch = torch
        engine.device = torch.device("cpu")
        engine.cache = {}
        engine.traj2d = {}
        engine.cad_cache = {}

        with tempfile.TemporaryDirectory() as directory:
            path = str(Path(directory) / "physics.pth")
            torch.save(checkpoint, path)
            suite = engine.run_benchmark({
                "model": path,
                "seeds": [70000],
                "horizons": [1],
            })
            cell, meta = engine.inspect_benchmark_cell({
                "model": path,
                "seed": 70000,
                "horizon": 1,
            })

        self.assertEqual(suite["provenance"]["verdict"], "clean")
        self.assertEqual(np.asarray(cell).shape, (3 * 16 * 16,))
        self.assertTrue(np.isfinite(cell).all())
        self.assertTrue(np.isfinite(meta["rel_l2"]))


if __name__ == "__main__":
    unittest.main()
