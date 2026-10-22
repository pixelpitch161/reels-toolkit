"""The Metronome Choir - renderer.

    python -m reelkit.sheet render.py test/sheet.png 0.5 4 8 11 15 20 23 27 32 36 39 46 54 59
    python -m reelkit.video render.py audio.wav metronome_choir.mp4

Per frame: ray-cast hall shell (floor with skylight sun patches + loft shadows, organ wall, side walls with
wings/balconies/sconces, coffered ceiling with the skylight) -> giant cans -> platform (per-pixel top) ->
tiers back to front: step, carpet (per-pixel), riser cans, board edges, maple board top (per-pixel, with every
metronome's shadow and every swinging rod's shadow), metronomes (cream bodies, section caps, phase-coloured tips,
LED panels in the drop) -> podium + golden soloist -> piano + cat, timpani, seats + pawns, truss -> locked focus
on the choir -> sunbeam + motes -> glows -> bloom -> grade -> HUD + captions."""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, ss, ease, nrm, mixc
from reelkit.camera import Cam, interp_keys
from reelkit.raster import Frame, painter, apply_glows
from reelkit.lighting import Lights, shade
from reelkit.shadows import MaskGrid, proj_dir
from reelkit.lens import coc_map, dof
from reelkit.post import bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, panel, MINT, CREAM, AMBER, CORAL, SKYB, INK, PINK
from reelkit import font as RF
from reelkit.font import text_mask, stamp, rich_line
from reelkit.geometry import box, prism
import timeline as T
import layout as LY
import scene as S
import motion as MO

for ch, rows in {'→': ".....|..#..|...#.|#####|...#.|..#..|.....",
                 '✓': ".....|....#|...#.|#.#..|.#...|.....|.....",
                 '▼': ".....|#####|.###.|..#..|.....|.....|....."}.items():
    RF.GLYPHS[ch] = np.array([[c == '#' for c in r] for r in rows.split('|')])

SUN = S.SUN
WARM = S.WARM
LED = [(124, 250, 200), (110, 200, 255), (176, 140, 255), (255, 120, 200), (255, 118, 96), (255, 204, 88), (178, 250, 140)]


# ---------------------------------------------------------------- helpers
def hash2(a, b):
    v = np.sin(a * 127.1 + b * 311.7) * 43758.5453
    return v - np.floor(v)


def sync_color(d):
    """|relative phase| -> tip colour: mint = with the reference, coral = way off (#1's ramp)"""
    stops = [(0.0, (124, 250, 200)), (0.30, (178, 250, 140)), (0.85, (255, 204, 88)), (1.7, (255, 118, 96)), (math.pi, (240, 86, 128))]
    d = min(abs(d), math.pi)
    for (a, ca), (b, cb) in zip(stops, stops[1:]):
        if d <= b:
            u = (d - a) / (b - a)
            return tuple(ca[k] + (cb[k] - ca[k]) * u for k in range(3))
    return stops[-1][1]


class Boxes:
    """axis-aligned occluders (lo, hi, group) for per-point sun visibility"""
    def __init__(self):
        self.lo, self.hi, self.g = [], [], []

    def add(self, lo, hi, g):
        self.lo.append(np.asarray(lo, float)); self.hi.append(np.asarray(hi, float)); self.g.append(g)

    def done(self):
        self.lo = np.array(self.lo); self.hi = np.array(self.hi); self.g = np.array(self.g)
        return self

    def subset(self, keep):
        b = Boxes()
        m = np.array([keep(g) for g in self.g], bool) if len(self.g) else np.zeros(0, bool)
        b.lo, b.hi, b.g = self.lo[m], self.hi[m], self.g[m]
        return b

    def vis(self, P, pg=-1, d=SUN):
        P = np.atleast_2d(P)
        if not len(self.g) or not len(P):
            return np.ones(len(P), np.float32)
        inv = 1.0 / np.where(np.abs(d) < 1e-9, 1e-9, d)
        out = np.ones(len(P), np.float32)
        pg = np.broadcast_to(np.asarray(pg), (len(P),))
        for s in range(0, len(P), 4096):
            p = P[s:s + 4096]
            t1 = (self.lo[None] - p[:, None]) * inv
            t2 = (self.hi[None] - p[:, None]) * inv
            tn = np.minimum(t1, t2).max(-1)
            tf = np.maximum(t1, t2).min(-1)
            hit = (tf >= np.maximum(tn, 1e-3)) & (self.g[None] != pg[s:s + 4096, None])
            out[s:s + 4096] = ~hit.any(1)
        return out


# ---------------------------------------------------------------- camera
SOLO_TOP = LY.PLAT_T + LY.POD_H + 0.8 * LY.SOLO_S


def KF(t, x, y, z, d, yaw, pitch, fov=46, sy=330):
    return (t, np.array([x, y, z], float), d, yaw, pitch, fov, sy)


OPEN = KF(0.0, 0.0, 4.4, -7.6, 30.0, 0, 21, 46, 336)
KEYS = [
    OPEN,
    KF(2.9, 0.0, 4.6, -7.8, 28.5, 3, 21, 46, 336),
    KF(3.25, 2.2, 8.6, -17.2, 12.5, 26, 20, 42, 330),            # sweep: high bells ...
    KF(5.9, -0.6, 2.4, -0.4, 10.0, 22, 16, 42, 340),            # ... down to the bass
    KF(6.6, 1.0, 2.0, 0.6, 16.5, 32, 24, 44, 300),              # bass onto its cans
    KF(9.0, 0.6, 2.4, 0.0, 15.0, 24, 22, 44, 300),
    KF(12.6, 0.0, 2.9, -1.6, 12.0, -16, 26, 44, 300),           # bass locked, tenor behind still random
    KF(13.6, 0.4, 3.8, -4.4, 13.0, 22, 22, 44, 300),            # tenor
    KF(16.5, 0.0, 4.2, -5.0, 13.0, 12, 25, 44, 300),
    KF(17.6, 0.0, 5.6, -10.0, 19.5, -14, 23, 46, 300),          # alto + soprano
    KF(20.6, 0.0, 5.8, -10.0, 19.0, -6, 23, 46, 300),
    KF(21.6, 0.0, 3.8, -7.6, 30.0, 0, 18, 46, 290),             # all five, out of step (frontal)
    KF(24.8, 0.0, 3.9, -7.6, 29.0, 3, 18, 46, 290),
    KF(25.8, 0.0, 3.4, -4.0, 31.0, 34, 8, 48, 330),             # the hoist: giant cans roll in
    KF(29.0, 0.0, 4.6, -5.0, 30.0, 26, 10, 48, 330),
    KF(30.4, 0.0, 5.6, -8.8, 29.0, -8, 18, 46, 292),            # sections pull together
    KF(33.6, 0.0, 5.6, -8.8, 28.0, 6, 18, 46, 292),
    KF(34.6, 0.0, 2.3 + SOLO_TOP - 0.6, 4.5, 8.5, 12, 11, 42, 340),   # spotlight on the soloist
    KF(37.92, 0.0, 2.3 + SOLO_TOP - 0.55, 4.5, 5.4, -6, 8, 42, 350),
]
DROP_SEGS = [   # cuts on bar lines during the song
    (T.T_DROP, T.T_DROP + 2 * T.BAR, KF(0, 0.0, 7.4, -8.0, 26.0, 0, 13, 46, 330), KF(0, 0.0, 7.4, -8.0, 24.5, 0, 14, 46, 330)),
    (T.T_DROP + 2 * T.BAR, T.T_DROP + 4 * T.BAR, KF(0, 0.0, 6.4, -6.0, 22.0, -34, 12, 46, 336), KF(0, 0.0, 6.6, -7.0, 22.0, -18, 15, 46, 336)),
    (T.T_DROP + 4 * T.BAR, T.T_DROP + 6 * T.BAR, KF(0, 0.0, 7.4, -8.0, 24.5, 4, 18, 46, 330), KF(0, 0.0, 7.4, -8.0, 23.0, 0, 15, 46, 330)),
    (T.T_DROP + 6 * T.BAR, T.T_DROP + 7 * T.BAR, KF(0, 0.0, 6.4, -6.0, 22.0, 30, 12, 46, 336), KF(0, 0.0, 6.4, -6.0, 22.0, 20, 14, 46, 336)),
]
KEYS_OUT = [
    KF(T.T_DROP + 7 * T.BAR, 0.0, 6.6, -7.4, 33.0, 8, 15, 48, 334),       # wide: the whole choir, final chorus
    KF(57.6, 0.0, 6.3, -7.4, 31.0, 2, 18, 47, 334),
    KF(58.6, 0.0, 4.6, -7.6, 30.5, 0, 21, 46, 336),
    (60.0,) + OPEN[1:],
]


def cam_key(t):
    t = t % T.DUR
    if t < T.T_DROP:
        return interp_keys(KEYS, t)
    for t0, t1, ka, kb in DROP_SEGS:
        if t0 <= t < t1:
            u = ease((t - t0) / (t1 - t0)) * 0.3 + 0.7 * (t - t0) / (t1 - t0)
            k = [t] + [np.asarray(ka[i], float) + (np.asarray(kb[i], float) - np.asarray(ka[i], float)) * u for i in range(1, 7)]
            return Cam(k[1], float(k[2]), float(k[3]), float(k[4]), float(k[5]), float(k[6]))
    return interp_keys(KEYS_OUT, t)


