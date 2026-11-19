# What it prints: the weapon kind `tiptoe` (BASSOON's TIPTOE, fight.py 3294 to 3428) and the abilities `mountain`
# (MOUNTAIN KING) and `hook` (BOCAL HOOK, 3437 to 3475) on hand-built state: `_behind` in its three cases, hops
# launched within reach, beyond it and against the ropes, landings that stomp from behind, from the front, from
# inside the enemy (stepped out) and that miss, MOUNTAIN KING's stomp, `tiptoe_tick` on and off its grid (the jitter
# drawn), while tumbling and for the follow-up hop, MOUNTAIN KING's first hops round the enemy and its end,
# `tiptoe_step` mid-hop and at the landing, and the hook's wait, opening, turning and landing, as float.hex() of
# every float. The expected numbers in studio/crates/bf-sim/src/kinds/tiptoe/kit_vectors.rs are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/tiptoe_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the tiptoe's and the hook's unit tests: hand-built state, the same calls, what fight.py gave.
Usage: tiptoe_unit_vectors.py <repo root>"""
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


def new(seed=9):
    f = F.Fight('BASSOON', 'FLUTE', seed=seed, rules=episode_rules(), record=False)
    f.tiptoe_setup(0)
    f.striker_setup(1)
    f.t = 12.5
    return f


def place(f, bp, op, ov):
    f.balls[0].p = np.array(bp)
    f.balls[1].p = np.array(op)
    f.balls[1].v = np.array(ov)


def show_ball(f, tag=''):
    B, O = f.balls
    print('   ', tag, 'p', vec(B.p), 'v', vec(B.v), 'air', B.air, 'blade_len', hx(B.blade_len), 'stomp_r',
          hx(B.stomp_r), 'O.p', vec(O.p), 'O.v', vec(O.v), 'O.hp', hx(O.hp), 'O.boost', hx(O.boost))


def show_hop(f):
    H = f.balls[0].hop
    if H is None:
        print('    hop None')
    else:
        print('    hop p0', vec(H['p0']), 'p1', vec(H['p1']), 't0', hx(H['t0']), 't1', hx(H['t1']), 'kind', H['kind'])


R = episode_rules()
W = F.FIGHTERS['BASSOON']['weapon']
AB = F.FIGHTERS['BASSOON']['abilities']
print('beat', hx(R.beat), 'dt', hx(R.DT), 'rules dmg_scale', hx(R.dmg_scale), 'dmg_side', R.dmg_side)

print('== A: _behind: against the enemy\'s travel, toward the hopper, or [1, 0]')
f = new()
place(f, [0.3, -1.1], [1.7, 0.45], [-1.3, 2.9])
print('    moving', vec(f._behind(0)))
f.balls[1].v = np.array([0.0, 0.0])
print('    still', vec(f._behind(0)))
f.balls[1].p = f.balls[0].p.copy()
print('    together', vec(f._behind(0)))

print('== B: _hop_launch within reach, beyond it (scaled), against the ropes (clipped)')
for tag, bp, tgt, reach in (('within', [0.3, -1.1], [1.25, 0.8], 3.0), ('beyond', [0.3, -1.1], [-2.2, 1.9], 1.6),
                            ('ropes', [2.0, 1.7], [3.6, 2.9], 3.0)):
    f = new()
    place(f, bp, [1.7, 0.45], [-1.3, 2.9])
    f._hop_launch(0, 2, 0, np.array(tgt), reach)
    print('   ', tag)
    show_hop(f)
    show_ball(f)
    show_events(f)

print('== C: _hop_land: a stomp from behind, from the front, from inside the enemy, a miss, a MOUNTAIN KING stomp')
for tag, p1, ov, kind in (('behind', [1.15, -0.2], [1.1, 1.3], 0), ('front', [1.15, -0.2], [-1.1, -1.3], 0),
                          ('inside', [1.55, 0.3], [-1.3, 2.9], 1), ('miss', [-1.8, -2.0], [-1.3, 2.9], 0),
                          ('mountain', [1.15, -0.2], [1.1, 1.3], 2)):
    f = new()
    place(f, [0.3, -1.1], [1.7, 0.45], ov)
    if kind == 2:
        f.start_mountain(0, AB['low'])
        f.balls[0].mk = None
    n = len(f.events)
    f.balls[0].hop = dict(p0=np.array([0.3, -1.1]), p1=np.array(p1), t0=12.4, t1=12.5, kind=kind)
    f.balls[0].air = True
    f._hop_land(0)
    print('   ', tag)
    show_ball(f)
    show_events(f, n)

print('== D: tiptoe_tick: on its grid (the jitter drawn), off it, tumbling, the follow-up hop')
f = new()
place(f, [0.3, -1.1], [1.7, 0.45], [-1.3, 2.9])
f.tiptoe_tick(0, 5)
print('    off the grid: hop', f.balls[0].hop, 'events', len(f.events))
f.tiptoe_tick(0, 8)
show_hop(f)
show_ball(f)
n = show_events(f)
print('    next draw', hx(f.rng.uniform()))
f = new()
place(f, [0.3, -1.1], [1.7, 0.45], [-1.3, 2.9])
f.balls[0].boost = 0.5
f.tiptoe_tick(0, 8)
print('    tumbling: hop', f.balls[0].hop, 'events', len(f.events))
f.balls[0].boost = 0.0
f.balls[0].hop_bonus = True
f.tiptoe_tick(0, 9)
show_hop(f)
show_ball(f)
show_events(f)
print('    hop_bonus', f.balls[0].hop_bonus, 'next draw', hx(f.rng.uniform()))

print('== E: MOUNTAIN KING: its first hops round the enemy, the wait between them, its end')
f = new()
place(f, [0.3, -1.1], [1.7, 0.45], [-1.3, 2.9])
f.start_mountain(0, AB['low'])
n = show_events(f)
for k16 in (40, 41, 43, 44):
    f.balls[0].hop, f.balls[0].air = None, False
    f.tiptoe_tick(0, k16)
    Mk = f.balls[0].mk
    print('    k16', k16, 'i', Mk['i'], 'next', Mk['next'], 'ang', hx(Mk['ang']))
    show_hop(f)
    n = show_events(f, n)
f.balls[0].hop, f.balls[0].air = None, False
f.balls[0].mk['i'] = len(AB['low']['seq16'])
f.tiptoe_tick(0, 80)
print('    ended: mk', f.balls[0].mk, 'hop', f.balls[0].hop)
show_events(f, n)

print('== F: tiptoe_step mid-hop, then at its landing')
f = new()
place(f, [0.3, -1.1], [1.7, 0.45], [-1.3, 2.9])
f._hop_launch(0, 2, 0, np.array([1.25, 0.8]), 3.0)
n = len(f.events)
H = f.balls[0].hop
f.t = 12.5 + 0.37 * (H['t1'] - H['t0'])
f.tiptoe_step(0, R.DT)
print('    mid p', vec(f.balls[0].p), 'events', len(f.events) - n)
f.t = H['t1'] - 5e-10
f.tiptoe_step(0, R.DT)
show_hop(f)
show_ball(f)
show_events(f, n)

print('== G: BOCAL HOOK: the wait, the opening, the turning, the hook and the follow-up')
f = new()
ab = AB['unlock2']
place(f, [0.3, -1.1], [2.2, 0.9], [-1.3, 2.9])
print('    ready far', f.hook_ready(0, ab))
f.balls[1].p = np.array([1.05, 0.15])
print('    ready near', f.hook_ready(0, ab))
f.balls[0].air = True
print('    ready in the air', f.hook_ready(0, ab))
f.balls[0].air = False
f.start_hook(0, ab)
Hk = f.balls[0].hook
print('    aim', hx(Hk['aim']), 't_hook', hx(Hk['t_hook']))
n = show_events(f)
for j in range(3):
    f.balls[1].p = f.balls[1].p + np.array([0.04, -0.03])
    f.t += R.DT
    f.hook_step(0, R.DT)
    print('    step', j, 'aim', hx(Hk['aim']), 'state', Hk['state'])
f.t = Hk['t_hook']
f.hook_step(0, R.DT)
print('    hooked: aim', hx(Hk['aim']), 'state', Hk['state'], 't_end', hx(Hk['t_end']), 'hop_bonus',
      f.balls[0].hop_bonus)
show_ball(f)
n = show_events(f, n)
f.t = Hk['t_end'] - 1e-6
f.hook_step(0, R.DT)
print('    before t_end: live', f.balls[0].hook is not None)
f.t = Hk['t_end']
f.hook_step(0, R.DT)
print('    at t_end: live', f.balls[0].hook is not None, 'events', len(f.events) - n)

print('== H: BOCAL HOOK misses: out of its angle')
f = new()
place(f, [0.3, -1.1], [1.05, 0.15], [-1.3, 2.9])
f.start_hook(0, ab)
Hk = f.balls[0].hook
n = len(f.events)
f.balls[1].p = np.array([-0.6, -0.4])
f.t = Hk['t_hook']
f.hook_step(0, R.DT)
print('    aim', hx(Hk['aim']), 'hop_bonus', f.balls[0].hop_bonus, 'O.hp', hx(f.balls[1].hp))
show_events(f, n)
