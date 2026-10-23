"""The Mountain King Escape renderer.

background (cave dark, or the night sky + aurora + moon in the zoom-out) -> giant ring + mountain exterior ->
cavern shell (walls, roof, crack chimney) -> per-pixel floor (moon pool through the crack with live ring /
ball shadows, torch + crystal + ball light, contact AO) -> props, rings, shards, ball in one painter list ->
per-pixel cut-away front face (rock strata) -> locked depth of field on the rings -> moon shaft, dust, moths,
glows, trail, sparks -> bloom, grade, vignette, scrim -> HUD (countdown + rings left) + captions.

    python path.py && python audio.py
    python sheet_par.py test/sheet.png 0.5 4 6.5 8.3 12 20 30 38 46 50 52.4 54.4 56.5 58.5
    python -m reelkit.video render.py audio.wav mountain_king_escape.mp4"""
import math
import os
import sys
import colorsys
import numpy as np
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, dirv, ease
from reelkit.camera import Cam
from reelkit.raster import Frame, add_glow
from reelkit.lighting import Lights, shade, fog
from reelkit.shadows import MaskGrid, directional_mask, Occluders, contact_ao
from reelkit.lens import coc_map, dof
from reelkit.post import bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, panel, CREAM, AMBER, CORAL, MINT
from reelkit.font import text_mask, stamp
from reelkit.geometry import icosa
import timeline as T
import geom as G
import path as PA
import shards as SH
import scene as S

KEY = S.KEY
KEYC = np.array([0.78, 0.86, 1.0])
C = G.C
FOGC = np.array([40, 32, 78.0])
MOON_DIR = dirv(8, 10)                 # where the moon is drawn in the zoom-out sky (shading uses KEY)


_SIL = np.array(S.SIL + [S.SIL[0]])
_SX = np.linspace(-11, 11, 221)


def _sil_top(x):
    ys = []
    for (x0, y0), (x1, y1) in zip(_SIL, _SIL[1:]):
        if min(x0, x1) <= x <= max(x0, x1) and abs(x1 - x0) > 1e-9 and max(y0, y1) > 4:
            ys.append(y0 + (y1 - y0) * (x - x0) / (x1 - x0))
    return max(ys) if ys else -99.0


_STOP = np.array([_sil_top(x) for x in _SX])


def SNOW_Y(x):
    """snowline: ~1.1 below the silhouette's top edge, only on the upper slopes"""
    top = np.interp(x, _SX, _STOP)
    return np.where(top > 10.5, top - 1.1 - 0.25 * np.sin(x * 3.1), 99.0)


def beam_factor(P):
    """1 inside the moon shaft from the crack, ~0.45 outside (scattered light)"""
    P = np.atleast_2d(P)
    v = P - S.CRACK
    s = np.clip(v @ (-KEY), 0, None)
    d = np.linalg.norm(v + np.outer(s, KEY), axis=1)
    r = 0.55 + 0.27 * s
    x = np.clip((d / r - 0.55) / 0.75, 0, 1)
    return 0.45 + 0.55 * (1 - x * x * (3 - 2 * x))


# ================================================================== static faces
def _flatten(parts):
    Q, Cc, N, B, K, E, Gp, Pi = [], [], [], [], [], [], [], []
    for pi, p in enumerate(parts):
        for j, (q, n) in enumerate(p.faces):
            Q.append(np.asarray(q, float)); Cc.append(np.asarray(q, float).mean(0)); N.append(np.asarray(n, float))
            B.append(p.color * p.tone[j]); K.append(p.kind); E.append(p.emit); Gp.append(p.grp); Pi.append(pi)
    return Q, np.array(Cc), np.array(N), np.array(B), np.array(K), np.array(E), np.array(Gp), np.array(Pi)


INTERIOR = [p for p in S.PARTS if p.kind in ('shell', 'solid', 'crystal', 'statue', 'torch')]
SQ, SC, SN, SB, SK, SE, SG, SP = _flatten(INTERIOR)
OCC = Occluders([p for p in INTERIOR if p.cast and p.kind != 'shell'])
SKV = OCC.vis(SC + SN * 0.02, SG, d=KEY)
SKV = np.where(SN @ KEY > 0, SKV, 1.0)
SBF = beam_factor(SC)
SHELL = SK == 'shell'
CRY_PART = np.array([INTERIOR[i].name == 'crystal' for i in SP])
OUTQ, OUTC, OUTN, OUTB, OUTK, _, _, _ = _flatten([p for p in S.PARTS if p.kind in ('outside', 'island')])
CUTQ = [q for q, n in S.CUT]

# floor shadow grid + static key mask (props), cached
FGRID = MaskGrid((S.X0, S.X1, S.Z0, S.ZF, 0.025))
_casters = [q for p in INTERIOR if p.cast and p.kind != 'shell' for q, n in p.faces]
STATIC_MASK = directional_mask(FGRID, _casters, KEY, blur=1.5)

STARS = np.random.default_rng(3).normal(0, 1, (900, 3))
STARS[:, 1] = np.abs(STARS[:, 1]) * 0.8 - 0.25
STARS /= np.linalg.norm(STARS, axis=1, keepdims=True)
STAR_B = np.random.default_rng(4).uniform(0.3, 1.0, 900)