def shake(t):
    out = np.zeros(3)
    hits = [(T.LIFT[k][1], 0.04) for k in range(T.NSEC)] + [(T.PLAT[1], 0.12), (T.T_DROP, 0.22), (T.T_JOLT, 0.3)]
    hits += [(T.T_DROP + b * T.BAR, 0.06) for b in (2, 4, 6, 8)]
    for t0, amp in hits:
        if t0 <= t < t0 + 0.5:
            a = amp * (1 - (t - t0) / 0.5) ** 2
            out += a * np.array([math.sin(t * 91.0), math.sin(t * 77.0 + 1), math.sin(t * 63.0 + 2)])
    return out


def camera(t):
    c = cam_key(t)
    s = shake(t)
    if s.any():
        c.pos = c.pos + s
    if T.T_DROP <= t < T.T_JOLT:
        c.f *= 1 + 0.02 * T.beat_env(t)
    return c


# ---------------------------------------------------------------- lights
def spot_level(t):
    """front spots dim a little while the soloist has the spotlight"""
    return 1.0 - 0.35 * ss(33.8, 34.6, t) * (1 - ss(T.T_DROP - 0.05, T.T_DROP + 0.2, t))


def make_lights(t, cam, py):
    L = Lights(amb=(0.35, 0.37, 0.46), sky=(0.08, 0.09, 0.12))
    L.view = cam.view
    L.key('sun', SUN, S.SUN_RGB * 1.05)
    L.fill(nrm([0.25, 0.45, 1.0]), (0.08, 0.065, 0.05))
    k = spot_level(t)
    for sp in S.SPOTS:
        L.point(sp, WARM * 0.5 * k, 11.0)
    for x in (-5.0, 0.0, 5.0):
        L.point(np.array([x, py + LY.PLAT_T + S.FOOT_Y, LY.PLAT_Z1 + 0.15]), WARM * 0.32, 2.2)
    return L


def point_light(P, nv, L, skip_keys=True):
    """diffuse intensity from L's point lights + fills on surfaces with normals nv at P"""
    I = np.zeros((len(P), 3))
    nv = np.broadcast_to(np.asarray(nv, float), P.shape)
    for lp, lc, R in L.points:
        v = lp[None] - P
        dist = np.linalg.norm(v, axis=1) + 1e-6
        cosv = (v * nv).sum(1) / dist
        lam = np.clip(cosv, 0, None) * 0.85 + 0.15 * (cosv > -0.05)
        I += (lam / (1 + (dist / R) ** 2))[:, None] * lc[None]
    for d, c in L.fills:
        I += np.clip(nv @ d, 0, None)[:, None] * c[None]
    return I


# ---------------------------------------------------------------- textures
def floor_albedo(X, Z):
    """stage-front + hall floor: oak parquet in the hall, dark stage boards under the loft"""
    stage = Z < 11.5
    col = np.zeros((len(X), 3))
    pw = 0.16
    ci = np.floor(X / pw)
    off = hash2(ci, 3.0) * 1.2
    row = np.floor((Z + off) / 1.0)
    h = hash2(ci, row)
    col[:] = np.array([170, 116, 74.0])[None] * (0.82 + 0.24 * h)[:, None]
    u, v = (X / pw) % 1.0, ((Z + off) / 1.0) % 1.0
    col[(np.abs(u - 0.5) > 0.45) | (np.abs(v - 0.5) > 0.493)] *= 0.7
    if stage.any():
        pz = 0.22
        cz = np.floor(Z[stage] / pz)
        o2 = hash2(cz, 9.0) * 1.6
        hh = hash2(cz, np.floor((X[stage] + o2) / 1.8))
        c2 = np.array([96, 64, 46.0])[None] * (0.86 + 0.2 * hh)[:, None]
        uu, vv = (Z[stage] / pz) % 1.0, ((X[stage] + o2) / 1.8) % 1.0
        c2[(np.abs(uu - 0.5) > 0.44) | (np.abs(vv - 0.5) > 0.495)] *= 0.7
        col[stage] = c2
    return col


ORGAN_X = 12.5


def back_albedo(X, Y):
    col = np.zeros((len(X), 3))
    n = hash2(np.floor(X * 3), np.floor(Y * 3))
    # wood panelling
    col[:] = np.array([128, 82, 52.0])[None] * (0.92 + 0.08 * n)[:, None]
    pan = (X % 1.6)
    col[(np.abs(pan - 0.8) > 0.74)] *= 0.75
    col[np.abs(Y - 9.0) < 0.12] = [210, 160, 80]                       # gilded rail under the organ
    # organ facade: pipe flats and towers
    org = (np.abs(X) < ORGAN_X) & (Y > 9.15)
    if org.any():
        x, y = X[org], Y[org]
        pw = 0.42
        pi_ = np.floor(x / pw)
        u = (x / pw) % 1.0
        grp = np.floor((x + ORGAN_X) / 3.125)
        tower = (grp % 2 == 0)
        top = np.where(tower, 15.6, 13.6) + 0.9 * np.cos((pi_ * pw - (grp * 3.125 - ORGAN_X + 1.56)) * 1.1) ** 2 * np.where(tower, 1.0, -0.6)
        pipe = (np.abs(u - 0.5) < 0.42) & (y < top)
        mouth_y = 10.0 + 0.25 * (pi_ % 3)
        c = np.zeros((len(x), 3))
        c[:] = [70, 42, 30]                                             # case behind the pipes
        shine = 0.62 + 0.5 * np.sin(np.pi * np.clip(u / 0.84 + 0.04, 0, 1)) ** 2
        pc = np.array([214, 202, 178.0])[None] * shine[:, None]
        c[pipe] = pc[pipe]
        mouth = pipe & (np.abs(y - mouth_y) < 0.16) & (np.abs(u - 0.5) < 0.25)
        c[mouth] = [40, 34, 34]
        cap = (np.abs(y - top - 0.1) < 0.1) & (np.abs(u - 0.5) < 0.48)
        c[cap] = [226, 176, 84]
        crown = (y > 16.3) & (y < 16.7)
        c[crown] = [214, 166, 80]
        c[y >= 16.7] = [90, 56, 38]
        col[org] = c
    return col


def side_albedo(Z, Y):
    col = np.zeros((len(Z), 3))
    n = hash2(np.floor(Z * 4), np.floor(Y * 4))
    col[:] = np.array([206, 182, 146.0])[None] * (0.95 + 0.05 * n)[:, None]
    pil = (Z % 4.0) < 0.55
    col[pil] = [222, 202, 168]
    col[Y < 1.2] = [110, 70, 46]
    col[np.abs(Y - 1.2) < 0.05] = [200, 150, 80]
    wing = (Z > S.WING_Z[0]) & (Z < S.WING_Z[1]) & (Y < S.WING_Y)
    if wing.any():
        fold = 0.72 + 0.28 * np.abs(np.sin(Z[wing] * 3.4))
        col[wing] = np.array([92, 26, 44.0])[None] * fold[:, None]
        col[wing & (Y < 0.35)] *= 0.6
    pelmet = (Z > S.WING_Z[0] - 0.3) & (Z < S.WING_Z[1] + 0.3) & (Y >= S.WING_Y) & (Y < S.WING_Y + 0.7)
    col[pelmet] = [150, 40, 52]
    col[pelmet & (Y > S.WING_Y + 0.55)] = [214, 166, 80]
    balc = (Z > S.WING_Z[1] + 1.0) & (Y > 7.0) & (Y < 8.3)
    col[balc] = [150, 36, 46]
    col[(Z > S.WING_Z[1] + 1.0) & (np.abs(Y - 8.3) < 0.08)] = [214, 166, 80]
    col[(Z > S.WING_Z[1] + 1.0) & (np.abs(Y - 7.0) < 0.06)] = [214, 166, 80]
    col[Y > S.HY - 0.4] = [214, 166, 80]
    return col


SCONCES = [np.array([sx * (S.HX - 0.15), 5.0, z]) for sx in (-1, 1) for z in (9.5, 15.5, 21.5)] + \
          [np.array([x, 5.0, S.HZ0 + 0.15]) for x in (-13.8, 13.8)]


def ceil_albedo(X, Z):
    col = np.zeros((len(X), 3))
    col[:] = [228, 214, 186]
    u, v = (X / 2.0) % 1.0, (Z / 2.0) % 1.0
    beam = (np.abs(u - 0.5) > 0.42) | (np.abs(v - 0.5) > 0.42)
    inner = (np.abs(u - 0.5) < 0.28) & (np.abs(v - 0.5) < 0.28)
    col[~beam & ~inner] = [206, 190, 160]
    col[beam] = [120, 80, 54]
    return col


