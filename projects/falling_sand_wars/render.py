"""Falling Sand Wars renderer.

sky (night gradient, stars, low full moon) -> far mesas, dunes -> per-pixel desert floor (moon shadows of everything,
brazier pools, contact AO, wind ripples) -> pillars, lintel, hoppers, cradle, ring, braziers, palms -> the glass box:
per-pixel sand from the cached grid (tone jitter, lit tops, buried grains darker, knocked-out teams gone dark),
glass tint + fixed sheen, then the frame, gate and inlet valves -> the strata pile on the dune -> falling grains
(hopper streams, every grain that left the box) and dust -> locked DOF on the box -> glows, god rays, bloom
(sand excluded), grade -> share bar + impact captions.

    python -m reelkit.sheet render.py test/sheet.png 1 5 8 14 20.5 26 33 40 47 51 55 58.5
    python -m reelkit.video render.py audio.wav falling_sand_wars.mp4"""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import uniform_filter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, dirv, whole_period
from reelkit.camera import Cam
from reelkit.raster import Frame, painter, add_glow
from reelkit.lighting import Lights, shade, fog, sky_gradient
from reelkit.shadows import MaskGrid, directional_mask, point_mask, contact_ao
from reelkit.lens import coc_map, dof
from reelkit.post import god_rays, bloom, grade, vignette, top_scrim
from reelkit.geometry import Obj, mound, face, lathe, box as gbox
from reelkit import ui
import timeline as T
import scene as S

D = T.D
GRIDS = D['grids']
N = T.N
DUR = T.DUR

# ================================================================== camera + lights
FOV, DIST, SY = 36.0, 15.9, 250.0


def camera(t):
    yaw = 0.7 * math.sin(2 * math.pi * t / DUR + 0.4)
    pitch = 2.6 + 0.35 * math.sin(2 * math.pi * 2 * t / DUR + 1.1)
    return Cam(S.BC, DIST, yaw, pitch, FOV, SY)


MOON_DRAW = dirv(6.9, 3.0)              # low full moon over the far dunes, between the ring and the right pillar
KEY = dirv(15.0, 27.0)                  # shading direction of the moon (higher than drawn, same side)
KEY_COL = np.array([0.6, 0.66, 0.95])
WARM = np.array([1.15, 0.66, 0.3])


def flick(k, t):
    return 0.86 + 0.14 * math.sin(2 * math.pi * 47 * t / DUR + 1.7 * k) * math.sin(2 * math.pi * 29 * t / DUR + 0.6 + k)


def lights(t, cam):
    L = Lights(amb=(0.27, 0.27, 0.42), sky=(0.07, 0.08, 0.13))
    L.view = cam.view
    L.key('moon', KEY, KEY_COL)
    L.fill(dirv(180, 30), (0.17, 0.17, 0.26))
    for k, b in enumerate(S.BRAZIERS):
        L.point(b + [0, S.FLAME_Y, 0], WARM * 1.25 * flick(k, t), 2.9)
    return L


# ================================================================== static shadows
FLOOR = MaskGrid((-9.0, 9.0, -18.0, 7.0, 0.035))
MOON_MASK = directional_mask(FLOOR, S.CASTERS, KEY, blur=1.5)
BRZ_MASK = [point_mask(FLOOR, [Q for o in S.STATIC if o.cast and np.linalg.norm(o.ctr[[0, 2]] - b[[0, 2]]) > 0.6 for Q, n in o.faces],
                       b + [0, S.FLAME_Y, 0], blur=2.5) for b in S.BRAZIERS]
STATIC_ITEMS = [o for o in S.STATIC] + S.CRADLE


def rotz(P, th):
    c, s = math.cos(th), math.sin(th)
    P = np.asarray(P, float)
    out = P.copy()
    out[..., 0] = c * P[..., 0] - s * P[..., 1]
    out[..., 1] = s * P[..., 0] + c * P[..., 1]
    return out


def world_faces(faces, th):
    """box-local faces -> world (rotate about the box centre)"""
    return [(rotz(Q, th) + S.BC, rotz(n[None], th)[0]) for Q, n in faces]


