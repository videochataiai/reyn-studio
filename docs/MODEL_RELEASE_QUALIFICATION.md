# First production `.reynmodel`: release-qualification gate

**Assessment date:** 2026-07-25  
**Decision:** **BLOCKED — no current artifact is qualified**  
**Intended product lane:** Reyn Studio external-CAD execution  
**Required model lane:** geometry-conditioned 3D obstacle flow  
**Evidence status:** read-only source/document/result review; no checkpoint was deserialized or converted

## Executive decision

There is no `.reynmodel` in this workspace and no locally accessible `.pth`, `.ckpt`,
`.safetensors`, or `.onnx` checkpoint from which one can be produced. The workspace does
contain H64 evaluation records and older 2D/3D research claims, but not the model bytes
bound to those records. No artifact can therefore pass identity, conversion parity,
bundle verification, app import, scientific evaluation, signing, or rollback rehearsal.

Even if the referenced H64 checkpoint bytes are recovered, they are 2D
`DirectFlowMap` checkpoints. Studio's external STL path requires a 3D, 4-input/3-output,
geometry-conditioned obstacle checkpoint. The public site says the shipping product
runs the 3D operator. The documented 3D model is a 64³, H8 de-risk run that was
budget-limited and explicitly still awaiting cluster-scale training. It cannot support
an H32/H64/H128 production claim.

The first production model is releasable only when all four gates below pass for the
same immutable artifact:

1. **Scientific gate:** sealed held-out H32/H64/H128 evaluation, physics/load
   guardrails, and three-training-seed aggregation pass.
2. **Artifact gate:** provenance, calibration report, source checkpoint identity,
   benchmark hashes, deterministic conversion, and `.reynmodel` verification pass.
3. **Runtime/app gate:** CPU and MPS model smoke, 3D CAD geometry cases, save/reopen,
   evidence export, and rollback pass on supported clean machines.
4. **Distribution gate:** the exact model and app/runtime are authenticated; the app is
   Developer ID signed, notarized, stapled, and Gatekeeper-assessed.

A bundle that passes `convert_model_bundle.py verify` is structurally safe and
integrity-checked, but the verifier deliberately reports `authenticity: "unsigned"`.
That is not production qualification.

## Scope and evidence policy

Graphify was used as the navigation layer, then relevant source, documentation, and
result files were inspected directly. Obsidian CLI search was attempted read-only, but
the CLI could not find a running Obsidian instance; no vault result is included and no
vault file was changed.

Evidence labels in this report:

- **Current evidence:** a number or behavior present in an inspected source, result,
  or dated report.
- **Proposed release gate:** a threshold fixed here for future qualification. It is not
  an achieved result.
- **Missing:** required evidence was not found locally.

Historical reports are not treated as artifact-bound evidence unless a qualification
record can bind the exact checkpoint hash, evaluation manifest, evaluator source,
runtime, and result hash.

## Artifact and result inventory

### Model bytes

- Local `.reynmodel`: **missing**.
- Local legacy checkpoint (`.pth`/`.ckpt`): **missing**.
- Local raw Safetensors or ONNX candidate: **missing**.
- The source tree therefore contains **zero convertible local candidates**.

The result records below name remote paths under
`/home/ec2-user/reyn_research/`; those paths are metadata, not locally accessible
artifacts:

- `obstacle_physics_h64_v3_data512_updates5120_seed0_raw_epoch40.pth`
  (`H64v3_D_data512_u5120`);
- `obstacle_physics_h64_v3_data512_updates5120_seed0_ema_epoch40.pth`;
- `obstacle_physics_h64_v3_data256_updates5120_seed0_raw_epoch80.pth`
  (`H64v3_C_data256_u5120`);
- `obstacle_physics_h64_v3_energy_dissipation_matched_control_seed0_epoch40.pth`;
- `obstacle_physics_h64_v3_resolved_energy_dissipation_screen_seed0_epoch40.pth`;
- `obstacle_physics_h64_v3_innovation_control_seed0_epoch40.pth`;
- `obstacle_physics_h64_v3_vortex_transport_screen_seed0_epoch40.pth`.

Do not run the conversion command on recovered bytes until their custody, SHA-256, and
trusted first-party status are established. Conversion uses
`torch.load(..., weights_only=False)` and may execute pickle payload code.

### Locally accessible evaluation records

