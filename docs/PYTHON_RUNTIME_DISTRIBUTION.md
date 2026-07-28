# Python Runtime Commercial Distribution Decision

**Decision date:** 2026-07-25  
**Status:** implementation decision; release gates identified below remain open  
**Scope:** Reyn Studio on macOS, including CPython, NumPy, PyTorch, model delivery,
integrity, update, rollback, licensing, and clean-machine acceptance

## Decision

Reyn Studio should ship a **pinned, immutable, arm64 factory runtime inside the
notarized app**, then support **independently updated, immutable managed runtime
slots** under the user's Application Support directory. A user-provided Python
must remain an explicitly unsupported Developer-mode escape hatch, not the
commercial default.

The first commercial compute target should be:

- Apple Silicon (`arm64`);
- macOS 14.0 or later;
- CPython 3.14.6 from the `python-build-standalone` 20260610 release;
- PyTorch 2.13.0;
- NumPy 2.5.1;
- the exact transitive wheel closure and hashes in this report;
- CPU and MPS execution, with CPU as the safe fallback;
- no account, telemetry, shell, Homebrew, system Python, or network required for
  factory-runtime launch and offline review.

The native shell may remain `universal2` so an Intel Mac can open and review
projects, but **Intel compute must be reported as unsupported**. PyTorch 2.13.0
publishes macOS wheels only for `arm64`; its wheel tag is
`macosx_14_0_arm64`. The final official Intel wheel found on PyPI is PyTorch
2.2.2, uploaded 2024-03-27 for CPython 3.12. PyTorch states that security fixes
are applied only to the current release and are not backported [S8]. Shipping
that old Intel stack as a commercial compute runtime would therefore create a
separate, unmaintained security and numerical-qualification product.

This is a staged architecture, not two competing runtime systems:

1. The signed factory slot closes offline and clean-machine requirements.
2. Managed slots permit security/runtime updates without replacing the app.
3. The factory slot remains an immutable last-known-good rollback target.
4. User-provided Python remains available only for research/development.

The target PyTorch 2.13.0 runtime must pass the full numerical qualification
suite before release. The current research environment declares
`torch>=2.12.1` and `numpy>=2.5.0`, and the observed development environment
uses PyTorch 2.12.1 and NumPy 2.5.0. A version increase must never be treated as
a packaging-only change.

## Implementation status: offline runtime foundation

**2026-07-25 — partial implementation; `N6-PKG-01` and `N6-PKG-02` remain
open.** The native offline selection/health foundation now provides:

- strict deterministic `com.reyn.runtime-manifest/1` parsing and RFC 8785
  canonical bytes for this integer/string-only schema;
- manifest identity recomputation with `runtime_id` omitted, lowercase SHA-256
  validation, sorted unique safe paths, exact file inventory/length/hash
  validation, internal-only symlink resolution, and hard-link rejection on
  Unix;
- exact macOS/arm64, macOS 14, CPython 3.14.6, PyTorch 2.13.0, NumPy 2.5.1,
  engine-protocol, SBOM, notices, and app engine/research-closure gates;
- discovery of the immutable factory slot at
  `Contents/Frameworks/ReynPython` and pointer-bounded managed slots under
  `~/Library/Application Support/Reyn Studio/Runtime/slots`;
- active, previous, and last-known-good managed selection with verified factory
  fallback; arbitrary unreferenced slots are never selected;
- deterministic strict state parsing plus write/sync/atomic-rename/directory-
  sync publication primitives. Interrupted temporary state is not a pointer;
- staged-candidate validation at a deterministic per-runtime staging path,
  followed by an isolated, bounded subprocess protocol that captures at most
  64 KiB from each output stream and rejects timeout, non-zero exit, malformed
  metadata, wrong versions/platform, or paths outside the verified slot;
- same-volume atomic publication of a validated immutable slot before one
  atomic state-pointer change. An interruption between those operations leaves
  the old pointer intact and only an unreferenced complete slot;
- last-known-good promotion only after both the staged self-test and a
  successful production engine request;
- per-active-runtime consecutive startup and crash counters, with automatic
  rollback after 2 startup failures or 3 crash/connection failures. Rolled-back
  runtime IDs are disabled and cannot re-enter active/previous/last-known-good
  discovery, preventing an automatic rollback loop;
- conservative slot garbage collection that never removes IDs named by
  factory, active, previous, or last-known-good state and skips malformed or
  non-directory slot entries;
- packaged-engine startup through the selected verified interpreter, including
  an isolated import/version/architecture/path probe and a 30-second bound on
  the sidecar `READY` protocol. `REYN_PYTHON` remains the explicit Developer
  override, while non-bundle resource/research/engine overrides keep the
  existing local-development path;
- stable diagnostics distinguishing `runtime.missing`,
  `runtime.dependencies`, `runtime.integrity`, `runtime.platform`,
  `runtime.state`, `runtime.smoke_timeout`, `runtime.smoke_failed`,
  `runtime.activation`, `runtime.startup_timeout`, and
  `runtime.checkpoints_missing`.

Adversarial Rust coverage includes modified canonical manifests, changed and
missing payload files, partial slots, wrong architecture and dependency
versions, stale active pointers, previous/factory fallback, unsupported Intel
hosts, deterministic manifests, bounded timeout and captured subprocess
failure, invalid candidates, activation interruption, startup/crash thresholds,
rollback-loop prevention, last-known-good promotion, and garbage-collection
safety.

This slice performs **local integrity and compatibility validation only**. It
does not make managed slots authentic against a local attacker and does not
implement a secure updater. TUF metadata and rollback/freeze defenses,
detached-manifest signature verification, Apple Team ID/code-signature/
notarization checks, download/resume/quarantine/extraction, disk-space policy,
an offline-media installer/UI and cross-process installer lock, signed
non-pickle model delivery, SBOM/notices generation, and clean-machine
qualification remain intentionally unimplemented. Candidate activation and
garbage collection are local controller APIs only; no network or removable-
media ingestion path invokes them yet.
No network, telemetry, package resolution, remote update, or model
deserialization was added.

## Release blockers

The following are blockers, not follow-up polish:

1. **No qualified checkpoint is present in the inspected source tree.** The
   package tests deliberately exclude `*.pth`. A clean-machine compute
   acceptance cannot pass until a qualified model artifact exists.
