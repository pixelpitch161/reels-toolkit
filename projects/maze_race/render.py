"""Maze Race renderer.

back wall + window (night sky, moon) -> per-pixel desk (walnut planks; lamp pool with shadows of everything, candle
pools, moon, contact AO) -> props behind the board -> the board: per-pixel floor from a board-space texture (carved
maple cells, baked lamp/candle/moon shadows of every wall, the explorers' paint: FLOOD's ink, DIVER's thread and
stains, SEEKER's light wash and pool, the marble's scuffs, the goal inlay) -> rim, walls and explorer heads
(flat-shaded polygons, painter's order) -> props in front -> locked DOF on the board -> glows (lamp, candles, orb,
bead, finish bursts, dust in the beam) -> bloom, grade -> race legend, corner labels, impact captions.

    python -m reelkit.sheet render.py test/sheet.png 0.5 3 6 12 22 30 37 42 45.5 49 54 58.5 60.5
    python -m reelkit.video render.py audio.wav maze_race.mp4"""
import math
import os
import sys

import numpy as np
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, SAFE, dirv, whole_period, ss
from reelkit.camera import Cam
from reelkit.raster import Frame, painter, apply_glows
from reelkit.lighting import Lights, shade
from reelkit.shadows import MaskGrid, directional_mask, point_mask, contact_ao, proj_point
from reelkit.lens import coc_map, dof
from reelkit.post import bloom, grade, vignette, top_scrim
from reelkit.geometry import box, icosa
from reelkit import ui
from reelkit.font import ADV
import timeline as T
import scene as S

DUR = T.DUR
HALF = S.HALF
N = S.N

# ================================================================== camera
BASE = dict(tx=1.25, tz=0.0, dist=84.0, yaw=0.0, pitch=52.0, fov=43.0, sy=256.0)
CLOSE = dict(tx=0.5, tz=0.0, dist=42.0, yaw=0.0, pitch=58.0, fov=43.0, sy=256.0)
OPEN = dict(tx=1.25, tz=0.0, dist=86.0, yaw=-1.5, pitch=51.0, fov=43.0, sy=256.0)
T_IN0, T_IN1 = T.T_FINAL + 1.2, T.T_FINAL + 3.4          # dolly into the centre for the final stretch
_tw = T.T_WIN if T.T_WIN is not None else T.T_STOP - 4
T_OUT0, T_OUT1 = max(_tw + 1.2, T_IN1 + 0.5), max(_tw + 3.6, T_IN1 + 2.9)


def _mix(a, b, u):
    return {k: a[k] + (b[k] - a[k]) * u for k in a}


def cam_params(t):
    e = T.ease
    p = _mix(OPEN, BASE, e(t / 4.0))                                  # settle from the opening view by 4 s
    p = _mix(p, CLOSE, e((t - T_IN0) / (T_IN1 - T_IN0)) * (1 - e((t - T_OUT0) / (T_OUT1 - T_OUT0))))
    p = _mix(p, OPEN, e((t - 57.6) / 3.4))                            # drift back to the opening view for the loop
    sway = math.sin(2 * math.pi * t / DUR)                            # one gentle whole cycle per video
    p['yaw'] += 1.6 * sway
    p['dist'] += 0.8 * math.sin(2 * math.pi * 2 * t / DUR + 0.7)
    return p


def camera(t):
    p = cam_params(t)
    return Cam(np.array([p['tx'], S.YB, p['tz']]), p['dist'], p['yaw'], p['pitch'], p['fov'], p['sy'])


# ================================================================== lights
LAMP_COL = np.array([1.25, 1.05, 0.78])
CANDLE_COL = np.array([1.2, 0.62, 0.26])
MOON_DIR = (S.MOON_POS - S.PIVOT) / np.linalg.norm(S.MOON_POS - S.PIVOT)
MOON_COL = np.array([0.32, 0.4, 0.62])
FLAME = [c + [0, fy, 0] for c, fy in zip(S.CANDLES, S.FLAME_Y)]
COS_IN, COS_OUT = math.cos(math.radians(30)), math.cos(math.radians(52))
LAMP_R = 30.0


def flick(k, t):
    return 0.84 + 0.16 * math.sin(2 * math.pi * 53 * t / DUR + 1.9 * k) * math.sin(2 * math.pi * 31 * t / DUR + 0.4 + k)


def spot(P):
    """lamp cone x falloff at points P (N, 3), normalised so the board centre is ~1"""
    v = np.atleast_2d(P) - S.LAMP[None]
    d = np.linalg.norm(v, axis=1) + 1e-6
    c = (v / d[:, None]) @ S.LAMP_AX
    cone = np.clip((c - COS_OUT) / (COS_IN - COS_OUT), 0, 1)
    cone = cone * cone * (3 - 2 * cone)
    return cone * (1 + (np.linalg.norm(S.LAMP - S.PIVOT) / LAMP_R) ** 2) / (1 + (d / LAMP_R) ** 2)


def lights(t, cam):
    L = Lights(amb=(0.34, 0.31, 0.40), sky=(0.07, 0.07, 0.1))
    L.view = cam.view
    L.key('lamp', (S.LAMP - S.PIVOT) / np.linalg.norm(S.LAMP - S.PIVOT), LAMP_COL)
    L.key('moon', MOON_DIR, MOON_COL)
    L.fill(dirv(180, 25), (0.1, 0.1, 0.16))
    for k, fp in enumerate(FLAME):
        L.point(fp, CANDLE_COL * 1.5 * flick(k, t), 6.0)
    return L


# ================================================================== desk shadows (static)
DESK = MaskGrid((-S.DESK_X, S.DESK_X, S.DESK_Z0, S.DESK_Z1, 0.1))
LAMP_MASK = point_mask(DESK, S.CASTERS, S.LAMP, blur=2.0)
MOON_MASK = directional_mask(DESK, S.CASTERS, MOON_DIR, blur=1.5)
CANDLE_MASK = [point_mask(DESK, [Q for o in S.STATIC if o.cast and np.linalg.norm(o.ctr[[0, 2]] - fp[[0, 2]]) > 2.2 for Q, n in o.faces]
                          + [Q for Q, n in S.BOARD_BOX], fp, blur=2.5) for fp in FLAME]


