# What it prints: the weapon kind `spotlight` (ALTO SAX's SPOTLIGHT SOLO, fight.py 4369 to 4512) and the ability
# kinds `octave`, `altissimo` and `encore` (2958 to 2965) on hand-built state, as the bits of every float:
# `spot_setup`; `spot_tick` (a sweep on the grid with its jitter draw, a burn and its knock, a burn at the light's
# centre, a miss, a shielded enemy, no sweep for a stunned or smashed fighter or off the grid, a haste's grid);
# `spot_step` (a sweep's end, the growth of the light and the burn to their caps, a sweep that missed); ALTISSIMO's
# narrow hot light and its end; OCTAVE KEY's second light there and back; ENCORE's chase light, its phone lights on
# the 8ths, its end; and the fighter in the right corner.
# The output is studio/crates/bf-sim/src/kinds/spotlight/vectors.txt, which the kind's unit tests
# (studio/crates/bf-sim/src/kinds/spotlight/tests.rs) read: they make the same calls and compare every line.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/spotlight_unit_vectors.py . \
#       > studio/crates/bf-sim/src/kinds/spotlight/vectors.txt
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
#
# A line is `<label> | <state>` (compared bit for bit) or `<label> ~ <vx> <vy> <boost>` (the enemy's heading and
# boost, compared to 1e-12: a knock goes through np.linalg.norm, whose fused sum the engine's `Fight::knock` does
# not have). The state: the fighter's light radius (`bl`), burn, `oct_until`, `alt_until`, ENCORE's end; every light
# of `Fight.spots` (id, owner, kind, hot, passes, grow, hit, t0, t1, r, dmg, where it is now, its line's ends);
# the phone lights; the enemy's HP and the damage dealt; the events since the line before.
"""Python vectors for the spotlight weapon's unit tests: hand-built state, the same calls, what fight.py gave.
Usage: spotlight_unit_vectors.py <repo root>"""
import json
import struct
import sys
import zipfile

import numpy as np

sys.stdout.reconfigure(newline=chr(10))             # LF on every platform: the tests read the file
clone = sys.argv[1]
sys.path.insert(0, clone + '/projects/ballfight')
import fight as F  # noqa: E402


def rules():
    """the engine's default rules at the example episode's tempo and start (no handicap)"""
    meta = json.loads(zipfile.ZipFile(clone + '/studio/reference/fights/flute_vs_trombone/ep392.bfrec').read('meta.json'))
    r = meta['rules']
    return F.Rules(bpm=r['bpm'], t_fight=r['t_fight'])


def bits(x):
    return '%016x' % struct.unpack('<Q', struct.pack('<d', float(x)))[0]


def opt(x):
    return 'none' if x is None else bits(x)


def tok(v):
    if isinstance(v, (bool, np.bool_, int, np.integer)):
        return 'i%d' % int(v)
    if isinstance(v, (float, np.floating)):
        return 'f' + bits(v)
    return 's' + str(v)


R = rules()
T = 12.5
S16 = R.beat / 4.0
DT = R.DT
ZERO = np.zeros(2)


class Case:
    """ALTO SAX against FLUTE (or the other way round), the balls where the tests put them, the alto set up"""

    def __init__(self, name, alto_left=True):
        print('== ' + name)
        left, right = ('ALTO SAX', 'FLUTE') if alto_left else ('FLUTE', 'ALTO SAX')
        f = F.Fight(left, right, seed=392, rules=rules(), record=False)
        k = 0 if alto_left else 1
        f.balls[k].p = np.array([-1.0, 0.5])
        f.balls[1 - k].p = np.array([1.2, -0.7])
        f.balls[k].v = np.array([1.0, -2.0])
        f.balls[1 - k].v = np.array([-0.5, 2.0])
        f.t = T
        f.spot_setup(k)
        self.f, self.k, self.n = f, k, len(f.events)

    def step(self, label):
        f, k = self.f, self.k
        B, O = f.balls[k], f.balls[1 - k]
        En = getattr(B, 'encore', None)
        parts = ['bl=' + bits(B.blade_len), 'burn=' + bits(B.burn), 'oct=' + opt(getattr(B, 'oct_until', None)),
                 'alt=' + opt(getattr(B, 'alt_until', None)), 'enc=' + opt(None if En is None else En['t1'])]
        spots = []
        for Sp in f.spots:
            c = f.spot_pos(Sp, f.t)
            a, b = Sp.get('a', ZERO), Sp.get('b', ZERO)
            spots.append(':'.join([str(Sp['id']), str(Sp['owner']), str(Sp['kind']), str(Sp['hot']), str(Sp['passes']),
                                   str(int(Sp['grow'])), str(int(Sp['hit'])), bits(Sp['t0']), bits(Sp['t1']),
                                   bits(Sp['r']), bits(Sp['dmg']), bits(c[0]), bits(c[1]), bits(a[0]), bits(a[1]),
                                   bits(b[0]), bits(b[1])]))
        parts.append('spots=[' + ';'.join(spots) + ']')
        parts.append('phones=[' + ';'.join(':'.join([bits(t), bits(q[0]), bits(q[1])]) for t, q in f.phones) + ']')
        parts.append('hp=' + bits(O.hp))
        parts.append('dealt=' + bits(B.dealt))
        ev = ['%s:%s:%s' % (bits(e[0]), e[1], ','.join(tok(v) for v in e[2:])) for e in f.events[self.n:]]
        self.n = len(f.events)
        parts.append('events=[' + ';'.join(ev) + ']')
        print(label + ' | ' + ' '.join(parts))
        print(label + ' ~ ' + ' '.join(repr(float(x)) for x in (O.v[0], O.v[1], O.boost)))


