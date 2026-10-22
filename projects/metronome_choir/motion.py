"""Kinematics shared by picture and sound: riser lifts onto their soda cans, the platform hoist onto the giant
cans, the sway (every swing shoves the board, as in #1), the kick-out + thump at the end, pendulum angles,
and the in-sync bookkeeping for tips and the HUD. Everything reads sim.npz (run sim.py first)."""
import math
import os
import numpy as np
import timeline as T
import layout as LY

HERE = os.path.dirname(os.path.abspath(__file__))
SIM = np.load(os.path.join(HERE, 'sim.npz'))
THETA = SIM['theta']                 # (1801, 200) unwrapped phases at frame times
ZSEC = SIM['Z']                      # (1801, 5) section mean fields
ZG = SIM['zg']                       # (1801,) whole-choir mean field
FL_SEC = SIM['fl_sec']
FL_GLOB = SIM['fl_glob']
T_LOCK = SIM['t_lock']               # per section: locked for good
T_SOLO = float(SIM['t_solo'])        # soloist joins for good
AMP = math.radians(30)               # pendulum swing amplitude
X_OFF = 10.4                         # riser cans wait just past the step ends (off frame)
GX_OFF = 17.5                        # giant cans wait in the wings
A_BOARD, A_PLAT = 0.30, 0.20         # sway amplitudes (units per unit mean sin(theta))


def fidx(t):
    return int(min(len(THETA) - 1, max(0, round(t * T.FPS))))


def ss(a, b, x):
    x = min(1.0, max(0.0, (x - a) / (b - a)))
    return x * x * (3 - 2 * x)


def ease(x):
    x = min(1.0, max(0.0, x))
    return x * x * x * (x * (6 * x - 15) + 10)


def bounce(u, amp, freq=18.0, decay=7.0):
    return amp * math.exp(-u * decay) * abs(math.sin(u * freq))


def _lift(t, a, b, h_on, h_up):
    """height of a board/platform being lifted at a, hovering, settling onto its cans at b"""
    if t < a:
        return 0.0
    if t < a + 0.55:
        return h_up * ease((t - a) / 0.55)
    if t < b - 0.3:
        return h_up + 0.03 * math.sin((t - a) * 9)
    if t < b:
        u = (t - (b - 0.3)) / 0.3
        return h_up + (h_on - h_up) * u * u
    return h_on + bounce(t - b, 0.07)


def board_state(k, t):
    """riser k: (lift above its step, sway dx, can |x| or None, can roll angle)"""
    a, b = T.LIFT[k]
    h_on = 2 * LY.CAN_R
    y = _lift(t, a, b, h_on, h_on + 0.5)
    cx, roll = None, 0.0
    if a + 0.2 <= t < T.T_JOLT:
        u = ease((t - (a + 0.2)) / (b - 0.3 - (a + 0.2)))
        cx = X_OFF + (LY.CAN_X - X_OFF) * u
    dx = 0.0
    if b <= t < T.T_JOLT:
        f = fidx(t)
        m = LY.MEMBERS[k]
        dx = -A_BOARD * float(np.sin(THETA[f, m]).mean()) * ss(b, b + 0.6, t)
    if t >= T.T_JOLT:                        # the thump: board drops, cans pop out sideways
        u = t - T.T_JOLT
        y = max(0.0, h_on * (1 - (u / 0.1) ** 2)) if u < 0.1 else bounce(u - 0.1, 0.05)
        if u < 0.5:
            cx = LY.CAN_X + (X_OFF + 0.4 - LY.CAN_X) * (u / 0.5) ** 0.7
    if cx is not None:
        roll = (X_OFF - cx) / LY.CAN_R + dx / LY.CAN_R * 0.5
    return y, dx, cx, roll


def can_hop(t):
    """vertical hop of the riser cans as they fly out at the jolt"""
    u = t - T.T_JOLT
    if 0 <= u < 0.5:
        return 0.9 * math.sin(math.pi * u / 0.5)
    return 0.0


