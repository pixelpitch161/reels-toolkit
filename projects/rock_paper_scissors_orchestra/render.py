"""Rock Paper Scissors Orchestra renderer.

dark house -> proscenium, chandelier, curtains, valance, boxes + patrons (painter order) -> per-pixel stage boards
(chandelier key light with real token shadows, footlight wash, contact AO, painted ring) -> back rim -> the 120 tokens
(flat-shaded, facing their direction of travel, a punch + glow on every catch) -> front rim, footlights, apron, pit,
stands, conductor -> dust in the light -> locked DOF on the stage -> glows, bloom, grade -> share bar, rule legend,
impact captions.

    python -m reelkit.sheet render.py test/sheet.png 0.3 1.6 5 14 24 32 38 44 50 54 59 60.9
    python -m reelkit.video render.py audio.wav rock_paper_scissors_orchestra.mp4"""
import math
import os
import sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, dirv, nrm
from reelkit.camera import Cam
from reelkit.raster import Frame, add_glow
from reelkit.lighting import Lights, shade
from reelkit.shadows import MaskGrid, point_mask
from reelkit.lens import coc_map, dof
from reelkit.post import bloom, grade, vignette, top_scrim
from reelkit.geometry import Obj
from reelkit import ui
import timeline as T
import scene as S
import tokens as TK

DUR = T.DUR
TAU = 2 * math.pi

# ================================================================== camera
FOV, DIST, PITCH, SY = 40.0, 26.0, 46.0, 292.0
TARGET = np.array([0.0, 0.0, 0.35])


def bump(t, a, b, ramp=1.2):
    """0 -> 1 -> 0 between a and b with eased ramps"""
    return T.ss(a, a + ramp, t) * (1 - T.ss(b - ramp, b, t))


def camera(t):
    yaw = 2.4 * math.sin(TAU * t / DUR + 0.5)
    pitch = PITCH + 0.8 * math.sin(TAU * 2 * t / DUR + 1.3)
    dist = DIST - 0.6 * (1 - T.ss(0.0, 3.5, t)) * 0
    dist -= 1.0 * bump(t, T.T1 - 0.5, T.T1 + 7.5, 1.6)          # lean in for the twist
    dist -= 1.4 * bump(t, T.T_WIN - 0.3, 59.8, 1.8)             # and for the winner
    return Cam(TARGET, dist, yaw, pitch, FOV, SY)


# ================================================================== lights
CH_LIGHT = S.CHAND + np.array([0, -0.3, 0])
WARM = np.array([1.15, 0.74, 0.38])
CH_COL = np.array([1.05, 0.9, 0.7])
FOOT_COL = np.array([1.2, 0.66, 0.3])


def flick(k, t):
    return 0.9 + 0.1 * math.sin(TAU * (37 + 3 * k) * t / DUR + 1.7 * k) * math.sin(TAU * (23 + k) * t / DUR + 0.6 + k)


def chand_k(t):
    return 1.0 + 0.28 * T.blaze(t) + 0.1 * T.blaze(t) * T.beat_env(t, 0.2)


def foot_k(t):
    return 1.0 + 0.35 * T.blaze(t) * T.beat_env(t, 0.18)


def lights(t, cam):
    L = Lights(amb=(0.24, 0.2, 0.3), sky=(0.06, 0.05, 0.08))
    L.view = cam.view
    L.key('chandelier', nrm(CH_LIGHT - [0, 0.2, 0]), CH_COL * 0.62 * chand_k(t))
    L.fill(dirv(0, 30), (0.2, 0.2, 0.32))                     # cool house fill from the auditorium
    L.point(np.array([0.0, 0.4, 3.9]), FOOT_COL * 0.7 * foot_k(t), 2.6)
    L.point(np.array([-2.6, 0.4, 2.8]), FOOT_COL * 0.45 * foot_k(t), 2.0)
    L.point(np.array([2.6, 0.4, 2.8]), FOOT_COL * 0.45 * foot_k(t), 2.0)
    return L


def back_lights(t, cam):
    """curtains + valance: lit from the stage (its glow and the footlights) and the chandelier above"""
    L = Lights(amb=(0.34, 0.25, 0.32), sky=(0.06, 0.05, 0.07))
    L.view = cam.view
    L.key('chandelier', nrm(CH_LIGHT - [0, 2.0, 0]), CH_COL * 0.45 * chand_k(t))
    L.point(np.array([0.0, 1.4, -0.6]), np.array([1.0, 0.72, 0.48]) * 0.95 * chand_k(t), 5.0)
    L.point(np.array([0.0, 0.3, 3.6]), FOOT_COL * 0.5 * foot_k(t), 6.0)
    return L


