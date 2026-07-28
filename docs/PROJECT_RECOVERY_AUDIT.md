# Project Persistence and Recovery Resilience Audit

**Audit date:** 2026-07-25  
**Scope:** Reyn Studio project persistence, autosave, recovery, migrations, case templates, undo/redo boundaries, immutable runs/evidence, atomicity, corruption handling, path portability, and concurrent or failed writes.  
**Mode:** Read-only source audit. No application or settings source was changed.

## Executive summary

Reyn Studio has a strong persistence substrate: project files are versioned, manifest-validated, content-addressed, written through same-directory temporary files, and tested for migration, corruption diagnostics, path-independent reopening, and append-only run/evidence behavior.

The main resilience failure sits above that substrate. Case Setup edits are held only in `ReynApp.cad` and the session-only undo stack until a later revision commit. They do not mark `ProjectLifecycle` dirty, are not included in autosave/recovery, and are not written by Save. After a previously saved project is edited, Save can report success while the visible draft remains non-durable; New, Open, Quit, or a crash can then lose it without an unsaved-changes prompt.

The next largest risks are concurrent last-writer-wins saves/recoveries and presentation-only enforcement of read-only evidence mode. Settings and case templates also have a single-file corruption boundary with no backup or schema migration envelope.

### Top risks

1. **P1 — Visible Case Setup work can be silently lost:** draft edits, template application, model selection, and pending orientation state live outside project dirty/save/autosave/recovery state.
2. **P1 — Concurrent instances can overwrite projects and each other's recovery snapshots:** there is no lock, generation check, or conflict copy; recovery filenames are shared by project ID.
3. **P1 — “Read-only evidence mode” is not enforced by persistence APIs:** mutation methods and run start do not reject writes, and integrity-only bundle failures do not enter read-only mode.
4. **P2 — Settings corruption can strand templates and trust state:** malformed settings load as defaults, with no backup/quarantine or settings schema migration.
5. **P2 — Failed execution attempts disappear from the immutable ledger:** the engine-error path clears pending state without appending a terminal `Failed` run.

## Method and evidence

- Queried the existing workspace Graphify graph for Reyn Studio persistence and recovery flows. The traversal identified `ProjectDocument::save_atomic`, `ProjectLifecycle::{open,save,save_as,autosave_if_due,recover}`, project migration and corruption tests, case history, run/evidence append paths, and case-template persistence.
- Verified every finding against the current source rather than relying on graph edges.
- Reviewed the canonical requirements relevant to this audit:
  - `REQ-N6-PROJ-01` / `N6-PROJ-01`
  - `REQ-N6-PROJ-02` / `N6-PROJ-02`, `N6-PROJ-03`, `N6-PROJ-04`
  - `REQ-N6-PROJ-03` / `N6-PROJ-05`, `N6-PROJ-06`, `N6-PROJ-07`
  - `REQ-N6-SET-01`, `REQ-N6-SET-02` / `N6-SET-01`
  - `REQ-N6-RUN-01` / `N6-RUN-01`
  - `REQ-LOCAL-01` / `LOCAL-AC-01`
  - `REQ-SCI-01` / `SCI-AC-01`, `SCI-AC-02`, `SCI-AC-03`
- Attempted to use the Obsidian CLI for Reyn lifecycle notes. The installed CLI could not connect because Obsidian was not running, so no vault content was read or modified.
- Ran the current focused tests:
  - `cargo test ... project`: **26 passed**
  - `cargo test ... settings::tests`: **12 passed**

The passing tests confirm existing positive controls, but they do not cover the split draft/lifecycle state, multi-process conflicts, read-only enforcement, or injected durability failures identified below.

## Confirmed defects

### [P1] Include active Case Setup drafts in dirty, Save, autosave, recovery, and transition guards

**Evidence**