# ================================================================== the falling grains (precomputed paths)
G_ACC = 7.5
EX_F, EX_S = D['ex_f'].astype(float), D['ex_s'].astype(float)
EX_TE = (EX_F + (EX_S + 1) / 3.0) / FPS
EX_M = D['ex_m'].astype(int) - 1
EX_R, EX_C = D['ex_r'].astype(float), D['ex_c'].astype(float)
EX_TONE = D['ex_t'].astype(float) / 63.0
_th = np.radians(np.array([T.theta(t) for t in EX_TE])) if len(EX_TE) else np.zeros(0)
_lx = ((EX_C - 0.5) / N) * 2 - 1
_ly = 1 - ((EX_R - 0.5) / N) * 2
EX_X0 = S.BC[0] + np.cos(_th) * _lx * S.HALF - np.sin(_th) * _ly * S.HALF
EX_Y0 = S.BC[1] + np.sin(_th) * _lx * S.HALF + np.cos(_th) * _ly * S.HALF
_nx = np.where(EX_C >= N + 1, 1.0, np.where(EX_C <= 0, -1.0, 0.0))
_ny = np.where(EX_R >= N + 1, -1.0, np.where(EX_R <= 0, 1.0, 0.0))
_rng = np.random.default_rng(5)
EX_VX = (np.cos(_th) * _nx - np.sin(_th) * _ny) * 0.45 + _rng.normal(0, 0.12, len(EX_TE))
EX_VY = (np.sin(_th) * _nx + np.cos(_th) * _ny) * 0.45 - 0.6 + _rng.normal(0, 0.1, len(EX_TE))
EX_Z = 0.16 + _rng.uniform(0, 0.12, len(EX_TE))

# the pile sits where the grains land
PILE_Z = 0.42
PILE_H, PILE_R = 0.5, 1.0
_land_x = EX_X0 + EX_VX * 0.8
PILE_X = float(np.clip(np.median(_land_x), -0.9, 0.9)) if len(_land_x) else 0.0


PILE_RINGS = 9


def pile_dome(h, r):
    """a low-poly dome built ring by ring, so every ring is one stratum of the drained sand"""
    ys = np.linspace(0, h, PILE_RINGS + 1)
    prof = [(float(y), float(r * max(0.0, 1 - (y / h)) ** 0.62)) for y in ys]
    prof[-1] = (float(h), 0.0)
    faces = lathe(PILE_X, PILE_Z, prof, n=14, rot=0.2)
    return [(Q * [1, 1, 1] + [0, 0, 0], n) for Q, n in faces]


def pile_shape(t):
    v = T.pile_volume(t)
    k = max(v, 0.0) ** (1 / 3)
    return PILE_H * k, PILE_R * k


def pile_height_at(x, z, h, r):
    h = np.asarray(h, float)
    r = np.maximum(np.asarray(r, float), 0.05)
    u = (x - PILE_X) / r
    w = (z - PILE_Z) / (r * 0.7)
    return np.where(h > 0.01, h * np.clip(1 - u * u - w * w, 0, 1) ** 0.8, 0.0)


# landing time of every grain (on the pile as it will be about then, or the dune floor)
_hp = np.array([pile_shape(t + 0.5)[0] for t in EX_TE]) if len(EX_TE) else np.zeros(0)
_rp = np.array([pile_shape(t + 0.5)[1] for t in EX_TE]) if len(EX_TE) else np.zeros(0)
_u = np.linspace(0, 2.5, 51)
EX_TL = np.full(len(EX_TE), 2.0)
for i0 in range(0, len(EX_TE), 4000):
    sl = slice(i0, i0 + 4000)
    X = EX_X0[sl, None] + EX_VX[sl, None] * _u[None]
    Y = EX_Y0[sl, None] + EX_VY[sl, None] * _u[None] - 0.5 * G_ACC * _u[None] ** 2
    hh = np.stack([pile_height_at(X[:, j], EX_Z[sl], _hp[sl], np.maximum(_rp[sl], 0.05)) for j in range(len(_u))], 1)
    below = Y <= hh
    first = np.where(below.any(1), below.argmax(1), len(_u) - 1)
    EX_TL[sl] = _u[first]
EX_TL = EX_TE + EX_TL

# dust motes drifting on the wind (whole wraps over the video), and brazier sparks
_dr = np.random.default_rng(8)
NDUST = 34
DUST = [(_dr.uniform(-5, 5), _dr.uniform(0.2, 6.5), _dr.uniform(-2.5, 3.0), int(_dr.integers(1, 4)), _dr.uniform(0, 6.3),
         _dr.uniform(0.35, 0.9)) for _ in range(NDUST)]
SPARKS = [(k, _dr.uniform(0, 1), _dr.uniform(-0.12, 0.12), int(_dr.integers(9, 15))) for k in range(2) for _ in range(9)]


# ================================================================== sky + far layers
STAR_DIRS = []
_sr = np.random.default_rng(13)
for _ in range(420):
    az, el = _sr.uniform(-25, 25), _sr.uniform(1.5, 26)
    STAR_DIRS.append((dirv(az, el), _sr.uniform(0.2, 1.0) ** 2, _sr.uniform(0, 6.3), int(_sr.integers(5, 23))))


def sky(cam, t):
    img = sky_gradient(cam, (10, 12, 44), (66, 62, 122), horizon2=(34, 38, 96), glow_dir=MOON_DRAW,
                       glow_col=(120, 130, 200), glow_k=0.22)
    for d, b, ph, k in STAR_DIRS:
        s = cam.screen_of_dir(d)
        if s is None:
            continue
        x, y = int(s[0]), int(s[1])
        if 0 <= x < W and 0 <= y < H:
            bb = b * (0.7 + 0.3 * math.sin(2 * math.pi * k * t / DUR + ph))
            img[y, x] = np.maximum(img[y, x], 110 + 140 * bb)
    return img