def box_lights(t, cam):
    """lights for the boxes and patrons: the sconces"""
    L = Lights(amb=(0.36, 0.28, 0.36), sky=(0.06, 0.05, 0.07))
    L.view = cam.view
    L.key('chandelier', nrm(CH_LIGHT - [0, 2.0, 0]), CH_COL * 0.4 * chand_k(t))
    L.fill(dirv(0, 30), (0.2, 0.2, 0.3))
    L.point(np.array([0.0, 1.4, 0.0]), np.array([1.0, 0.72, 0.48]) * 0.8 * chand_k(t), 6.5)
    for k, s in enumerate(S.SCONCES):
        L.point(s, WARM * 0.75 * flick(k, t), 0.9)
    return L


# ================================================================== the stage floor
SHADOW_GRID = MaskGrid((-3.8, 3.8, -3.8, 3.8, 0.02))
PLANK = 0.27


def _floor(fr, cam, L, t, token_faces, feet):
    arr, depth = fr.array(), fr.depth()
    ext = np.array([[S.R_BOARD * math.cos(a), 0, S.R_BOARD * math.sin(a)] for a in np.linspace(0, TAU, 25)])
    sp = cam.wp(ext)
    x0, x1 = int(max(0, sp[:, 0].min() - 1)), int(min(W, sp[:, 0].max() + 2))
    y0, y1 = int(max(0, sp[:, 1].min() - 1)), int(min(H, sp[:, 1].max() + 2))
    ys, xs = np.mgrid[y0:y1, x0:x1]
    ys, xs = ys.ravel(), xs.ravel()
    X, Z, d = cam.ground_hit(xs.astype(float), ys.astype(float))
    r = np.hypot(X, Z)
    ok = (r <= S.R_BOARD) & (d > 0) & (d < depth[ys, xs])
    xs, ys, X, Z, d, r = xs[ok], ys[ok], X[ok], Z[ok], d[ok], r[ok]
    # boards: planks running left-right, staggered butt joints, a painted ring marking the arena
    row = np.floor(Z / PLANK)
    u = (Z / PLANK) - row
    rh = (np.sin(row * 12.9898) * 43758.5453) % 1.0
    seg = np.floor(X / 1.6 + rh * 1.6)
    sh = (np.sin(row * 7.1 + seg * 3.3) * 9631.7) % 1.0
    tone = 0.88 + 0.16 * sh + 0.05 * np.sin(X * 9.0 + row * 2.1)
    alb = np.array([176, 116, 72.0])[None] * tone[:, None]
    seam = (u < 0.07) | (((X / 1.6 + rh * 1.6) % 1.0) < 0.012)
    alb[seam] *= 0.55
    ring = np.abs(r - (S.R_ARENA - 0.02)) < 0.035
    alb[ring] = alb[ring] * 0.4 + np.array([236, 200, 120.0]) * 0.6
    # chandelier light with real shadows of the tokens
    smask = point_mask(SHADOW_GRID, token_faces, CH_LIGHT, blur=1.1, min_y=0.0)
    lp = CH_LIGHT
    dist = np.sqrt((lp[0] - X) ** 2 + lp[1] ** 2 + (lp[2] - Z) ** 2)
    lam = lp[1] / dist
    I = np.tile(L.amb + L.sky, (len(X), 1))
    fk = 1.0 + 0.1 * T.blaze(t)
    I += (lam * (1 - 0.82 * SHADOW_GRID.sample(smask, X, Z)) * 1.18 * fk / (1 + (dist / 7.5) ** 2))[:, None] * CH_COL[None]
    # pool of light (the chandelier cone) brighter toward the centre
    I += (0.18 * np.exp(-(r / 2.6) ** 2) * fk)[:, None] * CH_COL[None]
    # footlight wash from the front rim
    for k, fp in enumerate(S.FOOT):
        dd = np.sqrt((fp[0] - X) ** 2 + 0.12 + (fp[2] - Z) ** 2)
        I += (0.16 * flick(k, t) * foot_k(t) / (1 + (dd / 0.9) ** 2))[:, None] * FOOT_COL[None]
    I += (L.fills[0][1] * 0.45)[None]
    # contact AO under every token
    ao = np.ones(len(X), np.float32)
    for fx, fz in feet:
        dd = (X - fx) ** 2 + (Z - fz) ** 2
        ao *= 1 - 0.45 * np.exp(-dd / (0.17 ** 2))
    I *= ao[:, None]
    col = alb * I
    arr[ys, xs] = np.floor(col / 5 + BAYER_FULL[ys, xs, None]) * 5
    depth[ys, xs] = d
    fr.set_array(arr, depth)


