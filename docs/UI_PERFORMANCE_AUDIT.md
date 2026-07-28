# Reyn Studio UI performance audit

Date: 2026-07-25  
Scope: independent, read-only source audit of `reyn-studio` UI responsiveness paths  
Runtime measurements: none were taken in this audit

## Interpretation and evidence standard

- **Fact** means the behavior follows directly from the cited source.
- **Hypothesis** means the source exposes a measurable risk, but a runtime profile is required to establish user-visible cost.
- Severity ranks risk to interaction latency, frame pacing, or sustained resource use; it does **not** claim a regression.
- Line references describe the audited working tree. Graphify's `app.rs` offsets were older than the current source in several places, so direct source inspection is authoritative.

## Executive summary

The largest risks are not shader complexity or unconditional texture rebuilding. They are CPU work and scheduling around the immediate-mode UI:

1. Busy and unavailable engine states can force continuous repaint, while even an idle app repaints every 150 ms.
2. Every repaint reconstructs project dependency state, including several allocation-heavy content-reference traversals.
3. Several derived views redo whole-field work during paint: an `O(N³)` CAD-mask bounds scan, particle projection/allocation/upload, and `O(N²)` 2D insight extraction.
4. A CAD result is drained on the UI thread and synchronously cloned, encoded, hashed twice, persisted, transformed, and scanned before the frame can continue.
5. Autosave, STL import/voxelization, model-checkpoint bundling, PNG encoding, and file writes run synchronously from UI callbacks.

The existing section, field, painter, benchmark, and GPU-version caches do prevent many texture rebuilds. The remaining texture risk is change-time churn and redundant geometry uploads, especially during horizon playback and live window resize.

## Ranked findings

### High 1 — Repaint scheduling can keep the entire UI and GPU pipeline continuously active

**Facts**

- `ReynApp::ui` drains messages and renders the whole interface, then requests another frame unconditionally:
  - while `engine_busy`, it calls `request_repaint()` (`src/app.rs:1026-1037`);
  - otherwise it calls `request_repaint_after(150 ms)` (`src/app.rs:1038-1040`).
- `engine_busy` includes `!self.engine_ok`, pending field/benchmark/library work, sandbox live mode, and a pending CAD run (`src/app.rs:1029-1035`).
- An engine I/O or startup error can set `engine_ok = false` (`src/app.rs:934-945`). If the worker has exited, no later message is guaranteed to restore it, yet `!engine_ok` continues to satisfy `engine_busy`.
- The worker uses a standard MPSC channel (`src/engine.rs:315-322`) and the UI polls it with an unbounded `try_recv` loop (`src/app.rs:664-665`). There is no source-level repaint wake attached to `Msg` delivery.
- Horizon playback and elapsed-time labels already schedule their own lower-frequency refreshes at 120 ms and 250 ms (`src/app.rs:1395-1411`, `src/app.rs:5928-5931`).

**Hypotheses to measure**

- A failed engine startup may produce sustained near-vsync repaint while showing a static unavailable state.
- Long model or solver work may repaint the static workbench at full rate and compete with MPS/wgpu work on the same GPU.
- Idle 150 ms polling may be visible in energy use even when no UI state changes.

**Action**

1. Give the engine sender a wake hook (`egui::Context::request_repaint` or an injected repaint callback) and wake exactly when a message is enqueued.
2. Track `Starting`, `Busy`, `Ready`, and `Unavailable` separately. `Unavailable` must be event-driven, not a permanent busy state.
3. Remove the unconditional idle 150 ms repaint. Let input and animations schedule frames.
4. For visible elapsed counters or indeterminate progress, use `request_repaint_after(100–250 ms)` rather than unrestricted `request_repaint()`.
5. Record repaint reason and frame count so this can be verified rather than inferred.

### High 2 — Project dependency reconciliation rebuilds multiple maps and sets on every repaint

**Facts**

