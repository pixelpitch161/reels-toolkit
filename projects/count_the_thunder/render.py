"""Count the Thunder - renderer.

    python -m reelkit.sheet render.py test/sheet.png 0.6 6.5 10.6 19.6 26.1 33 41 50.05 57
    python -m reelkit.video render.py audio.wav count_the_thunder.mp4

Per frame: storm sky (moon, stars) -> far hills -> per-pixel prairie (moon + lightning shadows from
cached ground masks, the expanding sound ring of every strike) -> storm cells -> rain curtain ->
outdoor objects -> rain -> ray-traced cabin shell (depth-tested: plank floor with the moon pool and
flash-lit window-frame shadows, log walls, the window seen through to the storm) -> cut-away edges
-> indoor objects (lamp, fire, cat) -> locked depth of field -> lightning bolts (depth-tested) ->
glows -> window light shafts -> bloom -> grade -> vignette/scrim -> labels, HUD, captions.

Photosensitivity: one luminance pulse per strike (no restrike flicker), <= 0.5 flashes/s, and the
flash lights the sky, window, floor patch and prairie while the warm lamps keep the room lit, so no
frame goes white. flash_check.py measures it."""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, ss, ease, nrm, mixc, dirv
from reelkit.camera import Cam, interp_keys
from reelkit.raster import Frame, painter, apply_glows, add_glow
from reelkit.lighting import Lights, shade, sky_gradient
from reelkit.shadows import MaskGrid, directional_mask, point_mask, Occluders, contact_ao
from reelkit.lens import coc_map, dof
from reelkit.post import bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, panel, slide_offset, MINT, CREAM, AMBER, CORAL, SKYB, CYAN
from reelkit.font import text_mask, stamp, GLYPHS
from reelkit.geometry import Obj, box, prism, face, icosa
import timeline as T
import scene as S

# runtime glyph (project-local, like the lowest-note project)
GLYPHS['≈'] = np.array([[c == '#' for c in row] for row in ".....|.##.#|#..#.|.....|.##.#|#..#.|.....".split('|')])

# ---------------------------------------------------------------- lights
MOON_L = dirv(5, 30)                    # light direction (moon pool lands on the floor)
MOON_SKY = dirv(2.5, 5.8)               # where the disc is painted (low over the barn, visible in frame)
MOON_RGB = np.array([0.62, 0.72, 1.0])
FLASH_RGB = np.array([0.86, 0.88, 1.0])
WARM = np.array([1.0, 0.64, 0.32])
FIRE_RGB = np.array([1.0, 0.5, 0.2])


