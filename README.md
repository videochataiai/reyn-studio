# Reyn Studio

Reyn Studio is a native desktop app for setting up fluid-flow cases, inspecting
model results, comparing them with reference evidence, and keeping the evidence
with the project.

The interface is Rust, egui, and native `wgpu`. A local Python sidecar runs the
PyTorch model code over a framed loopback connection. Projects, geometry,
results, settings, and recovery data stay on the user's computer.

## Current status

- The corrective release under review is **0.1.1**.
- The app is an early research preview, not production-qualified CFD software.
- macOS packaging now requires an exact arm64 factory runtime and pinned private
  research checkout. A universal2 shell may open projects on Intel, but compute
  is unsupported there. Developer ID signing, notarization, and a qualified
  production model remain external release gates.
- Windows 11 x64 portable packaging is implemented but remains labeled
  **preview pending clean-machine verification**.
- Windows CUDA, an installer, and automatic updates are not available.
- No model weights are included. No available 2D or 3D model has completed the
  full release-qualification gate.
- A run requires a compatible, verified `.reynmodel` bundle and its detached
  signature.

See [the Windows release guide](docs/WINDOWS_RELEASE.md) and
[the macOS release guide](docs/MACOS_RELEASE.md) for exact support boundaries.

## Build from source

Install the pinned Rust toolchain and provide a compatible Reyn research
checkout and Python environment:

```bash
export REYN_RESEARCH_DIR=/path/to/reyn-research
export REYN_PYTHON=/path/to/python
cargo run
```

Python resolution falls back to `<research>/.venv/bin/python` and then
`python3` on `PATH` for local development. Packaged builds use their verified,
bundled runtime and do not fall back to a developer checkout.

Public source builds do not require the invitation service. Official YC
artifacts are compiled with a login gate and do not construct the studio,
inspect projects/models, or start Python before authentication. Expired
sessions tear down the sidecar and in-memory studio before returning to login.
The username, password digest, and hashing keys are encrypted Cloudflare Worker
secrets and are not present in this repository or the binary. The service implementation is in
[`services/yc-access-worker`](services/yc-access-worker/).

## Test

```bash
cargo fmt --check
cargo test
cargo check --release
REYN_RESEARCH_SOURCE_DIR=/path/to/reyn-research \
  python3 -m unittest discover -s tests -p "test_*.py"

cd services/yc-access-worker
npm install
npm run types
npm run check
```

The repository also includes engine, project-recovery, packaging, supply-chain,
geometry, export, rendering, signature, and access-contract tests. Tests marked
`ignored` require a real platform, GPU, network credential, or explicit
performance run.

## Offline signature verification

Signing creates a detached `*.sig.json` evidence artifact. It records the
Ed25519 signature, key ID, public key fingerprint, canonical payload hash, and
source run lineage. A signature proves byte integrity and signer possession; it
does not prove that a simulation is scientifically valid.

```bash
reyn-studio verify-signature \
  --report reyn_report_card.json \
  --signature reyn_report_card.sig.json \
  --trusted-fingerprint <organization-key-sha256>
```

Pass each revoked fingerprint with `--revoked-fingerprint <sha256>`. Without a
trusted fingerprint, Reyn reports a cryptographically valid signature as
`VALID_UNTRUSTED_KEY`. Compare trusted fingerprints through an independent
channel.

On macOS, private signing seeds use non-synchronizing, device-only Keychain
items that require user presence. Windows evidence signing remains unavailable
until an equivalent native key-storage path is implemented and verified.

## License and safety

Source code in this repository is licensed under the
[Apache License 2.0](LICENSE). The `NOTICE` file applies. The Reyn name, logo,
and visual identity are not granted under the source license.

Reyn Studio produces numerical and machine-learning approximations. Do not use
it as the sole basis for engineering, safety, regulatory, or operational
decisions. Independently validate every material result.
