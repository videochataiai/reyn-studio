# Visual QA Punch List — full-app audit (Tier 2 state)

**Date:** 2026-07-24 · **Auditor:** read-only visual QA pass
**Method:** live capture of the running release binary (window-ID `screencapture`, Projects screen + close-up crops under `docs/qa/`) plus a line-by-line layout audit of `src/app.rs`, `src/library.rs`, `src/settings.rs`, `src/theme.rs`, `src/fonts.rs`, `src/main.rs` against `docs/DESIGN_OVERHAUL.md` §3.

**Build caveat (read first):** the on-screen instance was launched at 14:42; `target/release/reyn-studio` was rebuilt at 14:56 by the concurrently-working agent. Items tagged **[re-verify]** were observed live but may already differ in the newest binary. Everything with a `file:line` reference is reproducible from source as of this audit. Screenshots could not cover screens other than Projects (no GUI driver; synthetic click injection was not permitted), so all other screens are audited from source.

**Captures:**
- `docs/qa/01_projects_landing.png` — full window, Projects screen
- `docs/qa/02_top_chrome_closeup.png` — titlebar + top-bar band
- `docs/qa/03_right_edge_closeup.png` — center/right-panel boundary (clipped ember CTA sliver)
- `docs/qa/04_nav_rail_closeup.png` — nav rail icons + blocked Results row
- `docs/qa/full.png` — source frame for the crops
- Founder's Settings screenshot (referenced as "Settings screenshot")

Severity: **Blocker** = broken/unreachable/artifact visible in normal use · **Major** = visibly wrong or overlapping at common sizes · **Minor** = polish debt.

---

## Disposition — implementation pass, 2026-07-24 (annotation; original audit below is unchanged)

