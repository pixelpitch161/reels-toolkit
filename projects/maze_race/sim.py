"""Run the chosen race (seed.txt: "<maze seed> <tilt seed>", picked by seed_search.py) and cache it -> sim.pkl.
The race is played exactly as simulated; nothing downstream changes an event."""
import os
import pickle
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import maze as M

PATH = os.path.join(HERE, 'sim.pkl')


def seeds():
    a, b = open(os.path.join(HERE, 'seed.txt')).read().split()[:2]
    return int(a), int(b)


def build():
    ms, ts = seeds()
    r = M.Race(ms, ts).run(fps=30)
    data = dict(
        maze_seed=ms, tilt_seed=ts, N=M.N, D0=r.mz.D0,
        open=[tuple(sorted(tuple(e))) for e in r.mz.open],
        walls=r.mz.walls(), dist=dict(r.mz.dist),
        steps=r.steps[:3], fin=list(r.fin), prog=r.prog,
        mpos=r.mpos, mcells=r.mcells, hits=r.hits,
        leaders=r.leaders(), traps=r.seeker_traps(), deadends=r.diver_deadends(), runs=r.marble_runs(5),
    )
    with open(PATH, 'wb') as fh:
        pickle.dump(data, fh)
    return data


def load():
    if not os.path.exists(PATH) or os.path.getmtime(PATH) < max(os.path.getmtime(os.path.join(HERE, f)) for f in ('maze.py', 'seed.txt')):
        return build()
    with open(PATH, 'rb') as fh:
        return pickle.load(fh)


if __name__ == '__main__':
    d = build()
    print(f"maze {d['maze_seed']} tilt {d['tilt_seed']}: D0 {d['D0']}, finishes", [None if f is None else round(f, 2) for f in d['fin']])
    print('leaders', [(round(t, 1), M.NAMES[q]) for t, q in d['leaders']])
    print('seeker traps', [(round(t, 1), c) for t, c in d['traps']])
    print('diver dead ends', [(round(t, 1), n) for t, n in d['deadends']][:20])
    print('marble runs', [(round(t, 1), n) for t, n in d['runs']], 'hits', len(d['hits']))
