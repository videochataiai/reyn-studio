import unittest
from pathlib import Path
import sys

import numpy as np

from engine.n5_inspector import (
    inspector_payload,
    inspector_variable_maps,
    recovered_pressure,
    spatial_divergence,
)
from engine.reyn_engine import Engine

try:
    import torch
except ImportError:
    torch = None


class InspectorVariableEvidenceTests(unittest.TestCase):
    def test_taylor_green_pressure_matches_analytic_solution(self):
        n = 48
        axis = 2.0 * np.pi * np.arange(n) / n
        y, x = np.meshgrid(axis, axis, indexing="ij")
        velocity = np.stack([
            np.sin(2.0 * x) * np.cos(2.0 * y),
            -np.cos(2.0 * x) * np.sin(2.0 * y),
        ])
        expected = 0.25 * (np.cos(4.0 * x) + np.cos(4.0 * y))

        pressure = recovered_pressure(velocity)
        relative_l2 = np.linalg.norm(pressure - expected) / np.linalg.norm(expected)

        self.assertLess(relative_l2, 1e-6)
        self.assertAlmostEqual(float(pressure.mean()), 0.0, places=6)

    @unittest.skipIf(torch is None, "torch is not installed")
    def test_pressure_matches_reyn_canonical_torch_operator(self):
        research_dir = Path(__file__).resolve().parents[2] / "reyn-research"
        sys.path.insert(0, str(research_dir))
        from flow_quantities import pressure_from_velocity

        n = 32
        axis = 2.0 * np.pi * np.arange(n) / n
        y, x = np.meshgrid(axis, axis, indexing="ij")
        velocity = np.stack([
            np.sin(2.0 * x) * np.cos(3.0 * y),
            -0.4 * np.cos(4.0 * x) * np.sin(y),
        ])

        expected = pressure_from_velocity(
            torch.from_numpy(velocity).unsqueeze(0)
        ).numpy()
        actual = recovered_pressure(velocity)

        self.assertTrue(np.allclose(actual, expected, atol=2e-6))

    def test_variable_maps_include_pointwise_divergence_evidence(self):
        n = 48
        axis = 2.0 * np.pi * np.arange(n) / n
        y, x = np.meshgrid(axis, axis, indexing="ij")
        truth = np.stack([
            np.sin(x) * np.cos(y),
            -np.cos(x) * np.sin(y),
        ])
        prediction = truth.copy()
        prediction[0] += 0.2 * np.sin(3.0 * x)

        evidence = inspector_variable_maps(prediction, truth)
        maps = evidence["maps"]
        div_index = evidence["variables"].index("divergence")
        expected_divergence_error = 0.6 * np.cos(3.0 * x)

        self.assertEqual(
            evidence["variables"],
            ["velocity", "vorticity", "pressure", "divergence"],
        )
        self.assertEqual(evidence["signed"], [False, True, True, True])
        self.assertEqual(maps.shape, (4, 3, n, n))
        self.assertLess(float(np.sqrt(np.mean(maps[div_index, 1] ** 2))), 1e-6)
        self.assertTrue(
            np.allclose(
                maps[div_index, 2],
                expected_divergence_error,
                atol=2e-6,
            )
        )

        velocity_index = evidence["variables"].index("velocity")
        vector_error = np.sqrt(np.sum((prediction - truth) ** 2, axis=0))
        self.assertTrue(np.allclose(maps[velocity_index, 2], vector_error))

        payload, metadata = inspector_payload(prediction, truth)
        self.assertEqual(
            metadata,
            {
                "schema": "reyn.benchmark-inspector.maps.v2",
                "protocol_version": 2,
                "shape": [4, 3, n, n],
                "layout": "variable,model_reference_error,y,x",
                "variables": [
                    "velocity",
                    "vorticity",
                    "pressure",
                    "divergence",
                ],
                "signed": [False, True, True, True],
                "units": [
                    "solver_velocity_unit",
                    "inverse_solver_time_unit",
                    "solver_velocity_unit_squared",
                    "inverse_solver_time_unit",
                ],
                "panel_sources": [
                    ["MODEL", "SOLVER_REFERENCE", "DERIVED"],
                    [
                        "DERIVED_FROM_MODEL",
                        "DERIVED_FROM_SOLVER_REFERENCE",
                        "DERIVED",
                    ],
                    [
                        "RECOVERED_FROM_MODEL",
                        "RECOVERED_FROM_SOLVER_REFERENCE",
                        "DERIVED",
                    ],
                    [
                        "DERIVED_FROM_MODEL",
                        "DERIVED_FROM_SOLVER_REFERENCE",
                        "DERIVED",
                    ],
                ],
                "domain": "periodic_2pi",
                "derivative": "fourier_spectral_nyquist_zero",
                "pressure": "advective_poisson_density_normalized_zero_mean",
            },
        )
        self.assertTrue(np.array_equal(payload.reshape(metadata["shape"]), maps))

    def test_divergence_uses_x_for_u_and_y_for_v(self):
        n = 32
        axis = 2.0 * np.pi * np.arange(n) / n
        y, x = np.meshgrid(axis, axis, indexing="ij")
        field = np.stack([np.sin(2.0 * x), np.sin(5.0 * y)])
        expected = 2.0 * np.cos(2.0 * x) + 5.0 * np.cos(5.0 * y)

        self.assertTrue(
            np.allclose(spatial_divergence(field), expected, atol=3e-6)
        )

    def test_vorticity_mode_uses_dv_dx_minus_du_dy(self):
        n = 32
        axis = 2.0 * np.pi * np.arange(n) / n
        y, x = np.meshgrid(axis, axis, indexing="ij")
        model = np.stack([np.sin(4.0 * y), np.sin(3.0 * x)])
        truth = np.zeros_like(model)
        expected = 3.0 * np.cos(3.0 * x) - 4.0 * np.cos(4.0 * y)

        evidence = inspector_variable_maps(model, truth)
        vorticity_index = evidence["variables"].index("vorticity")

        self.assertTrue(
            np.allclose(
                evidence["maps"][vorticity_index, 0],
                expected,
                atol=3e-6,
            )
        )
        self.assertTrue(
            np.array_equal(
                evidence["maps"][vorticity_index, 0],
                evidence["maps"][vorticity_index, 2],
            )
        )

    def test_rejects_mismatched_or_nonfinite_fields(self):
        valid = np.zeros((2, 8, 8), dtype=np.float32)
        with self.assertRaisesRegex(ValueError, "share a shape"):
            inspector_variable_maps(valid, np.zeros((2, 9, 9)))
        valid[0, 0, 0] = np.nan
        with self.assertRaisesRegex(ValueError, "non-finite"):
            spatial_divergence(valid)


