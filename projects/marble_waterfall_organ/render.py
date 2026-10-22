"""Marble Waterfall Organ renderer.

void (gradient + chord-tinted nebula + parallax stars + comets + orb) -> far floating shapes ->
per-pixel plinth top (orb shadows of the whole machine, lantern + hit light, contact AO) ->
machine faces + near shapes (one painter list; static orb visibility cached) -> marbles z-tested
against the depth buffer -> locked depth of field on the machine -> glows, trails, sparks ->
god rays from the orb -> bloom, grade, vignette, scrim -> HUD (gate pips) + captions.

    python sim.py && python audio.py
    python -m reelkit.sheet render.py test/sheet.png 0.5 4.5 7 12 16 21 27 33 37.5 40.2 45 50 54 58.5
    python -m reelkit.video render.py audio.wav marble_waterfall_organ.mp4"""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, dirv
from reelkit.camera import interp_keys, clip_near
from reelkit.raster import Frame, add_glow
from reelkit.lighting import Lights, shade, fog
from reelkit.shadows import MaskGrid, directional_mask, Occluders, contact_ao
from reelkit.lens import coc_map, dof
from reelkit.post import god_rays, bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, panel, CREAM, AMBER
from reelkit.font import text_mask, stamp
from reelkit.geometry import face, icosa
import timeline as T
import machine as M
import scene as S
import sim as SIM

D = SIM.load()
FLASH = D['flash']
HITS = D['hits']
LANDS = D['lands']
COLS = np.array(T.COL, float)
KEY = S.KEY

# ================================================================== static faces (machine + plinth)
_FQ, _FC, _FN, _FB, _FEL, _FEM, _FDB, _FGL, _FGR, _KIND = [], [], [], [], [], [], [], [], [], []
for pi, p in enumerate(M.PARTS):
    r = np.random.default_rng(pi * 13 + 5)
    kind = M.ELEMS[p.elem].kind if p.elem >= 0 else ''
    for Q, n in p.faces:
        _FQ.append(np.asarray(Q, float)); _FC.append(np.asarray(Q, float).mean(0)); _FN.append(np.asarray(n, float))
        _FB.append(p.color * (1 + r.uniform(-0.045, 0.045))); _FEL.append(p.elem); _FEM.append(p.emit)
        _FDB.append(p.double); _FGL.append(p.glass); _FGR.append(p.grp); _KIND.append(kind)
FC = np.array(_FC); FN = np.array(_FN); FB = np.array(_FB); FEL = np.array(_FEL); FEM = np.array(_FEM)
FDB = np.array(_FDB); FGL = np.array(_FGL); FGR = np.array(_FGR)
NV = np.array([len(q) for q in _FQ]); OFF = np.concatenate([[0], np.cumsum(NV)])
FV = np.concatenate(_FQ)
DIP = np.array([{'bar': 0.012, 'bell': 0.012, 'xylo': 0.01, 'kick': 0.01, 'tom': 0.008, 'snare': 0.008, 'hat': 0.01,
                 'crash': 0.02, 'ride': 0.016, 'bowl': 0.0, 'pipe': 0.0}.get(k, 0.0) for k in _KIND])


class _Occ:
    def __init__(self, Qs, grp):
        V = np.concatenate(Qs)
        self.lo, self.hi, self.grp, self.cast = V.min(0), V.max(0), grp, True


_NOOCC = ('plinth', 'basin_in', 'basin_out', 'basin_lip', 'housing', 'housing_band', 'orb', 'lantern')
OCC = Occluders([_Occ([Q for Q, n in p.faces], p.grp) for p in M.PARTS if p.cast and p.name not in _NOOCC])
FKV = OCC.vis(FC + FN * 0.02, FGR, d=KEY)
FKV = np.where(FN @ KEY > 0, FKV, 1.0)

# plinth top: shadow mask of the whole machine under the orb (static, cached once)
TOP_GRID = MaskGrid((-1.9, 1.9, -1.9, 1.9, 0.008))
_casters = [Q for p in M.PARTS if p.cast and p.name not in ('plinth', 'basin_in', 'basin_out', 'basin_lip') for Q, n in p.faces]
TOP_MASK = directional_mask(TOP_GRID, _casters, KEY, blur=1.6)
FEET = ([(M.KICK_C[0], M.KICK_C[2], M.KICK_R, 0.45), (0, 0, 0.31, 0.5), (M.LEVER_PIVOT[0], M.LEVER_PIVOT[2], 0.08, 0.4)] +
        [(x, z, r, 0.4) for x, z, r, h in M.TOMS] +
        [(M.P(M.PIPE_R, a, 0)[0], M.P(M.PIPE_R, a, 0)[2], 0.06, 0.35) for a in M.PIPE_TH] +
        [(M.SNARE_C[0], M.SNARE_C[2], 0.06, 0.3)] + [(M.P(r, a, 0)[0], M.P(r, a, 0)[2], 0.05, 0.3) for r, a, y in M.DISCS])

# marble pile in the basin trough (part of the 1,000)
_pr = np.random.default_rng(7)
NPILE = 640
_ang = _pr.uniform(0, 2 * math.pi, NPILE)
_rad = _pr.uniform(M.R_BASIN[0] + 0.04, M.R_BASIN[1] - 0.04, NPILE)
_lay = (_pr.uniform(0, 1, NPILE) < 0.4).astype(float)
PILE = np.stack([_rad * np.sin(_ang), M.R_M + 0.004 + _lay * 0.045, _rad * np.cos(_ang)], 1)
PILE_C = COLS[_pr.integers(0, 8, NPILE)] * _pr.uniform(0.7, 0.95, (NPILE, 1))