def flash_dir(i):
    """direction from the window toward the bright part of strike i (clamped so the pool hits the floor)"""
    main, _ = S.BOLTS[i]
    mid = main[len(main) // 3]
    v = mid - S.WINDOW_C
    hz = math.hypot(v[0], v[2])
    el = min(max(math.atan2(v[1], hz), math.radians(26)), math.radians(40))
    az = math.atan2(v[0], -v[2])
    az = min(max(az, math.radians(-25)), math.radians(25))
    return np.array([math.sin(az) * math.cos(el), math.sin(el), -math.cos(az) * math.cos(el)])


FLASH_D = [flash_dir(i) for i in range(T.N_STRIKES)]

# ---------------------------------------------------------------- shadow casters + cached masks
WALL_CASTERS = []                       # back wall around the window hole + left wall (for outdoor shadows)
for (x0, x1, y0, y1) in ((S.RX0 - S.WT, S.WX0, 0, S.WALL_H), (S.WX1, S.RX1, 0, S.WALL_H), (S.WX0, S.WX1, 0, S.WY0), (S.WX0, S.WX1, S.WY1, S.WALL_H)):
    WALL_CASTERS.append(Obj(box(0.5 * (x0 + x1), 0.5 * (y0 + y1), S.RZ0 - S.WT / 2, 0.5 * (x1 - x0), 0.5 * (y1 - y0), S.WT / 2), (0, 0, 0), 'wall', grp=-50))
WALL_CASTERS.append(Obj(box(S.RX0 - S.WT / 2, S.WALL_H / 2, 0, S.WT / 2, S.WALL_H / 2, S.RZ1), (0, 0, 0), 'wall', grp=-51))
OCC = Occluders(S.OBJS_IN + S.OBJS_OUT + WALL_CASTERS)

OUTG = MaskGrid((-16.0, 16.0, -34.0, 8.0, 0.05))
_lift = np.array([0, -S.GROUND_Y, 0])
OUT_FACES = [Q + _lift for o in S.OBJS_OUT + WALL_CASTERS if o.cast for Q, n in o.faces] + \
            [Q + _lift for Q, n in S.windmill_fan(0.0)]
OUT_MOON = directional_mask(OUTG, OUT_FACES, MOON_L, blur=1.5)


def _out_flash_dir(i):
    """direction from the prairie toward strike i's channel (for ground shadows); clamped elevation"""
    main, _ = S.BOLTS[i]
    p = main[len(main) // 2]
    v = p - np.array([0.0, S.GROUND_Y, -2.0])
    hz = math.hypot(v[0], v[2])
    el = min(max(math.atan2(v[1], hz), math.radians(14)), math.radians(55))
    az = math.atan2(v[0], -v[2])
    return np.array([math.sin(az) * math.cos(el), math.sin(el), -math.cos(az) * math.cos(el)])


OUT_FLASH_D = [_out_flash_dir(i) for i in range(T.N_STRIKES)]
_OUT_FLASH = {}


def out_flash_mask(i):
    """ground shadow mask for strike i's light (computed on first use; many strikes share a direction)"""
    key = tuple(np.round(OUT_FLASH_D[i], 2))
    if key not in _OUT_FLASH:
        _OUT_FLASH[key] = directional_mask(OUTG, OUT_FACES, OUT_FLASH_D[i], blur=1.5)
    return _OUT_FLASH[key]

INFL = MaskGrid((S.RX0, S.RX1, S.RZ0, S.RZ1, 0.01))
IN_STATIC = [o for o in S.OBJS_IN if o.cast]
IN_FACES = [Q for o in IN_STATIC for Q, n in o.faces] + [Q for Q, n in S.rocking_chair(0.0)]
IN_MOON = directional_mask(INFL, IN_FACES, MOON_L, blur=1.0)
_IN_FLASH = {}


def in_flash_mask(i):
    key = tuple(np.round(FLASH_D[i], 2))
    if key not in _IN_FLASH:
        _IN_FLASH[key] = directional_mask(INFL, IN_FACES, FLASH_D[i], blur=1.0)
    return _IN_FLASH[key]
LAMP_MASK = point_mask(INFL, [Q for o in IN_STATIC if o.grp not in (S.LAMP_G,) for Q, n in o.faces] + [Q for Q, n in S.rocking_chair(0.0)], S.LAMP_P, blur=2.5)
FIRE_MASK = point_mask(INFL, [Q for o in IN_STATIC if o.grp not in (S.FP_G,) for Q, n in o.faces] + [Q for Q, n in S.rocking_chair(0.0)], S.FIRE_P + [0, 0.25, 0.25], blur=3.0)

# ---------------------------------------------------------------- camera
CLX = (np.array([0.3, 0.75, -0.6]), 7.1, 6, 18, 60, 405)       # climax / cold-open framing: sky over the rod
CAB = (np.array([0.05, 0.6, -0.3]), 6.5, 9, 22, 60, 395)        # cabin: cat on the sill, window, room
WIDE = (np.array([-1.0, 0.0, -11.0]), 22.0, 3, 25, 52, 305)     # high and wide: rings crossing the prairie
MED = (np.array([-0.6, 0.2, -5.5]), 14.0, 6, 23, 56, 360)       # rings closing on the cabin
NEAR = (np.array([-0.3, 0.3, -3.8]), 11.5, 7, 22, 58, 385)      # the tagged ring reaches the cabin
FENCE = (np.array([-0.4, 0.3, -3.5]), 10.5, 8, 21, 58, 400)     # the storm crosses the fence posts


def _k(t, base, dd=0.0, **kw):
    tgt, d, yaw, pitch, fov, sy = base
    return (t, kw.get('tgt', tgt), d + dd, kw.get('yaw', yaw), kw.get('pitch', pitch), kw.get('fov', fov), kw.get('sy', sy))


KEYS = [  # (t, target, dist, yaw, pitch, fov, sy)
    _k(0.0, CLX),                       # cold open: the overhead strike
    _k(2.9, CLX, -0.25),
    _k(5.0, WIDE),                      # the rewind pulls back to the horizon
    _k(12.0, WIDE, -1.0, yaw=4),
    _k(15.5, MED),
    _k(18.6, NEAR),
    _k(21.6, NEAR, 0.3, yaw=8),
    _k(24.5, FENCE),
    _k(28.6, FENCE, -0.3),
    _k(31.8, CAB),                      # the cat leaves the sill
    (42.0, np.array([0.15, 0.65, -0.4]), 6.8, 8, 20, 60, 400),
    _k(48.8, CLX),
    _k(51.6, CLX, -0.1),
    _k(54.4, FENCE, -0.6, yaw=7),       # the second cell races in
    _k(57.0, FENCE, -0.9, yaw=7),
    _k(59.3, CLX),
    _k(60.0, CLX),
]


def camera(t):
    # 2.9 -> 5.0 s: the pull-back from the rod to the horizon IS the rewind
    return interp_keys(KEYS, t)


# ---------------------------------------------------------------- procedural textures
def hash2(a, b):
    v = np.sin(a * 127.1 + b * 311.7) * 43758.5453
    return v - np.floor(v)


def flicker(t, i=0):
    return 0.88 + 0.08 * math.sin(t * 11.0 + i * 1.7) * math.sin(t * 6.3 + i) + 0.04 * math.sin(t * 23.0 + 2 * i)


def floor_albedo(X, Z):
    """wide pine planks along z + braided rag rug in front of the hearth"""
    pw = 0.17
    col = np.floor(X / pw)
    off = hash2(col, 3.0) * 0.9
    plank = np.floor((Z + off) / 0.9)
    h = hash2(col, plank)
    base = np.array([160, 110, 66], float)[None] * (0.84 + 0.22 * h)[:, None]
    base *= (0.97 + 0.05 * np.sin(Z * 29 + col * 2.3))[:, None]
    seam = (np.abs(((X / pw) % 1.0) - 0.5) > 0.465) | (np.abs((((Z + off) / 0.9) % 1.0) - 0.5) > 0.49)
    base[seam] *= 0.7
    rx, rz = (X + 0.42) / 0.78, (Z + 0.05) / 0.62
    rr = np.sqrt(rx * rx + rz * rz)
    rug = rr < 1.0
    band = np.floor(rr * 7)
    pal = np.array([[150, 60, 52], [196, 160, 96], [90, 110, 140], [170, 90, 60], [120, 140, 100], [196, 160, 96], [140, 70, 66]], float)
    rc = pal[np.clip(band.astype(int), 0, 6)]
    rc *= (0.9 + 0.12 * (np.sin(np.arctan2(rz, rx) * 22 + band * 1.3) > 0))[:, None]
    base[rug] = rc[rug]
    return base


def log_albedo(U, Y):
    """horizontal logs with light chinking between them; U runs along the wall"""
    li = np.floor(Y / S.LOG)
    v = (Y / S.LOG) % 1.0
    hh = hash2(li, 7.0)
    wood = np.array([132, 86, 52], float)[None] * (0.86 + 0.2 * hh)[:, None]
    wood *= (0.95 + 0.06 * np.sin(U * 17 + li * 3.1) * np.sin(U * 5.3 + li))[:, None]
    roundness = 0.78 + 0.3 * np.sin(np.pi * np.clip(v, 0, 1))
    col = wood * roundness[:, None]
    chink = (v < 0.09) | (v > 0.93)
    col[chink] = np.array([178, 164, 140.0]) * 0.8
    return col


def grass_albedo(X, Z, wet):
    h = hash2(np.floor(X * 2.2), np.floor(Z * 2.2))
    band = 0.5 + 0.5 * np.sin(Z * 0.9 + np.sin(X * 0.35) * 2.0)
    base = np.array([66, 100, 58], float)[None] * (0.8 + 0.18 * h + 0.12 * band)[:, None]
    # dirt yard around the cabin + track to the barn
    dx = np.maximum(np.maximum(S.RX0 - 0.6 - X, X - (S.RX1 + 0.6)), 0)
    dz = np.maximum(np.maximum(S.RZ0 - 0.6 - Z, Z - (S.RZ1 + 0.6)), 0)
    yard = np.hypot(dx, dz) < 0.25 + 0.15 * hash2(np.floor(X * 6), np.floor(Z * 6))
    a, b = np.array([S.RX1, S.RZ0 - 0.3]), np.array([S.BARN_C[0], S.BARN_C[2] + S.BARN_HZ])
    ab = b - a
    u = np.clip(((X - a[0]) * ab[0] + (Z - a[1]) * ab[1]) / (ab @ ab), 0, 1)
    dpath = np.hypot(X - (a[0] + u * ab[0]), Z - (a[1] + u * ab[1]))
    track = dpath < 0.28
    dirt = np.array([118, 92, 64], float)[None] * (0.85 + 0.2 * h)[:, None]
    base = np.where((yard | track)[:, None], dirt, base)
    return base * (1 - 0.18 * wet)


# ---------------------------------------------------------------- sky
SKY_CLEAR = ((26, 34, 86), (86, 112, 164))
SKY_STORM = ((34, 36, 54), (78, 82, 104))


def sky_cols(t):
    s = min(1.0, T.storm(t) * 1.15) * 0.92
    z = mixc(SKY_CLEAR[0], SKY_STORM[0], s)
    h = mixc(SKY_CLEAR[1], SKY_STORM[1], s)
    return z, h


STARS = np.random.default_rng(9).normal(0, 1, (160, 3))
STARS[:, 1] = np.abs(STARS[:, 1]) + 0.05
STARS /= np.linalg.norm(STARS, axis=1, keepdims=True)


def draw_sky(cam, t, flash):
    z, h = sky_cols(t)
    mk = T.moon_k(t)
    sky = sky_gradient(cam, z, h, glow_dir=MOON_SKY, glow_col=(120, 140, 190), glow_k=0.55 * mk, step=7.0)
    fr = Frame(sky)
    sk = (1 - min(1.0, T.storm(t) * 1.6)) * 0.85 + 0.15 * mk
    for k, sd in enumerate(STARS):
        s = cam.screen_of_dir(sd)
        if s is None:
            continue
        x, y = int(s[0]), int(s[1])
        if 0 <= x < W and 0 <= y < H:
            b = (120 + 100 * (0.5 + 0.5 * math.sin(t * (1.5 + k % 5) + k))) * sk
            if b > 30:
                c = fr.pil.getpixel((x, y))
                fr.draw.point((x, y), fill=tuple(int(min(255, c[j] + b * (0.9, 0.92, 1.0)[j] * 0.6)) for j in range(3)))
    ms = cam.screen_of_dir(MOON_SKY)
    if ms is not None and mk > 0.02:
        r = 0.028 * cam.f
        base = np.array(fr.pil.getpixel((int(np.clip(ms[0], 0, W - 1)), int(np.clip(ms[1], 0, H - 1)))), float)
        col = base * (1 - mk) + np.array([240, 236, 212.0]) * mk
        fr.disc(ms, r, col, n=16)
        fr.disc((ms[0] + r * 0.25, ms[1] - r * 0.2), r * 0.28, col * 0.92, n=8)
        fr.disc((ms[0] - r * 0.35, ms[1] + r * 0.3), r * 0.2, col * 0.93, n=8)
    return fr, ms


# ---------------------------------------------------------------- outdoor ground, per pixel
HORIZ_D = 72.0


RING_CAP = 12


def sound_rings(t):
    """active rings: (centre xz, radius, strength, counted?). A ring leaves its strike at the flash and
    reaches the cabin exactly when its thunder sounds (radius follows the same distance mapping)."""
    rk = T.ring_k(t)
    if rk <= 0:
        return []
    out = []
    for i in range(T.N_STRIKES):
        tf, d = T.STRIKES[i][:2]
        if d < 0.4 or T.KIND[i] == 'rod':
            continue
        tau = t - tf
        if tau < 0 or tau > d + 0.35:
            continue
        u = min(1.0, tau / d)
        P = S.STRIKE_P[i]
        R0 = math.hypot(P[0], P[2])
        km = T.true_km(i)
        R = R0 - S.dmap(km * (1 - u)) + S.ROD_D * ss(0.75, 1.0, u)
        if tau > d:
            R = R0 + (tau - d) * 12
        a = (1 - ss(d, d + 0.35, tau)) * (0.55 + 0.45 * ss(0, 0.15, tau)) * rk
        counted = i in T.COUNTED
        if not counted:
            a *= 0.6 * (0.62 + 0.38 * math.exp(-km / 2.5))    # chaos rings stay in the background
        out.append((P[[0, 2]], R, a, counted))
    if len(out) > RING_CAP:                                     # safety cap: drop the faintest
        out.sort(key=lambda r: -(r[2] + (9 if r[3] else 0)))
        out = out[:RING_CAP]
    return out


def ground_pass(fr, cam, t, Lx, flashes, z_hor, h_hor):
    arr, depth = fr.array(), fr.depth()
    dw = cam.pixel_rays(XX, YY)
    p = cam.pos
    with np.errstate(divide='ignore', invalid='ignore'):
        tg = np.where(dw[..., 1] < -1e-6, (S.GROUND_Y - p[1]) / dw[..., 1], 1e9)
        X, Z = p[0] + tg * dw[..., 0], p[2] + tg * dw[..., 2]
        hd = tg * np.hypot(dw[..., 0], dw[..., 2])
    m = (tg < 1e8) & (hd < HORIZ_D)
    ys, xs = np.nonzero(m)
    if not len(xs):
        return
    X, Z, td, hd = X[ys, xs], Z[ys, xs], tg[ys, xs], hd[ys, xs]
    alb = grass_albedo(X, Z, T.rain(t))
    I = np.tile(Lx.amb + Lx.sky * 0.8, (len(X), 1))
    mk = T.moon_k(t)
    I += ((1 - OUTG.sample(OUT_MOON, X, Z)) * 0.55 * mk)[:, None] * MOON_RGB[None]
    for i, env in flashes:
        d = OUT_FLASH_D[i]
        sp = S.STRIKE_P[i]
        dist = np.hypot(X - sp[0], Z - sp[2])
        fall = 0.35 + 0.65 * np.exp(-dist / 9.0)
        I += ((1 - OUTG.sample(out_flash_mask(i), X, Z)) * fall * env * T.flash_gain(i) * 1.15)[:, None] * FLASH_RGB[None]
    # warm spill from the window and the cut-away onto the yard
    sp = np.hypot((X - 0.0) / 2.2, (Z - 1.6) / 1.1)
    I += (0.22 * np.exp(-sp * sp))[:, None] * WARM[None]
    ao = contact_ao(X, Z, S.OUT_FEET)
    I *= ao[:, None]
    col = alb * I
    # the sound of each strike: a pale ring crawling across the prairie
    for c, R, a, counted in sound_rings(t):
        rr = np.hypot(X - c[0], Z - c[1])
        dd = np.abs(rr - R)
        w = (0.26 if counted else 0.13) + (0.02 if counted else 0.014) * R
        ring = np.clip(1 - dd / w, 0, 1) ** 1.3 + 0.4 * np.clip(1 - np.abs(rr - (R - 0.7 - 0.04 * R)) / w, 0, 1)
        tint = np.array([1.0, 0.78, 0.4]) if counted else np.array([0.6, 0.85, 1.0])
        col += (ring * a * (270 if counted else 190))[:, None] * tint[None]
    # fog to the horizon
    k = np.clip((hd - 9.0) / (HORIZ_D - 14.0), 0, 1) ** 0.85 * 0.92
    col = col * (1 - k[:, None]) + np.asarray(h_hor, float)[None] * k[:, None]
    arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
    depth[ys, xs] = td
    fr.set_array(arr, depth)


def draw_far(fr, cam, t, h_hor):
    hc = np.asarray(h_hor, float)
    for Q, n in S.FAR2:
        fr.poly(cam, Q, hc * 0.86 + np.array([0, 2, 8]), depth=False)
    for Q, n in S.FAR:
        fr.poly(cam, Q, hc * 0.76 + np.array([0, 4, 6]), depth=False)


# ---------------------------------------------------------------- storm cells
CLOUD_RGB = np.array([70, 76, 104], float)


def draw_cell(fr, cam, t, base, alpha, flashes, h_hor, z_hor, Lx):
    if alpha <= 0.01:
        return
    lift = S.cell_lift(float(np.hypot(base[0], base[2])))
    items = []
    for off, faces in S.CELL_FACES:
        c = base + off + np.array([0, lift, 0])
        items.append((np.linalg.norm(c - cam.pos), c, faces))
    for dist, c, faces in painter(items):
        fc = [(Q + c, n) for Q, n in faces if np.dot(n, cam.pos - (Q.mean(0) + c)) > 0]
        if not fc:
            continue
        N = np.array([n for _, n in fc])
        C = np.array([Q.mean(0) for Q, _ in fc])
        # flat cloud shading: moon rim on top, darker bellies, lit from within by the flash
        up = np.clip(N[:, 1], -1, 1)
        hgt = np.clip((C[:, 1] - S.CELL_BOTTOM - lift) / 5.5, 0, 1)
        I = 0.5 + 0.3 * up + 0.45 * hgt
        I += T.moon_k(t) * 0.55 * np.clip(N @ nrm(MOON_SKY + [0, 0.4, 0]), 0, None)
        col = CLOUD_RGB[None] * I[:, None]
        for i, env in flashes:
            bt = S.BOLTS[i][0][0]
            dd = np.linalg.norm(C - bt[None], axis=1)
            col += (env * T.flash_gain(i) * 150 * np.exp(-dd / 4.5))[:, None] * np.array([0.82, 0.84, 1.0])[None]
        fk = np.clip((dist - 14) / 50.0, 0, 0.7)
        col = col * (1 - fk) + np.asarray(h_hor, float)[None] * fk
        if alpha < 1:
            col = col * alpha + np.asarray(mixc(z_hor, h_hor, 0.5), float)[None] * (1 - alpha)
        col = np.round(col / 5) * 5
        for (Q, n), cc in zip(fc, col):
            fr.poly(cam, Q, cc)


def draw_rain_curtain(fr, cam, t, base, alpha, rk):
    """grey streaks hanging under the cell"""
    if alpha <= 0.02 or rk <= 0.02:
        return
    r = np.random.default_rng(77)
    n = int(70 * rk * alpha)
    pts = r.uniform(-1, 1, (90, 2)) * 3.0
    for k in range(n):
        x, z = base[0] + pts[k, 0], base[2] + pts[k, 1]
        ph = (T.phase(t) * 1.3125 + k * 0.37) % 1.0
        y1 = S.CELL_BOTTOM + S.cell_lift(float(np.hypot(base[0], base[2]))) - ph * 1.2
        y0 = max(S.GROUND_Y, y1 - 2.6)
        P = np.array([[x + 0.35, y1, z], [x, y0, z]])
        zc = cam.depth(P)
        if (zc < 0.5).any():
            continue
        fk = min(0.6, np.linalg.norm(P[0] - cam.pos) / 80)
        c = np.array([120, 128, 156.0]) * (1 - fk) + np.array([90, 96, 120.0]) * fk
        fr.line(cam, P, tuple(int(v) for v in c), 1)


# ---------------------------------------------------------------- object shading
def lights_out(t, cam, flashes):
    s = T.storm(t)
    L = Lights(amb=(0.3 - 0.06 * s, 0.33 - 0.06 * s, 0.5 - 0.08 * s), sky=(0.06, 0.07, 0.12))
    L.view = cam.view
    L.key('moon', MOON_L, MOON_RGB * 0.7 * T.moon_k(t))
    if flashes:
        i, env = flashes[0]
        L.key('flash', OUT_FLASH_D[i], FLASH_RGB * env * T.flash_gain(i) * 1.3)
    else:
        L.key('flash', MOON_L, (0, 0, 0))
    return L


def lights_in(t, cam, flashes):
    L = Lights(amb=(0.4, 0.34, 0.34), sky=(0.04, 0.04, 0.06))
    L.view = cam.view
    L.key('moon', MOON_L, MOON_RGB * 0.8 * T.moon_k(t))
    if flashes:
        i, env = flashes[0]
        L.key('flash', FLASH_D[i], FLASH_RGB * env * T.flash_gain(i) * 1.5)
    else:
        L.key('flash', MOON_L, (0, 0, 0))
    lk = 1.05 * flicker(t, 0)
    fk = 1.0 * flicker(t * 1.3, 3)
    L.point(S.LAMP_P, WARM * lk, 1.25)
    L.point(S.FIRE_P + [0, 0.25, 0.3], FIRE_RGB * fk, 1.15)
    return L, lk, fk


def draw_obj(fr, cam, L, o, inside, emit=0.0, kd=None):
    fc = [(cam.depth(Q.mean(0))[0], Q, n, j) for j, (Q, n) in enumerate(o.faces) if o.double or np.dot(n, cam.pos - Q.mean(0)) > 0]
    if not fc:
        return
    fc.sort(key=lambda a: -a[0])
    C = np.array([q.mean(0) for _, q, _, _ in fc])
    N = np.array([n for _, _, n, _ in fc])
    if o.double:
        flip = (N * (cam.pos[None] - C)).sum(1) < 0
        N = np.where(flip[:, None], -N, N)
    Pv = C + N * 0.02
    kv = np.ones((len(fc), 2), np.float32)
    for ki in range(2):
        d = L.keys[ki][1]
        if L.keys[ki][2].max() <= 1e-4:
            kv[:, ki] = 0
            continue
        v = OCC.vis(Pv, o.grp, d=d)
        if inside:
            v = v * S.win_lit(Pv, d)
        kv[:, ki] = v
    base = np.array([o.color * o.tone[j] for *_, j in fc])
    cols = shade(L, N, C, base, kvis=kv, emit=emit)
    for (z, Q, n, j), c in zip(fc, cols):
        fr.poly(cam, Q, c)


# ---------------------------------------------------------------- the cabin shell, per pixel (depth-tested)
def shell_pass(fr, cam, t, L, lk, fk, flashes, cat_masks, rk):
    arr, depth = fr.array(), fr.depth()
    dw = cam.pixel_rays(XX, YY)
    p = cam.pos
    INF = 1e9
    with np.errstate(divide='ignore', invalid='ignore'):
        tf = np.where(dw[..., 1] < -1e-6, -p[1] / dw[..., 1], INF)
        Xf, Zf = p[0] + tf * dw[..., 0], p[2] + tf * dw[..., 2]
        okf = (tf > 0) & (Xf >= S.RX0) & (Xf <= S.RX1) & (Zf >= S.RZ0) & (Zf <= S.RZ1)
        tb = np.where(dw[..., 2] < -1e-6, (S.RZ0 - p[2]) / dw[..., 2], INF)
        Xb, Yb = p[0] + tb * dw[..., 0], p[1] + tb * dw[..., 1]
        okb = (tb > 0) & (Xb >= S.RX0) & (Xb <= S.RX1) & (Yb >= 0) & (Yb <= S.WALL_H)
        tl = np.where(dw[..., 0] < -1e-6, (S.RX0 - p[0]) / dw[..., 0], INF)
        Yl, Zl = p[1] + tl * dw[..., 1], p[2] + tl * dw[..., 2]
        okl = (tl > 0) & (Zl >= S.RZ0) & (Zl <= S.RZ1) & (Yl >= 0) & (Yl <= S.WALL_H)
    winpx = okb & S.win_open(Xb, Yb)
    tf = np.where(okf, tf, INF)
    tbs = np.where(okb, tb, INF)
    tl_ = np.where(okl, tl, INF)
    tmin = np.minimum(np.minimum(tf, tbs), tl_)
    vis = tmin < depth * 1.0005                               # depth test against what is already drawn
    mf = okf & (tf <= tmin) & vis
    mb = okb & (tbs <= tmin) & ~mf & vis
    ml = okl & (tl_ <= tmin) & ~mf & ~mb & vis
    mbw = mb & winpx
    mb = mb & ~winpx

    def warm_at(P, nv, floor=False):
        I = np.zeros((len(P), 3))
        for lp, R, k, col, msk in ((S.LAMP_P, 1.25, lk, WARM, LAMP_MASK), (S.FIRE_P + [0, 0.25, 0.3], 1.15, fk, FIRE_RGB, FIRE_MASK)):
            v = lp[None] - P
            dist = np.linalg.norm(v, axis=1) + 1e-6
            lam = np.clip((v * nv[None]).sum(1) / dist, 0, None) * 0.85 + 0.15
            sh = 1.0 if not floor else (1 - INFL.sample(msk, P[:, 0], P[:, 2]) * 0.8)
            I += (lam * sh * k / (1 + (dist / R) ** 2))[:, None] * col[None] * 1.1
        return I

    # floor
    ys, xs = np.nonzero(mf)
    if len(xs):
        X, Z = Xf[ys, xs], Zf[ys, xs]
        P = np.stack([X, np.zeros_like(X), Z], 1)
        alb = floor_albedo(X, Z)
        I = np.tile(L.amb + L.sky, (len(X), 1))
        mk = T.moon_k(t)
        if mk > 0.01:
            ml_ = S.win_lit(P + [0, 0.002, 0], MOON_L) * (1 - np.maximum(INFL.sample(IN_MOON, X, Z), INFL.sample(cat_masks[0], X, Z)))
            I += (ml_ * 0.95 * mk)[:, None] * MOON_RGB[None]
        for i, env in flashes:
            d = FLASH_D[i]
            fl = S.win_lit(P + [0, 0.002, 0], d) * (1 - np.maximum(INFL.sample(in_flash_mask(i), X, Z), INFL.sample(cat_masks[1], X, Z)))
            I += (fl * env * T.flash_gain(i) * 1.55)[:, None] * FLASH_RGB[None]
            I += 0.06 * env * T.flash_gain(i) * FLASH_RGB[None]
        I += warm_at(P, np.array([0, 1.0, 0]), True)
        ao = contact_ao(X, Z, S.FEET)
        wall_ao = 1 - 0.3 * np.exp(-(X - S.RX0) / 0.22) - 0.28 * np.exp(-(Z - S.RZ0) / 0.22)
        I *= (ao * wall_ao)[:, None]
        col = alb * I
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tf[ys, xs]
    # walls
    for m, U_, Y_, tt, nv, which in ((mb, Xb, Yb, tb, np.array([0, 0, 1.0]), 'back'), (ml, Zl, Yl, tl, np.array([1.0, 0, 0]), 'left')):
        ys, xs = np.nonzero(m)
        if not len(xs):
            continue
        U, Y = U_[ys, xs], Y_[ys, xs]
        P = np.stack([U, Y, np.full_like(U, S.RZ0)], 1) if which == 'back' else np.stack([np.full_like(U, S.RX0), Y, U], 1)
        alb = log_albedo(U + (0 if which == 'back' else 7.3), Y)
        I = np.tile(L.amb * 0.92, (len(U), 1))
        I += warm_at(P, nv)
        for i, env in flashes:
            I += 0.07 * env * T.flash_gain(i) * FLASH_RGB[None]
        ao = 1 - 0.32 * np.exp(-Y / 0.18)
        ao *= 1 - 0.25 * np.exp(-((U - S.RX0) if which == 'back' else (U - S.RZ0)) / 0.25)
        col = alb * I * ao[:, None]
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tt[ys, xs]
    # window: the storm seen through the glass, mullions, reveal, rain on the pane
    ys, xs = np.nonzero(mbw)
    if len(xs):
        Xw, Yw = Xb[ys, xs], Yb[ys, xs]
        d2 = dw[ys, xs]
        tr = (S.RZ0 - S.WT - p[2]) / d2[:, 2]
        Xr, Yr = p[0] + tr * d2[:, 0], p[1] + tr * d2[:, 1]
        reveal = ~S.win_open(Xr, Yr)
        mull = S.win_mullion(Xw, Yw)
        col = arr[ys, xs] * np.array([0.86, 0.9, 1.0])[None] + np.array([6, 9, 16.0])[None]
        # rain on the glass: drops + running streaks
        if rk > 0.02:
            cx = np.floor(Xw * 48)
            hs = hash2(cx, 5.0)
            speed = 0.25 + 0.5 * hs
            yy = (Yw * 9 + T.phase(t) * speed * 9 + hs * 37) % 3.0
            streak = (hs > 1 - 0.55 * rk) & (yy < 0.32 + 0.4 * hs) & (np.abs((Xw * 48) % 1 - 0.5) < 0.22)
            col[streak] = col[streak] * 0.6 + np.array([170, 186, 214.0]) * 0.4
            drop = hash2(np.floor(Xw * 60), np.floor(Yw * 60) + np.floor(T.phase(t) * 0.7)) > 1 - 0.05 * rk
            col[drop] = col[drop] * 0.5 + np.array([200, 214, 236.0]) * 0.5
        Pw = np.stack([Xw, Yw, np.full_like(Xw, S.RZ0)], 1)
        Iw = (L.amb * 0.7)[None] + warm_at(Pw, np.array([0, 0, 1.0]))
        for i, env in flashes:
            Iw = Iw + 0.5 * env * T.flash_gain(i) * FLASH_RGB[None]
        frame_c = np.array([170, 118, 74.0])
        col[mull] = (frame_c[None] * Iw)[mull]
        rc = log_albedo(Xw * 3.0, Yr) * (Iw * 0.85 + 0.08)
        col[reveal & ~mull] = rc[reveal & ~mull]
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = np.where(reveal | mull, tb[ys, xs], depth[ys, xs])
    fr.set_array(arr, depth)


def cabin_edges(fr, cam, t, L, lk):
    """cut-away edges: wall tops, cut faces with log ends, foundation stones under the floor"""
    G = S.GROUND_Y
    top = (150, 104, 66)
    cut = (112, 74, 46)
    stone = (112, 106, 100)
    polys = [
        ([[S.RX0 - S.WT, S.WALL_H, S.RZ0 - S.WT], [S.RX1, S.WALL_H, S.RZ0 - S.WT], [S.RX1, S.WALL_H, S.RZ0], [S.RX0 - S.WT, S.WALL_H, S.RZ0]], top),
        ([[S.RX0 - S.WT, S.WALL_H, S.RZ0], [S.RX0, S.WALL_H, S.RZ0], [S.RX0, S.WALL_H, S.RZ1], [S.RX0 - S.WT, S.WALL_H, S.RZ1]], top),
        ([[S.RX1, 0, S.RZ0 - S.WT], [S.RX1, S.WALL_H, S.RZ0 - S.WT], [S.RX1, S.WALL_H, S.RZ0], [S.RX1, 0, S.RZ0]], cut),
        ([[S.RX0 - S.WT, 0, S.RZ1], [S.RX0, 0, S.RZ1], [S.RX0, S.WALL_H, S.RZ1], [S.RX0 - S.WT, S.WALL_H, S.RZ1]], cut),
        ([[S.RX0 - S.WT, G, S.RZ1], [S.RX1, G, S.RZ1], [S.RX1, 0, S.RZ1], [S.RX0 - S.WT, 0, S.RZ1]], stone),
        ([[S.RX1, G, S.RZ1], [S.RX1, G, S.RZ0 - S.WT], [S.RX1, 0, S.RZ0 - S.WT], [S.RX1, 0, S.RZ1]], (96, 90, 86)),
    ]
    for Q, c in polys:
        Q = np.array(Q, float)
        n = face(Q)[1]
        ctr = Q.mean(0)
        if np.dot(n, cam.pos - ctr) < 0:
            n = -n
        I = L.amb + np.clip(n @ np.array([0.3, 0.8, 0.5]), 0, None) * 0.25
        v = S.LAMP_P - ctr
        I = I + WARM * 0.35 * lk / (1 + (np.linalg.norm(v) / 1.6) ** 2)
        fr.poly(cam, Q, np.clip(np.array(c) * I, 0, 255))
    # log ends on the two cut faces + stone joints on the foundation
    for k in range(int(S.WALL_H / S.LOG)):
        yc = (k + 0.5) * S.LOG
        for ctr, ax in ((np.array([S.RX1 + 0.002, yc, S.RZ0 - S.WT / 2]), 'x'), (np.array([S.RX0 - S.WT / 2, yc, S.RZ1 + 0.002]), 'z')):
            r = S.LOG * 0.46
            ang = np.linspace(0, 2 * np.pi, 9)[:-1]
            if ax == 'x':
                Q = np.array([[ctr[0], ctr[1] + r * math.sin(a), ctr[2] + r * math.cos(a) * 0.75] for a in ang])
            else:
                Q = np.array([[ctr[0] + r * math.cos(a) * 0.75, ctr[1] + r * math.sin(a), ctr[2]] for a in ang])
            sh = 0.8 + 0.25 * hash2(k, 1.0 if ax == 'x' else 2.0)
            fr.poly(cam, Q, np.clip(np.array([196, 150, 100.0]) * sh * (L.amb + 0.25 + WARM * 0.25 * lk), 0, 255))
    for xx in np.arange(S.RX0 - S.WT + 0.15, S.RX1, 0.3):
        fr.line(cam, [[xx, G + 0.01, S.RZ1 + 0.002], [xx, -0.01, S.RZ1 + 0.002]], (70, 66, 64), 1)
    fr.line(cam, [[S.RX0 - S.WT, 0.0, S.RZ1 + 0.003], [S.RX1, 0.0, S.RZ1 + 0.003]], (180, 130, 82), 1)


# ---------------------------------------------------------------- the cat
CAT_RUG = np.array([-0.62, 0.0, -0.32])
CAT_FLOOR = np.array([S.CAT_SILL[0] + 0.05, 0.0, S.RZ0 + 0.6])


def _hop(a, b, u, h):
    p = a + (b - a) * u
    p[1] += h * 4 * u * (1 - u)
    return p


def cat_state(t):
    """(position, yaw, pose, phase): yaw 0 = facing -z (out of the window)"""
    mode, ct = T.cat_clock(t)
    if mode == 'co':                                   # cold open bolt (and its replay backwards in the rewind)
        j0, j1 = T.CO_JUMP
        r0, r1 = T.CO_RUN
        if ct < j0:
            return S.CAT_SILL.copy(), 0.0, 'sit', 0.0
        if ct < j1:
            u = ease((ct - j0) / (j1 - j0))
            return _hop(S.CAT_SILL, CAT_FLOOR, u, 0.25), math.pi, 'walk', u * 2
        if ct < r1:
            u = ease((ct - r0) / (r1 - r0))
            p = CAT_FLOOR + (CAT_RUG - CAT_FLOOR) * u
            yaw = math.atan2(-(CAT_RUG - CAT_FLOOR)[0], (CAT_RUG - CAT_FLOOR)[2]) + math.pi
            return p, yaw, 'walk', (ct - r0) * 8
        return CAT_RUG.copy(), 0.6, 'curl', 0.0
    j0, j1 = T.CAT_JUMP_DOWN
    w0, w1 = T.CAT_WALK
    b0, b1 = T.CAT_WALK_BACK
    u0, u1 = T.CAT_JUMP_UP
    if t < j0 or t >= u1:
        return S.CAT_SILL.copy(), 0.0, 'sit', 0.0
    if t < j1:
        u = ease((t - j0) / (j1 - j0))
        return _hop(S.CAT_SILL, CAT_FLOOR, u, 0.25), math.pi, 'walk', u * 2
    if t < w1:
        u = ease((t - w0) / (w1 - w0))
        p = CAT_FLOOR + (CAT_RUG - CAT_FLOOR) * u
        yaw = math.atan2(-(CAT_RUG - CAT_FLOOR)[0], (CAT_RUG - CAT_FLOOR)[2]) + math.pi
        return p, yaw, 'walk', (t - w0) * 5
    if t < b0:
        return CAT_RUG.copy(), 0.6, 'curl', 0.0
    if t < b1:
        u = ease((t - b0) / (b1 - b0))
        p = CAT_RUG + (CAT_FLOOR - CAT_RUG) * u
        yaw = math.atan2(-(CAT_FLOOR - CAT_RUG)[0], (CAT_FLOOR - CAT_RUG)[2]) + math.pi
        return p, yaw, 'walk', (t - b0) * 5
    u = ease((t - u0) / (u1 - u0))
    return _hop(CAT_FLOOR, S.CAT_SILL, u, 0.35), 0.0, 'walk', 0.0


def cat_faces(t):
    pos, yaw, pose, ph = cat_state(t)
    fl = T.cat_flinch(t)
    r2 = np.random.default_rng(11)
    f = []
    if pose == 'sit':
        crouch = 1 - 0.12 * fl
        f += icosa([0, 0.11 * crouch, 0.02], 0.1, (0.85, 1.12 * crouch, 0.9), jit=0.07, rng=r2)
        f += icosa([0, 0.16 * crouch, -0.05], 0.07, (0.9, 1.0, 0.8), jit=0.05, rng=r2)
        hc = np.array([0.0, 0.27 * crouch, -0.06])
        look = 0.35 * math.sin(2 * math.pi * T.phase(t) / 12.0) * (1 - fl)
        head = icosa(hc, 0.062, (1.05, 0.92, 1.0), jit=0.05, rng=r2)
        for sgn in (-1, 1):
            tilt = 0.035 * fl
            head.append(face([hc + [0.022 * sgn, 0.035, 0.0], hc + [0.05 * sgn, 0.03 - tilt, 0.01], hc + [0.04 * sgn + tilt * sgn, 0.1 - tilt * 1.5, 0.0]], hc + [0, 0, 1]))
        R = np.array([[math.cos(look), 0, math.sin(look)], [0, 1, 0], [-math.sin(look), 0, math.cos(look)]])
        f += [((Q - hc) @ R.T + hc, n @ R.T) for Q, n in head]
        p = np.array([0.06, 0.02, 0.06])
        for k in range(7):
            sw = 0.6 * math.sin(2 * math.pi * T.phase(t) / 3.0 - k * 0.5) * (1 + fl)
            q = p + np.array([0.035 + 0.004 * k, 0.003, 0.02 * sw - 0.012])
            w = 0.022 - 0.002 * k
            f.append(face([p + [0, w, 0], p + [0, -0.004, 0], q + [0, -0.004, 0], q + [0, w * 0.9, 0]], p + [0, 0, 1]))
            f.append(face([p + [0, w, 0], p + [0, w, w], q + [0, w * 0.9, w * 0.9], q + [0, w * 0.9, 0]], p + [0, 1, 0]))
            p = q
    elif pose == 'walk':
        step = math.sin(ph * math.pi)
        f += icosa([0, 0.13, 0], 0.085, (0.8, 0.75, 1.55), jit=0.06, rng=r2)
        hc = np.array([0, 0.2, -0.15])
        f += icosa(hc, 0.06, (1.0, 0.92, 1.05), jit=0.05, rng=r2)
        for sgn in (-1, 1):
            f.append(face([hc + [0.022 * sgn, 0.035, 0.0], hc + [0.05 * sgn, 0.03, 0.01], hc + [0.04 * sgn, 0.1, 0.0]], hc + [0, 0, -1]))
        for (lx, lz, sg) in ((-0.04, -0.09, 1), (0.04, -0.09, -1), (-0.04, 0.09, -1), (0.04, 0.09, 1)):
            sw = 0.035 * step * sg
            f += box(lx, 0.05, lz + sw, 0.014, 0.05, 0.014)
        p = np.array([0, 0.15, 0.12])
        for k in range(6):
            q = p + np.array([0.0, 0.035, 0.03 - 0.006 * k])
            w = 0.02 - 0.002 * k
            f.append(face([p + [-w, 0, 0], p + [w, 0, 0], q + [w, 0, 0], q + [-w, 0, 0]], p + [0, 0, 1]))
            p = q
    else:  # curl on the rug, breathing
        br = 1 + 0.03 * math.sin(2 * math.pi * T.phase(t) / 3.0)
        crouch = 1 - 0.1 * fl
        f += icosa([0, 0.07 * br * crouch, 0], 0.12, (1.15, 0.55 * br * crouch, 1.0), jit=0.07, rng=r2)
        hc = np.array([0.09, 0.08 + 0.03 * fl, -0.07])
        f += icosa(hc, 0.058, (1.0, 0.85, 1.0), jit=0.05, rng=r2)
        for sgn in (-1, 1):
            f.append(face([hc + [0.02 * sgn - 0.01, 0.03, 0.0], hc + [0.045 * sgn, 0.025, 0.01], hc + [0.035 * sgn, 0.085 + 0.02 * fl, 0.0]], hc + [0, 0, 1]))
        for k in range(8):
            a0, a1 = 0.4 + k * 0.32, 0.4 + (k + 1) * 0.32
            p0 = np.array([0.15 * math.cos(a0), 0.025, 0.13 * math.sin(a0)])
            p1 = np.array([0.15 * math.cos(a1), 0.025, 0.13 * math.sin(a1)])
            f.append(face([p0 + [0, 0.025, 0], p1 + [0, 0.025, 0], p1, p0], np.array([0, 1, 0.2])))
    c, s = math.cos(yaw), math.sin(yaw)
    R = np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])
    return [(Q @ R.T + pos, n @ R.T) for Q, n in f]


