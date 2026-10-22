"""Domino Symphony renderer.

    python -m reelkit.sheet render.py test/sheet.png 0.5 3 6 10 16 23 29 35 39.8 46 54 58
    python -m reelkit.video render.py audio.wav domino_symphony.mp4

Per frame: ray-traced gym shell (floor planks + court lines, window wall with the low sun, bleacher wall,
far wall) -> screen-space sun shadow mask (every standing domino, props, the cat) -> per-pixel floor light
(window patches x shadow mask + warm work lamps) -> tiny dominoes splatted as pixels -> chunks of dominoes,
props and the cat in painter's order -> locked DOF -> chime/lamp glows -> window beams + dust -> bloom,
grade, scrim -> captions + domino counter."""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, ease, dirv
from reelkit.camera import Cam, interp_keys
from reelkit.raster import Frame, apply_glows
from reelkit.lighting import Lights, shade
from reelkit.lens import coc_map, dof
from reelkit.post import bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, panel, label, CREAM, AMBER
from reelkit.font import GLYPHS
import timeline as T
import scene as S
import cat as CAT
from boxes import Boxes, FACE_IDX

B = Boxes()
NB = B.N
DOM = B.is_dom
SUN = S.SUN
SUN_COL = np.array([1.30, 0.98, 0.64])
TOTAL = int(DOM.sum())

# ---------------------------------------------------------------- chunks (painter units): runs of <= 12 boxes in a lane
CHUNKS = []
_lane, _start = None, 0
for i in range(NB + 1):
    key = (B.lane[i], B.kind[i], B.sec[i]) if i < NB else None
    if i == NB or key != _lane or (i - _start) >= 12 or B.big[i] or (i > 0 and B.big[i - 1]) or B.kind[i] == 1:
        if i > _start:
            CHUNKS.append(np.arange(_start, i))
        _start = i
        _lane = key
CH_CTR = np.array([np.stack([B.cx[c], B.cz[c]], 1).mean(0) for c in CHUNKS])
CH_Y = np.array([(B.y0[c] + B.dh[c] / 2).mean() for c in CHUNKS])

# ---------------------------------------------------------------- camera
CS = S.C_S
HOLD_P = np.array([CS[0], 0.13, CS[1]])
GIANT_P = np.array([B.cx[B.sec == 10][0], 0.0, B.cz[B.sec == 10][0]])
TAP_P = np.array([0.0, 0.035, S.LY.ROOT[1]])
ZF_B = S.LY.Z_FB
ZF_T = S.LY.Z_FT
MID = np.array([0.0, 0.0, 0.5 * (S.LY.ZTOP + S.LY.ZBOT)])