# ================================================================== camera
A_TGT = np.array([0.15, 2.62, 0.25])
WIDE = np.array([0.0, 1.5, 0.0])
KICK_T = np.array([0.0, 0.58, 0.62])
T2 = np.array([0.6, 1.12, 0.35])
T3 = np.array([-0.62, 1.2, 0.35])
T45 = np.array([0.0, 2.3, 0.2])
G8 = np.array([0.0, 4.08, 0.30])
G8H = np.array([0.0, 3.95, 0.45])
KEYS = [  # (t, target, dist, yaw, pitch, fov, sy)
    (0.0, A_TGT, 3.3, 16, 9, 44, 360),
    (2.7, A_TGT + [-0.03, -0.03, 0], 3.0, 22, 10, 44, 360),
    (5.6, WIDE, 13.6, -6, 7, 40, 376),
    (6.4, WIDE, 13.1, -4, 8, 40, 376),
    (8.0, KICK_T, 4.0, 10, 22, 42, 388),
    (9.7, KICK_T, 3.7, 14, 20, 42, 388),
    (10.6, T2, 3.4, 30, 14, 42, 372),
    (13.3, T2, 3.25, 36, 12, 42, 372),
    (14.9, T3, 3.9, -20, 10, 42, 380),
    (17.4, T3, 3.6, -30, 10, 42, 380),
    (18.9, T45, 4.6, 6, 8, 42, 362),
    (23.5, T45, 4.3, -10, 8, 42, 362),
    (24.6, np.array([0, 2.1, 0]), 6.8, -30, 9, 42, 372),
    (29.6, np.array([0, 2.1, 0]), 6.4, 34, 10, 42, 372),
    (32.4, np.array([0, 3.2, 0.2]), 4.4, 30, 6, 42, 352),
    (34.6, np.array([0, 3.2, 0.2]), 4.2, 14, 5, 42, 352),
    (35.4, G8H, 2.3, 14, 17, 44, 340),
    (39.8, G8H, 1.9, 10, 15, 44, 340),
    (40.0, G8H + [0, -0.06, -0.03], 2.2, 9, 13, 46, 345),
    (44.0, np.array([0, 2.55, 0.3]), 3.1, -6, 8, 46, 360),
    (48.0, np.array([0, 0.85, 0.4]), 3.4, -26, 14, 46, 372),
    (52.0, WIDE, 13.6, 20, 6, 40, 376),
    (56.6, WIDE, 13.0, -10, 9, 40, 376),
    (57.2, WIDE, 12.6, -12, 9, 40, 376),
    (60.0, A_TGT, 3.3, 16, 9, 44, 360),
]


def camera(t):
    return interp_keys(KEYS, t % T.DUR)


SUBJ = np.concatenate([M.SUBJECT_PTS, np.array([e.pos for e in M.ELEMS]), np.array([M.gate_point(g) for g in range(8)])])


def focus(cam):
    """locked focus on the machine: the sharp band covers every on-screen part of it"""
    zf = cam.depth(cam.target)[0]
    Pc = cam.to_cam(SUBJ)
    s = cam.proj(Pc)
    on = (Pc[:, 2] > 0.3) & (s[:, 0] > -30) & (s[:, 0] < W + 30) & (s[:, 1] > -30) & (s[:, 1] < H + 30)
    z = Pc[on, 2] if on.any() else np.array([zf])
    sharp = float(np.clip(np.abs(1 - zf / np.maximum(z, 0.2)).max() + 0.04, 0.06, 3.0))
    K = 0.011 * cam.f * min(1.2, 4.0 / zf)
    return zf, sharp, K, z


def subject_blur(cam):
    zf, sharp, K, z = focus(cam)
    return float(coc_map(z, zf, K, sharp).max())


# ================================================================== lights
def lights(t, cam, fl):
    L = Lights(amb=(0.27, 0.22, 0.34), sky=(0.10, 0.08, 0.13))
    L.view = cam.view
    L.key('orb', KEY, S.KEY_COL)
    L.fill(dirv(-140, 28), (0.30, 0.10, 0.26))         # magenta nebula light from the front-left
    L.fill(dirv(140, 18), (0.08, 0.25, 0.29))          # teal from the front-right
    for i, c in enumerate(M.LANTERNS):
        fk = 0.92 + 0.08 * math.sin(t * (11 + i) + i) * math.sin(t * 7.3 + 2 * i)
        L.point(c, np.array([0.9, 0.56, 0.26]) * fk, 0.6)
    L.point(M.ORB_C, np.array([0.45, 0.55, 0.75]) * (1 + 0.8 * T.beat_env(t)), 1.1)
    top = np.argsort(fl)[-6:]
    for i in top:
        if fl[i] > 0.15:
            e = M.ELEMS[i]
            L.point(e.pos + [0, 0.06, 0], e.col / 255 * fl[i] * 0.9, 0.42)
    return L