def desk_albedo(X, Z):
    """walnut planks along x with grain"""
    plank = np.floor((Z + 100) / 3.1)
    h = np.sin(plank * 12.9898) * 43758.5453
    h = h - np.floor(h)
    base = np.array([96, 58, 36], float)[None] * (0.86 + 0.24 * h)[:, None]
    grain = 0.06 * np.sin(X * 1.7 + plank * 3.1 + np.sin(X * 0.31 + plank) * 2.0)
    seam = (np.abs(((Z + 100) % 3.1) - 0.04) < 0.07) * 0.35
    return base * (1 + grain - seam)[:, None]


# ================================================================== the board: texture space
TP = 12                                       # texels per cell
TEX = N * TP
_i = (np.arange(TEX) + 0.5) / TP
LZ, LX = np.meshgrid(_i - HALF, _i - HALF, indexing='ij')   # texel centres (board-local), [row, col]
RR = np.minimum((np.arange(TEX) // TP), N - 1)[:, None].repeat(TEX, 1)
CC = np.minimum((np.arange(TEX) // TP), N - 1)[None, :].repeat(TEX, 0)
FV = ((np.arange(TEX) % TP) + 0.5)[:, None].repeat(TEX, 1) / TP
FU = ((np.arange(TEX) % TP) + 0.5)[None, :].repeat(TEX, 0) / TP

WALL_SET = set()
for kind, a, b0, b1 in T.WALLS:
    WALL_SET.add((kind, a, b0))


def _has_wall(r, c, side):
    if side == 0:
        return ('h', r, c) in WALL_SET
    if side == 2:
        return ('h', r + 1, c) in WALL_SET
    if side == 3:
        return ('v', c, r) in WALL_SET
    return ('v', c + 1, r) in WALL_SET


def _wood():
    tone = rng_cell = np.random.default_rng(5).uniform(0.92, 1.06, (N, N))
    base = np.array([226, 186, 132], float)
    a = base[None, None] * tone[RR, CC][..., None]
    grain = 0.05 * np.sin(LX * 9.0 + np.sin(LZ * 1.3 + LX * 0.4) * 3.0) + 0.03 * np.sin(LZ * 23.0 + LX * 2.0)
    a *= (1 + grain)[..., None]
    # carved channels: darker next to walls (contact shading)
    ao = np.ones((TEX, TEX))
    W_N = np.array([[_has_wall(r, c, 0) for c in range(N)] for r in range(N)])
    W_S = np.array([[_has_wall(r, c, 2) for c in range(N)] for r in range(N)])
    W_W = np.array([[_has_wall(r, c, 3) for c in range(N)] for r in range(N)])
    W_E = np.array([[_has_wall(r, c, 1) for c in range(N)] for r in range(N)])
    k = 0.28
    ao *= 1 - k * W_N[RR, CC] * np.clip(1 - FV / 0.22, 0, 1)
    ao *= 1 - k * W_S[RR, CC] * np.clip(1 - (1 - FV) / 0.22, 0, 1)
    ao *= 1 - k * W_W[RR, CC] * np.clip(1 - FU / 0.22, 0, 1)
    ao *= 1 - k * W_E[RR, CC] * np.clip(1 - (1 - FU) / 0.22, 0, 1)
    a *= ao[..., None]
    # goal inlay: brass with a dark star
    g0 = (S.N // 2 - 1)
    gx, gz = LX, LZ
    ing = (np.abs(gx) < 1.0) & (np.abs(gz) < 1.0)
    ang = np.arctan2(gz, gx)
    rr = np.hypot(gx, gz)
    star = rr < 0.45 + 0.4 * np.abs(np.cos(4 * ang)) ** 3
    brass = np.where(star[..., None], np.array([150, 104, 40.0]), np.array([212, 170, 84.0]))
    ring = (rr > 0.86) & (rr < 0.96)
    brass = np.where(ring[..., None], np.array([120, 84, 34.0]), brass)
    a = np.where(ing[..., None], brass, a)
    # start pads: a ring of each explorer's colour in its corner cell
    for q in range(4):
        r0, c0 = T.CORNER[q]
        m = (RR == r0) & (CC == c0)
        d = np.hypot(FU - 0.5, FV - 0.5)
        pad = m & (d > 0.3) & (d < 0.4)
        a = np.where(pad[..., None], T.PAINT[q] * 0.55 + 60, a)
    return a, ing


WOOD, GOAL_MASK = _wood()

# wall shadows on the floor, from the lamp, the two candles and the moon (board-local, untilted)
_FACES_L = [Q for Q in S.wall_boxes(T.WALLS)[0]]
for Bf in S.rim_boxes():
    _FACES_L += [Q for Q, n in Bf]
TGRID = MaskGrid((-HALF, HALF, -HALF, HALF, 1.0 / TP))
_lamp_l = S.LAMP - S.PIVOT
SH_LAMP = point_mask(TGRID, _FACES_L, _lamp_l, blur=0.8, min_y=0.01)
SH_MOON = directional_mask(TGRID, _FACES_L, MOON_DIR, blur=0.8, min_y=0.01)
SH_CAND = [point_mask(TGRID, _FACES_L, fp - S.PIVOT, blur=1.0, min_y=0.01) for fp in FLAME]
_P = np.stack([LX, np.zeros_like(LX), LZ], -1).reshape(-1, 3) + S.PIVOT
_v = S.LAMP[None] - _P
_lam = np.clip(_v[:, 1] / np.linalg.norm(_v, axis=1), 0, 1)
LMAP_LAMP = (spot(_P) * (0.35 + 0.65 * _lam)).reshape(TEX, TEX)
LMAP_MOON = max(0.0, MOON_DIR[1]) * 0.9
LMAP_CAND = []
for fp in FLAME:
    v = fp[None] - _P
    d = np.linalg.norm(v, axis=1)
    LMAP_CAND.append(((0.3 + 0.7 * np.clip(v[:, 1] / d, 0, 1)) / (1 + (d / 6.0) ** 2)).reshape(TEX, TEX))
STATIC_LIGHT = (np.array([0.27, 0.25, 0.33])[None, None] + 0.05
                + 0.85 * LMAP_LAMP[..., None] * (1 - SH_LAMP)[..., None] * LAMP_COL[None, None]
                + LMAP_MOON * (1 - SH_MOON)[..., None] * MOON_COL[None, None])

# wall geometry (board-local), shaded per frame after the tilt
WQ, WN, WID, WCTR = S.wall_boxes(T.WALLS)
WCOL = np.where(WN[:, 1:2] > 0.5, np.array([232, 192, 136.0])[None], np.array(S.WOOD_WALL, float)[None])
WCOL = WCOL * np.random.default_rng(9).uniform(0.95, 1.05, (len(WQ), 1))
RIMQ, RIMN, RIMID = [], [], []
_k = 0
for Bf in S.rim_boxes():
    pass
# the rim, cut into unit segments so it sorts with the walls
for side in range(4):
    for s in range(-11, 11):
        a, b = s, s + 1
        if side in (0, 1):
            cz = -(HALF + S.RIM_W / 2) if side == 0 else HALF + S.RIM_W / 2
            fs = box((a + b) / 2, (S.RIM_H - S.SLAB) / 2, cz, 0.5, (S.RIM_H + S.SLAB) / 2, S.RIM_W / 2)
            if abs((a + b) / 2) > HALF + S.RIM_W:
                continue
        else:
            if abs((a + b) / 2) > HALF:
                continue
            cx = -(HALF + S.RIM_W / 2) if side == 2 else HALF + S.RIM_W / 2
            fs = box(cx, (S.RIM_H - S.SLAB) / 2, (a + b) / 2, S.RIM_W / 2, (S.RIM_H + S.SLAB) / 2, 0.5)
        for Q, n in fs:
            if n[1] < -0.5:
                continue
            RIMQ.append(Q)
            RIMN.append(n)
            RIMID.append(10000 + _k)
        _k += 1
RIMQ, RIMN, RIMID = np.array(RIMQ), np.array(RIMN), np.array(RIMID)
RIMCOL = np.where(RIMN[:, 1:2] > 0.5, np.array([150, 92, 56.0])[None], np.array(S.WOOD_RIM, float)[None])
RIMCOL = RIMCOL * np.random.default_rng(11).uniform(0.95, 1.05, (len(RIMQ), 1))
BQ = np.concatenate([WQ, RIMQ])
BN = np.concatenate([WN, RIMN])
BID = np.concatenate([WID, RIMID])
BCOL = np.concatenate([WCOL, RIMCOL])
BCTR = BQ.mean(1)
_ids, _inv = np.unique(BID, return_inverse=True)
BOXC = np.array([BCTR[_inv == k].mean(0) for k in range(len(_ids))])      # centre of each box
BOX_OF = _inv
SPHERE = icosa([0, 0, 0], 1.0, jit=0.0)
SPH_Q = np.array([Q for Q, n in SPHERE])
SPH_N = np.array([n for Q, n in SPHERE])


def tilt_matrix(t):
    gx, gz = T.tilt(t)
    k = math.radians(1.8)
    ax, az = k * gz, -k * gx                           # near side dips for +gz, right side dips for +gx
    ca, sa, cb, sb = math.cos(ax), math.sin(ax), math.cos(az), math.sin(az)
    Rx = np.array([[1, 0, 0], [0, ca, -sa], [0, sa, ca]])
    Rz = np.array([[cb, -sb, 0], [sb, cb, 0], [0, 0, 1]])
    return Rz @ Rx


def to_world(P, R):
    return np.asarray(P) @ R.T + S.PIVOT


# ================================================================== paint state at time t (texture space)
FLOW = 0.17
BG_FADE = 0.0


def fade_all(t):
    """trails fade at the end; 1 = full"""
    return 1.0 - ss(T.T_FADE0, T.T_FADE1, t)


def _fill(q, t):
    tu = T.UNC_T[q][RR, CC]
    age = t - tu
    dr = T.UNC_DIR[q][RR, CC]
    s = np.where(dr == 0, FV, np.where(dr == 1, 1 - FU, np.where(dr == 2, 1 - FV, np.where(dr == 3, FU,
                 2 * np.maximum(np.abs(FU - 0.5), np.abs(FV - 0.5))))))
    flow = FLOW if q < 3 else 0.12
    filled = (age >= 0) & (age / flow >= s)
    return filled, age


def diver_state(t):
    """thread polyline (cells), the growing tip fraction, retraction info"""
    i = T.last_step(1, t)
    if i < 0:
        return [], 0.0, None
    e = T.STEPS[1][i]
    stack = T.DIVER_STACK[i]
    u = t - e['t']
    if e['back']:                                         # retract (0.12 s), then grow into the new cell
        tr = 0.12
        if u < tr:
            prev = T.DIVER_STACK[i - 1] if i > 0 else []
            nback = len(e['back'])
            keep = len(prev) - nback * min(1.0, u / tr)
            return prev, keep - math.floor(keep), (prev, keep)
        u -= tr
    grow = min(1.0, u / FLOW)
    return stack, grow, None


def board_texture(t, light):
    """composite floor colour (TEX, TEX, 3) at time t, given the per-texel light (TEX, TEX, 3)"""
    base = WOOD * light
    lum = light.mean(-1, keepdims=True)
    out = base.copy()
    fa = fade_all(t)
    route_on = T.T_ROUTE0 <= t
    dim = 1.0 - 0.6 * ss(T.T_ROUTE0, T.T_ROUTE0 + 0.4, t)
    if t < T.T_HOME1 and fa > 0:
        # FLOOD ink, MARBLE scuffs, SEEKER wash, DIVER stains (oldest layers first)
        for q, a_max in ((0, 0.86), (3, 0.38), (2, 0.55), (1, 0.34)):
            filled, age = _fill(q, t)
            if not filled.any():
                continue
            col = T.PAINT[q][None, None]
            fresh = np.exp(-np.clip(age, 0, None) / (0.9 if q == 0 else 0.6))[..., None]
            a = (a_max * filled * fa * dim)[..., None]
            if q == 0:
                # the newest ring surges
                i = T.last_step(0, t)
                if i >= 0 and t < (T.FIN[0] or 99) + 0.6:
                    dcur = T.FLOOD_DEPTH[i] // 4 * 4                 # every 4th ring surges (as the strings do)
                    j = int(np.searchsorted(T.FLOOD_DEPTH, dcur))
                    if j < len(T.STEPS[0]):
                        pulse = math.exp(-max(0.0, t - T.STEPS[0][j]['t']) / 0.35)
                        fresh = fresh * (1 + 0.8 * pulse)
            paint = col * (0.5 + 0.55 * lum) + col * 0.42 * fresh
            out = out * (1 - a) + paint * a
        # DIVER's thread (on top)
        stack, grow, retract = diver_state(t)
        if stack:
            img = Image.new('L', (TEX, TEX), 0)
            dr = ImageDraw.Draw(img)
            pts = [((c + 0.5) * TP, (r + 0.5) * TP) for r, c in stack]
            if retract is None:
                if len(pts) >= 2:
                    p0, p1 = pts[-2], pts[-1]
                    pts[-1] = (p0[0] + (p1[0] - p0[0]) * grow, p0[1] + (p1[1] - p0[1]) * grow)
            else:
                prev, keep = retract
                n = max(1, int(math.floor(keep)))
                fr = keep - math.floor(keep)
                pts = [((c + 0.5) * TP, (r + 0.5) * TP) for r, c in prev[:n + 1]]
                if len(pts) >= 2:
                    p0, p1 = pts[-2], pts[-1]
                    pts[-1] = (p0[0] + (p1[0] - p0[0]) * fr, p0[1] + (p1[1] - p0[1]) * fr)
            if len(pts) >= 2:
                dr.line(pts, fill=150, width=5, joint='curve')
                dr.line(pts, fill=255, width=3, joint='curve')
            m = np.asarray(img, np.float32)[..., None] / 255.0
            edge = (m > 0.4) & (m < 0.8)
            core = m >= 0.8
            th = T.PAINT[1][None, None]
            a = fa * dim
            out = np.where(edge, out * (1 - 0.7 * a) + np.array([70, 10, 16.0]) * 0.7 * a, out)
            out = np.where(core, out * (1 - a) + (th * (0.62 + 0.5 * lum) + th * 0.25) * a, out)
    # the winning route lights up cell by cell (bars 26-28) and fades with the rest
    if route_on and T.ROUTE and fa > 0:
        lit = np.zeros((N, N))
        for k, (cell, rt) in enumerate(zip(T.ROUTE, T.ROUTE_T)):
            if t >= rt:
                lit[cell] = 1.0 + 2.0 * math.exp(-(t - rt) / 0.3)
        if t >= T.ROUTE_T[-1]:                                    # the whole route breathes on the resolve
            lit *= 1.0 + 0.5 * math.exp(-(t - T.ROUTE_T[-1]) / 0.6)
        L = lit[RR, CC][..., None]
        wc = T.PAINT[T.WINNER][None, None]
        on = (L > 0) * fa
        out = out * (1 - 0.85 * on) + (wc * (0.75 + 0.4 * lum) + wc * 0.35 * (L - 1).clip(0)) * 0.85 * on
    # goal room: glows in the finishers' colours
    for q in range(4):
        if T.FIN[q] is not None and T.FIN[q] <= t < T.T_HOME1:
            k = 0.35 + 0.65 * math.exp(-(t - T.FIN[q]) / 0.6) if T.PLACE[q] == 0 else 0.25 * math.exp(-(t - T.FIN[q]) / 0.8)
            k *= fa if T.PLACE[q] else max(fa, 0.0)
            gm = GOAL_MASK[..., None] * k
            out = out * (1 - 0.5 * gm) + T.PAINT[q][None, None] * (1.1 * gm)
    return out


def seeker_pos(t):
    """the gold light: board-local (x, z) of the orb, darting along its path after each step"""
    i = T.last_step(2, t)
    if i < 0:
        r, c = T.CORNER[2]
        return np.array([c + 0.5 - HALF, r + 0.5 - HALF]), 0.0
    e = T.STEPS[2][i]
    path = e['path']
    dur = min(0.26, 0.05 * len(path)) if len(path) > 2 else FLOW
    u = min(1.0, (t - e['t']) / dur)
    u = u * u * (3 - 2 * u)
    if len(path) < 2:
        r, c = path[0]
        return np.array([c + 0.5 - HALF, r + 0.5 - HALF]), 0.0
    if len(path) == 2 or True:
        f = u * (len(path) - 1)
        k = min(int(f), len(path) - 2)
        a = f - k
        r0, c0 = path[k]
        r1, c1 = path[k + 1]
        r, c = r0 + (r1 - r0) * a, c0 + (c1 - c0) * a
        speed = (len(path) - 1) / dur if u < 1 else 0.0
        return np.array([c + 0.5 - HALF, r + 0.5 - HALF]), speed


def diver_head(t):
    stack, grow, retract = diver_state(t)
    if not stack:
        r, c = T.CORNER[1]
        return np.array([c + 0.5 - HALF, r + 0.5 - HALF])
    if retract is not None:
        prev, keep = retract
        n = max(1, int(math.floor(keep)))
        fr = keep - math.floor(keep)
        cells = prev[:n + 1]
        if len(cells) >= 2:
            (r0, c0), (r1, c1) = cells[-2], cells[-1]
            return np.array([c0 + (c1 - c0) * fr + 0.5 - HALF, r0 + (r1 - r0) * fr + 0.5 - HALF])
        r, c = cells[-1]
        return np.array([c + 0.5 - HALF, r + 0.5 - HALF])
    if len(stack) >= 2:
        (r0, c0), (r1, c1) = stack[-2], stack[-1]
        return np.array([c0 + (c1 - c0) * grow + 0.5 - HALF, r0 + (r1 - r0) * grow + 0.5 - HALF])
    r, c = stack[-1]
    return np.array([c + 0.5 - HALF, r + 0.5 - HALF])


def head_home(q, t, pos):
    """59.03-60.9: the heads glide back to their corners (the marble rewinds instead)"""
    if t < T.T_HOME0 or q == 3:
        return pos
    r, c = T.CORNER[q]
    home = np.array([c + 0.5 - HALF, r + 0.5 - HALF])
    u = T.ease((t - T.T_HOME0) / (T.T_HOME1 - T.T_HOME0))
    return pos + (home - pos) * u


# ================================================================== frame
def desk_pass(fr, cam, t, L):
    arr, depth = fr.array(), fr.depth()
    ys, xs = np.mgrid[0:H, 0:W]
    ys, xs = ys.ravel(), xs.ravel()
    X, Z, d = cam.ground_hit(xs.astype(float), ys.astype(float))
    ok = (d > 0) & (Z > S.WALL_Z) & (Z < S.DESK_Z1) & (np.abs(X) < S.DESK_X) & (d < depth[ys, xs])
    xs, ys, X, Z, d = xs[ok], ys[ok], X[ok], Z[ok], d[ok]
    P = np.stack([X, np.zeros_like(X), Z], 1)
    I = np.tile(L.amb + 0.06, (len(X), 1))
    v = S.LAMP[None] - P
    lam = np.clip(v[:, 1] / np.linalg.norm(v, axis=1), 0, 1)
    I += ((1 - DESK.sample(LAMP_MASK, X, Z)) * spot(P) * (0.3 + 0.7 * lam))[:, None] * LAMP_COL[None] * 0.95
    I += ((1 - DESK.sample(MOON_MASK, X, Z)) * max(0.0, MOON_DIR[1]) * 0.9)[:, None] * MOON_COL[None]
    for k, fp in enumerate(FLAME):
        dist = np.sqrt((fp[0] - X) ** 2 + fp[1] ** 2 + (fp[2] - Z) ** 2)
        I += ((1 - DESK.sample(CANDLE_MASK[k], X, Z)) * fp[1] / dist * 1.5 * flick(k, t) / (1 + (dist / 6.0) ** 2))[:, None] * CANDLE_COL[None]
    I *= contact_ao(X, Z, S.FEET)[:, None]
    col = desk_albedo(X, Z) * I
    arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
    depth[ys, xs] = d
    fr.set_array(arr, depth)


def draw_objs(fr, cam, L, objs):
    for _, o in painter([(np.linalg.norm(o.ctr - cam.pos), o) for o in objs]):
        fc = [(cam.depth(Q.mean(0))[0], Q, n, j) for j, (Q, n) in enumerate(o.faces) if np.dot(n, cam.pos - Q.mean(0)) > 0 or o.name in ('dial', 'bezel')]
        if not fc:
            continue
        fc.sort(key=lambda a: -a[0])
        C = np.array([q.mean(0) for _, q, _, _ in fc])
        Nn = np.array([n for _, _, n, _ in fc])
        kv = np.stack([spot(C), np.ones(len(C))], 1)
        if hasattr(o, 'face_col'):
            js = [j for *_, j in fc]
            cols = shade(L, Nn, C, o.face_col[js] * o.tone[js][:, None], kvis=kv)
            cols = np.clip(cols + o.face_col[js] * o.face_emit[js][:, None] * 0.6, 0, 255)
        else:
            cols = shade(L, Nn, C, np.array([o.color * o.tone[j] for *_, j in fc]), kvis=kv, emit=S.EMISSIVE.get(o.name, 0.0))
        for (z, Q, n, j), c in zip(fc, cols):
            fr.poly(cam, Q, c)


def back_wall(fr, cam, L, t):
    C = np.array([Q.mean(0) for Q in S.WALL_FACES])
    Nn = np.tile([0, 0, 1.0], (len(C), 1))
    kv = np.stack([spot(C), np.zeros(len(C))], 1)
    cols = shade(L, Nn, C, np.array([[96, 72, 78.0]]) * S.WALL_TONE[:, None], kvis=kv)
    for Q, c in zip(S.WALL_FACES, cols):
        fr.poly(cam, Q, c)
    # the window: night sky, stars, the moon
    pts = fr.poly(cam, S.PANE, (40, 54, 104))
    if pts is not None:
        P = np.array(pts)
        y0, y1 = int(max(0, P[:, 1].min())), int(min(H, P[:, 1].max()))
        x0, x1 = int(max(0, P[:, 0].min())), int(min(W, P[:, 0].max()))
        if y1 > y0 and x1 > x0:
            arr = fr.array()
            yy = np.arange(y0, y1)[:, None]
            k = (yy - y0) / max(1, y1 - y0)
            sky = np.array([22, 30, 80.0]) * (1 - k[..., None]) + np.array([60, 70, 130.0]) * k[..., None]
            reg = arr[y0:y1, x0:x1]
            m = (np.abs(reg - np.array([40, 54, 104.0])).sum(-1) < 3)
            reg[m] = np.floor((np.broadcast_to(sky, reg.shape)[m]) / 7 + BAYER_FULL[y0:y1, x0:x1][m][:, None]) * 7
            rs = np.random.default_rng(3)
            for _ in range(26):
                sx, sy = rs.integers(x0, max(x0 + 1, x1)), rs.integers(y0, max(y0 + 1, y1))
                if m[sy - y0, sx - x0]:
                    b = 150 + 90 * (0.6 + 0.4 * math.sin(2 * math.pi * 7 * t / DUR + sx))
                    reg[sy - y0, sx - x0] = b
            arr[y0:y1, x0:x1] = reg
            fr.set_array(arr, fr.depth())
    ms = cam.wp(S.MOON_POS)
    fr.disc(ms, 0.9 * cam.f / max(1.0, cam.depth(S.MOON_POS)[0]), (244, 238, 214), n=14, z=cam.depth(S.MOON_POS)[0])
    fr.glow(ms[0], ms[1], 5.0 * cam.f / max(1.0, cam.depth(S.MOON_POS)[0]), (120, 140, 200), 0.3)


def board_pass(fr, cam, t, R, L):
    """per-pixel floor (z-tested) then rim + walls + heads in painter's order"""
    # floor
    nrm = R @ np.array([0, 1.0, 0])
    corners = to_world(np.array([[-HALF, 0, -HALF], [HALF, 0, -HALF], [HALF, 0, HALF], [-HALF, 0, HALF]]), R)
    sc = cam.wp(corners)
    x0, x1 = int(max(0, sc[:, 0].min() - 2)), int(min(W, sc[:, 0].max() + 3))
    y0, y1 = int(max(0, sc[:, 1].min() - 2)), int(min(H, sc[:, 1].max() + 3))
    arr, depth = fr.array(), fr.depth()
    ys, xs = np.mgrid[y0:y1, x0:x1]
    ys, xs = ys.ravel(), xs.ravel()
    dw = cam.pixel_rays(xs.astype(float), ys.astype(float))
    tt = ((S.PIVOT - cam.pos) @ nrm) / np.minimum(dw @ nrm, -1e-6)
    P = cam.pos[None] + tt[:, None] * dw
    Lc = (P - S.PIVOT) @ R                       # board-local
    ok = (np.abs(Lc[:, 0]) < HALF) & (np.abs(Lc[:, 2]) < HALF) & (tt > 0) & (tt < depth[ys, xs])
    xs, ys, Lc, tt = xs[ok], ys[ok], Lc[ok], tt[ok]
    # per-texel light: static + flickering candles + the seeker's pool
    light = STATIC_LIGHT.copy()
    for k in range(2):
        light += (LMAP_CAND[k] * (1 - SH_CAND[k]) * 1.5 * flick(k, t))[..., None] * CANDLE_COL[None, None]
    sp, spd = seeker_pos(t)
    sp = head_home(2, t, sp)
    orb_k = orb_strength(t)
    if orb_k > 0:
        dd = (LX - sp[0]) ** 2 + (LZ - sp[1]) ** 2
        light += (orb_k * 0.9 / (1 + dd / 0.9))[..., None] * np.array([1.0, 0.78, 0.32])[None, None]
    tex = board_texture(t, light)
    # marble shadow (away from the lamp) + contact
    mx, mz = T.marble_xz(t) - HALF
    dl = _lamp_l[[0, 2]] - np.array([mx, mz])
    dl /= np.linalg.norm(dl)
    sx, sz = mx - dl[0] * 0.22, mz - dl[1] * 0.22
    sh = np.exp(-((LX - sx) ** 2 + (LZ - sz) ** 2) / 0.06)
    tex *= (1 - 0.55 * sh)[..., None]
    iu = np.clip(((Lc[:, 0] + HALF) * TP).astype(int), 0, TEX - 1)
    iv = np.clip(((Lc[:, 2] + HALF) * TP).astype(int), 0, TEX - 1)
    col = tex[iv, iu]
    arr[ys, xs] = np.floor(col / 5 + BAYER_FULL[ys, xs, None]) * 5
    depth[ys, xs] = tt
    fr.set_array(arr, depth)
    # rim + walls (+ heads) in painter's order
    Qw = BQ @ R.T + S.PIVOT
    Nw = BN @ R.T
    Cw = Qw.mean(1)
    vis = ((cam.pos[None] - Cw) * Nw).sum(1) > 0
    boxd = np.linalg.norm(BOXC @ R.T + S.PIVOT - cam.pos[None], axis=1)
    items = []
    idx = np.nonzero(vis)[0]
    kv = np.stack([spot(Cw[idx]), np.ones(len(idx))], 1)
    cols = shade(L, Nw[idx], Cw[idx], BCOL[idx], kvis=kv)
    fd = np.linalg.norm(Cw[idx] - cam.pos[None], axis=1)
    for j, i in enumerate(idx):
        items.append((boxd[BOX_OF[i]], fd[j], Qw[i], cols[j]))
    # heads: marble (steel sphere), diver bead
    m3 = to_world(np.array([[mx, 0.27, mz]]), R)[0]
    sq = SPH_Q * 0.27 + m3
    sn = SPH_N @ np.eye(3)
    sc_ = sq.mean(1)
    v_ = ((cam.pos[None] - sc_) * sn).sum(1) > 0
    kvm = np.stack([spot(sc_), np.ones(len(sc_))], 1)
    mcol = shade(L, sn, sc_, np.tile(T.PAINT[3], (len(sc_), 1)), kvis=kvm, rimk=0.9)
    # specular glint toward the lamp
    hv = (S.LAMP - m3) / np.linalg.norm(S.LAMP - m3) + cam.view
    hv /= np.linalg.norm(hv)
    spec = np.clip(sn @ hv, 0, 1) ** 12
    mcol = np.clip(mcol + spec[:, None] * 160, 0, 255)
    md = np.linalg.norm(m3 - cam.pos)
    for k in np.nonzero(v_)[0]:
        items.append((md, np.linalg.norm(sc_[k] - cam.pos), sq[k], mcol[k]))
    bh = head_home(1, t, diver_head(t))
    if t < T.T_HOME1 + 0.2:
        b3 = to_world(np.array([[bh[0], 0.17, bh[1]]]), R)[0]
        bq = SPH_Q * 0.17 + b3
        bc = bq.mean(1)
        bv = ((cam.pos[None] - bc) * SPH_N).sum(1) > 0
        bcol = shade(L, SPH_N, bc, np.tile([236, 54, 60.0], (len(bc), 1)), kvis=np.stack([spot(bc), np.ones(len(bc))], 1), emit=0.35)
        bd = np.linalg.norm(b3 - cam.pos)
        for k in np.nonzero(bv)[0]:
            items.append((bd, np.linalg.norm(bc[k] - cam.pos), bq[k], bcol[k]))
    items.sort(key=lambda it: (-it[0], -it[1]))
    for _, _, Q, c in items:
        fr.poly(cam, Q, c)
    return m3, bh, sp, spd


def orb_strength(t):
    return 1.0 if t < T.T_HOME1 else max(0.0, 1 - (t - T.T_HOME1) / 0.1)


def render(f):
    t = f / FPS
    cam = camera(t)
    L = lights(t, cam)
    R = tilt_matrix(t)
    fr = Frame((14, 12, 22))
    back_wall(fr, cam, L, t)
    desk_pass(fr, cam, t, L)
    zb = np.linalg.norm(S.PIVOT - cam.pos)
    behind = [o for o in S.STATIC if np.linalg.norm(o.ctr - cam.pos) > zb]
    front = [o for o in S.STATIC if np.linalg.norm(o.ctr - cam.pos) <= zb]
    draw_objs(fr, cam, L, behind)
    m3, bh, sp, spd = board_pass(fr, cam, t, R, L)
    draw_objs(fr, cam, L, front)
    draw_life(fr, cam, t, L)
    # lens: focus locked on the board
    img, depth = fr.array(), fr.depth()
    zf, band = focus(cam, R)
    img = dof(img, coc_map(depth, zf, 0.012 * cam.f * min(1.2, 60.0 / zf), sharp=band))
    glows(fr, cam, t, R, m3, bh, sp)
    img = apply_glows(img, fr.glows)
    beam(img, cam, t)
    img = grade(bloom(np.clip(img, 0, 255), thr=205, k=0.35))
    img = np.clip(img * vignette(strength=0.6, cap=0.3) * top_scrim(), 0, 255).astype(np.uint8)
    return hud(img, t, cam, R)


def focus(cam, R):
    """focus depth = board centre; the sharp band covers every corner of the board (rim included)"""
    zf = cam.depth(S.PIVOT)[0]
    a = HALF + S.RIM_W
    pts = to_world(np.array([[x, y, z] for x in (-a, a) for z in (-a, a) for y in (-S.SLAB, S.RIM_H + 0.3)]), R)
    zz = cam.depth(pts)
    band = float(np.max(np.abs(1 - zf / zz))) + 0.012
    return zf, band


def glows(fr, cam, t, R, m3, bh, sp):
    # lamp mouth + bulb
    s = cam.wp(S.LAMP_HEAD)
    zl = cam.depth(S.LAMP_HEAD)[0]
    fr.glow(s[0], s[1], 2.6 * cam.f / zl, (255, 214, 150), 0.55)
    for k, fp in enumerate(FLAME):
        p = cam.wp(fp)
        z = cam.depth(fp)[0]
        fl = flick(k, t)
        fr.disc(p, 0.26 * cam.f / z, (255, 236, 170), n=6)
        fr.glow(p[0], p[1], 2.4 * cam.f / z, (255, 160, 70), 0.42 * fl)
    # seeker orb, diver bead, the newest flood cell
    k = orb_strength(t)
    if k > 0:
        o3 = to_world(np.array([[sp[0], 0.45, sp[1]]]), R)[0]
        p = cam.wp(o3)
        z = cam.depth(o3)[0]
        fr.glow(p[0], p[1], 1.6 * cam.f / z, (255, 200, 80), 0.75 * k)
        fr.glow(p[0], p[1], 0.5 * cam.f / z, (255, 250, 210), 0.9 * k)
    p = cam.wp(m3 + np.array([0, 0.2, 0]))
    fr.glow(p[0], p[1], 0.7 * cam.f / cam.depth(m3)[0], (200, 220, 255), 0.35)
    b3 = to_world(np.array([[bh[0], 0.2, bh[1]]]), R)[0]
    p = cam.wp(b3)
    fr.glow(p[0], p[1], 0.9 * cam.f / cam.depth(b3)[0], (255, 70, 70), 0.4)
    # the spark that runs the winning route (bars 26-28)
    if T.ROUTE and T.ROUTE_T[0] <= t < T.ROUTE_T[-1] + 0.4:
        k = int(np.searchsorted(T.ROUTE_T, t, side='right')) - 1
        k = max(0, min(k, len(T.ROUTE) - 1))
        if k + 1 < len(T.ROUTE):
            u = min(1.0, (t - T.ROUTE_T[k]) / max(1e-3, T.ROUTE_T[k + 1] - T.ROUTE_T[k]))
        else:
            u = 0.0
        (r0, c0) = T.ROUTE[k]
        (r1, c1) = T.ROUTE[min(k + 1, len(T.ROUTE) - 1)]
        px, pz = c0 + (c1 - c0) * u + 0.5 - HALF, r0 + (r1 - r0) * u + 0.5 - HALF
        s3 = to_world(np.array([[px, 0.4, pz]]), R)[0]
        p = cam.wp(s3)
        z = cam.depth(s3)[0]
        wc = tuple(T.PAINT[T.WINNER])
        fr.glow(p[0], p[1], 1.4 * cam.f / z, wc, 0.8)
        fr.glow(p[0], p[1], 0.45 * cam.f / z, (255, 255, 235), 0.9)
    # finish bursts in the goal room
    for q in range(4):
        if T.FIN[q] is not None and 0 <= t - T.FIN[q] < 1.2:
            u = t - T.FIN[q]
            g3 = to_world(np.array([[0.0, 0.3, 0.0]]), R)[0]
            p = cam.wp(g3)
            big = 1.0 if T.PLACE[q] == 0 else 0.55
            fr.glow(p[0], p[1], (1.5 + 5.0 * u) * cam.f / cam.depth(g3)[0] * big, tuple(T.PAINT[q]), 0.9 * (1 - u / 1.2) * big)


def beam(img, cam, t):
    """dust motes drifting in the lamp's beam (whole cycles over the video)"""
    for k, (u, a, r, ph, spd) in enumerate(S.DUST):
        w = 2 * math.pi * (1 + int(spd * 3)) / DUR
        uu = (u + 0.25 * math.sin(w * t + ph * 6.28)) % 1.0
        dist = 2.0 + uu * 13.0
        ang = a * 6.283 + 0.4 * math.sin(w * t + ph)
        rad = r * dist * math.tan(math.radians(26))
        ax = S.LAMP_AX
        t1 = np.cross(ax, [0, 1.0, 0])
        t1 /= np.linalg.norm(t1)
        t2 = np.cross(ax, t1)
        P = S.LAMP + ax * dist + rad * (math.cos(ang) * t1 + math.sin(ang) * t2)
        z = cam.depth(P)[0]
        if z <= 1:
            continue
        x, y = cam.wp(P)
        xi, yi = int(x), int(y)
        if 0 <= xi < W - 1 and 0 <= yi < H - 1:
            b = (0.45 + 0.55 * math.sin(2 * math.pi * 5 * t / DUR + ph * 9)) * 70
            img[yi, xi] += b * np.array([1.0, 0.9, 0.7])


def draw_life(fr, cam, t, L):
    """the moth circling the lamp; the clock's second hand ticking on the beat"""
    P = whole_period(S.MOTH_PERIOD)
    a = 2 * math.pi * t / P
    c = S.LAMP_HEAD + np.array([S.MOTH_R * math.cos(a), 1.2 + 0.6 * math.sin(2 * a), S.MOTH_R * math.sin(a) * 0.7]) - S.LAMP_AX * 1.0
    flap = 0.5 + 0.5 * math.sin(2 * math.pi * 280 * t / DUR)
    s = cam.wp(c)
    z = cam.depth(c)[0]
    if z > 1:
        w = 0.55 * cam.f / z
        col = (200, 180, 150)
        for sgn in (-1, 1):
            fr.screen_poly([(s[0], s[1]), (s[0] + sgn * w, s[1] - w * (0.3 + 0.7 * flap)), (s[0] + sgn * w * 0.8, s[1] + w * 0.4)], col, z)
    beat_i = int(t / T.BEAT)
    ang = -2 * math.pi * beat_i / 62 + math.pi / 2
    d0 = S.CLOCK_DIAL + [0, 0, 0.04]
    tip = d0 + 1.35 * np.array([math.cos(ang), math.sin(ang), 0])
    fr.line(cam, [d0, tip], (40, 30, 26), 1)
    fr.line(cam, [d0, d0 + 0.9 * np.array([math.cos(1.1), math.sin(1.1), 0])], (30, 24, 20), 1)


# ================================================================== HUD
LANE_X0, LANE_X1, LANE_Y1 = SAFE[0] + 6, SAFE[2] - 6, SAFE[3] - 5
LANE_H = 11


def _corner_screen(q, cam, R):
    r, c = T.CORNER[q]
    p = to_world(np.array([[c + 0.5 - HALF, 0.5, r + 0.5 - HALF]]), R)[0]
    return cam.wp(p)


def rank_slots(t):
    """smoothed row position of each racer (lanes slide 0.3 s when the ranking changes)"""
    out = np.zeros(4)
    for q in range(4):
        acc = 0.0
        for k, dt in enumerate(np.linspace(0, 0.3, 7)):
            acc += T.ranking(max(0.0, t - dt)).index(q)
        out[q] = acc / 7
    return out


def hud(img, t, cam, R):
    out = img.copy()
    # corner labels (0-2 s), sliding into their lanes (2.2-3.6 s)
    slots = rank_slots(t)
    leg_a = min(1.0, max(0.0, (t - T.LEGEND_IN) / 0.4)) * (1 - min(1.0, max(0.0, (t - T.LEGEND_OUT[0]) / (T.LEGEND_OUT[1] - T.LEGEND_OUT[0]))))
    lanes = []
    lead = T.ranking(t)[0]
    for q in range(4):
        fin = T.finished(q, t)
        place = T.PLACE[q] + 1 if fin else None
        fl = 0.0
        if fin:
            fl = math.exp(-(t - T.FIN[q]) / 0.5) * (0.6 + 0.4 * (math.sin((t - T.FIN[q]) * 25) > 0))
        lost = t >= T.T_STOP and not fin
        a_l = 1.0 if t >= T.LANE_T[q] + 0.45 else 0.0
        lanes.append(dict(name=T.NAMES[q], sub=T.INSTR[q], color=T.TEXT[q], progress=T.progress(q, t), slot=slots[q],
                          place=place, flash=fl, lost=lost, lead=(q == lead and t >= 6.0 and T.T_WIN is not None and t < T.T_WIN),
                          alpha=a_l))
    if leg_a > 0:
        ui.race_lanes(out, lanes, x0=LANE_X0, x1=LANE_X1, y1=LANE_Y1, lane_h=LANE_H, alpha=leg_a)
    # labels: pop at each corner, then fly down to the lane
    y0_lanes = LANE_Y1 - 4 * LANE_H - 6
    for q in range(4):
        tl = T.LABEL_T[q]
        if not (tl <= t < T.LANE_T[q] + 0.45):
            continue
        name, sub = T.NAMES[q], T.INSTR[q]
        w = (len(name) + 1 + len(sub)) * ADV - 1
        cx, cy = _corner_screen(q, cam, R)
        far = q in (0, 1)
        lx = cx + (6 if q in (0, 3) else -6 - w)
        ly = cy + (8 if far else -16)
        lx = min(max(lx, SAFE[0] + 2), SAFE[2] - 2 - w)
        ly = min(max(ly, SAFE[1] + 52), SAFE[3] - 64)
        u = T.ease((t - T.LANE_T[q]) / 0.45)
        tx, ty = LANE_X0 + 15, y0_lanes + 3 + slots[q] * LANE_H + 2
        x = lx + (tx - lx) * u
        y = ly + (ty - ly) * u
        pop = 1.0 + 0.6 * max(0.0, 1 - (t - tl) / 0.12) ** 2
        a_in = min(1.0, (t - tl) / 0.12)                          # fades in over ~4 frames (GO lands on frame 0)
        keep = out.copy() if a_in < 1.0 else None
        # backing pill
        x0, y0b, x1, y1b = int(x - 3), int(y - 3), int(x + w + 3), int(y + 10)
        if x0 >= 0 and y0b >= 0:
            reg = out[y0b:y1b, x0:x1].astype(np.float32)
            k = 0.55 * (1 - u)
            out[y0b:y1b, x0:x1] = (reg * (1 - k) + np.array([12, 16, 30.0]) * k).astype(np.uint8)
            ui._note(x0, y0b, x1, y1b, 'panel', 'corner label')
        sc = 1
        ui.label(out, name, x, y + (1 - pop) * 3, T.TEXT[q], sc)
        ui.label(out, sub, x + (len(name) + 1) * ADV, y + (1 - pop) * 3, (240, 230, 206), sc)
        if keep is not None:
            out[:] = (keep * (1 - a_in) + out * a_in).astype(np.uint8)
    caps = [(a, b, l) for a, b, l, hi, alt in T.CAPTIONS]
    for a, b, l, hi, alt in T.CAPTIONS:
        if a <= t < b:
            ui.draw_captions(out, t, [(a, b, l)], style='impact', hi=hi, alt=alt or ui.SKYB, puff=(232, 200, 140))
    return out
