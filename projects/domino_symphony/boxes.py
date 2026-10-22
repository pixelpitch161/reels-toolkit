"""All dominoes + chime bars as one vectorised box set (from layout.npz).

Each domino rotates about its front-bottom edge: theta 0 = standing, theta_c = touching the next one,
theta_r = resting on it (pi/2 = flat on the floor). The holdout also rocks backwards (theta < 0) about
its back edge. Everything here is numpy over all ~10,250 boxes at once; render.py does the drawing."""
import math
import os
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))

# corner index = ia*4 + ib*2 + ic ; ia: 0 back / 1 front, ib: 0 right / 1 left, ic: 0 bottom / 1 top
FACES = {'top': [1, 5, 7, 3], 'front': [4, 6, 7, 5], 'back': [0, 1, 3, 2], 'left': [2, 3, 7, 6], 'right': [0, 4, 5, 1]}
FACE_NAMES = list(FACES)
FACE_IDX = np.array([FACES[k] for k in FACE_NAMES])


def _ease(x):
    x = np.clip(x, 0, 1)
    return x * x * (3 - 2 * x)


class Boxes:
    def __init__(self, path=os.path.join(HERE, 'layout.npz')):
        D = np.load(path)
        for k in D.files:
            setattr(self, k, D[k])
        self.N = len(self.cx)
        self.F = np.stack([self.fx, np.zeros(self.N), self.fz], 1)
        self.L = np.stack([self.fz, np.zeros(self.N), -self.fx], 1)
        self.is_dom = self.kind == 0
        self.base = np.stack([self.cr, self.cg, self.cb], 1).astype(float)
        self.back = np.stack([self.br, self.bg, self.bb], 1).astype(float)
        self.ts = np.maximum(0.06, 1.6 * self.tc)
        self.big = (self.dh > 0.2) | (self.sec == 9) | (self.sec == 10)
        # local corner coordinates (8 per box)
        ia = np.array([k // 4 for k in range(8)])
        ib = np.array([(k // 2) % 2 for k in range(8)])
        ic = np.array([k % 2 for k in range(8)])
        self.la = np.where(ia[None] == 1, 0.0, -self.dt[:, None])
        self.lb = np.where(ib[None] == 1, 1.0, -1.0) * self.dw[:, None] / 2
        self.lc = np.where(ic[None] == 1, 1.0, 0.0) * self.dh[:, None]
        self.holdout = int(np.where(self.sec == 9)[0][np.argmin(self.dh[self.sec == 9])])
        self.tip_angle = math.atan2(self.dt[self.holdout], self.dh[self.holdout])
        self.rng = np.random.default_rng(5)
        self.tone = 1 + self.rng.uniform(-0.05, 0.05, self.N)

    # ------------------------------------------------------------------ state
    def angles(self, tau, holdout_theta=None):
        u = (tau - self.t0) / np.maximum(self.tc, 1e-4)
        settle = _ease((tau - self.t0 - self.tc) / self.ts)
        th = np.where(u < 1, self.thc * np.clip(u, 0, 1) ** 1.7, self.thc + (self.thr - self.thc) * settle)
        th = np.where(tau < self.t0, 0.0, th)
        th = np.where(self.is_dom, th, 0.0)
        if holdout_theta is not None:
            th[self.holdout] = holdout_theta
        return th

    def holdout_theta(self, tau):
        """the holdout on the ledge: jolted at 32.0 (inner spiral lane hits the post) -> rocks and settles;
        jolted again at 35.5 (outer lane) -> rocks up to the balance point and teeters there; tips at ~37.6
        and reaches the next step at 38.25. None = use the normal fall model."""
        tip = self.tip_angle
        thc = float(self.thc[self.holdout])
        if tau < 32.0:
            return 0.0
        if tau < 34.3:
            u = tau - 32.0
            return 0.93 * tip * math.exp(-u / 0.5) * math.sin(2 * math.pi * u / 0.44)
        if tau < 35.5:
            return 0.0
        if tau < 37.55:
            from scipy.interpolate import PchipInterpolator
            if not hasattr(self, '_hp'):
                k = [35.5, 35.74, 35.98, 36.55, 36.95, 37.25, 37.55]
                v = [0.0, 0.86, -0.34, 0.992, 0.984, 0.996, 0.99]
                self._hp = PchipInterpolator(k, v)
            tr = 0.004 * math.sin(tau * 47.0) if tau > 36.55 else 0.0
            return float(self._hp(tau)) * tip + tr * tip
        if tau < 38.25:
            u = (tau - 37.55) / 0.70
            return 0.99 * tip + (thc - 0.99 * tip) * u ** 2.6
        return None

    def holdout_knocks(self):
        """times the holdout's base slaps back onto the plank (theta crosses 0 going backwards)"""
        ts = np.arange(32.0, 38.2, 0.002)
        th = np.array([self.holdout_theta(t) for t in ts])
        k = np.where((th[:-1] > 0) & (th[1:] <= 0))[0]
        return ts[k]

    def bar_hop(self, tau):
        """chime bars jump a few mm when struck"""
        dt = tau - self.t0
        dt = np.clip(dt, -1, 1)
        return np.where((~self.is_dom) & (dt >= 0) & (dt < 0.4), 0.004 * np.exp(-dt / 0.07) * np.abs(np.sin(dt * 60)), 0.0)

    def corners(self, th, lift=None):
        """(N, 8, 3) world corners for tilt angles th"""
        c, s = np.cos(th)[:, None], np.sin(th)[:, None]
        neg = (th < 0)[:, None]
        a2 = np.where(neg, self.la + self.dt[:, None], self.la)
        ap = a2 * c + self.lc * s
        cp = -a2 * s + self.lc * c
        ap = np.where(neg, ap - self.dt[:, None], ap)
        y0 = self.y0 if lift is None else self.y0 + lift
        P = (np.stack([self.cx, np.zeros(self.N), self.cz], 1)[:, None, :]
             + self.F[:, None, :] * (ap + self.dt[:, None] / 2)[..., None]
             + self.L[:, None, :] * self.lb[..., None]
             + np.array([0, 1.0, 0])[None, None, :] * (cp + y0[:, None])[..., None])
        return P

    def normals(self, th):
        """(N, 5, 3) face normals in FACE_NAMES order"""
        c, s = np.cos(th)[:, None], np.sin(th)[:, None]
        U = np.array([0, 1.0, 0])[None]
        Fr = self.F * c - U * s
        Ur = self.F * s + U * c
        return np.stack([Ur, Fr, -Fr, self.L, -self.L], 1)