- `ReynApp::controls_engineering_case` mutates `case.workflow` directly and records only a session snapshot. On change it invalidates the current result, but does not update `ProjectLifecycle` or mark the project dirty: `src/app.rs:2259-2263`, `src/app.rs:2315-2316`, `src/app.rs:2683-2997`, `src/app.rs:3239-3256`.
- `ReynApp::record_case_draft_change` writes only to `CaseDraftHistory`: `src/app.rs:1228-1248`.
- `CaseDraftHistory` explicitly stores a bounded, session-only stack: `src/engineering.rs:648-714`.
- A case revision reaches the manifest only through `ReynApp::commit_active_case_revision`, normally when orientation is applied or a run starts: `src/app.rs:1686-1819`, `src/app.rs:1842-1848`.
- `ProjectLifecycle::save` and `save_as` serialize only `ProjectDocument`: `src/project_lifecycle.rs:498-523`.
- `ProjectLifecycle::autosave_if_due` snapshots only `self.document`: `src/project_lifecycle.rs:528-557`.
- New/Open/Recover/Quit guarding checks only `self.project.is_dirty()`: `src/app.rs:7757-7766`.

**Failure scenario**

1. Open and save a project.
2. Change velocity, density, viscosity, reference pressure, horizon, transform approval, waiver, model, or apply a case template.
3. The UI shows the changed draft, but `ProjectLifecycle::dirty` can remain false.
4. Save writes the older persisted case revision and can report “Saved atomically.”
5. New, Open, Quit, or a crash can discard the visible draft without a recovery snapshot or unsaved-changes prompt.

Pending orientation edits in `orientation_draft` are also session-only and outside the guard.

**User impact**

Silent loss of engineering setup work and false assurance from Save/Saved Locally status. Undo history is also lost because it is intentionally session-only, but the more serious defect is that the current draft itself is not durable.

**Recommended fix**

- Introduce a versioned case-draft overlay owned by `ProjectLifecycle`, or commit every meaningful draft transaction into a non-run case draft record.
- Make one aggregate `has_unsaved_work` predicate include manifest changes, content changes, active case draft changes, and pending orientation changes.
- Serialize the active draft into both explicit Save and recovery snapshots.
- On Save, either commit the draft as a new case revision or explicitly persist a draft object; never report success while visible project-scoped state remains outside the saved document.
- Keep source/model/run/evidence identity outside undo payloads, preserving the current safe history boundary.

### [P1] Detect concurrent project saves and isolate recovery snapshots per session

**Evidence**

- `project::write_atomic` uses a unique sibling temporary file and rename but performs no lock, generation check, on-open digest comparison, or conflict detection: `src/project.rs:2086-2110`.
- `ProjectLifecycle::open` retains no file identity or baseline digest for a later compare-before-replace: `src/project_lifecycle.rs:454-495`.
- `ProjectLifecycle::save` unconditionally replaces the active path: `src/project_lifecycle.rs:498-501`.
- Recovery path identity is only `SHA-256(project_id)`: `src/project_lifecycle.rs:756-764`.
- Autosaves from two processes therefore target the same recovery file: `src/project_lifecycle.rs:528-568`.
- A successful save deletes the active project's shared recovery path: `src/project_lifecycle.rs:630-644`.
- Recent-project state is also read-modify-replaced without inter-process coordination: `src/project_lifecycle.rs:650-666`, `src/project_lifecycle.rs:811-823`.

**Failure scenario**

Two Reyn Studio instances open the same project. Each makes valid edits. The last save silently replaces the first instance's work. Their autosaves also replace the same recovery file; a successful save in one instance may delete the only recovery snapshot containing unsaved work from the other.

**User impact**

Silent lost updates in the authoritative project and loss of the fallback recovery copy. Atomic rename prevents torn bytes, but it does not prevent logical overwrite.

**Recommended fix**

- Store the opened file's canonical digest plus stable file identity/mtime and compare it immediately before publish.
- Use an advisory project lock where supported, but retain optimistic conflict detection for unreliable/network filesystems.
- On conflict, refuse overwrite and offer reload, Save As, or a timestamped conflict copy.
- Include an application-instance UUID in recovery filenames and index all snapshots by project ID plus instance ID.
- Delete only the recovery snapshot created by the saving session.
- Apply equivalent locking/version checks to recent-project and settings state.

### [P1] Enforce read-only evidence mode inside lifecycle mutation APIs

