# Reyn Studio — Design Overhaul: Investigation & Redesign Roadmap

**Status:** research report + actionable roadmap (no code changed)
**Date:** 2026-07-24
**Scope:** UI/UX of the native shell (`src/`), grounded in `PRD.md` §3 (premium scientific-instrument direction), REQ-UX-01/UX-AC-01, REQ-UX-02/UX-AC-02, and the SCI-AC-* scientific-semantics rules.
**Primary evidence:** annotated screenshot of the Model Library screen (current build), full read of `src/theme.rs`, `src/fonts.rs`, `src/icons.rs`, `src/library.rs`, `src/settings.rs`, structural read of `src/app.rs` (10.5k lines), the four local design skills, and a source-level teardown of Rerun's `re_ui` design system (sparse clone of `rerun-io/rerun@main`, 2026-07-24).

Everything in this document respects the PRD's hard rules: no fake state, no decorative sci-fi, no persistent Support CTA, explicit evidence semantics, local-first, status never by color alone. The goal is refinement into **"premium scientific instrument"** — not a generic SaaS look.

---

## 1. Executive summary

**What's wrong.** The app is honest, dense, and structurally sound, but it *reads* as a debug build of itself: one font weight for every character on screen (the bundled Inter/JetBrains Mono are variable TTFs, and egui renders only their Regular default instance, so `.strong()` changes color, never weight), ALL-CAPS 9.5–11px monospace labels doing the work a type scale should do, every surface boxed by the same 1px brown border at three different corner radii, and a status system where full-saturation red validation panels, red REJECTED tags, an orange active-nav fill, an orange primary CTA, and gold warnings all shout simultaneously — the screenshot's Model Library shows the same `runtime.missing_setting` string rendered twice in two adjacent red boxes while three "Model Library" titles are visible at once (window title, center header, right-panel header) above an in-app File/Edit/View/Window row that duplicates the macOS menu bar. Navigation mixes a lifecycle (Project → Case Setup → Results → Evidence) with destinations (Model Library, Settings) under one "ENGINEERING WORKFLOW" label, sprinkles a context-less "VOXEL DIAGNOSTICS" card into global nav, and silently re-routes clicks on Results to Case Setup when no result exists. There is no motion system (every state change snaps), no elevation system (everything is flat + bordered), no keyboard surface beyond ⌘N/O/S, and the muddy low-contrast brown surface ladder flattens what hierarchy remains.

**What award-level means for Reyn.** The instruments users compare Reyn against — Linear, Raycast, Figma, Rerun, Shapr3D (Apple Design Award 2020), DaVinci Resolve — win praise through *discipline*, not decoration: one accent color spent only on the next primary action; a real type scale (3–4 sizes, 2–3 true weights, tight tracking) instead of caps-lock hierarchy; tonal surface steps and one soft shadow level instead of borders-on-everything; 120–250 ms ease-out motion that confirms state changes and never delays work; plain-language first with expert detail one disclosure away; and a keyboard-first command surface. Rerun proves every one of these is achievable in egui specifically — its `re_ui` crate is a full design-token system (numbered gray ramp, semantic alert recipes at ~20% alpha, 24px list rows, one 50px-blur popup shadow, animated toggles and collapses) built by egui's own creator, and it is the single most important reference for this overhaul. For Reyn, "award-level" concretely means: the warm ember-on-dark identity is kept but recalibrated (near-neutral warm surfaces, salmon text replaced by warm off-white, ember reserved for exactly one primary action per screen); scientific states become calm, named, and glanceable instead of alarming; the shell collapses to a single chrome; and every loading/empty/error/stale state is designed to the same standard as the happy path — which is precisely what UX-AC-01 already demands.

---

## 2. Severity-ranked UX audit

Legend: **Blocker** = incompatible with "award-level / truly premium"; **Major** = visibly cheapens or confuses; **Minor** = polish debt. Every issue lists evidence (file:line) and a fix direction. Screens: **ML** = Model Library, **Shell** = chrome/nav/panels, **All** = global.

