"""Bounce Royale renderer.

sky (indigo -> magenta city glow, magenta moon, drones) -> the city across the street (towers, lit windows, neon
banners, beacons; fogged) -> the wet roof per pixel (moonlight + shadows, door lamp, sign, projector and the ring's
own coloured light, puddles, the mirrored hologram, rain sparkles) -> props in painter order -> the hologram: dark
inner disc, projector beam, 48 neon tiles (repaint flashes, hit bumps), splash arcs, balls + trails, pops,
collision sparks -> locked DOF on the ring -> glows, bloom, grade -> rain -> share bar, labels, impact captions.

    python -m reelkit.sheet render.py test/sheet.png 0.5 1.5 6 14 21 25 27 33 40 45 48 53 59.7
    python -m reelkit.video render.py audio.wav bounce_royale.mp4"""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, SAFE, SAFE_CX, dirv, whole_period
from reelkit.camera import Cam
from reelkit.raster import Frame, painter, add_glow
from reelkit.lighting import Lights, shade, fog
from reelkit.shadows import MaskGrid, directional_mask, Occluders, contact_ao
from reelkit.lens import coc_map, dof
from reelkit.post import bloom, grade, vignette, top_scrim
from reelkit.geometry import icosa, Obj
from reelkit import ui, font
import timeline as T
import scene as S

font.GLYPHS['·'] = np.array([[c == '#' for c in r] for r in ".....|.....|.....|..#..|.....|.....|.....".split('|')])

DUR = T.DUR
NT = T.NT
DTH = 2 * math.pi / NT

# ================================================================== camera
FOV, D0, SY = 50.0, 8.67, 255.0
F_PX = (H / 2) / math.tan(math.radians(FOV / 2))
RING_SX = SAFE_CX                       # ring centred on the safe box (x 168), not the frame


def camera(t):
    d = D0 * T.cam_zoom(t)
    yaw = 2.2 * math.sin(2 * math.pi * t / DUR + 0.3)
    pitch = 1.0 + 0.5 * math.sin(2 * math.pi * 2 * t / DUR + 1.2)
    off = (W / 2 - RING_SX) * d / F_PX                 # shift the target right so the ring sits at x 168
    c, s = math.cos(math.radians(yaw)), math.sin(math.radians(yaw))
    tgt = S.RC + np.array([off * c, 0.0, -off * s])
    return Cam(tgt, d, yaw, pitch, FOV, SY)


# ================================================================== static lighting
FLOOR = MaskGrid((-9.0, 9.0, -6.2, 10.0, 0.03))
MOON_MASK = directional_mask(FLOOR, S.CASTERS, S.KEY, blur=1.6)
OCC = Occluders(S.STATIC)
STAT_KV = []
for o in S.STATIC:
    C = np.array([Q.mean(0) for Q, n in o.faces]); Nn = np.array([n for Q, n in o.faces])
    STAT_KV.append((C, Nn, OCC.vis(C + Nn * 0.02, o.grp, d=S.KEY)))
WARM = np.array([1.25, 0.72, 0.36])
PINK = np.array([1.2, 0.35, 0.8])
CYAN = np.array([0.35, 0.9, 1.15])


def sign_flick(t):
    return 0.9 + 0.1 * math.sin(2 * math.pi * 37 * t / DUR) * math.sin(2 * math.pi * 23 * t / DUR + 0.7)


def win_pulse(t):
    """the winner's bounces pulse the city's neon (after the win), gentle"""
    if T.T_WIN is None or t < T.T_WIN:
        return 0.0
    i1 = np.searchsorted(T.HIT_T, t)
    i0 = np.searchsorted(T.HIT_T, t - 0.6)
    v = 0.0
    for i in range(i0, i1):
        if T.HIT_Q[i] == T.WINNER:
            v += math.exp(-(t - T.HIT_T[i]) / 0.16)
    fade = 1 - T.ease((t - (T.T_COLLAPSE + 1.5)) / 2.0)
    return min(1.0, v) * fade


def seam_charge(t):
    """0..1: the closing point charges up into the snap (end of the video) and fades out of it (frame 0)"""
    if t > T.T_COLLAPSE:
        u = (t - T.T_COLLAPSE) / (DUR - T.T_COLLAPSE)
        return u ** 2
    if t < 0.35:
        return 1 - t / 0.35
    return 0.0


LILAC = np.array([220, 200, 255.0])


