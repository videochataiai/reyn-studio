# Reyn Studio — Canonical Product Requirements

**Status:** canonical implementation contract  
**Current-state baseline:** 2026-07-30 · corrective release 0.1.1
**Applies to:** Reyn Studio native app, Python engine integration, project schema, evidence, packaging, and product UX  
**Companion evidence:** [`docs/CFD_APP_LANDSCAPE.md`](docs/CFD_APP_LANDSCAPE.md)

This document is the product authority for Reyn Studio. It replaces screen-led milestone notes
as the forward implementation contract while preserving the verified native-track history.
Detailed vendor comparisons, source links, and the dated bibliography remain in the companion
landscape report; they are evidence for this PRD, not duplicated requirements.

Supporting current-state references: [`README.md`](README.md) and [`PRODUCT.md`](PRODUCT.md).

---

## 1. Product vision and position

Reyn Studio is a **local-first neural-CFD scientific instrument** for creating, interrogating,
comparing, and preserving defensible incompressible-flow predictions. A native Rust/egui/wgpu
application provides responsive 2D/3D interaction; a Python sidecar runs PyTorch models,
reference solvers, and scientific derivations behind an engine boundary.

Reyn is not a general-purpose replacement for Fluent, STAR-CCM+, COMSOL, SimScale, OpenFOAM, or
SU2. Its wedge is narrower and more defensible:

- make neural-flow model applicability and limitations legible;
- put prediction, numerical reference, derivation, provenance, and integrity evidence in one
  coherent workflow;
- preserve the exact relationship between source, contract, run, and evidence;
- remain useful locally and offline, including read-only review when compute dependencies fail.

The product promise is: **a technical user can move from a source or checkpoint to an
evidence-complete result and explain what produced every important number.**

### 1.1 Product principles

1. **Evidence before spectacle.** Visualization exists to measure, compare, and decide.
2. **Honest by construction.** Unknown, unsupported, recovered, derived, and independently
   checked states remain distinct.
3. **Durable engineering objects.** Screens never substitute for project, case, run, and
   evidence lineage.
4. **Fast expert interaction.** The shell stays responsive; long engine work is asynchronous.
5. **Progressive disclosure.** Broad capability is organized into coherent journeys, not exposed
   as every possible control at once.
6. **Local ownership.** Public source builds are account-free. Official gated artifacts require YC
   authentication before constructing the studio or starting compute; after access is granted,
   project data, execution, export, and review remain local.

### 1.2 Non-goals

- Full multiphysics breadth, a universal materials/boundary-condition editor, or arbitrary CFD
  setup beyond supported model/reference contracts.
- A full direct or parametric CAD kernel.
- A universal high-fidelity mesher or solver.
- In-product model training before independent split lineage, qualification, and applicability
  evidence are durable.
- Mandatory cloud storage, collaboration, or remote execution.
- A black-box confidence score that conflates consistency, verification, validation, and accuracy.
- Feature-count parity with incumbents or permanent exposure of every expert control.

---

## 2. Audiences and jobs

These segments are confirmed by the existing product definition. They are target users, not yet
claims of commercial validation.

### 2.1 B2B

**Physics-ML and surrogate-model teams**

- Qualify a checkpoint against an independent protocol and a meaningful baseline.
- Find failure regimes and the worst seed/horizon/variable without rebuilding analysis scripts.
- Export reproducible evidence for model release, review, and downstream decisions.

**Simulation, verification, and validation leads**

- Determine whether a prediction is applicable to the declared geometry, regime, and horizon.
- Separate model output, solver reference, recovered quantities, and derived quantities.
- Audit source, settings, model, runtime, warnings, lineage, integrity, and signer authenticity.

**Design partners evaluating fast flow prediction**

- Import supported geometry, review preprocessing and model support, run quickly, and inspect
  engineering-relevant fields and surface behavior.
- Compare selected surrogate results with reference evidence without treating the surrogate as a
  general CFD solver.

### 2.2 B2C / individual technical users

**Independent physics-ML researchers**

- Explore checkpoints and initial conditions locally, including confidential or unpublished work.
- Compare model behavior, reference fields, spectra, pressure recovery, and self-consistency.

**Educators and advanced learners**

- Build intuition with Flow Painter and calibrated 2D/3D views.
- See the difference between a prediction, numerical reference, derivation, verification,
  validation, integrity hash, and signature.

Individual use does not mean a simplified toy mode. Defaults should make the first valid path
clear; contextual inspectors reveal expert depth when the user asks for it.

---

## 3. Premium scientific-instrument direction

The design ambition is award-level product quality, but **premium means disciplined hierarchy,
interaction quality, precision, and grounded evidence—not luxury decoration**.

### 3.1 Required visual and interaction character

- Warm-dark, low-chrome native workspace; Inter for interface text and JetBrains Mono for
  measurements, identifiers, timestamps, and tabular data.
- Tonal surfaces, 1 px separators, 2–4 px radii, restrained spacing, and a clear 40 px layout
  rhythm. Use hierarchy and negative space before adding containers.
- Ember marks the next primary action. Gold, blue, green, and red retain consistent scientific
  or status meanings. Color is never the only carrier of status.
- Shared calibrated scales, explicit units, source labels, keyboard focus, and numeric values.
- Motion communicates state or continuity, respects reduced-motion settings, and never delays
  scientific work.
- Dense information is acceptable when grouped by task, aligned, comparable, and scannable.
- Every loading, unavailable, stale, blocked, waived, failed, and read-only state is designed.

### 3.2 Anti-patterns

- **No visible “Support” button** in the persistent app chrome. Documentation, diagnostics,
  feedback, and contact routes belong under Help/About or a contextual failure state.
- No cheap generic SaaS chrome: no fake account/avatar, pricing/upgrade furniture, oversized KPI
  cards, interchangeable card grids, or dashboard filler.
- No decorative sci-fi: no cyber-blue control room, glassmorphism, blur orbs, ornamental grids,
  unexplained glow, or animated “AI” decoration.
- No incumbent ribbon clone, giant universal simulation tree, or app-to-app workflow fragmentation.
- No feature clutter. “Most features” means the needed capability exists in the right workflow,
  with safe defaults and progressive disclosure—not that all controls are simultaneously visible.
- No glow on annotations, evidence states, or text. Render bloom may communicate field intensity
  in the scientific viewport; it must not imply confidence or validity.
- No fake users, online state, projects, checksums, signatures, results, or enabled dead controls.

