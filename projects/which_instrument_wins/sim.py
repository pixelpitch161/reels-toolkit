"""Honest 2D physics for #26: 16 equal balls in a rotating ring with one widening gap.

Ring frame = the plane of the ring (x right, y up, origin at the ring centre). Gravity pulls -y. The wall is an arc
of radius R_RING (inner surface) and thickness RING_T, missing over the gap; the gap's two end caps are segments
(they sweep balls: near-misses). A ball whose centre clears the outer surface inside the gap is OUT: it keeps
falling (and drifts toward the camera, so it passes in front of the ring) until it lands in the pit.

    from sim import run_seed
    R = run_seed(1889)          # dict of per-frame arrays + event list; cached to sim_<seed>.npz by cache_seed()

numba makes a run ~1 s; without it the same code runs in pure Python (slow, but only the seed search needs many runs).
"""
import math
import os
import numpy as np
import timeline as T

try:
    from numba import njit
except ImportError:                       # pure-Python fallback
    def njit(*a, **k):
        if a and callable(a[0]):
            return a[0]
        return lambda f: f

N = 16
H_PIT = 4.0              # ring centre height above the pit floor
Z_DRIFT = 2.2            # toward-camera speed of an eliminated ball (units/s)
MAX_EV = 40000
EV_WALL, EV_BALL, EV_EDGE, EV_OUT, EV_LAND = 0, 1, 2, 3, 4


@njit(cache=True)
def _gap_w(t, open0, open1, w0, w1, t1):
    if t <= open0:
        return 0.0
    if t < open1:
        u = (t - open0) / (open1 - open0)
        return w0 * u * u * (3 - 2 * u)
    if t < t1:
        return w0 + (w1 - w0) * (t - open1) / (t1 - open1)
    return w1


@njit(cache=True)
def _wrap(a):
    return (a + math.pi) % (2 * math.pi) - math.pi


