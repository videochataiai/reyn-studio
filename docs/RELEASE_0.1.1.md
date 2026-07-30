# Reyn Studio 0.1.1 corrective release

0.1.1 corrects lifecycle, evidence, and packaging contracts; it does not
qualify Reyn Studio as production CFD software.

- Official YC-gated builds create no studio, project/model state, or Python
  sidecar before authentication. Session expiry drops that state and terminates
  compute before login returns. Public source builds remain ungated.
- External-flow attempts are persisted at start and retain exact input lineage,
  timestamps, and succeeded/failed/cancelled outcomes. Cancellation and timeout
  terminate the blocking sidecar and restart a clean engine for retry.
- FEA CSV is **source-frame surface traction/load data**, not conservative
  target-mesh mapping. It includes the full transform, frame semantics, SI
  units, operating references, integration-area weights, immutable lineage,
  reported resultants, exported-sample resultants, and reconciliation residuals.
- Calculation CSV write failures are surfaced. Malformed settings are moved to
  a recovery file before defaults can be saved.
- PowerShell CI commands have immediate `$LASTEXITCODE` guards, enforced by a
  regression test.
- macOS compute is arm64/macOS 14+ only. A universal2 shell may support Intel
  review, but Intel compute is unsupported. Package assembly requires an exact
  arm64 factory runtime, runtime lock, SBOM/notices, and pinned private research
  revision.

External release gates remain: the private research repository must publish the
module/lock closure in `packaging/macos/RESEARCH_SOURCE_REQUEST.json`; Developer
ID signing, notarization/stapling, clean-machine qualification, a production TUF
root ceremony, and a separately qualified signed model are not bypassed.