| # | Screen / region | Issue | Severity | Evidence | Fix direction |
|---|---|---|---|---|---|
| A1 | All | **No true typographic hierarchy.** Bundled `Inter.ttf` and `JetBrainsMono.ttf` are *variable* fonts (wght 100–900); egui has no variable-axis support, so everything renders at Regular 400. `.strong()` in egui only shifts color. Titles, body, buttons differ only by px size (9.5→30) and caps. | Blocker | `fonts.rs:5-29`; `assets/` (fvar axes verified); `caps()` `app.rs:635-640` | Ship static instances (Inter Medium + SemiBold, JBMono Medium) as named `FontFamily`s; define a real `text_styles` scale; kill blanket caps (§3.1, sketch §5.3.1) |
| A2 | Shell | **Triple header + double chrome.** macOS title bar ("Unsaved project — Reyn Studio") + in-app brand row with File/Edit/View/Window/Analysis menus + per-screen H1 + right-panel H1 repeat the same context. On ML, "Model Library" is on screen 3×. | Blocker | `main.rs:54-62`; `app.rs:2470-2672` (menus), `library.rs:197,478-488` | Single chrome: hide native titlebar text, fullsize content view, move menus to the real macOS menu bar (`muda`), one screen title owned by the content region (§4.1, sketch §5.3.5) |
| A3 | ML (also Case gates) | **Alarm soup / duplicated red.** The same validation message renders twice (notice box + STRUCTURED VALIDATION panel), both `DATA_RED` fill + `DATA_RED` 1px border + red caps title; plus red REJECTED tags on cards, orange IMPORT CTA, orange active nav, gold UNKNOWNs — 5+ hot elements compete. Violates "restrained… color is never the only carrier" spirit of PRD §3.1 and reads as emergency. | Blocker | `library.rs:491-533` (both boxes), `49-63` (status colors); screenshot | One calm alert recipe (semantic tint at 10–14% alpha fill, hairline same hue at ~30%, icon + sentence-case text, full color only on the icon/keyword); deduplicate message sources; red reserved for destructive/failed-gate (§3.3, §4.6) |
| A4 | ML right rail | **Junk-drawer panel.** One 330px rail stacks: duplicate H1, error console, FIND search, dimension filter, active-checkpoint card, Refresh, and a shouting all-caps `IMPORT CHECKPOINT…` ember block — unrelated concerns, no ownership, two scroll-less bottom-anchored buttons. | Blocker | `library.rs:471-621`; `app.rs:2857-2866` (330px fixed) | Dissolve: search/filter become a toolbar row above the card grid; validation feedback moves inline to the import flow; active checkpoint becomes a compact header chip; import becomes the screen's single primary action (§4.6) |
| A5 | ML cards | **Developer jargon as user-facing copy.** `runtime.missing_setting · train_args.stride is required for local execution` shown raw, twice, with severity/field only in hover tooltips. No plain-language line, no recovery action. | Blocker | `library.rs:523-530`, `engine.rs` validation structs; screenshot | Two-layer copy: human sentence first ("This checkpoint can't run yet — its training config is missing `stride`.") + "Show technical detail" disclosure revealing exact codes (SCI semantics preserved verbatim inside) (§3.8, §4.6) |
| A6 | Shell nav | **Workflow and destinations conflated; nav lies about state.** Project/Case Setup/Results/Evidence (a lifecycle) + Model Library/Settings (destinations) sit under one "ENGINEERING WORKFLOW" eyebrow; clicking Results with no result silently redirects to Case Setup; stages show no per-stage state (ready/stale/blocked). | Blocker | `app.rs:2730-2759` (one group), `2738-2749` (silent redirect) | Two groups: "Workflow" rail with per-stage state glyphs (●/◐/○ + label) and disabled-with-reason stages; "Library/Settings" separated below; never silently re-route (§4.1) |
| A7 | All | **Boxes-in-boxes, one border to rule them all.** `card()` = SURFACE fill + 1px OUTLINE_VARIANT + r3; nav card, notice frames, chips all the same recipe; radii mixed 2/3/4 px non-concentrically (seg r2 inside container r3 with 3px margin). No elevation levels at all. | Major | `app.rs:10243-10253` (card), `10019-10032` (seg), `2652-2668`; `library.rs` frames | Elevation ladder (§3.4): flat groups divided by full-span hairlines; 1 card level (tonal + inner top-light); 1 overlay level (shadow 0/15/50); concentric radius rule (outer = inner + gap) |
| A8 | All | **Muddy brown surface ladder + salmon text.** BG `#1c110b` → SURFACE `#291d16` → HIGH `#34272 0` are near-identical in value; TEXT `#f5ded3` is a saturated peach, so *text itself* carries the brown cast; TEXT_MUTE `#a78b7d` on SURFACE_HIGH ≈ 4.6:1, borderline at 10-11px sizes it's used at. | Major | `theme.rs:5-22` | Recalibrated warm-neutral ladder with bigger value steps and desaturated text (§3.3); contrast targets ≥4.5:1 body, ≥3:1 large/secondary (ui-ux-pro-max §1) |
| A9 | ML cards | **Metadata dump without progressive disclosure.** Raw filename as title (`direct_v1_latest`), 5-row fact grid where `Regime UNKNOWN`, `Role UNKNOWN` get equal weight with Size/Modified; `2D · 64² grid · 2 → 2 ch · h16` contract string unexplained; rejected cards visually identical in structure/weight to healthy ones; `Set active` disabled with no visible reason. | Major | `library.rs:261-467`, `136-154`, `425-459`; screenshot | Card = name + health + one-line contract summary in plain words; facts behind the existing disclosure; UNKNOWNs aggregated ("3 fields undeclared — legacy checkpoint"); rejected cards visually recede (dimmed, red only on the status chip); disabled buttons always carry inline reason (§4.6) |
| A10 | All | **Zero motion; no pressed/focus states.** All state changes snap; `action_button` hover = `gamma_multiply(1.12)` color jump, no press feedback, no focus ring (keyboard focus invisible); collapsing headers use egui defaults; panel switches hard-cut. | Major | `app.rs:10161-10198`, `10108-10130`; absence of any `animate_*` call in `src/` | Motion spec §3.7 (120–220ms ease-out, `ctx.animate_bool_with_time_and_easing`), pressed = 1px shrink + darken ≤160ms, visible focus ring (ember 60%), crossfade+8px slide on panel change; respect reduced-motion (Settings) |
| A11 | Shell viewport | **Three floating fragments, three styles.** Camera chip (mono 12), engine pill (mono 11.5, floats over content top-right, overlaps scroll content on ML/Settings), hint line bottom-center (proportional 12.5) — all separately styled surfaces. | Major | `app.rs:6543-6613`, pill drawn on non-analysis screens too (`6574`) | One status bar region in the shell (bottom hairline strip or top-bar right cluster), consistent chip recipe; viewport-only overlays limited to camera + hints (§4.1) |
| A12 | Shell nav | **"VOXEL DIAGNOSTICS" context leakage.** Global nav shows helicity/enstrophy/Q of whatever particle field happens to be loaded — on Model Library and Settings too; numbers are real but unattributed to any visible object (honesty-adjacent risk under SCI-AC-01: no source label). | Major | `app.rs:2792-2846`, `diagnostics()` `612-626` | Move to case/results context (Results right rail or viewport inspector); label source explicitly; nav rail carries only navigation + project identity (§4.1) |
| A13 | ML, landing | **Competing primary CTAs.** Ember is spent on: sidebar "New External-Flow Analysis", active nav item fill, "IMPORT CHECKPOINT…", "Run analysis", landing "Import Geometry…" — several visible at once. One primary action per screen (ui-ux-pro-max §4 `primary-action`; PRD: "Ember marks the next primary action"). | Major | `app.rs:2716-2727`, `10108-10130`; `library.rs:598-613`; `app.rs:3360-3371` | Ember = exactly one button per screen. Active nav uses a 2px ember *edge marker* + text color, not a filled orange row; secondary buttons become quiet/tonal (§3.3, §4.1) |
| A14 | Shell | **Rigid geometry.** Left 276 / right 330 fixed, non-resizable; content max-width 920–980 leaves dead gutters at 1440 default; ML grid is `columns(2)` with `min_height 210` — cards stretch/pad to fit, uneven bottom edges, no reflow at narrow widths (min window 1100 → center 494px). | Major | `app.rs:2676,2859`, `library.rs:258-273,3322` | Resizable panels with sane min/max + persisted widths; card grid via `egui_flex`/manual wrap with 280–360px min card width; density review at min window (§4.6, §5.2) |
| A15 | All | **Loading/empty states are text-only; no skeletons or determinate progress.** "Waiting for the engine inventory.", filtered-empty single line, `VALIDATING…` label swap; import validation has no progress affordance; first-run landing depends on engine with no designed degraded state on ML. | Major | `library.rs:590-596,249-255,602-607`; `app.rs:3296+` | Designed state set per screen (loading skeleton rows ≈ final layout, empty with one action, error with recovery, read-only) — skeleton shimmer sketch §5.3; states are already a PRD requirement (UX-AC-01 "all changed loading/error/stale/read-only states are implemented") |
| A16 | Shell footer | **"Docs" opens the PRD via `file://`.** Product chrome linking a repo markdown file is developer scaffolding in the product shell. | Major | `app.rs:2848-2853` | Replace with Help menu (native menu bar) → user docs/About; keep PRD out of product UI |
| A17 | All | **Icon set is hand-drawn and uneven.** 12 icons, stroke widths 1.2/1.3/1.6/2.1 mixed in one set; naive metaphors (Heart, Book); no filled/outline discipline; several nav concepts share icons (Chart for Results *and* Benchmark). | Minor | `icons.rs:21-158` | Adopt Phosphor Regular via `egui-phosphor` (one stroke voice, 1000+ glyphs); keep bespoke drawing only for viewport/data glyphs (§3.6) |
| A18 | All | **Casing and punctuation inconsistency.** "Set active" vs "IMPORT CHECKPOINT…" vs "Refresh inventory" vs "◉ SANDBOX LIVE"; `·` separators everywhere including inside sentences; ellipsis usage varies. | Minor | `library.rs:605`, `app.rs:2638`, various | Voice rules §3.8: sentence case for all interactive text; caps only for §3.1 overline style; `·` reserved for mono data strings |
| A19 | ML header | **Debug-counter phrasing.** "0 contract OK · 9 metadata review · 38 total" + "engine ready · mps" read as logs; "0 OK" is alarming with no explanation or next step. | Minor | `library.rs:211-221`; `app.rs` engine status strings | Human phrasing + glyphs: "38 checkpoints — 9 need metadata review"; "Engine ready · Apple GPU (MPS)"; counters become filter chips (§4.6) |
| A20 | Results viewport | **Always-on 40px background grid** under Results/Metrics. Decorative, uncalibrated (no units/scale), risks PRD "ornamental grids" anti-pattern. | Minor | `app.rs:6423-6443` | Remove, or make it a real calibrated reference (labeled spacing tied to domain units) shown only in section view |
| A21 | Shell | **Title command spam.** `ViewportCommand::Title` sent every frame from `top_bar`; dirty state also shown in sidebar caps — duplicated status. | Minor | `app.rs:2473-2478` | Send title only on change; one canonical dirty indicator (title bar dot) |
| A22 | All | **Unconditional 60Hz+ repaint.** `ctx.request_repaint()` every frame regardless of activity — burns battery, and removes the headroom that would make animation cheap. | Minor (perf-of-feel) | `app.rs:607` | Event-driven repaint + `request_repaint_after` while animating/streaming only (§5.4) |
| A23 | Settings | Structure is actually good (section + row + helper text), but rows use the same flat recipe, theme names ("Instrument Dark") appear without preview, and destructive "Revoke key" arming is text-only. | Minor | `settings.rs:267+` | Keep structure; apply new tokens; theme preview swatch; destructive confirm follows §3.3 danger recipe |
| A24 | Positive findings (keep) | ●/○ glyphs pair with color (not color-only); UNKNOWN is honest and explicit; no fake data anywhere; no Support CTA; inline delete confirmation; disabled-delete hover reason exists; empty library state has one clear action; settings helper text. | — | `library.rs`, `app.rs`, `settings.rs` | Preserve these behaviors through the redesign — they are PRD compliance already working |

**Root-cause reading.** A1 (no weights), A7 (no elevation), A8 (mud), A3/A13 (color spent everywhere) are four systems failures that *every* screen inherits. Fixing the four systems fixes ~70% of the visual complaints; A2/A4/A6 are the three structural IA failures that fix most of the UX complaints.

---

## 3. Reyn design language v2 — "calm instrument"

Identity kept: warm-dark, ember accent, mono for measurement, evidence-first density. Everything below is tokens + rules, directly implementable in `theme.rs`/`fonts.rs`.

### 3.1 Typography

Fonts: **Inter** (UI) + **JetBrains Mono** (data) stay — the PRD mandates them and Rerun ships Inter as its only UI font (`re_ui/data/Inter-Medium.otf`), which settles the "is Inter premium enough?" question for this category. (The high-end-visual-design skill bans Inter for *marketing sites*; for instrument UI the differentiator is weight/tracking discipline, not a trendier family. Conflict noted and resolved in favor of the PRD.)

Ship **static instances**, not the variable files (egui renders only the default instance — audit A1): `Inter-Regular`, `Inter-Medium`, `Inter-SemiBold`, `JetBrainsMono-Regular`, `JetBrainsMono-Medium`. Register Medium/SemiBold as named `FontFamily`s (sketch §5.3.1).

| Token | Font / weight | Size / line-height | Tracking | Casing | Use |
|---|---|---|---|---|---|
| `display` | Inter SemiBold | 22 / 28 | −0.2px | Sentence | Screen title (one per screen, content-owned) |
| `title` | Inter SemiBold | 16 / 22 | −0.1px | Sentence | Card/panel titles, dialog titles |
| `body-strong` | Inter Medium | 13 / 18 | 0 | Sentence | Emphasis, buttons, nav labels, table headers |
| `body` | Inter Regular | 13 / 18 | 0 | Sentence | Default UI text (egui `TextStyle::Body`) |
| `caption` | Inter Regular | 11.5 / 16 | 0 | Sentence | Helper text, timestamps prose |
| `overline` | Inter Medium | 10.5 / 14 | **+0.8px** | UPPERCASE | Section eyebrows ONLY — the *single* sanctioned caps style; ≤1 per group |
| `mono` | JBMono Regular | 12.5 / 18 | 0 | as-is | Measurements, units, IDs, hashes, timestamps, tabular numbers |
| `mono-s` | JBMono Regular | 11 / 15 | 0 | as-is | Dense evidence detail, hash suffixes |
| `mono-chip` | JBMono Medium | 10.5 / 14 | +0.5px | UPPERCASE | Scientific state tokens only: `MODEL`, `RECOVERED`, `UNKNOWN`, `UNSIGNED`… |

Rules:
- **Kill blanket ALL-CAPS.** `caps()` (`app.rs:635`) currently uppercases every section label at 10px; only `overline` and `mono-chip` may be caps, and `mono-chip` is reserved for SCI-AC-* state tokens so that caps *means* "scientific state", never "I couldn't think of hierarchy". Rerun's equivalent: one 11px section-header size (`list_header_font_size()`), everything else sentence case.
- Filenames/IDs are **data**, not titles: model cards get the human name in `title` and the filename in `mono-s` beneath (with full value on hover — truncation per ui-ux-pro-max `truncation-strategy`).
- Numbers inside prose = Inter; numbers in columns/chips = mono (design-taste-frontend Rule: mono for data, VISUAL_DENSITY 8+ pattern).
- Line-height and tracking are settable per text in egui 0.35 (`RichText::line_height`, `RichText::extra_letter_spacing` — verified against egui 0.35 source), so the scale above is fully implementable.
- Retina note: 13px Inter Medium at 2× renders crisply; Rerun uses 12px default — Reyn sits one step larger because measurement-heavy screens are read, not scanned.