### 3.3 Progressive-disclosure model

1. **Journey level:** start from a project/case intent such as Geometry, Painted IC, or Model
   qualification.
2. **Stage level:** show Source → Contract → Discretization → Run → Evidence readiness.
3. **Task level:** show only controls required for the active stage and supported contract.
4. **Inspector level:** reveal methods, tolerances, metadata, derivations, and expert diagnostics
   beside the object they affect.
5. **Artifact level:** provide the complete manifest and verification details in evidence exports.

Defaults may accelerate setup, but units, transforms, support assumptions, and derivations must
remain inspectable and consequential confirmations must never be silent.

---

## 4. Current implementation truth

This section is historical grounding, not permission to infer unlisted features.

### 4.1 Verified native-track milestone history

| Milestone | Preserved status | Verified delivered scope | Explicit remainder |
|---|---|---|---|
| N1 — Python engine bridge | Done for the product unlock | Rust app spawns and communicates with the Python engine; real model fields can enter the native workflow; engine failure is handled without making the shell unusable. | Current transport is length-prefixed/framed loopback TCP with binary payloads. Shared memory and the original `<20 ms at 128³` transport gate are not complete. |
| N2 — GPU rendering | Done | Native wgpu particle and volume paths, HDR/bloom, slicing, shadows, streamline tubes, and software fallback. The recorded 1M-point benchmark is 112 fps on the tested Metal setup. | Point/volume camera-path unification and some overlays remain refinements, not release blockers. |
| N3 — 2D fields and pressure recovery | Done | TimeJump, velocity/vorticity/recovered-pressure views, shared scales, probes, field insights, pressure-recovery methods/residuals, model-vs-reference overlay, persistence baseline, and semigroup consistency. | Free-turbulence `direct_v3` needs its distinct data path; recovered-pressure error is not physical validation. |
| N4 — Flow Painter | Done | Brush, symmetries, presets, native divergence-free projection, diagnostics, and prediction handoff. | A solver reference for painted ICs remains deferred until viscosity/regime are explicit. |
| N5.1 — Benchmark suite | Done | Seed × horizon suite, persistence comparison, CSV, and deterministic canonical JSON with SHA-256 integrity. | SHA-256 is not a signature. |
| N5.2 — coherent evidence slice | Done, milestone still partial | Exact stream classification; legacy provenance findings; selected-cell velocity, vorticity, recovered-pressure, error, and spatial-divergence evidence with source/unit/method metadata and shared calibrated scales (`N5X-INSP-01` passed); energy spectra; legacy, mask-conditioned, and fixed-body-v2 benchmark contracts. Deterministic PNG/PDF reports derive from the same run/model/protocol/hash-linked canonical JSON (`N5X-EXPORT-01` passed). The signing slice now implements detached Ed25519 signatures over the raw canonical-payload SHA-256, explicit key ID/public-key fingerprint/signature bytes/verification state, portable offline CLI verification, revocation-aware trust, deterministic JSON/PNG/PDF sidecar lineage, append-only project evidence, and non-secret provider tests. | `N5X-SIGN-01` remains open until the production macOS Keychain/user-presence path is safely exercised on a supported app build; the canonical report remains explicitly `UNSIGNED` and authenticity lives in its derived sidecar. The overlap analysis is not yet integrated with an archived candidate artifact, so `N5X-VV-01` and `N5X-VV-02` also remain open. |
| External engineering case | Implementation complete; interactive release smoke pending | The default path is **Project → Case Setup → Run → Results → Evidence**. Run attempts are checkpointed at start and retain running/succeeded/failed/cancelled state and exact lineage. Cancellation and timeout terminate and replace the sidecar. FEA CSV is explicitly **source-frame surface traction/load data** with the complete transform, units, operating references, integration-area weights, source/model/run/solver lineage, and reported-versus-exported force/moment reconciliation metadata. | This remains managed tessellated import, not embedded/associative CAD or conservative target-mesh mapping. Pressure is recovered from predicted velocity; no structural stress or independent spatial error is shown without a reference. The full first-user path still requires an interactive packaged-app smoke. |
| N6 | Partial; release gate open | `N6-MODEL-01`, `N6-MODEL-02`, and `N6-PROJ-01` through `N6-PROJ-07` passed. Gated builds defer all sensitive studio/engine initialization until authentication. Malformed settings are quarantined before defaults can save. macOS packaging now requires the exact research pin and an arm64 factory-runtime prefix with manifest, lock, SBOM, and notices. | `N6-SET-01` remains open for production Keychain/user-presence exercise. Developer ID signing/notarization, clean-machine qualification, production TUF-root ceremony, and a separately qualified signed model remain external gates. Intel may run a universal2 review shell, but Intel compute is unsupported. |

Closeout verification on 2026-07-24: **89 Rust correctness tests passed** (the explicit GPU
performance benchmark remains ignored by default), **27 Python engine tests passed**,
`cargo fmt` completed cleanly, and the optimized release build passed. The Rust suite includes the
real Python-sidecar CAD round trip, geometry-linked engineering-section extraction/scaling, and
the persisted-project, immutable-rerun, migration, and navigation gates.

### 4.2 Scientific and technical caveats that must remain explicit

- **CAD:** current capability is STL import plus deterministic model-specific preprocessing. It
  is neither embedded CAD nor an associative CAD link.
- **Pressure coefficient:** an external engineering case shows physical-reference
  \(C_p=(p_{\mathrm{recovered}}-p_\infty)/(0.5\rho_\infty V_\infty^2)\) only after
  \(p_\infty,\rho_\infty,V_\infty\), units, and transform are recorded. Its pressure source
  remains recovered from model-predicted velocity, not an independent reference. Legacy sandbox
  views continue to say recovered pressure rather than `Cp`.
- **Surface loads:** pressure plus Newtonian viscous traction is integrated over the diffuse
  immersed interface. Exports are source-frame sample data with integration weights and
  reconciliation metadata, not conservative target-mesh mapping, structural stress, or
  independently validated loads.
- **Internal/HVAC:** the internal-flow contract is reference-only and execution-blocked until a
  compatible solver/model implements inlet, outlet, wall, material, and conservation semantics.
- **Transport:** framed loopback TCP is current. Named shared memory is a planned,
  benchmark-justified optimization.
- **Integrity:** a canonical SHA-256 digest detects modification. It does not establish signer
  identity or authenticity.