# ================================================================== colours
def ring_hue(k, t):
    """ring colours step one hue along with every ring broken (eased over 0.5 s)"""
    m = T.mtime(t) if t < T.T_FINAL or t >= T.T_WRAP else T.T_FINAL
    shift = 0.0
    if t < T.T_WRAP:
        for kb, mb in T.RING_BREAK.items():
            shift += min(1.0, max(0.0, (m - mb) / 0.5))
        if t >= T.T_REFORM0:
            shift *= 1 - min(1.0, (t - T.T_REFORM0) / (T.T_REFORM1 - T.T_REFORM0))
    h = (0.02 + (k - 1) / 12.0 + shift / 12.0) % 1.0
    r, g, b = colorsys.hsv_to_rgb(h, 0.68, 1.0)
    return np.array([r, g, b]) * 255


BALL_COL = np.array([255, 236, 176.0])

# ================================================================== camera
F0A = (C + [0, 0.06, 0], 7.0, 12.0, 8.0, 40.0, 352.0)
F0B = (C + [0, 0.04, 0], 6.3, 4.0, 10.0, 40.0, 352.0)
WIDE = (C + [0, 0.4, 0], 20.5, -6.0, 16.0, 40.0, 368.0)
WIDE2 = (C + [0, 0.35, 0], 19.8, -9.0, 16.5, 40.0, 368.0)


def d_ring(k):
    return 7.4 if k == 1 else 5.6 * (G.R[k] + 0.55)


def follow(t):
    """ring-follow framing (8 s .. final chord): distance eases to the current ring's after every break"""
    k = 1
    d = d_ring(1)
    for kb in range(1, T.NRINGS):
        tb = T.RING_BREAK[kb]
        u = ease((t - tb - 0.05) / 1.1)
        d = d + (d_ring(kb + 1) - d) * u
    tgt = C + [0, 0.05 + 0.02 * d, 0]
    return (tgt, d, 6.0, 9.0 + 4.0 * min(1.0, max(0.0, (d - 8.0) / 8.0)), 40.0, 356.0)


def _lerp_state(a, b, u):
    ta, da, ya, pa, fa, sa = a
    tb, db, yb, pb, fb, sb = b
    return (np.asarray(ta) + (np.asarray(tb) - np.asarray(ta)) * u, math.exp(math.log(da) + (math.log(db) - math.log(da)) * u),
            ya + (yb - ya) * u, pa + (pb - pa) * u, fa + (fb - fa) * u, sa + (sb - sa) * u)


END_KEYS = [
    (T.T_FINAL + 0.02, None),
    (T.T_FINAL + 0.85, (np.array([-0.4, 9.0, -1.0]), 32.0, -2.0, 6.0, 40.0, 352.0)),
    (T.T_FINAL + 1.9, (np.array([0.0, 4.6, -1.5]), 64.0, 0.0, 3.5, 40.0, 330.0)),
    (T.T_REENTER - 0.75, (np.array([0.0, 4.9, -1.5]), 60.0, 2.0, 4.0, 40.0, 334.0)),
    (T.T_REENTER + 0.35, (C + [0, 1.6, 0], 11.0, 8.0, 8.0, 40.0, 350.0)),
    (57.8, F0A),
]


def osc_w(t):
    if T.T_FINAL <= t < 57.0:
        return max(0.0, 1 - (t - T.T_FINAL) / 0.6)
    if 57.0 <= t < 58.5:
        return (t - 57.0) / 1.5
    return 1.0


def cam_state(t):
    if t < 2.8 or t >= 57.8:
        tt = t + 60 if t < 2.8 else t
        st = _lerp_state(F0A, F0B, ease((tt - 57.8) / 5.0))
    elif t < 5.9:
        st = _lerp_state(F0B, WIDE, ease((t - 2.8) / 3.1))
    elif t < 6.9:
        st = _lerp_state(WIDE, WIDE2, (t - 5.9) / 1.0)
    elif t < 8.05:
        st = _lerp_state(WIDE2, follow(8.05), ease((t - 6.9) / 1.15))
    elif t < T.T_FINAL + 0.02:
        st = follow(t)
    else:
        keys = [(END_KEYS[0][0], follow(T.T_FINAL))] + END_KEYS[1:]
        st = keys[-1][1]
        for (ta, sa), (tb, sb) in zip(keys, keys[1:]):
            if ta <= t < tb:
                st = _lerp_state(sa, sb, ease((t - ta) / (tb - ta)))
                break
    tgt, d, yaw, pitch, fov, sy = st
    w = osc_w(t)
    yaw += w * (8.0 * math.sin(2 * math.pi * t / 15.0) + 3.0 * math.sin(2 * math.pi * t / 20.0 + 1.0))
    pitch += w * 2.2 * math.sin(2 * math.pi * t / 20.0)
    # shake: ring breaks (decaying) + the prestissimo rumble
    m = T.mtime(t)
    sh = 0.0
    for kb, mb in T.RING_BREAK.items():
        if 0 <= m - mb < 0.6 and t < T.T_WRAP:
            sh = max(sh, (0.35 + 0.05 * kb) * math.exp(-(m - mb) / 0.12))
    if 44.0 <= t < T.T_FINAL + 0.5:
        sh = max(sh, 0.12 + 0.25 * (t - 44.0) / (T.T_FINAL - 44.0))
    if sh > 0:
        yaw += sh * math.sin(t * 37.0) * 0.26
        pitch += sh * math.sin(t * 43.0 + 1.3) * 0.2
    return tgt, d, yaw, pitch, fov, sy


def camera(t):
    tgt, d, yaw, pitch, fov, sy = cam_state(t)
    return Cam(tgt, d, yaw, pitch, fov, sy, near=0.05)


