# Reyn Studio 0.3.0 release notes

**Release:** 0.3.0  
**Channel:** invitation-only research preview  
**Platforms:** Apple-silicon macOS and Windows x64

## Verified in-app update downloads

Reyn Studio now checks a signed, versioned release feed at startup and on
demand. The same updater is available before preview login and inside Studio.
It selects only the exact supported platform, rejects expired or rolled-back
metadata, downloads to a temporary file, and verifies the signed byte count and
SHA-256 before publishing the archive. Settings shows progress, release notes,
integrity status, and a Finder/Explorer handoff.

Installation remains explicit and manual. The pinned Ed25519 release key
authenticates Reyn's metadata and package hash; it does not replace Apple
Developer ID/notarization or Windows Authenticode. The 0.3.0 preview archives
are described as unsigned unless the corresponding release build records real
platform signing.

Existing 0.2.0 installations do not contain the updater and therefore require
one final download from reynflow.com. Direct in-app delivery starts with 0.3.0.

## Packaging and runtime hardening

- Release manifests and runtime contracts are written before the final
  sign/notarize seal, so packaging never mutates a sealed bundle.
- Runtime-contract validation accepts truthful platform-signing flags when the
  package was actually signed.
- CAD prediction requests use CAD-aware timeout budgeting and intermediate
  progress so valid even-horizon runs do not fail against the old fixed
  120-second ceiling.

## Engineering results

- Customer Results and reports no longer present semigroup or divergence-RMS
  model-lab instrumentation as engineering quality or accuracy.
- Immutable evidence retains the developer diagnostics needed for forensic
  review without placing them on the customer decision path.

## STEP and CAD provenance

- STEP import now carries the source B-rep identity separately from a canonical
  analyzed-triangle-mesh SHA-256.
- Ordered translate, tessellate, weld, diagnose, orient, and voxelize derivation
  steps travel with the exact case contract and immutable HTML/PDF/PNG evidence.
- The mesh digest has a fixed cross-platform byte encoding and a known-answer
  parity test.
- The fail-closed STEP corpus covers additional bounded topology and malformed
  input cases. Vendor-export qualification for SolidWorks, NX, CATIA, and Creo
  remains open and is not claimed.

## Internal CAD bridge boundary

The crash-isolated, length-prefixed CAD bridge contract and stub process are
included for engineering qualification. They are not wired into public STEP
import, link no Open CASCADE code, and do not imply assembly, healing, IGES, or
vendor-native CAD support.
