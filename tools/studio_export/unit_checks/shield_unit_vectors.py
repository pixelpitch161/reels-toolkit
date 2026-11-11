# What it prints: the ability kind `shield` (TUBA's, HORN's and ORGAN's, fight.py 2847 to 2848) on hand-built
# state: what `trigger` sets for each of the three, a shield fired while one still holds, and a hit on a shielded
# ball (`damage` 335 to 337: a `block` event with the victim, the attacker, x, y and what hit, nothing taken off),
# as repr() of every float.
# The expected numbers in studio/crates/bf-sim/src/abilities/shield.rs (its tests) are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/shield_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the shield's unit tests: hand-built state, the same calls, what fight.py gave.
Usage: shield_unit_vectors.py <repo root>"""
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


R = episode_rules()
print('beat', fl(R.beat))
T = 12.5

print('== trigger: each shield sets B.shield = ab_dur(ab)')
for name, slot in (('TUBA', 'unlock2'), ('HORN', 'unlock1'), ('ORGAN', 'unlock1')):
    for k in (0, 1):
        pair = (name, 'FLUTE') if k == 0 else ('FLUTE', name)
        f = F.Fight(pair[0], pair[1], seed=392, rules=episode_rules(), record=False)
        f.t = T
        f.trigger(k, slot)
        ab = F.FIGHTERS[name]['abilities'][slot]
        print('  ', name, 'side', k, slot, ab['name'], 'dur_beats', ab.get('dur_beats'), 'shield', fl(f.balls[k].shield),
              'other', fl(f.balls[1 - k].shield), 'used', sorted(f.balls[k].used))
        show_events(f)

print('== a shield fired while one holds starts the time again')
f = F.Fight('TUBA', 'FLUTE', seed=392, rules=episode_rules(), record=False)
f.t = T
f.balls[0].shield = 5.25
f.trigger(0, 'unlock2')
print('   shield', fl(f.balls[0].shield))

print('== a hit on a shielded ball is blocked: block event, no damage, no dealt, damage() is False')
f = F.Fight('FLUTE', 'TUBA', seed=392, rules=episode_rules(), record=False)
f.t = T
f.balls[1].shield = 0.75
hp, dealt = f.balls[1].hp, f.balls[0].dealt
landed = f.damage(0, 1, 3.5, 'fang', 0.25, -0.5, 2)
print('   landed', landed, 'hp', fl(f.balls[1].hp), 'was', fl(hp), 'dealt', fl(f.balls[0].dealt), 'was', fl(dealt))
show_events(f)
# and once it has run out the same hit lands (with the episode's handicap on the attacker)
f.balls[1].shield = 0.0
landed = f.damage(0, 1, 3.5, 'fang', 0.25, -0.5, 2)
print('   landed', landed, 'hp', fl(f.balls[1].hp), 'dealt', fl(f.balls[0].dealt))
show_events(f, 1)

print('== a ring that passes a shielded enemy: blocked, still knocked, no slow')
f = F.Fight('TROMBONE', 'TUBA', seed=392, rules=episode_rules(), record=False)
f.t = T
f.balls[0].p = np.array([0.0, 0.0])
f.balls[1].p = np.array([1.5, 0.5])
f.balls[1].v = np.array([0.0, 2.6])
f.balls[1].shield = 1.0
f.trigger(0, 'unlock1')                                  # SAD TROMBONE, with its slow
for i in range(40):
    f.t = T + (i + 1) * R.DT
    n0 = len(f.events)
    f.effects_step(R.DT)
    if len(f.events) > n0:
        print('   step', i + 1, 't', fl(f.t))
        show_events(f, n0)
        break
B = f.balls[1]
print('   v', '[' + ', '.join(fl(x) for x in B.v) + ']', 'boost', fl(B.boost), 'slow', fl(B.slow), 'hp', fl(B.hp))