# ================================================================== ring meshes (vectorised)
def ring_mesh(k, ph):
    """faces of ring k with its gap centred at ph: returns quads (n,4,3), normals (n,3), seg angle (n,)"""
    n = G.NSEG[k]
    g = G.GAP[k]
    span = 2 * math.pi - 2 * g
    a0 = ph + g + span * np.arange(n) / n
    a1 = a0 + span / n
    am = (a0 + a1) / 2
    r0, r1 = G.R[k] - G.TH / 2, G.R[k] + G.TH / 2

    def P(r, a, z):
        return np.stack([C[0] + r * np.cos(a), C[1] + r * np.sin(a), np.full_like(a, z)], -1)
    fi0, fo0, fo1, fi1 = P(r0, a0, G.DZ), P(r1, a0, G.DZ), P(r1, a1, G.DZ), P(r0, a1, G.DZ)
    bi0, bo0, bo1, bi1 = P(r0, a0, -G.DZ), P(r1, a0, -G.DZ), P(r1, a1, -G.DZ), P(r0, a1, -G.DZ)
    nr = np.stack([np.cos(am), np.sin(am), np.zeros(n)], -1)
    quads = np.concatenate([np.stack([fi0, fo0, fo1, fi1], 1), np.stack([bi1, bo1, bo0, bi0], 1),
                            np.stack([fo0, bo0, bo1, fo1], 1), np.stack([fi1, bi1, bi0, fi0], 1)])
    norms = np.concatenate([np.tile([0, 0, 1.0], (n, 1)), np.tile([0, 0, -1.0], (n, 1)), nr, -nr])
    angs = np.concatenate([am, am, am, am])
    # gap end caps
    caps = []
    for a, side in ((a0[0], -1), (a1[-1], 1)):
        c, s = math.cos(a), math.sin(a)
        q = np.array([[C[0] + r0 * c, C[1] + r0 * s, G.DZ], [C[0] + r1 * c, C[1] + r1 * s, G.DZ],
                      [C[0] + r1 * c, C[1] + r1 * s, -G.DZ], [C[0] + r0 * c, C[1] + r0 * s, -G.DZ]])
        caps.append((q, np.array([-s, c, 0.0]) * side, a))
    quads = np.concatenate([quads, np.array([c[0] for c in caps])])
    norms = np.concatenate([norms, np.array([c[1] for c in caps])])
    angs = np.concatenate([angs, np.array([c[2] for c in caps])])
    return quads, norms, angs


def giant_mesh(t):
    k_n, r, th, dz = 96, S.GIANT_R, 0.55, 0.45
    ph = math.radians(-62) + 0.08 * (t - 52.0)
    g = 0.11
    span = 2 * math.pi - 2 * g
    a0 = ph + g + span * np.arange(k_n) / k_n
    a1 = a0 + span / k_n
    am = (a0 + a1) / 2
    gc = S.GIANT_C

    def P(rr, a, z):
        return np.stack([gc[0] + rr * np.cos(a), gc[1] + rr * np.sin(a), np.full_like(a, z)], -1)
    r0, r1 = r - th / 2, r + th / 2
    fr = np.stack([P(r0, a0, dz), P(r1, a0, dz), P(r1, a1, dz), P(r0, a1, dz)], 1)
    ou = np.stack([P(r1, a0, dz), P(r1, a0, -dz), P(r1, a1, -dz), P(r1, a1, dz)], 1)
    inn = np.stack([P(r0, a1, dz), P(r0, a1, -dz), P(r0, a0, -dz), P(r0, a0, dz)], 1)
    nr = np.stack([np.cos(am), np.sin(am), np.zeros(k_n)], -1)
    return (np.concatenate([fr, ou, inn]), np.concatenate([np.tile([0, 0, 1.0], (k_n, 1)), nr, -nr]))


# ================================================================== helpers
def ground_shadow_polys(quads, d=KEY):
    """project quads along the key onto the floor (XZ polygons)"""
    P = quads - (quads[..., 1:2] / d[1]) * d
    return P[..., [0, 2]]


def lights_for(t, cam, ball_p):
    m = T.mtime(t)
    L = Lights(amb=(0.30, 0.28, 0.44), sky=(0.06, 0.06, 0.10))
    L.view = cam.view
    L.key('moon', KEY, KEYC)
    L.fill(dirv(150, 25), (0.12, 0.11, 0.18))
    ne = T.note_env(m, 0.1)
    be = T.break_env(m)
    for i, tp in enumerate(S.TORCHES):
        fl = 0.85 + 0.15 * math.sin(t * 13.1 + i * 2) * math.sin(t * 7.3 + i)
        L.point(tp, np.array([1.25, 0.66, 0.30]) * fl, 2.3)
    for i, (cp, col, s) in enumerate(S.CRYSTALS):
        k = 0.30 + 0.35 * ne + 0.6 * be
        L.point(cp, col / 255.0 * k, 1.1 + s)
    L.point(ball_p, np.array([1.0, 0.92, 0.7]) * (1.1 + 0.9 * ne), 1.0)
    return L, ne, be


def hit_flash(k, t):
    """(angle, strength) of the latest wall hit on ring k"""
    m = T.mtime(t)
    i = int(np.searchsorted(T.NOTE_M, m, 'right') - 1)
    for j in (i, i - 1):
        if 0 <= j < T.N_NOTES and T.NOTES[j]['ring'] == k and T.NOTES[j]['kind'] == 'bounce' and t < T.T_WRAP + 10:
            dtm = m - T.NOTES[j]['m']
            if 0 <= dtm < 0.5:
                return PA.ANG[j], math.exp(-dtm / 0.14)
    return None, 0.0


