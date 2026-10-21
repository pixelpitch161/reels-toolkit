"""Dinosaur Brass Band renderer.

sky (sunset, sun disc, clouds, pterosaurs) -> per-pixel lit ground (sun shadows from static plants +
the moving herd, contact AO, river reflecting the sky with a sun glint) -> far layer (hills, forest,
volcano + smoke) -> near objects in painter order (box-occlusion sun shadows, rim light, crest glow
as a warm local light) -> X-ray crest / CT slices / glowing air -> locked depth of field -> glows,
sound rings, god rays, bloom, grade -> HUD (tube panel, piano roll, bottle) + captions + notes.

    python -m reelkit.sheet render.py test/sheet.png 1 6 10 14 18 22 26 32 40 48 54 59
    python -m reelkit.video render.py audio.wav dinosaur_brass_band.mp4"""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, ss, ease, dirv, mixc
from reelkit.camera import Cam, interp_keys, clip_near
from reelkit.raster import Frame, apply_glows, SKY_DEPTH
from reelkit.lighting import Lights, shade
from reelkit.shadows import MaskGrid, directional_mask, Occluders, contact_ao
from reelkit.lens import coc_map, dof
from reelkit.post import god_rays, bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, panel, label, slide_offset, CREAM, AMBER, MINT, CYAN, SKYB
from reelkit.font import text_mask, stamp
from reelkit.geometry import icosa
import timeline as T
import scene as S
import dino as D

SUN = S.SUN
DINOS = [D.Dino(v['scale'], v['crest_f'], v['pal'], head_f=v['head_f'], seed=k) for k, v in enumerate(T.VOICES)]
XRAY = np.array([90, 230, 255], float)

# ------------------------------------------------------------------ static shadow mask
GRID = MaskGrid((-40.0, 60.0, -48.0, 46.0, 0.08))
_static_casters = [Q for o in S.OBJS if o.cast for Q, n in o.faces]
_static_casters += [q for f in S.FERNS for q in f.quads]
STATIC_MASK = directional_mask(GRID, _static_casters, SUN, blur=1.6)


class Box:
    def __init__(self, lo, hi, grp):
        self.lo, self.hi, self.grp, self.cast = lo, hi, grp, True


STATIC_BOXES = [Box(o.lo, o.hi, o.grp) for o in S.OBJS if o.cast and o.name not in ('horsetail',)]
DINO_GRP = [10000 + k for k in range(len(DINOS))]


# ------------------------------------------------------------------ poses
def pose(k, t):
    v = T.VOICES[k]
    d = DINOS[k]
    r = T.raise_amt(k, t)
    dx, dz, hdeg, off, spd = T.walk(k, t)
    hd = math.radians(hdeg)
    pos = np.array([v['pos'][0] + dx, 0.0, v['pos'][1] + dz])
    ph = off / (1.7 * v['scale']) * math.pi
    amp = min(1.0, spd * 1.4)
    swing = 0.36 * amp * math.sin(ph)
    bob = 0.06 * v['scale'] * amp * abs(math.sin(ph))
    lev = T.honk_level(k, t)
    idle = math.sin(t * 1.15 + k * 1.7) * 0.025
    neck = math.radians(24 + 40 * r) + idle + 0.05 * lev * math.sin(t * 9 + k)
    head = math.radians(-34 + 68 * r) + idle * 0.6 + 0.04 * lev
    if spd > 0.05:
        neck -= 0.12 * amp
        head -= 0.1 * amp
    return d.mats(pos, hd, neck, head, swing, bob), pos


def dino_world(k, M):
    """[(Q, n, colour, tag)] for every face of dino k in world coords"""
    d = DINOS[k]
    out = []
    for part, m in d.parts(M):
        V = D.xf(m, part.V)
        N = D.xn(m, part.N)
        for j, idx in enumerate(part.F):
            out.append((V[idx], N[j], part.C[j], part.tag[j]))
    return out


def head_point(k, M, local):
    d = DINOS[k]
    return D.xf(M['head'], np.array([local], float))[0]


