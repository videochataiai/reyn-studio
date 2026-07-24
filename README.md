# Reyn Studio

Fully-native neural-CFD workbench. **Rust + egui + `wgpu`** (native Metal/Vulkan/DX12 —
*not* browser WebGPU), linked to the PyTorch models through a Python engine sidecar.

## Why this stack
- **Native `wgpu`** renders the 3D volumetric view directly on the GPU (Metal on macOS).
- **egui** is the fully-native immediate-mode UI (panels, sliders, metrics).
- **Python engine** keeps the PyTorch models where they belong; the native app talks to it
  over framed loopback TCP. Shared-memory field transport remains a planned optimization.
  The protocol can later sit in front of a native ONNX/ExecuTorch backend without changing
  the scientific UI.

## Run
```bash
cargo run            # debug
cargo run --release  # smooth 3D
```

The engine uses `REYN_RESEARCH_DIR` for the local research checkout. Python resolution is:
`REYN_PYTHON`, then `<research>/.venv/bin/python`, then `python3` on `PATH`.

## Status
- [x] N1–N4 — Python sidecar, native `wgpu` volume/particle rendering, 2D pressure recovery,
  solver-reference and self-consistency evidence, Flow Painter, and recovered-pressure CAD
  surface analysis
- [x] N5.1 — benchmark seed×horizon suite, persistence baseline, CSV, and canonical
  SHA-256 report-card integrity
- [x] N5.2 coherent slice — exact training/mixed-fork/validation/fresh-test stream
  classification; legacy provenance findings; mouse/keyboard cell selection; on-demand
  model/solver-reference velocity, vorticity, recovered-pressure, error, and
  spatial-divergence maps with explicit methods, units, source classes, and shared scales;
  plus energy spectra. Benchmark inference honors legacy, mask-conditioned, and fixed-body-v2
  physics contracts, including checkpoint-declared viscosity normalization.
- [x] N5.3 export slice — deterministic PNG/PDF report cards generated from the canonical
  JSON payload with visible run/protocol/model/checkpoint and payload hashes, selected-cell
  methodology, units, shared scales, warnings, limitations, and verification instructions.
  Visual exports include the matching JSON sidecar and remain explicitly `UNSIGNED`.
- [ ] Remaining N5 — integrate the tested field-space nearest-training-IC/trajectory-overlap
  analysis into benchmark evidence. The Ed25519 signing core, detached sidecars, portable
  verification, revocation handling, signed JSON/PNG/PDF bundle path, and native Keychain
  provider are implemented; `N5X-SIGN-01` remains open until the production macOS
  Keychain/user-presence path is exercised safely on a supported app build.
- [ ] N6 — validated model import/library plus `N6-PROJ-01`–`N6-PROJ-07` now pass.
  Schema-v2 projects reopen Benchmark Lab cases, model/source hashes, immutable parented runs,
  selected calibrated evidence, warnings, and deterministic scalar comparisons; shipped v1
  evidence migrates without loss. Self-contained SHA-256-addressed sources/artifacts, dynamic
  engine/model reconciliation, read-only evidence mode, integrity diagnostics, deduplication,
  and safe relinking are implemented. Compare/IA, signing-key integration, packaging, and
  clean-machine gates remain open.

Benchmark seeds are exact RNG seeds and default to `70000+`. The app never presents the
`train seed + 50000` validation/checkpoint-selection stream as independent testing.

## Test
```bash
cargo fmt --check
cargo test
cargo check --release
python3 -m unittest engine/test_reyn_engine.py engine/test_n5_inspector.py engine/test_n5_overlap.py
```

Current verification: 85 Rust tests passed (one explicit performance benchmark ignored) and
27 Python engine tests passed, including generated physics-conditioned checkpoint, inspector
protocol, recovered-pressure, and bounded overlap-analysis coverage.

## Offline signature verification

The canonical report remains immutable and explicitly `UNSIGNED`. Signing creates a separate
`*.sig.json` evidence artifact with Ed25519 algorithm, key ID, public key, SHA-256 public-key
fingerprint, signature bytes, signed canonical-payload hash, and source run/report lineage.
PNG and PDF presentations embed that sidecar's SHA-256 and the same payload hash.

```bash
reyn-studio verify-signature \
  --report reyn_report_card.json \
  --signature reyn_report_card.sig.json \
  --trusted-fingerprint <organization-key-sha256>
```

Pass each current revoked fingerprint with `--revoked-fingerprint <sha256>`. Without a trusted
fingerprint, Reyn can report a valid Ed25519 signature but must keep organization identity
`VALID_UNTRUSTED_KEY`. Compare fingerprints through an independent channel.

On macOS, newly created private seeds are stored as non-synchronizing, this-device-only Keychain
items requiring user presence. Settings, projects, reports, logs, and sidecars contain only the
key reference and public verification material. The provider boundary keeps deterministic tests
non-secret and prevents application code from reading private bytes.
