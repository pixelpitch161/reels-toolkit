# What it prints: the ability kind `snip` (OBOE's DOUBLE REED, BASS CLARINET's SLAP TONGUE; fight.py 2526 to 2567
# and `trigger` 2882 to 2883) on hand-built state, as repr() of every float: `snip_ready` at the edge of its
# distance; a snip that opens, follows the enemy, snaps and hits, then goes away; one that misses by reach and one
# that misses by angle; the same snip from the right corner (ball 1); and BASS CLARINET's snip started on a ball
# whose own fighter has none (what IDENTITY CRISIS does with a copied snip).
# The expected numbers in studio/crates/bf-sim/src/abilities/snip.rs (its tests) are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/snip_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the snip's unit tests: hand-built state, the same calls, what fight.py gave.
Usage: snip_unit_vectors.py <repo root>"""
import json, sys, zipfile
import numpy as np
clone = sys.argv[1]
sys.path.insert(0, clone + '/projects/ballfight')
import fight as F


def rules():
    """the engine's default rules at the example episode's tempo and start (no handicap)"""
    meta = json.loads(zipfile.ZipFile(clone + '/studio/reference/fights/flute_vs_trombone/ep392.bfrec').read('meta.json'))
    r = meta['rules']
    return F.Rules(bpm=r['bpm'], t_fight=r['t_fight'])


def fl(x):
    return repr(float(x))


def vec(a):
    return '[' + ', '.join(fl(x) for x in a) + ']'


def show_events(f, start=0):
    for e in f.events[start:]:
        print('   ', tuple(float(x) if isinstance(x, (float, np.floating)) else x for x in e))


def snip(f, k):
    Sn = getattr(f.balls[k], 'snip', None)
    if Sn is None:
        return 'None'
    return (f"state {Sn['state']} aim {fl(Sn['aim'])} t_snap {fl(Sn['t_snap'])} "
            f"t_end {None if Sn['t_end'] is None else fl(Sn['t_end'])}")


def duel(left, right, k, enemy):
    """the snipper is ball k at (-1, 0.5), its enemy at `enemy`, the clock at T"""
    f = F.Fight(left, right, seed=392, rules=rules(), record=False)
    f.balls[k].p = np.array([-1.0, 0.5])
    f.balls[1 - k].p = np.array(enemy, float)
    f.balls[k].v = np.array([1.0, -2.0])
    f.balls[1 - k].v = np.array([-0.5, 2.0])
    f.t = T
    return f


def run(f, k, drift=(0.0, 0.0), jump=None, show=(1, 2)):
    """snip_step once per step from T until the blades are gone (at most 80 steps); the enemy drifts by `drift` a
    step and, with `jump` = (step, place), is put at `place` before that step"""
    R = f.R
    for i in range(1, 81):
        f.t = T + i * R.DT
        f.balls[1 - k].p = f.balls[1 - k].p + np.array(drift)
        if jump and i == jump[0]:
            f.balls[1 - k].p = np.array(jump[1], float)
        n0 = len(f.events)
        f.snip_step(k, R.DT)
        if i in show or len(f.events) > n0:
            print('   step', i, 't', fl(f.t), snip(f, k))
            show_events(f, n0)
        if getattr(f.balls[k], 'snip', None) is None:
            print('   gone at step', i, 't', fl(f.t))
            break
    O = f.balls[1 - k]
    print('   victim hp', fl(O.hp), 'v', vec(O.v), 'boost', fl(O.boost), 'dealt', fl(f.balls[k].dealt))


R = rules()
T = 12.5
print('beat', fl(R.beat), 'dt', fl(R.DT))

print('== snip_ready: the enemy within `ready` beyond the two surfaces')
ab = F.FIGHTERS['OBOE']['abilities']['unlock1']
for x in (2.29, 2.3, 2.31):
    f = duel('OBOE', 'FLUTE', 0, (-1.0 + x, 0.5))
    print('   dist', fl(x), 'ready', f.snip_ready(0, ab), 'left', fl(float(np.hypot(*(f.balls[1].p - f.balls[0].p))) - 2 * R.R))
f = duel('FLUTE', 'OBOE', 1, (1.29, 0.5))
print('   from the right corner', f.snip_ready(1, ab), 'its enemy asked', f.snip_ready(0, ab))

print('== OBOE left: the blades open, follow the enemy, snap, hit and go')
f = duel('OBOE', 'FLUTE', 0, (0.5, -0.1))
f.trigger(0, 'unlock1')
show_events(f)
print('  ', snip(f, 0), 'used', sorted(f.balls[0].used))
run(f, 0, drift=(0.0, 0.02))

print('== a miss by reach: the enemy too far at the snap')
f = duel('OBOE', 'FLUTE', 0, (1.2, -0.7))
f.trigger(0, 'unlock1')
run(f, 0, show=())

print('== a miss by angle: the enemy jumps over the oboe one step before the snap')
f = duel('OBOE', 'FLUTE', 0, (0.5, -0.1))
f.trigger(0, 'unlock1')
run(f, 0, jump=(27, (-1.0, 1.9)), show=(26,))

print('== OBOE right (ball 1): the same snip')
f = duel('FLUTE', 'OBOE', 1, (0.5, -0.1))
f.trigger(1, 'unlock1')
show_events(f)
run(f, 1, drift=(0.0, 0.02))

print('== a copied snip: BASS CLARINET\'s SLAP TONGUE started on FLUTE\'s ball (FLUTE against TROMBONE)')
slap = F.FIGHTERS['BASS CLARINET']['abilities']['unlock1']
f = duel('FLUTE', 'TROMBONE', 0, (0.2, 0.3))
print('   ready', f.snip_ready(0, slap))
f.start_snip(0, slap)
show_events(f)
print('  ', snip(f, 0), 'used', sorted(f.balls[0].used))
run(f, 0)