def sky_bg(cam, t):
    """night sky: gradient, aurora curtains, stars, moon"""
    d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1) @ cam.M
    d /= np.linalg.norm(d, axis=-1, keepdims=True)
    el = np.arcsin(np.clip(d[..., 1], -1, 1))
    az = np.arctan2(d[..., 0], -d[..., 2])
    k = np.clip((el + 0.3) / 1.0, 0, 1)
    col = np.array([52, 30, 96.0]) * (1 - k[..., None]) + np.array([10, 14, 44.0]) * k[..., None]
    # aurora: two curtains of vertical rays
    for a0, amp, cc, ph in ((-0.25, 0.22, (60, 255, 170), 0.0), (0.35, 0.18, (200, 90, 255), 1.7)):
        y0 = 0.18 + amp * np.sin(az * 2.3 + ph + t * 0.15) + 0.05 * np.sin(az * 7.0 + ph)
        band = np.exp(-((el - y0) / 0.07) ** 2) * (el > y0 - 0.02) + np.exp(-((el - y0 - 0.12) / 0.18) ** 2) * 0.5 * (el > y0)
        rays = 0.6 + 0.4 * np.sin(az * 60 + ph * 3 + t * 0.6) ** 2
        col = col + band[..., None] * rays[..., None] * np.array(cc) * 0.42 * np.exp(-np.abs(az - a0) / 0.9)[..., None]
    md = (d * MOON_DIR).sum(-1)
    col = col + np.clip((md - 0.93) / 0.07, 0, 1)[..., None] ** 3 * np.array([120, 130, 180.0]) * 0.5
    col = np.floor(col / 6 + BAYER_FULL[..., None]) * 6
    img = np.clip(col, 0, 255)
    # stars
    Pc = STARS @ cam.M.T
    ok = Pc[:, 2] > 0.1
    sp = np.stack([cam.cx + cam.f * Pc[:, 0] / np.maximum(Pc[:, 2], 1e-3), cam.cy - cam.f * Pc[:, 1] / np.maximum(Pc[:, 2], 1e-3)], 1)
    for (x, y), b, o in zip(sp, STAR_B, ok):
        if o and 0 <= x < W and 0 <= y < H:
            tw = 0.7 + 0.3 * math.sin(t * 3 + x * 0.37 + y)
            img[int(y), int(x)] = np.minimum(255, img[int(y), int(x)] + 200 * b * tw)
    ms = cam.screen_of_dir(MOON_DIR)
    return img, ms


def stamp_text(img, s, x, y, col, scale):
    m = text_mask(s, scale)
    stamp(img, m, int(x), int(y), col)
    return m.shape[1]


