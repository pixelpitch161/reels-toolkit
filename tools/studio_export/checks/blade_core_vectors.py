# What it prints: np.clip on edge cases (NaN, signed zeros, crossed bounds) and Fight.kill_keys (the events and
# the arrays left).
# The expected numbers of the np_clip and kill_keys tests (studio/crates/bf-sim/src/geom.rs, fight.rs) are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/checks/blade_core_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for np.clip and Fight.kill_keys. Usage: blade_core_vectors.py <repo root>"""
import sys
import numpy as np
clone = sys.argv[1]
sys.path.insert(0, clone + '/projects/ballfight')
import fight as F

print('== np.clip(x, lo, hi)')
for lo, hi in ((0, 1.0), (0, -0.2), (0.0, 0.0)):
    xs = np.array([-1.0, 0.5, 2.0, np.nan, -0.0, 0.0, 1.0, -0.2, -0.3])
    out = np.clip(xs, lo, hi)
    print('  lo', lo, 'hi', hi, [repr(float(x)) for x in xs], '->', [repr(float(x)) for x in out])

print('== kill_keys')
f = F.Fight('FLUTE', 'TROMBONE', seed=392, rules=F.Rules(bpm=133.77, t_fight=3.5), record=False)
f.t = 7.123456
for i, (own, pos) in enumerate([(0, (0.5, -0.25)), (1, (1.5, 2.0)), (0, (-1.0, 0.125)), (1, (0.0, 0.0))]):
    f.spawn_key(own, np.array(pos), 0.3, 3.0, 2.0 + i, 7, i)
f.kill_keys(np.array([False, False, False, False]), 'wall')
print('  none marked: events', f.events, 'ids', f.kid.tolist())
f.kill_keys(np.array([True, False, True, True]), 'wall')
print('  wall:', f.events, 'left ids', f.kid.tolist(), 'own', f.kown.tolist(), 'kp', f.kp.tolist(), 'dmg', f.kdmg.tolist())
f.kill_keys(np.array([True]), 'parry', 0)
print('  parry by 0:', f.events[-1], 'left', len(f.kp))