def draw_moon(fr, cam):
    ms = cam.screen_of_dir(MOON_DRAW)
    if ms is None:
        return None
    r = 0.017 * cam.f
    fr.disc(ms, r, (248, 244, 226), n=22)
    for dx, dy, rr in [(-0.3, -0.2, 0.28), (0.25, 0.15, 0.2), (0.05, 0.4, 0.15), (-0.35, 0.35, 0.12)]:
        fr.disc((ms[0] + dx * r, ms[1] + dy * r), rr * r, (226, 222, 210), n=10)
    fr.glow(ms[0], ms[1], 0.06 * cam.f, (150, 155, 230), 0.35)
    return ms


# ================================================================== frame
def render(f):
    t = f / FPS
    th_deg = T.theta(t)
    th = math.radians(th_deg)
    cam = camera(t)
    L = lights(t, cam)
    fr = Frame(sky(cam, t))
    ms = draw_moon(fr, cam)
    for Q, n in S.FAR:
        c = fog(np.array([70, 56, 86.0]), 60.0, (86, 82, 140), 0, 80, 0.55)
        fr.poly(cam, Q, c, depth=False)
    for Q, n in S.FAR2:
        c = fog(np.array([96, 70, 84.0]), 36.0, (86, 82, 140), 0, 80, 0.4)
        fr.poly(cam, Q, c, depth=False)
    for o in sorted(S.DUNES, key=lambda o: -float(cam.depth(o.ctr)[0])):
        _draw_obj(fr, cam, L, o, kv_d=None, fogd=True)

    # dynamic casters for the floor: ring + spokes, the box (frame + its glass slab), the pile
    dyn = [Q for Q, n in world_faces(S.RING + S.SPOKES, th)]
    sq = np.array([[-1, -1, 0], [1, -1, 0], [1, 1, 0], [-1, 1, 0]], float) * (S.HALF + S.FR)
    dyn.append(rotz(sq, th) + S.BC)
    ph, pr = pile_shape(t)
    pile_faces = pile_dome(ph, pr) if ph > 0.02 else []
    dyn += [Q for Q, n in pile_faces]
    dyn_mask = directional_mask(FLOOR, dyn, KEY, blur=1.2)
    _floor(fr, cam, L, t, dyn_mask, ph, pr)

    # ---- painter list
    items = []
    for o in STATIC_ITEMS:
        items.append((float(cam.depth(o.ctr)[0]), 'obj', o))
    for k, hp in enumerate(S.HOPPERS):
        items.append((float(cam.depth([hp['x'], hp['ny'] + 0.3, 0.0])[0]), 'hopper', k))
    items.append((float(cam.depth(S.BC + [0, 0, S.RZ0])[0]), 'ring', None))
    items.append((float(cam.depth(S.BC)[0]), 'box', None))
    if pile_faces:
        items.append((float(cam.depth([PILE_X, ph * 0.3, PILE_Z])[0]), 'pile', (pile_faces, ph)))
    for o in S.FORE:
        items.append((float(cam.depth(o.ctr)[0]), 'fore', o))
    sand_mask = np.zeros((H, W), bool)
    for _, kind, it in painter(items):
        if kind == 'obj':
            _draw_obj(fr, cam, L, it)
        elif kind == 'fore':
            _draw_obj(fr, cam, L, it, kv_d=None)
        elif kind == 'hopper':
            _draw_hopper(fr, cam, L, t, it)
        elif kind == 'ring':
            _draw_ring(fr, cam, L, th)
            for a, b in S.CHAINS:
                _chain(fr, cam, a, b)
        elif kind == 'box':
            sand_mask = _draw_box(fr, cam, L, t, f, th)
        elif kind == 'pile':
            _draw_pile(fr, cam, L, *it)
    _streams(fr, cam, t)
    _grains(fr, cam, t)
    _braziers_fx(fr, cam, t)
    _dust(fr, cam, t, ph, pr)

    # ---- lens: focus locked on the box
    img, depth = fr.array(), fr.depth()
    zf = cam.depth(S.BC)[0]
    img = dof(img, coc_map(depth, zf, 7.0, sharp=focus_band(cam)))
    for g in fr.glows:
        add_glow(img, *g)
    if ms is not None:
        img = god_rays(img, depth, ms, (150, 150, 220), 0.14)
    img = grade(bloom(np.clip(img, 0, 255), thr=212, k=0.32, exclude=sand_mask), contrast=0.14, sat=1.12)
    img = np.clip(img * vignette(strength=0.7, cap=0.32) * top_scrim(), 0, 255).astype(np.uint8)
    out = img.copy()
    _ui(out, t)
    return out


