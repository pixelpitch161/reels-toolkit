"""Ball path: every melody note is one wall hit, planned so it looks like a ball rattling round the rings.

Per ring, a dynamic programme over wall angles (0.5 deg grid anchored on the escape angle) picks one hit angle
per note:
- the ball always circulates one way round the ring (alternating per ring) in long chords: ~138 deg for
  eighths, ~158 deg for quarters, ~172 deg for longer notes, so each crossing reads clearly;
- every hit is clear of the moving gap (the gap's angle at that note's time), except the planned near-misses,
  which skim the gap edge;
- the last chord ends exactly in the gap centre at the escape note's time (the ring's own rotation decides
  where that is), crossing the wall steeply, and the continuation into the next ring must not land in that
  ring's gap.
Between hits the ball moves in straight lines at constant speed (speed = chord / note gap, so it speeds up with
the tempo); after an escape it bursts out and slows towards its first hit on the next ring. The scripted tail
(up the light shaft, off the giant ring, back down through the aligned gaps) is a cubic spline.
Not a physics simulation: the bounces are timed to the score (stated nowhere as real physics).

    python path.py      # solves, caches path.npz, prints checks"""
import math
import os
import numpy as np
import timeline as T
import geom as G

HERE = os.path.dirname(os.path.abspath(__file__))
CACHE = os.path.join(HERE, 'path.npz')
VERSION = 3
D = math.radians(0.5)
NG = 720


def target_deg(beats):
    return 138.0 if beats <= 0.51 else (158.0 if beats <= 1.01 else 172.0)


def solve_ring(k, th0, idx, esc_i, next_first=None):
    """idx: note indices of the bounces on ring k (idx[0] = first hit at th0); esc_i: escape note index.
    Returns the hit angles for idx (th0 first)."""
    m_e = T.NOTES[esc_i]['m']
    alpha = G.phase(k, m_e)
    grid = alpha + np.arange(NG) * D
    dr = G.DIR[k]
    rho, gap, rb = G.RHO[k], G.GAP[k], G.RB[k]
    clear = gap + rb / rho + 0.06
    lo, hi = math.radians(100), math.radians(178)

    def allowed(i):
        ph = G.phase(k, T.NOTES[i]['m'])
        dist = np.abs((grid - ph + math.pi) % (2 * math.pi) - math.pi)
        if T.NOTES[i].get('near'):
            e = gap + rb / rho
            return (dist >= e + 0.006) & (dist <= e + 0.03)
        return dist >= clear

    def fwd(a_from, a_to):
        return (dr * (a_to - a_from)) % (2 * math.pi)

    n = len(idx)
    INF = 1e18
    back = []
    # step 1: exact th0 -> grid
    beats = T.NOTES[idx[1]]['b'] - T.NOTES[idx[0]]['b'] if n > 1 else T.NOTES[esc_i]['b'] - T.NOTES[idx[0]]['b']
    if n > 1:
        dl = fwd(th0, grid)
        tg = math.radians(target_deg(beats))
        cost = np.where((dl >= lo) & (dl <= hi) & allowed(idx[1]), (dl - tg) ** 2, INF)
        for s in range(2, n):
            beats = T.NOTES[idx[s]]['b'] - T.NOTES[idx[s - 1]]['b']
            tg = math.radians(target_deg(beats))
            best = np.full(NG, INF)
            arg = np.zeros(NG, int)
            for o in range(int(math.ceil(lo / D)), int(hi / D) + 1):
                cand = np.roll(cost, dr * o) + (o * D - tg) ** 2
                better = cand < best
                best[better] = cand[better]
                arg[better] = o
            best[~allowed(idx[s])] = INF
            back.append(arg)
            cost = best
    else:
        cost = np.zeros(NG)
    # final chord into the gap centre (grid index 0 = alpha)
    final = T.NOTES[esc_i]['kind'] == 'final'
    elo, ehi = (math.radians(166), math.radians(179)) if final else (math.radians(105), math.radians(178))
    etg = math.radians(176 if final else 150)
    thn = grid if n > 1 else np.array([th0])
    dl = fwd(thn, alpha)
    ec = np.where((dl >= elo) & (dl <= ehi), (dl - etg) ** 2, INF)
    if next_first is not None:                                   # continuation must not land in ring k+1's gap
        k2, m_first = next_first
        for j in range(len(thn)):
            if ec[j] >= INF:
                continue
            q = continuation(k, thn[j], alpha, k2)
            if G.angdist(q, G.phase(k2, m_first)) < G.GAP[k2] + G.RB[k2] / G.RHO[k2] + 0.08:
                ec[j] = INF
    tot = (cost if n > 1 else np.zeros(1)) + ec
    j = int(np.argmin(tot))
    assert tot[j] < INF, f'ring {k}: no solution'
    if n == 1:
        return [th0], alpha
    ang = [grid[j]]
    for arg in reversed(back):
        o = arg[j]
        j = (j - dr * o) % NG
        ang.append(grid[j])
    ang.append(th0)
    return ang[::-1], alpha