2. **The current model importer is unsafe for untrusted files.**
   `checkpoint_card()` and `_load()` explicitly invoke
   `torch.load(..., weights_only=False)`. PyTorch warns never to load untrusted
   data because its unpickler can execute arbitrary code [S9]. Production must
   either use a non-pickle tensor format or accept only first-party,
   signature-verified checkpoints and disable arbitrary `.pth` import.
3. **The current managed-model location is not commercial-ready.** Models are
   discovered in `<research_dir>/*.pth` and
   `<research_dir>/reyn_models/*.pth`, while the packaged research directory is
   immutable app content.
4. **Runtime selection is not yet release-qualified.** Packaged startup now
   validates and selects active/rollback/factory slots and now has local
   candidate, health, rollback, and collection primitives, but authenticity,
   Apple-signing policy, installer coordination, and production update delivery
   are not implemented. Explicit Developer/non-bundle paths still use the
   flexible import-only probe by design.
5. **The current packaging workflow is unsigned and unnotarized.** It explicitly
   does not close the existing P0 packaging gate.
6. **No staged production runtime lock, runtime SBOM, third-party notice bundle,
   TUF repository, authenticated installer, or update transport exists.** The
   deterministic local candidate, activation, rollback, and collection
   controller described above is foundation code, not that release system.
7. **Startup bounds still require clean-machine qualification.** Both the
   interpreter smoke and sidecar `READY` reads are bounded, but representative
   cold starts and failure cleanup must be qualified on supported hardware.
8. **MPS qualification is incomplete.** The engine enables automatic CPU
   fallback for unsupported MPS operations. This helps correctness but can hide
   substantial performance cliffs; the product must show the selected device
   and qualify representative workloads on both MPS and CPU [S10].

Until these gates close, the correct external claim is “development packaging,”
not “standalone commercial distribution.”

## Evidence and method

Graphify was queried first as a navigation aid on 2026-07-25. Some broad graph
queries reached the configured output budget, so no conclusion relies on a
truncated graph response. The final closure below was verified directly against
the Python imports, Rust startup code, packaging resource lists, and packaging
tests.

The Obsidian CLI is installed, but it reported that it could not find a running
Obsidian instance. No vault context was therefore available or used.

This report uses the following distinction:

- **Fact** means behavior verified in the inspected source, metadata read from
  the named artifact/API, or a requirement stated by an authoritative source.
- **Recommendation** means the proposed Reyn policy or architecture. Words such
  as “should,” “selected,” and “recommended” identify these decisions.
- **Estimate** is used only where final signed production bytes do not yet
  exist. Measured byte counts are labeled separately.

Internal source evidence is from the repository state inspected on 2026-07-25:

- [I1] `engine/reyn_engine.py`
- [I2] `src/engine.rs`
- [I3] `src/settings.rs`
- [I4] `scripts/macos_packaging.py`
- [I5] `tests/test_macos_packaging.py`
- [I6] `docs/MACOS_RELEASE.md`
- [I7] `PRD.md`
- [I8] `../reyn-research/pyproject.toml`
- [I9] the research modules listed below

## Current engine contract

### Native-to-Python startup

The current contract is:

1. Resolve `engine/reyn_engine.py` from app resources, an explicit
   `REYN_ENGINE_SCRIPT`, or a development root. A packaged app does not silently
   fall through to a developer checkout.
2. Resolve research content from `REYN_RESEARCH_DIR`, a resource override,
   configured settings, or `Contents/Resources/research`.
3. Require the 11 research files listed in the next section.
4. For an unoverridden app bundle, validate state-referenced managed slots in
   active/previous/last-known-good order, then validate the embedded factory
   slot. For `REYN_PYTHON` and explicit non-bundle/resource/research/engine
   development paths, retain the configured interpreter behavior
   (`<research_dir>/.venv/bin/python`, then `python3`).
5. Managed/factory validation checks the deterministic manifest identity,
   complete file inventory and hashes, exact platform/dependency/protocol/app-
   resource closure, then runs an isolated exact-version/path probe. Developer
   interpreters run the flexible import probe:

   ```text
   python -c "import numpy, torch; print(numpy.__version__); print(torch.__version__)"
   ```

   The Developer probe discards those printed versions and checks only process
   success; it is not a commercial-runtime qualification.
6. Spawn:

   ```text
   python -u engine/reyn_engine.py \
     --research-dir <resolved-research> \
     --device <auto|mps|cpu>
   ```

7. Read one stdout line synchronously. It must be
   `READY {"port":...,"device":...,"research_dir":...}` or a
   `READY {"error":...}` object.
8. Connect to the ephemeral loopback TCP port and exchange length-prefixed
   JSON/binary frames.

The Python process prepends the research directory to `sys.path`, changes its
working directory to that directory, and imports PyTorch. In `auto` mode it
selects CUDA, then MPS, then CPU; CUDA is irrelevant to the proposed macOS
artifact. It sets `PYTORCH_ENABLE_MPS_FALLBACK=1` before importing PyTorch.

Settings persist a Python path, research directory, and device. Telemetry is
forced off when settings load and the UI states that no analytics endpoint is
bundled [I3].

### Exact local Python closure

The sidecar resources are:

```text
engine/n5_inspector.py       # imported by the sidecar
engine/reyn_engine.py        # entry point
engine/n5_overlap.py         # packaged utility; not imported by the sidecar
```

The exact packaged research closure required by Rust and asserted by packaging
tests is:

```text
dataset.py
dataset_3d.py
flow_contract.py
flow_quantities.py
models_3d.py
obstacle_dataset.py
obstacle_solver.py
obstacle_solver_3d.py
spectral_solver.py
spectral_solver_3d.py
time_moe_operator.py
```

Directly executed third-party imports are NumPy and PyTorch. The research
modules use PyTorch subpackages such as `torch.nn`, `torch.nn.functional`,
`torch.utils.data`, and `torch.utils.checkpoint`; these are part of the PyTorch
wheel, not separate distributions.

Some research files contain imports of `dataset` or `physics_losses` inside
self-test functions. They are not reached by Reyn Studio's runtime operations
and are not part of the production closure. Tests already assert that the
packaged import closure equals the 11-file resource list [I5].

