# What it prints: the five abilities FLUTE and TROMBONE use (haste, dance, wave, shockwave with its slow, slam)
# on hand-built state: trigger, abilities_step and effects_step, each case with the values fight.py gives, as
# repr() of every float.
# The expected numbers in studio/crates/bf-sim/src/abilities/tests.rs are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/abilities_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the abilities' unit tests: hand-built state, the same calls, what fight.py gave.
Usage: abilities_unit_vectors.py <repo root>"""
import json, sys, zipfile
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


def new_fight(names=('FLUTE', 'TROMBONE'), **over):
    """the hand-built fight of the Rust tests: seed 392, the episode's rules, the striker set up"""
    f = F.Fight(names[0], names[1], seed=392, rules=episode_rules(**over), record=False)
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


def show_wave(W):
    print('    wave owner', W['owner'], 'c', vec(W['c']), 'r', fl(W['r']), 'speed', fl(W['speed']), 'rmax',
          fl(W['rmax']),
          'dmg', fl(W['dmg']), 'knock', fl(W['knock']), 'kind', W['kind'], 'hit', W['hit'], 'name', W['name'],
          't0', fl(W['t0']), 'slow', W['slow'])


def show_slam(S):
    print('    slam owner', S['owner'], 'c', vec(S['c']), 't0', fl(S['t0']), 't_slam', fl(S['t_slam']), 'radius',
          fl(S['radius']), 'dmg', fl(S['dmg']), 'knock', fl(S['knock']), 'done', S['done'], 'name', S['name'])


def show_ball(f, k, what='ball'):
    B = f.balls[k]
    print('   ', what, k, 'p', vec(B.p), 'v', vec(B.v), 'boost', fl(B.boost), 'hp', fl(B.hp), 'dealt', fl(B.dealt),
          'slow', fl(B.slow), 'slow_mult', fl(B.slow_mult), 'shield', fl(B.shield))


R = episode_rules()
dt, beat = R.DT, R.beat
T = 12.5
print('beat', fl(beat), 'dt', fl(dt))

print('== ab_dur: dur_beats at the episode tempo, and dur when the entry has one')
f = new_fight()
for name in ('FLUTE', 'TROMBONE'):
    for slot, ab in F.FIGHTERS[name]['abilities'].items():
        if 'dur' in ab or 'dur_beats' in ab:
            print('  ', name, slot, ab['name'], fl(f.ab_dur(ab)))
print('   dur 1.25 beside dur_beats 8:', fl(f.ab_dur(dict(dur=1.25, dur_beats=8))))
f.t = T
print('   SNAKE DANCE fired at', fl(f.t), 'lasts until', fl(f.t + f.ab_dur(F.FIGHTERS['FLUTE']['abilities']['unlock2'])))

print('== trigger: the ability event, used, and what each kind sets')
f = new_fight()
f.t = T
f.balls[0].p, f.balls[0].v = np.array([-1.0, 0.5]), np.array([1.0, 2.4])
f.balls[1].p, f.balls[1].v = np.array([1.5, -0.25]), np.array([-2.4, 1.0])
for k, slot in ((0, 'unlock1'), (0, 'unlock1'), (0, 'unlock2'), (0, 'low'), (1, 'unlock1'), (1, 'unlock2'),
                (1, 'low'), (1, 'low')):
    n0 = len(f.events)
    f.t += dt
    f.trigger(k, slot)
    B = f.balls[k]
    print('  trigger', k, slot, 'at', fl(f.t), 'used', sorted(B.used))
    show_events(f, n0)
    print('    haste', fl(B.haste), 'haste_mult', fl(B.haste_mult), 'dance_until', fl(getattr(B, 'dance_until', -9.0)),
          'waves', len(f.waves), 'slams', len(f.slams))
for W in f.waves:
    show_wave(W)
for S in f.slams:
    show_slam(S)
# the wave's centre is a copy: the owner moves on, the centre stays
f.balls[0].p += 1.0
print('    after the owner moved: c', vec(f.waves[0]['c']))