# ---------------------------------------------------------------- fire
def draw_fire(fr, cam, t, fk, glows):
    base = S.FIRE_P + np.array([0, -0.1, 0.03])
    for k in range(6):
        x = base[0] + (k - 2.5) * 0.055
        h = 0.18 + 0.13 * (0.5 + 0.5 * math.sin(t * (9 + k) + k * 1.7)) * fk
        w = 0.05
        tip = np.array([x + 0.02 * math.sin(t * 5 + k * 2), base[1] + h, base[2] + 0.02 * math.sin(t * 7 + k)])
        fr.poly(cam, np.array([[x - w, base[1], base[2]], [x + w, base[1], base[2]], tip]), (255, 150 + 30 * (k % 2), 50))
        mid = np.array([x, base[1], base[2] + 0.01])
        fr.poly(cam, np.array([[x - w * 0.5, base[1], base[2] + 0.01], [x + w * 0.5, base[1], base[2] + 0.01], mid + (tip - mid) * 0.6]), (255, 228, 140))
    s = cam.wp(S.FIRE_P)
    zc = cam.depth(S.FIRE_P)[0]
    glows.append((s[0], s[1], 0.55 * cam.f / max(zc, 0.5), (255, 140, 60), 0.55 * fk))


# ---------------------------------------------------------------- lightning bolts (post-lens, depth-tested)
def draw_bolts(img, depth, cam, flashes, glows):
    for i, env in flashes:
        if env < 0.03:
            continue
        main, branches = S.BOLTS[i]
        g = T.flash_gain(i)
        core = Image.new('L', (W, H), 0)
        halo = Image.new('L', (W, H), 0)
        zimg = Image.new('F', (W, H), 1e9)
        dc, dh, dz = ImageDraw.Draw(core), ImageDraw.Draw(halo), ImageDraw.Draw(zimg)
        zc_all = cam.depth(main)
        wcore = 3 if T.KIND[i] == 'rod' else (2 if zc_all.mean() < 14 else 1)
        for pts, wk in [(main, 1.0)] + [(b, 0.55) for b in branches]:
            Pc = cam.to_cam(pts)
            if (Pc[:, 2] < 0.3).any():
                continue
            s = cam.proj(Pc)
            for a, b, za in zip(s, s[1:], Pc[:-1, 2]):
                seg = [tuple(a), tuple(b)]
                dc.line(seg, fill=int(255 * wk), width=wcore)
                dh.line(seg, fill=int(150 * wk), width=wcore + 4)
                dz.line(seg, fill=float(za), width=wcore + 4)
        zb = np.asarray(zimg)
        vis = depth > zb * 0.97
        c = np.asarray(core, np.float32) / 255 * vis
        hl = np.asarray(halo, np.float32) / 255 * vis
        hl = np.asarray(Image.fromarray((hl * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(1.5)), np.float32) / 255
        a = env * (0.55 + 0.45 * g)
        img += (hl * a * 0.55)[..., None] * np.array([150, 150, 255.0])[None, None]
        img[:] = img * (1 - (c * a)[..., None]) + (c * a)[..., None] * np.array([250, 248, 255.0])[None, None]
        # a soft glow where the channel leaves the cloud (sky only)
        top = cam.wp(main[0])
        glows.append((top[0], top[1], 30, (150, 150, 230), 0.22 * env * g))


# ---------------------------------------------------------------- light shafts through the window
QH, QW = H // 4, W // 4
_QY, _QX = np.mgrid[0:QH, 0:QW].astype(np.float32)


def window_beam(cam, depth, d):
    dirs = cam.pixel_rays(_QX * 4 + 2, _QY * 4 + 2)
    dq = np.minimum(depth[2::4, 2::4], 16.0)
    n = 12
    acc = np.zeros((QH, QW), np.float32)
    jit = (BAYER_FULL[2::4, 2::4] - 0.5)
    for i in range(n):
        s = (i + 0.5 + jit) / n * dq
        P = cam.pos[None, None] + dirs * s[..., None]
        inside = (P[..., 0] > S.RX0) & (P[..., 0] < S.RX1) & (P[..., 2] > S.RZ0) & (P[..., 2] < S.RZ1) & (P[..., 1] > 0) & (P[..., 1] < S.WALL_H)
        lit = S.win_lit(P.reshape(-1, 3), d).reshape(QH, QW) * inside
        acc += lit * dq / n
    beam = 1 - np.exp(-acc * 0.5)
    beam = np.asarray(Image.fromarray((beam * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(1.0)), np.float32) / 255.0
    return np.asarray(Image.fromarray((beam * 255).astype(np.uint8)).resize((W, H), Image.BILINEAR), np.float32) / 255.0


# ---------------------------------------------------------------- rain particles (outdoors only)
RAIN = np.random.default_rng(31).uniform([-9, 0, -24], [9, 7, 3], (900, 3))


def draw_rain(fr, cam, t, rk):
    if rk < 0.02:
        return
    n = int(len(RAIN) * min(1.0, rk))
    wv = np.array([0.5 + 1.2 * T.wind(t), 0, 0.3])
    for k in range(n):
        p0 = RAIN[k]
        y = (p0[1] - T.phase(t) * 9.0417 - k * 0.13) % 7.0 + S.GROUND_Y
        P = np.array([p0[0] + (y - 3.5) * 0.12 * wv[0], y, p0[2]])
        if (S.RX0 - 0.4 < P[0] < S.RX1 + 0.4) and P[2] > S.RZ0 - 0.4:
            continue                                               # no rain inside or in front of the cut-away
        if P[2] > S.RZ0 - 0.4 and abs(P[0]) < 3.2:
            continue
        Q = np.array([P, P + np.array([-0.06 * wv[0], -0.36, -0.03])])
        zc = cam.depth(Q)
        if (zc < 0.6).any():
            continue
        fk = min(0.7, zc.mean() / 40)
        c = np.array([150, 162, 196.0]) * (1 - fk) + np.array([90, 96, 120.0]) * fk
        fr.line(cam, Q, tuple(int(v) for v in c), 1)


# ---------------------------------------------------------------- windmill + tufts + wires + vane
def _wm_speed(t):
    if T.rewinding(t):
        return -26.0 * math.sin(math.pi * T.rew_u(t)) ** 0.7
    return 0.9 + 9.0 * T.wind(t)


_tw = np.linspace(0, 60, 6001)
_wv = np.array([_wm_speed(x) for x in _tw])
_pos = np.where(_wv > 0, _wv, 0.0)
_neg = np.where(_wv < 0, _wv, 0.0)
_ip = np.concatenate([[0], np.cumsum(0.5 * (_pos[1:] + _pos[:-1]) * np.diff(_tw))])
_in = np.concatenate([[0], np.cumsum(0.5 * (_neg[1:] + _neg[:-1]) * np.diff(_tw))])
_step = 2 * math.pi / 12
_scale = (round((_ip[-1] + _in[-1]) / _step) * _step - _in[-1]) / _ip[-1]     # forward part scaled -> seamless loop


def windmill_angle(t):
    return float(np.interp(t, _tw, _ip)) * _scale + float(np.interp(t, _tw, _in))


def tuft_faces(t):
    w = T.wind(t)
    out = []
    for (x, z, h, ph) in S.TUFTS:
        sway = (0.04 + 0.12 * w) * math.sin(2 * math.pi * t / 2.0 * (1 + 0.5 * w) + ph)
        b = np.array([x, S.GROUND_Y, z])
        for k in range(3):
            a = ph + k * 2.1
            dx, dz = 0.06 * math.cos(a), 0.06 * math.sin(a)
            tip = b + np.array([dx * 1.5 + sway, h, dz * 1.5])
            out.append(face([b + [dx, 0, dz], b + [-dz * 0.6, 0, dx * 0.6], tip], b + [0, -1, 0]))
    return out


def draw_wires(fr, cam, Lx):
    for y in S.FENCE_WIRE_Y:
        for k in range(26):
            a = S.FENCE_START + S.FENCE_DIR * (k * 1.45)
            b = S.FENCE_START + S.FENCE_DIR * ((k + 1) * 1.45)
            P = np.array([[a[0], y, a[2]], [b[0], y - 0.04, b[2]]])
            zc = cam.depth(P)
            if (zc < 0.5).any():
                continue
            fr.line(cam, P, (140, 144, 156), 1)


def vane_faces(t):
    a = 0.6 + 0.8 * math.sin(2 * math.pi * t / 15.0) * T.wind(t) + 1.5 * T.storm(t)
    c, s = math.cos(a), math.sin(a)
    R = np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])
    f = S.cyl([0, 0, 0], [0, 0.35, 0], 0.012, 4)
    f += [face([[0, 0.3, 0], [0.22, 0.3, 0], [0.22, 0.24, 0.0], [0.04, 0.24, 0]], (0, 0, 1))]
    f += [face([[-0.12, 0.33, 0], [-0.02, 0.27, 0], [-0.12, 0.21, 0]], (0, 0, 1))]
    return [(Q @ R.T + S.VANE_P, n @ R.T) for Q, n in f]