# ================================================================== the frame
def render(f):
    t = f / FPS
    m = T.mtime(t)
    cam = camera(t)
    ball_p, ball_r = PA.ball(t)
    L, ne, be = lights_for(t, cam, ball_p)
    far = cam_state(t)[1] > 26
    # ---------------- background
    ms = None
    if far:
        bg, ms = sky_bg(cam, t)
        fr = Frame(bg)
        if ms is not None:
            fr.disc(ms, 0.022 * cam.f, (240, 236, 214), n=14)
            fr.glow(ms[0], ms[1], 0.09 * cam.f, (150, 160, 230), 0.5)
    else:
        fr = Frame((30, 24, 56))
    # ---------------- far: giant ring + mountain exterior
    if far:
        gq, gn = giant_mesh(t)
        gc = gq.mean(1)
        vis = ((cam.pos - gc) * gn).sum(1) > 0
        cols = shade(L, gn[vis], gc[vis], np.array([255, 196, 92.0]), emit=0.25)
        dep = cam.depth(gc[vis])
        for j in np.argsort(-dep):
            fr.poly(cam, gq[vis][j], cols[j])
        oc = OUTC
        cols = shade(L, OUTN, oc, OUTB)
        cols = fog(cols, np.linalg.norm(oc - cam.pos, axis=1), np.array([44, 30, 90.0]), 50, 140, 0.5)
        dep = cam.depth(oc)
        for j in np.argsort(-dep):
            if np.dot(OUTN[j], cam.pos - oc[j]) > 0:
                fr.poly(cam, OUTQ[j], cols[j])
    # ---------------- static shading (all interior faces at once)
    emit = SE.copy()
    emit[CRY_PART] = 0.45 + 0.5 * ne + 0.7 * be
    scol = shade(L, SN, SC, SB, kvis=(SKV * SBF)[:, None], emit=emit[:, None])
    sdist = np.linalg.norm(SC - cam.pos, axis=1)
    scol = fog(scol, sdist, FOGC, 16, 46, 0.45)
    sdep = cam.depth(SC)
    sfacing = ((cam.pos - SC) * SN).sum(1) > 0
    # shell first (always behind)
    for j in np.argsort(-sdep):
        if SHELL[j] and sfacing[j]:
            fr.poly(cam, SQ[j], scol[j])
    # ---------------- floor (per pixel)
    rings_now = [k for k in range(1, T.NRINGS + 1) if SH.ring_whole(k, t)]
    rmesh = {k: ring_mesh(k, G.phase(k, G.ring_m(t))) for k in rings_now}
    shadow_polys = []
    for k in rings_now:
        q, n, a = rmesh[k]
        ns = G.NSEG[k]
        shadow_polys += list(ground_shadow_polys(q[:2 * ns]))
    bp = ball_p - (ball_p[1] / KEY[1]) * KEY
    shadow_polys.append(np.array([[bp[0] + ball_r * math.cos(a), bp[2] + ball_r * 0.6 * math.sin(a)] for a in np.linspace(0, 6.28, 9)[:-1]]))
    ring_mask = FGRID.raster(shadow_polys, blur=1.0)
    alb = Image.new('RGB', (W, H)); msk = Image.new('L', (W, H))
    ad, md = ImageDraw.Draw(alb), ImageDraw.Draw(msk)
    for Q, col in [(S.FLOOR_BG, np.array([70, 62, 98]))] + S.FLOOR_CELLS:
        Pc = cam.to_cam(Q)
        if (Pc[:, 2] > cam.near).all():
            pts = [tuple(p) for p in cam.proj(Pc)]
            ad.polygon(pts, fill=tuple(int(c) for c in col)); md.polygon(pts, fill=255)
    arr, depth = fr.array(), fr.depth()
    ys, xs = np.nonzero(np.asarray(msk))
    if len(xs):
        X, Z, dd = cam.ground_hit(xs.astype(float), ys.astype(float))
        I = np.tile(L.amb + L.sky, (len(X), 1))
        pool = np.hypot(X - S.POOL_C[0], (Z - S.POOL_C[2]) * 1.15)
        pk = 0.32 + 1.05 * np.clip(1 - (pool - 1.2) / 2.6, 0, 1) ** 1.6
        sh = np.maximum(FGRID.sample(STATIC_MASK, X, Z), FGRID.sample(ring_mask, X, Z))
        I += ((1 - sh) * pk * KEY[1])[:, None] * KEYC[None]
        for lp, lc, R in L.points:
            dist = np.sqrt((lp[0] - X) ** 2 + lp[1] ** 2 + (lp[2] - Z) ** 2)
            I += (max(lp[1], 0.15) / dist / (1 + (dist / R) ** 2))[:, None] * lc[None] * 1.1
        I *= contact_ao(X, Z, S.FEET)[:, None]
        col = np.asarray(alb)[ys, xs] * I
        col = fog(col, dd, FOGC, 16, 46, 0.45)
        arr[ys, xs] = np.floor(col / 5 + BAYER_FULL[ys, xs, None]) * 5
        depth[ys, xs] = dd
    fr.set_array(arr, depth)
    # ---------------- painter list: props + rings + shards + ball
    items = []
    for j in np.nonzero(~SHELL & sfacing)[0]:
        items.append((sdep[j], SQ[j], scol[j]))
    for k in rings_now:
        q, n, a = rmesh[k]
        c = q.mean(1)
        fac = ((cam.pos - c) * n).sum(1) > 0
        q, n, a, c = q[fac], n[fac], a[fac], c[fac]
        hue = ring_hue(k, t)
        em = np.full(len(q), 0.16)
        ha, hs = hit_flash(k, t)
        if hs > 0:
            dist = np.abs((a - ha + math.pi) % (2 * math.pi) - math.pi)
            em += 1.4 * hs * np.clip(1 - dist / 0.6, 0, 1)
        if T.T_REFORM0 <= t < T.T_WRAP:
            em += 0.5 * max(0.0, 1 - (t - SH.RING_DONE[k]) / 0.3) * (t >= SH.RING_DONE[k])
        seg = np.round((a - G.phase(k, G.ring_m(t))) * G.NSEG[k] / (2 * math.pi)).astype(int)
        tone = 1 + 0.09 * np.sin(seg * 2.39 + k) * (n[:, 2] > 0.5) - 0.12 * (np.abs(n[:, 2]) < 0.5)
        cols = shade(L, n, c, hue[None] * tone[:, None], kvis=beam_factor(c)[:, None], emit=em[:, None], rimk=0.7)
        dep = cam.depth(c)
        for j in range(len(q)):
            items.append((dep[j], q[j], cols[j]))
    shard_glints = []
    for s in SH.SHARDS:
        if t < s.t0 - 1e-6:
            continue
        P = SH.pose(s, t)
        if P is None:
            continue
        c = P.mean(0)
        n = np.cross(P[1] - P[0], P[2] - P[0])
        n /= np.linalg.norm(n) + 1e-9
        if np.dot(n, cam.pos - c) < 0:
            n = -n
        hue = ring_hue(s.k, min(t, s.t0 + 0.01))
        landed = t > s.t2
        col = shade(L, n[None], c[None], hue * (0.8 if landed else 1.0), kvis=beam_factor(c)[:, None], emit=0.25 if not landed else 0.12)[0]
        items.append((cam.depth(c)[0], P, col))
        if not landed and (s.seg + s.half) % 5 == 0:
            shard_glints.append((c, hue))
    # ball
    bq = icosa(ball_p, ball_r, jit=0.0)
    for Q, n in bq:
        c = Q.mean(0)
        if np.dot(n, cam.pos - c) > 0:
            col = shade(L, n[None], c[None], BALL_COL, emit=0.95)[0]
            items.append((cam.depth(c)[0] - 0.01, Q, col))
    items.sort(key=lambda it: -it[0])
    for dz, Q, col in items:
        fr.poly(cam, Q, col)
    # ---------------- the cut-away front face (per pixel strata)
    cm = Image.new('L', (W, H))
    cd = ImageDraw.Draw(cm)
    for Q in CUTQ:
        Pc = cam.to_cam(Q)
        if (Pc[:, 2] > cam.near).all():
            cd.polygon([tuple(p) for p in cam.proj(Pc)], fill=255)
    arr, depth = fr.array(), fr.depth()
    ys, xs = np.nonzero(np.asarray(cm))
    if len(xs):
        dw = cam.pixel_rays(xs.astype(float), ys.astype(float))
        s_ = (S.ZF - cam.pos[2]) / np.where(np.abs(dw[:, 2]) < 1e-6, -1e-6, dw[:, 2])
        okp = s_ > 0
        xs, ys, s_, dw = xs[okp], ys[okp], s_[okp], dw[okp]
        PX = cam.pos[0] + s_ * dw[:, 0]
        PY = cam.pos[1] + s_ * dw[:, 1]
        band = (PY + 0.22 * np.sin(PX * 0.9) + 0.12 * np.sin(PX * 2.6)) / 0.55
        bi = np.floor(band).astype(int) % 4
        pal = np.array([[96, 82, 122], [104, 88, 126], [90, 78, 118], [110, 92, 128.0]])
        base = pal[bi] * (1 + 0.05 * np.sin(PX * 3.3 + PY * 1.7))[:, None]
        snow = np.clip((PY - SNOW_Y(PX)) / 0.25, 0, 1)
        base = base * (1 - snow[:, None]) + np.array([214, 226, 246.0]) * snow[:, None]
        rim = np.clip(1 - np.abs(PY - 0.0) / 0.25, 0, 1)
        base = base * (1 - 0.3 * rim[:, None])
        I = np.tile(L.amb * 1.05 + 0.10, (len(xs), 1))
        for lp, lc, R in L.points[:4]:
            dist = np.sqrt((lp[0] - PX) ** 2 + (lp[1] - PY) ** 2 + (lp[2] - S.ZF) ** 2)
            I += (0.6 / (1 + (dist / (R * 1.6)) ** 2))[:, None] * lc[None]
        col = base * I
        col = fog(col, s_, np.array([44, 30, 90.0]), 50, 140, 0.5)
        arr[ys, xs] = np.floor(col / 5 + BAYER_FULL[ys, xs, None]) * 5
        depth[ys, xs] = s_
    fr.set_array(arr, depth)
    # ---------------- lens (focus locked on the rings)
    img, depth = fr.array(), fr.depth()
    zf, sharp = focus(cam, t)
    K = 0.011 * cam.f * min(1.2, 4.5 / zf)
    coc = coc_map(depth, zf, K, sharp=sharp)
    img = dof(img, coc)
    # ---------------- light: shaft, glows, trail, sparks
    img = add_shaft(img, depth, cam, t, ne)
    for i, tp in enumerate(S.TORCHES):
        sp = cam.wp(tp)
        z = cam.depth(tp)[0]
        if z > 0.2 and _vis(depth, sp, z):
            fl = 0.85 + 0.15 * math.sin(t * 13.1 + i * 2) * math.sin(t * 7.3 + i)
            add_glow(img, sp[0], sp[1], 1.0 * cam.f / z, (255, 160, 70), 0.55 * fl)
            add_glow(img, sp[0], sp[1] - 0.08 * cam.f / z, 0.28 * cam.f / z, (255, 230, 150), 0.9 * fl)
    for cp, col, s in S.CRYSTALS:
        sp = cam.wp(cp)
        z = cam.depth(cp)[0]
        if z > 0.3 and _vis(depth, sp, z, 0.6):
            add_glow(img, sp[0], sp[1], 1.1 * s * cam.f / z, col, 0.18 + 0.25 * ne + 0.35 * be)
    if t >= 28.0 and t < T.T_WRAP:
        eye = min(1.0, (t - 28.0) / 2.0) * (0.75 + 0.25 * ne + 0.5 * be)
        for ep in S.STATUE_EYES:
            sp = cam.wp(ep)
            z = cam.depth(ep)[0]
            if z > 0.3 and _vis(depth, sp, z, 0.4):
                add_glow(img, sp[0], sp[1], max(1.5, 0.07 * cam.f / z), (255, 70, 40), 0.9 * eye)
    cs = cam.wp(S.CRACK_SKY)
    zc = cam.depth(S.CRACK_SKY)[0]
    if zc > 0.5 and _vis(depth, cs, zc, 1.5):
        add_glow(img, cs[0], cs[1], 0.7 * cam.f / zc, (200, 220, 255), 0.7)
    draw_trail(img, cam, t, depth)
    draw_sparks(img, cam, t, depth)
    for c, hue in shard_glints:
        sp = cam.wp(c)
        z = cam.depth(c)[0]
        if z > 0.3:
            add_glow(img, sp[0], sp[1], max(1.5, 0.06 * cam.f / z), hue, 0.45)
    bs = cam.wp(ball_p)
    bz = cam.depth(ball_p)[0]
    if bz > 0.2 and _vis(depth, bs, bz, 0.35 + 2 * ball_r):
        add_glow(img, bs[0], bs[1], max(3.0, 4.2 * ball_r * cam.f / bz), (255, 210, 130), 0.5 + 0.4 * ne)
        add_glow(img, bs[0], bs[1], max(1.5, 1.9 * ball_r * cam.f / bz), (255, 250, 230), 0.8)
    burst(img, cam, t)
    # ---------------- post
    img = grade(bloom(np.clip(img, 0, 255), thr=215, k=0.35), contrast=0.14, sat=1.12)
    img = np.clip(img * vignette(0.7) * top_scrim(), 0, 255).astype(np.uint8)
    out = np.ascontiguousarray(img)
    hud(out, t)
    draw_captions(out, t, T.CAPTIONS)
    return out


