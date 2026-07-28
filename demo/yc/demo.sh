#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
YC="$ROOT/demo/yc"
ASSETS="$YC/assets"
STATE="$YC/.state"
RESEARCH="$ROOT/../reyn-research"
PRIMARY="$ASSETS/primary_capsule_d80_l260mm.stl"
MANIFEST="$ASSETS/fixture-manifest.json"

say() { printf '%s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

reset_demo() {
  if [[ -f "$STATE/app.pid" ]]; then
    pid="$(<"$STATE/app.pid")"
    if [[ "$pid" =~ ^[0-9]+$ ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  fi
  rm -rf "$STATE"
  mkdir -p "$STATE/config" "$STATE/home"
  say "Reset demo-only state: $STATE"
}

validate_assets() {
  python3 - "$YC/scripts" "$MANIFEST" <<'PY'
import sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
from fixture_tools import validate_manifest
for item in validate_manifest(Path(sys.argv[2])):
    print(f"OK {item['file']}: {item['triangles']} triangles, sha256={item['sha256']}")
PY
}

check_environment() {
  mkdir -p "$STATE"
  python3 "$YC/scripts/check_environment.py" | tee "$STATE/environment.json"
}

locate_or_build() {
  local binary="$ROOT/target/release/reyn-studio"
  command -v cargo >/dev/null || fail "cargo is required to build Reyn Studio"
  say "Building or verifying the optimized local app binary..." >&2
  cargo build --release --manifest-path "$ROOT/Cargo.toml" >&2
  [[ -x "$binary" ]] || fail "release binary was not produced at $binary"
  printf '%s\n' "$binary"
}

python_for_app() {
  if [[ -n "${REYN_PYTHON:-}" ]]; then
    printf '%s\n' "$REYN_PYTHON"
  elif [[ -x "$RESEARCH/.venv/bin/python" ]]; then
    printf '%s\n' "$RESEARCH/.venv/bin/python"
  else
    command -v python3
  fi
}

launch_demo() {
  local screenshot="${1:-}"
  local binary python_path
  binary="$(locate_or_build)"
  python_path="$(python_for_app)"
  local -a env_args=(
    "HOME=$STATE/home"
    "REYN_STUDIO_CONFIG_DIR=$STATE/config"
    "REYN_RESEARCH_DIR=$RESEARCH"
    "REYN_PYTHON=$python_path"
    "REYN_STUDIO_START_NAV=projects"
    "REYN_STUDIO_IMPORT=$PRIMARY"
  )
  if [[ -n "$screenshot" ]]; then
    env_args+=("REYN_STUDIO_SHOT=$screenshot")
  fi
  env "${env_args[@]}" "$binary" >"$STATE/reyn-studio-demo.log" 2>&1 &
  printf '%s\n' "$!" >"$STATE/app.pid"
  say "Launched isolated demo process $! using:"
  say "  $binary"
  say "Demo log:"
  say "  $STATE/reyn-studio-demo.log"
  say "No result fixture is supplied: the supported flow shows source evidence and preflight."
  say "See $YC/REFERENCE_RUN_BLOCKER.md for the reference-run schema decision."
}

smoke() {
  reset_demo
  validate_assets
  check_environment
  local shot="$STATE/native-smoke.png"
  launch_demo "$shot"
  for _ in {1..60}; do
    [[ -s "$shot" ]] && break
    sleep 0.5
  done
  [[ -s "$shot" ]] || fail "native screenshot was not written; inspect $STATE/reyn-studio-demo.log"
  say "Native screenshot smoke passed: $shot"
}

run_tests() {
  python3 -m unittest discover -s "$YC/tests" -p 'test_*.py' -v
}

case "${1:-prepare}" in
  prepare)
    reset_demo
    validate_assets
    check_environment
    launch_demo
    ;;
  validate)
    validate_assets
    check_environment
    binary="$(locate_or_build)"
    say "Release app binary: $binary"
    ;;
  inspect)
    validate_assets
    python3 -m json.tool "$MANIFEST"
    ;;
  smoke)
    smoke
    ;;
  test)
    run_tests
    ;;
  reset|cleanup)
    reset_demo
    ;;
  *)
    fail "usage: $0 {prepare|validate|inspect|smoke|test|reset}"
    ;;
esac
