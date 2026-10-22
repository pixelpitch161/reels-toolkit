"""The Lowest Note in the Universe: renderer.

    python -m reelkit.sheet render.py test/sheet.png 1 6 12 22 30 40 48 58
    python -m reelkit.video render.py audio.wav lowest_note.mp4

Pipeline: banded nebula backdrop + stars + far galaxies -> island underside -> per-pixel rim floor
(starlight, core light and lamp, each with real shadow masks, + contact AO) -> per-pixel gas basin
(hot-gas haze, pressure-wave crests with their shadows, colour bands per octave) -> painter-sorted
faces (lip, props, black hole, jets, bubbles, crest ridges, galaxies, satellite) -> locked DOF on
the black hole -> glows -> light shafts -> bloom -> grade -> scrim -> captions + octave HUD."""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, ease, dirv, mixc
from reelkit.camera import Cam, interp_keys
from reelkit.raster import Frame, apply_glows, SKY_DEPTH
from reelkit.lighting import Lights, shade, fog
from reelkit.shadows import MaskGrid, directional_mask, point_mask, Occluders, contact_ao, proj_point
from reelkit.lens import coc_map, dof
from reelkit.post import god_rays, bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, panel, label, slide_offset, MINT, CREAM, AMBER, CYAN, CORAL, PINK, SKYB
from reelkit import font as RF
from reelkit.geometry import face, icosa, lathe
import timeline as T
import scene as S

# glyphs this video needs (added at runtime, reelkit itself is unchanged)
for ch, rows in {'♭': "#....|#....|#.##.|##..#|#...#|#..#.|###..",
                 '≈': ".....|.##.#|#..#.|.....|.##.#|#..#.|.....",
                 '→': ".....|..#..|...#.|#####|...#.|..#..|....."}.items():
    RF.GLYPHS[ch] = np.array([[c == '#' for c in r] for r in rows.split('|')])

# ------------------------------------------------------------------ camera: gentle 2.5D moves
TGT = np.array([0.0, 1.35, 0.0])
OPEN = (0.0, TGT, 17.2, -10, 31, 48, 352)
KEYS = [  # (t, target, dist, yaw, pitch, fov, sy)
    OPEN,
    (3.6, TGT, 16.6, -8, 30, 48, 350),
    (7.6, S.CORE + [0, -0.15, 0], 12.6, 4, 22, 46, 368),        # push in: jets fire, bubble inflates
    (10.8, TGT + [0, -0.2, 0], 16.0, 8, 36, 48, 336),           # haze lights up
    (14.0, TGT, 17.0, 6, 40, 48, 330),
    (21.0, TGT, 18.0, -6, 46, 48, 326),
    (28.6, TGT, 17.0, -14, 44, 48, 330),
    (34.8, TGT, 15.5, -4, 40, 48, 332),
    (38.4, TGT + [0, -0.2, 0], 16.5, 6, 58, 48, 330),           # look down on the colour bands
    (44.0, TGT, 17.0, 10, 50, 48, 330),
    (52.6, TGT, 18.0, -12, 38, 48, 342),
    (58.4, TGT, 17.2, -10, 31, 48, 352),
    (60.0,) + OPEN[1:],
]

# ------------------------------------------------------------------ lights + cached shadow masks
STAR_COL = np.array([0.42, 0.5, 0.78])
CORE_COL = np.array([1.25, 0.98, 0.74])
LAMP_COL = np.array([1.1, 0.66, 0.3])
FLOOR = MaskGrid((-5.6, 5.6, -5.6, 5.6, 0.025))
_cast = [o for o in S.OBJS if o.cast]
_cast_faces = [Q for o in _cast for Q, n in o.faces]
_lip_faces = [Q for o in S.LIP for Q, n in o.faces]
STAR_MASK0 = directional_mask(FLOOR, _cast_faces + _lip_faces, S.STAR_D, blur=1.6)
CORE_MASK0 = point_mask(FLOOR, _cast_faces + _lip_faces, S.CORE, blur=2.2)
LAMP_MASK0 = point_mask(FLOOR, [Q for o in _cast if o.name not in ('post',) for Q, n in o.faces], S.LAMP, blur=2.6)
OCC = Occluders(_cast)

# static prop faces: precompute centres, normals, shadow visibility (static lights)
_SF = []
for o in S.OBJS:
    for j, (Q, n) in enumerate(o.faces):
        _SF.append((Q, n, o.color * o.tone[j], o.emit, o.grp, o.name))
SF_Q = [f[0] for f in _SF]
SF_C = np.array([f[0].mean(0) for f in _SF])
SF_N = np.array([f[1] for f in _SF])
SF_B = np.array([f[2] for f in _SF])
SF_E = np.array([f[3] for f in _SF])
SF_G = np.array([f[4] for f in _SF])
SF_NAME = [f[5] for f in _SF]
SF_STARV = OCC.vis(SF_C + SF_N * 0.02, SF_G, d=S.STAR_D)
SF_COREV = OCC.vis(SF_C + SF_N * 0.02, SF_G, lp=S.CORE)
SF_HOT = np.array([nm in ('slit', 'win', 'lamp') for nm in SF_NAME])
SF_LIP = np.array([nm == 'lip' for nm in SF_NAME])
UF_Q = [Q for o in S.UNDER for Q, n in o.faces]
UF_C = np.array([Q.mean(0) for Q in UF_Q])
UF_N = np.array([n for o in S.UNDER for Q, n in o.faces])
UF_B = np.array([o.color * o.tone[j] for o in S.UNDER for j in range(len(o.faces))])

