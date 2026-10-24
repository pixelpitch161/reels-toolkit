"""Run the chosen honest seed (seed.txt, picked by seed_search.py) and cache every frame in sim.npz.

    python sim.py            # -> sim.npz (positions, headings, teams per frame, catches, wipe-outs, winner)"""
import os
import sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import rps
import episode as E

PATH = os.path.join(HERE, 'sim.npz')


def seed():
    return int(open(os.path.join(HERE, 'seed.txt')).read().split()[0])


def load():
    if not os.path.exists(PATH):
        main()
    return dict(np.load(PATH, allow_pickle=False))


def main():
    s = seed()
    rec = rps.run(s, E.PARAMS)
    if rec['t_win'] is None:
        raise SystemExit(f'seed {s} has no winner before the gather')
    rps.save(rec, PATH)
    c = rps.counts(rec)
    print(f'seed {s}: outs', [(round(t, 2), rps.NAMES[q]) for t, q in rec['outs']], 'win', round(rec['t_win'], 2),
          rps.NAMES[rec['winner']], 'catches', len(rec['catches']))


if __name__ == '__main__':
    main()