# ------------------------------------------------------------------ focus
_SUBJ = np.concatenate([np.array([S.BC + [S.R_OUT * math.cos(a), S.R_OUT * math.sin(a), S.RZ0] for a in np.linspace(0, 6.28, 24)]),
                        np.array([S.BC + [S.R_SWEEP * math.cos(a), S.R_SWEEP * math.sin(a), S.ZF + 0.04] for a in np.linspace(0, 6.28, 24)]),
                        np.array([[hp['x'], hp['ny'] + 0.6, 0.0] for hp in S.HOPPERS]),
                        np.array([[PILE_X - 1.6, 0.0, PILE_Z + 1.2], [PILE_X + 1.6, 0.0, PILE_Z + 1.2], [PILE_X, 1.0, PILE_Z]])])


def focus_band(cam):
    zf = cam.depth(S.BC)[0]
    z = cam.depth(_SUBJ)
    return max(0.05, float(np.abs(1 - zf / z).max()) + 0.01)


# ------------------------------------------------------------------ objects
def _draw_obj(fr, cam, L, o, kv_d=KEY, fogd=False, emit=0.0, faces=None, base=None):
    faces = o.faces if faces is None else faces
    fc = [(cam.depth(Q.mean(0))[0], Q, n, j) for j, (Q, n) in enumerate(faces) if np.dot(n, cam.pos - Q.mean(0)) > 0]
    if not fc:
        return
    fc.sort(key=lambda a: -a[0])
    C = np.array([q.mean(0) for _, q, _, _ in fc])
    Nn = np.array([n for _, _, n, _ in fc])
    if kv_d is not None:
        kv = np.array([1.0 - _ground_shadow(c) if c[1] < 0.6 else 1.0 for c in C])[:, None]
    else:
        kv = None
    bc = np.array([(o.color if base is None else base) * o.tone[j % len(o.tone)] for *_, j in fc])
    cols = shade(L, Nn, C, bc, kvis=kv, emit=emit)
    if fogd:
        dist = np.array([z for z, *_ in fc])
        cols = fog(cols, dist, np.array([86, 82, 140.0]), 14, 40, 0.5)
    for (z, Q, n, j), c in zip(fc, cols):
        fr.poly(cam, Q, c)


def _ground_shadow(c):
    return float(FLOOR.sample(MOON_MASK, np.array([c[0] - c[1] / KEY[1] * KEY[0]]), np.array([c[2] - c[1] / KEY[1] * KEY[2]]))[0])


def _floor(fr, cam, L, t, dyn_mask, ph, pr):
    arr, depth = fr.array(), fr.depth()
    ys, xs = np.nonzero(YY > 150)
    X, Z, d = cam.ground_hit(xs.astype(float), ys.astype(float))
    ok = (d > 0) & (d < depth[ys, xs]) & (Z > -30)
    xs, ys, X, Z, d = xs[ok], ys[ok], X[ok], Z[ok], d[ok]
    rip = 0.93 + 0.07 * np.sin(Z * 9.0 + np.sin(X * 1.7) * 1.4 + X * 0.6)
    spk = (BAYER_FULL[ys, xs] > 0.88) * 0.05
    alb = np.array(S.SAND_FLOOR, float)[None] * (rip + spk)[:, None]
    I = np.tile(L.amb + L.sky, (len(X), 1))
    lit = (1 - FLOOR.sample(MOON_MASK, X, Z)) * (1 - FLOOR.sample(dyn_mask, X, Z))
    I += (lit * KEY[1] * 1.15)[:, None] * KEY_COL[None]
    for k, b in enumerate(S.BRAZIERS):
        lp = b + [0, S.FLAME_Y, 0]
        dist = np.sqrt((lp[0] - X) ** 2 + lp[1] ** 2 + (lp[2] - Z) ** 2)
        I += (lp[1] / dist * (1 - FLOOR.sample(BRZ_MASK[k], X, Z)) * 2.4 * flick(k, t) / (1 + (dist / 1.9) ** 2))[:, None] * WARM[None]
    feet = S.FEET + ([(PILE_X, PILE_Z, pr * 0.85, 0.25)] if ph > 0.05 else [])
    I *= contact_ao(X, Z, feet)[:, None]
    fogk = np.clip((d - 18) / 30, 0, 0.5)[:, None]
    col = alb * I * (1 - fogk) + np.array([86, 82, 140.0]) * fogk
    arr[ys, xs] = np.floor(col / 5 + BAYER_FULL[ys, xs, None]) * 5
    depth[ys, xs] = d
    fr.set_array(arr, depth)


