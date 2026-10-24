"""Run honest seeds of the chase and rank them by how well their story fits the beat sheet. Nothing is edited: the
chosen seed plays exactly as simulated.

    python seed_search.py 0 400 --stop 5      # stops once 5 seeds fit; ranked list -> check/seeds.txt
                                              # every run's counts (0.5 s steps) -> check/raw_A_B.jsonl for re-scoring

Shape : first wipe-out ~37 s (32-42 ok), last straggler caught ~50 s (46-54 ok), mop-up >= 6 s.
Story : a team crashes to a handful (<= 6 pieces) between 16 s and the wipe-out, then comes back (>= 3x its low and
        >= 15 pieces) before it; everyone still in the game at 10 s; the doomed team's last catches spread over a few
        seconds (stragglers dodging), lead changes."""
import argparse
import json
import os
import sys
import numpy as np
from multiprocessing import Pool

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import rps
import episode as E

FPS = rps.FPS


def story(rec):
    outs = rec['outs']
    if len(outs) < 2 or rec['t_win'] is None:
        return None
    t1, q1 = outs[0]
    tw = rec['t_win']
    c = rps.counts(rec)
    f1 = int(t1 * FPS)
    s = {'t1': round(t1, 2), 'tw': round(tw, 2), 'out': rps.NAMES[q1], 'win': rps.NAMES[rec['winner']]}
    if not (32.0 <= t1 <= 42.0 and 46.0 <= tw <= 54.0 and tw - t1 >= 6.0):
        return None
    if c[10 * FPS].min() < 15:
        return None
    # dip + comeback before the wipe-out
    best = None
    for q in range(3):
        a, b = 16 * FPS, max(16 * FPS + 1, f1 - 4 * FPS)
        seg = c[a:b, q]
        if len(seg) == 0:
            continue
        k = int(seg.argmin())
        low = int(seg[k])
        if low > 6:
            continue
        after = c[a + k:f1, q]
        peak = int(after.max())
        if peak >= max(15, 3 * max(low, 1)):
            tl = (a + k) / FPS
            tp = (a + k + int(after.argmax())) / FPS
            sc = 6 + min(peak, 40) * 0.1 - abs(tl - 24) * 0.25 - max(0.0, tp - 37) * 0.5
            if best is None or sc > best[0]:
                best = (sc, q, low, peak, tl, tp)
    if best is None:
        return None
    sc = 10.0 + best[0]
    s.update(dying=rps.NAMES[best[1]], low=best[2], peak=best[3], t_low=round(best[4], 1), t_peak=round(best[5], 1))
    sc -= abs(t1 - 37.0) * 0.6 + abs(tw - 50.0) * 0.6
    # stragglers: the doomed team's last 3 catches
    doomed = rps.prey_of(rec['winner'])
    late = [t for t, h, cc, nt, ot in rec['catches'] if ot == doomed and t >= t1]
    if len(late) >= 3:
        spread = late[-1] - late[-3]
        sc += min(spread, 6) * 0.6
        s['tail'] = round(spread, 1)
    # lead changes (0.5 s samples, 3-piece hysteresis) before the wipe-out
    lead, ch = int(np.argmax(c[3 * FPS])), 0
    for f in range(3 * FPS, f1, 15):
        m = int(np.argmax(c[f]))
        if m != lead and c[f, m] >= c[f, lead] + 3:
            lead, ch = m, ch + 1
    sc += min(ch, 6) * 0.4
    s['lead_changes'] = ch
    s['score'] = round(sc, 2)
    return s


def job(seed):
    rec = rps.run(seed, E.PARAMS, record=False)
    c = rps.counts(rec)
    raw = dict(outs=[(round(t, 2), q) for t, q in rec['outs']], tw=rec['t_win'], win=rec['winner'],
               c=c[::15].tolist())                     # counts every 0.5 s: enough to re-score without re-running
    return seed, story(rec), raw


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('a', type=int)
    ap.add_argument('b', type=int)
    ap.add_argument('--stop', type=int, default=0)
    ap.add_argument('--procs', type=int, default=os.cpu_count() or 2)
    a = ap.parse_args()
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    fits, n = [], 0
    with Pool(a.procs) as P:
        rawf = open(os.path.join(HERE, 'check', f'raw_{a.a}_{a.b}.jsonl'), 'a')
        for seed, s, raw in P.imap_unordered(job, range(a.a, a.b)):
            n += 1
            rawf.write(json.dumps(dict(seed=seed, **raw)) + '\n')
            rawf.flush()
            if s:
                fits.append((s['score'], seed, s))
                print(f'seed {seed}: {s}', flush=True)
                if a.stop and len(fits) >= a.stop:
                    P.terminate()
                    break
    fits.sort(key=lambda x: -x[0])
    with open(os.path.join(HERE, 'check', 'seeds.txt'), 'w') as fh:
        fh.write(f'# {n} seeds run, {len(fits)} fit\n')
        for sc, seed, s in fits:
            fh.write(f'{seed}\t{s}\n')
    print(f'{n} seeds run, {len(fits)} fit; winners:', {w: sum(1 for *_, s in fits if s["win"] == w) for w in rps.NAMES})
