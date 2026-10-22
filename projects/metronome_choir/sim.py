"""Two-layer coupled-metronome simulation (Kuramoto mean field), the same physics as #1, twice:

    dtheta_i/dt = omega_i + Kb_k(t) w_r R_k sin(psi_k - theta_i)      riser k on its cans couples its own 39/40
                          + Kg(t)     R   sin(psi   - theta_i)        the shared platform couples everyone

Kb_k switches on when riser k settles on its soda cans; Kg switches on when the whole loft settles on the
giant cans, and surges as the sway grows (as in #1), which finally drags in the soloist (no riser coupling,
15 % fast). A metronome ticks at each end of its swing (theta = pi/2 + k pi): right end for even k.

Calibration (all exact, the model is rotation invariant):
  1. each riser is uncoupled from the others until the platform, so rotating a riser's starting phases
     rotates its whole trajectory: we set the 5 sections' phase offsets ("flams") before the platform;
  2. a global rotation puts the locked choir's left-end ticks on whole seconds (the 120 BPM grid; bar
     lines on even seconds, so the drop at 38.0 s is a downbeat);
  3. the soloist's starting phase is chosen so it joins for good just before 38 s;
  4. the jolt at 58 s resets every phase to theta_i(0) - omega_i (60 - 58): free running after the jolt
     lands exactly on the starting state at 60 s (seamless loop).

    python sim.py      -> sim.npz (per-frame phases + every tick) and a lock-in report"""
import numpy as np
import timeline as T
import layout as LY

FPS = T.FPS
N, NR, SOLO = LY.N, LY.NR, LY.SOLO
SEED = 1600
DT = 1 / 1200.0
TMAX = T.DUR + 0.6
SIGMA = 0.006                     # natural-tempo spread (relative), as #1
DELTA_SOLO = 0.15                 # the soloist runs 15 % fast
ROW_W = np.array([1.22, 1.07, 0.93, 0.78])      # front rows lock a little first ("row by row")
FLAM_MS = np.array([0.0, 90.0, -70.0, 150.0, -120.0])   # section offsets before the platform (vs bass, + = late)
P = dict(kb=5.0, kb_tau=0.4, kg0=0.36, kg_tau=2.2, kg1=0.02, kg2=3.6, t2=36.55, kmax=8.0)


def draws():
    rng = np.random.default_rng(SEED)
    theta0 = rng.uniform(0, 2 * np.pi, N)
    d = np.clip(rng.normal(0, SIGMA, N), -2.2 * SIGMA, 2.2 * SIGMA)
    for k in range(T.NSEC):
        m = LY.MEMBERS[k]
        d[m] -= d[m].mean()
    d[:NR] -= DELTA_SOLO / NR
    d[SOLO] = DELTA_SOLO
    return theta0, d


def Kb(t, k, p=P):
    ts = T.LIFT[k][1]
    if t < ts or t >= T.T_JOLT:
        return 0.0
    return p['kb'] * (1 - np.exp(-(t - ts) / p['kb_tau']))


def Kg(t, p=P):
    ts = T.PLAT[1]
    if t < ts or t >= T.T_JOLT:
        return 0.0
    u = t - ts
    K = p['kg0'] * (1 - np.exp(-u / p['kg_tau'])) + p['kg1'] * u
    if t > p['t2']:
        K += p['kg2'] * (t - p['t2']) ** 2
    return min(K, p['kmax'])