def _draw_hopper(fr, cam, L, t, k):
    hp = S.HOPPERS[k]
    q = T.ORDER[k]
    o = Obj(hp['body'], (120, 104, 92), 'hopper', grp=hp['grp'])
    _draw_obj(fr, cam, L, o, kv_d=None)
    ob = Obj(hp['band'], tuple(T.SAND[q] * 0.9 + 20), 'band', grp=hp['grp'])
    _draw_obj(fr, cam, L, ob, kv_d=None, emit=0.15)
    orod = Obj(hp['rod'], (70, 62, 58), 'rod', grp=hp['grp'])
    _draw_obj(fr, cam, L, orod, kv_d=None)
    # angled spout from the nozzle to the shared inlet above the box centre
    a = np.array([hp['x'], hp['ny'], 0.0])
    b = np.array([0.0, S.BC[1] + S.R_SWEEP + 0.02, 0.0])
    sa, sb = cam.wp(a), cam.wp(b)
    fr.draw.line([tuple(sa), tuple(sb)], fill=(54, 48, 46), width=3)
    fr.draw.line([tuple(sa), tuple(sb)], fill=(118, 104, 92), width=1)


def _chain(fr, cam, a, b):
    n = 16
    for i in range(n):
        p = a + (b - a) * (i + 0.5) / n
        s = cam.wp(p)
        r = 1.2
        col = (150, 130, 96) if i % 2 else (104, 92, 74)
        fr.draw.ellipse([s[0] - r, s[1] - r * 1.6, s[0] + r, s[1] + r * 1.6], outline=col)


def _draw_ring(fr, cam, L, th):
    o = Obj(world_faces(S.RING, th), (196, 150, 72), 'ring', jitter=0.03)
    _draw_obj(fr, cam, L, o, kv_d=None)
    ot = Obj(world_faces(S.TICKS, th), (250, 226, 160), 'tick', jitter=0.0)
    _draw_obj(fr, cam, L, ot, kv_d=None, emit=0.2)
    osp = Obj(world_faces(S.SPOKES, th), (120, 96, 60), 'spoke', jitter=0.0)
    _draw_obj(fr, cam, L, osp, kv_d=None)


# ------------------------------------------------------------------ the glass box: per-pixel sand
CELL_UP = {}


def _cell_colors(f, t, th_deg):
    g = GRIDS[f]
    m = (g >> 5).astype(int)
    tone = (g & 31).astype(float) / 31.0
    sand = m > 0
    base = np.zeros((N, N, 3))
    for q in range(4):
        sel = m == q + 1
        if not sel.any():
            continue
        if T.out_now(q, t):
            dead = T.OUT_T[q]
            k = min(1.0, (t - dead) / 0.35)
            c = T.SAND[q] * (1 - k) + np.array([78, 72, 76.0]) * k
        else:
            c = T.SAND[q]
        base[sel] = c
    base *= (0.84 + 0.3 * tone)[..., None]
    # lit tops: the cell above (in the world) is empty
    a = math.radians(th_deg)
    dc, dr = int(round(math.sin(a))), int(round(-math.cos(a)))
    pad = np.pad(sand, 1)
    up = pad[1 + dr:N + 1 + dr, 1 + dc:N + 1 + dc]
    top = sand & ~up
    occ = uniform_filter(sand.astype(float), 9, mode='constant')
    ao = 1 - 0.3 * np.clip((occ - 0.55) / 0.45, 0, 1)
    col = base * ao[..., None]
    col[top] = col[top] * 1.12 + np.array([34, 38, 52.0])
    return m, col


