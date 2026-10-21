"""Sand grains on a vibrating plate. Grains get bounced hardest where the
plate moves most and drift toward the nodal lines, where it stays still.
Writes sand_pos.f32 / sand_h.f16 (per frame) and drive.npz."""
import math
import numpy as np
from modes import mode_val_grad
import timeline as T

G = 12000
SUB = 8                        # sim substeps per frame
DT = 1.0 / (T.FPS * SUB)
GAMMA = 0.03                   # resonance width (fraction of f)
rng = np.random.default_rng(21)


def lorentz(f, f0):
    return 1.0 / math.sqrt(1.0 + ((f - f0) / (GAMMA * f0)) ** 2)


def drive(t):
    """-> (list of (mode name, coef)), V, mu, sigma, sigma0, hop_scale"""
    # hook
    for nm, t0, t1 in T.HOOK:
        if t0 <= t < t1:
            V = min(1.0, (t - t0) / 0.06)
            return [(nm, 1.0)], V, 0.11, 0.9, 0.06, 0.02
    # slow-motion demo
    if T.T_SLOW0 <= t < T.T_SLOW1:
        if t < T.T_SLOW_FLING:
            V = 0.22 * min(1, (t - T.T_SLOW0) / 0.5)
        else:
            V = 0.22 + 0.78 * min(1, (t - T.T_SLOW_FLING) / 0.4)
        V *= 1 - max(0.0, (t - (T.T_SLOW1 - 0.3)) / 0.3)
        return [('S', 1.0)], V, 0.045, 0.55, 0.03, 0.10
    # sweep
    if T.T_SWEEP0 <= t < T.T_SWEEP1:
        f = T.sweep_freq(t)
        rs = sorted(((lorentz(f, T.res_hz(nm)), nm) for nm, _, _ in T.SWEEP), reverse=True)[:2]
        tot = rs[0][0] + rs[1][0]
        modes = [(nm, r / tot) for r, nm in rs]
        V = 0.28 + 0.72 * rs[0][0]
        V *= min(1, (t - T.T_SWEEP0) / 0.3)
        return modes, V, 0.11, 0.9, 0.05, 0.02
    # song
    if T.T_SONG <= t < T.T_FINAL:
        cur = None
        for t0, t1, p in T.SONG:
            if t1 - t0 < 0.1:          # grace notes are too short to move sand
                continue
            if t0 <= t:
                cur = (t0, p)
        V = 0.75 + 0.25 * math.exp(-(t - cur[0]) / 0.12)
        return [(cur[1], 1.0)], V, 0.30, 1.2, 0.05, 0.02
    if T.T_FINAL <= t < T.T_FINAL_OFF:
        V = 0.35 + 0.65 * math.exp(-(t - T.T_FINAL) / 1.6)
        return [('FIN', 1.0)], V, 0.14, 0.9, 0.04, 0.02
    return [], 0.0, 0.0, 0.0, 0.0, 0.0


def scramble_plan(t0, P):
    delay = rng.uniform(0, 0.18, len(P))
    fly = rng.uniform(0.38, 0.62, len(P))
    target = P + rng.normal(0, 0.5, (len(P), 2))
    for _ in range(3):
        target = np.where(target > 0.985, 1.97 - target, target)
        target = np.where(target < -0.985, -1.97 - target, target)
    peak = rng.uniform(0.12, 0.42, len(P))
    return dict(t0=t0, start=P.copy(), delay=delay, fly=fly, target=target, peak=peak)


def main():
    P = rng.uniform(-0.985, 0.985, (G, 2))
    phase = rng.uniform(0, 2 * np.pi, G)
    pos = np.memmap('sand_pos.f32', dtype=np.float32, mode='w+', shape=(T.NF, G, 2))
    hgt = np.memmap('sand_h.f16', dtype=np.float16, mode='w+', shape=(T.NF, G))
    Vs = np.zeros(T.NF)
    freq = np.zeros(T.NF)
    activity = np.zeros(T.NF)       # how much sand is moving (for the hiss)
    cache = {}
    scr = None
    for f in range(T.NF):
        hop = np.zeros(G)
        moved = 0.0
        act = 0.0
        for s in range(SUB):
            t = (f * SUB + s) * DT
            for ts in (T.T_SCRAMBLE1, T.T_SCRAMBLE2):
                if abs(t - ts) < DT / 2:
                    scr = scramble_plan(ts, P)
            modes, V, mu, sig, sig0, hs = drive(t)
            sig0 = sig0 * 0.7
            P_old = P.copy()
            if V > 0 and modes:
                a = np.zeros(G)
                g = np.zeros((G, 2))
                kk = 0.0
                for nm, c in modes:
                    av, gv = mode_val_grad(P, *T.MODES[nm])
                    a += c * av
                    g += c * gv
                    kk += c * T.kval(nm)
                if not (T.T_SLOW0 <= t < T.T_SLOW1):
                    mu = mu * (40.0 / kk) ** 0.6
                    sig0 = sig0 * min(1.4, math.sqrt(kk / 40.0))
                step = -2 * mu * V * a[:, None] * g * DT
                noise = (sig * V * np.abs(a) + sig0 * V)[:, None] * math.sqrt(DT) * rng.normal(0, 1, (G, 2))
                P = P + step + noise
                # reflect at the edges
                P = np.where(P > 0.985, 1.97 - P, P)
                P = np.where(P < -0.985, -1.97 - P, P)
                if s == SUB - 1:
                    hop = hs * V * np.abs(a) * np.abs(np.sin(phase + t * 37.0))
                    act = V * float(np.abs(a).mean())
            if scr is not None:
                u = (t - scr['t0'] - scr['delay']) / scr['fly']
                flying = (u > 0) & (u < 1)
                landed = u >= 1
                waiting = u <= 0
                uu = np.clip(u, 0, 1)[:, None]
                P = np.where(waiting[:, None], scr['start'], P)
                P = np.where(flying[:, None], scr['start'] + (scr['target'] - scr['start']) * uu, P)
                if s == SUB - 1:
                    hop = np.where(flying, scr['peak'] * 4 * uu[:, 0] * (1 - uu[:, 0]), hop)
                if landed.all():
                    P = scr['target'].copy()
                    scr = None
                elif s == SUB - 1:
                    pass
            moved += np.abs(P - P_old).sum(1).mean()
        pos[f] = P
        hgt[f] = hop
        t = f / T.FPS
        _, V, *_ = drive(t)
        Vs[f] = V
        if T.T_SWEEP0 <= t < T.T_SWEEP1:
            freq[f] = T.sweep_freq(t)
        activity[f] = act
        if f % 150 == 0:
            print('frame', f, 'V', round(V, 2), 'activity', round(activity[f], 3), flush=True)
    pos.flush(); hgt.flush()
    np.savez('drive.npz', V=Vs, freq=freq, activity=activity, G=G)


if __name__ == '__main__':
    main()
