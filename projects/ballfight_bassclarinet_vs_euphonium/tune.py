"""Rule tuning for #38 (once; the result is frozen in projects/ballfight/fighters.py and timeline.DMG_SCALE). Both
fighters are new, so their own numbers are the levers. Reports the win share, median K.O., lead changes, fit rate,
caption count and the hit mix over seeds, for a grid of settings.

    python tune.py 80 ad=5,6 md=2.5,3 sc=0.8,1.0

Grid keys: ad = BASS CLARINET anchor smash damage, ao = its base orbit, ag = its orbit growth, at = turns per whirl, ar = release boost,
al = plant lead, sd = SLAP TONGUE damage, ld = LIGHTS OUT damage; md = EUPHONIUM shell damage, ms = its splash,
ml = its lead, mg = its growth (shells per landed salvo), mp = its spread, bd = NOT A TUBA shell damage; sc = the shared
damage scale. Seeds 1000.. (the honest search later uses 1..), so the tuning never picks the episode's run."""
import itertools
import os
import sys
from multiprocessing import Pool
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import sim  # noqa: E402  (sets up the engine path)
import timeline as T  # noqa: E402
from fight import Fight, lead_changes  # noqa: E402
from fighters import FIGHTERS  # noqa: E402

BC, EU = T.FIGHTERS
KEYS = {'ad': (BC, 'weapon', 'dmg'), 'ao': (BC, 'weapon', 'base'), 'at': (BC, 'weapon', 'turns'),
        'ar': (BC, 'weapon', 'release'), 'ag': (BC, 'weapon', 'grow'), 'al': (BC, 'weapon', 'lead'), 'sd': (BC, 'unlock1', 'dmg'),
        'ld': (BC, 'low', 'dmg'), 'md': (EU, 'weapon', 'dmg'), 'ms': (EU, 'weapon', 'splash'),
        'ml': (EU, 'weapon', 'lead'), 'mg': (EU, 'weapon', 'grow'), 'mp': (EU, 'weapon', 'spread'),
        'bd': (EU, 'low', 'dmg')}


def apply(cell):
    for k, v in cell:
        if k in KEYS:
            name, part, field = KEYS[k]
            d = FIGHTERS[name]['weapon'] if part == 'weapon' else FIGHTERS[name]['abilities'][part]
            d[field] = v


def one(args):
    cell, seed = args
    apply(cell)
    c = dict(cell)
    r = Fight(BC, EU, seed=seed, rules=sim.rules(c.get('sc')), record=False).run()
    st = sim.story(r)
    fits, _ = sim.score(st)
    lc, _ = lead_changes(r['events'])
    hp = [100.0, 100.0]
    n_hits = [0, 0]
    copies = sum(e[1] == 'copy' for e in r['events'])
    for e in r['events']:
        if e[1] == 'hit':
            hp[e[3]] -= e[4]
            n_hits[e[2]] += 1
    return (cell, seed, r['winner'], r['t_ko'] or 99, lc, max(hp), len(st['caps']), n_hits, st['max_gap'], fits,
            len(st['lows']), copies)


if __name__ == '__main__':
    n = int(sys.argv[1])
    axes = []
    for a in sys.argv[2:]:
        k, v = a.split('=')
        axes.append([(k, float(x)) for x in v.split(',')])
    cells = list(itertools.product(*axes)) or [()]
    jobs = [(c, s) for c in cells for s in range(1000, 1000 + n)]
    with Pool(2) as P:
        res = P.map(one, jobs, chunksize=4)
    for cell in cells:
        r = [x for x in res if x[0] == cell]
        dec = [x for x in r if x[2] is not None and x[3] < 99]
        g = np.mean([x[2] == 0 for x in dec]) if dec else 0.0
        tko = np.array([x[3] for x in r])
        lab = ' '.join(f'{k}={v:g}' for k, v in cell) or 'as built'
        hits = np.mean([x[7] for x in r], 0)
        print(f'{lab}: BASS CL wins {100 * g:3.0f}% of {len(dec)}  KO med {np.median(tko):5.1f} '
              f'({np.percentile(tko, 10):4.1f}-{np.percentile(tko, 90):4.1f})  no-KO {100 * np.mean(tko >= 99):2.0f}%  '
              f'lc {np.mean([x[4] for x in r]):.2f}  whp {np.median([x[5] for x in r]):3.0f}  caps {np.mean([x[6] for x in r]):.1f}  '
              f'both lows {100 * np.mean([x[10] == 2 for x in r]):2.0f}%  gap med {np.median([x[8] for x in r]):.1f}  '
              f'fit {100 * np.mean([x[9] for x in r]):.1f}%  hits B/E {hits[0]:.0f}/{hits[1]:.0f}  copies {np.mean([x[11] for x in r]):.1f}',
              flush=True)