### 3.2 Spacing & layout grid

4px base grid, 8px rhythm (both skills and Rerun agree; Rerun: `item_spacing 8`, `view_padding 12`).

| Token | Value | Use |
|---|---|---|
| `s-1` | 4 | icon↔label, chip padding-y |
| `s-2` | 8 | item spacing (egui default kept), chip padding-x |
| `s-3` | 12 | panel inner padding, card gaps |
| `s-4` | 16 | card padding, section internal |
| `s-5` | 24 | between sections |
| `s-6` | 32 | screen top/bottom breathing |
| `s-7` | 40 | hero/landing rhythm (PRD's "40px layout rhythm") |

Heights: top bar **44** (traffic-light aligned, single chrome), nav row **32**, list row **28**, button **28** (primary **36**), input **28**, status bar **24**. Hit areas stay ≥ 24px on desktop with pointer (44pt rule is touch; desktop instrument density follows Rerun's 24px rows).
Panels: left rail 240 (resizable 200–300), right inspector 320 (resizable 280–420, collapsible), content max-width 1040 centered with `s-6` gutters.

### 3.3 Color system v2

Keep the amber-on-dark identity; fix the mud by (a) desaturating *surfaces* toward warm-neutral, (b) desaturating *text* to warm off-white, (c) spending chroma only on ember + semantic + data colors. All values are sRGB hex; contrast figures vs their intended background.

**Surface ladder (warm neutral, hue ≈ 25°, chroma low):**

| Token | Hex | Use |
|---|---|---|
| `bg-viewport` | `#0E0C0A` | 3D/2D scientific canvas (darkest — data glows here, PRD-sanctioned bloom only) |
| `bg-0` | `#151210` | app canvas |
| `bg-1` | `#1C1916` | rails, top bar, status bar |
| `bg-2` | `#242019` | cards, inputs (level-1 elevation) |
| `bg-3` | `#2D2822` | hover, raised, menus base |
| `bg-4` | `#37312A` | overlays, active fills |
| `hairline` | `#3B342C` | separators, quiet borders (≈1.6:1 vs bg-1 — visible, not loud) |
| `outline` | `#574C41` | interactive borders, focus-adjacent |

**Text (on bg-1/bg-2):**

| Token | Hex | Contrast | Use |
|---|---|---|---|
| `text-primary` | `#F1ECE6` | ≈13:1 | titles, values |
| `text-secondary` | `#C6BCB1` | ≈8:1 | body, labels |
| `text-tertiary` | `#8F8478` | ≈4.6:1 | captions, placeholders (≥11.5px only) |

**Accent (spent, not sprayed):**

| Token | Hex | Rule |
|---|---|---|
| `ember` | `#FF7A1A` | **Exactly one primary action per screen** + running-state indicator. Never a nav fill, never a panel border, never body text. |
| `ember-dim` | ember @ 60% | focus ring (2px, offset 1px), active-nav edge marker (2px bar) |
| `brand` | `#FFB68E` | wordmark + rare brand moments only (not data, not labels) |

**Semantic status (used sparingly, never color-only — every status pairs glyph + word):**

| Token | Hex | Fill recipe | Meaning |
|---|---|---|---|
| `ok` | `#3FBF8A` | text/icon full; fill @ 12%; hairline @ 30% | pass **with named proposition** (SCI-AC-03) |
| `warn` | `#E3A93C` | same recipe | UNKNOWN / metadata gaps / stale |
| `danger` | `#E5544B` | same recipe | failed gate, destructive actions, REJECTED |
| `info` | `#8ACEFF` | same recipe | neutral notices; also remains the tertiary *data* blue |

This is Rerun's alert recipe verbatim in spirit (`alert_*` in `dark_theme.ron`: fill = color at alpha ~50/255, icon full color, text at a readable tint) — the single highest-leverage fix for the "alarm soup" screenshot. Full-saturation fills are allowed in exactly two places: the ember primary button and the danger *confirm* button.

**Data colors** (viewport/plots, PRD §3.1 meanings unchanged): gold `#F7BE1D` secondary data, blue `#8ACEFF` tertiary data, red reserved for data extrema semantics (peak loads) — never mixed with status usage in the same view without labels.

### 3.4 Elevation & borders

Replace "border everything" with a 4-level system (high-end-visual-design "double-bezel" translated to tonal steps; design-taste-frontend "anti-card-overuse"):

- **Level 0 — flat group.** No box. Related rows separated by full-span `hairline` dividers (Rerun's `full_span_separator` pattern). Default for settings rows, fact lists, evidence tables.
- **Level 1 — card.** `bg-2` fill, **no outer border**; instead a 1px inner top-edge highlight (white @ 4%) painted along the top, radius `r-2`. Cards read as raised by tone, not by outline. Optional 3% vertical luminance gradient (mesh) for surfaces ≥ 200px tall.
- **Level 2 — raised / hover.** `bg-3`, shadow `(0, 6, blur 16, black 25%)`.
- **Level 3 — overlay (menus, popovers, dialogs).** `bg-3/4`, shadow `(0, 15, blur 50, black 50%)`, **no stroke** — exactly Rerun's popup shadow (`design_tokens.rs:735-745`), which is why their menus look native-quality.
- Hairline borders remain for: inputs (focus swaps to ember-dim), tables, and the *outer* window edge. Strokes are always 1 physical px (align to pixel grid via `painter.round_to_pixel`-style snapping).

### 3.5 Radius tokens

| Token | px | Use |
|---|---|---|
| `r-1` | 4 | buttons, inputs, chips, segmented items |
| `r-2` | 6 | cards, menus, popovers (Rerun: small 4 / normal 6) |
| `r-3` | 10 | native window corner (custom-frame mode; Rerun uses 10) |
| Concentric rule | inner = outer − gap | e.g. segmented thumb r-1=4 inside container r-2=6 with 2px inset — kills the current 2-inside-3 mismatch (A7) |

### 3.6 Iconography

- Adopt **Phosphor Regular** via `egui-phosphor 0.13` as the single UI icon voice (consistent stroke, 1,500+ glyphs, ships as font — crisp at any DPI). Nav, buttons, status glyphs at 16px against 13px text; 14px in dense lists (Rerun `small_icon_size: 14`).
- One weight level per hierarchy: Regular everywhere; Fill variant only for the *active* nav item and status dots (filled = current/asserted, outline = available — ui-ux-pro-max "filled vs outline discipline").
- Keep hand-painted vectors (`icons.rs`) only where they encode domain meaning the icon font lacks (e.g., orbit/slice glyphs inside the viewport HUD).
- Every icon-only control has a tooltip + accessible label; nav always icon **+ text** (already true — keep).
- Status glyph vocabulary (paired with words, never alone): `●` running/live, `◐` partial/stale, `○` idle/unavailable, `✓` named pass, `!` warning, `×` failed gate.

### 3.7 Motion spec

Emil Kowalski's decision framework (emil-design-eng skill), adapted to an immediate-mode 120Hz-capable shell. Purpose first: motion communicates state change or continuity; it never delays scientific work (PRD §3.1) and never runs on keyboard-invoked actions (skill: "never animate keyboard-initiated actions").

| Event | Duration | Easing | What animates |
|---|---|---|---|
| Hover in | 120ms | ease-out (`cubic_out`) | fill lerp bg→bg+1, text secondary→primary |
| Hover out | 80ms | ease-out | reverse (exit faster than enter) |
| Press | 90ms | ease-out | fill darken + content inset 1px (scale ≈0.97 equivalent) |
| Selection / segmented thumb | 160ms | ease-out | thumb x-position, text color crossfade |
| Collapse/expand | 180ms | ease-out | openness (egui `animate_bool` retargets mid-flight = interruptible, as the skill demands) |
| Panel/screen content swap | 160–200ms | ease-out | opacity 0→1 + 8px translate-up; **no** scale-from-zero (skill: "never animate from scale(0)") |
| Modal / dialog | 200ms in / 140ms out | ease-out | fade + scale 0.98→1.0 from center |
| Toast/notice | 220ms | ease-out | slide from status-bar edge + fade; auto-dismiss 4s non-error |
| Skeleton shimmer | 1100ms loop | linear | gradient sweep on placeholder blocks |
| Running indicator | 900ms loop | linear | ember dot pulse ±15% alpha (state, not decoration) |
| Reduced motion (Settings toggle + system) | 0ms transforms | — | keep ≤120ms opacity fades only |

Implementation: `ctx.animate_bool_with_time_and_easing` / `ctx.animate_value_with_time` + `emath::easing::cubic_out` (all verified in egui 0.35); `style.animation_time = 0.16` as the global default; repaint scheduling per §5.4. Numbers themselves never tween (a count-up `Cd` would be fake state — SCI-AC risk); value *containers* may fade in.

### 3.8 Voice & microcopy

1. **Plain language first, jargon one level down.** Pattern for every engine/validation message: `<Human sentence naming the object and consequence>` + disclosure `Technical detail` → verbatim `code · field · severity` (SCI-AC-01 keeps exact check names; they move *into* the disclosure, not off the screen). Example (screenshot case): "This checkpoint can't run locally — its training configuration doesn't declare `stride`. Import a checkpoint exported with training args, or re-export this one." ▸ `runtime.missing_setting · train_args.stride · error`.
2. **Every error names a recovery** (retry, fix path, or "why this is blocked") — ui-ux-pro-max `error-clarity`/`error-recovery`.
3. Sentence case everywhere interactive; `IMPORT CHECKPOINT…` → "Import checkpoint…". Caps = §3.1 rules only.
4. Counts read as sentences with the loudest fact first: "38 checkpoints · 9 need metadata review" (chip-filter tappable), not "0 contract OK · 9 metadata review · 38 total".
5. Status strings drop dev shorthand: "Engine ready · Apple GPU (MPS)", "Predicting · 64³ grid", "Run complete · 24 steps · Re 300" (mono for the measured parts).
6. UNKNOWN stays UNKNOWN (never guessed, SCI-AC-03) but is *explained once per surface*: "UNKNOWN — this checkpoint predates declared metadata" instead of three naked UNKNOWN rows.
7. No marketing filler words in-product (design-taste-frontend "no filler"): the UI never says premium/seamless/powerful; quality is shown, not claimed.

---

## 4. Screen-by-screen redesign direction

### 4.1 Shell: single chrome, honest nav, one status home

**Window chrome (kills A2).**
- `ViewportBuilder::with_fullsize_content_view(true).with_titlebar_shown(true).with_title_shown(false)` (APIs verified in egui 0.35): traffic lights stay native, title text disappears, content extends under the bar. Top bar becomes a **44px** `bg-1` strip: traffic-light inset (~78px), then project name + dirty dot (the *only* place project name appears in chrome), centered nothing, right side = engine chip + run action (contextual).
- **Native macOS menu bar** via `muda 0.19` (File/Edit/View/Analysis/Window/Help with proper ⌘ shortcuts and macOS conventions) — the in-app menu row is deleted (`app.rs:2497-2616`). Help menu hosts docs/about/diagnostics per PRD §3.2 (no Support CTA in chrome).
- Optional, founder sign-off required: `window-vibrancy 0.8` `NSVisualEffectMaterial::Sidebar` behind the left rail only. Strictly a native-material choice, not glassmorphism-as-decoration; if in doubt against PRD "no decorative sci-fi / glassmorphism", skip — the tonal ladder works without it.

**Left rail (kills A6, A12, A13).** 240px, `bg-1`, resizable:
1. Project identity block (name `title`, path + schema `mono-s`, state chip).
2. `overline` "Workflow" → **stage list with real state**: Project ▸ Case Setup ▸ Results ▸ Evidence, each row = icon + label + right-aligned state glyph (`✓` complete, `◐` stale, `○` empty, `!` blocked-with-reason tooltip). Disabled stages are visibly disabled with a reason line — clicking Results with no result explains "No completed run yet — run the case first" *in place* instead of silently redirecting (A6). Active row: `bg-3` fill + 2px ember left edge + `body-strong` text (no more orange slab).
3. `overline` "Library" → Model Library, Settings (separated group — destinations, not stages).
4. Research Sandbox group (Developer setting, unchanged behavior, `warn`-tinted eyebrow keeps its "not engineering evidence" tag).
5. Bottom: nothing but the collapse handle. Voxel diagnostics leave the rail (→ Results inspector, A12); Docs leaves for the Help menu (A16).
- Primary CTA ("New external-flow analysis") lives on the **Project screen and empty states**, not pinned above nav on every screen — ember appears once per screen (A13).

**Status home (kills A11).** A 24px bottom **status bar** (`bg-1`, top hairline): left = engine state chip (`● Engine ready · Apple GPU (MPS)` / `○ Engine unavailable — read-only review`, glyph+word+color), center = long-operation progress (determinate when known), right = active run/model shorthand in `mono-s`. The floating engine pill disappears; the viewport keeps only the camera chip + interaction hint, restyled to one recipe.

### 4.2 Projects / landing (first-run experience)

Current: 25px "Projects" + hero card + notice + document card + recents/recovery lists (`app.rs:3296+`); functional, visually flat, hero competes with sidebar CTA.
Direction:
- **First-run (no recents):** a deliberate two-zone landing — left: `display` "Start an analysis", one ember button ("Import geometry (STL)…"), caption naming the contract honestly ("Fixed-body external flow · STL import and preprocessing" — Stage-0 CAD language per PRD §8). Right: quiet "Open project…" + drag-drop target (level-1 card, dashed hairline). Below: `overline` "How Reyn works" → three *text* steps (Source → Run → Evidence) in one flat group — no icon-card row (banned "3 equal cards" pattern, design-taste-frontend).
- **Returning:** recents as level-0 rows (name `body-strong`, path `mono-s`, opened-when caption, state chip), hover `bg-3`, no per-row boxes. Recovery entries get a `warn` recipe row group with one-click restore + discard. Current-project card keeps its save state chip; "SAVED LOCALLY ✓" phrasing stays (honest local-first signal, LOCAL-AC-01).
- Engine unavailable at launch → landing still fully works (project ops don't need the engine, N6-PROJ-01); a status-bar chip explains read-only compute, per §4.1.

### 4.3 Case Setup

Current: 9.5px mono caps stage stepper with `→` glyphs, 28px case title, preflight fact-pill wall, gate cards (`app.rs:2027-2270`, `1141-1580`).
Direction:
- **Stepper becomes the spine:** left-aligned vertical stage list (Source · Preflight · Contract · Discretization · Operating point · Run) with per-stage state glyph + one-line status, current stage expanded, others collapsed (progressive disclosure, REQ-UX-02). The horizontal caps ribbon goes away.
- Preflight facts stop being uniform pills: **gate verdict first** (`ok`/`warn`/`danger` recipe with the named check — "Watertight ✓ · 0 boundary edges"), then a facts table (label `body`, value `mono`) in a level-0 group. Waivers render as `warn` rows with the named waiver + who/when (existing data, better clothes).
- Blocked run states: the Run button (ember, the screen's one primary action) is disabled with the blocking reason listed directly beneath it — never a dead control (UX-AC-01).
- Unit/transform confirmations become a focused dialog (level-3 overlay, 200ms in) with the exact 4×4 transform in `mono` behind a disclosure — consequential confirmations stay explicit (PRD §3.3.5).

### 4.4 Results

Current: viewport + right-rail stack of sliders/checkboxes + gold "EXPORT CALCULATIONS"; results numbers live in a sidebar card (`app.rs:2857-3040`, `1583-1920`).
Direction:
- **Applicability banner first** (PRD §7): a slim strip above the viewport naming model support status for *this* case ("Within declared envelope · h16 horizon ✓" or `warn` "No applicability envelope declared — UNKNOWN"), sourced from existing contract data. Then numbers, then pictures.
- **Quantities panel:** force/moment coefficients and hotspots as a proper measurement table — label `body`, value `mono` right-aligned, unit column, source chip (`MODEL` / `RECOVERED` mono-chip per N5X-EV-02) on every row. The current `diag()` key-value pairs upgrade into this. Cp range keeps its "recovered pressure" honesty (N5X-PHYS-01) with the nondimensionalization note one disclosure away.
- Viewport controls (slicing, isosurface, opacity, shadows) collapse into an **inspector accordion** with per-group overlines; sliders get the redesigned quiet look (trailing fill ember-dim, value in `mono` right-aligned, stable width). "Normalized recovered pressure" checkbox keeps its precise label + tooltip (already correct — keep).
- Voxel diagnostics (helicity/enstrophy/Q/count) return here, labeled with source ("From active run field · MODEL") — fixing A12's honesty gap.
- Export = one quiet button ("Export…") opening a small menu (FEA loads CSV, calculations, report) — the gold caps slab retires; gold returns to *data* duty only.

### 4.5 Evidence

Current: run lineage cards + hashes + availability states (`app.rs:1968-2352`).
Direction: this is the screen that should feel most like a *ledger*.
- Level-0 table rows: run ID (`mono-s`), created (UTC, `mono-s`), stages, integrity state chip, signature state chip. `BUNDLED · VERIFIED` / `UNSIGNED` chips keep their exact wording (SCI-AC-02: integrity ≠ authenticity) with the §3.3 recipes (ok/warn) instead of raw green/gold text.
- Hash values: 12-char prefix + `…` + copy button + full value tooltip (truncation strategy). Every evidence row deep-links to its immutable run (N6-COMP-01 pattern).
- Missing-dependency read-only mode gets a designed banner (info recipe): "Read-only review — engine unavailable. Stored fields and evidence remain inspectable." (exact behavior already exists; give it the calm dress.)

### 4.6 Model Library (the screenshot, in detail)

Target: from "error console with cards" to "instrument inventory".

**Layout.** Right rail dissolves (A4). The screen becomes: header row → toolbar → card grid, full width (max 1040):
1. **Header:** `display` "Model Library" + caption (one line, kept) — the *only* title (A2). Right-aligned: active checkpoint chip (`◆ flow3d_obs_v1 · 3D · 32³` in `mono-s`, click → scrolls to card) and "Import checkpoint…" as the screen's single ember button (A13).
2. **Toolbar:** search field (placeholder "Filter by name or regime…"), dimension segmented control (All · 2D · 3D — redesigned control §5.3.4), health filter chips ("9 need review", "1 rejected") that encode the old counter as *actionable* filters (A19), refresh icon-button with tooltip.
3. **Card grid:** wrap layout, min card width 300, gap `s-3` (A14).

**Card anatomy (kills A9):**
- Row 1: health dot + human name (`title`, e.g. "Direct v1 — latest") with filename `mono-s` beneath (`direct_v1_latest.pth`, hover = full path); right: status chip — `Rejected` (danger recipe), `Needs review` (warn), `Ready` (ok, names its proposition on hover: "declared contract validates against engine requirements").
- Row 2: plain-language contract line: "2D velocity field · 64² grid · 2→2 channels · horizon 16" — same data as `support_summary()`, human-ordered, `body`; raw form available in the disclosure.
- Row 3 (conditional): **one** guidance line replacing the red `status_detail` dump: "Can't run locally — training config missing `stride`." (danger *text*, no filled box) + "Details" disclosure → verbatim structured validation (code/field/severity in `mono-s`) — SCI semantics intact, jargon one level down (A5).
- Facts (Regime/Epoch/Role/Size/Modified) move **inside** the existing "Applicability, limitations & report hashes" disclosure, which becomes "Details & provenance"; UNKNOWN rows aggregate: "3 fields undeclared (regime, role, envelope) — legacy checkpoint" (warn text once, not three shouting rows).
- Footer: "Set active" (quiet button; disabled state always carries inline reason: "Rejected checkpoints can't be activated"), overflow menu (Delete inside, with the existing inline confirm restyled to the danger recipe).
- Rejected cards render *dimmed* (text-secondary, dot+chip only red element) — failure recedes, health pops (A3).
- Active card: 2px ember edge + `◆ Active` chip — not a brighter fill.

**States:** loading = 4 skeleton cards (shimmer §3.7); empty = current copy (already good) restyled level-1; import-validating = determinate-feel progress row pinned under toolbar ("Validating checkpoint contract…" + spinner ≤300ms rule then skeleton); import-rejected = toast (danger recipe) + the rejected card appears with its guidance line — **one** message, one place (kills the duplicate red boxes).

### 4.7 Settings

Keep the section/row/helper structure (`settings.rs` — best-structured screen today). Apply: level-0 rows with hairline dividers per section, `overline` section eyebrows, inputs restyled (28px, `bg-2`, focus ring), theme picker gains 3-swatch preview per mode, "Reduced motion" toggle added under Appearance (feeds §3.7), signing-key state chips use §3.3 recipes with SCI wording untouched, revoke flow = danger recipe + typed confirm. Research Sandbox toggle keeps its explicit consequence text.

---

## 5. egui implementation playbook (mapped to this codebase)

### 5.1 File-by-file

| File | Change |
|---|---|
| `assets/` | Add static font instances: `Inter-Regular.ttf`, `Inter-Medium.ttf`, `Inter-SemiBold.ttf`, `JetBrainsMono-Regular.ttf`, `JetBrainsMono-Medium.ttf` (replace the two variable files; ~5 files, subset if size matters) |
| `src/fonts.rs` | Register 5 fonts; named families `"inter-medium"`, `"inter-semibold"`, `"mono-medium"`; keep Proportional=Regular, Monospace=JBMono-Regular defaults; add `egui_phosphor::add_to_fonts` |
| `src/theme.rs` | Becomes the token module: surface ladder, text triad, semantic recipes (§3.3), radius consts, spacing consts, `text_styles` map (§5.3.1), shadow tokens, `style.animation_time = 0.16`, scroll-bar 6px + fade, `visuals.popup_shadow`/`window_shadow` (§3.4 L3), focus ring via `visuals.selection` + `widgets.*` per-state table (model: Rerun `design_tokens.rs::set_colors/set_spacing` — strokes off buttons, `hovered.expansion 2.0`) |
| `src/app.rs` helpers | `caps()` → `overline()` (tracked, Medium, only styling — call sites reviewed one by one); `card()` → elevation-aware (§5.3.2); `nav_row()` → state-glyph + ember edge + animated hover (§5.3.3); `action_button()` → variants (primary/quiet/danger) with press/focus states; `seg()` → animated segmented control (§5.3.4); `diag()` → measurement-table row (label body / value mono / unit / source chip); delete in-app menus after `muda` lands |
| `src/app.rs` shell | `top_bar` 44px traffic-light-aware; status bar panel (new, 24px bottom); left rail regroup (§4.1); remove `request_repaint()`-always (§5.4); remove per-frame Title command (A21) |
| `src/library.rs` | Screen restructure per §4.6; `show_controls` deleted (rail dissolved); message dedup (one owner: notice OR inline card line) |
| `src/settings.rs` | Token adoption + reduced-motion setting + theme previews |
| `src/main.rs` | ViewportBuilder chrome flags (§5.3.5); vibrancy behind a build flag if approved |
| `src/icons.rs` | Shrinks to viewport/domain glyphs; UI icons come from Phosphor constants |

### 5.2 Crates to add (versions verified on crates.io, 2026-07-24)

| Crate | Version | Purpose | Note |
|---|---|---|---|
| `egui-phosphor` | 0.13.0 | coherent icon set (Phosphor) as font | supports egui 0.35; `Variant::Regular` + `Fill` for active states |
| `muda` | 0.19.3 | native macOS menu bar (kills double chrome) | tauri-maintained; works with winit window handles |
| `egui_flex` | 0.7.0 | flex/wrap layout for the card grid & toolbars | from `hello_egui` (lucasmerlin); mature tier |
| `egui_inbox` | 0.12.0 | cleaner engine→UI message delivery with repaint wakeups | replaces manual `try_recv` loop + always-repaint |
| `window-vibrancy` | 0.8.0 | optional `NSVisualEffectMaterial::Sidebar` | **only with founder sign-off**; PRD glassmorphism caution |
| `egui_animation` | 0.12.0 | optional easing/transition helpers beyond `ctx.animate_*` | small; can skip if §5.3 patterns suffice |
| *(reference only)* `catppuccin-egui` | 5.7.0 | study its `Style→Visuals` mapping as a token-application template — do **not** adopt its palette | |
| *(reference only)* `egui_tiles` | 0.16.0 / `egui_dock` 0.20.1 | only if dockable layouts become a requirement post-v1 | Rerun uses `egui_tiles` |

Already present and sufficient: `eframe/egui 0.35` (current stable, June 2026), `rfd 0.17` (native dialogs).

### 5.3 Five highest-impact code sketches

Sketches use real egui 0.35 APIs (verified against the 0.35 source) and this codebase's names.

**1) Type scale via `text_styles` + true weights (`fonts.rs`, `theme.rs`)**

```rust
// fonts.rs
use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

pub const FAMILY_MEDIUM: &str = "inter-medium";
pub const FAMILY_SEMIBOLD: &str = "inter-semibold";

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    for (key, bytes) in [
        ("inter", &include_bytes!("../assets/Inter-Regular.ttf")[..]),
        ("inter-medium", &include_bytes!("../assets/Inter-Medium.ttf")[..]),
        ("inter-semibold", &include_bytes!("../assets/Inter-SemiBold.ttf")[..]),
        ("jbmono", &include_bytes!("../assets/JetBrainsMono-Regular.ttf")[..]),
    ] {
        fonts.font_data.insert(key.into(), Arc::new(FontData::from_static(bytes)));
    }
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "inter".into());
    fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "jbmono".into());
    fonts.families.insert(FontFamily::Name(FAMILY_MEDIUM.into()), vec!["inter-medium".into(), "inter".into()]);
    fonts.families.insert(FontFamily::Name(FAMILY_SEMIBOLD.into()), vec!["inter-semibold".into(), "inter".into()]);
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
}

// theme.rs — real named styles instead of ad-hoc .size() calls
pub fn display() -> egui::TextStyle { egui::TextStyle::Name("display".into()) }
pub fn title()   -> egui::TextStyle { egui::TextStyle::Name("title".into()) }
pub fn overline()-> egui::TextStyle { egui::TextStyle::Name("overline".into()) }

pub fn apply(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        use egui::{FontId, FontFamily::*, TextStyle::*};
        style.text_styles = [
            (Name("display".into()),  FontId::new(22.0, Name(crate::fonts::FAMILY_SEMIBOLD.into()))),
            (Name("title".into()),    FontId::new(16.0, Name(crate::fonts::FAMILY_SEMIBOLD.into()))),
            (Name("overline".into()), FontId::new(10.5, Name(crate::fonts::FAMILY_MEDIUM.into()))),
            (Body,      FontId::new(13.0, Proportional)),
            (Button,    FontId::new(13.0, Name(crate::fonts::FAMILY_MEDIUM.into()))),
            (Small,     FontId::new(11.5, Proportional)),
            (Monospace, FontId::new(12.5, Monospace)),
            (Heading,   FontId::new(16.0, Name(crate::fonts::FAMILY_SEMIBOLD.into()))),
        ].into();
        style.animation_time = 0.16;
    });
}

// call site — an overline eyebrow with tracked caps (the ONLY caps style):
pub fn overline_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .text_style(crate::theme::overline())
            .extra_letter_spacing(0.8)   // egui 0.35: real tracking
            .color(TEXT_TERTIARY),
    );
}
```

**2) Elevation: card with inner top-light + soft shadow + optional gradient mesh (`app.rs::card()` v2)**

```rust
fn card<R>(ui: &mut egui::Ui, level: u8, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let (fill, shadow) = match level {
        2 => (BG3, egui::epaint::Shadow { offset: [0, 6], blur: 16, spread: 0,
                                          color: egui::Color32::from_black_alpha(64) }),
        _ => (BG2, egui::epaint::Shadow::NONE),
    };
    let frame = egui::Frame::new()
        .fill(fill)
        .corner_radius(egui::CornerRadius::same(R2))   // r-2 = 6
        .inner_margin(egui::Margin::same(16))
        .shadow(shadow);                                // Frame carries shadows in 0.35
    let response = frame.show(ui, |ui| { ui.set_width(ui.available_width()); add(ui) });

    // 1px inner top-edge highlight: reads as machined elevation without a border
    let rect = response.response.rect;
    let painter = ui.painter();
    let y = rect.top() + 0.5; // hairline on the physical pixel grid
    painter.line_segment(
        [egui::pos2(rect.left() + R2 as f32, y), egui::pos2(rect.right() - R2 as f32, y)],
        egui::Stroke::new(1.0, egui::Color32::from_white_alpha(10)),
    );
    // Optional: vertical luminance gradient for tall surfaces (vertex-colored mesh)
    if rect.height() > 200.0 {
        let mut mesh = egui::Mesh::default();
        let top = fill.gamma_multiply(1.04);
        let bottom = fill.gamma_multiply(0.98);
        let r = rect.shrink(1.0);
        mesh.colored_vertex(r.left_top(), top);
        mesh.colored_vertex(r.right_top(), top);
        mesh.colored_vertex(r.right_bottom(), bottom);
        mesh.colored_vertex(r.left_bottom(), bottom);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        // paint beneath content: use ui.painter_at with layer ordering, or paint before frame content
    }
    response.inner
}
```

**3) Animated hover/press for interactive rows (`nav_row` v2 — pattern for all custom widgets)**

```rust
fn nav_row(ui: &mut egui::Ui, icon: &str, label: &str, state: StageState, active: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), egui::Sense::click());
    // 120ms ease-out in, retargetable mid-flight (interruptible per motion spec)
    let t_hover = ui.ctx().animate_bool_with_time_and_easing(
        resp.id.with("hover"), resp.hovered(), 0.12, emath::easing::cubic_out);
    let t_press = ui.ctx().animate_bool_with_time_and_easing(
        resp.id.with("press"), resp.is_pointer_button_down_on(), 0.09, emath::easing::cubic_out);

    let bg = if active { BG3 } else { BG1.lerp_to_gamma(BG3, t_hover) };
    let inset = 1.0 * t_press; // press = 1px content inset ≈ scale(0.97) feel
    let p = ui.painter();
    p.rect_filled(rect.shrink(inset), egui::CornerRadius::same(R1), bg);
    if active { // 2px ember edge marker — accent as a mark, not a slab
        p.rect_filled(egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 6.0),
                      egui::vec2(2.0, rect.height() - 12.0)), 1.0, EMBER);
    }
    if resp.has_focus() { // keyboard focus is always visible
        p.rect_stroke(rect.expand(1.0), egui::CornerRadius::same(R1),
                      egui::Stroke::new(2.0, EMBER.gamma_multiply(0.6)), egui::StrokeKind::Outside);
    }
    let fg = TEXT_SECONDARY.lerp_to_gamma(TEXT_PRIMARY, t_hover.max(active as u8 as f32));
    p.text(rect.min + egui::vec2(10.0, 16.0), egui::Align2::LEFT_CENTER, icon,
           egui::FontId::proportional(16.0), fg);          // phosphor glyph
    p.text(rect.min + egui::vec2(34.0, 16.0), egui::Align2::LEFT_CENTER, label,
           egui::FontId::new(13.0, egui::FontFamily::Name(crate::fonts::FAMILY_MEDIUM.into())), fg);
    state.paint_glyph(p, rect); // right-aligned ✓ / ◐ / ○ / ! + tooltip reason
    if t_hover > 0.0 || t_press > 0.0 { ui.ctx().request_repaint(); } // only while animating
    resp.clicked()
}
```

**4) Segmented control redesign (`seg()` → animated thumb, concentric radii)**

```rust
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut egui::Ui, id: egui::Id, value: &mut T, options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    egui::Frame::new()
        .fill(BG2)
        .corner_radius(egui::CornerRadius::same(R2))          // outer r=6
        .inner_margin(egui::Margin::same(2))                  // gap=2 → thumb r=4 (concentric)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let selected_index = options.iter().position(|(v, _)| v == value).unwrap_or(0);
            // animate thumb x across item slots — 160ms ease-out
            let t = ui.ctx().animate_value_with_time(id, selected_index as f32, 0.16);
            let slot_w = 92.0_f32;
            let origin = ui.cursor().min;
            let thumb = egui::Rect::from_min_size(
                origin + egui::vec2(t * slot_w, 0.0), egui::vec2(slot_w, 24.0));
            ui.painter().rect_filled(thumb, egui::CornerRadius::same(R1), BG4);
            ui.painter().rect_stroke(thumb, egui::CornerRadius::same(R1),
                egui::Stroke::new(1.0, HAIRLINE), egui::StrokeKind::Inside);
            for (option, label) in options {
                let selected = value == option;
                let resp = ui.add_sized([slot_w, 24.0],
                    egui::Button::new(egui::RichText::new(*label)
                        .color(if selected { TEXT_PRIMARY } else { TEXT_SECONDARY }))
                    .fill(egui::Color32::TRANSPARENT).frame(false));
                if resp.clicked() && !selected { *value = *option; changed = true; }
            }
            if (t - selected_index as f32).abs() > f32::EPSILON { ui.ctx().request_repaint(); }
        });
    changed
}
// Labels: "All · 2D fields · 3D volumes" → "All", "2D", "3D" (sentence case, no caps)
```

**5) Single chrome: fullsize content view + native menus (+ optional vibrancy) (`main.rs`, `app.rs`)**

