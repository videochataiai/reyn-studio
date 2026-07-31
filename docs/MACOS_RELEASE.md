# Reyn Studio macOS packaging

This workflow targets **REQ-N6-PKG-01** / **N6-PKG-01** and **N6-PKG-02**, but it does not
close them. It produces a checksummed local-development `.app` without Apple credentials.
The result is intentionally reported as **not standalone, not Developer ID signed, and not
notarized**.

## Exact local packaging commands

From `reyn-studio/`, build the native Rust-host architecture with:

```bash
SOURCE_DATE_EPOCH=315532800 python3 scripts/package_macos.py \
  --target host \
  --research-source-dir /path/to/pinned/reyn/reyn-research \
  --runtime-dir /path/to/arm64/ReynPython \
  --build-number 1
```

Explicit thin and universal targets are:

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin

SOURCE_DATE_EPOCH=315532800 python3 scripts/package_macos.py \
  --target aarch64-apple-darwin --runtime-dir /path/to/arm64/ReynPython --build-number 1
SOURCE_DATE_EPOCH=315532800 python3 scripts/package_macos.py \
  --target x86_64-apple-darwin --runtime-dir /path/to/arm64/ReynPython --build-number 1
SOURCE_DATE_EPOCH=315532800 python3 scripts/package_macos.py \
  --target universal2 --runtime-dir /path/to/arm64/ReynPython --build-number 1
```

The preflight detects the Rust host, macOS SDK, required commands, and installed Rust standard
libraries before Cargo starts. A missing target reports the exact `rustup target add ...` command.
An Apple-silicon Mac can cross-build x86_64 without Rosetta, but it needs Rosetta to execute that
slice locally. Add `--require-runnable-architectures` when local x86_64 follow-up tests are a
required gate; a missing Rosetta installation then fails with an install remedy instead of being
silently treated as tested.

Outputs for build number `1` can include:

- `dist/macos/Reyn Studio.app`
- `dist/macos/Reyn-Studio-0.1.1-build.1-arm64.app.zip`
- `dist/macos/Reyn-Studio-0.1.1-build.1-x86_64.app.zip`
- `dist/macos/Reyn-Studio-0.1.1-build.1-universal2.app.zip`
- `dist/macos/SHA256SUMS`

The command performs a locked optimized Cargo build in `target/package-macos`, assembles the
bundle, includes the `.icns`, Python sidecar, exact lightweight `reyn-research` Python-module
import closure, relocatable arm64 factory runtime, model-trust contract, SPDX/CycloneDX runtime
inventories, third-party notices, and local
`docs/PRD.md`, validates metadata/resources/architectures/dynamic libraries, and creates a
fixed-timestamp ZIP plus SHA-256. `Cargo.lock` pins Rust dependencies; `SOURCE_DATE_EPOCH` normalizes
archive timestamps. The release manifest uses only bundle-relative paths and records the byte count
and SHA-256 of every staged file except the manifest itself. `SHA256SUMS` is rewritten atomically in
filename order and covers every architecture archive for the same app/build number already in the
output directory.

Release Cargo invocations disable incremental compilation, pass path remaps through
`CARGO_ENCODED_RUSTFLAGS` (so workspace names containing spaces remain one argument), disable debug
information, and strip symbols at link time. The remaps cover the workspace, user home, and explicit
Cargo/Rustup homes; Cargo registry and git checkout roots are remapped first to deterministic
`crate-sources` / `git-sources` prefixes so a broader home remap cannot preserve registry structure.
A workspace-only rustc wrapper normalizes compile-time
`CARGO_MANIFEST_DIR` to `.` so live string constants cannot retain the build checkout. Packaging
fingerprints `Cargo.toml`, `Cargo.lock`, `PRD.md`, Rust source, toolchain/config files, and embedded
assets before the first architecture build and after every slice. A second package fingerprint
covers those inputs plus the exact engine/research Python closure, packaging scripts, icon, trust
contract, SBOM, and notices before and after staging. If either set changes, all outputs are
discarded instead of creating a mixed-source or mixed-resource package. The package fingerprint is
recorded as `release_input_sha256`; the Rust-only fingerprint is
`rust_source_input_sha256`.

Each thin Cargo output must contain exactly its requested Mach-O architecture. A `universal2`
package is created only after both thin outputs pass `lipo -verify_arch`; the merged executable must
then report exactly `arm64` and `x86_64`, and both slices independently pass deployment-target and
dynamic-library checks. The v2 release manifest records the declared target, actual architectures,
source thin-binary hashes/sizes, and an architecture-neutral resource-set inventory/hash. Validation
recomputes that resource inventory, preventing architecture-specific packages from silently
drifting in bundled Python modules, icon, or runtime contract.

The research source must be a Git checkout at the exact private revision in
`packaging/macos/release-pins.json`; a directory with matching-looking files but a different or
unverifiable revision fails closed. Pass `--research-source-dir PATH` or set
`REYN_RESEARCH_SOURCE_DIR`. `--runtime-dir` is mandatory and must name a relocatable arm64 prefix
whose installed versions match `python-runtime.lock.json`.

The currently accessible private revision does not yet contain
`pressure_channel_contract_3d.py`, `pressure_model_contract_3d.py`, or the four security packages
required in its `uv.lock`. `packaging/macos/RESEARCH_SOURCE_REQUEST.json` is the exact handoff
contract. Consequently, 0.1.1 package assembly intentionally remains blocked until the private
repository publishes a replacement commit and `release-pins.json` is updated; local uncommitted
research files are never substituted.

Validate an existing bundle and require a specific target with:

```bash
python3 scripts/validate_macos_bundle.py "dist/macos/Reyn Studio.app"
python3 scripts/validate_macos_bundle.py \
  "dist/macos/Reyn Studio.app" --expect-target universal2