def run(theta0, delta, p=P, record=True):
    omega = 2 * np.pi * (1 + delta)
    theta = theta0.copy()
    w = np.r_[ROW_W[LY.ROW], 0.0]
    sec = LY.SEC_ALL.copy()
    M = np.zeros((N, T.NSEC))
    for k in range(T.NSEC):
        M[LY.MEMBERS[k], k] = 1.0 / len(LY.MEMBERS[k])
    reset = np.mod(theta0 - omega * (T.DUR - T.T_JOLT), 2 * np.pi)
    nsteps = int(round(TMAX / DT))
    sub = int(round(1 / (FPS * DT)))
    nfr = nsteps // sub + 1
    fr_theta = np.zeros((nfr, N))
    fr_z = np.zeros((nfr, T.NSEC), complex)
    fr_zg = np.zeros(nfr, complex)
    tick_t, tick_i, tick_k = [], [], []
    prev_k = np.floor((theta - np.pi / 2) / np.pi)
    jolted = False
    for s in range(nsteps + 1):
        t = s * DT
        if not jolted and t >= T.T_JOLT:
            jolted = True
            # continue the branch so tick counting doesn't see a jump: same angle mod 2 pi, nearest branch
            theta = theta + np.mod(reset - theta + np.pi, 2 * np.pi) - np.pi
            prev_k = np.floor((theta - np.pi / 2) / np.pi)
        e = np.exp(1j * theta)
        Z = e @ M
        zg = e[:NR].mean() * (NR / N) + e[SOLO] / N
        if s % sub == 0:
            f = s // sub
            fr_theta[f] = theta
            fr_z[f] = Z
            fr_zg[f] = zg
        kbv = np.array([Kb(t, k, p) for k in range(T.NSEC)] + [0.0])
        zi = np.r_[Z, 0.0][sec]
        dth = omega + kbv[sec] * w * np.imag(zi * np.conj(e)) + Kg(t, p) * np.imag(zg * np.conj(e))
        theta_new = theta + dth * DT
        if record:
            k_new = np.floor((theta_new - np.pi / 2) / np.pi)
            for i in np.nonzero(k_new > prev_k)[0]:
                target = k_new[i] * np.pi + np.pi / 2
                frac = (target - theta[i]) / (theta_new[i] - theta[i])
                tick_t.append(t + frac * DT)
                tick_i.append(i)
                tick_k.append(int(k_new[i]))
            prev_k = k_new
        theta = theta_new
    out = dict(theta=fr_theta, Z=fr_z, zg=fr_zg, omega=omega)
    if record:
        o = np.argsort(tick_t)
        out['tick_t'] = np.array(tick_t)[o]
        out['tick_i'] = np.array(tick_i)[o]
        out['tick_right'] = (np.array(tick_k)[o] % 2 == 0)
    return out


def frame_of(t):
    return int(round(t * FPS))


def solo_search(out, theta0, delta, cands=72, p=P):
    """the soloist barely moves the crowd (1/200 weight): integrate it alone against the crowd's mean field
    for many starting phases; pick the one that joins for good latest before 37.9 s with the smallest error
    on the drop"""
    zg = out['zg']
    ts = np.arange(len(zg)) / FPS
    om = 2 * np.pi * (1 + delta[SOLO])
    steps = int(round(39.5 / DT))
    tsteps = np.arange(steps) * DT
    zs = np.interp(tsteps, ts, zg.real) + 1j * np.interp(tsteps, ts, zg.imag)
    ks = np.array([Kg(t, p) for t in tsteps])
    th = theta0[SOLO] + 2 * np.pi * np.arange(cands) / cands
    rows_t, rows_d = [], []
    for s in range(steps):
        th = th + (om + ks[s] * np.imag(zs[s] * np.exp(-1j * th))) * DT
        if s % 40 == 0:
            rows_t.append(tsteps[s])
            rows_d.append(np.angle(np.exp(1j * th) * np.conj(zs[s])))
    rt, rd = np.array(rows_t), np.array(rows_d)            # (S,), (S, cands)
    best = None
    for c in range(cands):
        d = rd[:, c]
        bad = rt[(np.abs(d) > np.radians(14)) & (rt < 39.4)]
        t_in = bad.max() if len(bad) else 0
        err = abs(d[np.argmin(np.abs(rt - T.T_DROP))])
        off = np.abs(d[(rt > 34.0) & (rt < 37.0)]).mean()     # how off-beat in the spotlight
        score = (0 if 37.2 <= t_in <= 37.9 else 10 + abs(t_in - 37.6)) + np.degrees(err) / 10 - off
        if best is None or score < best[0]:
            best = (score, c, t_in, np.degrees(err), off)
    return best


def calibrate():
    theta0, delta = draws()
    # 1) section offsets before the platform
    a = run(theta0, delta, record=False)
    f = frame_of(T.PLAT[1] - 0.2)
    ph = np.angle(a['Z'][f])
    target = ph[0] - 2 * np.pi * FLAM_MS / 1000.0          # positive = ticks late
    alpha = target - ph
    th = theta0.copy()
    for k in range(T.NSEC):
        th[LY.MEMBERS[k]] += alpha[k]
    # 2) global rotation: left-end ticks on whole seconds after the lock
    b = run(th, delta, record=False)
    tref = 44.0
    psi = np.angle(b['zg'][frame_of(tref)])
    beta = (3 * np.pi / 2 - psi)
    th = th + beta
    b['zg'] = b['zg'] * np.exp(1j * beta)
    # 3) the soloist
    sc, c, t_in, err, off = solo_search(b, th, delta)
    th[SOLO] += 2 * np.pi * c / 72
    print(f'soloist start candidate {c}: joins for good at {t_in:.2f} s, error on the drop {err:.1f} deg, mean |offset| 34-37 s {np.degrees(off):.0f} deg')
    # 4) refine the global rotation with the chosen soloist (its pull shifts the choir by a few ms)
    d = run(th, delta, record=False)
    th = th + (3 * np.pi / 2 - np.angle(d['zg'][frame_of(tref)]))
    return np.mod(th, 2 * np.pi), delta


