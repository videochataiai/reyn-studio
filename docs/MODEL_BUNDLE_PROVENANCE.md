# Vendored model-bundle loader provenance

`engine/model_bundle.py` is app-owned release code. It is packaged beside
`reyn_engine.py` so official builds never read an unpinned ambient research
checkout to verify a model.

- Upstream source: `reyn-research/model_bundle.py`
- Upstream source SHA-256:
  `e2719405fde3d82cb5df3084d229196a9e1ce466c40895057543ffb97ddd2dfc`
- Vendored: 2026-07-30
- Reason: pinned research revision
  `0333b13bd117e6129d989aa41dd7e3057c11d116` does not contain the loader.

The vendored source preserves the upstream fail-closed bundle, Safetensors,
detached Ed25519, and TUF verification contract. The only portability change is
the trusted-state lock: Unix uses `fcntl.flock`; Windows uses a bounded
`msvcrt.locking` loop over the same lock file.

`engine/pinned_model_trust.py` embeds the public threshold-signed TUF root for
the YC 0.1.1 preview model. The root expires on 2027-01-31 and is bound to the
release artifact by SHA-256. Private model and TUF role keys are not present in
the source repository or package. A bundle outside this root and its delegated
model target fails before tensor loading.

This establishes publisher authenticity for the exact preview bundle; it does
not turn the model's replicated research result into production CFD
qualification.

Any future loader update must record the new upstream digest, review the local
Windows-lock delta, run the upstream model-bundle tests, and regenerate the
hashed Windows Python runtime lock.
