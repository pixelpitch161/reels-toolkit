# What it prints: the striker's tests that drive the fight loop: run() with both balls held in place and haste
# switched on by hand, the lance kept idle on that one Fight object (the kit's files are not edited).
# The expected numbers in studio/crates/bf-sim/src/kinds/striker/tests.rs are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/checks/striker_grid_vectors.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""Python checks of the striker tests that drive the fight loop: both balls held in place (set at the end of every
step, which is before the next one), haste switched on by hand. Usage: striker_grid_vectors.py <repo root>"""
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


def held(steps, haste_from=None):
    R = episode_rules()
    R.t_max = R.t_fight + (steps - 0.5) * R.DT
    f = F.Fight('FLUTE', 'TROMBONE', seed=392, rules=R, record=False)
    f.n = 0
    # the TROMBONE's lance is not what is looked at: keep it from thrusting, hitting or swatting
    f.start_thrust = lambda k: None
    f.blade_step = lambda k, dt: None
    orig = f.abilities_step

    def hold():
        f.balls[0].p = np.array([-1.85, 1.85])
        f.balls[1].p = np.array([-0.8, 0.8])
        if haste_from is not None and f.n >= haste_from:
            f.balls[0].haste = 1.0
            f.balls[0].haste_mult = 2.0

    def patched(on_beat):
        orig(on_beat)
        f.n += 1
        hold()
    f.abilities_step = patched
    hold()
    f.run()
    out = []
    for e in f.events:
        if e[1] == 'strike':
            step = round((e[0] - R.t_fight) / R.DT)
            s16 = R.beat / 4
            k16 = math.floor((R.t_fight + step * R.DT + 1e-9) / s16) - 32
            out.append((step, k16, k16 // 4 if k16 % 4 == 0 else None, e[3], e[6]))
    return f, out


f, out = held(300)
print('no haste, 300 steps: (step, 16th, beat, basket, tk)')
for o in out:
    print('  ', o)
print('  steps', f.n, 'kinds', sorted(set(e[1] for e in f.events)))
f, out = held(330, haste_from=150)
print('haste from step 150, 330 steps:')
for o in out:
    print('  ', o)

print('== a swatted cobra misses and waits')
beat, dt = episode_rules().beat, episode_rules().DT
f = F.Fight('FLUTE', 'TROMBONE', seed=392, rules=episode_rules(), record=False)
f.striker_setup(0)
B, O = f.balls
B.t_strike = B.w['t_strike']
bk = dict(id=0, home=np.array([0.5, -0.25]), cp=np.array([0.5, -0.25]), t_drop=10.0, strike=None)
B.next_bk = 1
B.baskets.append(bk)
O.p = np.array([1.9, 0.6])
T = 10.0 + beat
f.t = T
f.striker_fire(0)
for i in range(1, 5):
    f.t = T + i * dt
    f.striker_step(0, dt)
aim = bk['strike']['aim']
across = np.array([-math.sin(aim), math.cos(aim)])
along = np.array([math.cos(aim), math.sin(aim)])
a = np.array([0.5 + along[0] * 0.2 - across[0] * 0.5, -0.25 + along[1] * 0.2 - across[1] * 0.5])
f.swat_cobras(1, a, across, 1.0)
print('  events', [tuple(float(x) if isinstance(x, float) else x for x in e) for e in f.events])
print('  down', repr(bk['down']), 't', repr(f.t), 't + 2 beat', repr(f.t + 2 * beat), 'done', bk['strike']['done'])
down = bk['down']
for i in range(5, 46):
    f.t = T + i * dt
    f.striker_step(0, dt)
print('  after 45 steps: events', len(f.events), 'hp', O.hp, 'strike', bk['strike'])
f.t = down - dt
f.striker_fire(0)
print('  fire at down - dt: events', len(f.events))
f.t = down
f.striker_fire(0)
print('  fire at down: events', len(f.events), f.events[-1][1])