print('== trigger, slam: the target is where the enemy is heading, held inside the ropes')
for p, v in (((0.2, -0.4), (1.5, -2.1)), ((2.2, -2.3), (2.6, 0.0)), ((-2.3, 2.39), (-1.0, 2.4)),
             ((0.0, 0.0), (0.0, 0.0))):
    f = new_fight()
    f.t = T
    f.balls[0].p, f.balls[0].v = np.array(p, float), np.array(v, float)
    f.trigger(1, 'low')
    print('  enemy p', p, 'v', v)
    show_slam(f.slams[0])

print('== abilities_step: the unlock loop')
f = new_fight()
B = f.balls[0]


def step(t, on_beat, note):
    n0 = len(f.events)
    f.t = t
    f.abilities_step(on_beat)
    print('  t', fl(t), 'on_beat', on_beat, note, '-> level', B.level, 'ready',
          {s: fl(x) for s, x in sorted(B.ready.items())}, 'low_done', B.low_done, 'haste', fl(B.haste),
          'dance_until', fl(B.dance_until), 'waves', len(f.waves))
    show_events(f, n0)


t0 = 10.0
B.dealt = 40.0
step(t0, False, 'both levels at once, between beats')
step(t0 + dt, False, 'between beats')
step(t0 + 2 * dt, True, 'a beat: the first unlock')
r1 = B.ready['unlock1']
step(t0 + 3 * dt, True, 'the next beat: the second unlock')
r2 = B.ready['unlock2']
step(t0 + 4 * dt, True, 'a beat with nothing ready')
step(r1 - 2e-6, True, 'a beat 2e-6 s before unlock1 is ready')
step(r1 - 5e-7, True, 'a beat 5e-7 s before unlock1 is ready')
B.hp = 24.9
step(r2 + 0.5, False, 'low HP between beats, unlock2 ready')
step(r2 + 0.5 + dt, True, 'low HP on a beat, unlock2 ready: the low ability takes the turn')
step(r2 + 0.5 + 2 * dt, True, 'the next beat: unlock2')
B.hp = 10.0
step(r2 + 0.5 + 3 * dt, True, 'still low: nothing more')

print('== abilities_step: an unlock fires on a beat exactly 1e-6 s before its time (t >= ready - 1e-6)')
f = new_fight()
B = f.balls[0]
B.dealt = 14.0
step(10.0, True, 'level 2 on a beat: the unlock at once')
r1 = B.ready['unlock1']
step(float(np.nextafter(r1 - 1e-6, 0.0)), True, 'a float before ready - 1e-6')
step(r1 - 1e-6, True, 'exactly ready - 1e-6')

print('== abilities_step: fighter 0 before fighter 1 in the same step')
f = new_fight()
f.balls[0].dealt = 14.0
f.balls[1].dealt = 14.0
f.t = 11.0
f.abilities_step(True)
show_events(f)

print('== effects_step: PICCOLO (wave), the FLUTE at (-1.0, 0.5), the enemy 2.5 away')


def run_wave(f, steps, show_r=(), moves=None):
    """effects_step once per step at T + i * dt; prints each event with the step it came in"""
    for i in range(1, steps + 1):
        f.t = T + i * dt
        if moves and i in moves:
            moves[i](f)
        n0, w0 = len(f.events), len(f.waves)
        f.effects_step(dt)
        if i in show_r:
            print('    step', i, 'r', [fl(W['r']) for W in f.waves])
        if len(f.waves) != w0:
            print('    step', i, 'waves', w0, '->', len(f.waves))
        if len(f.events) > n0:
            print('    step', i)
            show_events(f, n0)
            for k in (0, 1):
                show_ball(f, k)


f = new_fight()
f.t = T
f.balls[0].p, f.balls[0].v = np.array([-1.0, 0.5]), np.array([1.0, 2.4])
f.balls[1].p, f.balls[1].v = np.array([1.5, 0.5]), np.array([-2.4, 1.0])
f.trigger(0, 'low')
run_wave(f, 170, show_r=(1, 2, 3, 31, 32, 33, 34, 100, 166, 167, 168))

