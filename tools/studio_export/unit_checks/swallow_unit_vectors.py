# What it prints: the weapon kind `swallow` (TUBA's the bell, fight.py 1045 to 1130) on hand-built state: where the
# bell faces at the start, an oom on and off its grid, plain and under haste, the aim's turn, the suction's steer, a
# gulp (the hit, the held enemy, the growth), the ride inside, the pah with and without a catch (the seeded jitter,
# the spit), the cannonball on a rope hit and its time-out, a gulp on a shield, a rolling
# enemy with and without the rule `swallow_rollers`, a spit from the middle of the ring and the tuba as the
# right-hand fighter, as float.hex() of every float.
# The expected numbers in studio/crates/bf-sim/src/kinds/swallow/tests.rs are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/swallow_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the swallow's unit tests: hand-built state, the same calls, what fight.py gave.
Usage: swallow_unit_vectors.py <repo root>"""
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


def new(seed=392, k=0, rules=None):
    names = ('TUBA', 'FLUTE') if k == 0 else ('FLUTE', 'TUBA')
    f = F.Fight(names[0], names[1], seed=seed, rules=rules or episode_rules(), record=False)
    f.swallow_setup(k)
    f.t = 12.5
    return f


def place(f, k, bp, op, ov):
    f.balls[k].p = np.array(bp)
    f.balls[1 - k].p = np.array(op)
    f.balls[1 - k].v = np.array(ov)


def show(f, k, what=''):
    B, O = f.balls[k], f.balls[1 - k]
    print('   ', what, 'th', hx(B.th), 'reach', hx(B.blade_len), 'O.p', vec(O.p), 'O.v', vec(O.v), 'boost', hx(O.boost),
          'held', bool(getattr(O, 'held', False)), 'hp', hx(O.hp), 'dealt', hx(B.dealt))
    print('    sw', None if B.sw is None else ' '.join(f'{n}={hx(v) if isinstance(v, float) else v}' for n, v in B.sw.items()),
          'cannon', None if B.cannon is None else f"t_end={hx(B.cannon['t_end'])} ev_i={B.cannon['ev_i']}")


R = episode_rules()
DT = R.DT
print('beat', hx(R.beat), 'dt', hx(DT), 'R', hx(R.R), 'A', hx(R.A), 'V', hx(R.V), 'dmg_scale', hx(R.dmg_scale),
      'dmg_side', R.dmg_side)

print('== A: the bell starts facing the enemy')
for k in (0, 1):
    f = F.Fight(*(('TUBA', 'FLUTE') if k == 0 else ('FLUTE', 'TUBA')), seed=392, rules=episode_rules(), record=False)
    f.swallow_setup(k)
    print('    side', k, 'th', hx(f.balls[k].th), 'reach', hx(f.balls[k].blade_len), 'sw', f.balls[k].sw, 'cannon',
          f.balls[k].cannon)
f = new()
place(f, 0, [0.5, 1.0], [-1.25, -0.75], [1.0, 0.0])
f.swallow_setup(0)
print('    from [0.5, 1.0] at [-1.25, -0.75]: th', hx(f.balls[0].th))

print('== B: an oom on the grid, none off it, none while one runs; under haste twice as often and twice as fast')
f = new()
f.swallow_beat(0, 5)
print('    beat 5: sw', f.balls[0].sw, 'events', len(f.events))
f.swallow_beat(0, 8)
show(f, 0, 'beat 8:')
n = show_events(f)
f.t = 13.0
f.swallow_beat(0, 12)
print('    beat 12 while one runs: events', len(f.events), 't0', hx(f.balls[0].sw['t0']))
f.balls[0].sw['done'] = True
f.swallow_beat(0, 12)
show(f, 0, 'beat 12 after the pah:')
n = show_events(f, n)
f = new()
f.balls[0].haste, f.balls[0].haste_mult = 1.0, 2.0
f.swallow_beat(0, 5)
print('    hasted, beat 5: sw', f.balls[0].sw)
f.swallow_beat(0, 6)
show(f, 0, 'hasted, beat 6:')
show_events(f)

print('== C: the aim turns at most aim_rate x dt a step; with no oom nothing else happens')
f = new()
place(f, 0, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0])
f.balls[0].th = 1.0
for i in range(3):
    f.t += DT
    f.swallow_step(0, DT)
    show(f, 0, f'step {i + 1}:')
f.balls[0].th = 5.79
f.swallow_step(0, DT)
print('    close to the aim: th', hx(f.balls[0].th))
f.balls[0].th = 0.01
place(f, 0, [-1.0, 0.5], [1.2, 0.45], [1.3, -2.0])
f.swallow_step(0, DT)
print('    across 0: th', hx(f.balls[0].th))

print('== D: the suction steers an enemy within the reach (its speed kept), and no further out')
f = new()
place(f, 0, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0])
f.swallow_beat(0, 8)
f.t += DT
f.swallow_step(0, DT)
show(f, 0, 'in reach:')
print('    dist - 2R', hx(float(np.hypot(*(f.balls[1].p - f.balls[0].p))) - 2 * R.R), 'speed', hx(float(np.linalg.norm(f.balls[1].v))))
f = new()
place(f, 0, [-2.5, 0.5], [2.4, -0.7], [1.3, -2.0])
f.swallow_beat(0, 8)
f.t += DT
f.swallow_step(0, DT)
show(f, 0, 'out of reach:')
print('    dist - 2R', hx(float(np.hypot(*(f.balls[1].p - f.balls[0].p))) - 2 * R.R))

print('== E: a gulp, the ride inside, the pah (one draw), the cannonball on a rope hit')
f = new()
place(f, 0, [-1.0, 0.5], [-0.3, 0.1], [1.3, -2.0])
f.swallow_beat(0, 8)
n = show_events(f)
f.t += DT
f.swallow_step(0, DT)
show(f, 0, 'gulp:')
n = show_events(f, n)
f.balls[0].p = np.array([-0.9, 0.6])                     # the tuba moves on: the enemy rides inside
f.t += DT
f.swallow_step(0, DT)
show(f, 0, 'inside:')
print('    events', len(f.events) - n)
f.t = f.balls[0].sw['t_pah'] - 0.5 * DT
f.swallow_step(0, DT)
print('    before the pah: done', f.balls[0].sw['done'], 'events', len(f.events) - n, 'th', hx(f.balls[0].th))
f.t += DT
f.swallow_step(0, DT)
show(f, 0, 'pah:')
n = show_events(f, n)
f.t += DT
f.swallow_step(0, DT)
show(f, 0, 'flying:')
f.ev('rope', 0, 1, -1, 0.25, -3.0, 4.0)                  # the tuba's own rope hit is not the cannonball
f.t += DT
f.swallow_step(0, DT)
show(f, 0, 'own rope:')
n = len(f.events)
f.ev('rope', 1, 0, 1, 3.25, -1.5, 5.0)
f.t += DT
f.swallow_step(0, DT)
show(f, 0, 'the cannonball:')
show_events(f, n)
print('    next draw', hx(f.rng.uniform()))
g = new()
print('    a fresh generator: first', hx(g.rng.uniform(-1, 1)), 'second', hx(g.rng.uniform()))

print('== F: the cannonball times out')
f = new()
place(f, 0, [-1.0, 0.5], [-0.3, 0.1], [1.3, -2.0])
f.swallow_beat(0, 8)
f.t += DT
f.swallow_step(0, DT)
f.t = f.balls[0].sw['t_pah']
f.swallow_step(0, DT)
show(f, 0, 'pah:')
t_end = f.balls[0].cannon['t_end']
f.t = t_end
f.swallow_step(0, DT)
print('    at t_end: cannon', f.balls[0].cannon is not None)
f.t = t_end + DT
f.swallow_step(0, DT)
print('    past t_end: cannon', f.balls[0].cannon is not None)
n = len(f.events)
f.ev('rope', 1, 0, 1, 3.25, -1.5, 5.0)
f.t += DT
f.swallow_step(0, DT)
print('    a late rope hit: events', len(f.events) - n, 'hp', hx(f.balls[1].hp))

print('== G: a pah with nothing caught')
f = new()
place(f, 0, [-2.5, 0.5], [2.4, -0.7], [1.3, -2.0])
f.swallow_beat(0, 8)
n = show_events(f)
f.t = f.balls[0].sw['t_pah']
f.swallow_step(0, DT)
show(f, 0, 'pah:')
show_events(f, n)
print('    next draw', hx(f.rng.uniform(-1, 1)))

print('== H: a gulp on a shield: a block, the inhale ends')
f = new()
place(f, 0, [-1.0, 0.5], [-0.3, 0.1], [1.3, -2.0])
f.balls[1].shield = 0.75
f.swallow_beat(0, 8)
n = show_events(f)
f.t += DT
f.swallow_step(0, DT)
show(f, 0, 'blocked:')
n = show_events(f, n)
f.t += DT
f.swallow_step(0, DT)
show(f, 0, 'next step:')
print('    events', len(f.events) - n)

print('== I: a rolling enemy is steered but not gulped; with the rule swallow_rollers it is')
for on in (False, True):
    rules = episode_rules()
    if on:
        rules.swallow_rollers = True
    f = new(rules=rules)
    place(f, 0, [-1.0, 0.5], [-0.3, 0.1], [1.3, -2.0])
    f.balls[1].rolling = True
    f.swallow_beat(0, 8)
    n = len(f.events)
    f.t += DT
    f.swallow_step(0, DT)
    show(f, 0, f'swallow_rollers {on}:')
    show_events(f, n)

print('== J: from the middle of the ring the spit goes where the bell points; the right-hand fighter as the tuba')
f = new()
place(f, 0, [0.1, -0.2], [0.6, 0.1], [1.3, -2.0])
f.swallow_beat(0, 8)
f.t += DT
f.swallow_step(0, DT)
n = len(f.events)
f.t = f.balls[0].sw['t_pah']
f.swallow_step(0, DT)
show(f, 0, 'pah:')
show_events(f, n)
f = new(seed=7, k=1)
place(f, 1, [3.0, -3.1], [2.6, -2.8], [1.3, -2.0])
f.swallow_beat(1, 8)
f.t += DT
f.swallow_step(1, DT)
n = show_events(f)
f.balls[1].p = np.array([3.05, -3.1])
f.t = f.balls[1].sw['t_pah']
f.swallow_step(1, DT)
show(f, 1, 'pah:')
show_events(f, n)
