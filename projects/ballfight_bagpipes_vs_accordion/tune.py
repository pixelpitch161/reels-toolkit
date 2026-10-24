"""Rule tuning for #30 (once, then frozen in timeline.py / fighters.py): win share, median K.O., lead changes and fit
rate over seeds, for a grid of roster / rule overrides.

    python tune.py 40 P.dmg=0.8,0.9 A.dmg=5,6 A.every=4 scale=1.0
        P.<key> / A.<key>      BAGPIPES / ACCORDION weapon fields;  P.u1.<key>, A.low.<key> ...  ability fields
        scale                  the damage scale (timeline.DMG_SCALE)

Also reports how the damage splits (aura ticks vs bellows push / suction) and how often the accordion's suction drags
it into the drone (aura contacts that start within 0.5 s after a suction hit)."""
import itertools
import os
import sys
from multiprocessing import Pool
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import sim  # noqa: E402  (sets up the engine path)
import fighters as FT  # noqa: E402
import timeline as T  # noqa: E402
from fight import Fight, Rules, lead_changes  # noqa: E402

WHO = {'P': 'BAGPIPES', 'A': 'ACCORDION'}
SLOT = {'u1': 'unlock1', 'u2': 'unlock2', 'low': 'low'}


def apply(cell):
    sc = T.DMG_SCALE
    for key, val in cell:
        if key == 'scale':
            sc = val
            continue
        parts = key.split('.')
        cfg = FT.FIGHTERS[WHO[parts[0]]]
        if len(parts) == 2:
            cfg['weapon'][parts[1]] = val
        else:
            cfg['abilities'][SLOT[parts[1]]][parts[2]] = val
    return sc


def one(args):
    cell, seed = args
    sc = apply(cell)
    r = Fight(T.FIGHTERS[0], T.FIGHTERS[1], seed=seed, rules=Rules(bpm=T.BPM, t_fight=T.T_FIGHT, dmg_scale=sc),
              record=False).run()
    lc, _ = lead_changes(r['events'])
    hp = [100.0, 100.0]
    by = {}
    contacts, dragged, last_suck = 0, 0, -9.0
    for e in r['events']:
        if e[1] == 'hit':
            hp[e[3]] -= e[4]
            by[e[7]] = by.get(e[7], 0.0) + e[4]
            if e[7] == 'suction':
                last_suck = e[0]
            if e[7] == 'aura' and e[8]:
                contacts += 1
                dragged += e[0] - last_suck < 0.5
    caps = len(sim.captions(r['events']))
    return (cell, seed, r['winner'], r['t_ko'] or 99, lc, max(hp), caps, by, contacts, dragged)


if __name__ == '__main__':
    n = int(sys.argv[1])
    axes = []
    for a in sys.argv[2:]:
        k, v = a.split('=')
        axes.append([(k, float(x)) for x in v.split(',')])
    cells = list(itertools.product(*axes))
    jobs = [(c, s) for c in cells for s in range(1000, 1000 + n)]
    with Pool(2) as P:
        res = P.map(one, jobs, chunksize=4)
    for cell in cells:
        r = [x for x in res if x[0] == cell]
        g = np.mean([x[2] == 0 for x in r if x[2] is not None]) if any(x[2] is not None for x in r) else 0
        tko = np.array([x[3] for x in r])
        lc = np.mean([x[4] for x in r])
        whp = np.median([x[5] for x in r])
        fit = np.mean([(49 <= x[3] <= 53) and x[4] >= 2 and x[5] <= 30 and x[6] >= 5 for x in r])
        w_ko = np.mean([49 <= x[3] <= 53 for x in r])
        w_lc = np.mean([x[4] >= 2 for x in r])
        w_cap = np.mean([x[6] >= 5 for x in r])
        kinds = sorted({k for x in r for k in x[7]})
        split = ' '.join(f'{k} {np.mean([x[7].get(k, 0) for x in r]):.0f}' for k in kinds)
        con = np.mean([x[8] for x in r])
        drag = np.mean([x[9] for x in r])
        lab = ' '.join(f'{k}={v:g}' for k, v in cell)
        print(f'{lab}: bagpipes win {100 * g:3.0f}%  KO med {np.median(tko):5.1f} ({np.percentile(tko, 10):4.1f}-'
              f'{np.percentile(tko, 90):4.1f})  lc {lc:.2f}  whp {whp:3.0f}  fit {100 * fit:3.1f}% (ko {100 * w_ko:.0f} lc2 '
              f'{100 * w_lc:.0f} caps5 {100 * w_cap:.0f})  '
              f'contacts {con:.0f} (dragged {drag:.1f})  dmg: {split}', flush=True)