**Evidence**

- `ProjectLifecycle::reconcile_dependencies` computes `ReadOnlyEvidence`: `src/project_lifecycle.rs:339-451`.
- `ProjectLifecycle::transact`, `add_content_with_digest`, and `relink_content` do not enforce access mode: `src/project_lifecycle.rs:266-333`.
- `ReynApp::record_case_draft_change` avoids recording history in read-only mode, but the controls have already mutated `case.workflow`: `src/app.rs:1228-1248`, `src/app.rs:2259-2263`.
- `ReynApp::run_external_flow` checks engine availability and readiness, but not `ProjectAvailability::is_read_only_evidence`: `src/app.rs:1822-1900`.
- Merely selecting a stored run mutates persisted selection and marks the project dirty through `transact`: `src/app.rs:9240-9292`.

**User impact**

A project advertised as read-only can acquire draft mutations, persisted selection mutations, or a new case revision. Depending on which dependency is missing, a run can reach persistence/engine paths before failing. This weakens the safety claim around evidence review and makes “read-only” behavior dependent on UI call-site discipline.

**Recommended fix**

- Add mutation classes to `ProjectLifecycle` and reject scientific/project mutation while access mode is read-only.
- Permit only explicitly safe operations such as content relink, Save As of an unchanged degraded document, and ephemeral UI selection.
- Keep review selection out of the authoritative manifest, or provide a separate local-view-state store that does not dirty scientific project content.
- Disable Case Setup controls and run commands from the same central capability decision.

### [P1] Treat bundle-integrity mismatch as a mutation-blocking state

**Evidence**

- `ProjectDocument::decode` records a `BundleIntegrity` diagnostic when the index digest mismatches but still returns the document: `src/project.rs:872-885`.
- This diagnostic does not set `needs_normalization`: `src/project.rs:886-962`.
- `ProjectLifecycle::open` sets dirty only for migration or normalization, while warning that the project opened in “evidence-safe mode”: `src/project_lifecycle.rs:461-488`.
- `reconcile_dependencies` excludes `DependencyKind::Integrity` from its blocking set, so integrity-only failures remain `ProjectAccessMode::Full`: `src/project_lifecycle.rs:396-447`.
- The UI explicitly labels that state “BUNDLE INTEGRITY NOTICE,” not read-only: `src/app.rs:6873-6881`.

**User impact**

A project whose bundle index failed verification can still be edited and used for new work. A later save can normalize away the diagnostic, making it harder to distinguish an intentional repair from overwriting potentially tampered input.

**Recommended fix**

- Make any bundle-integrity failure mutation-blocking until the user explicitly chooses a repair workflow.
- Preserve the original bytes or force repair to a new path.
- Record a repair event with the original and repaired digests.
- Distinguish “content individually verified but index integrity failed” from missing/corrupt object states without granting full access.

### [P2] Preserve corrupted settings and templates instead of silently falling back to a replaceable default

**Evidence**

- `AppSettings::load` returns `AppSettings::default()` for any read or JSON error: `src/settings.rs:384-410`.
- User case templates, operating presets, signing public state, and revoked fingerprints live in that one settings document: `src/settings.rs:283-340`.
- There is no settings schema version or migration dispatcher on `AppSettings`; compatibility relies on serde defaults: `src/settings.rs:283-340`.
- The next successful `AppSettings::save` replaces the same path: `src/settings.rs:485-488`, `src/settings.rs:2465-2473`.
- `CaseTemplate` is strict with `deny_unknown_fields`, so an incompatible nested template can make the entire settings file fail to deserialize: `src/settings.rs:138-160`.

**User impact**

A truncated, partially written, manually edited, or forward-version settings file makes all templates and trust/revocation state disappear from the running UI. If the user then saves any preference, the original file is replaced by defaults plus the new edit.

**Recommended fix**

- Add a versioned settings envelope and explicit migrations.
- Before replacement, maintain a last-known-good backup and validate the newly written file by reopening it.
- On load failure, quarantine/preserve the original and expose recovery/import actions; do not make an in-memory default eligible to overwrite it without explicit confirmation.
- Store user templates in an independently recoverable collection or include them in backup/restore.
- Preserve unknown top-level fields across downgrade, or reject future settings versions without rewriting them.