KEYS = [  # (t, target, dist, yaw, pitch, fov, sy)
    (0.0, np.array([0.06, 0.17, 5.45]), 0.95, -155, 10, 46, 360),       # in front: the cat's face looms behind domino #1
    (0.7, np.array([0.04, 0.14, 5.40]), 1.12, -150, 16, 46, 350),
    (2.2, np.array([0.0, 0.0, 4.85]), 2.5, -70, 58, 44, 330),
    (3.6, np.array([0.0, 0.0, 4.2]), 5.6, -15, 74, 44, 330),
    (4.6, np.array([0.0, 0.0, 3.75]), 7.4, 0, 80, 44, 330),
    (6.1, np.array([-0.82, 1.82, 4.5]), 5.5, 72, 4, 60, 300),      # establishing: across the gym, the low sun in a window
    (7.4, np.array([0.05, 0.0, 2.05]), 3.6, 14, 60, 44, 340),
    (8.6, np.array([0.25, 0.0, 1.45]), 2.5, -4, 62, 44, 340),
    (11.0, np.array([0.45, 0.0, 0.85]), 2.4, 0, 66, 44, 340),
    (14.0, np.array([0.55, 0.0, 0.25]), 2.7, 6, 64, 44, 340),
    (17.0, np.array([0.75, 0.0, -0.25]), 3.0, 8, 70, 44, 340),
    (19.6, np.array([0.1, 0.0, -0.75]), 4.6, 2, 86, 44, 380),
    (22.5, np.array([0.0, 0.0, -1.45]), 5.4, 0, 88.5, 44, 360),
    (25.6, np.array([0.0, 0.0, -1.72]), 6.1, 0, 88.5, 44, 340),
    (27.0, np.array([CS[0], 0.0, CS[1] + 0.12]), 2.6, -6, 87, 44, 330),
    (30.4, np.array([CS[0], 0.0, CS[1]]), 2.0, 40, 87, 44, 330),
    (32.15, HOLD_P + [0, 0.0, -0.02], 0.66, 28, 5, 44, 400),       # 3/4 view: red holdout, the staircase receding behind
    (37.9, HOLD_P + [0, 0.0, -0.01], 0.5, 24, 4, 44, 405),
    (38.6, np.array([CS[0], 0.13, CS[1] - 0.35]), 1.2, 40, 4, 46, 420),
    (39.6, np.array([0.0, 0.42, GIANT_P[2] - 0.2]), 3.0, 56, 8, 48, 420),
    (41.0, np.array([0.0, 0.0, GIANT_P[2] - 0.8]), 4.8, 40, 34, 48, 360),
    (46.0, np.array([0.0, 0.0, -2.2]), 10.0, 14, 72, 46, 330),
    (51.0, MID, 16.0, 0, 88.5, 46, 320),
    (56.0, MID + [0, 0, 0.1], 16.4, 0, 88.5, 46, 320),
    (57.6, np.array([0.0, 0.0, 2.6]), 7.0, -30, 76, 44, 330),
    (59.0, TAP_P + [0.03, 0.04, 0.0], 1.4, -120, 40, 46, 345),
    (59.6, np.array([0.06, 0.168, 5.448]), 0.97, -154.5, 10.5, 46, 359),
    (60.0, np.array([0.06, 0.17, 5.45]), 0.95, -155, 10, 46, 360),
]


def camera(t):
    return interp_keys(KEYS, t)


def subject(t, tau):
    """focus point (locked): the action the camera is following"""
    k = interp_keys(KEYS, t)
    return k.target


# ---------------------------------------------------------------- lights
LAMPS = S.LAMPS


def lamp_k(t):
    return 0.9 + 0.35 * T.beat_env(t, 0.1, 14.0, 52.2) + 0.25 * T.beat_env(t, 0.12, 40.0, 52.2)


def lights(t, cam):
    L = Lights(amb=(0.46, 0.46, 0.54), sky=(0.12, 0.12, 0.14))
    L.view = cam.view
    L.key('sun', SUN, SUN_COL)
    L.fill(dirv(105, 35), (0.12, 0.12, 0.16))
    k = lamp_k(t)
    for lp in LAMPS:
        L.point(lp, np.array([0.62, 0.40, 0.20]) * k, 1.5)
    return L


# ---------------------------------------------------------------- room shell (per pixel)
SKY_HI = np.array([92, 140, 206.0])
SKY_LO = np.array([228, 160, 104.0])