```rust
// main.rs
let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([1100.0, 700.0])
        .with_title("Reyn Studio")            // still used by Mission Control / Dock
        .with_fullsize_content_view(true)     // content under the (kept) traffic lights
        .with_titlebar_shown(true)
        .with_title_shown(false),             // no text: our 44px bar owns identity
    renderer: eframe::Renderer::Wgpu,
    ..Default::default()
};

// app::new(cc) — native menu bar replaces in-app File/Edit/View/Window row:
// muda 0.19: build Menu → File(New ⌘N, Open ⌘O, Recent, Save ⌘S, Save As ⇧⌘S, Import…, Export…)
// · Edit · View · Analysis (contextual enable) · Window · Help(Docs, About, Diagnostics)
// then menu.init_for_nsapp(); route MenuEvent::receiver() through the existing
// request_project_action / import / export handlers each frame (same pattern as engine.rx).

// top_bar(): height 44, left inset 78.0 for traffic lights when not fullscreen:
// ui.add_space(if fullscreen { 16.0 } else { 78.0 });
// drag region: respond to background drags with ViewportCommand::StartDrag.

// OPTIONAL (needs founder sign-off vs PRD glassmorphism rule) — sidebar vibrancy:
// window-vibrancy 0.8: apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, None)
// then left rail fill becomes translucent bg-1 (alpha ~0.85).
```