def ring_lights(t, own, R):
    """8 coloured lights round the ring (the hologram lights the roof and props); continuous across the loop point"""
    out = []
    pulse = 1 + 0.3 * win_pulse(t)
    ch = seam_charge(t)
    gain = 0.3 + 0.7 * min(1.0, T.ring_sim(t))
    if t > T.T_COLLAPSE:
        gain = gain * (1 - ch) + 1.0 * ch
    for j in range(8):
        a = math.pi / 2 - (j + 0.5) * 2 * math.pi / 8
        k = int(((math.pi / 2 - a) % (2 * math.pi)) // DTH) % NT
        cols = T.COL[own[[(k + d) % NT for d in (-2, -1, 0, 1, 2)]]].mean(0) / 255.0
        cols = cols * (1 - ch) + LILAC / 255.0 * ch
        p = S.RC + np.array([math.cos(a), math.sin(a), 0.0]) * S.RW * max(R, 0.02)
        out.append((p, cols * 0.42 * pulse * gain, 1.3))
    return out


def lights(t, cam, rl):
    L = Lights(amb=S.AMB, sky=S.SKY_AMB)
    L.view = cam.view
    L.key('moon', S.KEY, S.KEY_COL)
    L.fill(dirv(160, 25), (0.10, 0.10, 0.2))
    L.point(S.DOOR_LAMP, WARM * 1.1, 1.6)
    L.point(S.SIGN_C + [0, 0, 0.3], PINK * 0.9 * sign_flick(t), 1.8)
    L.point(S.PROJ_LENS + [0, 0.1, 0], CYAN * 0.6, 0.9)
    for p, c, r in rl:
        L.point(p, c, r)
    return L


# ================================================================== sky + city
STAR = []
_sr = np.random.default_rng(13)
for _ in range(90):
    STAR.append((dirv(_sr.uniform(-22, 22), _sr.uniform(8, 30)), _sr.uniform(0.2, 0.8) ** 2, _sr.uniform(0, 6.3), int(_sr.integers(5, 19))))
HAZE = np.array([136, 62, 138.0])         # city haze toward the horizon


def sky(cam, t):
    d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1) @ cam.M
    d /= np.linalg.norm(d, axis=-1, keepdims=True)
    el = np.arcsin(np.clip(d[..., 1], -1, 1))
    k = np.clip(el / 0.38, 0, 1)[..., None] ** 0.7
    top, mid, low = np.array([24, 18, 66.0]), np.array([70, 34, 118.0]), np.array([190, 72, 156.0])
    col = np.where(k < 0.45, low * (1 - k / 0.45) + mid * (k / 0.45), mid * (1 - (k - 0.45) / 0.55) + top * ((k - 0.45) / 0.55))
    cg = (d * S.MOON_DRAW[None, None]).sum(-1)
    col = col + (np.clip((cg - 0.93) / 0.07, 0, 1) ** 2.5)[..., None] * np.array([150, 70, 150.0]) * 0.75
    img = np.floor(col / 6 + BAYER_FULL[..., None]) * 6
    for dv, b, ph, kk in STAR:
        s = cam.screen_of_dir(dv)
        if s is None:
            continue
        x, y = int(s[0]), int(s[1])
        if 0 <= x < W and 0 <= y < H:
            img[y, x] = np.maximum(img[y, x], 90 + 120 * b * (0.7 + 0.3 * math.sin(2 * math.pi * kk * t / DUR + ph)))
    return img


def draw_moon(fr, cam):
    ms = cam.screen_of_dir(S.MOON_DRAW)
    if ms is None:
        return None
    r = 0.026 * cam.f
    fr.disc(ms, r * 1.3, (200, 90, 196), n=20)
    fr.disc(ms, r, (255, 214, 252), n=20)
    for dx, dy, rr in [(-0.3, -0.2, 0.26), (0.28, 0.18, 0.2), (0.05, 0.42, 0.14)]:
        fr.disc((ms[0] + dx * r, ms[1] + dy * r), rr * r, (250, 176, 240), n=10)
    fr.glow(ms[0], ms[1], 0.1 * cam.f, (255, 110, 230), 0.6)
    fr.glow(ms[0], ms[1], 0.035 * cam.f, (255, 220, 255), 0.5)
    return ms


TW_FACES = []                             # per tower: [(Q, n)], window index range
_wi = 0
_wt = np.zeros(len(S.WIN_P), int)
for ti, d in enumerate(S.TOWERS):
    from reelkit.geometry import box as _box
    hy = (d['top'] - S.STREET_Y) / 2
    fs = _box(d['x'], S.STREET_Y + hy, d['z'], d['hw'], hy, d['hd'], d['rot'])
    TW_FACES.append(fs)
# windows -> tower index (nearest centre)
_tc = np.array([[d['x'], d['z']] for d in S.TOWERS])
_wt = np.argmin(((S.WIN_P[:, None, [0, 2]] - _tc[None]) ** 2).sum(-1), 1)
WIN_BY_T = [np.nonzero(_wt == ti)[0] for ti in range(len(S.TOWERS))]


def city(fr, cam, L, t):
    arr = None
    wp = win_pulse(t)
    for ti, d in enumerate(S.TOWERS):
        dist = float(np.linalg.norm(np.array([d['x'], 0, d['z']]) - cam.pos))
        fk = min(0.82, max(0.0, (dist - 10) / 90))
        for Q, n in TW_FACES[ti]:
            if np.dot(n, cam.pos - Q.mean(0)) <= 0:
                continue
            c = shade(L, n[None], Q.mean(0)[None], d['col'])[0]
            c = c * (1 - fk) + HAZE * fk * 0.8
            fr.poly(cam, Q, c)
        idx = WIN_BY_T[ti]
        if len(idx):
            P = S.WIN_P[idx]
            Pc = cam.to_cam(P)
            ok = Pc[:, 2] > 1
            if ok.any():
                s = cam.proj(Pc[ok])
                xi, yi = s[:, 0].astype(int), s[:, 1].astype(int)
                inb = (xi >= 0) & (xi < W - 1) & (yi >= 0) & (yi < H)
                if inb.any():
                    if arr is None:
                        arr = np.asarray(fr.pil).copy()
                        dep = np.asarray(fr.dep).copy()
                    col = S.WIN_C[idx][ok][inb] * (1 - fk * 0.6) * (1 + 0.25 * wp) + HAZE * fk * 0.3
                    z = Pc[ok][inb, 2]
                    xi, yi = xi[inb], yi[inb]
                    vis = z <= dep[yi, xi] + 0.5
                    xi, yi, col = xi[vis], yi[vis], np.clip(col[vis], 0, 255).astype(np.uint8)
                    arr[yi, xi] = col
                    arr[yi, xi + 1] = (col * 0.8).astype(np.uint8)
                    yb = np.minimum(yi + 1, H - 1)
                    arr[yb, xi] = (col * 0.6).astype(np.uint8)
                    fr.set_array(arr.astype(np.float32), dep)
                    arr = None
    for q, col, ph, k in S.BANNERS:
        flick = 0.85 + 0.15 * math.sin(2 * math.pi * k * t / DUR + ph)
        c = col * flick * (1 + 0.35 * wp)
        dist = float(np.linalg.norm(q.mean(0) - cam.pos))
        fk = min(0.6, max(0.0, (dist - 15) / 110))
        pts = fr.poly(cam, q, np.clip(c * (1 - fk) + HAZE * fk, 0, 255))
        if pts is not None:
            s = cam.wp(q.mean(0))
            fr.glow(s[0], s[1], 4 + 70 / dist, tuple(col), 0.22 * flick * (1 + 0.5 * wp))
    for p, ph, k in S.BEACONS:
        on = ((k * t / DUR + ph) % 1.0) < 0.14
        if on:
            s = cam.wp(p)
            fr.glow(s[0], s[1], 3, (255, 60, 60), 0.9)
    for c0, rx, h, k, ph in S.DRONES:
        a = 2 * math.pi * k * t / DUR + ph
        p = c0 + np.array([rx * math.cos(a), h * math.sin(2 * a) * 0.3, rx * 0.4 * math.sin(a)])
        s = cam.wp(p)
        if 0 <= s[0] < W and 0 <= s[1] < H:
            fr.draw.rectangle([s[0] - 2, s[1], s[0] + 2, s[1]], fill=(30, 28, 44))
            blink = ((3 * k * t / DUR * 10 + ph) % 1.0) < 0.3
            fr.glow(s[0] - 2, s[1], 2, (255, 70, 70) if blink else (120, 40, 40), 0.8)
            fr.glow(s[0] + 2, s[1], 2, (70, 255, 140), 0.6)