def shell(cam, L, lk):
    d = cam.pixel_rays(XX, YY)                     # camera-z component 1 -> t = depth
    p = cam.pos
    big = 1e9
    tf = np.where(d[..., 1] < -1e-6, -p[1] / np.minimum(d[..., 1], -1e-6), big)
    tc = np.where(d[..., 1] > 1e-6, (S.RH - p[1]) / np.maximum(d[..., 1], 1e-6), big)
    txl = np.where(d[..., 0] < -1e-6, (S.RX0 - p[0]) / np.minimum(d[..., 0], -1e-6), big)
    txr = np.where(d[..., 0] > 1e-6, (S.RX1 - p[0]) / np.maximum(d[..., 0], 1e-6), big)
    tzf = np.where(d[..., 2] < -1e-6, (S.RZ0 - p[2]) / np.minimum(d[..., 2], -1e-6), big)
    tzn = np.where(d[..., 2] > 1e-6, (S.RZ1 - p[2]) / np.maximum(d[..., 2], 1e-6), big)
    T6 = np.stack([tf, tc, txl, txr, tzf, tzn], 0)
    which = np.argmin(T6, 0)
    t = np.min(T6, 0)
    Pw = p[None, None] + d * t[..., None]
    img = np.zeros((H, W, 3), np.float32)
    # walls / ceiling (floor done later, after the shadow mask)
    wall = (which >= 2)
    if wall.any():
        ys = Pw[..., 1]
        u = np.where((which == 2) | (which == 3), Pw[..., 2], Pw[..., 0])
        alb = np.empty((H, W, 3), np.float32)
        alb[:] = (226, 214, 196)
        row = np.floor(ys / 0.2)
        mortar = ((ys % 0.2) < 0.012) | (((u + (row % 2) * 0.2) % 0.4) < 0.012)
        alb[mortar] = (198, 188, 172)
        pad = ys < S.PAD_H
        alb[pad] = (54, 96, 176)
        alb[pad & ((u % 1.22) < 0.02)] = (40, 72, 140)
        alb[pad & (np.abs(ys - S.PAD_H) < 0.02)] = (40, 72, 140)
        # wall light: ambient + lamps (cheap) + sun patches through the windows
        I = np.full((H, W, 3), 0.62, np.float32)
        nrm = {2: np.array([1.0, 0, 0]), 3: np.array([-1.0, 0, 0]), 4: np.array([0, 0, 1.0]), 5: np.array([0, 0, -1.0])}
        for k_, n in nrm.items():
            m = which == k_
            if not m.any():
                continue
            nd = float(n @ SUN)
            if nd > 0:
                lit = S.win_lit(Pw[..., 0][m], Pw[..., 1][m], Pw[..., 2][m])
                I[m] += (lit * nd * 1.1)[:, None] * SUN_COL[None]
        img = np.where(wall[..., None], alb * I, img)
        # window openings in the left wall: the late-afternoon sky + the sun
        wm = (which == 2)
        if wm.any():
            wo = wm & S.win_open(Pw[..., 2], Pw[..., 1])
            fr_ = wm & S.win_frame(Pw[..., 2], Pw[..., 1])
            dn = d / np.linalg.norm(d, axis=-1, keepdims=True)
            el = np.clip(dn[..., 1], -0.2, 1)
            sky = SKY_LO[None, None] + (SKY_HI - SKY_LO)[None, None] * np.clip(el / 0.55, 0, 1)[..., None] ** 0.7
            cs = (dn * SUN[None, None]).sum(-1)
            glow = np.clip((cs - 0.97) / 0.03, 0, 1) ** 2.5
            sky = sky * (1 - 0.25 * glow[..., None]) + glow[..., None] * np.array([70, 40, 0.0])
            sky = np.where((cs > 0.9992)[..., None], np.array([255, 246, 214.0]), sky)
            sky = np.where(((cs > 0.9986) & (cs <= 0.9992))[..., None], np.array([255, 196, 120.0]), sky)
            img = np.where(wo[..., None], sky, img)
            img = np.where(fr_[..., None], np.array([214, 210, 204.0]) * 0.75, img)
    cm = which == 1
    if cm.any():
        cx_ = Pw[..., 0]
        cz_ = Pw[..., 2]
        cl = np.full((H, W, 3), (70, 74, 88), np.float32)
        beam = ((cz_ % 2.0) < 0.12) | ((cx_ % 3.5) < 0.08)
        cl[beam] = (48, 52, 64)
        img = np.where(cm[..., None], cl, img)
    floor = which == 0
    return img, t, floor, Pw, d


# ---------------------------------------------------------------- shadows (screen space)
def ground_proj(P):
    """project world points along the sun onto y = 0"""
    return P - (P[..., 1:2] / SUN[1]) * SUN


def proj_px(cam, P):
    Pc = (P - cam.pos) @ cam.M.T
    z = np.maximum(Pc[..., 2], 1e-3)
    return np.stack([cam.cx + cam.f * Pc[..., 0] / z, cam.cy - cam.f * Pc[..., 1] / z], -1), Pc[..., 2]


def faces_to_mask(dr, cam, faces, near=0.12):
    for Q, n in faces:
        if float(n @ SUN) <= 0.0 or Q[:, 1].max() < 0.003:
            continue
        G = ground_proj(Q)
        s, z = proj_px(cam, G)
        if (z < near).any():
            continue
        dr.polygon(s.ravel().tolist(), fill=255)