The principal candidate evidence is under
`reyn-research/.tmp_remote_eval_patch/results/`:

- `h64_v3_fixed_update_C_vs_D.json`
- `h64_v3_compute_scaling_ctrl_vs_C.json`
- `h64_v3_D_viscosity_raw_seed86000.json`
- `h64_v3_D_viscosity_ema_seed86000.json`
- `h64_v3_energy_control_vs_dissipation.json`
- `h64_v3_energy_dissipation_efficacy.json`
- `h64_v3_vortex_control_vs_transport.json`
- `h64_v3_vortex_paired_metrics.json`
- `h64_v3_vortex_transport_efficacy.json`

These records are useful candidate-selection evidence, but their temporary location,
absence of local checkpoint bytes, and absence of a signed release ledger prevent them
from being release evidence on their own.

## Product support envelope

### Required envelope for the first shipping model

The first production model for Studio's external-CAD workflow must declare and pass:

- **dimension:** 3;
- **architecture:** `reyn.direct-flow-map.3d/1`;
- **channels:** 4 inputs (3 velocity + solid fraction), 3 velocity outputs;
- **scenario/physics:** `obstacle` / `fixed_body_brinkman.v1`;
- **grid:** one exact trained grid, matched by STL voxelization;
- **horizon:** 1–128 steps for this release gate;
- **direction:** +X free stream only;
- **Reynolds envelope:** 60–400, tested at the boundaries and interior bands;
- **geometry:** fixed, external, watertight STL bodies after classifier-v2 preflight;
- **excluded:** internal/HVAC flow, thermal/compressible/multiphase physics,
  out-of-envelope Reynolds numbers, horizons beyond 128, and grid-independence claims.

Studio enforces 3D, grid equality, obstacle scenario, channel shape, model hash, the
declared horizon, +X flow, and Reynolds 60–400. A release report must not broaden those
limits.

### Contract gap that must be closed by evidence

The 3D bundle schema records one scalar `kinematic_viscosity`, while Studio develops
each CAD initial state with `nu = characteristic_length / Reynolds` and accepts
Reynolds 60–400. The 3D model has no explicit viscosity input. Consequently, the site
and UI envelope is broader than the model manifest can express.

For the first release, the sealed benchmark report must bind all Reynolds strata and
show that the same model passes them. Otherwise narrow the product envelope to the
actually qualified viscosity/Reynolds regime before release. A fixed scalar in the
manifest plus an untested 60–400 UI gate is insufficient.

### Site-claim reconciliation

Current site claims and caveats are internally useful but do not qualify a model:

- Shipping is described as 3D.
- 3D evidence reports whole-field RelL2 `0.006–0.007` at H8 on one held-out
  obstacle case, 64³, 24 trajectories, 18 geometries, width 32, 15 epochs.
- The same site labels this a single-Mac de-risk run that was still improving and
  says cluster-scale training is next.
- The 3D wake metric is near persistence at H1 and becomes better around H8.
- Mature 2D evidence (`0.0030` held-out multi-shape RelL2 and 2D load claims) is
  explicitly separate from the shipping 3D lane.
- No installer is published and `packagingComplete` is false.

Therefore the 2D numbers may inform proposed thresholds, but must not be presented as
achieved 3D production accuracy.

## Current experiment evidence

### H64 candidate D — current 2D evidence only

`h64_v3_fixed_update_C_vs_D.json` evaluates one training seed (seed 0) on 32 fresh
trajectories: four test-seed streams × eight trajectories. For
`H64v3_D_data512_u5120` raw fixed-final weights:

| Horizon | Whole-field RelL2 mean (95% case-bootstrap CI) | Historical `wake_rel_l2` mean (95% CI) |
|---|---:|---:|
| H32 | 0.046887 (0.032224–0.063720) | 0.086041 (0.061868–0.111324) |
| H64 | 0.089419 (0.061305–0.121888) | 0.167313 (0.118786–0.221385) |
| H128 | **Missing; model/evaluation stops at H64** | **Missing** |

Here `wake_rel_l2` is not a spatial wake metric. It is relative L2 of
`prediction - (1,0)` over the whole domain, including the solid. It is called
**whole-perturbation RelL2** below.

Across H1–H64, D minus C whole-perturbation mean difference was `-0.006279`
(95% CI `-0.011851` to `-0.002076`) on the same 32 cases. This supports D over C
for that endpoint under one training seed; it does not qualify D for production.