def crest_top(k, M):
    cs = D.crest_secs(DINOS[k].cf)
    x, y = cs[len(cs) // 2][0], cs[len(cs) // 2][1]
    return head_point(k, M, (x, y + 0.1, 0))


# ------------------------------------------------------------------ camera
def P(k, local, r, t=0.0):
    """world position of a head-local point of voice k at raise amount r (for planning keys)"""
    v = T.VOICES[k]
    hd = math.radians(v['heading'])
    pos = np.array([v['pos'][0], 0.0, v['pos'][1]])
    M = DINOS[k].mats(pos, hd, math.radians(24 + 40 * r), math.radians(-34 + 68 * r))
    return D.xf(M['head'], np.array([local], float))[0]


HEAD_OPEN = P(0, (0.2, 0.15, 0), 1.0)          # adult head, honking
HEAD_HOLD = P(0, (-0.25, 0.2, 0), 0.55)        # adult head + crest, held for the X-ray section
HEAD_S = P(3, (0.2, 0.1, 0), 0.8)              # juvenile head
HERD_C = np.array([np.mean([v['pos'][0] for v in T.VOICES]) + 0.6, 2.7, np.mean([v['pos'][1] for v in T.VOICES])])
OPEN_T = HEAD_OPEN * 0.55 + np.array([0.0, 2.6, 0.0]) * 0.45 + np.array([-0.4, 0, 0])
TWO_T = (HEAD_HOLD + HEAD_S) / 2 + np.array([0.5, -0.6, 0])

def fit(t, ids, yaw, pitch, fov, sy, box):
    """target + distance so the posed animals ids (tails trimmed) fill the screen box (x0, x1, y0, y1)"""
    pts = []
    for k in ids:
        M, pos = pose(k, t)
        for part, m in DINOS[k].parts(M):
            V = part.V if part is not DINOS[k].body else part.V[part.V[:, 0] > -3.2]
            pts.append(D.xf(m, V))
    pts = np.concatenate(pts)
    tgt = pts.mean(0)
    dist = 20.0
    x0, x1, y0, y1 = box
    for _ in range(12):
        cam = Cam(tgt, dist, yaw, pitch, fov, sy)
        s = cam.wp(pts)
        bx0, bx1, by0, by1 = s[:, 0].min(), s[:, 0].max(), s[:, 1].min(), s[:, 1].max()
        z = cam.depth(tgt)[0]
        dx = ((bx0 + bx1) / 2 - (x0 + x1) / 2) * z / cam.f
        dy = ((by0 + by1) / 2 - (y0 + y1) / 2) * z / cam.f
        tgt = tgt + cam.M[0] * dx - cam.M[1] * dy
        ratio = max((bx1 - bx0) / (x1 - x0), (by1 - by0) / (y1 - y0))
        dist = dist * (0.35 + 0.65 * ratio)
    return tgt, dist


def fk(t, ids, yaw, pitch, fov=42, sy=300, box=(22, 338, 160, 500), at=None):
    tgt, dist = fit(at if at is not None else t, ids, yaw, pitch, fov, sy, box)
    return (t, tgt, dist, yaw, pitch, fov, sy)


ALL = [0, 1, 2, 3, 4]
KEYS = [  # (t, target, dist, yaw, pitch, fov, sy)
    (0.0, OPEN_T, 13.0, 30, 1, 42, 340),
    (3.3, OPEN_T, 12.4, 29, 1, 42, 340),
    (5.6, HEAD_HOLD, 6.2, 30, 6, 40, 320),
    (8.0, HEAD_HOLD, 6.0, 33, 8, 40, 320),
    (10.3, HEAD_HOLD, 5.9, 40, 11, 40, 320),
    (12.4, HEAD_HOLD, 6.0, 31, 7, 40, 320),
    (15.6, HEAD_HOLD, 6.3, 27, 5, 40, 320),
    (17.0, HEAD_HOLD + [0, -0.9, 0], 9.6, 28, 6, 42, 250),
    (19.0, HEAD_HOLD + [0, -0.9, 0], 9.9, 27, 6, 42, 250),
    fk(21.0, [0, 3], 29, 6, box=(26, 334, 205, 500), at=22.0),
    fk(24.0, [0, 3], 30, 6, box=(26, 334, 205, 495), at=25.0),
    fk(28.0, [0, 3], 29, 7, box=(26, 334, 205, 495), at=28.0),
    fk(30.6, ALL, 29, 7, box=(16, 344, 215, 505), at=32.0),
    fk(34.5, ALL, 31, 7, box=(16, 344, 215, 505), at=34.5),
    fk(40.5, ALL, 27, 6, box=(20, 340, 210, 505), at=41.0),
    fk(46.5, ALL, 30, 5, box=(14, 346, 200, 508), at=47.0),
    fk(52.4, ALL, 31, 6, box=(14, 346, 200, 508), at=52.0),
    (56.6, OPEN_T, 13.2, 30, 1, 42, 340),
    (60.0, OPEN_T, 13.0, 30, 1, 42, 340),
]


def camera(t):
    return interp_keys(KEYS, t)


# ------------------------------------------------------------------ sky
ZEN = np.array([78, 70, 158], float)
MID = np.array([236, 128, 150], float)
HOR = np.array([255, 164, 98], float)
HAZE = np.array([250, 186, 136], float)


def sky_color(d, flare=0.0):
    """colour of the sky in directions d (..., 3), sun glow included"""
    el = np.arcsin(np.clip(d[..., 1], -1, 1))
    k = np.clip(el / 0.62, 0, 1)
    k1 = np.clip(k / 0.32, 0, 1)[..., None]
    k2 = np.clip((k - 0.32) / 0.68, 0, 1)[..., None] ** 0.8
    col = HOR * (1 - k1) + MID * k1
    col = col * (1 - k2) + ZEN * k2
    cosang = (d * SUN).sum(-1)
    g = np.clip((cosang - 0.955) / 0.045, 0, 1) ** 2
    halo = np.clip((cosang - 0.2) / 0.8, 0, 1) ** 5
    col = col + (g * (0.28 + 0.4 * flare))[..., None] * np.array([255, 176, 96]) + (halo * (0.12 + 0.18 * flare))[..., None] * np.array([255, 150, 90])
    return col


def sky_image(cam, flare):
    d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1) @ cam.M
    d /= np.linalg.norm(d, axis=-1, keepdims=True)
    col = sky_color(d, flare)
    return np.clip(np.floor(col / 7 + BAYER_FULL[..., None]) * 7, 0, 255)


def flare_amt(t):
    base = ss(T.T_CLIMAX - 0.4, T.T_CLIMAX + 0.5, t) * (1 - ss(T.T_END - 0.3, T.T_END + 1.0, t))
    if T.T_CLIMAX <= t < T.T_END:
        ph = ((t - T.T_SONG) % (2 * T.BEAT)) / (2 * T.BEAT)
        base += 0.25 * math.exp(-ph * 6)
    return base


# ------------------------------------------------------------------ lights
def lights(t, cam, crest_lights):
    fl = flare_amt(t)
    L = Lights(amb=(0.47, 0.39, 0.45), sky=(0.15, 0.12, 0.2))
    L.view = cam.view
    L.key('sun', SUN, np.array([1.15, 0.8, 0.48]) * (1 + 0.25 * fl))
    L.fill(dirv(168, 24), (0.5, 0.37, 0.36))       # warm sky bounce from the camera side
    L.fill(dirv(95, 62), (0.12, 0.12, 0.2))        # cool zenith
    for pos, col, R in crest_lights:
        L.point(pos, col, R)
    return L, fl


def ff_bright(t):
    return 0.5 + 0.5 * np.sin(t * 2.3 + S.FF_PH) * np.sin(t * 0.9 + S.FF_PH * 2.1)


