# What it prints: the weapon kind `combo` (TRUMPET's VALVE COMBO, fight.py 898 to 966) and the ability kind `beam`
# (its HIGH C: 2990 to 3016 and the ability beams of beams_step 857 to 897) on hand-built state, as repr() of every
# float: the valves' first aim, a combo queued on its beat (and under haste), three punches against an enemy
# standing in reach (the events, the hits, the reach growing), a shielded enemy (blocks, and the pistons pull
# straight back), the reach's cap; then HIGH C: the charge's aim, the blast when its beat comes, its one hit, and
# the enemy's projectiles it clears.
# The expected numbers in studio/crates/bf-sim/src/kinds/combo/tests.rs are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/combo_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the combo's and the ability beam's unit tests: hand-built state, the same calls, what
fight.py gave. Usage: combo_unit_vectors.py <repo root>"""
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


def fl(x):
    return repr(float(x))


def show_events(f, start=0):
    for e in f.events[start:]:
        print('   ', tuple(float(x) if isinstance(x, (float, np.floating)) else x for x in e))


def duel():
    f = F.Fight('TRUMPET', 'FLUTE', seed=392, rules=episode_rules(), record=False)
    f.balls[0].p = np.array([-1.0, 0.25])
    f.balls[1].p = np.array([0.5, 1.0])
    f.balls[0].v = np.array([0.0, 0.0])
    f.balls[1].v = np.array([0.0, 0.0])
    f.t = T
    f.combo_setup(0)
    return f


def run_combo(f, steps):
    """combo_step(0, dt) once per step at T + i * dt (step 0 is the step of the beat)"""
    for i in range(steps):
        f.t = T + i * R.DT
        n0 = len(f.events)
        f.combo_step(0, R.DT)
        if len(f.events) > n0:
            print('   step', i)
            show_events(f, n0)


R = episode_rules()
print('beat', fl(R.beat), 'dt', fl(R.DT), 'R', fl(R.R))
T = 12.5

print('== setup: the valves face the enemy')
f = duel()
B = f.balls[0]
print('   th', fl(B.th), 'reach', fl(B.blade_len), 'queue', B.punch_q, B.punch_i, B.punches, B.combo_n)
f = duel()
f.balls[1].p = np.array([-2.0, -1.0])
f.combo_setup(0)
print('   th (enemy behind and below)', fl(f.balls[0].th))

print('== beat: a combo every 2 beats; with haste every beat and half the gap')
f = duel()
for kb in range(4):
    n0 = len(f.events)
    f.combo_beat(0, kb)
    print('   kb', kb, 'queue', [fl(x) for x in f.balls[0].punch_q], 'combo_n', f.balls[0].combo_n)
    show_events(f, n0)
f = duel()
f.balls[0].haste, f.balls[0].haste_mult = 1.0, 2.0
for kb in range(3):
    n0 = len(f.events)
    f.combo_beat(0, kb)
    print('   hasted kb', kb, 'queue', [fl(x) for x in f.balls[0].punch_q], 'combo_n', f.balls[0].combo_n)
    show_events(f, n0)

print('== three punches at an enemy in reach')
f = duel()
f.balls[1].v = np.array([0.0, 2.6])
f.combo_beat(0, 0)
run_combo(f, 80)
B, O = f.balls
print('   th', fl(B.th), 'reach', fl(B.blade_len), 'live', len(B.punches), 'queue', B.punch_q, 'punch_i', B.punch_i)
print('   enemy hp', fl(O.hp), 'v', '[' + ', '.join(fl(x) for x in O.v) + ']', 'boost', fl(O.boost), 'dealt', fl(B.dealt))

print('== an enemy out of reach: the punches start and miss')
f = duel()
f.balls[1].p = np.array([2.5, 2.0])
f.combo_beat(0, 0)
run_combo(f, 80)
print('   reach', fl(f.balls[0].blade_len), 'th', fl(f.balls[0].th), 'enemy hp', fl(f.balls[1].hp))

print('== a shielded enemy blocks, and the piston pulls straight back')
f = duel()
f.balls[1].shield = 9.0
f.combo_beat(0, 0)
run_combo(f, 10)
P = f.balls[0].punches[0]
print('   first punch t0', fl(P['t0']), 'hit', P['hit'], 'reach', fl(f.balls[0].blade_len), 'hp', fl(f.balls[1].hp))

print('== the reach stops at max_len')
f = duel()
f.balls[0].blade_len = 1.68
f.combo_beat(0, 0)
run_combo(f, 80)
print('   reach', fl(f.balls[0].blade_len))

print('== haste: the punches run twice as fast')
f = duel()
f.balls[0].haste, f.balls[0].haste_mult = 5.0, 2.0
f.combo_beat(0, 0)
run_combo(f, 40)
print('   reach', fl(f.balls[0].blade_len), 'live', len(f.balls[0].punches), 'k', [fl(P['k']) for P in f.balls[0].punches])

print('== HIGH C: the charge')
f = duel()
f.balls[0].v = np.array([1.5, -0.5])
f.balls[1].v = np.array([-0.8, 2.4])
f.trigger(0, 'low')
ch = f.balls[0].ab_charge
print('   aim', fl(ch['aim']), 't_fire', fl(ch['t_fire']))
show_events(f)
print('== ... fires when its beat comes, hits once, and clears the enemy\'s projectiles in its path')
f = duel()                                                     # both standing still: the aim is the enemy itself
f.trigger(0, 'low')
f.spawn_key(1, np.array([-0.2, 0.65]), 0.0, 0.0, 1.0, 0, 0)    # an enemy projectile on the line
f.spawn_key(1, np.array([-1.0, -2.5]), 0.0, 0.0, 1.0, 0, 1)    # one far from it
f.spawn_key(0, np.array([0.1, 0.8]), 0.0, 0.0, 1.0, 1, 0)      # its own, on the line
for i in range(1, 120):
    f.t = T + i * R.DT
    n0 = len(f.events)
    f.beams_step(R.DT)
    f.abilities_step(False)
    if len(f.events) > n0:
        print('   step', i, 't', fl(f.t), 'beams', len(f.beams))
        show_events(f, n0)
    if f.balls[0].ab_charge is None and not f.beams:
        print('   beam gone at step', i)
        break
B, O = f.balls
print('   enemy hp', fl(O.hp), 'v', '[' + ', '.join(fl(x) for x in O.v) + ']', 'boost', fl(O.boost), 'reach', fl(B.blade_len),
      'keys left', [int(x) for x in f.kid])
print('== a beam at a shielded enemy')
f = duel()
f.balls[1].shield = 9.0
f.trigger(0, 'low')
for i in range(1, 120):
    f.t = T + i * R.DT
    n0 = len(f.events)
    f.beams_step(R.DT)
    f.abilities_step(False)
    if len(f.events) > n0:
        print('   step', i)
        show_events(f, n0)
print('   enemy hp', fl(f.balls[1].hp), 'boost', fl(f.balls[1].boost))
