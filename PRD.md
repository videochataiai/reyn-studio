# Reyn Studio — Native App PRD (continuation)

*Fully-native (Rust · egui · wgpu/Metal) neural-CFD workbench, linked to the PyTorch
models through a Python engine. This document is the forward plan: where we are, the
target architecture, and every step to get there. Companion to
`reyn-research/product/01_PRD_DESKTOP_APP.md` (the retired PySide6 plan).*

---

## 0. Status — what already works (as of commit `97e4e76`)

- **Native shell** (`egui 0.35` + `eframe`, `wgpu` renderer = native Metal): top menu bar
  with 2D/3D toggle + Live Session, left project rail (Import Model, nav, Voxel
  Diagnostics), right 3D-controls panel (slicing planes, isosurface density, opacity,
  shadows, streamlines, export). Themed with the precision-instrument ember palette.
- **Typography + icons**: bundled **Inter** (UI) + **JetBrains Mono** (data); hand-drawn
  vector icon set (`icons.rs`) — no icon-font dependency.
- **Interactive 3D viewport** (`viewport.rs` + `flow.rs`): 6000-particle ABC/Beltrami
  vortex field, **mouse-orbit + scroll-zoom + `G` regenerate**, depth-sorted halo+core
  glow. Every control drives it (opacity→alpha, density→|ω| threshold, slice-X→clip,
  streamlines→integrated lines). **Live** Voxel Diagnostics computed from the field.
- **Nav switching** works; non-3D views are honest placeholders.

**The seam that matters:** `flow::generate()` produces the field the viewport renders.
Replacing that procedural field with the Python engine's real predicted field is the
next milestone (N1) and turns this from a beautiful demo into the actual product.

---

## 1. Product definition

**What it is.** A local-first desktop workbench for 2D/3D incompressible flow with
pluggable neural flow-map surrogates, that shows its error bars. Native Rust UI + native
GPU rendering; the trained PyTorch models run in a Python engine the app talks to.

**Why native (recap of the decision).** The models are PyTorch → Python stays the AI
engine. The *app* is Rust + `wgpu` (native Metal, **not** browser WebGPU) for maximum
UI/render performance and a small fast binary. Inference sits behind one `Engine` trait,
so the Python sidecar can later be swapped for a fully-native ONNX/ExecuTorch backend
with zero UI change.

**Users.** (1) physics-ML researchers, (2) simulation/V&V team leads (Reyn Verify design
partners), (3) educators. Same three as the strategy doc.

---

## 2. Target architecture