# ---------------------------------------------------------------- spot_setup
for name, left in (('setup', True), ('setup, the right corner', False)):
    c = Case(name, left)
    c.step('set up')

# ---------------------------------------------------------------- a sweep: its line, its burns, its end
c = Case('sweep')
f, k = c.f, c.k
O = f.balls[1 - k]
f.spot_tick(k, 0)
c.step('a sweep on the grid')
f.t = T + S16
f.spot_tick(k, 1)
c.step('the next 16th')
f.t = T + 3 * S16
O.p = f.spot_pos(f.spots[0], f.t) + np.array([0.1, 0.05])
f.spot_tick(k, 3)
c.step('a burn')
f.t = T + 4 * S16
O.v = np.array([-0.5, 2.0])
O.boost = 0.0
O.p = f.spot_pos(f.spots[0], f.t).copy()
f.spot_tick(k, 4)
c.step('a burn at the centre')
f.t = T + 5 * S16
O.p = np.array([3.0, 3.0])
f.spot_tick(k, 5)
c.step('a miss')
f.t = T + 6 * S16
O.shield = 1.0
O.p = f.spot_pos(f.spots[0], f.t) + np.array([0.1, 0.05])
f.spot_tick(k, 6)
c.step('a shielded enemy')
f.spot_step(k, DT)
c.step('a step while it runs')
f.t = f.spots[0]['t1'] + DT
f.spot_step(k, DT)
c.step('its end')

# ---------------------------------------------------------------- growth to the caps
c = Case('growth')
f, k = c.f, c.k
for j in range(14):
    f.t = T + j * 2.0
    f.spot_tick(k, 0)
    f.spots[-1]['hit'] = True
    f.t = f.t + 1.0
    f.spot_step(k, DT)
    c.step('sweep %d landed' % j)
f.t = T + 40.0
f.spot_tick(k, 0)
f.t = f.t + 1.0
f.spot_step(k, DT)
c.step('a sweep that missed')

# ---------------------------------------------------------------- when no sweep starts, and a haste's grid
c = Case('no sweep')
f, k = c.f, c.k
B = f.balls[k]
f.spot_tick(k, 8)
c.step('off the grid')
B.stun = 1.0
f.spot_tick(k, 0)
c.step('stunned')
B.stun = 0.0
B.smashed = 1.0
f.spot_tick(k, 0)
c.step('smashed')
B.smashed = 0.0
B.haste, B.haste_mult = 1.0, 2.0
f.spot_tick(k, 8)
c.step('a haste halves the grid')
f.spot_tick(k, 4)
c.step('and no further')

# ---------------------------------------------------------------- ALTISSIMO
c = Case('altissimo')
f, k = c.f, c.k
O = f.balls[1 - k]
f.trigger(k, 'unlock2')
c.step('fired')
f.spot_tick(k, 0)
c.step('a hot sweep')
f.t = T + 3 * S16
O.p = f.spot_pos(f.spots[0], f.t) + np.array([0.05, 0.02])
f.spot_tick(k, 3)
c.step('a hot burn')
O.v = np.array([-0.5, 2.0])
O.boost = 0.0
f.t = f.balls[k].alt_until
f.spot_step(k, DT)
f.spot_tick(k, 0)
c.step('a sweep when it has run out')

# ---------------------------------------------------------------- OCTAVE KEY
c = Case('octave')
f, k = c.f, c.k
f.trigger(k, 'unlock1')
c.step('fired')
f.spot_tick(k, 0)
c.step('two lights')
for j in range(1, 7):
    f.t = T + j * S16
    c.step('16th %d' % j)
f.t = f.spots[0]['t1'] + DT
f.spots[1]['hit'] = True
f.spot_step(k, DT)
c.step('their end')
f.t = f.balls[k].oct_until
f.spot_tick(k, 0)
c.step('a sweep when it has run out')

# ---------------------------------------------------------------- ENCORE
for name, left in (('encore', True), ('encore, the right corner', False)):
    c = Case(name, left)
    f, k = c.f, c.k
    O = f.balls[1 - k]
    f.spot_tick(k, 0)
    c.step('a sweep')
    f.trigger(k, 'low')
    c.step('fired')
    for j in range(5):
        f.t = f.t + DT
        f.spot_step(k, DT)
        c.step('chase step %d' % j)
    f.spot_tick(k, 1)
    c.step('an odd 16th')
    f.spot_tick(k, 2)
    c.step('an 8th: the phones')
    O.p = f.spots[0]['c'] + np.array([0.05, 0.0])
    f.spot_tick(k, 3)
    c.step('the chase light burns')
    O.v = np.array([-0.5, 2.0])
    O.boost = 0.0
    f.spot_tick(k, 4)
    c.step('the phones again')
    O.v = np.array([-0.5, 2.0])
    O.boost = 0.0
    f.spot_tick(k, 16)
    c.step('no sweep on the grid')
    f.t = f.t + DT
    f.spot_step(k, DT)
    c.step('the light closes in')
    f.t = f.balls[k].encore['t1']
    f.spot_step(k, DT)
    c.step('its end')
    f.t = f.t + DT
    f.spot_step(k, DT)
    c.step('the step after')
    O.v = np.array([-0.5, 2.0])
    O.boost = 0.0
    f.spot_tick(k, 0)
    c.step('sweeps again')