- Every `ReynApp::ui` call clones every valid model digest into a fresh `Vec<String>` and calls `reconcile_dependencies` (`src/app.rs:980-989`).
- Reconciliation builds a lowercased `BTreeSet`, walks cases and source revisions, builds and sorts issue vectors, runs content diagnostics, and computes a bundle summary (`src/project_lifecycle.rs:339-451`).
- `ProjectDocument::diagnostics` constructs a fresh content-reference map, clones diagnostics/references, and formats missing-owner text (`src/project.rs:700-734`).
- `ProjectDocument::summary` constructs another content-reference set and calls `diagnostics` again (`src/project.rs:737-751`).
- `ProjectManifest::content_references` lowercases digest strings, clones owner IDs, sorts, and deduplicates them (`src/project.rs:1122-1161`).
- Therefore one `reconcile_dependencies` call invokes `content_references()` three times: once from its direct `diagnostics()` call, once directly from `summary()`, and once from `summary()`'s second `diagnostics()` call.
- Cost grows with model inventory, source revisions, run outputs, and evidence. The repaint policy in Finding 1 multiplies this cost.

**Hypothesis to measure**

- Projects with long run/evidence histories may spend a meaningful share of every frame rebuilding identical availability state and transient strings.

**Action**

1. Reconcile only when one of these versions changes: engine availability, model inventory, project manifest, bundled-content diagnostics, or relink state.
2. Store validated model digests in a persistent set instead of cloning them in paint.
3. Cache content references and `BundleSummary` inside the project document, invalidated by manifest/content mutation.
4. Make `summary()` consume an already-computed diagnostics/reference snapshot so it cannot repeat the traversal.

### High 3 — Whole-field analysis and large allocations occur inside paint paths

#### 3A. CAD fit bounds scan the complete mask every 3D frame

**Facts**

- The 3D viewport constructs `ViewOpts::fit_bounds` by calling `cad::mask_bounds` on every paint (`src/app.rs:10208-10215`), whether or not a fit command was requested.
- `mask_bounds` visits every cell in three nested loops (`src/cad.rs:687-706`).
- This is exactly `N³` mask tests per painted 3D frame: 32,768 at `N=32`, 262,144 at `N=64`.
- The mask changes only when case geometry/result state changes, not with camera motion.

**Action**

Cache bounds on `CadCase` when the mask is created or replaced. Pass the cached value to the viewport and only recompute on geometry invalidation.

#### 3B. Particle mode rebuilds and uploads the complete visible instance list every frame

**Facts**

- Particle paint allocates `proj` with capacity equal to the particle count, projects/culls every particle, then allocates a second `Vec<GpuInstance>` (`src/viewport.rs:800-872`).
- `flow::from_field` targets roughly 8,000 samples, with actual count determined by its integer stride (`src/flow.rs:68-133`).
- `FlowCallback::prepare` uploads all instances with `queue.write_buffer` on every callback preparation (`src/gpu.rs:721-749`).
- Optional streamlines allocate polylines/segments again (`src/viewport.rs:873-878`, `src/viewport.rs:921-974`).
- Volume raymarch mode returns before this path (`src/viewport.rs:717-740`), so this finding applies to particle/point rendering, not the default volume callback.

**Hypothesis to measure**

- Full-rate busy repaint can turn static particle data into repeated CPU projection, allocation, and GPU transfer work.

**Action**

Store stable particle attributes in a versioned GPU buffer. Move camera projection/culling into the vertex shader or cache CPU instances until particle data, camera, slices, viewport size, or display controls change. Reuse scratch vectors if CPU projection remains.

#### 3C. 2D insights rederive multiple `N²` arrays every repaint

**Facts**

- `field2d_view` calls `field2d::insights` every paint while insights are enabled (`src/app.rs:11191-11219`).
- `insights` derives vorticity, allocates an absolute-vorticity array, derives speed, and, with truth, derives model/truth/error arrays (`src/field2d.rs:314-360`).
- `scalar` allocates one `N²` vector for each derived quantity (`src/field2d.rs:82-110`).
- With truth enabled, this path can allocate six `N²` float vectors in one repaint: vorticity, absolute vorticity, speed, displayed model scalar, displayed truth scalar, and absolute error.

**Action**

Cache insights by `(f2d_gen, variable, truth-visible)` and store scalar maps produced during texture generation for reuse by insight and probe rendering.