@njit(cache=True)
def _sim(p0, theta0, spin, nf, sub, dt, R, TH, r, g, e_w, e_b, vcap, grip, open0, open1, w0, w1, t1, h_pit, zdrift,
         t_max):
    P = p0.copy()
    V = np.zeros((N, 2))
    Z = np.zeros(N)
    VZ = np.zeros(N)
    state = np.zeros(N, np.int64)        # 0 in play, 1 out (falling), 2 landed
    fP = np.zeros((nf, N, 2))
    fV = np.zeros((nf, N, 2))
    fZ = np.zeros((nf, N))
    fS = np.zeros((nf, N), np.int64)
    fTh = np.zeros(nf)
    fW = np.zeros(nf)
    ev = np.zeros((MAX_EV, 7))           # t, ball, kind, speed, x, y, other
    ne = 0
    t_out = np.full(N, -1.0)
    t_land = np.full(N, -1.0)
    n_alive = N
    t_win = -1.0
    last_f = nf
    for f in range(nf):
        for k in range(sub):
            t = (f * sub + k) * dt
            th = theta0 + spin * t
            w = _gap_w(t, open0, open1, w0, w1, t1)
            if t_win >= 0:                       # rule: the ring seals the moment one ball is left
                w = 0.0
            # integrate
            for i in range(N):
                if state[i] == 2:
                    continue
                V[i, 1] -= g * dt
                P[i, 0] += V[i, 0] * dt
                P[i, 1] += V[i, 1] * dt
                if state[i] == 1:
                    Z[i] += VZ[i] * dt
                    if P[i, 1] - r <= -h_pit:
                        P[i, 1] = -h_pit + r
                        state[i] = 2
                        t_land[i] = t
                        if ne < MAX_EV:
                            ev[ne, 0] = t; ev[ne, 1] = i; ev[ne, 2] = 4; ev[ne, 3] = -V[i, 1]
                            ev[ne, 4] = P[i, 0]; ev[ne, 5] = P[i, 1]; ev[ne, 6] = -1
                            ne += 1
                        V[i, 0] = 0.0; V[i, 1] = 0.0
            # ball-ball (in play only)
            for i in range(N):
                if state[i] != 0:
                    continue
                for j in range(i + 1, N):
                    if state[j] != 0:
                        continue
                    dx = P[j, 0] - P[i, 0]; dy = P[j, 1] - P[i, 1]
                    d2 = dx * dx + dy * dy
                    if d2 < 4 * r * r and d2 > 1e-12:
                        d = math.sqrt(d2)
                        nx = dx / d; ny = dy / d
                        ov = 2 * r - d
                        P[i, 0] -= nx * ov / 2; P[i, 1] -= ny * ov / 2
                        P[j, 0] += nx * ov / 2; P[j, 1] += ny * ov / 2
                        vr = (V[j, 0] - V[i, 0]) * nx + (V[j, 1] - V[i, 1]) * ny
                        if vr < 0:
                            jimp = -(1 + e_b) * vr / 2
                            V[i, 0] -= jimp * nx; V[i, 1] -= jimp * ny
                            V[j, 0] += jimp * nx; V[j, 1] += jimp * ny
                            if -vr > 0.6 and ne < MAX_EV:
                                ev[ne, 0] = t; ev[ne, 1] = i; ev[ne, 2] = 1; ev[ne, 3] = -vr
                                ev[ne, 4] = P[i, 0] + nx * r; ev[ne, 5] = P[i, 1] + ny * r; ev[ne, 6] = j
                                ne += 1
            # wall, end caps, exits
            for i in range(N):
                if state[i] != 0:
                    continue
                px = P[i, 0]; py = P[i, 1]
                d = math.sqrt(px * px + py * py)
                if d < R - r - 1e-9:
                    continue
                phi = math.atan2(py, px)
                dl = _wrap(phi - th)
                ingap = abs(dl) < w / 2
                if not ingap:
                    nx = px / d; ny = py / d                     # outward
                    if d < R + TH / 2:                            # inner surface
                        cx = nx * R; cy = ny * R
                        wvx = -spin * cy; wvy = spin * cx
                        rvx = V[i, 0] - wvx; rvy = V[i, 1] - wvy
                        vn = rvx * nx + rvy * ny                  # > 0 = moving outward
                        if vn > 0:
                            tx = -ny; ty = nx
                            vt = rvx * tx + rvy * ty
                            vn2 = -e_w * vn
                            vt2 = vt * (1 - grip)
                            V[i, 0] = vn2 * nx + vt2 * tx + wvx
                            V[i, 1] = vn2 * ny + vt2 * ty + wvy
                            if vn > 0.5 and ne < MAX_EV:
                                ev[ne, 0] = t; ev[ne, 1] = i; ev[ne, 2] = 0; ev[ne, 3] = vn
                                ev[ne, 4] = cx; ev[ne, 5] = cy; ev[ne, 6] = -1
                                ne += 1
                        P[i, 0] = nx * (R - r); P[i, 1] = ny * (R - r)
                    else:                                         # swept onto the outer surface by the turning wall
                        if ne < MAX_EV:
                            ev[ne, 0] = t; ev[ne, 1] = i; ev[ne, 2] = 5; ev[ne, 3] = d
                            ev[ne, 4] = px; ev[ne, 5] = py; ev[ne, 6] = -1
                            ne += 1
                        P[i, 0] = nx * (R + TH + r + 1e-3); P[i, 1] = ny * (R + TH + r + 1e-3)
                        vn = V[i, 0] * nx + V[i, 1] * ny
                        if vn < 0:
                            V[i, 0] -= 2 * vn * nx; V[i, 1] -= 2 * vn * ny
                # end caps (segments at the gap edges)
                for sgn in (-1.0, 1.0):
                    if w <= 0:
                        break
                    ae = th + sgn * w / 2
                    ux = math.cos(ae); uy = math.sin(ae)
                    ax = ux * R; ay = uy * R
                    s = (P[i, 0] - ax) * ux + (P[i, 1] - ay) * uy
                    if s < 0:
                        s = 0.0
                    if s > TH:
                        s = TH
                    qx = ax + ux * s; qy = ay + uy * s
                    ex = P[i, 0] - qx; ey = P[i, 1] - qy
                    dd = math.sqrt(ex * ex + ey * ey)
                    if dd < r and dd > 1e-9:
                        nx = ex / dd; ny = ey / dd
                        wvx = -spin * qy; wvy = spin * qx
                        rvx = V[i, 0] - wvx; rvy = V[i, 1] - wvy
                        vn = rvx * nx + rvy * ny
                        P[i, 0] = qx + nx * r; P[i, 1] = qy + ny * r
                        if vn < 0:
                            V[i, 0] = rvx - (1 + e_w) * vn * nx + wvx
                            V[i, 1] = rvy - (1 + e_w) * vn * ny + wvy
                            if ne < MAX_EV:
                                ev[ne, 0] = t; ev[ne, 1] = i; ev[ne, 2] = 2; ev[ne, 3] = -vn
                                ev[ne, 4] = qx; ev[ne, 5] = qy; ev[ne, 6] = sgn
                                ne += 1
                # out?
                px = P[i, 0]; py = P[i, 1]
                d = math.sqrt(px * px + py * py)
                if d > R + TH + r:
                    state[i] = 1
                    VZ[i] = zdrift
                    t_out[i] = t
                    n_alive -= 1
                    if ne < MAX_EV:
                        ev[ne, 0] = t; ev[ne, 1] = i; ev[ne, 2] = 3; ev[ne, 3] = math.sqrt(V[i, 0] ** 2 + V[i, 1] ** 2)
                        ev[ne, 4] = px; ev[ne, 5] = py; ev[ne, 6] = n_alive
                        ne += 1
                    if n_alive == 1 and t_win < 0:
                        t_win = t
            # speed cap
            for i in range(N):
                if state[i] == 0:
                    sp = math.sqrt(V[i, 0] ** 2 + V[i, 1] ** 2)
                    if sp > vcap:
                        V[i, 0] *= vcap / sp; V[i, 1] *= vcap / sp
        tf = (f + 1) * sub * dt
        fP[f] = P; fV[f] = V; fZ[f] = Z; fS[f] = state
        fTh[f] = theta0 + spin * tf
        fW[f] = 0.0 if t_win >= 0 else _gap_w(tf, open0, open1, w0, w1, t1)
        if t_win > 0 and tf > t_win + 2.5:
            last_f = f + 1
            break
        if tf > t_max:
            last_f = f + 1
            break
    return fP[:last_f], fV[:last_f], fZ[:last_f], fS[:last_f], fTh[:last_f], fW[:last_f], ev[:ne], t_out, t_land, t_win