def ff_pos(t):
    p = S.FIREFLIES.copy()
    p[:, 0] += 0.3 * np.sin(t * 0.7 + S.FF_PH)
    p[:, 1] += 0.15 * np.sin(t * 1.1 + S.FF_PH * 1.3)
    p[:, 2] += 0.3 * np.cos(t * 0.6 + S.FF_PH * 0.7)
    return p


# ------------------------------------------------------------------ drawing helpers
def proj_poly(cam, Q):
    Pc = cam.to_cam(Q)
    if (Pc[:, 2] < cam.near).any():
        if (Pc[:, 2] < cam.near).all():
            return None
        Pc = clip_near(Pc, cam.near)
        if Pc is None:
            return None
    return [tuple(p) for p in cam.proj(Pc)]


def onscreen(cam, c, r):
    z = cam.depth(c)[0]
    if z < -r:
        return False
    if z < 0.5:
        return True
    s = cam.wp(c)
    m = r * cam.f / z
    return -m - 40 < s[0] < W + m + 40 and -m - 40 < s[1] < H + m + 40


def add_ring(img, x, y, r, width, col, inten):
    if r < 1:
        return
    x0, x1 = int(max(0, x - r - width - 2)), int(min(W, x + r + width + 3))
    y0, y1 = int(max(0, y - r - width - 2)), int(min(H, y + r + width + 3))
    if x1 <= x0 or y1 <= y0:
        return
    yy, xx = np.mgrid[y0:y1, x0:x1].astype(np.float32)
    d = np.abs(np.hypot(xx - x, yy - y) - r)
    v = np.clip(1 - d / width, 0, 1)
    v = np.floor(v * 4 + BAYER_FULL[y0:y1, x0:x1] * 0.999) / 4
    img[y0:y1, x0:x1] += v[..., None] * np.array(col, float) * inten


NOTE = ["...##..", "...#.#.", "...#..#", "...#...", "...#...", ".###...", "####...", "####...", ".##...."]
NOTE_M = np.array([[c == '#' for c in row] for row in NOTE])
NOTE_M2 = np.repeat(np.repeat(NOTE_M, 2, 0), 2, 1)


# ------------------------------------------------------------------ locked focus
def visible_faces(cam, dfaces, margin=4):
    """camera depths of (vertices, face means) of every dinosaur face that touches the screen"""
    verts, means = [], []
    for k, fl_ in dfaces.items():
        for Q, n, c, tg in fl_:
            dz = cam.depth(Q)
            if (dz < cam.near).all():
                continue
            s = cam.wp(Q)
            on = (dz > cam.near) & (s[:, 0] > -margin) & (s[:, 0] < W + margin) & (s[:, 1] > -margin) & (s[:, 1] < H + margin)
            if not on.any():
                cx, cy = s[:, 0], s[:, 1]
                if not (cx.min() < W and cx.max() > 0 and cy.min() < H and cy.max() > 0 and (dz > cam.near).all()):
                    continue
            verts.append(dz[dz > cam.near])
            means.append(float(dz.mean()))
    if not verts:
        return np.zeros(0), np.zeros(0)
    return np.concatenate(verts), np.array(means)


def focus(cam, dfaces):
    """focus locked on the subject: zf at the camera target, sharp band wide enough for every visible animal"""
    zf = cam.depth(cam.target)[0]
    v, m = visible_faces(cam, dfaces)
    need = 0.0
    if len(v):
        d = np.concatenate([v, m[m > cam.near]])
        need = float(np.abs(1 - zf / d).max())
    sharp = max(0.08, need + 0.03)
    K = 0.012 * cam.f * min(1.2, 9.0 / zf)
    return zf, sharp, K


def subject_blur(cam, dfaces, zf, sharp, K):
    """worst circle of confusion (px) on any visible dinosaur face (depth buffer uses face means)"""
    v, m = visible_faces(cam, dfaces)
    m = m[m > cam.near]
    if not len(m):
        return 0.0
    return float(np.clip(K * np.maximum(0.0, np.abs(1 - zf / m) - sharp), 0, 10.5).max())


def scene_state(t):
    """camera + posed herd faces only (used by the focus check without rendering)"""
    cam = camera(t)
    dfaces = {}
    for k in range(len(DINOS)):
        if T.present(k, t):
            M, pos = pose(k, t)
            dfaces[k] = dino_world(k, M)
    return cam, dfaces


