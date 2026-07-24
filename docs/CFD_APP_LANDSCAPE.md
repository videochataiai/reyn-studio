# CFD application landscape for Reyn Studio

**Research date:** 2026-07-23
**Scope:** Current commercial, cloud, and open CFD workflows; CAD strategy; and product implications for Reyn Studio.
**Evidence policy:** Product descriptions below are sourced primarily from vendor documentation and first-party product pages. Statements labeled **Reyn recommendation** are synthesis, not claims about those products. All URLs were checked on the research date.

---

## Executive findings

1. **A real CFD product is organized around durable engineering objects, not around screens.** The recurring hierarchy is portfolio or workspace → project → case/design → setup → run → result/evidence. Ansys Workbench uses connected systems and design points; Autodesk CFD uses Design Study → Design → Scenario; SimScale puts multiple geometries, meshes, simulations, and runs in a project; OpenFOAM uses a transparent case directory. The UI differs, but users can always tell what input produced which result.

2. **CAD is incorporated in several materially different ways.** “Supports CAD” can mean embedded modeling, an associative link to an external CAD system, a managed import/reimport, or only a tessellated/volume mesh input. These are not interchangeable. Discovery and STAR-CCM+ have embedded geometry tools; COMSOL can combine native geometry sequences with LiveLink associativity; Autodesk CFD has CAD launch/update workflows; SimScale pulls geometry from Onshape or Fusion and provides in-workbench preparation; core OpenFOAM consumes triangulated surfaces; SU2 normally consumes an already-generated mesh.

3. **Geometry preparation is a first-class stage even when authoring remains elsewhere.** Repair, defeaturing, fluid-volume extraction, named selections, unit handling, and mesh/discretization checks are visible steps in Fluent, STAR-CCM+, SimScale, COMSOL, Altair, and OpenFOAM front ends. Silent geometry conversion is not the category norm for defensible work.

4. **Solver residuals are necessary but not sufficient.** Mature products expose residuals alongside engineering quantities such as lift, drag, mass flow, forces, moments, surface averages, probes, and conservation checks. Fluent explicitly supports report definitions for convergence and mesh-independence work; SimScale result controls are configured before a run and monitored during it. NASA’s CFD V&V guidance separately requires iterative convergence, consistency, spatial/temporal convergence, and comparison to experimental evidence.

5. **Variants, sweeps, and automation reuse a stable setup.** The high-value pattern is not “more knobs”; it is the ability to change a geometry or declared parameter without rebuilding the entire analysis. STAR-CCM+ Replace Part, Workbench design points, Autodesk Designs/Scenarios, COMSOL parametric sweeps, SimScale parallel parameter runs, HyperStudy, OpenFOAM dictionaries, and SU2 configuration files all implement versions of this idea.

6. **Cloud and collaboration are execution/data-management choices, not prerequisites for CFD.** SimScale is cloud-native; Ansys, Siemens, COMSOL, Autodesk, and Altair support remote or cloud compute; open ecosystems use MPI and external schedulers. Enterprise provenance and collaboration are often handled by separate SPDM systems such as Ansys Minerva or Teamcenter Simulation. Reyn can remain local-first while making execution and evidence portable.

7. **AI-surrogate workflows are moving into the same lifecycle as high-fidelity simulation.** Current products increasingly support dataset selection, training, testing, prediction, and side-by-side high-fidelity comparison: Ansys SimAI, STAR-CCM+ Design Manager with PhysicsAI, SimScale Physics AI, COMSOL Surrogate Model Training, and Altair PhysicsAI. The credible pattern is hybrid: use a surrogate for exploration and validate selected candidates with a high-fidelity solver. Reyn’s evidence-first model qualification is therefore a stronger wedge than trying to match general-purpose CFD breadth.

8. **Reyn’s immediate product gap is the case and evidence lifecycle, not another physics panel.** The current app has strong field interrogation, truth comparison, pressure recovery, a Flow Painter, geometry-conditioned inference, and a promising Benchmark Lab. It does not yet have a saved project/case/run model, explicit geometry-unit and transform review, immutable run lineage, setup staleness, or reproducible reopen. Those are release-critical for a scientific instrument.

9. **The current CAD feature is geometry import plus preprocessing, not embedded or associative CAD.** It accepts STL, auto-fits and voxelizes it, then runs a fixed model contract. That is a useful capability, but it must be labeled precisely. The applied transform, units assumption, grid adequacy, geometry/model support envelope, Reynolds number, and recovered-pressure semantics need to become visible evidence.

10. **Do not build a CAD kernel.** Reyn should own the evidence-sensitive boundary—source identity, units, transforms, geometry diagnostics, voxelization, named regions, and reimport mapping—while integrating a neutral-format translator or a CAD API later. Full direct modeling would consume disproportionate effort and dilute the verification product.

---

## 1. Method and product frame

### 1.1 How to read this study

- **Sourced product fact** means the behavior is stated in an official manual, vendor documentation, first-party release note, or product page linked inline.
- **Observed Reyn fact** means the behavior is visible in the repository’s `README.md`, `PRD.md`, `PRODUCT.md`, `Cargo.toml`, or current Rust/Python source as inspected on 2026-07-23.
- **Reyn recommendation** is the product conclusion drawn from those facts. It is deliberately narrower than a generic CFD feature list.
- Marketing performance claims are not used as requirements. When a vendor page makes a speed or accuracy claim, this study either omits it or treats it only as evidence of product direction.

### 1.2 What Reyn Studio currently is

Reyn Studio is a local-first native Rust/egui/wgpu workbench with a Python PyTorch/solver sidecar. Its strongest implemented product behaviors are:

- responsive 2D and 3D field interrogation with calibrated legends, probes, critical-point pins, and derived quantities;
- model-prediction vs solver-reference comparison, persistence baselines, semigroup self-consistency, and pressure-recovery residuals;
- a native Flow Painter with an explicit divergence-free projection;
- a Benchmark Lab with exact seed-stream classification, selected-cell field/error/spectral evidence, CSV export, and canonical SHA-256 integrity;
- STL ingestion, voxelization, geometry-conditioned prediction, recovered surface pressure, and critical surface points;
- honest handling of known evidence gaps, including the distinction between an integrity digest and a cryptographic signature.

The current product is **not** a general CFD preprocessor or solver. It exposes a small number of trained incompressible-flow contracts and solver-reference paths. That constraint is strategically useful: Reyn can make the contract and its evidence exceptionally legible instead of reproducing the multiphysics breadth of Fluent, STAR-CCM+, COMSOL, or OpenFOAM.

### 1.3 Current-state gaps confirmed in the repository

