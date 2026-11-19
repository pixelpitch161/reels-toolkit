# What it prints: the weapon kind `drill` (SOUSAPHONE's DRILL SET, fight.py 4758 to 5005) and its abilities
# `flash_spin` and `company_front` on hand-built state: a set started plain and under haste (the line, its end, the
# footfall times, the blast time, the heading), a set cut short by the ropes, the march's position mid-step, a
# trample (the hit, the knock, the growth), the blast and its hit, the cone test at chosen points, the trampled
# baskets, HORN FLASH's start and hit, COMPANY FRONT's start, step, hit and baskets, as float.hex() of every float.
# The expected numbers in studio/crates/bf-sim/src/kinds/drill.rs (its test `the_kits_numbers`) are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/drill_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the drill's unit tests: hand-built state, the same calls, what fight.py gave.
Usage: drill_unit_vectors.py <repo root>"""
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
    f = F.Fight('SOUSAPHONE', 'FLUTE', seed=seed, rules=episode_rules(), record=False)
    f.drill_setup(0)
    f.striker_setup(1)
    f.t = 12.5
    return f


def place(f, bp, op, ov):
    f.balls[0].p = np.array(bp)
    f.balls[1].p = np.array(op)
    f.balls[1].v = np.array(ov)


def show_march(f):
    M = f.balls[0].march
    print('    march p0', vec(M['p0']), 'p1', vec(M['p1']), 'u', vec(M['u']), 'n', M['n'], 'kind', M['kind'])
    print('    t_steps', vec(M['t_steps']), 't_blast', hx(M['t_blast']), 'v', vec(f.balls[0].v))


R = episode_rules()
print('beat', hx(R.beat), 'dt', hx(R.DT), 'rules dmg_scale', hx(R.dmg_scale), 'dmg_side', R.dmg_side)

print('== A: a set starts on its grid (the jitter drawn), not off it')
f = new()
place(f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0])
f.drill_tick(0, 5)
print('    off the grid: march', f.balls[0].march, 'events', len(f.events))
f.drill_tick(0, 32)
show_march(f)
n = show_events(f)
print('    next draw', hx(f.rng.uniform()))

print('== B: under haste the set is every bar, on the 16ths')
f = new()
place(f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0])
f.balls[0].haste, f.balls[0].haste_mult = 1.0, 2.0
f.drill_tick(0, 16)
show_march(f)
show_events(f)

print('== C: a line cut short by the ropes, and the minimum length behind a near enemy')
f = new()
place(f, [2.1, 2.2], [2.3, 2.35], [0.5, 2.5])
f.drill_tick(0, 0)
show_march(f)
show_events(f)

print('== D: the march mid-step, then a trample')
f = new()
place(f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0])
f.drill_tick(0, 0)
n = len(f.events)
f.t = 12.5 + 0.3
f.drill_step(0, R.DT)
print('    p at +0.3', vec(f.balls[0].p), 'events', len(f.events) - n)
f.t = 12.5 + 0.4
f.balls[1].p = f.balls[0].p + np.array([0.9, 0.35])
f.balls[1].v = np.array([-2.0, 1.0])
hp = f.balls[1].hp
f.drill_step(0, R.DT)
print('    p at +0.4', vec(f.balls[0].p), 'hp', hx(f.balls[1].hp), 'was', hx(hp), 'O.v', vec(f.balls[1].v),
      'boost', hx(f.balls[1].boost), 'blade_len', hx(f.balls[0].blade_len), 'last', hx(f.balls[0].march['last']))
n = show_events(f, n)
f.t += R.DT                                               # within `cool`: no second trample
f.drill_step(0, R.DT)
print('    a step later: events', len(f.events) - n, 'hp', hx(f.balls[1].hp))

print('== E: the footfalls, the halt and the blast from drill_tick; the blast hits')
f = new()
place(f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0])
f.drill_tick(0, 0)
M = f.balls[0].march
n = len(f.events)
f.t = M['t_steps'][1]
f.drill_tick(0, 4)
n = show_events(f, n)
print('    i', M['i'], 'phase', M['phase'], 'walking', f.balls[0].walking)
f.t = M['t_steps'][-1]
f.drill_tick(0, 16)
n = show_events(f, n)
print('    phase', M['phase'], 'p', vec(f.balls[0].p), 'walking', f.balls[0].walking)
f.t = M['t_blast']
f.balls[0].blade_len = 2.24
f.drill_tick(0, 18)
n = show_events(f, n)
Bl = f.balls[0].blast
print('    blast o', vec(Bl['o']), 'd', vec(Bl['d']), 'len', hx(Bl['len']), 'w', hx(Bl['w']), 'w1', hx(Bl['w1']),
      't1', hx(Bl['t1']), 'march', f.balls[0].march)
nn = np.array([-Bl['d'][1], Bl['d'][0]])
f.balls[1].p = Bl['o'] + Bl['d'] * 1.5 + nn * 0.8
f.balls[1].v = np.array([0.4, 2.5])
f.drill_step(0, R.DT)
print('    hp', hx(f.balls[1].hp), 'O.v', vec(f.balls[1].v), 'boost', hx(f.balls[1].boost), 'blade_len',
      hx(f.balls[0].blade_len), 'hit', Bl['hit'], 'live', f.balls[0].blast is not None)
n = show_events(f, n)
f.t = Bl['t1']
f.drill_step(0, R.DT)
print('    at t1: live', f.balls[0].blast is not None, 'events', len(f.events) - n)

print('== F: _capsule_hit at chosen points (o = (0.25, -0.5), d at 0.6 rad, 2.24 long, 0.5 to 1.6 wide)')
o, d = np.array([0.25, -0.5]), np.array([np.float64(0.8253356149096783), np.float64(0.5646424733950354)])
for p in ([1.0, 0.2], [2.4, 1.9], [0.1, -1.4], [2.9, 0.9], [0.25, -0.5]):
    for wd1 in (1.6, None):
        hit, off = f._capsule_hit(o, d, 2.24, 0.5, np.array(p), wd1)
        print('    p', p, 'wd1', wd1, 'hit', bool(hit), 'off', vec(off))

print('== G: _trample_baskets: the cobras within r of c go, in the floor\'s order')
f = new()
f.balls[1].baskets = [dict(id=j, home=np.array(c), cp=np.array(c), t_drop=0.0, strike=None)
                      for j, c in enumerate(([0.5, 0.0], [2.0, 2.0], [-0.3, 0.4], [0.0, 0.8000000000000002]))]
f._trample_baskets(0, np.array([0.0, 0.0]), R.R + 0.2)
print('    left', [b['id'] for b in f.balls[1].baskets])
show_events(f)

print('== H: HORN FLASH starts facing away, turns the drawn way, hits once')
f = new()
place(f, [-0.4, 0.3], [0.9, 1.1], [1.0, 1.0])
ab = F.FIGHTERS['SOUSAPHONE']['abilities']['unlock1']
f.balls[0].blade_len = 2.36
f.start_flash(0, ab)
Fl = f.balls[0].flash
print('    a0', hx(Fl['a0']), 't1', hx(Fl['t1']), 'dir', hx(Fl['dir']), 'len', hx(Fl['len']), 'w', hx(Fl['w']))
n = show_events(f)
print('    next draw', hx(f.rng.uniform()))
f.t = 12.5 + 0.1 * R.beat
f.drill_step(0, R.DT)
print('    at 0.1 beat: hit', Fl['hit'], 'events', len(f.events) - n)
f.t = 12.5 + 0.5 * R.beat
hp = f.balls[1].hp
f.drill_step(0, R.DT)
print('    at 0.5 beat: hit', Fl['hit'], 'hp', hx(f.balls[1].hp), 'O.v', vec(f.balls[1].v), 'boost', hx(f.balls[1].boost))
n = show_events(f, n)
f.t = Fl['t1']
f.drill_step(0, R.DT)
print('    at t1: live', f.balls[0].flash is not None, 'events', len(f.events) - n)

print('== I: COMPANY FRONT: the axis, the steps, the hit, the baskets, the end')
f = new()
place(f, [0.6, 1.9], [-0.2, -1.4], [1.0, 1.0])
ab = F.FIGHTERS['SOUSAPHONE']['abilities']['low']
f.start_front(0, ab)
Fr = f.balls[0].front
print('    ax', Fr['ax'], 'sg', hx(Fr['sg']), 'a', hx(Fr['a']), 'b', hx(Fr['b']), 't1', hx(Fr['t1']), 'n', Fr['n'])
n = show_events(f)
f.balls[1].baskets = [dict(id=j, home=np.array(c), cp=np.array(c), t_drop=0.0, strike=None)
                      for j, c in enumerate(([0.5, 2.4], [2.0, 0.9], [-0.3, -2.9]))]
for beats in (0.3, 1.25, 2.7, 5.6):
    f.t = 12.5 + beats * R.beat
    f.drill_step(0, R.DT)
    print('    at', beats, 'beats: pos', hx(Fr['pos']), 'i', Fr['i'], 'hit', Fr['hit'], 'hp', hx(f.balls[1].hp),
          'baskets', [b['id'] for b in f.balls[1].baskets])
    n = show_events(f, n)
print('    O.v', vec(f.balls[1].v), 'boost', hx(f.balls[1].boost))
f.t = Fr['t1']
f.drill_step(0, R.DT)
print('    at t1: live', f.balls[0].front is not None)
show_events(f, n)

print('== J: the abilities on a ball that is not a driller (trigger tests hasattr(B, "march"))')
f = F.Fight('FLUTE', 'TROMBONE', seed=392, rules=episode_rules(), record=False)
f.t = 12.5
B = f.balls[0]
B.cfg = dict(B.cfg, abilities=dict(B.cfg['abilities'], unlock1=F.FIGHTERS['SOUSAPHONE']['abilities']['unlock1'],
                                   low=F.FIGHTERS['SOUSAPHONE']['abilities']['low']))
f.trigger(0, 'unlock1')
f.trigger(0, 'low')
show_events(f)
print('    flash', getattr(B, 'flash', 'no attribute'), 'front', getattr(B, 'front', 'no attribute'))
g = F.Fight('FLUTE', 'TROMBONE', seed=392, rules=episode_rules(), record=False)
print('    no draw', hx(f.rng.uniform()) == hx(g.rng.uniform()))
