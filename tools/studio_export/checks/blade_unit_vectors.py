# What it prints: the thrust blade on hand-built state (start_thrust, thrust_step, the hit test, the parry, the
# grid): each case with the values fight.py gives, as repr() of every float.
# The expected numbers in studio/crates/bf-sim/src/kinds/blade/tests.rs are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/checks/blade_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the thrust blade's unit tests: hand-built state, the same calls, what fight.py gave.
Usage: blade_unit_vectors.py <repo root>"""
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


def new_fight(names=('FLUTE', 'TROMBONE'), seed=392, **over):
    f = F.Fight(names[0], names[1], seed=seed, rules=episode_rules(**over), record=False)
    for k, B in enumerate(f.balls):
        if B.w['kind'] == 'striker':
            f.striker_setup(k)
            B.t_strike = B.w['t_strike']
    return f


def fl(x):
    return repr(float(x))


def vec(v):
    return '[' + ', '.join(fl(x) for x in v) + ']'


def show_events(f, start=0):
    for e in f.events[start:]:
        print('   ', tuple(float(x) if isinstance(x, (float, np.floating)) else x for x in e))


def lance(f, k=1):
    B = f.balls[k]
    return f'th {fl(B.th)} ext {fl(B.ext)} thr {None if B.thr is None else fl(B.thr)} hit {B.thr_hit} k {fl(B.thr_k)} bl {fl(B.blade_len)} fr {B.frets}'


R0 = episode_rules()
beat, dt = R0.beat, R0.DT
print('beat', fl(beat), 'dt', fl(dt))

print('== new fight: the lance at the start')
f = new_fight()
print('  ', lance(f), 'p', vec(f.balls[1].p), 'enemy', vec(f.balls[0].p))
f = new_fight(names=('TROMBONE', 'FLUTE'))
print('   swapped', lance(f, 0))

print('== start_thrust')
for name, p, o, haste, bl in [
    ('plain', (1.0, -0.5), (-1.2, 0.7), 0.0, None),
    ('below (angle wraps to 2 pi)', (1.0, 0.5), (1.6, -1.0), 0.0, None),
    ('haste', (1.0, -0.5), (-1.2, 0.7), 1.0, 1.35),
    ('haste over', (1.0, -0.5), (-1.2, 0.7), 0.0, 1.35),
]:
    f = new_fight()
    B, O = f.balls[1], f.balls[0]
    B.p, O.p = np.array(p, float), np.array(o, float)
    B.haste, B.haste_mult = haste, 2.0
    B.thr_hit = True
    if bl is not None:
        B.blade_len = bl
    f.t = 12.5
    f.start_thrust(1)
    print(' ', name, lance(f))
    show_events(f)

print('== idle: the closed lance tracks the enemy')
for name, th0, p, o in [
    ('near: lands on the aim', 2.62, (1.0, -0.5), (-1.2, 0.7)),
    ('near, from above', 2.7, (1.0, -0.5), (-1.2, 0.7)),
    ('far, turning up', 1.0, (1.0, -0.5), (-1.2, 0.7)),
    ('far, turning down', 5.0, (1.0, -0.5), (-1.2, 0.7)),
    ('through 0 upward', 6.27, (0.0, 0.0), (1.0, 0.5)),
    ('through 0 downward', 0.01, (0.0, 0.0), (1.0, -0.5)),
    ('exactly opposite', 0.0, (0.0, 0.0), (-1.0, 0.0)),
]:
    f = new_fight()
    B, O = f.balls[1], f.balls[0]
    B.p, O.p = np.array(p, float), np.array(o, float)
    B.th = th0
    B.ext = 0.9
    ths = []
    for i in range(4):
        f.t = 12.5 + i * dt
        f.thrust_step(1, dt)
        ths.append(fl(B.th))
    print(' ', name, 'th', '[' + ', '.join(ths) + ']', 'ext', fl(B.ext), 'thr', B.thr, 'events', len(f.events))

print('== a whole thrust, nobody in reach: ext per step')
for name, haste, bl in [('plain', 0.0, None), ('haste', 1.0, None), ('grown', 0.0, 2.47)]:
    f = new_fight(swat_cobras=False)
    B, O = f.balls[1], f.balls[0]
    B.p, O.p = np.array([2.0, -2.0]), np.array([-2.0, 2.0])
    B.haste, B.haste_mult = haste, 2.0
    if bl is not None:
        B.blade_len = bl
    T = 12.5
    f.t = T
    f.start_thrust(1)
    rows = []
    for i in range(0, 42):
        f.t = T + i * dt
        f.thrust_step(1, dt)
        rows.append((i, fl(B.ext), B.thr is not None))
    print(' ', name, 'th', fl(B.th), 'k', fl(B.thr_k))
    last_live = max(i for i, _, on in rows if on)
    print('    thrusting through step', last_live, 'idle from step', last_live + 1, 'events', len(f.events))
    print('    ext', '[' + ', '.join(e for _, e, _ in rows[:last_live + 3]) + ']')

print('== a hit: once per thrust')
f = new_fight()
B, O = f.balls[1], f.balls[0]
B.p, O.p = np.array([1.0, -0.5]), np.array([-0.35, 0.3])
O.v = np.array([2.0, 1.0])
O.v = O.v / np.linalg.norm(O.v) * f.R.V
T = 12.5
f.t = T
f.start_thrust(1)
print('  start', lance(f), 'dist', fl(np.hypot(*(O.p - B.p))))
for i in range(0, 42):
    f.t = T + i * dt
    n0 = len(f.events)
    f.thrust_step(1, dt)
    if len(f.events) > n0:
        print('  step', i, lance(f))
        show_events(f, n0)
        print('    enemy hp', fl(O.hp), 'v', vec(O.v), 'boost', fl(O.boost), 'dealt', fl(B.dealt))
print('  after', lance(f), 'events', len(f.events), [e[1] for e in f.events])

print('== a hit while held (the enemy walks into the lance)')
f = new_fight()
B, O = f.balls[1], f.balls[0]
B.p, O.p = np.array([1.0, -0.5]), np.array([-1.2, 0.7])
T = 12.5
f.t = T
f.start_thrust(1)
for i in range(0, 42):
    f.t = T + i * dt
    if i == 12:
        # the enemy steps across the held lance, off its tip line
        d = np.array([math.cos(B.th), math.sin(B.th)])
        nrm = np.array([-d[1], d[0]])
        O.p = B.p + d * (0.6 * 0.9 + 0.6 * 0.1 + B.blade_len + 0.3) + nrm * 0.2
    n0 = len(f.events)
    f.thrust_step(1, dt)
    if len(f.events) > n0:
        print('  step', i, lance(f), 'enemy', vec(O.p))
        show_events(f, n0)
print('  after', lance(f), [e[1] for e in f.events])

print('== too late: the enemy touches the lance while it pulls back')
f = new_fight()
B, O = f.balls[1], f.balls[0]
B.p, O.p = np.array([1.0, -0.5]), np.array([-1.2, 0.7])
T = 12.5
f.t = T
f.start_thrust(1)
for i in range(0, 42):
    f.t = T + i * dt
    if i == 17:
        d = np.array([math.cos(B.th), math.sin(B.th)])
        O.p = B.p + d * 1.0
    f.thrust_step(1, dt)
    if i in (16, 17, 18):
        print('  step', i, 'u', fl((f.t - B.thr) / B.thr_k) if B.thr is not None else None, lance(f))
print('  after', lance(f), [e[1] for e in f.events], 'hp', fl(O.hp))

print('== a blocked thrust pulls straight back')
for name, haste in [('plain', 0.0), ('haste', 1.0)]:
    f = new_fight()
    B, O = f.balls[1], f.balls[0]
    B.p, O.p = np.array([1.0, -0.5]), np.array([-0.35, 0.3])
    O.v = np.array([2.0, 1.0])
    O.v = O.v / np.linalg.norm(O.v) * f.R.V
    v0 = O.v.copy()
    O.shield = 1.0
    B.haste, B.haste_mult = haste, 2.0
    T = 12.5
    f.t = T
    f.start_thrust(1)
    rows = []
    for i in range(0, 42):
        f.t = T + i * dt
        n0 = len(f.events)
        f.thrust_step(1, dt)
        rows.append((i, fl(B.ext), B.thr is not None))
        if len(f.events) > n0:
            print(' ', name, 'step', i, lance(f))
            show_events(f, n0)
            print('    enemy hp', fl(O.hp), 'v same', bool((O.v == v0).all()), 'boost', fl(O.boost), 'dealt', fl(B.dealt))
    last_live = max(i for i, _, on in rows if on)
    print('   ', name, 'thrusting through step', last_live, 'events', [e[1] for e in f.events])
    print('    ext', '[' + ', '.join(e for _, e, _ in rows[:last_live + 3]) + ']')
    print('    after', lance(f))

print('== growth: 0.08 a landed thrust up to max_len')
f = new_fight(dmg_side=(1.0, 0.01))
B, O = f.balls[1], f.balls[0]
seq = []
for n in range(24):
    B.p, O.p = np.array([1.0, -0.5]), np.array([0.2, -0.5])
    f.t = 12.5 + n
    f.start_thrust(1)
    f.thrust_step(1, dt)
    seq.append((fl(B.blade_len), B.frets, f.events[-1][1], f.events[-1][-1]))
    B.thr = None
print('  (blade_len, frets, last event, its sub)')
for s in seq:
    print('   ', s)
print('  hits', sum(1 for e in f.events if e[1] == 'hit'), 'hp', fl(O.hp))
for bl0 in (2.42, 2.4200000001, 2.43):
    f = new_fight()
    B, O = f.balls[1], f.balls[0]
    B.p, O.p = np.array([1.0, -0.5]), np.array([0.2, -0.5])
    B.blade_len = bl0
    f.t = 12.5
    f.start_thrust(1)
    f.thrust_step(1, dt)
    print('  from', fl(bl0), '->', fl(B.blade_len), B.frets)

print('== swat: the live lance through a risen cobra')


def basket(f, k, p, t_drop, cp=None):
    B = f.balls[k]
    bk = dict(id=B.next_bk, home=np.array(p, float), cp=np.array(p if cp is None else cp, float), t_drop=t_drop, strike=None)
    B.next_bk += 1
    B.baskets.append(bk)
    return bk


for name, swat_rule, cobra, t_drop in [
    ('going out', True, (-0.3, -0.5), 10.0),
    ('rule off', False, (-0.3, -0.5), 10.0),
    ('still rising', True, (-0.3, -0.5), 12.3),
    ('beside the line', True, (-0.3, -0.3), 10.0),
    ('just inside', True, (-0.3, -0.32), 10.0),
    ('beyond the tip', True, (-0.75, -0.5), 10.0),
    ('at the tip', True, (-0.73, -0.5), 10.0),
]:
    f = new_fight(swat_cobras=swat_rule)
    B, O = f.balls[1], f.balls[0]
    B.p, O.p = np.array([1.0, -0.5]), np.array([-2.5, -0.5])
    bk = basket(f, 0, cobra, t_drop)
    T = 12.5
    f.t = T
    f.start_thrust(1)
    for i in range(0, 42):
        f.t = T + i * dt
        n0 = len(f.events)
        f.thrust_step(1, dt)
        if len(f.events) > n0:
            print(' ', name, 'step', i, lance(f), 'down', fl(bk['down']))
            show_events(f, n0)
    print(' ', name, 'events', [e[1] for e in f.events], 'down', bk.get('down'))

print('== swat then strike: the cobra cannot lunge for 2 beats')
f = new_fight()
B, O = f.balls[1], f.balls[0]
B.p, O.p = np.array([1.0, -0.5]), np.array([-2.5, -0.5])
bk = basket(f, 0, (-0.3, -0.5), 10.0)
T = 12.5
f.t = T
f.start_thrust(1)
for i in range(0, 5):
    f.t = T + i * dt
    f.thrust_step(1, dt)
t_swat = [e for e in f.events if e[1] == 'swat'][0]
print('  swat', tuple(t_swat), 'down', fl(bk['down']), 'down - t', fl(bk['down'] - (T + 2 * dt)))
B.p = np.array([0.6, -0.1])           # the trombone in the cobra's reach
for off in (-0.2, -2e-9, 0.0):
    f.t = bk['down'] + off
    n0 = len(f.events)
    f.striker_fire(0)
    print('  fire at down', off, '->', [e[1] for e in f.events[n0:]], 'strike', bk['strike'] is not None)
    bk['strike'] = None

print('== swat and hit in one step: the swat comes first; a lunge in the air is spoiled')
f = new_fight()
B, O = f.balls[1], f.balls[0]
B.p, O.p = np.array([1.0, -0.5]), np.array([-0.4, -0.5])
bk = basket(f, 0, (0.1, -0.62), 10.0)
T = 12.5
f.t = T - 2 * dt
f.striker_fire(0)
print('  strike', {k: (fl(v) if isinstance(v, float) else v) for k, v in bk['strike'].items()})
f.t = T
f.start_thrust(1)
for i in range(0, 20):
    f.t = T + i * dt
    n0 = len(f.events)
    f.thrust_step(1, dt)
    f.striker_step(0, dt)
    if len(f.events) > n0:
        print('  step', i, lance(f))
        show_events(f, n0)
print('  strike', bk['strike'], 'down', fl(bk['down']), 'events', [e[1] for e in f.events], 'flute blade_len', fl(O.blade_len))

print('== parry: the live lance kills enemy projectiles it touches')


def parry_case(name, keys, parry=True, steps=(4,), stop_after=None):
    f = new_fight()
    B, O = f.balls[1], f.balls[0]
    if not parry:
        B.w = dict(B.w, parry=False)
    B.p, O.p = np.array([1.0, -0.5]), np.array([-2.0, 2.0])
    T = 12.5
    f.t = T
    f.start_thrust(1)
    out = []
    for i in range(0, 42):
        f.t = T + i * dt
        if i in steps:
            d = np.array([math.cos(B.th), math.sin(B.th)])
            nrm = np.array([-d[1], d[0]])
            a = B.p + d * 0.6 * 0.9
            for (owner, along, side) in keys:
                pos = a + d * along + nrm * side
                f.spawn_key(owner, pos, 0.3, 3.0, 2.0, 7, len(f.kid))
                out.append(vec(pos))
        n0 = len(f.events)
        f.thrust_step(1, dt)
        if i in steps:
            print(' ', name, 'step', i, 'ext', fl(B.ext), 'L', fl(0.6 * 0.1 + B.ext), 'th', fl(B.th))
            print('    spawned at', out)
            show_events(f, n0)
            print('    left: ids', f.kid.tolist(), 'owners', f.kown.tolist(), 'idx', f.kidx.tolist())


L4 = None
parry_case('on and off the line', [(0, 0.3, 0.0), (0, 0.3, 0.149), (0, 0.3, 0.151), (1, 0.3, 0.0), (0, 0.3, -0.149),
                                   (0, -0.1, 0.0), (0, -0.16, 0.0), (0, 0.6, 0.0), (0, 0.7, 0.0)])
parry_case('no parry option', [(0, 0.3, 0.0)], parry=False)
parry_case('pulling back', [(0, 0.3, 0.0)], steps=(20,))
parry_case('held', [(0, 0.3, 0.0), (0, 1.1, 0.0), (0, 1.17, 0.0)], steps=(12,))
parry_case('only its own', [(1, 0.3, 0.0), (1, 0.2, 0.05)])

print('== the grid: thrusts on the 16ths, every beat; every 8th under haste')
F.FIGHTERS['BAG'] = dict(color=(200, 200, 200), dark=(90, 90, 90), weapon=dict(kind='none'),
                         abilities=dict(unlock1=dict(name='N', kind='none', every_bars=4), unlock2=dict(name='N', kind='none', every_bars=4),
                                        low=dict(name='N', kind='none')), pp='', register=(60, 72), idiom='')


def held(steps, names, haste_from=None, haste_to=None):
    R = episode_rules()
    R.t_max = R.t_fight + (steps - 0.5) * R.DT
    f = F.Fight(names[0], names[1], seed=392, rules=R, record=False)
    k = [i for i, B in enumerate(f.balls) if B.w.get('mode') == 'thrust'][0]
    f.n = 0
    orig = f.abilities_step

    def hold():
        f.balls[0].p = np.array([-1.85, 1.85])
        f.balls[1].p = np.array([1.85, -1.85])
        if haste_from is not None and haste_from <= f.n < (haste_to or 10 ** 9):
            f.balls[k].haste = 1.0
            f.balls[k].haste_mult = 2.0

    def patched(on_beat):
        orig(on_beat)
        f.n += 1
        hold()
    f.abilities_step = patched
    hold()
    f.run()
    out = []
    for e in f.events:
        if e[1] == 'thrust':
            step = round((e[0] - R.t_fight) / R.DT)
            out.append((step, e[0], e[3]))
    return f, out


f, out = held(300, ('TROMBONE', 'BAG'))
print('  no haste, 300 steps: (step, t, reach)', out)
print('    kinds', sorted(set(e[1] for e in f.events)), 'th', fl(f.balls[0].th), 'ext', fl(f.balls[0].ext), 'thr', f.balls[0].thr)
f, out = held(330, ('BAG', 'TROMBONE'), haste_from=150)
print('  haste from step 150, 330 steps:', out)
f, out = held(330, ('TROMBONE', 'BAG'), haste_from=100, haste_to=200)
print('  haste on steps 100 to 199, 330 steps:', out)
del F.FIGHTERS['BAG']

print('== negative times: a block with t_out = 0 and t_hold < 0')
f = new_fight()
B, O = f.balls[1], f.balls[0]
B.w = dict(B.w, t_out=0.0, t_hold=-0.05)
B.p, O.p = np.array([1.0, -0.5]), np.array([0.2, -0.5])
O.shield = 1.0
f.t = 12.5
f.start_thrust(1)
try:
    for i in range(4):
        f.t = 12.5 + i * dt
        f.thrust_step(1, dt)
        print('  step', i, lance(f), [e[1] for e in f.events])
except Exception as e:
    print('  raised', type(e).__name__, e)
f = new_fight()
B, O = f.balls[1], f.balls[0]
B.haste, B.haste_mult = 1.0, 0.0
f.t = 12.5
try:
    f.start_thrust(1)
except Exception as e:
    print('  haste mult 0: start_thrust raised', type(e).__name__, e)