The raw D causal-viscosity record reports aggregate correct-conditioning fluid-only
perturbation RelL2 `0.092066` (95% CI `0.065078–0.118713`, 16 same-state base
cases), correct-minus-wrong `-0.007680` (`-0.010672` to `-0.005207`), and
sensitivity ratio `0.594` (`0.568–0.618`). The completed historical H64-v3 gate
separately records sensitivity `0.354` (0.310–0.403); the protocols/checkpoints must
not be conflated.

### Completed negative and inconclusive evidence

- The matched H64-v3 safeguard against the historical H64 endpoint failed over
  H33–H64: whole-field `+8.5%`, whole-perturbation `+9.4%`, vorticity `+8.0%`,
  divergence-error MSE `+20.4%`, and divergence RMS `+10.0%`; every paired 95% CI
  was above zero. The research decision explicitly blocked H128 and H64-v3 seed
  replication at that point.
- The energy/dissipation seed-0 candidate improved several direct metrics, but failed
  to establish wake-energy non-inferiority at the preregistered 1.05 ceiling. It is
  **promising but gate-inconclusive**, default off.
- The vortex-transport weight-0.01 candidate regressed divergence-error MSE by 17.61%
  and divergence RMS by 7.63%; its transport geometry was worse. It is
  **rejected/deferred**.
- H128 scale-up is documented as **prepared, not launched**. No H128 checkpoint or
  H128 evaluation result was found.
- The 3D record reaches H8, not H32/H64/H128. No three-seed 3D release study,
  true-wake study, 3D load-validation report, or production calibration report was
  found.

## Proposed scientific release gate

All thresholds in this section are **proposed release gates**, not achieved results.
They must be frozen before opening the sealed release set.

### Required endpoint table

The release report must contain per-case values, per-training-seed summaries, and the
three-seed aggregate at H32, H64, and H128. It must also provide curves/AUC over every
integer horizon H1–H128; endpoint-only optimization is not sufficient.

| Metric | H32 gate | H64 gate | H128 gate |
|---|---:|---:|---:|
| Whole-field RelL2 mean | ≤ 0.05 | ≤ 0.10 | ≤ 0.15 |
| Whole-perturbation RelL2 mean | ≤ 0.10 | ≤ 0.20 | ≤ 0.30 |
| Upper 95% CI / corresponding absolute mean ceiling | ≤ 1.30× | ≤ 1.30× | ≤ 1.30× |
| Model/persistence mean ratio, both L2 endpoints | ≤ 0.80 | ≤ 0.80 | ≤ 0.80 |
| Upper 95% CI of model/persistence ratio | < 1.00 | < 1.00 | < 1.00 |
| Candidate/locked-reference upper 95% ratio CI | ≤ 1.05 | ≤ 1.05 | ≤ 1.05 |

The H32/H64 absolute ceilings are deliberately coarse guardrails around the available
2D H64 evidence, not estimates of 3D accuracy. The H128 ceiling is a release ceiling,
not an extrapolated achieved value. Comparative and persistence gates remain mandatory
because dimensionality, geometry, and signal scale make absolute RelL2 alone unsafe.

### True wake and regional field gate

Because no current true spatial-wake result exists, no achieved number is claimed.
The release evaluator must add, without replacing the historical endpoint:

- fluid-only perturbation RelL2;
- geometry-relative near-wake RelL2;
- far-wake RelL2;
- a fluid-side boundary-annulus RelL2;
- per-component velocity errors;
- wake-energy relative error.

The region definition, body centroid/scale method, mask threshold, denominator floor,
and implementation version must be frozen in the benchmark contract. At every
H32/H64/H128 endpoint:

- each wake metric's mean must be ≤ `0.95 × persistence`;
- its paired model/persistence 95% ratio CI must have upper bound < `1.00`;
- its candidate/reference 95% ratio CI must have upper bound ≤ `1.05`;
- no geometry or Reynolds stratum with at least eight independent cases may exceed
  `1.05 × reference` in mean;
- wake-energy candidate/reference upper 95% ratio CI must be ≤ `1.05`.

### Held-out geometry, viscosity, and horizon design

The sealed set must be content-hashed before candidate selection and disjoint from
training, checkpoint validation, development testing, and threshold tuning.

Minimum design per training seed:

- at least 32 independent base cases;
- at least four evaluation RNG streams with eight cases each;
- unseen geometry families plus unseen parameterizations within families;
- Reynolds strata `60–150`, `(150,275)`, and `275–400`, including exact 60 and 400
  boundary cases;
- all integer horizons 1–128, with preregistered H1–H32, H33–H64, H65–H96, and
  H97–H128 bands;
- paired solver truth, persistence, locked reference, and candidate on identical
  initial state, geometry, viscosity/Reynolds setting, and horizon.

The scientific geometry set must include bluff, curved, elongated, lifting, and
thin-feature cases. The repository's cube, sphere, cross-flow cylinder, capsule, and
NACA0012 wing are suitable pipeline fixtures, but release accuracy needs sealed variants
not used to tune preprocessing.

For a viscosity-conditioned 2D fallback study, use complete same-state forks and report
correct, mean, and wrong-viscosity controls. Correct conditioning must beat both controls
with paired 95% CIs below zero, and sensitivity must not fall more than 10% from the
locked reference. For the 3D production model, which has no explicit viscosity input,
the Reynolds-stratified test above is mandatory and cannot be replaced by 2D fork
evidence.

### Divergence, solid, stability, and load gate

At H32/H64/H128 and for horizon-band AUC:

- divergence-error MSE and prediction divergence RMS must not regress in mean against
  the locked reference;
- at least one divergence metric must improve by ≥3% in mean;
- each divergence candidate/reference upper 95% ratio CI must be ≤1.05;
- solid compliance/velocity energy must not regress in mean and its upper ratio CI must
  be ≤1.05;
- all predictions, recovered pressure, Cp, tractions, forces, moments, and exported
  fields must be finite;
- no case may mutate the input, violate shape/dtype, or become nondeterministic on CPU.

Load validation must use solver-derived reference pressure/loads on the same held-out
3D cases. The current 2D site claim of 0.5–1.6% load accuracy is context only, not a 3D
result. Proposed 3D gates:

- force-coefficient vector relative L2 mean ≤5%, upper 95% CI ≤10%;
- moment-coefficient vector relative L2 mean ≤10%, upper 95% CI ≤15%;
- drag sign correct in every nondegenerate case;
- side/lift signs correct where the reference magnitude exceeds the preregistered
  denominator floor;
- no qualifying geometry/Reynolds stratum has mean force error >10%;
- symmetry cases have side/lift within their solver/tolerance envelope;
- load and suction hotspot displacement is reported in body lengths and must meet a
  preregistered, solver-resolution-aware tolerance.

Pressure recovered from model velocity and diffuse-interface loads must remain labeled
model-derived fluid loads, not structural stress.

### Training seeds and confidence intervals

- Train/evaluate seeds 0, 1, and 2 under one frozen contract.
- Report each training seed separately.
- Within each seed, bootstrap paired base cases, preserving all horizons and viscosity
  branches for a base case as one cluster; use at least 2,000 resamples.
- Report the across-training-seed mean and range. With only three training seeds, do not
  claim a precise population CI from pooled trajectories.
- Both primary L2 endpoints must pass in at least two of three training seeds and the
  three-seed mean must pass.
- A pooled case bootstrap is secondary and must not represent repeated cases from one
  trained model as independent training replicates.
- Selection uses checkpoint-validation/development data. Open the sealed release set
  once, after model ID, weight variant, thresholds, and code hashes are frozen.

## Provenance, calibration, and benchmark reports

The release ledger must bind:

- custody record and SHA-256 of the trusted legacy checkpoint;
- selected role (`fixed_final_raw` or `fixed_final_ema`), epoch, declared epochs, and
  explicit raw/EMA variant;
- model config, full experiment contract, training seed, dataset/manifests, optimizer
  updates, horizon sampling, augmentation, and EMA details;
- training-source fingerprint and exact evaluator-source fingerprint;
- solver identity/configuration; geometry, train/validation/development/release manifest
  hashes; no-overlap/leak-check result;
- Python, PyTorch, Safetensors, NumPy, CUDA/MPS/CPU, OS, hardware, dtype, and device;
- `.reynmodel` bundle SHA-256, `weights.safetensors` SHA-256, and canonical manifest
  bytes/hash;
- post-conversion numerical parity against the trusted source checkpoint on canonical
  inputs;
- scientific qualification JSON, plots, logs, calibration JSON, model card, licenses,
  and their SHA-256 values;