def _vis(depth, sp, z, slack=0.25):
    x, y = int(round(sp[0])), int(round(sp[1]))
    if not (0 <= x < W and 0 <= y < H):
        return 0 <= x + 20 < W + 40 and 0 <= y + 20 < H + 40
    return depth[y, x] > z - slack


def focus(cam, t):
    """focus distance + sharp band: every intact ring (and the ball) stays at CoC 0"""
    tgt, d, *_ = cam_state(t)
    if T.T_FINAL <= t < T.T_REENTER + 0.4:
        zf = float(cam.depth(tgt)[0])
        pts = [PA.ball(t)[0], tgt]
    else:
        zf = float(cam.depth(C)[0])
        kmax = max([k for k in range(1, T.NRINGS + 1) if SH.ring_whole(k, t)] or [T.NRINGS])
        a = np.linspace(0, 2 * math.pi, 24, endpoint=False)
        r = G.R[kmax] + G.TH / 2
        ring = np.stack([C[0] + r * np.cos(a), C[1] + r * np.sin(a), np.zeros_like(a)], 1)
        pts = list(ring + [0, 0, G.DZ]) + list(ring - [0, 0, G.DZ]) + [PA.ball(t)[0]]
    z = cam.depth(np.array(pts))
    z = z[z > 0.1]
    need = float(np.max(np.abs(1 - zf / z))) + 0.03 if len(z) else 0.1
    return zf, max(need, min(0.28, 1.0 / zf))