- **Reference language:** a named numerical solver output is a **solver reference**, not
  automatically physical truth. “Truth Overlay” is a legacy UI label and should become
  “Reference overlay” unless the source is a documented analytical exact result.
- **Evidence meanings:** semigroup self-consistency is not accuracy; validation/checkpoint-
  selection data is not independent test data; exact RNG-stream non-collision is not field-space
  non-overlap; recovered pressure is not an independently measured pressure field.

---

## 5. Canonical product object model

```text
Project
├── Source[]
│   ├── GeometrySource revisions
│   ├── ModelSource revisions
│   └── ReferenceSource revisions
├── Case[]
│   ├── active source revisions
│   ├── supported physics/model contract
│   ├── discretization record
│   ├── output/view definitions
│   └── Run[]  (immutable attempts)
│       └── EvidenceArtifact[]  (immutable or explicitly derived)
├── BenchmarkProtocol[]
└── ProjectEvent[]
```

The user-facing shorthand is **Project → Case → immutable Run → Evidence**.

- **Project:** portable container for sources, cases, protocols, events, and evidence links.
- **Case:** reusable intent and supported setup. Editing a case creates a new revision and marks
  dependent stages stale; it does not rewrite completed runs.
- **Run:** immutable execution attempt containing exact inputs, versions, device, outputs, logs,
  warnings, stop reason, and parent lineage.
- **Evidence:** content-addressed comparison, report, field, scalar, derivation, integrity, or
  signature artifact linked to the run(s) that produced it.

Minimum manifest fields:

- stable IDs; schema version; UTC creation, modification, and run timestamps;
- source filename/URI hint, byte size, SHA-256, import time, units, frame, revision, and exact
  transform;
- app, engine, model, solver, and converter names, versions, and hashes;
- exact contract, settings, seeds, device, parent revision/run, runtime, and stop reason;
- warnings, waivers, missing dependencies, derivation method/version, and output hashes;
- integrity digest and, when present, a distinct authenticity signature record.

Lifecycle states are **Draft, Ready, Running, Complete, Stale, Failed,** and
**Evidence-locked**. State is calculated from dependencies and evidence, not decorative color.

---

## 6. Primary user journeys

### J1 — Imported geometry to defensible prediction

Create/open project → **New External-Flow Analysis / Import Geometry** → review source/hash, units,
defects, transform, voxel adequacy, and model support → set the operating point → confirm the
locked contract → create an immutable run → inspect applicability, `Cp`, fluid loads,
force/moment coefficients, hotspots, wake indicators, and source-labeled 3D/2D evidence → compare
a parented variant or export mapped FEA loads, a source-frame neutral VTK field, and run-linked
evidence.

**Success:** another user can inspect every transformation and assumption, reproduce into a new
run when dependencies exist, or understand precisely why reproduction is unavailable.

### J2 — Qualify a neural flow model

Import checkpoint → structured validation → model card → choose independent benchmark protocol →
run suite → inspect worst cell/variable, field-space provenance, divergence, and spectra → lock
report → optionally sign with an organization key.

**Success:** every CLEAN/FLAGGED/UNKNOWN statement names the exact check; checkpoint-selection
data is never presented as independent test evidence.

### J3 — Developer Research Sandbox

Enable **Settings → Developer → Research Sandbox** → use procedural 3D, Flow Painter, standalone
2D fields, or Benchmark Lab → keep every sandbox output visibly separate from engineering case
results and evidence.

**Success:** “no independent reference” stays visible; adding a reference creates linked evidence
or a new run and never rewrites the original prediction.

### J4 — Compare a small design family

Create case variant or geometry revision → change only declared inputs → review inherited/stale
stages → run bounded variants → compare quantities, error/consistency, runtime, and applicability
on shared scales → open any aggregate point as its immutable run.

### J5 — Review or reproduce evidence

Open portable project/evidence bundle → verify schema, hashes, optional signature, and dependency
status → inspect read-only if dependencies are missing → rerun into a new immutable run when
available → compare against declared tolerances.

---

## 7. Information architecture

The release-defining path is **Project → Case Setup → Run → Results → Evidence**:

- **Project** — local/recent projects, recovery, portable bundles, and the first visible
  **New External-Flow Analysis / Import Geometry** action.
- **Case Setup** — Source, Preflight, Contract, Discretization, model support, waivers, and
  operating point. Unsupported values cannot run.
- **Results** — applicability first, then forces/moments and critical load/suction regions, then
  geometry-linked 3D and section evidence.
- **Evidence** — exact source/case/run/model lineage, methods, warnings, immutable field/scalar
  artifacts, comparison links, mapped FEA export, and neutral VTK export of a completed persisted
  external-flow field.
- **Model Library** and **Settings** remain contextual product destinations.

Procedural 3D, Flow Painter, standalone Fields (2D), and Benchmark Lab are not primary product
destinations. They remain permanently available through **Settings → Developer → Enable Research
Sandbox**, are hidden by default, and never present their output as an engineering case result.
“Project Alpha,” “Live Session,” disconnected “Metrics (3D),” and disconnected “Fields (2D)”
labels are absent from the default workflow.

---

## 8. CAD taxonomy and staged strategy

### 8.1 Exact taxonomy

| Term | Exact product promise |
|---|---|
| Embedded CAD | Geometry authoring/editing lives in Reyn and participates in its dependency graph. Reyn does not have this. |
| Associative CAD link | Reyn retains identity to an external CAD document/revision and maps downstream assignments across changes. Reyn does not have this. |
| Managed import/reimport | A source is copied/identified, transforms and immutable revisions are recorded, and a new import creates explicit lineage. Implemented for STL revisions; associative mapping across CAD topology changes is not. |
| Geometry preprocessing | Analysis-specific diagnostics, repair/defeaturing, scaling, placement, tessellation, or voxelization. Reyn owns a bounded STL diagnostic/transform/voxel path and does not silently repair source geometry. |
| Mesh-only/tessellated ingestion | Analysis begins from a surface/volume mesh created elsewhere. Current STL import is on this rung. |

### 8.2 Capability ladder

**Stage 0 — current: tessellated import**

- Retain STL parsing and deterministic model-specific voxelization.
- Label it “STL import and preprocessing”; never “embedded CAD” or “associative CAD.”

**Stage 1 — N5.4: source-aware import and preflight — release candidate**

- Source hash, units, frame, extents, transform preview/approval, revision, and run manifest.
- Watertight/open/non-manifold/degenerate/component/winding/self-intersection diagnostics.
- Three-axis inside/outside classification with persisted disagreement and odd-scanline evidence;
  solid fraction, boundary clearance, disconnected voxel components, morphological resolved-core
  thickness, target grid, and model-support checks.