- evaluator verdict, approvers, date, and explicit limitations.

The `.reynmodel` manifest supports only SHA-256 strings in `benchmark_reports`; it does
not embed reports. Release requires a non-empty list whose hashes resolve to immutable
companion reports in the release ledger.

Current tooling permits an empty report list, and the converter has no CLI option to
attach report hashes. Unless the selected trusted checkpoint already contains correct
`benchmark_reports` metadata, this is an artifact-tooling blocker. Do not hand-edit the
ZIP: canonical manifest, file inventory, and deterministic identity would be invalidated.

## Conversion and safe bundle verification

Run conversion only in an isolated offline environment after the checkpoint is
first-party-trusted and its pre-conversion SHA-256 has been recorded:

```bash
cd "/Users/hamza/Documents/Pioneer RI/reyn-research"
uv run python convert_model_bundle.py convert \
  "$TRUSTED_CHECKPOINT" "$QUAL_DIR/reyn-flow3d-h128-v1.reynmodel" \
  --model-id reyn-flow3d-h128 \
  --model-version 1.0.0 \
  --source-digest "$TRAINING_SOURCE_SHA256" \
  --trusted
```

Then verify without pickle deserialization:

```bash
uv run python convert_model_bundle.py verify \
  "$QUAL_DIR/reyn-flow3d-h128-v1.reynmodel" \
  | tee "$QUAL_DIR/bundle-verification.json"
shasum -a 256 "$QUAL_DIR/reyn-flow3d-h128-v1.reynmodel"
```

Verification must report:

- schema `com.reyn.inference-model-bundle/1`;
- architecture `reyn.direct-flow-map.3d/1`;
- expected model ID/version and tensor count;
- exact bundle and weights SHA-256;
- `ok: true`;
- `authenticity: "unsigned"` before the separate signing/distribution gate.

The loader additionally enforces a canonical two-member ZIP_STORED archive
(`manifest.json`, `weights.safetensors`), bounded sizes/header/tensor count, safe paths,
exact architecture/tensor schema, finite tensors, file hash, support envelope, and
source metadata.

## Runtime, app, and packaging gate

### Current state

- Studio version is 0.1.1; bundle ID is `com.reyn.studio`; minimum declared macOS is
  11.0.
- Apple silicon/Metal is the exercised product target. Intel compute is not established.
- Python is not bundled. The local package resolves `REYN_PYTHON`, a research virtual
  environment, then `python3`.
- Research modules are bundled, including `model_bundle.py`; model checkpoints are not.
- The Python project requires Python ≥3.14 and dependencies including NumPy ≥2.5,
  PyTorch ≥2.12.1, and Safetensors ≥0.8.
- Packaging's runtime probe currently declares only NumPy and PyTorch, despite
  `model_bundle.py` importing Safetensors. A clean runtime can pass the declared probe
  and still fail model import. This is a release blocker.
- Packaging metadata still advertises `.pth` checkpoint locations, while the production
  engine lists/imports/loads only `.reynmodel`. This contract drift is a release blocker.
- The built-in Benchmark Lab currently rejects 3D checkpoints and uses only horizons
  `[1,4,8,16]`; it cannot execute the production H32/H64/H128 qualification. The sealed
  3D evaluator/report must therefore be supplied externally and app-bound by hash.
- The local packaging workflow is explicitly not standalone, not Developer ID signed,
  and not notarized.

### Required app smoke

On each supported clean Apple-silicon machine/OS/device matrix:

1. Verify the downloaded model signature/authorized metadata before import.
2. Import only the `.reynmodel`; confirm the model card is `clean`, 3D, 4→3,
   obstacle, fixed-final, grid-matched, H128, and shows the expected bundle/source/report
   hashes and limitations.
3. Run the same canonical case twice on CPU; require bitwise deterministic fields and
   metadata.
4. Run on MPS; compare to CPU using separately frozen field/load tolerances and record
   any fallback. A hidden performance cliff is a failure.
5. Exercise H32, H64, and H128 on a valid cube, sphere, capsule, and sealed lifting-body
   STL. Confirm finite velocity, pressure, Cp, traction, force, and moment output.
6. Confirm classifier-v2 accepts valid fixtures; hard-blocks the open sphere, missing
   cap, nested shell, and intersecting boxes; and does not permit a scientific release
   case through a geometry waiver.