def subject_blur(t):
    """max CoC (px) over the subject points for the focus audit"""
    cam = camera(t)
    zf, sharp = focus(cam, t)
    K = 0.011 * cam.f * min(1.2, 4.5 / zf)
    kmax = max([k for k in range(1, T.NRINGS + 1) if SH.ring_whole(k, t)] or [T.NRINGS])
    a = np.linspace(0, 2 * math.pi, 24, endpoint=False)
    pts = [PA.ball(t)[0]]
    if not (T.T_FINAL <= t < T.T_REENTER + 0.4):
        for k in range(1, kmax + 1):
            if SH.ring_whole(k, t):
                r = G.R[k]
                ring = np.stack([C[0] + r * np.cos(a), C[1] + r * np.sin(a), np.zeros_like(a)], 1)
                pts += list(ring + [0, 0, G.DZ]) + list(ring - [0, 0, G.DZ])
    z = cam.depth(np.array(pts))
    sx = cam.wp(np.array(pts))
    on = (z > 0.1) & (sx[:, 0] >= 0) & (sx[:, 0] < W) & (sx[:, 1] >= 0) & (sx[:, 1] < H)
    if not on.any():
        return 0.0
    return float(coc_map(z[on], zf, K, sharp=sharp).max())


SHAFT_R0, SHAFT_K = 0.55, 0.27
DUST = np.random.default_rng(21).uniform(0, 1, (160, 3))
MOTHS = np.random.default_rng(22).uniform(0, 1, (4, 4))


def add_shaft(img, depth, cam, t, ne):
    """volumetric moon shaft from the crack to the floor (screen-space, depth-tested), dust, moths"""
    s_end = S.CRACK[1] / KEY[1]
    ss = np.linspace(0, s_end, 14)
    axis = S.CRACK[None] - np.outer(ss, KEY)
    Pc = cam.to_cam(axis)
    if (Pc[:, 2] < 0.3).any():
        return img
    sp = cam.proj(Pc)
    rad = (SHAFT_R0 + SHAFT_K * ss) * cam.f / Pc[:, 2]
    # distance of each pixel to the projected axis polyline, with interpolated radius / depth
    a = sp[0]
    b = sp[-1]
    ab = b - a
    L2 = float(ab @ ab) + 1e-9
    u = np.clip(((XX - a[0]) * ab[0] + (YY - a[1]) * ab[1]) / L2, 0, 1)
    px = a[0] + u * ab[0]
    py = a[1] + u * ab[1]
    dist = np.hypot(XX - px, YY - py)
    r = np.interp(u, np.linspace(0, 1, len(ss)), rad)
    zb = np.interp(u, np.linspace(0, 1, len(ss)), Pc[:, 2])
    prof = np.clip(1 - (dist / np.maximum(r, 1e-3)) ** 2, 0, 1) ** 1.4
    inten = prof * (0.75 - 0.35 * u) * (depth > zb - 1.2)
    inten = np.floor(inten * 7 + BAYER_FULL * 0.999) / 7
    img = img + inten[..., None] * np.array([150, 175, 235.0]) * 0.34
    # dust motes drifting in the shaft
    for i, (a1, a2, a3) in enumerate(DUST):
        s = ((a1 + t * 0.012 * (0.5 + a2)) % 1.0) * s_end
        ang = a2 * 6.283 + t * 0.2 * (a3 - 0.5)
        rr = (SHAFT_R0 + SHAFT_K * s) * math.sqrt(a3) * 0.9
        u1 = np.cross(KEY, [1, 0, 0]); u1 /= np.linalg.norm(u1)
        u2 = np.cross(KEY, u1)
        P = S.CRACK - KEY * s + (u1 * math.cos(ang) + u2 * math.sin(ang)) * rr
        z = cam.depth(P)[0]
        if z < 0.3:
            continue
        q = cam.wp(P)
        x, y = int(q[0]), int(q[1])
        if 0 <= x < W and 0 <= y < H and depth[y, x] > z - 0.2:
            tw = 0.5 + 0.5 * math.sin(t * 4 + i)
            img[y, x] += np.array([200, 215, 255.0]) * (0.45 + 0.4 * tw)
    # moths fluttering round the shaft
    for i, (a1, a2, a3, a4) in enumerate(MOTHS):
        s = 2.5 + a1 * 4.5 + 0.6 * math.sin(t * (0.7 + a2) + i)
        ang = t * (0.9 + a3) + a4 * 6.283
        rr = (SHAFT_R0 + SHAFT_K * s) * (0.7 + 0.25 * math.sin(t * 1.3 + i))
        u1 = np.cross(KEY, [1, 0, 0]); u1 /= np.linalg.norm(u1)
        u2 = np.cross(KEY, u1)
        P = S.CRACK - KEY * s + (u1 * math.cos(ang) + u2 * math.sin(ang)) * rr
        z = cam.depth(P)[0]
        if z < 0.3:
            continue
        q = cam.wp(P)
        x, y = int(q[0]), int(q[1])
        flap = math.sin(t * 38 + i * 3) > 0
        if 2 <= x < W - 2 and 2 <= y < H - 2 and depth[y, x] > z - 0.2:
            wing = np.array([210, 196, 160.0])
            img[y, x] = img[y, x] * 0.4 + wing * 0.6
            if flap:
                img[y - 1, x - 1] = img[y - 1, x - 1] * 0.5 + wing * 0.5
                img[y - 1, x + 1] = img[y - 1, x + 1] * 0.5 + wing * 0.5
            else:
                img[y, x - 1] = img[y, x - 1] * 0.6 + wing * 0.4
                img[y, x + 1] = img[y, x + 1] * 0.6 + wing * 0.4
    return img