- Explicit warnings/waivers and source-aware reimport diff; no silent geometry mutation. Measured
  topology/classification failures are hard gates and cannot be replaced by prose waivers.

**Stage 2 — post-N6: neutral B-rep translation — integrate**

- Isolated Open CASCADE or commercial CAD SDK adapter for STEP/IGES/native translation, healing,
  and tessellation.
- Record translator/version/options, repair log, tolerances, tessellation settings, and output
  hashes. Converter topology IDs cannot be the sole evidence identity.

**Stage 3 — optional: source-aware CAD connector — integrate**

- Pilot Onshape only with design-partner demand.
- Pin document/version/microversion/element/configuration IDs; refresh explicitly; preserve a
  local cached revision and mapping report.

**Stage 4 — embedded editing — do not build under this PRD**

Revisit only if repeated customer evidence makes geometry editing a top-three blocker and proves
that neutral import/prep plus connectors are insufficient.

Build and own the evidence boundary. Integrate translation and external reference solvers. Do not
build a general CAD kernel, universal mesher, or universal high-fidelity solver.

### 8.3 Internal/HVAC follow-on contract

Internal flow is a separate product case kind after the external-flow release gate. Its contract
must record inlet/outlet/wall assignments, fluid properties, mass-flow or pressure-drop targets,
comfort/contaminant outputs, and an external solver-reference strategy. The current external
fixed-body surrogate must reject this contract. Setup and evidence shells may be implemented
before a qualified internal model, but surrogate execution stays blocked until model support,
reference validation, and internal-flow acceptance criteria exist.

The versioned reference-only schema is `internal_flow.reference_only.v1`. It carries named and
stable region IDs; typed velocity/mass-flow/pressure/wall conditions; density, viscosity,
temperature, and optional scalar diffusivity; pressure-drop pairs and mass-balance tolerance;
comfort/contaminant quantity requests; and solver/configuration/mesh reference identity. The
execution gate additionally requires a distinct compatible model ID and a qualified reference
state. Empty future UI shells or the external-model ID can never clear that gate.

---

## 9. Prioritized implementation requirements

Priority meanings: **P0** blocks the named phase/release; **P1** is the next coherent capability;
**P2** is post-v1. “Depends on” lists requirement IDs; `—` means no feature dependency.

### 9.1 Product-wide requirements

| Requirement ID | Priority | Requirement | Depends on | Acceptance IDs |
|---|---:|---|---|---|
| REQ-UX-01 | P0 | Preserve the grounded premium scientific-instrument design and complete state coverage. | — | UX-AC-01 |
| REQ-UX-02 | P0 | Organize breadth through journeys, lifecycle stages, contextual inspectors, and progressive disclosure. | REQ-UX-01 | UX-AC-02 |
| REQ-SCI-01 | P0 | Keep prediction, reference, recovered, derived, verification, validation, consistency, provenance, integrity, and authenticity semantics distinct. | — | SCI-AC-01, SCI-AC-02, SCI-AC-03 |
| REQ-LOCAL-01 | P0 | Public builds remain account-free. Official gated artifacts authenticate before studio initialization; authorized creation, run, export, reopen, and review remain local-first. | — | LOCAL-AC-01 |
| REQ-PERF-01 | P0 | Keep UI rendering/input off engine work; coalesce interactive inference and degrade gracefully. | — | PERF-AC-01 |

### 9.2 N5.x requirements

| Requirement ID | Priority | Requirement | Depends on | Acceptance IDs |
|---|---:|---|---|---|
| REQ-N5-EV-01 | P0 | Give every interactive/exported result a stable temporary run identity and complete manifest before N6 persistence. | REQ-SCI-01 | N5X-EV-01 |
| REQ-N5-EV-02 | P0 | Expose source class and derivation method for every field/scalar. | REQ-SCI-01 | N5X-EV-02 |
| REQ-N5-VV-01 | P0 | Finish field-space nearest-training-IC and trajectory-overlap analysis with bounded claims. | REQ-N5-EV-01 | N5X-VV-01, N5X-VV-02 |
| REQ-N5-INSP-01 | P0 | Add variable-specific and spatial-divergence selected-cell inspection. | REQ-N5-EV-02 | N5X-INSP-01 |
| REQ-N5-EXPORT-01 | P0 | Export portable PNG/PDF evidence from the canonical report data. | REQ-N5-EV-01 | N5X-EXPORT-01 |
| REQ-N5-SIGN-01 | P0 | Add real organization-key signing without conflating it with SHA-256 integrity. | REQ-N5-EXPORT-01 | N5X-SIGN-01 |
| REQ-N5-CAD-01 | P0 | Add source-aware CAD preflight, transform approval, adequacy/support gates, and waivers. | REQ-N5-EV-01 | N5X-CAD-01, N5X-CAD-02, N5X-CAD-03, N5X-CAD-04 |
| REQ-N5-CAD-02 | P0 | Classify occupancy independently along X/Y/Z, persist normalized axis disagreement and odd scanline counts, block disagreement above 2%, and hard-gate open, non-manifold, inconsistent-winding, multi-shell, or self-intersecting sources. Pre-v2 single-axis projects must re-import. | REQ-N5-CAD-01 | N5X-CAD-05 |
| REQ-N5-PHYS-01 | P0 | Correct pressure terminology and permit `Cp` only after physical nondimensionalization is recorded. | REQ-N5-EV-02 | N5X-PHYS-01 |
| REQ-N5-LOAD-01 | P0 | Produce versioned pressure/viscous fluid traction, force/moment integration, hotspots, wake indicators, and source-frame FEA-load export with transform, exact reference quantities, lineage, area weights, and reconciliation metadata. Do not claim conservative target-mesh mapping. | REQ-N5-CAD-01, REQ-N5-PHYS-01 | N5X-LOAD-01, N5X-LOAD-02, N5X-LOAD-03 |

### 9.3 N6 requirements

