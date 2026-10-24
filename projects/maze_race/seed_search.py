"""Find an honest race whose story fits the beat sheet. Nothing is ever edited: the chosen seeds are played exactly
as simulated.

Stage A (the maze): run the three paint explorers on maze seeds A..B and score their race alone.
Stage B (the tilt): for the best mazes, run the marble under tilt seeds and score the whole four-way race.
Both stages run in parallel and stop early once enough candidates fit (--stop).

    python seed_search.py 0 4000 --stop 5          # -> check/mazes_A_B.txt, check/races_A_B.txt

Story targets (beat sheet, 61 s):
    first finish ~46 s (44-48), a close second (by 51 s), lead changes during the race (one in 24-34 s),
    SEEKER trapped in a dead end near the centre around 17-28 s ("SO CLOSE..."), the early leader (~11 s) doesn't
    win, two contenders in the final stretch, and something happening every few seconds (lead change, a trap near
    the centre, a long backtrack, a long marble run, a finish). The marble should matter: a contender, ideally a
    finisher."""
import argparse
import os
import sys
import time
from multiprocessing import Pool

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import maze as M

T_FIRST = 46.0


def events(r):
    """story events (t, kind) during the race"""
    ev = [(t, 'lead') for t, q in r.leaders()[1:]]
    ev += [(t, 'trap') for t, c in r.seeker_traps()]
    ev += [(t, 'back') for t, n in r.diver_deadends() if n >= 5]
    ev += [(t, 'run') for t, n in r.marble_runs(5)]
    ev += [(f, 'fin') for f in r.fin if f is not None]
    return sorted(ev)


def score(r, with_marble):
    fins = sorted((f, q) for q, f in enumerate(r.fin) if f is not None)
    if not fins:
        return -99.0, {}
    f1, w = fins[0]
    if not (43.8 <= f1 <= 48.2):
        return -50 - abs(f1 - T_FIRST), {}
    s = 10 - 1.5 * abs(f1 - T_FIRST)
    info = dict(first=round(f1, 2), winner=M.NAMES[w])
    # a close second
    if len(fins) >= 2:
        gap = fins[1][0] - f1
        s += 4 if gap <= 2.5 else (2.5 if gap <= 4.5 else 1)
        info['second'] = (round(fins[1][0], 2), M.NAMES[fins[1][1]])
    else:
        s -= 3
    # two contenders in the final stretch
    p = sorted([r.progress_at(q, f1 - 1.5) for q in range(4 if with_marble else 3)], reverse=True)
    s += 3 * min(1.0, max(0.0, (p[1] - 0.6) / 0.3))
    # lead changes
    L = r.leaders()
    ch = [t for t, q in L[1:] if t < f1]
    s += min(len(ch), 5) * 1.2
    if any(24 <= t <= 34 for t in ch):
        s += 2.5
    early = [q for t, q in L if t <= 11.0]
    if early and early[-1] != w:
        s += 2.5                                             # the early leader doesn't win
    info['leads'] = [(round(t, 1), M.NAMES[q]) for t, q in L]
    # SEEKER trapped near the centre ("SO CLOSE...")
    traps = [t for t, c in r.seeker_traps()]
    if any(17 <= t <= 28 for t in traps):
        s += 3
    info['traps'] = [round(t, 1) for t in traps][:6]
    # pacing: the longest stretch with nothing happening (6 s .. first finish)
    ev = [t for t, k in events(r) if 6.0 <= t <= f1] + [6.0, f1]
    ev.sort()
    worst = max(b - a for a, b in zip(ev, ev[1:]))
    s -= max(0.0, worst - 5.0) * 1.2
    info['gap'] = round(worst, 1)
    if with_marble:
        pm = r.progress_at(3, M.T_STOP)
        s += 5 * pm
        if r.fin[3] is not None:
            s += 6
        runs = r.marble_runs(5)
        if any(14 <= t <= 21 for t, n in runs):
            s += 1.5                                           # the tilt sends it down a long corridor
        if any(32 <= t <= 41 for t, n in runs):
            s += 1.5                                           # ... and frees it later
        info['marble'] = (round(pm, 2), None if r.fin[3] is None else round(r.fin[3], 1), len(r.hits))
    return s, info


def run_a(seed):
    r = M.Race(seed).run_paint()
    s, info = score(r, False)
    w = int(np.argmin([f if f is not None else 99 for f in r.fin[:3]]))
    return seed, s, info, r.mz.D0, w, min(f for f in r.fin[:3] if f is not None) if any(r.fin[:3]) else None


def run_b(args):
    ms, ts = args
    r = M.Race(ms, ts).run()
    s, info = score(r, True)
    return ms, ts, s, info


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('a', type=int)
    ap.add_argument('b', type=int)
    ap.add_argument('--stop', type=int, default=5)
    ap.add_argument('--mazes', type=int, default=6, help='how many of the best mazes get a tilt search')
    ap.add_argument('--tilts', type=int, default=1500)
    ap.add_argument('--procs', type=int, default=os.cpu_count() or 2)
    a = ap.parse_args()
    os.makedirs('check', exist_ok=True)
    t0 = time.time()
    good, rows, wins, firsts = [], [], np.zeros(3, int), []
    with Pool(a.procs) as P:
        for seed, s, info, d0, w, f1 in P.imap_unordered(run_a, range(a.a, a.b), chunksize=8):
            rows.append((s, seed, info, d0))
            wins[w] += 1
            firsts.append(f1 or 99)
            if s > 18:
                good.append(seed)
            if len(good) >= a.stop * 3:
                P.terminate()
                break
    rows.sort(key=lambda x: -x[0])
    with open(f'check/mazes_{a.a}_{a.b}.txt', 'w') as fh:
        fh.write(f'{len(rows)} mazes run; paint winners FLOOD/DIVER/SEEKER {wins.tolist()}; '
                 f'first finish median {np.median(firsts):.1f} s\n')
        for s, seed, info, d0 in rows[:60]:
            fh.write(f'{s:6.1f}  maze {seed:5d}  D0 {d0}  {info}\n')
    print(f'stage A: {len(rows)} mazes in {time.time() - t0:.0f} s; winners F/D/S {wins.tolist()}; top:')
    for s, seed, info, d0 in rows[:8]:
        print(f'  {s:6.1f}  maze {seed}  D0 {d0}  {info}')
    best = [seed for s, seed, info, d0 in rows[:a.mazes] if s > 0]
    out = []
    for ms in best:
        found = 0
        with Pool(a.procs) as P:
            for res in P.imap_unordered(run_b, [(ms, ts) for ts in range(a.tilts)], chunksize=10):
                out.append(res)
                if res[2] > 34:
                    found += 1
                if found >= a.stop:
                    P.terminate()
                    break
        print(f'  maze {ms}: tilt search done ({time.time() - t0:.0f} s), best {max(x[2] for x in out if x[0] == ms):.1f}, '
              f'{found} fit', flush=True)
    out.sort(key=lambda x: -x[2])
    with open(f'check/races_{a.a}_{a.b}.txt', 'w') as fh:
        for ms, ts, s, info in out[:80]:
            fh.write(f'{s:6.1f}  maze {ms:5d}  tilt {ts:5d}  {info}\n')
    print('stage B top:')
    for ms, ts, s, info in out[:10]:
        print(f'  {s:6.1f}  maze {ms}  tilt {ts}  {info}')


if __name__ == '__main__':
    main()