### High 4 — CAD completion does large synchronous work in an unbounded UI-thread drain

**Facts**

- Engine messages are drained with `while let Ok(...)`, with no message or time budget (`src/app.rs:664-665`).
- A recordable `CadField` immediately calls `persist_external_flow_run` from that loop (`src/app.rs:730-775`).
- Persistence clones all five field arrays into an `EngineeringFieldBlob` (`src/app.rs:1925-1933`).
- The blob contains `9 × N³` floats: velocity and traction are three components each; pressure, mask, and `Cp` are one each (`src/engineering.rs:1033-1055`).
- At `N=64`, one complete field payload is 9,437,184 bytes (9 MiB) before container overhead. Persistence temporarily holds the received arrays, another 9 MiB of cloned arrays, and a new approximately 9 MiB encoded byte buffer.
- Encoding validates all values and writes all `9 × N³` floats into a new byte vector (`src/engineering.rs:1058-1086`).
- The encoded field is SHA-256 hashed in `persist_external_flow_run` (`src/app.rs:1934`) and then hashed again by `add_content_with_digest` (`src/project.rs:656-665`).
- The result JSON is likewise serialized and hashed before insertion (`src/app.rs:1942-1986`); evidence byte size serializes the same JSON again (`src/app.rs:2111-2124`).
- After persistence, the same UI message handler performs particle extraction, vorticity-volume generation, insight scans, `Cp` scaling, and two `N³` byte transposes before installing display data (`src/app.rs:774-815`).

**Hypothesis to measure**

- CAD completion can create a long UI frame and a transient memory spike, especially at 64³, even though inference itself runs off-thread.
- A burst of queued responses can extend the stall because the drain has no frame budget.

**Action**

1. Prepare a `CadFieldReady` package off the UI thread: encoded bytes, one digest, display volume, insights, surface bytes, and compact metadata.
2. Change the content-store API to compute and return the digest once, or accept a verified digest without immediately hashing the same immutable buffer again.
3. Encode from borrowed slices or move the arrays into the encoder; do not clone all `9 × N³` floats.
4. Avoid serializing `result_json` solely to calculate `byte_size` a second time; retain the first serialized length.
5. Drain with a per-frame message/time budget. Large results should advance through explicit completion stages while the UI remains paintable.

### High 5 — Autosave, imports, checkpoint bundling, and exports can block the UI thread

**Facts**

- `autosave_if_due` is called from every UI update (`src/app.rs:990-995`). When due and dirty, it serializes the project, parses it back into `serde_json::Value`, pretty-serializes a recovery document, and atomically writes it (`src/project_lifecycle.rs:528-569`).
- STL import synchronously reads the entire file, hashes it, parses it, diagnoses it, and voxelizes it from the UI action (`src/app.rs:9437-9472`). Parsing/diagnosis/voxelization are data-size-dependent (`src/cad.rs:38-40`, `src/cad.rs:252-255`, `src/cad.rs:389-405`).
- Preparing a benchmark case may synchronously read and hash an entire checkpoint before bundling it (`src/app.rs:8016-8041`). Library formatting explicitly supports GiB-sized checkpoint sizes (`src/library.rs:130-137`).
- Project save/open and many report/export writes are also invoked directly from UI callbacks; representative paths are `src/app.rs:7659`, `src/app.rs:7789`, and `src/app.rs:4938-5316`.

**Hypotheses to measure**

- Dirty projects with large bundled fields/checkpoints may hitch at the autosave boundary even while the user is manipulating the viewport.
- Large STL import or first-time benchmark checkpoint bundling may freeze input long enough to appear hung.

**Action**

1. Snapshot immutable project state quickly, then serialize/write recovery data on a worker. Publish success/failure back through the repaint-waking result channel.
2. Move STL read, hash, parse, diagnostics, and voxelization into a cancellable import job with progress.
3. Stream or memory-map checkpoint verification/bundling off-thread rather than reading and hashing it synchronously in the action handler.
4. Keep only the native file dialog on the UI path; perform post-selection work asynchronously.

### Medium 6 — Texture uploads are version-gated, but playback repeats immutable geometry work and resource binding

**Facts**