# ---------------------------------------------------------------- pips for the big dominoes
PIP_LAYOUT = {0: [], 1: [(0, 0)], 2: [(-0.5, -0.5), (0.5, 0.5)], 3: [(-0.5, -0.5), (0, 0), (0.5, 0.5)],
              4: [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)], 5: [(-0.5, -0.5), (0.5, -0.5), (0, 0), (-0.5, 0.5), (0.5, 0.5)],
              6: [(-0.5, -0.6), (0.5, -0.6), (-0.5, 0), (0.5, 0), (-0.5, 0.6), (0.5, 0.6)]}


def local_to_world(i, th, a, b, c):
    cth, sth = math.cos(th), math.sin(th)
    ap = a * cth + c * sth
    cp = -a * sth + c * cth
    return (np.array([B.cx[i], 0.0, B.cz[i]]) + B.F[i] * (ap + B.dt[i] / 2) + B.L[i] * b + np.array([0, 1.0, 0]) * (cp + B.y0[i]))


def draw_pips(fr, cam, i, th, which, col):
    n = int(B.pips[i])
    if n <= 0:
        return
    a = 0.0 if which == 'front' else -B.dt[i]
    eps = 0.002 * (1 if which == 'front' else -1)
    w, h = B.dw[i], B.dh[i]
    r = 0.085 * w
    # centre bar
    bar = [local_to_world(i, th, a + eps, bb, cc) for bb, cc in ((-0.4 * w, 0.49 * h), (0.4 * w, 0.49 * h), (0.4 * w, 0.51 * h), (-0.4 * w, 0.51 * h))]
    fr.poly(cam, np.array(bar), col)
    for half, cnt in ((0, n), (1, n if n < 6 else 6)):
        cc0 = 0.25 * h + half * 0.5 * h
        for (u, v) in PIP_LAYOUT[cnt]:
            pc = (u * 0.3 * w, cc0 + v * 0.3 * w)
            ring = [local_to_world(i, th, a + eps, pc[0] + r * math.cos(k * 1.047), pc[1] + r * math.sin(k * 1.047)) for k in range(6)]
            fr.poly(cam, np.array(ring), col)


# ---------------------------------------------------------------- dust motes in the beams
_dr = np.random.default_rng(77)
DUST = np.stack([_dr.uniform(-3.5, 3.5, 900), _dr.uniform(0.05, 3.2, 900), _dr.uniform(-8, 8, 900)], 1)
DUST_PH = _dr.uniform(0, 6.28, 900)


# ---------------------------------------------------------------- '...' glyph for "64 LINES..."
if '…' not in GLYPHS:
    GLYPHS['…'] = np.array([[c == '#' for c in row] for row in ".....|.....|.....|.....|.....|.....|#.#.#".split('|')])


def fmt_count(n):
    return f'{n:,}'