| Area | Observed state on 2026-07-23 | Product consequence |
|---|---|---|
| Project lifecycle | Schema-v2 New/Open/Save/Save As, recents, autosave, recovery, migration, and strict Project → Case revision → immutable Run → Evidence persistence are wired into `ReynApp`. Portable `.reynproj` documents carry deduplicated SHA-256-addressed source/artifact bytes, deterministic integrity records, and precise missing/corrupt diagnostics; machine paths remain optional hints. | Stored evidence remains locally reviewable when compute or source dependencies are unavailable, and relinking restores content without rewriting completed runs. |
| Navigation | Models and Settings are active local workflows; Flow Painter, Fields (2D), Metrics (3D), and Benchmark Lab remain separate top-level destinations. | Powerful tools exist, but there is no case context tying them together. |
| Model import | The Model Library probes checkpoint structure, supported contracts, and state-dict loadability before copying; rejection is structured and leaves the active model unchanged. | Compatibility and metadata gaps are inspectable, while broader project/model lifecycle integration remains N6 work. |
| CAD | `src/cad.rs` parses STL, auto-scales the largest cross-stream extent to `0.6` solver units, centers it at the trained obstacle station, and voxelizes with ray parity. | This is **geometry import + deterministic preprocessing**. It is neither embedded CAD nor an associative CAD link. |
| Geometry review | The UI reports triangle count, solid voxels, grid, and characteristic length in a status string, but there is no preflight review, units confirmation, transform approval, repair report, or stored source hash. | A scientifically important transformation is effectively silent. |
| Physics setup | CAD prediction defaults to `Re = 150`, derives viscosity from `U=1`, `L=0.6`, and uses the checkpoint’s fixed domain/warmup contract. There is no user-facing materials/BC/initialization case editor. | Reyn should display this as a locked, versioned **physics contract**, not imply arbitrary CFD setup. |
| Surface pressure | The engine returns density-normalized spectrally recovered pressure, and the UI labels its color-normalized surface view “Normalized recovered pressure.” | A physical pressure coefficient remains unavailable. NASA defines \(C_p=(p-p_\infty)/q_\infty\), with \(q_\infty=\rho_\infty V_\infty^2/2\) ([NASA CFPOST equations](https://www.grc.nasa.gov/WWW/winddocs/cfpost/appc.html)); Reyn does not yet record that reference state. |
| Runs | Benchmark Lab writes model-hash-linked immutable runs, rerun parent lineage, precise staleness, deterministic scalar differences, and persisted calibrated selection into the active project. Other interactive prediction paths are not yet migrated. | Benchmark reopen and audit are durable; full cross-workflow case migration and run comparison remain application-level gaps. |
| Evidence | Benchmark suite and selected-cell snapshots are bundled as verified content-addressed artifacts and remain inspectable in read-only evidence mode. CAD and other interactive runs do not yet carry equivalent persisted artifacts. | Benchmark evidence is portable; evidence semantics still need completion across the remaining workflows. |
| Transport | The current engine transfers framed JSON and binary payloads over loopback TCP. Shared memory remains a target optimization in the PRD/README, not the current transport. | Product requirements should not claim the deferred transport acceptance criterion is complete. |

**Reyn recommendation:** treat N5.x and N6 as the transition from an excellent live instrument to a durable case-and-evidence product. Do not expand physics breadth before that transition is complete.

---

## 2. The common CFD product lifecycle

Across products, the durable conceptual model is:

```text
Workspace / portfolio
└── Project
    ├── Sources
    │   ├── geometry revision
    │   ├── material / operating data
    │   ├── model or solver version
    │   └── reference / experimental data
    ├── Case / design / scenario
    │   ├── geometry preparation
    │   ├── discretization / mesh
    │   ├── physics and boundary contract
    │   └── numerical and output controls
    ├── Run (immutable attempt)
    │   ├── runtime history and stop reason
    │   ├── fields and quantities of interest
    │   └── logs, warnings, and environment
    └── Evidence / decision
        ├── comparisons and sensitivity
        ├── V&V and provenance
        └── export / report
```

The important category convention is **dependency visibility**. If geometry, a boundary assignment, a material, a model, or a numerical control changes, downstream mesh/run/result nodes become stale. Ansys Workbench represents this through connected system cells in a project schematic ([Fluent in Workbench User’s Guide, 2025 R2](https://ansyshelp.ansys.com/public/Views/Secured/corp/v252/en/pdf/Ansys_Fluent_in_Ansys_Workbench_Users_Guide.pdf)). SimScale uses required/error/optional/complete states in a top-to-bottom simulation tree and does not permit a run until required settings are complete ([Platform Introduction](https://www.simscale.com/docs/platform/)). COMSOL’s geometry sequence is associative with selections, physics, mesh, and plots ([Model Builder](https://www.comsol.com/comsol-multiphysics/model-builder)). OpenFOAM makes the dependencies inspectable as `0/`, `constant/`, and `system/` case content ([case file structure](https://www.openfoam.com/documentation/user-guide/2-openfoam-cases/2.1-file-structure-of-openfoam-cases)).

**Reyn recommendation:** use this object model without copying an incumbent’s giant simulation tree. A five-stage instrument strip—**Source → Contract → Discretization → Run → Evidence**—can expose readiness and staleness in Reyn’s restrained visual language.

---

## 3. How CAD is actually incorporated

### 3.1 CAD integration taxonomy

| Term | What it means | What it does not mean |
|---|---|---|
| **Embedded CAD** | The analysis product contains geometry authoring/editing operations and a geometry model that participates in the analysis dependency graph. | Merely displaying or tessellating an imported STL. |
| **Associative CAD link** | The simulation retains an identity relationship to an external CAD document/revision and attempts to preserve entity assignments when the CAD changes. | Reimporting a new file by hand with no source identity or mapping report. |
| **Managed geometry import/reimport** | A file or API payload is copied into the project, its identity and transform are recorded, and an update can replace it with explicit assignment mapping. | Full bidirectional parametric CAD. |
| **Geometry preprocessing** | Repair, defeaturing, capping, fluid-volume extraction, simplification, named selections, scaling, and tessellation/voxelization performed for analysis. | Authoritative product design. |
| **Mesh-only ingestion** | The solver receives a surface or volume mesh whose CAD creation and meshing happened elsewhere. | CAD support in the solver itself. |

### 3.2 Product-by-product CAD answer

| Product/ecosystem | Embedded CAD | Associative or managed link | Import and preprocessing | Practical classification |
|---|---|---|---|---|
| **Ansys Discovery / Workbench / Fluent** | Discovery combines interactive geometry modeling and simulation. | Workbench connects Geometry and Fluent/Meshing cells; geometry/design parameters can drive design points. Discovery can update from new CAD and transfer geometry/physics. | Discovery/SpaceClaim repair, defeature, extract fluid volumes, and share topology. Fluent’s guided workflow imports CAD, generates a surface mesh, describes/caps/extracts regions, and creates a volume mesh. | Embedded/direct geometry in Discovery; orchestration and associativity in Workbench; imported geometry plus task-based preprocessing in Fluent. |
| **Simcenter STAR-CCM+** | 3D-CAD is an integrated geometry environment. | Replace Part/Assembly swaps the geometric definition while retaining the simulation object references; Teamcenter adds source traceability. | Native/neutral CAD import, surface repair, wrapping, automated CAD-to-mesh pipelines, morphing, and automated remeshing. | Deep embedded preprocessing with managed replacement and enterprise associativity. |
| **SimScale** | CAD cannot be authored from scratch in the platform. CAD Edit is an integrated prep environment. | Direct Onshape pull and Fusion push integrations; edited geometry is manually promoted into a simulation with entity-mapping feedback. | Import, delete/extrude/scale/split, add CAD, flow-volume extraction, and cleanup. | Managed cloud import plus integrated preprocessing; not an authoritative CAD modeler. |
| **Autodesk CFD** | General product CAD remains in Inventor, Revit, SolidWorks, NX, SpaceClaim, and other CAD systems. | CAD launchers and Design Study Manager coordinate updates; Autodesk explicitly recommends launching from CAD when associativity is needed. | Files can also be opened directly; model assessment and fluid-part creation prepare the analysis. | Associative CAD launch/update or file import; not embedded general CAD. |
| **Autodesk Fusion Electronics Cooling** | Yes: the simulation study lives beside Fusion’s CAD/electronics design. | The study follows the Fusion design workspace. | The solver can automatically simplify components and create the surrounding air domain for this narrow study type. | Embedded, CAD-integrated application-specific simulation—not general-purpose CFD. |
| **COMSOL** | Core geometry sequences create solids/surfaces/curves and Boolean operations. | LiveLink synchronizes supported CAD systems, parameters, materials, selections, and entity associations; topology changes can still break mappings. | CAD Import/Design modules repair and defeature; virtual operations can suppress mesh-impacting details without changing curvature. | Embedded geometry plus optional high-grade associative CAD links. |
| **Altair HyperMesh CFD / AcuSolve** | HyperMesh CFD can create/edit analysis geometry, but it is principally CAE preprocessing rather than the design authority. | Geometry and morph/CAD parameters can feed HyperStudy workflows. | Import, validate, repair/defeature, surface/volume mesh, and local remesh are integrated. | Integrated geometry preparation and design-study automation. |
| **OpenFOAM core** | No CAD kernel. Analytical searchable primitives exist for meshing controls. | No native CAD-document associativity. | `snappyHexMesh` consumes triangulated surfaces such as STL/OBJ/VTK, refines a background hex mesh, snaps, and optionally adds layers. Front ends such as SimFlow translate STEP/IGES/BRep and expose preparation through a GUI. | Mesh/surface-driven core with a broad external preprocessing ecosystem. |
| **SU2 / SU2GUI** | No. | No native CAD-document link. | SU2 consumes `.su2` or supported CGNS meshes; boundary marker tags connect mesh regions to configuration. SU2GUI currently requires a `.su2` mesh. | Mesh-first solver and case GUI; geometry/meshing are external. |
| **Onshape + SimScale** | Onshape is the authoritative cloud CAD system; SimScale is the CFD system. | The connector/API transfers a selected Onshape document state without manual export/upload. Onshape versions and microversions provide immutable revision identities. | Simulation preparation occurs in SimScale. | A useful model for a future optional Reyn connector: source-aware, fileless import rather than embedded CAD. |
| **Altair SimSolid** | Directly analyzes full CAD assemblies. | CAD is the analysis source. | It avoids conventional geometry simplification and meshing. | Structural analysis only, not CFD. It proves that “no mesh UI” is valid only when the solver method truly supports it; it does not justify hiding Reyn’s voxel/discretization limits. |

Supporting sources: [Ansys Discovery](https://www.ansys.com/products/3d-design/ansys-discovery), [Fluent guided meshing](https://ansyshelp.ansys.com/public/Views/Secured/corp/v251/en/flu_wb/flu_tgrd_wb_start_fluent_workflow.html), [STAR-CCM+ CAD preparation](https://blogs.sw.siemens.com/simcenter/cad-preparation-for-cfd-simulation-the-even-easier-way/), [STAR Replace Part](https://blogs.sw.siemens.com/simcenter/star-ccm-v12-04-preview-out-with-the-old-in-with-the-new/), [SimScale CAD preparation](https://www.simscale.com/docs/cad-preparation/), [Autodesk CAD connection](https://help.autodesk.com/cloudhelp/2026/CHT/SimCFD-Self-paced/files/GUID-80AA459D-61D5-4984-8DD9-C47A0092F271.htm), [COMSOL LiveLink specification](https://www.comsol.com/products/specifications/cad/livelinkaa-interface/), [Altair introductory CFD workflow](https://2025.help.altair.com/2025/hwcfdsolvers/acusolve/topics/tutorials/acu/acu_1000_intro_cfd_t.htm), [OpenFOAM `snappyHexMesh`](https://www.openfoam.com/documentation/user-guide/4-mesh-generation-and-conversion/4.4-mesh-generation-with-the-snappyhexmesh-utility), [SU2GUI mesh input](https://su2code.github.io/su2gui/Mesh-File/), [Onshape API architecture](https://onshape-public.github.io/docs/api-intro/architecture/), and [SimSolid product introduction](https://2026.help.altair.com/2026/ss/en_us/topics/simsolid/get_started/product_intro_r.htm).

### 3.3 Direct answer for Reyn Studio

Reyn currently incorporates CAD only in the colloquial sense. Technically it has:

1. **geometry file import:** STL, binary or ASCII;
2. **preprocessing:** deterministic auto-fit, placement, ray-parity voxelization, and smoothing;
3. **analysis coupling:** the resulting mask selects a compatible geometry-conditioned model and initializes a fixed wind-tunnel-like solver contract;
4. **postprocessing:** recovered pressure displayed on the voxelized surface.

It does **not** yet have:

- an embedded B-rep/direct modeling system;
- an associative link to the source CAD document or revision;
- STEP/IGES/native CAD translation;
- durable face/region IDs or named selections;
- explicit source units and transform approval;
- geometry repair/defeaturing operations;
- update/reimport mapping;
- user-controlled domain, boundary conditions, material model, or mesh.

That is not a failure. It is the correct first rung of a staged CAD strategy, provided the UI and evidence artifact say exactly what happened.

---

## 4. Product profiles

### 4.1 Ansys Fluent, Workbench, Discovery, and SpaceClaim

**Sourced product facts**

- Workbench projects are schematic workflows composed of connected systems. Geometry creation/import, meshing, Fluent setup/solve, and CFD-Post can be separate data-integrated applications whose state is saved as one project ([Fluent in Workbench User’s Guide](https://ansyshelp.ansys.com/public/Views/Secured/corp/v252/en/pdf/Ansys_Fluent_in_Ansys_Workbench_Users_Guide.pdf)).
- Workbench parameters can represent geometry, material, pressure, and other analysis values; each design point is a set of input values with calculated outputs ([parameters](https://ansyshelp.ansys.com/public/Views/Secured/corp/v252/en/wb2_help/wb2h_parameters.html), [design points](https://ansyshelp.ansys.com/public/Views/Secured/corp/v252/en/wb2_help/wb2h_designpoints.html)).
- Fluent’s Watertight Geometry workflow is a task sequence for CAD import, surface mesh, geometry description, capping/region extraction, and volume mesh. A similar geometry can be swapped into an existing Workbench workflow when object labels remain compatible ([guided workflow](https://ansyshelp.ansys.com/public/Views/Secured/corp/v251/en/flu_wb/flu_tgrd_wb_start_fluent_workflow.html)).
- Discovery combines interactive geometry modeling with live/high-fidelity simulation, can open/repair/edit imported CAD, and can transfer geometry or physics to Fluent and Workbench ([Discovery product page](https://www.ansys.com/products/3d-design/ansys-discovery)). Its repair tools are explicitly for preparing imported geometry for analysis ([repair documentation](https://ansyshelp.ansys.com/public/Views/Secured/corp/v251/en/discovery/Discovery/user_manual/repair_overview.html)).
- Fluent report definitions compute field, surface, volume, force, moment, and flux quantities during iterations/time steps. They can be plotted, written, used in convergence conditions, and used to verify mesh independence ([monitoring/reporting](https://ansyshelp.ansys.com/public/Views/Secured/corp/v252/en/flu_ug/flu_ug_reporting_sec_monitoring_solution.html), [convergence conditions](https://ansyshelp.ansys.com/public/views/secured/corp/v251/en/flu_ug/flu_ug_convergence_conditions.html)).
- PyFluent exposes meshing and solver sessions, reusable/custom meshing workflows, local/container/remote launch, and scheduler integration ([launching Fluent](https://fluent.docs.pyansys.com/version/stable/user_guide/session/launching_ansys_fluent.html), [meshing workflows](https://fluent.docs.pyansys.com/version/stable/user_guide/meshing/new_meshing_workflows.html)).
- Workbench design points support parametric evaluation; optiSLang adds process integration, DOE/sensitivity work, and robust design optimization across simulation toolchains ([optiSLang 2026 R1](https://www.ansys.com/products/connect/ansys-optislang)).
- Current Fluent supports CPU/GPU and on-prem/cloud execution paths ([Fluent 2026 R1 product page](https://www.ansys.com/products/fluids/ansys-fluent)). Ansys Minerva provides separate SPDM functions such as versioning, lifecycles, branching, workflows, traceability, and shared simulation data ([Minerva](https://www.ansys.com/products/connect/ansys-minerva)).
- Ansys has both a Fluent/Workbench 3D-ROM workflow and SimAI, which trains field predictors from simulation geometry/results and optional operating-condition data ([Fluent ROM setup](https://ansyshelp.ansys.com/public/Views/Secured/corp/v252/en/flu_ug/flu_ug_rom_setup.html), [SimAI](https://www.ansys.com/products/ai/simai), [SimAI data import](https://simai-pro.docs.ansys.com/version/stable/user_guide/data_preparation/step_data_import)).

**Reyn recommendation**

Adopt the visible stage status, reusable workflow, and first-class report-definition ideas. Reject Workbench’s multi-window application fragmentation and Fluent’s full solver-control breadth. Reyn’s equivalent should be one native case view with a narrow, model-supported contract and embedded evidence monitors.

### 4.2 Siemens Simcenter STAR-CCM+

**Sourced product facts**

- STAR-CCM+ presents preprocessing, meshing, multiphysics setup, solving, analysis, visualization, automation, and design exploration in one integrated environment ([product page](https://www.siemens.com/en-gb/products/simcenter/fluids-thermal-simulation/star-ccm/)).
- 3D-CAD is integrated into STAR-CCM+ and supports importing and reducing large assemblies before meshing ([CAD preparation](https://blogs.sw.siemens.com/simcenter/cad-preparation-for-cfd-simulation-the-even-easier-way/)).
- Replace Part/Assembly changes the geometric definition beneath an existing part object, preserving references from mesh operations, boundaries, scenes, and reports where mapping succeeds ([Replace Part](https://blogs.sw.siemens.com/simcenter/star-ccm-v12-04-preview-out-with-the-old-in-with-the-new/)).
- The simulation tree contains setup and instrumentation objects. Tags, filters, groups, and custom trees help manage large object graphs, while simulation-tree comparison exposes differences between revisions ([setup organization](https://blogs.sw.siemens.com/simcenter/the-secret-to-easy-cfd-simulation-setup/), [tree comparison](https://blogs.sw.siemens.com/simcenter/star-ccm-v11-06-building-a-better-sim-file-part-2-of-2/)).
- Simulation templates (`.simt`), query-based selections, simulation operations, and Java macros support repeatable automation ([templates](https://blogs.sw.siemens.com/simcenter/simulation-templates-your-magic-typewriter-for-cfd-automation/)).
- Design Manager supports parameter studies and optimization. Simcenter X supplies managed AWS execution, while Teamcenter Simulation manages geometry relationships, inputs, desired outputs, results, and traceability ([STAR-CCM+ product page](https://www.siemens.com/en-gb/products/simcenter/fluids-thermal-simulation/star-ccm/), [Teamcenter integration](https://blogs.sw.siemens.com/simcenter/going-exploring-with-spdm-plm/)).
- STAR-CCM+ 2602 integrated geometric deep learning into Design Manager, including reuse of existing results and side-by-side CFD/AI prediction comparison; 2606 added multi-GPU training ([2602 release](https://blogs.sw.siemens.com/simcenter/simcenter-star-ccm-2602-released/), [2606 release](https://blogs.sw.siemens.com/simcenter/simcenter-star-ccm-2606-released/)).

**Reyn recommendation**

The strongest transferable idea is that a case is both setup and instrumentation: named selections, monitors, scenes, and evidence should be reusable together. Reyn should also preserve assignments across a geometry update, but with a visible mapping report. Do not reproduce STAR’s tree density; surface only the five lifecycle stages and let expert detail live in contextual inspectors.

### 4.3 SimScale

**Sourced product facts**

- A SimScale project can contain multiple geometries, meshes, and simulations. Each simulation is a top-to-bottom tree whose content is constrained by the selected analysis type; required, error, optional, and complete states gate execution. Compute-intensive jobs run on cloud instances, and the job panel reports status, runtime, and core-hour use ([Platform Introduction](https://www.simscale.com/docs/platform/)).
- SimScale cannot create CAD from scratch, but it accepts uploads, direct Onshape pulls, and Fusion pushes. CAD Edit provides deletion, extrusion, scaling, splitting, adding CAD, and fluid-volume extraction. An edited geometry does not silently replace the simulation geometry; the user explicitly updates it and receives mapping feedback ([CAD preparation](https://www.simscale.com/docs/cad-preparation/), [CAD Edit](https://www.simscale.com/docs/cad-preparation/cad-mode/)).
- Simulation setup includes materials, boundary conditions, mesh, numerical controls, simulation controls, and result controls. Advanced numerical settings exist but defaults are recommended for most cases ([simulation setup](https://www.simscale.com/docs/simulation-setup/)).
- Result controls must be defined before a run and can monitor forces, moments, surface values, field calculations, probes, and convergence. Intermediate fields and convergence plots are available during a run ([result controls](https://www.simscale.com/docs/simulation-setup/result-control/), [multi-purpose analysis](https://www.simscale.com/docs/analysis-types/multi-purpose-analysis/)).
- Projects can be shared with view, copy, or edit permissions; editing is single-writer at a time. The dashboard has spaces/folders, activity, API keys, and public/private project controls ([collaboration](https://www.simscale.com/docs/platform/collaboration/), [dashboard](https://www.simscale.com/docs/platform/dashboard-folders-and-spaces/)).
- Some analyses support parallel parameter runs, with individual run results and an aggregate curve ([multi-purpose parametric study](https://www.simscale.com/docs/analysis-types/multi-purpose-analysis/)). APIs/SDKs are documented as platform interfaces ([API/SDK index](https://www.simscale.com/docs/platform/api-and-sdk-documentation/)).
- Physics AI training selects projects/runs of a supported analysis type, declares varying geometry or boundary inputs, trains a versioned model, and releases it for prediction. The product positions AI prediction alongside, not as a replacement for, high-fidelity validation ([AI model training](https://www.simscale.com/docs/ai-model-training/), [Physics AI](https://www.simscale.com/product/physics-ai/)).

**Reyn recommendation**

Adopt SimScale’s concise setup completeness, explicit geometry promotion, named run, and result-control patterns. Preserve Reyn’s local-first posture: a login, cloud upload, or core-hour economy must not become a prerequisite. Optional remote execution can use the same run contract later.

### 4.4 Autodesk CFD and Fusion simulation

**Sourced product facts**

- Autodesk CFD’s durable hierarchy is **Design Study → Design → Scenario**. A Design represents a unique geometry; Scenarios under it share geometry and vary settings such as materials or operating conditions ([CAD connection and model hierarchy](https://help.autodesk.com/cloudhelp/2026/CHT/SimCFD-Self-paced/files/GUID-80AA459D-61D5-4984-8DD9-C47A0092F271.htm)).
- The standard flow is CAD/fluid-part preparation → materials/boundary conditions/mesh → iterative solve → results → comparison. The Decision Center compares designs and scenarios ([CFD process](https://help.autodesk.com/cloudhelp/2026/ENU/SimCFD-UsersGuide/files/GUID-1DD9447C-E53A-4431-ADAF-BF70E95ED09E.htm)).
- Design Study Manager coordinates CAD launches and updates; Design Study Builder, templates, rules, Solver Manager, Solution Monitor, and Decision Center support repeatable multi-case work ([Design Study automation](https://help.autodesk.com/cloudhelp/2024/ENU/SimCFD-Learning/files/GUID-A31B38D1-7C94-440F-8634-98C13CA8C540.htm)).
- Current 2026 help documents local, remote, and entitled cloud solver choices. Its Python API is primarily used for results extraction while QT scripting automates model setup ([cloud solving](https://help.autodesk.com/cloudhelp/2026/ENU/SimCFD-UsersGuide/files/GUID-21D9C1F2-04CC-460E-9915-B6D36D3C4BF1.htm), [API](https://help.autodesk.com/cloudhelp/2024/ENU/SimCFD-Learning/files/GUID-B56DEB46-56B0-4AB6-9BA9-380E2A208065.htm)).
- Fusion’s CFD-like simulation scope is narrower. Electronics Cooling is a CAD-integrated study that automatically simplifies components and the surrounding air volume, accepts heat/fan inputs, runs a pre-check, solves, and exposes component temperature, air temperature, air velocity, and risk factor ([setup](https://help.autodesk.com/cloudhelp/ENU/Fusion-Simulate/files/SIM-ECOOLING-OVERVIEW-TASK.htm), [study definition](https://help.autodesk.com/cloudhelp/ENU/Fusion-Simulate/files/SIM-E-COOLING-SDY-CONCEPT.htm)).

**Reyn recommendation**

Use the clarity of Design versus Scenario: geometry variants and operating/model variants should not be conflated. A future Reyn **Case Variant** can inherit a geometry revision and change only a supported model parameter, seed, horizon, or reference method. Do not describe Fusion as general CFD or use its application-specific automation to justify hiding Reyn’s model envelope.

### 4.5 COMSOL Multiphysics

**Sourced product facts**

- Model Builder provides one model tree for geometry/CAD, physics, meshing, studies, solvers, visualization, and results ([COMSOL product overview](https://www.comsol.com/comsol-multiphysics), [Model Builder](https://www.comsol.com/comsol-multiphysics/model-builder)).
- Native geometry is a parameterized sequence of operations with associative selections. CAD Import and Design modules add repair/defeaturing; virtual operations can suppress small/sliver entities for meshing without changing the underlying curvature ([Model Builder](https://www.comsol.com/comsol-multiphysics/model-builder)).
- LiveLink synchronizes supported CAD documents and parameters while preserving materials, physics, and boundary assignments on CAD entities when topology permits. The documentation explicitly warns that topology changes can weaken associativity ([LiveLink for SOLIDWORKS](https://www.comsol.com/livelink-for-solidworks), [LiveLink node](https://doc.comsol.com/6.4/doc/com.comsol.help.llsw/llsw_ug_livelink_interface.5.07.html)).
- Physics interfaces supply suitable default study, discretization, solver, and result settings, while retaining expert editability. Studies can contain sequences and sweeps over geometry, physics, material, or function parameters; cluster sweeps distribute parameter sets ([Model Builder](https://www.comsol.com/comsol-multiphysics/model-builder), [Cluster Sweep](https://doc.comsol.com/6.4/doc/com.comsol.help.comsol/comsol_ref_solver.36.043.html)).
- Model Manager versions models and auxiliary CAD/mesh/experimental data, supports comparison/conflict handling, permissions, and a Java API ([Model Manager](https://doc.comsol.com/6.4/doc/com.comsol.help.comsol/model_manager_ref_introduction.55.3.html), [API](https://doc.comsol.com/6.4/doc/com.comsol.help.comsol/model_manager_ref_api.60.01.html)).
- Surrogate Model Training performs DOE sampling and can create DNN, Gaussian-process, polynomial-chaos, or least-squares surrogates. The DNN workflow exposes training and validation loss; GP models can expose predictive standard deviation ([Surrogate Model Training](https://doc.comsol.com/6.4/doc/com.comsol.help.comsol/comsol_ref_solver.36.009.html), [surrogate overview](https://www.comsol.com/blogs/surrogate-models-for-faster-simulations-and-apps/)).

**Reyn recommendation**

Adopt associative selections, explicit study sequences, and a visible difference between model input, derived quantity, and result. Reject equation/multiphysics generality. A Reyn model contract should make unsupported choices impossible instead of exposing a universal solver tree.

### 4.6 Altair HyperMesh CFD, AcuSolve, HyperStudy, and PhysicsAI

**Sourced product facts**

- HyperMesh CFD provides an integrated workflow from CAD import and validation through setup, mesh, AcuSolve launch, and postprocessing. Its Geometry ribbon validates defects and supports defeaturing; Flow sets equations, solver settings, materials, heat sources, and related properties; Mesh controls surface, boundary-layer, volume, and zone meshing; Solution defines monitors and field output ([2025 introductory workflow](https://2025.help.altair.com/2025/hwcfdsolvers/acusolve/topics/tutorials/acu/acu_1000_intro_cfd_t.htm)).
- AcuProbe/AcuTail monitor variables and runtime behavior; HyperMesh CFD Post creates contours and cuts ([basic flow setup](https://help.altair.com/hwdesktop/cfd/topics/tutorials/acu/acu_1000_intro_cfd_t.htm)).
- HyperStudy can drive shape/CAD/solver-parameter DOE workflows, including repeated geometry operations, boundary setup, remeshing, solver execution, and response extraction ([DOE studies](https://help.altair.com/hwdesktop/cfd/topics/pre_processing/morph/doe_studies_t.htm)).
- AcuRun and Altair Compute Console support scalar, shared-memory, MPI, hybrid, local, and remote execution; Altair One provides managed cloud appliances ([AcuRun](https://2025.help.altair.com/2025/hwcfdsolvers/acusolve/topics/acusolve/solver_programs_acurun.htm), [Compute Console](https://help.altair.com/hwcfdsolvers/acusolve/topics/acusolve/run_acusolve_from_altair_compute_console_r.htm), [Altair One appliances](https://2025.help.altair.com/altairone/topics/get_started/hpc_appliances_altairone.htm)).
- PhysicsAI is a geometric-deep-learning workflow for fields, KPIs, and curves from CAD/mesh and other physical inputs. Its CFD documentation distinguishes supported use envelopes and notes that prediction quality depends on similarity to training data ([PhysicsAI](https://2026.help.altair.com/2026/simlab/help/en_us/topics/PhysicsAI/physicsAI.htm), [CFD capabilities](https://2026.help.altair.com/2026/hwdesktop/cfd/topics/chapter_heads/physicsAI_r.htm)).
- `romAI` is distinct: it creates system-level, real-time reduced models and does not reproduce fields ([romAI FAQ](https://help.altair.com/compose/help/en_us/topics/reference/romai_faq.htm)).

**Reyn recommendation**

Adopt explicit geometry validation, monitor/output setup, and train/test/predict separation. Altair’s distinction between field predictors and system-level ROMs is important: Reyn should identify model output topology and intended use, not use “AI model,” “surrogate,” and “ROM” as synonyms.

### 4.7 OpenFOAM and front ends

**Sourced product facts**

- A core OpenFOAM case is transparent: time directories contain initial/boundary fields and results; `constant/` holds mesh and physical properties; `system/` holds run controls, discretization schemes, solver tolerances, and algorithms ([case structure](https://www.openfoam.com/documentation/user-guide/2-openfoam-cases/2.1-file-structure-of-openfoam-cases)).
- Boundary conditions are declared for every solved field and tied to named mesh patches. Physical-property dictionaries and `controlDict` make setup and output behavior inspectable ([boundary conditions](https://www.openfoam.com/documentation/user-guide/5-models-and-physical-properties/5.1-boundary-conditions), [run/data control](https://www.openfoam.com/documentation/user-guide/6-solving/6.1-time-and-data-inputoutput-control)).
- `snappyHexMesh` reads triangulated geometry, refines and snaps a background hex mesh, optionally adds layers, checks quality, and runs in parallel ([meshing](https://www.openfoam.com/documentation/user-guide/4-mesh-generation-and-conversion/4.4-mesh-generation-with-the-snappyhexmesh-utility)).
- Runtime output includes residuals, iterations, Courant number, continuity, and execution time; `foamLog` extracts histories. ParaView/`paraFoam` is the primary postprocessor ([monitoring](https://www.openfoam.com/documentation/user-guide/6-solving/6.4-monitoring-and-managing-jobs), [postprocessing](https://www.openfoam.com/documentation/user-guide/7-post-processing/7.1-parafoam)).
- Domain decomposition and MPI provide parallel execution ([parallel guide](https://www.openfoam.com/documentation/user-guide/3-running-applications/3.2-running-applications-in-parallel)). Shape/topology optimization and DMD-based field reconstruction exist in the core ecosystem ([adjoint optimization manual](https://www.openfoam.com/documentation/files/adjointOptimisationFoamManual_v2312.pdf), [DMD field reconstruction](https://www.openfoam.com/news/main-news/openfoam-v2312/post-processing)).
- OpenFOAM v2606 introduced `pybFoam` Python bindings for fields, operators, models, meshing, and embedded solvers, explicitly enabling data/ML workflows without defining one opinionated model lifecycle ([v2606 plugins](https://www.openfoam.com/news/main-news/openfoam-v2606/plugins)).
- SimFlow is a representative commercial front end that packages geometry import, hex-dominant meshing, physics/BC setup, real-time convergence and force monitoring, and ParaView-based postprocessing over OpenFOAM ([SimFlow](https://sim-flow.com/), [simulation setup](https://help.sim-flow.com/documentation/panels/setup)).

**Reyn recommendation**

Borrow the inspectable case-manifest idea and adapter-friendly headless execution, not the burden of exposing every dictionary. A portable Reyn project should contain a human-readable manifest even if large arrays remain binary/content-addressed. OpenFOAM also demonstrates why a GUI should not erase the underlying contract.

### 4.8 SU2 and SU2GUI

**Sourced product facts**

- SU2’s fundamental case inputs are a configuration file and an unstructured mesh. Boundary marker tags in the mesh are referenced by boundary-condition options in the configuration ([configuration file](https://su2code.github.io/docs/Configuration-File/), [mesh file](https://su2code.github.io/docs/Mesh-File/)).
- SU2_CFD prints convergence updates and writes histories, volume/surface results, and restart data. Formats include VTK/ParaView, Tecplot, CGNS, and CSV depending on the path ([postprocessing](https://su2code.github.io/docs/Post-processing/)).
- MPI/Python scripts orchestrate parallel solves, mesh deformation, direct/adjoint evaluation, and shape optimization; `shape_optimization.py` repeatedly runs direct and adjoint analyses and deforms the mesh with SU2_DEF ([software components](https://su2code.github.io/docs/Software-Components/), [execution](https://su2code.github.io/docs/Execution/)).
- SU2GUI creates/loads cases, edits configuration, runs the solver, and analyzes results, but currently requires an existing `.su2` mesh ([SU2GUI introduction](https://su2code.github.io/su2gui/Introduction/), [mesh input](https://su2code.github.io/su2gui/Mesh-File/)).
- SU2 has targeted ML integration—for example a physics-informed neural-network equation of state—but its official workflow is not a general in-product field-surrogate registry ([data-driven fluid tutorial](https://su2code.github.io/tutorials/NICFD_nozzle_datadriven/)).

**Reyn recommendation**

SU2 validates the value of a small, reproducible case bundle and headless orchestration. A post-N6 SU2/OpenFOAM reference adapter would be strategically more valuable than implementing another in-house high-fidelity solver: Reyn could ingest trustworthy reference fields and concentrate on comparison, model qualification, and evidence.

---

## 5. Cross-category conventions

### 5.1 Conventions worth adopting

| Convention | Evidence in the category | Reyn interpretation |
|---|---|---|
| Durable project/case/run hierarchy | Workbench systems/design points; Autodesk Design Study/Design/Scenario; SimScale Project/Simulation/Run; OpenFOAM case | Introduce Project → Case → immutable Run before v1. |
| Explicit completeness and staleness | Workbench cell states; SimScale tree states; COMSOL dependency graph | Every stage says Draft, Ready, Running, Complete, Stale, Failed, or Evidence-locked. |
| CAD semantics are declared | Discovery/SpaceClaim, STAR 3D-CAD, COMSOL LiveLink, Autodesk CAD launch, SimScale CAD Edit | Label Reyn as geometry import/preprocessing; show source identity, units, and transform. |
| Named selections survive downstream work | COMSOL selections; STAR object/query references; SimScale entity sets; SU2/OpenFOAM boundary markers | Add named regions only when they have stable identity and an update-mapping report. |
| Preflight before expensive work | Geometry validation/repair and mesh quality across Fluent, Altair, SimScale, OpenFOAM | Block or explicitly waive invalid geometry, inadequate voxel resolution, unsupported model regime, and incomplete metadata. |
| Analysis-type-specific setup | SimScale short tree; COMSOL physics defaults; SimFlow solver wizard | Expose only controls supported by the selected Reyn model/solver contract. |
| Materials, physics, and BCs are separate but linked | Fluent/STAR/COMSOL/Altair distinguish physical models and materials from entity/patch assignments; OpenFOAM/SU2 tie field/BC definitions to named patches | Show a locked physics contract separately from geometry-region assignments; never imply unsupported freedom. |
| Physical and numerical controls are not conflated | Mature tools separate operating conditions/models from solver algorithms, tolerances, time stepping, initialization, and output cadence | Keep model horizon/reference physics separate from pressure-recovery method, tolerance, device, and output controls. |
| Residuals plus quantities of interest | Fluent reports; SimScale result controls; AcuProbe; OpenFOAM function objects | Monitor trust/error/conservation and engineering outputs, not one generic progress spinner. |
| Immutable, named runs | SimScale runs; Autodesk scenarios; OpenFOAM time/restart artifacts | Never silently replace evidence. A rerun creates a new run with lineage. |
| Postprocessing definitions are saved with the case | Fluent reports/scenes, STAR reports/scenes, SimScale result controls, COMSOL Results, OpenFOAM function objects | Persist fields, legends, probes, quantities of interest, and comparison definitions rather than treating views as transient UI. |
| Shared scales and direct comparison | CFD postprocessors, Autodesk Decision Center, current Reyn truth overlay | Keep Reyn’s shared scales; add variant and run comparison. |
| Sweeps inherit a qualified setup | Workbench design points, STAR Design Manager, Autodesk scenarios, COMSOL sweeps, HyperStudy, SimScale parameter runs | A variant changes declared parameters and records invalidation; it does not duplicate opaque state. |
| Templates and automation use the same contract | STAR templates/macros, PyFluent, HyperStudy, OpenFOAM/SU2 text cases | Make the persisted project schema usable by both GUI and future CLI/API. |
| V&V and provenance are durable evidence | Solver monitors support numerical checks; Minerva, Teamcenter, and Model Manager add lifecycle/traceability | Embed a minimal run manifest and derivation graph locally; leave enterprise governance optional. |
| Hybrid surrogate/high-fidelity loop | SimAI, STAR PhysicsAI, SimScale Physics AI, COMSOL, Altair | Keep solver truth and independent benchmark evidence beside predictions; do not show an unqualified “AI confidence” score. |
| Separate data management from compute | Minerva, Teamcenter, Model Manager; cloud/cluster backends | Keep the project local and portable; let execution backend be replaceable. |

### 5.2 Conventions worth rejecting

1. **The universal simulation tree.** Reyn’s target users do not need hundreds of multiphysics nodes. A compact lifecycle with contextual evidence is more aligned with the scientific-instrument design contract.
2. **Ribbons and app-to-app handoffs.** Workbench’s breadth requires separate applications; Reyn should preserve one coherent native workspace.
3. **“One-click” setup that hides assumptions.** Automation is useful only when units, transforms, model support, and reference conditions remain inspectable.
4. **Residual-only convergence.** Residual decay can coexist with drifting forces, poor conservation, or a wrong physical model.
5. **Unqualified green status.** “Clean” must name the test performed. Exact RNG-stream non-overlap is not field-space leak analysis; a SHA-256 digest is not a signature; a semigroup score is not ground-truth error.
6. **Calling tessellated import embedded CAD.** This would set the wrong expectation for editing, associativity, and region persistence.
7. **Calling recovered pressure \(C_p\) without a reference state.** Display normalization is not physical nondimensionalization.
8. **Cloud-required collaboration.** It conflicts with local-first use, confidential models, offline laboratories, and deterministic packaging.
9. **Black-box surrogate confidence.** If calibration, support envelope, holdout provenance, and error definition are absent, the UI should say “unknown,” not infer trust from visual plausibility.
10. **General-purpose material and BC editors before compatible models exist.** Unsupported freedom is worse than an honest locked contract.
11. **Building a full CAD kernel.** It is outside Reyn’s wedge and introduces translation, topology, persistent naming, repair, tessellation, and licensing problems before the evidence workflow is mature.

---

## 6. Validation, provenance, and what “evidence-first” should mean

NASA distinguishes **verification**—whether the computational implementation and calculation solve the equations correctly—from **validation**—whether the model agrees with physical reality ([NASA overview](https://www.grc.nasa.gov/www/wind/valid/tutorial/overview.html)). Its validation process calls for iterative convergence, consistency checks such as conservation, spatial and temporal convergence, comparisons with experimental data, and model-uncertainty assessment ([validation assessment](https://www.grc.nasa.gov/www/wind/valid/tutorial/valassess)). ASME V&V 20 similarly frames validation as a quantified comparison at specified validation variables/points that includes both numerical and experimental uncertainty ([ASME V&V 20-2009, reaffirmed 2021](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-fluid-dynamics-and-heat-transfer)).

That leads to a strict evidence vocabulary for Reyn:

| Reyn term | Required meaning |
|---|---|
| **Prediction** | Output from a named checkpoint/model and declared input contract. |
| **Solver reference** | Output from a named numerical solver/configuration, not automatically “truth.” |
| **Analytical reference** | A documented exact/benchmark solution in its stated regime. |
| **Experimental reference** | Measurements with source, setup, units, and uncertainty metadata. |
| **Recovered quantity** | A quantity computed from prediction/reference fields by a named method and residual/tolerance, e.g. pressure recovery. |
| **Derived quantity** | A deterministic transform such as vorticity, Q-criterion, or spectrum with method/version. |
| **Verification evidence** | Conservation, residual, discretization sensitivity, solver/implementation checks, and numerical uncertainty. |
| **Validation evidence** | Comparison against experimental/physical observations within a declared use domain. |
| **Consistency evidence** | Self-consistency such as semigroup error; useful without a reference, but not accuracy. |
| **Provenance evidence** | Source identity, version/hash, split/seed lineage, environment, parameters, and derivation graph. |
| **Integrity** | Detection of artifact modification, e.g. SHA-256. |
| **Authenticity/signature** | Cryptographic evidence that a named key signed the artifact. |
| **Support envelope** | The geometry/physics/parameter region represented by training and evaluation. |

**Reyn recommendation:** every visible number should be able to answer: *what produced it, under which contract, compared with what, and with which limitations?* That is a stronger and more defensible differentiator than a generic “trust score.”

---

## 7. Key Reyn user journeys

### Journey A — Imported geometry to defensible prediction

1. Create or open a project.
2. Create a **Geometry case** and import STL initially; later STEP or a connector revision.
3. Review source name/hash, units, extents, orientation, watertightness/degeneracy, applied transform, voxel resolution, smallest resolved thickness, and model support.
4. Confirm or change the explicit transform; never silently auto-fit.
5. Review the locked physics/model contract: domain, reference velocity/length, Reynolds number, viscosity, boundary template, initialization, grid, horizon, and supported ranges.
6. Run. Observe stage/runtime, stop reason, engine/device, and available consistency/reference monitors.
7. Inspect velocity, vorticity, recovered pressure, derived structures, surface pressure, and critical points. Each layer carries a source tag.
8. Export an evidence artifact tied to the immutable run.

**Success test:** another user can open the project, see every transformation and assumption, reproduce the run or understand why it cannot be reproduced, and verify the evidence hash/signature.

### Journey B — Qualify a neural flow model

1. Import a checkpoint into the Model Library.
2. Validate shape/channel/grid/conditioning contracts and read checkpoint metadata.
3. Review model card: source hash, training dataset fingerprint, split/seed policy, selection role, supported geometry/physics/horizon, known gaps, and previous reports.
4. Choose an independent benchmark protocol; default seeds remain explicit.
5. Run the suite with runtime and progress visibility.
6. Inspect global results, per-cell velocity/error/divergence/spectrum, and field-space leak/trajectory overlap when available.
7. Lock a report card and sign it with an organization key.

**Success test:** no validation/checkpoint-selection data is presented as independent test evidence; every CLEAN/FLAGGED/UNKNOWN statement names the exact test.

### Journey C — Flow Painter experiment

1. Create a **Painted IC case**.
2. Paint or apply a preset; inspect enstrophy and symmetry semantics.
3. Apply or auto-apply the Leray projection; record residual/iterations and divergence check.
4. Select a compatible model and horizon inside its support envelope.
5. Predict; use semigroup consistency when no reference exists.
6. Optionally run the spectral solver reference once viscosity/regime are explicitly chosen.
7. Compare on shared scales and export the run.

**Success test:** “no reference” remains visible; adding a solver reference creates a new derived/reference run rather than rewriting the original prediction.

### Journey D — Compare a small design family

1. Duplicate a case as a variant or attach a new geometry revision.
2. Change only declared inputs: geometry revision, supported Reynolds number, model, seed, or horizon.
3. Review which assignments mapped, which stages became stale, and which settings are inherited.
4. Run a bounded sweep locally or through an execution backend.
5. Compare quantities of interest, error/trust evidence, runtime, and applicability on shared axes/scales.
6. Promote selected variants for high-fidelity reference evaluation.

**Success test:** each point on a curve opens the exact run and evidence that produced it.

### Journey E — Review or reproduce an existing result

1. Open a portable project/evidence bundle.
2. Verify schema, source/model hashes, signature, and app/engine compatibility.
3. Enter read-only mode if required dependencies are absent.
4. Inspect fields, logs, evidence, warnings, and lineage without rerunning.
5. If dependencies are available, rerun into a new immutable run and compare with recorded tolerances.

**Success test:** missing dependencies degrade honestly; they do not erase the original evidence or fabricate an online state.

---

## 8. Feature hierarchy and information architecture

### 8.1 Product objects

```text
Project
├── Sources
│   ├── GeometrySource[]
│   ├── ModelSource[]
│   └── ReferenceSource[]
├── Cases
│   └── Case[]
│       ├── source revision
│       ├── physics/model contract
│       ├── discretization record
│       ├── view definitions
│       └── Run[] (immutable)
├── BenchmarkProtocols
├── EvidenceArtifacts
└── ProjectEvents
```

Minimum source fields:

- stable project/case/run IDs;
- schema version;
- source URI/path bookmark where permitted, original filename, byte size, SHA-256, and import time;
- units and coordinate frame;
- applied transform matrix and reason;
- app, engine, model, solver, and converter versions/hashes;
- exact settings and random seeds;
- parent run/revision IDs;
- warnings, waivers, and stop reason;
- artifact hashes and optional signatures.

### 8.2 Recommended navigation

Keep the current warm-dark, low-chrome, monospaced-measurement instrument aesthetic. Reorganize without turning Reyn into a generic dashboard:

- **Projects** — recent/local projects and recovery, shown only after persistence exists.
- **Cases** — current project’s cases and variants.
- **Models** — validated model library and model cards.
- **Benchmark Lab** — protocols, suites, selected-cell evidence.
- **Evidence** — locked reports, comparisons, references, and signatures.
- **Settings** — compute/engine, storage, privacy, appearance, signing keys.

Inside a case, use compact tabs:

- **Setup** — Source, Contract, Discretization, Outputs.
- **Fields 2D**
- **Volume 3D**
- **Compare**
- **Run history**

Flow Painter and CAD import become **case source types**, not disconnected product islands. They can retain direct shortcuts in the rail until the project shell is complete.

### 8.3 Visual rules

- Use one horizontal lifecycle ledger rather than a dense universal tree.
- Keep critical measurements in JetBrains Mono, shared physical colors, calibrated legends, and explicit units.
- Reserve ember for the next primary action; gold/blue/green/red remain semantic data colors.
- Never use glow on annotations or evidence states; current 1 px pins and compact chips are appropriate.
- Keep **Unsaved project** until a persisted project is active.
- Keep timer-driven inference labeled as the literal **Auto-advance prediction** action, not as a connected experiment or continuously solved state.

---

## 9. CAD strategy: build the evidence boundary, integrate the kernel

### 9.1 Recommended capability ladder

#### Stage 0 — Current: tessellated import

- STL input;
- deterministic parser/voxelizer;
- fixed trained-domain placement;
- no associativity.

Keep it, but describe and record it accurately.

#### Stage 1 — Source-aware import and preflight

Build in Reyn:

- source hash and unit declaration;
- transform/orientation preview and explicit confirmation;
- watertight/open-edge, duplicate/degenerate triangle, component count, normal consistency, and bounds diagnostics;
- voxel adequacy metrics: solid fraction, cells across minimum thickness, disconnected components after voxelization, boundary clearance, and resolution warning;
- editable placement only within model-supported limits;
- immutable geometry revision and reimport diff;
- user-defined named regions only where the representation can preserve them.

This stage provides most of the evidence value without a CAD kernel.

#### Stage 2 — Neutral B-rep translation

Integrate a converter behind an isolated process boundary:

- **Open CASCADE Technology** supports STEP translation and configurable shape healing ([OCCT 7.9 STEP translator](https://dev.opencascade.org/doc/occt-7.9.0/overview/html/occt_user_guides__step.html), [Shape Healing](https://dev.opencascade.org/doc/occt-7.9.0/overview/html/occt_user_guides__shape_healing.html)). It minimizes license cost but adds C++ integration, packaging, tolerance, and persistent-naming work.
- A commercial SDK such as **CAD Exchanger** supports STEP, IGES, Parasolid, and native formats with assemblies/metadata and healing ([supported formats](https://docs324x.cadexchanger.com/sdk/sdk_supported_formats.html), [SDK](https://cadexchanger.com/products/sdk/)). It reduces translator breadth/support risk but adds license, binary, platform, and commercial-dependency costs.

Use the converter to emit a versioned neutral intermediate plus tessellation/region metadata. Do not let converter-specific topology IDs become the sole evidence identity.

#### Stage 3 — Source-aware CAD connector

Pilot Onshape only if design partners request it:

- store document/workspace/version/microversion/element/configuration IDs;
- import immutable versions or pin a workspace microversion;
- fetch STEP or tessellation through official export APIs ([Onshape import/export API](https://onshape-public.github.io/docs/api-adv/translation/));
- require explicit refresh;
- show added/removed/changed regions and assignment mapping;
- preserve local cached geometry and evidence for offline reopen.

This is associative **source tracking**, not embedded CAD.

#### Stage 4 — Embedded editing only if the product changes

Do not plan full direct modeling unless repeated customer evidence shows that:

- geometry editing is a top-three blocker;
- neutral import/prep and CAD connectors are insufficient;
- users will accept the packaging/licensing footprint;
- Reyn intends to compete in design authoring rather than model verification.

### 9.2 Build-versus-integrate decision

| Capability | Build | Integrate | Decision |
|---|---|---|---|
| Source identity, hashes, units, transform, evidence manifest | Strong Reyn differentiation; small, testable scope | Generic SDKs do not know Reyn’s model contract | **Build** |
| STL parsing and model-specific voxelization | Already implemented and model-coupled | General mesh tools add little near-term value | **Build/retain** |
| Geometry diagnostics tied to voxel/model support | Product-specific | Generic repair tools cannot decide model applicability | **Build** |
| STEP/IGES/native CAD translation and healing | Large format/tolerance burden | Mature kernels/SDKs already exist | **Integrate** |
| Full direct modeling | Very high complexity and off-wedge | Discovery, STAR, COMSOL, and CAD products already own this | **Do not build** |
| Associative cloud CAD | Auth, revision, export, mapping, governance burden | Onshape exposes revision and export APIs | **Optional connector** |
| High-fidelity meshing/solver | Multi-year general CFD scope | OpenFOAM/SU2 and commercial solvers already expose automation/results | **Integrate references, do not clone** |

---

## 10. Research-informed roadmap

### 10.1 Near-term N5.x — finish evidence, correct scientific semantics

#### N5.3 — complete the already-declared Benchmark Lab scope

- field-space nearest-training-IC and trajectory-overlap analysis;
- per-variable velocity/vorticity/pressure inspector;
- spatial divergence map;
- PNG/PDF evidence export;
- organization-key signing;
- explicit schema/version and verification instructions for the report card.

Do not broaden N5.3 into project management. Finish the promised evidence claims first.

#### N5.4 — CAD and run evidence integrity

1. Add a CAD import review step with source hash, units, extents, triangle/component/defect counts, proposed transform, target grid, cells-across-feature estimate, and model support.
2. Require explicit confirmation of unknown units and auto-fit transform.
3. Record model, geometry, transform, grid, Reynolds number, characteristic length, horizon, warmup/initialization, solver/reference method, pressure-recovery method/residual, engine/app versions, random seed, device, runtime, and warnings in every CAD evidence export.
4. Keep the current **Normalized recovered pressure** label until physical \(C_p\) is computed from recorded \(p_\infty,\rho_\infty,V_\infty\).
5. Add a visible source badge to each layer: **MODEL**, **SOLVER REFERENCE**, **RECOVERED**, or **DERIVED**.
6. Show the selected model’s support envelope and treat out-of-support conditions as warning or blocked, never silently valid.

### 10.2 N6 — make v1 a durable local scientific instrument

The original N6 Models/Settings/Import/packaging scope remains valid, but a minimal project lifecycle is a v1 release gate. Split N6:

#### N6.1 — Model Library and settings

- model cards with hash, metadata/provenance status, contract compatibility, support envelope, benchmark reports, and known limitations;
- import validation before activation;
- compute device, engine path/status, storage, privacy/telemetry-off-by-default, appearance, and signing-key settings;
- explicit distinction between bundled, local imported, and missing model artifacts.

#### N6.2 — Project, case, and run substrate

- New/Open/Save/Save As, recent projects, autosave, crash recovery, and schema migration;
- Project → Case → immutable Run IDs;
- portable `.reynproj` bundle or directory with human-readable manifest and content-addressed binary artifacts;
- stage readiness/staleness and dependency invalidation;
- run history, parent lineage, compare, and evidence locking;
- read-only reopen when model/engine dependencies are unavailable;
- no hardcoded fake project state.

#### N6.3 — standalone packaging

- bundled/pinned Python engine and model acquisition with checksums;
- codesign/notarization;
- first-run dependency verification;
- clean-machine and offline-reopen tests;
- release documentation that states supported macOS/hardware and current model contracts.

The previous three-day N6 estimate is no longer credible if project persistence is included. Treat N6 as a release phase, not a packaging chore.

### 10.3 Post-N6, in order

#### P1 — Case templates and external references

- versioned, model-supported physics templates rather than a universal BC editor;
- explicit operating conditions, reference scales, and quantities of interest;
- import reference fields/curves from VTK, CSV, CGNS, or a narrow OpenFOAM/SU2 adapter;
- solver-reference provenance and uncertainty metadata;
- three-resolution discretization study support where the model/solver path permits it.

#### P2 — Geometry revision and neutral CAD

- source-aware reimport with mapping/staleness report;
- STEP support through an isolated translator;
- named regions and face/part metadata;
- repair/defeature recommendations before destructive automatic actions;
- no native proprietary format promise until partner demand and licensing are understood.

#### P3 — Variants, sweeps, and headless automation

- case inheritance and explicit parameter definitions;
- bounded local sweep queue with resource limits;
- aggregate curves linked to immutable runs;
- CLI/API using the same project schema;
- resume/cancel/retry and per-run stop reason;
- optional remote execution backend only after local headless runs are deterministic.

#### P4 — Collaboration and compute

- signed, read-only evidence bundle first;
- project comments/review annotations only if they preserve local ownership;
- optional artifact sync, organization policy, and remote/HPC adapter;
- no mandatory Reyn cloud account;
- role/permission model only when multiuser storage exists.

#### P5 — Associative CAD pilot

- Onshape connector with pinned revision IDs and explicit refresh;
- mapping-quality report and user resolution for lost regions;
- no implicit mutation of a completed/evidence-locked run.

#### P6 — Surrogate lifecycle and active learning

- dataset registry with immutable split membership and source fingerprints;
- training/validation/test distinction and checkpoint-selection lineage;
- scalar and field error metrics, calibration, and applicability tests;
- candidate selection for high-fidelity evaluation;
- active-learning loop that records why a sample was added;
- side-by-side surrogate and high-fidelity results;
- exportable model card and evidence, not a generic “AI confidence” badge.

---

## 11. Acceptance criteria

### 11.1 N5.x evidence gates

- **N5X-EV-01:** Every exported run/report contains a stable run UUID and explicit unsaved-session UUID before N6; after the N6 project substrate lands, project and case IDs are also mandatory. It always records UTC timestamp, schema version, app/engine version, model identifier and SHA-256, exact settings/seeds, runtime device, stop reason, and artifact digests.
- **N5X-EV-02:** Every displayed field or scalar can be classified as model prediction, solver/analytical/experimental reference, recovered quantity, or derived quantity; the class and method are visible without opening a report.
- **N5X-CAD-01:** Before CAD prediction, the user sees source filename/hash, declared units, original extents, triangle count, connected-component count, open/non-manifold/degenerate diagnostics, proposed transform, target grid, solid-voxel count estimate, and resolution warnings.
- **N5X-CAD-02:** Unknown units or auto-fit require an explicit confirmation. The exact 4×4 transform and unit conversion are stored and exported.
- **N5X-CAD-03:** A mesh that voxelizes empty, touches forbidden domain boundaries, creates disconnected artifacts, or has critical thickness below the documented cell threshold is blocked or requires a named waiver recorded in evidence.
- **N5X-CAD-04:** The active model card shows supported grid, conditioning channels, geometry regime, Reynolds/viscosity range, horizon, and training envelope. Unsupported inputs cannot receive an unqualified pass state.
- **N5X-PHYS-01:** The label `Cp` appears only when the implementation records \(p_\infty,\rho_\infty,V_\infty\) and computes \(C_p=(p-p_\infty)/(0.5\rho_\infty V_\infty^2)\). Otherwise the UI and exports say recovered pressure.
- **N5X-VV-01:** `CLEAN` from seed provenance states “no collision in checked RNG streams” and cannot imply field-space non-overlap until that analysis succeeds.
- **N5X-VV-02:** Field-space/trajectory checks report algorithm, representation, threshold, candidate set, and nearest matches; missing training data returns UNKNOWN.
- **N5X-SIGN-01:** Integrity hash and signature are separate fields. A signed report includes algorithm, key ID, signature bytes, signed canonical payload hash, and a documented verification command; absent/revoked keys do not produce “signed.”

### 11.2 N6 project and packaging gates

- **N6-PROJ-01:** New, Open, Save, Save As, autosave, crash recovery, and recent projects work without the Python engine.
- **N6-PROJ-02:** A saved project reopens to the same cases, source/model hashes, run history, selected views, calibrated scales, warnings, and evidence links.
- **N6-PROJ-03:** Changing a source, transform, model, or contract marks only dependent stages/runs stale. Existing immutable runs remain inspectable and are never rewritten.
- **N6-PROJ-04:** A rerun creates a new run ID with a parent/reference to the prior run. Identical deterministic inputs reproduce declared scalar values within documented tolerance; differences are reportable.
- **N6-PROJ-05:** A project with missing model/engine files opens read-only with precise missing-dependency messages; stored results and evidence remain available.
- **N6-PROJ-06:** Project manifests contain no required machine-specific absolute path. External bookmarks may be stored as hints but bundled hash-addressed sources remain authoritative.
- **N6-PROJ-07:** Schema migrations are versioned, tested from every shipped schema, and never silently discard evidence fields.
- **N6-MODEL-01:** Importing an incompatible or malformed checkpoint leaves the active model unchanged and produces a structured validation result.
- **N6-MODEL-02:** Model cards distinguish metadata-backed facts from unknown legacy fields and link all benchmark reports by hash.
- **N6-PKG-01:** A notarized app launches on a clean supported Mac, verifies bundled engine/model checksums, creates/saves/reopens a project, runs one bundled smoke case, and exports verifiable evidence without a terminal.
- **N6-PKG-02:** Offline launch and read-only project review work with telemetry disabled and no account.

### 11.3 Post-N6 gates

- **P-CAD-01:** STEP import records translator name/version/options, source units, B-rep repair log, tessellation settings, and resulting topology/mesh hashes.
- **P-CAD-02:** Reimport produces a mapping report with preserved, changed, added, removed, and ambiguous regions. Ambiguous BC/selection mapping blocks rerun until resolved.
- **P-SWEEP-01:** Every sweep point is an immutable run; aggregate plots deep-link to it and never contain values without run evidence.
- **P-REF-01:** An external solver/reference import records source solver/version, case/config hash, mesh/discretization identity, units, coordinate transform, quantities imported, and any conversion loss.
- **P-VV-01:** A discretization study supports at least three levels, reports the monitored quantity and refinement relation, and never calls a result grid-independent solely because one image looks unchanged.
- **P-AI-01:** Training, validation/checkpoint selection, independent test, and production feedback are separate immutable sets.
- **P-AI-02:** A model release cannot be marked qualified without declared intended use, support envelope, independent metrics, baseline comparison, known failure modes, and dataset/model fingerprints.
- **P-REMOTE-01:** Local and remote backends consume the same run manifest; remote execution cannot mutate a locked local run and returns content-addressed artifacts plus logs.

---

## 12. Product measures

Avoid vanity metrics such as number of solver options or total runs. Measure:

- median time from source import to **first evidence-complete run**;
- percentage of runs with complete source/model/contract provenance;
- percentage of CAD imports with known units and accepted transform;
- percentage of unsupported/out-of-domain attempts caught before inference;
- deterministic reopen/rerun pass rate;
- rate of UNKNOWN versus CLEAN/FLAGGED provenance, with reasons;
- time to inspect and explain the worst benchmark cell;
- percentage of reports whose hash/signature verifies;
- crash-recovery success rate;
- percentage of comparisons whose points deep-link to immutable run evidence.

---

## 13. Dated bibliography

All sources below were accessed **2026-07-23**. A release/version or publication date is included where the source supplies one.

### Ansys

- Ansys, *Fluent in Ansys Workbench User’s Guide*, **2025 R2**: https://ansyshelp.ansys.com/public/Views/Secured/corp/v252/en/pdf/Ansys_Fluent_in_Ansys_Workbench_Users_Guide.pdf
- Ansys, *Using the Fluent Guided Meshing Workflows*, **2025 R1**: https://ansyshelp.ansys.com/public/Views/Secured/corp/v251/en/flu_wb/flu_tgrd_wb_start_fluent_workflow.html
- Ansys, *Monitoring and Reporting Solution Data*, **2025 R2**: https://ansyshelp.ansys.com/public/Views/Secured/corp/v252/en/flu_ug/flu_ug_reporting_sec_monitoring_solution.html
- Ansys, *Determining Mesh Statistics and Quality*, **2025 R2**: https://ansyshelp.ansys.com/public/Views/Secured/corp/v252/en/flu_ug/tgd_user_report.html
- Ansys, *Discovery product and geometry capabilities*, current page carrying **2026** product context: https://www.ansys.com/products/3d-design/ansys-discovery
- PyAnsys, *PyFluent meshing workflows*, stable documentation, accessed **2026-07-23**: https://fluent.docs.pyansys.com/version/stable/user_guide/meshing/new_meshing_workflows.html
- Ansys, *Fluent*, **2026 R1**: https://www.ansys.com/products/fluids/ansys-fluent
- Ansys, *Defining a ROM*, **2025 R2**: https://ansyshelp.ansys.com/public/Views/Secured/corp/v252/en/flu_ug/flu_ug_rom_setup.html
- Ansys, *SimAI*, **2026 R1** product structure: https://www.ansys.com/products/ai/simai
- Ansys, *SimAI Pro Data Import*, stable documentation, accessed **2026-07-23**: https://simai-pro.docs.ansys.com/version/stable/user_guide/data_preparation/step_data_import
- Ansys, *Minerva SPDM*: https://www.ansys.com/products/connect/ansys-minerva
- Ansys, *optiSLang Process Integration & Design Optimization*, **2026 R1**: https://www.ansys.com/products/connect/ansys-optislang

### Siemens

- Siemens, *Simcenter STAR-CCM+*, current product page referencing **2606 / 2026**: https://www.siemens.com/en-gb/products/simcenter/fluids-thermal-simulation/star-ccm/
- Siemens, *CAD preparation for CFD simulation—the even easier way*, **2021-06-16**: https://blogs.sw.siemens.com/simcenter/cad-preparation-for-cfd-simulation-the-even-easier-way/
- Siemens, *Replace Part—Out with the old, in with the new*, **2017** feature explanation: https://blogs.sw.siemens.com/simcenter/star-ccm-v12-04-preview-out-with-the-old-in-with-the-new/
- Siemens, *Simulation Templates for CFD Automation*, **2022**: https://blogs.sw.siemens.com/simcenter/simulation-templates-your-magic-typewriter-for-cfd-automation/
- Siemens, *Simcenter STAR-CCM+ 2602 release*, **2026**: https://blogs.sw.siemens.com/simcenter/simcenter-star-ccm-2602-released/
- Siemens, *Simcenter STAR-CCM+ 2606 release*, **2026**: https://blogs.sw.siemens.com/simcenter/simcenter-star-ccm-2606-released/
- Siemens, *Design Manager integration with Teamcenter Simulation*, **2024**: https://blogs.sw.siemens.com/simcenter/going-exploring-with-spdm-plm/
- Siemens, *Simcenter Reduced Order Modeling 2504*, **2025**: https://blogs.sw.siemens.com/simcenter/reduced-order-modeling-2504/

### SimScale

- SimScale, *Platform Introduction*, last updated **2025-07-22**: https://www.simscale.com/docs/platform/
- SimScale, *CAD Preparation & Upload*: https://www.simscale.com/docs/cad-preparation/
- SimScale, *CAD Edit*: https://www.simscale.com/docs/cad-preparation/cad-mode/
- SimScale, *Simulation Setup*: https://www.simscale.com/docs/simulation-setup/
- SimScale, *Result Control*: https://www.simscale.com/docs/simulation-setup/result-control/
- SimScale, *Collaboration*: https://www.simscale.com/docs/platform/collaboration/
- SimScale, *Multi-purpose Analysis and parametric runs*: https://www.simscale.com/docs/analysis-types/multi-purpose-analysis/
- SimScale, *AI Model Training*: https://www.simscale.com/docs/ai-model-training/
- SimScale, *Physics AI*: https://www.simscale.com/product/physics-ai/

### Autodesk

- Autodesk, *The CFD Process*, **Autodesk CFD 2026**: https://help.autodesk.com/cloudhelp/2026/ENU/SimCFD-UsersGuide/files/GUID-1DD9447C-E53A-4431-ADAF-BF70E95ED09E.htm
- Autodesk, *CAD Connection and Basic Model Interactions*, **Autodesk CFD 2026**: https://help.autodesk.com/cloudhelp/2026/CHT/SimCFD-Self-paced/files/GUID-80AA459D-61D5-4984-8DD9-C47A0092F271.htm
- Autodesk, *Design Study Automation*, **2024 documentation**: https://help.autodesk.com/cloudhelp/2024/ENU/SimCFD-Learning/files/GUID-A31B38D1-7C94-440F-8634-98C13CA8C540.htm
- Autodesk, *Autodesk CFD API*, **2024 documentation**: https://help.autodesk.com/cloudhelp/2024/ENU/SimCFD-Learning/files/GUID-B56DEB46-56B0-4AB6-9BA9-380E2A208065.htm
- Autodesk, *Running CFD Simulations in the Cloud*, **Autodesk CFD 2026**: https://help.autodesk.com/cloudhelp/2026/ENU/SimCFD-UsersGuide/files/GUID-21D9C1F2-04CC-460E-9915-B6D36D3C4BF1.htm
- Autodesk, *Fusion Electronics Cooling setup*, current help, accessed **2026-07-23**: https://help.autodesk.com/cloudhelp/ENU/Fusion-Simulate/files/SIM-ECOOLING-OVERVIEW-TASK.htm

### COMSOL

- COMSOL, *Model Builder*, **COMSOL 6.4 / 2026**: https://www.comsol.com/comsol-multiphysics/model-builder
- COMSOL, *LiveLink Interface Specification*, **COMSOL 6.4 / 2026**: https://www.comsol.com/products/specifications/cad/livelinkaa-interface/
- COMSOL, *LiveLink for SOLIDWORKS node*, **COMSOL 6.4**: https://doc.comsol.com/6.4/doc/com.comsol.help.llsw/llsw_ug_livelink_interface.5.07.html
- COMSOL, *Model Manager*, **COMSOL 6.4**: https://doc.comsol.com/6.4/doc/com.comsol.help.comsol/model_manager_ref_introduction.55.3.html
- COMSOL, *Cluster Sweep*, **COMSOL 6.4**: https://doc.comsol.com/6.4/doc/com.comsol.help.comsol/comsol_ref_solver.36.043.html
- COMSOL, *Surrogate Model Training*, **COMSOL 6.4**: https://doc.comsol.com/6.4/doc/com.comsol.help.comsol/comsol_ref_solver.36.009.html

### Altair

- Altair, *HyperMesh CFD UI Introduction*, **2025**: https://2025.help.altair.com/2025/hwcfdsolvers/acusolve/topics/tutorials/acu/acu_1000_intro_cfd_t.htm
- Altair, *DOE Studies*, current help, accessed **2026-07-23**: https://help.altair.com/hwdesktop/cfd/topics/pre_processing/morph/doe_studies_t.htm
- Altair, *AcuRun*, **2025**: https://2025.help.altair.com/2025/hwcfdsolvers/acusolve/topics/acusolve/solver_programs_acurun.htm
- Altair, *PhysicsAI in CFD*, **2026**: https://2026.help.altair.com/2026/hwdesktop/cfd/topics/chapter_heads/physicsAI_r.htm
- Altair, *SimSolid Product Introduction*, **2026**: https://2026.help.altair.com/2026/ss/en_us/topics/simsolid/get_started/product_intro_r.htm

### OpenFOAM, SimFlow, and SU2

- OpenCFD, *OpenFOAM Case File Structure*: https://www.openfoam.com/documentation/user-guide/2-openfoam-cases/2.1-file-structure-of-openfoam-cases
- OpenCFD, *`snappyHexMesh`*: https://www.openfoam.com/documentation/user-guide/4-mesh-generation-and-conversion/4.4-mesh-generation-with-the-snappyhexmesh-utility
- OpenCFD, *Monitoring and Managing Jobs*: https://www.openfoam.com/documentation/user-guide/6-solving/6.4-monitoring-and-managing-jobs
- OpenCFD, *New DMD ROM Field Reconstruction*, **OpenFOAM v2312 / 2023**: https://www.openfoam.com/news/main-news/openfoam-v2312/post-processing
- OpenCFD, *Python bindings (`pybFoam`)*, **OpenFOAM v2606 / 2026**: https://www.openfoam.com/news/main-news/openfoam-v2606/plugins
- SimFlow, *CFD workflow and OpenFOAM GUI*, **SimFlow 2026**: https://sim-flow.com/
- SU2, *Software Components*: https://su2code.github.io/docs/Software-Components/
- SU2, *Configuration File*: https://su2code.github.io/docs/Configuration-File/
- SU2, *Mesh File*: https://su2code.github.io/docs/Mesh-File/
- SU2GUI, *Introduction and Case Management*: https://su2code.github.io/su2gui/Introduction/

### CAD integration and V&V

- Onshape, *REST API Architecture—Workspaces, Versions, and Microversions*: https://onshape-public.github.io/docs/api-intro/architecture/
- Onshape, *Import and Export API*: https://onshape-public.github.io/docs/api-adv/translation/
- Open CASCADE, *STEP Translator*, **OCCT 7.9.0**: https://dev.opencascade.org/doc/occt-7.9.0/overview/html/occt_user_guides__step.html
- Open CASCADE, *Shape Healing*, **OCCT 7.9.0**: https://dev.opencascade.org/doc/occt-7.9.0/overview/html/occt_user_guides__shape_healing.html
- CAD Exchanger, *SDK Supported Formats*, accessed **2026-07-23**: https://docs324x.cadexchanger.com/sdk/sdk_supported_formats.html
- NASA Glenn/NPARC, *Overview of CFD Verification & Validation*, last updated **2021-02-10**: https://www.grc.nasa.gov/www/wind/valid/tutorial/overview.html
- NASA Glenn/NPARC, *Validation Assessment*: https://www.grc.nasa.gov/www/wind/valid/tutorial/valassess
- NASA Glenn, *CFPOST Equations—Pressure Coefficient*: https://www.grc.nasa.gov/WWW/winddocs/cfpost/appc.html
- ASME, *V&V 20-2009 (R2021), Standard for Verification and Validation in CFD and Heat Transfer*: https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-fluid-dynamics-and-heat-transfer
