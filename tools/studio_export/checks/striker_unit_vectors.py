# What it prints: the striker on hand-built state (striker_beat, striker_fire, striker_step, swat_cobras): each
# case with the values fight.py gives, as repr() of every float.
# The expected numbers in studio/crates/bf-sim/src/kinds/striker/tests.rs are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/checks/striker_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the striker's unit tests: hand-built state, one call, what fight.py gave.
Usage: striker_unit_vectors.py <repo root>"""
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


def new_fight(names=('FLUTE', 'TROMBONE'), seed=392):
    f = F.Fight(names[0], names[1], seed=seed, rules=episode_rules(), record=False)
    for k, B in enumerate(f.balls):
        if B.w['kind'] == 'striker':
            f.striker_setup(k)
            B.t_strike = B.w['t_strike']
    return f


def fl(x):
    return repr(float(x))


def show_events(f, start=0):
    for e in f.events[start:]:
        print('   ', tuple(float(x) if isinstance(x, (float, np.floating)) else x for x in e))


def basket(f, k, p, t_drop, cp=None):
    B = f.balls[k]
    bk = dict(id=B.next_bk, home=np.array(p, float), cp=np.array(p if cp is None else cp, float), t_drop=t_drop, strike=None)
    B.next_bk += 1
    B.baskets.append(bk)
    return bk


def show_strike(bk):
    S = bk['strike']
    if S is None:
        print('    strike None')
    else:
        print('    strike', {k: (fl(v) if isinstance(v, float) else v) for k, v in S.items()})


beat = episode_rules().beat
dt = episode_rules().DT
print('beat', fl(beat), 'dt', fl(dt))

print('== beat: drops')
f = new_fight()
for kb in range(0, 14):
    f.t = 3.5 + kb * 0.5
    f.balls[0].p = np.array([-1.85 + 0.25 * kb, 1.85 - 0.125 * kb])
    f.striker_beat(0, kb)
    print('  kb', kb, 'ids', [b['id'] for b in f.balls[0].baskets], 'next', f.balls[0].next_bk)
show_events(f)
print('  baskets', [(b['id'], b['home'].tolist(), b['cp'].tolist(), b['t_drop'], b['strike']) for b in f.balls[0].baskets])

print('== fire')


def fire_case(name, enemy, cobra=(0.5, -0.25), t_off=0.0, pre=None):
    f = new_fight()
    bk = basket(f, 0, cobra, 10.0)
    f.balls[1].p = np.array(enemy, float)
    f.t = 10.0 + beat + t_off
    if pre:
        pre(f, bk)
    f.striker_fire(0)
    print(' ', name, 't', fl(f.t))
    show_events(f)
    show_strike(bk)
    return f, bk


def haste(f, bk):
    f.balls[0].haste = 1.0
    f.balls[0].haste_mult = 2.0


def down_future(f, bk):
    bk['down'] = f.t + 0.25


def down_past(f, bk):
    bk['down'] = f.t


def grown(f, bk):
    f.balls[0].blade_len = 2.4
    f.balls[0].t_strike = 0.07


fire_case('a in reach', (1.9, 0.6))
fire_case('b far', (-2.3, 2.3), cobra=(2.0, -2.0))
fire_case('d haste', (1.9, 0.6), pre=haste)
fire_case('e near', (0.9, 0.05))
fire_case('h1 not risen', (1.9, 0.6), t_off=-2e-9)
fire_case('h2 risen within the slack', (1.9, 0.6), t_off=-5e-10)
fire_case('g1 down', (1.9, 0.6), pre=down_future)
fire_case('g2 down over', (1.9, 0.6), pre=down_past)
fire_case('i grown, far end of the reach', (2.9, 1.9), pre=grown)
fire_case('j just out of reach', (2.9, 1.9))


def edge(off):
    f = new_fight()
    bk = basket(f, 0, (0.0, 0.0), 10.0)
    f.balls[1].p = np.array([0.6 + 1.4 + 0.25 + off, 0.0])
    f.t = 10.0 + beat
    f.striker_fire(0)
    print('  edge', off, 'x', fl(f.balls[1].p[0]), 'struck', bk['strike'] is not None)
    show_events(f)


for off in (-1e-9, 0.0, 1e-9, 2.5e-16, 4.5e-16):
    edge(off)
print(' two baskets, one busy')
f = new_fight()
b0 = basket(f, 0, (0.5, -0.25), 10.0)
b1 = basket(f, 0, (-0.75, 1.0), 10.2)
b2 = basket(f, 0, (1.0, 1.0), 10.3)
f.balls[1].p = np.array([0.4, 0.9])
f.t = 10.3 + beat
b1['strike'] = dict(t0=f.t - 0.2, t_hit=f.t - 0.08, t_end=f.t + 0.112, aim=0.5, len=1.0, done=True)
f.striker_fire(0)
print('  t', fl(f.t))
show_events(f)
for b in (b0, b1, b2):
    show_strike(b)

print('== step: a bite, step by step')


def bite_run(name, enemy, pre=None, mover=None, steps=45, cobra=(0.5, -0.25)):
    f = new_fight()
    bk = basket(f, 0, cobra, 10.0)
    O = f.balls[1]
    O.p = np.array(enemy, float)
    O.v = np.array([1.0, -2.4])
    f.t = 10.0 + beat
    if pre:
        pre(f, bk)
    f.striker_fire(0)
    t0 = f.t
    log = []
    for i in range(1, steps + 1):
        f.t = t0 + i * dt
        if mover:
            mover(f, i)
        n = len(f.events)
        was = None if bk['strike'] is None else bk['strike']['done']
        f.striker_step(0, dt)
        now = None if bk['strike'] is None else bk['strike']['done']
        if was != now or len(f.events) != n:
            log.append((i, was, now, len(f.events) - n))
    print(' ', name, 'log', log)
    show_events(f)
    B = f.balls[0]
    print('    hp', fl(O.hp), 'dealt', fl(B.dealt), 'v', [fl(x) for x in O.v], 'boost', fl(O.boost), 'bl', fl(B.blade_len),
          't_strike', fl(B.t_strike), 'cp', [fl(x) for x in bk['cp']])
    return f, bk


def away(f, i):
    if i == 5:
        f.balls[1].p = np.array([2.3, 2.3])


def shield(f, bk):
    f.balls[1].shield = 0.5


def side(f, i):
    if i == 5:
        f.balls[1].p = np.array([1.5, 1.14])


def side2(f, i):
    if i == 5:
        f.balls[1].p = np.array([1.5, 1.16])


def behind(f, i):
    if i == 5:
        f.balls[1].p = np.array([0.1, -0.7])


bite_run('a lands', (1.9, 0.6))
bite_run('b enemy gone: miss', (1.9, 0.6), mover=away)
bite_run('c shield', (1.9, 0.6), pre=shield)
bite_run('d haste', (1.9, 0.6), pre=haste)
bite_run('e enemy moved aside but still touching the fang line', (1.9, 0.6), mover=side)
bite_run('f moved aside, just out', (1.9, 0.6), mover=side2)
bite_run('g enemy behind the cobra: the fang line starts at the cobra', (1.9, 0.6), mover=behind)

print('== step: growth, 20 bites in a row')
f = new_fight()
bk = basket(f, 0, (0.5, -0.25), 10.0)
O = f.balls[1]
B = f.balls[0]
bl, ts = [], []
f.t = 20.0
for n in range(20):
    O.p = np.array([1.2, 0.1])
    bk['strike'] = dict(t0=f.t - 0.1, t_hit=f.t, t_end=f.t + 0.2, aim=0.4, len=1.0, done=False)
    f.striker_step(0, dt)
    bl.append(B.blade_len)
    ts.append(B.t_strike)
print('  bl', [fl(x) for x in bl])
print('  ts', [fl(x) for x in ts])
print('  hits', sum(e[1] == 'hit' for e in f.events), 'hp', fl(O.hp), 'dealt', fl(B.dealt))

print('== step: slither home and dance')
f = new_fight()
bk = basket(f, 0, (0.5, -0.25), 10.0, cp=(0.56, -0.2))
O = f.balls[1]
O.p = np.array([2.0, 2.0])
f.t = 12.0
cps = []
for i in range(1, 6):
    f.t = 12.0 + i * dt
    f.striker_step(0, dt)
    cps.append([fl(x) for x in bk['cp']])
print('  home', cps)
f = new_fight()
bk = basket(f, 0, (0.5, -0.25), 10.0)
O = f.balls[1]
O.p = np.array([1.9, 0.6])
f.balls[0].dance_until = 12.0 + 20.5 * dt
cps = []
for i in range(1, 31):
    f.t = 12.0 + i * dt
    if i == 24:
        O.p = np.array([-1.0, 1.0])
    f.striker_step(0, dt)
    cps.append([fl(x) for x in bk['cp']])
print('  dance', {i + 1: cps[i] for i in (0, 1, 17, 18, 19, 20, 21, 22, 23, 29)})
print('  dist to enemy after step 20', fl(np.hypot(*(np.array([1.9, 0.6]) - np.array([float(x) for x in cps[19]])))))
f = new_fight()
bk = basket(f, 0, (0.5, -0.25), 10.0, cp=(0.8, 0.1))
O = f.balls[1]
O.p = np.array([2.3, 2.3])
f.t = 12.0
bk['strike'] = dict(t0=11.95, t_hit=12.07, t_end=12.262, aim=0.5, len=1.0, done=False)
f.striker_step(0, dt)
print('  striking, not dancing: cp', [fl(x) for x in bk['cp']])
f.balls[0].dance_until = 13.0
f.striker_step(0, dt)
print('  striking, dancing: cp', [fl(x) for x in bk['cp']])

print('== swat')
th = 0.7
d = np.array([math.cos(th), math.sin(th)])
print('  d', [fl(x) for x in d])


def swat_case(name, a, L_, pre=None, t_off=0.0, cobra=(0.5, -0.25), names=('FLUTE', 'TROMBONE')):
    f = new_fight(names=names)
    kf = names.index('FLUTE')
    bk = basket(f, kf, cobra, 10.0)
    f.t = 10.0 + beat + t_off
    if pre:
        pre(f, bk)
    f.swat_cobras(1 - kf, np.array(a, float), d, L_)
    print(' ', name, 'a', [fl(x) for x in a], 'L', L_, 't', fl(f.t), 'down', fl(bk['down']) if 'down' in bk else None)
    show_events(f)
    show_strike(bk)
    return f, bk


side_n = np.array([-d[1], d[0]])


def base(back, off, cobra=(0.5, -0.25)):
    return (np.array(cobra) - d * back + side_n * off).tolist()


def already_down(f, bk):
    bk['down'] = f.t + 0.3


def down_over(f, bk):
    bk['down'] = f.t


def in_air(f, bk):
    bk['strike'] = dict(t0=f.t - 0.06, t_hit=f.t + 0.06, t_end=f.t + 0.252, aim=2.2, len=1.5, done=False)


def bitten(f, bk):
    bk['strike'] = dict(t0=f.t - 0.15, t_hit=f.t - 0.03, t_end=f.t + 0.162, aim=2.2, len=1.5, done=True)


def in_air_near(f, bk):
    bk['strike'] = dict(t0=f.t - 0.03, t_hit=f.t + 0.09, t_end=f.t + 0.282, aim=2.2, len=1.5, done=False)


def started_now(f, bk):
    bk['strike'] = dict(t0=f.t, t_hit=f.t, t_end=f.t + 0.2, aim=2.2, len=1.5, done=False)


swat_case('a body, lance passes 0.1 beside', base(1.0, 0.1), 1.5)
swat_case('b lance passes 0.2 beside: no touch', base(1.0, 0.2), 1.5)
swat_case('c lance too short by 0.25', base(1.0, 0.1), 0.75)
swat_case('c2 lance short by 0.15: its tip touches', base(1.0, 0.1), 0.85)
swat_case('d not risen', base(1.0, 0.1), 1.5, t_off=-2e-9)
swat_case('d2 risen within the slack', base(1.0, 0.1), 1.5, t_off=-5e-10)
swat_case('e already down', base(1.0, 0.1), 1.5, pre=already_down)
swat_case('e2 down just over', base(1.0, 0.1), 1.5, pre=down_over)
swat_case('f head in the air crosses the lance', base(1.0, 0.5), 2.5, pre=in_air)
swat_case('f2 the lance beyond the head: no touch', base(1.0, 1.2), 2.5, pre=in_air)
swat_case('g strike already done: only the body counts', base(1.0, 0.5), 2.5, pre=bitten)
swat_case('g2 done strike, lance on the body', base(1.0, 0.1), 1.5, pre=bitten)
swat_case('h head near the lance without crossing', base(1.0, 0.5), 2.5, pre=in_air_near)
swat_case('i strike with no flight time (t_hit == t0)', base(1.0, 0.5), 2.5, pre=started_now)
swat_case('j swapped sides: the lance is fighter 0', base(1.0, 0.1), 1.5, names=('TROMBONE', 'FLUTE'))
print(' two cobras, both touched; and no baskets')
f = new_fight()
b0 = basket(f, 0, (0.5, -0.25), 10.0)
b1 = basket(f, 0, (0.5 + d[0] * 0.4 + 0.05, -0.25 + d[1] * 0.4), 10.1)
b2 = basket(f, 0, (-2.0, 2.0), 10.1)
f.t = 10.1 + beat
aa = base(1.0, 0.1)
f.swat_cobras(1, np.array(aa), d, 1.5)
print('  a', [fl(x) for x in aa], 't', fl(f.t), 'b1 cp', [fl(x) for x in b1['cp']])
show_events(f)
print('  down', [fl(b['down']) if 'down' in b else None for b in (b0, b1, b2)])
f = new_fight()
f.t = 12.0
f.swat_cobras(1, np.array([0.0, 0.0]), d, 1.5)
f.swat_cobras(0, np.array([0.0, 0.0]), d, 1.5)
print('  no baskets: events', len(f.events))