def skylight(X, Z, cam_dirs, p):
    """inside the skylight ring: sky through the glazing (sun disc where the view ray meets it)"""
    dx, dz = X - S.SKY_C[0], Z - S.SKY_C[1]
    r = np.sqrt(dx * dx + dz * dz)
    ang = np.arctan2(dz, dx)
    col = np.zeros((len(X), 3))
    el = np.clip(cam_dirs[:, 1] / (np.linalg.norm(cam_dirs, axis=1) + 1e-9), 0, 1)
    sky = np.array([120, 178, 236.0])[None] * (1 - el[:, None]) * 0.6 + np.array([70, 130, 220.0])[None] * (0.4 + 0.6 * el[:, None])
    cosang = (cam_dirs / (np.linalg.norm(cam_dirs, axis=1, keepdims=True) + 1e-9)) @ SUN
    sky += (np.clip((cosang - 0.94) / 0.06, 0, 1) ** 2)[:, None] * np.array([150, 120, 60.0])
    sky[cosang > 0.9985] = [255, 248, 222]
    col[:] = sky
    bars = (np.abs(r - S.SKY_R * 0.55) < 0.12) | (np.abs(((ang / (2 * np.pi) * 16) % 1.0) - 0.5) * r * 2 * np.pi / 16 > (2 * np.pi * r / 16 * 0.5 - 0.09))
    bars &= r > 1.2
    col[bars] = [64, 58, 62]
    col[r > S.SKY_R - 0.35] = [80, 70, 64]
    return col, r < S.SKY_R


# ---------------------------------------------------------------- per-frame state
class State:
    def __init__(self, t, cam):
        self.t = t
        self.cam = cam
        self.P, self.boards, (self.py, self.pdx) = MO.metronome_bases(t)
        self.phi = MO.pendulum(t)
        self.f = MO.fidx(t)
        self.theta = MO.THETA[self.f]
        self.pstate = MO.plat_state(t)
        self.L = make_lights(t, cam, self.py)
        # occluders: platform, steps, boards, cans, podium, every metronome body, piano, timpani
        bx = Boxes()
        py, pdx = self.py, self.pdx
        bx.add([pdx - LY.PLAT_HX, py, LY.PLAT_Z0], [pdx + LY.PLAT_HX, py + LY.PLAT_T, LY.PLAT_Z1], 1)
        for k in range(T.NSEC):
            y, dx, cx, roll = self.boards[k]
            bx.add([pdx - LY.STEP_HX, py + LY.PLAT_T, LY.ZT[k] - LY.STEP_HZ], [pdx + LY.STEP_HX, py + LY.YT[k], LY.ZT[k] + LY.STEP_HZ], 10 + k)
            yb = py + LY.YT[k] + y
            bx.add([pdx + dx - LY.BOARD_HX, yb, LY.ZT[k] - LY.BOARD_HZ], [pdx + dx + LY.BOARD_HX, yb + LY.BOARD_T, LY.ZT[k] + LY.BOARD_HZ], 20 + k)
            if cx is not None and y > 0.05:
                for sx in (-1, 1):
                    xc = pdx + dx * 0.5 + sx * cx
                    yc = py + LY.YT[k] + LY.CAN_R + S_canhop(t)
                    bx.add([xc - LY.CAN_R, yc - LY.CAN_R, LY.ZT[k] - 2.1], [xc + LY.CAN_R, yc + LY.CAN_R, LY.ZT[k] + 2.1], 30 + k)
        bx.add([pdx + LY.POD[0] - LY.POD_R, py + LY.PLAT_T, LY.POD[1] - LY.POD_R], [pdx + LY.POD[0] + LY.POD_R, py + LY.PLAT_T + LY.POD_H, LY.POD[1] + LY.POD_R], 40)
        self.big = Boxes()
        for lo, hi, g in zip(bx.lo, bx.hi, bx.g):
            self.big.add(lo, hi, g)
        gx = self.pstate[2]
        if gx is not None:
            for sx in (-1, 1):
                for z in LY.GCAN_Z:
                    xc = pdx * 0.5 + sx * gx
                    self.big.add([xc - LY.GCAN_R, 0, z - LY.GCAN_L / 2], [xc + LY.GCAN_R, 2 * LY.GCAN_R, z + LY.GCAN_L / 2], 50)
        for o in S.FRONT_OBJS:
            if o.cast:
                self.big.add(o.lo, o.hi, 60 + o.grp)
        self.big.done()
        sc = np.r_[np.ones(LY.NR), LY.SOLO_S]
        for i in range(LY.N):
            p = self.P[i]
            s = sc[i]
            bx.add(p + np.array([-0.33, 0, -0.27]) * s, p + np.array([0.33, S.MET_H, 0.27]) * s, 1000 + i)
        self.occ = bx.done()


def S_canhop(t):
    return MO.can_hop(t)