### Checkpoint closure

Current discovery is exactly:

```text
<research_dir>/*.pth
<research_dir>/reyn_models/*.pth
```

No checkpoint was present in the inspected source locations. Packaging excludes
all `*.pth` files. A checkpoint is loaded as a Python pickle-backed dictionary
containing at least `model_config` and `model_state_dict`; other code reads
training arguments, physics metadata, role/metric fields, and optional
experiment metadata. The engine hashes the checkpoint, but a hash calculated
after receiving an untrusted file is identity, not authenticity.

## macOS and interpreter constraints

### Architecture and operating system facts

- **Fact:** The existing native packaging code can build `arm64`, `x86_64`, or
  `universal2`, and currently validates a macOS 11.0 deployment target [I4,
  I6].
- **Fact:** The official Python 3.14 macOS installer is universal2 and currently
  supports macOS 10.15 or later [S1]. That does not imply that extension wheels
  have the same support floor.
- **Fact:** PyPI lists six macOS wheels for PyTorch 2.13.0: CPython 3.10–3.14
  regular ABI and CPython 3.14 free-threaded ABI. Every one is
  `macosx_14_0_arm64`; there is no x86_64 or universal2 wheel [S7].
- **Fact:** NumPy 2.5.1 publishes CPython 3.14 wheels for both arm64 and x86_64,
  so PyTorch—not NumPy or the Rust shell—is the binding architecture
  constraint [S13].
- **Fact:** Apple's current PyTorch/MPS guidance requires Apple Silicon and
  macOS 14 or later. MPS maps PyTorch operations to MPS Graph and tuned Metal
  kernels [S10].
- **Fact:** An Intel source build is technically possible, but it would be a
  Reyn-owned fork/build with separate ABI, performance, signing, vulnerability,
  and scientific-qualification obligations. It is not equivalent to consuming
  an official current wheel.
- **Recommendation:** Set the effective compute support floor to
  `arm64 + macOS 14.0`. A universal2 shell may retain x86_64 offline review, but
  it must not fall through to ambient Python or imply compute support.

### Interpreter assembly choices

**In-process CPython embedding**

CPython can be initialized inside a native process, but this would collapse the
current process-isolation boundary. A Python/PyTorch crash or native-extension
fault could take down the UI; interpreter ABI and initialization become linked
to the Rust executable; Python extension loading and hardened-runtime
entitlements become app-process concerns; and runtime replacement becomes more
coupled to the app. It also does not remove the need to ship the same Python
standard library, PyTorch, NumPy, native libraries, notices, and SBOM.

**Recommendation:** Preserve the sidecar process and framed loopback protocol.
Treat it as a product security/reliability boundary, then version and time-bound
its handshake.

**Standalone CPython prefix**

`python-build-standalone` produces redistributable CPython installation
prefixes and includes distribution-specific license metadata [S4]. Installing
the exact wheels into that prefix preserves the existing sidecar contract while
removing reliance on system Python.

**Recommendation:** Use the pinned stripped `install_only` prefix, prove
relocation at release time, and ship no build toolchain.

**Virtual environment**

A venv is appropriate for development and for recreating an environment on the
same machine. Python explicitly calls it inherently non-portable because
installed scripts contain absolute interpreter paths [S2].

**Recommendation:** Do not copy a developer `.venv` into the app.

**uv**

`uv` provides lock, exact synchronization, and offline/local-file operation
[S3]. It is useful for producing the wheel closure in CI, but shipping it would
add an unnecessary resolver/updater and another executable trust surface.

**Recommendation:** Use `uv` only in the build pipeline. Customers receive
already assembled, immutable bytes.

**Freezer/single-file tools**

PyInstaller-style freezing does not remove PyTorch's large native closure and
typically adds extraction, hidden-import, signing, MPS, and diagnostics
complexity. No measured Reyn artifact currently demonstrates a size or startup
advantage.

**Recommendation:** Do not select a freezer without a separately qualified,
measured prototype. The standalone-prefix sidecar has fewer changes to the
current runtime contract.

## Distribution options considered

### Fully bundled runtime

This means CPython, all wheels, and local Python modules live inside the app and
are replaced only with an app update.

Advantages:

- strongest clean-machine and offline behavior;
- simplest deterministic support matrix;
- nested code can be signed inside-out and notarized with the app;
- no first-run download or partial-install state;
- no dependence on ambient Python, package indexes, or administrator access.

Costs:

- approximately 0.60–0.65 GB of installed runtime per app version;
- every PyTorch security update requires app rebuild, signing, notarization, and
  download;
- rollback generally means replacing the whole app;
- side-by-side app copies duplicate the runtime.

Conclusion: use this as the **factory slot**, not as the only update mechanism.

### Managed first-run runtime

There are two materially different forms:

- resolving/installing packages from PyPI on the customer machine; and
- downloading one prebuilt, immutable, signed Reyn runtime artifact.

The first form is rejected. It makes availability, resolver behavior, package
index state, and build tags part of first launch. It is not an offline or
reproducible commercial product even if `uv` makes the install fast.

The second form is appropriate for updates. It keeps app and runtime lifecycles
separate, supports atomic slots, and can use the same bytes as CI qualification.
On its own, however, it fails offline first launch and introduces quarantine,
notarization, disk-full, interrupted-download, and proxy UX.

Conclusion: use prebuilt managed artifacts **after** a factory slot is present.

### User-provided runtime

Advantages:

- smallest vendor artifact;
- useful to researchers who need custom PyTorch builds;
- permits source-built Intel or experimental stacks.

Costs:

- not standalone;
- environment drift and poor supportability;
- Python ABI, wheel architecture, deployment-target, and PATH failures;
- no vendor-controlled SBOM or vulnerability response;
- users can accidentally select an environment containing incompatible or
  malicious packages;
- setup requires Python expertise and often network access.

Conclusion: retain only under Developer mode with an “unsupported custom
runtime” label and a full diagnostic probe. It must never be selected silently.

### Why not ship a copied virtual environment

Python documents that virtual-environment scripts contain absolute interpreter
paths and that environments are inherently non-portable [S2]. A copied `.venv`
is therefore not the release artifact.