| Requirement ID | Priority | Requirement | Depends on | Acceptance IDs |
|---|---:|---|---|---|
| REQ-N6-MODEL-01 | P0 | Ship a validated Model Library with compatibility, provenance, support envelope, limitations, and report links. | REQ-N5-EV-01 | N6-MODEL-01, N6-MODEL-02 |
| REQ-N6-SET-01 | P0 | Ship settings for compute/engine, storage, privacy, appearance, and signing-key state. | REQ-N6-MODEL-01 | N6-SET-01 |
| REQ-N6-PROJ-01 | P0 | Implement New/Open/Save/Save As/recent/autosave/crash recovery around a versioned project. | — | N6-PROJ-01 |
| REQ-N6-PROJ-02 | P0 | Persist Project → Case → immutable Run → Evidence with calibrated views and lineage. | REQ-N6-PROJ-01, REQ-N5-EV-01 | N6-PROJ-02, N6-PROJ-03, N6-PROJ-04 |
| REQ-N6-PROJ-03 | P0 | Support precise staleness, evidence locking, missing-dependency read-only review, and portable manifests. | REQ-N6-PROJ-02 | N6-PROJ-05, N6-PROJ-06, N6-PROJ-07 |
| REQ-N6-COMP-01 | P0 | Compare runs/variants on shared scales and deep-link every point to immutable evidence. | REQ-N6-PROJ-02 | N6-COMP-01 |
| REQ-N6-IA-01 | P0 | Make the external engineering case the default project path and preserve procedural 2D/3D/Painter/Benchmark workflows behind the persisted Developer Research Sandbox. | REQ-N6-PROJ-02, REQ-UX-02 | N6-IA-01 |
| REQ-N6-PKG-01 | P0 | Ship a checksummed, codesigned, notarized standalone app with clean-machine and offline review. | REQ-N6-PROJ-03, REQ-N6-MODEL-01, REQ-N6-SET-01 | N6-PKG-01, N6-PKG-02 |
| REQ-N6-SET-02 | P1 | Ship deep, categorized preferences (units & formatting, appearance incl. UI scale and field colormap/range, viewport & camera, workflow defaults incl. named operating-point presets, read-only shortcut reference) with per-setting and confirmed global reset, all serde-defaulted so older settings files load cleanly. Display/entry unit preferences must never alter stored SI evidence, run manifests, or versioned export schemas. | REQ-N6-SET-01 | N6-SET-01 |
| REQ-N6-UNITS-01 | P1 | Provide unit-aware operating-point entry (per-field unit selection converting to SI on entry, stored value always SI and visible) and unit-system-aware display of results and reference values (SI/Imperial with significant-digit and notation preferences). | REQ-N6-SET-02 | N6-IA-01 |
| REQ-N6-REPORT-01 | P1 | Export a single self-contained HTML engineering report per immutable run: full provenance chain (source SHA-256 → case revision → run → model SHA-256), operating point, geometry preflight, coefficients and physical loads, optional stored-field section figure, and an explicit limitations block. A report is never produced from a draft. | REQ-N6-PROJ-02 | N6-PROJ-04 |
| REQ-N6-EXPORT-02 | P1 | Export the rendered engineering section and the composited 3D viewport as PNG from the Results screen; results summary (named coefficients, loads, reference values, provenance ids) copyable as tab-separated text. | REQ-N6-PROJ-02 | N6-COMP-01 |
| REQ-N6-FIELD-EXPORT-01 | P1 | Export the selected completed external-flow run's persisted velocity, recovered pressure, physical-reference `Cp`, fluid traction, and solid occupancy as a ParaView-readable VTK StructuredGrid. Points and vector components use the approved imported-source frame in SI units; schema, source/case/run/model/contract identity, field hash, units, source classes, methods, and transform travel inside the file. Draft, preview, stale, malformed, non-finite, unapproved-transform, missing-content, and non-canonical-model inputs cannot export. Writing streams through a sibling temporary file and publishes atomically. | REQ-N5-EV-02, REQ-N5-PHYS-01, REQ-N6-PROJ-02, REQ-N6-PROJ-03 | N6-FIELD-EXPORT-01 |
| REQ-N6-NAV-01 | P0 | Provide a complete viewport navigation floor: orbit with a sensitivity preference, pan, zoom-to-cursor honouring the invert-scroll preference, zoom-to-fit on the real geometry bounds, and standard view stations named in flow terms (upstream/downstream/side/top/bottom/iso) with smooth, reduced-motion-aware interpolation. Bindings are switchable between a Reyn default and SolidWorks- and Fusion-style mappings, and every mapping is documented in Settings and reachable from the keyboard-shortcut reference and the viewport hint. | REQ-N6-SET-02 | N6-NAV-01 |
| REQ-N6-RUN-01 | P0 | Every in-flight run is cancellable, and progress is never fabricated: show elapsed time and a named indeterminate state unless the engine reports genuine step progress. Cancellation terminates the blocking sidecar, persists the immutable attempt as `Cancelled` with its stop reason, starts a fresh engine for immediate retry, and never creates result evidence. Any stale response is discarded without touching the active attempt. | REQ-N6-PROJ-02 | N6-RUN-01 |
| REQ-N6-HORIZON-01 | P1 | Let the operator step, scrub, and play through model horizon steps 1..H for a completed case in both the 3D and 2D section views. Every step is labeled as a model prediction at horizon step k — with physical lead time only when it can be derived honestly from the operating point — and steps other than the recorded horizon are display-only previews that never enter the content store, the run ledger, or evidence. A step that has not been computed says so rather than showing another step's field. | REQ-N6-RUN-01 | N6-HORIZON-01 |
| REQ-N6-AOA-01 | P0 | Support body attitude (angle of attack, yaw, roll) applied as a geometry transform before voxelization, since the model's free stream is fixed on +X. The angles are recorded in the geometry preflight, folded into the preprocessing transform, carried in the case revision and report, re-open transform approval, and invalidate results like any other case edit. Force and moment coefficients state their reference frame explicitly and are reported in wind axes, never rotated with the body. | REQ-N5-CAD-01 | N6-AOA-01 |
| REQ-N6-PROBE-01 | P1 | Clicking the 3D result reports local Cp, recovered pressure, and traction magnitude at the picked surface cell with source-class chips, the point's position in the approved source frame, and the horizon step it came from — consistent with the 2D section probe. | REQ-N5-EV-02 | N6-PROBE-01 |
| REQ-N6-HAZARD-01 | P0 | Any decorative or analytic overlay that is not driven by model output is quarantined to the Developer Research Sandbox, disabled elsewhere with a reason on hover, and labeled in place whenever it renders. The gate is pinned by a test. | REQ-SCI-01 | N6-HAZARD-01 |
| REQ-N6-DIAG-01 | P0 | A blocked preprocessing path must name its actual cause and the action that would clear it, and may never suggest a repair the artifact does not need. Rendering programs are parsed and validated by the test suite itself, so a machine with no GPU adapter cannot produce a green gate for a build that cannot paint. | REQ-SCI-01 | N6-DIAG-01 |