# ================================================================== void
def void_bg(cam, t):
    d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1) @ cam.M
    d /= np.linalg.norm(d, axis=-1, keepdims=True)
    el = np.arcsin(np.clip(d[..., 1], -1, 1))
    az = np.arctan2(d[..., 0], -d[..., 2])
    e = el / (math.pi / 2)
    k_hi = (np.clip((e + 0.02) / 0.30, 0, 1) ** 0.8)[..., None]
    k_lo = (np.clip(-(e + 0.02) / 0.26, 0, 1) ** 0.8)[..., None]
    col = S.GRAD_MID * (1 - k_hi - k_lo) + S.GRAD_TOP * k_hi + S.GRAD_LOW * k_lo
    iy = np.clip(((el / math.pi + 0.5) * (S.NEB_H - 1)).astype(np.int32), 0, S.NEB_H - 1)
    ix = (((az / (2 * math.pi) + 0.5) % 1.0) * (S.NEB_W - 1)).astype(np.int32)     # seam behind the camera
    A, B = S.harmony_mix(t)
    pulse = 1 + 0.035 * T.beat_env(t, 0.16) + 0.05 * T.bar_env(t, 0.35)
    n1, n2, du = S.NEB1[iy, ix][..., None], S.NEB2[iy, ix][..., None], S.DUST[iy, ix][..., None]
    n1 = np.clip((n1 - 0.18) * 1.7, 0, 1); n2 = np.clip((n2 - 0.15) * 1.7, 0, 1)
    col = col * (1 - 0.35 * n1 - 0.3 * n2) + (n1 * A * 0.62 + n2 * B * 0.55) * pulse + du * np.array([36, 24, 46.0]) * 0.5
    # drop: a light burst ring from the orb on every bar line
    tt = t % T.DUR
    if T.T_DROP - 0.05 <= tt < T.T_LEVER + 1.0:
        tb = T.T_DROP + math.floor((tt - T.T_DROP) / T.BAR) * T.BAR if tt >= T.T_DROP else T.T_DROP
        age = tt - tb
        if age >= 0:
            ca = np.clip((d * S.ORB_DIR[None, None]).sum(-1), -1, 1)
            ang = np.arccos(ca)
            rho = 0.04 + 0.85 * age
            big = 1.25 if abs(tb - T.T_DROP) < 1e-6 else 1.0
            ring = np.exp(-((ang - rho) / (0.05 + 0.08 * age)) ** 2) * math.exp(-age / 0.55) * big
            rise = min(1.0, age / 0.1)                          # 3-frame rise: a bloom, not a strobe
            col = col + ring[..., None] * A[None, None] * 0.26 * rise + (np.exp(-ang / 0.18) * math.exp(-age / 0.3) * 0.12 * big * rise)[..., None] * A[None, None]
    col = np.floor(col / 7 + BAYER_FULL[..., None]) * 7
    return np.clip(col, 0, 255)


def draw_stars(img, cam, t):
    u = (t % T.DUR) / T.DUR
    for li, (V, b, ph, tw) in enumerate(S.STARS):
        Pc = cam.to_cam(V)
        ok = Pc[:, 2] > 1.0
        s = cam.proj(Pc[ok])
        bb = b[ok] * (0.65 + 0.35 * np.sin(2 * math.pi * tw[ok] * 9 * u + ph[ok])) * (1 + 0.5 * T.beat_env(t, 0.1))
        xi, yi = s[:, 0].astype(int), s[:, 1].astype(int)
        m = (xi >= 1) & (xi < W - 1) & (yi >= 1) & (yi < H - 1)
        xi, yi, bb = xi[m], yi[m], bb[m]
        c = S.STAR_COLS[(np.arange(len(m))[m]) % 4]
        v = c * np.clip(0.3 + bb, 0, 1.2)[:, None]
        img[yi, xi] = np.maximum(img[yi, xi], v)
        big = bb > (0.7 if li else 0.85)
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            img[yi[big] + dy, xi[big] + dx] = np.maximum(img[yi[big] + dy, xi[big] + dx], v[big] * 0.5)


def comets(add, cam, t):
    tt = t % T.DUR
    for (t0, dur, az, el, saz, sel, col) in S.COMETS:
        for off in (0.0, -T.DUR):
            u = (tt - t0 - off) / dur
            if not 0 <= u <= 1.25:
                continue
            pts = []
            for k in range(10):
                uu = u - k * 0.035
                if uu < 0 or uu > 1:
                    continue
                s = cam.screen_of_dir(dirv(az + saz * uu, el + sel * uu))
                if s is not None:
                    pts.append((s, 1 - k / 10))
            fade = min(1.0, (1.25 - u) / 0.25) if u > 1 else 1.0
            for (a, wa), (b, wb) in zip(pts, pts[1:]):
                add.line([tuple(a), tuple(b)], fill=tuple(int(c * wa * 0.8 * fade) for c in col), width=1)
            if pts and u <= 1:
                x, y = pts[0][0]
                add.rectangle([x - 1, y - 1, x + 1, y + 1], fill=tuple(int(c * fade) for c in col))


# ================================================================== dynamic geometry
H_TURNS = (4.30 - 0.42) / 0.45
HELIX_PITCH = 0.45


def helix_faces(t):
    s = T.lift_turns(t)
    out = []
    hs = np.arange(0, H_TURNS, 1 / 12)
    for h0, h1 in zip(hs, hs[1:]):
        a0, a1 = 2 * math.pi * (h0 + 0.5 * s), 2 * math.pi * (h1 + 0.5 * s)
        y0, y1 = 0.42 + HELIX_PITCH * h0, 0.42 + HELIX_PITCH * h1
        q = np.array([[0.045 * math.sin(a0), y0, 0.045 * math.cos(a0)], [0.165 * math.sin(a0), y0, 0.165 * math.cos(a0)],
                      [0.165 * math.sin(a1), y1, 0.165 * math.cos(a1)], [0.045 * math.sin(a1), y1, 0.045 * math.cos(a1)]])
        n = np.cross(q[1] - q[0], q[3] - q[0]); n /= np.linalg.norm(n)
        if n[1] < 0:
            n = -n
        out.append((q, n))
    return out


