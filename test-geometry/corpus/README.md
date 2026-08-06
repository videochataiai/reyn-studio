# STEP qualification corpus

**Purpose:** Expand beyond the two shipped Truck fixtures so translator ceilings
are evidence-backed before any OCCT bridge is wired into Studio import.

**Policy:** Truck remains the default in-process translator. Real SolidWorks /
NX / CATIA / Creo / Inventor / Fusion / Onshape exports should be added here as
single-part files with the metadata table below. Do not claim “full STEP” until
every **Supported** row has deterministic triangle identity on macOS arm64 and
Windows x64.

## Inventory

| Fixture | Source / exporter | Schema | Units | Expected Truck outcome | Status |
|---|---|---|---|---|---|
| `../cuboid_ap214.step` | Formlabs / foxtrot (AP214) | AP214 | m | Closed manifold; voxelizes | **In suite** |
| `../part_ap242.step` | Onshape AP242 Ed. 2 curved | AP242 | m | Imports; open-boundary defects stay visible | **In suite** |
| `assembly_occurrence.step` | Synthetic minimal | — | — | Hard reject (`assemblies`) | **In suite** |
| `conflicting_units.step` | Synthetic minimal | — | m + mm | Hard reject (multiple length units) | **In suite** |
| `malformed_truncated.step` | Synthetic truncated | — | — | Hard reject (malformed) | **In suite** |
| `vendor/solidworks_*.step` | *pending real export* | AP203/214 | TBD | Record identity + topology | **Slot** |
| `vendor/nx_*.step` | *pending real export* | AP214/242 | TBD | Record identity + topology | **Slot** |
| `vendor/fusion_*.step` | *pending real export* | AP242 | TBD | Record identity + topology | **Slot** |
| `vendor/onshape_extra_*.step` | *pending real export* | AP242 | TBD | Broader than `part_ap242` | **Slot** |

## Adding a vendor fixture

1. Export a **single-part** solid (no assembly occurrence graph).
2. Record exporter product + version, schema, declared units, and expected
   bounding-box extents (metres) in this table.
3. Drop the file under `test-geometry/corpus/vendor/` (git-lfs if large).
4. Add a Rust test in `src/cad_step.rs` locking units, triangle identity across
   two parses, and topology/voxel gates (or the honest failure mode).
5. If Truck leaves open seams or fails tessellation on a valid closed solid,
   keep the failure visible — that evidence gates OCCT slice 3.

## Non-goals

- Silent healing.
- Treating assemblies as one body.
- Shipping OCCT because a slot is empty — empty slots are not a ceiling.
