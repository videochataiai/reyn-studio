# Reyn Studio — App-State Analysis After Tier 1

**Date:** 2026-07-24
**Question answered:** the founder looked at the Tier 1 build and said it "didn't even change." This document quantifies why that reaction is correct, verifies which Tier 1 mechanics actually work, and states what Tier 2 must change for the difference to be unmistakable.

---

## 1. How much of the UI still bypasses the new token system

Tier 1 installed a real type scale (`display` / `title` / `body-strong` / `overline` / `mono-s` / `mono-chip`, with true Inter Medium / SemiBold and JetBrains Mono Medium instances). Almost nothing on screen consumes it. Counts below are call sites in the five UI-bearing files (measured 2026-07-24, `rg` on the working tree):

| Bypass category | app.rs | library.rs | settings.rs | field2d.rs | viewport.rs | Total |
|---|---:|---:|---:|---:|---:|---:|
| Hardcoded `.size(px)` on `RichText` | **137** | 26 | 16 | 0 | 1 | **180** |
| Explicit `FontId::…` (painter text) | **57** | 0 | 0 | 0 | 1 | **58** |
| `.monospace()` (ad-hoc mono, no scale) | 36 | 8 | 8 | 0 | 0 | **52** |
| `gamma_multiply` color hacks | 10 | 0 | 0 | 0 | 3 | **13** |
| Direct `Color32::from_*` literals | 2 | 0 | 0 | 2 | 2 | **6** |
| `.strong()` (fake-bold shim, pre-weights) | 21 | 2 | 3 | 0 | 0 | **26** |

Against that, the call sites that **do** use the named scale (`display_text` / `title_text` / `overline_text` / `chip_text` / `.text_style(...)`):

| File | On-scale call sites |
|---|---:|
| app.rs | 23 |
| library.rs | 9 |
| settings.rs | 5 |
| **Total** | **37** |

**Headline: ~290 text call sites set their own per-call font/size versus 37 that use the scale — roughly 89% of text bypasses the system.** The `FontId::` breakdown in app.rs is 39 × `monospace`, 16 × `proportional`, 2 × `new` — the whole painter-drawn chrome (nav rows, engine pill, camera chip, section headers, hint lines) is hand-sized.

The size histogram shows where the eye actually lands. Distribution of the 137 hardcoded sizes in app.rs:

| px | count | | px | count |
|---:|---:|---|---:|---:|
| 10.5 | 37 | | 13.5 | 6 |
| 9.5 | 18 | | 13.0 | 5 |
| 11.0 | 13 | | 12.5 | 5 |
| 12.0 | 12 | | 11.5 | 5 |
| 9.0 | 11 | | 8.5 | 2 |
| 10.0 | 9 | | 18 / 15 / 14 | 1 each |

**87 of 137 (64%) are ≤ 10.5 px** — i.e. most of the app's visible text is still the 9–11 px micro-mono that the overhaul report identified as the core "debug build" signature (audit A1). The `caps()` helper is now routed through the sanctioned `overline_text` style, but it still has **38 call sites** in app.rs uppercasing every section label, so caps-lock is still the dominant hierarchy device on screen.

### Top offenders (what the eye actually sees)

1. **The in-app menu row + brand row** (`app.rs` `top_bar`) — hand-sized 18 px brand + 13.5 px File/Edit/View/Window menus duplicating the macOS menu bar. First thing seen on every screen; unchanged geometry.
2. **`nav_row`** — painter text at hand-built `FontId 13.5` + hand-drawn icons; 10 rows always visible.
3. **`diag()` rows** (label 13 / value mono 14, hardcoded) — used ~60× across case setup, results, evidence, project rails.
4. **`project_fact` pills** — 9.5 px mono caps chips, dozens per case screen.
5. **Floating engine pill + camera chip + hint line** — three separately styled painter fragments (`FontId::monospace(11.5)`, `monospace(12.0)`, `proportional(12.5)`) floating over content on every analysis screen.
6. **Model Library cards** — title `body_strong().size(14.0)` (style referenced, then size-overridden), facts at 10.5 px, hashes at 9.5 px.
7. **Settings rows** — label 12.5 / helper 10.5 hardcoded, ~20 rows.

## 2. Do the Tier 1 mechanics actually work at runtime?

Verified mechanically against the working tree:

| Mechanic | Verdict | Evidence |
|---|---|---|
| Static font instances shipped | **Works** | `assets/` now has Inter-{Regular,Medium,SemiBold}.ttf + JetBrainsMono-{Regular,Medium}.ttf; the two variable TTFs are deleted |
| Named `FontFamily` registration ↔ theme lookups | **Works** | `fonts.rs` inserts font-data keys `inter-medium`, `inter-semibold`, `jbmono-medium` and registers families under `FAMILY_MEDIUM` / `FAMILY_SEMIBOLD` / `FAMILY_MONO_MEDIUM`; `theme.rs::apply_with_contrast` resolves the *same constants*, so no silent fallback is possible. (A regression test locking this now exists in `fonts.rs`.) |
| Named text styles installed | **Works** | `display` 22 SemiBold, `title` 16 SemiBold, `body-strong` 13 Medium, `overline` 10.5 Medium, `mono-s` 11, `mono-chip` 10.5 JBMono-Medium, plus Body 13 / Button 13 Medium / Small 11.5 / Monospace 12.5 — all installed via `ctx.all_styles_mut` |
| Styles referenced by views | **Barely** | 37 call sites total (§1). Two of those override the style's size anyway (`seg()` forces 11.5, library card title forces 14.0) |
| Surface/text/semantic tokens | **Works, via aliasing** | Old names (`BG`, `SURFACE`, `TEXT_DIM`, `DATA_RED`, …) are aliased to the new ladder, so every call site inherited the recalibrated values without moving |
| Alert recipe (`tint_fill`/`tint_hairline`) | **Works where applied** | Library rejection panel, project notices, settings notices — genuinely calmer |
| Ember budget / edge-marker nav | **Works** | Active nav is a 2 px ember edge + tonal fill; export/import buttons demoted to quiet |
| Popup shadows, 6 px scrollbars, `animation_time 0.16` | **Works but invisible** | `animation_time` is set, yet **zero `animate_*` call sites exist** in `src/`, so nothing actually animates; the unconditional `ctx.request_repaint()` at the end of `ui()` still burns a full repaint every frame |

Conclusion: Tier 1's plumbing is sound — the fonts, families, styles, and tokens are real and correctly wired. It failed to *show* because the views don't consume it.

## 3. Why the founder can't see it (the honest perceptual accounting)

**What changed:** true SemiBold on the six `display_text`/`title_text` headers; text desaturated from peach `#f5ded3` to off-white `#f1ece6`; alert boxes went from solid red fills to 12% tints; active nav went from an orange slab to an ember edge; menus gained a soft shadow.

**What did not change — and dominates:**

1. **Identical layout geometry.** The same 52 px top bar with the same in-app File/Edit/View/Window menu row, the same fixed 276 px left rail with the same 10 nav rows and the same diagnostics card, the same fixed 330 px right rail, the same `columns(2)` card grid, the same 920–980 px left-anchored content with the same dead right gutter at 1440. Every rectangle on screen has the same position and size as before Tier 1.
2. **Identical chrome grammar.** Every container is still `fill + 1 px hairline border + small radius` (`card()`, notice frames, chips, the nav card). The elevation system exists only in menu shadows, which are rarely open.
3. **Dark-on-dark value shifts.** The canvas moved `#1c110b → #151210` and cards `#291d16 → #242019` — both pairs are ~L\*7–15 near-black browns. Side by side they differ; from memory they read as "the same dark brown app." The only high-chroma pixels (ember, gold, status colors) kept their hues.
4. **Body text same size and effective weight.** Body was and is ~13 px Regular; 64% of visible text is still ≤10.5 px mono/caps (§1). The new Medium/SemiBold weights land on exactly 37 call sites, mostly headers whose *positions* didn't move.
5. **Nothing moves.** No hover/press animation, no transitions — `animation_time` is configured but unconsumed, so interaction feel is byte-identical.

In short: Tier 1 changed the *paint recipe* while deliberately freezing every *shape*, and at these luminance levels the paint difference is below the casual-glance threshold. The founder's reaction is the expected outcome of that trade, not a measurement error.

## 4. What Tier 2 must change for the difference to be unmistakable

Ordered by perceptual dominance:

1. **Chrome silhouette.** Hidden native title text + fullsize content view, a 44 px traffic-light-inset top bar owning project identity + run action, the in-app menu row deleted (commands to the native macOS menu bar), and a 24 px status bar as the single status home. This changes the outline of the app — the one thing impossible to miss.
2. **Navigation shape.** Two visibly distinct rail groups (workflow lifecycle with per-stage ●/◐/○ state glyphs and disabled-with-reason, vs. Library/Settings destinations), voxel-diagnostics card gone from global nav (relocated to Results with an explicit source label), animated hover, resizable rail.
3. **Model Library layout.** The 330 px junk-drawer rail dissolves into a toolbar (search + dimension filter + clickable count chips) above a width-reflowing card grid with restructured card anatomy — a different screen, not a recolored one.
4. **Motion.** 120–220 ms ease-out hover/press on every interactive row/button, focus rings, skeleton shimmer — the app starts *feeling* engineered rather than snapping.
5. **Type actually on the scale** at the high-visibility sites listed in §1 (nav labels, diag rows, fact pills, status strings, card titles, settings rows), so the weights bought in Tier 1 are finally visible at density.
6. **One icon voice** (Phosphor Regular) replacing the hand-drawn mixed-stroke set.

Everything above is Tier 2 scope in `docs/DESIGN_OVERHAUL.md` (§4.1, §4.6, §3.7, §6 items 2.1–2.5, 2.7). Items untouched by Tier 2 (Case Setup spine, Results measurement table, Evidence ledger, landing redesign, command palette) remain Tier 3.