`uv` is recommended as a **build-time resolver and installer** because it
supports exact synchronization, lockfiles, and offline operation from local
files/cache [S3]. It is not needed on customer machines. CI should install
hashed wheels directly into a relocatable standalone CPython prefix and test
that prefix at multiple absolute paths.

## Selected architecture

### App layout

The signed app should have this relevant layout:

```text
Reyn Studio.app/
  Contents/
    MacOS/
      reyn-studio
    Frameworks/
      ReynPython/
        bin/
          python3 -> python3.14
          python3.14
        lib/
          libpython3.14.dylib
          python3.14/
            site-packages/
              numpy/
              torch/
              ...exact locked transitive packages...
              *.dist-info/
        runtime-manifest.cjson
        runtime-manifest.sig
        runtime-sbom.cdx.json
        THIRD_PARTY_NOTICES.html
        licenses/
    Resources/
      engine/
        n5_inspector.py
        n5_overlap.py
        reyn_engine.py
      research/
        dataset.py
        dataset_3d.py
        flow_contract.py
        flow_quantities.py
        models_3d.py
        obstacle_dataset.py
        obstacle_solver.py
        obstacle_solver_3d.py
        spectral_solver.py
        spectral_solver_3d.py
        time_moe_operator.py
      update-trust/
        root.json
      runtime-compatibility.json
```

`ReynPython` is a relocatable installation prefix, not a venv. Every Mach-O
executable, dylib, and extension module is signed before the outer app is
signed. The runtime is immutable while inside the app.

### Per-user layout

```text
~/Library/Application Support/Reyn Studio/
  Runtime/
    state.json
    slots/
      <runtime-id>/
        ReynPython/
          ...same logical prefix and signed metadata as factory...
        install-receipt.json
    downloads/
      <target>.partial
  Models/
    <model-id>/
      <model-version>/
        model.json
        weights.safetensors
        model-manifest.cjson
        model-manifest.sig
        model-sbom.cdx.json
        LICENSES/
  Quarantine/
    <failed-artifact-id>/
      failure.json
```

`state.json` records `active`, `previous`, `factory_runtime_id`,
`last_known_good`, activation time, app version, and last self-test result. It
is replaced with write-to-temp, `fsync`, and atomic rename. Activation never
mutates a slot and never uses an unvalidated symlink.

### Release repository layout

```text
reyn-release/
  metadata/
    1.root.json
    root.json
    timestamp.json
    snapshot.json
    targets.json
    runtimes.json
    models.json
  targets/
    runtime/
      macos-arm64/
        reyn-runtime-cpython3.14.6-torch2.13.0-numpy2.5.1-r1.dmg
    model/
      <model-id>/
        <version>/
          <model-id>-<version>.reynmodel
  offline/
    Reyn-Studio-<app-version>-Complete.dmg
  release/
    Reyn-Studio-<app-version>.dmg
```

The standard app DMG contains the app and factory runtime. The Complete DMG
also contains at least one qualified `.reynmodel` artifact and instructions
that the app can execute without a terminal. Managed runtime DMGs are signed,
notarized, and stapled. A `.reynmodel` is a deterministic ZIP containing data
only; it is authenticated by TUF and the signed internal manifest.

### Runtime manifest

`runtime-manifest.cjson` is RFC 8785 canonical JSON and includes:

```json
{
  "schema": "com.reyn.runtime-manifest/1",
  "runtime_id": "sha256:<identity-digest>",
  "platform": "macos",
  "architecture": "arm64",
  "minimum_macos": "14.0",
  "python": "3.14.6",
  "torch": "2.13.0",
  "numpy": "2.5.1",
  "engine_protocol": 1,
  "research_closure_sha256": "<digest>",
  "source_revision": "<git-commit>",
  "build_epoch": 0,
  "files": [{"path": "...", "size": 0, "sha256": "..."}],
  "sbom_sha256": "<digest>",
  "notices_sha256": "<digest>"
}
```

The actual manifest must use real sizes and digests. Paths are relative,
UTF-8, slash-separated, unique, sorted, and may not contain `..`, absolute
prefixes, devices, hard links, or links escaping the slot.

To avoid a self-referential hash, compute `identity-digest` as SHA-256 over the
RFC 8785 canonical manifest with `runtime_id` omitted. Insert that ID, canonicalize
again, and sign the SHA-256 of the final manifest. Compute
`research_closure_sha256` over a canonical sorted array of `{path,size,sha256}`
entries for the production engine and research files. The detached signature
object is:

```json
{
  "schema": "com.reyn.detached-signature/1",
  "algorithm": "Ed25519",
  "key_id": "<release-manifest-key-id>",
  "signed": "sha256",
  "digest": "<sha256-of-final-canonical-manifest>",
  "signature": "<base64-standard-with-padding>"
}
```

The verifier must first recompute `digest`, then verify the Ed25519 signature
against a key delegated for the target class by the embedded trust root. Use
the same signature schema for model manifests with a distinct delegated model
key. This signature is intentionally in addition to TUF: TUF authenticates
transport/repository state, while the internal signature remains portable with
an exported offline artifact.

`runtime-compatibility.json` in the app defines supported runtime-manifest
schema versions, engine protocol, minimum/maximum runtime generation, research
closure digest, model format versions, and operating-system/architecture
constraints. Compatibility is explicit; package semver inference is not used.

## Exact version and artifact pins

The proposed initial lock is:

```text
CPython                  3.14.6, python-build-standalone release 20260610
filelock                 3.32.0
fsspec                   2026.6.0
Jinja2                   3.1.6
MarkupSafe               3.0.3
mpmath                   1.3.0
networkx                 3.6.1
NumPy                    2.5.1
setuptools               83.0.0
SymPy                    1.14.0
PyTorch                  2.13.0
typing-extensions        4.16.0
```

The selected source artifacts and SHA-256 values are:

```text
cpython-3.14.6+20260610-aarch64-apple-darwin-install_only_stripped.tar.gz
875516e13be36296f8f7dd0972b22ba3bed069ed08d27d5f0069caf227522921

torch-2.13.0-cp314-cp314-macosx_14_0_arm64.whl
d849b390e07d8d333ce8ecaf91b273c656c598379a19c9acf1318a883f6b391c

numpy-2.5.1-cp314-cp314-macosx_14_0_arm64.whl
efd736408cc97c79b9e6917338dfc8f06013b2274f992e96b1d9a81a71e2a2c2

markupsafe-3.0.3-cp314-cp314-macosx_11_0_arm64.whl
c47a551199eb8eb2121d4f0f15ae0f923d31350ab9280078d1e5f12b249e0026

filelock-3.32.0-py3-none-any.whl
d396bea984af47333ef05e50eae7eff88c84256de6112aea0ec48a233c064fe3

fsspec-2026.6.0-py3-none-any.whl
02e0b71817df9b2169dc30a16832045764def1191b43dcff5bb85bdee212d2a1

jinja2-3.1.6-py3-none-any.whl
85ece4451f492d0c13c5dd7c13a64681a86afae63a5f347908daf103ce6d2f67

mpmath-1.3.0-py3-none-any.whl
a0b2b9fe80bbcd81a6647ff13108738cfb482d481d826cc0e02f5b35e5c88d2c

networkx-3.6.1-py3-none-any.whl
d47fbf302e7d9cbbb9e2555a0d267983d2aa476bac30e90dfbe5669bd57f3762

setuptools-83.0.0-py3-none-any.whl
29b23c360f22f414dc7336bb39178cc7bcbf6021ed2733cde173f09dba19abb8

sympy-1.14.0-py3-none-any.whl
e091cc3e99d2141a0ba2847328f5479b05d94a6635cb96148ccb3f34671bd8f5

typing_extensions-4.16.0-py3-none-any.whl
481caa481374e813c1b176ada14e97f1f67a4539ce9cfeb3f350d78d6370c2e8
```

These are decision-time pins, not permission to skip qualification. CI must
reject any filename, length, digest, dependency, or license-expression drift.
The lock and wheelhouse are source inputs; customers never resolve from PyPI.

## Integrity and signature chain

Use two complementary trust systems:

1. **Apple Developer ID/notarization** authenticates executable macOS code and
   satisfies Gatekeeper.
2. **TUF metadata** authenticates Reyn update targets and protects update
   ordering, delegation, rollback, freeze, and mix-and-match behavior [S11].

The verification chain is:

```text
Apple trust store
  -> Developer ID signature on inner Mach-O files
  -> Developer ID hardened-runtime signature on Reyn Studio.app
  -> Apple notarization ticket stapled to app/DMG

Reyn Studio.app signed resources
  -> embedded TUF root.json (offline threshold root keys)
  -> timestamp.json (short-lived online key)
  -> snapshot.json (consistent metadata versions/hashes)
  -> targets.json / delegated runtimes.json or models.json
  -> target byte length + SHA-256
  -> canonical internal manifest + detached release signature
  -> per-file SHA-256 before execution/import
  -> runtime/model self-test
```

Recommended TUF key policy:

- all role keys use Ed25519 and metadata uses SHA-256 target hashes;
- root: 2-of-3 offline keys, one in separate custody; one-year expiry;
- targets: 2-of-3 offline/release keys; 90-day expiry;
- delegated runtime and model roles: 2-of-3 release keys; 30-day expiry;
- snapshot: isolated online key; seven-day expiry;
- timestamp: isolated online key; 24-hour expiry;
- root rotation follows the sequential-version procedure in the TUF
  specification.

An expired update timestamp must stop the update check, not disable an already
installed and verified runtime or offline project review. An explicit user
rollback may select a previously trusted local slot even though online metadata
would reject it as an automatic update.

Downloaded bytes go to a `.partial` file. The app verifies TUF length/hash
before opening the container, verifies path safety during extraction, verifies
the internal manifest and every file, verifies the Developer ID Team ID and
code signatures of executable content, runs the self-test, then atomically
renames the slot. A failure is quarantined and never added to `state.json`.

Apple requires hardened runtime for notarization and supports stapling tickets
so Gatekeeper can validate without a network connection [S5]. Nested code must
be signed inside-out before the enclosing app [S6]. Do not use
`--deep` as a signing strategy; use it only as one verification check.

Start with no broad hardened-runtime exceptions. If measured PyTorch behavior
requires JIT entitlement, add only `com.apple.security.cs.allow-jit` to the
Python executable after security review. Do not disable library validation or
allow unsigned executable memory without a demonstrated requirement.

## Update and rollback flow

### Update

1. Update checks are manual by default. A user may opt into periodic checks.
   There is no analytics event, device identifier, project/model name, or
   telemetry payload. The UI discloses that the download host necessarily sees
   ordinary request metadata such as IP address.
2. Refresh TUF metadata and enforce signatures, versions, expiry, delegation,
   and consistent-snapshot rules.
3. Filter targets by app compatibility, `arm64`, minimum macOS, engine protocol,
   research closure, and model format.
4. Show version, download size, installed size, security/qualification notes,
   and whether restart is required.
5. Download resumably into `downloads/*.partial`.
6. Verify and install into a new immutable slot.
7. Run:
   - isolated interpreter/import probe;
   - exact version/path/architecture probe;
   - sidecar READY handshake with a 30-second deadline;
   - deterministic CPU smoke;
   - MPS smoke when available;
   - signed qualified-model compatibility probe.
8. Set `previous=active`, activate the new slot atomically, and restart only
   the engine.
9. Mark the slot last-known-good after the first successful production request.
10. Keep the previous managed slot and factory slot. Garbage-collect older
    slots only after explicit policy and disk-space confirmation.

Version 1 should ship full runtime targets, not binary deltas. A full immutable
artifact is easier to reproduce, sign, inspect, and roll back. Add deltas only
after measuring that the saved bandwidth justifies their additional
verification surface.

### Automatic rollback

Rollback to `previous`, then factory, when:

- manifest/signature verification fails;
- the process cannot start;
- READY times out or has an incompatible protocol;
- imports/versions/paths do not match;
- the deterministic smoke test fails;
- the engine crashes before first successful request.

Do not automatically roll back solely because one customer model is
incompatible. Keep the runtime, mark that model incompatible, and offer the
previous qualified pairing. Persist a local diagnostic containing versions,
hashes, result codes, and sanitized stderr; never include fields, geometry,
project paths, or model bytes unless the user explicitly exports diagnostics.

## Model and checkpoint delivery