# ---------------------------------------------------------------- chimney smoke (small life; periodic over 60 s)
def smoke_puffs(t):
    out = []
    w = T.wind(t)
    for k in range(7):
        u = (T.phase(t) * 0.25 + k / 7.0) % 1.0        # phase clock: 12 cycles over the loop, backwards in the rewind
        age = u * 4.0
        p = S.SMOKE_P + np.array([(0.12 + 0.7 * w) * age, 0.32 * age * (1 - 0.45 * w), 0.1 * age])
        r_ = 0.05 + 0.05 * age
        a = 0.8 * (1 - u) ** 1.5 * min(1.0, u * 8)
        out.append((p, r_, a, k))
    return out


def draw_smoke(img, cam, depth, t):
    """soft, banded smoke sprites blended over the frame (after the lens), hidden behind nearer things"""
    from reelkit.raster import GLOW
    lit = 0.75 + 0.35 * T.moon_k(t)
    for p, r_, a, k in sorted(smoke_puffs(t), key=lambda q: -np.linalg.norm(q[0] - cam.pos)):
        if a < 0.05:
            continue
        z = cam.depth(p)[0]
        if z < 0.5:
            continue
        s = cam.wp(p)
        rp = int(max(2, min(40, round(1.6 * r_ * cam.f / z))))
        x, y = int(round(s[0])), int(round(s[1]))
        x0, y0, x1, y1 = x - rp, y - rp, x + rp + 1, y + rp + 1
        if x1 <= 0 or y1 <= 0 or x0 >= W or y0 >= H:
            continue
        kk = GLOW[rp]
        kx0, ky0 = max(0, -x0), max(0, -y0)
        kx1, ky1 = kk.shape[1] - max(0, x1 - W), kk.shape[0] - max(0, y1 - H)
        sub = (slice(max(0, y0), min(H, y1)), slice(max(0, x0), min(W, x1)))
        al = kk[ky0:ky1, kx0:kx1] * a * 0.7 * (depth[sub] > z * 0.98)
        col = np.array([168, 170, 184.0]) * lit
        img[sub] = img[sub] * (1 - al[..., None]) + col[None, None] * al[..., None]


