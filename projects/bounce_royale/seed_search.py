"""Run many honest seeds and rank them by how well their story fits the beat sheet. Nothing is ever edited:
the chosen seed is played exactly as simulated (echo SEED > seed.txt && python sim.py).

    python seed_search.py 0 3000 --stop 5      # stop once 5 seeds fit the shape -> ranked list + check/seeds_A_B.txt

Shape : first pop ~27 s, second ~38 s, the final pop (the win) ~49 s, at least 6 s apart.
Fit   : that shape, plus a close duel (finalists within 16 tiles on average) and a colour down to 1-2 tiles between
        16 s and the first pop (the "1 SEGMENT LEFT!" beat).
Story : something changes every 4-5 s (lead changes, big swings, near-deaths, pops), a colour drops to 1-2 tiles
        before the first pop (and ideally claws back), a close duel with at least one lead change, the winner
        was not the leader when the ring started shrinking."""
import os
import sys
import numpy as np
from multiprocessing import Pool

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import royale as R

TARGET = (27.0, 38.0, 49.0)
FPS = R.FPS


def leaders(cnt, f0, f1, hyst=3):
    """leader changes between frames f0..f1 (a new leader must lead by hyst tiles)"""
    lead = int(np.argmax(cnt[f0]))
    seq, times = [lead], []
    for f in range(f0, f1):
        c = cnt[f]
        m = int(np.argmax(c))
        if m != lead and c[m] >= c[lead] + hyst:
            lead = m
            seq.append(m)
            times.append(f / FPS)
    return seq, times


def story(rec):
    cnt = rec['count'].astype(int)
    nf = len(cnt)
    pops = rec['pops']
    ts = [t for t, q, _ in pops]
    out = dict(score=-99.0, shape=False, pops=[(round(t, 1), q) for t, q, _ in pops], win=rec['winner'])
    if len(ts) != 3:
        return out
    shape = (23.5 <= ts[0] <= 31.0 and 34.5 <= ts[1] <= 42.0 and 45.0 <= ts[2] <= 53.0 and
             ts[1] - ts[0] >= 6 and ts[2] - ts[1] >= 6)
    if not shape:
        return out
    f_end = min(nf - 1, int(ts[2] * FPS))
    seq, ltimes = leaders(cnt, int(6 * FPS), f_end)
    # events that make "something change": lead changes, pops, near-deaths (<= 2 tiles), big swings (>= 5 tiles in 1.5 s)
    ev = list(ltimes) + ts + [16.0]
    low_t, comeback = None, 0
    for q in range(4):
        c = cnt[:, q]
        dead_f = int(next((t for t, qq, _ in pops if qq == q), 99) * FPS)
        f = int(8 * FPS)
        while f < min(dead_f, f_end):
            if c[f] <= 2:
                ev.append(f / FPS)
                if low_t is None or f / FPS < low_t:
                    if f / FPS < ts[0]:
                        low_t = f / FPS
                later = c[f:min(dead_f, f_end)]
                if later.max() >= 7:
                    comeback = max(comeback, int(later.max()))
                f += int(3 * FPS)
            else:
                f += 3
    sw = np.abs(cnt[int(1.5 * FPS):f_end] - cnt[:f_end - int(1.5 * FPS)]).max(1) if f_end > 60 else np.zeros(1)
    for f in np.nonzero(sw >= 5)[0][::15]:
        ev.append((f + 45) / FPS)
    ev = sorted(e for e in ev if 8.0 <= e <= ts[2])
    gaps = np.diff([8.0] + ev + [ts[2]])
    longest = float(gaps.max()) if len(gaps) else 99.0
    # the duel: both finalists, lead changes and how close it stays
    fin = [q for q in range(4) if q not in (pops[0][1], pops[1][1])]
    fa, fb = int((ts[1] + 1) * FPS), max(int((ts[1] + 1) * FPS) + 1, int((ts[2] - 2) * FPS))
    dseq, _ = leaders(cnt[:, :], fa, fb, hyst=2)
    duel_changes = len(dseq) - 1
    gap = np.abs(cnt[fa:fb, fin[0]] - cnt[fa:fb, fin[1]]).mean() if fb > fa else 48
    fav = int(np.argmax(cnt[int(16 * FPS)]))
    s = 10 - sum(abs(a - b) for a, b in zip(ts, TARGET)) * 0.5
    s += min(len(seq) - 1, 5) * 0.8                       # lead changes
    s -= max(0.0, longest - 5.0) * 1.2                    # long stretches with nothing changing
    s += 2.5 if low_t is not None and 19 <= low_t <= ts[0] else 0.0
    s += 2.0 if comeback >= 7 else 0.0
    s += min(duel_changes, 3) * 1.0
    s += 2.0 * max(0.0, 1 - gap / 20.0)
    s += 1.5 if rec['winner'] != fav else 0.0
    shape = shape and gap <= 16 and (low_t is not None and low_t >= 16)      # a close duel + a near-death before pop 1
    out.update(score=s, shape=shape, leads=seq, longest=longest, low_t=low_t, comeback=comeback,
               duel_changes=duel_changes, duel_gap=float(gap), fav=fav)
    return out


def one(seed):
    rec = R.Run(seed).run(stop_at_win=True)
    st = story(rec)
    st['seed'] = seed
    return st


if __name__ == '__main__':
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument('a', type=int, nargs='?', default=0)
    ap.add_argument('b', type=int, nargs='?', default=500)
    ap.add_argument('--stop', type=int, default=None, help='stop once this many seeds fit the shape')
    opt = ap.parse_args()
    res = []
    with Pool(os.cpu_count() or 2) as P:
        for st in P.imap_unordered(one, range(opt.a, opt.b), chunksize=4):
            res.append(st)
            if st['shape']:
                print(f"* seed {st['seed']:5d}  pops " + ' '.join(f"{R.NAMES[q][:2]}@{t:4.1f}" for t, q in st['pops']) +
                      f"  win {R.NAMES[st['win']]}  score {st['score']:.1f}", flush=True)
            if opt.stop is not None and sum(r['shape'] for r in res) >= opt.stop:
                P.terminate()
                break
    nm = [n[:2] for n in R.NAMES]
    lines = []
    for r in sorted([r for r in res if r['shape']], key=lambda r: -r['score'])[:12]:
        lines.append(f"seed {r['seed']:5d} score {r['score']:5.1f}  pops " + ' '.join(f"{nm[q]}@{t:4.1f}" for t, q in r['pops']) +
                     f"  win {nm[r['win']]}  fav@16s {nm[r['fav']]}  leads {'>'.join(nm[q] for q in r['leads'])}  longest quiet {r['longest']:.1f}s"
                     f"  low@{r['low_t'] if r['low_t'] is None else round(r['low_t'], 1)}  comeback {r['comeback']}"
                     f"  duel changes {r['duel_changes']} gap {r['duel_gap']:.1f}")
    wins = np.bincount([r['win'] for r in res if r['win'] is not None], minlength=4)
    lines.append(f"{sum(r['shape'] for r in res)}/{len(res)} fit the shape; wins (all runs) " + ' '.join(f'{n} {w}' for n, w in zip(nm, wins)) +
                 f"; undecided {sum(r['win'] is None for r in res)}")
    print('\n'.join(lines))
    os.makedirs('check', exist_ok=True)
    with open(f'check/seeds_{opt.a}_{opt.b}.txt', 'w') as f:
        f.write('\n'.join(lines) + '\n')
