# Reyn Studio 0.2.0 release notes

**Branch:** `feature/step-import`  
**Scope:** Comprehensive-package phases **0.3–0.5** — CAD handoff depth, expert workflows, and closeout instruments — shipped as research preview 0.2.0.

## Phase 0.3 — CAD handoff

- Single-part **STEP** (Truck) with units, tessellation provenance, hard assembly rejection.
- Multi-shell STEP without an assembly graph: **pick-one solid** chooser before tessellation.
- **3MF** Core mesh import with schema `unit` (mm/cm/m/in/ft).
- Off-UI-thread `reyn-geometry-import` worker (generation IDs; cancel discards the generation).
- OCCT out-of-process bridge remains design-only (`docs/OCCT_BRIDGE_SPIKE.md`).

## Phase 0.4 — Expert workflows

- Model-field streamlines on Results (ABC analytic field stays sandbox-only).
- ParaView navigation scheme alongside Reyn / SolidWorks / Fusion; axis triad in the 3D well.
- Small follow-on run queue; named region authoring on Case Setup.
- Engineering lab sheets (HTML / PNG / PDF) with optional Ed25519 sidecar; viewport capture provenance footer.
- Case `view_state` (colormap, Cp range, streamlines) persists and restores on reopen.

## Phase 0.5 — Closeout instruments

- Force-vs-variant **Cd / Cs / Cl** grouped bars on Results (parent vs current).
- Results **Run detail** card: stop reason, runtime, warnings for the selected attempt.
- **Sign engineering result…** — detached Ed25519 authenticity evidence over the persisted engineering-result digest.
- Determinate CAD **stage progress** from the sidecar (`prepare → develop → predict → recover`) with an honest stage-based fraction plus elapsed time and Cancel.

## Packaging honesty

Research preview only — not production-qualified CFD. Apple-silicon ZIP is ad-hoc signed, not notarized. Windows x64 ZIP is portable and not Authenticode-signed. STEP remains single-part / pick-one shell, not universal CAD.

## Remaining (later)

- Consistency evidence attached to engineering runs.
- Broader STEP exporter corpus; OCCT bridge when Truck ceiling is proven.
- Assembly pick-one when occurrence transforms can be preserved honestly.