def lift_marbles(t):
    s = T.lift_turns(t)
    n = int(H_TURNS * 6)
    h = (np.arange(n) / 6 + 0.5 * s) % H_TURNS
    a = 2 * math.pi * (h + 0.5 * s)
    pos = np.stack([0.112 * np.sin(a), 0.42 + HELIX_PITCH * h + M.R_M + 0.004, 0.112 * np.cos(a)], 1)
    alpha = np.clip(np.minimum(h / 0.25, (H_TURNS - h) / 0.25), 0, 1)
    return pos, COLS[np.arange(n) % 8] * 0.95, alpha


def gate_faces(g, t):
    a = T.gate_open(g, t)
    th = M.GATE_TH[g]
    rad = M.dirv_xz(th)
    hinge = M.P(M.GATE_R0, th, M.GATE_Y[g] + 0.016)
    ang = math.radians(62) * a
    tip = hinge + (rad * math.cos(ang) + np.array([0, 1.0, 0]) * math.sin(ang)) * (M.GATE_R1 - M.GATE_R0)
    shut = a < 0.3
    col = np.array(T.COL[g]) * 0.8 + 50 if shut else np.array([236, 70, 58.0])
    return M.beam(hinge, tip, 0.096, 0.016), col


def lever_angle(t):
    tt = t % T.DUR
    rest, pulled = -26.0, 58.0
    if tt < T.T_LEVER:
        return rest
    d = tt - T.T_LEVER
    if d < 0.08:
        return rest + (pulled - rest) * (d / 0.08) ** 2
    if d < 0.45:
        return pulled + 6 * math.sin((d - 0.08) * 30) * math.exp(-(d - 0.08) * 9)
    if d < 1.6:
        return pulled + (rest - pulled) * T.ease_out_back((d - 0.45) / 1.15, 1.6)
    return rest


def lever_faces(t):
    a = math.radians(lever_angle(t))
    pv = M.LEVER_PIVOT
    dv = np.array([0, math.cos(a), math.sin(a)])
    end = pv + dv * M.LEVER_LEN
    out = [(f, (150, 120, 70)) for f in M.beam(pv - [0, 0.12, 0], pv + [0, 0.02, 0], 0.16, 0.12)]
    out += [(f, M.BRASS) for f in M.beam(pv, end, 0.034, 0.034, up=(1, 0, 0))]
    out += [(f, (236, 52, 66)) for f in icosa(end, 0.062, jit=0.0)]
    return out


def rock_faces(t):
    u = (t % T.DUR) / T.DUR
    out = []
    for (r, ph, y, s, seed) in M.ROCKS:
        c = M.P(r, ph + 360 * u, y + 0.12 * math.sin(2 * math.pi * 2 * u + seed))
        R = S._rot([0.3, 1, 0.2], 2 * math.pi * u * (1 + seed % 2))
        for Q, n in icosa([0, 0, 0], s, jit=0.25, rng=np.random.default_rng(seed)):
            out.append(((Q @ R.T + c, n @ R.T), np.array(M.STONE) * 0.95))
    return out


# ================================================================== the plinth top (per pixel)
def plinth_top(fr, cam, t, L, fl):
    msk = Image.new('L', (W, H))
    Pc = cam.to_cam(M.PLINTH_TOP)
    if (Pc[:, 2] < cam.near).any():
        Pc = clip_near(Pc, cam.near)
        if Pc is None:
            return
    ImageDraw.Draw(msk).polygon([tuple(p) for p in cam.proj(Pc)], fill=255)
    ys, xs = np.nonzero(np.asarray(msk))
    if not len(xs):
        return
    X, Z, dd = cam.ground_hit(xs.astype(float), ys.astype(float))
    r = np.hypot(X, Z)
    th = np.arctan2(X, Z)
    # albedo: radial stone tiles inside the basin, brass trough, outer stone ring
    ring = np.floor(r / 0.24)
    nseg = 8 + 4 * ring
    seg = np.floor((th / (2 * math.pi) + 0.5) * nseg)
    chk = (ring + seg) % 2
    alb = np.where(chk[:, None] > 0, np.array([168, 146, 186.0]), np.array([146, 124, 168.0]))
    grout = (np.abs(r / 0.24 - np.round(r / 0.24)) * 0.24 < 0.008) | \
            (np.abs((th / (2 * math.pi) + 0.5) * nseg - np.round((th / (2 * math.pi) + 0.5) * nseg)) * (2 * math.pi * r / np.maximum(nseg, 1)) < 0.008)
    alb = np.where(grout[:, None], np.array([96, 80, 116.0]), alb)
    inner = r < 0.26
    alb = np.where(inner[:, None], np.array([130, 100, 70.0]), alb)
    trough = (r >= M.R_BASIN[0]) & (r <= M.R_BASIN[1])
    alb = np.where(trough[:, None], np.array([92, 70, 72.0]), alb)
    outer = r > M.R_BASIN[1] + 0.06
    alb = np.where(outer[:, None], np.array([150, 130, 172.0]) * (0.94 + 0.06 * np.sin(th * 18))[:, None], alb)
    # light
    I = np.tile(L.amb + L.sky, (len(X), 1))
    for dfil, cfil in L.fills:
        I += max(0.0, dfil[1]) * cfil[None]
    sh = TOP_GRID.sample(TOP_MASK, X, Z)
    I += ((1 - sh) * KEY[1] * 1.15)[:, None] * S.KEY_COL[None]
    for lp, lc, R in L.points:
        dist = np.sqrt((lp[0] - X) ** 2 + lp[1] ** 2 + (lp[2] - Z) ** 2)
        I += (lp[1] / dist * 0.9 / (1 + (dist / (R * 1.25)) ** 2))[:, None] * lc[None]
    kf = fl[0]
    if kf > 0.02:                                           # the kick drum lights the stone red on every beat
        dist = np.hypot(X - M.KICK_C[0], Z - M.KICK_C[2])
        I += (kf * 0.55 * np.exp(-np.maximum(dist - M.KICK_R, 0) / 0.35))[:, None] * (COLS[0] / 255)[None]
    I *= contact_ao(X, Z, FEET, 0.16)[:, None]
    col = alb * I
    arr, depth = fr.array(), fr.depth()
    arr[ys, xs] = np.floor(col / 5 + BAYER_FULL[ys, xs, None]) * 5
    depth[ys, xs] = dd
    fr.set_array(arr, depth)


