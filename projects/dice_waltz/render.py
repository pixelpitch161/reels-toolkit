"""Mozart's Dice Waltz - renderer.

    python -m reelkit.sheet render.py test/sheet.png 1 6 9.5 14 18 22 26 31 37 43 49 56
    python -m reelkit.video render.py audio.wav dice_waltz.mp4

Pipeline per frame: void sky -> ray-traced room shell (parquet floor with the moon pool through
the window, candle light + real shadow masks, wallpaper walls, the window backdrop) -> objects in
painter order (shadowed by the moon through the window + AABB occluders, rim-lit) with overlays
(staff notes, histogram, calendar, keys) -> dice/cups -> locked depth of field -> glows ->
volumetric moonbeam -> bloom -> grade -> vignette/scrim -> captions/HUD."""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, ss, ease, nrm, mixc
from reelkit.camera import Cam, interp_keys
from reelkit.raster import Frame, painter, apply_glows
from reelkit.lighting import Lights, shade, sky_gradient
from reelkit.shadows import MaskGrid, directional_mask, point_mask, Occluders, contact_ao
from reelkit.lens import coc_map, dof
from reelkit.post import bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, panel, slide_offset, MINT, CREAM, AMBER, CORAL, SKYB, INK
from reelkit.font import text_mask, stamp
from reelkit.geometry import Obj, box, prism, face, icosa
import timeline as T
import scene as S
import waltz_table as WT

# ---------------------------------------------------------------- lights
MOON_D0 = nrm([-0.878, 0.368, -0.306])          # through the window, across the tray, onto the floor
MOON_RGB = np.array([0.62, 0.72, 1.0])
WARM = np.array([1.0, 0.64, 0.32])
LAMPS = [  # (position, radius, strength) clustered candle lights
    (np.mean(S.CANDLES[0:3], 0), 1.5, 1.0),       # candelabra on the fortepiano
    (np.mean(S.CANDLES[3:6], 0), 1.7, 1.15),      # standing candelabra, front left
    (S.CANDLES[6], 1.0, 0.7),                     # sconce left
    (S.CANDLES[7], 1.0, 0.7),                     # sconce right
]
FLOOR = MaskGrid((S.RX0, S.RX1, S.RZ0, S.RZ1, 0.02))
STATIC_CAST = [o for o in S.OBJS if o.cast]
CAST_FACES = [Q for o in STATIC_CAST for Q, n in o.faces]
OCC = Occluders(S.OBJS)
MOON_MASK0 = directional_mask(FLOOR, CAST_FACES, MOON_D0, blur=1.2)
LAMP_GRPS = [S.OBJS[[i for i, o in enumerate(S.OBJS) if o.name == 'candelabra'][0]].grp,
             S.OBJS[[i for i, o in enumerate(S.OBJS) if o.name == 'floor_candelabra'][0]].grp, -1, -1]
LAMP_MASKS = [point_mask(FLOOR, [Q for o in STATIC_CAST if o.grp != LAMP_GRPS[i] and o.name not in ('sconce',) for Q, n in o.faces], lp, blur=3.0)
              for i, (lp, R, k) in enumerate(LAMPS)]

# ---------------------------------------------------------------- camera
TC = S.SUBJECT
KEYS = [  # (t, target, dist, yaw, pitch, fov, sy)   target = what sits at screen y = sy
    (0.0, np.array([-0.62, 0.98, -0.62]), 4.7, 32, 21, 40, 450),        # hook: tray low, staff above, moon window left
    (3.6, np.array([-0.58, 0.97, -0.6]), 4.5, 34, 21, 40, 450),
    (6.8, np.array([-0.05, 0.6, 0.1]), 7.2, 20, 27, 42, 400),           # establishing: whole salon, chart unrolls
    (8.6, np.array([0.0, 0.62, 0.15]), 5.6, 14, 32, 42, 420),           # demo roll: tray, chart row 7, staff
    (11.8, np.array([0.05, 0.5, 0.3]), 5.3, 10, 38, 42, 430),
    (15.4, np.array([0.1, 0.3, 0.6]), 4.4, 22, 54, 42, 400),            # drift over the grid
    (17.0, np.array([-0.55, 0.9, -0.75]), 4.8, 12, 20, 42, 440),        # histogram + tray
    (19.8, np.array([-0.5, 0.9, -0.7]), 4.9, 15, 22, 42, 440),
    (21.4, np.array([0.0, 0.6, 0.05]), 6.1, 16, 29, 42, 420),           # chart columns 8 / 16 + staff
    (27.4, np.array([0.0, 0.65, -0.05]), 5.9, 20, 27, 42, 430),
    (29.2, np.array([-1.15, 1.3, -0.15]), 5.4, 46, 12, 42, 400),         # window + calendar time-lapse
    (34.0, np.array([-1.1, 1.3, -0.15]), 5.2, 42, 13, 42, 400),
    (36.2, np.array([-0.05, 0.75, -0.25]), 6.3, 15, 25, 42, 440),       # two cups
    (40.6, np.array([0.0, 0.45, 0.1]), 7.0, 18, 30, 42, 430),           # ten cups on the floor
    (45.8, np.array([0.0, 0.45, 0.05]), 6.7, 22, 30, 42, 430),
    (51.6, np.array([-0.15, 0.75, -0.35]), 5.8, 18, 25, 42, 440),
    (55.4, np.array([-0.55, 0.95, -0.58]), 5.1, 30, 21, 40, 445),
    (60.0, np.array([-0.62, 0.98, -0.62]), 4.7, 32, 21, 40, 450),
]

# ---------------------------------------------------------------- palette
PARCH = np.array([236, 214, 168], float)
INK_C = np.array([46, 32, 26], float)
GOLD = np.array([255, 196, 88], float)


def lamp_flicker(t, i):
    return 0.9 + 0.07 * math.sin(t * 11.0 + i * 1.7) * math.sin(t * 6.3 + i) + 0.03 * math.sin(t * 23.0 + 2 * i)


def moon_dir(t):
    u = T.moon_phase(t)
    if u <= 0:
        return MOON_D0
    k = (u * 5.0) % 1.0                    # five fast nights
    az = math.radians(-150 + 120 * k)      # sweeps across the window
    el = math.radians(8 + 30 * math.sin(math.pi * k))
    d = np.array([math.cos(el) * math.cos(az), math.sin(el), math.cos(el) * math.sin(az)])
    w = ss(0.0, 0.06, u) * (1 - ss(0.94, 1.0, u))
    return nrm(MOON_D0 * (1 - w) + d * w)


def make_lights(t, cam, d):
    L = Lights(amb=(0.36, 0.33, 0.42), sky=(0.06, 0.06, 0.1))
    L.view = cam.view
    L.key('moon', d, MOON_RGB * 0.95)
    L.fill(nrm([0.6, 0.5, 0.8]), (0.1, 0.09, 0.12))
    ks = []
    for i, (lp, R, k) in enumerate(LAMPS):
        kk = k * lamp_flicker(t, i)
        ks.append(kk)
        L.point(lp, WARM * kk * 0.9, R)
    return L, ks


# ---------------------------------------------------------------- procedural textures
def hash2(a, b):
    v = np.sin(a * 127.1 + b * 311.7) * 43758.5453
    return v - np.floor(v)


def floor_albedo(X, Z):
    """parquet planks (along x, staggered) + oval rug under the table"""
    pw = 0.11
    row = np.floor(Z / pw)
    off = hash2(row, 3.0) * 0.6
    plank = np.floor((X + off) / 0.62)
    h = hash2(row, plank)
    base = np.array([150, 92, 54], float)[None] * (0.86 + 0.22 * h)[:, None]
    base *= (0.97 + 0.05 * np.sin(X * 37 + row * 3.1))[:, None]
    seam = (np.abs(((Z / pw) % 1.0) - 0.5) > 0.46) | (np.abs((((X + off) / 0.62) % 1.0) - 0.5) > 0.493)
    base[seam] *= 0.72
    # rug
    rx, rz = (X - S.TABLE_C[0]) / 0.92, (Z - S.TABLE_C[2]) / 0.66
    rr = np.sqrt(rx * rx + rz * rz)
    rug = rr < 1.0
    rc = np.where((rr > 0.86)[:, None], np.array([214, 168, 84.0])[None], np.array([132, 34, 42.0])[None])
    rc = np.where(((rr > 0.78) & (rr <= 0.86))[:, None], np.array([70, 24, 34.0])[None], rc)
    med = (np.abs(np.sin(np.arctan2(rz, rx) * 8)) * 0.25 + 0.35 > rr) & (rr < 0.6)
    rc = np.where(med[:, None], np.array([176, 112, 70.0])[None], rc)
    base[rug] = rc[rug]
    return base


def wall_albedo(U, Y, which):
    """U = coordinate along the wall. Wainscot below the chair rail, striped damask above, crown."""
    n = len(U)
    col = np.zeros((n, 3))
    paper = np.where(((np.floor(U / 0.14)) % 2 == 0)[:, None], np.array([88, 120, 112.0])[None], np.array([78, 108, 102.0])[None])
    # damask diamonds
    du = (U / 0.14) % 1.0 - 0.5
    dy = (Y / 0.2) % 1.0 - 0.5
    dia = (np.abs(du) + np.abs(dy) * 0.7) < 0.18
    paper = np.where(dia[:, None], paper * 1.12 + np.array([10, 8, 0])[None], paper)
    col[:] = paper
    wain = Y < S.RAIL_Y
    pu = (U / 0.42) % 1.0
    py = Y / S.RAIL_Y
    panel_in = (pu > 0.1) & (pu < 0.9) & (py > 0.2) & (py < 0.85)
    wc = np.where(panel_in[:, None], np.array([120, 74, 48.0])[None], np.array([104, 62, 40.0])[None])
    edge = panel_in & ((pu < 0.13) | (pu > 0.87) | (py < 0.24) | (py > 0.81))
    wc = np.where(edge[:, None], np.array([140, 90, 58.0])[None], wc)
    col[wain] = wc[wain]
    rail = np.abs(Y - S.RAIL_Y) < 0.025
    col[rail] = [176, 128, 80]
    base = Y < 0.09
    col[base] = [70, 40, 28]
    crown = Y > S.CROWN_Y
    col[crown] = [226, 206, 170]
    col[crown & (np.abs(Y - S.CROWN_Y - 0.04) < 0.012)] = [190, 160, 118]
    return col