def _draw_box(fr, cam, L, t, f, th):
    """back glass + sand per pixel + glass tint/sheen, then the frame, the gate and the inlet valves on top"""
    m, ccol = _cell_colors(f, t, math.degrees(th))
    r_scr = S.R_SWEEP * cam.f / cam.depth(S.BC)[0] + 4
    c0 = cam.wp(S.BC + [0, 0, S.ZF])
    x0, x1 = int(max(0, c0[0] - r_scr)), int(min(W, c0[0] + r_scr))
    y0, y1 = int(max(0, c0[1] - r_scr)), int(min(H, c0[1] + r_scr))
    ys, xs = np.mgrid[y0:y1, x0:x1]
    ys, xs = ys.ravel().astype(float), xs.ravel().astype(float)
    dw = cam.pixel_rays(xs, ys)
    tt = (S.ZF - cam.pos[2]) / dw[:, 2]
    P = cam.pos[None] + tt[:, None] * dw
    X, Y = P[:, 0] - S.BC[0], P[:, 1] - S.BC[1]
    c, s = math.cos(th), math.sin(th)
    lx, ly = c * X + s * Y, -s * X + c * Y
    inside = (np.abs(lx) < S.HALF) & (np.abs(ly) < S.HALF)
    xs, ys, lx, ly, tt, P = xs[inside], ys[inside], lx[inside], ly[inside], tt[inside], P[inside]
    col_i = np.clip(((lx + S.HALF) / (2 * S.HALF) * N).astype(int), 0, N - 1)
    row_i = np.clip(((S.HALF - ly) / (2 * S.HALF) * N).astype(int), 0, N - 1)
    arr, depth = fr.array(), fr.depth()
    yi, xi = ys.astype(int), xs.astype(int)
    mm = m[row_i, col_i]
    sand = mm > 0
    # light on the box face: ambient + front fill + braziers (falloff in the world)
    Lp = np.tile(L.amb + L.fills[0][1] * 0.75, (len(xs), 1))
    for k, b in enumerate(S.BRAZIERS):
        lp = b + [0, S.FLAME_Y, 0]
        v = lp[None] - P
        dist = np.linalg.norm(v, axis=1)
        lam = 0.35 + 0.65 * np.clip(v[:, 2] / dist, 0, 1)
        Lp += (lam / (1 + (dist / 2.9) ** 2) * 1.25 * flick(k, t))[:, None] * WARM[None]
    sc = ccol[row_i, col_i] * Lp
    mx = sc.max(1, keepdims=True)
    sc = np.where(mx > 250, sc * 250 / np.maximum(mx, 1), sc)
    bg = arr[yi, xi] * np.array([0.72, 0.8, 0.94]) + np.array([8, 10, 18.0])
    sheen = ((((xs * 0.8 + ys) % 97) < 9) * 16 + (((xs * 0.8 + ys + 30) % 97) < 3) * 10)[:, None]
    out = np.where(sand[:, None], sc, bg + sheen)
    arr[yi, xi] = np.floor(out / 4 + BAYER_FULL[yi, xi, None] * 0.999) * 4
    depth[yi, xi] = tt
    fr.set_array(arr, depth)
    sand_mask = np.zeros((H, W), bool)
    sand_mask[yi[sand], xi[sand]] = True
    # frame bars + caps
    for faces, nm in S.FRAME:
        o = Obj(world_faces(faces, th), (150, 112, 64) if nm == 'bar' else (206, 168, 92), nm, jitter=0.02)
        _draw_obj(fr, cam, L, o, kv_d=None)
    _gate(fr, cam, L, t, th)
    _valves(fr, cam, L, t, th)
    return sand_mask


GATE_EVENTS = []          # (t, opening?) from the run's hole state
_h = D['hole'].astype(bool)
for _f in range(1, len(_h)):
    if _h[_f] != _h[_f - 1]:
        GATE_EVENTS.append((_f / FPS, bool(_h[_f])))


def gate_open(t):
    """0 shut .. 1 open, with a quick snap"""
    state, tlast = False, -9.0
    for te, op in GATE_EVENTS:
        if t >= te:
            state, tlast = op, te
    u = min(1.0, (t - tlast) / 0.09)
    return u if state else 1 - u


import episode as E


def gate_span(t):
    """height of the opening up the side wall (box-local units): the notch, or the whole wall in the closing pour"""
    up0 = max(4, int(round(E.PARAMS.hole_w * N)))
    up = up0
    if E.T_ROLL1 <= t < E.T_REPLUG:
        up = up0 + int((N - up0) * E.ease((t - E.T_ROLL1) / E.T_GATE))
    return up / N * 2 * S.HALF


def _gate(fr, cam, L, t, th):
    """a brass plate over the corner notch, sliding up the side wall to open (all the way in the closing pour)"""
    side = 1.0 if D['hole_side'] == 'R' else -1.0
    hu = gate_span(t)
    hn = gate_span(0.0)                               # the plate covers the notch
    op = gate_open(t)
    x = side * (S.HALF + S.FR + 0.012)
    yb = min(-S.HALF - S.FR + op * (hu + 0.14), S.HALF - hn - 0.06)
    faces = gbox(x, yb + hn / 2 + 0.06, S.ZF + 0.03, 0.02, hn / 2 + 0.06, 0.03)
    o = Obj(world_faces(faces, th), (226, 186, 96), 'gate', jitter=0.0)
    _draw_obj(fr, cam, L, o, kv_d=None, emit=0.1)
    if op > 0.02:                                   # dark notch showing
        nc = np.array([[side * S.HALF, -S.HALF - S.FR, S.ZF + 0.005], [side * (S.HALF + S.FR), -S.HALF - S.FR, S.ZF + 0.005],
                       [side * (S.HALF + S.FR), -S.HALF + hu * op, S.ZF + 0.005], [side * S.HALF, -S.HALF + hu * op, S.ZF + 0.005]])
        fr.poly(cam, rotz(nc, th) + S.BC, (24, 18, 22))
    for te, op_ in GATE_EVENTS:                    # a pop of light on every snap
        if 0 <= t - te < 0.3:
            p = rotz(np.array([[side * (S.HALF + S.FR), -S.HALF, S.ZF]]), th)[0] + S.BC
            sp = cam.wp(p)
            fr.glow(sp[0], sp[1], 8 + 20 * (t - te), (255, 220, 150), 0.7 * (1 - (t - te) / 0.3))


