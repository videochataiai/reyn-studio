# Reyn Studio Design System

**Status:** implemented design contract  
**Scope:** native Reyn Studio application  
**Current-state basis:** `src/theme.rs`, `src/fonts.rs`, `src/app.rs`, `src/library.rs`, `src/settings.rs`, `src/field2d.rs`, `src/engineering_section.rs`, and `src/viewport.rs`

## 1. Purpose and authority

Reyn Studio is a local-first neural-CFD scientific instrument. This document describes the
design that is implemented now. It is not a speculative redesign and does not authorize
features that are absent from the product.

The source-of-truth order is:

1. `PRD.md` defines product requirements and scientific semantics.
2. Shipped Rust code defines current visual and interaction behavior.
3. `PRODUCT.md` defines personality, users, principles, and anti-references.
4. `docs/DESIGN_OVERHAUL.md` is a dated research report and roadmap. Use its rationale only
   where the current code has adopted it; do not treat its remaining proposals as shipped.

This contract supports:

- `REQ-UX-01` / `UX-AC-01`: grounded instrument quality and complete states.
- `REQ-UX-02` / `UX-AC-02`: journeys, stages, inspectors, and progressive disclosure.
- `REQ-SCI-01` / `SCI-AC-01`–`SCI-AC-03`: explicit source, method, evidence, and status meanings.
- `REQ-N6-IA-01` / `N6-IA-01`: the engineering case is the default path; experimental tools
  remain inside the Developer Research Sandbox.

## 2. Physical reference and scene

The physical reference is a calibrated laboratory instrument inside a native desktop shell:
quiet, mechanical, exacting, and designed to be read at length. The visual scene is a warm-dark
workbench surrounding a darker scientific viewport well. Data receives the strongest visual
energy; chrome recedes.

The reference is expressed through implemented details rather than literal skeuomorphism:

- warm-neutral stepped surfaces resemble separate instrument planes;
- one subtle top-edge highlight gives a level-one card a machined edge;
- 1-point separators define assemblies without boxing every row;
- compact mono readouts behave like measurement labels;
- ember marks the next consequential action or an active running state;
- the near-black viewport is the only place where calibrated scientific data may glow.

This is not a cockpit, a futuristic control room, or a luxury-object simulation. It is a local
engineering workspace for moving from Source → Case → immutable Run → Evidence.

## 3. Visual principles

1. **Evidence before spectacle.** A visualization must support measurement, comparison, or a
   decision.
2. **Honesty is structural.** Unknown, unsupported, recovered, derived, referenced, stale,
   unsigned, corrupt, and verified states remain visibly distinct.
3. **One physical vocabulary.** Units, source classes, status meanings, scales, and interaction
   grammar remain consistent across 2D, 3D, benchmarks, evidence, and exports.
4. **Expert speed.** Keep interaction native, responsive, keyboard-reachable, and predictable.
5. **Progressive disclosure.** Put the verdict and next action first; place methods, exact codes,
   tolerances, hashes, and diagnostics beside the object they explain.
6. **Durable objects over screen state.** Project, source revision, case revision, run, and
   evidence lineage outlive a view.
7. **Premium through precision.** Alignment, type, spacing, color, state coverage, and copy do
   the work. Decoration does not.
8. **Local ownership.** Core creation, execution, review, and export do not require an account or
   mandatory cloud service.

## 4. Surface and color tokens

All color values below are implemented in `src/theme.rs`.

### 4.1 Surface ladder

| Token | Value | Role |
|---|---:|---|
| `BG_VIEWPORT` | `#0E0C0A` | Darkest 2D/3D scientific viewport well |
| `BG_0` / `BG` | `#151210` | Document and application canvas |
| `BG_1` / `SURFACE_LOWEST` / `SURFACE_LOW` | `#1C1916` | Top bar, status bar, rails |
| `BG_2` / `SURFACE` | `#242019` | Cards and input-level surfaces |
| `BG_3` / `SURFACE_HIGH` | `#2D2822` | Hover, raised controls, menu base |
| `BG_4` / `SURFACE_HIGHEST` | `#37312A` | Active fills and highest tonal plane |
| `HAIRLINE` / `OUTLINE_VARIANT` | `#3B342C` | Quiet separators and borders |
| `OUTLINE` | `#574C41` | Interactive and focus-adjacent boundaries |