# ================================================================== objects
def _draw_obj(fr, cam, L, o, emit=0.0, base=None, faces=None, fog_k=0.0):
    faces = o.faces if faces is None else faces
    fc = [(cam.depth(Q.mean(0))[0], Q, n, j) for j, (Q, n) in enumerate(faces) if np.dot(n, cam.pos - Q.mean(0)) > 0]
    if not fc:
        return
    fc.sort(key=lambda a: -a[0])
    C = np.array([q.mean(0) for _, q, _, _ in fc])
    Nn = np.array([n for _, _, n, _ in fc])
    bc = np.array([(o.color if base is None else base) * o.tone[j % len(o.tone)] for *_, j in fc])
    cols = shade(L, Nn, C, bc, emit=emit)
    if fog_k:
        cols = cols * (1 - fog_k) + np.array([40, 26, 44.0]) * fog_k
    for (z, Q, n, j), c in zip(fc, cols):
        fr.poly(cam, Q, c)


BACK_RIM = Obj([f for f in S.RIM.faces if f[0].mean(0)[2] < 0.0], S.GOLD, 'rimb', cast=False, jitter=0.03)
FRONT_RIM = Obj([f for f in S.RIM.faces if f[0].mean(0)[2] >= 0.0], S.GOLD, 'rimf', cast=False, jitter=0.03)
PATRON_FACES = [(S.pawn(b, 0.9), ph) for b, ph, g in S.PATRONS]
BACK_OBJS = S.ARCH + [S.BACKFLOOR, S.CURTAINS, S.VALANCE, S.FRINGE_O] + S.TASSELS
PIT_FLOOR = Obj([(S.PIT_FLOOR_Q, np.array([0, 1.0, 0]))], (56, 30, 34), 'pitfloor', cast=False)
CARPET = Obj([(S.CARPET_Q, np.array([0, 1.0, 0]))], (96, 34, 46), 'carpet', cast=False)
STALL_PAWNS = [(S.pawn(b, 0.75), ph) for b, ph in S.STALL_PATRONS]


def _house_bg(cam):
    """the dark auditorium beyond: a deep plum gradient, banded and dithered"""
    k = np.clip(YY / H, 0, 1)
    top = np.array([34, 18, 38.0])
    mid = np.array([62, 30, 54.0])
    col = top[None, None] * (1 - k[..., None]) + mid[None, None] * k[..., None]
    return np.floor(col / 6 + BAYER_FULL[..., None]) * 6


