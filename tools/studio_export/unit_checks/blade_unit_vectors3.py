# What it prints: the edges of the thrust blade's hit test, with the events each case adds.
# The expected numbers in studio/crates/bf-sim/src/kinds/blade/tests.rs are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/blade_unit_vectors3.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""More Python vectors for the thrust blade's unit tests: the hit test's edges. Usage: blade_unit_vectors3.py <repo root>"""
import json, math, sys, zipfile
import numpy as np
clone = sys.argv[1]
sys.path.insert(0, clone + '/projects/ballfight')
import fight as F


def episode_rules(**over):
    meta = json.loads(zipfile.ZipFile(clone + '/studio/reference/fights/flute_vs_trombone/ep392.bfrec').read('meta.json'))
    r = meta['rules']
    R = F.Rules(bpm=r['bpm'], t_fight=r['t_fight'])
    for k, v in r.items():
        setattr(R, k, tuple(v) if isinstance(v, list) else v)
    for k, v in over.items():
        setattr(R, k, v)
    return R


def fl(x):
    return repr(float(x))


def vec(v):
    return '[' + ', '.join(fl(x) for x in v) + ']'


def show_events(f, start=0):
    for e in f.events[start:]:
        print('   ', tuple(float(x) if isinstance(x, (float, np.floating)) else x for x in e))


dt = episode_rules().DT
T = 12.5


def case(name, enemy_at_step, enemy, v0=(1.0, 2.4)):
    """the lance at (1.0, -0.5) thrusts along -x (the enemy starts at (-2.5, -0.5)); at `enemy_at_step` the enemy is
    put at `enemy`"""
    f = F.Fight('FLUTE', 'TROMBONE', seed=392, rules=episode_rules(swat_cobras=False), record=False)
    B, O = f.balls[1], f.balls[0]
    B.p, O.p = np.array([1.0, -0.5]), np.array([-2.5, -0.5])
    O.v = np.array(v0, float)
    f.t = T
    f.start_thrust(1)
    for i in range(0, 42):
        f.t = T + i * dt
        if i == enemy_at_step:
            O.p = np.array(enemy, float)
        n0 = len(f.events)
        f.thrust_step(1, dt)
        if len(f.events) > n0:
            print(' ', name, 'step', i, 'ext', fl(B.ext), 'th', fl(B.th))
            show_events(f, n0)
            print('    enemy v', vec(O.v), 'boost', fl(O.boost), 'hp', fl(O.hp))
    print(' ', name, 'events', [e[1] for e in f.events], 'thr_hit', B.thr_hit)


print('== the enemy behind the owner, on the line of the lance: no hit')
case('behind, on the line', 3, (2.3, -0.5))
case('behind, beside the line', 3, (2.3, -0.2))
print('== the ball centre a hair off the lance: the contact point')
case('1e-4 beside', 12, (-0.2, -0.5001))
case('1e-8 beside', 12, (-0.2, -0.50000001))
case('on it', 12, (-0.2, -0.5))
print('== within R of the held lance, or not (the lance lies on y = -0.5 from x = 0.46 to -0.55)')
case('0.599 beside', 12, (-0.2, -1.099))
case('0.601 beside', 12, (-0.2, -1.101))
case('0.599 beyond the tip', 12, (-1.149, -0.5))
case('0.601 beyond the tip', 12, (-1.151, -0.5))
print('== the knock is along the lance, not away from the owner')
case('beside the tip', 12, (-0.5, -1.0))