# ================================================================== faces
def shade_faces(L, cam, t, fl):
    """visible static faces -> (depth, Q, colour) lists"""
    camp = cam.pos
    view = camp[None] - FC
    facing = (FN * view).sum(1)
    vis = (facing > 0) | FDB
    idx = np.nonzero(vis)[0]
    N = FN[idx] * np.where(facing[idx] > 0, 1, -1)[:, None]
    el = FEL[idx]
    f = np.where(el >= 0, fl[np.maximum(el, 0)], 0.0)
    emit = FEM[idx] + f * 0.25
    cols = shade(L, N, FC[idx], FB[idx], kvis=FKV[idx][:, None], emit=emit[:, None], rimk=0.6)
    tint = np.where(el >= 0, 1, 0)[:, None] * f[:, None] * COLS[[M.ELEMS[e].tier if e >= 0 else 0 for e in el]] * 0.7
    cols = np.clip(cols + tint, 0, 255)
    return idx, cols, f


def dyn_faces(L, cam, t):
    """helix, gates, lever, rocks: (Q, colour) after shading"""
    out = []
    hf = helix_faces(t)
    Q = [q for q, n in hf]; Nn = np.array([n for q, n in hf]); C = np.array([q.mean(0) for q in Q])
    cols = shade(L, Nn, C, np.array([200, 160, 84.0]), kvis=np.full((len(Q), 1), 0.4))
    out += list(zip(Q, cols))
    for g in range(8):
        fs, col = gate_faces(g, t)
        Nn = np.array([n for q, n in fs]); C = np.array([q.mean(0) for q, n in fs])
        cols = shade(L, Nn, C, col, emit=0.25)
        out += [(q, c) for (q, n), c in zip(fs, cols) if np.dot(n, cam.pos - q.mean(0)) > 0]
    for (q, n), base in [((f[0], f[1]), c) for f, c in lever_faces(t)] + rock_faces(t):
        if np.dot(n, cam.pos - q.mean(0)) > 0:
            out.append((q, shade(L, n[None], q.mean(0)[None], np.asarray(base, float))[0]))
    return out


def crystal_items(L, cam, t):
    """floating void shapes -> (depth, Q, colour) + glows"""
    items, glows = [], []
    kick = T.beat_env(t, 0.14)
    vcol = S.GRAD_MID
    for spec in S.CRYSTALS:
        fs, ctr = crystal_faces_cached(spec, t)
        z = cam.depth(ctr)[0]
        if z < 0.3:
            continue
        s = cam.wp(ctr)
        rad = spec[5] * 2.0 * cam.f / z
        if s[0] < -rad or s[0] > W + rad or s[1] < -rad or s[1] > H + rad:
            continue
        dist = np.linalg.norm(ctr - cam.pos)
        Nn = np.array([n for q, n in fs]); C = np.array([q.mean(0) for q, n in fs])
        cols = shade(L, Nn, C, np.array(spec[6], float), emit=0.25 + 0.75 * kick, rimk=0.8)
        cols = fog(cols, dist, vcol, 8, 40, 0.55)
        for (q, n), c in zip(fs, cols):
            if np.dot(n, cam.pos - q.mean(0)) > 0:
                items.append((cam.depth(q.mean(0))[0], q, c))
        glows.append((s[0], s[1], min(60, rad * 1.4), spec[6], 0.10 + 0.3 * kick))
    return items, glows


def crystal_faces_cached(spec, t):
    return S.crystal_faces(spec, t)


def draw_items(fr, cam, items):
    for z, Q, col in sorted(items, key=lambda it: -it[0]):
        fr.poly(cam, Q, col)