def render(f):
    t = f / FPS
    cam = camera(t)
    L = lights(t, cam)
    LB = box_lights(t, cam)
    LK = back_lights(t, cam)
    fr = Frame(_house_bg(cam))

    # ---- back: proscenium, chandelier chain, curtains, backstage floor, boxes, patrons
    for o in S.ARCH:
        _draw_obj(fr, cam, LB, o, fog_k=0.25)
    a, b = cam.wp(S.CHAIN[0]), cam.wp(S.CHAIN[1])
    fr.draw.line([tuple(a), tuple(b)], fill=(70, 54, 40), width=2)
    for o in sorted(S.BOXES, key=lambda o: -float(cam.depth(o.ctr)[0])):
        _draw_obj(fr, cam, LB, o, emit=0.0)
        if o.name == 'boxfloor':
            for (faces, ph), (base, _, g) in zip(PATRON_FACES, S.PATRONS):
                if g == o.grp:
                    bob = 0.025 * math.sin(TAU * 7 * t / DUR + ph * 3)
                    pf = [(Q + [0, bob, 0], n) for Q, n in faces]
                    _draw_obj(fr, cam, LB, Obj(pf, (52, 40, 46), 'pawn', cast=False), faces=pf)
    for o in [S.BACKFLOOR, S.CURTAINS, S.VALANCE, S.FRINGE_O] + S.TASSELS:
        _draw_obj(fr, cam, LK, o, emit=0.08 if o.name == 'fringe' else 0.0)
    _draw_chandelier(fr, cam, L, t)

    # ---- tokens (positions, headings, teams for this frame)
    fi = T.frame_of(t)
    pos = T.POS[fi]
    head = T.HEAD[fi]
    team = T.TEAM[fi]
    scale = np.ones(T.NPC)
    pop = np.zeros(T.NPC)
    for ci in T.recent_catches(t, 0.3):
        j = T.C_CAUGHT[ci]
        u = (t - T.CT[ci]) / 0.3
        scale[j] = max(scale[j], 1.0 + 0.45 * (1 - u) ** 2)
        pop[j] = max(pop[j], 1 - u)
    hook = np.zeros(T.NPC)
    if t < 3.0:
        for q, tl in enumerate(T.HOOK_LAND):
            if tl <= t < tl + 0.45:
                hook[team == q] = 1 - (t - tl) / 0.45
    lift = np.zeros(T.NPC)
    if T.T_WIN + 0.2 < t < T.T_G0 + 0.4:
        amp = 0.16 * T.ss(T.T_WIN + 0.2, T.T_WIN + 1.0, t) * (1 - T.ss(T.T_G0 - 0.6, T.T_G0 + 0.3, t))
        ph = TAU * t / T.BAR - np.arctan2(pos[:, 1], pos[:, 0])            # a wave round the ring, once per bar
        lift = amp * np.clip((np.sin(ph) - 0.55) / 0.45, 0, 1) ** 1.5
    faces_all, token_faces = [], []
    for i in range(T.NPC):
        Qs, Ns, Cs = TK.place(team[i], pos[i, 0], pos[i, 1], head[i], scale[i] * 1.12, lift[i])
        token_faces += Qs
        em = 0.35 * hook[i] + 0.25 * pop[i]
        for Q, n, c in zip(Qs, Ns, Cs):
            faces_all.append((Q, n, c, em))
    feet = [(pos[i, 0], pos[i, 1]) for i in range(T.NPC)]

    # ---- the stage floor (per pixel), back rim
    _floor(fr, cam, L, t, token_faces, feet)
    _draw_obj(fr, cam, L, BACK_RIM, emit=0.05)

    # ---- tokens: back-face cull, shade in one go, painter order
    vis = [(Q, n, c, em) for Q, n, c, em in faces_all if np.dot(n, cam.pos - Q.mean(0)) > 0]
    if vis:
        C = np.array([v[0].mean(0) for v in vis])
        Nn = np.array([v[1] for v in vis])
        base = np.array([v[2] for v in vis])
        em = np.array([v[3] for v in vis])
        cols = shade(L, Nn, C, base, rimk=0.7)
        cols = np.clip(cols + em[:, None] * base * 0.8, 0, 255)
        z = cam.depth(C)
        for k in np.argsort(-z):
            fr.poly(cam, vis[k][0], cols[k])
    for ci in T.recent_catches(t, 0.3):
        j = T.C_CAUGHT[ci]
        u = (t - T.CT[ci]) / 0.3
        s = cam.wp([pos[j, 0], 0.12, pos[j, 1]])
        fr.glow(s[0], s[1], 5 + 6 * u, T.GLOW[T.C_NEW[ci]], 0.55 * (1 - u))

    # ---- front: rim, footlights, apron, pit, stands, conductor
    _draw_obj(fr, cam, L, FRONT_RIM, emit=0.05)
    for o in S.FOOT_SHELLS:
        _draw_obj(fr, cam, L, o)
    _draw_obj(fr, cam, L, S.APRON)
    _draw_obj(fr, cam, L, S.APRON_TRIM, emit=0.08)
    _draw_obj(fr, cam, L, PIT_FLOOR)
    for o in S.STANDS:
        _draw_obj(fr, cam, LB, o, emit=0.15 if o.name == 'sheet' else 0.0)
    _draw_obj(fr, cam, LB, S.PODIUM)
    _conductor(fr, cam, t)
    _draw_obj(fr, cam, L, S.PIT_WALL)
    _draw_obj(fr, cam, L, S.PIT_RAIL, emit=0.08)
    _draw_obj(fr, cam, LB, CARPET)
    seats = sorted(S.SEATS, key=lambda o: -float(cam.depth(o.ctr)[0]))
    for o in seats:
        _draw_obj(fr, cam, LB, o)
    for k, (faces, ph) in enumerate(STALL_PAWNS):
        lean = 0.03 * math.sin(TAU * (3 + k) * t / DUR + k)
        pf = [(Q + [lean, 0, 0], n) for Q, n in faces]
        _draw_obj(fr, cam, LB, Obj(pf, (46, 36, 44), 'pawn', cast=False), faces=pf)
    _lamps(fr, cam, t)
    _dust(fr, cam, t)

    # ---- lens: focus locked on the stage
    img, depth = fr.array(), fr.depth()
    zf = cam.depth(S.SUBJECT)[0]
    img = dof(img, coc_map(depth, zf, 5.5, sharp=focus_band(cam)))
    for g in fr.glows:
        add_glow(img, *g)
    img = grade(bloom(np.clip(img, 0, 255), thr=205, k=0.36), contrast=0.12, sat=1.14)
    img = np.clip(img * vignette(strength=0.6, cap=0.28) * top_scrim(), 0, 255).astype(np.uint8)
    out = img.copy()
    _ui(out, t)
    return out