### 9.4 Post-N6 requirements

| Requirement ID | Priority | Requirement | Depends on | Acceptance IDs |
|---|---:|---|---|---|
| REQ-P-REF-01 | P1 | Add versioned supported case templates and external reference field/curve import. | REQ-N6-PROJ-03 | P-REF-01, P-VV-01 |
| REQ-P-CAD-01 | P1 | Add source-aware geometry revisions, named regions, and STEP through an isolated translator. | REQ-N5-CAD-01, REQ-N6-PROJ-03 | P-CAD-01, P-CAD-02 |
| REQ-P-SWEEP-01 | P1 | Add inherited case variants, bounded local sweeps, and immutable aggregate lineage. | REQ-N6-COMP-01 | P-SWEEP-01 |
| REQ-P-API-01 | P1 | Add headless CLI/API over the same schema and deterministic local execution. | REQ-N6-PROJ-03 | P-API-01 |
| REQ-P-REMOTE-01 | P2 | Add an optional remote/HPC backend using the same run manifest. | REQ-P-API-01 | P-REMOTE-01 |
| REQ-P-AI-01 | P2 | Add dataset registry, immutable split lineage, qualification/calibration, and an active-learning candidate loop. | REQ-P-REF-01 | P-AI-01, P-AI-02 |
| REQ-P-COLLAB-01 | P2 | Add signed evidence sharing before optional sync, comments, or organization policy. | REQ-N5-SIGN-01, REQ-N6-PROJ-03 | P-COLLAB-01 |
| REQ-P-CONNECT-01 | P2 | Pilot an Onshape source-aware connector only with validated partner demand. | REQ-P-CAD-01 | P-CONNECT-01 |
| REQ-P-INTERNAL-01 | P1 | Add an internal/HVAC contract with boundary assignments and target quantities, but keep surrogate execution blocked until a distinct qualified internal model and reference suite exist. | REQ-P-REF-01, REQ-N6-PROJ-03 | P-INTERNAL-01 |

---

## 10. Acceptance criteria

Acceptance IDs are durable. Do not mark a requirement complete because UI exists; its associated
criteria must pass with evidence from relevant automated and/or clean-machine tests.

### 10.1 Product-wide

- **UX-AC-01:** A design review finds no persistent Support CTA, generic SaaS/account furniture,
  decorative sci-fi, fake state, dead controls, status-by-color-only, or evidence-obscuring
  effects; all changed loading/error/stale/read-only states are implemented.
- **UX-AC-02:** A first-time technical user can complete J1 or J2 from one visible next action;
  expert metadata and controls remain reachable contextually without showing a universal control
  wall.
- **SCI-AC-01:** Every visible result names source class and method; solver references are not
  labeled physical truth without an analytical/experimental basis.
- **SCI-AC-02:** Consistency, independent error, numerical verification, validation, provenance,
  applicability, integrity, and authenticity are distinct fields and statuses.
- **SCI-AC-03:** Green/CLEAN/pass states state the proposition checked; missing evidence is UNKNOWN,
  not inferred.
- **LOCAL-AC-01:** With network disabled and no account, a supported user can open the app, create
  or open a local project, run available local compute, export evidence, and inspect stored
  results; missing compute dependencies degrade to explicit read-only status.
- **PERF-AC-01:** No engine RPC blocks the UI thread; TimeJump-style work has at most one in-flight
  request with stale-result handling; engine loss leaves navigation and stored evidence usable.

### 10.2 N5.x

- **N5X-EV-01:** Every run/export has a stable run UUID and unsaved-session UUID before N6. It
  records UTC time, schema, app/engine/model versions and hashes, exact settings/seeds, device,
  runtime, stop reason, warnings, source/derivation metadata, and artifact digests.
- **N5X-EV-02:** Every displayed field/scalar exposes MODEL, SOLVER/ANALYTICAL/EXPERIMENTAL
  REFERENCE, RECOVERED, or DERIVED semantics without opening a report.
- **N5X-VV-01:** CLEAN says exactly “no collision in checked RNG streams” until field-space and
  trajectory checks pass; unavailable data returns UNKNOWN.
- **N5X-VV-02:** Field-space/trajectory checks record algorithm, representation, threshold,
  candidate set, nearest matches, and reproducible inputs.
- **N5X-INSP-01:** A selected benchmark cell can switch among supported velocity, vorticity,
  pressure, error, and spatial-divergence views; shared scales, units, source labels, and scalar
  summaries match exported evidence.
- **N5X-EXPORT-01:** PNG and PDF exports are generated from the same canonical report data as JSON,
  identify run/protocol/model/hash, preserve units/legends/warnings, and verify against golden
  fixtures without fabricating a signature.
- **N5X-SIGN-01:** Integrity and authenticity are separate. A signature records algorithm, key ID,
  signed canonical-payload hash, signature bytes, and verification instructions; absent/revoked
  keys never produce “signed.”
- **N5X-CAD-01:** Preflight displays source/hash, declared units, extents, triangle/component and
  defect counts, proposed transform, target grid, estimated solid voxels, clearance, critical
  thickness/resolution, and support warnings.
- **N5X-CAD-02:** Unknown units and auto-fit require confirmation; exact conversion and 4×4
  transform persist in run/evidence data.
- **N5X-CAD-03:** Empty voxelization, forbidden boundary contact, disconnected artifacts, or
  under-resolved critical thickness blocks execution or records a named waiver.
- **N5X-CAD-04:** The model displays supported grid/channels/geometry/physics/horizon; unsupported
  inputs cannot receive an unqualified green state.
- **N5X-CAD-05:** The five valid STL fixtures classify with no more than 2% three-axis
  disagreement. The small-hole and missing-cap fixtures block on topology and measured
  disagreement; nested and intersecting shells block on component/intersection ambiguity; old
  single-axis cases block until re-import. None of these gates accepts a prose waiver.
- **N5X-PHYS-01:** `Cp` appears only when \(p_\infty,\rho_\infty,V_\infty\) are recorded and
  \(C_p=(p-p_\infty)/(0.5\rho_\infty V_\infty^2)\) is computed. Otherwise UI and exports say
  recovered pressure.