def continuation(k, th_n, alpha, k2):
    """angle where the escape line (hit on ring k at th_n -> gap centre) meets ring k2's hit circle"""
    p = np.array([math.cos(th_n), math.sin(th_n)]) * G.RHO[k]
    g = np.array([math.cos(alpha), math.sin(alpha)]) * G.R[k]
    d = (g - p) / np.linalg.norm(g - p)
    r = G.RHO[k2]
    b = 2 * (g @ d)
    c = g @ g - r * r
    s = (-b + math.sqrt(b * b - 4 * c)) / 2
    q = g + s * d
    return math.atan2(q[1], q[0])


def solve():
    N = T.N_NOTES
    X = np.zeros((N, 3))                 # ball centre at each note onset
    ANG = np.full(N, np.nan)
    th0 = -math.pi / 2
    for k in range(1, T.NRINGS + 1):
        idx = [i for i in range(N) if T.NOTES[i]['ring'] == k and T.NOTES[i]['kind'] == 'bounce']
        esc = [i for i in range(N) if T.NOTES[i]['ring'] == k and T.NOTES[i]['kind'] != 'bounce'][0]
        nf = (k + 1, T.NOTES[esc + 1]['m']) if k < T.NRINGS else None
        ang, alpha = solve_ring(k, th0, idx, esc, nf)
        for i, a in zip(idx, ang):
            ANG[i] = a
            X[i] = G.pol(G.RHO[k], a)
        ANG[esc] = alpha
        X[esc] = G.pol(G.R[k], alpha)
        if k < T.NRINGS:
            th0 = continuation(k, ang[-1], alpha, k + 1)
    return X, ANG


def load():
    if os.path.exists(CACHE):
        d = np.load(CACHE)
        if int(d['version']) == VERSION and len(d['X']) == T.N_NOTES:
            return d['X'], d['ANG']
    X, ANG = solve()
    np.savez(CACHE, X=X, ANG=ANG, version=VERSION)
    return X, ANG


X, ANG = load()
KIND = [n['kind'] for n in T.NOTES]

# ------------------------------------------------------------------ the scripted tail
CRACK = np.array([-0.45, 9.3, -1.7])
SUMMIT = np.array([-1.1, 16.7, -3.3])
GIANT_C = np.array([0.0, 4.5, 0.0])
GIANT_R = 15.5
GIANT_TOP = GIANT_C + np.array([-1.7, math.sqrt(GIANT_R ** 2 - 1.7 ** 2) - 0.55, 0.0])


def _hermite(p0, v0, p1, v1, u, T_):
    h00 = 2 * u ** 3 - 3 * u ** 2 + 1
    h10 = u ** 3 - 2 * u ** 2 + u
    h01 = -2 * u ** 3 + 3 * u ** 2
    h11 = u ** 3 - u ** 2
    return h00 * p0 + h10 * T_ * v0 + h01 * p1 + h11 * T_ * v1