Document screens use `BG_0`; render screens use the configured theme-sanctioned viewport
background. The user may choose the darkest instrument well or the app canvas, but may not
replace it with an arbitrary color.

### 4.2 Text

| Token | Value | Role |
|---|---:|---|
| `TEXT` | `#F1ECE6` | Titles, principal values, selected text |
| `TEXT_DIM` | `#C6BCB1` | Body copy, labels, quiet controls |
| `TEXT_MUTE` | `#8F8478` | Captions, placeholders, timestamps, unavailable detail |

`TEXT_MUTE` is reserved for the implemented 11.5-point caption size or larger. High Contrast
uses white primary text and promotes quiet boundaries from `HAIRLINE` to `OUTLINE`.

### 4.3 Accent, status, and data

| Token | Value | Meaning |
|---|---:|---|
| `EMBER` | `#FF7A1A` | One primary action per screen; running-state indicator |
| `ON_EMBER` | `#2A1400` | Text/icon on an ember fill |
| `BRAND` | `#FFB68E` | Reyn wordmark and rare brand/provenance moments |
| `OK` / `SUCCESS` | `#3FBF8A` | A named proposition passed |
| `WARN` | `#E3A93C` | Unknown, metadata gap, stale, partial, or unavailable |
| `DANGER` / `DATA_RED` | `#E5544B` | Failed gate, rejection, destructive confirmation; labeled data extrema |
| `INFO` / `TERTIARY` | `#8ACEFF` | Neutral information and tertiary data blue |
| `GOLD` | `#F7BE1D` | Secondary data and highlighted comparison values |

Color rules:

- Ember is not a generic selection color, panel border, body-text color, or navigation fill.
  Active navigation uses a 2-point ember edge marker.
- Status always combines a hue with a glyph and words.
- Green means a specific proposition passed; it never means broad trustworthiness.
- Full-saturation fills are limited to the ember primary action and the danger confirmation.
- Calm alerts use `color.gamma_multiply(0.12)` for fill and
  `color.gamma_multiply(0.30)` for the hairline, with full color on the leading glyph or keyword.
- Gold and blue are data colors. Red can serve status or extrema only when the view labels the
  meaning and does not ask color alone to disambiguate it.

### 4.4 Interaction colors

The global egui state ladder is tonal:

- noninteractive: `BG_2`;
- inactive: `BG_3`;
- hovered: `BG_4` with `OUTLINE` and primary text;
- pressed/active: `BG_4`, not an ember flash;
- open: `BG_3`;
- selection: ember at 35% fill with a 1-point ember stroke;
- keyboard focus: 2-point ember at approximately 60%, drawn outside the control.

## 5. Typography and iconography

### 5.1 Families

- **Inter Regular** is the default interface face.
- **Inter Medium** is used for buttons, navigation, overlines, table headers, and emphasis.
- **Inter SemiBold** is used for screen, card, panel, and dialog titles.
- **JetBrains Mono Regular** is used for measurements, units, identifiers, hashes, timestamps,
  tabular numbers, and dense evidence.
- **JetBrains Mono Medium** is reserved for scientific-state chips.
- **Phosphor Regular** is the single general UI icon voice.

The shipped files are static font instances because egui does not expose variable-font axes.
Named weight families retain fallback chains. Inter and JetBrains Mono assets are stripped of
Private Use Area mappings so they cannot shadow Phosphor glyphs; replacing font files requires
preserving that constraint.

### 5.2 Implemented type scale