### 5.4 Performance-of-feel

- **Repaint discipline:** replace the unconditional `ctx.request_repaint()` (`app.rs:607`) with: repaint requests from `egui_inbox` when engine messages arrive; `request_repaint()` inside widgets only while their animation values are mid-flight (see sketches); `request_repaint_after(Duration::from_millis(120))` while a run is active for the pulse indicator. Result: idle app ≈ 0% GPU, and ProMotion 120Hz is automatically used during interaction/animation since egui paints on demand at display cadence.
- **No layout jitter:** changing numbers render in `mono` with fixed decimal formats (`{:>7.4}`) so widths are stable; status chips get `min_size` so text swaps ("Ready"→"Validating…") don't reflow neighbors; truncate long names with `Label::truncate()` + hover tooltip rather than wrapping cards taller.
- **Hairlines on the pixel grid:** draw 1px strokes at `.5` offsets (or `painter.round_to_pixel`) so they don't gray-blur on retina.
- **Skeletons > spinners** for >300ms loads (ui-ux-pro-max `progressive-loading`); shimmer via a time-based gradient mesh, capped to visible rect.
- **Text layout caching:** egui caches galleys per frame automatically; avoid re-formatting strings every frame (current status strings rebuild each frame — cache until change).

---

## 6. Prioritized roadmap

