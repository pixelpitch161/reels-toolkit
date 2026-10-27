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

RANGES = dict(bd=(0.9, 1.6), bb=(2.0, 3.0), bs=(0.18, 0.30), bg=(0.004, 0.015), bj=(20, 60), bt=(0.30, 0.60),
              rd=(4.0, 7.0), rk=(1.4, 2.2), sl=(2.0, 4.0), sk=(1.2, 2.0), rs=(0.30, 0.60), rx=(1.6, 2.4), gd=(5, 9),
              kd=(3, 6), bm=(0.8, 1.6), qd=(8, 14), sc=(0.38, 0.50))
FIXED = dict(bl=0.6, gs=9.0, qs=9.0, ks=1.0)


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