| Style | Face | Size | Tracking | Use |
|---|---|---:|---:|---|
| `display` | Inter SemiBold | 22 | −0.2 | One content-owned screen title |
| `title` / heading | Inter SemiBold | 16 | −0.1 | Card, panel, and dialog titles |
| `body-strong` | Inter Medium | 13 | 0 | Emphasis, buttons, nav, table headers |
| `body` | Inter Regular | 13 | 0 | Default interface copy |
| `caption` / small | Inter Regular | 11.5 | 0 | Helper text and timestamp prose |
| `overline` | Inter Medium | 10.5 | +0.8 | Uppercase section eyebrow only |
| `mono` | JetBrains Mono Regular | 12.5 | 0 | Measurements and tabular data |
| `mono-s` | JetBrains Mono Regular | 11 | 0 | Dense evidence and abbreviated hashes |
| `mono-chip` | JetBrains Mono Medium | 10.5 | +0.5 | Uppercase scientific state tokens |

The implementation fixes sizes and tracking but does not encode a separate line-height token
table. Do not claim exact line-height values that the renderer does not define.

Typography rules:

- Sentence case is the default.
- Uppercase is limited to section overlines and compact scientific-state tokens such as
  `MODEL`, `RECOVERED`, `UNKNOWN`, and `UNSIGNED`.
- Filenames and IDs are data, not headings. Show a humanized title and preserve the exact value
  in mono with full text on hover.
- Numbers in prose use Inter; aligned measurements, chips, hashes, and data columns use mono.
- Icon-only controls require a tooltip and an accessible meaning. Navigation always pairs an
  icon with text.

## 6. Spacing and layout

Dimensions are egui logical points.

The implementation follows a 4-point construction grid and an 8-point default rhythm, but does
not expose a complete named `s-*` spacing token set. The current shared values are:

- global item spacing: `8 × 8`;
- global button padding: `10 × 6`;
- floating scrollbar width: `6`;
- ordinary card padding: `16`;
- settings-section padding: `18`;
- common alert padding: `10–14`;
- primary document top spacing: `24–28`;
- section separation: commonly `12`, `16`, `20`, or `24`.

### 6.1 Window and shell

- default window: `1440 × 900`;
- minimum window: `1100 × 700`;
- top bar: `44`;
- bottom status bar: `24`;
- workflow/destination navigation row: `32`;
- left rail: resizable, default `248`, range `208–320`;
- right inspector: resizable, default `330`, range `280–420`;
- document column: centered, maximum `980`;
- document gutter: at least `34` where space permits.

Document width is capped, never stretched to fill wide windows. At narrow widths, content wraps
or stacks rather than shrinking type. Measurement rows stack below 360 points; settings rows
stack below 400 points.

### 6.2 Composition rhythm

- Use negative space and full-width hairlines before adding another container.
- A screen owns one display title in its content region.
- The left rail communicates lifecycle and destinations; it does not host unrelated diagnostics.
- The optional right rail describes or edits the active object.
- Render controls may overlay a scientific view only when they directly control or measure that
  view and there is enough room to place them without collision.

## 7. Radii, borders, and elevation

| Token | Value | Use |
|---|---:|---|
| `R1` | `4` | Buttons, inputs, chips, nav rows, segmented items |
| `R2` | `6` | Cards, menus, popovers, dialogs |

Use concentric geometry: a 4-point inner thumb sits inside a 6-point container with a 2-point
inset.

Implemented elevation:

- **Flat group:** no box; rows use a full-span hairline.
- **Level-one card:** `BG_2`, radius `R2`, 16-point padding, no outer border in the shared
  `card()` helper, and a 1-point white-at-4% inner top highlight.
- **Raised/active control:** `BG_3` or `BG_4`; specialized cards may retain a hairline where
  selection, input, or inventory boundaries need it.
- **Overlay:** `BG_3`, radius `R2`, no window stroke, shadow offset `(0, 15)`, blur `50`, black at
  50%.