# ------------------------------------------------------------------ main render
def render(f):
    t = f / FPS
    cam = camera(t)
    # ---- poses + crest lights
    poses = {}
    crest_lights = []
    for k in range(len(DINOS)):
        if not T.present(k, t):
            continue
        M, pos = pose(k, t)
        poses[k] = (M, pos)
        lev = T.honk_level(k, t)
        if lev > 0.01:
            c = np.array(T.VOICES[k]['col'], float) / 255.0
            crest_lights.append((crest_top(k, M), c * 0.75 * lev, 1.6 * T.VOICES[k]['scale'] + 0.6))
    L, fl = lights(t, cam, crest_lights)
    # ---- sky, sun, clouds, pterosaurs
    fr = Frame(sky_image(cam, fl))
    sp = cam.screen_of_dir(SUN)
    for Q in S.CLOUDS:
        pts = proj_poly(cam, Q)
        if pts is None:
            continue
        el = math.degrees(math.asin(Q[:, 1].mean() / 900.0))
        u = min(1.0, max(0.0, (el - 5) / 18))
        col = mixc((255, 178, 138), (196, 120, 170), u) * (1 + 0.15 * fl)
        fr.screen_poly(pts, np.floor(col / 6) * 6, z=900.0)
    for p in S.PTEROS:
        u = (t - p['t0']) / (p['t1'] - p['t0'])
        if not 0 <= u <= 1:
            continue
        az = p['az0'] + (p['az1'] - p['az0']) * u
        c = dirv(az, p['el'] + 1.5 * math.sin(u * 9)) * 300.0
        fwd = np.array([math.cos(math.radians(az)), 0, math.sin(math.radians(az))]) * np.sign(p['az1'] - p['az0'])
        side = np.array([0, 0, 0.0]) + np.cross(fwd, [0, 1, 0])
        flap = math.sin(t * 5 + p['az0']) * 0.6
        s = p['size']
        body = [c - fwd * s * 0.6, c + fwd * s * 0.9, c + [0, 0.12 * s, 0]]
        wl = [c, c + side * 2.2 * s + [0, flap * s, 0] - fwd * 0.4 * s, c - fwd * 0.5 * s]
        wr = [c, c - side * 2.2 * s + [0, flap * s, 0] - fwd * 0.4 * s, c - fwd * 0.5 * s]
        for Q in (body, wl, wr):
            pts = proj_poly(cam, np.array(Q))
            if pts:
                fr.screen_poly(pts, (92, 60, 104), z=300.0)
    if sp is not None:
        rs = (0.026 + 0.006 * fl) * cam.f
        fr.disc(sp, rs * 1.3, (255, 206, 132), n=16)
        fr.disc(sp, rs, (255, 246, 214), n=16)
    # ---- ground: albedo + water mask -> per-pixel light, shadows, AO, reflections, fog
    alb = Image.new('RGB', (W, H), S.FAR_GROUND)
    gm = Image.new('L', (W, H))
    wm = Image.new('L', (W, H))
    ad, gd, wd = ImageDraw.Draw(alb), ImageDraw.Draw(gm), ImageDraw.Draw(wm)
    big = np.array([[-3000, 0, -3000], [3000, 0, -3000], [3000, 0, 3000], [-3000, 0, 3000]], float)
    pts = proj_poly(cam, big)
    if pts:
        gd.polygon(pts, fill=255)
    for Q, col, wat in S.GROUND:
        pts = proj_poly(cam, Q)
        if pts is None:
            continue
        xs_ = [p[0] for p in pts]
        ys_ = [p[1] for p in pts]
        if max(xs_) < 0 or min(xs_) > W or max(ys_) < 0 or min(ys_) > H:
            continue
        ad.polygon(pts, fill=tuple(int(c) for c in col))
        if wat:
            wd.polygon(pts, fill=255)
        else:
            wd.polygon(pts, fill=0)
    # dynamic shadows from the herd
    dyn_faces = []
    feet = list(S.FEET)
    dfaces = {}
    for k, (M, pos) in poses.items():
        fl_ = dino_world(k, M)
        dfaces[k] = fl_
        dyn_faces += [Q for Q, n, c, tg in fl_ if tg not in ('eye', 'eyehi')]
        sc = T.VOICES[k]['scale']
        for fx, fz in DINOS[k].feet(M):
            feet.append((fx, fz, 0.16 * sc, 0.5))
        bx = (M['body'] @ np.array([-0.4, 0, 0, 1.0]))[:3]
        feet.append((bx[0], bx[2], 1.1 * sc, 0.32))
    dmask = directional_mask(GRID, dyn_faces, SUN, blur=1.4) if dyn_faces else 0
    arr = fr.array()
    depth = fr.depth()
    ys, xs = np.nonzero(np.asarray(gm))
    if len(xs):
        X, Z, dd = cam.ground_hit(xs.astype(float), ys.astype(float))
        m = GRID.sample(STATIC_MASK, X, Z)
        if dyn_faces:
            m = np.maximum(m, GRID.sample(dmask, X, Z))
        lit = 1 - m
        I = np.tile(L.amb * 0.75 + L.sky, (len(X), 1))
        I += (lit * 0.62)[:, None] * L.keys[0][2][None]
        I += 0.42 * L.fills[0][1][None] + 0.86 * L.fills[1][1][None]
        for lp, lc, R in L.points:
            dist = np.sqrt((lp[0] - X) ** 2 + lp[1] ** 2 + (lp[2] - Z) ** 2)
            I += (0.8 / (1 + (dist / R) ** 2))[:, None] * lc[None]
        ffp, ffb = ff_pos(t), ff_bright(t)
        for p, b in zip(ffp, ffb):
            dist2 = (p[0] - X) ** 2 + p[1] ** 2 + (p[2] - Z) ** 2
            I += (0.35 * b / (1 + dist2 / 0.5))[:, None] * np.array([1.0, 0.85, 0.35])[None]
        I *= contact_ao(X, Z, feet)[:, None]
        a = np.asarray(alb)[ys, xs].astype(np.float32)
        col = a * I
        wat = np.asarray(wm)[ys, xs] > 0
        if wat.any():
            dw = cam.pixel_rays(xs[wat].astype(float), ys[wat].astype(float))
            dw /= np.linalg.norm(dw, axis=1, keepdims=True)
            rf = dw * np.array([1, -1, 1])
            refl = sky_color(rf, fl)
            Xw, Zw = X[wat], Z[wat]
            rip = 0.5 + 0.5 * np.sin(Zw * 3.3 + t * 2.2 + np.sin(Xw * 0.6) * 1.5) * np.sin(Zw * 1.3 - t * 1.1 + Xw * 0.21)
            cosang = (rf * SUN).sum(1)
            glint = np.clip((cosang - 0.95) / 0.05, 0, 1) ** 1.5 * (rip > 0.62)
            wcol = refl * (0.68 + 0.14 * rip)[:, None] + np.array([30, 50, 80]) * 0.3
            wcol *= (0.72 + 0.28 * lit[wat])[:, None]
            wcol += glint[:, None] * np.array([255, 236, 190]) * 0.9
            col[wat] = wcol
        k = np.clip((dd - 60.0) / 460.0, 0, 0.75)[:, None]
        col = col * (1 - k) + HAZE[None] * k
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = dd
    fr.set_array(arr, depth)
    # fish ripples on the river
    for fx, fz, ft in S.FISH:
        u = (t - ft) / 1.6
        if 0 <= u <= 1:
            for i, rr in enumerate((0.3 + 1.6 * u, 0.1 + 1.0 * u)):
                ring = np.array([[fx + rr * math.cos(a), 0.03, fz + rr * 0.8 * math.sin(a)] for a in np.linspace(0, 6.283, 17)])
                if cam.depth(ring.mean(0))[0] > 1:
                    fr.line(cam, ring, mixc((255, 236, 210), (200, 150, 150), u), 1)
    # ---- far layer
    for faces, col, emit, name in S.FAR:
        C = np.array([Q.mean(0) for Q, n in faces])
        N = np.array([n for Q, n in faces])
        cols = shade(L, N, C, col, kvis=np.ones((len(faces), 1)), rimk=0.2, emit=emit)
        dist = np.linalg.norm(C - cam.pos, axis=1)
        kf = np.clip((dist - 40) / 500, 0, 0.78)[:, None] if name != 'crater' else np.zeros((len(faces), 1))
        cols = cols * (1 - kf) + HAZE[None] * kf
        order = np.argsort(-dist)
        for j in order:
            fr.poly(cam, faces[j][0], cols[j])
    for Q in S.LAVA:
        fr.poly(cam, Q, (255, 124 + 30 * math.sin(t * 3), 50))
    cs = cam.wp(S.CRATER)
    if cam.depth(S.CRATER)[0] > 0:
        fr.glow(cs[0], cs[1], 0.05 * cam.f * 120 / cam.depth(S.CRATER)[0], (255, 110, 50), 0.6 + 0.15 * math.sin(t * 2.1))
    for i in range(9):
        u = ((t * 0.035 + i / 9) % 1.0)
        pc = S.CRATER + np.array([8 + 40 * u + 3 * math.sin(i), 4 + 70 * u, -10 * u])
        rr = 5 + 15 * u
        fcs = icosa(pc, rr, (1.2, 0.8, 1), rng=np.random.default_rng(i))
        C = np.array([Q.mean(0) for Q, n in fcs])
        N = np.array([n for Q, n in fcs])
        pcol = mixc((150, 120, 140), (210, 170, 180), u)
        cols = shade(L, N, C, pcol, kvis=np.ones((len(fcs), 1)), rimk=0.4)
        cols = cols * 0.55 + HAZE * 0.45
        for (Q, n), c in sorted(zip(fcs, cols), key=lambda a: -cam.depth(a[0][0].mean(0))[0]):
            if np.dot(n, cam.pos - Q.mean(0)) > 0:
                fr.poly(cam, Q, c)
    # ---- near objects
    items = []
    for o in S.OBJS:
        if not onscreen(cam, o.ctr, np.linalg.norm(o.hi - o.lo) / 2 + 1):
            continue
        items.append((np.linalg.norm(o.ctr - cam.pos), 'obj', o))
    for fe in S.FERNS:
        if not onscreen(cam, fe.ctr, fe.s * 1.3):
            continue
        items.append((np.linalg.norm(fe.ctr - cam.pos), 'fern', fe))
    for k in poses:
        M, pos = poses[k]
        items.append((np.linalg.norm(pos + [0, 2.4 * T.VOICES[k]['scale'], 0] - cam.pos), 'dino', k))
    for i, df in enumerate(S.DRAGONFLIES):
        items.append((np.linalg.norm(df['c'] - cam.pos), 'fly', i))
    boxes = list(STATIC_BOXES)
    for k, (M, pos) in poses.items():
        for part, m in DINOS[k].parts(M):
            V = D.xf(m, part.V)
            boxes.append(Box(V.min(0), V.max(0), DINO_GRP[k]))
    OCC = Occluders(boxes)
    xk = ss(T.T_XRAY0, T.T_XRAY0 + 0.8, t) * (1 - ss(T.T_XRAY1 - 0.8, T.T_XRAY1, t))
    ctk = ss(T.T_CT0 - 0.2, T.T_CT0 + 0.2, t) * (1 - ss(T.T_CT1, T.T_CT1 + 0.4, t))
    items.sort(key=lambda a: -a[0])
    subj_depths = []
    for dist, kind, ob in items:
        if kind == 'obj':
            faces = [(Q, n, ob.color * ob.tone[j]) for j, (Q, n) in enumerate(ob.faces)]
            grp, double, trans = ob.grp, ob.double, 0.25 if ob.name in ('leaf',) else 0.0
            emit = 0.0
        elif kind == 'fern':
            Qs = ob.at(t)
            faces = []
            for j, Q in enumerate(Qs):
                n = np.cross(Q[1] - Q[0], Q[3] - Q[0])
                n = n / (np.linalg.norm(n) + 1e-9)
                faces.append((Q, n, ob.col * ob.tone[j]))
            grp, double, trans, emit = ob.grp, True, 0.35, 0.0
        elif kind == 'fly':
            df = S.DRAGONFLIES[ob]
            ph = t * np.array(df['w']) + ob * 1.7
            p = df['c'] + np.array(df['r']) * np.sin(ph)
            v = np.array(df['r']) * np.array(df['w']) * np.cos(ph)
            fw = v / (np.linalg.norm(v) + 1e-9)
            sd = np.cross(fw, [0, 1, 0]); sd /= np.linalg.norm(sd) + 1e-9
            wing = 0.5 + 0.5 * math.sin(t * 60 + ob)
            body = [p - fw * 0.09, p + fw * 0.06 + [0, 0.01, 0], p + fw * 0.06 - [0, 0.01, 0]]
            w1 = [p, p + sd * 0.11 + [0, 0.04 * wing, 0], p + sd * 0.1 - fw * 0.03]
            w2 = [p, p - sd * 0.11 + [0, 0.04 * wing, 0], p - sd * 0.1 - fw * 0.03]
            faces = [(np.array(body), np.array([0, 1.0, 0]), np.array(df['col'], float)),
                     (np.array(w1), np.array([0, 1.0, 0]), np.array([220, 240, 255.0])),
                     (np.array(w2), np.array([0, 1.0, 0]), np.array([220, 240, 255.0]))]
            grp, double, trans, emit = 99999, True, 0.6, 0.2
        else:
            k = ob
            faces = [(Q, n, c) for Q, n, c, tg in dfaces[k]]
            tags = [tg for Q, n, c, tg in dfaces[k]]
            grp, double, trans, emit = DINO_GRP[k], False, 0.0, 0.0
        fc = []
        for j, (Q, n, c) in enumerate(faces):
            ctr = Q.mean(0)
            facing = np.dot(n, cam.pos - ctr)
            if facing <= 0:
                if not double:
                    continue
                n = -n
            fc.append((cam.depth(ctr)[0], Q, n, c, j))
        if not fc:
            continue
        fc.sort(key=lambda a: -a[0])
        C = np.array([a[1].mean(0) for a in fc])
        N = np.array([a[2] for a in fc])
        B = np.array([a[3] for a in fc])
        kv = OCC.vis(C + N * 0.03, grp, d=SUN)[:, None]
        em = np.zeros(len(fc))
        if kind == 'dino':
            tg = [tags[a[4]] for a in fc]
            lev = T.honk_level(k, t)
            em = np.array([0.35 * lev if x == 'crest' else (0.5 if x == 'eyehi' else 0.0) for x in tg])
        cols = shade(L, N, C, B, kvis=kv, trans=trans, emit=0.0)
        if kind == 'dino':
            vc = np.array(T.VOICES[k]['col'], float)
            cols = cols + em[:, None] * vc[None]
            if k == 0 and xk > 0:
                isx = np.array([x in ('crest', 'skull', 'eye', 'eyehi') for x in tg])
                hide = 1.0 - 0.75 * ctk
                xr = np.array([18, 40, 92.0]) + 30 * np.clip(N[:, 2:3], 0, 1)
                cols = np.where(isx[:, None], cols * (1 - 0.82 * xk) + xr * 0.82 * xk, cols)
                cols = np.where(isx[:, None], cols * hide + np.array([10, 20, 40.0]) * (1 - hide), cols)
        for (z, Q, n, c, j), col in zip(fc, cols):
            fr.poly(cam, Q, col)
        if kind == 'dino' and k == 0 and xk > 0.05:
            ec = XRAY * (0.35 + 0.4 * xk) * (1 - 0.6 * ctk)
            for (z, Q, n, c, j), x in zip(fc, tg):
                if x in ('crest', 'skull'):
                    fr.line(cam, np.vstack([Q, Q[:1]]), ec, 1)
    # ---- X-ray air passage, CT slices
    if 0 in poses and xk > 0:
        M = poses[0][0]
        d0 = DINOS[0]
        path = D.xf(M['head'], np.c_[d0.air, np.zeros(len(d0.air))])
        reveal = ss(T.T_XRAY0 + 0.4, 7.4, t)
        nvis = int(len(path) * reveal)
        if nvis > 1:
            base = XRAY * (0.45 + 0.5 * xk) * (1 - 0.7 * ctk)
            pts = path[:nvis]
            fr.line(cam, pts, base * 0.8, 2)
            fr.line(cam, pts, np.minimum(base * 1.4, 255), 1)
        if ctk > 0.01:
            cards = d0.ct
            nc = int(len(cards) * T.ct_frac(t))
            polys = []
            angs = np.linspace(0, 2 * math.pi, 9)[:-1] + math.pi / 8
            for i, (x, yc, ry, rz, hole) in enumerate(cards[:nc]):
                ring = np.array([[x, yc + ry * math.cos(a), rz * math.sin(a)] for a in angs])
                inner = np.array([[x, yc + 0.02 + ry * hole * math.cos(a), rz * hole * math.sin(a)] for a in angs])
                Wp = D.xf(M['head'], ring)
                Wi = D.xf(M['head'], inner)
                polys.append((cam.depth(Wp.mean(0))[0], i, Wp, Wi))
            polys.sort(key=lambda a: -a[0])
            for z, i, Wp, Wi in polys:
                newk = max(0.0, 1 - (nc - 1 - i) / 2.0)
                col = mixc((60, 150, 200), (160, 244, 255), 0.25 + 0.75 * (i / max(1, len(cards)))) * ctk + np.array([255, 255, 255]) * newk * 0.22
                fr.poly(cam, Wp, np.minimum(col, 255))
                fr.poly(cam, Wi, np.array([10, 22, 50.0]) * ctk + np.array([20, 40, 80]) * (1 - ctk))
                fr.line(cam, np.vstack([Wp, Wp[:1]]), np.minimum(col * 1.3 + 30, 255), 1)
            if nc < len(cards) and nc > 0:
                x, yc, ry, rz, hole = cards[nc - 1]
                sq = np.array([[x, yc - 0.45, -0.4], [x, yc + 0.45, -0.4], [x, yc + 0.45, 0.4], [x, yc - 0.45, 0.4]]) * [1, 1, 1]
                Wp = D.xf(M['head'], sq)
                fr.line(cam, np.vstack([Wp, Wp[:1]]), (200, 250, 255), 1)
        # glowing air flowing through the passage
        airk = ss(T.T_AIR0, T.T_AIR0 + 0.4, t) * (1 - ss(T.T_AIR1 - 0.4, T.T_AIR1, t))
        if airk > 0:
            npath = len(path)
            for j in range(14):
                u = ((t - T.T_AIR0) * 0.55 + j / 14) % 1.0
                i = u * (npath - 1)
                i0 = int(i)
                p = path[i0] + (path[min(i0 + 1, npath - 1)] - path[i0]) * (i - i0)
                s = cam.wp(p)
                fr.glow(s[0], s[1], 0.02 * cam.f / max(cam.depth(p)[0], 0.5), (255, 220, 120), 0.9 * airk)
            snout = path[0]
            s = cam.wp(snout)
            fr.glow(s[0], s[1], 0.05 * cam.f / max(cam.depth(snout)[0], 0.5), (255, 200, 120), 0.4 * airk)
    # small flowers and fireflies (sprites with depth)
    for p in S.FLOWERS:
        z = cam.depth(p)[0]
        if z > 0.5:
            s = cam.wp(p)
            if -5 < s[0] < W + 5 and -5 < s[1] < H + 5:
                fr.disc(s, max(1.0, 0.09 * cam.f / z), (255, 226, 236), n=5, z=z)
    for p, b in zip(ff_pos(t), ff_bright(t)):
        z = cam.depth(p)[0]
        if z > 0.5 and b > 0.15:
            s = cam.wp(p)
            fr.glow(s[0], s[1], max(1.5, 0.05 * cam.f / z), (255, 230, 110), 0.55 * b)
    # ---- lens: focus locked on the subject (the whole visible herd stays inside the sharp band)
    img, depth = fr.array(), fr.depth()
    zf, sharp, K = focus(cam, dfaces)
    coc = coc_map(depth, zf, K, sharp=sharp)
    img = dof(img, coc)
    if os.environ.get('FOCUS_LOG'):
        with open(os.environ['FOCUS_LOG'], 'a') as fh:
            fh.write(f'{f} {subject_blur(cam, dfaces, zf, sharp, K):.4f}\n')
    # glows (sun, crater, air, fireflies, crest glow) and sound rings
    if sp is not None:
        fr.glow(sp[0], sp[1], min(40, 0.045 * cam.f * (1 + 0.5 * fl)), (255, 180, 100), 0.22 + 0.3 * fl)
    for k, (M, pos) in poses.items():
        lev = T.honk_level(k, t)
        if lev > 0.01:
            p = crest_top(k, M)
            z = cam.depth(p)[0]
            if z > 0.3:
                s = cam.wp(p)
                fr.glow(s[0], s[1], max(3, 0.5 * T.VOICES[k]['scale'] * cam.f / z), T.VOICES[k]['col'], 0.35 * lev)
    img = apply_glows(img, fr.glows)
    for h in T.HONKS:
        k = h[2]
        if k not in poses or not (h[0] <= t <= h[0] + h[1] + 1.2):
            continue
        M = poses[k][0]
        p = head_point(k, M, (1.05, -0.12, 0))       # the snout
        z = cam.depth(p)[0]
        if z < 0.3:
            continue
        s = cam.wp(p)
        sc = cam.f / z * T.VOICES[k]['scale']
        nr = 1 + int(min(h[1], 3.0) / 0.45)
        for i in range(nr):
            age = t - h[0] - i * 0.45
            if 0 < age < 1.2:
                add_ring(img, s[0], s[1], (0.3 + 2.2 * age) * sc, 1.6, T.VOICES[k]['col'], 0.8 * (1 - age / 1.2))
    if sp is not None:
        img = god_rays(img, depth, sp, (255, 190, 120), 0.32 + 0.3 * fl)
    img = grade(bloom(np.clip(img, 0, 255), thr=210, k=0.3 + 0.2 * fl), contrast=0.2, sat=1.18)
    img = np.clip(img * vignette(0.8, 0.6, 0.35) * top_scrim(), 0, 255).astype(np.uint8)
    # ---- UI (never blurred)
    out = draw_ui(img, t, cam, poses)
    return out