def backdrop(Yb, Zb, t, moon_yz, stars_k=1.0):
    """painted night sky over Vienna roofs, seen through the window"""
    n = len(Yb)
    k = np.clip((Yb - 0.6) / 3.2, 0, 1) ** 0.7
    col = np.array([70, 112, 160.0])[None] * (1 - k[:, None]) + np.array([22, 26, 74.0])[None] * k[:, None]
    # moon glow + disc
    my, mz = moon_yz
    dm = np.sqrt((Yb - my) ** 2 + (Zb - mz) ** 2)
    col += (np.clip(1 - dm / 1.5, 0, 1) ** 2)[:, None] * np.array([70, 80, 100.0])[None]
    disc = dm < 0.34
    col[disc] = [244, 238, 214]
    crater = disc & (hash2(np.floor(Yb * 14), np.floor(Zb * 14)) > 0.82)
    col[crater] = [222, 214, 190]
    # stars (twinkle)
    gy, gz = np.floor(Yb * 7), np.floor(Zb * 7)
    hs = hash2(gy, gz)
    fy, fz = (Yb * 7) % 1 - 0.5, (Zb * 7) % 1 - 0.5
    star = (hs > 0.93) & (np.abs(fy) < 0.09) & (np.abs(fz) < 0.09) & (Yb > 1.3) & ~disc
    tw = 0.6 + 0.4 * np.sin(t * (3 + 5 * hs) + hs * 40)
    col[star] = (np.array([255, 250, 230.0])[None] * tw[star, None] * stars_k + col[star] * (1 - tw[star, None] * stars_k))
    # clouds drifting across
    cl = np.sin(Zb * 1.3 + t * 0.12) * 0.25 + np.sin(Zb * 3.1 - t * 0.2) * 0.12 + 2.9
    cloud = np.abs(Yb - cl) < 0.09 + 0.05 * np.sin(Zb * 5 + t * 0.3)
    col[cloud] = col[cloud] * 0.6 + np.array([120, 128, 170.0]) * 0.4
    # roofs: base height + chimneys + St. Stephen's spire + a dome
    hroof = 0.55 + 0.18 * np.sin(Zb * 2.2) + 0.12 * (np.floor(Zb * 1.6) % 2) + 0.06 * hash2(np.floor(Zb * 3), 9.0)
    sp = np.clip(1 - np.abs(Zb + 2.6) / 0.22, 0, 1)
    hroof = np.maximum(hroof, 0.6 + 2.1 * sp ** 1.6)
    dome = np.clip(1 - ((Zb - 1.2) / 0.55) ** 2, 0, None)
    hroof = np.maximum(hroof, 0.75 + 0.5 * np.sqrt(dome))
    roof = Yb < hroof
    col[roof] = [30, 34, 62]
    win_l = roof & (hash2(np.floor(Yb * 9), np.floor(Zb * 9)) > 0.9) & (Yb < hroof - 0.12)
    col[win_l] = [255, 196, 110]
    return col


# ---------------------------------------------------------------- the room shell, per pixel
def shell_pass(fr, cam, t, L, ks, d, moon_mask, chart):
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
    winpx = okl & S.win_open(Yl, Zl)
    tf = np.where(okf, tf, INF)
    tb = np.where(okb, tb, INF)
    tls = np.where(okl & ~winpx, tl, INF)
    tw = np.where(winpx, tl, INF)
    tmin = np.minimum(np.minimum(tf, tb), np.minimum(tls, tw))
    mf = okf & (tf <= tmin)
    mb = okb & (tb <= tmin) & ~mf
    ml = okl & ~winpx & (tls <= tmin) & ~mf & ~mb
    mw = winpx & (tw <= tmin) & ~mf & ~mb

    def lamps_at(P, nrmv, mask_floor=None):
        I = np.zeros((len(P), 3))
        for i, (lp, R, k) in enumerate(LAMPS):
            v = lp[None] - P
            dist = np.linalg.norm(v, axis=1) + 1e-6
            lam = np.clip((v * nrmv[None]).sum(1) / dist, 0, None) * 0.85 + 0.15
            sh = 1.0 if mask_floor is None else (1 - FLOOR.sample(LAMP_MASKS[i], P[:, 0], P[:, 2]) * 0.85)
            I += (lam * sh * ks[i] / (1 + (dist / R) ** 2))[:, None] * WARM[None] * 1.05
        return I

    # floor
    ys, xs = np.nonzero(mf)
    if len(xs):
        X, Z = Xf[ys, xs], Zf[ys, xs]
        P = np.stack([X, np.zeros_like(X), Z], 1)
        alb = floor_albedo(X, Z)
        emis = np.zeros_like(alb)
        if chart is not None:
            alb, emis = chart(X, Z, alb, emis)
        I = np.tile(L.amb + L.sky, (len(X), 1))
        mlit = S.win_lit(P + [0, 0.002, 0], d) * (1 - FLOOR.sample(moon_mask, X, Z))
        I += (mlit * max(0.0, d[1]) * 2.1)[:, None] * MOON_RGB[None]
        I += lamps_at(P, np.array([0, 1.0, 0]), True)
        ao = contact_ao(X, Z, S.FEET)
        wall_ao = 1 - 0.3 * np.exp(-(X - S.RX0) / 0.25) - 0.3 * np.exp(-(Z - S.RZ0) / 0.25)
        I *= (ao * wall_ao)[:, None]
        col = alb * I + emis
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tf[ys, xs]
    # walls
    for m, U_, Y_, tt, nrmv, which in ((mb, Xb, Yb, tb, np.array([0, 0, 1.0]), 'back'), (ml, Zl, Yl, tl, np.array([1.0, 0, 0]), 'left')):
        ys, xs = np.nonzero(m)
        if not len(xs):
            continue
        U, Y = U_[ys, xs], Y_[ys, xs]
        P = np.stack([U, Y, np.full_like(U, S.RZ0)], 1) if which == 'back' else np.stack([np.full_like(U, S.RX0), Y, U], 1)
        alb = wall_albedo(U, Y, which)
        I = np.tile(L.amb * 0.95, (len(U), 1))
        I += lamps_at(P, nrmv)
        if which == 'back':
            I += (np.clip(nrmv @ L.fills[0][0], 0, None) * L.fills[0][1])[None]
        ao = 1 - 0.35 * np.exp(-Y / 0.2)
        ao *= 1 - 0.28 * np.exp(-((U - S.RX0) if which == 'back' else (U - S.RZ0)) / 0.3)
        col = alb * I * ao[:, None]
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tt[ys, xs]
    # window: backdrop through the glass, mullions, reveal
    ys, xs = np.nonzero(mw)
    moon_scr = None
    if len(xs):
        Yw, Zw = Yl[ys, xs], Zl[ys, xs]
        d2 = dw[ys, xs]
        tr = (S.RX0 - S.WT - p[0]) / d2[:, 0]
        Yr, Zr = p[1] + tr * d2[:, 1], p[2] + tr * d2[:, 2]
        reveal = ~S.win_open(Yr, Zr)
        mull = S.win_mullion(Yw, Zw)
        tbd = (S.BACKDROP_X - p[0]) / d2[:, 0]
        Ybd, Zbd = p[1] + tbd * d2[:, 1], p[2] + tbd * d2[:, 2]
        # the painted moon sits in the upper half of the arch for every camera (stage backdrop)
        u = T.moon_phase(t)
        if u > 0:
            k = (u * 5.0) % 1.0
            wz = S.WIN_Z0 + 0.1 + (S.WIN_Z1 - S.WIN_Z0 - 0.2) * k
            wy = S.WIN_Y1 - 0.25 + 0.5 * math.sin(math.pi * k)
        else:
            wz, wy = S.WIN_ZC - 0.12, S.WIN_Y1 + 0.12
        mpt = np.array([S.RX0, wy, wz])
        dirm = mpt - p
        s_ = (S.BACKDROP_X - p[0]) / dirm[0]
        moon_yz = (p[1] + s_ * dirm[1], p[2] + s_ * dirm[2])
        col = backdrop(Ybd, Zbd, t, moon_yz)
        col = col * 0.92 + np.array([6, 10, 18.0])
        frame_c = np.array([214, 200, 176.0])
        Iw = (L.amb * 0.6 + np.array([0.1, 0.1, 0.14]))[None] + lamps_at(np.stack([np.full_like(Yw, S.RX0), Yw, Zw], 1), np.array([1.0, 0, 0]))
        col[mull] = (frame_c[None] * Iw)[mull]
        rc = np.array([206, 190, 166.0])[None] * (Iw * 0.9 + 0.12)
        col[reveal & ~mull] = rc[reveal & ~mull]
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = np.where(reveal | mull, tl[ys, xs], tbd * 1.0)
        moon_scr = cam.wp(np.array([S.BACKDROP_X, moon_yz[0], moon_yz[1]]))
    fr.set_array(arr, depth)
    return moon_scr