# ================================================================== the wet roof
def _puddle_mask(X, Z):
    m = np.zeros(np.shape(X), np.float32)
    for (px, pz, rx, rz) in S.PUDDLES:
        d = ((X - px) / rx) ** 2 + ((Z - pz) / rz) ** 2
        m = np.maximum(m, np.clip((1.15 - d) * 3, 0, 1))
    wob = 0.5 + 0.5 * np.sin(X * 2.3 + np.sin(Z * 1.7) * 2) * np.sin(Z * 2.9 + X * 0.8)
    return np.clip(m * (0.75 + 0.25 * wob), 0, 1)


def ring_texture(t, own, R, balls):
    """the hologram drawn in its own plane (for the roof's mirror image): 128 x 128 over [-1.3, 1.3] ring radii"""
    n = 128
    im = Image.new('RGB', (n, n), (0, 0, 0))
    dr = ImageDraw.Draw(im)
    sc = n / 2.6
    c = n / 2
    rr = R * sc
    th = max(2, int(0.11 * sc))
    for k in range(NT):
        a0 = 90 - (k + 1) * 7.5 + 0.5
        a1 = 90 - k * 7.5 - 0.5
        col = tuple(int(v) for v in T.COL[own[k]] * 0.9)
        dr.arc([c - rr, c - rr, c + rr, c + rr], -a1, -a0, fill=col, width=th)
    for q, (x, y, alive) in enumerate(balls):
        if alive:
            bx, by = c + x * sc, c - y * sc
            r = 0.07 * sc
            dr.ellipse([bx - r, by - r, bx + r, by + r], fill=tuple(int(v) for v in T.COL[q]))
    im = im.filter(ImageFilter.GaussianBlur(1.6))
    return np.asarray(im, np.float32), sc, c


def roof(fr, cam, L, t, own, R, balls, rl, f):
    arr, depth = fr.array(), fr.depth()
    ys, xs = np.nonzero(YY > 255)
    X, Z, d = cam.ground_hit(xs.astype(float), ys.astype(float))
    ok = (d > 0) & (d < depth[ys, xs]) & (Z > S.PAR_Z + 0.15) & (np.abs(X) < 9)
    xs, ys, X, Z, d = xs[ok], ys[ok], X[ok], Z[ok], d[ok]
    # concrete slabs with seams, darker when wet
    sx, sz = (X + 20) / 1.25, (Z + 20) / 1.25
    seam = (np.abs(sx % 1 - 0.5) > 0.47) | (np.abs(sz % 1 - 0.5) > 0.47)
    tone = 0.9 + 0.1 * (((np.floor(sx) * 7 + np.floor(sz) * 3) % 5) / 4)
    pud = _puddle_mask(X, Z)
    alb = np.array([74, 70, 92.0])[None] * tone[:, None]
    alb = np.where(seam[:, None], alb * 0.6, alb)
    alb = alb * (1 - 0.45 * pud[:, None])
    I = np.tile(L.amb + L.sky, (len(X), 1))
    lit = 1 - FLOOR.sample(MOON_MASK, X, Z)
    I += (lit * S.KEY[1] * 1.1)[:, None] * S.KEY_COL[None]
    for lp, lc, Rr in L.points:
        dist = np.sqrt((lp[0] - X) ** 2 + lp[1] ** 2 + (lp[2] - Z) ** 2)
        I += (lp[1] / dist * 1.9 / (1 + (dist / (Rr * 1.3)) ** 2))[:, None] * lc[None]
    I *= contact_ao(X, Z, S.FEET)[:, None]
    col = alb * I
    # mirror image of the hologram (eye mirrored in the roof, ray to the ring plane z = 0)
    eye = cam.pos * np.array([1, -1, 1])
    dz = Z - eye[2]
    s = -eye[2] / np.where(np.abs(dz) < 1e-6, -1e-6, dz)
    Qx = eye[0] + s * (X - eye[0])
    Qy = eye[1] + s * (0 - eye[1])
    rip = 0.04 * np.sin(X * 13 + t * 2 * math.pi * 3) * np.sin(Z * 11 - t * 2 * math.pi * 2)
    tex, sc, c0 = ring_texture(t, own, R, balls)
    u = ((Qx - S.RC[0]) / S.RW + rip) * sc + c0
    v = c0 - ((Qy - S.RC[1]) / S.RW + rip * 0.6) * sc
    inside = (s > 1) & (Z > 0.05) & (u >= 0) & (u < 127) & (v >= 0) & (v < 127)
    wet = 0.22 + 0.55 * pud
    if inside.any():
        rc = tex[v[inside].astype(int), u[inside].astype(int)]
        fall = np.exp(-np.maximum(Z[inside] - 0.4, 0) / 3.5)
        col[inside] += rc * (wet[inside] * fall * 0.95)[:, None]
    # neon streaks: the sign, the door lamp and the moon smeared down the wet roof
    for lp, lc, k in ((S.SIGN_C, np.array([255, 80, 200.0]), 0.5 * sign_flick(t)), (S.DOOR_LAMP, np.array([255, 170, 90.0]), 0.45)):
        m = cam.wp(lp * np.array([1, -1, 1]))
        dx = (xs - m[0]) / 9.0
        dyv = (ys - m[1]) / 60.0
        g = np.exp(-dx * dx - dyv * dyv) * k * (0.3 + 0.7 * pud)
        col += g[:, None] * lc[None]
    # a sheen of the magenta sky toward the parapet
    far = np.clip((d - 6) / 10, 0, 1)
    col += (far * (0.25 + 0.4 * pud))[:, None] * np.array([90, 34, 80.0])[None]
    # rain hits: tiny sparkles (deterministic per frame)
    rr = np.random.default_rng(f * 7 + 1)
    hit = rr.random(len(X)) < 0.004
    col[hit] += np.array([90, 90, 120.0])
    arr[ys, xs] = np.floor(col / 5 + BAYER_FULL[ys, xs, None]) * 5
    depth[ys, xs] = d
    fr.set_array(arr, depth)