# ================================================================== marbles
def marble_layer(cam, arr, depth, groups, glows, fl_ok=True):
    """groups: list of (pos (n,3), colour (n,3), alpha (n,)). Draws z-tested shaded discs."""
    if not groups:
        return arr, depth, np.zeros((0, 3)), np.zeros(0, bool)
    P = np.concatenate([g[0] for g in groups])
    C = np.concatenate([g[1] for g in groups])
    A = np.concatenate([g[2] for g in groups])
    Pc = cam.to_cam(P)
    z = Pc[:, 2]
    s = cam.proj(Pc)
    rr = M.R_M * cam.f / np.maximum(z, 1e-3)
    ok = (z > 0.15) & (A > 0.04) & (s[:, 0] > -rr - 2) & (s[:, 0] < W + rr + 2) & (s[:, 1] > -rr - 2) & (s[:, 1] < H + rr + 2)
    lay = Image.new('RGB', (W, H))
    ldp = Image.new('F', (W, H), 1e9)
    dr, ddr = ImageDraw.Draw(lay), ImageDraw.Draw(ldp)
    ls = cam.M @ KEY
    lx, ly = ls[0], -ls[1]
    ln = math.hypot(lx, ly) + 1e-6
    lx, ly = lx / ln, ly / ln
    order = np.nonzero(ok)[0]
    order = order[np.argsort(-z[order])]
    for i in order:
        x, y, r = s[i, 0], s[i, 1], rr[i]
        a = A[i]
        r = r * (0.45 + 0.55 * min(1.0, a * 1.6))
        c = C[i] * (0.55 + 0.45 * min(1.0, a * 1.5))
        zi = float(z[i])
        if r < 0.9:
            dr.point((int(x), int(y)), fill=tuple(int(v) for v in np.clip(c * 1.05, 0, 255)))
            ddr.point((int(x), int(y)), fill=zi)
            continue
        if r < 1.6:
            dr.rectangle([int(x), int(y), int(x) + 1, int(y) + 1], fill=tuple(int(v) for v in np.clip(c, 0, 255)))
            ddr.rectangle([int(x), int(y), int(x) + 1, int(y) + 1], fill=zi)
            continue
        ro = r + 0.7
        dr.ellipse([x - ro, y - ro, x + ro, y + ro], fill=tuple(int(v) for v in c * 0.28))
        dr.ellipse([x - r, y - r, x + r, y + r], fill=tuple(int(v) for v in np.clip(c * 0.78, 0, 255)))
        r2 = r * 0.68
        ox, oy = lx * r * 0.26, ly * r * 0.26
        dr.ellipse([x + ox - r2, y + oy - r2, x + ox + r2, y + oy + r2], fill=tuple(int(v) for v in np.clip(c * 1.08 + 18, 0, 255)))
        if r > 2.2:
            hx, hy = x + lx * r * 0.45, y + ly * r * 0.45
            hr = max(0.6, r * 0.16)
            dr.ellipse([hx - hr, hy - hr, hx + hr, hy + hr], fill=(255, 252, 240))
        ddr.ellipse([x - ro, y - ro, x + ro, y + ro], fill=zi)
    la = np.asarray(lay).astype(np.float32)
    ld = np.asarray(ldp)
    m = ld < depth + 0.07
    arr[m] = la[m]
    depth = np.where(m, ld, depth)
    # glows for visible bright marbles
    xi = np.clip(s[:, 0].astype(int), 0, W - 1); yi = np.clip(s[:, 1].astype(int), 0, H - 1)
    seen = ok & (depth[yi, xi] >= z - 0.08)
    for i in np.nonzero(seen & (rr > 0.5))[0]:
        glows.append((s[i, 0], s[i, 1], min(40, rr[i] * 2.6 + 1.5), tuple(C[i]), 0.11 * min(1.0, A[i] * 1.5)))
    return arr, depth, s, seen


def trails(add, cam, t, mids, pos, seen_map, everyone):
    """additive light streaks behind escapees (and every marble after the drop)"""
    K, dt = 10, 1 / 55
    order = np.argsort(mids)
    ms = mids[order]
    prev = cam.proj(cam.to_cam(pos))
    zc = cam.depth(pos)
    for k in range(1, K + 1):
        mk, pk, ak, _ = SIM.positions(t - k * dt, D)
        j = np.searchsorted(ms, mk)
        j = np.clip(j, 0, len(ms) - 1)
        hit = ms[j] == mk
        rows = order[j[hit]]
        pk = pk[hit]
        if not len(rows):
            break
        sel = everyone | D['mtrail'][mids[rows]]
        sel &= seen_map[rows]
        rows, pk = rows[sel], pk[sel]
        if not len(rows):
            continue
        Pc = cam.to_cam(pk)
        okz = Pc[:, 2] > 0.2
        sp = cam.proj(Pc)
        w = (1 - (k - 1) / K) ** 1.3
        for r_, a, b, oz in zip(rows, prev[rows], sp, okz):
            if not oz:
                continue
            col = COLS[D['mch'][mids[r_]]] * w * 0.85
            add.line([tuple(a), tuple(b)], fill=tuple(int(v) for v in col), width=1 if zc[r_] > 2.2 else 2)
            prev[r_] = b


def clunk_sparks(add, cam, t):
    tt = t % T.DUR
    for k, t_ev in enumerate(T.T_TRY + [T.T_DROP]):
        age = tt - t_ev
        if not 0 <= age < 0.35:
            continue
        rr = np.random.default_rng(90 + k)
        hinge = M.P(M.GATE_R0 + 0.06, M.GATE_TH[7], M.GATE_Y[7] + 0.03)
        for j in range(16):
            v = rr.normal(0, 1, 3); v = v / np.linalg.norm(v) * rr.uniform(0.6, 1.6); v[1] = abs(v[1]) + 0.3
            a = hinge + v * age + np.array([0, -4.9, 0]) * age ** 2
            b = hinge + v * max(0, age - 0.03) + np.array([0, -4.9, 0]) * max(0, age - 0.03) ** 2
            Pa, Pb = cam.to_cam(np.array([a, b]))
            if Pa[2] < 0.2 or Pb[2] < 0.2:
                continue
            sa, sb = cam.proj(np.array([Pa, Pb]))
            fade = 1 - age / 0.35
            c = (255, 190, 90) if t_ev != T.T_DROP else T.COL[7]
            add.line([tuple(sa), tuple(sb)], fill=tuple(int(x * fade) for x in c), width=1)


