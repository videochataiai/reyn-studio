#!/usr/bin/env python3
"""Validate a staged Reyn Studio app without claiming Apple release readiness."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from macos_packaging import (
    TARGET_ARCHITECTURES,
    has_failures,
    load_config,
    print_checks,
    standalone_blockers,
    validate_bundle,
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "bundle",
        nargs="?",
        default="dist/macos/Reyn Studio.app",
        help="path to the .app bundle",
    )
    parser.add_argument(
        "--require-standalone",
        action="store_true",
        help="also fail while runtime, file-open, signing, or notarization gates remain",
    )
    parser.add_argument(
        "--expect-target",
        choices=tuple(TARGET_ARCHITECTURES),
        help="require the bundle to match this Rust target/universal2 architecture set",
    )
    parser.add_argument(
        "--require-runnable-architectures",
        action="store_true",
        help=(
            "on Apple silicon, fail an x86_64 slice when Rosetta is unavailable "
            "for follow-up runtime tests"
        ),
    )
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[1]
    bundle = Path(args.bundle)
    if not bundle.is_absolute():
        bundle = root / bundle
    config = load_config(root)
    checks = validate_bundle(
        bundle.resolve(),
        config,
        expected_architectures=(
            TARGET_ARCHITECTURES[args.expect_target]
            if args.expect_target is not None
            else None
        ),
        require_runnable_architectures=args.require_runnable_architectures,
    )
    print_checks(checks)
    if has_failures(checks):
        return 1
    if args.require_standalone and standalone_blockers(root):
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