- **N5X-LOAD-01:** The versioned result payload records pressure and viscous fluid traction,
  area-weighted force/moment coefficients, physical forces/moments, reference quantities, units,
  normals/integration method, residual indicators, and model applicability.
- **N5X-LOAD-02:** Constant-pressure and analytical pressure-gradient fixtures verify closed-surface
  cancellation, sign/direction, translation about the recorded moment origin, and physical
  \(qA\)/\(qAL\) scaling; malformed or nonphysical contracts fail deterministically.
- **N5X-LOAD-03:** The Results view exposes load/suction hotspots, wake indicators, CAD-linked 3D
  and useful 2D sections. FEA CSV contains source/case/run/model IDs, transforms, units, and method
  on every export and is explicitly labeled fluid loads rather than structural stress.

### 10.3 N6

- **N6-MODEL-01:** Malformed/incompatible checkpoint import leaves the active model unchanged and
  returns structured validation.
- **N6-MODEL-02:** Model cards distinguish metadata-backed facts from UNKNOWN legacy fields, show
  support/limitations, and link benchmark reports by hash.
- **N6-SET-01:** Compute changes restart/revalidate the engine without blocking the UI; storage,
  privacy, appearance, and signing-key changes persist; telemetry is off by default.
- **N6-PROJ-01:** New/Open/Save/Save As/autosave/recovery/recent projects work with the engine
  unavailable.
- **N6-PROJ-02:** Reopen restores cases, source/model hashes, immutable run history, calibrated
  views/scales, warnings, and evidence links.
- **N6-PROJ-03:** Input changes stale only dependent stages. Completed/locked runs are never
  mutated.
- **N6-PROJ-04:** Rerun creates a new ID with parent lineage; deterministic inputs reproduce
  declared scalar values within documented tolerance or expose the difference.
- **N6-PROJ-05:** Missing model/engine dependencies open read-only with precise status while stored
  fields and evidence remain inspectable.
- **N6-PROJ-06:** No machine-specific absolute path is authoritative; portable content-addressed
  sources/artifacts are sufficient to review the project.
- **N6-PROJ-07:** Every schema migration is versioned and tested from each shipped schema and never
  silently drops evidence.
- **N6-COMP-01:** Run/variant comparisons use shared units/scales and every plotted/table value
  opens the exact immutable run and evidence.
- **N6-IA-01:** The first visible action starts an external-flow geometry case; Setup → Run →
  Results → Evidence is navigable in context. Research tools are hidden by default, persistently
  enabled only from Developer settings, and “Project Alpha,” “Live Session,” disconnected
  “Metrics (3D),” and disconnected “Fields (2D)” labels are absent from the default workflow.
- **N6-PKG-01:** A notarized app on a clean supported Mac verifies engine/model artifacts,
  creates/saves/reopens a project, runs one smoke case, and exports verifiable evidence without a
  terminal.
- **N6-PKG-02:** Offline launch/read-only review work without an account and with telemetry off.
- **N6-FIELD-EXPORT-01:** A completed selected run exports deterministic legacy-VTK
  `STRUCTURED_GRID` bytes whose dimensions and array lengths match the persisted field. Grid
  cell-centre coordinates and velocity/traction components round-trip through rotated,
  isotropically scaled approved source transforms into metres and source-frame Cartesian
  components. Embedded field data records schema, source revision, case revision, run ID, canonical
  model and field SHA-256, contract kind, transform, units, source classes, and methods. Tests reject
  incomplete/stale run states, missing canonical identity, malformed lengths, non-finite values,
  invalid occupancy, and unapproved or invalid transforms. The Results and Evidence affordances
  write through a flushed sibling temporary file; external ParaView/manual-open validation remains
  a release smoke rather than an automated claim.
- **N6-NAV-01:** Orbit, pan, zoom-to-cursor, zoom-to-fit, and the named view stations behave
  identically under every shipped mouse scheme; fit frames the actual geometry bounds with every
  corner on screen; view snaps interpolate and settle exactly on their station; zoom-to-cursor keeps
  the pointed-at point under the pointer; and the active scheme's bindings are readable in Settings,
  in the shortcut reference, and in the viewport hint. A station's on-screen orientation matches the
  physical claim in its label: the side and plan views put the free stream left to right, the
  upstream station stands ahead of the body looking downstream, and no station rolls the view.
- **N6-RUN-01:** Cancelling an in-flight run terminalizes its immutable attempt as `Cancelled`,
  terminates the blocking sidecar, starts a fresh engine, and makes retry available when readiness
  returns. No result evidence or partial field is written, and stale responses from the terminated
  generation cannot affect the retry. No percentage is displayed unless genuinely reported.
- **N6-HORIZON-01:** Scrubbing to a cached horizon step is instant and never re-requests it; an
  uncomputed step is named as such in both the 3D and section views instead of showing another
  step's field; every step other than the recorded horizon is chipped as a preview; and previews are
  discarded whenever the case contract changes.
- **N6-AOA-01:** A non-zero attitude changes the voxel mask and the recorded transform, round-trips
  solver points back to the approved source frame, appears in the case revision and the HTML report,
  clears transform approval, and invalidates the result. Zero attitude is bit-identical to the
  imported-orientation path.
- **N6-PROBE-01:** A 3D pick on the body reports Cp, pressure, and traction with source classes and
  the horizon step; a pick that misses the body says so rather than reporting a value.
- **N6-HAZARD-01:** The analytic streamline overlay cannot render outside the research sandbox, its
  control is disabled elsewhere with a reason, and a test pins the gate.
- **N6-DIAG-01:** A watertight body that auto-fits to less than one cell thick is reported as a grid
  resolution limit, with the measured thickness and the axis it is thin across, rather than as a mesh
  repair problem. Every shipped shader parses and validates in the test suite whether or not the test
  machine has a GPU adapter.

### 10.4 Post-N6

- **P-REF-01:** External references record solver/source version, case/config/mesh identities,
  units/frame transform, imported quantities, uncertainty where applicable, and conversion loss.
- **P-VV-01:** A discretization study uses at least three levels and reports quantity/refinement;
  visual similarity alone cannot be called grid independence.
- **P-CAD-01:** STEP import records translator/version/options, source units, repair log,
  tessellation settings, and output hashes.
- **P-CAD-02:** Reimport reports preserved/changed/added/removed/ambiguous regions and blocks
  unresolved assignments.
- **P-SWEEP-01:** Every sweep point is an immutable run and every aggregate point deep-links to it.
- **P-API-01:** GUI and CLI/API consume the same versioned schema and produce equivalent manifests
  for equivalent deterministic local runs.