- GPU volume and surface uploads are correctly guarded by version checks (`src/gpu.rs:889-899`). This audit found no unconditional per-frame 3D texture upload.
- A horizon display refresh recomputes particles, vorticity, insights, the mask byte volume, and the pressure byte volume, then increments both volume and CAD versions (`src/app.rs:1458-1511`).
- The CAD mask is geometry and is unchanged between horizon steps, but `upload_surface` writes both mask and pressure textures whenever `surf_version` changes (`src/gpu.rs:621-671`).
- At `N=64`, each R8 cube is 262,144 bytes. A playback step uploads volume, mask, and pressure (786,432 bytes total); the mask third is redundant when geometry is unchanged.
- `upload_volume` recreates a texture view and bind group after each data update even when dimensions and texture allocation are unchanged (`src/gpu.rs:585-618`).
- `upload_surface` recreates two views and another bind group after each update (`src/gpu.rs:631-671`).
- If volume and surface versions both change in one prepare, `rebuild_volume_bg` can run once after each upload (`src/gpu.rs:889-899`).

**Hypothesis to measure**

- Playback frame installation may show CPU/GPU spikes from redundant mask conversion/upload and bind-group churn.

**Action**

Split geometry-mask identity from pressure/result identity. Upload mask only when geometry changes; upload pressure and vorticity by result version. Retain views for texture lifetime and rebuild the combined bind group once after all changed resources have been updated.

### Medium 7 — Exact-size render targets churn during live resize

**Facts**

- `ensure_targets` recreates targets whenever the callback pixel size differs by one pixel (`src/gpu.rs:510-513`).
- A recreation allocates one full-resolution HDR scene texture, two half-resolution HDR bloom textures, and four bind groups (`src/gpu.rs:428-507`).
- Callback size is rounded from the current viewport rectangle each paint (`src/gpu.rs:1011-1029`, `src/gpu.rs:1037-1069`).

**Hypothesis to measure**

- Drag-resizing a window can allocate and retire this target set on each resize frame, causing allocation spikes or uneven resize animation.

**Action**

Profile resize first. If confirmed, grow targets in buckets, retain a larger target until resize settles, or debounce recreation while rendering into the last valid size.

### Medium 8 — Library rendering clones and reformats the full visible inventory; narrow windows do not virtualize it

**Facts**

- `visible` lowercases the search text for every model and, for a non-empty query, lowercases model name and scenario strings (`src/library.rs:90-106`).
- Every library paint scans health counts and allocates summary/filter strings (`src/library.rs:109-127`, `src/library.rs:509-606`).
- The filtered inventory is materialized by cloning every matching `ModelCard`, including nested metadata vectors (`src/library.rs:644-648`).
- The vertical `ScrollArea` still builds every card; it does not use row virtualization (`src/library.rs:663-681`).
- Each card repeatedly humanizes/compacts names and formats contract, epoch, size, modified time, support, limitations, and provenance text (`src/library.rs:686-838`).
- Column count falls to one on narrow widths (`src/library.rs:612-614`), producing a much taller all-card layout while retaining the same per-card work.

**Hypothesis to measure**

- Large managed inventories may make search typing and narrow-window resize expensive because all cards are cloned and laid out for each repaint.

**Action**

Normalize searchable strings when inventory data arrives, compute the trimmed lowercase query once, filter to borrowed indices/references, cache formatted card facts, and use row virtualization (`show_rows` or equivalent) for large inventories.

### Low 9 — Settings performs repeated allocation/layout work and has no narrow-width stack mode

**Facts**

- Settings uses a fixed 188-point category rail beside the content at every width (`src/settings.rs:699-758`).
- Footer dirty state deep-compares complete `AppSettings` values, including strings and vectors, every paint (`src/settings.rs:800-818`).
- Many category controls call `AppSettings::default()` during rendering, recreating owned default paths/strings; examples are `src/settings.rs:915-950` and `src/settings.rs:954-1004`.
- The scope footnote computes the config path and allocates/layouts owned strings for height, then computes the path again for rendering (`src/settings.rs:861-903`).
- Shortcut categories rebuild vectors of owned strings each paint (`src/settings.rs:1577-1604`, `src/settings.rs:1620-1648`).