python3 scripts/validate_macos_bundle.py \
  "dist/macos/Reyn Studio.app" --expect-target universal2 \
  --require-runnable-architectures
python3 scripts/validate_macos_bundle.py \
  "dist/macos/Reyn Studio.app" --require-standalone
```

The strict command is expected to exit `2` while the blockers below remain.

## Runtime and file-type boundaries

- Packaged startup resolves `Contents/Resources/engine/reyn_engine.py` from `current_exe`; it never
  falls back to a developer checkout when an app bundle is incomplete. `REYN_ENGINE_SCRIPT` and
  `REYN_RESOURCES_DIR` are explicit diagnostic/development overrides. Non-bundle development builds
  discover `engine/reyn_engine.py` and the sibling `reyn-research` directory from runtime ancestors,
  without embedding `CARGO_MANIFEST_DIR` or a developer home path.
- **Open Docs** passes a local path directly to the platform opener; it does not construct a file
  URL or use the network. In an app bundle it resolves only
  `Contents/Resources/docs/PRD.md`, and a missing packaged file produces an in-app error instead of
  falling back to a developer checkout. In development it searches bounded current-directory and
  executable ancestors for `PRD.md`, without embedding `CARGO_MANIFEST_DIR` or a developer path.
- The bundle includes the sidecar's complete local Python import closure under
  `Contents/Resources/research`: model definitions, datasets, flow contracts/quantities, and the 2D
  and 3D solvers they import, including `model_bundle.py` and `physics_losses.py`. Tests,
  training/evaluation programs, candidate datasets, and checkpoints are intentionally excluded.
- The package requires `Contents/Frameworks/ReynPython`, a relocatable **arm64-only** factory
  runtime. Its canonical manifest inventories every payload file and binds Python `3.14.6`,
  cryptography `49.0.0`, safetensors `0.8.0`, python-tuf `7.0.0`,
  securesystemslib `1.4.0`, NumPy `2.5.1`, and PyTorch `2.13.0` to the exact app research closure.
  Missing, non-arm64, externally prefixed, or version-drifted runtimes fail packaging.
- Model assets are not bundled. Production loading accepts only an adjacent
  `<name>.reynmodel`, `<name>.reynmodel.sig`, and `<name>.reynmodel.tuf/metadata/` set. Metadata is
  offline-only and contains versioned root, targets, delegated `models`, and snapshot roles plus
  `timestamp.json`; bundle and signature target lengths/hashes and model/release identities must
  agree. Pickle-backed `.pth`, `.pt`, and `.ckpt` files are forbidden from the app.
- `PINNED_TUF_ROOT_JSON` is intentionally unset in `model_bundle.py`. The packaged
  `security/MODEL_TRUST_CONTRACT.json` records that fact, and strict packaging rejects all model
  triplets while it remains unset. Runtime model authentication therefore fails closed. No test
  root, ephemeral key, private key, detached signature, or TUF repository is presented as a
  production trust anchor.
- `security/SBOM.spdx.json`, the runtime CycloneDX SBOM, package/runtime notices, upstream license
  files, `LICENSE`, and `NOTICE` record the redistributed dependency inventory.
- Bundle metadata declares Reyn project/template UTIs for `.reyn`, the currently implemented
  `.reynproj`, and `.reyntemplate`. It deliberately omits `CFBundleDocumentTypes`: startup does not
  handle Finder/LaunchServices open events, `.reyn` is not accepted by the current project dialogs,
  and templates are imported through Settings. Claiming a Finder association now would launch the
  app without opening the document.
- Engine startup failures reach the app as `engine unavailable` with explicit sidecar/resource,
  research-module, interpreter, NumPy/PyTorch, and pre-handshake stderr diagnostics.
- The UI shell may be arm64, x86_64, or `universal2` with a macOS 11.0 shell floor. Compute is
  qualified only for arm64 on macOS 14 or later. An Intel/universal2 shell must remain review-only
  and must not fall through to ambient Python. Clean-machine launch, rendering, engine,
  save/reopen, and evidence tests remain required on supported arm64 hardware.

Bundle validation fails if a required sidecar/research module is absent, if the release manifest
omits or mis-hashes a staged file, or if any staged file—including the Mach-O executable and opaque
resources—contains the current workspace, the current user home, `/Users/...`, `/home/...`, Windows
developer-home paths, any `file://.../PRD.md`, Cargo/Rustup stores, common CI build roots,
mounted-volume roots, or build-temporary paths. Required runtime paths such as
`/System/Library/...` and `/usr/lib/...` are not classified as developer paths. These checks
establish path portability; they do not establish a standalone inference runtime.