```
┌──────────────────────────── reyn-studio (Rust) ────────────────────────────┐
│  ui/            egui panels: top bar, rail, controls, per-view content      │
│  render/        wgpu render passes (volume/points/isosurface) in egui       │
│                 paint-callbacks; HDR + bloom post                           │
│  engine/        Engine trait  +  PythonSidecar backend (spawn, IPC, shmem)  │
│  domain/        Field, Camera, ModelCard, BenchmarkResult, flow gen         │
└───────────────────────────────────┬─────────────────────────────────────────┘
                         control socket (JSON, length-prefixed)
                         + shared memory ring (zero-copy field arrays)
┌───────────────────────────────────┴──────── reyn-engine (Python) ───────────┐
│  loads reyn-research code: contract.py, spectral_solver(_3d), models,        │
│  obstacle solvers, flow_quantities. Serves: load_model, predict, roll_solver │
│  paint→leray, pressure_poisson, run_benchmark. Emits fields into shmem.      │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Module plan (Rust, `src/`):** current `app.rs`/`theme.rs`/`fonts.rs`/`icons.rs`/
`flow.rs`/`viewport.rs` grow into `ui/` (one file per view), `render/` (wgpu), `engine/`
(IPC), `domain/`. Refactor happens in N2.

---

## 3. Milestones (native track: N1–N6)

Each milestone is shippable and demoable on its own. Ordered by leverage.

| # | Milestone | Unlocks | Est. | Status |
|---|---|---|---|---|
| **N1** | **Python engine bridge** | real model data in the viewport | ~3–4 d | ✅ done |
| **N2** | **GPU render upgrade (wgpu + bloom)** | the mockup's glow & true volumetrics | ~4–5 d | ✅ done — bloom · volume raymarch · streamline tubes · 112 fps @ 1M pts |
| **N3** | **2D field views + Pressure Recovery** | TimeJump, V/ω/P, Truth Overlay, Trust Meter | ~4 d | ✅ done — TimeJump · V/ω/P · Truth Overlay · Trust Meter · Recovery Settings (spectral/FD + residual) · model selector |
| **N4** | **Flow Painter** | paint IC → Leray → generate | ~3 d | ▢ |
| **N5** | **Benchmark Lab (Reyn Verify seed)** | suite analysis, report card, CSV | ~4 d | ▢ |
| **N6** | **Models · Settings · Import · packaging** | notarized `.app`, first-run | ~3 d | ▢ |

---

## 4. N1 — Python engine bridge  *(the unlock)*

**Goal:** the 3D viewport renders a **real field from a real model**, not the procedural
ABC field. Load `flow3d_obs_v1.pth` (or `obstacle_v2_shapes` for 2D), predict, stream the
velocity field into the viewport.

### 4.1 IPC contract (spec)
- **Transport.** Rust spawns `python -m reyn_engine` as a child; a **control channel** on a
  localhost TCP port (or Unix socket) carries **length-prefixed JSON** requests/responses;
  **bulk field arrays** go through a **named shared-memory segment** (`multiprocessing.shared_memory`
  on Python, `memmap`/`shared_memory` crate on Rust) — zero-copy.
- **Handshake.** On spawn: engine prints `READY {port, shmem_name, shmem_bytes}` on stdout;
  Rust connects. Heartbeat every 2 s; if 3 missed → mark engine "disconnected", banner in UI.
- **Requests** (JSON): `list_models`, `load_model{path}`, `predict{model_id, dt, ic_id}`,
  `roll_solver{steps, scenario, nu, ...}`, `regen_ic{seed, scenario}`.
- **Responses.** Small metadata in JSON (`{shape:[C,D,H,W], dtype, offset, valid_report}`);
  the array itself already written to the shmem segment at `offset`. Rust reads the slice,
  uploads to the GPU.
- **Errors.** Every response is `{ok:true,...}` or `{ok:false, error}`; Rust surfaces
  `error` as a toast; never panics on engine failure (the app must stay usable with the
  engine down — "solver/AI unavailable" state).

### 4.2 Steps
1. `reyn-engine/` package in the app repo (or a subdir of reyn-research). `pyproject` pins
   torch + the reyn-research modules (path dep). Entry `__main__.py`.
2. Implement the server: stdin/stdout handshake, JSON control loop, one shmem ring buffer
   (double-buffered so a new frame can write while Rust reads the last).
3. `load_model` → `contract.EddyCheckpointAdapter`; return the validation report (green/amber).
4. `predict`/`roll_solver` → write the `[C,D,H,W]` (3D) or `[C,H,W]` (2D) field into shmem.
5. Rust `engine/mod.rs`: `trait Engine { fn load_model; fn predict; fn latest_field; }` +
   `PythonSidecar` impl (spawn, connect, shmem map, request/response, heartbeat).
6. Rust `domain/field.rs`: `Field { channels, dims, data: Vec<f32> }`; convert engine field →
   particles/voxels for the viewport (sample the field, compute |ω| for color).
7. Wire: `Import Model` button → file dialog (`rfd` crate) → `load_model`; Model Card dialog
   shows the validation report (reuse the green/amber provenance logic).
8. Replace `flow::generate` call in `app.rs` with `engine.latest_field()` when a model is
   loaded; keep procedural fallback when the engine is down.

### 4.3 Acceptance criteria
- **N1-AC1** Launch app → engine spawns → `list_models` populates the Models view within 2 s.
- **N1-AC2** Load `flow3d_obs_v1.pth` → viewport shows the **model's** vortex field; the Model
  Card shows the validation status (amber for the current one). Voxel Diagnostics reflect it.
- **N1-AC3** Kill the Python process manually → app shows a "engine disconnected" banner and
  stays interactive (procedural field), reconnects when relaunched. No crash.
- **N1-AC4** A 128³ field crosses the boundary in < 20 ms (shared-memory, measured).

---

## 5. N2 — GPU render upgrade (wgpu + bloom)

**Goal:** replace the egui-painter particle projection with a **native `wgpu` render pass**
inside the egui viewport (paint callback), giving the mockup's **glow** and true volumetrics.

### 5.1 Design decisions
- **egui + wgpu paint callback** (the confirmed pattern): render the 3D scene to an offscreen
  **HDR** texture in `CallbackTrait::prepare`, composite into the panel in `paint`.
- **Two render modes** (matches the mockup's controls):
  - **Instanced points** (default): the field's high-|ω| samples as GPU point-sprites,
    additive-blended, colored by the ember↔blue map. Handles millions.
  - **Volume raymarch** (isosurface density): a 3D texture of |ω| (or Q-criterion),
    raymarched with the Density range as the isovalue window; slicing planes = clip in the
    shader; volumetric shadows = a second light-march.
- **Bloom**: HDR target → bright-pass threshold → separable Gaussian blur → additive
  composite. This is the single biggest "AAA look" lever (the ember cores glow).
- Camera/orbit already exists → port to a view-projection matrix uniform.
- *Alternative considered:* full **Bevy** engine via `bevy_egui`. Decision: stay with raw
  `wgpu` in egui first (lighter, we control the pipeline); revisit Bevy only if we need its
  ECS/asset pipeline. Documented so we don't relitigate.

### 5.2 Steps
1. `render/gpu.rs`: wgpu device from eframe's `RenderState`; HDR `Rgba16Float` offscreen target.
2. Point pipeline (WGSL): instanced quads, additive blend, size attenuation, ember colormap
   in-shader. Feed positions from the engine field (GPU buffer, updated when the field changes).
3. Camera uniform (view-proj from `viewport::Camera`); orbit/zoom already wired.
4. Volume pipeline (WGSL): upload |ω| as a `3d` texture; fragment raymarch with isovalue
   window (Density), slice planes, optional shadow march.
5. Bloom post: threshold + two-pass blur + composite passes.
6. egui `Callback` that runs prepare/paint; the panel becomes the composited texture.
7. Toggle: points ↔ volume driven by the existing controls; keep the painter path as a
   `--software` fallback for machines without the needed features.

### 5.3 Acceptance criteria
- **N2-AC1** ✅ 3D viewport renders via wgpu at ≥ 60 fps for 1M points — **measured 112 fps** (8.94 ms/frame, 111.8M pts/s) at 1280×800 including upload + particle pass + bloom + composite (`bench_million_points`, run `cargo test -- --ignored --nocapture`).
- **N2-AC2** ✅ Ember cores visibly **bloom** — the HDR bright-pass + Gaussian bloom is live and asserted by the headless GPU test.
- **N2-AC3** ✅ Density window, all three slice planes, and volumetric-shadows toggle change the render live — in point mode via CPU projection, and in the volume raymarch **in-shader** (isovalue window, clip planes, light-march shadows). Streamlines now render as **GPU ribbon tubes** (additive HDR, they bloom).
- **N2-AC4** ✅ Graceful fallback to the software painter path when wgpu is unavailable (`gpu_ready` flag; CPU halo+core path retained in `viewport.rs`).

### 5.4 Status — 2026-07-14 (N2 complete, tested on Metal)
Implemented in **`src/gpu.rs`** (+ `viewport.rs`/`app.rs`/`flow.rs`/`main.rs` wiring), all via egui + `wgpu` paint callbacks on native Metal/Vulkan/DX12 (not browser WebGPU), registered once from eframe's `RenderState`:
- **Bloom core:** additive HDR `Rgba16Float` **particle pass** (instanced point-sprites, soft-gaussian dots, ember↔blue colormap + per-core HDR gain in-shader) → **bright-pass** threshold → **2× separable Gaussian** at half-res → **tonemapped additive composite**. The scene→bloom→composite stages are shared by every scene source.
- **Volume raymarch** (the "3d_volumetric_analysis" view): the field's |ω| is uploaded as an `R8Unorm` 3D texture; a fullscreen pass casts an orbit-camera ray per pixel, ray-box clips to `[-1,1]³`, emission-absorption composites the **density window** (`density_lo/hi`), clips at the **slice planes**, and light-marches for **volumetric shadows** — output feeds the same bloom so isosurfaces glow. Toggle: "Volume Raymarch" in Rendering Options (3D only). Placeholder ABC volume until a model field arrives; real |ω| from the engine field otherwise.
- **Streamline tubes:** a second instanced pipeline expands each projected streamline segment into a camera-facing HDR **ribbon** (round cross-section in-shader), additive → blooms into glowing tubes. Replaces the egui-line streamlines in GPU mode; the CPU line path stays as fallback.
- **Tests (all green on Metal, skip without an adapter):** `bloom_renders_and_glows` (core + bloom spread + a streamline ribbon), `volume_raymarch_glows` (a dense blob raymarched shows a bright, high-contrast isosurface), and the ignored `bench_million_points` (112 fps). `engine_round_trip` still green; clean release build, 0 warnings.
- **Deferred (minor):** camera projection for point mode is still CPU-side (NDC instances) rather than a GPU view-proj uniform — the raymarch already uses a GPU orbit camera; unifying them is cosmetic. Streamlines render in point mode only (not overlaid on the volume) for now.

---

## 6. N3 — 2D field views + Pressure Recovery

**Goal:** the 2D side of the workbench (the `pressure_recovery_view` mockup) — a field view
with a **Vorticity / Velocity / Pressure** toggle, a **TimeJump** scrubber, and the
verification trio (Truth Overlay, Trust Meter), plus **pressure recovery** via Poisson solve.

### 6.1 Features + AC
- **F-TimeJump** ✅ — horizontal scrubber; drag → engine `predict2d{steps}` (coalesced, single
  in-flight, stale re-fires) → 2D field re-renders. Latency HUD (~0.36s/scrub on MPS) and a
  beyond-trained-horizon warning above 16 steps.
- **F-FieldToggle** ✅ — Velocity / Vorticity / Pressure. Velocity & vorticity derived client-side
  from the field; **Pressure** recovered in the engine (spectral, `flow_quantities`) and shipped
  in the `[3,N,N]` (u,v,p) payload. Switching is instant (re-colormap only, no round-trip); the
  panel shows Peak/Low recovered pressure. *(Pressure L2-recovery-error metric not surfaced yet.)*
- **F-RecoverySettings** ✅ — solver method (**Spectral** = exact FFT inversion, or **FD** =
  iterative conjugate-gradient), tolerance (`1e-2…1e-8`), boundary (periodic/dirichlet), and
  **Recompute**. Each request reports the **L2 recovery error** (Poisson residual) + CG
  iterations — spectral lands ~3.6e-5 (float32), FD tracks the tolerance (1e-3→9.5e-4). The
  honest, live "Reyn Verify" demonstration of exact-vs-approximate recovery.
- **F-TruthOverlay** ✅ — compare AI vs solver at the horizon: AI | Truth | |error| split +
  RelL2, persistence floor, and the beats-persistence ratio. Honest metrics straight off the
  engine (`want_truth`).
- **F-TrustMeter** ✅ — live semigroup self-consistency (predict h vs h/2∘h/2), no ground truth
  needed; badge colored green/amber, updates when the scrub settles.

### 6.2 Status — 2026-07-14 (N3 done, tested)
- **Engine** (`engine/reyn_engine.py`): `predict2d` returns AI velocity + **recovered pressure**
  `[3,N,N]`, a **semigroup** self-consistency number, the **pressure recovery residual** (+ CG
  iters + method), and — with `want_truth` — the solver **truth** `[3,N,N]` + RelL2/persistence.
  Pressure recovery has two methods: **spectral** (exact FFT, residual = float32 eps) and **FD**
  (matrix-free conjugate-gradient on the 5-point Laplacian, periodic or Dirichlet, stops at a
  relative-residual tolerance). A per-`(model,seed)` **trajectory cache** + **MPS** inference make
  TimeJump scrubs ~0.35s (was 0.9s/forward on CPU); CPU fallback where MPS is absent.
- **Protocol** (`src/engine.rs`): `Cmd::Predict2D { …, method, tolerance, boundary }` /
  `Msg::Field2D`; the worker parses the payload + all verification/recovery metrics.
- **View** (`src/field2d.rs` + `app.rs`): a "Fields (2D)" nav entry — colormapped central image
  (diverging for signed ω/p, ember heat for |v|; Truth Overlay = AI│Truth│|error| panels) and a
  2D control panel: **model selector** (obstacle-family checkpoints), variable toggle, TimeJump
  slider + latency HUD + beyond-horizon warning, Trust Meter, Truth Overlay + RelL2/persistence,
  and the **Pressure Recovery** card (method/tolerance/boundary/Recompute + live L2 recovery
  error, CG iterations, peak/low). Textures rebuilt only on change.
- **Tests:** `predict2d_round_trip` (real engine → AI+truth planes, RelL2 < persistence, semigroup
  present, spectral residual < 1e-3), `field2d::colormap_produces_varied_image` /
  `error_of_identical_fields_is_uniform`. Full suite green (6 + 1 ignored), 0 warnings.
- **Deferred:** the **free-turbulence 2D model** (`direct_v3`) needs its own data path (64² grid,
  no-mask velocity, free-turbulence generator) — a separate `_traj2d` branch; the obstacle-family
  models are wired now.

---

## 7. N4 — Flow Painter

**Goal:** the `flow_painter` mockup — paint an initial vorticity field, project it
divergence-free, generate a flow.

### 7.1 Features + AC
- **Brush** (radius, strength), left/right drag = +/− vorticity; live 2D field texture. *AC:*
  paint is smooth at 60 fps; field updates under the cursor.
- **Presets** — Vortex Pair, Shear Layer, Kármán Street stamp analytic fields. *AC:* each
  stamps the documented structure.
- **Symmetries** — horizontal/vertical/radial (fold count, center) mirror strokes live. *AC:*
  toggling radial fold=4 produces 4-fold symmetric painting.
- **Apply Leray Projection** — engine `paint→leray` makes it divergence-free. *AC:* Divergence
  Check reads < 1e-12 after projection; live Total Energy / Mean Enstrophy diagnostics.
- **Generate Flow** — commit the IC → set as the viewport's field / hand to a model. *AC:* the
  painted IC becomes the active field in the 2D/3D view.

---

## 8. N5 — Benchmark Lab  *(the Reyn Verify seed — Strategy v2 R2)*

**Goal:** the `benchmark_lab` mockup — a **Model Suite Analysis** that runs a model across
seeds × horizons, does leak/provenance analysis, and emits a **signed report card**. This is
the architectural seed of the enterprise product; keep it a headless-capable core.

### 8.1 Features + AC
- **Run Full Suite** — engine `run_benchmark{model, seeds, horizons}` → the RelL2
  seed×horizon table (color-coded Excellent/Nominal/Warning/High-error). *AC:* table matches a
  known-good run of `obstacle_eval`/`eval_3d`; status/runtime/global-RelL2 header.
- **Leak & Provenance** — min seed-distance (L2), trajectory overlap %, spectral consistency,
  protocol → CLEAN/flagged badge. *AC:* reproduces the seed-leak detector's verdict.
- **Cell Inspector** — Split / Error-Map / Divergence of a chosen (seed, t, variable);
  model-vs-truth panes + **energy spectrum** overlay. *AC:* spectra overlay correctly; error
  map matches the compare service.
- **Export CSV** + **signed Report Card** (JSON/PDF, hash-signed evidence artifact). *AC:* the
  report card is machine-readable and carries a signature (the R2 wedge).

---

## 9. N6 — Models · Settings · Import · packaging

- **Models view** — the library (cards with provenance flags), set active, delete. *AC:* mirrors
  the engine's `list_models`; green/amber cards.
- **Settings** — the `desktop_settings` mockup: compute device (MPS/CPU), engine path, theme,
  telemetry off by default. *AC:* device change reloads the engine.
- **Import Model** — `rfd` file dialog → validate → add to library. *AC:* rejects non-checkpoints
  with a clear message.
- **Packaging** — `cargo bundle`/`tauri`-free `.app`; bundle the Python engine (PyInstaller or a
  pinned venv) inside the `.app`; **codesign + notarize** (needs the $99 Apple Developer account,
  final step only). First-run downloads bundled model weights. *AC:* a notarized `.app` launches
  on a clean Mac, engine included, no terminal.

---

## 10. Cross-cutting specs

### 10.1 Performance budgets
- App cold start < 1.5 s to first frame; engine ready < 2 s.
- 3D viewport ≥ 60 fps at 1M points (N2); field transfer < 20 ms at 128³.
- TimeJump scrub: ≤ 1 in-flight inference, stale-drop; UI never blocks on the engine.

### 10.2 Testing
- **Rust**: unit tests for engine framing (JSON + shmem round-trip with a mock engine),
  colormap, camera projection; a smoke test that boots the app headless (offscreen) and asserts
  no panic. `cargo test` in CI.
- **Python engine**: pytest for each RPC (load/predict/leray/pressure/benchmark) against the
  real research checkpoints; assert field shapes + validation reports.
- **Integration**: a scripted session (load model → predict → benchmark) asserting the field and
  the report card, run in CI on macOS.

### 10.3 Threading (must-hold rule)
The egui thread renders and handles input only. The engine client runs on a worker; requests are
coalesced (single in-flight for scrub/predict); results delivered via a channel. No blocking
call on the UI thread — ever.

### 10.4 Design contract
Tokens stay identical to `reyn-site`/DESIGN.md (ember warm-dark, Inter + JetBrains Mono, 2–4px
radii, 1px borders, 40px grid). Any token change updates both.

---

## 11. Every-step sequence (the build order, concretely)

1. **N1.1** scaffold `reyn-engine` python package + `__main__` handshake.
2. **N1.2** JSON control loop + shared-memory ring; `list_models`/`load_model`.
3. **N1.3** Rust `engine::PythonSidecar` (spawn, connect, heartbeat, shmem map).
4. **N1.4** `predict`/`roll_solver` → field into shmem → Rust `Field` → viewport.
5. **N1.5** Import Model dialog + Model Card (validation report). *Ship N1.*
6. **N2.1** wgpu offscreen HDR target + camera uniform in an egui callback.
7. **N2.2** instanced-point pipeline (WGSL) fed by the engine field.
8. **N2.3** bloom post (threshold + blur + composite).
9. **N2.4** volume raymarch pipeline + slicing + shadows; wire to Density/slice controls.
10. **N2.5** software-painter fallback path. *Ship N2.*
11. **N3.1** 2D field renderer + Vorticity/Velocity/Pressure toggle.
12. **N3.2** TimeJump scrubber (coalesced predict) + latency HUD.
13. **N3.3** pressure Poisson RPC + Recovery Settings + metrics.
14. **N3.4** Truth Overlay + Trust Meter. *Ship N3.*
15. **N4.1** paint canvas + brush + live field texture.
16. **N4.2** presets + symmetries.
17. **N4.3** Leray projection RPC + diagnostics + Generate. *Ship N4.*
18. **N5.1** `run_benchmark` RPC + seed×horizon table.
19. **N5.2** leak/provenance panel + cell inspector + spectra.
20. **N5.3** CSV + signed report card. *Ship N5.*
21. **N6.1** Models library + Settings + Import polish.
22. **N6.2** `.app` bundle + embed engine + codesign/notarize + first-run. *Ship v1.*

---

## 12. Open decisions (defaults chosen; revisit only with a reason)
- **Bevy vs raw wgpu** for the viewport → **raw wgpu** first (control, weight). Revisit at N2 end.
- **Engine transport** → **shared memory + localhost socket** (not stdin pipes) for the field
  throughput. Unix socket on mac/linux, TCP loopback on windows.
- **Engine packaging** → pinned venv inside the `.app` first; PyInstaller if size matters.
- **Fully-native inference** (drop Python) → deferred behind the `Engine` trait; exercise
  ONNX/`ort` or ExecuTorch only when shipping a standalone max-perf binary.
