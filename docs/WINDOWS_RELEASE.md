# Reyn Studio Windows 11 x64 preview

## Status

Windows support is in development. The current target is a portable ZIP for
Windows 11 x64 with a bundled CPython, NumPy, and CPU-only PyTorch runtime.
CUDA is not supported or qualified.

The package must remain labeled "Windows preview pending verification" until
the Windows CI artifact passes the clean-machine matrix below. A successful
macOS build or cross-target `cargo check` does not establish Windows support.

## Package layout

```text
Reyn-Studio-<version>-windows-x64/
  Reyn Studio.exe
  ReynStudio.ico
  LICENSE
  NOTICE
  ReynPython/
    python.exe
  resources/
    engine/
    research/
    docs/
  THIRD_PARTY_NOTICES.md
  SBOM.spdx.json
  dependency-closure.json
  release-manifest.json
  resource-inventory.json
```

The executable resolves resources from `resources/` beside `Reyn Studio.exe`.
The factory runtime is `ReynPython/` beside the executable. Managed runtime
state uses `%LOCALAPPDATA%\Reyn Studio\Runtime`.
The default project directory comes from the Windows Documents Known Folder,
with `%USERPROFILE%\Documents` used only when that API is unavailable.

Release inputs are pinned in `packaging/windows/release-pins.json`.
`packaging/windows/python-runtime.lock` pins the complete Python closure with
artifact hashes. The research checkout must match the recorded commit exactly.

## Build commands

On a Windows 11 x64 builder:

```powershell
rustup target add x86_64-pc-windows-msvc
$env:REYN_ACCESS_REQUIRED = "1"
$env:REYN_ACCESS_ENDPOINT = "https://reynflow.com/api/yc-access/v1/session"
cargo test --locked --all-targets
cargo check --locked --target x86_64-pc-windows-msvc
cargo build --locked --release
python -m unittest discover -s tests -p "test_*.py"
python scripts/package_windows.py `
  --runtime-dir "C:\path\to\ReynPython" `
  --research-source-dir "C:\path\to\reyn-research" `
  --binary "target\release\reyn-studio.exe" `
  --runtime-smoke
```

The official YC artifact must be built with those two access variables. The
packager executes `Reyn Studio.exe --print-access-contract` and fails if the
actual binary does not require the exact HTTPS endpoint and legal-policy
versions. Credentials remain Worker secrets and are never build variables.

The runtime directory must be relocatable and must contain `python.exe`,
NumPy 2.5.1, the CPU build of PyTorch 2.13.0, Cryptography 49.0.0,
Safetensors 0.8.0, secure-systems-lib 1.4.0, and python-tuf 6.0.0. The package
validator rejects a runtime that reports CUDA. It also imports the app-owned
`model_bundle.py` from the staged engine directory and exercises the real model
card and import rejection paths; file presence alone is not sufficient. The YC
package must contain exactly one default 2D model,
`reyn-h64-tail-brinkman-seed0-v1.reynmodel`, with its detached signature,
threshold-signed TUF repository, release manifest, and three-seed replication
evidence.

Regenerate the Python lock only when intentionally updating the runtime:

```powershell
uv pip compile packaging/windows/python-runtime.in `
  --output-file packaging/windows/python-runtime.lock `
  --generate-hashes `
  --python-platform windows `
  --python-version 3.14 `
  --index-strategy unsafe-best-match
```

Packaging generates the SPDX SBOM, runtime CycloneDX SBOM, dependency closure,
and notices from locked Cargo metadata and installed Python distribution
metadata. Packaging fails if a dependency lacks required license or source
metadata, or if the staged Python closure differs from the hashed lock.

## Optional Authenticode signing

Signing is opt-in and fails closed. Readiness checklist before a commercial ZIP:

1. Code-signing PFX (or cloud HSM) issued to the publisher, plus the password in
   `REYN_AUTHENTICODE_PFX_PASSWORD` (never commit the PFX or password).
2. Windows SDK `signtool` on the builder PATH.
3. RFC 3161 timestamp URL reachable from the builder (packager default is used
   unless overridden).
4. Post-sign `signtool verify /pa /all` must succeed; SmartScreen reputation still
   needs clean-machine attestation on the exact artifact hash.

```powershell
$env:REYN_AUTHENTICODE_PFX_PASSWORD = "<password>"
python scripts/package_windows.py `
  --runtime-dir "C:\path\to\ReynPython" `
  --research-source-dir "C:\path\to\reyn-research" `
  --binary "target\release\reyn-studio.exe" `
  --sign-pfx "C:\secure\reyn-studio.pfx"
