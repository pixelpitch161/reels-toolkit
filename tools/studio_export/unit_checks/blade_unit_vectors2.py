# What it prints: the thrust blade without `rest`, and the lance at the boundaries of its out / hold / back phases.
# The expected numbers in studio/crates/bf-sim/src/kinds/blade/tests.rs are this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/blade_unit_vectors2.py .
# The argument is the repo root: the kit is imported from its projects/ballfight. This is a check script, not
# an export: the tool's export discovery does not look in this folder.
"""More Python vectors for the thrust blade's unit tests: a lance without `rest`, and the phase boundaries.
Usage: blade_unit_vectors2.py <repo root>"""
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


def fl(x):
    return repr(float(x))


dt = episode_rules().DT

print('== a lance without `rest`: 0.0 on the new ball, 0.25 once it steps')
F.FIGHTERS['NOREST'] = dict(F.FIGHTERS['TROMBONE'], weapon={k: v for k, v in F.FIGHTERS['TROMBONE']['weapon'].items() if k != 'rest'})
f = F.Fight('FLUTE', 'NOREST', seed=392, rules=episode_rules(swat_cobras=False), record=False)
B, O = f.balls[1], f.balls[0]
print('  new ball ext', fl(B.ext))
B.p, O.p = np.array([2.0, -2.0]), np.array([-2.0, 2.0])
f.t = 12.5
f.thrust_step(1, dt)
print('  after an idle step ext', fl(B.ext))
f.start_thrust(1)
rows = []
for i in range(0, 41):
    f.t = 12.5 + i * dt
    f.thrust_step(1, dt)
    rows.append(fl(B.ext))
print('  ext', '[' + ', '.join(rows[:12]) + ']')
print('  ext 17..20', '[' + ', '.join(rows[17:21]) + ']', 'ext 38..40', rows[38:41])
del F.FIGHTERS['NOREST']

print('== phase boundaries: t_out = 2 dt, t_hold = 2 dt, t_back = 4 dt, the thrust started at t = 0')
for bl in (0.95, 1.03, 1.11, 2.47):
    f = F.Fight('FLUTE', 'TROMBONE', seed=392, rules=episode_rules(swat_cobras=False), record=False)
    B, O = f.balls[1], f.balls[0]
    B.w = dict(B.w, t_out=2 * dt, t_hold=2 * dt, t_back=4 * dt)
    B.blade_len = bl
    B.p, O.p = np.array([2.0, -2.0]), np.array([-2.0, 2.0])
    f.t = 0.0
    f.start_thrust(1)
    out = []
    for i in range(0, 10):
        f.t = i * dt
        if i in (4, 8):
            # the enemy stands on the lance exactly as the hold ends / as the thrust ends
            d = np.array([math.cos(B.th), math.sin(B.th)])
            O.p = B.p + d * 1.0
        else:
            O.p = np.array([-2.0, 2.0])
        n0 = len(f.events)
        f.thrust_step(1, dt)
        u = None if B.thr is None else f.t / B.thr_k
        out.append((i, fl(B.ext), B.thr is not None, [e[1] for e in f.events[n0:]]))
    print('  bl', fl(bl), 'u == t_out at step 2:', 2 * dt == 2 * dt, 'sum', (2 * dt + 2 * dt) == 4 * dt, (2 * dt + 2 * dt + 4 * dt) == 8 * dt)
    for o in out:
        print('   ', o)
    e = 1.0
    print('   mutated out branch at e = 1 would give', fl(0.3 + (bl - 0.3) * (1 - (1 - e) ** 2)), 'vs bl', fl(bl))