All items preserve: no fake state, no Support CTA, scientific semantics verbatim (SCI-AC-01..03), local-first (LOCAL-AC-01), complete state coverage (UX-AC-01), one-visible-next-action journeys (UX-AC-02). Each tier ships a visibly better app on its own.

### Tier 1 — Quick wins (≤1 day, ~70% of perceived lift)

| # | Change | Impact | PRD mapping |
|---|---|---|---|
| 1.1 | Static font instances + `text_styles` scale + `overline()` replacing `caps()` (§5.3.1) | Entire app gains real hierarchy instantly | REQ-UX-01 (hierarchy), UX-AC-01 unaffected |
| 1.2 | `theme.rs` token swap: surface ladder, text triad, semantic recipes (§3.3) — pure constant changes + `apply()` | Kills mud + salmon text + alarm soup base | REQ-UX-01; SCI colors keep meanings (§3.1 PRD) |
| 1.3 | Alert recipe applied to library notice/validation + message dedup (one owner) | The screenshot's worst region becomes calm | UX-AC-01 (error states designed); SCI-AC-01 (codes kept in disclosure) |
| 1.4 | Ember budget: active nav → edge marker; "EXPORT CALCULATIONS"/"IMPORT CHECKPOINT…" → sentence case, one ember per screen | Instant "designed, not alarmed" read | UX-AC-01 (no status-by-color-only; fewer competing signals) |
| 1.5 | Kill per-frame Title command; status strings humanized ("Engine ready · Apple GPU (MPS)") | Polish + honesty of tone | SCI-AC-03 phrasing intact |
| 1.6 | Popup/menu shadows (L3 recipe), scroll-bar 6px + fade, `animation_time 0.16` | Menus/popovers stop looking default | REQ-UX-01 |

### Tier 2 — Structural (2–5 days)

| # | Change | Impact | PRD mapping |
|---|---|---|---|
| 2.1 | Single chrome: fullsize content view + 44px top bar + `muda` native menus; delete in-app menu row (§5.3.5) | Biggest "native premium" jump; kills A2 | REQ-UX-01 (low-chrome native); UX-AC-01 |
| 2.2 | Left rail regroup: workflow stages with state glyphs + disabled-with-reason; Library/Settings group; voxel card → Results; Docs → Help menu | Nav stops lying; IA matches PRD journeys | REQ-UX-02 / UX-AC-02; N6-IA-01 wording preserved |
| 2.3 | Status bar (24px) as the single status home; retire floating engine pill | State always findable, never overlapping | UX-AC-01 (states designed), PERF-AC-01 visibility |
| 2.4 | Model Library restructure per §4.6 (rail dissolved, toolbar, card anatomy, filter chips, skeletons) | The audited screen reaches the new standard | N6-MODEL-01/02 semantics unchanged; UX-AC-02 |
| 2.5 | Motion pass: hover/press/focus on `nav_row`/`action_button`/rows (§5.3.3), panel crossfade, reduced-motion setting | The app starts *feeling* engineered | PRD §3.1 motion rules; UX-AC-01 |
| 2.6 | `egui_inbox` + repaint discipline (§5.4) | Battery + 120Hz headroom | REQ-PERF-01 / PERF-AC-01 |
| 2.7 | Phosphor icons adoption; `icons.rs` reduced to domain glyphs | Coherent icon voice | REQ-UX-01 |

