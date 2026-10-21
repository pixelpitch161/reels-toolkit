import numpy as np
from sim import N, HOLDOUT, T_UNLOCK, FPS, PARAMS, K_of_t

def batch(p, seeds, tmax=42.0, dt=1/400):
    S = len(seeds)
    th = np.zeros((S, N)); om = np.zeros((S, N))
    for j, sd in enumerate(seeds):
        rng = np.random.default_rng(sd)
        d = rng.normal(0, p['sigma'], N)
        d = np.clip(d, -p.get('clip', 2.2)*p['sigma'], p.get('clip', 2.2)*p['sigma'])
        d[HOLDOUT] = p['hold']; d -= d.mean()
        om[j] = 2*np.pi*(1+d); th[j] = rng.uniform(0, 2*np.pi, N)
    steps = int(tmax/dt); every = int(round(1/(FPS*dt)))
    frames = []
    for s in range(steps+1):
        t = s*dt
        z = np.exp(1j*th).mean(1)
        if s % every == 0:
            frames.append(np.angle(np.exp(1j*(th - np.angle(z)[:, None]))))
        K = K_of_t(t, p)
        th = th + (om + K*np.abs(z)[:, None]*np.sin(np.angle(z)[:, None] - th))*dt
    return np.array(frames)   # [F, S, N] relative phase

def flags(rel, enter=np.radians(14), leave=np.radians(26)):
    cur = np.zeros(rel.shape[1:], bool); out = np.zeros(rel.shape, bool)
    for f in range(rel.shape[0]):
        a = np.abs(rel[f]); cur = np.where(cur, a < leave, a < enter); out[f] = cur
    return out

if __name__ == '__main__':
    import sys
    p = dict(PARAMS)
    for kv in sys.argv[1].split(','):
        k, v = kv.split('='); p[k] = float(v)
    seeds = list(range(1, 81))
    rel = batch(p, seeds)
    fl = flags(rel)
    cnt = fl.sum(2)                               # [F, S]
    others = np.delete(fl, HOLDOUT, axis=2).sum(2)
    rows = []
    for j, sd in enumerate(seeds):
        o = others[:, j]; bad = np.nonzero(o < 99)[0]
        oth = (bad.max()+1)/FPS if len(bad) else 0
        c = cnt[:, j]; badf = np.nonzero(c < 100)[0]
        full = (badf.max()+1)/FPS if len(badf) else 0
        cc = [int(c[int(t*FPS)]) for t in (16, 18, 20, 22, 24, 26, 28, 30, 32)]
        chaos = c[int(5*FPS):int(12*FPS)].mean()
        rows.append((sd, cc, oth, full, chaos))
    lo, hi = float(sys.argv[2]), float(sys.argv[3])
    for r in sorted(rows, key=lambda r: -r[1][3]):
        if lo < r[2] < hi:
            print(r[0], r[1], f"others@{r[2]:.1f} full@{r[3]:.1f} chaos={r[4]:.1f}")
