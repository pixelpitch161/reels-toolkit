"""Seed search for #26: run honest simulations and keep the ones whose own run fits the beat sheet.

    python seed_search.py 1 400 --stop 6        # seeds 1..400 on all cores, stop once 6 runs fit every window

Hard windows (timeline.py): first out, final 3, duel start, the win, and no lull longer than MAX_LULL before the
duel. Soft extras rank the fits: a real duel (several close calls at the gap edge, 3-7 s long), a near-miss in
20-32 s, an underdog going deep. Writes check/seeds.txt (ranked) so later episodes can reuse it. Never edits a run.
"""
import argparse
import os
import sys
import numpy as np
from multiprocessing import Pool

import timeline as T
import sim
from pp_bridge import NAMES

UNDERDOGS = ('UKULELE', 'BAGPIPES')


def score(seed):
    R = sim.run_seed(seed)
    st = sim.story(R)
    tm = st['times']
    info = dict(seed=seed, n=st['n_out'], winner=NAMES[st['winner']] if st['winner'] >= 0 else '-',
                times=[round(float(x), 2) for x in tm])
    if st['n_out'] < 15 or st['winner'] < 0:
        return 0, -99.0, info
    first, final3, duel, win = tm[0], tm[12], tm[13], tm[14]
    lull = float(np.max(np.diff(np.concatenate([[0.0], tm[:14]]))))
    ok = (T.T_FIRST_OUT[0] <= first <= T.T_FIRST_OUT[1] and T.T_FINAL3[0] <= final3 <= T.T_FINAL3[1]
          and T.T_DUEL[0] <= duel <= T.T_DUEL[1] and T.T_LAST_OUT[0] <= win <= T.T_LAST_OUT[1] and lull <= T.MAX_LULL)
    ev = R['EV']
    order = list(st['order']) + [st['winner']]
    finalists = order[-2:]
    edges = ev[ev[:, 2] == 2]
    duel_edges = int(((edges[:, 0] > duel) & (edges[:, 0] < win) & np.isin(edges[:, 1], finalists)).sum())
    near = edges[(edges[:, 0] > 20) & (edges[:, 0] < 32)]
    survivors_near = [int(b) for b in near[:, 1] if R['T_OUT'][int(b)] < 0 or R['T_OUT'][int(b)] > 32.5]
    place = {int(b): 16 - k for k, b in enumerate(order)}            # 16 = first out ... 1 = winner
    under = min(place[NAMES.index(u)] for u in UNDERDOGS)
    soft = (min(duel_edges, 6) * 0.5 + (1.0 if 3.0 <= win - duel <= 7.0 else 0.0) + (1.0 if survivors_near else 0.0)
            + (1.0 if under <= 3 else 0.5 if under <= 5 else 0.0) - abs(win - 49.5) * 0.3 - max(0.0, lull - 4.5) * 0.3)
    info.update(lull=round(lull, 2), duel_edges=duel_edges, near=len(survivors_near), underdog_place=under,
                finalists=[NAMES[b] for b in finalists])
    return int(ok), float(soft), info


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('a', type=int)
    ap.add_argument('b', type=int)
    ap.add_argument('--stop', type=int, default=6)
    ap.add_argument('--procs', type=int, default=os.cpu_count() or 2)
    a = ap.parse_args()
    fits, rows = [], []
    with Pool(a.procs) as P:
        for ok, soft, info in P.imap_unordered(score, range(a.a, a.b + 1), chunksize=2):
            rows.append((ok, soft, info))
            if ok:
                fits.append((soft, info))
                print(f'FIT seed {info["seed"]:5d} soft {soft:5.2f} winner {info["winner"]:9s} final {info["finalists"]} '
                      f'duel_edges {info["duel_edges"]} near {info["near"]} underdog {info["underdog_place"]} '
                      f'times {info["times"]}', flush=True)
                if len(fits) >= a.stop:
                    P.terminate()
                    break
    os.makedirs('check', exist_ok=True)
    rows.sort(key=lambda r: (-r[0], -r[1]))
    with open('check/seeds.txt', 'w') as f:
        for ok, soft, info in rows:
            f.write(f'{"FIT " if ok else "    "} {soft:6.2f} {info}\n')
    print(f'{len(rows)} runs, {len(fits)} fits -> check/seeds.txt')


if __name__ == '__main__':
    sys.exit(main())