# ================================================================== props
def draw_static(fr, cam, L, idx_list):
    for i in idx_list:
        o = S.STATIC[i]
        C, Nn, kv = STAT_KV[i]
        vis = (Nn * (cam.pos[None] - C)).sum(1) > 0
        if not vis.any():
            continue
        ids = np.nonzero(vis)[0]
        ids = ids[np.argsort(-cam.depth(C[ids]))]
        emit = {'proj_lens': 0.9, 'lamp': 1.0}.get(o.name, 0.0)
        cols = shade(L, Nn[ids], C[ids], np.array([o.color * o.tone[j] for j in ids]), kvis=kv[ids][:, None], emit=emit)
        for j, c in zip(ids, cols):
            fr.poly(cam, o.faces[j][0], c)


SIGN_IMG = None


def _sign_img():
    """the neon sign face: a pink frame and a pixel music note"""
    w, h = 44, 48
    im = np.zeros((h, w, 3), np.float32)
    im[:, :] = (30, 16, 40)
    for b in range(2):
        im[b, :] = im[h - 1 - b, :] = im[:, b] = im[:, w - 1 - b] = (255, 90, 210)
    note = ["....#####", "....#...#", "....#...#", "....#...#", "....#...#", ".####.####", "#####.####", ".###...##."]
    for j, row in enumerate(note):
        for i, ch in enumerate(row):
            if ch == '#':
                im[14 + j * 3:17 + j * 3, 8 + i * 3:11 + i * 3] = (90, 240, 255)
    return im


def draw_sign(fr, cam, t):
    global SIGN_IMG
    if SIGN_IMG is None:
        SIGN_IMG = _sign_img()
    sg = S.SIGN
    q = np.array([[sg['x'] - sg['hw'], sg['y1'], sg['z']], [sg['x'] + sg['hw'], sg['y1'], sg['z']],
                  [sg['x'] + sg['hw'], sg['y0'], sg['z']], [sg['x'] - sg['hw'], sg['y0'], sg['z']]])
    s = cam.wp(q)
    x0, y0 = int(s[:, 0].min()), int(s[:, 1].min())
    x1, y1 = int(s[:, 0].max()), int(s[:, 1].max())
    if x1 <= x0 or y1 <= y0 or x0 >= W or x1 < 0:
        return
    im = np.asarray(Image.fromarray(np.clip(SIGN_IMG * sign_flick(t), 0, 255).astype(np.uint8)).resize((x1 - x0, y1 - y0), Image.NEAREST))
    arr, dep = fr.array(), fr.depth()
    z = float(cam.depth(S.SIGN_C)[0])
    xa, xb = max(0, x0), min(W, x1)
    ya, yb = max(0, y0), min(H, y1)
    sub = im[ya - y0:yb - y0, xa - x0:xb - x0]
    vis = dep[ya:yb, xa:xb] > z
    arr[ya:yb, xa:xb][vis] = sub[vis]
    dep[ya:yb, xa:xb][vis] = z
    fr.set_array(arr, dep)
    sc = cam.wp(S.SIGN_C)
    fr.glow(sc[0], sc[1], 16, (255, 80, 200), 0.3 * sign_flick(t))


# ================================================================== the hologram
TILE_SUB = 3


def tile_quad(k, R, bump, z, th):
    """front polygon of tile k (outer arc then inner arc), world space"""
    a0 = math.pi / 2 - k * DTH - 0.012
    a1 = math.pi / 2 - (k + 1) * DTH + 0.012
    ro = (R + th + bump)
    ri = max(0.0, R - th + bump)
    ang = np.linspace(a0, a1, TILE_SUB)
    outer = [[S.RC[0] + ro * math.cos(a), S.RC[1] + ro * math.sin(a), z] for a in ang]
    inner = [[S.RC[0] + ri * math.cos(a), S.RC[1] + ri * math.sin(a), z] for a in ang[::-1]]
    return np.array(outer + inner)


def ring_state(t):
    own = T.owner_at(t)
    f = T.frame_of(t)
    lf = T.LAST_FLIP[f].copy()
    i0, i1 = np.searchsorted(T.FLIP_T, [f / FPS + 1e-9, t + 1e-9])
    for i in range(i0, i1):
        lf[T.FLIP[i, 1]] = T.FLIP_T[i]
    return own, lf


