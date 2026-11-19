# What it prints: the weapon kind `anchor` (BASS CLARINET's ENDPIN ANCHOR; fight.py 2568 to 2658) and the ability
# kinds `twin` (MULTIPHONICS; `trigger` 2897 to 2899) and `lights_out` (LIGHTS OUT; 2767 to 2800) on hand-built
# state, as repr() of every float: pegs planted from several places (plain, by the ropes, on the enemy, from the
# right corner, with no orbit), the 16th grid of `anchor_tick` (haste, stun, shadow, a whirl in progress), whole
# whirls step by step (a smash, the orbit's growth and cap, the twin's arm, an enemy in shadow, a whirl copied onto
# another fighter's ball), LIGHTS OUT (a whirl pulled first, the steering, the eruption hitting, missing and on top
# of the enemy, a copied one), and what the kit does with roster numbers the Studio refuses.
# The expected numbers in studio/crates/bf-sim/src/kinds/anchor/tests.rs, abilities/lights_out/tests.rs and
# abilities/twin.rs are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/anchor_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the anchor's, the twin's and LIGHTS OUT's unit tests: hand-built state, the same calls, what
fight.py gave. Usage: anchor_unit_vectors.py <repo root>"""
import copy, json, sys, zipfile
import numpy as np
clone = sys.argv[1]
sys.path.insert(0, clone + '/projects/ballfight')
import fight as F

BCL = 'BASS CLARINET'
T = 12.5


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


def anc(f, k):
    An = f.balls[k].anc
    if An is None:
        return 'None'
    return (f"c {vec(An['c'])} th {fl(An['th'])} r0 {fl(An['r0'])} orbit {fl(An['orbit'])} om {fl(An['om'])} "
            f"t0 {fl(An['t0'])} t_end {fl(An['t_end'])} cool {vec(An['cool'])}")


def duel(left, right, k, p, v, op, ov):
    """ball k at p moving v, its enemy at op moving ov, the clock at T; anchor_setup for an anchorer"""
    f = F.Fight(left, right, seed=392, rules=rules(), record=False)
    for j, B in enumerate(f.balls):
        B.anc = None
        if B.w['kind'] == 'anchor':
            f.anchor_setup(j)
    f.balls[k].p, f.balls[k].v = np.array(p, float), np.array(v, float)
    f.balls[1 - k].p, f.balls[1 - k].v = np.array(op, float), np.array(ov, float)
    f.t = T
    return f


def whirl(f, k, show=(1, 2), drift=(0.0, 0.0), until=None):
    """anchor_step once per step from the clock until the peg is pulled; the enemy drifts by `drift` a step"""
    R = f.R
    t0 = f.t
    for i in range(1, 400):
        f.t = t0 + i * R.DT
        f.balls[1 - k].p = f.balls[1 - k].p + np.array(drift)
        n0 = len(f.events)
        f.anchor_step(k, R.DT)
        B = f.balls[k]
        if i in show or len(f.events) > n0:
            print('   step', i, 't', fl(f.t), 'p', vec(B.p), 'v', vec(B.v), 'blade', fl(B.blade_len))
            print('      ', anc(f, k))
            show_events(f, n0)
        if B.anc is None or (until is not None and i >= until):
            print('   ended at step', i, 't', fl(f.t), 'boost', fl(B.boost))
            break
    O = f.balls[1 - k]
    print('   enemy hp', fl(O.hp), 'v', vec(O.v), 'boost', fl(O.boost), 'dealt', fl(f.balls[k].dealt),
          'blade', fl(f.balls[k].blade_len))


R = rules()
print('beat', fl(R.beat), 'dt', fl(R.DT), 'A', fl(R.A), 'R', fl(R.R), 'V', fl(R.V))

print('== anchor_plant')
CASES = [('plain', BCL, 'FLUTE', 0, (-1.0, 0.5), (1.0, -2.0), (0.5, -0.1), (-0.5, 2.0), None),
         ('by the ropes', BCL, 'FLUTE', 0, (2.3, 2.2), (1.0, 2.0), (2.6, 2.6), (0.3, 0.2), 1.8),
         ('on the enemy', BCL, 'FLUTE', 0, (0.4, -0.3), (1.5, -0.5), (0.4, -0.3), (0.0, 0.0), None),
         ('from the right', 'FLUTE', BCL, 1, (1.0, 0.5), (-1.0, 2.0), (-0.5, 0.1), (0.5, -2.0), None),
         ('no orbit', BCL, 'FLUTE', 0, (-1.0, 0.5), (1.0, -2.0), (0.5, -0.1), (-0.5, 2.0), 0.0)]
for name, left, right, k, p, v, op, ov, orbit in CASES:
    f = duel(left, right, k, p, v, op, ov)
    B = f.balls[k]
    f.anchor_plant(k, B.w, B.blade_len if orbit is None else orbit)
    print('  ', name, '|', anc(f, k))
    show_events(f)

print('== anchor_tick: the 16ths a peg goes in (the whirl cleared after each)')


def grid(setup=None, n=41):
    f = duel(BCL, 'FLUTE', 0, (-1.0, 0.5), (1.0, -2.0), (0.5, -0.1), (-0.5, 2.0))
    out = []
    for k16 in range(n):
        if setup:
            setup(f, k16)
        f.anchor_tick(0, k16)
        if f.balls[0].anc is not None:
            out.append(k16)
            if not (setup and getattr(setup, 'keep', False)):
                f.balls[0].anc = None
    return out


def hasted(f, k16):
    f.balls[0].haste, f.balls[0].haste_mult = (1.0, 2.0) if 10 <= k16 < 30 else (0.0, 1.0)


def stunned(f, k16):
    f.balls[0].stun = 0.3 if k16 == 16 else 0.0
    f.balls[0].shadow = 0.2 if k16 == 32 else 0.0


def keep(f, k16):
    pass


keep.keep = True
print('   plain', grid())
print('   haste x 2 on 16ths 10 to 29', grid(hasted))
print('   stunned on 16, in shadow on 32', grid(stunned))
print('   the whirl left in place', grid(keep))

print('== a whirl: the enemy stands on the circle and is smashed')
f = duel(BCL, 'FLUTE', 0, (-1.0, 0.5), (1.0, -2.0), (0.5, -0.1), (-0.5, 2.0))
f.anchor_plant(0, f.balls[0].w, f.balls[0].blade_len)
print('  ', anc(f, 0))
f.balls[1].p = f.balls[0].anc['c'] + np.array([0.0, 0.95])
f.balls[1].v = np.array([0.0, 0.0])
whirl(f, 0, show=(1, 2, 20))

print('== the same with a twin: the ghost arm lands first (sub 1, x twin_dmg), each arm rests on its own')
f = duel(BCL, 'FLUTE', 0, (-1.0, 0.5), (1.0, -2.0), (0.5, -0.1), (-0.5, 2.0))
f.trigger(0, 'unlock2')
show_events(f)
print('   twin_until', fl(f.balls[0].twin_until))
f.anchor_plant(0, f.balls[0].w, f.balls[0].blade_len)
An = f.balls[0].anc
f.balls[1].p = An['c'] - 0.95 * np.array([np.cos(An['th'] + 0.3), np.sin(An['th'] + 0.3)])
f.balls[1].v = np.array([0.0, 0.0])
f.balls[0].blade_len = 1.78
print('   enemy at', vec(f.balls[1].p))
whirl(f, 0, show=(1,))

print('== an enemy sunk in shadow is not smashed; a shielded one blocks (no growth, the arm still rests)')
for what in ('shadow', 'shield'):
    f = duel(BCL, 'FLUTE', 0, (-1.0, 0.5), (1.0, -2.0), (0.5, -0.1), (-0.5, 2.0))
    f.anchor_plant(0, f.balls[0].w, f.balls[0].blade_len)
    f.balls[1].p = f.balls[0].anc['c'] + np.array([0.0, 0.95])
    f.balls[1].v = np.array([0.0, 0.0])
    setattr(f.balls[1], what, 9.0)
    print('  ', what)
    whirl(f, 0, show=())

print('== a copied whirl: BASS CLARINET\'s weapon planted on FLUTE\'s ball (FLUTE left, BASS CLARINET right)')
f = duel('FLUTE', BCL, 0, (-1.0, 0.5), (1.0, -2.0), (0.5, -0.1), (-0.5, 2.0))
f.balls[1].blade_len = 1.25
f.anchor_plant(0, f.balls[1].w, f.balls[1].blade_len)
print('  ', anc(f, 0))
show_events(f)
f.balls[1].p = f.balls[0].anc['c'] + np.array([0.0, 1.25])
f.balls[1].v = np.array([0.0, 0.0])
b0 = f.balls[0].blade_len
whirl(f, 0, show=(1,))
print('   FLUTE\'s blade_len before', fl(b0), 'BASS CLARINET\'s', fl(f.balls[1].blade_len))

print('== anchor_release mid-whirl, both spin directions')
for left, right, k in ((BCL, 'FLUTE', 0), ('FLUTE', BCL, 1)):
    f = duel(left, right, k, (-1.0, 0.5), (1.0, -2.0), (0.5, -0.1), (-0.5, 2.0))
    f.balls[1 - k].p = np.array([2.4, 2.4])
    f.anchor_plant(k, f.balls[k].w, f.balls[k].blade_len)
    f.balls[k].boost = 0.3 if k == 0 else 1.1
    whirl(f, k, show=(), until=10)
    n0 = len(f.events)
    f.anchor_release(k)
    B = f.balls[k]
    print('   k', k, 'p', vec(B.p), 'v', vec(B.v), 'boost', fl(B.boost), 'anc', B.anc)
    show_events(f, n0)


def shadow_run(f, k, show=(1, 2, 3), drift=(0.0, 0.0)):
    Rr = f.R
    t0 = f.t
    for i in range(1, 400):
        f.t = t0 + i * Rr.DT
        f.balls[1 - k].p = f.balls[1 - k].p + np.array(drift)
        n0 = len(f.events)
        f.lights_out_step(k, Rr.DT)
        B = f.balls[k]
        if i in show or len(f.events) > n0:
            print('   step', i, 't', fl(f.t), 'v', vec(B.v), 'boost', fl(B.boost), 'shadow', fl(B.shadow))
            show_events(f, n0)
        if B.lo is None:
            print('   ended at step', i)
            break
    O = f.balls[1 - k]
    print('   enemy hp', fl(O.hp), 'v', vec(O.v), 'boost', fl(O.boost), 'dealt', fl(f.balls[k].dealt))


print('== LIGHTS OUT from a whirl: the peg is pulled, it sinks, homes and erupts on the enemy')
f = duel(BCL, 'FLUTE', 0, (-1.0, 0.5), (1.0, -2.0), (0.5, -0.1), (-0.5, 2.0))
f.balls[1].p = np.array([2.4, 2.4])
f.anchor_plant(0, f.balls[0].w, f.balls[0].blade_len)
whirl(f, 0, show=(), until=10)
f.balls[1].p = np.array([0.5, -0.1])
n0 = len(f.events)
f.trigger(0, 'low')
show_events(f, n0)
B = f.balls[0]
print('   p', vec(B.p), 'v', vec(B.v), 'boost', fl(B.boost), 'shadow', fl(B.shadow), 'anc', B.anc,
      'lo t0', fl(B.lo['t0']), 't_end', fl(B.lo['t_end']), 'stunned', f._stunned(0))
print('   a hit on the shadow lands:', f.damage(1, 0, 5.0, 'strike', 0.0, 0.0), 'hp', fl(B.hp))
shadow_run(f, 0)

print('== the eruption misses a far enemy; on top of the enemy it knocks along the enemy\'s heading')
for name, op, ov in (('far', (2.0, 2.0), (0.3, -0.4)), ('on top', (-1.0, 0.5), (0.3, -0.4))):
    f = duel(BCL, 'FLUTE', 0, (-1.0, 0.5), (0.0, 0.0), op, ov)
    f.trigger(0, 'low')
    print('  ', name)
    f.t = f.balls[0].lo['t_end']
    n0 = len(f.events)
    f.lights_out_step(0, f.R.DT)
    show_events(f, n0)
    O = f.balls[1]
    print('   shadow', fl(f.balls[0].shadow), 'lo', f.balls[0].lo, 'enemy hp', fl(O.hp), 'v', vec(O.v), 'boost',
          fl(O.boost))

print('== a copied LIGHTS OUT on FLUTE\'s ball (FLUTE against TROMBONE), from the right corner')
lo = F.FIGHTERS[BCL]['abilities']['low']
f = duel('TROMBONE', 'FLUTE', 1, (1.0, 0.5), (-1.0, 2.0), (-0.5, 0.1), (0.5, -2.0))
f.start_lights_out(1, lo)
show_events(f)
shadow_run(f, 1, drift=(0.004, 0.002))

print('== what the kit does with numbers the Studio refuses')
for key, value in (('whirl_beats', 0), ('settle', 0), ('every', 1e308)):
    saved = F.FIGHTERS[BCL]
    F.FIGHTERS[BCL] = copy.deepcopy(saved)
    F.FIGHTERS[BCL]['weapon'][key] = value
    try:
        f = duel(BCL, 'FLUTE', 0, (-1.0, 0.5), (1.0, -2.0), (0.5, -0.1), (-0.5, 2.0))
        f.anchor_tick(0, 16)
        f.t += f.R.DT
        f.anchor_step(0, f.R.DT)
        print('  ', key, value, 'ran')
    except Exception as ex:                                    # noqa: BLE001
        print('  ', key, value, '->', type(ex).__name__, ex)
    F.FIGHTERS[BCL] = saved
for key in ('every', 'whirl_beats', 'turns', 'base', 'grow', 'max_orbit', 'dmg', 'knock'):
    saved = F.FIGHTERS[BCL]
    F.FIGHTERS[BCL] = copy.deepcopy(saved)
    del F.FIGHTERS[BCL]['weapon'][key]
    try:
        f = duel(BCL, 'FLUTE', 0, (-1.0, 0.5), (1.0, -2.0), (0.5, -0.1), (-0.5, 2.0))
        f.anchor_tick(0, 16)
        f.balls[1].p = f.balls[0].anc['c'] + np.array([0.0, 0.95])
        f.balls[0].blade_len = 0.95
        for i in range(60):
            f.t += f.R.DT
            if f.balls[0].anc is not None:
                f.anchor_step(0, f.R.DT)
        print('   without', key, 'ran')
    except Exception as ex:                                    # noqa: BLE001
        print('   without', key, '->', type(ex).__name__, ex)
    F.FIGHTERS[BCL] = saved