Inputs, tables, segmented containers, inventory cards, and shell meeting edges may use a
1-point hairline. Do not add borders merely to create hierarchy. Some feature modules still use
hairline-bordered cards; the design contract acknowledges that implemented mixture rather than
pretending the borderless helper is universal.

## 8. Components and states

### 8.1 Shared components

- **Top bar:** Reyn wordmark, project identity and dirty state, contextual Run action, and
  Results 2D/3D selector.
- **Status bar:** engine state, honest long-operation status, cancel affordance when applicable,
  active run, and active model.
- **Workflow stage row:** icon, label, lifecycle glyph, optional inline blocking reason, active
  ember edge, animated hover, and focus ring.
- **Destination row:** same interaction grammar without a lifecycle glyph.
- **Primary action:** ember fill with `ON_EMBER`; exactly one visible next action per screen.
- **Quiet action:** tonal or transparent fill with hairline/outline as needed.
- **Danger action:** saturated danger fill only after explicit confirmation.
- **Gated action:** quiet disabled presentation plus a visible or hover-reachable reason; it does
  not fire.
- **Card:** tonal level-one group with top-edge highlight.
- **Alert line/panel:** glyph + sentence-case message + semantic tint; exact codes may follow in
  technical detail.
- **Scientific-state chip:** `mono-chip`, uppercase, color plus explicit word.
- **Measurement row:** label, right-aligned mono value, stable unit column, and source-class chip.
  It stacks on narrow inspectors.
- **Ledger row:** level-zero row with a hairline, abbreviated mono identifier, full value on hover,
  and copy affordance.
- **Inspector group:** overline header, caret, tonal hover, persistent collapse state.
- **Case stage spine:** verdict glyph, stage title, one-line summary, expandable details, and a
  vertical connector.
- **Segmented control:** tonal selection; ember is not used for the selected thumb.
- **Filter chip:** tonal active state, optional semantic dot, focus ring.
- **Skeleton inventory:** four card-shaped placeholders; animation stops under reduced motion.
- **Drop target:** level-one surface with dashed outline that brightens during a real file hover.
- **Command palette:** `⌘K`, keyboard selection, live gating, and the same reasons/actions as the
  shell.

### 8.2 Required state coverage

Every touched workflow must account for:

- loading or inventory wait;
- in-flight work, with determinate progress only when the engine reports a real fraction;
- success tied to a named proposition;
- empty state with one relevant recovery action;
- unavailable dependency;
- read-only evidence mode;
- stale lineage;
- blocked gate with reason;
- waived condition with named rationale;
- failed validation with exact technical detail;
- rejected checkpoint;
- destructive confirmation;
- preview versus recorded result;
- unsigned, invalid, revoked, and verified authenticity/integrity states.

Never show a percentage for an opaque single-pass operation. Reyn currently shows elapsed time
and states that no per-step progress is available.

## 9. Scientific visualization and source classes

### 9.1 Durable source classes

The project schema defines:

- `ModelPrediction`
- `SolverReference`
- `AnalyticalReference`
- `ExperimentalReference`
- `Recovered`
- `Derived`
- `Integrity`
- `AuthenticitySignature`

Visible shorthand may use `MODEL`, `REFERENCE`, `RECOVERED`, `DERIVED`, `INTEGRITY`, or
`AUTHENTICITY`, but the method and exact durable class must remain inspectable. Geometry and
provenance labels describe lineage context; `PREVIEW` describes a display-only model result and
must never be mistaken for a stored source class.

Rules:

- solver output is a solver reference, not physical truth;
- recovered pressure is reconstructed from predicted velocity;
- physical-reference `Cp` is derived only when the complete reference state is recorded;
- fluid traction and loads are derived fluid quantities, not structural stress;
- consistency is not accuracy;
- SHA-256 integrity is not signer authenticity;
- missing evidence is `UNKNOWN`, never inferred.

### 9.2 View classes