# ---------------------------------------------------------------- the hall shell, per pixel
def shell_pass(fr, st):
    cam, L, t = st.cam, st.L, st.t
    arr, depth = fr.array(), fr.depth()
    dw = cam.pixel_rays(XX, YY)
    p = cam.pos
    INF = 1e9
    with np.errstate(divide='ignore', invalid='ignore'):
        def plane(axis, val, sign):
            den = dw[..., axis]
            tt = np.where(den * sign < -1e-7, (val - p[axis]) / den, INF)
            return np.where(tt > 0, tt, INF)
        tf = plane(1, 0.0, 1)
        Xf, Zf = p[0] + tf * dw[..., 0], p[2] + tf * dw[..., 2]
        okf = (np.abs(Xf) <= S.HX) & (Zf >= S.HZ0) & (Zf <= S.HZ1)
        tb = plane(2, S.HZ0, 1)
        Xb, Yb = p[0] + tb * dw[..., 0], p[1] + tb * dw[..., 1]
        okb = (np.abs(Xb) <= S.HX) & (Yb >= 0) & (Yb <= S.HY)
        tl = plane(0, -S.HX, 1)
        Yl, Zl = p[1] + tl * dw[..., 1], p[2] + tl * dw[..., 2]
        okl = (Yl >= 0) & (Yl <= S.HY) & (Zl >= S.HZ0) & (Zl <= S.HZ1)
        tr = plane(0, S.HX, -1)
        Yr, Zr = p[1] + tr * dw[..., 1], p[2] + tr * dw[..., 2]
        okr = (Yr >= 0) & (Yr <= S.HY) & (Zr >= S.HZ0) & (Zr <= S.HZ1)
        tc = plane(1, S.HY, -1)
        Xc, Zc = p[0] + tc * dw[..., 0], p[2] + tc * dw[..., 2]
        okc = (np.abs(Xc) <= S.HX) & (Zc >= S.HZ0) & (Zc <= S.HZ1)
    TT = [np.where(m, tt, INF) for m, tt in ((okf, tf), (okb, tb), (okl, tl), (okr, tr), (okc, tc))]
    tmin = np.minimum.reduce(TT)
    which = np.argmin(np.stack(TT), 0)
    which[tmin >= INF] = -1
    amb = L.amb

    def put(ys, xs, col, d):
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = d

    def sconce_light(P, nv):
        I = np.zeros((len(P), 3))
        for sp in SCONCES:
            v = sp[None] - P
            dist = np.linalg.norm(v, axis=1) + 1e-6
            lam = np.clip((v * nv).sum(1) / dist, 0, None) * 0.8 + 0.2
            I += (lam * 0.55 / (1 + (dist / 1.6) ** 2))[:, None] * WARM[None]
        return I

    # floor
    ys, xs = np.nonzero(which == 0)
    if len(xs):
        X, Z = Xf[ys, xs], Zf[ys, xs]
        P = np.stack([X, np.zeros_like(X), Z], 1)
        nv = np.array([0, 1.0, 0])
        I = np.tile(amb + L.sky, (len(X), 1)) + point_light(P, nv, L)
        sl = S.sky_lit(P)
        m = (sl > 0) & (ys % 2 == 0) & (xs % 2 == 0)
        if m.any():
            v = np.ones((H // 2 + 1, W // 2 + 1), np.float32)
            v[ys[m] // 2, xs[m] // 2] = st.big.vis(P[m] + [0, 0.01, 0], -1)
            sl *= v[ys // 2, xs // 2]
        I += (sl * SUN[1] * 1.15)[:, None] * L.keys[0][2][None]
        I *= (1 - 0.3 * np.exp(-(S.HX - np.abs(X)) / 0.4))[:, None]
        put(ys, xs, floor_albedo(X, Z) * I, tf[ys, xs])
    # back wall
    ys, xs = np.nonzero(which == 1)
    if len(xs):
        X, Y = Xb[ys, xs], Yb[ys, xs]
        P = np.stack([X, Y, np.full_like(X, S.HZ0)], 1)
        nv = np.array([0, 0, 1.0])
        I = np.tile(amb * 0.95, (len(X), 1)) + point_light(P, nv, L) + sconce_light(P, nv)
        I += 0.18 * np.exp(-((X - S.SKY_C[0]) / 9.0) ** 2)[:, None] * S.SUN_RGB[None] * (Y > 4)[:, None]   # sun bounce off the loft
        I *= (1 - 0.3 * np.exp(-Y / 0.4))[:, None]
        put(ys, xs, back_albedo(X, Y) * I, tb[ys, xs])
    # side walls
    for code, Yw, Zw, tw, sx in ((2, Yl, Zl, tl, -1), (3, Yr, Zr, tr, 1)):
        ys, xs = np.nonzero(which == code)
        if not len(xs):
            continue
        Y, Z = Yw[ys, xs], Zw[ys, xs]
        P = np.stack([np.full_like(Z, sx * S.HX), Y, Z], 1)
        nv = np.array([-sx, 0, 0.0])
        I = np.tile(amb * 0.92, (len(Z), 1)) + point_light(P, nv, L) + sconce_light(P, nv)
        sl = S.sky_lit(P + nv * 0.01) * max(0.0, float(nv @ SUN))
        I += (sl * 1.1)[:, None] * L.keys[0][2][None]
        I *= (1 - 0.25 * np.exp(-Y / 0.3))[:, None]
        put(ys, xs, side_albedo(Z, Y) * I, tw[ys, xs])
    # ceiling + skylight
    ys, xs = np.nonzero(which == 4)
    if len(xs):
        X, Z = Xc[ys, xs], Zc[ys, xs]
        P = np.stack([X, np.full_like(X, S.HY), Z], 1)
        nv = np.array([0, -1.0, 0])
        I = np.tile(amb * 0.9, (len(X), 1)) + point_light(P, nv, L) * 1.3 + 0.12
        col = ceil_albedo(X, Z) * I
        sk, inside = skylight(X, Z, dw[ys, xs], p)
        col[inside] = sk[inside]
        put(ys, xs, col, np.where(inside, 5e5, tc[ys, xs]))
    fr.set_array(arr, depth)
    # sconce flames
    for sp in SCONCES:
        zc = cam.depth(sp)[0]
        if zc > 0.5:
            s = cam.wp(sp)
            fr.glow(s[0], s[1], min(40, 0.5 * cam.f / zc), (255, 160, 80), 0.4)
            fr.disc(s, max(1.0, 0.07 * cam.f / zc), (255, 230, 170), 6, z=float(zc))


# ---------------------------------------------------------------- per-pixel horizontal surfaces
def hsurf(fr, cam, quad, y, alb_fn, light_fn):
    """draw a horizontal quad (world corners, all at height y) per pixel: albedo(X, Z) * light(P)"""
    Pc = cam.to_cam(quad)
    if (Pc[:, 2] < cam.near).all():
        return
    if (Pc[:, 2] < cam.near).any():
        from reelkit.camera import clip_near
        Pc = clip_near(Pc, cam.near)
        if Pc is None:
            return
    pts = [tuple(q) for q in cam.proj(Pc)]
    xs_ = [q[0] for q in pts]; ys_ = [q[1] for q in pts]
    x0, x1 = max(0, int(min(xs_)) - 1), min(W, int(max(xs_)) + 2)
    y0, y1 = max(0, int(min(ys_)) - 1), min(H, int(max(ys_)) + 2)
    if x1 <= x0 or y1 <= y0:
        return
    if cam.pos[1] <= y:
        return
    im = Image.new('L', (x1 - x0, y1 - y0), 0)
    ImageDraw.Draw(im).polygon([(a - x0, b - y0) for a, b in pts], fill=255)
    m = np.asarray(im) > 0
    ys, xs = np.nonzero(m)
    if not len(xs):
        return
    ys = ys + y0; xs = xs + x0
    X, Z, d = cam.ground_hit(xs.astype(float), ys.astype(float), y0=y)
    lo, hi = quad.min(0), quad.max(0)
    ok = (X >= lo[0] - 0.02) & (X <= hi[0] + 0.02) & (Z >= lo[2] - 0.02) & (Z <= hi[2] + 0.02)
    xs, ys, X, Z, d = xs[ok], ys[ok], X[ok], Z[ok], d[ok]
    if not len(xs):
        return
    P = np.stack([X, np.full_like(X, y), Z], 1)
    col = alb_fn(X, Z) * light_fn(P)
    arr = np.asarray(fr.pil).copy()
    arr[ys, xs] = np.clip(np.floor(col / 5 + BAYER_FULL[ys, xs, None]) * 5, 0, 255).astype(np.uint8)
    fr.pil.paste(Image.fromarray(arr))
    dep = np.asarray(fr.dep).copy()
    dep[ys, xs] = d
    fr.dep.paste(Image.fromarray(dep.astype(np.float32), 'F'))


def surf_light(st, P, sunvis):
    L = st.L
    I = np.tile(L.amb + L.sky, (len(P), 1)) + point_light(P, np.array([0, 1.0, 0]), L)
    I += (sunvis * SUN[1] * 1.05)[:, None] * L.keys[0][2][None]
    return I


# board-top shadow masks, in board-local coordinates (x from the board centre, z from ZT[k])
BGRID = MaskGrid((-LY.BOARD_HX - 0.1, LY.BOARD_HX + 0.1, -LY.BOARD_HZ - 0.1, LY.BOARD_HZ + 0.1, 0.014))
_body_faces = [Q for (Q, n), m in zip(S.MET_FACES, S.MET_MATS) if m in ('body', 'front', 'cap') and Q[:, 1].max() > 0.02]
BOARD_STATIC, BOARD_AO = [], []
for k in range(T.NSEC):
    polys = []
    m = LY.MEMBERS[k]
    for i in m:
        off = np.array([LY.LX[i], 0, LY.LZ[i]])
        polys += [proj_dir(Q + off, SUN) for Q in _body_faces]
    BOARD_STATIC.append(BGRID.raster(polys, 1.2))
    x0, x1, z0, z1, res = BGRID.ext
    gz, gx = np.mgrid[0:BGRID.H, 0:BGRID.W]
    Xg, Zg = x0 + (gx + 0.5) * res, z0 + (gz + 0.5) * res
    ao = np.ones_like(Xg, dtype=np.float32)
    for i in m:
        dd = np.maximum(np.abs(Xg - LY.LX[i]) - 0.33, 0) + np.maximum(np.abs(Zg - LY.LZ[i]) - 0.27, 0)
        ao *= 1 - 0.4 * np.exp(-dd / 0.05)
    BOARD_AO.append(ao)


def board_dyn_mask(k, st, top_y):
    """this frame's rod + weight shadows on board k (projected along the sun onto the board top)"""
    m = LY.MEMBERS[k]
    x0, x1, z0, z1, res = BGRID.ext
    im = Image.new('L', (BGRID.W, BGRID.H), 0)
    dr = ImageDraw.Draw(im)
    for i in m:
        phi = st.phi[i]
        dvec = np.array([math.sin(phi), math.cos(phi), 0.0])
        piv = np.array([LY.LX[i], 0, LY.LZ[i]]) + S.PIVOT
        tip = piv + S.ROD * dvec
        wgt = piv + S.ROD * 0.52 * dvec
        q = proj_dir(np.stack([piv, tip, wgt]), SUN)
        a = ((q[:, 0] - x0) / res, (q[:, 1] - z0) / res)
        dr.line([(a[0][0], a[1][0]), (a[0][1], a[1][1])], fill=210, width=2)
        dr.ellipse([a[0][2] - 3, a[1][2] - 3, a[0][2] + 3, a[1][2] + 3], fill=230)
    return np.asarray(im, np.float32) / 255.0


def maple(X, Z, k, xoff):
    lx = X - xoff
    pw = 0.36
    ci = np.floor((Z - LY.ZT[k]) / pw)
    h = hash2(ci, 7.0 + k)
    col = np.array(S.BOARD_TOP, float)[None] * (0.9 + 0.12 * h)[:, None]
    col *= (0.96 + 0.04 * np.sin(lx * 13 + ci * 1.7))[:, None]
    seam = np.abs(((Z - LY.ZT[k]) / pw) % 1.0 - 0.5) > 0.47
    col[seam] *= 0.82
    return col


def carpet(X, Z):
    n = hash2(np.floor(X * 9), np.floor(Z * 9))
    return np.array(S.CARPET, float)[None] * (0.9 + 0.12 * n)[:, None]


def stage_boards(X, Z):
    pz = 0.25
    cz = np.floor(X / pz)
    h = hash2(cz, 4.0)
    col = np.array(S.PLAT_WOOD, float)[None] * (0.86 + 0.2 * h)[:, None]
    col[np.abs((X / pz) % 1.0 - 0.5) > 0.45] *= 0.72
    return col


# ---------------------------------------------------------------- flat-shaded objects
def draw_faces(fr, st, faces, color, grp, emit=0.0, tone=None, double=False, occ=None, sunk=1.0):
    cam, L = st.cam, st.L
    fc = [(cam.depth(Q.mean(0))[0], Q, n, j) for j, (Q, n) in enumerate(faces) if double or np.dot(n, cam.pos - Q.mean(0)) > 0]
    if not fc:
        return
    fc.sort(key=lambda a: -a[0])
    C = np.array([q.mean(0) for _, q, _, _ in fc])
    N = np.array([n for _, _, n, _ in fc])
    if double:
        flip = (N * (cam.pos[None] - C)).sum(1) < 0
        N = np.where(flip[:, None], -N, N)
    Pv = C + N * 0.02
    occ = occ or st.big
    kv = (S.sky_lit(Pv) * occ.vis(Pv, grp) * sunk)[:, None]
    if np.ndim(color) == 2:
        base = np.array([color[j] * (1 if tone is None else tone[j]) for *_, j in fc])
    else:
        base = np.array([np.asarray(color, float) * (1 if tone is None else tone[j]) for *_, j in fc])
    cols = shade(L, N, C, base, kvis=kv, emit=emit)
    for (z, Q, n, j), c in zip(fc, cols):
        fr.poly(cam, Q, c)


def on_screen(cam, ctr, rad):
    zc = cam.depth(ctr)[0]
    if zc < -rad:
        return False
    if zc < 0.3:
        return True
    s = cam.wp(ctr)
    r = rad * cam.f / zc
    return -r < s[0] < W + r and -r < s[1] < H + r


def draw_can(fr, st, cx, cy, cz, r, half_len, roll, col, grp, n=8, label=None):
    side, ends, inner = S.can_faces(cx, cy, cz, r, half_len, roll, n)
    tones = [1.0 if (j % 4) else 0.8 for j in range(n)]
    cols = [np.array(col, float) * tn for tn in tones]
    draw_faces(fr, st, side, np.array(cols), grp)
    draw_faces(fr, st, ends, (214, 220, 228), grp)
    draw_faces(fr, st, inner, (150, 156, 168), grp)
    zc = cz + half_len + 0.006 if st.cam.pos[2] > cz else cz - half_len - 0.006
    a = roll + 1.2
    tab = np.array([[cx + r * (0.12 * math.cos(a) - 0.08 * math.sin(a)), cy + r * (0.12 * math.sin(a) + 0.08 * math.cos(a)), zc],
                    [cx + r * (0.12 * math.cos(a) + 0.08 * math.sin(a)), cy + r * (0.12 * math.sin(a) - 0.08 * math.cos(a)), zc],
                    [cx + r * (0.55 * math.cos(a) + 0.08 * math.sin(a)), cy + r * (0.55 * math.sin(a) - 0.08 * math.cos(a)), zc],
                    [cx + r * (0.55 * math.cos(a) - 0.08 * math.sin(a)), cy + r * (0.55 * math.sin(a) + 0.08 * math.cos(a)), zc]])
    fr.poly(st.cam, tab, (96, 100, 112))


def draw_giant_can(fr, st, cx, z, roll, col, grp):
    r, hl = LY.GCAN_R, LY.GCAN_L / 2
    n = 12
    cy = r
    # bands along z: rim | colour | white label | colour | rim
    zs = [z - hl, z - hl + 0.18, z - 0.55, z + 0.55, z + hl - 0.18, z + hl]
    bc = [S.CAN_END, col, (238, 234, 222), col, S.CAN_END]
    faces, cols = [], []
    for (za, zb), c in zip(zip(zs, zs[1:]), bc):
        fcs = S.cyl_z(cx, cy, za, zb, r if c != S.CAN_END else r * 0.97, n, rot=roll, cap=False)
        for j, f_ in enumerate(fcs):
            faces.append(f_)
            cols.append(np.array(c, float) * (1.0 if j % 3 else 0.86))
    draw_faces(fr, st, faces, np.array(cols), grp)
    ends = S.cyl_z(cx, cy, z - hl, z + hl, r * 0.97, n, rot=roll, cap=True)[n:]
    draw_faces(fr, st, ends, S.CAN_END, grp)
    inner = S.cyl_z(cx, cy, z - hl - 0.004, z + hl + 0.004, r * 0.8, n, rot=roll, cap=True)[n:]
    draw_faces(fr, st, inner, np.array(S.CAN_END) * 0.82, grp)
    # pull tab on the visible lid
    zc = z + hl + 0.01 if st.cam.pos[2] > z else z - hl - 0.01
    a = roll + 1.2
    tab = np.array([[cx + 0.3 * math.cos(a) - 0.1 * math.sin(a), cy + 0.3 * math.sin(a) + 0.1 * math.cos(a), zc],
                    [cx + 0.3 * math.cos(a) + 0.1 * math.sin(a), cy + 0.3 * math.sin(a) - 0.1 * math.cos(a), zc],
                    [cx + 0.62 * math.cos(a) + 0.1 * math.sin(a), cy + 0.62 * math.sin(a) - 0.1 * math.cos(a), zc],
                    [cx + 0.62 * math.cos(a) - 0.1 * math.sin(a), cy + 0.62 * math.sin(a) + 0.1 * math.cos(a), zc]])
    fr.poly(st.cam, tab, (170, 176, 184))


# ---------------------------------------------------------------- metronomes
def led_color(i, t):
    """LED show during the song: tiers stack into a 10 x 20 display seen from the front (row 0 = bass front row)"""
    if i == LY.SOLO:
        return T.GOLD, 0.7 + 0.3 * T.beat_env(t, 0.15)
    b = (t - T.T_DROP) / T.BEAT
    bi = int(math.floor(b))
    fr_ = b - bi
    bar = bi // 4
    k = LY.SEC[i]
    r = k * 4 + LY.ROW[i]                       # display row, 0 = bottom
    c = LY.COL[i]
    pulse = math.exp(-fr_ * 3.0)
    cx, cy = 4.5, 9.5
    if bar == 0:                                   # burst: rings race out from the soloist
        dist = math.hypot(c - 4.5, r + 2.0)
        k_ = int(math.floor(dist * 0.8 - b * 3.0)) % len(LED)
        return LED[k_], 0.8 + 0.2 * pulse
    if bar == 1:                                   # each beat lights the next section up the loft
        on = (bi % 4) == min(3, k) or (bi % 4 == 3 and k == 4)
        sc = T.SEC_COL[k]
        return (tuple(min(255, v * 1.1 + 30) for v in sc), 0.9 * pulse + 0.3) if on else (sc, 0.25)
    if bar == 2:                                   # checkerboard swaps on the beat
        on = ((r + c) % 2 == 0) ^ (bi % 2 == 1)
        return (LED[3], 0.6 + 0.4 * pulse) if on else (LED[2], 0.35)
    if bar == 3:                                   # equaliser: column heights jump on every beat
        hgt = 4 + int(14 * hash2(np.float64(c), np.float64(bi))) * (0.5 + 0.5 * pulse)
        on = r < hgt
        return (LED[[0, 0, 6, 6, 5, 5, 4, 4, 3, 3][min(9, r // 2)]], 0.85) if on else ((40, 60, 70), 0.15)
    if bar == 4:                                   # diagonal rainbow stripes
        k_ = int(math.floor((c + r) * 0.5 - b * 2)) % len(LED)
        return LED[k_], 0.75 + 0.25 * pulse
    if bar == 5:                                   # a heart
        HEART = ["..........", ".##....##.", "####..####", "##########", "##########", ".########.", "..######..", "...####...", "....##....", ".........."]
        rr = 14 - r
        on = 0 <= rr < 10 and HEART[rr][c] == '#'
        return (LED[3], 0.75 + 0.25 * pulse) if on else ((40, 50, 80), 0.2)
    if bar == 6:                                   # rings ripple out from the centre
        dist = math.hypot((c - cx) * 1.3, r - cy)
        k_ = int(math.floor(dist - (b - 24) * 2.2)) % len(LED)
        return LED[k_], 0.75 + 0.25 * pulse
    if bar == 7:                                   # a radar sweep
        ang = math.atan2(r - cy, (c - cx) * 1.6)
        a = (ang - b * math.pi / 2) % (2 * math.pi)
        return (LED[1], 0.25 + 0.75 * math.exp(-a * 1.2)) if a < 2.5 else ((30, 50, 70), 0.2)
    if bar == 8:                                   # section stripes pulse with the bells
        sc = T.SEC_COL[k]
        return tuple(min(255, v * 1.1 + 20) for v in sc), 0.5 + 0.5 * pulse
    k_ = int(math.floor(r * 0.5 - b * 4)) % len(LED)   # rainbow rushing up into the thump
    return LED[k_], 0.8 + 0.2 * pulse


def met_prepare(st):
    """shade every visible face of every metronome in one go"""
    cam, L, t = st.cam, st.L, st.t
    sc = np.r_[np.ones(LY.NR), LY.SOLO_S]
    C = S.MET_C[None] * sc[:, None, None] + st.P[:, None]           # (N, F, 3)
    vis = ((cam.pos[None, None] - C) * S.MET_N[None]).sum(-1) > 0
    vis[:, S.PANEL] = vis[:, S.FRONT]
    ii, jj = np.nonzero(vis)
    Cv = C[ii, jj]
    Nv = S.MET_N[jj]
    base = np.array([S.MET_COLS[LY.SEC_ALL[i] if i < LY.NR else -1][j] for i, j in zip(ii, jj)])
    Pv = Cv + Nv * 0.02
    kv = S.sky_lit(Pv)
    m = kv > 0
    if m.any():
        kv[m] *= st.occ.vis(Pv[m], 1000 + ii[m])
    cols = shade(L, Nv, Cv, base, kvis=kv[:, None])
    st.met_vis = vis
    st.met_cols = {}
    for (i, j), c in zip(zip(ii, jj), cols):
        st.met_cols[(i, j)] = c
    st.scale = sc


FRONT_FACES = {j for j, (Q, n) in enumerate(S.MET_FACES) if S.MET_MATS[j] in ('body', 'front') and n[2] > 0.3}


def draw_metronome(fr, st, i, glows, rel):
    cam, t = st.cam, st.t
    s = st.scale[i]
    base = st.P[i]
    if not on_screen(cam, base + [0, 0.6 * s, 0], 1.0 * s):
        return
    in_drop = T.T_DROP <= t < T.T_JOLT
    fogk = 0.0
    vis = st.met_vis[i]
    Q = S.MET_Q * s + base
    zs = cam.depth(S.MET_C * s + base)
    order = [j for j in np.argsort(-zs) if vis[j] and j != S.PANEL] + ([S.PANEL] if vis[S.PANEL] else [])
    led = led_color(i, t) if in_drop else None
    for j in order:
        col = st.met_cols[(i, j)]
        if led is not None and j in FRONT_FACES:
            lc, lb = led
            on = min(1.0, max(0.0, (lb - 0.2) / 0.7))
            dark = col * 0.32 + np.array([10, 14, 30.0])
            col = dark * (1 - on) + np.array(lc, float) * (1.05 if j == S.FRONT else 0.8) * on
        if j == S.PANEL and led is not None:
            lc, lb = led
            col = np.array(lc, float) * (0.15 + 0.95 * lb)
            pts = cam.wp(Q[j][:S.MET_NV[j]])
            pc = pts.mean(0)
            pr = max(2.0, abs(pts[0][0] - pts[1][0]) * 0.9)
            glows.append((pc[0], pc[1], pr * 2.4, lc, 0.32 * lb))
        fr.poly(cam, Q[j][:S.MET_NV[j]], col)
    if not vis[S.FRONT]:
        return
    # pendulum: rod, sliding weight, tip light
    phi = st.phi[i]
    piv = base + s * S.PIVOT
    dvec = np.array([math.sin(phi), math.cos(phi), 0.0])
    tip = piv + s * S.ROD * dvec
    wgt = piv + s * S.ROD * 0.52 * dvec
    zc = cam.depth(piv)[0]
    if zc < 0.2:
        return
    sp = cam.wp(np.stack([piv, tip, wgt]))
    ppu = cam.f / zc
    rodw = max(1, int(round(0.022 * ppu * s)))
    fr.draw.line([tuple(sp[0]), tuple(sp[1])], fill=(206, 212, 218), width=rodw)
    fr.ddraw.line([tuple(sp[0]), tuple(sp[1])], fill=float(zc), width=rodw)
    wr = 0.05 * ppu * s
    sd = sp[1] - sp[0]
    ln = np.linalg.norm(sd) + 1e-6
    along = sd / ln
    perp = np.array([-along[1], along[0]])
    cw = sp[2]
    quad = [cw + perp * wr * 1.3 - along * wr * 0.8, cw - perp * wr * 1.3 - along * wr * 0.8,
            cw - perp * wr * 0.9 + along * wr * 0.8, cw + perp * wr * 0.9 + along * wr * 0.8]
    fr.screen_poly(quad, (70, 74, 92) if i != LY.SOLO else (150, 110, 40), z=zc)
    # tip
    if in_drop:
        tcol, tint = led_color(i, t)
        tint = 0.6 + 0.4 * tint
    else:
        tcol = sync_color(rel[i])
        tint = 0.55 + 0.45 * abs(math.sin(st.theta[i])) ** 8
    tr = max(1.6, 0.085 * ppu * s) * (1.3 if in_drop else 1.0)
    tc = np.array(tcol, float)
    core = np.clip(tc * (0.6 + 0.45 * tint), 0, 255)
    x, y = sp[1]
    fr.screen_poly([(x + tr * math.cos(k * math.pi / 4 + 0.39), y + tr * math.sin(k * math.pi / 4 + 0.39)) for k in range(8)], core, z=zc)
    glows.append((x, y, min(30, tr * 3.2), tc, (0.34 if not in_drop else 0.45) * tint))


# ---------------------------------------------------------------- the loft
def draw_loft(fr, st, glows):
    cam, t = st.cam, st.t
    py, pdx = st.py, st.pdx
    rel = MO.rel_phase(t)
    # giant cans
    gy, gdx, gx, groll = st.pstate
    if gx is not None:
        items = []
        for ci, sx in enumerate((-1, 1)):
            for zi, z in enumerate(LY.GCAN_Z):
                xc = pdx * 0.5 + sx * gx
                items.append((np.linalg.norm(np.array([xc, LY.GCAN_R, z]) - cam.pos), xc, z, S.CAN_COLS[(ci + zi) % 2]))
        for d, xc, z, col in painter(items):
            if on_screen(cam, np.array([xc, LY.GCAN_R, z]), 2.2):
                draw_giant_can(fr, st, xc, z, groll * (1 if xc < 0 else -1), col, 50)
    # platform: sides, then its top per pixel
    pf = box(pdx, py + LY.PLAT_T / 2, 0.5 * (LY.PLAT_Z0 + LY.PLAT_Z1), LY.PLAT_HX, LY.PLAT_T / 2, 0.5 * (LY.PLAT_Z1 - LY.PLAT_Z0))
    draw_faces(fr, st, [f_ for f_ in pf if abs(f_[1][1]) < 0.5], (84, 58, 44), 1)
    yt = py + LY.PLAT_T
    plat_occ = st.big.subset(lambda g: 10 <= g < 25 or g == 40)

    def plat_light(P):
        sv = S.sky_lit(P)
        m = sv > 0
        if m.any():
            sv[m] *= plat_occ.vis(P[m] + [0, 0.01, 0], 1)
        return surf_light(st, P, sv)
    hsurf(fr, cam, np.array([[pdx - LY.PLAT_HX, yt, LY.PLAT_Z0], [pdx + LY.PLAT_HX, yt, LY.PLAT_Z0], [pdx + LY.PLAT_HX, yt, LY.PLAT_Z1], [pdx - LY.PLAT_HX, yt, LY.PLAT_Z1]]),
          yt, stage_boards, plat_light)
    # footlight bulbs along the front edge
    for x in np.arange(-9.0, 9.01, 1.5):
        bp = np.array([pdx + x, yt + 0.06, LY.PLAT_Z1 + 0.05])
        zc = cam.depth(bp)[0]
        if zc > 0.4:
            s_ = cam.wp(bp)
            fr.disc(s_, max(1.0, 0.06 * cam.f / zc), (255, 226, 160), 6, z=float(zc))
            glows.append((s_[0], s_[1], min(24, 0.35 * cam.f / zc), (255, 170, 90), 0.35))
    # tiers, back to front
    for k in range(T.NSEC - 1, -1, -1):
        y, dx, cx, roll = st.boards[k]
        for Q, n, col in S.step_faces(k):
            draw_faces(fr, st, [(Q + [pdx, py, 0], n)], col, 10 + k)
        ys = py + LY.YT[k]
        z0, z1 = LY.ZT[k] - LY.STEP_HZ, LY.ZT[k] + LY.STEP_HZ

        step_occ = st.big.subset(lambda g, k=k: g in (20 + k, 30 + k, 11 + k))

        def step_light(P, k=k, occ=step_occ):
            sv = S.sky_lit(P)
            m = sv > 0
            if m.any():
                sv[m] *= occ.vis(P[m] + [0, 0.01, 0], 10 + k)
            return surf_light(st, P, sv)
        hsurf(fr, cam, np.array([[pdx - LY.STEP_HX, ys, z0], [pdx + LY.STEP_HX, ys, z0], [pdx + LY.STEP_HX, ys, z1], [pdx - LY.STEP_HX, ys, z1]]),
              ys, carpet, step_light)
        for (pk, x, z, sc_) in S.PLANTS:
            if pk == k:
                pot, crown = S.plant_faces(k, 0, 0, sc_)
                off = np.array([pdx + x, ys, z])
                draw_faces(fr, st, S.shift(pot, off), (150, 84, 58), 70 + k)
                draw_faces(fr, st, S.shift(crown, off), (78, 138, 84), 70 + k)
        # riser cans
        if cx is not None:
            hop = MO.can_hop(t)
            for ci, sx in enumerate((-1, 1)):
                xc = pdx + dx * 0.5 + sx * cx
                if abs(xc - pdx) > LY.STEP_HX + 0.2 and hop <= 0:
                    continue
                draw_can(fr, st, xc, ys + LY.CAN_R + hop, LY.ZT[k], LY.CAN_R, 2.25, roll * sx, S.CAN_COLS[ci], 30 + k)
        # board edges + top
        yb = ys + y
        bo = np.array([pdx + dx, yb, LY.ZT[k]])
        draw_faces(fr, st, S.shift(S.BOARD_SIDES, bo), T.SEC_COL[k], 20 + k)
        top = yb + LY.BOARD_T
        xoff = pdx + dx
        dyn = None
        if cam.pos[1] > top:
            dyn = board_dyn_mask(k, st, top)

        def board_light(P, k=k, xoff=xoff, dyn=dyn):
            lx, lz = P[:, 0] - xoff, P[:, 2] - LY.ZT[k]
            sh = np.maximum(BGRID.sample(BOARD_STATIC[k], lx, lz), BGRID.sample(dyn, lx, lz) * 0.9)
            sv = S.sky_lit(P) * (1 - sh)
            I = surf_light(st, P, sv)
            return I * BGRID.sample(BOARD_AO[k], lx, lz)[:, None]
        hsurf(fr, cam, np.array([[xoff - LY.BOARD_HX, top, LY.ZT[k] - LY.BOARD_HZ], [xoff + LY.BOARD_HX, top, LY.ZT[k] - LY.BOARD_HZ],
                                  [xoff + LY.BOARD_HX, top, LY.ZT[k] + LY.BOARD_HZ], [xoff - LY.BOARD_HX, top, LY.ZT[k] + LY.BOARD_HZ]]),
              top, lambda X, Z, k=k, xoff=xoff: maple(X, Z, k, xoff), board_light)
        # the metronomes of this tier, far to near
        m = LY.MEMBERS[k]
        zc = cam.depth(st.P[m] + [0, 0.5, 0])
        for i in m[np.argsort(-zc)]:
            draw_metronome(fr, st, i, glows, rel)
    # podium + soloist
    po = np.array([pdx, py, 0])
    draw_faces(fr, st, S.shift(S.POD_FACES, po), S.POD_COL, 40)
    draw_faces(fr, st, S.shift(S.POD_TRIM, po), S.POD_TRIM_COL, 40)
    draw_metronome(fr, st, LY.SOLO, glows, rel)
    return rel


def draw_pawn(fr, st, pw):
    body, head, hat = S.pawn_faces(pw, st.t)
    g = 5000 + id(pw) % 997
    draw_faces(fr, st, body, pw['col'], g)
    draw_faces(fr, st, head, S.SKIN, g)
    if hat:
        draw_faces(fr, st, hat, (36, 34, 40) if pw['hat'] == 1 else (120, 80, 60), g)


def draw_front(fr, st, glows):
    cam, t = st.cam, st.t
    items = [(np.linalg.norm(o.ctr - cam.pos), 'obj', o) for o in S.FRONT_OBJS]
    items.append((np.linalg.norm(S.CAT_P - cam.pos) - 0.3, 'cat', None))
    for o in S.SEATS:
        items.append((np.linalg.norm(o.ctr - cam.pos) + 0.6, 'obj', o))
    for pw in S.PAWNS:
        items.append((np.linalg.norm(pw['p'] + [0, 0.25, 0] - cam.pos), 'pawn', pw))
    for _, kind, o in painter(items):
        if kind == 'obj':
            if on_screen(cam, o.ctr, np.linalg.norm(o.hi - o.lo) * 0.6):
                draw_faces(fr, st, o.faces, o.color, 60 + o.grp, tone=o.tone)
        elif kind == 'cat':
            if on_screen(cam, S.CAT_P, 0.8):
                cols = {'body': (226, 138, 58), 'head': (232, 146, 64), 'ears': (210, 120, 50), 'tail': (214, 126, 52)}
                for name, fcs in S.cat_faces(t).items():
                    draw_faces(fr, st, fcs, cols[name], 777, double=(name in ('ears', 'tail')))
        else:
            if on_screen(cam, o['p'] + [0, 0.25, 0], 0.5):
                draw_pawn(fr, st, o)
    # truss + spot fixtures
    if on_screen(cam, np.array([0, S.TRUSS_Y, S.TRUSS_Z]), 11):
        draw_faces(fr, st, S.TRUSS, (52, 52, 58), 90)
        for sp, aim in S.FIXTURES:
            body = prism(sp[0], sp[2], 0.22, sp[1] - 0.1, sp[1] + 0.35, 8)
            draw_faces(fr, st, body, (40, 40, 46), 91)
            lens = sp + aim * 0.15
            zc = cam.depth(lens)[0]
            if zc > 0.5:
                s_ = cam.wp(lens)
                fr.disc(s_, max(1.0, 0.16 * cam.f / zc), (255, 236, 190), 8, z=float(zc))
                glows.append((s_[0], s_[1], min(50, 1.1 * cam.f / zc), (255, 190, 110), 0.5 * spot_level(t)))


# ---------------------------------------------------------------- sunbeam (volumetric, quarter res) + motes
QH, QW = H // 4, W // 4
_QY, _QX = np.mgrid[0:QH, 0:QW].astype(np.float32)


def sunbeam(st, depth):
    cam = st.cam
    dirs = cam.pixel_rays(_QX * 4 + 2, _QY * 4 + 2)
    dq = np.minimum(depth[2::4, 2::4], 45.0)
    n = 16
    acc = np.zeros((QH, QW), np.float32)
    jit = BAYER_FULL[2::4, 2::4] - 0.5
    loft = Boxes()
    py, pdx = st.py, st.pdx
    loft.add([pdx - LY.STEP_HX, 0, LY.PLAT_Z0], [pdx + LY.STEP_HX, py + LY.YT[4] + 2.4, LY.ZT[4] + LY.STEP_HZ], 1)
    loft.add([pdx - LY.STEP_HX, 0, LY.ZT[3] - LY.STEP_HZ], [pdx + LY.STEP_HX, py + LY.YT[2] + 2.4, LY.ZT[1] + LY.STEP_HZ], 1)
    loft.add([pdx - LY.STEP_HX, 0, LY.ZT[0] - LY.STEP_HZ], [pdx + LY.STEP_HX, py + LY.YT[0] + 2.4, LY.ZT[0] + LY.STEP_HZ], 1)
    loft.done()
    for i in range(n):
        s = (i + 0.5 + jit) / n * dq
        P = cam.pos[None, None] + dirs * s[..., None]
        Pf = P.reshape(-1, 3)
        inside = (np.abs(Pf[:, 0]) < S.HX) & (Pf[:, 2] > S.HZ0) & (Pf[:, 2] < S.HZ1) & (Pf[:, 1] > 0) & (Pf[:, 1] < S.HY)
        lit = S.sky_lit(Pf) * inside
        m = lit > 0
        if m.any():
            lit[m] *= loft.vis(Pf[m], -9)
        acc += lit.reshape(QH, QW) * dq / n
    beam = 1 - np.exp(-acc * 0.06)
    im = Image.fromarray((beam * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(1.4))
    return np.asarray(im.resize((W, H), Image.BILINEAR), np.float32) / 255.0


def draw_motes(img, st, depth):
    cam, t = st.cam, st.t
    Mo = S.MOTES
    w = 2 * math.pi / 60.0
    P = Mo + np.stack([0.15 * np.sin(w * 3 * t + Mo[:, 1] * 5), 0.2 * np.sin(w * 2 * t + Mo[:, 0] * 3), 0.15 * np.cos(w * 4 * t + Mo[:, 2] * 4)], 1)
    lit = S.sky_lit(P)
    zc = cam.depth(P)
    scr = cam.wp(P)
    tw = 0.55 + 0.45 * np.sin(w * 17 * t + Mo[:, 0] * 40)
    for z, l, (x, y), a in zip(zc, lit, scr, tw):
        if l <= 0.5 or z < 0.8:
            continue
        xi, yi = int(x), int(y)
        if 0 <= xi < W and 0 <= yi < H and depth[yi, xi] > z:
            img[yi, xi] = img[yi, xi] * (1 - 0.5 * a) + np.array([255, 242, 206.0]) * 0.5 * a


# ---------------------------------------------------------------- focus: every metronome on screen stays sharp
def subject_points(st):
    P = st.P
    tops = P + [0, S.MET_H, 0]
    pts = np.concatenate([P, tops, st.P[LY.SOLO:LY.SOLO + 1] + [0, S.MET_H * LY.SOLO_S, 0]])
    s = st.cam.wp(pts)
    z = st.cam.depth(pts)
    on = (z > 0.3) & (s[:, 0] > -10) & (s[:, 0] < W + 10) & (s[:, 1] > -10) & (s[:, 1] < H + 10)
    return pts[on], z[on]


def focus(st):
    cam = st.cam
    zf = float(cam.depth(cam.target)[0])
    pts, zr = subject_points(st)
    if len(zr):
        sharp = float(np.clip(np.abs(1 - zf / np.maximum(zr, 0.2)).max() + 0.03, 0.05, 0.95))
    else:
        sharp = 0.2
    K = 0.012 * cam.f * min(1.3, 8.0 / zf)
    return zf, sharp, K


def subject_blur(st):
    zf, sharp, K = focus(st)
    pts, zr = subject_points(st)
    if not len(zr):
        return 0.0
    return float(coc_map(zr, zf, K, sharp).max())


# ---------------------------------------------------------------- UI
def center_text(img, txt, x, y, col, scale=1, alpha=1.0):
    m = text_mask(txt, scale)
    stamp(img, m, int(round(x - m.shape[1] / 2)), int(round(y - m.shape[0] / 2)), col, alpha=alpha)


HUD_Y = 496
HUD_WIN = [(8.6, 24.9), (29.3, T.T_DROP + 2.0)]      # hidden while the risers' cans / the giant cans roll in


def hud_offset(t):
    a = max(min(ease((t - a0) / 0.35), 1 - ease((t - a1) / 0.35)) for a0, a1 in HUD_WIN)
    return int(round((1 - a) * 160))


def draw_hud(img, st):
    t = st.t
    off = hud_offset(t)
    if off >= 150:
        return img
    y0 = HUD_Y + off
    x0, x1, hh = 14, 346, 78
    # translucent panel
    pil = Image.fromarray(img)
    lay = pil.copy()
    d = ImageDraw.Draw(lay)
    panel(d, x0, y0, x1, y0 + hh, fill=(14, 16, 32), edge=(70, 84, 124))
    pil = Image.blend(pil, lay, 0.82)
    d = ImageDraw.Draw(pil)
    th = st.theta
    phase2 = t >= T.PLAT[1] - 0.4
    locked = MO.sections_locked(t)
    rings = T.NSEC + (1 if phase2 else 0)
    span = (x1 - x0 - 12) / rings
    cy = y0 + 45
    for k in range(rings):
        cx = x0 + 6 + span * (k + 0.5)
        rr = 15 if k < T.NSEC else 11
        col = T.SEC_COL[k] if k < T.NSEC else T.GOLD
        d.ellipse([cx - rr, cy - rr, cx + rr, cy + rr], outline=tuple(int(c * 0.7) for c in col))
        d.ellipse([cx - 1, cy - 1, cx + 1, cy + 1], fill=tuple(int(c * 0.7) for c in col))
        idx = LY.MEMBERS[k] if k < T.NSEC else [LY.SOLO]
        z = np.exp(1j * th[idx]).mean()
        for i in idx:
            a = th[i] - math.pi / 2
            px, py_ = cx + rr * math.cos(a), cy + rr * math.sin(a)
            dc = col if k < T.NSEC else (CORAL if t < MO.T_SOLO else MINT)
            sz = 1 if k < T.NSEC else 2
            d.rectangle([px - sz, py_ - sz, px + sz, py_ + sz], fill=tuple(int(c) for c in dc))
        ok = (k < T.NSEC and locked[k]) or (k == T.NSEC and t >= MO.T_SOLO)
        ax = cx + (rr - 3) * abs(z) * math.cos(np.angle(z) - math.pi / 2)
        ay = cy + (rr - 3) * abs(z) * math.sin(np.angle(z) - math.pi / 2)
        d.line([cx, cy, ax, ay], fill=MINT if ok else (150, 160, 180), width=1)
    out = np.asarray(pil).copy()
    for k in range(rings):
        cx = x0 + 6 + span * (k + 0.5)
        lab = T.SEC_SHORT[k] if k < T.NSEC else 'SOLO'
        ok = locked[k] if k < T.NSEC else t >= MO.T_SOLO
        center_text(out, lab, cx, y0 + 69, MINT if ok else (170, 178, 200), 1)
    if not phase2:
        n = sum(locked)
        stamp(out, text_mask('SECTIONS LOCKED', 2), x0 + 9, y0 + 9, (170, 190, 210))
        center_text(out, f'{n}/5', x1 - 30, y0 + 15, MINT if n == 5 else AMBER, 3)
    else:
        n = MO.count_in_sync(t)
        stamp(out, text_mask('IN SYNC', 2), x0 + 9, y0 + 9, (170, 190, 210))
        center_text(out, f'{n}/200', x1 - 52, y0 + 15, MINT if n == 200 else AMBER, 3)
    return out


def section_tags(img, st):
    """3-6 s: each section's name floats beside its riser as the camera sweeps past; lock ticks later"""
    t, cam = st.t, st.cam
    for k in range(T.NSEC):
        show = 3.2 <= t < 6.1
        lk = MO.T_LOCK[k]
        locked_tag = lk <= t < lk + 2.2 and t < T.PLAT[0]
        if not (show or locked_tag):
            continue
        y = st.py + LY.YT[k] + st.boards[k][0] + LY.BOARD_T + 1.55
        p = np.array([st.pdx + st.boards[k][1] + 3.0, y, LY.ZT[k] + LY.BOARD_HZ])
        if cam.depth(p)[0] < 1:
            continue
        s = cam.wp(p)
        if not (-40 < s[0] < W + 40 and 160 < s[1] < H - 30):
            continue
        txt = T.SEC_NAMES[k] + (' ✓' if locked_tag else '')
        al = min(1.0, (t - (3.2 if show else lk)) / 0.15)
        hw = len(txt) * 6 + 4
        x = min(max(s[0], hw + 4), W - hw - 4)
        center_text(img, txt, x, s[1], MINT if locked_tag else tuple(min(255, c + 40) for c in T.SEC_COL[k]), 2, alpha=al)


def solo_marker(img, st):
    t, cam = st.t, st.cam
    if not (34.0 <= t < T.T_DROP + 0.6):
        return
    p = st.P[LY.SOLO] + [0, S.MET_H * LY.SOLO_S + 0.35 + 0.06 * math.sin(t * 9), 0]
    if cam.depth(p)[0] < 0.5:
        return
    x, y = cam.wp(p)
    sz = 9
    col = CORAL if t < MO.T_SOLO else MINT
    pil = Image.fromarray(img)
    d = ImageDraw.Draw(pil)
    tri = [(x - sz, y - sz * 1.2), (x + sz, y - sz * 1.2), (x, y)]
    d.polygon([(a + 1, b + 1) for a, b in tri], fill=INK)
    d.polygon(tri, fill=col)
    img[:] = np.asarray(pil)


def big_text(img, t):
    a0, a1 = T.BIG_TEXT
    if not (a0 <= t < a1):
        return
    u = t - a0
    sc = 9 if u < 0.25 else 8
    al = 1 - ss(a1 - 0.3, a1, t)
    center_text(img, '200/200', W / 2, 300 - (sc - 8) * 3, MINT, sc, alpha=al)
    center_text(img, 'IN SYNC!', W / 2, 352, CREAM, 3, alpha=al)


# ---------------------------------------------------------------- dust burst when the platform slams down
_dr = np.random.default_rng(58)
DUST = []
for _ in range(110):
    side = _dr.integers(3)
    if side == 0:
        p0 = np.array([_dr.uniform(-LY.PLAT_HX, LY.PLAT_HX), 0.1, LY.PLAT_Z1 + 0.1]); v = np.array([_dr.normal(0, 0.6), _dr.uniform(0.6, 1.8), _dr.uniform(1.0, 3.0)])
    else:
        sx = -1 if side == 1 else 1
        p0 = np.array([sx * (LY.PLAT_HX + 0.1), 0.1, _dr.uniform(LY.PLAT_Z0, LY.PLAT_Z1)]); v = np.array([sx * _dr.uniform(1.0, 3.0), _dr.uniform(0.6, 1.8), _dr.normal(0, 0.6)])
    DUST.append((p0, v, _dr.uniform(0.6, 1.1), _dr.uniform(0.7, 1.0)))


def dust_glows(st, glows):
    u = st.t - T.T_JOLT
    if not (0 <= u < 1.1):
        return
    cam = st.cam
    for p0, v, life, b in DUST:
        if u > life:
            continue
        k = 1 - math.exp(-u * 3.0)
        P = p0 + v * k / 3.0
        zc = cam.depth(P)[0]
        if zc < 0.5:
            continue
        s = cam.wp(P)
        a = (1 - u / life) ** 1.5 * b
        glows.append((s[0], s[1], min(40, (0.25 + 0.5 * u) * cam.f / zc), (150, 128, 104), 0.22 * a))


# ---------------------------------------------------------------- render
def render(f):
    t = f / FPS
    cam = camera(t)
    st = State(t, cam)
    fr = Frame((40, 30, 36))
    shell_pass(fr, st)
    glows = []
    met_prepare(st)
    draw_loft(fr, st, glows)
    draw_front(fr, st, glows)
    dust_glows(st, glows)
    img, depth = fr.array(), fr.depth()
    zf, sharp, K = focus(st)
    img = dof(img, coc_map(depth, zf, K, sharp=sharp))
    beam = sunbeam(st, depth)
    img = img + beam[..., None] * np.array([255, 226, 176.0])[None, None] * 0.26
    draw_motes(img, st, depth)
    img = apply_glows(img, fr.glows + glows)
    # the soloist's spotlight: the hall dims around it, a cone of light from the truss
    if 33.8 <= t < T.T_DROP + 0.3:
        a = ss(33.8, 34.6, t) * (1 - ss(T.T_DROP - 0.05, T.T_DROP + 0.25, t))
        p = st.P[LY.SOLO] + [0, 0.8, 0]
        if cam.depth(p)[0] > 0.5:
            s_ = cam.wp(p)
            spot = np.exp(-((XX - s_[0]) ** 2 + ((YY - s_[1]) * 0.8) ** 2) / (2 * (0.9 * cam.f / cam.depth(p)[0]) ** 2))
            img = img * (1 - a * 0.42 * (1 - spot))[..., None] + spot[..., None] * np.array([60, 48, 30.0]) * a
    img = grade(bloom(np.clip(img, 0, 255), thr=222, k=0.3), contrast=0.2, sat=1.14)
    img = np.clip(img * vignette(0.7, 0.6, 0.35) * top_scrim(0.55, 30, 150), 0, 255)
    # drop flash
    if T.T_DROP <= t < T.T_DROP + 0.25:
        k = (1 - (t - T.T_DROP) / 0.25) ** 2 * 0.55
        img = img * (1 - k) + np.array([230, 255, 240.0]) * k
    out = np.clip(img, 0, 255).astype(np.uint8)
    # UI
    section_tags(out, st)
    solo_marker(out, st)
    out = draw_hud(out, st)
    draw_captions(out, t, T.CAPTIONS, hi=AMBER, alt=MINT)
    big_text(out, t)
    return out