# ---------------------------------------------------------------- exterior of the cut-away (wall tops, slab edges)
def exterior_polys(cam, fr):
    top = (196, 176, 150)
    edge = (62, 40, 30)
    polys = [
        ([[S.RX0 - S.WT, S.WALL_H, S.RZ0 - S.WT], [S.RX1, S.WALL_H, S.RZ0 - S.WT], [S.RX1, S.WALL_H, S.RZ0], [S.RX0 - S.WT, S.WALL_H, S.RZ0]], top),
        ([[S.RX0 - S.WT, S.WALL_H, S.RZ0], [S.RX0, S.WALL_H, S.RZ0], [S.RX0, S.WALL_H, S.RZ1], [S.RX0 - S.WT, S.WALL_H, S.RZ1]], top),
        ([[S.RX1, 0, S.RZ0 - S.WT], [S.RX1, S.WALL_H, S.RZ0 - S.WT], [S.RX1, S.WALL_H, S.RZ0], [S.RX1, 0, S.RZ0]], (150, 132, 112)),
        ([[S.RX0 - S.WT, 0, S.RZ1], [S.RX0, 0, S.RZ1], [S.RX0, S.WALL_H, S.RZ1], [S.RX0 - S.WT, S.WALL_H, S.RZ1]], (150, 132, 112)),
        ([[S.RX0 - S.WT, -S.SLAB, S.RZ1], [S.RX1, -S.SLAB, S.RZ1], [S.RX1, 0, S.RZ1], [S.RX0 - S.WT, 0, S.RZ1]], edge),
        ([[S.RX1, -S.SLAB, S.RZ1], [S.RX1, -S.SLAB, S.RZ0 - S.WT], [S.RX1, 0, S.RZ0 - S.WT], [S.RX1, 0, S.RZ1]], (48, 30, 24)),
    ]
    for Q, c in polys:
        Q = np.array(Q, float)
        if np.dot(face(Q)[1], cam.pos - Q.mean(0)) < 0 and Q[:, 1].max() > 0.5 * S.WALL_H and Q[:, 1].min() > 0.5 * S.WALL_H:
            pass
        fr.poly(cam, Q, c)
    # a thin gold trim along the slab top edge
    fr.line(cam, [[S.RX0 - S.WT, 0.0, S.RZ1], [S.RX1, 0.0, S.RZ1]], (190, 150, 80), 1)
    fr.line(cam, [[S.RX1, 0.0, S.RZ1], [S.RX1, 0.0, S.RZ0 - S.WT]], (160, 124, 66), 1)


# ---------------------------------------------------------------- object shading
def draw_obj(fr, cam, L, d, o, emit=0.0, extra_vis=None, alpha_col=None):
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
    kv = (S.win_lit(Pv, d) * OCC.vis(Pv, o.grp, d=d))[:, None]
    base = np.array([o.color * o.tone[j] for *_, j in fc])
    cols = shade(L, N, C, base, kvis=kv, emit=emit)
    for (z, Q, n, j), c in zip(fc, cols):
        fr.poly(cam, Q, c)


# ---------------------------------------------------------------- dice + cups
def die_traj(r, t, start, rest, yaw):
    """pre-baked tumble: arc from the cup to the first contact, two bounces, slide to rest"""
    t0, t1 = r.t_throw, r.t_land
    first = rest + (start - rest) * np.array([0.28, 0, 0.28])
    first[1] = rest[1]
    if t <= t1:
        u = (t - t0) / (t1 - t0)
        P = start + (first - start) * u
        P[1] = start[1] + (first[1] - start[1]) * u + 0.16 * 4 * u * (1 - u)
    else:
        v = min(1.0, (t - t1) / 0.36)
        P = first + (rest - first) * (1 - (1 - v) ** 2)
        P[1] = rest[1] + 0.045 * abs(math.sin(math.pi * 2 * v)) * (1 - v) ** 2
    w = min(1.0, max(0.0, (t - t0) / (t1 + 0.36 - t0)))
    return P, w




def draw_die(fr, cam, L, d, P, R, scale=1.0, tint=None):
    faces = []
    for Q, n, v, a, b in S.DIE_FACES:
        Qw = (Q * scale) @ R.T + P
        nw = R @ n
        if np.dot(nw, cam.pos - Qw.mean(0)) <= 0:
            continue
        faces.append((cam.depth(Qw.mean(0))[0], Qw, nw, v, R @ a, R @ b))
    if not faces:
        return
    faces.sort(key=lambda f: -f[0])
    C = np.array([f[1].mean(0) for f in faces])
    N = np.array([f[2] for f in faces])
    kv = (S.win_lit(C + N * 0.01, d) * OCC.vis(C + N * 0.01, -5, d=d))[:, None]
    base = np.array([252, 244, 226.0]) if tint is None else np.array(tint, float)
    cols = shade(L, N, C, np.tile(base, (len(faces), 1)), kvis=kv, rimk=0.7)
    pipc = shade(L, N, C, np.tile([40, 30, 34.0], (len(faces), 1)), kvis=kv, rimk=0.2)
    h = S.DIE / 2 * scale
    zf = faces[0][0]
    px = cam.f * S.DIE * scale / max(zf, 0.1)
    for (z, Qw, nw, v, aw, bw), c, pc in zip(faces, cols, pipc):
        fr.poly(cam, Qw, c)
        if px < 6:
            continue
        ctr = Qw.mean(0) + nw * 0.0012
        r_ = h * 0.2
        for (i, j) in S.PIPS[v]:
            pc3 = ctr + (aw * i + bw * j) * h * 0.5
            q = np.array([pc3 + (aw + bw) * r_ * 0.7, pc3 + (aw - bw) * r_ * 0.7, pc3 - (aw + bw) * r_ * 0.7, pc3 - (aw - bw) * r_ * 0.7])
            fr.poly(cam, q, pc)


# ---------------------------------------------------------------- staff (parchment on the back wall)
LETTER = {0: 0, 2: 1, 4: 2, 5: 3, 7: 4, 9: 5, 11: 6}


