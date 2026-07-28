#!/usr/bin/env bash
set -euo pipefail

[[ $# -ge 1 && $# -le 2 ]] || {
  echo "usage: $0 INPUT.mov [OUTPUT.mp4]" >&2
  exit 2
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INPUT="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
OUTPUT="${2:-${INPUT%.*}-yc.mp4}"
command -v ffmpeg >/dev/null || {
  echo "ffmpeg is required (install with: brew install ffmpeg)" >&2
  exit 1
}
command -v ffprobe >/dev/null || {
  echo "ffprobe is required (install with: brew install ffmpeg)" >&2
  exit 1
}
[[ -f "$INPUT" ]] || { echo "input not found: $INPUT" >&2; exit 1; }

DURATION="$(ffprobe -v error -show_entries format=duration \
  -of default=noprint_wrappers=1:nokey=1 "$INPUT")"
python3 - "$DURATION" <<'PY'
import sys
duration = float(sys.argv[1])
if not 150.0 <= duration <= 170.0:
    raise SystemExit(f"input duration {duration:.3f}s must be 150–170s")
PY

HAS_AUDIO="$(ffprobe -v error -select_streams a:0 -show_entries stream=index \
  -of csv=p=0 "$INPUT")"
TARGET_BYTES=95000000
AUDIO_KBPS=96
VIDEO_KBPS="$(python3 - "$DURATION" "$TARGET_BYTES" "$AUDIO_KBPS" "$HAS_AUDIO" <<'PY'
import sys
duration = float(sys.argv[1])
target_bytes = int(sys.argv[2])
audio_kbps = int(sys.argv[3]) if sys.argv[4].strip() else 0
# Leave 3% for the MP4 container and encoder variance.
total_kbps = int(target_bytes * 8 * 0.97 / duration / 1000)
print(max(500, total_kbps - audio_kbps))
PY
)"

PASS_DIR="$(mktemp -d "${TMPDIR:-/tmp}/reyn-yc-pass.XXXXXX")"
trap 'rm -rf "$PASS_DIR"' EXIT
FILTER="fps=30,scale=trunc(iw/2)*2:trunc(ih/2)*2"

ffmpeg -y -i "$INPUT" -vf "$FILTER" -c:v libx264 -preset slow \
  -b:v "${VIDEO_KBPS}k" -pass 1 -passlogfile "$PASS_DIR/pass" \
  -an -f mp4 /dev/null

if [[ -n "$HAS_AUDIO" ]]; then
  ffmpeg -y -i "$INPUT" -vf "$FILTER" -c:v libx264 -preset slow \
    -b:v "${VIDEO_KBPS}k" -pass 2 -passlogfile "$PASS_DIR/pass" \
    -c:a aac -b:a "${AUDIO_KBPS}k" -movflags +faststart -pix_fmt yuv420p "$OUTPUT"
else
  ffmpeg -y -i "$INPUT" -vf "$FILTER" -c:v libx264 -preset slow \
    -b:v "${VIDEO_KBPS}k" -pass 2 -passlogfile "$PASS_DIR/pass" \
    -an -movflags +faststart -pix_fmt yuv420p "$OUTPUT"
fi

python3 "$SCRIPT_DIR/validate_video.py" "$OUTPUT"
echo "YC-ready video: $OUTPUT"
