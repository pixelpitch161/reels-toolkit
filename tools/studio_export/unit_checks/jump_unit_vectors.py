# What it prints: the weapon kind `jump` (CLARINET's TWELFTH JUMP, fight.py 2338 to 2424), the ability kinds
# `rhapsody` (2930 to 2932, its chain in `jump_tick`) and `stun` (SQUEAK: 2889 to 2892, the hit of `effects_step` 3072
# to 3074, `_stunned` 2355 to 2357, `move` 3135) on hand-built state, as Rust rows with repr() of every float:
#   - `_jump` on its own: each branch (a cut, a miss, the reach cap, the ropes, never inside the enemy, a target on
#     the ball, the enemy on the line, a jump of no length, a shielded enemy, the growth and its cap, a chain jump);
#   - `jump_tick` over the 16ths: the lock, the grid, haste, a stunned or shadowed jumper, the RHAPSODY chain, and
#     the fight generator's next draw after each run (one draw a regular jump, none otherwise);
#   - SQUEAK: the ring, its hit and the stun it leaves, a longer stun kept, a shielded enemy, a stun of 0, and a
#     stunned FLUTE that neither moves nor starts a strike while the timers run it down.
# The expected numbers in studio/crates/bf-sim/src/kinds/jump/tests.rs are this output, pasted.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/jump_unit_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python vectors for the jump's, the rhapsody's and the stun's unit tests: hand-built state, the same calls, what
fight.py gave. Usage: jump_unit_vectors.py <repo root>"""
import copy, json, sys, zipfile
import numpy as np
clone = sys.argv[1]
sys.path.insert(0, clone + '/projects/ballfight')
import fight as F

T = 12.5


def episode_rules():
    meta = json.loads(zipfile.ZipFile(clone + '/studio/reference/fights/flute_vs_trombone/ep392.bfrec').read('meta.json'))
    r = meta['rules']
    R = F.Rules(bpm=r['bpm'], t_fight=r['t_fight'])
    for k, v in r.items():
        setattr(R, k, tuple(v) if isinstance(v, list) else v)
    return R


def fl(x):
    return repr(float(x))


def v2(a):
    return '[' + ', '.join(fl(x) for x in a) + ']'


def opt_v2(a):
    return 'None' if a is None else 'Some(' + v2(a) + ')'


def val(x):
    if isinstance(x, (bool, np.bool_)):
        raise TypeError('a bool in an event')
    if isinstance(x, (int, np.integer)):
        return f'I({int(x)})'
    if isinstance(x, (float, np.floating)):
        return f'F({fl(x)})'
    return f'S("{x}".into())'


def events(f, start=0):
    return 'vec![' + ', '.join(f'({fl(e[0])}, "{e[1]}", vec![' + ', '.join(val(x) for x in e[2:]) + '])'
                               for e in f.events[start:]) + ']'


def fight(left='CLARINET', right='FLUTE'):
    f = F.Fight(left, right, seed=392, rules=episode_rules(), record=False)
    f.t = T
    for k in (0, 1):
        if f.balls[k].w['kind'] == 'jump':
            f.balls[k].w = copy.deepcopy(f.balls[k].w)       # a test may change a setting of its own fight
            f.jump_setup(k)
    return f


# ---------------------------------------------------------------- _jump
W = F.FIGHTERS['CLARINET']['weapon']
LOW = F.FIGHTERS['CLARINET']['abilities']['low']
REG = dict(theta=0.0, land=W['land'], reach=W['base'], dmg=W['dmg'], knock=W['knock'], cut=W['cut'], kind=0, grow=True)
CHAIN = dict(theta=LOW['zig'], land=LOW['land'], reach=LOW['reach'], dmg=LOW['dmg'], knock=LOW['knock'],
             cut=LOW['cut'], kind=1, grow=False)
A = episode_rules().A
RB = episode_rules().R
LIM = A - RB
CASES = [
    # name, k, p0, v0, enemy p, enemy v, enemy shield, reach it has (blade_len), target (None = the enemy), overrides
    ('a cut through the enemy', 0, (-1.0, 0.25), (1.2, -2.0), (0.5, 0.5), (0.3, 2.5), 0.0, 2.6, None, {}),
    ('turned off the line', 0, (-1.0, 0.25), (1.2, -2.0), (0.5, 0.5), (0.3, 2.5), 0.0, 2.6, None, dict(theta=33.0)),
    ('an enemy that moved on is missed', 0, (-1.0, 0.25), (1.2, -2.0), (0.4, 2.1), (0.3, 2.5), 0.0, 2.6, (0.5, 0.5), {}),
    ('turned the other way', 0, (-1.0, 0.25), (1.2, -2.0), (0.5, 0.5), (0.3, 2.5), 0.0, 2.6, None, dict(theta=-17.5)),
    ('the reach caps a long jump', 0, (-2.2, -2.0), (0.0, 2.6), (2.0, 2.2), (-2.6, 0.0), 0.0, 2.6, None, {}),
    ('the ropes stop it', 0, (0.5, 1.0), (2.0, 1.0), (1.9, 2.1), (0.0, -2.6), 0.0, 4.2, None, {}),
    ('never inside the enemy', 0, (0.0, 0.0), (2.6, 0.0), (1.4, 0.3), (0.0, 2.6), 0.0, 2.6, (0.2, 0.1), dict(land=1.0)),
    ('away from the enemy', 0, (0.0, 0.0), (2.6, 0.0), (1.4, 0.3), (0.0, 2.6), 0.0, 2.6, (-0.2, 0.1), {}),
    ('landing on the enemy steps out along the jump', 0, (0.0, 0.0), (2.6, 0.0), (2.0, 0.0), (0.0, 2.6), 0.0, 2.6,
     (1.0, 0.0), dict(land=1.0)),
    ('a target on the ball jumps along its heading', 0, (0.5, -0.5), (1.5, 2.0), (-1.5, 1.0), (1.0, 1.0), 0.0, 2.6,
     (0.5, -0.5), {}),
    ('the enemy on the line is cut from its middle', 0, (0.0, 0.0), (0.0, 2.6), (1.0, 0.0), (0.0, -2.6), 0.0, 2.6, None, {}),
    ('a jump of no length', 0, (LIM, LIM), (2.6, 0.0), (0.0, 0.0), (0.0, 2.6), 0.0, 2.6, (LIM + 1.0, LIM + 1.0), {}),
    ('a shielded enemy blocks the cut, is knocked, grows nothing', 0, (-1.0, 0.25), (1.2, -2.0), (0.5, 0.5), (0.3, 2.5),
     0.8, 2.6, None, {}),
    ('the reach grows to its cap', 0, (-1.0, 0.25), (1.2, -2.0), (0.5, 0.5), (0.3, 2.5), 0.0, 4.17, None, {}),
    ('a wide cut', 0, (-1.0, 0.25), (1.2, -2.0), (0.5, 0.5), (0.3, 2.5), 0.0, 2.6, None, dict(theta=33.0, cut=0.4)),
    ('a chain jump grows nothing', 0, (-1.0, 0.25), (1.2, -2.0), (0.5, 0.5), (0.3, 2.5), 0.0, 2.6, None, CHAIN),
    ('a chain jump the other way', 0, (-1.0, 0.25), (1.2, -2.0), (0.5, 0.5), (0.3, 2.5), 0.0, 2.6, None,
     dict(CHAIN, theta=-LOW['zig'])),
    ('from the right corner', 1, (1.5, -1.0), (-2.0, 1.5), (-0.5, 0.2), (2.6, 0.0), 0.0, 3.1, None, dict(theta=-8.0)),
]
print('/// `_jump` on hand-built state: what `fight.py` gave.')
print('fn cases() -> Vec<Case> {')
print('    vec![')
for name, k, p0, v0, op, ov, shield, blade, tgt, over in CASES:
    f = fight() if k == 0 else fight('FLUTE', 'CLARINET')
    B, O = f.balls[k], f.balls[1 - k]
    B.p, B.v, O.p, O.v = (np.array(x, float) for x in (p0, v0, op, ov))
    O.shield = shield
    B.blade_len = blade
    a = dict(REG, reach=blade)
    a.update(over)
    t = np.array(op if tgt is None else tgt, float)
    f._jump(k, t, a['theta'], a['land'], a['reach'], a['dmg'], a['knock'], a['cut'], a['kind'], a['grow'])
    print(f'        Case {{ name: "{name}", k: {k}, p0: {v2(p0)}, v0: {v2(v0)}, op: {v2(op)}, ov: {v2(ov)}, '
          f'shield: {fl(shield)}, blade: {fl(blade)}, j: Jump {{ tgt: {v2(t)}, theta: {fl(a["theta"])}, '
          f'land: {fl(a["land"])}, reach: {fl(a["reach"])}, dmg: {fl(a["dmg"])}, knock: {fl(a["knock"])}, '
          f'cut: {fl(a["cut"])}, kind: {a["kind"]}, grow: {str(a["grow"]).lower()} }}, p1: {v2(B.p)}, v1: {v2(B.v)}, '
          f'ov1: {v2(O.v)}, oboost: {fl(O.boost)}, ohp: {fl(O.hp)}, dealt: {fl(B.dealt)}, blade1: {fl(B.blade_len)}, '
          f'events: {events(f)} }},')
print('    ]')
print('}')


# ---------------------------------------------------------------- jump_tick
def row(f, k, k16):
    B = f.balls[k]
    rh = B.rhap
    rhs = 'None' if rh is None else (f'Some(({fl(rh["n"])}, {fl(rh["side"])}, '
                                     + ('None' if rh['next'] is None else f'Some({fl(rh["next"])})') + '))')
    return (f'        Row {{ k16: {k16}, tgt: {opt_v2(B.jump_tgt)}, p: {v2(B.p)}, v: {v2(B.v)}, blade: {fl(B.blade_len)}, '
            f'rhap: {rhs}, events: {len(f.events)} }},')


def enemy_at(f, k, k16):
    """the enemy walks a fixed path between 16ths, so a lock and a jump see different spots"""
    O = f.balls[1 - k]
    O.p = np.array([0.6 + 0.11 * k16, -0.9 + 0.07 * k16])
    O.v = np.array([1.0, 2.4])


def run(fn, title, k16s, prepare=None, before=None, k=0, pair=('CLARINET', 'FLUTE')):
    f = fight(*pair)
    f.balls[k].p = np.array([-1.2, 0.4])
    f.balls[k].v = np.array([2.4, 1.0])
    if prepare:
        prepare(f)
    print()
    print(f'/// `jump_tick`: {title}.')
    print(f'fn {fn}() -> Run {{')
    print('    Run {')
    print('        rows: vec![')
    for k16 in k16s:
        enemy_at(f, k, k16)
        if before:
            before(f, k16)
        f.jump_tick(k, k16)
        print('    ' + row(f, k, k16))
    print('        ],')
    print(f'        events: {events(f)},')
    print(f'        next_draw: {fl(f.rng.random())},')
    print('    }')
    print('}')


def lead(f):
    f.balls[0].w['lead'] = 0.5


def haste(f):
    f.balls[0].haste, f.balls[0].haste_mult = 1.0, 2.0


def no_lock(f):
    f.balls[0].w['lock16'] = 0
    f.balls[0].w['jitter'] = 0.0


def dazed(f, k16):
    f.balls[0].stun = 0.3 if k16 in (2, 3) else 0.0
    f.balls[0].shadow = 0.2 if k16 == 8 else 0.0


def chain(f, k16):
    if k16 == 5:
        f.trigger(0, 'low')


def chain_plain(f, k16):
    if k16 == 5:
        ab = dict(F.FIGHTERS['CLARINET']['abilities']['low'])
        del ab['cut']
        ab['n'] = 2
        ab['step16'] = 3
        f.trigger(0, 'low', ab=ab, quiet=True)


run('run_grid', 'the grid, the lock and the lead', range(0, 10), prepare=lead)
run('run_haste', 'haste halves the grid; the lock is at most one 16th before', range(0, 9), prepare=haste)
run('run_no_lock', 'no lock, no jitter', range(0, 9), prepare=no_lock)
run('run_dazed', 'stunned on 16ths 2 and 3, in shadow on 8', range(0, 10), before=dazed)
run('run_chain', 'RHAPSODY from 16th 5', range(3, 18), before=chain)
run('run_chain_plain', 'a quiet chain of 2, 3 16ths apart, with the cut of the weapon', range(3, 14), before=chain_plain)
run('run_right', 'from the right corner', range(0, 6), k=1, pair=('FLUTE', 'CLARINET'))

# ---------------------------------------------------------------- SQUEAK
print('// ---- SQUEAK')
R = episode_rules()


def squeak(shield=0.0, stun0=0.0, stun=None, pair=('CLARINET', 'FLUTE'), k=0):
    f = fight(*pair)
    B, O = f.balls[k], f.balls[1 - k]
    B.p, O.p = np.array([0.0, 0.0]), np.array([1.5, 0.5])
    O.v = np.array([0.0, 2.6])
    O.shield = shield
    if stun0:
        O.stun = stun0
    ab = dict(F.FIGHTERS['CLARINET']['abilities']['unlock1'])
    if stun is not None:
        ab['stun'] = stun
        f.trigger(k, 'unlock1', ab=ab, quiet=True)
    else:
        f.trigger(k, 'unlock1')
    W_ = f.waves[-1]
    print('    //   the ring', {a: (v2(b) if isinstance(b, np.ndarray) else b) for a, b in W_.items()})
    n0 = len(f.events)
    step = 0
    for i in range(60):
        f.t = T + (i + 1) * R.DT
        f.effects_step(R.DT)
        if len(f.events) > n0:
            step = i + 1
            break
    print(f'    Squeak {{ step: {step}, t: {fl(f.t)}, events: {events(f, n0)}, stun: {fl(getattr(O, "stun", 0.0))}, '
          f'v: {v2(O.v)}, boost: {fl(O.boost)}, hp: {fl(O.hp)}, waves: {len(f.waves)}, '
          f'stunned: {str(f._stunned(1 - k)).lower()} }}')
    return f


print('// a ring that lands')
f = squeak()
print('// ... and the stunned FLUTE: three steps of move() and the timers, then a beat it would have dropped a basket on')
O = f.balls[1]
for i in range(3):
    p_, th_ = O.p.copy(), O.th
    f.move(R.DT)
    O.stun = max(0.0, O.stun - R.DT)
    print(f'//   step {i} p {v2(O.p)} moved {bool((O.p != p_).any())} stun {fl(O.stun)}')
print('// a longer stun is kept')
squeak(stun0=0.75)
print('// a shielded enemy is not stunned')
squeak(shield=1.0)
print('// a stun of 0 stuns nobody')
squeak(stun=0.0)
print('// from the right corner')
squeak(pair=('FLUTE', 'CLARINET'), k=1)