def _valves(fr, cam, L, t, th):
    """the shared inlet on the top bar: open (dark slot) while the hoppers pour, shut brass cap after"""
    opn = t < T.T_VALVE_SHUT
    w = 0.09
    y = S.HALF + S.FR + 0.004
    if opn:
        q = np.array([[-w, y, S.ZF + 0.02], [w, y, S.ZF + 0.02], [w, y - S.FR * 0.9, S.ZF + 0.02], [-w, y - S.FR * 0.9, S.ZF + 0.02]])
        fr.poly(cam, rotz(q, th) + S.BC, (30, 24, 26))
    else:
        o = Obj(world_faces(gbox(0, y + 0.02, 0, w + 0.03, 0.025, S.ZF + 0.03), th), (222, 180, 92), 'valve', jitter=0.0)
        _draw_obj(fr, cam, L, o, kv_d=None)


# ------------------------------------------------------------------ the strata pile
def _draw_pile(fr, cam, L, faces, h):
    v = max(T.pile_volume(T_now[0]), 1e-6)
    cols = []
    for Q, n in faces:
        rho = float(np.clip(Q[:, 1].mean() / max(h, 1e-3), 0, 0.999))
        frac = (1 - (1 - rho) ** 3) * v                   # share of everything drained that lies below this height
        k = min(T.NBANDS - 1, int(frac * T.NBANDS))
        b = T.BANDS[min(T.NBANDS - 1, int(frac * T.NBANDS))]
        cols.append(T.SAND[int(np.argmax(b))] * 0.8 + (b @ T.SAND) * 0.2)       # crisp strata, a hint of the mix
    o = Obj(faces, (200, 160, 120), 'pile', jitter=0.05)
    fc = [(cam.depth(Q.mean(0))[0], Q, n, j) for j, (Q, n) in enumerate(faces) if np.dot(n, cam.pos - Q.mean(0)) > 0]
    fc.sort(key=lambda a: -a[0])
    if not fc:
        return
    C = np.array([q.mean(0) for _, q, _, _ in fc])
    Nn = np.array([nn for _, _, nn, _ in fc])
    sc = shade(L, Nn, C, np.array([cols[j] * o.tone[j] * 0.7 for *_, j in fc]))
    for (z, Q, nn, j), cc in zip(fc, sc):
        fr.poly(cam, Q, cc)


T_now = [0.0]


# ------------------------------------------------------------------ particles
def _put(fr, xs, ys, cols, z, size=1):
    arr = np.asarray(fr.pil)
    a = arr.copy()
    dep = np.asarray(fr.dep).copy()
    xi, yi = np.round(xs).astype(int), np.round(ys).astype(int)
    ok = (xi >= 0) & (xi < W - size) & (yi >= 0) & (yi < H - size)
    xi, yi, cols, z = xi[ok], yi[ok], cols[ok], np.broadcast_to(z, ok.shape)[ok]
    vis = z < dep[yi, xi] + 0.05
    xi, yi, cols, z = xi[vis], yi[vis], cols[vis], z[vis]
    for dy in range(size):
        for dx in range(size):
            a[yi + dy, xi + dx] = np.clip(cols, 0, 255).astype(np.uint8)
            dep[yi + dy, xi + dx] = np.minimum(dep[yi + dy, xi + dx], z)
    fr.set_array(a.astype(np.float32), dep)


def _streams(fr, cam, t):
    """the four hopper streams down the spouts' shared nozzle into the inlet (0-19 s), drawn from the pour rates"""
    f = T.frame_of(t)
    if t >= T.T_VALVE_SHUT + 0.35:
        return
    top = np.array([0.0, S.BC[1] + S.R_SWEEP + 0.02, 0.0])
    bot = np.array([0.0, S.BC[1] + S.HALF + S.FR, 0.0])
    xs, ys, cols = [], [], []
    rng = np.random.default_rng(f)
    for h in range(4):
        rate = D['rates'][h, min(f, D['rates'].shape[1] - 1)] if t < T.T_VALVE_SHUT else 0.0
        tail = max(0.0, 1 - (t - T.T_VALVE_SHUT) / 0.35) if t >= T.T_VALVE_SHUT else 1.0
        n = int(np.clip(rate * 4 + 2, 0, 26) * tail) if t < T.T_VALVE_SHUT else int(6 * tail)
        q = T.ORDER[h]
        for k in range(n):
            u = (t * 2.3 + k / max(n, 1) + h * 0.13) % 1.0
            p = top + (bot - top) * u + [rng.normal(0, 0.012) + (h - 1.5) * 0.008, 0, 0.13]
            s = cam.wp(p)
            xs.append(s[0]); ys.append(s[1]); cols.append(T.SAND[q] * 1.1 + 18)
    if xs:
        _put(fr, np.array(xs), np.array(ys), np.array(cols), float(cam.depth(bot + [0, 0, 0.13])[0]))


