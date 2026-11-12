# What it prints: the weapon kind `echo` (HORN's ECHO CALL, fight.py 2205 to 2281) and the ability `flick` (FINAL
# FLICK, 2290 to 2337) on hand-built state, in the branches no fight of the reference set reaches: a ring whose centre
# is within 1e-6 of the enemy (`u = O.v / (np.linalg.norm(O.v) + 1e-12)`), a reach that grows (`reach_grow` with
# `max_reach`), and a flick that misses, as float.hex() of every float. The expected lines in
# studio/crates/bf-sim/src/kinds/echo/kit_vectors.rs are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/echo_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not an
# export: the tool's export discovery does not look in this folder.
"""Python vectors for the echo's and the flick's unit tests: hand-built state, the same calls, what fight.py gave.
Usage: echo_unit_vectors.py <repo root>"""
import json, sys, zipfile
import numpy as np
clone = sys.argv[1]
sys.path.insert(0, clone + '/projects/ballfight')
import fight as F


def episode_rules():
    meta = json.loads(zipfile.ZipFile(clone + '/studio/reference/fights/flute_vs_trombone/ep392.bfrec').read('meta.json'))
    r = meta['rules']
    R = F.Rules(bpm=r['bpm'], t_fight=r['t_fight'])
    for k, v in r.items():
        setattr(R, k, tuple(v) if isinstance(v, list) else v)
    return R


def hx(x):
    return float(x).hex()


def vec(v):
    return '[' + ', '.join(hx(x) for x in v) + ']'


def show_events(f, start=0):
    for e in f.events[start:]:
        print('    ev', ' '.join(hx(x) if isinstance(x, (float, np.floating)) else str(x) for x in e))
    return len(f.events)


def new(w=None):
    f = F.Fight('HORN', 'FLUTE', seed=9, rules=episode_rules(), record=False)
    if w is not None:
        f.balls[0].w = dict(f.balls[0].w, **w)
        f.balls[0].blade_len = f.balls[0].w.get('base', 0.0)
    f.echo_setup(0)
    f.striker_setup(1)
    f.t = 12.5
    return f


def show(f):
    B, O = f.balls
    print('    n_echo', hx(B.n_echo), 'blade_len', hx(B.blade_len), 'O.p', vec(O.p), 'O.v', vec(O.v), 'O.hp', hx(O.hp),
          'O.boost', hx(O.boost))


R = episode_rules()
print('beat', hx(R.beat), 'dt', hx(R.DT), 'dmg_side', R.dmg_side)

print('== A: a ring whose centre is within 1e-6 of the enemy: the knock follows the enemy\'s own heading')
f = new()
f.balls[0].p = np.array([-1.2, 0.4])
f.balls[1].p = np.array([0.7, -0.3])
f.balls[1].v = np.array([1.3, -2.1])
f._echo_blast(0, f.balls[1].p + np.array([4e-7, -3e-7]), 0, 1)
f.echo_step(0, R.DT)
show(f)
show_events(f)

print('== B: a reach that grows: reach_grow 0.15 up to max_reach 1.1, two landed blasts')
f = new(dict(reach_grow=0.15, max_reach=1.1))
f.balls[0].p = np.array([-0.4, 0.2])
f.balls[1].p = np.array([0.5, 0.25])
f.balls[1].v = np.array([0.6, 0.9])
for call in (1, 2):
    n = len(f.events)
    f._echo_blast(0, f.balls[0].p.copy(), 0, call)
    f.echo_step(0, R.DT)
    show(f)
    show_events(f, n)

print('== C: FINAL FLICK out of reach: it misses at full reach, then is put away')
f = new()
ab = f.balls[0].cfg['abilities']['low']
f.balls[0].p = np.array([-2.1, -1.5])
f.balls[1].p = np.array([2.0, 1.9])
f.balls[1].v = np.array([0.4, -0.2])
f.start_flick(0, ab)
F_ = f.balls[0].flick
print('    wind t_hit', hx(F_['t_hit']), 'aim', hx(F_['aim']))
f.balls[1].p = np.array([2.3, 1.7])
f.t = F_['t_hit']
f.flick_step(0, R.DT)
F_ = f.balls[0].flick
print('    out state', F_['state'], 'aim', hx(F_['aim']), 'ext', hx(F_['ext']), 'hit', F_['hit'], 't_out', hx(F_['t_out']))
print('    rope', getattr(f.balls[0], 'flick_rope', None))
f.t = F_['t_out'] + 0.6
f.flick_step(0, R.DT)
print('    at show end', f.balls[0].flick is not None)
f.t = F_['t_out'] + 0.6 + R.DT
f.flick_step(0, R.DT)
print('    after', f.balls[0].flick)
show(f)
show_events(f)
