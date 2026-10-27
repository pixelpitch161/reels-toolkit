"""Random search over tune.py's grid keys (once, before freezing the rules): samples settings in the ranges below, runs
`n` seeds each (seeds 1000.., never the episode's) and ranks them by the hard-window fit rate, with penalties when the
balance leaves 40-60 % or the median K.O. leaves 49-53 s. The winner is then checked with tune.py and frozen.

    python tune_search.py 60 60          # 60 settings x 60 seeds -> check/tune_search.txt (best first)"""
import os
import sys
from multiprocessing import Pool
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import tune  # noqa: E402

# v2 (Oct 26): the BASSOON is frozen since v1; only the new BARI SAX (WRECKING BALL, GROWL, CABLE SNAP) and the shared
# damage scale are searched
RANGES = dict(wd=(8.0, 20.0), wp=(0.0, 2.5), wb=(1.5, 2.4), wg=(0.05, 0.20), wx=(2.6, 3.4), wt=(35.0, 60.0),
              wj=(0.0, 35.0), ws=(0.8, 1.3), wq=(1.2, 1.8), we=(0.0, 0.3), wm=(0.2, 0.6), wk=(1.5, 3.0), gd=(5.0, 9.0),
              cd=(16.0, 36.0), cr=(0.3, 0.9), sc=(0.32, 0.48))
FIXED = dict(wl=1.0, gs=9.0)


def evaluate(cell, n):
    res = [tune.one((cell, s)) for s in range(1000, 1000 + n)]
    dec = [x for x in res if x[2] is not None and x[3] < 99]
    bal = 100 * np.mean([x[2] == 0 for x in dec]) if dec else 0.0
    ko = float(np.median([x[3] for x in res]))
    fit = 100 * np.mean([x[9] for x in res])
    lc = float(np.mean([x[4] for x in res]))
    caps = float(np.mean([x[6] for x in res]))
    score = fit - 1.0 * max(0.0, abs(bal - 50) - 10) - 2.0 * max(0.0, abs(ko - 51) - 2) + 0.5 * lc + 0.3 * caps
    return score, fit, bal, ko, lc, caps


def job(args):
    i, n = args
    rng = np.random.default_rng(i)
    cell = tuple(sorted([(k, round(float(rng.uniform(*r)), 3)) for k, r in RANGES.items()] +
                        [(k, v) for k, v in FIXED.items()]))
    return (cell,) + evaluate(cell, n)


if __name__ == '__main__':
    m, n = int(sys.argv[1]), int(sys.argv[2])
    out = []
    with Pool(2) as P:
        for r in P.imap_unordered(job, [(i, n) for i in range(m)]):
            out.append(r)
            print(f'score {r[1]:6.2f} fit {r[2]:4.1f}% bal {r[3]:3.0f}% ko {r[4]:4.1f} lc {r[5]:.2f} caps {r[6]:.1f}', flush=True)
    out.sort(key=lambda r: -r[1])
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    with open(os.path.join(HERE, 'check', 'tune_search.txt'), 'w') as f:
        for r in out:
            f.write(f'score {r[1]:.2f} fit {r[2]:.1f} bal {r[3]:.0f} ko {r[4]:.1f} lc {r[5]:.2f} caps {r[6]:.1f}  ' +
                    ' '.join(f'{k}={v:g}' for k, v in r[0]) + '\n')
    print(open(os.path.join(HERE, 'check', 'tune_search.txt')).read()[:3000])