# ---------------------------------------------------------------- main
def render(f):
    t = f / FPS
    tau = T.scene_time(t)
    cam = camera(t)
    L = lights(t, cam)
    lk = lamp_k(t)

    # ---- boxes: state + geometry
    hth = B.holdout_theta(tau)
    th = B.angles(tau, hth)
    P = B.corners(th, B.bar_hop(tau))                  # (N, 8, 3)
    Pc = (P - cam.pos) @ cam.M.T
    zc8 = Pc[..., 2]
    scr = np.stack([cam.cx + cam.f * Pc[..., 0] / np.maximum(zc8, 1e-3), cam.cy - cam.f * Pc[..., 1] / np.maximum(zc8, 1e-3)], -1)
    ctr = P.mean(1)
    zc = zc8.mean(1)
    sx, sy = scr[..., 0], scr[..., 1]
    onscr = (zc8.min(1) > cam.near) & (sx.max(1) > -2) & (sx.min(1) < W + 2) & (sy.max(1) > -2) & (sy.min(1) < H + 2)
    size_px = (B.dh * cam.f / np.maximum(zc, 1e-3))
    poly_lod = onscr & (size_px >= 2.4)
    splat_lod = onscr & ~poly_lod

    # ---- room shell
    img, depth, floor, Pw, rays = shell(cam, L, lk)

    # ---- sun shadow mask (screen space)
    mask = Image.new('L', (W, H), 0)
    md = ImageDraw.Draw(mask)
    standing_ish = th < (B.thr - 0.12)
    sun_vis = S.win_lit(ctr[:, 0], ctr[:, 1], ctr[:, 2])
    G = ground_proj(P)
    Gs, Gz = proj_px(cam, G)
    nrm = B.normals(th)                                # (N, 5, 3)
    sunfacing = (nrm @ SUN) > 0                        # (N, 5)
    sel = np.where(poly_lod & standing_ish & (sun_vis > 0) & (Gz.min(1) > cam.near))[0]
    for i in sel:
        for k in np.where(sunfacing[i])[0]:
            md.polygon(Gs[i, FACE_IDX[k]].ravel().tolist(), fill=255)
    # tiny standing dominoes: shadow streaks as dots from base to the projected top
    sel2 = np.where(splat_lod & standing_ish & DOM & (sun_vis > 0))[0]
    if len(sel2):
        base = np.stack([B.cx[sel2], np.zeros(len(sel2)), B.cz[sel2]], 1)
        topg = ground_proj(base + np.array([0, 1.0, 0]) * (B.dh[sel2] * np.cos(th[sel2]))[:, None])
        mk = np.asarray(mask).copy()
        for u in np.linspace(0.15, 1.0, 6):
            q = base + (topg - base) * u
            s_, z_ = proj_px(cam, q)
            xi, yi = np.round(s_[:, 0]).astype(int), np.round(s_[:, 1]).astype(int)
            ok = (xi >= 0) & (xi < W) & (yi >= 0) & (yi < H) & (z_ > cam.near)
            mk[yi[ok], xi[ok]] = 255
        mask = Image.fromarray(mk)
        md = ImageDraw.Draw(mask)
    # props + cat + vacuum shadows
    cat_faces, cat_cols, cat_ctr = CAT.faces(tau)
    vac_faces, vac_p = S.vacuum_faces(tau)
    prop_list = []
    for o in S.PROPS:
        zo = cam.depth(o.ctr)[0]
        if zo < -1.0:
            continue
        prop_list.append((zo, o))
        if o.cast:
            faces_to_mask(md, cam, o.faces)
    faces_to_mask(md, cam, cat_faces)
    faces_to_mask(md, cam, vac_faces)
    mask_a = np.asarray(mask, np.float32) / 255.0

    # ---- floor light (per pixel)
    fy, fx = np.nonzero(floor)
    X, Z = Pw[fy, fx, 0], Pw[fy, fx, 2]
    alb = S.floor_albedo(X, Z)
    I = np.tile(L.amb + L.sky, (len(X), 1)).astype(np.float32)
    lit = S.win_lit(X, np.zeros_like(X), Z) * (1 - mask_a[fy, fx])
    I += (lit * SUN[1] * 1.7)[:, None] * SUN_COL[None]
    for lp, aim in zip(LAMPS, S.LAMP_AIM):
        v = np.stack([X - lp[0], -lp[1] * np.ones_like(X), Z - lp[2]], 1)
        dist = np.linalg.norm(v, axis=1)
        cone = np.clip((v @ aim) / dist, 0, 1) ** 3
        I += (lp[1] / dist * cone * 1.4 * lk / (1 + (dist / 1.7) ** 2))[:, None] * np.array([0.95, 0.62, 0.30])
    col = alb * I
    img[fy, fx] = np.floor(col / 5 + BAYER_FULL[fy, fx, None]) * 5
    fr = Frame(np.clip(img, 0, 255))
    fr.set_array(np.clip(img, 0, 255), depth.astype(np.float32))

    # ---- box shading (all on-screen boxes, vectorised)
    kv = sun_vis.copy()
    six = np.clip(np.round(scr[:, :, 0].mean(1)).astype(int), 0, W - 1)
    siy = np.clip(np.round(scr[:, :, 1].mean(1)).astype(int), 0, H - 1)
    # props / cat / giant shadows darken boxes standing in them (sampled at the box's floor point)
    kv = kv * (1 - 0.85 * mask_a[siy, six] * (~DOM | (th > 0.5) | (B.dh > 0.06)))
    fcent = P[:, FACE_IDX].mean(2)                     # (N, 5, 3)
    vis = ((nrm * (cam.pos[None, None] - fcent)).sum(-1) > 0)
    base = B.base * B.tone[:, None]
    backc = B.back * B.tone[:, None]
    fld = B.sec == 7
    tipped = fld & (th > 0.02)
    base[tipped] = backc[tipped]
    backc[fld & ~tipped] = base[fld & ~tipped]
    # chime bars flash when struck
    hit = (~DOM) & (tau >= B.t0) & (tau < B.t0 + 0.35)
    flash = np.where(hit, np.exp(-(tau - B.t0) / 0.09), 0.0)
    base = base + flash[:, None] * (255 - base) * 0.8
    on = np.where(onscr)[0]
    colors = np.zeros((NB, 5, 3))
    if len(on):
        nn = nrm[on].reshape(-1, 3)
        cc = fcent[on].reshape(-1, 3)
        bc = np.repeat(base[on][:, None, :], 5, 1)
        bc[:, 2, :] = backc[on]
        kvr = np.repeat(kv[on], 5)[:, None]
        emit = np.repeat(flash[on] * 0.6, 5)[:, None]
        colors[on] = shade(L, nn, cc, bc.reshape(-1, 3), kvis=kvr, emit=emit).reshape(-1, 5, 3)

    # ---- splats (tiny dominoes), depth-tested against the floor/walls
    sp = np.where(splat_lod)[0]
    if len(sp):
        arr = fr.array()
        dep = fr.depth()
        fallen = th[sp] > 0.7
        cs_ = np.where(fallen[:, None], colors[sp, 2], colors[sp, 0])
        xi = np.clip(np.round(scr[sp, :, 0].mean(1)).astype(int), 0, W - 1)
        yi = np.clip(np.round(scr[sp, :, 1].mean(1)).astype(int), 0, H - 1)
        ok = zc[sp] < dep[yi, xi] + 0.05
        order = np.argsort(-zc[sp][ok])
        xi, yi, cs_, zz = xi[ok][order], yi[ok][order], cs_[ok][order], zc[sp][ok][order]
        arr[yi, xi] = cs_
        dep[yi, xi] = zz - 0.01
        # fallen ones are longer: a second pixel along the fall direction when > 1 px
        fl2 = fallen[ok][order] & (size_px[sp][ok][order] > 1.2)
        if fl2.any():
            tip = (scr[sp][ok][order][fl2][:, [5, 7], :].mean(1))
            xt = np.clip(np.round(tip[:, 0]).astype(int), 0, W - 1)
            yt = np.clip(np.round(tip[:, 1]).astype(int), 0, H - 1)
            arr[yt, xt] = cs_[fl2]
        fr.set_array(arr, dep)

    # ---- painter: domino chunks, props, cat, vacuum
    items = []
    chz = cam.depth(np.stack([CH_CTR[:, 0], CH_Y, CH_CTR[:, 1]], 1))
    for ci, chunk in enumerate(CHUNKS):
        if not poly_lod[chunk].any():
            continue
        items.append((chz[ci], 'chunk', chunk))
    for zo, o in prop_list:
        items.append((zo, 'obj', o))
    items.append((cam.depth(cat_ctr)[0], 'cat', (cat_faces, cat_cols)))
    items.append((cam.depth(np.array([vac_p[0], 0.05, vac_p[1]]))[0], 'vac', vac_faces))
    items.sort(key=lambda it: -it[0])
    draw = fr.draw
    for zi, kind, obj in items:
        if kind == 'chunk':
            idx = obj[poly_lod[obj]]
            moving = th[idx] > 0
            order = np.concatenate([idx[moving][::-1], idx[~moving]])
            for i in order:
                big = B.big[i]
                for k in np.where(vis[i])[0]:
                    pts = scr[i, FACE_IDX[k]]
                    c = colors[i, k]
                    if big:
                        fr.poly(cam, P[i, FACE_IDX[k]], c)
                        if k in (1, 2):
                            draw_pips(fr, cam, i, th[i], 'front' if k == 1 else 'back', (250, 242, 230) if i == B.holdout else (40, 34, 40))
                    else:
                        draw.polygon(pts.ravel().tolist(), fill=(int(c[0]), int(c[1]), int(c[2])))
        else:
            if kind == 'obj':
                faces, colr, name = obj.faces, obj.color, obj.name
                tone = obj.tone
            elif kind == 'cat':
                faces, colr, name, tone = obj[0], np.array(obj[1], float), 'cat', None
            else:
                faces, colr, name, tone = obj, np.array([62, 64, 72.0]), 'vac', None
            fc = [(Q, n, j) for j, (Q, n) in enumerate(faces) if np.dot(n, cam.pos - Q.mean(0)) > 0 or (kind == 'obj' and obj.double)]
            if not fc:
                continue
            fc.sort(key=lambda a: -cam.depth(a[0].mean(0))[0])
            C = np.array([q.mean(0) for q, _, _ in fc])
            N = np.array([n if np.dot(n, cam.pos - q.mean(0)) > 0 else -n for q, n, _ in fc])
            kvv = S.win_lit(C[:, 0], C[:, 1], C[:, 2])[:, None]
            emit = 0.0
            if name in ('lamplens',):
                emit = 0.9
            bases = np.array([(colr[j] if colr.ndim == 2 else colr) * (tone[j] if tone is not None else 1.0) for _, _, j in fc])
            cols = shade(L, N, C, bases, kvis=kvv, emit=emit)
            for (Q, n, j), c in zip(fc, cols):
                fr.poly(cam, Q, c)

    # ---- lens: focus locked on the subject
    img = fr.array()
    depth = fr.depth()
    zf = max(cam.depth(subject(t, tau))[0], 0.05)
    need = focus_need(t, tau, cam, zf)
    K = 0.012 * cam.f * min(1.2, 4.5 / zf)
    sharp = max(need, min(0.3, 1.2 / zf))
    img = dof(img, coc_map(depth, zf, K, sharp=sharp))

    # ---- glows: lamp lenses, struck chime bars, vacuum LED
    glows = []
    for lp, aim in zip(LAMPS, S.LAMP_AIM):
        g = lp + aim * 0.06
        zz = cam.depth(g)[0]
        if zz > 0.2:
            s_ = cam.wp(g)
            if depth[int(np.clip(s_[1], 0, H - 1)), int(np.clip(s_[0], 0, W - 1))] > zz - 0.3:
                glows.append((s_[0], s_[1], min(40, 0.55 * cam.f / zz), (255, 186, 110), 0.55 * lk))
    for i in np.where(hit & onscr)[0]:
        s_ = scr[i].mean(0)
        r_ = max(3.0, 0.12 * cam.f / max(zc[i], 0.1))
        glows.append((s_[0], s_[1], min(r_, 26), tuple(int(v) for v in np.clip(B.base[i] * 0.7 + 90, 0, 255)), 0.75 * flash[i]))
    if (int(tau * 2) % 2 == 0):
        zz = cam.depth(np.array([vac_p[0], 0.1, vac_p[1]]))[0]
        if zz > 0.2:
            s_ = cam.wp(np.array([vac_p[0], 0.1, vac_p[1]]))
            glows.append((s_[0], s_[1], max(2, 0.03 * cam.f / zz), (110, 255, 160), 0.6))
    img = apply_glows(img, glows)

    # ---- window beams (volumetric, quarter res) + dust
    img = beams(img, cam, depth, rays, t)

    img = grade(bloom(np.clip(img, 0, 255), thr=236, k=0.3), contrast=0.2, sat=1.15)
    img = np.clip(img * vignette(0.7, 0.62, 0.35) * top_scrim(0.5, 30, 140), 0, 255).astype(np.uint8)

    # ---- UI
    pil = Image.fromarray(img)
    out = np.asarray(pil).copy()
    hud(out, t, tau)
    draw_captions(out, t, T.CAPTIONS)
    return out