print('== effects_step: SAD TROMBONE (shockwave with its slow), the TROMBONE at (1.0, -0.5), the enemy at (-0.8, 0.3)')
f = new_fight()
f.t = T
f.balls[1].p, f.balls[1].v = np.array([1.0, -0.5]), np.array([-2.4, 1.0])
f.balls[0].p, f.balls[0].v = np.array([-0.8, 0.3]), np.array([1.0, 2.4])
f.balls[0].boost = 0.25
f.trigger(1, 'unlock1')
run_wave(f, 60, show_r=(1, 21, 22, 23, 24, 55, 56, 57))

print('== effects_step: a shockwave on a shielded enemy: block, the knock, no slow')
f = new_fight()
f.t = T
f.balls[1].p, f.balls[1].v = np.array([1.0, -0.5]), np.array([-2.4, 1.0])
f.balls[0].p, f.balls[0].v = np.array([-0.8, 0.3]), np.array([1.0, 2.4])
f.balls[0].shield = 5.0
f.trigger(1, 'unlock1')
run_wave(f, 60)
print('    waves left', len(f.waves))

print('== effects_step: a shockwave that never reaches the enemy (4.5 away), and one whose enemy starts inside it')
f = new_fight()
f.t = T
f.balls[1].p = np.array([2.0, -0.5])
f.balls[0].p, f.balls[0].v = np.array([-2.0, 1.5617376914898995]), np.array([1.0, 2.4])
print('    the enemy is', fl(np.hypot(*(f.balls[0].p - f.balls[1].p))), 'away')
f.trigger(1, 'unlock1')
run_wave(f, 60)
print('    events', len(f.events) - 1, 'waves left', len(f.waves))
f = new_fight()
f.t = T
f.balls[1].p = np.array([1.0, -0.5])
f.balls[0].p, f.balls[0].v = np.array([0.2, -0.5]), np.array([1.0, 2.4])
f.trigger(1, 'unlock1')
run_wave(f, 60)
print('    inside from the start: events', len(f.events) - 1, 'waves left', len(f.waves))
f = new_fight()
f.t = T
f.balls[1].p = np.array([1.0, -0.5])
f.balls[0].p, f.balls[0].v = np.array([0.2, -0.5]), np.array([1.0, 2.4])
f.trigger(1, 'unlock1')


def away(f):
    f.balls[0].p = np.array([-1.0, -0.5])


run_wave(f, 60, moves={10: away})
print('    inside, then out past the front at step 10: events', len(f.events) - 1)

print('== effects_step: the edges of the hit test (the front passes dist - R / 2 in this step)')
for x in (2.2, 2.2 + 1e-12, 2.25, 2.2 - 1e-12, 2.15):
    f = new_fight()
    f.t = T
    f.balls[1].p = np.array([0.0, 0.0])
    f.balls[0].p = np.array([-(x + 0.3), 0.0])
    f.trigger(1, 'unlock1')
    hit = None
    for i in range(1, 60):
        f.t = T + i * dt
        r0 = f.waves[0]['r'] if f.waves else None
        n0 = len(f.events)
        f.effects_step(dt)
        if len(f.events) > n0:
            hit = (i, fl(r0), fl(f.waves[0]['r']))
    print('    front target', fl(x), 'hit at (step, r0, r)', hit)

print('== effects_step: the hit test on its exact edges (floats found by trying the neighbours of the decimal)')


def edge(what, want):
    # an enemy x away (on the x axis) whose `dist - R / 2` equals `want` exactly; the step it is hit in, if any
    for n in range(-4, 5):
        x = want + 0.3
        for _ in range(abs(n)):
            x = float(np.nextafter(x, np.inf if n > 0 else -np.inf))
        if float(np.hypot(x, 0.0)) - R.R * 0.5 == want:
            break
    else:
        print('   ', what, 'no float gives', fl(want))
        return
    f = new_fight()
    f.t = T
    f.balls[1].p = np.array([0.0, 0.0])
    f.balls[0].p, f.balls[0].v = np.array([-x, 0.0]), np.array([1.0, 2.4])
    f.trigger(1, 'unlock1')
    hit = None
    for i in range(1, 6):
        f.t = T + i * dt
        n0 = len(f.events)
        f.effects_step(dt)
        if len(f.events) > n0:
            hit = i
    print('   ', what, 'enemy at x =', fl(-x), 'dist - R / 2 =', fl(float(np.hypot(x, 0.0)) - R.R * 0.5),
          'hit at step', hit)