7. Confirm grid mismatch, 2D model, wrong channels, unsupported scenario, H129,
   Reynolds below 60/above 400, empty/nonfinite mask, malformed bundle, tampering, and
   renamed pickle all fail closed.
8. Save, close, reopen, and verify the project/run ledger retains model SHA-256,
   case revision, horizon, operating point, fields, and load provenance.
9. Export and independently verify evidence/load artifacts.
10. Launch the packaged app through LaunchServices, test offline, and repeat after
    quarantine on the exact signed/notarized archive.

Valid geometry evidence already records 0% three-axis disagreement for cube, sphere,
capsule, and the cross-flow cylinder. The cylinder was one-cell thick at 48³ and keeps
an explicit under-resolution warning. Defective fixtures are hard-blocked by
open-edge/disagreement, nested-shell, or self-intersection checks. Scientific benchmark
cases must have no geometry waiver.

### Unsigned versus signed status

There are two independent trust boundaries:

- **Model bundle:** SHA-256 proves identity/integrity, not publisher authenticity.
  Current verification always reports unsigned and Studio import does not establish a
  publisher signature.
- **macOS app/runtime:** current packaging does not perform Developer ID signing,
  hardened-runtime review, notarization, stapling, or Gatekeeper assessment.

Production release requires authorized metadata/signature for the exact bundle and
companion reports, plus Developer ID signing/notarization of the exact app/runtime
distribution. An ad-hoc Mach-O signature or a locally computed hash does not satisfy
this gate.

## Rollback

Before activation:

- retain the previous qualified model, report set, signature metadata, app/runtime
  version, and compatibility matrix;
- install the candidate under a new immutable model ID/version and filename;
- never overwrite the previous bundle;
- record active, previous, and last-known-good identities atomically;
- prove saved projects remain readable with their recorded model hash even if compute
  is unavailable.

Rollback immediately on signature/hash mismatch, import/load failure, deterministic
smoke failure, nonfinite output, repeated sidecar crash/startup timeout, CPU/MPS
tolerance failure, support-envelope bypass, materially incorrect loads, or post-release
metric regression. Reactivate the previous qualified tuple (app + runtime + model +
reports), quarantine the candidate, preserve diagnostics, and do not relabel old runs.

The current model-import directory uses unique filenames but no inspected production
model activation/previous-pointer mechanism. Until model rollback is implemented and
rehearsed, the distribution gate remains blocked.

## Hard rejection criteria

Reject, rather than waive, a candidate if any of the following occurs:

- missing or untrusted checkpoint custody/identity;
- no local bytes to bind to the claimed results;
- 2D artifact offered for external-CAD production;
- maximum horizon below 128 for this release;
- missing H32, H64, or H128 sealed result;
- any primary absolute, persistence, reference, wake, divergence, solid, or load gate
  fails;
- fewer than three training seeds or failed two-of-three replication;
- selection on sealed release data, train/test overlap, source mismatch, or evaluator
  identity ambiguity;
- incomplete/floating checkpoint role, epoch mismatch, implicit raw/EMA selection, or
  missing experiment contract;
- nonfinite training/evaluation/output, fork incoherence, or nondeterminism;
- unsupported/malformed geometry admitted to a scientific run;
- empty/unresolvable benchmark-report hashes or failed conversion parity;
- malformed/tampered/oversized/noncanonical bundle or production pickle load;
- unsigned/unapproved model distribution, unsigned/unnotarized app, missing runtime
  dependency closure, or failed clean-machine smoke;
- site/support claims broader than the qualified manifest and benchmark envelope;
- rollback not available or not rehearsed.

## Minimal executable acceptance checklist

This checklist intentionally fails today at the first model-byte assertion.