# nebula texture on the direction sphere (lat-long), two colour clouds
_rng = np.random.default_rng(5)


def _noise(h, w, octaves):
    out = np.zeros((h, w))
    for k, amp in octaves:
        g = _rng.normal(0, 1, (k, 2 * k))
        im = Image.fromarray(((g - g.min()) / (np.ptp(g) + 1e-9) * 255).astype(np.uint8)).resize((w, h), Image.BICUBIC)
        out += (np.asarray(im, float) / 255 - 0.5) * amp
    return out


NEB_H, NEB_W = 180, 360
NEB1 = np.clip(_noise(NEB_H, NEB_W, [(4, 1.0), (9, 0.6), (20, 0.3)]) * 1.6 + 0.1, 0, 1)
NEB2 = np.clip(_noise(NEB_H, NEB_W, [(5, 1.0), (11, 0.5), (24, 0.3)]) * 1.6 - 0.05, 0, 1)
HAZE_N = _noise(256, 256, [(5, 1.0), (12, 0.6), (28, 0.35)])          # gas turbulence (pool XZ)

# ------------------------------------------------------------------ colours
HAZE_STOPS = [(0.0, (255, 238, 210)), (0.55, (255, 196, 172)), (1.25, (246, 132, 190)), (2.0, (186, 98, 218)),
              (2.7, (118, 88, 214)), (3.45, (74, 74, 178))]
_HR = np.array([s[0] for s in HAZE_STOPS])
_HC = np.array([s[1] for s in HAZE_STOPS], float)
BAND_COLS = [(255, 112, 92), (255, 192, 84), (124, 244, 170), (96, 212, 255), (196, 126, 255)]
BAND_EDGES = [0.55, 1.1, 1.65, 2.2, 2.75, 3.3]
CREST = np.array([200, 246, 255], float)


def haze_col(r):
    return np.stack([np.interp(r, _HR, _HC[:, c]) for c in range(3)], -1)


def core_k(t):
    """black hole brightness: brighter when the jets fire, pumping on every crest / beat"""
    return (0.75 + 0.35 * T.jets_k(t)) * (1 + 0.35 * T.kick_env(t, 0.1) + 0.25 * T.beat_env(t))


# ------------------------------------------------------------------ backdrop
def backdrop(cam, t):
    d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1) @ cam.M
    d /= np.linalg.norm(d, axis=-1, keepdims=True)
    el = np.arcsin(np.clip(d[..., 1], -1, 1))
    az = np.arctan2(d[..., 0], -d[..., 2])
    e = el / (math.pi / 2)
    lo = np.array([30, 22, 78.0]); mid = np.array([84, 48, 138.0]); hi = np.array([20, 16, 58.0])
    k_lo = np.clip(-e / 0.7, 0, 1)[..., None] ** 0.8
    k_hi = np.clip(e / 0.5, 0, 1)[..., None] ** 0.8
    col = mid * (1 - k_lo - k_hi) + lo * k_lo + hi * k_hi
    iy = np.clip(((el / math.pi + 0.5) * (NEB_H - 1)).astype(np.int32), 0, NEB_H - 1)
    ix = (((az / (2 * math.pi)) % 1.0) * (NEB_W - 1)).astype(np.int32)
    n1, n2 = NEB1[iy, ix][..., None], NEB2[iy, ix][..., None]
    col = col + n1 * np.array([120, 40, 110.0]) * 0.8 + n2 * np.array([30, 90, 130.0]) * 0.75
    col = np.floor(col / 7 + BAYER_FULL[..., None]) * 7
    return np.clip(col, 0, 255)


def draw_stars(img, cam, t):
    Pc = S.STAR_V @ cam.M.T
    ok = Pc[:, 2] > 0.2
    p = cam.proj(Pc[ok] * 1e4)
    b = S.STAR_B[ok] * (0.7 + 0.3 * np.sin(2 * math.pi * S.STAR_TW[ok] * t / 60 * 7 + S.STAR_PH[ok]))
    b = b * (1 + 0.6 * T.beat_env(t))
    cols = S.STAR_C[ok]
    for (x, y), bb, c in zip(p, b, cols):
        xi, yi = int(x), int(y)
        if 0 <= xi < W and 0 <= yi < H:
            img[yi, xi] = np.maximum(img[yi, xi], c * min(1.0, 0.35 + bb))
            if bb > 0.72 and 1 <= xi < W - 1 and 1 <= yi < H - 1:
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    img[yi + dy, xi + dx] = np.maximum(img[yi + dy, xi + dx], c * 0.55 * bb)


def far_galaxies(fr, cam, t):
    for c, R, nrm, ccore, carm, spin in S.FAR_GALAXIES:
        faces = S.disc_galaxy(c, R, nrm, spin * 2 * math.pi * t / 60.0, n=16)
        for Q, n, ring_i, arm in faces:
            if ring_i > 0 and not arm:
                continue                                   # inter-arm gaps: let the nebula show through
            base = np.array(ccore if ring_i == 0 else carm, float)
            k = 1.0 if ring_i == 0 else (0.9 if ring_i == 1 else 0.7)
            col = fog(base * k, 200.0, (90, 56, 150), 50, 300, 0.4)
            fr.poly(cam, Q, col)
        s = cam.wp(c)
        fr.glow(s[0], s[1], R * cam.f / max(cam.depth(c)[0], 1) * 0.6, ccore, 0.18)


