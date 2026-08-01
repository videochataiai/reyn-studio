# OCCT out-of-process bridge — design spike

**Status:** design only (Phase 0.3). Not implemented in 0.1.2.  
**Constraint:** never statically link OpenCASCADE into the Reyn Studio desktop binary (LGPL + packaging / notarization cost).

## When to build

Only after the in-process Truck STEP path hits a hard ceiling on a representative customer corpus (curved AP242 seams that remain open after honest weld limits, unsupported surfaces, or assemblies that must be analyzed as multi-body with transforms).

## Shape

```
Reyn Studio (Rust, egui)
    │  length-prefixed JSON request over stdin/stdout or local socket
    ▼
reyn-cad-bridge (separate process)
    │  dynamically linked OCCT (LGPL-safe distribution)
    ▼
Tessellated mesh + units + shell inventory + warnings
```

## Contract

- Input: absolute path or bytes handle, requested chord tolerance, max triangles/shells.
- Output: triangle soup in source units, declared length unit, shell count, translator/version, deterministic hash of tessellation parameters.
- Fail closed on assemblies unless the request explicitly selects one occurrence path.
- Crash isolation: bridge panics must not take down the Studio UI; Studio records a structured import error.
- Licensing: ship OCCT as a separate dylib/so/dll with LGPL notices; Studio remains Apache/MIT Rust.

## Non-goals

- Silent healing, PMI, native CAD SDKs, Parasolid/JT kernels inside Studio.