_iF = T.N_NOTES - 1
_v_final = (X[_iF] - X[_iF - 1]) / (T.NOTES[_iF]['m'] - T.NOTES[_iF - 1]['m'])
_up = np.array([0, 1.0, 0])
TAIL = [  # (video t, position, velocity)
    (T.T_FINAL, X[_iF], _v_final * 0.9),
    (T.T_FINAL + 0.42, CRACK, np.array([-0.9, 6.5, -2.6])),
    (T.T_SUMMIT, SUMMIT, np.array([-0.5, 6.5, 2.0])),
    (T.T_BONG, GIANT_TOP, np.array([-0.4, 3.0, 1.6])),
    (T.T_BONG + 0.001, GIANT_TOP, np.array([-0.2, -4.0, -1.6])),
    (T.T_REENTER - 0.75, SUMMIT + [0, 0.6, 0], np.array([0.4, -9.0, 1.0])),
    (T.T_REENTER, CRACK, np.array([0.6, -6.0, 2.2])),
    (G.M_TOP + T.DUR, np.array([0.0, G.Y_TOP, 0.0]), np.array([0, -G.V0_DROP, 0.0])),
]


def ball(t):
    """ball centre (3D) and radius at video time t"""
    m = T.mtime(t)
    if T.T_FINAL <= t < G.M_TOP + T.DUR:
        for (t0, p0, v0), (t1, p1, v1) in zip(TAIL, TAIL[1:]):
            if t0 <= t < t1:
                u = (t - t0) / (t1 - t0)
                return _hermite(p0, v0, p1, v1, u, t1 - t0), G.RB[T.NRINGS + 1]
    if m < T.M0:
        return np.array([0.0, G.drop_y(m), 0.0]), G.RB[1]
    i = int(np.searchsorted(T.NOTE_M, m, 'right') - 1)
    i = min(max(i, 0), T.N_NOTES - 2)
    n0, n1 = T.NOTES[i], T.NOTES[i + 1]
    u = (m - n0['m']) / (n1['m'] - n0['m'])
    k = n0['ring']
    if KIND[i] == 'escape':
        s = u + 0.6 * u * (1 - u)                     # burst out, slowing into the first hit on ring k+1
        rb = G.RB[k] + (G.RB[k + 1] - G.RB[k]) * min(1.0, u * 1.5)
        return X[i] + (X[i + 1] - X[i]) * s, rb
    return X[i] + (X[i + 1] - X[i]) * u, G.RB[k]


def speed(t, dt=1 / 240):
    return float(np.linalg.norm(ball(t + dt)[0] - ball(t - dt)[0]) / (2 * dt))


if __name__ == '__main__':
    if os.path.exists(CACHE):
        os.remove(CACHE)
    X, ANG = load()
    worst = 9
    for i, n in enumerate(T.NOTES):
        if n['kind'] == 'bounce':
            k = n['ring']
            d = G.angdist(ANG[i], G.phase(k, n['m'])) - G.GAP[k] - G.RB[k] / G.RHO[k]
            if not n.get('near'):
                worst = min(worst, d)
            else:
                print(f'near-miss at {n["m"]:6.2f} s ring {k}: clearance {d:.3f} rad ({d * G.RHO[k] * 100:.1f} cm)')
    print(f'smallest clearance on normal hits: {worst:.3f} rad')
    ch = [math.degrees((G.DIR[n['ring']] * (ANG[i + 1] - ANG[i])) % (2 * math.pi)) for i, n in enumerate(T.NOTES[:-1])
          if n['kind'] == 'bounce' and T.NOTES[i + 1]['ring'] == n['ring']]
    print(f'chord angles {min(ch):.0f}..{max(ch):.0f} deg (mean {np.mean(ch):.0f})')
    sp = [speed(t) for t in np.arange(0.0, 52.0, 0.05)]
    for a in range(0, 52, 4):
        s = sp[a * 20:(a + 4) * 20]
        print(f'{a:2d}-{a + 4:2d}s ball speed median {np.median(s):5.1f} u/s  ring {T.ring_at(a + 2.0)}')
    print('ring 12 last hit angle', round(math.degrees(ANG[-2]), 1), 'exit gap', round(math.degrees(ANG[-1]), 1))
    for k in (1, 5, 12):
        print(f'ring {k}: gap half-width {math.degrees(G.GAP[k]):.1f} deg, omega {G.OMEGA[k]:.3f} rad/beat, align m {G.M_ALIGN[k]:.3f}')