# ---------------------------------------------------------------- HUD
def center_text(img, txt, x, y, col, scale=1, outline=(10, 20, 26)):
    m = text_mask(txt, scale)
    stamp(img, m, int(round(x - m.shape[1] / 2)), int(y), col, outline=outline)


def stopwatch_icon(dr, x, y, r, frac):
    dr.ellipse([x - r, y - r, x + r, y + r], fill=(30, 36, 60), outline=(200, 210, 230))
    dr.rectangle([x - 2, y - r - 4, x + 2, y - r - 1], fill=(200, 210, 230))
    a = 2 * math.pi * frac - math.pi / 2
    dr.line([x, y, x + (r - 2) * math.cos(a), y + (r - 2) * math.sin(a)], fill=(255, 198, 92), width=2)


def bolt_icon(dr, x, y, s, col):
    pts = [(x + 0.2 * s, y), (x + 0.75 * s, y), (x + 0.45 * s, y + 0.42 * s), (x + 0.8 * s, y + 0.42 * s),
           (x + 0.15 * s, y + 1.05 * s), (x + 0.35 * s, y + 0.55 * s), (x, y + 0.55 * s)]
    dr.polygon(pts, fill=col, outline=(20, 20, 30))


def _count_panel(dr, texts, t, y0):
    st = T.counting(t)
    if st is None:
        return
    k, sec, arrived = st
    d = T.STRIKES[k][1]
    shown = d if arrived else sec
    stopwatch_icon(dr, 38, y0 + 30, 14, (shown % 1.0) if not arrived else 0.0)
    texts.append(('COUNT', 62, y0 + 12, SKYB, 1))
    texts.append((f'{shown:.1f} S', 62, y0 + 24, AMBER if arrived else CREAM, 3))
    ncell = int(math.ceil(d - 1e-6))
    n_t = int(min(shown, d) + 1e-6)
    sp = 11 if ncell > 10 else 13
    for j in range(ncell):
        x = 26 + j * sp
        dr.rectangle([x, y0 + 66, x + sp - 5, y0 + 72], fill=(255, 198, 92) if j < n_t else (40, 48, 76))
    if arrived:
        if ss(0, 0.3, t - T.thunder_t(k)) > 0:
            texts.append(('÷ 5 = ' + T.mi_text(d), 196, y0 + 16, MINT, 2))
            texts.append(('÷ 3 = ' + T.km_text(d), 196, y0 + 44, CYAN, 2))
    else:
        bolt_icon(dr, 200, y0 + 16, 22, (255, 230, 120))
        texts.append(('FLASH: SEEN', 226, y0 + 20, CREAM, 1))
        dots = '.' * (1 + int(t * 3) % 3)
        texts.append(('THUNDER' + dots, 226, y0 + 38, SKYB, 1))