def plat_state(t):
    """shared platform: (lift above the floor, sway dx, giant can |x| or None, roll)"""
    a, b = T.PLAT
    h_on = 2 * LY.GCAN_R
    y = _lift(t, a, b, h_on, h_on + 0.7)
    gx = None
    if a + 0.45 <= t < T.T_KICK + 0.6:
        u = ease((t - (a + 0.45)) / (b - 0.35 - (a + 0.45)))
        gx = GX_OFF + (LY.GCAN_X - GX_OFF) * u
    dx = 0.0
    if b <= t < T.T_KICK + 0.3:
        f = fidx(t)
        dx = -A_PLAT * float(np.sin(THETA[f]).mean()) * ss(b, b + 0.7, t) * (1 - ss(T.T_KICK, T.T_KICK + 0.3, t))
    if t >= T.T_KICK:                        # kicked out: cans shoot to the wings, platform hangs, then slams
        u = (t - T.T_KICK) / 0.4
        gx = LY.GCAN_X + (GX_OFF - LY.GCAN_X) * min(1.0, u) ** 1.6 if u < 1.5 else None
        hang = T.T_JOLT - 0.16
        if t < hang:
            y = h_on
        elif t < T.T_JOLT:
            y = h_on * (1 - ((t - hang) / 0.16) ** 2)
        else:
            y = bounce(t - T.T_JOLT, 0.12, 16, 8)
    roll = 0.0 if gx is None else (GX_OFF - gx) / LY.GCAN_R + dx / LY.GCAN_R * 0.5
    return y, dx, gx, roll


def metronome_bases(t):
    """world base position (on the board top) of all 200 metronomes, and per-riser state"""
    py, pdx, _, _ = plat_state(t)
    P = np.zeros((LY.N, 3))
    boards = []
    for k in range(T.NSEC):
        y, dx, cx, roll = board_state(k, t)
        boards.append((y, dx, cx, roll))
        m = LY.MEMBERS[k]
        P[m, 0] = LY.LX[m] + dx + pdx
        P[m, 1] = py + LY.YT[k] + y + LY.BOARD_T
        P[m, 2] = LY.ZT[k] + LY.LZ[m]
    P[LY.SOLO] = [LY.POD[0] + pdx, py + LY.PLAT_T + LY.POD_H, LY.POD[1]]
    return P, boards, (py, pdx)


def pendulum(t):
    """pendulum angles (radians, + = toward +x) of all 200"""
    return AMP * np.sin(THETA[fidx(t)])


# ---------------------------------------------------------------- in-sync bookkeeping
def rel_phase(t):
    """phase of each metronome relative to its reference: own section before the platform, the whole choir
    after it (blended over 0.8 s), own section again after the jolt (matches the loop start)"""
    f = fidx(t)
    th = THETA[f]
    sec_ref = np.angle(ZSEC[f])[np.r_[LY.SEC, 0]]
    g_ref = np.angle(ZG[f])
    d_sec = np.angle(np.exp(1j * (th - sec_ref)))
    d_g = np.angle(np.exp(1j * (th - g_ref)))
    w = ss(T.PLAT[1], T.PLAT[1] + 0.8, t) * (1 - ss(T.T_JOLT, T.T_JOLT + 0.05, t))
    d = np.abs(d_sec) * (1 - w) + np.abs(d_g) * w
    d[LY.SOLO] = abs(d_g[LY.SOLO]) if t < T.T_JOLT else abs(d_sec[LY.SOLO])
    return d


def sections_locked(t):
    return [t >= T_LOCK[k] for k in range(T.NSEC)]


def count_in_sync(t):
    """metronomes in sync with the whole choir (hysteresis flags from sim.py), soloist included"""
    f = fidx(t)
    n = int(FL_GLOB[f, :LY.NR].sum())
    solo = t >= T_SOLO
    return n + (1 if solo else 0)