# ------------------------------------------------------------------ moving parts
def bubble_radius(phase, cyc):
    rmax = min(0.62, 0.16 + 0.28 * cyc)
    return 0.1 + (rmax - 0.1) * (phase ** 0.75), rmax


def moving_parts(t, cam):
    """returns faces [(Q, n, base, emit_k, kind)] and glows"""
    F = []
    G = []
    ck = core_k(t)
    jk = T.jets_k(t)
    kick = T.kick_env(t, 0.1)
    be = T.beat_env(t)
    # black hole: dark sphere, hot accretion disc, glow of the host galaxy
    for Q, n in icosa(S.CORE, 0.2, (1, 1, 1), jit=0.05, rng=np.random.default_rng(3)):
        F.append((Q, n, np.array([24, 18, 44.0]), 0.0, 'bh'))
    tilt = np.array([0.0, 1.0, 0.28]); tilt /= np.linalg.norm(tilt)
    u = np.cross(tilt, [1.0, 0, 0]); u /= np.linalg.norm(u)
    v = np.cross(tilt, u)
    nseg = 18
    spin = 2 * math.pi * t / 6.0
    for ring_i, (r0, r1, col) in enumerate(((0.26, 0.42, (255, 250, 230)), (0.42, 0.6, (255, 200, 120)), (0.6, 0.8, (255, 130, 80)))):
        for k in range(nseg):
            a0, a1 = spin + 2 * math.pi * k / nseg, spin + 2 * math.pi * (k + 1) / nseg
            P = lambda rr, a: S.CORE + rr * (math.cos(a) * u + math.sin(a) * v)
            Q = np.array([P(r0, a0), P(r0, a1), P(r1, a1), P(r1, a0)])
            flick = 0.9 + 0.1 * math.sin(k * 2.1 + t * 3 + ring_i)
            F.append((Q, tilt, np.array(col, float) * flick, 1.0, 'disc'))
    s = cam.wp(S.CORE)
    zc = max(cam.depth(S.CORE)[0], 0.5)
    pre = [(s[0], s[1], 1.6 * cam.f / zc, (255, 170, 120), 0.3 * ck), (s[0], s[1], 0.62 * cam.f / zc, (255, 220, 170), 0.32 * ck)]
    # jets
    L_up = 1.0 + 2.2 * jk * (1 + 0.12 * be)
    top = S.CORE + S.JET_AX * L_up
    bot = S.CORE - S.JET_AX * (S.CORE[1] - S.POOL_Y - 0.08) / S.JET_AX[1]
    jcol = np.array([176, 226, 255.0])
    for Q, n in S.cone(S.CORE + S.JET_AX * 0.12, top, 0.13 * (0.6 + 0.4 * jk), 6):
        F.append((Q, n, jcol, 0.95, 'jet'))
    for Q, n in S.cone(S.CORE - S.JET_AX * 0.12, bot, 0.11 * (0.6 + 0.4 * jk), 6):
        F.append((Q, n, jcol, 0.95, 'jet'))
    for k in range(7):
        u_ = ((t * 0.9 + k / 7) % 1.0)
        p = S.CORE + S.JET_AX * (0.2 + u_ * (L_up - 0.2))
        sp = cam.wp(p)
        G.append((sp[0], sp[1], 0.32 * cam.f / max(cam.depth(p)[0], 0.5), (150, 210, 255), (0.16 + 0.1 * kick) * jk * (1 - u_ * 0.5)))
    # bubbles: one inflating at the jet tip (and its partner dome in the gas under the core)
    ph, cyc = T.bubble_phase(t)
    rb, rmax = bubble_radius(ph, cyc)
    bc = top + S.JET_AX * rb * 0.9
    for Q, n in S.sphere(bc, rb):
        F.append((Q, n, np.array([226, 210, 255.0]), 0.12, 'bubble'))
    sb = cam.wp(bc)
    zb = max(cam.depth(bc)[0], 0.5)
    G.append((sb[0], sb[1], rb * 1.7 * cam.f / zb, (180, 150, 255), 0.14))
    G.append((sb[0] - 0.35 * rb * cam.f / zb, sb[1] - 0.4 * rb * cam.f / zb, max(1.5, 0.22 * rb * cam.f / zb), (255, 255, 255), 0.5))
    rd = T.R0 * (0.35 + 0.65 * ph ** 0.8)
    prof = [(0.0, rd), (rd * 0.38, rd * 0.92), (rd * 0.7, rd * 0.7), (rd * 0.92, rd * 0.38), (rd * 0.98, 0.0)]
    for Q, n in lathe(0.0, 0.0, prof, n=12, cy=S.POOL_Y - 0.02):
        F.append((Q, n, np.array([255, 196, 220.0]), 0.55, 'dome'))
    # ghost bubbles rising from earlier pops (outlines)
    ghosts = []
    for tl, a, g in T.recent_launches(t, 5):
        age = t - tl
        if age > 7.0 or tl < -6:
            continue
        i = int(np.searchsorted(T.LT, tl))
        cyc_prev = tl - T.LT[i - 1] if i > 0 else 17.1
        r_g = min(0.62, 0.16 + 0.28 * cyc_prev) * (1 + 0.12 * age)
        pos = S.CORE + S.JET_AX * (1.0 + 2.2 * T.jets_k(tl) + r_g) + np.array([0.12 * age, 0.42 * age ** 0.85, -0.05 * age])
        ghosts.append((pos, r_g, math.exp(-age / 3.0)))
    # pressure-wave crests (3D ridges for the well spaced ones)
    rings = T.rings(t)
    for r, amp, age, g, solid in rings:
        if not solid or r < T.R0 * 0.95:
            continue
        fade = 1 - T.ss(2.9, 3.45, r)
        flash = math.exp(-age / 0.22)
        h = 0.085 * amp * fade * (1 + 0.8 * flash) * min(1.0, (r - T.R0 + 0.12) / 0.3)
        if h < 0.012:
            continue
        for Q, n in S.ridge(r, h, 0.085):
            F.append((Q, n, CREST * (0.85 + 0.15 * amp), 0.3 + 0.9 * flash, 'crest'))
    # band edge ridges (one colour band per octave once the note appears)
    for bi in range(5):
        bk = T.band_k(bi - 4, t)
        if bk < 0.05:
            continue
        rr = BAND_EDGES[bi + 1]
        for Q, n in S.ridge(rr, 0.05 * bk, 0.05, n=int(10 * rr + 12)):
            F.append((Q, n, np.array(BAND_COLS[bi], float), 0.5 + 0.4 * be, 'bandedge'))
    # member galaxies
    for gi, (kind, r, hgt, phs, rev, size, col) in enumerate(S.GALAXIES):
        th = phs + rev * 2 * math.pi * t / 60.0
        pos = np.array([r * math.cos(th), hgt + 0.14 * math.sin(2 * math.pi * t / 20 + phs), r * math.sin(th)])
        glow_k = 0.22 * (1 + 0.8 * be)
        if kind == 'spiral':
            nrm = np.array([math.sin(phs * 2.3) * 0.5, 1.0, math.cos(phs * 1.7) * 0.5])
            for Q, n, ring_i, arm in S.disc_galaxy(pos, size, nrm, 2 * math.pi * t / 15.0 * (1 if gi % 2 else -1), n=12):
                if ring_i > 0 and not arm:
                    continue
                base = np.array([255, 232, 196.0]) if ring_i == 0 else np.array(col, float) * (0.95 if ring_i == 1 else 0.78)
                F.append((Q, n, base, 1.0, 'gal2'))
        else:
            for Q, n in icosa(pos, size * 0.42, (1.25, 0.85, 1.0), jit=0.1, rng=np.random.default_rng(40 + gi)):
                F.append((Q, n, np.array([255, 240, 214.0]), 1.0, 'gal2'))
            sg = cam.wp(pos)
            zg = max(cam.depth(pos)[0], 0.5)
            G.append((sg[0], sg[1], size * 2.6 * cam.f / zg, col, 0.42 * (1 + 0.6 * be)))
            G.append((sg[0], sg[1], size * 1.2 * cam.f / zg, (255, 236, 210), 0.35 * (1 + 0.6 * be)))
            continue
        sg = cam.wp(pos)
        G.append((sg[0], sg[1], size * 2.0 * cam.f / max(cam.depth(pos)[0], 0.5), col, glow_k * 0.6))
    # satellite: hovers front-right, aperture turned toward the core
    sp = np.array([-2.2, 3.7 + 0.12 * math.sin(2 * math.pi * t / 12), 2.15])
    yaw_s = math.pi - 0.35 + 0.12 * math.sin(2 * math.pi * t / 30)
    parts, light_a, light_b = S.satellite(sp, yaw_s, roll=1.05 + 0.1 * math.sin(2 * math.pi * t / 20), sc=1.0)
    for faces, col, em in parts:
        for Q, n in faces:
            F.append((Q, n, np.array(col, float), em, 'sat'))
    blink = 1.0 if (t * 1.5) % 1.0 < 0.18 else 0.0
    for lp, c in ((light_a, (255, 170, 80)), (light_b, (255, 120, 90))):
        sl = cam.wp(lp)
        G.append((sl[0], sl[1], 0.14 * cam.f / max(cam.depth(lp)[0], 0.5), c, 0.55 * blink + 0.12))
    # radio dishes (track the core with a little hunting)
    for k, (base, s, deg) in enumerate(S.DISHES):
        top_p = base + np.array([0, 0.42 * s / 0.36, 0])
        aim = S.CORE - top_p
        aim /= np.linalg.norm(aim)
        wob = 0.08 * math.sin(2 * math.pi * t / 20 + k * 2)
        aim = aim + np.array([wob, 0, -wob]); aim /= np.linalg.norm(aim)
        uu = np.cross(aim, [0, 1.0, 0]); uu /= np.linalg.norm(uu)
        vv = np.cross(aim, uu)
        ctr = top_p + aim * 0.05
        rim = [ctr + aim * 0.12 * s / 0.36 + s * 0.5 * (math.cos(2 * math.pi * j / 8) * uu + math.sin(2 * math.pi * j / 8) * vv) for j in range(8)]
        for j in range(8):
            Q = np.array([ctr, rim[j], rim[(j + 1) % 8]])
            nn = np.cross(Q[1] - Q[0], Q[2] - Q[0]); nn /= np.linalg.norm(nn) + 1e-9
            if np.dot(nn, aim) < 0:
                nn = -nn
            F.append((Q, nn, np.array([224, 224, 238.0]), 0.0, 'dish2'))
    # floating rocks under the island
    for k, (r, y, phs, dr, sz, seed) in enumerate(S.ROCKS):
        th = phs + dr * 2 * math.pi * t / 60.0
        pos = np.array([r * math.cos(th), y + 0.12 * math.sin(2 * math.pi * t / 15 + k), r * math.sin(th)])
        for Q, n in icosa(pos, sz, (1.2, 0.8, 1.0), jit=0.15, rng=np.random.default_rng(seed)):
            F.append((Q, n, np.array([132, 104, 160.0]), 0.0, 'frock'))
    return F, G, ghosts, rings, pre


