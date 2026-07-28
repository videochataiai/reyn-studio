---
title: Reyn Studio YC demo
duration_seconds: 160
format: screen-only with founder narration
---

# Timed storyboard and exact narration

## 00:00–00:12 — Open on the imported capsule

**Action:** Start only after the capsule is visible in Case Setup. Keep the project
rail and source summary on screen.

**Expected UI:** `primary_capsule_d80_l260mm`, preflight stage, no result.

**Narration:** “Engineering teams can get a fast flow prediction, but the hard part is
knowing whether the geometry, model, and result are actually usable. Reyn Studio makes
those checks part of the workflow.”

## 00:12–00:34 — Prove source identity and topology

**Action:** Open **Source / Preflight**. Point to SHA-256, 1,920 triangles, source
extents, zero open edges, and the 64-cubed diagnostic grid.

**Expected UI:** The real fixture hash begins `ca2b82f7`; topology is closed. A model
gate warning remains visible.

**Narration:** “This is a real 1,920-triangle capsule STL, generated in millimetres.
Reyn hashes the source, records its extents and transform, checks topology, and runs
three-axis occupancy diagnostics. This body is closed; the hash and every assumption
stay attached to the case.”

## 00:34–00:58 — Confirm physical setup

**Action:** Set geometry units to **mm**. Approve the proposed transform. Set free-stream
speed to **0.01 m/s**; leave density at `1.225 kg/m³`, dynamic viscosity at
`1.81e-5 Pa·s`, pressure at `101325 Pa`, flow direction `+X`, horizon `4`.

**Expected UI:** Units and transform become confirmed; Reynolds number is about 203.
The case remains blocked only by the unavailable model/support contract.

**Narration:** “I confirm millimetres instead of letting the app guess. Then I set the
operating point: speed, density, viscosity, reference pressure, direction, and model
horizon. Reyn computes the Reynolds number, but setup validity is separate from model
applicability.”

## 00:58–01:20 — Show the honest model gate

**Action:** Expand **Model support** and briefly open **Model Library**.

**Expected UI:** No compatible verified 3D bundle; inference is blocked. No synthetic
field or success state appears.

**Narration:** “Here is the important limitation. We do not currently have a qualified
3D production model in this workspace. The UI says that directly and blocks Run.
Geometry review still works, but Reyn will not turn a missing checkpoint into a fake
prediction.”

## 01:20–01:38 — Show provenance and unavailable exports

**Action:** Return to the case, open **Evidence** or the evidence summary, and point to
the source revision and content hash. Hover the disabled run/export action.

**Expected UI:** Source lineage remains present. Run-linked result and VTK/evidence
exports are unavailable because no completed persisted run exists.

**Narration:** “The source revision is already content-addressed and stored with the
case. Result exports stay unavailable because there is no completed immutable run.
That distinction—source evidence versus model output—is the product, not a footnote.”

## 01:38–02:08 — Demonstrate fail-closed geometry

**Action:** Choose **Import Geometry…** and select
`demo/yc/assets/defective_sphere_missing_cap_r50mm.stl`. Open Preflight and point to
the open-boundary diagnostic.

**Expected UI:** 1,680 triangles, 48 open boundary edges, `mesh.open_boundary`, and a
non-waivable execution block.

**Narration:** “Now I’ll import an intentionally broken STL. It is missing a cap. Reyn
finds 48 open boundary edges and blocks the case. This is not a warning I can dismiss
with a note, because inside-outside classification and surface loads would be
untrustworthy.”

## 02:08–02:30 — Contrast with the fallback

**Action:** In Finder, reveal `fallback_cube_100mm.stl` beside the manifest, or use the
manifest already open in the editor. Point to its 12 triangles, zero boundary edges,
and SHA-256. Do not import if the file dialog would cost time.

**Expected UI:** The manifest visibly distinguishes primary, fallback, and intentional
rejection fixtures.

**Narration:** “The package also has a twelve-triangle cube fallback, with deterministic
hashes and expected diagnostics for every file. So this demo is repeatable, and the
failure case is tested rather than staged.”

## 02:30–02:40 — Close

**Action:** Return to the blocked Case Setup screen with the geometry and model gate
both visible. Stop recording at 2:40.

**Expected UI:** Real preflight data; no completed neural result.

**Narration:** “Reyn Studio’s wedge is simple: fast neural CFD that stays inspectable
and refuses unsupported work. Today the workflow and evidence boundary are real; the
qualified 3D model and release gates are still work to finish.”

# Contingencies

- **Capsule has not imported when recording starts:** Wait off-camera. If it still does
  not appear, click **Import Geometry…** and select the primary fixture. Say:
  “I’m loading the same checked fixture manually; this is the ordinary import path.”
- **Engine reports unavailable:** Continue. Say: “The compute sidecar is unavailable,
  but local geometry review remains usable and the missing dependency is explicit.”
- **Capsule viewport is slow:** Reset and use `fallback_cube_100mm.stl`. Replace
  “capsule” with “100 millimetre cube” and “1,920” with “12”; keep every other claim.
- **Defective import takes too long:** Show `fixture-manifest.json` and say:
  “The automated package test measures 48 boundary edges and pins the same hard-gate
  diagnostic.”
- **A model unexpectedly appears:** Do not run it. Say: “A candidate bundle is present,
  but this demo has not reviewed artifact-bound qualification, so I am keeping
  inference blocked.”
- **Evidence navigation differs:** Stay in Case Setup and point to the visible source
  hash plus disabled Run. Do not imply that an export was produced.
