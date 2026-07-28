#!/usr/bin/env python3
"""Build and stage an unsigned Reyn Studio macOS application bundle."""

from __future__ import annotations

import argparse
import hashlib
import os
import plistlib
import shlex
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from macos_packaging import (
    DEFAULT_SOURCE_DATE_EPOCH,
    ENGINE_RESOURCES,
    PACKAGE_RUST_TARGETS,
    RESEARCH_RESOURCES,
    RUST_TARGET_ARCHITECTURES,
    TARGET_ARCHITECTURES,
    copy_documentation_resources,
    copy_engine_resources,
    copy_research_resources,
    copy_security_resources,
    deterministic_zip,
    file_manifest,
    has_failures,
    info_plist,
    load_config,
    print_checks,
    require_executable_architectures,
    require_local_architecture_runtime,
    require_packaging_toolchain,
    resolve_research_source,
    resource_metadata,
    run,
    rustc_host_target,
    runtime_requirements,
    sha256_file,
    standalone_blockers,
    validate_bundle,
    validate_research_dependency_lock,
    write_sha256sums,
    write_json,
)


def release_input_fingerprint(root: Path) -> str:
    candidates = [
        root / "Cargo.toml",
        root / "Cargo.lock",
        root / "PRD.md",
        root / "build.rs",
        root / "rust-toolchain",
        root / "rust-toolchain.toml",
        root / ".cargo/config",
        root / ".cargo/config.toml",
        *sorted((root / "src").rglob("*")),
        *sorted((root / "assets").rglob("*")),
    ]
    digest = hashlib.sha256()
    for path in candidates:
        if not path.is_file():
            continue
        digest.update(path.relative_to(root).as_posix().encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def package_input_fingerprint(root: Path, research_source: Path) -> str:
    candidates = [
        *(("studio", path) for path in (
            root / "Cargo.toml",
            root / "Cargo.lock",
            root / "PRD.md",
            root / "build.rs",
            root / "rust-toolchain",
            root / "rust-toolchain.toml",
            root / ".cargo/config",
            root / ".cargo/config.toml",
            *sorted((root / "src").rglob("*")),
            *sorted((root / "assets").rglob("*")),
            *(root / "engine" / name for name in ENGINE_RESOURCES),
            *sorted((root / "packaging/macos").glob("*")),
            root / "scripts/macos_packaging.py",
            root / "scripts/package_macos.py",
        )),
        *(("research", research_source / name) for name in RESEARCH_RESOURCES),
        ("research", research_source / "pyproject.toml"),
        ("research", research_source / "uv.lock"),
    ]
    digest = hashlib.sha256()
    for prefix, path in candidates:
        if not path.is_file():
            continue
        base = root if prefix == "studio" else research_source
        digest.update(f"{prefix}/{path.relative_to(base).as_posix()}".encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def release_build_environment(config, target_dir: Path) -> dict[str, str]:
    env = os.environ.copy()
    encoded = env.get("CARGO_ENCODED_RUSTFLAGS", "")
    if encoded:
        flags = [flag for flag in encoded.split("\x1f") if flag]
    else:
        flags = shlex.split(env.pop("RUSTFLAGS", ""))

    configured_home = env.get("HOME", "").strip()
    home = Path(configured_home).resolve() if configured_home else None
    cargo_home_value = env.get("CARGO_HOME", "").strip()
    cargo_home = (
        Path(cargo_home_value).resolve()
        if cargo_home_value
        else (home / ".cargo" if home is not None else None)
    )
    rustup_home_value = env.get("RUSTUP_HOME", "").strip()
    rustup_home = (
        Path(rustup_home_value).resolve()
        if rustup_home_value
        else (home / ".rustup" if home is not None else None)
    )

    remaps: list[tuple[Path, str]] = []
    if home is not None:
        remaps.append((home, "BUILD_HOME"))
    if cargo_home is not None:
        remaps.append((cargo_home, "BUILD_CARGO"))
    if rustup_home is not None:
        remaps.append((rustup_home, "BUILD_RUSTUP"))
    remaps.append((config.root.resolve(), "reyn-studio"))
    if cargo_home is not None:
        remaps.extend(
            (
                (cargo_home / "registry/src", "crate-sources"),
                (cargo_home / "git/checkouts", "git-sources"),
            )
        )
    if rustup_home is not None:
        remaps.append((rustup_home / "toolchains", "rust-toolchain"))

    seen_sources: set[Path] = set()
    for source, replacement in remaps:
        if source in seen_sources:
            continue
        seen_sources.add(source)
        flags.append(f"--remap-path-prefix={source}={replacement}")
    flags.extend(("-Cdebuginfo=0", "-Cstrip=symbols"))

    wrapper = target_dir / ".reyn-rustc-workspace-wrapper.sh"
    wrapper.parent.mkdir(parents=True, exist_ok=True)
    wrapper.write_text(
        "#!/bin/sh\n"
        "# Keep env!(\"CARGO_MANIFEST_DIR\") deterministic and non-authoritative.\n"
        "export CARGO_MANIFEST_DIR=.\n"
        'exec "$@"\n',
        encoding="utf-8",
    )
    wrapper.chmod(0o755)

    env["CARGO_TARGET_DIR"] = str(target_dir)
    env["CARGO_INCREMENTAL"] = "0"
    env["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(flags)
    env["RUSTC_WORKSPACE_WRAPPER"] = str(wrapper)
    env["MACOSX_DEPLOYMENT_TARGET"] = config.minimum_system_version
    return env


def build_binary(
    config, target: str, target_dir: Path, work_dir: Path
) -> tuple[Path, list[dict[str, object]], str]:
    targets = PACKAGE_RUST_TARGETS[target]
    env = release_build_environment(config, target_dir)
    source_fingerprint = release_input_fingerprint(config.root)
    binaries: list[Path] = []
    slices: list[dict[str, object]] = []
    for rust_target in targets:
        print(f"Building locked release for {rust_target}…")
        run(
            [
                "cargo",
                "build",
                "--locked",
                "--release",
                "--target",
                rust_target,
                "--bin",
                config.executable,
            ],
            cwd=config.root,
            env=env,
        )
        current_fingerprint = release_input_fingerprint(config.root)
        if current_fingerprint != source_fingerprint:
            raise RuntimeError(
                "Rust release inputs changed during architecture builds; "
                f"started at {source_fingerprint}, now {current_fingerprint}. "
                "Discard these outputs and retry after the shared source is stable."
            )
        binary = target_dir / rust_target / "release" / config.executable
        if not binary.is_file():
            raise RuntimeError(
                f"Cargo reported success but did not create expected binary: {binary}"
            )
        architecture = RUST_TARGET_ARCHITECTURES[rust_target]
        require_executable_architectures(
            binary,
            (architecture,),
            context=f"{rust_target} Cargo output",
        )
        binaries.append(binary)
        slices.append(
            {
                "architecture": architecture,
                "rust_target": rust_target,
                "bytes": binary.stat().st_size,
                "source_binary_sha256": sha256_file(binary),
            }
        )
    if len(binaries) == 1:
        return binaries[0], slices, source_fingerprint
    universal = work_dir / config.executable
    run(
        ["lipo", "-create", *(str(path) for path in binaries), "-output", str(universal)],
        cwd=config.root,
        capture=True,
    )
    require_executable_architectures(
        universal,
        TARGET_ARCHITECTURES["universal2"],
        context="lipo universal2 output",
    )
    return universal, slices, source_fingerprint


def package(args: argparse.Namespace) -> int:
    root = Path(__file__).resolve().parents[1]
    if sys.platform != "darwin":
        raise RuntimeError("macOS packaging must run on macOS")
    config = load_config(root)
    target = rustc_host_target(root) if args.target == "host" else args.target
    if target not in TARGET_ARCHITECTURES:
        raise ValueError(f"unsupported macOS target: {target}")
    require_packaging_toolchain(root, target)
    if args.require_runnable_architectures:
        require_local_architecture_runtime(TARGET_ARCHITECTURES[target])
    print(
        f"Packaging target {target}: "
        f"{', '.join(TARGET_ARCHITECTURES[target])}"
    )
    research_source = resolve_research_source(root, args.research_source_dir)
    dependency_errors = validate_research_dependency_lock(research_source)
    if dependency_errors:
        raise RuntimeError(
            "research dependency inventory does not match packaged SBOM: "
            + "; ".join(dependency_errors)
        )
    package_fingerprint = package_input_fingerprint(root, research_source)

    output_dir = (root / args.output_dir).resolve()
    target_dir = (root / args.target_dir).resolve()
    icon_source = root / "packaging/macos/ReynStudio.icns"
    output_dir.mkdir(parents=True, exist_ok=True)
    target_dir.mkdir(parents=True, exist_ok=True)
    stage_root = Path(tempfile.mkdtemp(prefix=".reyn-macos-", dir=output_dir))
    try:
        bundle = stage_root / f"{config.display_name}.app"
        contents = bundle / "Contents"
        macos = contents / "MacOS"
        resources = contents / "Resources"
        macos.mkdir(parents=True)
        resources.mkdir(parents=True)

        binary, architecture_slices, source_fingerprint = build_binary(
            config, target, target_dir, stage_root
        )
        bundled_binary = macos / config.executable
        shutil.copy2(binary, bundled_binary)
        bundled_binary.chmod(0o755)
        actual_architectures = require_executable_architectures(
            bundled_binary,
            TARGET_ARCHITECTURES[target],
            context="staged app executable",
        )

        with (contents / "Info.plist").open("wb") as stream:
            plistlib.dump(
                info_plist(config, args.build_number),
                stream,
                fmt=plistlib.FMT_BINARY,
                sort_keys=True,
            )
        shutil.copy2(icon_source, resources / "ReynStudio.icns")
        copy_documentation_resources(root, resources / "docs")
        copy_engine_resources(root, resources / "engine")
        copy_research_resources(research_source, resources / "research")
        copy_security_resources(root, resources / "security")
        write_json(resources / "runtime-requirements.json", runtime_requirements())
        current_package_fingerprint = package_input_fingerprint(root, research_source)
        if current_package_fingerprint != package_fingerprint:
            raise RuntimeError(
                "macOS package inputs changed during the build; "
                f"started at {package_fingerprint}, now {current_package_fingerprint}. "
                "Discard these outputs and retry after the shared source is stable."
            )

        manifest_files = [
            path for path in contents.rglob("*") if path.is_file()
        ]
        release_manifest = {
            "schema": "com.reyn.studio.local-package.v2",
            "app_version": config.version,
            "build_number": args.build_number,
            "bundle_identifier": config.bundle_identifier,
            "cargo_lock_sha256": sha256_file(root / "Cargo.lock"),
            "release_input_sha256": package_fingerprint,
            "rust_source_input_sha256": source_fingerprint,
            "minimum_macos_version": config.minimum_system_version,
            "rust_target": target,
            "architectures": list(actual_architectures),
            "architecture_slices": architecture_slices,
            "resource_set": resource_metadata(resources),
            "apple_credentials_used": False,
            "developer_id_signed": False,
            "notarized": False,
            "standalone": False,
            "standalone_blockers": standalone_blockers(root),
            "files": file_manifest(contents, manifest_files),
        }
        write_json(resources / "release-manifest.json", release_manifest)

        checks = validate_bundle(
            bundle,
            config,
            expected_architectures=TARGET_ARCHITECTURES[target],
            require_runnable_architectures=args.require_runnable_architectures,
        )
        print_checks(checks)
        if has_failures(checks):
            return 1
        if args.require_standalone and standalone_blockers(root):
            print("\nStandalone release gate remains blocked; bundle was not published.")
            return 2

        destination = output_dir / bundle.name
        if destination.exists():
            shutil.rmtree(destination)
        bundle.replace(destination)

        target_label = "universal2" if target == "universal2" else TARGET_ARCHITECTURES[target][0]
        archive = output_dir / (
            f"Reyn-Studio-{config.version}-build.{args.build_number}-{target_label}.app.zip"
        )
        temporary_archive = stage_root / archive.name
        deterministic_zip(destination, temporary_archive, args.source_date_epoch)
        temporary_archive.replace(archive)
        checksum = sha256_file(archive)
        matching_archives = output_dir.glob(
            f"Reyn-Studio-{config.version}-build.{args.build_number}-*.app.zip"
        )
        write_sha256sums(matching_archives, output_dir / "SHA256SUMS")

        print(f"\nLocal app bundle: {destination}")
        print(f"Deterministic archive: {archive}")
        print(f"SHA-256: {checksum}")
        print(
            "Boundary: local development package only — not Developer ID signed, "
            "not notarized, and not standalone."
        )
        return 0
    finally:
        shutil.rmtree(stage_root, ignore_errors=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Build a structurally valid local macOS bundle without using Apple credentials. "
            "The current source does not yet satisfy the standalone release gate."
        )
    )
    parser.add_argument(
        "--target",
        default="host",
        choices=("host", *TARGET_ARCHITECTURES),
        help="Rust target or universal2 (default: current Rust host)",
    )
    parser.add_argument(
        "--build-number",
        default=os.environ.get("REYN_BUILD_NUMBER", "1"),
        help="CFBundleVersion value (default: REYN_BUILD_NUMBER or 1)",
    )
    parser.add_argument(
        "--output-dir",
        default="dist/macos",
        help="output directory relative to the project root",
    )
    parser.add_argument(
        "--target-dir",
        default="target/package-macos",
        help="isolated Cargo target directory relative to the project root",
    )
    parser.add_argument(
        "--research-source-dir",
        type=Path,
        help=(
            "source directory for lightweight research modules "
            "(default: REYN_RESEARCH_SOURCE_DIR or sibling reyn-research)"
        ),
    )
    parser.add_argument(
        "--source-date-epoch",
        type=int,
        default=int(os.environ.get("SOURCE_DATE_EPOCH", DEFAULT_SOURCE_DATE_EPOCH)),
        help="fixed archive timestamp",
    )
    parser.add_argument(
        "--require-standalone",
        action="store_true",
        help="fail instead of publishing while runtime/file-open blockers remain",
    )
    parser.add_argument(
        "--require-runnable-architectures",
        action="store_true",
        help=(
            "on Apple silicon, fail x86_64/universal2 packaging unless Rosetta "
            "is available for follow-up runtime tests"
        ),
    )
    return parser.parse_args()


if __name__ == "__main__":
    try:
        raise SystemExit(package(parse_args()))
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"macOS packaging failed: {error}", file=sys.stderr)
        raise SystemExit(1)
