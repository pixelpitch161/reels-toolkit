# What it prints: the weapon kind `walk` (CONTRABASS's WALKING BASS, fight.py 3965 to 4040) and its abilities `slap`
# (4202 to 4219) and `stampede` (4302 to 4368) on hand-built state: a step started on its grid (the jitter drawn) and
# off it, under haste, held back by a knock's boost, stopped short of contact and by the ropes; the swing mid-step,
# the landing and its quake ring, the ring's hit growing the reach; a walker pinned in a clash with and without a
# step; the slap's wait, its ring; the stampede's start, its turn, a rope bounce, a trample, a stomp and its end, as
# float.hex() of every float. The expected numbers in studio/crates/bf-sim/src/kinds/walk.rs (its tests
# `kit_vectors`) are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/walk_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the walk's unit tests: hand-built state, the same calls, what fight.py gave.
Usage: walk_unit_vectors.py <repo root>"""
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


def new(seed=392):
    f = F.Fight('CONTRABASS', 'FLUTE', seed=seed, rules=episode_rules(), record=False)
    f.walk_setup(0)
    f.striker_setup(1)
    f.t = 12.5
    return f


def place(f, bp, op, ov):
    f.balls[0].p = np.array(bp)
    f.balls[1].p = np.array(op)
    f.balls[1].v = np.array(ov)


def show_step(f):
    S = f.balls[0].wstep
    print('    wstep p0', vec(S['p0']), 'p1', vec(S['p1']), 't0', hx(S['t0']), 't1', hx(S['t1']), 'kind', S['kind'],
          'v', vec(f.balls[0].v), 'walking', f.balls[0].walking)


def show_wave(W):
    print('    wave c', vec(W['c']), 'r', hx(W['r']), 'speed', hx(W['speed']), 'rmax', hx(W['rmax']), 'dmg',
          hx(W['dmg']), 'knock', hx(W['knock']), 'kind', W['kind'], 'name', W['name'], 'grow', W.get('grow', False),
          'clears', W.get('clears', True))


R = episode_rules()
print('beat', hx(R.beat), 'dt', hx(R.DT), 'base', hx(F.Fight('CONTRABASS', 'FLUTE', seed=1, rules=R).balls[0].w['base']))

print('== A: a step starts only when its footfall lands on the grid; the jitter is drawn then')
f = new()
place(f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0])
f.walk_tick(0, 1)
print('    off the grid: wstep', f.balls[0].wstep, 'events', len(f.events))
f.walk_tick(0, 2)
show_step(f)
n = show_events(f)
print('    next draw', hx(f.rng.uniform()))

print('== B: under haste a step every 8th, of one 16th')
f = new()
place(f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0])
f.balls[0].haste, f.balls[0].haste_mult = 1.0, 2.0
f.walk_tick(0, 1)
show_step(f)
show_events(f)

print('== C: no step while the knock boost is above stagger, held or stampeding')
f = new()
place(f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0])
f.balls[0].boost = 0.61
f.walk_tick(0, 2)
f.balls[0].boost = 0.6
f.balls[0].grabbed = True
f.walk_tick(0, 2)
f.balls[0].grabbed = False
f.balls[0].stamp = dict(t1=99.0)
f.walk_tick(0, 2)
print('    events', len(f.events), 'wstep', f.balls[0].wstep)

print('== D: a step stops short of contact; one at the ropes is clipped')
f = new()
place(f, [0.0, 0.0], [0.55, 0.2], [0.0, 0.0])
f.walk_tick(0, 2)
show_step(f)
f = new()
place(f, [2.3, 2.3], [2.5, -2.5], [0.0, 0.0])
f.balls[0].p = np.array([2.3, 2.3])
f.balls[1].p = np.array([2.5, 2.4])
f.walk_tick(0, 2)
show_step(f)
f = new()
place(f, [0.3, 0.3], [0.3, 0.3], [0.0, 0.0])
f.walk_tick(0, 2)
show_step(f)

print('== E: the swing mid-step, the landing, its quake ring and its hit')
f = new()
place(f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0])
f.walk_tick(0, 2)
S = f.balls[0].wstep
n = len(f.events)
f.t = S['t0'] + 0.1
f.walk_step(0, R.DT)
print('    p at +0.1', vec(f.balls[0].p))
f.t = S['t1']
f.walk_step(0, R.DT)
print('    p landed', vec(f.balls[0].p), 'walking', f.balls[0].walking, 'wstep', f.balls[0].wstep)
n = show_events(f, n)
show_wave(f.waves[-1])
f.balls[1].p = f.balls[0].p + np.array([0.95, 0.3])
f.balls[1].v = np.array([0.5, 2.4])
hp = f.balls[1].hp
for i in range(12):
    f.t += R.DT
    f.effects_step(R.DT)
print('    hp', hx(f.balls[1].hp), 'was', hx(hp), 'O.v', vec(f.balls[1].v), 'boost', hx(f.balls[1].boost),
      'blade_len', hx(f.balls[0].blade_len), 'waves', len(f.waves))
n = show_events(f, n)

print('== F: a step lands from walk_tick when it is due within a substep')
f = new()
place(f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0])
f.walk_tick(0, 2)
S = f.balls[0].wstep
f.t = S['t1'] - R.DT * 0.5
n = len(f.events)
f.walk_tick(0, 4)
print('    p', vec(f.balls[0].p), 'wstep', f.balls[0].wstep is not None)
show_events(f, n)

print('== G: the reach grows to its cap')
f = new()
f.balls[0].blade_len = 1.48
for i in range(2):
    W = dict(owner=0, c=np.array([0.0, 0.0]), r=R.R, speed=9.0, rmax=3.0, dmg=1.0, knock=0.1, kind='step',
             hit=False, name='WALKING BASS', t0=f.t, slow=None, grow=True, clears=False)
    f.waves = [W]
    f.balls[1].p = np.array([0.92, 0.0])
    f.effects_step(R.DT)
    print('    blade_len', hx(f.balls[0].blade_len))

print('== H: a walker pinned against the ropes in a clash: its step restarts where it is')
f = new()
f.balls[0].p = np.array([2.0, 0.0])
f.balls[1].p = np.array([2.2, 0.05])
f.balls[0].v = np.array([1.0, 0.0])
f.balls[1].v = np.array([1.0, 0.0])
f.balls[0].wstep = dict(p0=np.array([1.5, 0.0]), p1=np.array([2.2, 0.0]), t0=12.4, t1=12.75, kind=0)
f.balls[0].walking = True
f.move(R.DT)
print('    W.p', vec(f.balls[0].p), 'O.p', vec(f.balls[1].p), 'p0', vec(f.balls[0].wstep['p0']),
      'p1', vec(f.balls[0].wstep['p1']))
f = new()
f.balls[0].p = np.array([2.0, 0.0])
f.balls[1].p = np.array([2.2, 0.05])
f.balls[0].v = np.array([1.0, 0.0])
f.balls[1].v = np.array([1.0, 0.0])
f.move(R.DT)
print('    planted: W.p', vec(f.balls[0].p), 'O.p', vec(f.balls[1].p), 'wstep', f.balls[0].wstep)

print('== I: SLAP BASS waits for the enemy, then rings')
f = new()
ab = f.balls[0].cfg['abilities']['unlock1']
place(f, [0.0, 0.0], [2.0, 0.0], [0.0, 0.0])
print('    ready far', f.slap_ready(0, ab))
f.balls[1].p = np.array([1.6, 0.0])
print('    ready near', f.slap_ready(0, ab))
f.balls[1].p = np.array([2.0, 0.0])
f.balls[0].grabbed = True
print('    ready held', f.slap_ready(0, ab))
f.balls[0].grabbed = False
f.start_slap(0, ab)
show_wave(f.waves[-1])
show_events(f)

print('== J: THE ELEPHANT: start (a step under way stops), turn, rope, trample, stomp, end')
f = new()
ab = f.balls[0].cfg['abilities']['low']
place(f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0])
f.walk_tick(0, 2)
f.balls[0].boost = 0.3
f.start_stampede(0, ab)
St = f.balls[0].stamp
print('    v', vec(f.balls[0].v), 'walking', f.balls[0].walking, 'boost', hx(f.balls[0].boost), 'wstep',
      f.balls[0].wstep, 't1', hx(St['t1']), 'next', hx(St['next']))
n = show_events(f)
f.stampede_step(0, R.DT)
print('    after a step: p', vec(f.balls[0].p), 'v', vec(f.balls[0].v), 'next', hx(St['next']))
show_wave(f.waves[-1])
n = show_events(f, n)
f.t += R.DT
f.balls[0].p = np.array([2.395, 0.0])
f.balls[0].v = np.array([3.0, 0.4])
f.stampede_step(0, R.DT)
print('    rope: p', vec(f.balls[0].p), 'v', vec(f.balls[0].v))
n = show_events(f, n)
f.t += R.DT
f.balls[0].p = np.array([0.0, 0.0])
f.balls[0].v = np.array([3.0, 0.0])
f.balls[1].p = np.array([0.45, 0.1])
f.balls[1].v = np.array([-1.0, 0.0])
hp = f.balls[1].hp
f.stampede_step(0, R.DT)
print('    trample: hp', hx(f.balls[1].hp), 'was', hx(hp), 'O.v', vec(f.balls[1].v), 'boost', hx(f.balls[1].boost),
      'v', vec(f.balls[0].v), 'last', hx(St['last']))
n = show_events(f, n)
f.t += R.DT
f.stampede_step(0, R.DT)
print('    within cool: hp', hx(f.balls[1].hp))
n = show_events(f, n)
f.t = St['next']
f.stampede_step(0, R.DT)
print('    stomp due: next', hx(St['next']))
n = show_events(f, n)
f.t = St['t1']
f.stampede_step(0, R.DT)
print('    end: stamp', f.balls[0].stamp, 'walking', f.balls[0].walking)
show_events(f, n)