edge('exactly on the starting front (r0 < reach fails for ever)', 0.6)
edge('one float past the starting front', float(np.nextafter(0.6, 1.0)))
edge('exactly r + 1e-9 after step 1 (reach <= r + 1e-9 holds)', 0.65 + 1e-9)
edge('one float past r + 1e-9 after step 1', float(np.nextafter(0.65 + 1e-9, 1.0)))

print('== effects_step: an enemy that turns up exactly on the front (dist - R / 2 == r0) is not hit')
for nudge in (0, 1):
    f = new_fight()
    f.t = T
    f.balls[1].p = np.array([0.0, 0.0])
    f.balls[0].p, f.balls[0].v = np.array([-2.0, 2.5]), np.array([1.0, 2.4])
    f.trigger(1, 'unlock1')
    for i in range(1, 11):
        f.t = T + i * dt
        f.effects_step(dt)
    r0 = f.waves[0]['r']
    x = None
    for n in range(-4, 5):                                # a float x with x - R / 2 == r0 exactly
        c = r0 + 0.3
        for _ in range(abs(n)):
            c = float(np.nextafter(c, np.inf if n > 0 else -np.inf))
        if c - R.R * 0.5 == r0:
            x = c
            break
    if nudge:
        x = float(np.nextafter(x, 9.0))
    f.balls[0].p = np.array([x, 0.0])
    hit = None
    for i in range(11, 16):
        f.t = T + i * dt
        n0 = len(f.events)
        f.effects_step(dt)
        if len(f.events) > n0:
            hit = i
    print('    after 10 steps r0', fl(r0), 'the enemy at x =', fl(x), 'dist - R / 2 =', fl(x - R.R * 0.5),
          'hit at step', hit)

print('== effects_step: two waves in flight, the list keeps its order')
f = new_fight()
f.t = T
f.balls[0].p, f.balls[0].v = np.array([-0.9, 0.2]), np.array([1.0, 2.4])
f.balls[1].p, f.balls[1].v = np.array([0.9, -0.2]), np.array([-2.4, 1.0])
f.trigger(1, 'unlock1')
f.trigger(0, 'low')
run_wave(f, 60, show_r=(1, 56, 57))
print('    waves left', [(W['owner'], W['kind'], fl(W['r'])) for W in f.waves])

print('== effects_step: a wave clears the enemy projectiles its front passes')
f = new_fight()
f.t = T
f.balls[1].p = np.array([1.0, -0.5])
f.balls[0].p, f.balls[0].v = np.array([-2.0, 2.0]), np.array([1.0, 2.4])
f.trigger(1, 'unlock1')
# after 10 steps the front is at 1.1; in step 11 it goes from 1.1 to 1.15: the band is (0.95, 1.2]
for own, d in ((0, 1.0), (1, 1.0), (0, 0.95), (0, 0.9501), (0, 1.2), (0, 1.2001), (0, 0.5), (0, 3.0)):
    f.spawn_key(own, np.array([1.0 - d, -0.5]), 0.0, 0.0, 2.0, 0, 0)
for i in range(1, 12):
    f.t = T + i * dt
    n0 = len(f.events)
    f.effects_step(dt)
    if len(f.events) > n0:
        print('    step', i, 'r', fl(f.waves[0]['r']))
        show_events(f, n0)
        print('    left', [(int(o), int(i_), fl(p[0])) for o, i_, p in zip(f.kown, f.kid, f.kp)])

