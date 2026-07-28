# CAD interoperability plan for Reyn Studio

**Research date:** 2026-07-24
**Scope:** Engineering interchange formats, what Reyn's voxel pipeline actually requires from geometry, the Rust/native implementation landscape with licence analysis, mesh repair and its provenance, unit handling, and a phased plan.
**Evidence policy:** External claims cite a primary source (standards body, vendor documentation, package registry, or repository) with the URL checked on 2026-07-24. Internal claims cite `file:line` in this repository as inspected on 2026-07-24. Statements labelled **Reyn recommendation** are synthesis, not sourced claims. No vendor performance or marketing language is treated as a requirement.
**Relationship to prior work:** This report is the detailed execution study behind `docs/CFD_APP_LANDSCAPE.md` §9 ("CAD strategy: build the evidence boundary, integrate the kernel") and behind `PRD.md` REQ-P-CAD-01 / P-CAD-01 / P-CAD-02. It does not restate the competitor CAD-integration taxonomy (landscape §3.1, §9.1) or the customization findings in `docs/FEATURE_GAP_AUDIT.md`; it builds on them.

---

## Executive findings

1. **"Just add STEP" is three separate projects, and only one of them is a parser.** Getting from a STEP file to something Reyn can voxelize requires (a) reading Part 21, (b) evaluating and tessellating trimmed NURBS B-rep faces into a watertight triangle shell, and (c) recovering persistent region identity. (a) has a mature Apache-2.0 Rust option; (b) does not; (c) is an open problem that the standards community addresses with a *recommended practice* rather than a mechanism ([CAx-IF Persistent IDs v1.8, 2026-03-13](https://www.mbx-if.org/home/cax/recpractices/)).

2. **The formats that would most improve Reyn's evidence chain tomorrow are not STEP.** Reyn's model consumes an occupancy mask, not geometry (`src/cad.rs:185-198`, `src/cad.rs:265-490`). The two things missing from STL that Reyn provably needs *today* are an authoritative unit declaration and a watertightness guarantee. 3MF supplies both: a required-by-schema unit attribute defaulting to millimetre ([3MF Core Specification v1.3.0](https://3mf.io/wp-content/uploads/sites/55/2025/02/3MF_Core_Specification_v1.3.0.pdf)) and a manifold requirement in the specification itself. It costs a ZIP reader and an XML reader, not a geometry kernel.

3. **The single biggest licensing landmine is static linking of OpenCASCADE.** OCCT is LGPL-2.1 with an exception that covers *header material in object code only* ([OCCT licensing](https://dev.opencascade.org/resources/licensing)). An Open Cascade maintainer states directly that a static build "can NOT be linked to the commercial(private) code with static linking. That is why delivery is dynamic," and that a paid exception is the route for proprietary static linking ([OCCT issue #244](https://github.com/Open-Cascade-SAS/OCCT/issues/244)). The published Rust bindings do exactly the forbidden thing by default: `occt-sys` is "Static build of the C++ OpenCascade CAD Kernel for use as a Rust dependency" ([crates.io/crates/occt-sys](https://crates.io/crates/occt-sys), checked 2026-07-24).

4. **Crate-declared licences describe the wrapper, not the vendored C++.** `cadrum` — currently the most active OCCT-in-Rust option, v0.8.16 published 2026-07-17 — declares MIT on crates.io while statically linking OCCT 8.0.0 ([crates.io/crates/cadrum](https://crates.io/api/v1/crates/cadrum); [cadrum changelog](https://github.com/lzpel/cadrum/commit/86e19b76cde3ab9275f6efec07fe09d980eb9716)). An SPDX field in a manifest is not a licence audit of the shipped binary.

5. **Reyn's voxelizer fails *silently* on non-watertight input, and the preflight lets that reach a run.** Ray parity pairs sorted crossings two at a time and skips any grid row with fewer than two hits (`src/cad.rs:392-422`). A leak changes the crossing count, so entire interior spans are dropped or added with no error. Yet `mesh.open_boundary` is a **waivable** preflight issue (`src/engineering.rs:322-331`), waivable by any rationale of eight characters or more (`src/engineering.rs:470-489`). A reviewer can therefore approve a case whose occupancy mask is wrong by an unbounded amount, with a compliant-looking evidence chain.

6. **AP203 and AP214 — the two STEP flavours most CAD systems emit by default — have been withdrawn ISO standards since 2014-12-01** ([ISO 10303-203:2011](https://www.iso.org/standard/44305.html), [ISO 10303-214:2010](https://www.iso.org/standard/43669.html)). The current standard is AP242 Edition 4, ISO 10303-242:2025, published 2025-08 ([ISO 10303-242:2025](https://www.iso.org/standard/84300.html)); Edition 3 was withdrawn 2025-08-25 ([ISO 10303-242:2022](https://www.iso.org/standard/84667.html)). Reyn must read what people send, which means withdrawn schemas, not the current one.

7. **Semantic PMI is irrelevant to Reyn and will stay irrelevant.** The CAx-IF recommended practice is explicit: "PMI Representation (semantic) data shall only be used when exchanging exact geometry. It is not intended to be associated with tessellated geometry" ([Rec. Practices for PMI v4.1, 2024-06-20](https://www.mbx-if.org/home/wp-content/uploads/2024/06/rec_pracs_pmi_v41.pdf)). Reyn consumes a voxel mask. GD&T carries no information about an occupancy field. Any roadmap line item promising PMI support is scope theatre.

8. **Named boundary regions are the one capability that genuinely blocks the internal-flow follow-on, and they are not free even in STEP.** `InternalFlowContract` requires named inlet/outlet/wall assignments with `region_id` (`src/engineering.rs:621-631`, `src/engineering.rs:767-772`), and `PRD.md:373-375` requires "named and stable region IDs". STL cannot carry them; the app already says so in its own reimport diff (`src/app.rs:8451`). But a Siemens community thread reports NX emitting `ADVANCED_FACE('facename',…)` under AP214 and `ADVANCED_FACE('',…)` under AP242 ([Siemens community](https://community.sw.siemens.com/s/question/0D5KZ00000642js0AA/include-surface-names-in-step-242-export)), and STAR-CCM+ users are advised to route named faces through Parasolid rather than a neutral format ([Siemens community](https://community.sw.siemens.com/s/question/0D54O000061x9JqSAI/parasolid-export-naming-issue)). Region naming is an *ingestion-plus-authoring* problem, not a format checkbox.

9. **Reyn's dependency tree is currently pure Rust plus macOS system frameworks (`Cargo.toml:7-33`).** There is no vendored C or C++ build anywhere in it. Every CAD-kernel option breaks that property permanently, and on macOS it also means dylib packaging, code-signing of embedded libraries, notarization, and a universal-binary story for two architectures. That cost belongs to a separate process, not to `reyn-studio`'s link line.

10. **The honest v1 answer is a mesh-formats-plus-repair release, with STEP behind an out-of-process converter later.** This preserves the input-side unit gate that `docs/FEATURE_GAP_AUDIT.md:19` found to already exceed the category norm, closes the silent-leak hole that undermines every reported number, and defers the licence, packaging, and persistent-identity problems until there is a paying reason to take them on.

---

## 1. Method

### 1.1 What this report treats as fact

- **Sourced format fact** — stated in an ISO catalogue entry, a published specification, a consortium recommended practice, or first-party vendor documentation, linked inline.
- **Reported behaviour** — a vendor community thread or user report, labelled as such. Used only where no primary source exists, and never as the sole basis for a plan decision.
- **Observed Reyn fact** — visible in this repository at the cited `file:line` on 2026-07-24.
- **Reyn recommendation** — synthesis.

### 1.2 A note on line-number drift in prior docs

`docs/FEATURE_GAP_AUDIT.md:94` cites the unit gate at `src/engineering.rs:141-143`; the gate is now at `src/engineering.rs:156-158`. `docs/FEATURE_GAP_AUDIT.md` also lists body orientation (R4) as missing, but it is implemented: `BodyOrientation` at `src/cad.rs:212-255`, applied in `voxelize_oriented` at `src/cad.rs:279-324`, recorded in preflight at `src/engineering.rs:238-240`, and validated at `src/engineering.rs:390-400`. All line references in *this* document were re-checked on 2026-07-24.

---

## 2. What engineers actually hand off

### 2.1 Format capability matrix

"Named regions" means: can the format carry a name attached to a face or facet subset that survives export and can be used to assign a boundary condition?

| Format | Units in file | Assemblies | Named regions | Geometry | Practical prevalence |
|---|---|---|---|---|---|
| **STEP AP242** (ISO 10303-242:2025) | Yes — `length_unit` per geometric representation context; may differ between representations in one file | Yes, with transforms | Weakly — face `name` attributes exist but are exporter-dependent; stable identity needs the CAx-IF Persistent IDs practice | Exact B-rep/NURBS, plus optional tessellated shape | The current standard; export requires an MBD add-in in SOLIDWORKS |
| **STEP AP203 / AP214** | Yes, same mechanism | AP203 yes; AP214 adds colour/layer | AP214 commonly emits `ADVANCED_FACE('name',…)` (reported) | Exact B-rep | Withdrawn since 2014-12-01 yet still the default export nearly everywhere |
| **IGES 5.3** | Yes — Global Section parameter 14 Unit Flag, parameter 15 Unit Description | No true assembly model | Entity-level names via property entities; unreliable across translators | Surfaces, wireframe, optional BREP | Frozen at 5.3 (1996); legacy only |
| **Parasolid X_T / X_B** | **No** — geometry is always in metres, no unit label in the file | Yes | Yes — face/body names carried at kernel level | Exact B-rep | The lingua franca between Parasolid-kernel systems (NX, SOLIDWORKS, Solid Edge, STAR-CCM+) |
| **JT** (ISO 14306-4:2026) | Per file/model metadata | Yes, hierarchical, streamable, multi-LOD | Yes, via product structure | Tessellated (mandatory, multi-LOD) plus optional B-rep | Dominant in automotive/aero visualization and supplier exchange |
| **3MF** (Core v1.3.0) | **Yes** — `model/@unit`, default millimetre, from {micron, millimetre, centimetre, inch, foot, metre} | Objects + components with transforms | Yes — objects, and triangle sets via the Triangle Sets extension | Tessellated | Additive-manufacturing standard; increasingly the STL replacement |
| **glTF 2.0** | **Yes by specification** — all linear distances are metres; +Y up | Yes — scene graph with nodes | Node/mesh names only | Tessellated | Graphics/web/visualization; not an engineering handoff |
| **STL** | **No** | No | No | Tessellated, unstructured triangle soup | Ubiquitous, and what Reyn accepts today (`src/app.rs:8348-8350`) |
| **OBJ** | **No** — coordinates have no units; scale sometimes stated in a comment | No | `g` groups, `o` objects, `usemtl` material spans | Tessellated (plus free-form curves/surfaces, rarely used) | Common as a graphics interchange; occasionally used for CFD surface handoff |
| **PLY** | **No** | No | Custom element properties only, by convention | Tessellated / point clouds | Scan and research pipelines |
| **Native (SLDPRT, CATPart, .prt, Fusion)** | Yes | Yes | Yes | Exact B-rep + features + history | What the engineer actually has; readable only via a paid SDK or the originating CAD system |

Sources for the table, all checked 2026-07-24:

- STEP editions: [ISO 10303-242:2025](https://www.iso.org/standard/84300.html) (Edition 4, published 2025-08, currently at stage 90.92); [ISO 10303-242:2022](https://www.iso.org/standard/84667.html) (Edition 3, withdrawn 2025-08-25); [ISO 10303-203:2011](https://www.iso.org/standard/44305.html) and [ISO 10303-214:2010](https://www.iso.org/standard/43669.html) (both withdrawn 2014-12-01, superseded by AP242); [ISO/CD 10303-242](https://www.iso.org/standard/93277.html) (Edition 5 under development, comment period closed 2026-04-22).
- STEP units: OCCT's STEP translator reads length, plane-angle, and uncertainty from `shape_representation` entities ([OCCT STEP user guide](https://dev.opencascade.org/doc/occt-7.9.0/overview/html/occt_user_guides__step.html)). Multiple units can coexist in one model ([HOOPS Exchange units FAQ](https://techsoft3d.atlassian.net/wiki/spaces/KBHE/pages/503875084/FAQ+How+units+are+handled+in+HOOPS+Exchange)).
- STEP defaults in practice: STEP `mm`, IGES `mm`, Parasolid `m`, SolidWorks `m`, CATIA V5 `mm`, Inventor `cm`, ACIS `cm`, OBJ/STL/U3D "no unit" ([HOOPS Exchange units FAQ](https://techsoft3d.atlassian.net/wiki/spaces/KBHE/pages/503875084/FAQ+How+units+are+handled+in+HOOPS+Exchange)).
- AP242 export gating: SOLIDWORKS requires the MBD add-in to publish STEP 242 ([Javelin, 2025-10](https://www.javelin-tech.com/blog/2025/10/choosing-the-best-neutral-file-formats-in-solidworks/)).
- IGES: Global Section parameter 14 Unit Flag (required) and parameter 15 Unit Description ([NIST IR 4600, IGES 5.0 Recommended Practices Guide](https://nvlpubs.nist.gov/nistpubs/Legacy/IR/nistir4600.pdf)); OCCT reads files "up to and including version 5.3" and "all non-millimeter length unit values in the IGES file are converted to millimeters" ([OCCT IGES guide](https://dev.opencascade.org/doc/occt-7.4.0/overview/html/occt_user_guides__iges.html)).
- Parasolid: "The Parasolid units are always meters. When importing into a CAD program, the units should be scaled accordingly" ([3D-Tool](https://www.3d-tool.com/cad-files/parasolid-viewer.htm)); Rhino reports "Rhino units = inches. Parasolid units = meters. Scaling exported geometry by 0.0254" ([McNeel forum](https://discourse.mcneel.com/t/parasolid-export-units/96361)); Siemens states Parasolid data "is always stored in Metric units (actually in Meters)" ([Eng-Tips](https://www.eng-tips.com/threads/parasolid-unit-settings.157213/)).
- JT: [ISO 14306-4:2026](https://www.iso.org/standard/86064.html), published 2026-04-07, Part 4 Version 3; the monolithic [ISO 14306:2017](https://www.iso.org/standard/62770.html) was withdrawn 2026-04-07 in favour of the four-part series. Scope covers "facet information (triangles) … product manufacturing information (PMI); boundary representation (b-rep) solid model shape representation and associated metadata".
- 3MF: [3MF Core Specification v1.3.0](https://3mf.io/wp-content/uploads/sites/55/2025/02/3MF_Core_Specification_v1.3.0.pdf) — `unit`, type `ST_Unit`, default `millimeter`, "Specifies the unit used to interpret all vertices, locations, or measurements in the model. Valid values are micron, millimeter, centimeter, inch, foot, and meter." Spec repository: [3MFConsortium/spec_core](https://github.com/3MFConsortium/spec_core), BSD-2-Clause.
- glTF: linear distances in metres, +Y up, right-handed ([glTF 2.0 specification, Coordinate System and Units](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)).
- OBJ: "OBJ coordinates have no units, but OBJ files can contain scale information in a human readable comment line"; grouping via `g`, objects via `o`, materials via `usemtl` ([Wavefront .obj file](https://en.wikipedia.org/wiki/Wavefront_.obj_file); [EGFF summary](https://www.fileformat.info/format/wavefrontobj/egff.htm)).

### 2.2 STEP in detail, since it is what everyone asks for

**Units.** A STEP length unit is declared in the geometric representation context as an SI unit with a prefix (`SI_UNIT(.MILLI.,.METRE.)`) or as a conversion-based unit (`CONVERSION_BASED_UNIT('INCH',…)`). It is *per representation context*, so one file can legitimately carry more than one length unit; HOOPS Exchange documents exactly this hazard and provides `A3DAsmModelFileGetUnit` to find the first context flagged as coming from CAD ([HOOPS Exchange units FAQ](https://techsoft3d.atlassian.net/wiki/spaces/KBHE/pages/503875084/FAQ+How+units+are+handled+in+HOOPS+Exchange)). "STEP declares its units" is true; "a STEP file has a unit" is not.

**Names and regions.** Face-level names ride on the `name` attribute of geometric entities and on shape-aspect structures; the CAx-IF publishes a whole recommended practice for making identity survive a round trip ([Persistent IDs v1.8, 2026-03-13](https://www.mbx-if.org/home/cax/recpractices/)), and the Geometric Validation Properties practice notes that AP242's uniqueness rule for shape-aspect IDs exists for backward-compatibility reasons and "does not formally require the ID attribute to exist" ([Rec. Practices for GVP v4.5](https://www.mbx-if.org/home/wp-content/uploads/2024/05/rec_prac_gvp_v45.pdf)). User-defined attributes can be attached to "a section of the part shape, i.e. solids or surfaces" ([Rec. Practices for User Defined Attributes v1.8](https://www.mbx-if.org/home/wp-content/uploads/2024/05/rec_prac_user_def_attributes_v18.pdf)). All of this is *available*; none of it is *guaranteed by the file you are handed*.

**Semantic PMI.** Out of scope for Reyn, permanently — see Executive finding 7 and the CAx-IF quote there.

**What CFD tools actually do with regions.** Fluent's guided workflow builds boundary zones structurally rather than semantically: "Set the `Create One Zone Per` option to object, part, body, or face" ([Ansys Fluent User's Guide, 2025 R1](https://ansyshelp.ansys.com/public/views/secured/corp/v251/en/flu_ug/tgd_user_workflow_guided_tasks_import_part_manage.html)). That is the pattern Reyn should copy: derive candidate regions from structure, let the operator name and assign them, and store the mapping — rather than hoping the file carries CFD-ready names.

### 2.3 The honest prevalence ranking

For "an engineer emails you a part to analyse", ordered by how often it arrives and how much it costs to accept:

1. **STEP** (AP203/AP214/AP242 mixed, most often the withdrawn protocols) — always offered, expensive to accept.
2. **STL** — always available, cheapest to accept, carries nothing.
3. **Native CAD** — what they actually have; needs a paid SDK.
4. **Parasolid** — excellent fidelity between kernel-compatible systems; proprietary and unit-less.
5. **JT** — common in large OEM supply chains; four-part ISO standard, complex.
6. **3MF** — rising via additive manufacturing; cheap to accept, and carries the two things STL lacks.
7. **IGES** — legacy; declining; frozen specification.
8. **OBJ / PLY / glTF** — present in adjacent pipelines, not engineering handoffs.

---

## 3. What Reyn actually needs from geometry

This section is derived from the code path, not from a feature wish list.

### 3.1 The four needs

**N1 — A watertight occupancy mask at the model's grid.** `voxelize_oriented` casts one axis-aligned ray per (y,z) row, collects crossings, sorts them, and fills between consecutive *pairs* (`src/cad.rs:392-422`). Rows with fewer than two crossings are skipped entirely (`src/cad.rs:398-400`). Correctness of the mask is therefore conditional on even parity along every row, which is exactly the watertightness property. The mask is the only geometric object the model consumes (`src/cad.rs:185-198`).

**N2 — A reference length in physical units.** Reynolds number is `ρ · U · L · (metres per source unit) / μ` and returns `None` while the unit is `Unknown` (`src/engineering.rs:96-105`, `src/engineering.rs:137-147`). The voxelizer's own `char_len` is a fixed `0.6` solver units (`src/cad.rs:337`, `src/cad.rs:482`) — a solver-frame constant, not a physical length. The physical reference length comes from source extents plus a confirmed unit, and nothing else.

**N3 — A body transform and orientation.** `voxelize_oriented` rotates about the source bounding-box centre, refits, and composes rotation, isotropic scale, and placement into a single column-major 4×4 (`src/cad.rs:446-477`), which `solver_point_to_source_m` (`src/engineering.rs:976`) inverts to report results in the approved source frame. The transform is a similarity — rotation, uniform scale, translation. Anisotropic scaling, mirroring, and unit-per-axis differences are not representable and must never be silently introduced by an importer.

**N4 — Named boundary regions, for internal flow only.** `InternalBoundaryAssignment` carries `region_id`, `name`, and `role` (`src/engineering.rs:621-631`); `execution_blockers` requires non-empty inlet, outlet, and wall assignments (`src/engineering.rs:767-772`); `PRD.md:373-375` requires the IDs to be named *and stable*. External flow does not need this. Internal flow cannot start without it.

### 3.2 Which formats supply which need

| Need | STL | OBJ | PLY | 3MF | glTF | STEP | Parasolid | JT | Native |
|---|---|---|---|---|---|---|---|---|---|
| N1 watertight mask | Only if the file happens to be clean | Same | Same | Spec requires manifold objects | No guarantee | Yes, after tessellation of a valid solid | Yes | Yes (tessellated LODs) | Yes |
| N2 unit for reference length | No | No | No | **Yes, authoritative** | **Yes, by specification (metres)** | **Yes, per context** | No (always metres, unlabelled) | Yes | Yes |
| N3 transform / orientation | Identity only | Identity only | Identity only | Yes (item transforms) | Yes (node transforms) | Yes (assembly transforms) | Yes | Yes | Yes |
| N4 named regions | **No** | `g` / `usemtl` groups | No | Objects + triangle sets | Node/mesh names | Face names, exporter-dependent | Yes, kernel-level | Yes | Yes |

### 3.3 Minimum sufficient set

**Reyn recommendation — v1 (external flow):** **STL + 3MF + OBJ**, with PLY as a near-free follow-on.

Rationale, in the terms of §3.1:

- N1 is not solved by *any* format; it is solved by a repair-and-classification stage inside Reyn (§5). Adding STEP does not remove this stage — B-rep tessellation produces its own gaps at surface-trim boundaries, which is why OCCT ships an entire Shape Healing module ([OCCT Shape Healing](https://dev.opencascade.org/doc/occt-7.9.0/overview/html/occt_user_guides__shape_healing.html)).
- N2 is the highest-value gap and 3MF closes it authoritatively for a ZIP + XML reader. glTF closes it too but is not an engineering handoff, so it earns a lower priority.
- N3 is already implemented and needs no new format.
- N4 is not required for external flow, and no *tessellated* format gives stable identity across a reimport.

This set is deliberately unglamorous. It is defensible because every element of it improves a number that appears in a report.

**Reyn recommendation — internal-flow follow-on:** **STEP AP242/AP214 with B-rep, via an out-of-process converter, plus operator-authored region assignment.**

Internal flow needs N4, and N4 needs face-level identity that survives a reimport. Only exact-geometry formats can supply the topological anchor for that identity, and even then the identity has to be *constructed* by Reyn (a stable hash over face geometry and topology, recorded at first import) rather than trusted from the file — because AP242 does not require an ID attribute to exist ([Rec. Practices for GVP v4.5](https://www.mbx-if.org/home/wp-content/uploads/2024/05/rec_prac_gvp_v45.pdf)), and because a Siemens community report has AP242 export dropping face names that AP214 preserves ([Siemens community](https://community.sw.siemens.com/s/question/0D5KZ00000642js0AA/include-surface-names-in-step-242-export)).

### 3.4 The uncomfortable conclusion

Exact geometry is not the bottleneck for a credible external-flow v1. **Correct inside/outside classification is.** A B-rep import that lands in the same ray-parity voxelizer produces a mask with the same silent failure mode (Executive finding 5). Fixing classification first makes every subsequent format cheaper to accept; doing STEP first makes a bigger pipeline with the same hole in the middle.

---

## 4. Rust and native implementation options

### 4.1 Current dependency posture

`Cargo.toml:7-33` lists eframe/egui, wgpu (via eframe), bytemuck, serde, serde_json, rfd, sha2, uuid, flate2, fontdue, png, base64, ed25519-dalek, getrandom, zeroize, egui-phosphor, plus macOS-only muda and security-framework. There is no vendored C or C++ build in the tree. `flate2` is already present, which matters: 3MF is a ZIP container, and a ZIP reader is the only container dependency it needs.

### 4.2 STEP and B-rep options

All metadata from crates.io and repository pages, checked 2026-07-24.

| Option | What it does | Maturity | Build cost | Licence | Verdict |
|---|---|---|---|---|---|
| **`ruststep` 0.4.0** (2024-09-20) | EXPRESS schema → Rust types, Part 21 parsing | Real, Apache-2.0, ~18k recent downloads | Pure Rust | Apache-2.0 | **Viable for reading**, not for geometry |
| **`truck-stepio` 0.3.0**, `truck-modeling` 0.6.0, `truck-polymesh` 0.6.0, `truck-meshalgo` 0.4.0, `truck-topology` 0.6.0 (all 2024-09-20) | STEP in/out plus a Rust B-rep kernel with NURBS and meshing | Repository active (last push 2026-07-06, 1.5k stars, Apache-2.0) but **no crate release in ~22 months**; note that the crates.io name `truck` belongs to an unrelated package ("generates a cargo toml for you", v1.0.0, 2020), so the kernel is only reachable through its component crates | Pure Rust | Apache-2.0 | **Watch. Prototype against git, do not ship on pinned 0.6.x for production STEP** |
| **`opencascade` / `opencascade-sys` 0.2.0** (2023-08-16) with **`occt-sys` 0.6.0** (2024-11-30) | Rust FFI over the OCCT kernel | Stale relative to OCCT 8.0.0 (final released 2026-05, plus an 8.0.0p1 patch); high-level wrapper untouched for ~3 years | cxx + CMake + a full C++ kernel build | **LGPL-2.1 declared, and `occt-sys` is a *static* build** | **Do not link into `reyn-studio`** |
| **`cadrum` 0.8.16** (2026-07-17) | Statically linked headless OCCT 8.0.0, native + WASM, prebuilt binaries | Genuinely active (created 2026-03-29, frequent releases) | Downloads prebuilt OCCT or builds from source | Declares MIT — **but statically links LGPL OCCT** | **Do not link into `reyn-studio`; usable in a separate GPL/LGPL-compliant process** |
| **OCCT directly, dynamically linked, in a sidecar** | Full STEP/IGES translation, XDE names/colours/layers, shape healing, meshing | The reference implementation; OCCT 8.0.0 released 2026-05 | C++ build, dylib packaging, signing, notarization, two architectures | LGPL-2.1 with exception | **The realistic open path, out of process** |
| **Commercial SDK** (CAD Exchanger, HOOPS Exchange, Datakit) | Neutral + native formats, assemblies, metadata, healing | Mature | Vendor binaries | Paid, per-distribution | **The realistic paid path when native formats become a requirement** |

Notes on specific claims:

- `truck` component crate versions and dates: [truck-stepio](https://crates.io/crates/truck-stepio), [truck-modeling](https://crates.io/crates/truck-modeling), [truck-polymesh](https://crates.io/crates/truck-polymesh). Repository activity: [ricosjp/truck](https://github.com/ricosjp/truck) (last push 2026-07-06). The unrelated occupant of the bare `truck` crate name is visible in its own crates.io metadata.
- `occt-sys` self-description: "Static build of the C++ OpenCascade CAD Kernel for use as a Rust dependency", licence LGPL-2.1, v0.6.0 2024-11-30 ([crates.io/crates/occt-sys](https://crates.io/crates/occt-sys)).
- OCCT 8.0.0: [release notes](https://github.com/Open-Cascade-SAS/OCCT/releases/tag/V8_0_0), final planned for 2026-05-07, plus a subsequent 8.0.0p1 hot patch ([releases index](https://github.com/Open-Cascade-SAS/OCCT/releases)).
- OCCT XDE reads names, colours, layers, validation properties, and GD&T into a document via `STEPCAFControl_Reader` ([OCCT STEP guide](https://dev.opencascade.org/doc/occt-7.9.0/overview/html/occt_user_guides__step.html)) — that is the mechanism that would eventually feed Reyn's region names.

### 4.3 Mesh repair, classification, and voxelization options

| Option | Role | Version / date | Licence | Assessment |
|---|---|---|---|---|
| **Reyn's own code** | Parse, diagnose, ray-parity voxelize | `src/cad.rs` | proprietary | Already correct in structure; the gaps are classification robustness and repair |
| **`manifold` (C++) / `manifold3d` 0.3.3** (2026-07-06) | Guaranteed-manifold Boolean and SDF level-set meshing | Upstream v3.5.2, 2026-06-27, active | **Apache-2.0** (upstream and bindings) | Clean licence; but it *requires* manifold input and says so: "you'll get an error status if the imported mesh isn't manifold … in general you may need one of the automated repair tools" |
| **`mesh_to_sdf` 0.4.0** (2024-09-17) | Mesh → signed distance grid | Modest adoption | MIT OR Apache-2.0 | Useful for a distance-field classification fallback; needs care on non-watertight input, where sign is undefined |
| **`fast-surface-nets` 0.2.1** (2025-01-03) | Isosurface extraction from a grid | Small, single-file | MIT OR Apache-2.0 | Only needed if Reyn re-extracts a surface after voxel repair |
| **SideFX `WindingNumber`** | Fast generalized winding number (Barill et al., SIGGRAPH 2018), BVH-accelerated | Reference implementation | **MIT** | The permissive route to robust inside/outside on imperfect meshes; small enough to port or vendor |
| **libigl winding number** | Same algorithm | — | MPL-2.0 core, but the fast winding number example repository is MPL-2.0 and parts of the ecosystem are copyleft | Prefer the MIT SideFX implementation |
| **CGAL Polygon Mesh Processing** | Repair, self-intersection, hole filling | — | **GPLv3+ for PMP** | **Poisons a closed-source release** |
| **MeshLab / admesh** | Interactive and CLI repair | — | **GPL** | **Poisons a closed-source release**; usable only as a separately distributed external tool the user installs themselves |

Sources: [manifold3d](https://crates.io/crates/manifold3d), [elalish/manifold](https://github.com/elalish/manifold) (Apache-2.0, v3.5.2 2026-06-27), [mesh_to_sdf](https://crates.io/crates/mesh_to_sdf), [fast-surface-nets](https://crates.io/crates/fast-surface-nets), [sideeffects/WindingNumber](https://github.com/sideeffects/WindingNumber) (MIT).

### 4.4 Licensing analysis

**The landmine, stated precisely.** OCCT is LGPL-2.1 plus an exception that permits object code to incorporate material from OCCT *header files* under terms of your choice ([OCCT licensing](https://dev.opencascade.org/resources/licensing)). The exception does not permit static linking of the library into a proprietary binary. LGPL-2.1 §6 requires that a combined work be distributed on terms that "permit modification of the work for the customer's own use and reverse engineering for debugging such modifications", and the shared-library route exists precisely so the user can relink. Open Cascade's own maintainer states in [issue #244](https://github.com/Open-Cascade-SAS/OCCT/issues/244): "OCCT have LGPL 2.1. License and can NOT be linked to the commercial(private) code with static linking. That is why delivery is dynamic. Only open source solutions use static linking"; and separately, "OCCT offers the option to purchase an LGPL 2.1 exception, allowing static linking in proprietary applications". The same thread notes the WASM case is effectively impossible to make compliant because everything bundles into one binary.

Consequences for Reyn:

1. **`occt-sys` and `cadrum` are both static by construction.** Adding either to `Cargo.toml` puts OCCT object code inside `reyn-studio`'s binary. The wrapper's MIT or LGPL SPDX string does not change the obligation attaching to the combined work.
2. **Dynamic linking inside a signed macOS `.app` is the workable open route,** with OCCT dylibs in `Contents/Frameworks`, unmodified, with licence notices, and — critically — with the user able to substitute a modified OCCT. That is straightforward for a notarized DMG. It is not achievable inside a Mac App Store sandbox, so choosing OCCT constrains the distribution channel.
3. **Running OCCT in a separate executable that Reyn launches over IPC is cleaner still.** The converter is then a distinct program, its LGPL obligations are contained, Reyn's own binary stays pure Rust, and — as `docs/CFD_APP_LANDSCAPE.md:513` already recommends — a kernel crash cannot take down the app.
4. **GPL is an absolute no.** CGAL's Polygon Mesh Processing package, MeshLab, and admesh are GPL. Copying an algorithm's *idea* is fine; copying or linking its code is not. This must be an explicit rule in the repository, because mesh-repair code is exactly the domain where the best reference implementations are GPL.
5. **MPL-2.0 (libigl) is file-level copyleft and is survivable, but unnecessary** given that the MIT-licensed SideFX winding-number implementation exists.

**Reyn recommendation:** add a short `docs/DEPENDENCY_LICENSE_POLICY.md` (permitted: MIT, Apache-2.0, BSD, Zlib, ISC; case-by-case: MPL-2.0; forbidden in-process: LGPL, GPL, AGPL, SSPL; LGPL permitted only in a separately distributed process with dynamic linking) and enforce it with `cargo-deny` in CI. This is a half-day of work that prevents an unwindable mistake.

### 4.5 Commercial SDKs, order of magnitude

No vendor publishes a list price; all three require a quote.

- **CAD Exchanger SDK** — the fee has two parts: a development fee (a higher first-year joining fee, then a lower annual maintenance fee, scaled by which formats and add-ons you license) and a per-end-user-machine distribution fee reported quarterly ([CAD Exchanger SDK pricing](https://cadexchanger.com/products/sdk/pricing/)). The vendor's own published order of magnitude: "the price level of a few thousand US dollars a year (or dozens of thousands if you are in a higher tier)" ([CAD Exchanger on Medium](https://cadexchanger.medium.com/which-cad-exchanger-developer-tools-are-right-for-me-d32562a563bf)). Reads SolidWorks files on macOS without a CAD install ([SDK page](https://cadexchanger.com/products/sdk/)).
- **HOOPS Exchange (Tech Soft 3D)** and **Datakit** use comparable dev-licence-plus-royalty structures; neither publishes pricing. Tech Soft 3D's public documentation is nonetheless the best free reference on cross-format unit semantics ([units FAQ](https://techsoft3d.atlassian.net/wiki/spaces/KBHE/pages/503875084/FAQ+How+units+are+handled+in+HOOPS+Exchange)).
- **Parasolid** licensing from Siemens is an OEM kernel arrangement, materially more expensive than a translator SDK and not appropriate for a company that does not intend to model.

**Reyn recommendation:** a paid SDK is the right answer *only* when native-format ingestion (SLDPRT, CATPart, .prt) becomes a sales blocker. Until then it buys breadth Reyn cannot yet use, on top of a per-seat cost against an unproven price point. Revisit when a design partner asks for it by name.

---

## 5. Mesh repair and preflight hardening

### 5.1 What Reyn does today, and exactly how it fails

`diagnose_mesh` (`src/cad.rs:32-131`) quantizes vertices onto a grid of `diagonal × 1e-6`, builds an edge-use map, and reports triangles, connected components, degenerate triangles, boundary edges, non-manifold edges, inconsistent winding edges, and extents (`src/cad.rs:19-28`). That is a good diagnostic set. Four problems follow from what happens next.

**Problem 1 — leaks are waivable but not survivable.** `mesh.open_boundary` is waivable (`src/engineering.rs:322-331`), and `record_waiver` accepts any rationale of eight characters or more (`src/engineering.rs:470-489`). But §3.1/N1 shows the mask is only correct under even ray parity. A waived leak yields a mask that is wrong in an unbounded, unreported way, and every downstream number — forces, moments, Cp range, wake deficit — inherits that error while the evidence chain still reads "approved".

**Problem 2 — winding inconsistency is computed and then dropped.** `diagnose_mesh` returns `inconsistent_winding_edges` (`src/cad.rs:26`, `src/cad.rs:96-99`), and `import_cad_path` turns it into a free-text warning (`src/app.rs:8421-8426`), but `GeometryPreflight` has no field for it (`src/engineering.rs:223-250`) and `support_issues` never raises it (`src/engineering.rs:277-449`). It therefore never becomes a gate, a waiver, or a contract field, and it does not travel in `exact_contract()` (`src/engineering.rs:599-616`).

**Problem 3 — "critical thickness" is not thickness.** `minimum_cells_across` is the smallest dimension of the *bounding box* of occupied cells (`src/cad.rs:612-615`), but the preflight reports it as "Critical thickness resolves to only N cells" (`src/engineering.rs:425-434`). A wing with a two-cell trailing edge inside a 40-cell bounding box passes this check. The metric should be a morphological one — the largest ball that fits, or the minimum occupied run length along each axis — not a bounding-box extent.

**Problem 4 — one ray direction, one vote.** Parity is evaluated only along +x (`src/cad.rs:384-422`). There is no cross-check, so there is no signal that a row's classification is suspect.

### 5.2 The standard repair pipeline

The conventional order, as implemented across additive-manufacturing and CFD preprocessors:

1. **Vertex welding / duplicate removal** at an explicit tolerance, converting a triangle soup into a shared-vertex mesh.
2. **Degenerate and duplicate face removal** — zero-area triangles, coincident faces, unreferenced vertices.
3. **Orientation and winding repair** — propagate a consistent orientation across each shell, then flip shells whose signed volume is negative.
4. **Hole filling** — identify boundary loops, then fill by ear-clipping for planar loops or by a smooth patch for large loops.
5. **Self-intersection resolution** — detect and resolve triangles that pierce each other; the expensive step, and the one most likely to change the geometry.
6. **Shell splitting and selection** — separate disconnected components, discard interior voids or spurious shells, and decide which shells constitute the analysis body.

Upstream evidence that this stage is unavoidable even with exact geometry: OCCT ships a Shape Healing user guide devoted to it ([OCCT Shape Healing](https://dev.opencascade.org/doc/occt-7.9.0/overview/html/occt_user_guides__shape_healing.html)), and Manifold, whose whole thesis is guaranteed-manifold output, still tells users to bring a repair tool ([elalish/manifold README](https://github.com/elalish/manifold)).

**Reyn recommendation — do steps 1-3 and 6 as *repairs*, and replace steps 4-5 with robust classification.** Reyn does not need a valid mesh. It needs a correct occupancy field. A generalized winding number (Barill et al., SIGGRAPH 2018; MIT reference implementation at [sideeffects/WindingNumber](https://github.com/sideeffects/WindingNumber)) is defined for triangle soups with holes and self-intersections, so it answers the question Reyn is actually asking without modifying the customer's geometry at all. That is both cheaper to build and *far* easier to defend in a report: classifying differently is not the same as changing the part.

### 5.3 The honesty question: three evidence classes

**Reyn recommendation:** every import step is assigned one of three classes, and the class — not the operator's prose — decides what is allowed.

- **Class A — source-exact.** Parsing only. The analysed triangle set is a deterministic function of the source bytes. Reported numbers are traceable to the source hash with no qualification.
- **Class B — classification-tolerant.** Operations that cannot change the enclosed solid beyond a stated, *measured* bound: vertex welding below tolerance, degenerate-triangle removal, duplicate-face removal, winding and normal reorientation. These change the triangle list, not the body. Requires recording the tolerance, the before/after diagnostics, and the measured bounding-box and enclosed-volume deltas.
- **Class C — geometry-modifying.** Hole filling, self-intersection resolution, shell deletion, and any SDK-side shape healing. Occupancy may change in ways not bounded a priori. Never silent, never waivable by prose alone, always shown in the report.

**The rule that makes Class C tractable:** a repair that cannot change a single voxel cannot change a single reported number. If the largest filled hole's diameter is below one voxel edge at the target grid, and the volume delta is below one voxel volume, the repair is *evidentially inert* and may be recorded as such — with the numbers shown. Above that threshold, the case carries a visible "analysed geometry differs from source" state through to the report. This ties the honesty question to the discretization Reyn already tracks (`target_grid`, `src/engineering.rs:242`), rather than to a subjective judgement.

### 5.4 A concrete provenance representation

The repository already has the exact mechanism needed, used for unit approval: on approval, `commit_active_case_revision` creates a **new** `SourceRevision` with the same `content_sha256`, a bumped `revision`, `declared_units: Some(approved_units)`, a `frame` string, the approved `transform_4x4`, and `parent_revision_id` pointing at the imported revision (`src/app.rs:1494-1524`). Repair should use the same shape.

**Proposed additions** (implementation belongs to the source owner; this is the schema, not a patch):

```rust
// src/engineering.rs — GeometryPreflight gains:
pub analyzed_sha256: String,          // canonical hash of the triangle set actually voxelized
pub inconsistent_winding_edges: usize, // computed today at src/cad.rs:96, currently dropped
pub import_steps: Vec<ImportStep>,     // ordered, replayable

pub struct ImportStep {
    pub code: String,                 // "parse.stl" | "weld.vertices" | "orient.shells" | "fill.holes" | ...
    pub tool: String,                 // "reyn.mesh_repair" | "occt.shapehealing"
    pub tool_version: String,
    pub evidence_class: EvidenceClass, // SourceExact | ClassificationTolerant | GeometryModifying
    pub parameters: serde_json::Value, // tolerances, thresholds — everything needed to replay
    pub before: MeshDiagnosticsRecord,
    pub after: MeshDiagnosticsRecord,
    pub volume_delta_fraction: f64,    // measured, not asserted
    pub largest_change_diameter_source_units: f64,
    pub voxel_inert: bool,             // change is below one cell at target_grid
}
```

Three properties make this work with what already exists:

1. **It reaches the case contract for free.** `exact_contract()` serializes `preflight` wholesale (`src/engineering.rs:613`), and `CaseRevision.contract` is an untyped `serde_json::Value` (`src/project.rs:81`), so new preflight fields land in the immutable case revision without a manifest schema change.
2. **`SourceRevision` needs a real migration.** It is `#[serde(deny_unknown_fields)]` (`src/project.rs:38`), so a `repair_of` or extended `warnings` semantics is a versioned change. The cheapest compliant option is to reuse the existing `warnings: Vec<String>` (`src/project.rs:52`) for the human-readable repair log and keep the structured record in the preflight, which is where the case revision reads it from anyway.
3. **A repaired body becomes a derived geometry revision**, `source_kind: Geometry`, `content_sha256` = the canonical hash of the repaired triangle set, `parent_revision_id` = the imported revision, exactly mirroring `src/app.rs:1512-1524`. The chain then reads: source bytes → imported revision → repaired revision → approved revision → case revision → run. A reviewer can see at a glance that the analysed geometry is two derivations away from the bytes they were sent, and read the measured deltas at each hop.

### 5.5 What blocks and what may be waived

**Reyn recommendation** for `support_issues` (`src/engineering.rs:277-449`):

| Condition | Today | Proposed |
|---|---|---|
| `mesh.open_boundary` | waivable by prose | Waivable **only** when robust classification agrees with parity on ≥ 99.9% of cells and the disagreement set is reported; otherwise blocking until repaired |
| `mesh.non_manifold` | blocking | Keep blocking for parity; **downgrade to warning** once winding-number classification is the default, since it is well-defined on soups |
| `mesh.inconsistent_winding` | not an issue at all | New waivable issue, auto-repairable as Class B |
| `voxel.under_resolved` | bounding-box extent | Re-derive from a morphological thickness measure |
| New: `classification.ambiguous` | — | Blocking when the fraction of cells where multi-axis parity disagrees exceeds a threshold; this is the honest replacement for trusting a watertightness flag |

---

## 6. Units and orientation

### 6.1 How units arrive

| Format | Authority | What Reyn should record |
|---|---|---|
| STL, OBJ, PLY | None | `declared_units: None`; the operator's confirmation is the only source |
| 3MF | `model/@unit`, default `millimeter` | The literal attribute value, plus whether it was present or defaulted |
| glTF | Specification: metres | `"spec.gltf.meters"` as the authority, since no per-file statement exists |
| STEP | `length_unit` in each geometric representation context | The unit *and* whether the file contained more than one; multiple units is a blocking condition, not a silent pick |
| IGES | Global parameter 14 (+15) | Flag value and description string |
| Parasolid | None in file; kernel semantics are metres | `declared_units: None` with a note that the convention is metres; never auto-apply the customary ×1000 |
| JT / native | Per file metadata | Whatever the translator reports, plus the translator identity |

### 6.2 The gate must get stronger, not weaker

`docs/FEATURE_GAP_AUDIT.md:19` found Reyn's input-side unit gate stronger than the category norm. It is implemented as `LengthUnit::Unknown` being the default (`src/engineering.rs:55-56`, `src/engineering.rs:124`), a validation issue while unconfirmed (`src/engineering.rs:156-158`), `reynolds()` returning `None` without a unit (`src/engineering.rs:137-138`), and a non-waivable `transform.approval_required` (`src/engineering.rs:442-448`).

**Reyn recommendation:** when a format declares units, the declaration becomes *evidence*, never *consent*.

1. Store the declaration separately from the confirmation. `SourceRevision.declared_units` already exists for this (`src/project.rs:48`) and is currently `None` on every import (`src/app.rs:8556`) and only populated post-approval with the operator's choice (`src/app.rs:1520`). Split it: `declared_units` for what the file said, and the operating point's `length_unit` for what the human confirmed.
2. Prefill, do not auto-confirm. A declared millimetre preselects millimetre in the unit control; `transform.approval_required` still gates the run.
3. A mismatch between declared and confirmed is a first-class, non-silent record. It appears in the preflight, in the contract, and in the report. Overriding a file's own declaration is a legitimate engineering act — the file may be wrong — but it must be visible.
4. Absent declarations stay absent. A `None` from STL must never be filled in with a guess, however plausible.
5. Multiple declared units in one STEP file is blocking, not a majority vote.

### 6.3 The translator normalization trap

Any kernel-based path silently rescales. OCCT's `xstep.cascade.unit` is "normally MM", `read.precision.val` is expressed "in millimeters, independently of the length unit defined in the STEP file" ([OCCT STEP guide](https://dev.opencascade.org/doc/occt-7.9.0/overview/html/occt_user_guides__step.html)), and for IGES "all non-millimeter length unit values in the IGES file are converted to millimeters" ([OCCT IGES guide](https://dev.opencascade.org/doc/occt-7.4.0/overview/html/occt_user_guides__iges.html)). HOOPS Exchange normalizes everything to millimetres internally too ([units FAQ](https://techsoft3d.atlassian.net/wiki/spaces/KBHE/pages/503875084/FAQ+How+units+are+handled+in+HOOPS+Exchange)).

So a converted file has **two** unit facts: what the source declared, and what the translator emitted. Reyn must record both, plus translator name, version, and options — which is exactly what `PRD.md:576-577` (P-CAD-01) already requires. A single `declared_units` string is not sufficient for a converted source.

### 6.4 Orientation and frames

Reyn's solver frame is fixed: free stream on +X, angle of attack about +Y, yaw about +Z, roll about +X, composed as `R_yaw · R_aoa · R_roll` (`src/cad.rs:200-254`). Incoming formats disagree about up-axis — glTF specifies +Y up, most mechanical CAD is Z-up by convention — so any new format needs an explicit, recorded axis convention at import, not a guess. The mechanism exists: the transform is already displayed and approved as a 4×4, and `body_orientation_summary` (`src/engineering.rs:266-275`) already narrates the applied rotation. A format-level axis convention should appear in the same approval surface, with the same non-waivable gate.

---

## 7. Phased plan

Effort is engineer-days for one experienced Rust developer already familiar with this codebase, including tests.

### Phase 0 — Make the existing pipeline honest (no new dependencies)

**Scope.** Multi-axis parity voting (cast along x, y, and z; classify by majority; record the disagreement fraction). Surface `inconsistent_winding_edges` as a real preflight issue. Replace the bounding-box "critical thickness" with a morphological measure. Tighten the `mesh.open_boundary` waiver so it requires a measured agreement metric, not prose. Add a `classification.ambiguous` blocking issue.

**Files.** `src/cad.rs` (`voxelize_oriented`, `voxel_diagnostics`), `src/engineering.rs` (`GeometryPreflight` fields, `support_issues`), `src/app.rs` (preflight construction at `src/app.rs:8470-8489`, warning text).

**Dependencies.** None.

**Tests.** The existing fixtures (`test-geometry/cube_100mm.stl`, `sphere_d100mm.stl`, `cylinder_d60_l200mm.stl`, `capsule_d80_l260mm.stl`, `naca0012_wing_c120_s300mm.stl`) as the clean baseline; add deliberately damaged variants — one triangle deleted, one shell inverted, one duplicated-vertex soup, one self-intersecting pair — generated by extending `test-geometry/make_test_stls.py`. Assert: clean fixtures produce bit-identical masks to today (regression lock), damaged fixtures produce a non-zero disagreement fraction and the right blocking issues.

**Risks.** Three-axis voting triples voxelization cost. At 64³ with the existing (y,z) binning this is a small absolute number, but it must be measured, and the +x-only path must remain reproducible for existing runs.

**Effort.** 4-7 days.

**Why first.** It fixes a correctness hole that silently invalidates reported numbers, costs nothing in dependencies, licence, or packaging, and every later phase inherits the benefit.

### Phase 1 — 3MF and OBJ import, and the declared-versus-confirmed unit split

**Scope.** A `GeometrySource` abstraction over the current `parse_stl` (`src/cad.rs:134`). 3MF reader: ZIP container, `3D/3dmodel.model` XML, vertices/triangles, `model/@unit`, object transforms. OBJ reader: `v`/`f` plus `g`/`o` group capture (stored, not yet used for boundary conditions). PLY as a follow-on. Unit plumbing per §6.2. File-dialog filters at `src/app.rs:8348-8350`.

**Files.** New `src/cad_import.rs` (or a `src/cad/` module split), `src/cad.rs`, `src/engineering.rs`, `src/app.rs`, `src/project.rs` (declared-unit semantics).

**Dependencies.** A ZIP reader (`zip`, MIT) and an XML pull parser (`quick-xml`, MIT). `flate2` is already present (`Cargo.toml:16`). Both are pure Rust, permissive, and small. No C or C++ enters the tree.

**Tests.** Round-trip the existing STL fixtures through 3MF and OBJ and assert identical diagnostics and identical masks after unit normalization. Unit-declaration tests: 3MF with explicit `inch`, 3MF with the attribute omitted (must record "defaulted"), OBJ with no unit (must stay `Unknown`). A mismatch test: 3MF declaring millimetre, operator confirming inch, asserting that the mismatch is recorded and visible.

**Risks.** 3MF's manifold requirement is a specification requirement, not an enforcement mechanism — real files still break it, so Phase 0's classification work remains load-bearing. OBJ group semantics are loose (faces may belong to multiple groups), so groups should be recorded and displayed but not promoted to boundary regions until Phase 4.

**Effort.** 5-8 days.

### Phase 2 — Class B repair with full provenance

**Scope.** Vertex welding at an explicit tolerance, degenerate and duplicate face removal, per-shell orientation propagation and signed-volume flip, shell separation with an explicit operator choice of analysis shells. The `ImportStep` record of §5.4, the three evidence classes, the voxel-inert rule, and the derived repaired `SourceRevision`. Repair is opt-in and previewed, never automatic.

**Files.** New `src/mesh_repair.rs`, plus `src/engineering.rs` (preflight fields, issue codes), `src/app.rs` (preview UI, derived revision), `src/report.rs` (repair disclosure in the report).

**Dependencies.** None required. Optionally the MIT SideFX generalized-winding-number approach, ported rather than linked, if Phase 0's parity voting proves insufficient on real customer files.

**Tests.** Every repair asserts its own class invariant: Class B operations must leave enclosed volume unchanged within the stated tolerance, and the assertion runs in the test suite, not only in the UI. Provenance tests: a repaired case's contract must contain the ordered step list, and the analysed hash must differ from the source hash whenever any step is Class B or C. A report test asserting that a Class C repair cannot produce a report without the disclosure line.

**Risks.** Welding tolerance is a genuine physical choice — too large and thin features merge. It must be operator-visible and recorded, defaulting to a fraction of the *voxel pitch* rather than of the bounding-box diagonal (the current diagnostic tolerance at `src/cad.rs:50` is diagonal-relative, which is the wrong reference for a repair).

**Effort.** 8-14 days.

### Phase 3 — STEP via an out-of-process converter

**Scope.** A separate executable, `reyn-cad-bridge`, that links OCCT **dynamically**, reads STEP (and IGES, effectively for free), tessellates with recorded deflection parameters, extracts XDE names/colours/layers, and emits a versioned neutral intermediate: triangles, per-face region tags, declared units, translator identity and options, and healing log. Reyn consumes that intermediate through the Phase 1 abstraction and treats every converter operation as a Class C `ImportStep`.

**Files.** New workspace member outside `reyn-studio`'s dependency graph; `src/cad_import.rs` gains the bridge client; `src/engineering.rs` gains translator provenance fields to satisfy `PRD.md:576-577`.

**Dependencies.** OCCT, dynamically linked, LGPL-2.1 with exception — contained in the bridge process, never in `reyn-studio`. Prototype against `truck-stepio` + `truck-meshalgo` first (Apache-2.0, pure Rust): if their B-rep tessellation handles the real customer files, the entire licence and packaging problem disappears, and that possibility is worth two days of spiking before committing to OCCT.

**Tests.** A corpus of real STEP files exercising AP203, AP214, and AP242 exports from at least three CAD systems. Assert: declared units are read correctly including a non-millimetre case; multiple-unit files are blocked; tessellation deflection is recorded and reproducible; the same input produces byte-identical intermediates across runs; the bridge crashing produces a clean, actionable error rather than a partial import.

**Risks.** Highest of any phase. macOS packaging of dylibs plus code-signing plus notarization plus two architectures; the licence obligation ruling out Mac App Store distribution; tessellation quality driving mask quality in ways that are hard to bound; and a persistent-identity problem that the standards community itself addresses with a recommended practice rather than a mechanism.

**Effort.** 20-35 days, and it should not start until Phases 0-2 have shipped and a customer has actually blocked on STEP.

### Phase 4 — Named regions and the internal-flow contract

**Scope.** Stable region identity computed by Reyn (a hash over face geometry and topology recorded at first import), operator naming and role assignment, a reimport diff reporting preserved/changed/added/removed/ambiguous regions per `PRD.md:578`, and population of `InternalBoundaryAssignment` (`src/engineering.rs:621-631`).

**Files.** `src/engineering.rs`, `src/app.rs`, `src/report.rs`.

**Dependencies.** None beyond Phase 3.

**Tests.** Reimport an edited STEP part and assert the region diff is correct and that changed regions invalidate the case exactly as other edits do. Assert that `execution_blockers` (`src/engineering.rs:759-792`) still refuses to run internal flow without a qualified model and reference suite — that gate must survive this phase untouched.

**Risks.** The temptation to declare internal flow "supported" once assignments exist. `src/engineering.rs:747-749` is explicit that execution remains blocked until a qualified internal model and reference suite ship. This phase produces a *contract*, not a capability.

**Effort.** 15-25 days.

### Phase 5 — Commercial SDK or CAD connector (not scheduled)

Only on validated demand, per `docs/CFD_APP_LANDSCAPE.md:520-531` and `PRD.md:445` (REQ-P-CONNECT-01, P2). A paid SDK buys native formats; an Onshape connector buys source-aware revisions. Neither buys a better occupancy mask.

### 7.1 What NOT to do, and why

1. **Do not write a B-rep kernel.** `PRD.md` already lists this as a non-goal and `docs/CFD_APP_LANDSCAPE.md:550` marks full direct modelling "Do not build". NURBS trimming, tolerance management, and healing are multi-year problems that Reyn's mask consumer cannot exploit.
2. **Do not statically link OCCT.** §4.4. This is the one mistake that is expensive to undo after shipping.
3. **Do not take on any GPL-licensed mesh code.** CGAL PMP, MeshLab, admesh. Reimplement from published papers, or use the MIT SideFX winding-number implementation.
4. **Do not repair silently.** A tool that quietly fixes a customer's geometry and reports numbers against the fixed version is precisely the "random field predictions" failure mode, one level deeper.
5. **Do not add IGES as a first-class target.** The specification is frozen at 5.3 (1996), it has no assembly model, and any converter that reads STEP reads IGES anyway. Accept it as a bonus, never as a phase.
6. **Do not add native CAD readers by hand.** There is no free path. That is what a paid SDK is for, and only when it blocks a sale.
7. **Do not promise PMI, GD&T, or MBD.** Semantic PMI is defined only for exact geometry ([CAx-IF PMI v4.1](https://www.mbx-if.org/home/wp-content/uploads/2024/06/rec_pracs_pmi_v41.pdf)) and carries nothing a voxel mask can consume.
8. **Do not build region naming on STL.** There is no stable identity across a reimport, and `src/app.rs:8451` already tells the user so. Promising it on tessellated input would make the reimport diff lie.
9. **Do not replace ray parity wholesale.** Existing runs must remain reproducible. New classification is an additional, recorded method with a versioned identifier, not a silent substitution.
10. **Do not put any kernel in-process.** Out-of-process isolation buys licence containment, crash isolation, and the freedom to swap OCCT for a paid SDK later without touching the app.

---

## 8. Acceptance criteria

**Phase 0.** No case can reach `ready()` (`src/engineering.rs:491-493`) with a classification-disagreement fraction above threshold. Clean fixtures produce masks bit-identical to the pre-change build. Winding inconsistency appears in the contract, not only in a UI string.

**Phase 1.** A 3MF file declaring inches produces a preflight whose declared unit is `inch`, whose confirmed unit is still whatever the operator selects, and whose contract records both. An STL still produces `declared_units: None` and still blocks on unit confirmation.

**Phase 2.** Any case whose analysed hash differs from its source hash renders a disclosure in the report naming every step, its class, and its measured deltas. Class B operations are covered by volume-invariance assertions in the test suite.

**Phase 3.** P-CAD-01 as written in `PRD.md:576-577`: translator, version, options, source units, repair log, tessellation settings, and output hashes are all recorded. The bridge is a separate process and `cargo tree` for `reyn-studio` shows no LGPL dependency.

**Phase 4.** P-CAD-02 as written in `PRD.md:578`: reimport reports preserved, changed, added, removed, and ambiguous regions and blocks on ambiguity.

---

## 9. Open questions requiring a decision

1. **Distribution channel.** If Mac App Store distribution is ever intended, OCCT is off the table entirely and a paid SDK becomes the only route to STEP. This decision should be made before Phase 3 starts, not during it.
2. **Repair posture.** Does Reyn ever ship geometry-modifying repair (Class C), or does it stop at Class B plus robust classification and tell the operator to fix the part upstream? The second is more defensible and cheaper; the first is what customers will ask for.
3. **Where the region-identity hash lives.** Computed by the bridge (fast, kernel-aware, but couples identity to the translator version) or by Reyn from the neutral intermediate (slower, but survives a translator swap). `docs/CFD_APP_LANDSCAPE.md:518` already warns against letting converter-specific topology IDs become the sole evidence identity, which argues for the second.
4. **Whether to spike `truck`.** Two days against `truck-stepio` + `truck-meshalgo` at git HEAD would establish whether a pure-Rust, Apache-2.0 STEP path is viable for real customer files. The upside — no C++, no licence exposure, no dylib packaging — is large enough to justify the spike before committing to Phase 3's architecture.

---

## 10. Dated bibliography

All URLs checked 2026-07-24.

### Standards and specifications

- ISO, *ISO 10303-242:2025 — Managed model-based 3D engineering* (Edition 4, published 2025-08): https://www.iso.org/standard/84300.html
- ISO, *ISO 10303-242:2022* (Edition 3, withdrawn 2025-08-25): https://www.iso.org/standard/84667.html
- ISO, *ISO/CD 10303-242* (Edition 5, under development): https://www.iso.org/standard/93277.html
- ISO, *ISO 10303-203:2011* (withdrawn 2014-12-01): https://www.iso.org/standard/44305.html
- ISO, *ISO 10303-214:2010* (withdrawn 2014-12-01): https://www.iso.org/standard/43669.html
- ISO, *ISO 14306-4:2026 — JT file format specification, Part 4: Version 3* (published 2026-04-07): https://www.iso.org/standard/86064.html
- ISO, *ISO 14306:2017* (withdrawn 2026-04-07): https://www.iso.org/standard/62770.html
- 3MF Consortium, *3MF Core Specification v1.3.0*: https://3mf.io/wp-content/uploads/sites/55/2025/02/3MF_Core_Specification_v1.3.0.pdf and https://github.com/3MFConsortium/spec_core
- Khronos, *glTF 2.0 Specification* (coordinate system and units): https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html
- NIST, *IGES 5.0 Recommended Practices Guide* (NISTIR 4600; Global Section parameters 14-15): https://nvlpubs.nist.gov/nistpubs/Legacy/IR/nistir4600.pdf
- US PRO, *IGES 5.3* (ANS US PRO/IPO-100-1996), mirror: http://www.paulbourke.net/dataformats/iges/IGES.pdf

### Interoperability recommended practices

- MBx-IF, *CAx Recommended Practices index* (Persistent IDs v1.8, 2026-03-13; PMI Representation and Presentation v4.1, 2024-06-20): https://www.mbx-if.org/home/cax/recpractices/
- CAx-IF, *Recommended Practices for Representation and Presentation of PMI (AP242) v4.1*: https://www.mbx-if.org/home/wp-content/uploads/2024/06/rec_pracs_pmi_v41.pdf
- CAx-IF, *Recommended Practices for Geometric and Assembly Validation Properties v4.5*: https://www.mbx-if.org/home/wp-content/uploads/2024/05/rec_prac_gvp_v45.pdf
- CAx-IF, *Recommended Practices for User Defined Attributes v1.8*: https://www.mbx-if.org/home/wp-content/uploads/2024/05/rec_prac_user_def_attributes_v18.pdf
- NIST, *STEP File Analyzer User's Guide (Version 4)* (semantic vs graphical PMI): https://www.govinfo.gov/content/pkg/GOVPUB-C13-57638b091123bda7b2323ca0ec9e7509/pdf/GOVPUB-C13-57638b091123bda7b2323ca0ec9e7509.pdf
- prostep ivip, *SSB Fact Sheet: ISO 10303-242 (STEP AP242)*: https://www.prostep.org/fileadmin/fact-sheets/Public_SSB_Fact_Sheet__ISO_10303-242__STEP_AP242_-v15-20231219_082046.pdf

### Kernels, translators, and licensing

- Open Cascade, *Licensing* (LGPL-2.1 with the Open CASCADE Exception): https://dev.opencascade.org/resources/licensing
- Open CASCADE Technology, *License* (exception text): https://dev.opencascade.org/doc/occt-7.6.0/overview/html/occt_public_license.html
- Open-Cascade-SAS/OCCT, *Issue #244 — static library in release assets* (maintainer on static linking and the paid exception): https://github.com/Open-Cascade-SAS/OCCT/issues/244
- Open-Cascade-SAS/OCCT, *Release V8_0_0*: https://github.com/Open-Cascade-SAS/OCCT/releases/tag/V8_0_0
- OCCT, *STEP translator user guide* (XDE names/colours/layers; units from `shape_representation`; `xstep.cascade.unit`): https://dev.opencascade.org/doc/occt-7.9.0/overview/html/occt_user_guides__step.html
- OCCT, *IGES translator user guide* (IGES 5.3; non-millimetre values converted to millimetres): https://dev.opencascade.org/doc/occt-7.4.0/overview/html/occt_user_guides__iges.html
- OCCT, *Shape Healing user guide*: https://dev.opencascade.org/doc/occt-7.9.0/overview/html/occt_user_guides__shape_healing.html
- Tech Soft 3D, *FAQ: How units are handled in HOOPS Exchange* (per-format default units): https://techsoft3d.atlassian.net/wiki/spaces/KBHE/pages/503875084/FAQ+How+units+are+handled+in+HOOPS+Exchange
- CAD Exchanger, *SDK pricing* (development fee + distribution fee): https://cadexchanger.com/products/sdk/pricing/
- CAD Exchanger, *Which CAD Exchanger Developer Tools are Right for Me?* (vendor's own order-of-magnitude statement): https://cadexchanger.medium.com/which-cad-exchanger-developer-tools-are-right-for-me-d32562a563bf
- CAD Exchanger, *SDK* (self-contained libraries; SolidWorks on macOS): https://cadexchanger.com/products/sdk/

### Rust crates and libraries

- crates.io, *truck-stepio 0.3.0*, *truck-modeling 0.6.0*, *truck-polymesh 0.6.0*, *truck-meshalgo 0.4.0* (all 2024-09-20, Apache-2.0): https://crates.io/crates/truck-stepio
- GitHub, *ricosjp/truck* (Apache-2.0; last push 2026-07-06): https://github.com/ricosjp/truck
- crates.io, *ruststep 0.4.0* (2024-09-20, Apache-2.0): https://crates.io/crates/ruststep
- crates.io, *occt-sys 0.6.0* ("Static build of the C++ OpenCascade CAD Kernel", LGPL-2.1, 2024-11-30): https://crates.io/crates/occt-sys
- crates.io, *opencascade 0.2.0* / *opencascade-sys 0.2.0* (LGPL-2.1, 2023-08-16): https://crates.io/crates/opencascade
- crates.io, *cadrum 0.8.16* (MIT-declared wrapper, statically linked OCCT 8.0.0, 2026-07-17): https://crates.io/crates/cadrum; changelog: https://github.com/lzpel/cadrum/commit/86e19b76cde3ab9275f6efec07fe09d980eb9716
- GitHub, *elalish/manifold* (Apache-2.0; v3.5.2, 2026-06-27; manifold-input requirement): https://github.com/elalish/manifold
- crates.io, *manifold3d 0.3.3* (Apache-2.0 OR MIT, 2026-07-06): https://crates.io/crates/manifold3d
- crates.io, *mesh_to_sdf 0.4.0* (MIT OR Apache-2.0, 2024-09-17): https://crates.io/crates/mesh_to_sdf
- crates.io, *fast-surface-nets 0.2.1* (MIT OR Apache-2.0, 2025-01-03): https://crates.io/crates/fast-surface-nets
- GitHub, *sideeffects/WindingNumber* (MIT; Barill et al., "Fast Winding Numbers for Soups and Clouds", SIGGRAPH 2018): https://github.com/sideeffects/WindingNumber

### Vendor and practitioner reports

- Ansys, *Fluent User's Guide 2025 R1 — Importing CAD Geometries and Managing CAD Parts* ("Create One Zone Per: object, part, body, or face"): https://ansyshelp.ansys.com/public/views/secured/corp/v251/en/flu_ug/tgd_user_workflow_guided_tasks_import_part_manage.html
- Javelin, *Choosing the Best Neutral File Formats in SOLIDWORKS* (2025-10; AP242 export requires the MBD add-in): https://www.javelin-tech.com/blog/2025/10/choosing-the-best-neutral-file-formats-in-solidworks/
- Siemens community, *Include surface names in STEP 242 export* (reported: AP214 emits face names, AP242 does not): https://community.sw.siemens.com/s/question/0D5KZ00000642js0AA/include-surface-names-in-step-242-export
- Siemens community, *Parasolid export — naming issue* (named faces from NX into STAR-CCM+ via Parasolid): https://community.sw.siemens.com/s/question/0D54O000061x9JqSAI/parasolid-export-naming-issue
- 3D-Tool, *Parasolid Viewer and Converter* ("The Parasolid units are always meters"): https://www.3d-tool.com/cad-files/parasolid-viewer.htm
- McNeel forum, *Parasolid export units* (Rhino: "Parasolid units = meters. Scaling exported geometry by 0.0254"): https://discourse.mcneel.com/t/parasolid-export-units/96361
- Eng-Tips, *Parasolid unit settings* (Siemens: Parasolid data stored in metres): https://www.eng-tips.com/threads/parasolid-unit-settings.157213/
- Wikipedia, *Wavefront .obj file* (no units; `g`/`o`/`usemtl`): https://en.wikipedia.org/wiki/Wavefront_.obj_file
- Encyclopedia of Graphics File Formats, *Wavefront OBJ* (group and material semantics): https://www.fileformat.info/format/wavefrontobj/egff.htm

### Internal references

- `docs/CFD_APP_LANDSCAPE.md` — CAD integration taxonomy (§3.1), Reyn's current CAD state (§1.3), capability ladder and build-versus-integrate (§9).
- `docs/FEATURE_GAP_AUDIT.md` — unit-gate finding (line 19), shipped-capability table (line 94).
- `PRD.md` — REQ-N5-CAD-01 (line 407), REQ-P-CAD-01 (line 439), REQ-P-INTERNAL-01 (line 446), P-CAD-01/02 (lines 576-579), `internal_flow.reference_only.v1` (lines 373-375), Stage 2 neutral B-rep translation (lines 343-347).
- `src/cad.rs`, `src/engineering.rs`, `src/app.rs`, `src/project.rs`, `Cargo.toml` — as cited inline.
