"""Coupled-metronome simulation (Kuramoto mean-field). The board only couples
the metronomes once it is put on rollers; as the crowd syncs, the board's
sway grows and the coupling surges, which finally drags in the holdout.

Writes timeline.npz with per-frame phases and exact tick times."""
import numpy as np

FPS = 30
DUR = 60.0
NF = int(DUR * FPS)
N = 100
ROWS = COLS = 10
HOLDOUT = 9 * 10 + 4           # front row, column 4
T_UNLOCK = 12.4                # board settles on the cans
AMP = np.radians(30)           # pendulum swing amplitude
BASE_HZ = 1.0                  # one full period per second -> ticks at 120 BPM
SEED = 77

PARAMS = dict(sigma=0.006, clip=1.8, hold=0.14, k0=0.36, k1=0.008, tau=1.0,
              k2=0.15, t2=33.0)


def K_of_t(t, p):
    if t < T_UNLOCK:
        return 0.0
    u = t - T_UNLOCK
    K = p['k0'] * (1 - np.exp(-u / p['tau'])) + p['k1'] * u
    if t > p.get('t2', 99):
        K += p.get('k2', 0) * (t - p['t2']) ** 2   # sway is now huge: coupling surges
    return min(K, 8.0)


def run(p=PARAMS, seed=SEED, dt=1 / 1200.0, tmax=DUR + 0.5):
    rng = np.random.default_rng(seed)
    delta = rng.normal(0, p['sigma'], N)
    c = p.get('clip', 2.2) * p['sigma']
    delta = np.clip(delta, -c, c)
    delta[HOLDOUT] = p['hold']
    delta -= delta.mean()                      # collective tempo is exactly BASE_HZ
    omega = 2 * np.pi * BASE_HZ * (1 + delta)
    theta = rng.uniform(0, 2 * np.pi, N)
    nsteps = int(round(tmax / dt))
    sub = int(round(1 / (FPS * dt)))
    nfr = nsteps // sub + 1
    fr_theta = np.zeros((nfr, N))
    fr_R = np.zeros(nfr)
    fr_psi = np.zeros(nfr)
    fr_K = np.zeros(nfr)
    ticks = [[] for _ in range(N)]
    prev_k = np.floor((theta - np.pi / 2) / np.pi)
    for s in range(nsteps + 1):
        t = s * dt
        z = np.exp(1j * theta).mean()
        R, psi = np.abs(z), np.angle(z)
        K = K_of_t(t, p)
        if s % sub == 0:
            f = s // sub
            fr_theta[f] = theta
            fr_R[f] = R
            fr_psi[f] = psi
            fr_K[f] = K
        theta_new = theta + (omega + K * R * np.sin(psi - theta)) * dt
        # a metronome ticks whenever its pendulum reaches an extreme (theta = pi/2 + k*pi)
        k_new = np.floor((theta_new - np.pi / 2) / np.pi)
        for i in np.nonzero(k_new > prev_k)[0]:
            target = k_new[i] * np.pi + np.pi / 2
            frac = (target - theta[i]) / (theta_new[i] - theta[i])
            ticks[i].append(t + frac * dt)
        prev_k = k_new
        theta = theta_new
    return dict(theta=fr_theta, R=fr_R, psi=fr_psi, K=fr_K, ticks=ticks, omega=omega)


def sync_flags(theta, psi, enter=np.radians(14), leave=np.radians(26)):
    flags = np.zeros_like(theta, dtype=bool)
    cur = np.zeros(theta.shape[1], dtype=bool)
    for f in range(theta.shape[0]):
        d = np.abs(np.angle(np.exp(1j * (theta[f] - psi[f]))))
        cur = np.where(cur, d < leave, d < enter)
        flags[f] = cur
    return flags


def crowd_tick_times(ticks, idx, t0, t1):
    """Beat times of the synced crowd: median of the crowd's individual tick
    times, grouped per half-swing."""
    allt = np.sort(np.concatenate([ticks[i] for i in idx]))
    allt = allt[(allt >= t0) & (allt <= t1)]
    beats, group = [], [allt[0]]
    for t in allt[1:]:
        if t - group[-1] < 0.2:
            group.append(t)
        else:
            beats.append(np.median(group)); group = [t]
    beats.append(np.median(group))
    return np.array(beats)


if __name__ == '__main__':
    out = run()
    th = out['theta']
    fl = sync_flags(th, out['psi'])
    # the holdout only counts once it has joined for good
    hold_bad = np.nonzero(~fl[:NF, HOLDOUT])[0]
    t_hold_in = (hold_bad.max() + 1) / FPS
    disp_flags = fl.copy()
    disp_flags[: int(round(t_hold_in * FPS)), HOLDOUT] = False
    disp = disp_flags.sum(1)
    others = [i for i in range(N) if i != HOLDOUT]
    crowd = crowd_tick_times(out['ticks'], others, 30.0, DUR + 0.4)
    t_drop = crowd[crowd >= t_hold_in][0]
    print('holdout joins for good at', t_hold_in, ' drop tick at', round(t_drop, 4))
    g = np.diff(crowd[crowd > t_drop - 0.01])
    print('beat spacing after drop: mean %.4f  min %.4f  max %.4f' % (g.mean(), g.min(), g.max()))
    for t in range(0, 61, 2):
        f = min(int(t * FPS), NF)
        print(f't={t:2d} R={out["R"][f]:.2f} K={out["K"][f]:.2f} count={disp[f]:3d}')
    np.savez('timeline.npz', theta=th, R=out['R'], psi=out['psi'], K=out['K'],
             flags=disp_flags, count=disp, crowd_beats=crowd,
             t_hold_in=t_hold_in, t_drop=t_drop, omega=out['omega'],
             **{f'tick_{i}': np.array(out['ticks'][i]) for i in range(N)})