def tile_hits(t, R):
    """radial bump per tile from bounces in the last 0.14 s"""
    bump = np.zeros(NT)
    i0, i1 = np.searchsorted(T.HIT_T, [t - 0.14, t])
    for i in range(i0, i1):
        k = int(((math.pi / 2 - T.HIT_A[i]) % (2 * math.pi)) // DTH) % NT
        u = (t - T.HIT_T[i]) / 0.14
        for dk, w in ((0, 1.0), (-1, 0.4), (1, 0.4)):
            bump[(k + dk) % NT] = max(bump[(k + dk) % NT], w * (1 - u) * 0.045)
    return bump


def flood_flash(t, k):
    """the winner's flood: a bright wave round the ring from the final pop"""
    if T.T_WIN is None or not (T.T_WIN <= t < T.T_WIN + 1.2):
        return 0.0
    p = T.POPS[-1][2]
    a0 = math.atan2(p[1], p[0])
    ak = math.pi / 2 - (k + 0.5) * DTH
    dist = abs((ak - a0 + math.pi) % (2 * math.pi) - math.pi) / math.pi
    u = t - T.T_WIN - 0.7 * dist
    return math.exp(-(u / 0.12) ** 2) if u > -0.3 else 0.0


def draw_ring(fr, cam, L, t, own, lf, R, sub_mask):
    Rw = R * S.RW
    if Rw < 0.004:
        s = cam.wp(S.RC)
        fr.glow(s[0], s[1], 10, tuple(T.COL[T.WINNER]), 0.9)
        return
    th = S.TILE_TH * min(1.0, 0.45 + 0.55 * R) * min(1.0, R / 0.12)
    bump = tile_hits(t, R) * S.RW * min(1.0, R)
    wpul = win_pulse(t)
    pts_all = []
    # back faces first (a darker copy, gives the tiles depth when the camera sways)
    for k in range(NT):
        q = tile_quad(k, Rw, bump[k], -S.TILE_DZ, th)
        c = T.COL[own[k]] * 0.32
        fr.poly(cam, q, c)
    for k in range(NT):
        q = tile_quad(k, Rw, bump[k], S.TILE_DZ, th)
        base = T.COL[own[k]]
        fl = max(0.0, 1 - (t - lf[k]) / 0.28) if lf[k] > -1 else 0.0
        ff = flood_flash(t, k)
        scan = 0.92 + 0.08 * math.sin(2 * math.pi * 7 * t / DUR * 9 + k * 0.9)
        c = base * (0.86 * scan + 0.25 * wpul) + (np.array([255, 255, 255.0]) - base) * min(1.0, 0.75 * fl + 0.8 * ff)
        pts = fr.poly(cam, q, np.clip(c, 0, 255))
        if pts is not None:
            sub_mask.polygon(pts, fill=255)
            # bright inner edge line (neon tube)
            e = cam.wp(q[:TILE_SUB])
            fr.draw.line([tuple(p) for p in e], fill=tuple(int(v) for v in np.clip(c * 0.7 + 90, 0, 255)), width=1)
            m = cam.wp(q.mean(0))
            pts_all.append((m, base, fl))
    for m, base, fl in pts_all[::2]:
        fr.glow(m[0], m[1], 7, tuple(base), 0.16 + 0.2 * fl + 0.15 * wpul)


def draw_disc(img, cam, R):
    """darken the city behind the hologram's inner disc (colour only: depth untouched, so it still blurs)"""
    Rw = R * S.RW - S.TILE_TH * 0.6
    if Rw < 0.05:
        return
    ang = np.linspace(0, 2 * math.pi, 49)[:-1]
    P = S.RC[None] + np.stack([np.cos(ang) * Rw, np.sin(ang) * Rw, np.zeros_like(ang)], 1)
    s = cam.wp(P)
    m = Image.new('L', (W, H), 0)
    ImageDraw.Draw(m).polygon([tuple(p) for p in s], fill=255)
    mk = np.asarray(m) > 0
    img[mk] = img[mk] * 0.5 + np.array([10, 8, 26.0])


def draw_rings_grid(fr, cam, R, own):
    """faint radar circles + cross on the hologram disc (crisp: drawn at the ring's depth)"""
    Rw = R * S.RW
    if Rw < 0.08:
        return
    mc = T.COL[own].mean(0) * 0.22 + np.array([20, 30, 50.0])
    for k in (0.33, 0.66):
        ang = np.linspace(0, 2 * math.pi, 41)
        P = S.RC[None] + np.stack([np.cos(ang) * Rw * k, np.sin(ang) * Rw * k, np.zeros_like(ang)], 1)
        fr.line(cam, P, tuple(int(v) for v in mc))
    for a in (0, math.pi / 2):
        P = np.array([S.RC + [math.cos(a) * Rw * 0.9, math.sin(a) * Rw * 0.9, 0], S.RC - [math.cos(a) * Rw * 0.9, math.sin(a) * Rw * 0.9, 0]])
        fr.line(cam, P, tuple(int(v) for v in mc * 0.8))


def beam(fr, cam, t, R):
    """projector beam: faint lines from the lens up to the ring's lower arc"""
    if R < 0.05:
        return
    lens = cam.wp(S.PROJ_LENS)
    Rw = R * S.RW
    for a in np.linspace(math.radians(-150), math.radians(-30), 7):
        p = S.RC + np.array([math.cos(a) * Rw, math.sin(a) * Rw, 0.0])
        s = cam.wp(p)
        fr.draw.line([tuple(lens), tuple(s)], fill=(46, 96, 130), width=1)
    fr.glow(lens[0], lens[1], 9, (120, 240, 255), 0.55)


SPLASH_LIFE = 0.32


def draw_splashes(fr, cam, t, R):
    """every bounce: a paint splash arc as wide as the colour's share, just outside the ring, plus sparks"""
    Rw = R * S.RW
    i0, i1 = np.searchsorted(T.HIT_T, [t - SPLASH_LIFE, t + 1e-9])
    for i in range(i0, i1):
        u = (t - T.HIT_T[i]) / SPLASH_LIFE
        q = int(T.HIT_Q[i])
        a = T.HIT_A[i]
        half = max(0.5 * T.HIT_W[i] * DTH, 0.035)
        rr = Rw + S.TILE_TH + 0.03 + 0.12 * u
        ang = np.linspace(a - half, a + half, max(3, int(half / 0.05) + 2))
        P = S.RC[None] + np.stack([np.cos(ang) * rr, np.sin(ang) * rr, np.full_like(ang, 0.06)], 1)
        s = cam.wp(P)
        col = T.COL[q] * (1 - u) + np.array([255, 255, 255.0]) * 0.35 * (1 - u)
        fr.draw.line([tuple(p) for p in s], fill=tuple(int(v) for v in np.clip(col, 0, 255)), width=3 if u < 0.4 else 2)
        if u < 0.5:
            m = cam.wp(S.RC + np.array([math.cos(a) * rr, math.sin(a) * rr, 0.06]))
            fr.glow(m[0], m[1], 6 + 10 * min(1.0, T.HIT_W[i] / 3), tuple(T.COL[q]), 0.45 * (1 - 2 * u))
        rng = np.random.default_rng(i)
        for j in range(5):
            da = rng.normal(0, 0.35)
            sp = rng.uniform(0.25, 0.6)
            r2 = Rw + S.TILE_TH + sp * u * 1.2
            p = S.RC + np.array([math.cos(a + da * 0.4) * r2, math.sin(a + da * 0.4) * r2, 0.06])
            s2 = cam.wp(p)
            if 0 <= s2[0] < W - 1 and 0 <= s2[1] < H - 1:
                fr.glow(s2[0], s2[1], 1.5, tuple(T.COL[q] * 0.5 + 127), 0.9 * (1 - u))


BALL_FACES = icosa([0, 0, 0], S.BALL_R, jit=0.05, rng=np.random.default_rng(3))


def ball_world(q, t):
    p = T.ball_pos(q, t) * T.ball_scale(t)
    return S.RC + np.array([p[0] * S.RW, p[1] * S.RW, 0.0])


def draw_balls(fr, cam, L, t, sub_mask):
    order = sorted(range(4), key=lambda q: -float(cam.depth(ball_world(q, t))[0]))
    for q in order:
        if T.popped(q, t):
            continue
        launched = T.launched(q, t)
        c = ball_world(q, t)
        col = T.COL[q]
        # trail
        if launched:
            pts = []
            for j in range(0, 9):
                tj = t - j * 0.022
                if tj < T.T_LAUNCH[q]:
                    break
                pts.append(cam.wp(ball_world(q, tj)))
            for j in range(len(pts) - 1, 0, -1):
                a = 1 - j / 9
                cc = tuple(int(v) for v in np.clip(col * (0.25 + 0.6 * a), 0, 255))
                fr.draw.line([tuple(pts[j]), tuple(pts[j - 1])], fill=cc, width=3 if j < 3 else 2)
        # body
        fc = [(cam.depth(Q.mean(0) + c)[0], Q + c, n) for Q, n in BALL_FACES if np.dot(n, cam.pos - (Q.mean(0) + c)) > 0]
        fc.sort(key=lambda a: -a[0])
        if fc:
            C = np.array([Q.mean(0) for _, Q, _ in fc]); Nn = np.array([n for _, _, n in fc])
            cols = shade(L, Nn, C, col, emit=0.75 if launched else 0.45, rimk=0.9)
            for (z, Q, n), cc in zip(fc, cols):
                pts = fr.poly(cam, Q, np.clip(cc, 0, 255))
                if pts is not None:
                    sub_mask.polygon(pts, fill=255)
        s = cam.wp(c)
        rpx = S.BALL_R * cam.f / max(cam.depth(c)[0], 0.5)
        fr.draw.rectangle([s[0] - rpx * 0.45, s[1] - rpx * 0.55, s[0] - rpx * 0.45 + 1, s[1] - rpx * 0.55], fill=(255, 255, 255))
        if launched:
            fr.glow(s[0], s[1], rpx * 2.4, tuple(col), 0.45)
        else:
            pul = 0.5 + 0.5 * math.sin(2 * math.pi * t / T.BEAT * 2)
            fr.glow(s[0], s[1], rpx * 2.0, tuple(col), 0.25 + 0.2 * pul)


SHARDS = {}


def draw_pops(fr, cam, t):
    for tp, q, p in T.POPS:
        u = t - tp
        if not (0 <= u < 1.3):
            continue
        p = np.array(p) * T.ball_scale(tp)
        c0 = S.RC + np.array([p[0] * S.RW, p[1] * S.RW, 0.05])
        if q not in SHARDS:
            r = np.random.default_rng(100 + q)
            SHARDS[q] = [(r.uniform(0, 2 * math.pi), r.uniform(0.5, 1.4), r.uniform(0.02, 0.045), r.uniform(-8, 8), r.uniform(0, 6)) for _ in range(16)]
        fade = max(0.0, 1 - u / 1.3)
        for a, sp, sz, spin, ph in SHARDS[q]:
            d = sp * (1 - math.exp(-u * 2.2)) / 2.2 * 2.2
            cc = c0 + np.array([math.cos(a) * d, math.sin(a) * d - 0.25 * u * u, 0.0])
            rot = ph + spin * u
            tri = np.array([cc + [math.cos(rot + k * 2.1) * sz, math.sin(rot + k * 2.1) * sz, 0] for k in range(3)])
            col = T.COL[q] * (0.4 + 0.6 * fade) + 30 * fade
            fr.poly(cam, tri, np.clip(col, 0, 255))
        s = cam.wp(c0)
        if u < 0.4:
            rr = (0.05 + 0.9 * u) * cam.f / cam.depth(c0)[0]
            k = 1 - u / 0.4
            n = 24
            pts = [(s[0] + rr * math.cos(2 * math.pi * j / n), s[1] + rr * math.sin(2 * math.pi * j / n)) for j in range(n + 1)]
            fr.draw.line(pts, fill=tuple(int(v) for v in np.clip(T.COL[q] * k + 60 * k, 0, 255)), width=2)
            fr.glow(s[0], s[1], 14, tuple(T.COL[q]), 0.7 * k)


def draw_colls(fr, cam, t):
    i0, i1 = np.searchsorted(T.COLL_T, [t - 0.2, t + 1e-9])
    for i in range(i0, i1):
        u = (t - T.COLL_T[i]) / 0.2
        p = T.COLL_P[i] * T.ball_scale(T.COLL_T[i])
        c0 = S.RC + np.array([p[0] * S.RW, p[1] * S.RW, 0.08])
        r = np.random.default_rng(5000 + i)
        for j in range(6):
            a = r.uniform(0, 2 * math.pi)
            d = 0.04 + 0.25 * u * r.uniform(0.5, 1.0)
            s = cam.wp(c0 + np.array([math.cos(a) * d, math.sin(a) * d, 0]))
            q = T.COLL_IJ[i][j % 2]
            fr.glow(s[0], s[1], 1.6, tuple(T.COL[q] * 0.5 + 127), 0.9 * (1 - u))


def fx_props(fr, cam, t):
    # blinking mast light (whole cycles), door lamp glow, steam from the vent
    on = ((40 * t / DUR) % 1.0) < 0.45
    s = cam.wp(S.MAST_TOP)
    fr.glow(s[0], s[1], 4, (255, 50, 50), 0.9 if on else 0.15)
    s = cam.wp(S.DOOR_LAMP)
    fr.glow(s[0], s[1], 12, (255, 180, 100), 0.5)
    for j in range(9):
        u = (7 * t / DUR + j / 9) % 1.0
        p = S.VENT_TOP + np.array([0.1 * math.sin(u * 5 + j) + 0.5 * u * u, 1.3 * u, 0.0])
        s = cam.wp(p)
        r = (0.06 + 0.22 * u) * cam.f / cam.depth(p)[0]
        fr.glow(s[0], s[1], r, (120, 110, 150), 0.2 * (1 - u))


AC_FAN = []
for (x, z, rot) in S.AC:
    AC_FAN.append(np.array([x, 0.885, z]))


def fans(fr, cam, t):
    for k, c in enumerate(AC_FAN):
        a0 = 2 * math.pi * (45 + 7 * k) * t / DUR
        for b in range(3):
            a = a0 + b * 2 * math.pi / 3
            p1 = c + np.array([math.cos(a) * 0.26, 0.0, math.sin(a) * 0.26])
            fr.line(cam, [c, p1], (52, 54, 70), width=2)


# ================================================================== rain (screen space, loop-safe)
_rr = np.random.default_rng(77)
NRAIN = 240
RAIN = [(_rr.uniform(0, W), _rr.uniform(0, H + 30), int(_rr.integers(52, 80)), _rr.uniform(0.25, 1.0)) for _ in range(NRAIN)]


def rain(img, t):
    for x0, y0, n, b in RAIN:
        span = H + 30
        y = (y0 + n * span * t / DUR) % span - 15
        x = (x0 - 0.22 * (y + 15)) % W
        ln = 4 + 6 * b
        a = 0.12 + 0.22 * b
        for j in range(int(ln)):
            yy, xx = int(y - j), int(x + 0.22 * j)
            if 0 <= yy < H and 0 <= xx < W:
                img[yy, xx] = img[yy, xx] * (1 - a) + np.array([170, 190, 230.0]) * a


# ================================================================== focus
def focus_band(cam, R):
    zf = cam.depth(S.RC)[0]
    ang = np.linspace(0, 2 * math.pi, 24, endpoint=False)
    Rw = max(R, 0.05) * S.RW + S.TILE_TH + 0.2
    P = S.RC[None] + np.stack([np.cos(ang) * Rw, np.sin(ang) * Rw, np.zeros_like(ang)], 1)
    P = np.concatenate([P, P + [0, 0, 0.12], P - [0, 0, 0.12], [S.PROJ_LENS]])
    z = cam.depth(P)
    return max(0.05, float(np.abs(1 - zf / z).max()) + 0.01)


# ================================================================== frame
def render(f):
    t = f / FPS
    cam = camera(t)
    R = T.ring_vis(t)
    own, lf = ring_state(t)
    rl = ring_lights(t, own, max(R, 0.0))
    L = lights(t, cam, rl)
    fr = Frame(sky(cam, t))
    ms = draw_moon(fr, cam)
    city(fr, cam, L, t)
    balls = []
    for q in range(4):
        p = T.ball_pos(q, t) * T.ball_scale(t)
        balls.append((p[0], p[1], not T.popped(q, t)))
    roof(fr, cam, L, t, own, R, balls, rl, f)
    zr = float(cam.depth(S.RC)[0])
    items = [(float(cam.depth(S.STATIC[i].ctr)[0]), 'obj', i) for i in range(len(S.STATIC))]
    items.append((zr + 0.4, 'ring', None))
    items.append((float(cam.depth(S.SIGN_C)[0]) - 0.05, 'sign', None))
    sub = Image.new('L', (W, H), 0)
    sub_d = ImageDraw.Draw(sub)
    for _, kind, it in painter(items):
        if kind == 'obj':
            draw_static(fr, cam, L, [it])
        elif kind == 'sign':
            draw_sign(fr, cam, t)
        else:
            img = fr.array()
            draw_disc(img, cam, R)
            fr.set_array(img, fr.depth())
            beam(fr, cam, t, R)
            draw_rings_grid(fr, cam, R, own)
            draw_ring(fr, cam, L, t, own, lf, R, sub_d)
            draw_splashes(fr, cam, t, R)
            draw_balls(fr, cam, L, t, sub_d)
            draw_pops(fr, cam, t)
            draw_colls(fr, cam, t)
    for P in S.CABLES:
        fr.line(cam, P, (40, 38, 52))
    fans(fr, cam, t)
    fx_props(fr, cam, t)
    if T.T_COLLAPSE < t:
        u = (t - T.T_COLLAPSE) / (DUR - T.T_COLLAPSE)
        s = cam.wp(S.RC)
        c = (T.COL[T.WINNER] * 0.6 + 100) * (1 - u ** 2) + LILAC * u ** 2
        fr.glow(s[0], s[1], 6 + 20 * u, tuple(c), 0.25 + 0.6 * u ** 2)
    if t < 0.35:
        s = cam.wp(S.RC)
        fr.glow(s[0], s[1], 20 * (1 - t / 0.35) + 6, tuple(LILAC), 0.85 * (1 - t / 0.35))

    # ---- lens: focus locked on the ring
    img, depth = fr.array(), fr.depth()
    zf = cam.depth(S.RC)[0]
    img = dof(img, coc_map(depth, zf, 3.8, sharp=focus_band(cam, R)))
    for g in fr.glows:
        add_glow(img, *g)
    img = grade(bloom(np.clip(img, 0, 255), thr=196, k=0.38), contrast=0.12, sat=1.15)
    rain(img, t)
    img = np.clip(img * vignette(strength=0.6, cap=0.3) * top_scrim(amount=0.4), 0, 255).astype(np.uint8)
    out = img.copy()
    _ui(out, t)
    return out


# ================================================================== UI
BAR_Y = 368
_SC, _SLOTS = ui.share_layout(T.NAMES, SAFE[0] + 4, SAFE[2] - 4, 2)
NAME_Y = BAR_Y + 6 + 15
INSTR_Y = NAME_Y + 7 * _SC + 2


def _label_anchor(q):
    """where each opening label sits: beside its ball's launch spot, inside the ring's quadrant"""
    ax = [52, 52, -52, -52][q]
    ay = [-34, 34, 34, -34][q]
    return 168 + ax, 255 + ay


def _labels(out, t):
    for q in range(4):
        if t < T.T_LAUNCH[q] or t >= T.BAR_IN + 0.4:
            continue
        nm, ins = T.NAMES[q], T.INSTR[q]
        wn = len(nm) * 12 - 2
        wi = len('· ' + ins) * 6 - 1
        cx, cy = _label_anchor(q)
        xn, yn = cx - wn / 2, cy - 9
        xi, yi = cx - wi / 2, cy + 7
        u = T.ease((t - T.LABEL_FLY0) / (T.LABEL_FLY1 - T.LABEL_FLY0))
        sx0, sx1, scx = _SLOTS[q]
        xn = xn * (1 - u) + sx0 * u
        yn = yn * (1 - u) + NAME_Y * u
        ins_s = '· ' + ins if u < 0.5 else ins
        wi = len(ins_s) * 6 - 1
        xi = (cx - wi / 2) * (1 - u) + (scx - wi / 2) * u
        yi = yi * (1 - u) + INSTR_Y * u
        dt = t - T.T_LAUNCH[q]
        drop = int(round(-4 * max(0.0, 1 - dt / 0.1)))
        col = T.TEXT[q] if dt > 0.08 else (255, 255, 255)
        ui.label(out, nm, int(xn), int(yn) + drop, col, 2)
        ui.label(out, ins_s, int(xi), int(yi) + drop, (230, 226, 240), 1)


def _bar(out, t):
    a = T.bar_alpha(t)
    if a <= 0.01:
        return
    under = out.copy() if a < 0.999 else None
    x0, x1 = SAFE[0] + 4, SAFE[2] - 4
    bx0, by0, bx1, by1 = x0 - 4, BAR_Y - 4, x1 + 4, INSTR_Y + 9
    reg = out[by0:by1, bx0:bx1].astype(np.float32)
    out[by0:by1, bx0:bx1] = (reg * 0.45 + np.array(ui.INK, float) * 0.25).astype(np.uint8)
    edge = np.array([62, 74, 112], float)
    out[by0, bx0:bx1] = (out[by0, bx0:bx1] * 0.4 + edge * 0.6).astype(np.uint8)
    out[by1 - 1, bx0:bx1] = (out[by1 - 1, bx0:bx1] * 0.6 + edge * 0.4).astype(np.uint8)
    ui._note(bx0, by0, bx1, by1, 'panel', 'share bar')
    outs = [T.popped(q, t) for q in range(4)]
    res = ui.share_bar(out, T.shares(t), T.NAMES, T.BAR_COL, y=BAR_Y, out=outs, backing=False, show_pct=False)
    for q in range(4):
        sx0, sx1, scx = res['slots'][q]
        s = 'OUT' if outs[q] else T.INSTR[q]
        w = len(s) * 6 - 1
        col = (130, 126, 150) if outs[q] else (226, 222, 238)
        ui.label(out, s, int(round(scx - w / 2)), INSTR_Y, col, 1)
    if under is not None:
        out[:] = (under * (1 - a) + out * a).astype(np.uint8)


def _ui(out, t):
    _labels(out, t)
    _bar(out, t)
    for tt, t0, t1, lines, hi, alt in T.captions_at(t):
        ui.draw_captions(out, tt, [(t0, t1, lines)], hi=hi, alt=alt if alt else ui.SKYB, style='impact', puff=(255, 120, 230))
