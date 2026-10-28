#!/bin/bash
# The kit's regression set: frames + audio of the existing kit examples, before and after a kit change.
#   tools/ballfight_port/regress.sh base OUT_BASE        # once, before changing the kit
#   tools/ballfight_port/regress.sh check OUT_BASE OUT   # after: prints the frames that differ + each wav comparison
# Examples: the kit examples and kit episodes (add new ones here as they are ported).
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
export PYTHONHASHSEED=0
cd "$HERE/../../projects"
MODE=$1; BASE=$2; OUT=${3:-$BASE}
EPS="flute_vs_trombone clarinet_vs_oboe ukulele_vs_acoustic viola_vs_violin drums_vs_violin trombone_vs_tuba ballfight_electric_vs_acoustic ballfight_piano_vs_organ"
FR="0 60 700 1200 1510 1600 1800 1829"
mkdir -p "$OUT"
for ep in $EPS; do
  python "$HERE/kitshot.py" $ep "$OUT/$ep" $FR > "$OUT/$ep.log" 2>&1 &
  python "$HERE/kitaudio.py" $ep "$OUT/$ep.wav" >> "$OUT/$ep.alog" 2>&1
  wait
  if [ "$MODE" = check ]; then
    echo "== $ep"
    python "$HERE/cmp.py" frames "$BASE/$ep" "$OUT/$ep" | grep -v identical
    python "$HERE/cmp.py" wav "$BASE/$ep.wav" "$OUT/$ep.wav"
  fi
done
