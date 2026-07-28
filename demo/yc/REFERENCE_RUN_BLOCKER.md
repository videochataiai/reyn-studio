# Reference-run feasibility decision

Decision: **do not generate or load a capsule reference-run project with the current
engineering result contract.** Doing so would require false model identity or would
display unavailable quantities as measured results.

## What is technically available

- `reyn-research/obstacle_solver_3d.py::ObstacleFlowSolver3D` is a deterministic,
  bounded CPU reference solver at 16³ or 32³ when the seed, grid, timestep,
  viscosity, mask, and step count are fixed.
- Its optional pressure channel is the mean-zero periodic projection multiplier
  defined by `reyn.flow-field-3d.velocity-projection-pressure/1`. It is relative,
  nondimensional projection pressure, not absolute thermodynamic pressure.
- The portable project manifest can identify a solver component and can classify an
  evidence artifact as `solver_reference`.

Those pieces are not yet connected by an honest Reyn Studio result contract.

## Blocking mismatches in the real result path

1. `hydrate_engineering_run` requires `exact_contract.model` and reconstructs an
   `ExternalFlowCase` around model identity and support.
2. `engineering_field.f32le.v1` requires all nine arrays: velocity, `pressure_pa`,
   mask, Cp, and traction. It has no per-quantity availability or source class.
3. The completed-run writer classifies velocity as `model_prediction`, pressure as
   recovered from model velocity, and Cp/traction as derived from that prediction.
4. The VTK and FEA exporters require a model SHA-256 and write model-specific source
   and method labels into exported evidence.
5. The surface-load calibration gate covers only a sphere at grid size 96 or above.
   A capsule at 16³ or 32³ therefore has **unavailable** pressure-plus-viscous loads.
   Filling traction or force fields with zeros would turn “unavailable” into a false
   numerical result.
6. The existing CAD engine operation develops the imported voxel mask with the
   reference solver, but only as hidden warmup before a model pass. There is no
   source-classed reference-only operation or production result panel.

The persistent label `REFERENCE SOLVER FIXTURE — NOT MODEL INFERENCE` cannot correct
the semantic errors above: the UI and exports would still contradict it.

## Minimum production path required

Before creating this fixture, Reyn Studio needs:

1. A source-aware engineering result schema with optional quantities and an explicit
   source class for every field and scalar.
2. A reference-only CAD engine request that records the exact imported-mask hash,
   solver/config/source hashes, pressure contract, seed, and stop condition.
3. Result, probe, section, evidence, report, and VTK views that derive labels and
   availability from the persisted source classes instead of model defaults.
4. An explicit unavailable-load disposition for uncalibrated geometry/grid pairs.
5. Loader and export tests proving that a `model: null`, `solver_reference` run
   remains source-honest after save, reopen, probe, and export.

Until those changes exist, `SCRIPT_FALLBACK.md` is the supported recording path.
No `.reynproj`, field blob, solver image, or result export is included for the
proposed capsule run.
