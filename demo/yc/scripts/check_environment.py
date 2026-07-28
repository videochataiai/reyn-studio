#!/usr/bin/env python3
"""Report demo runtime and model prerequisites without claiming qualification."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
RESEARCH = ROOT.parent / "reyn-research"


def python_candidate() -> Path | None:
    if os.environ.get("REYN_PYTHON"):
        return Path(os.environ["REYN_PYTHON"]).expanduser()
    local = RESEARCH / ".venv" / "bin" / "python"
    if local.is_file():
        return local
    resolved = shutil.which("python3")
    return Path(resolved) if resolved else None


def dependency_probe(python: Path | None) -> dict[str, object]:
    if python is None or not python.is_file():
        return {"ok": False, "detail": "no Python interpreter found"}
    probe = (
        "import json,sys\n"
        "mods={}\n"
        "for name in ('numpy','torch','safetensors'):\n"
        "  try:\n"
        "    module=__import__(name); mods[name]=getattr(module,'__version__','unknown')\n"
        "  except Exception as exc:\n"
        "    mods[name]=f'MISSING: {exc}'\n"
        "print(json.dumps({'python':sys.version.split()[0],'modules':mods}))\n"
    )
    result = subprocess.run(
        [str(python), "-c", probe], capture_output=True, text=True, check=False
    )
    if result.returncode:
        return {"ok": False, "detail": result.stderr.strip() or "probe failed"}
    detail = json.loads(result.stdout)
    detail["ok"] = all(
        not str(version).startswith("MISSING:")
        for version in detail["modules"].values()
    )
    return detail


def model_candidates() -> list[dict[str, object]]:
    candidates = []
    for directory in (RESEARCH, RESEARCH / "reyn_models"):
        if not directory.is_dir():
            continue
        for bundle in sorted(directory.glob("*.reynmodel")):
            signature = Path(f"{bundle}.sig")
            candidates.append(
                {
                    "path": str(bundle),
                    "adjacent_signature": signature.is_file(),
                    "qualification": "UNKNOWN — candidate presence is not release qualification",
                }
            )
    return candidates


def report() -> dict[str, object]:
    python = python_candidate()
    candidates = model_candidates()
    return {
        "platform": sys.platform,
        "cargo": shutil.which("cargo"),
        "ffmpeg": shutil.which("ffmpeg"),
        "ffprobe": shutil.which("ffprobe"),
        "research_root": str(RESEARCH),
        "research_root_exists": RESEARCH.is_dir(),
        "python": str(python) if python else None,
        "runtime": dependency_probe(python),
        "model_candidates": candidates,
        "completed_inference_available": False,
        "model_gate": (
            "BLOCKED — no qualified signed production .reynmodel is supplied by this demo. "
            "Any discovered file remains an unqualified candidate until artifact-bound "
            "scientific, runtime, authenticity, and distribution evidence passes."
        ),
    }


if __name__ == "__main__":
    output = report()
    print(json.dumps(output, indent=2))
    raise SystemExit(
        0
        if output["cargo"] and output["research_root_exists"]
        else 1
    )
