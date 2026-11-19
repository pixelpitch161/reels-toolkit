# What it writes: stage dumps of kit.picture.render with the BASE-CLASS look (kit/fighters/base.py Look) for both
# fighters, as studio/reference/.cache/stages_default/flute_vs_trombone/f<frame>.npz + .json (git-ignored, not in
# the manifest; the same files the `stages` export writes, see tools/studio_export/stages.py).
# The ignored test dumps_of_every_stage_with_default_looks (studio/crates/bf-picture/tests/ring_stages.rs) compares
# every stage of the Studio's ring renderer, drawn with DefaultLook for both fighters, against these dumps.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/ring_default_looks.py . 150 600 1500
# The first argument is the repo root, the rest are the frames to dump (150 600 1500 when none is named). Nothing
# in the kit is edited: `ballfight.kit.fighters.look` is replaced, for this process only, before the renderer is
# imported. This is a check script, not an export: the tool's export discovery does not look in this folder.
"""Stage dumps of the kit's renderer with the base-class look for both fighters.
Usage: ring_default_looks.py <repo root> [frame ...]"""
import os
import sys

repo = os.path.abspath(sys.argv[1])
frames = [int(a) for a in sys.argv[2:]] or [150, 600, 1500]
sys.path.insert(0, repo)

from tools.studio_export import common            # noqa: E402
from tools.studio_export import frames as F       # noqa: E402
from tools.studio_export import stages as ST      # noqa: E402

common.kit()
from ballfight.kit import fighters as FM          # noqa: E402

FM.look = lambda name, P, k: FM.base.Look(P, k)   # every fighter draws as the bare ball
P = F.picture('ring_default_looks')
assert all(type(L) is FM.base.Look for L in P.LOOKS), [type(L) for L in P.LOOKS]
fight = F.check_recording(P)
ST.OUT = os.path.join(common.CACHE, 'stages_default', F.EPISODE)
for f in frames:
    ST.dump(P, f, fight)
print('ring_default_looks: ' + ', '.join(str(f) for f in frames))