def _grid_panel(dr, texts, t, y0):
    texts.append(('FLASH = SNARE', 26, y0 + 9, (255, 230, 120), 1))
    texts.append(('THUNDER = KICK', 248, y0 + 9, CORAL, 1))
    x0, x1 = 34, 326
    yl = y0 + 42
    dr.line([x0, yl, x1, yl], fill=(90, 104, 150), width=1)
    for bt in range(5):
        x = x0 + (x1 - x0) * bt / 4
        dr.line([x, yl - (7 if bt in (0, 4) else 4), x, yl + (7 if bt in (0, 4) else 4)], fill=(150, 164, 210))
    k = T.last_strike(t, T.LOCK)
    if k is None:
        return
    j = T.LOCK.index(k)
    d = T.STRIKES[k][1]
    dprev = T.STRIKES[T.LOCK[j - 1]][1] if j > 0 else d
    u = ease((t - T.STRIKES[k][0]) / 0.35)
    dd = dprev + (d - dprev) * u
    tb = t - T.STRIKES[k][0]
    px = x0 + (x1 - x0) * min(1.0, tb / T.BAR)
    dr.line([px, yl - 12, px, yl + 12], fill=(240, 240, 255), width=1)
    bolt_icon(dr, x0 - 7, yl - 26, 14, (255, 230, 120))
    kx = x0 + (x1 - x0) * min(dd, T.BAR) / T.BAR
    hit = math.exp(-max(0.0, t - T.thunder_t(k)) / 0.15) if t >= T.thunder_t(k) else 0.0
    r = 6 + 4 * hit
    dr.ellipse([kx - r, yl - r, kx + r, yl + r], fill=(255, int(118 + 100 * hit), int(96 + 100 * hit)), outline=(30, 20, 20))
    label = f'{T.fmt_sec(d)} S LATE' if d > 0 else 'TOGETHER!'
    texts.append((label, 180, y0 + 62, CREAM if d > 0 else AMBER, 2, 'c'))


