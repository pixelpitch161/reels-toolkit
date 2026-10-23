"""Run many honest seeds of the episode and rank them by how well their story fits the beat sheet.
Nothing is edited: the chosen seed is played exactly as simulated.

    python seed_search.py 0 300            # seeds 0..299 on all cores -> ranked list + check/seeds.txt

Shape  : knockouts ~11 / 21 / 30 s after the bell (31 / 41 / 50 s in a 20 s-bell beat sheet), spaced >= 6 s.
Story  : the bell leader falls (early leader out first or second), the winner was an underdog at the bell,
         lead changes in the fight, every team visible at the bell (>= 12 %)."""
import os
import sys
import numpy as np
from multiprocessing import Pool

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import episode as E

TARGET = (E.T_FIGHT + 11.0, E.T_FIGHT + 21.0, E.T_FIGHT + 30.0)     # knockouts ~11 / 21 / 30 s after the bell


def story(rec):
    O = E.OFFSET
    nt = rec['ntiles']
    ft = rec['frame_times']
    cnt = rec['count'] / nt
    outs = [(t - O, q) for t, q in rec['outs']]
    fb = int(np.searchsorted(ft, E.T_FIGHT)) - 1
    bell = cnt[fb]
    order = list(np.argsort(-bell))
    leader = order[0]
    # lead changes in the fight (sampled every 0.5 s, 1 % hysteresis)
    lead, changes = leader, 0
    leaders = [leader]
    for f in range(fb, len(ft), 15):
        c = cnt[f]
        m = int(np.argmax(c))
        if m != lead and c[m] > c[lead] + 0.01:
            lead = m
            changes += 1
            leaders.append(m)
        if rec['t_win'] is not None and ft[f] > rec['t_win']:
            break
    win = rec['winner']
    ts = [t for t, q in outs]
    b0 = E.T_FIGHT - E.OFFSET                       # the bell in video time
    shape = len(ts) == 3 and b0 + 7 <= ts[0] <= b0 + 15 and b0 + 17 <= ts[1] <= b0 + 25 and b0 + 26 <= ts[2] <= b0 + 33 and \
        ts[1] - ts[0] >= 6 and ts[2] - ts[1] >= 5
    s = 0.0
    if shape:
        s += 10 - sum(abs(a - b) for a, b in zip(ts, TARGET)) * 0.5
        out_order = [q for t, q in outs]
        if leader in out_order[:2]:
            s += 3                                   # the early leader falls
        s += order.index(win) * 1.5                  # underdog winner (rank at the bell)
        s += min(changes, 4) * 1.0                   # lead changes
        s += 2 if bell.min() >= 0.12 else -3         # everyone visible at the bell
        # a real final duel: both finalists still big one second after the second knockout
        f2 = min(len(ft) - 1, int(np.searchsorted(ft, ts[1] + O + 1.0)))
        alive = [q for q in range(4) if q not in out_order[:2]]
        duel = min(cnt[f2][q] for q in alive)
        s += 4 * min(1.0, duel / 0.3)
        # no long lingering: seconds any team spends under 6 % before its knockout
        linger = 0.0
        for t, q in outs:
            fo = int(np.searchsorted(ft, t + O))
            linger = max(linger, (cnt[fb:fo, q] < 0.06).sum() / 30.0)
        s -= max(0.0, linger - 2.5) * 1.5
    else:
        duel, linger = 0.0, 0.0
    return dict(score=s, shape=shape, outs=outs, win=win, bell=bell, leader=leader, changes=changes, leaders=leaders,
                duel=duel, linger=linger)


def one(seed):
    rec = E.make(seed).run(60.0)
    st = story(rec)
    st['seed'] = seed
    return st


if __name__ == '__main__':
    a = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    b = int(sys.argv[2]) if len(sys.argv) > 2 else 100
    with Pool(os.cpu_count() or 2) as P:
        res = P.map(one, range(a, b), chunksize=2)
    names = [n[:2] for n in ('SAND', 'BALL', 'RIPPLE', 'VINE')]
    wins = np.bincount([r['win'] for r in res if r['win'] is not None], minlength=4)
    lines = []
    for r in sorted(res, key=lambda r: -r['score'])[:15]:
        lines.append(f"seed {r['seed']:4d} score {r['score']:5.1f}  bell " + ' '.join(f'{n}{100 * x:3.0f}' for n, x in zip(names, r['bell'])) +
                     '  outs ' + ' '.join(f"{names[q]}@{t:4.1f}" for t, q in r['outs']) +
                     f"  win {names[r['win']] if r['win'] is not None else '-'}  lead changes {r['changes']} (" +
                     '>'.join(names[q] for q in r['leaders']) + f")  duel {100 * r['duel']:.0f}%  linger {r['linger']:.1f}s")
    lines.append(f"{sum(r['shape'] for r in res)}/{len(res)} fit the shape; wins " + ' '.join(f'{n} {w}' for n, w in zip(names, wins)) +
                 f"; undecided {sum(r['win'] is None for r in res)}")
    print('\n'.join(lines))
    os.makedirs('check', exist_ok=True)
    with open(f'check/seeds_{a}_{b}.txt', 'w') as f:
        f.write('\n'.join(lines) + '\n')