- **Engineering 3D:** imported body, calibrated surface/volume layers, source-aware probes,
  physical view stations, horizon playback, applicability strip, and optional field markers.
- **Engineering section:** X/Y/Z plane, persisted mask, axis labels, quantity, units, source,
  method, calibrated legend, and hover probe.
- **Standalone 2D sandbox:** model prediction, optional solver reference, and derived absolute
  error. Model and reference share one scale; independent normalization is prohibited.
- **Benchmark Lab:** seed × horizon comparison, persistence baseline, selected-cell spatial maps,
  spectra, provenance, and explicit fresh-test/validation meanings.
- **Flow Painter sandbox:** diverging vorticity canvas, brush polarity, projection diagnostics,
  and an explicit non-evidence boundary.

Every field view shows a calibrated legend and numeric values. Signed quantities use a diverging
scale centered at zero; nonnegative magnitudes use a sequential scale. Engineering `Cp` can use
an automatic symmetric extent or a pinned symmetric extent for cross-run comparison.

### 9.3 Colormaps

Interactive views offer:

- `Ember`: black-body sequential; blue → dark → gold diverging;
- `Viridis`: perceptually uniform sequential; Moreland cool–warm diverging;
- `Magma`: perceptually uniform sequential; Moreland cool–warm diverging.

The colormap preference changes interactive pixels only. Deterministic evidence exports pin the
calibrated Ember map so archived evidence is not mutated by a display preference.

Bloom is permitted only in the scientific viewport to communicate field intensity. It may not
decorate chrome, annotations, trust states, or text.

## 10. Motion and reduced motion

Motion communicates state or spatial continuity and must remain interruptible.

Implemented timing:

- global egui animation default: `160 ms`;
- component hover: typically `120–140 ms`, cubic ease-out;
- custom button press inset: `80 ms`;
- screen-switch veil: `180 ms`, cubic ease-out;
- inventory skeleton pulse: `1.1 s`;
- standard camera station/fit glide: `280 ms`, cubic ease-out.

Values and scientific counts do not tween. Motion may reveal a container or confirm selection,
but it may not fabricate intermediate data or delay work.

The persisted Reduce Motion preference:

- sets egui animation time to zero;
- makes shared hover/press motion resolve immediately;
- removes the screen transition;
- stops skeleton animation;
- makes camera station and fit changes immediate.

Do not claim roadmap-only modal, toast, or selection animations unless they are implemented at
the call site.

## 11. Accessibility

The product target is WCAG 2.2 AA-equivalent contrast and interaction quality within the native
egui surface.

- Primary text is approximately 13:1 and secondary text approximately 8:1 on intended surfaces.
- Tertiary text is approximately 4.6:1 and is not used below the 11.5-point caption size.
- High Contrast promotes primary text to white and strengthens control boundaries.
- Keyboard focus uses a visible outer ember ring.
- Status never relies on color alone; glyph and words travel with the color.
- Consequential workflows and actions are keyboard-reachable through native shortcuts,
  focusable controls, and the command palette.
- Disabled actions explain why they are disabled.
- Icon-only controls have tooltips; navigation uses icon plus text.
- Legends, units, source labels, and numeric values accompany field color.
- Interface scale persists from 80% to 140%.
- Reduced motion is persisted and applied to both interface and camera transitions.
- Long labels, paths, and hashes truncate instead of colliding; full values remain available on
  hover or copy.

This is a dense desktop instrument, not a touch UI. Do not claim a universal 44-point touch
target; implemented navigation rows are 32 points and dense list controls remain desktop-sized.

## 12. Voice and copy

- Use calm, direct, technical language.
- Lead with the object and consequence: “Import rejected — the active checkpoint is unchanged.”
- Put exact codes, fields, methods, and severity one disclosure level below plain-language
  guidance; never delete the technical meaning.