def diatonic(m):
    pc = m % 12
    sharp = pc not in LETTER
    if sharp:
        pc -= 1
    return (m // 12 - 1) * 7 + LETTER[pc] - 30, sharp      # 0 = E4 (bottom line)


def staff_overlay(fr, cam, t, L, ks, glows):
    """parchment, staff lines, clefs, bars as they land, the playhead"""
    x0, x1, y0, y1, z = S.STAFF_X0, S.STAFF_X1, S.STAFF_Y0, S.STAFF_Y1, S.STAFF_Z
    # parchment with lighting from the sconces
    Pc = np.array([0.0, 0.5 * (y0 + y1), z])
    I = L.amb * 1.05 + sum(WARM * ks[i] * 1.1 / (1 + (np.linalg.norm(LAMPS[i][0] - Pc) / LAMPS[i][1]) ** 2) for i in range(4))
    # 4x3 tiles so the warm falloff reads across it
    for i in range(6):
        for j in range(3):
            xa, xb = x0 + (x1 - x0) * i / 6, x0 + (x1 - x0) * (i + 1) / 6
            ya, yb = y0 + (y1 - y0) * j / 3, y0 + (y1 - y0) * (j + 1) / 3
            Pm = np.array([0.5 * (xa + xb), 0.5 * (ya + yb), z])
            Ii = L.amb * 1.05 + sum(WARM * ks[k] * 1.1 / (1 + (np.linalg.norm(LAMPS[k][0] - Pm) / LAMPS[k][1]) ** 2) for k in range(4))
            c = PARCH * (0.97 + 0.04 * hash2(np.array([i]), np.array([j]))[0]) * Ii
            fr.poly(cam, np.array([[xa, ya, z], [xb, ya, z], [xb, yb, z], [xa, yb, z]]), np.round(c / 5) * 5)
    ink = np.clip(INK_C * (I * 0.9 + 0.2), 0, 255)
    line_c = np.clip(PARCH * I * 0.62, 0, 255)
    for row in range(2):
        yb_ = S.SYS_BOTTOM[row]
        for k in range(5):
            yy = yb_ + k * S.LINE_DY
            fr.line(cam, [[S.BAR_X0 - 0.11, yy, z + 0.002], [S.BAR_X1, yy, z + 0.002]], line_c, 1)
        for b in range(9):
            xb = S.BAR_X0 + b * (S.BAR_X1 - S.BAR_X0) / 8
            fr.line(cam, [[xb, yb_, z + 0.002], [xb, yb_ + 4 * S.LINE_DY, z + 0.002]], line_c, 2 if b == 8 and row == 1 else 1)
        # treble clef: a small drawn glyph
        cx = S.BAR_X0 - 0.07
        s0 = cam.wp([cx, yb_ + 2 * S.LINE_DY, z])
        sp = abs(cam.wp([cx, yb_ + S.LINE_DY, z])[1] - s0[1])
        draw_clef(fr, s0, sp, tuple(int(c) for c in ink))
    # time signature
    ts = cam.wp([S.BAR_X0 - 0.02, S.SYS_BOTTOM[0] + 2 * S.LINE_DY, z])
    return ink, line_c


CLEF = ["..#..", ".#.#.", ".#.#.", "..##.", "..#..", ".##..", "#.#..", "#.###", "#.#.#", ".###.", "..#..", "#.#..", ".#..."]


def draw_clef(fr, s0, sp, col):
    hpx = max(9, int(round(sp * 4.2)))
    rows = len(CLEF)
    sc = hpx / rows
    x0 = s0[0] - 2.5 * sc * 0.8
    y0 = s0[1] - rows * sc * 0.55
    for r, line in enumerate(CLEF):
        for c, ch in enumerate(line):
            if ch == '#':
                xa, ya = x0 + c * sc * 0.8, y0 + r * sc
                fr.draw.rectangle([int(xa), int(ya), int(xa + max(1, sc * 0.8) - 1), int(ya + max(1, sc) - 1)], fill=col)


def draw_bar_notes(fr, cam, slot, bar, col, t_appear, t, outline=None, hl=0.0, z_off=0.004):
    """the bar's right-hand notes on the staff: heads, stems, beams, sharps, ledger lines"""
    rh, _ = WT.bar_notes(bar)
    u = min(1.0, (t - t_appear) / 0.28)
    drop = (1 - ease(u)) * 0.12 + 0.02 * math.sin(math.pi * u) * (u < 1)
    z = S.STAFF_Z + z_off
    groups = {}
    for o, dd, p in rh:
        groups.setdefault(o, []).append((dd, p))
    onsets = sorted(groups)
    heads = []
    for o in onsets:
        frac = (o + 0.5) / 6.0
        xw, yb = S.staff_xy(slot, frac)
        pts = []
        for dd, p in groups[o]:
            idx, sharp = diatonic(p)
            yw = yb + idx * S.LINE_DY / 2 + drop
            pts.append((idx, sharp, yw))
        heads.append((o, groups[o][0][0], xw, yb, pts))
    if not heads:
        return
    s_line = abs(cam.wp([0, S.SYS_BOTTOM[0], z])[1] - cam.wp([0, S.SYS_BOTTOM[0] + S.LINE_DY, z])[1])
    hw = max(2.0, s_line * 0.75)
    hh = max(1.5, s_line * 0.55)
    c = tuple(int(v) for v in col)
    oc = tuple(int(v) for v in outline) if outline is not None else None
    mean_idx = np.mean([pp[0] for h in heads for pp in h[4]])
    up = mean_idx < 4
    stem_len = 3.2 * s_line
    tips = []
    for o, dd, xw, yb, pts in heads:
        scr = [cam.wp([xw, yw, z]) for idx, sh, yw in pts]
        for (idx, sh, yw), s in zip(pts, scr):
            # ledger lines
            for li in list(range(-2, idx - 1, -2)) + list(range(10, idx + 1, 2)):
                ly = cam.wp([xw, yb + li * S.LINE_DY / 2 + drop, z])[1]
                fr.draw.line([s[0] - hw - 1, ly, s[0] + hw + 1, ly], fill=c)
            if oc is not None:
                fr.draw.ellipse([s[0] - hw - 1, s[1] - hh - 1, s[0] + hw + 1, s[1] + hh + 1], fill=oc)
            fr.draw.ellipse([s[0] - hw, s[1] - hh, s[0] + hw, s[1] + hh], fill=c)
            if sh:
                xs_ = s[0] - hw - 3
                fr.draw.line([xs_ - 1, s[1] - 1, xs_ + 2, s[1] - 1], fill=c)
                fr.draw.line([xs_ - 1, s[1] + 1, xs_ + 2, s[1] + 1], fill=c)
                fr.draw.line([xs_, s[1] - 2, xs_, s[1] + 2], fill=c)
                fr.draw.line([xs_ + 1, s[1] - 2, xs_ + 1, s[1] + 2], fill=c)
        ys_ = [s[1] for s in scr]
        sx = scr[0][0] + (hw - 0.5 if up else -hw + 0.5)
        if up:
            top, bot = min(ys_) - stem_len, max(ys_)
            tip = top
        else:
            top, bot = min(ys_), max(ys_) + stem_len
            tip = bot
        if dd <= 4:
            fr.draw.line([sx, top, sx, bot], fill=oc if oc is not None and False else c, width=1)
            tips.append((sx, tip, dd))
    # beams across consecutive short notes
    short = [tp for tp in tips if tp[2] <= 2]
    if len(short) >= 2:
        a, b = short[0], short[-1]
        for k in range(2 if all(tp[2] == 1 for tp in short) else 1):
            off = (k * 2.5) * (1 if up else -1)
            fr.draw.line([a[0], a[1] + off, b[0], b[1] + off], fill=c, width=2)
        if any(tp[2] == 1 for tp in short) and not all(tp[2] == 1 for tp in short):
            for i, tp in enumerate(short):
                if tp[2] == 1:
                    nb = short[i + 1] if i + 1 < len(short) else short[i - 1]
                    xm = tp[0] + (nb[0] - tp[0]) * 0.45
                    ym = tp[1] + (nb[1] - tp[1]) * 0.45 + 2.5 * (1 if up else -1)
                    fr.draw.line([tp[0], tp[1] + 2.5 * (1 if up else -1), xm, ym], fill=c, width=2)
    elif len(short) == 1:
        sx, tip, dd = short[0]
        fr.draw.line([sx, tip, sx + 3, tip + (3 if up else -3)], fill=c, width=1)


# ---------------------------------------------------------------- the floor chart (1792 table)
CELL_DOTS = {}
for r_ in range(11):
    for c_ in range(16):
        rh, _ = WT.bar_notes(WT.TABLE[r_][c_])
        pts = []
        for o, dd, p in sorted(rh)[:6]:
            idx, _ = diatonic(p)
            pts.append(((o + 0.5) / 6.0, np.clip(idx / 14.0, 0, 1)))
        CELL_DOTS[(r_, c_)] = pts
DOT_U = np.full((11, 16, 6), -9.0)
DOT_V = np.full((11, 16, 6), -9.0)
for (r_, c_), pts in CELL_DOTS.items():
    for k, (uu, vv) in enumerate(pts):
        DOT_U[r_, c_, k] = uu
        DOT_V[r_, c_, k] = vv


def chart_state(t):
    """how far the chart is unrolled (z of the roll) and its highlights"""
    a, b = T.T_UNFURL
    if t < a:
        zr = S.CHART_Z0 + 0.02
    else:
        zr = S.CHART_Z0 + 0.02 + (S.CHART_Z1 - S.CHART_Z0 - 0.02) * ease((t - a) / (b - a))
    if t > T.T_REROLL[0]:
        zr = S.CHART_Z0 + 0.02 + (zr - S.CHART_Z0 - 0.02) * (1 - ease((t - T.T_REROLL[0]) / (T.T_REROLL[1] - T.T_REROLL[0])))
    return zr


def chart_fn(t):
    zr = chart_state(t)
    hl_row = None
    if 8.85 <= t < 11.0:
        hl_row = (6, ss(8.85, 9.05, t) * (1 - ss(10.6, 11.0, t)))     # sum 7 -> row index 5? (sum-2)
    flashes = []
    for r in T.LEAD:
        if r.col is not None and r.t_land <= t < r.t_land + 0.9 and r.t_land < 30:
            flashes.append((r.sum - 2, r.col, 1 - (t - r.t_land) / 0.9))
    cols_glow = 0.0
    if T.T_QUAD + 0.4 <= t < T.T_SNAP + 2.5:
        cols_glow = ss(T.T_QUAD + 0.4, T.T_QUAD + 1.0, t) * (1 - ss(T.T_SNAP + 2.0, T.T_SNAP + 2.5, t))

    def fn(X, Z, alb, emis):
        inside = (X >= S.CHART_X0) & (X <= S.CHART_X1) & (Z >= S.CHART_Z0) & (Z <= zr)
        if not inside.any():
            return alb, emis
        Xi, Zi = X[inside], Z[inside]
        a2 = np.tile(PARCH, (len(Xi), 1)) * (0.94 + 0.08 * hash2(np.floor(Xi * 40), np.floor(Zi * 40)))[:, None]
        # darker burnt edge
        ed = np.minimum(np.minimum(Xi - S.CHART_X0, S.CHART_X1 - Xi), np.minimum(Zi - S.CHART_Z0, S.CHART_Z1 - Zi))
        a2 *= (0.72 + 0.28 * np.clip(ed / 0.05, 0, 1))[:, None]
        e2 = np.zeros_like(a2)
        gx = (Xi - S.GRID_X0) / S.CELL_W
        gz = (Zi - S.GRID_Z0) / S.CELL_D
        ing = (gx >= 0) & (gx < 16) & (gz >= 0) & (gz < 11)
        ci = np.clip(np.floor(gx).astype(int), 0, 15)
        ri = np.clip(np.floor(gz).astype(int), 0, 10)
        fu, fv = gx - np.floor(gx), gz - np.floor(gz)
        line = ing & ((fu < 0.05) | (fv < 0.07))
        a2[line] = a2[line] * 0.55 + INK_C * 0.45
        # header band + label column tint
        hdr = (Zi >= S.CHART_Z0 + 0.03) & (Zi < S.GRID_Z0) & (Xi >= S.GRID_X0)
        a2[hdr] *= 0.9
        # mini notes per cell (the real contour of each bar)
        if ing.any():
            uu = DOT_U[ri[ing], ci[ing]]
            vv = DOT_V[ri[ing], ci[ing]]
            du = (fu[ing][:, None] - uu) * S.CELL_W
            dv = ((1 - fv[ing])[:, None] * 0.8 + 0.1 - vv * 0.8) * S.CELL_D
            dmin = np.sqrt(du ** 2 + (dv * 1.0) ** 2).min(1)
            dot = dmin < 0.0085
            sub = a2[ing]
            sub[dot] = sub[dot] * 0.3 + INK_C * 0.7
            a2[ing] = sub
        # highlights (emissive)
        if hl_row is not None:
            rr, k = hl_row
            m = ing & (ri == rr - 1)
            e2[m] += np.array([90, 70, 20.0]) * k
            a2[m] = a2[m] * (1 - 0.25 * k) + GOLD * 0.25 * k
        for rr, cc, k in flashes:
            m = ing & (ri == rr) & (ci == cc)
            e2[m] += np.array([150, 120, 50.0]) * k
        if cols_glow > 0:
            for cc, colr in ((7, (120, 200, 255)), (15, (255, 150, 120))):
                m = ing & (ci == cc)
                e2[m] += np.array(colr, float) * 0.55 * cols_glow
        alb[inside] = a2
        emis[inside] = e2
        return alb, emis
    return fn


def chart_labels(img, cam, t, glows):
    """row labels 2..12 and column numbers, plus the roll"""
    zr = chart_state(t)
    for r in range(11):
        zc = S.GRID_Z0 + (r + 0.5) * S.CELL_D
        if zc > zr - 0.02:
            continue
        s = cam.wp([S.CHART_X0 + 0.075, 0.0, zc])
        txt = str(r + 2)
        col = (70, 44, 30)
        if 8.85 <= t < 11.0 and r == 5:
            col = (200, 110, 20)
        center_text(img, txt, s[0], s[1] - 3, col, 1, outline=None)
    if zr > S.GRID_Z0 - 0.02:
        for c in range(16):
            xc = S.GRID_X0 + (c + 0.5) * S.CELL_W
            s = cam.wp([xc, 0.0, S.CHART_Z0 + 0.06])
            col = (70, 44, 30)
            if T.T_QUAD + 0.4 <= t < T.T_SNAP + 2.5 and c in (7, 15):
                col = (40, 120, 200) if c == 7 else (200, 80, 50)
            center_text(img, str(c + 1), s[0], s[1] - 3, col, 1, outline=None)


def center_text(img, txt, x, y, col, scale=1, outline=(10, 20, 26)):
    m = text_mask(txt, scale)
    stamp(img, m, int(round(x - m.shape[1] / 2)), int(round(y)), col, outline=outline, shadow=outline is not None)




# ---------------------------------------------------------------- cups: where they live and how they tip
def cup_home(cup):
    """(base position, unit tip direction) for each cup"""
    if cup == 'A':
        return S.CUP_HOME.copy(), np.array([-1.0, 0, 0])
    if cup == 'B':
        return S.TRAY_C + np.array([-S.TRAY_HX - 0.1, -0.012, -0.06]), np.array([1.0, 0, 0])
    i = T.CUPS.index(cup) - 2                       # eight cups stand in an arc on the floor in front
    a = math.radians(-12 + i * (204 / 7.0))
    v = np.array([math.cos(a), 0, math.sin(a)])
    return np.array([S.TABLE_C[0], 0.0, S.TABLE_C[2]]) + v * np.array([1.0, 0, 0.92]), v


CUP_ROLLS = {c: [r for r in T.ROLLS if r.cup == c] for c in T.CUPS}
CUP_APPEAR = {c: (CUP_ROLLS[c][0].t_throw - 0.7 if c != 'A' else -1) for c in T.CUPS}
CUP_LEAVE = {c: (max(r.t_land for r in CUP_ROLLS[c]) + 0.9 if c != 'A' else 99) for c in T.CUPS}


def cup_state(cup, t):
    """base position, tilt (rad), tip azimuth and visibility (0..1 scale)"""
    pos, v = cup_home(cup)
    tdir = math.atan2(v[0], v[2])
    tilt, lift = 0.0, 0.0
    throws = [r.t_throw for r in CUP_ROLLS[cup]] + ([T.T_LOOP_THROW] if cup == 'A' else [])
    for t0 in throws:
        if t0 - 0.36 <= t <= t0 + 0.42:
            u = (t - (t0 - 0.36)) / 0.36
            if u < 1:
                lift = 0.09 * ease(u)
                tilt = 0.22 * math.sin(u * math.pi * 4)
            else:
                w = (t - t0) / 0.42
                tilt = 1.9 * math.sin(math.pi * min(1.0, w))
                lift = 0.09 * (1 - ease(w))
    if cup == 'A' and T.T_SHAKE <= t < T.T_LOOP_THROW - 0.36:
        u = t - T.T_SHAKE
        lift = 0.09 * ss(0, 0.3, u)
        tilt = 0.26 * math.sin(u * 24) * ss(0, 0.3, u)
    vis = 1.0
    if cup != 'A':
        vis = ease((t - CUP_APPEAR[cup]) / 0.35) * (1 - ease((t - CUP_LEAVE[cup]) / 0.4))
    return pos + np.array([0, lift, 0]), tilt, tdir, vis


def draw_cup(fr, cam, L, d, cup, t, glows):
    pos, tilt, tdir, vis = cup_state(cup, t)
    if vis <= 0.02:
        return
    body, inner, R = S.cup_faces(np.zeros(3), tilt, tdir)
    body = [(Q * vis + pos, n) for Q, n in body]
    inner = [(Q * vis + pos, n) for Q, n in inner]
    col = (122, 58, 40) if cup == 'A' else tuple(int(c * 0.5 + 50) for c in T.CUP_COLS[cup])
    draw_obj(fr, cam, L, d, Obj(body, col, 'cup', grp=-7))
    draw_obj(fr, cam, L, d, Obj(inner, (28, 16, 14), 'cupin', grp=-7, cast=False))
    rim = [(R @ np.array([0.094 * math.cos(a), 0.18, 0.094 * math.sin(a)])) * vis + pos for a in np.linspace(0, 6.283, 13)]
    if min(cam.depth(np.array(rim))) > 0.2:
        fr.line(cam, rim, (232, 188, 92) if cup == 'A' else T.CUP_COLS[cup], 1)
    if cup != 'A':
        s = cam.wp(pos + [0, 0.1, 0])
        glows.append((s[0], s[1], 0.22 * cam.f / max(cam.depth(pos)[0], 0.5), T.CUP_COLS[cup], 0.22 * vis))


def dice_world(r, t, which):
    """(position, rotation) of die 0/1 of roll r at time t"""
    rg = np.random.default_rng(r.seed * 7 + which)
    home, v = cup_home(r.cup)
    if r.where == 'tray':
        cx = S.TRAY_C[0] + (-0.12 if r.cup == 'A' else 0.1) + rg.uniform(-0.07, 0.06)
        cz = S.TRAY_C[2] + (which - 0.5) * 0.15 + rg.uniform(-0.04, 0.04)
        rest = np.array([cx, S.FELT_Y + S.DIE / 2, cz])
    else:
        side = np.array([-v[2], 0, v[0]])
        rest = home + v * (0.34 + rg.uniform(0, 0.12)) + side * ((which - 0.5) * 0.16 + rg.uniform(-0.04, 0.04))
        rest[0] = np.clip(rest[0], S.RX0 + 0.08, S.RX1 - 0.08)
        rest[2] = np.clip(rest[2], S.RZ0 + 0.3, S.RZ1 - 0.06)
        rest[1] = S.DIE / 2
    start = home + v * 0.07 + np.array([0, 0.17 + home[1] * 0, 0]) + np.array([-v[2], 0, v[0]]) * (which - 0.5) * 0.04
    if r.cup == 'A' or r.cup == 'B':
        start[1] = home[1] + 0.17
    yaw = rg.uniform(0, 6.283)
    P, w = die_traj(r, t, start, rest, yaw)
    Rf = S.rot_to_top(r.dice[which], yaw)
    ax = nrm(rg.normal(0, 1, 3))
    R = S.axis_rot(ax, (1 - w) ** 2 * rg.uniform(9, 14)) @ Rf
    return P, R, rest


def roll_end(r):
    nxt = next((q for q in CUP_ROLLS[r.cup] if q.t_throw > r.t_throw), None)
    if nxt is not None:
        return nxt.t_throw - 0.45
    if r.cup == 'A':
        return T.T_SETTLE
    return CUP_LEAVE[r.cup] - 0.15


def draw_all_dice(fr, cam, L, d, t, shadows_only=False):
    for r in T.ROLLS:
        tt = t
        loop = r is T.ROLLS[0] and t > T.T_LOOP_THROW - 0.01
        if loop:
            tt = t - 60.0
        t_end = 99 if loop else roll_end(r)
        if not (r.t_throw <= tt < t_end):
            continue
        home, v = cup_home(r.cup)
        cpos, _, _, cvis = cup_state(r.cup, r.t_throw)
        for which in (0, 1):
            P, R, rest = dice_world(r, tt, which)
            sc = 1.0
            if tt > t_end - 0.32:                     # scooped back into the cup
                u = (tt - (t_end - 0.32)) / 0.32
                mouth = cpos + np.array([0, 0.16, 0])
                P = rest + (mouth - rest) * ease(u)
                P[1] += 0.18 * math.sin(math.pi * u)
                sc = 1 - 0.7 * u
            if shadows_only:
                yield P, sc
            else:
                draw_die(fr, cam, L, d, P, R, scale=sc)


def draw_die_shadows(fr, cam, t):
    """soft contact shadows under dice (on the felt or the floor)"""
    for P, sc in draw_all_dice(None, cam, None, None, t, shadows_only=True):
        base_y = S.FELT_Y + 0.001 if abs(P[0] - S.TRAY_C[0]) < S.TRAY_HX and abs(P[2] - S.TRAY_C[2]) < S.TRAY_HZ and P[1] > 0.5 else 0.002
        hgt = max(0.0, P[1] - base_y - S.DIE / 2)
        rr = S.DIE * 0.7 * sc * (1 + hgt * 3)
        a = 1 / (1 + hgt * 12)
        q = np.array([[P[0] + rr * math.cos(k), base_y, P[2] + rr * math.sin(k)] for k in np.linspace(0, 6.283, 9)[:-1]])
        if cam.depth(q).min() < 0.2:
            continue
        pts = [tuple(p) for p in cam.wp(q)]
        arr = None
        fr.draw.polygon(pts, fill=(18, 30, 24) if base_y > 0.5 else (60, 34, 24)) if a > 0.45 else None


RAIN_DICE = [(tr, s, x, z, yaw, seed) for (tr, s, x, z, yaw, seed) in T.RAIN]


def draw_rain(fr, cam, L, d, t):
    for (tr, s, x, z, yaw, seed) in RAIN_DICE:
        if not (tr - 0.6 <= t < tr + 1.5):
            continue
        rg = np.random.default_rng(seed)
        a, b = T.pair_for(s, rg)
        for which, v in ((0, a), (1, b)):
            off = np.array([(which - 0.5) * 0.12, 0, rg.uniform(-0.04, 0.04)])
            rest = np.array([x, S.DIE / 2, z]) + off
            if t < tr:
                u = (t - (tr - 0.6)) / 0.6
                P = rest + np.array([rg.uniform(-0.2, 0.2) * (1 - u), 3.3 * (1 - u * u), 0])
                w = u * 0.7
            else:
                v_ = min(1.0, (t - tr) / 0.35)
                P = rest + np.array([0, 0.06 * abs(math.sin(math.pi * 2 * v_)) * (1 - v_) ** 2, 0])
                w = 0.7 + 0.3 * v_
            R = S.axis_rot(nrm(rg.normal(0, 1, 3)), (1 - w) ** 2 * 12) @ S.rot_to_top(v, yaw + which)
            sc = 1.0 - ss(tr + 1.1, tr + 1.5, t)
            if sc > 0.05:
                draw_die(fr, cam, L, d, P, R, scale=sc)


# ---------------------------------------------------------------- fortepiano keys (self-playing)
WIDX = {m: i for i, m in enumerate(S.WHITE)}


def draw_keys(fr, cam, L, d, t):
    pressed = set(T.playing_notes(t))
    zf, zb = -0.2, -0.365
    faces, base = [], []
    for i, m in enumerate(S.WHITE):
        x0 = S.PIANO_KX0 + i * S.KEY_W + 0.0012
        x1 = x0 + S.KEY_W - 0.0024
        y = S.KEYS_Y - (0.005 if m in pressed else 0.0)
        c = (255, 206, 120) if m in pressed else (236, 226, 202)
        faces.append((np.array([[x0, y, zb], [x0, y, zf], [x1, y, zf], [x1, y, zb]]), np.array([0, 1.0, 0])))
        base.append(c)
        faces.append((np.array([[x0, y - 0.02, zf], [x1, y - 0.02, zf], [x1, y, zf], [x0, y, zf]]), np.array([0, 0, 1.0])))
        base.append(tuple(int(v * 0.85) for v in c))
    for m in S.BLACK:
        w = WIDX.get(m - 1)
        if w is None:
            continue
        xc = S.PIANO_KX0 + (w + 1) * S.KEY_W
        hw = S.KEY_W * 0.3
        y = S.KEYS_Y + 0.014 - (0.005 if m in pressed else 0.0)
        c = (200, 140, 60) if m in pressed else (34, 26, 24)
        faces.append((np.array([[xc - hw, y, zb], [xc - hw, y, -0.275], [xc + hw, y, -0.275], [xc + hw, y, zb]]), np.array([0, 1.0, 0])))
        base.append(c)
        faces.append((np.array([[xc - hw, S.KEYS_Y, -0.275], [xc + hw, S.KEYS_Y, -0.275], [xc + hw, y, -0.275], [xc - hw, y, -0.275]]), np.array([0, 0, 1.0])))
        base.append(tuple(int(v * 0.7) for v in c))
    vis = [k for k, (Q, n) in enumerate(faces) if np.dot(n, cam.pos - Q.mean(0)) > 0]
    if not vis:
        return
    C = np.array([faces[k][0].mean(0) for k in vis])
    N = np.array([faces[k][1] for k in vis])
    kv = (S.win_lit(C + N * 0.01, d) * OCC.vis(C + N * 0.01, S.PIANO_G, d=d))[:, None]
    cols = shade(L, N, C, np.array([base[k] for k in vis], float), kvis=kv, emit=0.15)
    for k, c in zip(vis, cols):
        fr.poly(cam, faces[k][0], c)


# ---------------------------------------------------------------- histogram on the back wall
HIST_COL = [mixc((96, 176, 214), (255, 204, 92), 1 - abs(s - 7) / 5.0) for s in range(2, 13)]


def hist_heights(t):
    c = T.hist_counts(t)
    h = c / c.max() if c.max() > 0 else np.zeros(11)
    lock = ss(T.T_CLIMAX + 2.2, T.T_CLIMAX + 3.4, t)
    h = h * (1 - lock) + (T.TRIANGLE / T.TRIANGLE.max()) * lock
    grow = ss(T.T_HIST, T.T_HIST + 0.9, t) * (1 - ss(55.6, 57.6, t))
    return h * grow, lock


def hist_overlay(fr, cam, t, L, ks, glows, labels):
    h, lock = hist_heights(t)
    x0, x1, y0, y1, z = S.HIST_X0, S.HIST_X1, S.HIST_Y0, S.HIST_Y1, S.HIST_Z + 0.002
    bw = (x1 - x0 - 0.06) / 11
    ybase = y0 + 0.08
    hmax = y1 - ybase - 0.04
    Pc = np.array([0.5 * (x0 + x1), 0.5 * (y0 + y1), z])
    I = L.amb * 1.1 + sum(WARM * ks[k] * 1.1 / (1 + (np.linalg.norm(LAMPS[k][0] - Pc) / LAMPS[k][1]) ** 2) for k in range(4))
    fr.line(cam, [[x0 + 0.02, ybase - 0.004, z], [x1 - 0.02, ybase - 0.004, z]], tuple(np.clip(np.array([200, 190, 160]) * I, 0, 255)), 1)
    pairs_hl = T.HUD_PAIRS[0] <= t < T.HUD_PAIRS[1] + 0.3
    for k in range(11):
        xa = x0 + 0.03 + k * bw + bw * 0.14
        xb = xa + bw * 0.72
        hk = h[k] * hmax
        col = np.array(HIST_COL[k]) * (I * 0.8 + 0.35)
        if pairs_hl and k in (0, 5):
            col = col * 1.25 + 30
        if lock > 0:
            col = col * (1 + 0.25 * lock * (0.5 + 0.5 * math.sin(t * 8 - k * 0.5)))
        if hk > 0.003:
            fr.poly(cam, np.array([[xa, ybase, z], [xb, ybase, z], [xb, ybase + hk, z], [xa, ybase + hk, z]]), np.clip(col, 0, 255))
            fr.line(cam, [[xa, ybase + hk, z + 0.001], [xb, ybase + hk, z + 0.001]], tuple(np.clip(col * 1.3 + 20, 0, 255).astype(int)), 1)
        s = cam.wp([0.5 * (xa + xb), y0 + 0.035, z])
        labels.append((str(k + 2), s[0], s[1] - 3, (236, 222, 190) if not (pairs_hl and k in (0, 5)) else (255, 210, 90), 1, (12, 18, 16)))
        if pairs_hl and k in (0, 5):
            s2 = cam.wp([0.5 * (xa + xb), ybase + hk + 0.05, z])
            labels.append(('1/36' if k == 0 else '6/36', s2[0], s2[1] - 4, (255, 214, 120), 1, (12, 18, 16)))
            glows.append((s2[0], s2[1] + 6, 10, (255, 190, 80), 0.25))
    if lock > 0.5:
        s = cam.wp([x0 + 0.03 + 5.5 * bw, ybase + hmax + 0.02, z])
        glows.append((s[0], s[1], 14, (255, 200, 90), 0.35 * lock))


# ---------------------------------------------------------------- calendar (left wall)
def cal_year(t):
    if T.T_YEARS + 0.3 <= t < T.T_TWO - 0.1:
        u = (t - T.T_YEARS - 0.3) / (T.T_TWO - 0.4 - T.T_YEARS)
        return 1792 + int(720e6 * u ** 3.2)
    return 1792


def calendar_overlay(fr, cam, t, L, ks, labels):
    x = S.CAL_X + 0.004
    z0, z1, y0, y1 = S.CAL_Z0, S.CAL_Z1, S.CAL_Y0, S.CAL_Y1
    if cam.pos[0] <= x:
        return
    Pc = np.array([x, 0.5 * (y0 + y1), 0.5 * (z0 + z1)])
    I = L.amb * 1.1 + sum(WARM * ks[k] / (1 + (np.linalg.norm(LAMPS[k][0] - Pc) / LAMPS[k][1]) ** 2) for k in range(4))
    fr.poly(cam, np.array([[x, y0, z0], [x, y0, z1], [x, y1, z1], [x, y1, z0]]), np.clip(np.array([240, 230, 206]) * I, 0, 255))
    fr.poly(cam, np.array([[x + 0.001, y1 - 0.12, z0], [x + 0.001, y1 - 0.12, z1], [x + 0.001, y1, z1], [x + 0.001, y1, z0]]), np.clip(np.array([176, 44, 50]) * I, 0, 255))
    for k in range(4):
        zz = z0 + 0.06 + k * 0.13
        fr.line(cam, [[x + 0.002, y0 + 0.06, zz], [x + 0.002, y0 + 0.06, zz + 0.09]], tuple(np.clip(np.array([150, 140, 120]) * I, 0, 255).astype(int)), 1)
    yr = cal_year(t)
    s = cam.wp([x, 0.5 * (y0 + y1) - 0.02, 0.5 * (z0 + z1)])
    txt = f'{yr:,}' if yr > 99999 else str(yr)
    sc = 2 if len(txt) <= 5 else 1
    labels.append((txt, s[0], s[1] - 3.5 * sc, (40, 28, 24), sc, None))
    sh = cam.wp([x, y1 - 0.06, 0.5 * (z0 + z1)])
    labels.append(('JAHR', sh[0], sh[1] - 3, (250, 236, 210), 1, None))
    # torn-off pages flying away during the time-lapse
    if T.T_YEARS + 0.3 <= t < T.T_TWO - 0.1:
        for j in range(5):
            ph = (t * (2.5 + j * 0.9) + j * 0.37) % 1.0
            zc = 0.5 * (z0 + z1) + ph * 0.5
            yc = y1 - 0.15 - ph * 0.9
            ang = ph * 5
            hw, hh = 0.13 * (1 - ph * 0.3), 0.16 * (1 - ph * 0.3)
            Q = np.array([[x + 0.02 + ph * 0.5 + hh * math.sin(ang) * k1, yc + hh * k1 * math.cos(ang), zc + hw * k2] for k1, k2 in ((-1, -1), (-1, 1), (1, 1), (1, -1))])
            fr.poly(cam, Q, np.clip(np.array([236, 226, 200]) * I * (0.8 + 0.2 * math.cos(ang)), 0, 255))


# ---------------------------------------------------------------- flying bar cards (chart cell -> staff)
def card_flights(t):
    out = []
    for k, r in enumerate(T.LEAD[:16]):
        appear = r.t_land + (1.2 if r.demo else 0.35)
        t0 = r.t_land + (0.72 if r.demo else 0.06)
        if t0 <= t < appear:
            u = (t - t0) / (appear - t0)
            if r.t_land > T.T_UNFURL[1]:
                x0_, x1_, z0_, z1_ = S.cell_rect(r.sum - 2, r.col)
                src = np.array([0.5 * (x0_ + x1_), 0.01, 0.5 * (z0_ + z1_)])
            else:
                src = S.TRAY_C + np.array([0, 0.08, 0])
            xw, yb = S.staff_xy(k, 0.5)
            dst = np.array([xw, yb + 2 * S.LINE_DY, S.STAFF_Z + 0.03])
            mid = 0.5 * (src + dst) + np.array([0, 0.5, 0.35])
            e = ease(u)
            P = (1 - e) ** 2 * src + 2 * (1 - e) * e * mid + e * e * dst
            out.append((P, u, r))
    return out


def draw_cards(fr, cam, t, glows, labels):
    for P, u, r in card_flights(t):
        z = cam.depth(P)[0]
        if z < 0.3:
            continue
        s = cam.wp(P)
        w = 0.15 * cam.f / z * (1.3 if r.demo else 1.0)
        h = w * 0.62
        fr.draw.rectangle([s[0] - w / 2 - 1, s[1] - h / 2 - 1, s[0] + w / 2 + 1, s[1] + h / 2 + 1], fill=(70, 46, 30))
        fr.draw.rectangle([s[0] - w / 2, s[1] - h / 2, s[0] + w / 2, s[1] + h / 2], fill=(246, 228, 184))
        for j in range(4):
            fr.draw.line([s[0] - w * 0.4, s[1] - h * 0.3 + j * h * 0.2, s[0] + w * 0.4, s[1] - h * 0.3 + j * h * 0.2], fill=(196, 172, 130))
        for j in range(3):
            xx = s[0] - w * 0.25 + j * w * 0.25
            yy = s[1] + h * (0.1 - 0.15 * ((r.bar + j) % 3))
            fr.draw.ellipse([xx - 1.5, yy - 1, xx + 1.5, yy + 1], fill=(46, 32, 26))
        glows.append((s[0], s[1], w * 0.9, (255, 200, 110), 0.3 * (1 - u * 0.5)))
        if r.demo:
            labels.append((f'BAR {r.bar}', s[0], s[1] - h / 2 - 11, AMBER, 1, (10, 20, 26)))


# ---------------------------------------------------------------- moonbeam (volumetric, quarter resolution)
QH, QW = H // 4, W // 4
_QY, _QX = np.mgrid[0:QH, 0:QW].astype(np.float32)
MOTES = np.random.default_rng(5).uniform([-1.6, 0.1, -1.4], [1.0, 2.4, 1.2], (110, 3))


def moonbeam(cam, depth, d, t):
    dirs = cam.pixel_rays(_QX * 4 + 2, _QY * 4 + 2)
    dq = np.minimum(depth[2::4, 2::4], 14.0)
    n = 14
    acc = np.zeros((QH, QW), np.float32)
    jit = (BAYER_FULL[2::4, 2::4] - 0.5)
    for i in range(n):
        s = (i + 0.5 + jit) / n * dq
        P = cam.pos[None, None] + dirs * s[..., None]
        inside = (P[..., 0] > S.RX0) & (P[..., 0] < S.RX1) & (P[..., 2] > S.RZ0) & (P[..., 2] < S.RZ1) & (P[..., 1] > 0) & (P[..., 1] < S.WALL_H)
        lit = S.win_lit(P.reshape(-1, 3), d).reshape(QH, QW) * inside
        acc += lit * dq / n
    beam = 1 - np.exp(-acc * 0.45)
    beam = np.asarray(Image.fromarray((beam * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(1.2)), np.float32) / 255.0
    up = np.asarray(Image.fromarray((beam * 255).astype(np.uint8)).resize((W, H), Image.BILINEAR), np.float32) / 255.0
    return up


def draw_motes(img, cam, depth, d, t):
    P = MOTES + np.stack([0.05 * np.sin(t * 0.3 + MOTES[:, 1] * 5), 0.04 * np.sin(t * 0.21 + MOTES[:, 0] * 3) - 0.0, 0.05 * np.cos(t * 0.25 + MOTES[:, 2] * 4)], 1)
    lit = S.win_lit(P, d)
    zc = cam.depth(P)
    scr = cam.wp(P)
    for p, z, l, (x, y) in zip(P, zc, lit, scr):
        if l <= 0 or z < 0.3:
            continue
        xi, yi = int(x), int(y)
        if 0 <= xi < W and 0 <= yi < H and depth[yi, xi] > z:
            img[yi, xi] = img[yi, xi] * 0.4 + np.array([220, 230, 255.0]) * 0.6


STARS = np.random.default_rng(9).normal(0, 1, (120, 3))
STARS /= np.linalg.norm(STARS, axis=1, keepdims=True)


def draw_stars(fr, cam, t):
    for k, sd in enumerate(STARS):
        s = cam.screen_of_dir(sd)
        if s is None:
            continue
        x, y = int(s[0]), int(s[1])
        if 0 <= x < W and 0 <= y < H:
            b = 120 + 100 * (0.5 + 0.5 * math.sin(t * (1.5 + k % 5) + k))
            fr.draw.point((x, y), fill=(int(b), int(b), int(b * 0.9 + 25)))


# ---------------------------------------------------------------- HUD (bottom panels)
def die_icon(img_draw, x, y, v, sz=10, col=(250, 244, 226)):
    img_draw.rectangle([x, y, x + sz, y + sz], fill=col, outline=(40, 30, 30))
    c = sz / 2
    for (i, j) in S.PIPS[v]:
        px, py = x + c + i * sz * 0.28, y + c + j * sz * 0.28
        img_draw.rectangle([px - 1, py - 1, px, py], fill=(40, 30, 34))


def big_digits_snap(t):
    a, b = '45,949,729,863,572,161', '759,499,667,166,482'
    if t < T.T_SNAP:
        return a, None, 0.0
    u = min(1.0, (t - T.T_SNAP) / 0.45)
    return a, b, u


def draw_hud(out, t):
    pil = Image.fromarray(out)
    dr = ImageDraw.Draw(pil)
    texts = []
    # dice pairs: 2 can only be 1+1, 7 has six ways
    a, b = T.HUD_PAIRS
    if a - 0.4 <= t < b + 0.4:
        off = slide_offset(t, a, b, 170)
        y0 = 506 + off
        panel(dr, 14, y0, 346, y0 + 98)
        texts.append(('2', 30, y0 + 14, AMBER, 3))
        die_icon(dr, 62, y0 + 14, 1, 13)
        die_icon(dr, 79, y0 + 14, 1, 13)
        texts.append(('1 WAY', 108, y0 + 18, CREAM, 2))
        texts.append(('7', 30, y0 + 56, AMBER, 3))
        for k in range(6):
            x = 62 + k * 34
            die_icon(dr, x, y0 + 52, k + 1, 13)
            die_icon(dr, x + 15, y0 + 52, 6 - k, 13)
        texts.append(('6 WAYS', 62, y0 + 74, MINT, 1))
        texts.append(('OF 36', 300, y0 + 18, SKYB, 1))
        texts.append(('OF 36', 300, y0 + 60, SKYB, 1))
    # roll orders -> distinct waltzes
    a, b = T.HUD_ROLLS
    if a - 0.4 <= t < b + 0.4:
        off = slide_offset(t, a, b, 170)
        y0 = 506 + off
        panel(dr, 14, y0, 346, y0 + 98)
        old, new, u = big_digits_snap(t)
        if new is None:
            texts.append(('11 CHOICES, 16 TIMES:', 180, y0 + 12, SKYB, 1, 'c'))
            texts.append((old, 180, y0 + 34, CREAM, 2, 'c'))
            texts.append(('ROLL SEQUENCES', 180, y0 + 62, CREAM, 1, 'c'))
        else:
            if u < 1:
                texts.append((old, 180, y0 + 34 - int(18 * u), (int(252 - 160 * u), int(240 - 160 * u), int(214 - 140 * u)), 2, 'c'))
            texts.append(('BAR 8: 1 REAL CHOICE  BAR 16: 2', 180, y0 + 12, SKYB, 1, 'c'))
            if u > 0.3:
                yy = y0 + 34 + int(16 * (1 - ease((u - 0.3) / 0.7)))
                texts.append((new, 180, yy, AMBER, 2, 'c'))
            texts.append(('DIFFERENT WALTZES', 180, y0 + 64, MINT, 2, 'c'))
    # years to hear them all
    a, b = T.HUD_YEARS
    if a - 0.4 <= t < b + 0.4:
        off = slide_offset(t, a, b, 170)
        y0 = 506 + off
        panel(dr, 14, y0, 346, y0 + 98)
        u = ease((t - a - 0.3) / 4.8)
        n = int(round(720e6 * u, -5)) if u < 1 else 720_000_000
        texts.append(('PLAYED BACK TO BACK, 30 S EACH:', 180, y0 + 12, SKYB, 1, 'c'))
        texts.append((f'{n:,}', 180, y0 + 32, AMBER, 3, 'c'))
        texts.append(('YEARS', 180, y0 + 64, CREAM, 2, 'c'))
    out = np.asarray(pil).copy()
    for tx in texts:
        if len(tx) == 6:
            center_text(out, tx[0], tx[1], tx[2], tx[3], tx[4])
        else:
            m = text_mask(tx[0], tx[4])
            stamp(out, m, tx[1], tx[2], tx[3])
    return out


# ---------------------------------------------------------------- render
def render(f):
    t = f / FPS
    cam = interp_keys(KEYS, t)
    d = moon_dir(t)
    L, ks = make_lights(t, cam, d)
    fr = Frame(sky_gradient(cam, (18, 16, 52), (64, 44, 104), step=7.0))
    draw_stars(fr, cam, t)
    moon_mask = MOON_MASK0 if T.moon_phase(t) <= 0 else directional_mask(FLOOR, CAST_FACES, d, blur=1.2)
    exterior_polys(cam, fr)
    moon_scr = shell_pass(fr, cam, t, L, ks, d, moon_mask, chart_fn(t))
    glows, labels = [], []
    items = [(np.linalg.norm(o.ctr - cam.pos), 'obj', o) for o in S.OBJS]
    items.append((np.linalg.norm(np.array([0, 2.0, S.RZ0]) - cam.pos) + 0.3, 'staff', None))
    items.append((np.linalg.norm(np.array([-1.1, 1.22, S.RZ0]) - cam.pos) - 0.08, 'hist', None))
    for side in (-1, 1):
        items.append((np.linalg.norm(np.array([S.RX0 + 0.1, 1.5, S.WIN_ZC + side * 0.6]) - cam.pos), 'curtain', side))
    items.append((np.linalg.norm(S.CAT_P - cam.pos) - 0.05, 'cat', None))
    items.append((np.linalg.norm(np.array([S.RX0, 1.7, 0.4]) - cam.pos) - 0.05, 'calendar', None))
    items.append((np.linalg.norm(np.array([1.09, 0.72, -0.28]) - cam.pos) - 0.05, 'keys', None))
    zr = chart_state(t)
    if zr < S.CHART_Z1 - 0.01:
        items.append((np.linalg.norm(np.array([0.0, 0.05, zr]) - cam.pos), 'roll', zr))
    items.append((np.linalg.norm(S.TRAY_C - cam.pos) - 0.45, 'dice', None))
    for _, kind, o in painter(items):
        if kind == 'obj':
            draw_obj(fr, cam, L, d, o, emit=0.85 if o.name == 'candle' else 0.0)
            if o.name == 'felt':
                draw_die_shadows(fr, cam, t)
        elif kind == 'staff':
            ink, line_c = staff_overlay(fr, cam, t, L, ks, glows)
            for slot, cup, bar, appear in T.staff_state(t):
                col = ink if cup == 'A' else np.array(T.CUP_COLS[cup], float) * 0.85
                draw_bar_notes(fr, cam, slot, bar, col, appear, t, outline=None if cup == 'A' else (40, 26, 22))
            for slot, cup, bar, appear in T.staff_state(t):
                if cup == 'A':
                    xw, yb = S.staff_xy(slot, 0.5)
                    s = cam.wp([xw, yb - 0.045, S.STAFF_Z])
                    labels.append((str(bar), s[0], s[1] - 3, (120, 84, 60), 1, None))
            sf, pa = T.playhead(t)
            if sf is not None and sf < 16:
                slot = int(sf)
                xw, yb = S.staff_xy(slot, sf - slot)
                p0, p1 = [xw, yb - 0.04, S.STAFF_Z + 0.006], [xw, yb + 4 * S.LINE_DY + 0.05, S.STAFF_Z + 0.006]
                fr.line(cam, [p0, p1], (255, 206, 110), 2)
                sm = cam.wp([xw, yb + 2 * S.LINE_DY, S.STAFF_Z])
                glows.append((sm[0], sm[1], 16, (255, 190, 90), 0.3 * pa))
        elif kind == 'hist':
            hist_overlay(fr, cam, t, L, ks, glows, labels)
        elif kind == 'calendar':
            calendar_overlay(fr, cam, t, L, ks, labels)
        elif kind == 'keys':
            draw_keys(fr, cam, L, d, t)
        elif kind == 'curtain':
            draw_obj(fr, cam, L, d, Obj(S.curtain(t, o), (150, 40, 50), 'curtain', grp=S.CURT_G, double=True))
        elif kind == 'cat':
            look = 0.5 * math.sin(t * 0.37) - 0.2
            for name, fcs in S.cat_faces(t, look):
                draw_obj(fr, cam, L, d, Obj(fcs, (56, 52, 64), 'cat', grp=-11))
        elif kind == 'roll':
            draw_obj(fr, cam, L, d, Obj(S.cyl_x(S.CHART_X0 - 0.01, S.CHART_X1 + 0.01, 0.045, o + 0.03, 0.045, 8), tuple(PARCH * 0.9), 'roll', grp=-12))
        elif kind == 'dice':
            for cup in T.CUPS:
                draw_cup(fr, cam, L, d, cup, t, glows)
            for _ in draw_all_dice(fr, cam, L, d, t):
                pass
            draw_rain(fr, cam, L, d, t)
            draw_cards(fr, cam, t, glows, labels)
    for i, cp in enumerate(S.CANDLES):
        zc = cam.depth(cp)[0]
        if zc < 0.2:
            continue
        s = cam.wp(cp)
        fl = lamp_flicker(t, i)
        r = 0.022 * cam.f / zc
        fr.draw.ellipse([s[0] - r * 0.5, s[1] - r * 1.4, s[0] + r * 0.5, s[1] + r * 0.4], fill=(255, 228, 150))
        glows.append((s[0], s[1] - r * 0.3, 0.16 * cam.f / zc * fl, (255, 150, 60), 0.5 * fl))
    if moon_scr is not None:
        glows.append((moon_scr[0], moon_scr[1], 0.55 * cam.f / 6.0, (120, 140, 190), 0.3))
    # lens: focus locked on the tray; the sharp band covers staff, chart, histogram and cups
    img, depth = fr.array(), fr.depth()
    zf = cam.depth(S.SUBJECT)[0]
    readouts = np.array([[S.STAFF_X0, S.STAFF_Y1, S.STAFF_Z], [S.STAFF_X1, S.STAFF_Y0, S.STAFF_Z], [S.HIST_X0, S.HIST_Y0, S.HIST_Z],
                         [S.CHART_X0, 0, S.CHART_Z1], [S.CHART_X1, 0, S.CHART_Z1], [S.CHART_X0, 0, S.CHART_Z0], S.CUP_HOME,
                         [S.TABLE_C[0] - 1.0, 0, S.TABLE_C[2]], [S.TABLE_C[0], 0, S.TABLE_C[2] + 0.92]])
    zr_ = cam.depth(readouts)
    sharp = float(np.clip(np.abs(1 - zf / np.maximum(zr_, 0.3)).max() + 0.03, 0.05, 0.9))
    K = 0.011 * cam.f * min(1.2, 4.5 / zf)
    coc = coc_map(depth, zf, K, sharp=sharp)
    img = dof(img, coc)
    beam = moonbeam(cam, depth, d, t)
    img = img + beam[..., None] * np.array([96, 112, 150.0])[None, None] * 0.2
    draw_motes(img, cam, depth, d, t)
    img = apply_glows(img, fr.glows + glows)
    img = grade(bloom(np.clip(img, 0, 255)))
    img = np.clip(img * vignette() * top_scrim(), 0, 255)
    out = img.astype(np.uint8)
    chart_labels(out, cam, t, glows)
    for txt, x, y, col, sc, oc in labels:
        center_text(out, txt, x, y, col, sc, outline=oc)
    for r in T.LEAD:
        if r.t_land <= t < r.t_land + (2.0 if r.demo else 0.85) and r.where == 'tray':
            u = (t - r.t_land) / (2.0 if r.demo else 0.85)
            P = S.TRAY_C + np.array([-0.1, 0.2 + 0.05 * ease(min(1, u * 4)), 0])
            s = cam.wp(P)
            a = 1 - ss(0.8, 1.0, u)
            if a > 0.05:
                center_text(out, str(r.sum), s[0], s[1] - 10, AMBER, 3 if r.demo else 2)
    out = draw_hud(out, t)
    draw_captions(out, t, T.CAPTIONS)
    if os.environ.get('COC_CHECK'):
        render.last_coc = float(coc_map(np.array([[zf]]), zf, K, sharp=sharp)[0, 0])
        render.last_sharp = sharp
    return out