print('== effects_step: a projectile exactly on the inner edge of the band (r0 - 0.15) is not cleared')
f = new_fight()
f.t = T
f.balls[1].p = np.array([0.0, 0.0])
f.balls[0].p, f.balls[0].v = np.array([-2.0, 2.5]), np.array([1.0, 2.4])
f.trigger(1, 'unlock1')
for i in range(1, 11):
    f.t = T + i * dt
    f.effects_step(dt)
r0 = f.waves[0]['r']
inner = r0 - 0.15
f.spawn_key(0, np.array([inner, 0.0]), 0.0, 0.0, 2.0, 0, 0)
f.spawn_key(0, np.array([float(np.nextafter(inner, 9.0)), 0.0]), 0.0, 0.0, 2.0, 0, 1)
f.spawn_key(0, np.array([0.0, -inner]), 0.0, 0.0, 2.0, 0, 2)
print('    after 10 steps r', fl(r0), 'keys at', [vec(p) for p in f.kp], 'their distances',
      [fl(x) for x in np.hypot(*(f.kp - f.waves[0]['c'][None]).T)])
for i in range(11, 16):
    f.t = T + i * dt
    n0 = len(f.events)
    f.effects_step(dt)
    if len(f.events) > n0:
        print('    step', i, 'r', fl(f.waves[0]['r']))
        show_events(f, n0)
print('    left', [(int(o), int(i_), vec(p)) for o, i_, p in zip(f.kown, f.kid, f.kp)])

print('== effects_step: PEDAL TONE (slam): called, lands 2 beats later where the enemy was heading')


def run_slam(p, v, at_landing=None, boost=0.0, shield=0.0, steps=112):
    f = new_fight()
    f.t = T
    f.balls[0].p, f.balls[0].v = np.array(p, float), np.array(v, float)
    f.balls[0].boost = boost
    f.balls[0].shield = shield
    f.trigger(1, 'low')
    S = f.slams[0]
    for i in range(1, steps + 1):
        f.t = T + i * dt
        if at_landing is not None and f.t >= S['t_slam'] and not S['done']:
            f.balls[0].p = np.array(at_landing, float)
        n0 = len(f.events)
        f.effects_step(dt)
        if len(f.events) > n0:
            print('    step', i, 't', fl(f.t), 't_slam', fl(S['t_slam']))
            show_events(f, n0)
            show_ball(f, 0, 'enemy')
            show_ball(f, 1, 'owner')
    print('    slams', len(f.slams), 'done', S['done'], 'events', [e[1] for e in f.events])
    return f


c = np.array([0.2, -0.4]) + np.array([1.5, -2.1]) * (2 * beat) * 0.6
print('  c', vec(c))
print('  the enemy 0.5 from the centre:')
run_slam((0.2, -0.4), (1.5, -2.1), at_landing=c + np.array([0.3, 0.4]), boost=0.1)
print('  the enemy 1.70 from the centre (inside radius + 0.6 R = 1.71):')
run_slam((0.2, -0.4), (1.5, -2.1), at_landing=c + np.array([0.0, 1.70]))
print('  the enemy 1.72 from the centre: a miss')
run_slam((0.2, -0.4), (1.5, -2.1), at_landing=c + np.array([0.0, 1.72]))
print('  the enemy on the centre (5e-4 away): knocked along its own heading')
run_slam((0.2, -0.4), (1.5, -2.1), at_landing=c + np.array([5e-4, 0.0]))
print('  the enemy 2e-3 from the centre: knocked away from it')
run_slam((0.2, -0.4), (1.5, -2.1), at_landing=c + np.array([2e-3, 0.0]))
print('  a shielded enemy: block, and the knock')
run_slam((0.2, -0.4), (1.5, -2.1), at_landing=c + np.array([0.3, 0.4]), shield=5.0)

print('== effects_step: the slam on its exact edges: the time it lands, and its reach')
f = new_fight()
f.t = T
f.balls[0].p, f.balls[0].v = np.array([0.2, -0.4]), np.array([1.5, -2.1])
f.trigger(1, 'low')
S = f.slams[0]
for t in (float(np.nextafter(S['t_slam'], 0.0)), S['t_slam']):
    f.t = t
    n0 = len(f.events)
    f.effects_step(dt)
    print('    t', fl(t), 't_slam', fl(S['t_slam']), 'done', S['done'], [e[1] for e in f.events[n0:]])