- **P-REMOTE-01:** Local and remote backends consume the same manifest; remote execution cannot
  mutate a locked run and returns content-addressed artifacts plus logs.
- **P-AI-01:** Training, validation/checkpoint selection, independent test, and production feedback
  are immutable separate sets.
- **P-AI-02:** A qualified model declares intended use, support envelope, independent metrics and
  baseline, known failures, and dataset/model fingerprints.
- **P-COLLAB-01:** A recipient can verify and inspect a signed read-only evidence bundle without a
  Reyn account; later sync never changes locked local evidence.
- **P-CONNECT-01:** Onshape import pins source revision IDs, refreshes explicitly, caches locally,
  and produces an assignment mapping report.
- **P-INTERNAL-01:** Internal/HVAC cases record inlet/outlet/wall assignments, fluid properties,
  targets, and reference strategy. The external fixed-body model cannot execute them; a qualified
  internal surrogate is required before an internal run can leave blocked/unsupported state.

---

## 11. Roadmap and release gates

### 11.1 N5.x — finish evidence and correct semantics

1. **N5.3 Benchmark completion:** REQ-N5-VV-01, REQ-N5-INSP-01,
   REQ-N5-EXPORT-01, and REQ-N5-SIGN-01.
2. **N5.4 external engineering case:** REQ-N5-EV-01, REQ-N5-EV-02, REQ-N5-CAD-01,
   REQ-N5-PHYS-01, and REQ-N5-LOAD-01.
3. Keep physics controls locked to supported contracts; do not broaden into project management or
   arbitrary materials/BCs.

**N5 exit gate:** all N5X acceptance IDs pass; exported integrity and authenticity are distinct;
CAD preprocessing and recovered-pressure terminology are scientifically accurate.

### 11.2 N6 — durable v1 scientific instrument

1. **N6.1:** validated Model Library and settings.
2. **N6.2:** versioned Project → Case Setup → Run → Results → Evidence persistence, staleness,
   recovery, reopen, parented variant comparison, and Developer-only Research Sandbox migration.
3. **N6.3:** checksummed standalone packaging, codesign/notarization, clean-machine smoke path, and
   offline read-only review.

N6 is a release phase, not the former three-day placeholder/packaging estimate. Estimate it after
a persisted-schema spike.

**v1 release gate:** every P0 N6 acceptance ID passes on a clean supported Mac; no known path can
silently mutate a completed run, lose evidence on migration, claim unsupported CAD/physics, or
require an account for local review.

### 11.3 Post-N6 order

1. P1 supported case templates and external references.
2. P1 source-aware geometry revisions, named regions, and neutral CAD translation.
3. P1 qualified internal/HVAC contract and reference path; never route it through the external
   fixed-body surrogate.
4. P1 bounded sweeps and headless CLI/API over the shared schema.
5. P2 signed evidence sharing, then optional remote/HPC execution and collaboration.
6. P2 optional Onshape connector.
7. P2 dataset registry, model qualification/calibration, and active-learning candidate loop.

---

## 12. Product and evidence metrics

Primary measures:

- median time from source import to first **evidence-complete run**;
- percentage of runs with complete source/model/contract/derivation provenance;
- percentage of CAD imports with known units and accepted transform;
- percentage of unsupported or out-of-envelope attempts caught before inference;
- deterministic reopen/rerun pass rate and schema-migration evidence-retention rate;
- time to locate, inspect, and explain the worst benchmark cell;
- percentage of reports whose integrity hash verifies and, separately, whose signature verifies;
- crash-recovery success rate;
- percentage of comparisons whose values deep-link to immutable run evidence;
- distribution of CLEAN/FLAGGED/UNKNOWN outcomes with named reasons.

Guardrails:

- UI-thread stalls attributable to engine work;
- crashes, corrupted projects, silent stale-state errors, and evidence-loss incidents;
- unsupported claims found in UI/export reviews;
- task failure caused by clutter, hidden required assumptions, or misleading status.

Do not optimize for number of features, controls, solver options, generated runs, cards, or time
spent in the app.

---

## 13. Open decisions

Defaults are recorded so implementation does not repeatedly relitigate settled direction.

1. **Project persistence format:** portable `.reynproj` bundle versus directory. Decide after the
   N6 schema spike; both require a human-readable manifest and content-addressed artifacts.
2. **Signing implementation:** Ed25519 signs the raw canonical-payload SHA-256; private seeds live
   behind a non-synchronizing, this-device-only macOS Keychain provider requiring user presence;
   sidecars carry portable public keys/fingerprints and verify offline through the Reyn CLI.
   Revocation is fingerprint-based and supplied to verification independently. The production
   Keychain interaction gate remains open; never reuse an integrity label.
3. **Engine packaging:** pinned embedded environment first; compare PyInstaller only on measured
   size/startup/compatibility evidence.
4. **Bulk transport:** retain framed TCP until a 128³+ benchmark proves transfer is a material
   bottleneck; then add shared memory behind unchanged protocol semantics.
5. **Neutral CAD translator:** evaluate Open CASCADE versus a commercial SDK on format fidelity,
   healing, persistent metadata, packaging, licensing, and support.
6. **Model/reference terminology migration:** active UI uses model prediction and solver
   reference language; preserve compatibility when interpreting legacy saved evidence fields.
7. **Supported v1 hardware:** define macOS/hardware/model support before notarized release and
   encode it in first-run checks and release documentation.

---

## 14. Agent implementation contract

Any contributor changing Reyn Studio must:

1. Read this PRD before editing and name the `REQ-*` and acceptance IDs being implemented.
2. Check current/concurrent changes before editing; preserve other workers’ work.
3. Preserve the premium scientific-instrument direction, progressive disclosure, and scientific
   semantics in sections 3–4.
4. Never claim embedded/associative CAD, physical `Cp`, shared-memory transport, a signature, or
   independent validation unless its acceptance criteria pass.
5. Never add a persistent visible Support button or generic SaaS/sci-fi chrome.
6. Run tests relevant to the touched requirement and inspect changed UX states.
7. Update status in this PRD only after the associated acceptance IDs actually pass; record
   partial work as partial and do not invent completion.
8. Use [`docs/CFD_APP_LANDSCAPE.md`](docs/CFD_APP_LANDSCAPE.md) for rationale and vendor evidence
   rather than copying its research into this implementation contract.