# ------------------------------------------------------------------ shading helpers
def core_term(N, C, ck, vis=None):
    v = S.CORE[None] - C
    dist = np.linalg.norm(v, axis=1) + 1e-6
    lam = np.clip((N * v).sum(1) / dist, 0, None)
    k = lam / (1 + (dist / 3.0) ** 2) * ck * 1.55
    if vis is not None:
        k = k * vis
    return k[:, None] * CORE_COL[None]


def make_lights(t, cam):
    L = Lights(amb=(0.36, 0.32, 0.5), sky=(0.1, 0.08, 0.16))
    L.view = cam.view
    L.key('star', S.STAR_D, STAR_COL)
    L.fill(dirv(170, 20), (0.1, 0.09, 0.16))
    flick = 0.92 + 0.08 * math.sin(t * 11.3) * math.sin(t * 6.1)
    L.point(S.LAMP, LAMP_COL * flick, 1.2)
    return L, flick


# ------------------------------------------------------------------ the frame
def render(f):
    t = f / FPS
    cam = interp_keys(KEYS, t)
    L, flick = make_lights(t, cam)
    ck = core_k(t)
    fr = Frame(backdrop(cam, t))
    arr = fr.array()
    draw_stars(arr, cam, t)
    fr.set_array(arr)
    far_galaxies(fr, cam, t)
    F, G, ghosts, rings, pre = moving_parts(t, cam)

    # ---- island underside + floating rocks (painter, before the floor)
    under = []
    vis = (UF_N * (cam.pos[None] - UF_C)).sum(1) > 0
    if vis.any():
        bounce = np.clip(-UF_N[vis][:, 1:2], 0, None) * np.array([[0.42, 0.3, 0.62]]) + np.clip(UF_N[vis] @ np.array([0.0, 0.0, 1.0]), 0, None)[:, None] * np.array([[0.2, 0.16, 0.3]])
        cols = shade(L, UF_N[vis], UF_C[vis], UF_B[vis], kvis=np.ones((vis.sum(), 1)),
                     emit=core_term(UF_N[vis], UF_C[vis], ck) * 0.4 + bounce)
        for Q, c, z in zip([UF_Q[i] for i in np.nonzero(vis)[0]], cols, cam.depth(UF_C[vis])):
            under.append((z, Q, c))
    for Q, n, base, em, kind in F:
        if kind == 'frock':
            C = Q.mean(0)
            if np.dot(n, cam.pos - C) <= 0:
                continue
            c = shade(L, n[None], C[None], base, kvis=np.ones((1, 1)), emit=core_term(n[None], C[None], ck) * 0.5)[0]
            under.append((cam.depth(C)[0], Q, c))
    for z, Q, c in sorted(under, key=lambda a: -a[0]):
        fr.poly(cam, Q, c)

    # ---- rim floor: per-pixel starlight, core light, lamp, each with real shadows, + contact AO
    alb = Image.new('RGB', (W, H)); msk = Image.new('L', (W, H))
    ad, md = ImageDraw.Draw(alb), ImageDraw.Draw(msk)
    polys = [(S.TOP_POLY, (140, 128, 170))] + S.REGO
    for Q, col in polys:
        Pc = cam.to_cam(Q)
        if (Pc[:, 2] > cam.near).all():
            pts = [tuple(p) for p in cam.proj(Pc)]
            ad.polygon(pts, fill=tuple(int(c) for c in col)); md.polygon(pts, fill=255)
    for cx, cz, cr in S.CRATERS:
        ring = np.array([[cx + cr * math.cos(a), 0.003, cz + cr * math.sin(a)] for a in np.linspace(0, 6.283, 13)[:-1]])
        Pc = cam.to_cam(ring)
        if (Pc[:, 2] > cam.near).all():
            pts = [tuple(p) for p in cam.proj(Pc)]
            ad.polygon(pts, fill=(104, 92, 136))
            inner = np.array([[cx + 0.7 * cr * math.cos(a) + 0.04, 0.004, cz + 0.7 * cr * math.sin(a) + 0.03] for a in np.linspace(0, 6.283, 11)[:-1]])
            ad.polygon([tuple(p) for p in cam.wp(inner)], fill=(86, 76, 120))
    arr, depth = fr.array(), fr.depth()
    ys, xs = np.nonzero(np.asarray(msk))
    if len(xs):
        X, Z, d = cam.ground_hit(xs.astype(float), ys.astype(float))
        rr = np.hypot(X, Z)
        keep = rr >= S.R_POOL - 0.02
        xs, ys, X, Z, d, rr = xs[keep], ys[keep], X[keep], Z[keep], d[keep], rr[keep]
        # dynamic casters onto the floor: galaxies / satellite under starlight
        dyn = [Q for Q, n, b, e, kind in F if kind in ('sat', 'gal', 'gal2')]
        star_m = np.maximum(FLOOR.sample(STAR_MASK0, X, Z), FLOOR.sample(directional_mask(FLOOR, dyn, S.STAR_D, blur=1.4), X, Z) * 0.8) if dyn else FLOOR.sample(STAR_MASK0, X, Z)
        I = np.tile(L.amb + L.sky, (len(X), 1))
        I += ((1 - star_m) * S.STAR_D[1])[:, None] * STAR_COL[None]
        dx, dy, dz = -X, S.CORE[1], -Z
        dist = np.sqrt(dx * dx + dy * dy + dz * dz)
        I += (dy / dist / (1 + (dist / 3.0) ** 2) * 1.55 * ck * (1 - FLOOR.sample(CORE_MASK0, X, Z)))[:, None] * CORE_COL[None]
        lx, ly, lz = S.LAMP
        dl = np.sqrt((lx - X) ** 2 + ly ** 2 + (lz - Z) ** 2)
        I += (ly / dl * (1 - FLOOR.sample(LAMP_MASK0, X, Z)) * 1.25 * flick / (1 + (dl / 1.1) ** 2))[:, None] * LAMP_COL[None]
        I *= contact_ao(X, Z, S.FEET)[:, None]
        I *= (1 - 0.18 * np.exp(-np.maximum(rr - S.R_LIP, 0) / 0.25))[:, None]           # contact darkening at the lip
        col = np.asarray(alb)[ys, xs] * I
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = d

    # ---- the gas basin: haze + pressure-wave crests (+ their shadows) + octave colour bands
    pool = np.array([[S.R_POOL * math.cos(a), S.POOL_Y, S.R_POOL * math.sin(a)] for a in np.linspace(0, 6.283, 49)[:-1]])
    pm = Image.new('L', (W, H))
    ImageDraw.Draw(pm).polygon([tuple(p) for p in cam.wp(pool)], fill=255)
    ys, xs = np.nonzero(np.asarray(pm))
    if len(xs):
        X, Z, d = cam.ground_hit(xs.astype(float), ys.astype(float), y0=S.POOL_Y)
        r = np.hypot(X, Z)
        keep = r <= S.R_POOL
        xs, ys, X, Z, d, r = xs[keep], ys[keep], X[keep], Z[keep], d[keep], r[keep]
        ang = 2 * math.pi * t / 60.0
        ca, sa = math.cos(ang), math.sin(ang)
        u = ((X * ca - Z * sa) / 7.0 + 0.5) % 1.0
        v = ((X * sa + Z * ca) / 7.0 + 0.5) % 1.0
        nz = HAZE_N[(v * 255).astype(np.int32), (u * 255).astype(np.int32)]
        hk = T.haze_k(t)
        rq = r + 0.22 * nz
        rq = np.floor(rq / 0.16 + BAYER_FULL[ys, xs] * 0.999) * 0.16                    # dithered colour banding
        base = haze_col(rq)
        Hl = S.CORE[1] - S.POOL_Y
        dist = np.sqrt(r * r + Hl * Hl)
        hot = Hl / dist / (1 + (dist / 2.1) ** 2) * ck
        # galaxy shadows from the core light onto the gas
        low = [Q for Q, n, b, e, kind in F if kind in ('gal', 'gal2') and Q[:, 1].max() < S.CORE[1] - 0.2]
        if low:
            gm = point_mask(FLOOR, [Q - [0, S.POOL_Y, 0] for Q in low], S.CORE - [0, S.POOL_Y, 0], blur=1.5)
            hot = hot * (1 - 0.6 * FLOOR.sample(gm, X, Z))
        bright = hk * (0.66 + 0.12 * nz) + 0.3 * hot
        col = base * bright[:, None] + (hot * 34)[:, None] * np.array([1.0, 0.8, 0.55])
        # crests
        crest = np.zeros_like(r)
        shad = np.zeros_like(r)
        for rc, amp, age, g, solid in rings:
            fade = (1 - T.ss(2.9, 3.45, rc)) * min(1.0, max(0.0, (rc - T.R0 + 0.1) / 0.25))
            if fade <= 0:
                continue
            flash = math.exp(-age / 0.22)
            w = 0.075 if g != 'F' else 0.06
            crest += amp * fade * (0.55 + 0.9 * flash) * np.exp(-((r - rc) / w) ** 2)
            if solid:
                dlt = 0.085 + 0.1 * rc
                shad += amp * fade * np.exp(-((r - rc - dlt) / (0.06 + 0.03 * rc)) ** 2)
        sk = T.shimmer_k(t)
        if sk > 0:
            crest += sk * 0.25 * (0.5 + 0.5 * np.sin(r * 26 - t * 70 + nz * 3))
        col = col * (1 - 0.38 * np.clip(shad, 0, 1))[:, None] + np.clip(crest, 0, 1.6)[:, None] * CREST[None] * 0.62
        # octave colour bands (B-flat0 innermost ... B-flat4 outermost)
        be = T.beat_env(t)
        for bi in range(5):
            bk = T.band_k(bi - 4, t)
            if bk <= 0:
                continue
            r0, r1 = BAND_EDGES[bi], BAND_EDGES[bi + 1]
            inb = (r >= r0) & (r < r1)
            if not inb.any():
                continue
            q = 1.6 * 2 ** bi
            rip = 0.5 + 0.5 * np.cos(2 * math.pi * (r[inb] * q - t * (0.9 + 0.35 * bi)))
            rip = np.floor(rip * 4 + BAYER_FULL[ys[inb], xs[inb]] * 0.999) / 4
            k = bk * (0.55 + 0.45 * rip) * (1 + 0.45 * be)
            bc = np.array(BAND_COLS[bi], float)
            col[inb] = col[inb] * (1 - 0.72 * bk) + bc[None] * k[:, None] * (0.75 + 0.35 * hk)
        arr[ys, xs] = np.floor(np.clip(col, 0, 255) / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = d
    arr = apply_glows(arr, pre)
    fr.set_array(arr, depth)

    # ---- painter: lip, props, black hole, jets, bubbles, crests, galaxies, satellite
    items = []
    vis = (SF_N * (cam.pos[None] - SF_C)).sum(1) > 0
    if vis.any():
        idx = np.nonzero(vis)[0]
        emit = core_term(SF_N[idx], SF_C[idx], ck, SF_COREV[idx]) + SF_E[idx, None]
        emit[SF_HOT[idx]] += 0.9 * flick
        cols = shade(L, SF_N[idx], SF_C[idx], SF_B[idx], kvis=SF_STARV[idx, None], emit=emit)
        zs = cam.depth(SF_C[idx])
        for i, c, z in zip(idx, cols, zs):
            items.append((z, SF_Q[i], c))
    dyn = [(Q, n, b, e, k) for Q, n, b, e, k in F if k != 'frock']
    if dyn:
        C = np.array([q.mean(0) for q, *_ in dyn])
        N = np.array([n for _, n, *_ in dyn])
        B = np.array([b for _, _, b, _, _ in dyn])
        E = np.array([e for *_, e, _ in dyn])
        kinds = [k for *_, k in dyn]
        toward = (N * (cam.pos[None] - C)).sum(1)
        two_sided = np.array([k in ('disc', 'gal2', 'dish2', 'sat') for k in kinds])
        N = np.where((toward < 0)[:, None] & two_sided[:, None], -N, N)
        keep = (toward > 0) | two_sided
        lit = shade(L, N, C, B, kvis=np.ones((len(N), 1)), emit=core_term(N, C, ck) + E[:, None] * 0.6)
        glow = np.clip(B * (0.55 + 0.5 * E[:, None]) * np.array([1.0, 1.0, 1.0]), 0, 255)
        emissive = np.array([k in ('disc', 'jet') for k in kinds])
        cols = np.where(emissive[:, None], glow * 1.15, lit)
        gal2 = np.array([k == 'gal2' for k in kinds])
        cols[gal2] = B[gal2] * (1 + 0.35 * T.beat_env(t))
        bub = np.array([k == 'bubble' for k in kinds])
        if bub.any():
            cols[bub] = shade(L, N[bub], C[bub], B[bub], kvis=np.ones((bub.sum(), 1)), rimk=1.4,
                              emit=core_term(N[bub], C[bub], ck) * 0.7 + 0.1)
        bh = np.array([k == 'bh' for k in kinds])
        cols[bh] = np.clip(B[bh] + shade(L, N[bh], C[bh], (90, 70, 120), kvis=np.ones((bh.sum(), 1)), rimk=1.2) * 0.25, 0, 255) if bh.any() else cols[bh]
        zs = cam.depth(C)
        for (Q, *_), c, z, kp in zip(dyn, cols, zs, keep):
            if kp:
                items.append((z, Q, c))
    for z, Q, c in sorted(items, key=lambda a: -a[0]):
        fr.poly(cam, Q, c)
    # ghost bubbles: faint rising outlines
    for pos, rg, a in ghosts:
        sp = cam.wp(pos)
        rad = rg * cam.f / max(cam.depth(pos)[0], 0.5)
        pts = [(sp[0] + rad * math.cos(2 * math.pi * k / 14), sp[1] + rad * math.sin(2 * math.pi * k / 14)) for k in range(15)]
        fr.draw.line(pts, fill=tuple(int(c) for c in mixc((90, 60, 150), (220, 200, 255), a)), width=1)
        G.append((sp[0], sp[1], rad * 1.3, (170, 140, 255), 0.12 * a))

    img, depth = fr.array(), fr.depth()
    motes_and_comets(img, depth, cam, t, G)
    # ---- lens (focus locked on the black hole), glows, shafts, bloom, grade
    zf = cam.depth(S.SUBJECT)[0]
    K = 0.022 * cam.f * min(1.2, 8.0 / zf)
    sharp = 3.2 / zf
    coc = coc_map(depth, zf, K, sharp=sharp)
    img = dof(img, coc)
    # window / lamp / crystal glows
    for p, c, k in [(S.LAMP, (255, 180, 90), 0.5 * flick)] + [(cc + [0, 0.05, 0], col, 0.12) for cc, col in S.CRYSTALS]:
        sp = cam.wp(p)
        G.append((sp[0], sp[1], 0.42 * cam.f / max(cam.depth(p)[0], 0.5), c, k))
    img = apply_glows(img, G)
    sc = cam.wp(S.CORE)
    img = god_rays(img, depth, sc, (190, 160, 255), 0.13 * (0.6 + 0.4 * T.jets_k(t)))
    img = grade(bloom(np.clip(img, 0, 255), thr=196, k=0.42), contrast=0.16, sat=1.12)
    img = np.clip(img * vignette(0.7, 0.62, 0.35) * top_scrim(), 0, 255).astype(np.uint8)

    # ---- UI
    out = img.copy()
    hud(out, t)
    draw_captions(out, t, T.CAPTIONS)
    return out


COMETS = [(20.4, 22.2, (40, 236), (330, 190)), (46.0, 47.7, (320, 214), (20, 168))]


def motes_and_comets(img, depth, cam, t, G):
    """small moving life: dust motes drifting in the core's light, the odd comet across the sky"""
    for r, y, phs, rev, ph2 in S.MOTES:
        th = phs + rev * 2 * math.pi * t / 60.0
        p = np.array([r * math.cos(th), y + 0.25 * math.sin(2 * math.pi * t / 20 + ph2), r * math.sin(th)])
        z = cam.depth(p)[0]
        if z < 0.5:
            continue
        x, yy = cam.wp(p)
        xi, yi = int(x), int(yy)
        if 0 <= xi < W and 0 <= yi < H and depth[yi, xi] > z:
            tw = 0.6 + 0.4 * math.sin(2 * math.pi * 3 * t / 60 * 5 + ph2 * 3)
            c = np.array([255, 220, 180.0]) if rev > 0 else np.array([180, 230, 255.0])
            img[yi, xi] = np.maximum(img[yi, xi], c * tw)
    for t0, t1, a, b in COMETS:
        if t0 <= t <= t1:
            u = (t - t0) / (t1 - t0)
            hx, hy = a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u
            dx, dy = (b[0] - a[0]), (b[1] - a[1])
            ln = math.hypot(dx, dy)
            for k in range(26):
                tx, ty = hx - dx / ln * k * 2.2, hy - dy / ln * k * 2.2
                xi, yi = int(tx), int(ty)
                if 0 <= xi < W and 0 <= yi < H and depth[yi, xi] > 1e5:
                    img[yi, xi] = np.maximum(img[yi, xi], np.array([220, 240, 255.0]) * (1 - k / 26) * math.sin(math.pi * u) ** 0.5)
            G.append((hx, hy, 5, (200, 230, 255), 0.5 * math.sin(math.pi * u)))


METER_X0, METER_CELL = 32, 5


def hud(img, t):
    if t < T.T_PANEL_IN or t > T.T_PANEL_OUT + 0.4:
        return
    off = slide_offset(t, T.T_PANEL_IN, T.T_PANEL_OUT, dist=130)
    y0 = 516 + off
    k = T.octave(t)
    blink = math.exp(-(t - T.last_step(t)) / 0.12)
    kick = T.kick_env(t, 0.09)
    be = T.beat_env(t)
    pil = Image.fromarray(img)
    d = ImageDraw.Draw(pil)
    edge = tuple(int(c) for c in mixc((70, 84, 140), (200, 240, 255), max(kick, be)))
    panel(d, 14, y0, 346, y0 + 100, fill=(18, 16, 44), edge=edge)
    img[:] = np.asarray(pil)
    label(img, "OCTAVE", 26, y0 + 9, SKYB, 2)
    num = f"{k}" if k < 0 else "0"
    label(img, num, 26, y0 + 30, tuple(int(c) for c in mixc(MINT, (255, 255, 255), blink)), 5)
    rd = T.readout(k)
    if k >= -4:
        label(img, rd, 146, y0 + 10, CREAM, 2)
        label(img, T.NOTE_NAME[k], 146, y0 + 30, AMBER, 4)
        label(img, "ALL OCTAVES" if k == 0 else "A NOTE", 236, y0 + 44, MINT, 1)
    elif k >= -9:
        label(img, "1 WAVE = 1 BEAT" if k <= -7 else "TOO FAST TO COUNT", 146, y0 + 10, SKYB, 1)
        label(img, rd, 146, y0 + 22, AMBER, 3)
    else:
        label(img, "1 WAVE EVERY", 146, y0 + 10, SKYB, 1)
        label(img, rd, 146, y0 + 22, CREAM, 3 if len(rd) <= 10 else 2)
        label(img, "EACH STEP DOUBLES IT" if t > 16.9 else "PERSEUS CLUSTER", 146, y0 + 52, (170, 170, 220), 1)
    # 57-cell meter: one cell per octave climbed
    lit = k + 57
    for c in range(57):
        x = METER_X0 + c * METER_CELL
        u = c / 56
        if c < lit:
            col = mixc((150, 96, 230), (110, 240, 255), min(1, u * 1.6)) if u < 0.85 else mixc((110, 240, 255), (255, 200, 90), (u - 0.85) / 0.15)
            if c == lit - 1:
                col = mixc(col, (255, 255, 255), blink)
            col = mixc(col, (255, 255, 255), 0.35 * be)
        else:
            col = (44, 40, 84)
        img[y0 + 80:y0 + 90, x:x + METER_CELL - 1] = np.array(col, float).astype(np.uint8)
    label(img, "-57", METER_X0 - 2, y0 + 70, (150, 140, 200), 1)
    label(img, "0", METER_X0 + 56 * METER_CELL - 2, y0 + 70, (150, 140, 200), 1)