def formation(seed):
    """4 x 4 starting grid at the top of the ring (roster order: row-major), with a small seeded jitter"""
    rng = np.random.default_rng(seed)
    s = T.FORMATION_STEP
    P = np.array([[(c - 1.5) * s, T.FORMATION_Y + (1.5 - r) * s] for r in range(4) for c in range(4)])
    P += rng.uniform(-0.045, 0.045, P.shape)
    theta0 = rng.uniform(-math.pi, math.pi)
    return P, theta0


def run_seed(seed, t_max=58.0, **ov):
    """ov: rule overrides for experiments (spin, grav, e_wall, e_ball, vcap, grip, w0, w1, t1)"""
    p0, th0 = formation(seed)
    sub = int(round(1 / (T.SIM_DT * T.FPS)))
    nf = int(t_max * T.FPS) + 2
    g = lambda k, d: ov.get(k, d)
    out = _sim(p0, th0, g('spin', T.SPIN), nf, sub, T.SIM_DT, T.R_RING, T.RING_T, T.R_BALL, g('grav', T.GRAV),
               g('e_wall', T.E_WALL), g('e_ball', T.E_BALL), g('vcap', T.V_CAP), g('grip', T.WALL_GRIP),
               T.GAP_OPEN[0], T.GAP_OPEN[1], g('w0', T.GAP_W0), g('w1', T.GAP_W1), g('t1', T.GAP_T1), H_PIT, Z_DRIFT,
               t_max)
    fP, fV, fZ, fS, fTh, fW, ev, t_out, t_land, t_win = out
    return dict(seed=seed, P=fP, V=fV, Z=fZ, S=fS, TH=fTh, GW=fW, EV=ev, T_OUT=t_out, T_LAND=t_land,
                T_WIN=float(t_win), P0=p0, TH0=th0)


def story(R):
    """pacing facts the seed search scores"""
    tout = R['T_OUT']
    alive_end = int((tout < 0).sum())
    order = np.argsort(np.where(tout < 0, 1e9, tout))
    times = np.sort(tout[tout >= 0])
    winner = int(order[-1]) if alive_end == 1 else -1
    ev = R['EV']
    edge = ev[ev[:, 2] == 2]
    return dict(n_out=len(times), times=times, winner=winner, order=order[:len(times)],
                first=times[0] if len(times) else 99, last=R['T_WIN'],
                n_edge=len(edge))


def cache_seed(seed, path=None):
    path = path or os.path.join(os.path.dirname(os.path.abspath(__file__)), f'sim_{seed}.npz')
    R = run_seed(seed)
    np.savez_compressed(path, **{k: np.asarray(v) for k, v in R.items()})
    return R


def load(seed=None):
    here = os.path.dirname(os.path.abspath(__file__))
    if seed is None:
        seed = int(open(os.path.join(here, 'seed.txt')).read().split()[0])
    path = os.path.join(here, f'sim_{seed}.npz')
    if not os.path.exists(path):
        return cache_seed(seed, path)
    z = np.load(path)
    R = {k: z[k] for k in z.files}
    R['seed'] = int(R['seed']); R['T_WIN'] = float(R['T_WIN']); R['TH0'] = float(R['TH0'])
    return R


if __name__ == '__main__':
    import sys
    import time
    seeds = [int(a) for a in sys.argv[1:]] or [1, 2, 3]
    for s in seeds:
        t0 = time.time()
        R = run_seed(s)
        st = story(R)
        from pp_bridge import NAMES
        print(f'seed {s}: {time.time() - t0:.1f}s  out {st["n_out"]}  first {st["first"]:.1f}  win {st["last"]:.1f}  '
              f'edges {st["n_edge"]}  winner {NAMES[st["winner"]] if st["winner"] >= 0 else "-"}')
        print('   ', ' '.join(f'{x:.1f}' for x in st['times']))