The production target format is:

```text
<model-id>-<version>.reynmodel
  model.json
  weights.safetensors
  model-manifest.cjson
  model-manifest.sig
  model-sbom.cdx.json
  MODEL_CARD.md
  LICENSES/
  benchmark/
    canonical-summary.json
    canonical-summary.sig
```

`model.json` carries the model configuration, physics/flow contract,
dimensionality, supported device/dtype, required runtime IDs or compatibility
range, training-code revision, dataset/evidence identifiers, input/output
schema, limitations, and benchmark artifact hashes. Tensor values live in
Safetensors, a non-pickle format designed to store tensors without arbitrary
code execution [S12].

Until the engine supports that format, the only tolerable transitional policy
is:

- accept first-party `.pth` only after TUF and internal-manifest verification;
- verify bytes before the first `torch.load`;
- remove general `.pth` import from production UI;
- keep arbitrary checkpoint import in Developer mode with a high-severity
  warning;
- never describe `weights_only=False` validation as safe.

`weights_only=True` narrows remote-code-execution exposure but PyTorch documents
remaining denial-of-service and possible memory-corruption limits [S9]. It is
defense in depth, not authenticity. Signature, size limits, tensor shape/count
limits, metadata schema validation, and process resource limits remain
necessary.

Model updates are independent of runtime updates. A model becomes selectable
only when its signed compatibility declaration matches the active runtime and
research closure. Project evidence retains the exact model SHA-256 and runtime
ID, so updating a model never rewrites an existing run.

## Failure UX

All states must preserve project open, evidence inspection, export, and
read-only review where the project format permits it.

Use these user-facing states:

- **Compute ready — Factory runtime:** show exact Python/PyTorch/NumPy versions,
  CPU/MPS device, runtime ID, and “works offline.”
- **Compute ready — Managed runtime:** show active and rollback versions.
- **No qualified model:** “Reyn is ready, but no compatible model is installed.”
  Offer “Install from Complete media,” “Choose model pack,” and an explicit
  “Download” action.
- **Intel Mac:** “Project review is available. Reyn compute requires Apple
  Silicon; no supported Intel PyTorch runtime is published.” Do not offer a
  doomed download.
- **Runtime damaged:** keep review available. Offer “Use factory runtime,”
  “Repair from signed media,” and “Export diagnostics.”
- **Integrity/signature failure:** never execute. State which layer failed
  (update metadata, target hash, manifest, file hash, Developer ID, notarization,
  or self-test), quarantine the artifact, and provide a stable error code.
- **MPS unavailable:** use CPU in Auto mode and state why. If MPS was explicitly
  required, do not silently change the user's choice.
- **MPS operator fallback:** show “MPS with CPU fallback” in diagnostics and
  performance results; do not imply fully GPU-resident execution.
- **Insufficient disk:** show download size, installed size, required temporary
  headroom, available bytes, and safe cleanup candidates before downloading.
- **Update metadata expired/offline:** continue using the installed runtime and
  say that update freshness could not be verified.
- **New runtime fails:** state that Reyn restored the previous runtime, include
  both IDs, and keep the failed artifact disabled.

No remediation should instruct a commercial user to open Terminal, install
Homebrew, create a venv, or run pip.

## Size evidence

All numbers in this section are either measured inputs or explicitly labeled
estimates. MB uses 1,000,000 bytes.

Measured on 2026-07-25:

- standalone CPython archive: 25,998,180 bytes compressed;
- standalone CPython archive contents: 66,145,732 file bytes;
- PyTorch CPython 3.14 arm64 wheel: 111,227,066 bytes compressed and
  471,068,058 uncompressed file bytes;
- NumPy CPython 3.14 macOS 14 arm64 wheel: 5,335,944 bytes compressed and
  20,379,802 uncompressed file bytes;
- complete selected wheel closure: approximately 126,969,335 compressed bytes
  and 532,589,078 uncompressed file bytes;
- CPython plus selected wheel inputs: approximately 152,967,515 compressed
  bytes and 598,734,810 unpacked file bytes.

Estimates:

- signed factory runtime download contribution: **155–175 MB**;
- installed factory runtime: **0.60–0.65 GB**;
- peak first managed-slot update headroom: **0.9–1.3 GB**, because the active
  slot, download, extracted candidate, and rollback slot can coexist;
- default current 2D model architecture: 6,327,298 parameters, or 25,309,192
  raw FP32 bytes; an inference-only model pack is therefore estimated at
  **25–35 MB** before benchmark attachments;
- Complete DMG overhead: native app size plus approximately **180–220 MB** for
  runtime and one 2D model. This remains an estimate until the qualified model
  and signed app are built.

The measured source tree has no qualified checkpoint, so no production model
size is known. Optimizer/training state must not be included in an inference
artifact.

For context only, the last Intel PyTorch wheel found was
`torch-2.2.2-cp312-none-macosx_10_9_x86_64.whl`, 150,797,270 bytes. Its size
does not make it a supportable runtime.

## Licensing and SBOM

Commercial redistribution is technically feasible, subject to retaining and
shipping the applicable notices:

- CPython is under the PSF License Version 2 plus incorporated-component
  licenses [S1].
- `python-build-standalone` build tooling is MPL-2.0, but its maintainer states
  that no MPL-licensed software is in the built Python distributions. The
  specific archive's included license metadata must still be reviewed [S4].
- PyTorch 2.13.0 metadata declares the composite expression
  `Apache-2.0 AND Apache-2.0 WITH LLVM-exception AND BSD-2-Clause AND
  BSD-3-Clause AND BSL-1.0 AND MIT` and publishes a large set of third-party
  license files [S7].
- NumPy 2.5.1 declares
  `BSD-3-Clause AND 0BSD AND MIT AND Zlib AND CC0-1.0` [S13].
- Every transitive wheel's actual installed `.dist-info` license files and
  package metadata are authoritative release inputs.

For every app, runtime, and model artifact:

1. Generate CycloneDX 1.6 JSON, an Ecma standard, with package URLs, exact
   versions, file/source hashes, licenses, supplier, build toolchain, source
   revision, and dependency edges [S14].
2. Add file components for CPython, native dylibs, PyTorch/NumPy bundled
   libraries, and local Reyn Python modules; a Python package-only scan is
   insufficient.
