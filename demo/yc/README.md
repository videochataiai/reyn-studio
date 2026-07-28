# Reyn Studio — YC demo package

This is an honest 2:40 product demo. It shows real STL parsing, deterministic
classifier-v2 geometry preflight, operating-point setup, content identity, and
fail-closed model/export gates. It does **not** show neural inference: this workspace
has no artifact-bound, qualified, signed 3D `.reynmodel`.

No `.reynproj` template or recorded solver result is included. The research tree has
a deterministic 3D reference solver, but the current Studio engineering-field,
loader, probe, and export contracts require model-specific Cp/traction semantics.
`REFERENCE_RUN_BLOCKER.md` records the exact mismatch. A 16³/32³ capsule fixture would
therefore mislabel unavailable quantities, so this package does not invent one.

## One-command preparation

```bash
"/Users/hamza/Documents/Pioneer RI/reyn-studio/demo/yc/demo.sh" prepare
```

This validates fixture hashes and topology, reports Python/runtime/model status,
locates or builds the optimized app, resets only `demo/yc/.state/`, and launches the
primary capsule in an isolated configuration. It never opens or changes normal Reyn
projects, settings, or recovery data.

Repeat from a clean demo state:

```bash
"/Users/hamza/Documents/Pioneer RI/reyn-studio/demo/yc/demo.sh" reset
"/Users/hamza/Documents/Pioneer RI/reyn-studio/demo/yc/demo.sh" prepare
```

Other commands:

```bash
"/Users/hamza/Documents/Pioneer RI/reyn-studio/demo/yc/demo.sh" validate
"/Users/hamza/Documents/Pioneer RI/reyn-studio/demo/yc/demo.sh" inspect
"/Users/hamza/Documents/Pioneer RI/reyn-studio/demo/yc/demo.sh" test
"/Users/hamza/Documents/Pioneer RI/reyn-studio/demo/yc/demo.sh" smoke
```

## Fixtures

`assets/fixture-manifest.json` is authoritative and records units, generation source,
triangle count, topology expectations, byte size, diagnostics, and SHA-256 for:

- `primary_capsule_d80_l260mm.stl` — watertight, 1,920 triangles, millimetres; a
  road-vehicle-like external-flow body.
- `fallback_cube_100mm.stl` — watertight, 12 triangles, millimetres.
- `defective_sphere_missing_cap_r50mm.stl` — intentionally open, 1,680 triangles,
  48 boundary edges, expected hard gate `mesh.open_boundary`.

These are pipeline fixtures, not accuracy evidence.

The repository’s NACA 0012 wing is watertight, but the current 64³ diagnostic path
correctly reports it as too thin to resolve. The capsule is therefore the primary live
case; the wing is not forced through with a waiver or a larger stage-only grid.

## Recording assets

- `SCRIPT.md` — exact 2:40 source-evidence narration/action storyboard.
- `SCRIPT_FALLBACK.md` — the original no-model-gate-centered storyboard, preserved
  byte-for-byte from the first package revision.
- `REFERENCE_RUN_BLOCKER.md` — source-level feasibility decision and minimum
  production path for honest solver-reference results.
- `SHOT_CHECKLIST.md` — macOS capture and review checklist.
- `captions.srt` — no-audio fallback captions.
- `scripts/compress_demo.sh` — two-pass H.264 compression with hard duration/size
  validation.

Compress and validate:

```bash
"/Users/hamza/Documents/Pioneer RI/reyn-studio/demo/yc/scripts/compress_demo.sh" \
  "/Users/hamza/Desktop/reyn-studio-yc.mov" \
  "/Users/hamza/Desktop/reyn-studio-yc.mp4"
```

The validator requires 150–170 seconds and at most 100,000,000 bytes.

## Manual prerequisites

1. Grant the recording app macOS **Screen & System Audio Recording** permission before
   the take.
2. Install ffmpeg for final compression (`brew install ffmpeg`) if it is not already
   available.

A qualified model is deliberately not a prerequisite for this flow. Supplying one
would change the claim set and requires its immutable qualification, authenticity,
runtime, and distribution evidence to be reviewed first.
