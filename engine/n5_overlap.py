"""Bounded field-space and trajectory-overlap evidence for N5 benchmarks.

REQ-N5-VV-01 / N5X-VV-01..02 require archived training candidates.  This
module never regenerates a historical training set from current source code:
missing, malformed, or incomplete artifacts remain UNKNOWN.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import numpy as np


OVERLAP_SCHEMA = "reyn.benchmark-overlap-analysis.v1"
CANDIDATE_SCHEMA = "reyn.training-overlap-candidates.v1"
ALGORITHM = "exact_chunked_symmetric_relative_l2_v1"
DEFAULT_THRESHOLD = 1e-6


def _sha256_file(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _tensor_record(value):
    value = np.asarray(value)
    encoded = np.ascontiguousarray(value, dtype="<f4")
    header = json.dumps(
        {"dtype": "float32_le", "shape": list(encoded.shape), "order": "C"},
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    digest = hashlib.sha256()
    digest.update(header)
    digest.update(b"\n")
    digest.update(encoded.tobytes(order="C"))
    return {
        "sha256": digest.hexdigest(),
        "shape": list(encoded.shape),
        "encoding": "float32 little-endian C-order",
    }


def _queries(value, name):
    result = np.asarray(value, dtype=np.float32)
    if result.ndim < 2 or result.shape[0] < 1:
        raise ValueError(f"{name} must contain at least one query tensor")
    if not np.all(np.isfinite(result)):
        raise ValueError(f"{name} contains non-finite values")
    return np.ascontiguousarray(result)


def _query_ids(query_ids, count):
    if query_ids is None:
        return [f"benchmark-{index:04d}" for index in range(count)]
    result = [str(value) for value in query_ids]
    if len(result) != count:
        raise ValueError("query_ids must match the number of initial conditions")
    if len(set(result)) != len(result) or any(not value for value in result):
        raise ValueError("query_ids must be non-empty and unique")
    return result


def _scalar(archive, key):
    if key not in archive:
        raise ValueError(f"candidate artifact is missing {key}")
    values = np.asarray(archive[key])
    if values.size != 1:
        raise ValueError(f"candidate artifact {key} must be scalar")
    value = values.reshape(-1)[0]
    if isinstance(value, bytes):
        value = value.decode("utf-8")
    return value.item() if isinstance(value, np.generic) else value


def _ids(archive, key, count):
    if key not in archive:
        raise ValueError(f"candidate artifact is missing {key}")
    values = np.asarray(archive[key])
    if values.ndim != 1 or values.size != count:
        raise ValueError(f"candidate artifact {key} does not match candidate count")
    result = [str(value) for value in values.tolist()]
    if len(set(result)) != len(result) or any(not value for value in result):
        raise ValueError(f"candidate artifact {key} must be non-empty and unique")
    return result


def _load_candidates(path):
    path = Path(path)
    artifact_sha256 = _sha256_file(path)
    with np.load(path, allow_pickle=False) as archive:
        if str(_scalar(archive, "schema")) != CANDIDATE_SCHEMA:
            raise ValueError("unsupported training-overlap candidate schema")
        name = str(_scalar(archive, "candidate_set_name"))
        representation = str(_scalar(archive, "representation"))
        complete_value = _scalar(archive, "candidate_set_complete")
        if not isinstance(complete_value, (bool, np.bool_)):
            raise ValueError("candidate_set_complete must be a boolean")
        if not name or not representation:
            raise ValueError("candidate set name and representation are required")

        initial = _queries(archive["initial_conditions"], "initial_conditions")
        candidate_ids = _ids(archive, "candidate_ids", initial.shape[0])
        trajectories = None
        trajectory_ids = []
        if "trajectories" in archive:
            trajectories = _queries(archive["trajectories"], "trajectories")
            trajectory_ids = _ids(
                archive,
                "trajectory_candidate_ids",
                trajectories.shape[0],
            )
        manifest = {}
        if "generation_manifest_json" in archive:
            manifest = json.loads(str(_scalar(archive, "generation_manifest_json")))
            if not isinstance(manifest, dict):
                raise ValueError("generation_manifest_json must encode an object")

    return {
        "path": path,
        "sha256": artifact_sha256,
        "name": name,
        "representation": representation,
        "complete": bool(complete_value),
        "initial": initial,
        "candidate_ids": candidate_ids,
        "trajectories": trajectories,
        "trajectory_ids": trajectory_ids,
        "generation_manifest": manifest,
    }


def _distances(query, candidates, chunk_size=128):
    query = np.asarray(query, dtype=np.float64).reshape(-1)
    query_norm = np.linalg.norm(query)
    result = np.empty(candidates.shape[0], dtype=np.float64)
    for start in range(0, candidates.shape[0], chunk_size):
        stop = min(start + chunk_size, candidates.shape[0])
        chunk = np.asarray(candidates[start:stop], dtype=np.float64).reshape(
            stop - start, -1
        )
        denominator = np.maximum(
            np.maximum(np.linalg.norm(chunk, axis=1), query_norm),
            1e-12,
        )
        result[start:stop] = np.linalg.norm(chunk - query, axis=1) / denominator
    return result


def _check(kind, queries, candidates, query_ids, candidate_ids, threshold, complete, top_k):
    if queries is None:
        return {
            "status": "UNKNOWN",
            "proposition": f"{kind} overlap was not checked because query data is unavailable",
            "threshold": threshold,
            "nearest_matches": [],
        }
    if candidates is None:
        return {
            "status": "UNKNOWN",
            "proposition": f"{kind} overlap was not checked because training candidates are unavailable",
            "threshold": threshold,
            "nearest_matches": [],
        }
    if queries.shape[1:] != candidates.shape[1:]:
        return {
            "status": "UNKNOWN",
            "proposition": (
                f"{kind} overlap was not checked because query and candidate "
                "representations have different shapes"
            ),
            "threshold": threshold,
            "nearest_matches": [],
        }

    nearest = []
    collision = False
    for query_id, query in zip(query_ids, queries):
        distances = _distances(query, candidates)
        order = sorted(
            range(len(candidate_ids)),
            key=lambda index: (float(distances[index]), candidate_ids[index]),
        )[:top_k]
        matches = [
            {
                "candidate_id": candidate_ids[index],
                "distance": float(distances[index]),
                "at_or_below_threshold": bool(distances[index] <= threshold),
            }
            for index in order
        ]
        collision = collision or any(
            match["at_or_below_threshold"] for match in matches
        )
        nearest.append({"query_id": query_id, "matches": matches})

    if collision:
        status = "FLAGGED"
        proposition = f"{kind} match found at or below the declared threshold"
    elif complete:
        status = "CLEAN"
        proposition = (
            f"no {kind} match at or below the declared threshold in the "
            "complete checked training candidate set"
        )
    else:
        status = "UNKNOWN"
        proposition = (
            f"no {kind} match found in the checked candidates, but candidate-set "
            "completeness is not established"
        )
    return {
        "status": status,
        "proposition": proposition,
        "threshold": threshold,
        "nearest_matches": nearest,
    }


def _unknown_result(query_ids, initial, trajectories, path, thresholds, reason):
    requested = Path(path).name if path is not None else None
    return {
        "schema": OVERLAP_SCHEMA,
        "status": "UNKNOWN",
        "proposition": reason,
        "algorithm": {
            "id": ALGORITHM,
            "distance": "L2(a-b) / max(L2(a), L2(b), 1e-12)",
            "search": "exact chunked exhaustive nearest neighbour",
        },
        "representation": {
            "name": "UNKNOWN",
            "tensor_encoding": "float32 little-endian C-order",
            "reduction": "none",
        },
        "thresholds": thresholds,
        "candidate_set": {
            "status": "UNAVAILABLE",
            "requested_artifact": requested,
            "declared_complete": None,
            "initial_condition_candidates": None,
            "trajectory_candidates": None,
            "artifact_sha256": None,
        },
        "checks": {
            "initial_condition": {
                "status": "UNKNOWN",
                "proposition": reason,
                "threshold": thresholds["initial_condition"],
                "nearest_matches": [],
            },
            "trajectory": {
                "status": "UNKNOWN",
                "proposition": reason,
                "threshold": thresholds["trajectory"],
                "nearest_matches": [],
            },
        },
        "reproducible_inputs": {
            "query_ids": query_ids,
            "initial_conditions": _tensor_record(initial),
            "trajectories": (
                _tensor_record(trajectories) if trajectories is not None else None
            ),
        },
        "warnings": [reason],
    }


def analyze_field_trajectory_overlap(
    initial_conditions,
    trajectories=None,
    *,
    candidate_artifact=None,
    query_ids=None,
    initial_threshold=DEFAULT_THRESHOLD,
    trajectory_threshold=DEFAULT_THRESHOLD,
    nearest_count=3,
):
    """Compare benchmark inputs with an archived training candidate artifact."""
    initial = _queries(initial_conditions, "initial_conditions")
    trajectory_queries = (
        _queries(trajectories, "trajectories") if trajectories is not None else None
    )
    ids = _query_ids(query_ids, initial.shape[0])
    if trajectory_queries is not None and trajectory_queries.shape[0] != len(ids):
        raise ValueError("trajectories must match the number of initial conditions")
    thresholds = {
        "initial_condition": float(initial_threshold),
        "trajectory": float(trajectory_threshold),
    }
    if any(not np.isfinite(value) or value < 0.0 for value in thresholds.values()):
        raise ValueError("overlap thresholds must be finite and non-negative")
    if nearest_count < 1:
        raise ValueError("nearest_count must be positive")

    missing_reason = "training candidate artifact is unavailable; field-space and trajectory overlap are UNKNOWN"
    if candidate_artifact is None or not Path(candidate_artifact).is_file():
        return _unknown_result(
            ids,
            initial,
            trajectory_queries,
            candidate_artifact,
            thresholds,
            missing_reason,
        )
    try:
        candidates = _load_candidates(candidate_artifact)
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        return _unknown_result(
            ids,
            initial,
            trajectory_queries,
            candidate_artifact,
            thresholds,
            f"training candidate artifact is unusable ({error}); overlap is UNKNOWN",
        )

    initial_check = _check(
        "initial-condition field-space",
        initial,
        candidates["initial"],
        ids,
        candidates["candidate_ids"],
        thresholds["initial_condition"],
        candidates["complete"],
        nearest_count,
    )
    trajectory_check = _check(
        "aligned trajectory",
        trajectory_queries,
        candidates["trajectories"],
        ids,
        candidates["trajectory_ids"],
        thresholds["trajectory"],
        candidates["complete"],
        nearest_count,
    )
    statuses = {initial_check["status"], trajectory_check["status"]}
    status = "FLAGGED" if "FLAGGED" in statuses else (
        "UNKNOWN" if "UNKNOWN" in statuses else "CLEAN"
    )
    proposition = {
        "FLAGGED": "a field-space initial-condition or aligned-trajectory match was found",
        "UNKNOWN": "field-space and trajectory non-overlap is not established",
        "CLEAN": (
            "no field-space initial-condition or aligned-trajectory match at or "
            "below the declared thresholds in the complete checked training candidate set"
        ),
    }[status]
    return {
        "schema": OVERLAP_SCHEMA,
        "status": status,
        "proposition": proposition,
        "algorithm": {
            "id": ALGORITHM,
            "distance": "L2(a-b) / max(L2(a), L2(b), 1e-12)",
            "search": "exact chunked exhaustive nearest neighbour",
        },
        "representation": {
            "name": candidates["representation"],
            "tensor_encoding": "float32 little-endian C-order",
            "reduction": "none",
            "trajectory_alignment": "full supplied sequences at matching frame indices",
        },
        "thresholds": thresholds,
        "candidate_set": {
            "status": "AVAILABLE",
            "name": candidates["name"],
            "declared_complete": candidates["complete"],
            "initial_condition_candidates": int(candidates["initial"].shape[0]),
            "trajectory_candidates": (
                int(candidates["trajectories"].shape[0])
                if candidates["trajectories"] is not None
                else 0
            ),
            "artifact_filename": candidates["path"].name,
            "artifact_schema": CANDIDATE_SCHEMA,
            "artifact_sha256": candidates["sha256"],
            "generation_manifest": candidates["generation_manifest"],
        },
        "checks": {
            "initial_condition": initial_check,
            "trajectory": trajectory_check,
        },
        "reproducible_inputs": {
            "query_ids": ids,
            "initial_conditions": _tensor_record(initial),
            "trajectories": (
                _tensor_record(trajectory_queries)
                if trajectory_queries is not None
                else None
            ),
            "candidate_artifact_sha256": candidates["sha256"],
        },
        "warnings": (
            []
            if candidates["complete"]
            else ["candidate-set completeness is not established; absence cannot be CLEAN"]
        ),
    }