3. Generate `THIRD_PARTY_NOTICES.html` from the exact staged bytes, not from
   dependency names alone.
4. Preserve original license files under `licenses/`.
5. Hash SBOM and notices in the signed manifest and expose them in
   **About → Open-source licenses** without network access.
6. Produce a model BOM/model card including model format, weights hash,
   training-code revision, declared datasets/evidence, runtime requirements,
   and license.
7. Fail the build on an unknown license, forbidden policy result, missing
   notice, package drift, or SBOM component without provenance.

This report is an engineering decision, not legal advice; counsel should approve
the generated notices and model/data rights before release.

## Build and release pipeline

The release pipeline should be:

1. **Lock review**
   - update exact pins in a reviewed change;
   - resolve for CPython 3.14/macOS 14/arm64;
   - record one selected artifact and SHA-256 per package;
   - review upstream release notes, PyTorch security policy, licenses, and
     numerical-qualification impact.
2. **Fetch**
   - download only pinned artifacts;
   - verify filename, byte length, and SHA-256;
   - archive the source wheelhouse and CPython input in immutable release
     storage.
3. **Assemble without network**
   - use a clean arm64 macOS builder;
   - unpack standalone CPython;
   - install from the local wheelhouse with exact/offline synchronization;
   - do not create or copy a venv;
   - do not ship `uv`, compiler tools, wheel cache, tests, or training-only
     packages;
   - retain `.dist-info` and license files.
4. **Normalize**
   - remove caches and developer paths;
   - use deterministic timestamps/order/permissions;
   - reject absolute shebangs outside the runtime prefix;
   - prove relocation by testing at two unrelated paths including spaces and
     non-ASCII characters.
5. **Inventory**
   - generate the sorted per-file manifest, SBOM, notices, source provenance,
     and vulnerability scan;
   - run a second scanner and reconcile discrepancies for native bundled
     libraries.
6. **Qualify**
   - run Python/import/engine protocol tests;
   - run CPU and MPS scientific golden tests;
   - run the signed model compatibility and deterministic smoke suite;
   - record machine model, OS build, device, tolerance, and results.
7. **Sign nested code**
   - sign inner `.so`, `.dylib`, executables, and helper code with Developer ID
     and hardened runtime;
   - verify architectures, Team ID, entitlements, dependencies, and deployment
     targets;
   - sign the outer app last.
8. **Package/notarize**
   - build deterministic runtime and app containers;
   - submit with `notarytool`, require accepted status, retrieve the log, and
     staple;
   - validate staple and Gatekeeper assessment after copying through a
     quarantine-bearing download path;
   - notarize the final distribution container, not an earlier intermediate.
9. **Publish**
   - generate and threshold-sign TUF target/snapshot/root metadata;
   - publish targets before metadata;
   - publish timestamp last;
   - retain previous metadata and targets according to rollback policy.
10. **Independent clean-machine acceptance**
    - test exact public bytes from the public endpoint and Complete media;
    - release only after the acceptance matrix below passes.

Build and signing workers should be separate. Developer ID and TUF root/targets
keys must not be present on a general build worker. The release record retains
source revision, lock, input hashes, output hashes, SBOM, notices, notarization
submission/log, signing certificate identity, qualification results, and
approvals.

## Acceptance tests

### Artifact and supply-chain tests

- Rebuild twice from the same inputs; file manifest, SBOM content, notices, and
  unsigned payload hashes match. Code-signature/notary timestamps are recorded
  as expected nondeterministic envelopes.
- A clean, network-disabled build from the archived wheelhouse succeeds.
- Altering one byte in CPython, any wheel, runtime target, model target,
  manifest, SBOM, or TUF metadata fails before execution.
- Expired, rolled-back, mixed-version, wrong-delegation, and insufficient-
  threshold TUF metadata are rejected.
- Archive traversal, duplicate path, case-collision, absolute path, escaping
  symlink, hard-link, decompression-bomb, oversized file, and disk-full cases
  fail safely.
- Every Mach-O reports only allowed architectures/deployment targets and has
  the expected Team ID, hardened runtime, and reviewed entitlements.
- `codesign --verify --strict --deep`, `spctl`, notary-log review, and stapler
  validation pass on the exact public artifacts.
- SBOM components and dependency edges reconcile with all staged Python
  distributions and discovered Mach-O dependencies.

### Clean Apple Silicon, offline

On a factory-reset supported Mac with no developer tools, Homebrew, Python,
PyTorch, or NumPy:

- install the exact quarantined Complete DMG with networking disabled;
- launch without Terminal, account, Rosetta, or administrator package install;
- verify the factory runtime reports exactly Python 3.14.6, PyTorch 2.13.0, and
  NumPy 2.5.1 from inside the app;
- verify `sys.executable`, `torch.__file__`, `numpy.__file__`, and `sys.path`
  stay inside signed app/runtime roots and do not use user/site/system packages;
- import the included qualified model pack through the UI;
- run the canonical CPU smoke and compare within approved tolerances;
- run the canonical MPS smoke and compare within separately approved
  tolerances;
- create, save, close, and reopen a project;
- reopen immutable evidence with the engine deliberately unavailable;
- export and verify evidence offline;
- observe no attempted analytics endpoint and no project/model/field egress;
- reboot and repeat launch/reopen with the installer media removed.

Run the matrix on the oldest supported macOS 14 release, the current macOS
release, at least one base-memory Apple Silicon machine, and at least one newer
GPU family.

### Managed update and rollback

- Download through normal quarantine, an interrupted connection, a proxy, and
  a resumed transfer.
- Install with spaces/non-ASCII in the home path.
- Kill the app at every update phase; the next launch uses a complete previous
  or factory slot, never a partial slot.
- Simulate insufficient disk before download and before activation.
- Corrupt each verification layer and confirm the artifact is never executed.
- Install a runtime whose READY handshake hangs; confirm the deadline,
  termination, diagnostic, and automatic rollback.
- Install a runtime with wrong protocol/research digest/model compatibility and
  confirm pre-activation rejection.
- Confirm update activation does not alter existing project evidence.
- Confirm an offline user can keep using installed runtime/model artifacts when
  update metadata expires.