- Every error names a recovery, next step, or precise reason no recovery is available.
- Keep interactive labels in sentence case.
- Use `…` when an action opens a dialog or requires another step.
- Reserve `·` for compact metadata and mono data strings, not ordinary prose.
- Put the loudest fact first: “38 checkpoints — 9 need metadata review.”
- Explain `UNKNOWN` once on the surface; never replace it with a guess.
- Name time basis explicitly: elapsed time, horizon step, UTC timestamp, or physical lead time
  only when derivable.
- Never use “premium,” “seamless,” “powerful,” or other marketing filler in product UI.
- Never say “truth” for a numerical reference unless the source is documented as analytical or
  experimental exact evidence.

## 13. Prohibited patterns

- Persistent Support CTA, account furniture, upgrade prompts, pricing, or fake presence.
- Generic SaaS dashboards, interchangeable card grids, oversized KPI tiles, or filler metrics.
- Cyber-blue control rooms, glassmorphism, blur orbs, haze, ornamental grids, or “AI” animation.
- Ribbon clones, universal simulation trees, or every expert control visible at once.
- Decorative bloom, glow, or color that implies confidence or validity.
- Multiple competing primary actions or ember-filled navigation.
- Fake users, projects, checksums, signatures, results, progress, or enabled dead controls.
- Status by color alone, icon-only disclosure, or critical facts available only on hover.
- Silent navigation reroutes when a stage is blocked.
- Per-panel normalization of quantities that are being compared.
- Unsupported claims of embedded/associative CAD, general CAD support, physical pressure truth,
  structural stress, independent validation, shared-memory transport, or release readiness.
- Educational simplification that hides units, source, method, transform, provenance, or model
  limits.

## 14. Screen-level composition

### 14.1 Global shell

The native macOS title text is hidden while traffic lights remain. A 44-point in-app top bar is
the single visible chrome. Below it:

1. resizable left rail;
2. central content or scientific canvas;
3. optional resizable right inspector;
4. 24-point bottom status bar.

The left rail groups:

- project identity and local save state;
- **Workflow:** Project, Case Setup, Results, Evidence, each with lifecycle state;
- **Workbench:** Model Library and Settings;
- **Developer · Research Sandbox:** Procedural 3D, Flow Painter, Fields 2D, Benchmark Lab, only
  when explicitly enabled and labeled “NOT ENGINEERING EVIDENCE.”

Native macOS menus own app commands. Keyboard shortcuts and in-screen actions remain available
if menu installation fails.

### 14.2 Project

First run is a two-zone landing:

- left: “Start an analysis,” the single ember “Import geometry (STL)…” action, and exact
  fixed-body scope;
- right: quiet “Open project…” plus a real drag/drop target;
- below: flat Source → Run → Evidence explanation.

Returning use shows the active local project, real paths, save/recovery state, content
availability, cases, immutable run history, crash recovery, and recent projects. Recovery and
dependency failures use designed warning/read-only states. The right inspector covers project
identity, verified bundle objects, dependencies, storage, and save/open actions.

### 14.3 Case Setup

The center column owns the case title, exact lineage, and a vertical stage spine:

Source → Preflight → Contract → Operating point → Run.

Each stage puts its verdict before facts and expands in place. The right inspector edits source
approval, orientation, model contract, operating values, reusable defaults, waivers, and the
qualified run action. Source/model/run/evidence identity is not undoable; only safe case-draft
inputs participate in undo/redo.

When no geometry exists, the center presents one import action and an explicit internal-flow
reference-only card. Internal/HVAC execution remains blocked.

### 14.4 Results

An empty Results screen is a composed document state with one route back to Case Setup; it never
shows procedural placeholder data.

With a completed result:

- the center becomes the configured 3D viewport or geometry-linked 2D section;
- an applicability strip appears before the picture;
- the right inspector orders applicability, loads, derived quantities, reference values,
  horizon playback, variant comparison, layers/section controls, source-labeled voxel
  diagnostics, and export;
- source-aware probes name `Cp`, recovered pressure, traction, source-frame location, horizon
  step, and whether the field is recorded or preview-only.