Every item verified against the current source and a fresh release build (window-ID captures of the landing, Settings, and Case Setup at 1100×700). Of the 68 annotated rows (the audit's 54 deduplicated issues): **FIXED 58 · OPEN 7 (all minor, consciously deferred) · INVALID 3**. All 7 blockers and all 21 majors are closed.

**Blockers (7): all closed.**

| # | Disposition | How / why |
|---|---|---|
| P1 | **FIXED** | width-cap — hero text column capped to `available − button − gap`; description wraps, ember CTA stays inside the panel |
| P2 | **FIXED** | width-cap — identity column capped; location path elides with full-path hover; fact pills already `horizontal_wrapped` |
| P3 | **FIXED** | scroll — every right rail now goes through one `ScrollArea` at the `right_controls` dispatch point; `bottom_up` caption converted to flow |
| P4 | **FIXED** | sweep — all 8.5–10.5px `RichText::size` call sites moved to `mono_s()`/`caption()` (63 sites); only 2 intentional indicator dots remain small |
| P5 | **OPEN** | caps state tokens (`DEPENDENCIES RECONCILED` …) kept deliberately as mono state chips; sizes raised to the 11px floor by the P4 sweep |
| P6 | **FIXED** | truncate — recents name/path column width-capped, path elides with hover |
| P7 | **FIXED** | format_utc — epoch rendered as `YYYY-MM-DD HH:MM UTC` |
| P8 | **FIXED** | sweep — glyph and message now share the caption size, baselines align |
| P9 | **FIXED** | indent — blocked reason draws at the label column (x=36) |
| CS1 | **FIXED** | scroll — dispatch-level rail `ScrollArea`; the Run button is always reachable (verified at 1100×700) |
| CS2 | **FIXED** | gated — top-bar Run uses `action_button_gated` + `run_gate_reason()`; renders quiet/disabled-with-reason, never a dead ember (verified in capture) |
| CS3 | **FIXED** | truncate — `diag()` value elides into remaining width with full value on hover (fixes the whole G1 family) |
| CS4 | **FIXED** | sweep — matrix rows at `mono_s()` |
| CS5 | **FIXED** | styles — NOTICE rows wear the warn recipe (Tier 3); readiness tokens raised to `mono_s()` |
| CS6 | **FIXED** | spine — the horizontal caps ribbon was replaced by the §4.3 vertical stage spine (Tier 3) |
| CS7 | **FIXED** | style — subtitle at `mono_s()` with short ids |
| CS8 | **FIXED** | status-line — disabled button replaced with a plain non-interactive explanation |
| R1 | **FIXED** | scroll — same dispatch-level rail `ScrollArea` |
| R2 | **FIXED** | truncate + redesign — `diag()` elides; loads moved to §4.4 `measure_row` table (Tier 3) |
| R3 | **FIXED** | removed — decorative 40px grid retired |
| R4 | **FIXED** | replaced — applicability banner + sentence-case source line replaced the wrapping caps chip |
| R5 | **FIXED** | recipe — warn alert recipe at caption size (Tier 3) |
| R6 | **FIXED** | style — section rows at `mono_s()` (Tier 3) |
| R7 | **FIXED** | threshold — hint line is dropped below 420px viewport height so it can never collide with the legend; camera chip unchanged (single recipe already) |
| R8 | **OPEN** | painter-drawn section header at fixed offsets needs an engineering_section refactor; low blast radius, deferred |
| R9 | **FIXED** | card — empty states are level-1 cards with one action |
| E1 | **FIXED** | ledger — 12-char prefix + copy button + full-hash tooltip (`ledger_row`, Tier 3); `diag()` overflow fixed globally |
| E2 | **FIXED** | rows — one row per scientific statement (Tier 3 rewrite) |
| E3 | **FIXED** | casing — "Export surface loads for FEA…", "Save project" |
| E4 | **FIXED** | recipe — warn recipe at caption size (Tier 3) |
| L1 | **FIXED** | wrap — health chips moved to their own `horizontal_wrapped` row below search/segmented/refresh |
| L2 | **FIXED** | flow — footer follows content (no `bottom_up` inside unbounded grid cells) |
| L3 | **FIXED** | wording — `hUNKNOWN` → `horizon UNKNOWN` (matches `contract_line`); unit test updated to assert the new token |
| L4 | **FIXED** | truncate — card title elides, status chip keeps priority |
| L5 | **FIXED** | budget — ACTIVE chip quiet (`TEXT_DIM`); the 2px ember edge marker carries the accent |
| L6 | **FIXED** | margin — 24px bottom breathing inside the grid ScrollArea |
| L7 | **INVALID** | stale binary — icons render after rebuild (PUA-stripped fonts + regression test); phosphor is also in the Monospace chain now |
| S1 | **FIXED** | reserve — 56px action strip reserved before the ScrollArea (Phase-1 fix, re-verified in capture) |
| S2 | **FIXED** | flex-width — `path_control` uses remaining row width + full-path hover (Phase-1 fix, re-verified) |
| S3 | **FIXED** | yield — label column is `clamp(140, available − 216, 360)` so the control never overlaps; rows gained vertical padding |
| S4 | **FIXED** | threaded — `engine_ok` passed into `show_controls`; color from `engine_state_color()` (SUCCESS/WARN), glyph+word stay in the status string; honesty unit test added |
| S5 | **FIXED** | well — swatches sit on a BG_0 well with OUTLINE borders |
| S6 | **FIXED** | column — Settings uses `content_column(1040)` like the library |
| S7 | **FIXED** | sweep — fingerprint/key reference at `mono_s()`; state tokens stay caps by design |
| S8 | **FIXED** | elide — settings-file path truncates with full path on hover |
| C1 | **FIXED** | transparent titlebar — `with_titlebar_shown(false)` + fullsize content view (Phase-1 fix, re-verified: no gray strip, lights float over the 44px bar) |
| C2 | **INVALID** | stale binary — rebuilt top bar shows brand, project chip, and the gated Run action (verified in capture) |
| C3 | **FIXED** | PUA strip — Inter/JBMono PUA cmaps removed so Phosphor resolves; regression test guards it (Phase-1 fix, re-verified) |
| C4 | **FIXED** | chain — phosphor appended to Monospace and `FAMILY_MONO_MEDIUM` fallbacks (test asserts presence) |
| C5 | **FIXED** | per-edge — panels stroke nothing; each paints one meeting-edge hairline (top bar bottom, status top, sidebar right, rail left) |
| C6 | **FIXED** | truncate + hue — engine status elides before the right cluster with hover; busy indicator WARN, ember reserved for actions |
| C7 | **FIXED** | token — central fill is `BG_VIEWPORT` only for render screens (Results/Metrics/2D/Painter/Bench), `BG` for document screens |
| C8 | **FIXED** | veil — crossfade is a BG-colored overlay painted last (covers the wgpu pass too); `multiply_opacity` removed |
| C9 | **INVALID** | not in-app — window-ID capture of Settings shows no purple rectangle; consistent with a background window peeking past the app edge in the founder's full-screen screenshot |
| C10 | **FIXED** | inset — 78px traffic-light inset when not fullscreen |
| C11 | **FIXED** | grid + focus — nav rows 32px (blocked 48px); blocked rows are `Sense::click` (focusable) with activation ignored |
| C12 | **OPEN** | deliberate redundancy for now (title dot + chrome chip dot + sidebar state chip); consolidation deferred as a design decision |
| X1 | **FIXED** | scroll — covered by the dispatch-level rail ScrollArea; painter/bench `bottom_up` CTAs converted to flow |
| X2 | **FIXED** | casing — "Generate flow", "Apply Leray projection", "Run full suite" / "Running…" / "Inspecting cell…" |
| X3 | **FIXED** | gated + elide — bench run button uses `action_button_gated` with per-state reasons; all `action_button` labels elide inside the rect |
| X4 | **OPEN** | Fields 2D loading/empty states stay bare strings; skeleton treatment deferred (sandbox, low visibility) |
| X5 | **OPEN** | painter-drawn canvas chips/blocks keep hardcoded sizes; needs a canvas text-recipe pass (sandbox, deferred) |
| X6 | **FIXED** | color — combo `selected_text` in TEXT; BRAND stays wordmark-only |
| G1 | **FIXED** | helper — `diag()` truncates the value galley with full-value hover (root fix for R2/CS3/E1) |
| G2 | **FIXED** | sweep — 63 sub-floor `.size()` call sites replaced with named styles across `app.rs`/`settings.rs`; remaining two are intentional glyph dots |
| G3 | **OPEN** | content max-widths still vary (976/1048/1040); Settings now uses 1040 — full unification deferred |
| G4 | **OPEN** | `i8` margin gutters remain on Case/Evidence center views; works, but should move to `content_column` math — deferred |
| G5 | **FIXED** | survivable — verified live at 1100×700: rails scroll, `diag()` elides, setting rows yield, the spine replaced the wide ribbon; min size stays 1100×700 |

**Consciously left open (all minor except noted):** P5 (caps state-token convention), R8 (painter header refactor), C12 (dirty-indicator consolidation), X4/X5 (sandbox state/canvas polish), G3/G4 (layout unification). None affects reachability, honesty, or overlap at supported sizes.

---

## 1. Projects / landing

| # | Region | Issue | Severity | Evidence | Fix direction |
|---|---|---|---|---|---|
| P1 | Hero card | **Ember "Import Geometry…" CTA clipped under the right panel.** The hero card's `ui.horizontal` contains an unwrapped description label ("Import geometry, qualify the setup, run the fixed-body model…") whose intrinsic width exceeds the center panel; the frame grows past the panel and the right-to-left button lands beyond the clip — ember pixels are visible ~80 px *inside* the right rail region. | Blocker | `app.rs:3427-3455`; `qa/01`, `qa/03` (orange sliver at left edge of crop) | Constrain the inner `ui.vertical` width (`set_max_width(available − button_width − gap)`) so the label wraps; never place an unwrapped long label beside a right-aligned control |
| P2 | Current-project card | Same overflow: "LOCAL SESSION" state chip renders as "LOCAL SESSI…" cut by the panel edge; fact-pill row also clips (bracket sliver visible mid-height in `qa/03`). | Major | `app.rs:3495-3538`; `qa/01` | Same fix as P1 — width-cap the left column, keep `horizontal_wrapped` pills inside the frame width |
| P3 | Right rail | `controls_project` has **no ScrollArea**; at min window height (700) the card stack + 3 buttons + bottom-anchored autosave caption exceed panel height, so "Open Project…" clips and the `bottom_up` caption overlaps content. | Major | `app.rs:3121-3373` (buttons 3314-3351, bottom-up 3353-3369) | Wrap rail content in `ScrollArea::vertical()`, reserving fixed space for the bottom caption |
| P4 | Everywhere on screen | Micro-fonts below the 11.5 px caption floor: project-id mono 9.0 (`3207-3212`), diagnostics 9.5 (`3277-3292`), source rows 9.0 (`3707-3718`), determinism deltas **8.5 red** (`3843-3867`), evidence rows 8.5 (`3892-3908`), recents path 9.5 (`4049-4054`). | Major (count) | lines cited | Move all to `mono_s()` (11.0) minimum; deltas/evidence to 11 with truncation |
| P5 | Cards/rows | ALL-CAPS mono status strings outside the two sanctioned caps styles: "DEPENDENCIES RECONCILED" / "COMPUTE UNAVAILABLE · STORAGE READY" (`3221-3248`), "N PRECISE DIAGNOSTIC(S)" (`3253-3268`), "DEPENDENCIES CURRENT"/"STALE · …" (`3664-3684`), "COMPLETE · IMMUTABLE" (`3760-3780`). | Minor | lines cited | Convert to `chip_text` (state tokens) or sentence-case body + status glyph |
| P6 | Recent projects | Full filesystem paths in mono 9.5 with no truncation next to right-aligned Remove/Open buttons — long paths force the row past the frame (same clipping family as P1). | Major | `app.rs:4042-4081` | Middle-elide path (`…`) with full path on hover; `Label::truncate()` |
| P7 | Recovery rows | "autosaved UTC unix 1769…" — raw epoch integer in user-facing copy. | Minor | `app.rs:3973-3981` | Format as local date/time; keep exact value on hover |
| P8 | Notices | Alert glyph "!" is a separate label at a different size than the message → baseline misalignment in every notice (also in `controls_project` at 10.5 px, below floor). | Minor | `app.rs:3141-3153`, `3466-3485` | One `RichText` per notice or fixed-height horizontal with `Align::Center`; raise to caption size |
| P9 | Nav rail (live) | Blocked "Results" row's inline reason "○ needs a completed run" sits 6 px above the next row and starts at the icon column, not the label column — reads cramped/misaligned. | Minor | `app.rs:10320-10328`; `qa/04` | Indent reason to label x (40), add 4 px to the blocked row height (50→56) |

## 2. Case Setup

| # | Region | Issue | Severity | Evidence | Fix direction |
|---|---|---|---|---|---|
| CS1 | Right rail | **No ScrollArea and content ~2× the panel height.** "Source & transform" (10 diag rows + collapsing + combo + checkbox) + "Operating point" (combo + 5 diag + 5 DragValues + slider + 2 diag) ≈ 1000 px in a ~780 px panel: the readiness verdict, blocker list, waiver text field, and the **"Run qualified analysis" ember button are clipped off-screen and unreachable** at the default 1440×900 window. | Blocker | `app.rs:1177-1615` (run button `1588-1604`, waiver flow `1559-1583`) | `ScrollArea::vertical()` around rail content with the Run button pinned in a fixed bottom strip above the status bar |
| CS2 | Top bar | Ember "Run analysis" is always drawn on Case/Results — even with no geometry imported and no runnable case; simultaneously the empty-state shows the ember "Import Geometry…" → dead primary + two ember CTAs on one screen. | Major | `app.rs:2683-2699` + `2101-2112` | Disable-with-reason (or hide) top-bar Run until `workflow.ready()`; one ember per screen |
| CS3 | Right rail rows | `diag()` draws label left and value right with **no overlap protection**: "Solver characteristic length" (~190 px) + "0.123456 solver units" (~150 px mono) > 282 px content width → overlapping text at the default 330 px rail. Also "Extents"/"Voxel adequacy" values. | Major | `app.rs:10518-10526` (helper), `1271-1290` (call sites) | In `diag()`, truncate the value galley to `available − label − 8` with hover for full value; or stack label-over-value when narrow |
| CS4 | Preflight card | Transform 4×4 matrix rows at mono 9.0. | Minor | `app.rs:1291-1306` | `mono_s()` 11 px; it's inside a disclosure, space exists |
| CS5 | Warnings | "NOTICE · {warning}" at 9.5 px gold; readiness strings "READY · CONTRACT WITHIN QUALIFIED ENVELOPE" / "READY WITH MODEL METADATA REVIEW" / "N BLOCKER(S)" as caps mono 10.0. | Minor | `app.rs:1328-1334`, `1512-1543` | Warn alert recipe (tint fill + sentence case); state word as `chip_text`, explanation as caption |
| CS6 | Center view | Horizontal caps stage ribbon "SOURCE → PREFLIGHT → … → EVIDENCE" ≈ 520 px; at min window (center ≈ 520 px) it exactly fills/overflows the column and wraps arrows onto the next line. | Minor | `app.rs:2118-2136` | Short-term: allow wrap gracefully / shrink tracking; roadmap Tier-3 vertical spine replaces it |
| CS7 | Center view | Case subtitle "{file} · source revision {id} · case revision {id}" — unwrapped mono 10.5 line clips at narrow widths; micro-font. | Minor | `app.rs:2141-2159` | Split into two caption lines or truncate ids further |
| CS8 | Empty state | `internal_flow_reference_card`: "EXECUTION BLOCKED · {blocker}" caps mono; disabled button "Compatible solver/model unavailable" is a button-shaped non-action. | Minor | `app.rs:10484-10502` | Warn text row + explicit non-interactive status line instead of a disabled button |

## 3. Results

| # | Region | Issue | Severity | Evidence | Fix direction |
|---|---|---|---|---|---|
| R1 | Right rail | **No ScrollArea; ~1100 px of content** (loads card 9 rows + comparison card + clipping/section controls + voxel-diagnostics card + 3 buttons): "Create Operating-Point Variant", "Export Surface Loads for FEA…", "Open Evidence & Provenance" are **clipped behind the status bar** at default window height. This is the founder's "bottom buttons clipped" bug, on the money screen. | Blocker | `app.rs:1617-1966` (buttons `1949-1965`) | ScrollArea + pin the primary export action; move voxel diagnostics into a disclosure |
| R2 | Loads card | `diag()` overlap (see CS3) is *guaranteed* here: "Force coefficients · derived" (~150 px) + "[0.00000, 0.00000, 0.00000]" (~200 px mono) = 350 px > 282 px content width → label and value overprint each other at the default rail width. Same for moment rows and "Cp · derived from recovered p". | Major | `app.rs:1701-1748`, `10518-10526` | Same as CS3; consider label-above-value measurement rows (§4.4 table) |
| R3 | Viewport | Decorative uncalibrated 40 px background grid still painted across Results/Metrics (PRD "ornamental grids" anti-pattern, audit A20) — including behind the 2D section view. | Major | `app.rs:6511-6531` | Delete, or replace with a labeled, domain-unit reference in section view only |
| R4 | Rail header | `chip_text("SUPPORTED FIXED-BODY CONTRACT · MODEL-DERIVED LOADS")` ≈ 330 px caps mono wraps mid-token in the 282 px rail. | Minor | `app.rs:1688-1700` | Shorten token to "SUPPORTED CONTRACT" + caption sentence for the rest |
| R5 | Rail footer | "SPATIAL ERROR UNAVAILABLE · attach an exact solver reference…" mono 9.0 caps gold — micro + caps straggler. | Minor | `app.rs:1921-1928` | Warn alert recipe, caption size, sentence case (keep wording) |
| R6 | Section card | Caps mono 9.0 rows "VIEW +X · +Y right · +Z up", "GEOMETRY · stored diffuse CAD mask". | Minor | `app.rs:1902-1918` | `mono_s()` + sentence case; keep SCI labels verbatim |
| R7 | Viewport overlays | Camera chip (top-left) and hint line (bottom-center) remain two separately-styled floating fragments; hint text can collide with the section legend at short viewport heights; chip uses its own SURFACE+border recipe. | Major | `app.rs:6641-6658`, `6663-6679` | One overlay chip recipe; move the hint into the status-bar center slot when height < threshold |
| R8 | Section view header | Painter-drawn header block at fixed offsets: title 15 px prop, source mono 10, method 10.5; axis caption drawn at `panel.min.y − 8` can overlap the header block when the viewport is short. | Minor | `app.rs:6774-6799`, `6829-6849` | Use theme text styles; compute header height and clamp panel top below it |
| R9 | Empty states | "No engineering result" / "No current result" are bare labels + a button — no designed empty state (UX-AC-01). | Minor | `app.rs:1660-1678` | Level-1 card with one action, matching library empty state |

## 4. Evidence

| # | Region | Issue | Severity | Evidence | Fix direction |
|---|---|---|---|---|---|
| E1 | Lineage card | **Full 64-char SHA-256 and full UUID values in `diag()` rows** ("Geometry SHA-256", "Model SHA-256", "Immutable run"): ~480 px of mono in a 980 px column is fine at 1440, but at min window (center ≈ 540 px) value + label > width → overlapping text; no copy affordance, inconsistent with `short_hash` used everywhere else. | Major | `app.rs:2350-2382` | 12-char prefix + `…` + copy button + full value tooltip (§4.5); fix `diag()` overlap globally |
| E2 | Scientific labels card | Five statements packed into one `\n`-joined mono label — no rows, no dividers, cramped. | Minor | `app.rs:2386-2394` | One row per statement (label + state), hairline dividers |
| E3 | Rail | Buttons "Export Surface Loads for FEA…", "Save Project" in Title Case; casing inconsistent with sentence-case rule. | Minor | `app.rs:2055-2067` | "Export surface loads for FEA…", "Save project" |
| E4 | Warnings | "NOTICE · {warning}" 10.0 px gold caps prefix. | Minor | `app.rs:2396-2402` | Warn recipe, caption size |

## 5. Model Library

| # | Region | Issue | Severity | Evidence | Fix direction |
|---|---|---|---|---|---|
| L1 | Toolbar | Single non-wrapping horizontal row: search (230) + segmented (~130) + up to 3 filter chips ("38 checkpoints — 9 need metadata review" alone ≈ 230 px) + right-aligned refresh. Below ~1250 px window width the chips **collide with / run under the refresh button**. | Major | `library.rs:506-597` | `horizontal_wrapped` for chips, or shorten the All-chip to "All · 38"; give the row a second line at narrow widths |
| L2 | Cards | Footer actions use `Layout::bottom_up` inside an auto-sized frame inside `ui.columns` cells: card height is driven by the cell's available height, risking cards that stretch to the scroll viewport bottom and/or footers overlapping opened "Details & provenance" content; `set_min_height(200)` + independent row heights → ragged rows. **[re-verify live]** | Major | `library.rs:690-696`, `821-874`, `654-669` | Fixed card anatomy: content column + explicit footer row (no bottom_up); wrap grid via measured row height |
| L3 | Card facts / hover | `support_summary` emits **"hUNKNOWN"** when horizon is undeclared ("2D · 64² grid · 2 → 2 ch · hUNKNOWN") — shown in the Contract fact and the active-chip hover; the unit test even asserts this string. | Minor | `library.rs:199-206`, test `993` | "h ?" → "horizon UNKNOWN" (match `contract_line`'s wording) |
| L4 | Card title row | 34-char humanized title + right-aligned status chip in a ~268 px card content width → chip and title collide at min card width (1-col layouts). | Minor | `library.rs:697-719` | `Label::truncate()` on the title; chip keeps priority |
| L5 | Ember budget | Three ember elements at once: Import button (header), "◆ ACTIVE" chip, active-card edge marker. | Minor | `library.rs:468-478`, `844`, `876-887` | Chip → `chip_text` in TEXT with the edge marker carrying the accent |
| L6 | Grid | No bottom breathing: last card row touches the panel bottom; rows separated by 12 px but columns by egui default. | Minor | `library.rs:654-669` | 24 px bottom margin inside the ScrollArea; equalize gaps to `s-3` |
| L7 | Refresh | Only icon-only control in the app — renders as tofu/fallback letter if the phosphor issue (C3/C4) recurs; no text fallback. | Minor | `library.rs:583-596` | Keep, but confirm icon font after rebuild; tooltip exists (good) |

## 6. Settings

| # | Region | Issue | Severity | Evidence | Fix direction |
|---|---|---|---|---|---|
| S1 | Bottom actions | **"Save settings" / "Restore defaults" drawn via `bottom_up` *after* a `ScrollArea` with `auto_shrink(false)`** — the scroll area consumes all remaining height, so the button row is laid over the last scroll rows and clips behind the status bar (Settings screenshot: half-height ember sliver at bottom-left). | Blocker | `settings.rs:288-290` + `542-574`; Settings screenshot | Reserve a fixed 56 px action strip *before* the ScrollArea (`ScrollArea::max_height(available − 56)`), buttons in it |
| S2 | Path fields | Fixed `.desired_width(250.0)` on Python executable, Research checkout, Project directory — real paths render as "/Users/hamza/Documents/Pioneer R…" (founder-confirmed). | Blocker | `settings.rs:323-327`, `343-347`, `366-370` | `desired_width(f32::INFINITY)` within the control column; middle-elide display + full path tooltip |
| S3 | Rows | `setting_row`: fixed 360 px label column + right-to-left control, no minimum gap and no vertical row padding — cramped rows (founder), and at min window the control column overlaps the helper text. | Major | `settings.rs:683-692`; Settings screenshot | Row inner padding `s-3`; label column = `min(360, available − control_min − 16)`; wrap helper text |
| S4 | Right rail | `runtime_fact("ENGINE", engine_status, SUCCESS)` — **engine state is always green**, even when the status string is "○ engine unavailable…". Status-by-wrong-color; honesty bug. | Major | `settings.rs:611` | Pass the real `engine_ok` color (SUCCESS/GOLD) like the status bar does (`app.rs:2769`) |
| S5 | Theme row | Theme preview's first swatch is BG_1 on a SURFACE card with hairline stroke — invisible; preview reads as two floating white/orange chips (visible in Settings screenshot). | Minor | `settings.rs:695-714` | Draw swatches on a BG_0 well with a 1px OUTLINE border |
| S6 | Layout | No content max-width/gutters: sections stretch the full center width (~810 px+) while every other screen centers at 920–1040 — header alignment and line lengths inconsistent. | Minor | `settings.rs:277-290` | Use `content_column(ui, 1040, …)` like the library |
| S7 | Signing section | Raw caps mono state strings ("READY · ED25519", "NOT CONFIGURED", "OFF") at 11.0 with `.strong()`, fingerprint at 9.5, key reference 10.5 — token drift + micro-fonts. | Minor | `settings.rs:422-472`, `448-455`, `525-537` | `chip_text` for states; `mono_s()` floor for ids |
| S8 | Right rail | Settings-file path wraps mid-path in mono 10.0 inside the card. | Minor | `settings.rs:643-660` | Middle-elide + copy on click |

## 7. Chrome / shell / status bar

| # | Region | Issue | Severity | Evidence | Fix direction |
|---|---|---|---|---|---|
| C1 | Titlebar | **Pale artifact strip behind the traffic lights**: an opaque neutral-gray native titlebar (~28 px) with two dark "ghost pill" remnants sits *above* the app's dark top bar — the fullsize-content-view is set but the titlebar itself is not transparent, so the system paints its own bar in the system appearance over/above the content. Founder's #1 chrome bug; reproduced live. | Blocker | `main.rs:63-65`; `qa/02`; Settings screenshot | Also set titlebar transparency (winit `with_titlebar_transparent(true)` via eframe's `window_builder` hook) and keep `with_title_shown(false)`; then the 44 px bar must own the traffic-light strip (inset content, not a separate band) |
| C2 | Top bar | In the running build the 44 px band renders **empty** — no "Reyn Studio" brand, no project chip, no Run button — although `top_bar` draws them; strongly suggests the live binary predates the Tier-2 top bar. **[re-verify after rebuild]** | Blocker (if it persists) | `app.rs:2643-2751` vs `qa/02` | Rebuild + recapture; if still empty, check that `Panel::top` runs before the CentralPanel and the frame fill isn't drawn over children |
| C3 | Icons (global) | **Nav/save/open icons render as fallback letters** (ṁ, #, ⇒, ÿ, Š, f) in the running binary — the founder's icon bug. Source-side, `fonts.rs` registers Phosphor correctly for Proportional (and `Cargo.lock` pins a single `egui-phosphor 0.13.0`), so this is most likely a stale binary built before the phosphor font landed. **[re-verify after rebuild; if it persists, check for a second phosphor TTF or a failed `set_fonts`]** | Blocker (if it persists) | `qa/04`; Settings screenshot; `fonts.rs:56-58`; `Cargo.lock:721-723` | Rebuild, recapture nav rail; add a debug assert/log that "phosphor" is in the active font atlas at startup |
| C4 | Icon plumbing | Phosphor is **not in the Monospace fallback chain** (only Proportional): any `ph::` glyph composed into `mono`, `mono_s`, `mono_chip`, or `TextStyle::Monospace` text will tofu. No current call site does this, but it is one refactor away (status-bar chips are mono). | Major | `fonts.rs:63-89` | Append "phosphor" to the Monospace family and the `FAMILY_MONO_MEDIUM` chain |
| C5 | Panel borders | Top bar, status bar, sidebar, and right rail all stroke **all four edges** (`Frame::stroke`), doubling 1 px borders at every panel junction and drawing a hairline against the window edge — the "one border everywhere" look the overhaul was meant to kill. | Major | `app.rs:2621-2626`, `2761-2766`, `2822-2827`, `2950-2955` | Per-edge hairlines only (paint a single `line_segment` on the meeting edge), not full-frame strokes |
| C6 | Status bar | Engine status label has no truncation; long error strings ("○ engine io: Connection refused (os error 61) …") will run under the right-aligned run/model cluster. Busy text is EMBER (accent spent on a passive status). | Major | `app.rs:2768-2808` | Truncate-with-tooltip at `available − right_cluster − 24`; busy indicator in WARN/GOLD, ember reserved for actions |
| C7 | Central panel | Hardcoded fill `#0e0a07` ≠ `BG_VIEWPORT` token (`#0E0C0A`) and it backs **every** screen — Library/Settings/Projects sit on viewport-black instead of `BG_0` (#151210), flattening the surface ladder. | Major | `app.rs:6493` vs `theme.rs:12-14` | Fill per-screen: `BG_VIEWPORT` for 3D/2D screens, `BG` otherwise; use the token |
| C8 | Crossfade | Screen-switch fade multiplies opacity of egui content only; the wgpu 3D pass doesn't fade (comment admits it), so entering/leaving Results pops the scene at full brightness beneath fading UI. During the 180 ms window semi-transparent panels also let the black central fill bleed through card fills (candidate mechanism for "washed-out band" artifacts during nav). | Major | `app.rs:6496-6510` | Fade a `BG`-colored overlay rect *over* the content instead of multiplying content opacity; skip fade for the wgpu screens or fade the 3D pass via its own uniform |
| C9 | Stray purple rectangle (Settings screenshot, right edge) | Not reproduced in the live window-ID capture, and no purple exists in the app's palette or GPU clear colors (passes clear to black — `gpu.rs:1402,1671`). In the founder's screenshot the rectangle sits at/beyond the app window's right boundary — consistent with a **background window peeking past the app edge**, not an app-drawn rect. Keep open until a window-ID (not full-screen) capture of Settings confirms. | Minor (tracking) | Settings screenshot; `qa/01` (absent); `gpu.rs:1402` | Re-verify with `screencapture -l <windowID>` on the Settings screen; if it *is* in-window, suspect the egui_wgpu paint-callback rect on panel-resize frames |
| C10 | Traffic lights | Content inset is 64 px vs the 78 px spec; once C1 is fixed and the bar merges with the titlebar strip, the brand text will crowd the lights. | Minor | `app.rs:2647` | 78 px inset when not fullscreen |
| C11 | Nav rows | Row heights 38/50 px and chip height 26 px sit off the 4/8 grid and off spec (nav row 32, list row 28); blocked stage rows use `Sense::hover` → not keyboard-focusable, so the focus-ring code path can never fire for them. | Minor | `app.rs:10247-10256`, `library.rs:278` | Snap heights to the grid; keep blocked rows focusable (`Sense::click` + ignore activation) so the reason is reachable by keyboard |
| C12 | Window title | Title text hidden but still set every change for Mission Control — correct; however dirty-state appears in three places (title •, top-bar chip dot, sidebar chip). | Minor | `app.rs:2606-2617`, `2674-2677`, `2842-2851` | One canonical dirty indicator (chip dot) + title dot |

## 8. Sandbox screens (Procedural 3D, Flow Painter, Fields 2D, Benchmark Lab)

| # | Region | Issue | Severity | Evidence | Fix direction |
|---|---|---|---|---|---|
| X1 | All sandbox rails | Same systemic no-scroll bug: `controls_painter` (~850 px), `controls_bench`, `controls_2d`, and the default "3D Controls" rail have **no ScrollArea**; `bottom_up` CTAs ("GENERATE FLOW", "Export calculations…") overlap card content at min height. Only 3 ScrollAreas exist in all of `app.rs` (case/evidence/project center views). | Major | `app.rs:7124-7335`, `7919+`, `8737+`, `2994-3117` | ScrollArea per rail + pinned bottom action strip (one shared helper) |
| X2 | Buttons | Caps button labels: "APPLY LERAY PROJECTION", "GENERATE FLOW", "RUN FULL SUITE", "RUNNING…", "ENGINE UNAVAILABLE", "MODEL UNAVAILABLE", "INSPECTING CELL…". | Minor | `app.rs:7267-7277`, `7323-7334`, `8037-8047` | Sentence case per §3.8 |
| X3 | `action_button` states | The helper has **no disabled state**: "ENGINE UNAVAILABLE" renders as a normal quiet button with hover/press animation and a click sense — a dead control that animates (UX-AC-01 violation). Also its `layout_no_wrap` label can spill outside the button when `width` < text width. | Major | `app.rs:10405-10459` (no `enabled` param; galley `10444-10457`) | Add `enabled: bool` (dim + no hover/press + reason tooltip); clamp/elide the label galley to the rect |
| X4 | Fields 2D | Loading state is a bare "predicting…" center string; empty is "no field" — undesigned states on a 3-panel screen. | Minor | `app.rs:6866-6879` | Skeleton panels (library already has the shimmer pattern) |
| X5 | Painter/bench canvases | Painter-drawn micro/caps text: canvas chips mono 11, captions 12.5, bench provenance/energy panels draw 9–10 px caps blocks; "■ STORED CAD MASK · solid section" mono 9.5. | Minor | `app.rs:7093-7121`, `6842-6849`, `9353-9654` | `mono_s()` floor; sentence case; keep SCI wording |
| X6 | Bench rail | Bench combo `selected_text` colored BRAND 12.5 — salmon text straggler on an input. | Minor | `app.rs:7942-7949` | TEXT color; BRAND is wordmark-only |

## 9. Cross-cutting (counts once each, all screens inherit)

| # | Issue | Severity | Evidence | Fix direction |
|---|---|---|---|---|
| G1 | **`diag()` has no overflow handling** — root cause of R2/CS3/E1 overlap family. | Major | `app.rs:10518-10526` | Fix once in the helper (truncate value, tooltip full) |
| G2 | **79 call sites** set font sizes ≤ 10.5 px across `app.rs`/`settings.rs` (8.5–10.5), below the 11.5 caption floor of §3.1. | Major (systemic) | `rg "\.size\((8|9|10)"` → 79 hits | Sweep to the named text styles; delete raw `.size()` calls |
| G3 | Content max-widths inconsistent per screen: 920/976 (Projects), 980/1048 (Case/Evidence), 1040 (Library), none (Settings). | Minor | `app.rs:3399-3408`, `2078-2087`, `library.rs:446`, `settings.rs:277` | One `content_column(1040)` everywhere |
| G4 | Margin gutters cast to `i8` with magic clamps (99/93) to dodge overflow — fragile and undocumented off-grid numbers. | Minor | `app.rs:3399`, `2078`, `2325` | Use `content_column` (f32 math) instead of `Margin` gutters |
| G5 | Min-window (1100×700) is not survivable: center column drops to ~520 px (CS6/CS7 clip), Settings rows overlap (S3), Evidence hashes overlap (E1), rails clip harder (CS1/R1/X1). | Major | computed from `main.rs:58` + panel sizes `app.rs:2820-2821`, `2947-2949` | After the fixes above, do a dedicated 1100×700 pass; consider raising min width to 1200 until then |

---

## Top 10 by visual impact

1. **Opaque gray titlebar strip + ghost title pills above the dark top bar** — every screen, first thing seen (C1; `main.rs:63-65`).
2. **Icons render as fallback letters (ṁ/#/⇒/ÿ/Š/f) across nav and buttons** in the running build (C3; `qa/04` — re-verify after rebuild, plumbing gap C4 regardless).
3. **Right-rail panels don't scroll → primary buttons unreachable/clipped**: Run analysis (Case), all three Results actions, sandbox CTAs (CS1, R1, X1).
4. **Settings Save/Restore row overlaps content and clips behind the status bar** (S1; `settings.rs:288-290,542-574`).
5. **Projects hero: ember "Import Geometry…" CTA clipped under the right panel**, plus chip/pill clipping (P1/P2; `app.rs:3427-3455`).
6. **Overlapping label/value text in measurement rows** on Results/Case rails and Evidence hashes (G1/R2/CS3/E1; `app.rs:10518-10526`).
7. **Truncated 250 px path fields in Settings** (S2; `settings.rs:325,345,368`).
8. **Empty 44 px top bar** in the running build — no brand, no project chip, no Run button (C2 — re-verify).
9. **Micro-font epidemic**: 79 sites at 8.5–10.5 px, worst on Projects run/evidence rows at 8.5 px red (G2/P4).
10. **Decorative 40 px grid + mismatched hardcoded canvas fill behind every screen** (R3/C7; `app.rs:6511-6531`, `6493`).

## Issue count by severity

- **Blocker: 7** (P1, CS1, R1, S1, S2, C1, and C2/C3 counted as one pending-rebuild blocker pair — 5 confirmed-from-source + 2 live-observed [re-verify])
- **Major: 21** (P2, P3, P4, P6, CS2, CS3, R2, R3, R7, E1, L1, L2, S3, S4, C4, C5, C6, C7, C8, X1, X3, plus systemic G1/G2/G5 → tracked in §9)
- **Minor: 26** (P5, P7, P8, P9, CS4–CS8, R4–R6, R8, R9, E2–E4, L3–L7, S5–S8, C9–C12, X2, X4–X6, G3, G4)

*(Where an item is a family — e.g. G1 overlap — screens cite it once; the counts above deduplicate.)*

## Notes for the implementation agent

- The running app instance (PID 56908, launched 14:42) was **not** killed by this audit — it predates the audit session and may hold founder state; the on-disk release binary is newer (14:56). Re-run `qa` captures against a fresh launch before closing C2/C3.
- Highest-leverage single fixes: (1) a shared "rail = ScrollArea + pinned action strip" helper kills 5 blockers/majors; (2) fixing `diag()` kills the whole overlap family; (3) titlebar transparency + top-bar merge kills the worst chrome artifact.