def sync_flags(theta, ref, enter=np.radians(14), leave=np.radians(26)):
    """hysteresis 'in sync' flags vs a reference phase per frame (shape (F,) or (F, N))"""
    d = np.abs(np.angle(np.exp(1j * (theta - (ref if np.ndim(ref) == 2 else ref[:, None])))))
    flags = np.zeros_like(d, dtype=bool)
    cur = np.zeros(d.shape[1], dtype=bool)
    for f in range(d.shape[0]):
        cur = np.where(cur, d[f] < leave, d[f] < enter)
        flags[f] = cur
    return flags


if __name__ == '__main__':
    theta0, delta = calibrate()
    out = run(theta0, delta)
    th, Z, zg = out['theta'], out['Z'], out['zg']
    F = len(th)
    # in sync with own section / with the whole choir
    secref = np.angle(Z)[:, np.r_[LY.SEC, 0]]
    fl_sec = sync_flags(th, secref)
    fl_glob = sync_flags(th, np.angle(zg))
    report = {}
    for k in range(T.NSEC):
        m = LY.MEMBERS[k]
        frac = fl_sec[:, m].mean(1)
        ok = np.nonzero((frac < 0.97) & (np.arange(F) < frame_of(T.T_JOLT)))[0]
        t_lock = (ok.max() + 1) / FPS
        report[k] = t_lock
        print(f'{T.SEC_NAMES[k]:10s} settles {T.LIFT[k][1]:5.1f}  locked (97 %) for good at {t_lock:5.2f} s   R at 25 s {abs(Z[frame_of(25), k]):.3f}')
    cnt = fl_glob[:, :NR].sum(1)
    for tt in (26, 28, 29, 30, 31, 32, 33, 34, 35, 37):
        print(f'  t={tt}: in sync with the choir {cnt[frame_of(tt)]:3d}/199   Kg={Kg(tt):.2f}  R={abs(zg[frame_of(tt)]):.3f}')
    bad = np.nonzero(~fl_glob[:frame_of(T.T_JOLT), SOLO])[0]
    t_solo = (bad.max() + 1) / FPS
    print('soloist joins for good at', t_solo)
    tt_, ti, tr = out['tick_t'], out['tick_i'], out['tick_right']
    s_ticks = tt_[ti == SOLO]
    j = np.argmin(np.abs(s_ticks - T.T_DROP))
    print(f'soloist tick nearest the drop: {s_ticks[j]:.4f} s (error {1000 * (s_ticks[j] - T.T_DROP):+.1f} ms)')
    # choir ticks after the drop vs the grid
    for t0 in (38.0, 44.0, 50.0, 57.5):
        sel = (tt_ > t0 - 0.2) & (tt_ < t0 + 0.2)
        print(f'  ticks around {t0}: n={sel.sum()}  mean {1000 * (tt_[sel].mean() - t0):+.2f} ms  spread {1000 * tt_[sel].std():.2f} ms  right-end share {tr[sel].mean():.2f}')
    # section flams before the platform: mean tick offsets of each section's ticks near 24.0 s
    for k in range(T.NSEC):
        sel = (tt_ > 23.8) & (tt_ < 24.2) & np.isin(ti, LY.MEMBERS[k])
        print(f'  {T.SEC_NAMES[k]:10s} ticks near 24.0 s: {1000 * (tt_[sel].mean() - 24.0):+6.1f} ms (spread {1000 * tt_[sel].std():.1f})')
    # loop check
    d = np.abs(np.angle(np.exp(1j * (th[frame_of(60.0)] - th[0]))))
    print(f'loop: max phase mismatch frame 1800 vs 0 = {np.degrees(d.max()):.4f} deg')
    np.savez('sim.npz', theta=th[:T.NF + 1], Z=Z[:T.NF + 1], zg=zg[:T.NF + 1], fl_sec=fl_sec[:T.NF + 1],
             fl_glob=fl_glob[:T.NF + 1], omega=out['omega'], tick_t=tt_, tick_i=ti, tick_right=tr,
             t_solo=t_solo, t_lock=np.array([report[k] for k in range(T.NSEC)]))
    print('wrote sim.npz')