def _next_panel(dr, texts, t, y0):
    """the second cell: three flashes, each counted to its thunder: 3 S, 2 S, 1 S"""
    texts.append(('FLASH -> THUNDER', 180, y0 + 7, SKYB, 1, 'c'))
    for c, i in enumerate(T.CELL2):
        tf, d = T.STRIKES[i][:2]
        cx = 66 + c * 114
        if c:
            dr.line([cx - 57, y0 + 22, cx - 57, y0 + 76], fill=(50, 62, 98))
        lit = t >= tf
        bolt_icon(dr, cx - 5, y0 + 17, 10, (255, 230, 120) if lit else (60, 68, 100))
        if not lit:
            texts.append(('-', cx, y0 + 42, (70, 80, 112), 3, 'c'))
            continue
        arrived = t >= tf + d
        shown = d if arrived else t - tf
        hit = math.exp(-max(0.0, t - (tf + d)) / 0.2) if arrived else 0.0
        texts.append((f'{T.fmt_num(round(shown, 1)) if arrived else f"{shown:.1f}"} S', cx, y0 + 38 - int(2 * hit), AMBER if arrived else CREAM, 3, 'c'))
        if arrived:
            texts.append((T.km_text(d), cx, y0 + 66, CYAN, 1, 'c'))


def draw_hud(out, t):
    pil = Image.fromarray(out)
    dr = ImageDraw.Draw(pil)
    texts = []
    for (a, b), fn in ((T.HUD_COUNT, _count_panel), (T.HUD_GRID, _grid_panel), (T.HUD_NEXT, _next_panel)):
        if a - 0.4 <= t < b + 0.4:
            off = slide_offset(t, a, b, 170)
            y0 = 528 + off
            panel(dr, 14, y0, 346, y0 + 84)
            fn(dr, texts, t, y0)
    out = np.asarray(pil).copy()
    for tx in texts:
        if len(tx) == 6:
            center_text(out, tx[0], tx[1], tx[2], tx[3], tx[4])
        else:
            m = text_mask(tx[0], tx[4])
            stamp(out, m, tx[1], tx[2], tx[3])
    return out


def world_labels(out, cam, depth, t):
    for txt, P, D in S.MARKER_POS:
        zc = cam.depth(P)[0]
        if zc < 0.5:
            continue
        s = cam.wp(P)
        x, y = int(s[0]), int(s[1])
        if not (0 <= x < W and 0 <= y < H):
            continue
        if depth[y, x] < zc * 0.95:
            continue
        center_text(out, txt, x, y - 3, CREAM, 1)
    # the counted flashes get a tag where they hit the ground: "this one" -> its ring is amber
    for i in T.COUNTED:
        u = t - T.STRIKES[i][0]
        if not (0 <= u < 2.4) or T.ring_k(t) <= 0:
            continue
        P = S.STRIKE_P[i] + np.array([0, 0.6, 0])
        if cam.depth(P)[0] < 0.5:
            continue
        s = cam.wp(P)
        x, y = int(s[0]), int(s[1]) - 8
        if not (10 <= x < W - 10 and 20 <= y < H - 20):
            continue
        a = ss(0, 0.12, u) * (1 - ss(2.0, 2.4, u))
        if a < 0.05:
            continue
        col = tuple(int(c * a + 30 * (1 - a)) for c in AMBER)
        pil = Image.fromarray(out)
        ImageDraw.Draw(pil).polygon([(x - 4, y - 6), (x + 4, y - 6), (x, y)], fill=col, outline=(20, 16, 10))
        out[:] = np.asarray(pil)
        center_text(out, 'THIS ONE', x, y - 16, col, 1)