def focus_need(t, tau, cam, zf):
    """sharp band wide enough for every actively falling domino near the subject (and the holdout)"""
    act = DOM & (B.t0 <= tau) & (tau < B.t0 + B.tc + 0.25)
    pts = np.stack([B.cx[act], B.y0[act] + 0.02, B.cz[act]], 1) if act.any() else np.zeros((0, 3))
    tgt = subject(t, tau)
    pts = pts[np.linalg.norm(pts - tgt, axis=1) < 0.6] if len(pts) else pts
    extra = [tgt]
    if 31.5 < tau < 38.5:
        extra += [HOLD_P + [0, 0.05, 0], HOLD_P + [0, -0.05, 0]]
    if tau < 1.0:
        extra += [CAT.paw_target(tau), CAT.faces(tau)[2] + [0, 0.17, -0.05]]
    pts = np.concatenate([pts, np.array(extra)]) if len(pts) else np.array(extra)
    z = cam.depth(pts)
    z = z[z > 0.05]
    if not len(z):
        return 0.0
    return float(np.max(np.abs(1 - zf / z))) + 0.02


def beams(img, cam, depth, rays, t):
    h4, w4 = H // 4, W // 4
    ys = (np.arange(h4) * 4 + 2)
    xs = (np.arange(w4) * 4 + 2)
    d = rays[ys][:, xs]
    dep = np.minimum(depth[ys][:, xs], 30.0)
    acc = np.zeros((h4, w4), np.float32)
    K = 14
    for k in range(K):
        u = (k + 0.5 + 0.5 * BAYER_FULL[ys][:, xs]) / K
        tt = 0.25 + (dep - 0.25) * u
        P = cam.pos[None, None] + d * tt[..., None]
        inside = (P[..., 1] > 0.01) & (P[..., 1] < S.RH) & (P[..., 0] > S.RX0)
        acc += S.win_lit(P[..., 0], P[..., 1], P[..., 2]) * inside * (dep - 0.25) / K
    acc = np.clip(acc / 46.0, 0, 1)
    up = np.asarray(Image.fromarray((acc * 255).astype(np.uint8)).resize((W, H), Image.BILINEAR), np.float32) / 255.0
    up = np.floor(up * 6 + BAYER_FULL * 0.999) / 6
    img = img + up[..., None] * np.array([120, 84, 44.0]) * 0.28
    # dust motes drifting in the light
    D = DUST + np.stack([0.05 * np.sin(t * 0.3 + DUST_PH), 0.04 * np.sin(t * 0.21 + DUST_PH * 1.3), 0.06 * np.cos(t * 0.17 + DUST_PH)], 1)
    litd = S.win_lit(D[:, 0], D[:, 1], D[:, 2]) > 0
    if litd.any():
        Dz = cam.depth(D[litd])
        ok = Dz > 0.3
        s_ = cam.wp(D[litd][ok])
        Dz = Dz[ok]
        xi, yi = np.round(s_[:, 0]).astype(int), np.round(s_[:, 1]).astype(int)
        good = (xi >= 0) & (xi < W) & (yi >= 0) & (yi < H)
        xi, yi, Dz = xi[good], yi[good], Dz[good]
        vis = depth[yi, xi] > Dz
        a = np.clip(1.6 / Dz, 0.15, 0.9)[vis] * (0.6 + 0.4 * np.sin(t * 3 + xi[vis]))
        img[yi[vis], xi[vis]] += a[:, None] * np.array([150, 120, 80.0])
    return img


def hud(out, t, tau):
    if not (0.9 < t < 56.0 or T.T_REW < t < 59.5):
        return
    n = int(((B.t0 <= tau) & DOM).sum()) if tau >= 0 else 0
    txt = fmt_count(n)
    a = min(1.0, (t - 0.9) / 0.3) if t < 2 else 1.0
    pil = Image.fromarray(out)
    dr = ImageDraw.Draw(pil)
    x0, y0 = 14, 580
    w = 18 + len(txt) * 18
    panel(dr, x0, y0, x0 + max(w, 120), y0 + 46, fill=(20, 22, 40), edge=(90, 96, 140))
    out[:] = np.asarray(pil)
    label(out, 'DOMINOES', x0 + 8, y0 + 6, (190, 196, 230), 1)
    col = AMBER if n >= TOTAL and t < 56 else CREAM
    label(out, txt, x0 + 8, y0 + 18, col, 3)