# ------------------------------------------------------------------ focus: the whole stage + tokens sharp
_SUBJ = np.array([[S.R_RIM1 * math.cos(a), y, S.R_RIM1 * math.sin(a)] for a in np.linspace(0, TAU, 32) for y in (0.0, 0.3)])


def focus_band(cam):
    zf = cam.depth(S.SUBJECT)[0]
    z = cam.depth(_SUBJ)
    return max(0.05, float(np.abs(1 - zf / z).max()) + 0.01)


# ------------------------------------------------------------------ chandelier, lamps, conductor, dust
def _draw_chandelier(fr, cam, L, t):
    Lc = Lights(amb=(0.5, 0.42, 0.34), sky=(0.1, 0.08, 0.06))
    Lc.view = cam.view
    Lc.key('k', dirv(0, -60), CH_COL * 0.5)
    for o in S.CH_PARTS:
        _draw_obj(fr, cam, Lc, o, emit=0.25 + 0.3 * T.blaze(t))
    bk = chand_k(t)
    for k, p in enumerate(S.CANDLES):
        s = cam.wp(p)
        fr.glow(s[0], s[1], 4.0, (255, 210, 130), 0.75 * flick(k, t) * bk)
        fr.draw.rectangle([s[0] - 0.5, s[1] - 1.5, s[0] + 0.5, s[1] + 0.5], fill=(255, 244, 210))
    for p, k in S.CRYSTALS:
        s = cam.wp(p)
        tw = 0.5 + 0.5 * math.sin(TAU * (11 + k % 7) * t / DUR + k * 2.3)
        c = (int(200 + 55 * tw), int(220 + 35 * tw), 255)
        fr.draw.polygon([(s[0], s[1] - 2), (s[0] + 1, s[1]), (s[0], s[1] + 2), (s[0] - 1, s[1])], fill=c)
        if tw > 0.82:
            fr.glow(s[0], s[1], 3.0, (190, 220, 255), (tw - 0.82) * 4.0 * bk)
    s = cam.wp(S.CHAND)
    fr.glow(s[0], s[1] + 2, 26, (255, 196, 120), 0.32 * bk)


def _lamps(fr, cam, t):
    fk = foot_k(t)
    for k, p in enumerate(S.FOOT):
        s = cam.wp(p)
        fr.glow(s[0], s[1], 4.5, (255, 186, 96), 0.7 * flick(k, t) * fk)
        fr.draw.rectangle([s[0] - 1, s[1] - 1, s[0] + 1, s[1]], fill=(255, 232, 170))
    for k, p in enumerate(S.SCONCES):
        s = cam.wp(p)
        fr.glow(s[0], s[1], 5.0, (255, 176, 90), 0.6 * flick(k + 3, t))
        fr.draw.rectangle([s[0] - 0.5, s[1] - 1, s[0] + 0.5, s[1] + 1], fill=(255, 236, 180))
    for k, p in enumerate(S.STAND_LAMPS):
        s = cam.wp(p)
        fr.glow(s[0], s[1], 3.5, (255, 210, 130), 0.55)