def sparks(add, cam, t):
    """bright particles off every routed hit (0.22 s)"""
    lo = np.searchsorted(HITS[:, 0], t - 0.22)
    hi = np.searchsorted(HITS[:, 0], t + 1e-6)
    for row in HITS[lo:hi]:
        th, el, p, vel, src, ch = row
        if src != 0:
            continue
        age = t - th
        e = M.ELEMS[int(el)]
        rr = np.random.default_rng(int(th * 1000) * 7 + int(el))
        for k in range(5):
            v = rr.normal(0, 1, 3)
            v = v / np.linalg.norm(v) * rr.uniform(0.5, 1.1)
            v[1] = abs(v[1]) + 0.4
            q = e.pos + v * age + np.array([0, -4.9, 0]) * age * age
            Pc = cam.to_cam(q[None])
            if Pc[0, 2] < 0.2:
                continue
            sx, sy = cam.proj(Pc)[0]
            fade = (1 - age / 0.22)
            c = (COLS[int(ch)] * 0.6 + 100) * fade
            add.point((int(sx), int(sy)), fill=tuple(int(x) for x in np.clip(c, 0, 255)))


# ================================================================== HUD
def hud(img, t):
    pil = Image.fromarray(img)
    d = ImageDraw.Draw(pil)
    y0 = 600
    panel(d, 16, y0, 344, y0 + 30)
    n = T.n_shut(t)
    tt = t % T.DUR
    for g in range(8):
        x = 70 + g * 26
        shut = tt < T.T_LEVER and tt >= T.GATE_SHUT[g]
        if shut:
            k = 0.75 + 0.25 * min(1.0, FLASH[min(T.NF - 1, int(round(tt * FPS)))][[e.id for e in M.TIER_ELEMS[g]]].max())
            c = tuple(int(v * k) for v in T.COL[g])
            d.rectangle([x, y0 + 9, x + 19, y0 + 21], fill=c)
            d.rectangle([x, y0 + 9, x + 19, y0 + 10], fill=tuple(min(255, int(v * 1.25)) for v in c))
        else:
            wob = g == 7 and T.T_STICK <= tt < T.T_DROP
            d.rectangle([x, y0 + 9, x + 19, y0 + 21], fill=(30, 26, 48), outline=(120, 60, 70) if not wob else (255, 120, 90))
    out = np.asarray(pil).copy()
    stamp(out, text_mask('GATES', 1), 24, y0 + 12, CREAM)
    txt = f'{n}/8'
    stamp(out, text_mask(txt, 2), 286, y0 + 8, AMBER if n == 8 else CREAM)
    return out


