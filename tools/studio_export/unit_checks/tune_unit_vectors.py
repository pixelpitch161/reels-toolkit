# What it prints: the weapon kind `tune` (OBOE's TUNING A, fight.py 2425 to 2525), the ability kind `endless`
# (ENDLESS NOTE, 2884 to 2888) and a wave's `detune` option (ILL WIND, 2845 to 2846 and 3069 to 3071) on hand-built
# state, as repr() of every float: `tune_setup`; the 16th grid of `tune_tick` (the lock, the shot, the stings, a
# stunned oboe); `tune_step` (the aim's turn, a beam landing with its knock, detune and depth growth, the cap, a
# shielded enemy, a beam through the enemy's centre, a beam going out); the detuned attacker in `damage`; the held
# beam (its bite on the 8ths, its end, its own turn rate); the wave that detunes; and what Python does with the
# roster numbers the Studio refuses when the roster is read.
# The expected numbers in studio/crates/bf-sim/src/kinds/tune/tests.rs and in
# studio/crates/bf-core/src/roster/kinds/tune.rs (its tests) are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/tune_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the tune weapon's unit tests: hand-built state, the same calls, what fight.py gave.
Usage: tune_unit_vectors.py <repo root>"""
import copy, json, sys, zipfile
import numpy as np
clone = sys.argv[1]
sys.path.insert(0, clone + '/projects/ballfight')
import fight as F


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


def duel(left='OBOE', right='FLUTE'):
    """OBOE against FLUTE with the balls where the tests put them, the oboe set up"""
    f = F.Fight(left, right, seed=392, rules=rules(), record=False)
    k = 0 if left == 'OBOE' else 1
    f.balls[k].p = np.array([-1.0, 0.5])
    f.balls[1 - k].p = np.array([1.2, -0.7])
    f.balls[k].v = np.array([1.0, -2.0])
    f.balls[1 - k].v = np.array([-0.5, 2.0])
    f.t = T
    f.tune_setup(k)
    return f, k


def state(f, k):
    B = f.balls[k]
    print('   th', fl(B.th), 'depth', fl(B.depth), 'lock', None if B.tune_lock is None else fl(B.tune_lock),
          'beams', [(vec(b['o']), vec(b['d']), fl(b['t1']), b['hit']) for b in B.tbeams],
          'endless', None if B.endless is None else fl(B.endless['until']))


def victim(f, k):
    O = f.balls[1 - k]
    print('   victim hp', fl(O.hp), 'v', vec(O.v), 'boost', fl(O.boost), 'detune t/d/w', fl(O.detune_t), fl(O.detune_d),
          fl(O.detune_w), 'dealt', fl(f.balls[k].dealt))


R = rules()
T = 12.5
S16 = R.beat / 4.0
print('beat', fl(R.beat), 'dt', fl(R.DT), 's16', fl(S16))

for left, right in (('OBOE', 'FLUTE'), ('FLUTE', 'OBOE')):
    print('== tune_setup:', left, 'left')
    f, k = duel(left, right)
    state(f, k)
    print('   detune', [(fl(B.detune_t), fl(B.detune_d), fl(B.detune_w)) for B in f.balls])

print('== tune_tick: 40 16ths from T (the lock 3 16ths before each shot, the shot 3 16ths after the bar line)')
f, k = duel()
for k16 in range(40):
    f.t = T + k16 * S16
    n0 = len(f.events)
    f.tune_tick(k, k16)
    if len(f.events) > n0:
        print('  k16', k16, 't', fl(f.t))
        show_events(f, n0)
        state(f, k)

print('== tune_tick: the stings on the 8ths while the enemy is out of tune')
f, k = duel()
f.balls[1].detune_t, f.balls[1].detune_d, f.balls[1].detune_w = 1.0, 0.5, 0.6
for k16 in (4, 5, 6):
    f.t = T + k16 * S16
    n0 = len(f.events)
    f.tune_tick(k, k16)
    print('  k16', k16)
    show_events(f, n0)
victim(f, k)
f.balls[1].detune_t = 0.0
n0 = len(f.events)
f.tune_tick(k, 8)
print('  in tune again: events', len(f.events) - n0)

print('== tune_tick: a stunned oboe drops its lock and does nothing')
f, k = duel()
f.balls[0].tune_lock = 1.0
f.balls[0].stun = 0.5
f.tune_tick(k, 3)
print('   events', len(f.events))
state(f, k)

print('== tune_step: the aim turns to the enemy at `track`, and holds a lock')
f, k = duel()
f.balls[0].th = 3.0
for i in range(3):
    f.tune_step(k, R.DT)
    print('   th', fl(f.balls[0].th))
f.balls[0].tune_lock = 5.5
f.tune_step(k, R.DT)
print('   locked th', fl(f.balls[0].th))

print('== tune_step: a beam lands: hit, knock, detune, the depth grows')
f, k = duel()
f.tune_tick(k, 3)                                    # the shot (no lock: straight at the enemy)
show_events(f)
state(f, k)
f.balls[1].p = np.array([0.6, -0.3])                 # the enemy steps into the beam
f.tune_step(k, R.DT)
show_events(f, 1)
state(f, k)
victim(f, k)
f.tune_step(k, R.DT)                                 # a beam hits once
print('   events after a second step', len(f.events))
f.t = T + 0.18                                       # the beam goes out `hold` s after the shot
f.tune_step(k, R.DT)
state(f, k)

print('== tune_step: the detuned attacker hits weaker (damage 344 to 346)')
landed = f.damage(1, 0, 5.0, 'fang', 0.25, -0.5, 2)
print('   landed', landed, 'hp', fl(f.balls[0].hp), 'dealt', fl(f.balls[1].dealt))
show_events(f, len(f.events) - 1)

print('== tune_step: the depth stops at depth_max')
f, k = duel()
f.balls[0].depth = 0.95
f.tune_tick(k, 3)
f.balls[1].p = np.array([0.6, -0.3])
f.tune_step(k, R.DT)
show_events(f, 1)
state(f, k)

print('== tune_step: a shielded enemy: block, knocked all the same, no detune, no growth')
f, k = duel()
f.tune_tick(k, 3)
f.balls[1].p = np.array([0.6, -0.3])
f.balls[1].shield = 1.0
f.tune_step(k, R.DT)
show_events(f, 1)
state(f, k)
victim(f, k)

print('== tune_step: a beam through the enemy\'s centre')
f, k = duel()
f.balls[0].p = np.array([0.0, 0.0])
f.balls[1].p = np.array([2.0, 0.0])
f.balls[0].tune_lock = 0.0
f.tune_tick(k, 3)
f.tune_step(k, R.DT)
show_events(f)
victim(f, k)

print('== endless: trigger, the bite on the 8ths, its turn rate, its end')
f, k = duel()
f.balls[0].tune_lock = 1.0
f.tune_tick(k, 3)
f.events.clear()
f.trigger(k, 'unlock2')
show_events(f)
state(f, k)
print('   used', sorted(f.balls[0].used))
f.balls[0].th = 3.0
for i in range(2):
    f.tune_step(k, R.DT)
    print('   th', fl(f.balls[0].th))
f.balls[0].th = float(np.arctan2(-1.2, 2.2)) % (2 * np.pi)      # the held beam on the enemy
for k16 in (1, 2, 3, 4):
    f.t = T + k16 * S16
    n0 = len(f.events)
    f.tune_tick(k, k16)
    print('  k16', k16)
    show_events(f, n0)
victim(f, k)
f.balls[0].th = 1.0                                             # off the enemy: no bite
n0 = len(f.events)
f.tune_tick(k, 6)
print('   off the enemy: events', len(f.events) - n0)
f.t = T + 4 * R.beat
n0 = len(f.events)
f.tune_tick(k, 16)
show_events(f, n0)
state(f, k)

print('== ILL WIND: the wave detunes the enemy it lands on')
f, k = duel()
f.trigger(k, 'low')
for i in range(80):
    f.t = T + (i + 1) * R.DT
    n0 = len(f.events)
    f.effects_step(R.DT)
    if len(f.events) > n0:
        print('   step', i + 1, 't', fl(f.t))
        show_events(f, n0)
        break
victim(f, k)

print('== what Python does with roster numbers the Studio refuses')
base = copy.deepcopy(F.FIGHTERS['OBOE'])
for what, edit in (('every = 0.1 (a grid of 0 16ths)', lambda w: w.update(every=0.1)),
                   ('sting16 = 0', lambda w: w.update(sting16=0)),
                   ('no hold', lambda w: w.pop('hold')),
                   ('no depth0', lambda w: w.pop('depth0')),
                   ('no len', lambda w: w.pop('len')),
                   ('no sting', lambda w: w.pop('sting'))):
    cfg = copy.deepcopy(base)
    edit(cfg['weapon'])
    F.FIGHTERS['OBOE'] = cfg
    try:
        f = F.Fight('OBOE', 'FLUTE', seed=392, rules=rules(), record=False)
        f.run()
        print('  ', what, '-> ran to', fl(f.t))
    except Exception as e:
        print('  ', what, '->', type(e).__name__, e)
F.FIGHTERS['OBOE'] = base
for what, edit in (('ILL WIND without depth', lambda a: a['detune'].pop('depth')),
                   ('ILL WIND with an empty detune', lambda a: a.update(detune={}))):
    cfg = copy.deepcopy(base)
    edit(cfg['abilities']['low'])
    F.FIGHTERS['OBOE'] = cfg
    try:
        f, k = duel()
        f.trigger(k, 'low')
        for i in range(80):
            f.t = T + (i + 1) * R.DT
            f.effects_step(R.DT)
        print('  ', what, '-> events', [e[1] for e in f.events])
    except Exception as e:
        print('  ', what, '->', type(e).__name__, e)
F.FIGHTERS['OBOE'] = base
