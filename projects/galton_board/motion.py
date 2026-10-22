"""Ball positions, colours and events at any time t, evaluated from the cached simulation (balls.npz).
Used by render.py (picture) and audio.py (the same contact/landing events)."""
import os
import math
import numpy as np
import board as B
import timeline as T

HERE = os.path.dirname(os.path.abspath(__file__))
D = dict(np.load(os.path.join(HERE, 'balls.npz')))
N = len(D['run'])
RUN, KIND = D['run'], D['kind']
TC, HH, CC, JX = D['tc'], D['h'], D['c'].astype(int), D['jx']
BIN, SLOT, XS, YS, ULAND, TLAND = D['bin'].astype(int), D['slot'], D['xs'], D['ys'], D['u_land'], D['t_land']
XC = CC * (B.DX / 2)                                    # (N, 13) column x positions
XC[:, :12] += JX
REL = TC[:, 0] - HH                                     # leaves the outlet
MAIN = np.where(RUN == 0)[0]
TOR = np.where(RUN == 1)[0]
GOLD = int(np.where(RUN == 2)[0][0])
HERO, SECOND = 0, 1
G_DRAIN = 9.0
ROW_OF_SLOT = np.array([2 * (s // 7) + (1 if s % 7 >= 4 else 0) for s in SLOT])
DRAIN_T = T.T_DRAIN + 0.04 + 0.004 * ROW_OF_SLOT + 0.012 * ((np.arange(N) * 7919) % 5) / 5


def torrent_time(t):
    """effective time of the torrent balls, and a dim factor (0 = normal, 1 = frozen + dimmed).
    Returns (tau array or None, dim, visible-mask function)"""
    t = t % T.DUR
    if t >= T.T_POUR:
        return np.full(len(TOR), t - T.DUR), 0.0
    if t < T.T_FREEZE:
        return np.full(len(TOR), t), 0.0
    r0, r1 = T.T_REWIND
    dim = min(1.0, (t - T.T_FREEZE) / 0.3)
    if t < r0:
        return np.full(len(TOR), T.T_FREEZE), dim
    if t < r1:
        u = (t - r0) / (r1 - r0)
        u = u * u * (3 - 2 * u)
        return T.T_FREEZE - (T.T_FREEZE - REL[TOR] + 0.02) * u, dim
    return None, 1.0


def eval_balls(idx, tau):
    """positions (n,2), colour (n,3), landed flag, contacts made, for balls idx at their own times tau"""
    tc, xc = TC[idx], XC[idx]
    n = len(idx)
    K = (tau[:, None] >= tc).sum(1)
    x = np.zeros(n)
    y = np.zeros(n)
    landed = np.zeros(n, bool)
    pre = K == 0
    if pre.any():
        dt = np.maximum(tau[pre] - REL[idx][pre], 0)
        g = B.gravity(HH[idx][pre])
        y[pre] = B.Y_OUT - 0.5 * g * dt * dt
        x[pre] = xc[pre, 0] * np.clip(dt / HH[idx][pre], 0, 1)
    mid = (K >= 1) & (K <= 11)
    if mid.any():
        ii = np.where(mid)[0]
        k = K[ii] - 1
        ta, tb = tc[ii, k], tc[ii, k + 1]
        u = (tau[ii] - ta) / (tb - ta)
        x[ii] = xc[ii, k] + (xc[ii, k + 1] - xc[ii, k]) * u
        y[ii] = B.hop_y(B.Y_C[k], u)
    post = K == 12
    if post.any():
        ii = np.where(post)[0]
        gi = idx[ii]
        u = (tau[ii] - tc[ii, 11]) / HH[gi]
        xb = B.bin_x(BIN[gi])
        x1 = np.minimum(u, 1.0)
        xx = xc[ii, 11] + (xb - xc[ii, 11]) * x1
        ul = ULAND[gi]
        fall = u > 1
        v = np.clip((u - 1) / np.maximum(ul - 1, 1e-6), 0, 1)
        xx = np.where(fall, xb + (XS[gi] - xb) * v, xx)
        yy = B.hop_y(B.Y_C[11], np.minimum(u, ul))
        land = u >= ul
        # a small settle bounce after landing
        dl = (u - ul) * HH[gi]
        yy = np.where(land, YS[gi] + 0.35 * B.R_BALL * np.exp(-dl / 0.05) * np.abs(np.sin(dl * 60)) * (dl < 0.25), yy)
        x[ii], y[ii] = xx, yy
        landed[ii] = land
    # colour: after contact k the ball takes the colour of the side it bounced to
    kk = np.minimum(K, 12)
    cnext = CC[idx, np.minimum(kk, 12)]
    col = B.ball_color(np.where(K == 0, 0, cnext), kk)
    return np.stack([x, y], 1), col, landed, K


def state(t):
    """everything the renderer needs about the balls at time t.
    Returns dict: pos (n,2), col (n,3), dim (n,), alpha (n,), idx, landed (n,), special labels."""
    t = t % T.DUR
    P, C, DIM, A, IDX, LAND = [], [], [], [], [], []
    # main run
    vis = REL[MAIN] <= t
    if t >= T.T_DRAIN:
        vis &= ~((TLAND[MAIN] <= T.T_DRAIN) & (t > DRAIN_T[MAIN] + 0.5))
    idx = MAIN[vis]
    if len(idx):
        pos, col, landed, K = eval_balls(idx, np.full(len(idx), t))
        if t >= T.T_DRAIN:
            dr = landed & (t > DRAIN_T[idx])
            dt = t - DRAIN_T[idx][dr]
            pos[dr, 1] = YS[idx][dr] - 0.5 * G_DRAIN * dt * dt
            keep = (pos[:, 1] > B.Y_BF + 0.2 * B.R_BALL) | ~dr
            pos, col, landed, idx = pos[keep], col[keep], landed[keep], idx[keep]
        P.append(pos); C.append(col); DIM.append(np.zeros(len(idx))); A.append(np.ones(len(idx))); IDX.append(idx); LAND.append(landed)
    # staged golden ball
    if REL[GOLD] <= t < T.GOLD_FADE[1]:
        pos, col, landed, K = eval_balls(np.array([GOLD]), np.array([t]))
        a = 1 - max(0.0, min(1.0, (t - T.GOLD_FADE[0]) / (T.GOLD_FADE[1] - T.GOLD_FADE[0])))
        P.append(pos); C.append(np.array([B.GOLD])); DIM.append(np.zeros(1)); A.append(np.array([a])); IDX.append(np.array([GOLD])); LAND.append(landed)
    # torrent
    tau, dim = torrent_time(t)
    if tau is not None:
        vis = REL[TOR] <= tau
        idx = TOR[vis]
        if len(idx):
            pos, col, landed, K = eval_balls(idx, tau[vis])
            P.append(pos); C.append(col); DIM.append(np.full(len(idx), dim)); A.append(np.ones(len(idx))); IDX.append(idx); LAND.append(landed)
    if not P:
        z = np.zeros((0, 2))
        return dict(pos=z, col=np.zeros((0, 3)), dim=np.zeros(0), alpha=np.zeros(0), idx=np.zeros(0, int), landed=np.zeros(0, bool))
    return dict(pos=np.concatenate(P), col=np.concatenate(C), dim=np.concatenate(DIM), alpha=np.concatenate(A),
                idx=np.concatenate(IDX), landed=np.concatenate(LAND))


# ---------------------------------------------------------------- events (pings, landings), sorted per run
def _events():
    ev = []
    for i in range(N):
        for k in range(12):
            ev.append((TC[i, k], i, k))
    ev.sort()
    return np.array([e[0] for e in ev]), np.array([e[1] for e in ev]), np.array([e[2] for e in ev])


EV_T, EV_I, EV_K = _events()
EV_PEG = np.array([B.PEG_ID[(k, int(CC[i, k]))] for i, k in zip(EV_I, EV_K)])
EV_RUN = RUN[EV_I]


def peg_flash(t, decay=0.09, win=0.45):
    """(78,) flash intensity and (78,3) colour of every peg at time t"""
    t = t % T.DUR
    fl = np.zeros(len(B.PEGS))
    col = np.zeros((len(B.PEGS), 3))
    spans = [((EV_RUN == 0) | (EV_RUN == 2), t)]
    tau, dim = torrent_time(t)
    if tau is not None and dim == 0:
        spans.append((EV_RUN == 1, float(tau[0])))
    for m, te in spans:
        i0, i1 = np.searchsorted(EV_T, [te - win, te + 1e-9])
        sel = np.arange(i0, i1)
        sel = sel[m[sel]]
        if not len(sel):
            continue
        w = np.exp(-(te - EV_T[sel]) / decay)
        np.add.at(fl, EV_PEG[sel], w)
        bi = EV_I[sel]
        c = np.where((RUN[bi] == 2)[:, None], B.GOLD[None], B.ball_color(CC[bi, EV_K[sel] + 1], EV_K[sel] + 1))
        np.add.at(col, EV_PEG[sel], c * w[:, None])
    col = col / np.maximum(fl, 1e-6)[:, None]
    return np.minimum(fl, 1.6), col


def bin_counts(t):
    """(13,) balls sitting in each bin at time t (main run, or the torrent during the cold open)"""
    t = t % T.DUR
    out = np.zeros(B.NB, int)
    m = MAIN[(TLAND[MAIN] <= t)]
    if t >= T.T_DRAIN:
        m = m[t < DRAIN_T[m] + 0.12]
    np.add.at(out, BIN[m], 1)
    tau, dim = torrent_time(t)
    if tau is not None:
        sel = TOR[TLAND[TOR] <= tau]
        np.add.at(out, BIN[sel], 1)
    return out


def landed_total(t):
    return int(bin_counts(t).sum())


def bin_flash(t, decay=0.16):
    """(13,) landing flashes per bin (the 'equaliser')"""
    t = t % T.DUR
    out = np.zeros(B.NB)
    m = MAIN[(TLAND[MAIN] <= t) & (TLAND[MAIN] > t - 0.6)]
    np.add.at(out, BIN[m], np.exp(-(t - TLAND[m]) / decay))
    tau, dim = torrent_time(t)
    if tau is not None and dim == 0:
        te = float(tau[0])
        s = TOR[(TLAND[TOR] <= te) & (TLAND[TOR] > te - 0.6)]
        np.add.at(out, BIN[s], np.exp(-(te - TLAND[s]) / decay))
    return out