### [P2] Retry autosave promptly after transient failure

**Evidence**

`ProjectLifecycle::autosave_if_due` advances `last_autosave_attempt_utc_unix` before serialization and before the atomic write: `src/project_lifecycle.rs:536-557`.

**User impact**

An ENOSPC, temporary permission issue, antivirus lock, transient filesystem outage, or serialization error suppresses another recovery attempt for the full configured interval (30–3600 seconds), even if the condition clears immediately.

**Recommended fix**

- Track last successful autosave separately from last attempt.
- On failure, use a short bounded retry with backoff and a persistent visible warning.
- Clear the warning only after a verified successful recovery write.

### [P2] Do not claim durable atomic success when directory sync is ignored

**Evidence**

- Project and lifecycle writers sync the temporary file, rename it, then ignore parent-directory `sync_all` failure: `src/project.rs:2086-2110`, `src/project_lifecycle.rs:935-959`.
- Settings and template writers use `std::fs::write` plus a fixed temporary pathname, with no file sync or directory sync: `src/settings.rs:2465-2473`, `src/settings.rs:2502-2510`.
- The app reports “Saved atomically” after these paths return success: `src/app.rs:7650-7705`.

**User impact**

Atomic replacement protects against many torn-write cases, but ignored durability errors mean success can be reported even when a sudden power loss may lose the directory entry. Fixed settings/template temp names also collide across processes.

**Recommended fix**

- Use one shared atomic-replace implementation with a unique sibling temp file.
- Write, flush, sync the temp, publish with a platform-correct replace primitive, sync the parent directory where supported, and propagate durability failures.
- Reopen and validate critical JSON after publish.
- Distinguish “atomically replaced” from “durably committed” in status/error handling.

### [P2] Persist non-cancelled engine failures as terminal immutable attempts

**Evidence**

- A submitted engineering run stores its exact workflow in `pending_run`: `src/app.rs:1822-1869`.
- On `engine::Msg::Error`, the app clears `pending_run` and returns the case to Ready without appending a run: `src/app.rs:934-952`.
- The model supports `LifecycleState::Failed`, and `append_run` accepts it as terminal: `src/project.rs:86-96`, `src/project.rs:1250-1263`.
- The PRD defines a Run as an immutable execution attempt with runtime, warnings, and stop reason, and includes `Failed` as a lifecycle state: `PRD.md:218-245`.

**User impact**

Engine failures are visible only in transient UI state. Reopening the project cannot answer what was attempted, with which exact inputs/model/device, how long it ran, or why it failed.

**Recommended fix**

- When an accepted, non-cancelled request fails, append a terminal `Failed` run using the captured pending workflow, elapsed runtime, device/model identity, and sanitized error as stop reason/warning.
- Keep the current `REQ-N6-RUN-01` rule that cancelled and stale results create no run or evidence.
- Do not create evidence artifacts unless failure output bytes actually exist and verify.

### [P2] Add migration handling for local recent/recovery state and forward settings data

**Evidence**

- Recent and recovery wrappers use strict `STATE_SCHEMA_VERSION = 1`: `src/project_lifecycle.rs:17-21`, `src/project_lifecycle.rs:143-159`.
- Unsupported recent/recovery versions return errors rather than migrate: `src/project_lifecycle.rs:779-808`, `src/project_lifecycle.rs:914-924`.
- Startup converts those errors into warnings and presents no entries: `src/project_lifecycle.rs:175-214`.
- `AppSettings` has no equivalent schema field or migration switch: `src/settings.rs:283-340`.
- In contrast, the authoritative project document has a strict verified v1-to-v2 migration path: `src/project.rs:433-565`, `src/project.rs:823-867`.

**User impact**

After a downgrade or future local-state format change, valid recovery snapshots can become invisible at startup even though their files remain on disk. Future top-level settings fields can also be dropped by an older build when it next saves.

**Recommended fix**