reach = F.FIGHTERS['TROMBONE']['abilities']['low']['radius'] + R.R * 0.6
print('    radius + R * 0.6 =', fl(reach))
for x in (reach, float(np.nextafter(reach, 0.0))):
    f = new_fight()
    f.t = T
    f.balls[0].p, f.balls[0].v = np.array([0.0, 0.0]), np.array([0.0, 0.0])
    f.trigger(1, 'low')
    S = f.slams[0]
    f.t = S['t_slam']
    f.balls[0].p = np.array([x, 0.0])
    f.effects_step(dt)
    print('    the enemy', fl(x), 'from the centre', vec(S['c']),
          '(dist', fl(np.hypot(*(f.balls[0].p - S['c']))) + '):')
    show_events(f, 1)

print('== the fight loop: effects_step runs before abilities_step, so a ring fired in a step first spreads in the next')
# Fight.run() itself, recording every step (FPS = 120), for 4 steps; the TROMBONE starts one hit from level 2, so
# its shockwave fires in the first step (the fight's first beat)
f = F.Fight('FLUTE', 'TROMBONE', seed=392, rules=episode_rules(FPS=120, t_max=R.t_fight + 3.5 * dt), record=True)
f.balls[1].dealt = 14.0
r = f.run()
for i, row in enumerate(r['WAVES']):
    print('    frame', i, [(w[0], fl(w[3]), fl(w[4]), w[5], fl(w[6])) for w in row],
          'haste', vec(r['HA'][i]), 'level', list(r['LV'][i]))
show_events(f)

print('== effects_step: a ring that hits and a slam that lands in the same step: the ring first')
f = new_fight()
f.t = T
f.balls[1].p = np.array([0.0, 0.0])
f.balls[0].p, f.balls[0].v = np.array([2.28, 0.0]), np.array([0.0, 0.0])
f.trigger(1, 'low')
show_slam(f.slams[0])
for i in range(1, 111):
    f.t = T + i * dt
    n0 = len(f.events)
    f.effects_step(dt)
    if len(f.events) > n0:
        print('    step', i)
        show_events(f, n0)
        show_ball(f, 0, 'enemy')
    if i == 80:
        f.trigger(1, 'unlock1')

print('== the fight loop: the weapons step before the effects (a lance hit, then a slam, in one step)')
f = F.Fight('FLUTE', 'TROMBONE', seed=392, rules=episode_rules(FPS=120, t_max=R.t_fight + 0.5 * dt), record=True)
f.balls[1].p, f.balls[1].v = np.array([0.5, 0.0]), np.array([0.0, 2.6])
f.balls[0].p, f.balls[0].v = np.array([-0.8, 0.0]), np.array([0.0, -2.6])
f.slams.append(dict(owner=1, c=np.array([-0.8, 0.25]), t0=R.t_fight - 0.5, t_slam=R.t_fight, radius=1.35, dmg=24.0,
                    knock=1.6, done=False, name='PEDAL TONE'))
r = f.run()
show_events(f)
for k in (0, 1):
    show_ball(f, k)
print('    frames', len(r['HP']), 'the lance: ext', fl(f.balls[1].ext), 'th', fl(f.balls[1].th), 'reach',
      fl(f.balls[1].blade_len), 'frets', f.balls[1].frets, 'thr_hit', f.balls[1].thr_hit)

print('== the timers: haste and slow run down in the fight loop, a step at a time')
f = new_fight()
B = f.balls[0]
f.t = T
f.trigger(0, 'unlock1')
h = [B.haste]
for i in range(431):
    B.haste = max(0.0, B.haste - dt)
    h.append(B.haste)
print('    haste after 1, 2, 429, 430, 431 steps', [fl(h[i]) for i in (1, 2, 429, 430, 431)])