# ------------------------------------------------------------------ UI
def draw_bottle(d, out_marks, x, y, s, water, blowing, t):
    """pixel-art bottle icon at (x, y) top-left, s = pixel scale, water 0..1"""
    w, h = 9 * s, 22 * s
    neck_w, neck_h = 3 * s, 7 * s
    glass = (180, 230, 240)
    body = [x, y + neck_h, x + w, y + h]
    neck = [x + (w - neck_w) // 2, y, x + (w + neck_w) // 2, y + neck_h]
    d.rectangle([body[0] - 1, body[1] - 1, body[2] + 1, body[3] + 1], fill=(12, 20, 34))
    d.rectangle([neck[0] - 1, neck[1] - 1, neck[2] + 1, neck[3] + 1], fill=(12, 20, 34))
    d.rectangle(body, fill=(40, 70, 90))
    d.rectangle(neck, fill=(40, 70, 90))
    wy = body[3] - int((body[3] - body[1]) * water)
    d.rectangle([body[0] + 1, wy, body[2] - 1, body[3] - 1], fill=(70, 150, 230))
    d.rectangle([body[0] + 1, body[1] + 1, body[2] - 1, wy - 1], fill=(60, 96, 116))
    # air column highlight
    d.rectangle([neck[0] + 1, neck[1] + 1, neck[2] - 1, neck[3]], fill=(255, 214, 120) if blowing else (80, 120, 140))
    if blowing:
        d.rectangle([body[0] + 2, body[1] + 1, body[2] - 2, wy - 2], fill=(255, 200, 110))
    d.line([body[0] + s, body[1] + s, body[0] + s, body[3] - 2 * s], fill=glass)
    if blowing:
        for i in range(3):
            yy = neck[1] - 2 * s - i * 2 * s
            off = int(3 * s * math.sin(t * 12 + i))
            d.line([neck[0] - 4 * s + off, yy, neck[2] + 4 * s + off, yy], fill=(230, 240, 255))
    return wy


def tube(d, x, y, length, h, col, active, t, freq):
    d.rectangle([x - 1, y - 1, x + length + 1, y + h + 1], fill=(12, 20, 34))
    body = tuple(int(c * (0.42 if active else 0.26)) for c in col)
    d.rectangle([x, y, x + length, y + h], fill=body)
    d.rectangle([x, y, x + length, y + 2], fill=tuple(int(c * 0.6) for c in col))
    if length > 6:
        pts = []
        amp = (h / 2 - 3) * (1.0 if active else 0.3)
        for i in range(int(length)):
            u = i / length
            v = math.sin(math.pi * u * 0.5) * math.cos(t * freq) * amp          # closed-end tube: quarter wave
            pts.append((x + i, y + h / 2 + v))
        d.line(pts, fill=col if active else tuple(int(c * 0.6) for c in col), width=2)
    d.rectangle([x + length - 2, y - 2, x + length + 2, y + h + 2], fill=(160, 170, 190))     # closed end


def draw_ui(img, t, cam, poses):
    pil = Image.fromarray(img)
    d = ImageDraw.Draw(pil)
    texts = []
    # tube-length panel (16-20): longer tube = lower note, half the length = one octave up
    if T.T_TUBE0 <= t <= T.T_TUBE1 + 0.4:
        off = slide_offset(t, T.T_TUBE0, T.T_TUBE1, dist=150)
        y0 = 504 + off
        panel(d, 14, y0, 346, y0 + 112)
        g1 = ease((t - T.T_TUBE0 - 0.2) / 0.6)
        g2 = ease((t - 17.9) / 0.5)
        a1 = T.TEST_TONES[0][0] <= t <= T.TEST_TONES[0][0] + T.TEST_TONES[0][1]
        a2 = T.TEST_TONES[1][0] <= t <= T.TEST_TONES[1][0] + T.TEST_TONES[1][1]
        tube(d, 28, y0 + 20, 232 * g1, 24, (255, 150, 90), a1, t, 9.0)
        tube(d, 28, y0 + 66, 116 * g2, 24, (110, 220, 255), a2, t, 18.0)
        draw_bottle(d, None, 300, y0 + 26, 3, 0.45, a1 or a2, t)
        texts.append(("LOW", 28 + int(232 * g1) - 18 if g1 > 0.2 else -99, y0 + 8, (255, 170, 110), 1))
        texts.append(("HIGH", 28 + int(116 * g2) - 22 if g2 > 0.2 else -99, y0 + 52, (140, 230, 255), 1))
    # piano roll (who plays what: big = low, small = high)
    if T.T_STAFF0 <= t <= T.T_END + 0.6:
        off = slide_offset(t, T.T_STAFF0, T.T_END + 0.2, dist=150)
        y0 = 522 + off
        x0, x1 = 14, 346
        panel(d, x0, y0, x1, y0 + 96)
        play = 236
        spd = 46.0
        top, bot = y0 + 8, y0 + 88
        ym = lambda m: bot - (m - 26) * (bot - top) / (69 - 26)
        for m in (31, 43, 55, 67):
            d.line([x0 + 18, ym(m), x1 - 4, ym(m)], fill=(30, 38, 62))
        for h in T.HONKS:
            t0, dur, k, m, st = h
            if not T.present(k, t0):
                continue
            xa = play + (t0 - t) * spd
            xb = xa + dur * spd
            if xb < x0 + 18 or xa > x1 - 4:
                continue
            xa, xb = max(xa, x0 + 18), min(xb, x1 - 4)
            col = np.array(T.VOICES[k]['col'], float)
            on = t0 <= t <= t0 + dur
            if xb < play:
                col = col * 0.45
            elif not on and xa > play:
                col = col * 0.7
            y = ym(m)
            d.rectangle([xa, y - 2, xb, y + 1], fill=tuple(int(c) for c in col))
            if on:
                d.rectangle([play - 3, y - 3, play + 3, y + 2], fill=(255, 255, 240))
        d.line([play, y0 + 4, play, y0 + 92], fill=(200, 210, 240))
        texts.append(("HI", x0 + 3, top - 1, (140, 220, 255), 1))
        texts.append(("LO", x0 + 3, bot - 7, (255, 140, 120), 1))
    # bottle panel (try it)
    if T.T_END + 0.2 <= t <= 57.4:
        off = slide_offset(t, T.T_END + 0.4, 57.0, dist=160)
        y0 = 500 + off
        panel(d, 100, y0, 260, y0 + 118)
        blow = [b for b in T.BOTTLE_BLOWS if b[0] - 0.6 <= t]
        water = blow[-1][3] if blow else T.BOTTLE_BLOWS[0][3]
        blowing = any(b[0] <= t <= b[0] + b[1] for b in T.BOTTLE_BLOWS)
        draw_bottle(d, None, 162, y0 + 22, 4, water, blowing, t)
        texts.append(("MORE AIR", 112, y0 + 50, CREAM, 1))
        texts.append(("= LOWER", 206, y0 + 50, AMBER, 1))
    out = np.asarray(pil).copy()
    if 12.3 <= t <= 16.0:
        a = ss(12.3, 12.6, t) * (1 - ss(15.7, 16.0, t))
        if a > 0.5:
            m = text_mask("SANDIA LAB SIM, 1997", 1)
            stamp(out, m, (W - m.shape[1]) // 2, 600, (190, 230, 255))
            m2 = text_mask("A MODEL, NOT A RECORDING", 1)
            stamp(out, m2, (W - m2.shape[1]) // 2, 612, (150, 180, 210))
    for s, x, y, col, sc in texts:
        if x > -50:
            label(out, s, x, y, col, sc)
    # floating pixel notes from honking crests
    for h in T.HONKS:
        k = h[2]
        if k not in poses:
            continue
        spawns = [h[0] + 0.5 * i for i in range(1 + int(h[1] / 0.9))]
        for i, ts in enumerate(spawns):
            age = t - ts
            if not 0 <= age <= 1.5:
                continue
            p = crest_top(k, poses[k][0])
            z = cam.depth(p)[0]
            if z < 0.3:
                continue
            s = cam.wp(p)
            x = s[0] + 10 * math.sin(age * 4 + i + k) + 6 * (k - 2)
            y = s[1] - 10 - 34 * age
            if y < 130:
                continue
            a = 1 - ss(1.0, 1.5, age)
            stamp(out, NOTE_M2, int(x), int(y), T.VOICES[k]['col'], alpha=a)
    draw_captions(out, t, T.captions(t))
    return out
