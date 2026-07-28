# Voxel fidelity: measured evidence

Companion to `CAD_INTEROP_PLAN.md`. That report argued from code reading that the
occupancy mask can be silently wrong. This file measures it, so the fix has
numbers and fixtures instead of an argument.

Generator: `test-geometry/make_defective_stls.py` (writes `test-geometry/defective/`).
It reimplements the `src/cad.rs` parity fill exactly — +x rays, sorted crossings
paired two at a time, rows with fewer than two hits skipped — and compares the
result against an analytically known interior. Grid 64³, sphere r=50mm in a
200mm cube domain, boxes 100mm. Cell-center sampling, so a perfect implementation
still shows ~1% missing on a curved surface; that is the noise floor, not error.

## Measurements

| Fixture | Open edges | Odd rows | Cells wrong vs truth | Share of true interior |
|---|---|---|---|---|
| intact sphere (control) | 0 | 0 | 176 | 1.0% (discretization floor) |
| sphere, small pole hole | 30 | 8 | 228 | 1.3% |
| sphere, +z cap removed | 48 | 0 | 1316 | 7.6% |
| box, inverted normals | 0 | 0 | 0 | 0.0% |
| solid box + inner closed surface | 0 | 0 | 4096 | 12.5% |
| two interpenetrating boxes | 0 | 0 | 8788 | 16.4% |

All errors are missing interior cells; no fixture produced spurious solid cells.

## What the numbers change

**The watertightness gate is aimed at the wrong failure mode.** The two worst
cases — 12.5% and 16.4% of the interior missing — have **zero** open boundary
edges and **zero** odd-crossing rows. They pass every diagnostic the preflight
computes today, so no waiver is even required. Meanwhile the flagged case (a
small hole) costs 1.3%, barely above the 1.0% discretization floor. Repairing
holes alone would leave the larger errors untouched.

**An odd-crossing-count check is necessary but not sufficient.** Removing the
sphere's entire +z cap costs 7.6% of the interior while producing zero odd rows:
rays that would have crossed the cap now miss the surface entirely, so the count
stays even and parity stays "valid." Detection has to compare independent
evidence (multi-axis agreement), not audit one axis for parity.

**Winding is unvalidated and mask-invisible.** Reversing every triangle's
winding changes the mask by exactly zero cells, because parity does not care
about orientation. A follow-up code check corrected an important initial
assumption: current surface loads do **not** consume STL normals. The Python
engine derives its fluid-to-solid normal from the occupancy-mask gradient
(`engine/reyn_engine.py:167-180`), so inward source winding does not sign-flip
today's loads. Signed volume still belongs in provenance, and locally
inconsistent winding remains a hard topology defect, but global negative
winding is informational rather than grounds to mutate or reject the source.

**Nested closed surfaces are ambiguous, not merely wrong.** A solid box
containing a second closed surface reads as a hollow shell (the inner 16³ region
is left empty). Parity cannot distinguish "solid body with an internal void" from
"solid body with an inclusion." That is a modeling question the operator must
answer, not something to silently resolve.

## Requirements for the fix

1. Classify inside/outside with multi-axis agreement (x, y, z rays voting) and
   record the disagreement fraction as a first-class preflight number. Any
   disagreement above a defined threshold blocks rather than warns.
2. Surface `inconsistent_winding_edges` — already computed at `src/cad.rs:96`
   and currently dropped before it reaches the preflight — and add an
   outward-orientation check (signed volume) since load direction depends on it.
3. Detect self-intersection and nested shells explicitly. Neither is currently
   detected at all, and both produce double-digit interior error.
4. Tighten the waiver path. `record_waiver` (`src/engineering.rs:472`) accepts
   any rationale of eight or more characters, and `mesh.open_boundary`
   (`:322-330`) is waivable. A geometry-fidelity waiver must require a measured
   agreement metric, and must be impossible when measured disagreement exceeds
   the blocking threshold — prose cannot substitute for a number here.
5. Regression-test against `test-geometry/defective/`, asserting that each
   fixture is either corrected or blocked. Silent acceptance must fail the suite.

The evidence chain is the product's core claim. Today a case can read "approved"
on a mask that is 16% wrong, which is the one failure mode that cannot be allowed
to ship.

## Implemented classifier v2 — 2026-07-25

Classifier v2 independently fills along +X, +Y, and +Z and majority-votes each
cell. Its primary evidence number is:

`axis disagreement = cells where X/Y/Z do not all agree ÷ union of candidate solid cells`

The hard limit is **2.00%**. Valid shipped fixtures measured 0.00% at the tested
48³ import grid; the first invalid topology measured 10.00%. The threshold
therefore leaves a clear margin above valid numerical jitter without treating
a material classification conflict as judgment.

| Fixture | v1 outcome | v2 measured evidence | v2 disposition |
|---|---|---|---|
| cube | accepted | 0.00% disagreement; 0 odd rows | accepted |
| sphere | accepted | 0.00%; 0 odd rows | accepted |
| cross-flow cylinder | accepted but one-cell core | 0.00%; 0 odd rows | under-resolution waiver remains explicit |
| capsule | accepted | 0.00%; 0 odd rows | accepted |
| sphere, small pole hole | waivable; 1.3% truth error | 10.00%; odd rows Z=2 | hard-blocked by open edges and disagreement |
| sphere, cap removed | waivable; 7.6% truth error | 61.02%; odd rows Z=14 | hard-blocked by open edges and disagreement |
| box, inverted winding | accepted | 0.00%; signed volume negative | accepted with winding provenance; loads use mask gradients |
| box + inner closed shell | accepted; 12.5% truth error | 0.00%; two surface components | hard-blocked as nested/multi-shell ambiguity |
| interpenetrating boxes | accepted; 16.4% truth error | 0.00%; two components; intersecting triangle pairs detected | hard-blocked |

The result is not that malformed masks became physically correct. The result is
that none can become an approved run: measured topology/classification gates
are non-waivable, and classifier-v1 projects deserialize with version 0 and must
be re-imported. This preserves old evidence instead of silently relabeling it.

The previous bounding-box “critical thickness” was also replaced. V2 measures
the thickest local orthogonal core from contiguous occupied runs through every
solid cell. It identifies the shipped cross-flow cylinder as one cell thick at
48³ despite its large span; the old bounding-box minimum obscured this.

### Timing

Release-mode 128³ sphere import, including three binned ray classifications,
majority vote, and morphological diagnostics: **0.030 seconds** on the local
Apple machine. This is a measured test (`bench_three_axis_voxelization_128`),
not an estimate. Mesh topology/self-intersection diagnostics and file I/O are
outside that isolated classifier timing.

### Remaining limitation

The triangle-intersection sweep detects non-coplanar intersections. Exact
coplanar overlap classification is not claimed. Multi-component sources are
hard-blocked independently, so the shipped nested/interpenetrating fixtures are
still safe; a future exact-geometry translator should supply stronger B-rep
validity rather than treating this tessellated check as a universal CAD kernel.