def draw_trail(img, cam, t, depth):
    """light streaks behind the ball (from 20 s)"""
    if not (20.0 <= t < T.T_REENTER + 0.6 or t >= T.T_WRAP and False):
        return
    w = min(1.0, (t - 20.0) / 1.5)
    n = 26
    ts = t - np.linspace(0.24, 0.0, n)
    pts = []
    for tt in ts:
        if tt < 20.0 and t < 52:
            pts.append(None)
            continue
        p, r = PA.ball(tt)
        z = cam.depth(p)[0]
        if z < 0.2:
            pts.append(None)
            continue
        q = cam.wp(p)
        pts.append((q, z) if _vis(depth, q, z, 0.6) else None)
    lay = Image.new('F', (W, H), 0.0)
    dr = ImageDraw.Draw(lay)
    for i in range(1, n):
        if pts[i] is None or pts[i - 1] is None:
            continue
        v = (i / n) ** 1.6
        wd = max(1, int(round(1 + 2.5 * v * min(1.0, 8.0 / pts[i][1]))))
        dr.line([tuple(pts[i - 1][0]), tuple(pts[i][0])], fill=float(v), width=wd)
    a = np.asarray(lay)
    a = np.floor(a * 5 + BAYER_FULL * 0.999) / 5
    k = T.ring_at(T.mtime(t))
    hue = ring_hue(min(k, T.NRINGS), t)
    col = 0.55 * np.array([255, 226, 160.0]) + 0.45 * hue
    img += a[..., None] * col * 0.9 * w


def draw_sparks(img, cam, t, depth):
    m = T.mtime(t)
    i = int(np.searchsorted(T.NOTE_M, m, 'right') - 1)
    for j in range(max(0, i - 2), i + 1):
        n = T.NOTES[j]
        age = m - n['m']
        if not (0 <= age < 0.35) or n['kind'] != 'bounce' or (T.T_FINAL <= t < T.T_WRAP):
            continue
        r = np.random.default_rng(j)
        p0 = PA.X[j]
        k = n['ring']
        out = -np.array([math.cos(PA.ANG[j]), math.sin(PA.ANG[j]), 0])
        hue = ring_hue(k, t)
        cnt = 10 if n.get('near') else 6
        for s in range(cnt):
            v = out * r.uniform(0.6, 1.8) + r.normal(0, 0.7, 3)
            p = p0 + v * age + np.array([0, -2.0 * age * age, 0])
            z = cam.depth(p)[0]
            if z < 0.2:
                continue
            q = cam.wp(p)
            x, y = int(q[0]), int(q[1])
            if 0 <= x < W - 1 and 0 <= y < H - 1:
                b = (1 - age / 0.35)
                c = (np.array([255, 245, 210.0]) * 0.6 + hue * 0.4) * b
                img[y, x] += c
                if n.get('near'):
                    img[y, x + 1] += c * 0.7


def burst(img, cam, t):
    """an expanding flash ring where each ring breaks"""
    m = T.mtime(t)
    if t >= T.T_WRAP:
        return
    for k, mb in T.RING_BREAK.items():
        age = m - mb
        if 0 <= age < 0.45:
            g = PA.X[[i for i, n in enumerate(T.NOTES) if n['ring'] == k and n['kind'] != 'bounce'][0]]
            s = cam.wp(g)
            z = cam.depth(g)[0]
            if z > 0.3:
                hue = ring_hue(k, mb - 0.01)
                add_glow(img, s[0], s[1], (0.6 + 2.2 * age) * cam.f / z * 0.6, hue, 0.9 * (1 - age / 0.45))
                add_glow(img, s[0], s[1], 0.3 * cam.f / z, (255, 250, 235), 1.2 * (1 - age / 0.45) ** 2)


def hud(out, t):
    pil = Image.fromarray(out)
    d = ImageDraw.Draw(pil)
    panel(d, 16, 560, 344, 618)
    out[:] = np.asarray(pil)
    left = T.countdown(t)
    col = CREAM if left > 20 else (AMBER if left > 10 else CORAL)
    if T.T_FINAL <= t < T.T_REENTER:
        col = MINT
    s = f'{left:05.2f}'
    stamp_text(out, s, 30, 572, col, 4)
    stamp_text(out, 'RINGS LEFT', 214, 568, (170, 182, 220), 1)
    n = T.hud_rings(t)
    for i in range(12):
        x = 214 + (i % 6) * 20
        y = 582 + (i // 6) * 14
        on = i < n
        c = ring_hue(i + 1, t) if on else np.array([50, 56, 84])
        out[y:y + 9, x:x + 14] = (10, 14, 26)
        out[y + 1:y + 8, x + 1:x + 13] = c.astype(np.uint8)