- Confirm explicit rollback and later forward activation preserve both runtime
  IDs in diagnostics.

### Intel behavior

On a clean supported x86_64 Mac, if the universal review shell is shipped:

- the signed/notarized app launches without attempting the arm64 runtime;
- project/evidence open, verify, and export work offline;
- compute controls clearly state Apple Silicon is required;
- no Python download or fallback to system `python3` occurs;
- no claim suggests that Rosetta provides a supported compute path.

### Scientific and security acceptance

- CPU and MPS run the full golden corpus with separately versioned tolerances;
  performance claims include device and fallback state.
- The MPS suite covers every operation family exercised by qualified models;
  unexpected CPU fallback is a qualification finding.
- Runtime/model version changes create new qualification records and never
  inherit old results by version-range assumption.
- A malicious pickle supplied as a renamed model is not deserialized in the
  production path.
- Oversized tensor shapes/counts, malformed Safetensors headers, NaN/Inf policy
  violations, and incompatible model metadata fail before allocation or run.
- Model and runtime signatures are checked before any Python import or model
  load involving target-controlled content.
- Sidecar execution is bounded by startup timeout and appropriate process
  resource limits; failure preserves read-only review.

## Support and lifecycle policy

- Support one active runtime generation and one previous generation for
  rollback; the factory runtime remains available for recovery.
- Because PyTorch does not backport security fixes, assess every current
  PyTorch release promptly and publish a qualified managed slot or an explicit
  risk advisory [S8].
- Never change package bytes under an existing runtime ID.
- Never silently downgrade for compatibility.
- Keep app, runtime, model, engine protocol, research closure, and evidence
  schema versions distinct.
- Security expiry may prevent new compute/model acquisition while preserving
  offline evidence review; do not hold a customer's saved work hostage to an
  update server.
- Publish end-of-support dates and affected runtime IDs without telemetry.

## Authoritative external sources

All URLs were accessed 2026-07-25 unless a publication date is stated.

- **[S1] Python 3.14.6 macOS and release artifacts.** Python documentation says
  current macOS installers are universal2, signed, and notarized; the 3.14.6
  release page supplies hashes and Sigstore metadata.
  - https://docs.python.org/3/using/mac.html
  - https://www.python.org/downloads/release/python-3146/
- **[S2] Python virtual environments.** Python 3.14 documents absolute
  shebangs and says environments are inherently non-portable.
  - https://docs.python.org/3.14/library/venv.html
- **[S3] uv locking, exact sync, and offline operation.**
  - https://docs.astral.sh/uv/concepts/projects/sync/
  - https://docs.astral.sh/uv/reference/cli/
- **[S4] python-build-standalone redistribution and licensing.** The project
  describes highly redistributable builds and distribution-specific license
  metadata; maintainer clarification dated 2025-02-20 states that built Python
  distributions contain no MPL-licensed software.
  - https://github.com/astral-sh/python-build-standalone
  - https://gregoryszorc.com/docs/python-build-standalone/stable/running.html
  - https://github.com/astral-sh/python-build-standalone/issues/534
  - https://github.com/astral-sh/python-build-standalone/releases/tag/20260610
- **[S5] Apple notarization and stapling.** Apple documents Developer ID,
  hardened runtime, notarization, and stapling for offline Gatekeeper lookup;
  its custom workflow notes that ZIPs cannot be stapled directly.
  - https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
  - https://developer.apple.com/documentation/security/customizing-the-notarization-workflow
- **[S6] Apple nested-code signing.** TN2206 requires nested code to be signed
  before its outer bundle.
  - https://developer.apple.com/library/archive/technotes/tn2206/_index.html
- **[S7] PyTorch 2.13.0 package metadata, wheel artifacts, and licenses.**
  - https://pypi.org/project/torch/2.13.0/
  - https://pypi.org/pypi/torch/2.13.0/json
  - https://github.com/pytorch/pytorch/releases/tag/v2.13.0
- **[S8] PyTorch security and release support.** The security policy says fixes
  apply only to the current release and are never backported.
  - https://github.com/pytorch/pytorch/security/policy
  - https://github.com/pytorch/pytorch/blob/main/RELEASE.md
- **[S9] PyTorch serialization security.** `torch.load` uses an unpickler and
  warns never to load untrusted data; `weights_only=True` narrows but does not
  remove the security surface.
  - https://docs.pytorch.org/docs/stable/generated/torch.load.html
  - https://docs.pytorch.org/docs/stable/notes/serialization.html
- **[S10] Apple Silicon MPS and fallback.** Apple lists Apple Silicon and
  macOS 14 for its current PyTorch/MPS instructions; PyTorch documents
  `PYTORCH_ENABLE_MPS_FALLBACK`.
  - https://developer.apple.com/metal/pytorch/
  - https://docs.pytorch.org/docs/stable/mps_environment_variables.html
- **[S11] The Update Framework.** The specification defines signed
  root/targets/snapshot/timestamp metadata and defenses against rollback,
  freeze, and mix-and-match attacks.
  - https://theupdateframework.github.io/specification/latest/
- **[S12] Safetensors format.** The project specifies a tensor-only,
  non-pickle format with bounded/validated metadata and offsets.
  - https://github.com/huggingface/safetensors
- **[S13] NumPy 2.5.1 package metadata, wheels, and license expression.**
  - https://pypi.org/project/numpy/2.5.1/
  - https://pypi.org/pypi/numpy/2.5.1/json
- **[S14] CycloneDX 1.6 / ECMA-424.** First edition published June 2024.
  - https://ecma-international.org/publications-and-standards/standards/ecma-424/

## Final commercial recommendation

Proceed with the factory-plus-managed-slot architecture for an Apple
Silicon/macOS 14 commercial compute release. Keep a universal native shell only
for clearly labeled Intel read-only review. Build the runtime from the exact
artifacts above, sign/notarize the factory and managed code, use TUF for update
trust and rollback protection, and deliver a first-party model as a separate
signed non-pickle pack.

Do not ship the current user-provided-Python contract as “standalone,” do not
resolve packages on customer machines, do not claim Intel compute using
PyTorch 2.2.2, and do not expose current arbitrary `.pth` import in a production
build. The release is commercially supportable only after the blockers and
clean-machine acceptance matrix in this report are closed.