def rewind_fx(out, t):
    """VHS-style rewind: cool desaturated tint, two tracking bands running down, scanlines, REW tag"""
    k = ss(T.T_CO, T.T_CO + 0.2, t) * (1 - ss(T.T_REW - 0.16, T.T_REW, t))
    if k <= 0.01:
        return out
    img = out.astype(np.float32)
    g = img.mean(2, keepdims=True)
    img = img * (1 - 0.35 * k) + g * 0.35 * k + np.array([-8.0, 0.0, 16.0])[None, None] * k
    img[1::2] *= 1 - 0.07 * k
    for j in range(2):
        yb = int((t * 640 + j * 330) % 720) - 40
        hb = 12 + 6 * j
        for y in range(max(0, yb), min(H, yb + hb)):
            sh = int(round(k * (5 + 4 * math.sin(y * 0.9 + t * 40))))
            img[y] = np.roll(img[y], sh, axis=0)
            noise = (hash2(np.arange(W, dtype=np.float32), y + math.floor(t * 30)) > 0.55) * 34 * k
            img[y] += noise[:, None]
    out = np.clip(img, 0, 255).astype(np.uint8)
    a = min(1.0, k * 1.2)
    col = tuple(int(c * a) for c in CREAM)
    pil = Image.fromarray(out)
    dr = ImageDraw.Draw(pil)
    for dx in (0, 11):
        dr.polygon([(28 + dx, 14), (28 + dx, 26), (18 + dx, 20)], fill=col, outline=(10, 14, 24))
    out = np.asarray(pil).copy()
    m = text_mask('REW', 2)
    stamp(out, m, 46, 14, col, outline=(10, 14, 24))
    return out


# ---------------------------------------------------------------- render
def scene_state(t):
    cam = camera(t)
    flashes = [(i, T.flash_env(t, i)) for i in range(T.N_STRIKES) if T.flash_env(t, i) > 0.004]
    flashes.sort(key=lambda a: -a[1])
    return cam, flashes


def focus(cam):
    zf = cam.depth(S.WINDOW_C)[0]
    readouts = np.array([[S.RX0, 0, S.RZ1], [S.RX1, 0, S.RZ1], [S.RX0, S.WALL_H, S.RZ0], [S.RX1, S.WALL_H, S.RZ0],
                         [S.RX0, 0, S.RZ0], [S.RX1, 0, S.RZ0], S.CAT_SILL + [0, 0.35, 0], S.LAMP_P])
    zr = cam.depth(readouts)
    sharp = float(np.clip(np.abs(1 - zf / np.maximum(zr, 0.3)).max() + 0.03, 0.05, 0.9))
    K = 0.011 * cam.f * min(1.2, 5.0 / zf)
    return zf, sharp, K


def render(f):
    t = f / FPS
    cam, flashes = scene_state(t)
    rk = T.rain(t)
    z_hor, h_hor = sky_cols(t)
    Lx = lights_out(t, cam, flashes)
    Li, lk, fk = lights_in(t, cam, flashes)
    fr, ms = draw_sky(cam, t, flashes)
    draw_far(fr, cam, t, h_hor)
    ground_pass(fr, cam, t, Lx, flashes, z_hor, h_hor)
    # storm cells: the main one, the two far flanks (chaos), the second cell racing in for the loop
    cells = [(S.cell_ground(t), S.main_cell_alpha(t))] + S.flank_cells(t)
    if t >= 51.8:
        cells.append((S.cell2_ground(t), S.cell2_alpha(t)))
    cells.sort(key=lambda c: -np.linalg.norm(c[0] - cam.pos))
    for base, al in cells:
        draw_rain_curtain(fr, cam, t, base, al, max(rk, 0.25 * al))
        draw_cell(fr, cam, t, base, al, flashes, h_hor, z_hor, Lx)
    # outdoor objects (painter), then rain
    items = [(np.linalg.norm(o.ctr - cam.pos), 'o', o) for o in S.OBJS_OUT]
    items.append((np.linalg.norm(S.WM_HUB - cam.pos) - 0.3, 'fan', None))
    items.append((np.linalg.norm(S.VANE_P - cam.pos) - 0.1, 'vane', None))
    items.append((np.linalg.norm(np.array([0, 0, -12.0]) - cam.pos), 'wires', None))
    tf_ = tuft_faces(t)
    for j in range(0, len(tf_), 3):
        c = tf_[j][0].mean(0)
        items.append((np.linalg.norm(c - cam.pos), 'tuft', tf_[j:j + 3]))
    for _, kind, o in painter(items):
        if kind == 'o':
            draw_obj(fr, cam, Lx, o, False, emit=0.0)
        elif kind == 'fan':
            draw_obj(fr, cam, Lx, Obj(S.windmill_fan(windmill_angle(t)), (170, 170, 178), 'fan', grp=S.WM_G, double=True), False)
        elif kind == 'vane':
            draw_obj(fr, cam, Lx, Obj(vane_faces(t), (40, 40, 46), 'vane', grp=S.BARN_G, double=True), False)
        elif kind == 'wires':
            draw_wires(fr, cam, Lx)
        elif kind == 'tuft':
            draw_obj(fr, cam, Lx, Obj(o, (86, 124, 64), 'tuft', grp=-30, double=True, cast=False), False)
    draw_rain(fr, cam, t, rk)
    # the cabin
    cf = cat_faces(t)
    cmask_m = directional_mask(INFL, [Q for Q, n in cf], MOON_L, blur=1.0) if T.moon_k(t) > 0.01 else np.zeros((INFL.H, INFL.W), np.float32)
    cmask_f = directional_mask(INFL, [Q for Q, n in cf], FLASH_D[flashes[0][0]], blur=1.0) if flashes else np.zeros((INFL.H, INFL.W), np.float32)
    shell_pass(fr, cam, t, Li, lk, fk, flashes, (cmask_m, cmask_f), rk)
    cabin_edges(fr, cam, t, Li, lk)
    glows = []
    items = [(np.linalg.norm(o.ctr - cam.pos), 'o', o) for o in S.OBJS_IN]
    items.append((np.linalg.norm(S.RC + [0, 0.5, 0] - cam.pos), 'chair', None))
    cpos = cat_state(t)[0]
    items.append((np.linalg.norm(cpos + [0, 0.12, 0] - cam.pos) - 0.06, 'cat', None))
    items.append((np.linalg.norm(S.FIRE_P - cam.pos) - 0.05, 'fire', None))
    for _, kind, o in painter(items):
        if kind == 'o':
            em = 0.0
            if o.name == 'lamp_glass':
                em = 0.75 * lk
            elif o.name == 'firebox':
                em = 0.25 * fk
            draw_obj(fr, cam, Li, o, True, emit=em)
        elif kind == 'chair':
            draw_obj(fr, cam, Li, Obj(S.rocking_chair(T.phase(t)), (120, 76, 46), 'chair', grp=S.RC_G), True)
        elif kind == 'cat':
            draw_obj(fr, cam, Li, Obj(cf, (206, 124, 62), 'cat', grp=-11), True)
        elif kind == 'fire':
            draw_fire(fr, cam, t, fk, glows)
    s = cam.wp(S.LAMP_P)
    zl = cam.depth(S.LAMP_P)[0]
    glows.append((s[0], s[1], 0.6 * cam.f / max(zl, 0.5), (255, 176, 90), 0.5 * lk))
    if ms is not None and T.moon_k(t) > 0.02:
        glows.append((ms[0], ms[1], 0.09 * cam.f, (130, 150, 200), 0.25 * T.moon_k(t)))
    # lens: focus locked on the window + cat; the sharp band covers the whole room
    img, depth = fr.array(), fr.depth()
    zf, sharp, K = focus(cam)
    coc = coc_map(depth, zf, K, sharp=sharp)
    img = dof(img, coc)
    for i, env in flashes:
        sky = (depth > 5e5).astype(np.float32)[..., None]
        img = img + sky * env * T.flash_gain(i) * np.array([26, 28, 44.0])[None, None]
    draw_smoke(img, cam, depth, t)
    draw_bolts(img, depth, cam, flashes, glows)
    if T.moon_k(t) > 0.02:
        beam = window_beam(cam, depth, MOON_L)
        img = img + beam[..., None] * np.array([96, 112, 150.0])[None, None] * 0.22 * T.moon_k(t)
    if flashes and flashes[0][1] > 0.05:
        i, env = flashes[0]
        beam = window_beam(cam, depth, FLASH_D[i])
        img = img + beam[..., None] * np.array([150, 156, 200.0])[None, None] * 0.35 * env * T.flash_gain(i)
    img = apply_glows(img, fr.glows + glows)
    img = grade(bloom(np.clip(img, 0, 255), thr=210, k=0.35))
    img = np.clip(img * vignette(0.7, 0.62, 0.35) * top_scrim(), 0, 255)
    out = rewind_fx(img.astype(np.uint8), t)
    world_labels(out, cam, depth, t)
    out = draw_hud(out, t)
    draw_captions(out, t, T.CAPTIONS)
    if os.environ.get('COC_CHECK'):
        render.last = (zf, sharp, K)
    return out