**Hypothesis to measure**

- This is probably secondary at current settings sizes, but the fixed two-column structure can increase text reflow and clipping pressure during narrow resize.

**Action**

Use a breakpoint that turns the rail into a compact selector above the content; keep a prebuilt default settings value; track a dirty bit on edits; cache the config-path label and static shortcut rows.

### Medium 10 — Screenshot encoding and writes are synchronous; production GPU-readback cost is unmeasured

**Facts**

- Viewport export requests an egui compositor screenshot and waits for an event (`src/app.rs:5186-5224`).
- On event arrival, it crops/copies the image, builds a full RGBA byte vector, PNG-encodes it, and writes it synchronously before returning to UI work (`src/app.rs:5225-5240`, `src/app.rs:14990-15021`).
- The QA full-window capture follows the same synchronous encode/write path (`src/app.rs:5150-5183`).
- No production `map_async`, `copy_texture_to_buffer`, or `device.poll(wait_indefinitely)` call was found in `reyn-studio` source. The explicit blocking GPU readback at `src/gpu.rs:1488-1490` is inside `#[cfg(test)]` (`src/gpu.rs:1359-1360`) and must not be attributed to the shipped screenshot path.

**Hypotheses to measure**

- The egui/wgpu backend may introduce GPU synchronization while fulfilling `ViewportCommand::Screenshot`; that implementation is outside the audited application source.
- Regardless of backend behavior, 4K crop, RGBA conversion, PNG compression, and file write can create a visible completion hitch on the UI thread.

**Action**

Measure request-to-event, event-to-crop, PNG encode, and write separately. Move crop/encode/write to a worker after taking ownership of the image. Only replace the compositor screenshot with an application-owned readback path if profiling proves backend synchronization is material.

## Existing caches and controls that are working

These paths should be preserved while addressing the findings:

- Engineering sections use a signature and skip extraction/texture creation when it matches (`src/app.rs:10956-10975`); texture creation occurs only after a miss (`src/app.rs:10996-11018`).
- 2D field textures use a generation/variable/truth signature (`src/app.rs:9378-9422`).
- Painter texture rebuild is guarded by `paint_dirty` (`src/app.rs:11256-11283`).
- Benchmark textures return once three handles are present and are cleared on selection/variable changes (`src/app.rs:11641-11748`).
- GPU 3D textures use volume and surface versions (`src/gpu.rs:889-899`), and allocation is reused while dimensions match (`src/gpu.rs:585-600`, `src/gpu.rs:631-648`).
- Camera glides and screen crossfades schedule repaint only while animation remains active (`src/viewport.rs:675-692`, `src/app.rs:10124-10136`).

These facts mean the main texture problem is not "every texture rebuilds every frame." It is redundant work at invalidation boundaries, plus unrelated per-frame CPU work that continuous repaint exposes.

## Profiling plan

### 1. Instrument repaint causes and UI phases

Add release-profile-compatible timing spans and counters around:

- engine drain and each message variant;
- dependency reconciliation;
- autosave due check versus actual serialization/write;
- top/status/sidebar/viewport/library/settings paint;
- `mask_bounds`, 2D insights, particle projection, callback preparation;
- section/field/benchmark texture cache hit or miss;
- texture bytes uploaded and render-target/view/bind-group creations;
- screenshot request, event, crop, encode, and write.

Record frame CPU time, inter-frame interval, repaint reason, allocation count/bytes, message count, and queue-upload bytes. Use macOS Instruments Time Profiler + Allocations, with `os_signpost` or `tracing` intervals so samples map to these phases.

### 2. Reproducible scenarios

Run each for at least 60 seconds after warmup:

1. Idle project, engine ready, static Projects screen.
2. Engine startup failure/unavailable state.
3. Long CAD run pending with a static UI.
4. 64³ result in volume mode, then particle mode.
5. Horizon playback through cached and newly fetched steps.
6. 2D field with truth and insights enabled at representative maximum `N`.
7. Projects with 1, 100, and 1,000 runs/evidence objects.
8. Library inventories with 10, 100, and 1,000 synthetic metadata cards; repeat search and narrow resize.
9. Dirty project autosave with 1, 10, and 100 MiB of bundled content.
10. STL imports at small/median/large triangle counts and first-time checkpoint bundling.
11. Continuous 3D-window resize.
12. 1080p, Retina, and 4K viewport screenshots.