```bash
set -euo pipefail

ROOT="/Users/hamza/Documents/Pioneer RI"
CHECKPOINT="${CHECKPOINT:?trusted first-party checkpoint path required}"
QUAL_DIR="${QUAL_DIR:?external immutable qualification directory required}"
BUNDLE="$QUAL_DIR/reyn-flow3d-h128-v1.reynmodel"

test -f "$CHECKPOINT"
test -f "$QUAL_DIR/scientific-qualification.json"
test -f "$QUAL_DIR/calibration.json"
test -f "$QUAL_DIR/release-manifest.json"
shasum -a 256 "$CHECKPOINT" \
  "$QUAL_DIR/scientific-qualification.json" \
  "$QUAL_DIR/calibration.json" \
  "$QUAL_DIR/release-manifest.json"

cd "$ROOT/reyn-research"
uv run pytest -q test_model_bundle.py
uv run python convert_model_bundle.py convert \
  "$CHECKPOINT" "$BUNDLE" \
  --model-id reyn-flow3d-h128 \
  --model-version 1.0.0 \
  --source-digest "${TRAINING_SOURCE_SHA256:?required}" \
  --trusted
uv run python convert_model_bundle.py verify "$BUNDLE" \
  | tee "$QUAL_DIR/bundle-verification.json"

cd "$ROOT/reyn-studio"
python3 -m unittest \
  engine/test_reyn_engine.py \
  engine/test_n5_inspector.py \
  engine/test_n5_overlap.py
cargo test
SOURCE_DATE_EPOCH=315532800 python3 scripts/package_macos.py \
  --target aarch64-apple-darwin \
  --build-number "${BUILD_NUMBER:?required}" \
  --require-standalone \
  --require-runnable-architectures
python3 scripts/validate_macos_bundle.py \
  "dist/macos/Reyn Studio.app" \
  --expect-target aarch64-apple-darwin \
  --require-standalone \
  --require-runnable-architectures
```

Passing these commands is necessary, not sufficient. The current repository has no
executable 3D H32/H64/H128 Benchmark Lab path, no model/report authenticity verifier,
and no signed/notarized packaging workflow. The external scientific report, signature
verification, clean-machine matrix, app smoke, and rollback evidence must be attached
before approval.

## Ownership handoff

| Owner | Required deliverable | Approval condition |
|---|---|---|
| ML training | Trusted fixed-final 3D H128 checkpoints for seeds 0/1/2, source and experiment identities | Bytes and metadata complete; no role/epoch ambiguity |
| Evaluation | Sealed geometry/Re/H1–H128 report with per-case records, paired CIs, loads, leak check, and three-seed summary | Every scientific gate passes without post hoc threshold changes |
| Model release | Custody ledger, checkpoint hash, conversion parity, bundle/weights hash, benchmark hash resolution, model card/licenses | One immutable artifact/report set is reproducible |
| Studio/engine | 3D evaluator binding, dependency closure including Safetensors, `.reynmodel` runtime contract, model authenticity check | Clean CPU/MPS import and CAD smoke pass |
| CAD/physics | Classifier-v2 geometry matrix and 3D pressure/load reference validation | No scientific waiver; load thresholds pass |
| macOS release/security | Signed model metadata, Developer ID/hardened runtime, notarization/stapling, Gatekeeper and clean-machine evidence | Exact archived distribution passes |
| Product/site | Claim-to-envelope review | Site/UI claims do not exceed qualified dimension, Re, grid, horizon, loads, platform, or signing state |
| Release manager | Activation/rollback rehearsal and final evidence index | Previous tuple can be restored without corrupting project evidence |

No owner should approve by inheriting another row's evidence. Final approval requires
all rows to reference the same bundle SHA-256 and release-manifest hash.

## Primary inspected sources

- `graphify-out/graph.json`
- `reyn-research/model_bundle.py`
- `reyn-research/convert_model_bundle.py`
- `reyn-research/test_model_bundle.py`
- `reyn-research/evaluate_horizon_ablation.py`
- `reyn-research/docs/CFD_L2_OPTIMIZATION_RESEARCH.md`
- `reyn-research/docs/H128_SCALE_UP_PLAN.md`
- `reyn-research/docs/ML_INNOVATION_LOG.md`
- `reyn-research/RESEARCH_REPORT.md`
- `reyn-research/product/03_MODEL_INTEGRATION_SPEC.md`
- `reyn-studio/engine/reyn_engine.py`
- `reyn-studio/src/engineering.rs`
- `reyn-studio/src/app.rs`
- `reyn-studio/scripts/macos_packaging.py`
- `reyn-studio/docs/PYTHON_RUNTIME_DISTRIBUTION.md`
- `reyn-studio/docs/MACOS_RELEASE.md`
- `reyn-studio/docs/VOXEL_FIDELITY_EVIDENCE.md`
- `reyn-studio/test-geometry/make_test_stls.py`
- `reyn-studio/test-geometry/make_defective_stls.py`
- `reyn-site/src/config.ts`
