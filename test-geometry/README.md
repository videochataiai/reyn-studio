# Geometry test fixtures

The generated STL fixtures are produced by the scripts in this directory.

The STEP fixtures are retained as real exporter outputs because STEP translator
coverage depends on the application protocol and authoring system:

- `cuboid_ap214.step` — Formlabs `foxtrot` AP214 cuboid fixture, Apache-2.0,
  source: <https://github.com/Formlabs/foxtrot/blob/master/examples/cuboid.step>,
  SHA-256 `e387e33aae1808f681f5ed306834cb6f426522ac0099f3540531e33d8827371f`.
- `part_ap242.step` — Onshape AP242 Edition 2 part fixture from
  `jorgensd/ida-presentation`, MIT, source:
  <https://github.com/jorgensd/ida-presentation/blob/main/part.step>,
  SHA-256 `a0ba5c75e56b608b83fab9d55a21e6f2447c3f2e43fad78a57d373ce4f58faa5`.

Do not replace either fixture with a hand-edited schema label. Tests need real
entity output from the stated protocol and exporter.