The export menu separates immutable-run artifacts from the current view.

### 14.5 Evidence

The center reads as a ledger:

- exact source, case, run, model, and method lineage;
- run rows with UTC creation time and lifecycle state;
- abbreviated hashes with full-value hover and copy;
- scientific labels that distinguish model, recovered, derived, and not-computed quantities;
- designed read-only mode when compute is unavailable.

The right inspector summarizes the active evidence object and offers only provenance-safe exports.
No export may use a draft field or an unrecorded horizon preview.

### 14.6 Model Library

Model Library owns the full center width and has no right inspector:

1. one title, active-checkpoint chip, and single ember import action;
2. search, dimension selector, actionable health filters, and refresh;
3. one inline import-feedback owner;
4. reflowing inventory cards;
5. skeleton, empty, filtered-empty, rejected, and destructive-confirm states.

Cards show a humanized name, exact filename, contract summary, health, guidance, then disclosed
provenance and limitations. Rejected cards recede; active cards use an ember edge. Unknown
metadata remains explicit.

### 14.7 Settings

Settings uses the shared centered document column with an internal category rail. Only one
category body renders at a time:

Compute, Units, Appearance, Viewport, Workflow, Shortcuts, Storage, Signing, or Developer.

Rows pair a title and helper text with a control, stack at narrow widths, and keep per-setting
reset affordances. The right inspector summarizes the saved runtime, engine truth, units,
telemetry, signing state, and local settings path. Save is the screen’s primary action.

### 14.8 Research Sandbox

Procedural 3D, Flow Painter, standalone 2D fields, and Benchmark Lab are real implemented tools
but are not part of engineering case evidence. They remain hidden unless Developer mode is
enabled, carry an explicit sandbox label, and must not leak placeholder or analytical-demo data
into engineering Results or Evidence.

## 15. Deliberate implementation constraints

- Reyn is an immediate-mode egui application. Shared tokens and helpers are preferred, but
  specialized scientific drawing uses the painter directly.
- The current layout is desktop-first and has a hard minimum window size.
- The code implements only `R1` and `R2`; a roadmap-only window-radius token is not part of the
  current shared theme.
- The code has an 8-point default rhythm but no exhaustive public spacing-token module.
- Static font instances and the PUA-stripping rule are functional requirements, not optional
  asset cleanup.
- Interactive appearance preferences must never mutate stored SI evidence, manifests, source
  classes, or deterministic export pixels.
- Engine loss must degrade to local project operations and read-only evidence, not a broken shell.
- The scientific viewport may use GPU bloom and custom drawing; document screens remain tonal and
  low-chrome.
- Tessellated STL import is managed preprocessing, not embedded or associative CAD.
- Research Sandbox tools are deliberately separated from the default engineering workflow.
- Any design change that changes scientific wording, source labels, calibrated scales, evidence
  lineage, or status semantics is a product-contract change, not cosmetic polish.

## 16. Implementation anchors

- `src/theme.rs` — palette, text styles, radii, focus, motion preference, content width, global
  widget states.
- `src/fonts.rs` — static font registration, named weights, Phosphor fallback, PUA guards.
- `src/main.rs` — native window geometry and single-chrome configuration.
- `src/app.rs` — shell, workflow stages, document screens, inspectors, scientific canvases,
  reusable widgets, state coverage.
- `src/library.rs` — inventory composition, filters, cards, import feedback, skeleton/empty/error
  states.
- `src/settings.rs` — persisted accessibility and appearance preferences, category layout,
  responsive rows.
- `src/field2d.rs` — interactive colormaps, shared-scale comparisons, insight classes, probes.
- `src/engineering_section.rs` — engineering quantity names, units, sources, methods, and scales.
- `src/project.rs` — durable evidence source classes and lineage semantics.
- `src/viewport.rs` — camera navigation, standard physical view stations, reduced-motion
  interpolation.