def _conductor(fr, cam, t):
    """a silhouette on the podium, beating time (a 4-beat pattern), baton catching the light"""
    b = S.COND_BASE
    beat = (t / T.BEAT) % 4
    k = int(beat)
    u = beat - k
    pts = [(0.0, 0.55), (-0.32, 0.38), (0.34, 0.4), (0.0, 0.72)]            # down, left, right, up (x, y) in metres
    pa, pb = np.array(pts[k]), np.array(pts[(k + 1) % 4])
    e = u * u * (3 - 2 * u)
    hand = pa + (pb - pa) * e + np.array([0, -0.08 * math.sin(math.pi * e)])
    body_top = b + [0, 0.62, 0]
    head = b + [0, 0.78, 0]
    sh = cam.wp(head)
    zb = float(cam.depth(b)[0])
    sc = cam.f / zb
    dark = (24, 16, 22)
    torso = [cam.wp(b + [-0.15, 0, 0]), cam.wp(b + [0.15, 0, 0]), cam.wp(body_top + [0.18, 0, 0]), cam.wp(body_top + [-0.18, 0, 0])]
    fr.draw.polygon([tuple(p) for p in torso], fill=dark)
    fr.draw.ellipse([sh[0] - 0.1 * sc, sh[1] - 0.12 * sc, sh[0] + 0.1 * sc, sh[1] + 0.1 * sc], fill=dark)
    shoulder = cam.wp(body_top + [0.16, -0.02, 0])
    hw = cam.wp(b + [hand[0] + 0.16, hand[1], -0.1])
    fr.draw.line([tuple(shoulder), tuple(hw)], fill=dark, width=max(1, int(0.06 * sc)))
    tip = cam.wp(b + [hand[0] + 0.16 + 0.12, hand[1] + 0.3, -0.25])
    fr.draw.line([tuple(hw), tuple(tip)], fill=(236, 226, 200), width=1)
    ls = cam.wp(body_top + [-0.16, -0.02, 0])
    lh = cam.wp(b + [-0.28, 0.42 + 0.04 * math.sin(TAU * t / T.BAR), -0.12])
    fr.draw.line([tuple(ls), tuple(lh)], fill=dark, width=max(1, int(0.06 * sc)))


_dr = np.random.default_rng(8)
DUST = [(_dr.uniform(-2.6, 2.6), _dr.uniform(0.6, 6.0), _dr.uniform(-2.6, 1.8), int(_dr.integers(1, 4)), _dr.uniform(0, 6.3),
         _dr.uniform(0.35, 0.9)) for _ in range(40)]


def _dust(fr, cam, t):
    for x0, y0, z0, kk, ph, b in DUST:
        y = (y0 - 6.0 * kk * t / DUR * 0.25) % 5.6 + 0.4
        x = x0 + 0.25 * math.sin(TAU * kk * t / DUR + ph)
        p = np.array([x, y, z0])
        s = cam.wp(p)
        xi, yi = int(s[0]), int(s[1])
        if 0 <= xi < W and 0 <= yi < H:
            near = math.exp(-((x - CH_LIGHT[0]) ** 2 + (z0 - CH_LIGHT[2]) ** 2) / 9.0)
            fr.glow(s[0], s[1], 1.2, (255, 226, 170), (0.25 + 0.5 * near) * b)


# ------------------------------------------------------------------ UI
LEGEND_Y = None


def _ui(out, t):
    a = T.bar_alpha(t)
    info = None
    if a > 0.01:
        sh = T.shares(t)
        outs = [T.out_now(q, t) for q in range(3)]
        info = ui.share_bar(out, sh, T.NAMES, T.BAR_COL, out=outs, alpha=a)
    for t0, t1, lines, hi, alt, stg, y0 in T.CAPTIONS:
        if t0 <= t < t1:
            old = ui.IMPACT_STAGGER
            ui.IMPACT_STAGGER = stg
            try:
                ui.draw_captions(out, t, [(t0, t1, lines)], y0=y0, hi=hi, alt=alt if alt else ui.SKYB, style='impact',
                                 puff=(236, 214, 170))
            finally:
                ui.IMPACT_STAGGER = old
    if T.LEGEND[0] <= t < T.LEGEND[1]:
        _legend(out, t)


LEG = [('ROCK', 0), (' > ', None), ('SCISSORS', 2), (' > ', None), ('PAPER', 1), (' > ', None), ('ROCK', 0)]


def _legend(out, t):
    """the rule cycle in team colours under the second caption; each name pops in on a beat"""
    total = sum(len(s) for s, q in LEG) * 6 - 1
    x = int(round(ui.SAFE_CX - total / 2))
    y = 100 + 2 * 21 + 4
    k = 0
    for s, q in LEG:
        if q is not None:
            tk = T.LEGEND[0] + k * T.BEAT / 2
            k += 1
            if t < tk:
                break
        col = T.TEXT[q] if q is not None else (200, 190, 170)
        ui.label(out, s.replace(' ', ' '), x, y, col, 1)
        x += len(s) * 6
