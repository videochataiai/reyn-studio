# Reyn Studio 0.1.2 release notes (candidate)

**Branch:** `feature/step-import`  
**Scope:** Continue the comprehensive package — STEP + 3MF import, viewport triad, evidence lab sheets, Cp profile, model streamlines, ParaView nav scheme, async CAD import, multi-shell pick-one, run queue, named regions.

## Geometry

- Single-part **STEP** (Truck) with units, tessellation provenance, hard assembly rejection.
- Multi-shell STEP without an assembly graph: **pick-one solid** chooser before tessellation.
- **3MF** Core mesh import with schema `unit` (mm/cm/m/in/ft).
- File dialog, drop target, and Case Setup provenance chips cover STL / STEP / 3MF.
- Initial translate/diagnose/voxelize runs on the off-UI-thread `reyn-geometry-import` worker (generation IDs; cancel discards the generation; stale results discarded).

## Viewport & evidence

- Axis triad (stream / side / up) in the 3D well.
- ParaView-style navigation scheme alongside Reyn / SolidWorks / Fusion.
- Engineering report export: HTML, PNG lab sheet, PDF lab sheet (optional Ed25519 sidecar when a Keychain signing key is configured).
- Viewport PNG captures compose a provenance footer.
- Section mid-line quantity profile plot under the section image.
- Case `view_state` (colormap, Cp range, streamlines) persists in the exact contract and restores on reopen.
- Model-field streamlines (labeled MODEL) when Results streamlines are enabled; ABC demo remains sandbox-only.
- Named region authoring on Case Setup; small follow-on run queue drains after the in-flight attempt.

## Packaging honesty

The macOS packaging test `test_research_resource_list_is_the_sidecar_import_closure` can fail on **both** the 0.1.1 baseline and this 0.1.2 candidate when the engine import graph references `pressure_model_contract_3d` / `pressure_channel_contract_3d` that are absent from `RESEARCH_RESOURCES` in `scripts/macos_packaging.py`. That is **not** a STEP regression. Fixing it requires aligning the pinned research checkout with the resource allowlist in a dedicated packaging change — do not silently expand the allowlist without the files present.

## Remaining (later)

- Broader STEP exporter corpus qualification.
- Out-of-process OCCT bridge (never static-link into the desktop binary) — see `docs/OCCT_BRIDGE_SPIKE.md`.
- Assembly pick-one when occurrence transforms can be preserved honestly.