Validation also fails on private-key markers, key/container files, test or ephemeral key names,
unversioned/test TUF roots, symlinks, pickle-backed model files, orphan signatures/repositories,
unpaired model bundles, trust-contract/SBOM/license drift, or a bundled model while the production
root is unset. The generic model-layout validator checks detached-signature bundle hashes and TUF
target relative paths, byte counts, SHA-256 values, and custom model/signature identity. It does not
replace python-tuf's threshold-signature, expiry, rollback, delegation, or root-rotation verification
at runtime.

## Apple release boundary

The workflow never invokes Developer ID signing or notarization. A distributable build still needs:

1. A Developer ID Application certificate/private key and the intended Team ID.
2. Hardened-runtime signing of every executable/runtime component with reviewed entitlements.
3. Notary submission credentials, successful notarization, and ticket stapling.
4. Gatekeeper assessment of the exact archived artifact after download/quarantine.
5. Offline production-root ceremony, independent root metadata review, threshold key custody,
   publisher-key authorization/revocation policy, and a qualified signed model metadata set.

Do not relabel the local ZIP as a signed or notarized release. A linker-generated ad-hoc Mach-O
signature is not Developer ID signing.

## Manual release tests still required

On clean arm64 and x86_64 Macs, verify the archive against `SHA256SUMS`, inspect the release
manifest's exact architecture claim, and test first launch, Metal rendering,
engine-missing/read-only behavior, production-root and qualified signed-model authentication, one smoke
case, project save/reopen, evidence export/verification, offline reopen with telemetry off, template
import, Keychain user-presence, and all launch errors. Finder opening for project/template files
remains blocked until application startup handles document-open events and the canonical project
extension is resolved.
