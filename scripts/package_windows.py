#!/usr/bin/env python3
"""Build and stage a Windows 11 x64 portable Reyn Studio package."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

from windows_packaging import (
    DEFAULT_SOURCE_DATE_EPOCH,
    authenticode_sign,
    copy_resources,
    deterministic_zip,
    generate_supply_chain_artifacts,
    inventory,
    loader_probe,
    prepare_runtime_manifest,
    runtime_probe,
    safe_copy_file,
    safe_copy_tree,
    safe_files,
    sha256_file,
    validate_stage,
    write_json,
    write_sha256sums,
)

EXPECTED_ACCESS_ENDPOINT = "https://reynflow.com/api/yc-access/v1/session"
EXPECTED_TERMS_VERSION = "1.0"
EXPECTED_PRIVACY_VERSION = "1.0"


def cargo_version(root: Path) -> str:
    for line in (root / "Cargo.toml").read_text(encoding="utf-8").splitlines():
        if line.startswith("version = "):
            return line.split('"', 2)[1]
    raise ValueError("Cargo.toml has no package version")


def resolve_research_source(root: Path, configured: Path | None) -> Path:
    candidate = configured or (
        Path(os.environ["REYN_RESEARCH_SOURCE_DIR"])
        if os.environ.get("REYN_RESEARCH_SOURCE_DIR")
        else root.parent / "reyn-research"
    )
    candidate = candidate.absolute()
    if not candidate.is_dir():
        raise ValueError(
            f"research source is missing: {candidate}; pass --research-source-dir"
        )
    safe_files(candidate)
    return candidate


def git_revision(path: Path) -> str:
    return subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=path,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def release_pins(root: Path) -> dict[str, object]:
    return json.loads(
        (root / "packaging/windows/release-pins.json").read_text(encoding="utf-8")
    )


def require_pinned_toolchain(root: Path, expected: str) -> None:
    version = subprocess.run(
        ["rustc", "--version"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.split()
    found = version[1] if len(version) > 1 else ""
    if found != expected:
        raise ValueError(f"rustc {found or 'unknown'} does not match release pin {expected}")


def build_binary(root: Path, target_dir: Path) -> Path:
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(target_dir)
    env["CARGO_INCREMENTAL"] = "0"
    subprocess.run(
        [
            "cargo",
            "build",
            "--locked",
            "--release",
            "--target",
            "x86_64-pc-windows-msvc",
            "--bin",
            "reyn-studio",
        ],
        cwd=root,
        env=env,
        check=True,
    )
    binary = target_dir / "x86_64-pc-windows-msvc/release/reyn-studio.exe"
    if not binary.is_file():
        raise ValueError(f"Cargo did not create {binary}")
    return binary


def preview_access_contract(binary: Path) -> dict[str, object]:
    completed = subprocess.run(
        [str(binary), "--print-access-contract"],
        check=True,
        capture_output=True,
        text=True,
        timeout=15,
    )
    try:
        contract = json.loads(completed.stdout.strip())
    except (ValueError, TypeError) as error:
        raise ValueError("Windows binary returned a malformed access contract") from error
    expected = {
        "schema": "com.reyn.studio.preview-access/1",
        "required": True,
        "endpoint": EXPECTED_ACCESS_ENDPOINT,
        "terms_version": EXPECTED_TERMS_VERSION,
        "privacy_version": EXPECTED_PRIVACY_VERSION,
    }
    if contract != expected:
        raise ValueError(
            "Windows binary does not contain the required YC preview access contract: "
            f"expected {expected}, found {contract}"
        )
    return contract


def package(args: argparse.Namespace) -> int:
    root = Path(__file__).resolve().parents[1]
    pins = release_pins(root)
    require_pinned_toolchain(root, str(pins["rust_toolchain"]))
    if args.source_date_epoch != pins["source_date_epoch"]:
        raise ValueError(
            f"SOURCE_DATE_EPOCH {args.source_date_epoch} does not match release pin "
            f"{pins['source_date_epoch']}"
        )
    version = cargo_version(root)
    runtime = args.runtime_dir.absolute()
    research = resolve_research_source(root, args.research_source_dir)
    research_revision = git_revision(research)
    if research_revision != pins["research_revision"]:
        raise ValueError(
            f"research revision {research_revision} does not match release pin "
            f"{pins['research_revision']}"
        )
    output = (root / args.output_dir).absolute()
    target = (root / args.target_dir).absolute()
    output.mkdir(parents=True, exist_ok=True)
    target.mkdir(parents=True, exist_ok=True)
    safe_files(output)
    binary = args.binary.absolute() if args.binary else build_binary(root, target)
    if not binary.is_file():
        raise ValueError(f"Windows executable is missing: {binary}")
    access_contract = preview_access_contract(binary)

    temporary = Path(tempfile.mkdtemp(prefix=".reyn-windows-", dir=output))
    stage = temporary / f"Reyn-Studio-{version}-windows-x64"
    try:
        stage.mkdir()
        safe_copy_file(binary, root, stage / "Reyn Studio.exe")
        safe_copy_file(
            root / "packaging/windows/ReynStudio.ico",
            root,
            stage / "ReynStudio.ico",
        )
        safe_copy_file(root / "LICENSE", root, stage / "LICENSE")
        safe_copy_file(root / "NOTICE", root, stage / "NOTICE")
        safe_copy_tree(runtime, stage / "ReynPython")
        copy_resources(root, research, stage)
        probe = runtime_probe(stage / "ReynPython")
        model_loader_probe = loader_probe(stage)
        dependency_closure = generate_supply_chain_artifacts(
            root,
            stage / "ReynPython",
            stage,
            root / "packaging/windows/python-runtime.lock",
        )
        source_revision = git_revision(root)
        prepare_runtime_manifest(stage, source_revision, probe, dependency_closure)
        authenticode_sign(
            stage / "Reyn Studio.exe",
            args.sign_pfx,
            args.sign_password_env,
            args.timestamp_url,
        )
        release_manifest = {
            "schema": "com.reyn.studio.windows-portable/1",
            "app_version": version,
            "platform": "windows",
            "architecture": "x86_64",
            "minimum_windows": "11",
            "package_format": "portable-zip",
            "runtime_bundled": True,
            "runtime_layout": "ReynPython/python.exe",
            "compute_devices": ["automatic", "cpu"],
            "cuda_supported": False,
            "preview_access": access_contract,
            "authenticode_requested": args.sign_pfx is not None,
            "authenticode_signed": args.sign_pfx is not None,
            "windows_verified": False,
            "verification_boundary": (
                "Package structure validated during build. DX12, dialogs, DPI, "
                "process cleanup, SmartScreen, Authenticode trust, and clean-machine "
                "launch require the Windows acceptance matrix."
            ),
            "executable_sha256": sha256_file(stage / "Reyn Studio.exe"),
            "cargo_lock_sha256": dependency_closure["cargo_lock_sha256"],
            "python_lock_sha256": dependency_closure["python_lock_sha256"],
            "research_revision": research_revision,
            "rust_toolchain": pins["rust_toolchain"],
            "model_loader_probe": model_loader_probe,
        }
        write_json(stage / "release-manifest.json", release_manifest)
        write_json(
            stage / "resource-inventory.json",
            inventory(
                stage,
                excluded={"release-manifest.json", "resource-inventory.json"},
            ),
        )
        errors = validate_stage(stage, run_runtime_probe=args.runtime_smoke)
        if errors:
            raise ValueError("Windows package validation failed: " + "; ".join(errors))

        destination = output / stage.name
        if destination.exists():
            safe_files(destination)
            shutil.rmtree(destination)
        stage.replace(destination)
        archive = output / f"{destination.name}.zip"
        deterministic_zip(destination, archive, args.source_date_epoch)
        write_sha256sums([archive], output / "SHA256SUMS")
        print(f"Portable directory: {destination}")
        print(f"Portable ZIP: {archive}")
        print(f"SHA-256: {sha256_file(archive)}")
        print(
            "Boundary: Windows package created and structurally validated; "
            "support remains pending the Windows acceptance matrix."
        )
        return 0
    finally:
        shutil.rmtree(temporary, ignore_errors=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime-dir", required=True, type=Path)
    parser.add_argument("--research-source-dir", type=Path)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output-dir", default="dist/windows")
    parser.add_argument("--target-dir", default="target/package-windows")
    parser.add_argument(
        "--source-date-epoch",
        type=int,
        default=int(os.environ.get("SOURCE_DATE_EPOCH", DEFAULT_SOURCE_DATE_EPOCH)),
    )
    parser.add_argument("--runtime-smoke", action="store_true")
    parser.add_argument("--sign-pfx", type=Path)
    parser.add_argument(
        "--sign-password-env",
        default="REYN_AUTHENTICODE_PFX_PASSWORD",
    )
    parser.add_argument(
        "--timestamp-url",
        default="http://timestamp.digicert.com",
    )
    return parser.parse_args()


if __name__ == "__main__":
    try:
        raise SystemExit(package(parse_args()))
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"Windows packaging failed: {error}", file=os.sys.stderr)
        raise SystemExit(1)