- Apply the project-document pattern to settings, recent lists, and recovery wrappers: inspect version first, migrate known versions, reject future versions without overwrite, and retain the original bytes.
- Continue scanning recovery files independently so one unsupported/corrupt entry does not hide valid siblings.

### [P3] Make case-template application one coherent undo transaction

**Evidence**

- `CaseDraftSnapshot` captures operating values, transform approval, and waivers, but not preferred section axis/quantity: `src/engineering.rs:677-702`.
- Applying a case template changes both operating values and the app's section axis/quantity: `src/app.rs:2831-2848`.
- Undo restores only `CaseDraftSnapshot`: `src/app.rs:1186-1215`.

**User impact**

Undo after applying a template restores the operating point but leaves the template's preferred view active. This is not scientific data corruption, but the command is not fully reversible from the user's perspective.

**Recommended fix**

Either include template-owned view defaults in the undo transaction or explicitly define/apply the view change as a separate non-undoable display action with matching UI copy.

## Runtime hypotheses requiring fault or platform testing

These risks are source-backed but were not reproduced in this macOS read-only audit.

### [H1/P1] Large project autosave may stall the UI and amplify memory/disk pressure

- Autosave runs synchronously inside `eframe::App::update`: `src/app.rs:990-995`.
- `ProjectDocument::to_bytes` hex-encodes every required bundled object, doubling binary payload size before JSON overhead: `src/project.rs:768-816`.
- Recovery then reparses that full JSON into a `serde_json::Value` and serializes a wrapper copy: `src/project_lifecycle.rs:541-557`.

For large engineering fields/evidence, this can cause frame stalls, multiple full-size allocations, long recovery writes, and increased ENOSPC exposure. Measure before assigning a confirmed severity.

### [H2/P1] Replacing an existing file may fail on Windows

All writers publish with `std::fs::rename(temp, destination)`: `src/project.rs:2105`, `src/project_lifecycle.rs:954`, `src/settings.rs:2473`, `src/settings.rs:2510`.

Replacement semantics differ by platform; Windows commonly rejects rename when the destination already exists unless a replace-capable primitive is used. If reproduced, ordinary second saves, subsequent autosaves, settings updates, and template overwrite exports fail. Validate on a Windows CI runner and adopt a platform-correct atomic replace implementation.

### [H3/P2] Network/removable filesystems may not honor expected rename, lock, or directory-sync semantics

The current code assumes local filesystem behavior and has no capability check or post-publish verification. Test SMB/NFS/cloud-synced folders and removable media, then document supported storage or provide a safer copy/verify/replace fallback.

## Positive controls verified

- **Strict authoritative schema:** project envelope and manifest use `deny_unknown_fields`; future schemas are rejected rather than silently truncated: `src/project.rs:13-26`, `src/project.rs:421-441`, `src/project.rs:823-867`.
- **Verified migration:** v1 integrity is checked before v1-to-v2 conversion: `src/project.rs:829-843`.
- **Clone-before-commit:** `ProjectLifecycle::transact` edits a manifest clone and swaps only after the closure succeeds: `src/project_lifecycle.rs:321-334`.
- **Append-only public API:** runs and evidence are private collections exposed through immutable slices; append paths validate IDs, lineage, terminal state, hashes, views, and signatures: `src/project.rs:997-1346`.
- **Same-directory project/recovery temp files:** project and local lifecycle writes use unique sibling temps, sync file contents, and only then rename: `src/project.rs:2086-2108`, `src/project_lifecycle.rs:935-957`.
- **Failed pre-publish save preserves old bytes:** covered by `project::tests::failed_atomic_save_preserves_the_last_valid_document`: `src/project.rs:2602-2627`.
- **Malformed/future Open preserves current work:** covered by `project_lifecycle::tests::malformed_and_future_open_leave_active_unsaved_work_untouched`: `src/project_lifecycle.rs:1131-1162`.
- **Recovery preserves bundled content and immutable evidence:** `src/project_lifecycle.rs:1220-1259`, `src/project_lifecycle.rs:1358-1393`.
- **Corrupt content remains diagnosable without deleting manifest lineage:** `src/project.rs:872-962`, `src/project.rs:2751-2785`.
- **Portable project content:** authoritative lookup is digest-based; source URI is only a hint. Move/reopen is tested: `src/project.rs:409-419`, `src/project.rs:2713-2749`.
- **Portable case templates:** templates contain SI defaults and view preferences, exclude identity/runs/evidence, validate schema and ranges, and reject future schema imports: `src/settings.rs:130-239`, `src/settings.rs:2476-2515`.
- **Undo identity boundary:** source/model/run/evidence identities are excluded from snapshots, and history rebases across identity/revision transitions: `src/engineering.rs:648-714`, `src/app.rs:1816-1819`.