print('== a kind with no branch in trigger (here: none), and a slow that is an empty table')
plain = dict(F.FIGHTERS['FLUTE'])
added = {
    'NOBODY': dict(plain, abilities=dict(unlock1=dict(name='A', kind='none', every_bars=2),
                                         unlock2=dict(name='B', kind='none', every_bars=3),
                                         low=dict(name='C', kind='none'))),
    'EMPTY SLOW': dict(plain, abilities=dict(F.FIGHTERS['FLUTE']['abilities'],
                                             low=dict(F.FIGHTERS['FLUTE']['abilities']['low'], slow={}))),
    'FULL SLOW': dict(plain, abilities=dict(F.FIGHTERS['FLUTE']['abilities'],
                                            low=dict(F.FIGHTERS['FLUTE']['abilities']['low'],
                                                     slow=dict(mult=0.5, dur=0.75)))),
    # the TROMBONE with a shockwave whose final radius is off the grid of its steps (0.6 + n * 0.05)
    'OFF GRID': dict(F.FIGHTERS['TROMBONE'],
                     abilities=dict(F.FIGHTERS['TROMBONE']['abilities'],
                                    unlock1=dict(F.FIGHTERS['TROMBONE']['abilities']['unlock1'], radius=3.42))),
}
F.FIGHTERS.update(added)
try:
    f = new_fight(('NOBODY', 'FLUTE'))
    B = f.balls[0]
    B.dealt, B.hp = 40.0, 10.0
    before = (B.haste, B.haste_mult, B.shield, B.slow, len(f.waves), len(f.slams))
    for i, on_beat in enumerate((True, True, True, True)):
        f.t = 10.0 + i * dt
        f.abilities_step(on_beat)
    show_events(f)
    print('    ready', {s_: fl(x) for s_, x in sorted(B.ready.items())}, 'used', sorted(B.used), 'unchanged',
          before == (B.haste, B.haste_mult, B.shield, B.slow, len(f.waves), len(f.slams)))
    for pair in (('FLUTE', 'FLUTE'), ('EMPTY SLOW', 'FLUTE'), ('FLUTE', 'EMPTY SLOW'), ('FULL SLOW', 'FLUTE'),
                 ('FLUTE', 'TROMBONE')):
        print('    uses_ext', pair, new_fight(pair).uses_ext)
    for name in ('EMPTY SLOW', 'FULL SLOW'):
        f = new_fight((name, 'FLUTE'))
        f.t = T
        f.balls[0].p, f.balls[0].v = np.array([-1.0, 0.5]), np.array([1.0, 2.4])
        f.balls[1].p, f.balls[1].v = np.array([1.5, 0.5]), np.array([-2.4, 1.0])
        f.trigger(0, 'low')
        print('   ', name, 'wave slow', f.waves[0]['slow'])
        run_wave(f, 40)
    print('== a ring whose final radius (3.42) is off the grid of its steps: the last step stops at the radius')
    for reach in (3.41, 3.44):
        f = new_fight(('FLUTE', 'OFF GRID'))
        f.t = T
        f.balls[1].p = np.array([0.0, 0.0])
        f.balls[0].p, f.balls[0].v = np.array([-(reach + 0.3), 0.0]), np.array([1.0, 2.4])
        f.trigger(1, 'unlock1')
        print('    the enemy with dist - R / 2 =', fl(float(np.hypot(*f.balls[0].p)) - R.R * 0.5))
        for i in range(1, 60):
            f.t = T + i * dt
            n0, w0 = len(f.events), len(f.waves)
            r0 = f.waves[0]['r'] if f.waves else None
            f.effects_step(dt)
            if len(f.events) > n0:
                print('      hit at step', i, 'r0', fl(r0), 'r', fl(f.waves[0]['r']) if f.waves else 'gone',
                      [e[1] for e in f.events[n0:]])
            if len(f.waves) != w0:
                print('      step', i, 'r0', fl(r0), 'waves', w0, '->', len(f.waves))
            elif i in (55, 56):
                print('      step', i, 'r', fl(f.waves[0]['r']))
        print('      events', [e[1] for e in f.events])
finally:
    for name in added:
        del F.FIGHTERS[name]