# ================================================================== frame
def render(f):
    t = f / FPS
    cam = camera(t)
    fl = FLASH[f % T.NF]
    L = lights(t, cam, fl)
    fr = Frame(void_bg(cam, t))
    arr = fr.array()
    draw_stars(arr, cam, t)
    fr.set_array(arr)
    glows = []
    ms = cam.screen_of_dir(S.ORB_DIR)
    if ms is not None:
        fr.disc(ms, 9.0, (255, 246, 226), n=16)
        glows += [(ms[0], ms[1], 34, (255, 210, 170), 0.55), (ms[0], ms[1], 14, (255, 250, 236), 0.6)]
    # far void shapes behind the plinth, then the plinth top
    citems, cglows = crystal_items(L, cam, t)
    glows += cglows
    zpl = cam.depth(np.array([0.0, 0.0, 0.0]))[0] + 1.8
    draw_items(fr, cam, [it for it in citems if it[0] > zpl])
    plinth_top(fr, cam, t, L, fl)
    # machine + near shapes in one painter list
    idx, cols, ff = shade_faces(L, cam, t, fl)
    PC = (FV - cam.pos) @ cam.M.T
    zc = (FC[idx] - cam.pos) @ cam.M[2]
    items = []
    for j, i in enumerate(idx):
        items.append((zc[j], i, cols[j]))
    for Q, c in dyn_faces(L, cam, t):
        items.append((cam.depth(Q.mean(0))[0], Q, c))
    for it in citems:
        if it[0] <= zpl:
            items.append(it)
    items.sort(key=lambda it: -it[0])
    draw, ddraw = fr.draw, fr.ddraw
    for z, Q, col in items:
        if isinstance(Q, (int, np.integer)):
            i = Q
            Pc = PC[OFF[i]:OFF[i + 1]]
            if FEL[i] >= 0 and DIP[i] > 0 and fl[FEL[i]] > 0.04:
                Pc = Pc + cam.M @ np.array([0, -DIP[i] * fl[FEL[i]], 0])
        else:
            Pc = cam.to_cam(Q)
        if (Pc[:, 2] < cam.near).any():
            if (Pc[:, 2] < cam.near).all():
                continue
            Pc = clip_near(Pc, cam.near)
            if Pc is None:
                continue
        pts = [tuple(p) for p in cam.proj(Pc)]
        draw.polygon(pts, fill=tuple(int(c) for c in col))
        ddraw.polygon(pts, fill=float(Pc[:, 2].mean()))
    img, depth = fr.array(), fr.depth()
    # marbles (z-tested), pile, lift, gate lamps
    mids, mpos, malpha, mvel = SIM.positions(t, D)
    mcol = COLS[D['mch'][mids]] if len(mids) else np.zeros((0, 3))
    lp, lc, la = lift_marbles(t)
    gl_pos = np.array([M.P(0.215, M.GATE_TH[g] + 13, M.GATE_Y[g] + 0.085) for g in range(8)])
    gl_col = np.array([np.array(T.COL[g]) * 0.6 + 100 if T.gate_open(g, t) < 0.3 else np.array([255, 70, 60.0]) for g in range(8)])
    groups = [(mpos, mcol, malpha), (PILE, PILE_C, np.ones(NPILE)), (lp, lc, la)]
    img, depth, sm, seen = marble_layer(cam, img, depth, groups, glows)
    n_m = len(mids)
    # lens: focus locked on the machine
    zf, sharp, K, _ = focus(cam)
    img = dof(img, coc_map(depth, zf, K, sharp))
    # gate lamps, lanterns, crown orb, hit flashes, basin splashes
    for g in range(8):
        s = cam.wp(gl_pos[g])
        z = cam.depth(gl_pos[g])[0]
        if z > 0.3:
            r = max(1.0, 0.03 * cam.f / z)
            glows.append((s[0], s[1], min(30, r * 4.0), tuple(gl_col[g]), 0.6))
            glows.append((s[0], s[1], max(1, r * 0.9), (255, 255, 240), 0.5))
    tt = t % T.DUR
    for g in range(8):
        gp = M.gate_point(g)
        z = cam.depth(gp)[0]
        if z < 0.3:
            continue
        s = cam.wp(gp)
        for t_ev, colr, dur in ((T.GATE_SHUT[g], T.COL[g], 0.45), (T.T_LEVER, (255, 80, 60), 0.5)):
            age = tt - t_ev
            if 0 <= age < dur:
                glows.append((s[0], s[1], min(60, 0.22 * cam.f / z * (1 + 2.2 * age / dur)), colr, 0.85 * (1 - age / dur)))
    lk = M.LEVER_PIVOT + np.array([0, math.cos(math.radians(lever_angle(t))), math.sin(math.radians(lever_angle(t)))]) * M.LEVER_LEN
    age = tt - T.T_LEVER
    if 0 <= age < 0.7:
        z = cam.depth(lk)[0]
        if z > 0.3:
            s = cam.wp(lk)
            glows.append((s[0], s[1], min(50, 0.3 * cam.f / z), (255, 90, 80), 0.9 * (1 - age / 0.7)))
    for c in M.LANTERNS:
        z = cam.depth(c)[0]
        if z > 0.3:
            s = cam.wp(c)
            glows.append((s[0], s[1], min(40, 0.2 * cam.f / z), (255, 170, 80), 0.32))
    z = cam.depth(M.ORB_C)[0]
    if z > 0.3:
        s = cam.wp(M.ORB_C)
        glows.append((s[0], s[1], min(50, 0.32 * cam.f / z), (190, 230, 255), 0.4 + 0.4 * T.beat_env(t)))
    for i in np.nonzero(fl > 0.05)[0]:
        e = M.ELEMS[i]
        z = cam.depth(e.pos)[0]
        if z < 0.3:
            continue
        s = cam.wp(e.pos)
        xi, yi = int(np.clip(s[0], 0, W - 1)), int(np.clip(s[1], 0, H - 1))
        occl = 0.3 if depth[yi, xi] < z - 0.25 else 1.0
        r = e.size * cam.f / z * (0.55 + 0.45 * fl[i])
        glows.append((s[0], s[1], min(60, r), tuple(e.col), 0.5 * fl[i] * occl))
        glows.append((s[0], s[1], min(30, r * 0.35), (255, 250, 235), 0.4 * fl[i] * occl))
    lo = np.searchsorted(LANDS[:, 0], t - 0.14)
    hi = np.searchsorted(LANDS[:, 0], t + 1e-6)
    for row in LANDS[lo:hi]:
        age = t - row[0]
        q = row[1:4]
        z = cam.depth(q)[0]
        if z > 0.3:
            s = cam.wp(q)
            glows.append((s[0], s[1], min(20, 0.07 * cam.f / z), tuple(COLS[int(row[4])]), 0.45 * (1 - age / 0.14)))
    for g in glows:
        add_glow(img, *g)
    # additive streaks: trails, sparks, comets
    addl = Image.new('RGB', (W, H))
    ad = ImageDraw.Draw(addl)
    if n_m:
        seen_m = seen[:n_m]
        trails(ad, cam, t, mids, mpos, seen_m, T.drop_k(t) > 0.5)
    sparks(ad, cam, t)
    clunk_sparks(ad, cam, t)
    comets(ad, cam, t)
    img = img + np.asarray(addl).astype(np.float32)
    if ms is not None:
        img = god_rays(img, depth, ms, (255, 214, 176), 0.13, r1=30.0, r2=90.0)
    img = grade(bloom(np.clip(img, 0, 255), thr=218, k=0.26))
    img = np.clip(img * vignette(0.6, 0.62, 0.35) * top_scrim(0.5), 0, 255).astype(np.uint8)
    out = hud(img, t)
    draw_captions(out, t % T.DUR, T.CAPTIONS)
    return out