def _grains(fr, cam, t):
    act = (EX_TE <= t) & (EX_TL > t)
    if not act.any():
        return
    u = t - EX_TE[act]
    X = EX_X0[act] + EX_VX[act] * u
    Y = EX_Y0[act] + EX_VY[act] * u - 0.5 * G_ACC * u * u
    Z = EX_Z[act]
    P = np.stack([X, Y, Z], 1)
    s = cam.wp(P)
    m = EX_M[act]
    tone = EX_TONE[act]
    col = T.SAND[m] * (0.9 + 0.3 * tone[:, None]) * 1.05 + 10
    for q in range(4):                        # a knocked-out team's grains are dark
        if T.out_now(q, t):
            col[m == q] = np.array([90, 84, 88.0])
    z = cam.depth(P)
    _put(fr, s[:, 0], s[:, 1], col, z)
    fast = (EX_VY[act] - G_ACC * u) < -2.6               # a short streak behind fast grains
    if fast.any():
        _put(fr, s[fast, 0], s[fast, 1] - 1, col[fast] * 0.8, z[fast])


def _braziers_fx(fr, cam, t):
    for k, b in enumerate(S.BRAZIERS):
        fl = flick(k, t)
        base = b + [0, 1.38, 0]
        sb = cam.wp(base)
        sc = cam.f / cam.depth(base)[0]
        for j in range(5):
            ph = 2 * math.pi * (13 + 4 * j) * t / DUR + j * 1.3 + k
            hgt = (0.28 + 0.12 * math.sin(ph)) * sc * (0.7 + 0.3 * fl)
            wdt = (0.09 - 0.012 * j) * sc
            off = (j - 2) * 0.05 * sc + 0.02 * sc * math.sin(ph * 1.7)
            col = [(255, 120, 40), (255, 160, 60), (255, 200, 90), (255, 236, 160), (255, 150, 50)][j]
            pts = [(sb[0] + off - wdt, sb[1]), (sb[0] + off + wdt, sb[1]), (sb[0] + off + 0.3 * wdt * math.sin(ph), sb[1] - hgt)]
            fr.draw.polygon(pts, fill=col)
        sf = cam.wp(b + [0, S.FLAME_Y, 0])
        fr.glow(sf[0], sf[1], 0.55 * sc, (255, 150, 60), 0.5 * fl)
        fr.glow(sf[0], sf[1] + 4, 0.22 * sc, (255, 220, 140), 0.55 * fl)
    for k, ph0, dx, cyc in SPARKS:
        b = S.BRAZIERS[k]
        u = (cyc * t / DUR + ph0) % 1.0
        p = b + [dx + 0.15 * math.sin(u * 9 + ph0 * 6), 1.45 + u * 1.9, 0.0]
        s = cam.wp(p)
        fr.glow(s[0], s[1], 1.5, (255, 190, 90), 0.8 * (1 - u))


def _dust(fr, cam, t, ph, pr):
    xs, ys, cols, zs = [], [], [], []
    for x0, y0, z0, kk, ph0, b in DUST:
        x = (x0 + 10.0 * kk * t / DUR + 5) % 10.0 - 5
        y = y0 + 0.15 * math.sin(2 * math.pi * kk * 2 * t / DUR + ph0)
        p = np.array([x, y, z0])
        s = cam.wp(p)
        xs.append(s[0]); ys.append(s[1]); zs.append(float(cam.depth(p)[0]))
        cols.append(np.array([230, 210, 180.0]) * b + 20)
    # wind blowing last run's pile away during the pour
    if T.T_ERODE0 < t < T.T_ERODE1 + 0.5 and ph > 0.05:
        rng = np.random.default_rng(1234)
        for k in range(40):
            u = (t * 0.9 + k / 40) % 1.0
            p = np.array([PILE_X + rng.uniform(-0.5, 0.5) * pr + u * 2.6, ph * (0.6 + 0.4 * rng.random()) + u * 0.5 - u * u * 0.6,
                          PILE_Z + rng.uniform(-0.2, 0.3)])
            s = cam.wp(p)
            xs.append(s[0]); ys.append(s[1]); zs.append(float(cam.depth(p)[0]))
            c = T.BANDS[min(T.NBANDS - 1, int(rng.random() * T.NBANDS * T.pile_volume(t)))] @ T.SAND
            cols.append(c * (1 - u) + np.array([200, 180, 160.0]) * u)
    if xs:
        _put(fr, np.array(xs), np.array(ys), np.array(cols), np.array(zs))


# ------------------------------------------------------------------ UI
def _ui(out, t):
    a = T.bar_alpha(t)
    if a > 0.01:
        sh = T.shares(t)
        outs = [T.out_now(q, t) for q in range(4)]
        ui.share_bar(out, sh, T.NAMES, T.BAR_COL, out=outs, alpha=a)
    for t0, t1, lines, hi, alt in T.CAPTIONS:
        if t0 <= t < t1:
            ui.draw_captions(out, t, [(t0, t1, lines)], hi=hi, alt=alt if alt else ui.SKYB, style='impact')


_render = render


def render(f):          # noqa: F811  (keeps the pile's clock in step with the frame)
    T_now[0] = f / FPS
    return _render(f)