@unittest.skipIf(torch is None, "torch is not installed")
class InspectorEndpointTests(unittest.TestCase):
    def test_engine_endpoint_negotiates_full_schema_without_breaking_legacy(self):
        n = 16
        axis = 2.0 * np.pi * np.arange(n) / n
        y, x = np.meshgrid(axis, axis, indexing="ij")
        truth_np = np.stack([
            np.sin(x) * np.cos(y),
            -np.cos(x) * np.sin(y),
        ]).astype(np.float32)
        truth = torch.from_numpy(truth_np).unsqueeze(0)
        prediction = 0.9 * truth
        initial = torch.zeros_like(truth)
        trajectory = torch.cat([initial, truth], dim=0)

        engine = Engine.__new__(Engine)
        engine.torch = torch
        info = {
            "is3d": False,
            "ta": {"dt": 0.01, "stride": 4},
            "checkpoint_meta": {
                "epoch": 20,
                "checkpoint_role": "fixed_final",
                "source_fingerprint": {"digest": "fixture"},
                "train_args": {
                    "seed": 0,
                    "dataset": "standard",
                    "epochs": 20,
                },
            },
        }
        engine._load = lambda _model: info
        engine._traj2d = (
            lambda _model, _seed, _length, seed_offset=0: (
                initial,
                torch.zeros(1, 1, n, n),
                trajectory,
                {},
            )
        )
        engine._run_model_2d = (
            lambda _info, _state, _mask, _context, _horizon: prediction
        )
        request = {"model": "fixture.pth", "seed": 70000, "horizon": 1}

        legacy_payload, legacy_meta = engine.inspect_benchmark_cell(request)
        full_payload, full_meta = engine.inspect_benchmark_cell({
            **request,
            "evidence_schema": "reyn.benchmark-inspector.maps.v2",
        })

        self.assertEqual(legacy_meta["shape"], [3, n, n])
        self.assertEqual(legacy_payload.size, 3 * n * n)
        self.assertEqual(full_meta["shape"], [4, 3, n, n])
        self.assertEqual(
            full_meta["schema"], "reyn.benchmark-inspector.maps.v2"
        )
        self.assertEqual(full_meta["protocol_version"], 2)
        self.assertEqual(
            full_meta["layout"], "variable,model_reference_error,y,x"
        )
        self.assertEqual(full_payload.size, 4 * 3 * n * n)
        divergence = full_payload.reshape(full_meta["shape"])[3]
        self.assertLess(float(np.sqrt(np.mean(divergence[1] ** 2))), 1e-6)


if __name__ == "__main__":
    unittest.main()