## Recommended test plan

### 1. Draft durability and guard integration

- Save a baseline project, edit each Case Setup field, apply a preset/template, add/remove a waiver, change model identity, and stage orientation.
- After every edit, assert the aggregate unsaved predicate becomes true.
- Assert explicit Save persists exactly the visible draft.
- Assert autosave/restart/recover restores the visible draft.
- Assert New/Open/Quit always prompts while any project-scoped draft state is unsaved.
- Assert Save never clears dirty while visible project state remains unpersisted.

### 2. Concurrent writer tests

- Open the same project in two lifecycle instances, save A, then save B; require a conflict rather than last-writer-wins.
- Autosave both instances and verify independent recovery entries survive.
- Save A and verify only A's recovery snapshot is removed.
- Repeat for settings and recent-project state.

### 3. Fault-injected atomicity and retry tests

Inject failures after create, partial write, file sync, rename, and directory sync.

- The previous project must remain readable or the new project must be fully readable; never neither.
- A failed autosave must retry on the short failure schedule, not wait the full normal interval.
- Temporary files must be safely cleaned or ignored on next startup.
- ENOSPC and permission errors must leave dirty state and recovery warnings visible.

### 4. Read-only and corruption matrix

Open projects with:

- missing engine;
- missing model;
- missing source;
- missing artifact;
- corrupt object bytes;
- duplicate bundle object;
- bundle-index integrity mismatch.

For each state, exercise every mutation entry point and assert central policy enforcement. Review navigation should remain available without dirtying scientific project content.

### 5. Migration and downgrade fixtures

- Project v1, current v2, malformed, unknown-field, and future-version fixtures.
- Settings, recent list, recovery, and case-template fixtures for every supported version plus future versions.
- Verify unknown/future data is never overwritten by an older build.
- Verify corrupt settings are preserved and last-known-good templates/trust state can be restored.

### 6. Immutable terminal-run tests

- Successful request creates one `Complete` run and linked evidence.
- Engine failure after accepted submission creates one `Failed` run with exact captured contract and stop reason, and no fabricated evidence.
- Cancelled and stale requests create no run/result/evidence, preserving `REQ-N6-RUN-01`.
- Persistence failure must not present the transient result as a durable project result and must offer retry/recovery.

### 7. Undo/redo boundary tests

- Template application undoes/redoes all template-owned fields as one transaction.
- Source/model identity changes rebase history.
- Undo/redo never changes completed runs, evidence, hashes, signing state, IDs, or lineage.
- Recovery restores the current draft but starts with an intentionally empty session undo stack unless undo persistence becomes an explicit requirement.

### 8. Platform and storage tests

- Windows: second project save, repeated autosave, repeated settings save, and template overwrite.
- macOS/Linux: power-loss simulation around rename and directory sync.
- SMB/NFS/cloud-synced/removable destinations: replace behavior, conflict detection, and reopen verification.

### 9. Large-bundle performance and capacity tests

- Build representative 100 MiB, 500 MiB, and 1 GiB content-addressed projects.
- Measure UI-frame stall, peak RSS, temporary disk amplification, autosave duration, and recovery startup duration.
- Trigger low-disk conditions and verify prompt retries, preserved dirty state, and no loss of the last valid project/recovery.

## Release recommendation

Do not treat `N6-PROJ-01` crash recovery as release-complete until the active case draft participates in dirty/save/autosave/guard semantics and concurrent writers are conflict-safe. Read-only evidence mode should also be enforced centrally before relying on it as an integrity boundary.