### Tier 3 — Deep polish (1–2 weeks)

| # | Change | Impact | PRD mapping |
|---|---|---|---|
| 3.1 | Case Setup vertical stage spine + gate verdict blocks + waiver rows (§4.3) | J1 journey feels guided end-to-end | UX-AC-02; N5X-CAD-01..04 displays intact |
| 3.2 | Results: applicability banner, measurement table with source chips, inspector accordion, export menu (§4.4) | The money screen becomes defensible *and* beautiful | N5X-EV-02, N5X-PHYS-01, N5X-LOAD-03 |
| 3.3 | Evidence ledger restyle + copyable hashes + read-only banner (§4.5) | Evidence reads as the product's spine | SCI-AC-02, N6-PROJ-05/06 |
| 3.4 | Landing/first-run per §4.2 + designed engine-degraded states everywhere | First 5 minutes = instrument, not prototype | UX-AC-02, LOCAL-AC-01, N6-PROJ-01 |
| 3.5 | Command palette (⌘K: navigate stages, open recent, run case, import, toggle overlays — actions only, no fake content) | Keyboard-first credibility (Linear/Raycast pattern; Rerun ships one) | REQ-UX-02; no dead controls (UX-AC-01) |
| 3.6 | Elevation/gradient pass on viewport HUD + card L1 top-lights everywhere; hairline pixel-snapping | Final "machined" finish | REQ-UX-01 |
| 3.7 | Panel resize + persisted layout; card grid `egui_flex`; min-window density audit | Feels professional on every display | REQ-UX-01 |
| 3.8 | Optional (sign-off): sidebar vibrancy; custom-frame experiment with 10px window radius (Rerun's `native_window_corner_radius`) | Last-mile native luxury | Must pass PRD §3.2 review (no glassmorphism-as-decoration) |

**Acceptance guardrail for every tier:** re-run the UX-AC-01 review checklist (no Support CTA, no SaaS/sci-fi furniture, no fake state, no dead controls, no status-by-color-only, all loading/error/stale/read-only states implemented) plus SCI-AC-01..03 wording diffs on any copy change. Items 2.2/2.4 touch N6-IA-01/N6-MODEL-* surfaces — their acceptance wording ("first visible action", hidden research tools, UNKNOWN handling) is preserved by design above.

---

## Appendix A — What "award-level" actually is (Part B research)

### A.1 Awards evidence

- **Apple Design Awards** (apple.com; winners list verified via Wikipedia): the pro/creator tools that won did so for *discipline in complex domains* — **Shapr3D (2020)**: full CAD modeling made direct and legible on touch — proof that "serious engineering tool" and "design award" are compatible; **Procreate (2013)** and **Procreate Dreams (2024, Innovation)**: pro-grade density behind progressive disclosure; **Flighty (2023, Interaction)**: dense live operational data, calm hierarchy, plain language over jargon — the closest interaction analog to what Reyn's status system should feel like; **Affinity Designer (2015)**, **Pixelmator (2011)**, **Things (2009)**: restrained chrome, type-led hierarchy. Common thread: **one accent, real type scale, dense-but-grouped data, designed states, zero decoration**.
- **Red Dot / iF** both maintain interface & UX categories (red-dot.org "Brands & Communication Design — Interface & UX"; ifdesign.com "Communication/UX"). Winning software entries there are consistently judged on typographic clarity, consistency of the component system, and honest state communication — the same rubric as this report's §3.
- **What this means for Reyn:** the award bar is not visual spectacle; it is *"nothing in the frame is accidental."* Reyn's evidence-first honesty is already award-compatible content; the delivery (type, color budget, states, motion) is what's missing.

### A.2 Best-in-class pro tools — transferable patterns

| Tool | What to steal (and what not) |
|---|---|
| **Linear** (linear.app) | Type scale of ~3 sizes + 2 weights carries the whole app; keyboard-first with ⌘K command palette; density via 28–32px rows, not boxes; one accent used ~1×/screen; instant (non-animated) keyboard actions. Not: its marketing gradients. |
| **Raycast** (raycast.com) | Zero-animation open/close for 100×/day actions (Emil's frequency rule embodied); every item = icon + title + subtitle + right-aligned mono shortcut — a perfect list-row grammar for Reyn's runs/checkpoints. |
| **Arc** (arc.net) | Chrome minimization: the app *is* content + one rail; identity via one material and one accent. Confirmation that killing double-chrome is the premium move. |
| **Figma** (figma.com) | Inspector discipline: right panel is *properties of the selection*, never a junk drawer — the exact fix for A4; 11–13px UI type at high density with real weights. |
| **Rive** (rive.app) | Dark pro-tool surfaces: near-neutral darks with one saturated accent; state machines visualized calmly. |
| **Frame.io** (frame.io) | Review tool: media-first canvas, metadata in quiet mono, comments/status as tinted chips not filled panels. |
| **DaVinci Resolve** (blackmagicdesign.com/products/davinciresolve) | The reference "instrument": page-based workflow rail (Media→Edit→Color→Deliver ≈ Reyn's Source→Run→Evidence), flat dark surfaces, data-dense panels grouped by task, nearly zero chrome decoration. |
| **Blender 4.x** (blender.org/download/releases/4-0) | A community-driven overhaul proving an aging dense tool can modernize: flattened themes, consistent 1-value radius, icon set unification, type upsizing — a roadmap-shaped precedent for incremental overhaul. |
| **Shapr3D** (shapr3d.com) | Progressive disclosure in CAD: one visible next action, expert depth on demand — literally UX-AC-02's demand. |
| **Onshape / nTop / Luminary Cloud / SimScale** (onshape.com, ntop.com, luminarycloud.com, simscale.com) | Where simulation UX is going: setup as *guided gates* (Reyn already has the data for this), results as shareable evidence pages, restrained dark themes with one accent, plain-language solver states. Reyn's differentiator — evidence honesty — is ahead of them; its shell is behind. |
| **Zed / Lapce** (zed.dev, lapce.dev) | Rust-native proof that GPU-drawn custom UI can feel platform-grade: Zed's obsession with latency-as-design and hairline precision; both ship real type scales in custom toolkits. |

### A.3 Distilled: the 10 properties of an award-level scientific-instrument UI

1. One accent, spent on the single next action (everything else tonal).
2. 3–4 type sizes, 2–3 true weights, tracking on caps, mono for measurements.
3. Surfaces separated by value steps + one soft shadow level — not borders.
4. 4/8px spatial grid with a visible 24–40px rhythm.
5. Status = glyph + word + calm tint; red is rare and means it.
6. Every state designed (loading/empty/error/stale/read-only) — the unhappy paths look as intentional as the happy one.
7. Motion 120–250ms ease-out, interruptible, absent from keyboard-frequency actions.
8. Density grouped by task with full-span rows and disclosure — never a control wall.
9. Keyboard surface: palette + shortcuts + visible focus.
10. Microcopy in the domain's plain language; jargon preserved verbatim one level down.

---

## Appendix B — Rerun `re_ui` teardown (the egui existence proof)

Source: `github.com/rerun-io/rerun`, `crates/viewer/re_ui` (sparse-cloned @ main, 2026-07-24). Rerun's viewer is widely cited as the best-looking egui app in production; its CTO **Emil Ernerfeldt is egui's creator** (github.com/emilk — "creator of egui, CTO of rerun.io"), so `re_ui` is effectively *the reference implementation of "egui done right."* Everything below is verifiable in the linked files.

**Architecture.** Design tokens live in data files, not code: `data/color_table.ron` (numbered color ramps), `data/dark_theme.ron` / `light_theme.ron` (semantic aliases over the ramps), parsed by `src/design_tokens.rs` into a `DesignTokens` struct that writes egui `Style`/`Visuals`. Debug builds hot-reload tokens (`hot_reload_design_tokens.rs`). → Reyn's `theme.rs` should become exactly this shape (constants → semantic tokens → one `apply()`).

**Concrete numbers (dark theme):**
- **Gray ramp 0–1000**: surfaces `Gray.100` (panels/top bar), `150` (faint bg), `200` (tab bar/text-edit), separators `250`, widget fills/hovers `300`; text **three tiers** — subdued `550`, default `750`, strong `1000`. Text sits ≥400 steps above surfaces: that gap is the "expensive" look.
- **One accent**: `Blue.500` for selection; focus = `Blue.400` 1px outline + `Blue.350` 2px halo @ ~30% alpha. Nothing else in the chrome is saturated.
- **Alerts** (`alert_success/info/warning/error`): fill = semantic color @ alpha 50/255 (~20%), icon = full color, text = 900-tint of the hue. No full-red panels anywhere.
- **Buttons**: primary is *inverse* (light `Gray.800` fill, dark text) — not a hot color; secondary `Gray.300`; ghost transparent-until-hover; **no strokes on any button**; hover adds fill + `expansion: 2.0` (the widget physically grows 2px — geometry as motion).
- **Typography**: ONE default — Inter **Medium**, 12px, 16px line-height, −0.15px tracking (`Global.Typography.Default`), Heading 16px; welcome-screen-only display styles (41/27/15/13/10.5) registered as named `TextStyle`s. `set_fonts` ships `Inter-Medium.otf` as the only UI font.
- **Metrics**: list rows 24px, top bar 28px, panel title bars 24px, table rows 20 (dense) / 32 (spacious), view padding 12, item spacing 8, indent 14, scroll bar 6px with fade (strength 0.6), tooltip width 600.
- **Radii**: small 4 / normal & window 6 / native window 10 (unmaximized custom frame).
- **Shadow**: one recipe for menus/popups — offset (0,15), blur 50, `#00000080`, stroke NONE (`design_tokens.rs:735-745`). This single value is most of why Rerun's overlays look native.
- **Widgets that matter**: `list_item/` (full-span hover/selection rows with animated collapse `openness`, per-state text/icon token table); `section_collapsing_header.rs`; `command_palette.rs` (⌘K); `notifications.rs` (panel + toasts); `modal.rs`; `alert.rs`; `loading_indicator.rs`; animated `toggle_switch` via `ui.animate_bool` with lerped fill and radius (`ui_ext.rs:895-940`); `filter_widget.rs` (inline list filtering); custom window frame support (`WindowFrameConfig::Custom` + rounded native corners).

**Transfer map to Reyn:** ramp+alias tokens → §3.3; alert recipe → §3.3/§4.6; no-stroke tonal buttons + expansion → §3.4/§5.3.3; single-font-weight discipline (Reyn upgrades to 3 weights because its screens are more editorial than Rerun's) → §3.1; 24px rows/6px scrollbars/12px padding → §3.2; shadow recipe → §3.4 L3; list_item/full-span pattern → Evidence & recents rows; command palette → roadmap 3.5.

---

## Appendix C — Design-skill principles translated to immediate-mode GUI

Each principle is cited to its source skill and restated as an egui-actionable rule. Where a skill conflicts with the PRD, the PRD wins (noted).

**From `high-end-visual-design` (Awwwards-tier web skill):**
- "Generic 1px solid gray borders + harsh dark shadows = instant fail" → Reyn's uniform `OUTLINE_VARIANT` borders are precisely this; replace with tonal elevation (§3.4). *(A7)*
- Double-bezel/nested-radius concentricity (outer = inner + padding) → §3.5 concentric rule; egui: compute child `CornerRadius` from parent minus inset.
- Eyebrow tags: microscopic caps with wide tracking used *sparingly* → the §3.1 `overline` style (and the only caps survivor).
- Custom cubic-bezier motion, never linear → `emath::easing::cubic_out` in all `animate_*` calls.
- Banned-fonts list (bans Inter) → **overridden by PRD** (Inter mandated) and by Rerun precedent; the *intent* (avoid default-looking type) is honored via true weights + tracking instead.
- Glassmorphism/mesh-orbs → **rejected**: PRD §3.2 explicitly bans decorative blur/glow. Only possible echo: native macOS sidebar vibrancy, gated on founder sign-off.

**From `design-taste-frontend` (bias-correction skill):**
- Max 1 accent, saturation <80%, neutrals consistent (no warm/cool mixing) → ember-only accent; warm-neutral ladder fixes today's warm-on-warm mush. *(A3, A8, A13)*
- "Dashboard hardening: cards are banned where a divider groups better" → level-0 flat groups as the default; cards only when elevation means something. *(A7)*
- Mandatory interaction cycles: loading skeletons (not spinners), composed empty states, inline errors, `:active` press feedback → §3.7 press spec + §4.6 states. *(A10, A15)*
- Mono for all data numbers at cockpit density → already Reyn doctrine; scoped by §3.1 (mono = data, Inter = prose counts).
- Label above input, helper text persistent → Settings rows already comply; Case Setup forms adopt it.

**From `emil-design-eng` (Emil Kowalski motion philosophy):**
- Frequency test: 100×/day actions never animate → no animation on ⌘-shortcuts, nav clicks get ≤120ms fades only; command palette opens instantly (Raycast precedent). *(§3.7)*
- ease-out for entries, exits faster than entries, never ease-in → §3.7 table.
- Durations: press 100–160ms, dropdowns 150–250ms, modals 200–500ms, everything <300ms → §3.7 table.
- Never `scale(0)`; scale from ≈0.97 with opacity → modal spec 0.98→1.0.
- Interruptible transitions beat keyframes → egui's `animate_bool` retargets mid-flight natively; use it exclusively (no fixed keyframe sequences).
- Buttons must respond on press (`scale(0.97)`) → 1px inset + darken (§5.3.3).
- `prefers-reduced-motion` → an explicit Appearance setting (macOS global read isn't exposed via egui; setting satisfies the PRD's "respects reduced-motion settings").
- "Review animations the next day, in slow motion" → add a debug 0.25× animation-speed flag during Tier-2 development.

**From `ui-ux-pro-max` (UX checklist database):**
- Contrast ≥4.5:1 body / 3:1 large (§1 accessibility) → §3.3 text triad measured against bg-1/bg-2.
- Color never the only carrier (matches PRD UX-AC-01) → glyph+word+tint triple in §3.6.
- One primary CTA per screen (`primary-action`) → ember budget rule. *(A13)*
- Progressive disclosure of complexity (`progressive-disclosure`), error cause+fix (`error-clarity`), recovery paths (`error-recovery`) → §3.8 copy pattern. *(A5)*
- Nav: active state visible (`nav-state-active`), primary vs secondary separated (`nav-hierarchy`), unavailable destinations explained not hidden (`empty-nav-state`) → §4.1 rail. *(A6)*
- Disabled ≠ dead: reduced emphasis + reason (`disabled-states`) → inline reasons everywhere (Set active, Run).
- Skeletons over spinners >300ms (`progressive-loading`), reserve space to avoid layout shift (`content-jumping`) → §5.4.
- Tabular/mono figures to prevent number jitter (`number-tabular`) → §5.4 stable widths.
- 4/8pt spacing rhythm (`spacing-scale`), type scale 12/14/16/18/24/32-family (`font-scale`) → §3.2, §3.1.
- Confirm destructive actions + undo where possible (`confirmation-dialogs`, `undo-support`) → delete flows keep confirm; deletion of managed checkpoints gains an undo toast where reversible.

---

## 7. References

**Primary code evidence (this repo):** `src/theme.rs`, `src/fonts.rs`, `src/icons.rs`, `src/library.rs`, `src/settings.rs`, `src/app.rs`, `src/main.rs`, `assets/Inter.ttf` + `assets/JetBrainsMono.ttf` (variable-font axes verified with fontTools); `PRD.md` §3, §9–10.

**Rerun / egui (verified at source, 2026-07-24):**
- Rerun repo: https://github.com/rerun-io/rerun — `crates/viewer/re_ui/` (design system), esp. `data/dark_theme.ron`, `data/color_table.ron`, `src/design_tokens.rs`, `src/list_item/`, `src/command_palette.rs`, `src/ui_ext.rs` (toggle switch), `src/alert.rs`, `src/notifications.rs`
- Rerun product: https://rerun.io
- egui author = Rerun CTO: https://github.com/emilk (profile: "creator of egui, CTO of rerun.io")
- egui 0.35 (current stable, 2026-06-25): https://crates.io/crates/egui · https://docs.rs/egui/0.35 — APIs verified in source: `ViewportBuilder::{with_fullsize_content_view, with_titlebar_shown, with_title_shown, with_titlebar_buttons_shown, with_movable_by_window_background}`, `Context::{animate_bool_with_time, animate_bool_with_time_and_easing, animate_value_with_time}`, `emath::easing`, `RichText::{extra_letter_spacing, line_height}`, `Frame::shadow`, `Mesh::colored_vertex`, `FontFamily::Name`
- eframe 0.35: https://crates.io/crates/eframe

**Crates (versions checked on crates.io, 2026-07-24):**
- egui-phosphor 0.13.0: https://crates.io/crates/egui-phosphor · https://github.com/amPerl/egui-phosphor · https://phosphoricons.com
- muda 0.19.3 (native menus): https://crates.io/crates/muda · https://github.com/tauri-apps/muda
- hello_egui family (lucasmerlin): https://github.com/lucasmerlin/hello_egui — egui_flex 0.7.0, egui_inbox 0.12.0, egui_dnd 0.16.0, egui_animation 0.12.0, egui_virtual_list, egui_router
- window-vibrancy 0.8.0 (NSVisualEffectView): https://crates.io/crates/window-vibrancy · https://github.com/tauri-apps/window-vibrancy
- egui_tiles 0.16.0 (Rerun's docking): https://crates.io/crates/egui_tiles ; egui_dock 0.20.1: https://crates.io/crates/egui_dock
- catppuccin-egui 5.7.0 (token-application reference only): https://github.com/catppuccin/egui
- egui_taffy 0.13.0 (grid/flex alternative): https://crates.io/crates/egui_taffy

**Awards & platform guidance:**
- Apple Design Awards: https://developer.apple.com/design/awards/ · winners list (verified): https://en.wikipedia.org/wiki/Apple_Design_Awards — Shapr3D (2020), Procreate (2013), Procreate Dreams (2024 Innovation), Flighty (2023 Interaction), Affinity Designer (2015), Pixelmator (2011), Things (2009)
- Apple Human Interface Guidelines: https://developer.apple.com/design/human-interface-guidelines
- Red Dot (Interface & UX category): https://www.red-dot.org · iF Design Award: https://ifdesign.com
- Material 3 motion/elevation tokens (cross-reference): https://m3.material.io

**Pro-tool references:** Linear https://linear.app · Raycast https://www.raycast.com · Arc https://arc.net · Figma https://www.figma.com · Rive https://rive.app · Frame.io https://frame.io · DaVinci Resolve https://www.blackmagicdesign.com/products/davinciresolve · Blender 4.0 release https://www.blender.org/download/releases/4-0/ · Shapr3D https://www.shapr3d.com · Onshape https://www.onshape.com · nTop https://www.ntop.com · Luminary Cloud https://www.luminarycloud.com · SimScale https://www.simscale.com · Zed https://zed.dev · Lapce https://lapce.dev

**Typography:** Inter (features, static instances): https://rsms.me/inter · JetBrains Mono: https://www.jetbrains.com/lp/mono/

**Local design skills (source material, cited throughout Appendix C):**
- `/Users/hamza/.agents/skills/high-end-visual-design/SKILL.md`
- `/Users/hamza/.agents/skills/design-taste-frontend/SKILL.md`
- `/Users/hamza/.agents/skills/emil-design-eng/SKILL.md`
- `/Users/hamza/.claude/plugins/cache/ui-ux-pro-max-skill/ui-ux-pro-max/2.5.0/.claude/skills/ui-ux-pro-max/SKILL.md`