### 3. GPU evidence

- Use an Xcode Metal capture or wgpu timestamp queries around scene, bright, blur, and composite passes.
- Count `queue.write_buffer`/`write_texture` bytes by resource and version.
- Count texture, texture-view, and bind-group creation during static paint, playback, and resize.
- Check whether screenshot fulfillment inserts a GPU bubble before the screenshot event. Do not infer this from the test-only `wait_indefinitely`.

### 4. Response-path evidence

Stamp engine messages with worker send time and record:

- send-to-first-drain latency;
- time spent handling the message on the UI thread;
- messages processed per frame and oldest-message age;
- CAD completion peak resident memory and allocated bytes.

This will distinguish channel polling latency from completion-processing latency.

### 5. Proposed acceptance gates

Treat these as targets to validate with product owners, not measured current behavior:

- Static ready/unavailable screens are event-driven and do not sustain periodic frames.
- Pending work repaints only at the cadence required by visible progress.
- No ordinary interactive frame contains project serialization, whole-file read/hash, PNG compression, or disk write.
- Project-history growth does not change static-frame cost unless project state changes.
- Static 3D volume/particle views upload zero field bytes after warmup.
- Horizon steps do not re-upload an unchanged geometry mask.
- Screenshot and autosave completion cannot create a UI-thread task over 50 ms on the reference machine.
- Report p50/p95/p99 frame CPU and input-to-paint latency for every scenario; do not rely on average FPS alone.

## Graphify evidence

The required graph-first investigation was run from `/Users/hamza/Documents/Pioneer RI` against the existing `graphify-out/graph.json`.

Focused traversals identified these navigation anchors:

- `ReynApp` → `reyn-studio/src/app.rs` (graph location `L288`).
- `.ui()` → `reyn-studio/src/app.rs` (graph location `L629`; current implementation begins at `L657`).
- `.upload_volume()` → `reyn-studio/src/gpu.rs:L578`.
- `.upload_surface()` → `reyn-studio/src/gpu.rs:L623`.
- `.rebuild_volume_bg()` → `reyn-studio/src/gpu.rs:L674`.
- volume callback `.prepare()` → `reyn-studio/src/gpu.rs:L875`.
- `.ensure_cad_section_texture()` → graph location `reyn-studio/src/app.rs:L10604`; current source `L10929`.
- `show_settings()` → `reyn-studio/src/settings.rs:L677`.
- `section()` and each settings category were connected by extracted call edges; for example, `category_compute()` → `section()` at `src/settings.rs:L915`.
- `library.rs` and `model()` → `reyn-studio/src/library.rs:L1` and `L924`.
- `json_strings()` → `reyn-studio/src/engine.rs:L705`.
- screenshot nodes `.export_section_png()`, `.handle_qa_shot()`, `.handle_screenshot_events()`, and `color_image_png_bytes()` were returned in `app.rs`.

Graph integrity/coverage note:

- The graph traversals were broad and included unrelated monorepo communities; several were output-truncated despite returning all or most matched nodes.
- `app.rs` graph offsets were stale by roughly 300 lines for screenshot and section helpers (for example, graph `.handle_screenshot_events()` at `L4917`, current source at `L5214`). This audit used those nodes as navigation aids and verified every finding in current source.
- No graph edge or stale location is used as sole evidence for a finding.

## Conclusion

The first optimization pass should be scheduling and invalidation, not shader simplification:

1. wake the UI on engine messages and stop permanent/idle repaint loops;
2. version dependency availability and field-derived caches;
3. move result preparation, autosave, import, checkpoint hashing, and PNG work off the UI thread;
4. remove duplicate field copies/hashes and unchanged-mask uploads;
5. then profile resize target churn and library virtualization.

This order removes repeated work that scales with every repaint and establishes clean measurements before changing visual fidelity or GPU algorithms.