```

If the password or `signtool` is unavailable, packaging stops. An unsigned
package must not claim Authenticode or SmartScreen reputation.

## Current product limits

- Windows 11 x64 only
- Portable ZIP only; no installer or automatic updater
- Automatic and CPU compute choices only
- CUDA is not supported or qualified
- Evidence signing controls are unavailable because Windows key storage is not implemented
- The bundled H64 model is a replicated research preview, not production-qualified CFD
- Official YC binaries require an online login at each app launch; public source builds do not
- Reyn Studio is not production-qualified

## Acceptance matrix

Automated Windows CI must pass:

- Rust formatting, tests, and `x86_64-pc-windows-msvc` check
- Python engine and packaging tests
- Release build with the Windows icon resource
- Fail-closed verification of the compiled YC access contract
- CPU runtime plus authenticated loading of the bundled H64 model from the staged directory
- Package inventory, SBOM, notices, checksum, and deterministic ZIP validation
- Runner launch smoke for the staged executable
- Bundled engine READY on loopback
- Gated app reaches its login window without starting the engine before unlock,
  then runner-controlled termination leaves no orphaned Python process

On a non-Windows host, a code-only cross-target check can set
`REYN_SKIP_WINDOWS_RESOURCES=1` while running `cargo check`. This skips only
the Windows `.ico` resource compiler; CI and release packaging do not set it.

The GitHub-hosted `windows-2025` job is a packaging and integration check. It
cannot establish interactive normal-close behavior from its service desktop and
does not qualify Windows 11. The separate manual
`windows-11-clean-machine.yml` workflow requires a self-hosted Windows 11
runner and records the exact artifact hash after the manual matrix is attested.

Manual clean-machine testing must then pass on Windows 11 x64 with no Rust,
Python, Git, or developer tools installed:

- Verify the ZIP against `SHA256SUMS`
- Extract to a path containing spaces and non-ASCII characters
- Launch online, accept the current legal versions, and unlock with the issued YC credentials
- Confirm invalid credentials and repeated attempts fail without revealing which field was wrong
- After unlocking, disable networking and confirm local projects and the engine continue to work
- Restart while offline and confirm the invitation gate fails closed
- Confirm DX12 rendering and standard window move, resize, minimize, maximize, and close behavior
- Confirm file dialogs, drag and drop, DPI scaling, and keyboard shortcuts
- Confirm `ReynPython\python.exe` loads all required DLLs without ambient Python
- Confirm Automatic selects CPU and the UI does not offer MPS or CUDA
- Import a supported STL, save, close, and reopen a project
- Run the bundled H64 model smoke case and preserve its exact bundle hash in the record
- Export and reopen evidence with the engine stopped
- Confirm sidecar cleanup after normal exit, forced close, and engine failure
- Test antivirus scanning, SmartScreen, and quarantine behavior
- Verify Authenticode on signed release candidates
- Confirm no project, model, field, or analytics data leaves the machine

Real DX12 behavior, DLL loading, Windows Known Folder behavior, process cleanup,
dialogs, DPI, antivirus, SmartScreen, Authenticode trust, and clean-machine
launch cannot be verified from macOS. They remain release blockers until this
matrix is recorded against an exact artifact hash.

Before any broad commercial release, counsel should review the service terms,
privacy disclosures, Apache-2.0 boundary, and separately licensed assets. This
engineering checklist records implementation facts and is not legal advice.
