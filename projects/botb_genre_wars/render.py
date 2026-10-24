"""Battle of the Bands: Genre Wars renderer (adapted from projects/battle_of_the_bands/render.py).

void (gradient, nebula, stars, moon) -> searchlight beams behind the stage -> per-pixel stage boards
(moon shadows of the rig, LED-wall glow, moving-head pools and the stage mood of the leading genre, footlights,
glossy reflection of the wall, contact AO) -> painter list: rig, stacks, props, the LED wall (per pixel, from
the cached battle), crowd -> locked DOF on the wall -> genre FX (ROCK pyro, EDM lasers, JAZZ smoky spots,
ORCHESTRA gold glow), glows, sparks -> god rays, bloom, grade, scrim -> share bar (genre + instrument), labels,
title tag, impact captions.

The stage mood follows the leader (timeline.mood): when the lead changes, the song changes genre and so do the
lights.

    python sim.py && python audio.py
    python -m reelkit.sheet render.py test/sheet.png 1 6 12 19 21 27 34 44 54 60.6
    python -m reelkit.video render.py audio.wav botb_genre_wars.mp4"""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, SAFE, dirv, whole_period
from reelkit.camera import Cam
from reelkit.raster import Frame, painter, add_glow
from reelkit.lighting import Lights, shade
from reelkit.shadows import MaskGrid, directional_mask, Occluders, contact_ao
from reelkit.lens import coc_map, dof
from reelkit.post import god_rays, bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, panel, label, slide_offset, share_bar, CREAM, INK
from reelkit import ui
from reelkit import font
from reelkit.geometry import Obj, box, prism, icosa, face, lathe
import timeline as T
import scene as S
import music as MU

# headphones glyph for "LISTEN @"
font.GLYPHS['@'] = np.array([[c == '#' for c in r] for r in ".###.|#...#|#...#|##.##|##.##|##.##|.....".split('|')])
font.GLYPHS['·'] = np.array([[c == '#' for c in r] for r in ".....|.....|.....|..#..|.....|.....|.....".split('|')])

D = T.D
OWN = D['owner']
CLAIM = D['claim_t'].astype(np.float32)
STATE = D['state']
FIELDS = D['ring_fields']
NOTES = MU.notes()
N = S.N
TEX = 4                                   # LED texture texels per tile
TCOL = np.array(S.TEAM_COL)
EMPTY_COL = np.array([24, 26, 46.0])
_trng = np.random.default_rng(5)
TILE_TONE = _trng.uniform(0.9, 1.06, (N, N))
SAND_SPECK = _trng.uniform(0.82, 1.12, (N, N))
ARENA = OWN[0] >= 0

# ================================================================== camera
TGT = np.array([0.22, 3.1, 0.0])


def camera(t):
    tt = t % T.DUR
    yaw = 1.8 * math.sin(2 * math.pi * tt / whole_period(30.0, T.DUR))
    pitch = 10.0 + 0.6 * math.sin(2 * math.pi * tt / whole_period(20.0, T.DUR) + 1.0)
    dist = 12.0 + 0.15 * math.sin(2 * math.pi * tt / whole_period(15.0, T.DUR))
    # punch-ins on the knockouts and the win, a slow push through the solo
    for te, _ in T.OUTS:
        u = tt - te
        if 0 <= u < 1.6:
            dist -= 0.35 * math.sin(math.pi * min(1, u / 0.25)) * math.exp(-u / 0.6)
    if T.T_WIN is not None:
        u = tt - T.T_WIN
        if 0 <= u < T.T_CRACK - T.T_WIN:
            a = T.ease(u / 4.0) * (1 - T.ease((tt - (T.T_CRACK - 1.6)) / 1.6))     # back to the base framing by the crack
            dist -= 0.35 * a
    return Cam(TGT, dist, yaw, pitch, 52.0, 262.0)


# ================================================================== static objects, shadows
STATIC = list(S.OBJS)
OCC = Occluders([o for o in STATIC if o.cast and o.name not in ('slab', 'slab_under', 'slab_root')])
STAT_FACES = []
for o in STATIC:
    C = np.array([Q.mean(0) for Q, n in o.faces])
    Nn = np.array([n for Q, n in o.faces])
    kv = OCC.vis(C + Nn * 0.02, o.grp, d=S.KEY)
    kv = np.where(Nn @ S.KEY > 0, kv, 1.0)
    STAT_FACES.append((C, Nn, kv))
FLOOR = MaskGrid((-8.0, 8.0, -6.0, 10.5, 0.02))
_cast = [Q for o in STATIC if o.cast and o.name not in ('slab', 'slab_under', 'slab_root', 'step', 'foot', 'barrier') for Q, n in o.faces]
_cast += [Q for st in S.STACKS for fs, c, R in st['cabinets'] for Q, n in fs]
MOON_MASK = directional_mask(FLOOR, _cast, S.KEY, blur=1.5)


def octagon_contains(X, Z):
    r = S.STAGE_R * math.cos(math.pi / 8)
    x, z = X - S.STAGE_C[0], Z - S.STAGE_C[2]
    ok = np.ones(np.shape(X), bool)
    for k in range(8):
        a = math.pi / 8 + k * math.pi / 4 + math.pi / 8
        ok &= x * math.cos(a) + z * math.sin(a) <= r
    return ok


# ================================================================== the void
_vr = np.random.default_rng(31)


def _noise(h, w, octaves, rng):
    out = np.zeros((h, w))
    for k, amp in octaves:
        g = rng.normal(0, 1, (k, 2 * k))
        g = np.concatenate([g, g[:, :1]], 1)
        im = Image.fromarray(((g - g.min()) / (np.ptp(g) + 1e-9) * 255).astype(np.uint8)).resize((w + w // (2 * k), h), Image.BICUBIC)
        out += (np.asarray(im, float)[:, :w] / 255 - 0.5) * amp
    return out


NEB = np.clip(_noise(120, 240, [(4, 1.0), (9, 0.6), (20, 0.3)], _vr) * 1.8 + 0.05, 0, 1)
NEB2 = np.clip(_noise(120, 240, [(5, 1.0), (12, 0.5)], _vr) * 1.8 - 0.1, 0, 1)
STARS = _vr.normal(0, 1, (700, 3))
STARS /= np.linalg.norm(STARS, axis=1, keepdims=True)
STARS[:, 2] = -np.abs(STARS[:, 2])
STAR_B = _vr.uniform(0.25, 1.0, 700) ** 2
STAR_PH = _vr.uniform(0, 2 * math.pi, 700)


def void(cam, t):
    d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1) @ cam.M
    d /= np.linalg.norm(d, axis=-1, keepdims=True)
    el = np.arcsin(np.clip(d[..., 1], -1, 1))
    az = np.arctan2(d[..., 0], -d[..., 2])
    top, mid, low = np.array([22, 18, 70.0]), np.array([92, 34, 132.0]), np.array([196, 70, 150.0])
    k = np.clip((el + 0.05) / 0.55, 0, 1)
    col = np.where(k[..., None] < 0.5, low * (1 - 2 * k[..., None]) + mid * 2 * k[..., None],
                   mid * (2 - 2 * k[..., None]) + top * (2 * k[..., None] - 1))
    u = ((az / (2 * math.pi) + 0.5) * 240).astype(int) % 240          # static nebula (the loop must be seamless)
    v = np.clip(((0.6 - el) / 1.2 * 120).astype(int), 0, 119)
    n1 = NEB[v, u][..., None]
    n2 = NEB2[v, (u + 60) % 240][..., None]
    col = col + n1 * np.array([60, 30, 90.0]) * 0.8 + n2 * np.array([20, 70, 110.0]) * 0.6
    # moon glow
    cg = (d * S.MOON_DRAW[None, None]).sum(-1)
    col += (np.clip((cg - 0.9) / 0.1, 0, 1) ** 3)[..., None] * np.array([120, 120, 170.0]) * 0.6
    col = np.floor(col / 7 + BAYER_FULL[..., None]) * 7
    return np.clip(col, 0, 255)


def stars(img, cam, t):
    Pc = (STARS * 1e4) @ cam.M.T
    ok = Pc[:, 2] > 1
    s = cam.proj(Pc[ok])
    b = STAR_B[ok] * (0.75 + 0.25 * np.sin(2 * math.pi * 19 * t / T.DUR + STAR_PH[ok]))
    for (x, y), bb in zip(s, b):
        xi, yi = int(x), int(y)
        if 0 <= xi < W and 0 <= yi < H:
            img[yi, xi] = np.maximum(img[yi, xi], 120 + 135 * bb)


def searchlights(img, cam, t):
    """slow sweeping beams from behind the stage into the void (drawn before the rig, so it hides their roots)"""
    over = np.zeros((H, W), np.float32)
    cols = np.zeros((H, W, 3), np.float32)
    mw = T.mood(t)
    for p0, ph, col in S.SEARCH:
        col = np.array(col, float) * (1 - 0.6 * mw[0]) + np.array([255, 200, 110.0]) * 0.6 * mw[0]
        a = 2 * math.pi * (t / whole_period(12.0, T.DUR) + ph)
        dvec = np.array([0.55 * math.sin(a), 1.0, -0.25 + 0.2 * math.cos(a * 0.6)])
        dvec /= np.linalg.norm(dvec)
        s0, s1 = cam.wp(p0), cam.wp(p0 + dvec * 40)
        if cam.depth(p0)[0] < 0.5:
            continue
        L = np.hypot(*(s1 - s0)) + 1e-6
        ux, uy = (s1 - s0) / L
        px, py = XX - s0[0], YY - s0[1]
        along = px * ux + py * uy
        perp = np.abs(-px * uy + py * ux)
        width = 2.0 + along * 0.07
        m = np.clip(1 - perp / np.maximum(width, 1), 0, 1) * np.clip(along / 30, 0, 1) * np.exp(-np.clip(along, 0, None) / 260)
        m = m * (along > 0)
        over += m
        cols += m[..., None] * np.array(col, np.float32)
    k = 0.22
    img += np.floor((cols * k) / 4 + BAYER_FULL[..., None]) * 4


# ================================================================== the LED wall
def sim_index(t):
    return T.sim_frame(int(round(t * FPS)) % T.NF)


_TEXC = {}


def led_texture(t, f):
    """the wall as a (N*TEX, N*TEX, 3) float texture at video time t (memoised per frame)"""
    key = round(t * FPS, 3)
    if key not in _TEXC:
        _TEXC.clear()
        _TEXC[key] = _led_texture(t)
    return _TEXC[key]


def _led_texture(t):
    sf = sim_index(t)
    ts = T.sim_time(t)
    own = OWN[sf].astype(int)
    col = np.where(own[..., None] > 0, TCOL[np.clip(own - 1, 0, 3)], EMPTY_COL)
    col = col * TILE_TONE[..., None]
    # sand speckle, vine youth (fresh growth is lighter), claim flash
    age = ts - CLAIM[sf]
    m = own == 1
    col[m] *= SAND_SPECK[m][:, None]
    vy = (own == 4) & (age < 1.6)
    col[vy] = col[vy] * (0.7 + 0.3 * (age[vy] / 1.6))[:, None] + np.array([190, 210, 255.0]) * (0.3 * (1 - age[vy] / 1.6))[:, None]
    fl = (own > 0) & (age < 0.35) & (age >= 0)
    col[fl] += (np.array([255, 255, 240.0]) - col[fl]) * (0.55 * np.exp(-age[fl] / 0.09))[:, None]
    # embers: tiles that just went dark (a knocked-out band) fade from their old colour
    em = (own == 0) & ARENA & (age < 0.9) & (age >= 0) & (ts > T.T_BELL + T.OFFSET)
    if em.any():
        ys, xs = np.nonzero(em)
        for y, x in zip(ys, xs):
            k = int(round(age[y, x] * FPS)) + 1
            prev = OWN[max(0, sf - k), y, x]
            if prev > 0:
                a = 1 - age[y, x] / 0.9
                col[y, x] = EMPTY_COL + (TCOL[prev - 1] * np.array([1.0, 0.55, 0.45]) - EMPTY_COL) * a * a
    # ripple rings travelling through its land
    st = STATE[sf]
    rs = st[2].get('rings', [])
    if rs:
        band = np.zeros((N, N))
        for rid, r in rs:
            dfield = FIELDS[rid]
            band = np.maximum(band, np.exp(-((dfield - r) / 1.1) ** 2))
        band *= (own == 3)
        col += band[..., None] * np.array([170, 255, 252.0]) * 0.55
    # the winner's solo: radial pulses on every beat
    if T.T_WIN is not None and T.T_WIN <= t < T.T_CRACK:
        yy, xx = np.mgrid[0:N, 0:N]
        rr = np.hypot(yy - N / 2 + 0.5, xx - N / 2 + 0.5) / (N / 2)
        ph = T.beat_phase(t)
        wave = np.exp(-((rr - ph * 1.3) / 0.12) ** 2) * (1 - ph)
        on = min(1.0, (t - T.T_WIN) / 0.6)
        col = col * (1 + 0.35 * on * wave[..., None]) + on * wave[..., None] * 40
    col[~ARENA] = 0
    tex = np.repeat(np.repeat(col, TEX, 0), TEX, 1)
    # sub-tile features
    def dot(y, x, c, r=0.0, a=1.0):
        yi, xi = int(y * TEX), int(x * TEX)
        R = int(math.ceil(r * TEX))
        for dy in range(-R, R + 1):
            for dx in range(-R, R + 1):
                if dy * dy + dx * dx <= (r * TEX) ** 2 + 0.5:
                    yy_, xx_ = yi + dy, xi + dx
                    if 0 <= yy_ < N * TEX and 0 <= xx_ < N * TEX:
                        tex[yy_, xx_] = tex[yy_, xx_] * (1 - a) + np.array(c) * a
    for y, x in st[0].get('grains', []):
        dot(y, x, (255, 250, 220), 0.25)
    for y, x in st[0].get('landed', []):
        dot(y + 0.5, x + 0.5, (255, 255, 255), 0.6, 0.6)
    for i, j in st[3].get('tips', []):
        dot(i + 0.5, j + 0.5, (200, 215, 255), 0.45)
    # ball + trail
    b = st[1].get('ball')
    if b is not None:
        for k in range(6, 0, -1):
            pb = STATE[max(0, sf - k)][1].get('ball')
            if pb is not None and sf - k >= 0:
                dot(pb[0], pb[1], (255, 200, 170), 0.7, 0.25 * (1 - k / 7))
        dot(b[0], b[1], (255, 248, 240), 1.05)
        dot(b[0], b[1], (255, 120, 90), 0.6, 0.6)
    for i, j in st[1].get('hits', []):
        dot(i + 0.5, j + 0.5, (255, 255, 255), 0.8, 0.8)
    # note sparkles: every note played lights the tile its move claimed (a small ring)
    a0, a1 = np.searchsorted(NOTES['t'], [t - 0.28, t + 1e-6])
    for n in NOTES[a0:a1]:
        if n['i'] < 0 or n['kind'] in QUIET_KINDS:
            continue
        u = (t - n['t']) / 0.28
        r = 0.6 + 1.8 * u
        c = np.minimum(255, TCOL[n['team']] * 0.6 + 140)
        aa = (1 - u) * 0.9
        yc, xc = (n['i'] + 0.5) * TEX, (n['j'] + 0.5) * TEX
        for k in range(16):
            ang = 2 * math.pi * k / 16
            yy_, xx_ = int(yc + r * TEX * math.sin(ang)), int(xc + r * TEX * math.cos(ang))
            if 0 <= yy_ < N * TEX and 0 <= xx_ < N * TEX:
                tex[yy_, xx_] = tex[yy_, xx_] * (1 - aa) + c * aa
    return tex


QUIET_KINDS = {'rhat', 'ehat', 'eohat', 'phat', 'ride', 'brush', 'troll', 'cello', 'pmute'}


# the crack at 60.4 s: jagged lines from the centre, then every tile drops off the wall
_cr = np.random.default_rng(58)
CRACKS = []
for k in range(9):
    a = 2 * math.pi * k / 9 + _cr.uniform(-0.25, 0.25)
    pts = [(0.0, 0.0)]
    r = 0.0
    while r < 1.05:
        r += _cr.uniform(0.08, 0.16)
        a += _cr.uniform(-0.35, 0.35)
        pts.append((r * math.cos(a), r * math.sin(a)))
    CRACKS.append(pts)
DROP_T = T.T_CRACK + 0.1 + _cr.uniform(0, 0.28, (N, N))      # every tile gone by ~60.96 s: the wall is empty at 61


def led_wall(fr, cam, t, f):
    """per-pixel pass: intersect pixel rays with the wall plane z = 0, sample the LED texture"""
    corners = [S.WC + [sx * S.RW, sy * S.RW, 0] for sx in (-1, 1) for sy in (-1, 1)]
    sc = np.array([cam.wp(c) for c in corners])
    x0, x1 = int(max(0, sc[:, 0].min() - 2)), int(min(W, sc[:, 0].max() + 3))
    y0, y1 = int(max(0, sc[:, 1].min() - 2)), int(min(H, sc[:, 1].max() + 3))
    ys, xs = np.mgrid[y0:y1, x0:x1]
    ys, xs = ys.ravel().astype(float), xs.ravel().astype(float)
    dw = cam.pixel_rays(xs, ys)
    tt = (0.0 - cam.pos[2]) / dw[:, 2]
    P = cam.pos[None] + tt[:, None] * dw
    u = (P[:, 0] - S.WC[0]) / S.RW
    v = (P[:, 1] - S.WC[1]) / S.RW
    inside = (u * u + v * v <= 1.0) & (tt > 0)
    xs, ys, u, v, tt = xs[inside], ys[inside], u[inside], v[inside], tt[inside]
    arr, depth = fr.array(), fr.depth()
    yi, xi = ys.astype(int), xs.astype(int)
    front = tt < depth[yi, xi]
    xs, ys, u, v, tt, yi, xi = xs[front], ys[front], u[front], v[front], tt[front], yi[front], xi[front]
    fj = (u + 1) / 2 * N
    fi = (1 - v) / 2 * N
    if t >= T.T_CRACK and t < T.T_RESTART:
        tex = led_texture(T.T_CRACK - 0.01, f)
        col = _crack_colors(t, fi, fj, tex)
    else:
        tex = led_texture(t, f)
        ti = np.clip((fi * TEX).astype(int), 0, N * TEX - 1)
        tj = np.clip((fj * TEX).astype(int), 0, N * TEX - 1)
        col = tex[ti, tj]
    # faint scanline shimmer so the wall reads as an LED screen
    col = col * (0.94 + 0.06 * ((yi + int(t * 12)) % 3 == 0))[:, None]
    arr[yi, xi] = col
    depth[yi, xi] = tt
    fr.set_array(arr, depth)
    return tex


def _crack_colors(t, fi, fj, tex):
    """T_CRACK + 0-0.6 s: cracks race out from the centre, then each tile falls off (dark behind)"""
    out = np.tile(EMPTY_COL, (len(fi), 1)).astype(np.float32)
    ti, tj = fi.astype(int), fj.astype(int)
    ok = (ti >= 0) & (ti < N) & (tj >= 0) & (tj < N)
    # tiles still on the wall, and tiles falling: a tile at (i, j) falling for s seconds sits at i + 0.5 * g * s^2
    g = 320.0
    # inverse lookup: for each pixel row fi, which source tile row is drawn here? search falling offsets
    best = np.full(len(fi), -1.0)
    for src_off in range(0, 40):
        si = ti - src_off
        okk = ok & (si >= 0)
        s_i = np.clip(si, 0, N - 1)
        tj_ = np.clip(tj, 0, N - 1)
        s = t - DROP_T[s_i, tj_]
        fall = np.where(s > 0, 0.5 * g * s * s, 0.0)
        hit = okk & (np.abs(fall - src_off) < 0.5 + 0.02 * src_off) & (best < 0)
        if hit.any():
            texi = np.clip(((fi[hit] - fall[hit]) * TEX).astype(int), 0, N * TEX - 1)
            texj = np.clip((fj[hit] * TEX).astype(int), 0, N * TEX - 1)
            fade = np.clip(1 - np.where(s[hit] > 0, s[hit], 0) / 0.18, 0, 1)
            out[hit] = EMPTY_COL + (tex[texi, texj] - EMPTY_COL) * fade[:, None]
            best[hit] = 1
    # crack lines (bright) during the first 0.25 s
    u = (t - T.T_CRACK) / 0.25
    if 0 <= u < 1.6:
        cu = (fj / N * 2 - 1)
        cv = (1 - fi / N * 2)
        rr = np.hypot(cu, cv)
        lit = np.zeros(len(fi), bool)
        for pts in CRACKS:
            for (ax, ay), (bx, by) in zip(pts, pts[1:]):
                if math.hypot(ax, ay) > min(1.0, u) * 1.1:
                    break
                dx, dy = bx - ax, by - ay
                L2 = dx * dx + dy * dy + 1e-9
                w_ = np.clip(((cu - ax) * dx + (cv - ay) * dy) / L2, 0, 1)
                dd = np.hypot(cu - ax - w_ * dx, cv - ay - w_ * dy)
                lit |= dd < 0.018
        out[lit] = np.array([255, 255, 255.0]) * max(0.0, 1 - max(0.0, u - 1.0) / 0.6)
    return out


def crack_gone(t):
    """0..1: share of the wall that has fallen off during the crack (0 outside it)"""
    if not (T.T_CRACK <= t < T.T_RESTART):
        return 0.0
    return float(np.mean(t - DROP_T > 0.09))


def wall_mean_color(tex):
    m = tex.reshape(-1, 3)
    return m.mean(0)


# ================================================================== dynamic objects
def pump(q, t):
    """speaker cone kick from team q's notes"""
    a0, a1 = np.searchsorted(NOTES['t'], [t - 0.2, t + 1e-6])
    v = 0.0
    for n in NOTES[a0:a1]:
        if n['team'] == q:
            v = max(v, n['vel'] * math.exp(-(t - n['t']) / 0.07))
    return v


LEADS = T.LEADS[1:]                      # (t, genre) of every lead change
WASH = [np.array(c) for c in ((1.0, 0.76, 0.38), (1.0, 0.28, 0.22), (0.25, 0.9, 1.0), (0.32, 0.42, 1.0))]


def mood_col(t):
    """the stage mood colour (0..1 rgb): the leading genre's wash, eased at each lead change"""
    w = T.mood(t)
    return sum(w[q] * WASH[q] for q in range(4))


def lead_flare(q, t):
    """0..1 flare when team q has just taken the lead"""
    v = 0.0
    for tl, m in LEADS:
        if m == q and 0 <= t - tl < 1.2:
            v = max(v, math.exp(-(t - tl) / 0.35))
    return v


def stack_light(q, t):
    """0..1 brightness of team q's stack LEDs (dark once knocked out)"""
    if q in T.OUT_T and T.T_REVIVE <= t < T.T_REVIVE + 0.25:
        return 0.6 * (0.5 + 0.5 * math.sin((t - T.T_REVIVE) * 70))
    if not T.alive(q, t):
        te = T.OUT_T[q]
        return max(0.0, 1 - (t - te) / 0.35) * (0.5 + 0.5 * math.sin((t - te) * 60))
    g = MU.gains(t)[:, 1]
    return min(1.0, 0.35 + 0.65 * g[q] / max(0.3, g.max()) + lead_flare(q, t))


def crowd_objs(t, cam):
    objs = []
    be = T.beat_env(t, 0.18)
    win = T.T_WIN is not None and T.T_WIN <= t < T.T_CRACK + 0.3
    lights = []
    for k, (x, z, h, ph, fan, phone, sc) in enumerate(S.CROWD):
        bob = 0.06 * abs(math.sin(math.pi * ((t / T.BEAT) + ph * 0.3)))
        y0 = S.PIT_Y + bob
        body = prism(x, z, 0.19 * sc, y0, y0 + h * 0.8, 6, r1=0.16 * sc, rot=ph)
        head = icosa([x, y0 + h * 0.8 + 0.13, z], 0.13 * sc, rng=np.random.default_rng(k))
        col = S.CROWD_COLS[k % len(S.CROWD_COLS)]
        objs.append(Obj(body + head, col, 'person', cast=False, jitter=0.03, rng=np.random.default_rng(k)))
        cheer = win or k % 3 != 2
        up = 0.65 + 0.35 * math.sin(2 * math.pi * (t / (2 * T.BEAT) + ph)) if cheer else 0.25 + 0.08 * math.sin(2 * math.pi * 29 * t / T.DUR + ph * 6)
        for side in (-1, 1):
            if side == 1 and not (win or k % 2 == 0):
                continue
            sh = np.array([x + side * 0.17 * sc, y0 + h * 0.76, z])
            spread = 0.1 + 0.12 * (1 - up)
            hand = sh + np.array([side * spread, 0.62 * (0.35 + 0.65 * up), 0.02])
            mid = (sh + hand) / 2
            objs.append(Obj(box(mid[0], mid[1], mid[2], 0.05, np.linalg.norm(hand - sh) / 2, 0.05), col * 0.92, 'arm', cast=False))
            if phone and side == 1:
                team = T.WINNER if win else fan
                if not T.alive(team, t) and not win:
                    team = -1
                lights.append((hand + [0, 0.06, 0], team))
    return objs, lights


def hopper_x(t):
    """the hopper's rail position from the run; during the crack (no sand falls) it slides back to where the next
    run starts, so the loop seam doesn't jump"""
    st = STATE[sim_index(t)][0]
    x = S.col_to_x(st.get('hopper', S.N / 2 - 7))
    if T.T_CRACK <= t < T.T_RESTART:
        x0 = S.col_to_x(STATE[sim_index(T.T_CRACK)][0].get('hopper', S.N / 2 - 7))
        x1 = S.col_to_x(STATE[0][0].get('hopper', S.N / 2 - 7))
        x = x0 + (x1 - x0) * T.ease((t - T.T_CRACK) / (T.T_RESTART - T.T_CRACK - 0.08))
    return x


# ================================================================== lights
def lights(t, cam, wall_col):
    L = Lights(amb=(0.25, 0.22, 0.36), sky=(0.08, 0.07, 0.14))
    L.view = cam.view
    L.key('moon', S.KEY, S.KEY_COL)
    L.fill(dirv(175, 28), (0.16, 0.13, 0.2))
    wc = np.clip(wall_col / 255.0, 0, 1) * 1.6
    L.point(S.WC + [0, -0.6, 1.4], wc, 3.2)
    L.point(np.array([0, 0.3, S.FOOT_Z + 0.2]), np.array([1.0, 0.62, 0.32]) * 0.7, 1.8)
    L.point(np.array([0, 4.6, 3.2]), mood_col(t) * 0.75, 4.5)          # the leading genre's stage wash
    for q, st in enumerate(S.STACKS):
        b = stack_light(q, t)
        c = st['leds'][0][0]
        L.point(c + [0, 0, 0.4], S.TEAM_COL[q] / 255.0 * 0.5 * b, 1.0)
    return L


HEAD_PATHS = [(7, 0.0, -2.2), (10, 0.25, -0.6), (8, 0.5, 0.8), (6, 0.75, 2.3)]     # whole cycles per 60 s


def head_pools(t):
    """moving-head spot pools on the stage boards / crowd: (x, z, radius, rgb)"""
    out = []
    for k, (sp, ph, x0) in enumerate(HEAD_PATHS):
        a = 2 * math.pi * (t * sp / T.DUR + ph)
        x = x0 * 0.7 + 1.4 * math.sin(a)
        z = 2.8 + 1.6 * math.sin(2 * a + k)
        c = mood_col(t) * 0.85 + 0.12
        out.append((x, z, 0.95, c * 0.55))
    return out


# ================================================================== frame
def render(f):
    t = f / FPS
    cam = camera(t)
    fr = Frame(void(cam, t))
    img = fr.array()
    stars(img, cam, t)
    ms = cam.screen_of_dir(S.MOON_DRAW)
    searchlights(img, cam, t)
    fr.set_array(img)
    if ms is not None:
        fr.disc(ms, 0.042 * cam.f, (246, 244, 255), n=20)
        fr.disc((ms[0] + 5, ms[1] - 3), 0.03 * cam.f, (222, 222, 248), n=16)
        fr.glow(ms[0], ms[1], 0.14 * cam.f, (180, 180, 240), 0.45)
    tex_prev = led_texture(min(t, T.T_CRACK - 0.01) if t < T.T_RESTART else t, f)
    gone = crack_gone(t)
    wall_col = wall_mean_color(tex_prev) * (1 - gone) + wall_mean_color(np.where(ARENA[..., None], EMPTY_COL, 0.0)) * gone
    L = lights(t, cam, wall_col)

    # ---- painter list: static objects, dynamic props, the LED wall, crowd
    items = []
    for o, (C, Nn, kv) in zip(STATIC, STAT_FACES):
        if o.name in ('slab', 'slab_under', 'slab_root'):
            continue
        items.append((float(cam.depth(o.ctr)[0]), 'obj', (o, C, Nn, kv)))
    hx = hopper_x(t)
    hf, car = S.hopper_faces(hx)
    for fs, nm, col in ((hf, 'hopper', tuple(S.TEAM_COL[0] * 0.95)), (car, 'car', (120, 122, 140))):
        o = Obj(fs, col, nm, cast=False)
        C = np.array([Q.mean(0) for Q, n in fs]); Nn = np.array([n for Q, n in fs])
        items.append((float(cam.depth(o.ctr)[0]), 'obj', (o, C, Nn, np.ones(len(fs)))))
    for q, st in enumerate(S.STACKS):
        items.append((float(cam.depth(st['cabinets'][0][1])[0]) - 0.6, 'stack', q))
    items.append((float(cam.depth(S.WC)[0]) + 0.3, 'wall', None))
    crowd, phones = crowd_objs(t, cam)
    for o in crowd:
        C = np.array([Q.mean(0) for Q, n in o.faces]); Nn = np.array([n for Q, n in o.faces])
        items.append((float(cam.depth(o.ctr)[0]), 'obj', (o, C, Nn, np.ones(len(o.faces)))))

    # ---- stage slab sides first, then the boards per pixel
    for o in STATIC:
        if o.name in ('slab_under', 'slab_root', 'slab'):
            fc = [(cam.depth(Q.mean(0))[0], Q, n, j) for j, (Q, n) in enumerate(o.faces) if np.dot(n, cam.pos - Q.mean(0)) > 0 and n[1] < 0.9]
            fc.sort(key=lambda a: -a[0])
            if fc:
                C = np.array([q.mean(0) for _, q, _, _ in fc]); Nn = np.array([n for _, _, n, _ in fc])
                cols = shade(L, Nn, C, np.array([o.color * o.tone[j] for *_, j in fc]))
                for (z, Q, n, j), c in zip(fc, cols):
                    fr.poly(cam, Q, c)
    _boards(fr, cam, t, L, wall_col)

    tex = None
    for _, kind, it in painter(items):
        if kind == 'wall':
            tex = led_wall(fr, cam, t, f)
        elif kind == 'stack':
            _draw_stack(fr, cam, t, L, it)
        else:
            o, C, Nn, kv = it
            vis = np.array([np.dot(n, cam.pos - c) > 0 for n, c in zip(Nn, C)])
            if not vis.any():
                continue
            idx = np.nonzero(vis)[0]
            dep = cam.depth(C[idx])
            idx = idx[np.argsort(-dep)]
            emit = {'hopper': 0.55, 'balltube': 0.25, 'ripple_horn': 0.3, 'leaf': 0.2}.get(o.name, 0.0)
            cols = shade(L, Nn[idx], C[idx], np.array([o.color * o.tone[j] for j in idx]), kvis=kv[idx][:, None], emit=emit)
            for j, c in zip(idx, cols):
                fr.poly(cam, o.faces[j][0], c)
            if o.name == 'truss_beam':
                for a, b in S.TRUSS_LINES:
                    fr.line(cam, [a, b], (110, 116, 140))
    _props_fx(fr, cam, t, hx)
    for p in S.FOOTLIGHTS:
        s = cam.wp(p + [0, 0.1, 0.1])
        fr.glow(s[0], s[1], 7, (255, 170, 90), 0.5)
    for p, team in phones:
        s = cam.wp(p)
        c = S.TEAM_COL[team] if team >= 0 else np.array([200, 200, 220.0])
        fr.glow(s[0], s[1], 4, tuple(c), 0.7)
    _pennants(fr, cam, t)

    # ---- lens: focus locked on the wall
    img, depth = fr.array(), fr.depth()
    zf = cam.depth(S.WC)[0]
    img = dof(img, coc_map(depth, zf, 5.4, sharp=focus_band(cam)))
    img = apply_glows_fx(img, fr.glows, cam, t)
    img = genre_fx(img, cam, t)
    if ms is not None:
        img = god_rays(img, depth, ms, (150, 150, 220), 0.25)
    img = grade(bloom(np.clip(img, 0, 255), thr=210, k=0.35), contrast=0.14, sat=1.12)
    img = np.clip(img * vignette(strength=0.7, cap=0.35) * top_scrim(), 0, 255).astype(np.uint8)
    out = img.copy()
    _ui(out, cam, t)
    return out


SUBJ_PTS = np.concatenate([np.array([S.WC + [S.RW * math.cos(a), S.RW * math.sin(a), 0] for a in np.linspace(0, 2 * math.pi, 24)]),
                           np.array([S.WC, S.RIPPLE_EMIT, S.VINE_POT, S.BALL_TUBE[0], [S.HOP_X0, S.HOP_RAIL_Y, 0.12],
                                     [S.HOP_X1, S.HOP_RAIL_Y, 0.12]])])


def focus_band(cam):
    """sharp band that covers the whole wall and its props (CoC 0 on the subject in every frame)"""
    zf = cam.depth(S.WC)[0]
    z = cam.depth(SUBJ_PTS)
    need = np.abs(1 - zf / z).max() + 0.01
    return max(need, 0.05)


def _boards(fr, cam, t, L, wall_col):
    """the stage top (y = 0) per pixel: dark boards, moon shadows, wall glow, pools, footlights, reflection"""
    arr, depth = fr.array(), fr.depth()
    ys, xs = np.nonzero(YY > 300)
    X, Z, d = cam.ground_hit(xs.astype(float), ys.astype(float))
    ok = octagon_contains(X, Z) & (d > 0) & (d < depth[ys, xs])
    xs, ys, X, Z, d = xs[ok], ys[ok], X[ok], Z[ok], d[ok]
    plank = np.floor((X + 20) / 0.42).astype(int)
    seam = (np.abs(((X + 20) / 0.42) % 1 - 0.5) > 0.46)
    shift = (plank * 0.37) % 1.0
    joint = np.abs(((Z + shift * 2.2) / 2.2) % 1 - 0.5) > 0.485
    alb = np.array([54, 50, 76.0]) * (0.92 + 0.08 * ((plank * 7) % 3)[:, None] / 2)
    alb = np.where((seam | joint)[:, None], alb * 0.55, alb)
    I = np.tile(L.amb + L.sky + 0.05, (len(X), 1))
    lit = 1 - FLOOR.sample(MOON_MASK, X, Z)
    I += (lit * S.KEY[1])[:, None] * S.KEY_COL[None]
    # wall glow (emissive disc above the boards)
    wc = np.clip(wall_col / 255.0, 0, 1)
    dz = Z - 0.3
    dist = np.sqrt(X ** 2 + (S.WC[1]) ** 2 + dz ** 2)
    I += (np.clip(dz, 0, None) / dist * 2.6 / (1 + (dist / 3.2) ** 2))[:, None] * wc[None] * 1.4
    # moving-head pools
    for (px, pz, pr, pc) in head_pools(t):
        dd = np.hypot(X - px, Z - pz) / pr
        I += (np.clip(1 - dd, 0, 1) ** 0.6)[:, None] * pc[None] * 1.8
    # footlights (warm strip along the front edge)
    fd = np.abs(Z - S.FOOT_Z)
    I += (np.exp(-fd / 0.5) * (np.abs(X) < 3.8))[:, None] * np.array([1.0, 0.6, 0.3]) * 0.55
    I *= contact_ao(X, Z, S.FEET)[:, None]
    col = alb * I
    mw = T.mood(t)
    if mw[3] > 0.01:                       # JAZZ: low blue haze rolling over the boards
        hz = 0.5 + 0.5 * np.sin(X * 1.3 + Z * 0.7 + 2 * math.pi * t / whole_period(9.0, T.DUR)) * np.sin(X * 0.6 - 2 * math.pi * t / whole_period(13.0, T.DUR))
        col += (mw[3] * (0.35 + 0.65 * hz) * np.clip(Z / 4.5, 0.2, 1.0))[:, None] * np.array([30, 44, 90.0])
    # glossy reflection of the LED wall (mirror the eye in the boards, sample the wall plane)
    eye = cam.pos * np.array([1, -1, 1])
    P = np.stack([X, np.zeros_like(X), Z], 1)
    dv = P - eye
    tw = (0.0 - eye[2]) / np.where(np.abs(dv[:, 2]) < 1e-6, -1e-6, dv[:, 2])
    Rw = eye[None] + tw[:, None] * dv
    u = (Rw[:, 0] - S.WC[0]) / S.RW
    v = (Rw[:, 1] - S.WC[1]) / S.RW
    inw = (u * u + v * v <= 1) & (tw > 0) & (Z > 0.2)
    if inw.any():
        tex = led_texture(min(t, T.T_CRACK - 0.01) if t < T.T_RESTART else t, 0)
        ti = np.clip(((1 - v[inw]) / 2 * N * TEX).astype(int), 0, N * TEX - 1)
        tj = np.clip(((u[inw] + 1) / 2 * N * TEX).astype(int), 0, N * TEX - 1)
        fade = np.exp(-(Z[inw] - 0.2) / 2.2) * 0.32 * (1 - crack_gone(t))
        col[inw] += tex[ti, tj] * fade[:, None]
    arr[ys, xs] = np.floor(col / 5 + BAYER_FULL[ys, xs, None]) * 5
    depth[ys, xs] = d
    fr.set_array(arr, depth)


def _draw_stack(fr, cam, t, L, q):
    st = S.STACKS[q]
    b = stack_light(q, t)
    pmp = pump(q, t) * (1 if T.alive(q, t) else 0)
    if st.get('hung'):
        r0, r1 = st['rig']
        fr.line(cam, [r0, r1], (120, 124, 146))
    for fs, c, R in st['cabinets']:
        vis = [(cam.depth(Q.mean(0))[0], Q, n) for Q, n in fs if np.dot(n, cam.pos - Q.mean(0)) > 0]
        vis.sort(key=lambda a: -a[0])
        C = np.array([Q.mean(0) for _, Q, _ in vis]); Nn = np.array([n for _, _, n in vis])
        kv = OCC.vis(C + Nn * 0.02, st['grp'], d=S.KEY)[:, None]
        cols = shade(L, Nn, C, np.array([44, 44, 60.0]), kvis=kv)
        for (z, Q, n), cc in zip(vis, cols):
            fr.poly(cam, Q, cc)
    tc = S.TEAM_COL[q]
    for c, r, nrm in st['cones']:
        a = np.cross(nrm, [0, 1.0, 0]); a /= np.linalg.norm(a)
        bb = np.cross(nrm, a)
        def ring(rr, off, n=12):
            return np.array([c + nrm * off + rr * (math.cos(2 * math.pi * k / n) * a + math.sin(2 * math.pi * k / n) * bb) for k in range(n)])
        fr.poly(cam, ring(r, 0.0), (22, 22, 30))
        fr.poly(cam, ring(r * 0.82, 0.004 + 0.03 * pmp), np.array([58, 58, 74.0]) + tc * 0.15 * b)
        fr.poly(cam, ring(r * 0.32, 0.008 + 0.045 * pmp), np.array([90, 90, 110.0]) + tc * 0.25 * b)
    for p0, p1 in st['leds']:
        s0, s1 = cam.wp(p0), cam.wp(p1)
        cc = np.array([30, 30, 40.0]) + (tc - 30) * b
        fr.draw.line([tuple(s0), tuple(s1)], fill=tuple(int(x) for x in np.clip(cc, 0, 255)), width=2)
        if b > 0.05:
            m = (s0 + s1) / 2
            fr.glow(m[0], m[1], 9 + 10 * lead_flare(q, t), tuple(tc), 0.18 * b + 0.35 * lead_flare(q, t))
    # knocked-out smoke puff
    if q in T.OUT_T and 0 <= t - T.OUT_T[q] < 2.4:
        u = (t - T.OUT_T[q]) / 2.4
        top = st['cabinets'][0][1] + [0, 0.6 + 1.6 * u, 0.3]
        s = cam.wp(top)
        fr.glow(s[0], s[1], 10 + 18 * u, (150, 140, 160), 0.5 * (1 - u))


def _props_fx(fr, cam, t, hx):
    """sand stream from the hopper, ripple emitter pulse on every ring, ball launcher glint"""
    sf = sim_index(t)
    st = STATE[sf]
    if st[0].get('pouring', True) and T.alive(0, t) and not (T.T_CRACK <= t < T.T_RESTART):
        top = np.array([hx, S.HOP_RAIL_Y - 0.26, 0.12])
        ytop = S.WC[1] + math.sqrt(max(0.0, S.RW ** 2 - (hx - S.WC[0]) ** 2))
        for k in range(10):
            u = ((t * 3.0 + k / 10) % 1.0)
            p = np.array([hx + 0.02 * math.sin(k * 7.1 + t * 9), top[1] + (ytop - top[1]) * u, 0.1])
            s = cam.wp(p)
            if 0 <= s[0] < W and 0 <= s[1] < H:
                fr.draw.point((s[0], s[1]), fill=(255, 232, 170))
    if T.alive(2, t):
        ph = T.beat_phase(t)
        s = cam.wp(S.RIPPLE_EMIT)
        fr.glow(s[0], s[1], 6 + 6 * ph, (120, 255, 240), 0.5 * (1 - ph))
    if T.alive(1, t):
        s = cam.wp(S.BALL_TUBE[1])
        fr.glow(s[0], s[1], 4, (255, 140, 110), 0.35)


def _pennants(fr, cam, t):
    for x, q in S.PENNANTS:
        top = np.array([x, S.TY - 0.14, S.TZ + 0.16])
        sway = 0.08 * math.sin(2 * math.pi * 22 * t / T.DUR + x)
        pts = [top + [-0.18, 0, 0], top + [0.18, 0, 0], top + [sway, -0.42, 0.05]]
        c = S.TEAM_COL[q] if T.alive(q, t) else np.array([70, 70, 84.0])
        fr.poly(cam, np.array(pts), c * 0.9)


def apply_glows_fx(img, glows, cam, t):
    for g in glows:
        add_glow(img, *g)
    _fireworks(img, cam, t)
    return img


_gr = np.random.default_rng(25)
MOTES = [(x, z, ph, n) for x, z, ph, n in zip(_gr.uniform(-3.2, 3.2, 60), _gr.uniform(0.4, 3.6, 60), _gr.uniform(0, 1, 60),
                                              _gr.integers(5, 12, 60))]
LASER_COL = [np.array([40, 232, 226.0]), np.array([255, 90, 210.0])]


def _hash(*xs):
    v = math.sin(sum(x * c for x, c in zip(xs, (12.9898, 78.233, 37.719, 4.581)))) * 43758.5453
    return v - math.floor(v)


def genre_fx(img, cam, t):
    """the leading genre's stage effects (mood-weighted, so they ease in and out at each lead change):
    ROCK pyro jets on every bar, EDM lasers that step on every beat, JAZZ smoky spot cones, ORCHESTRA gold motes"""
    w = T.mood(t)
    live = t < T.T_CRACK
    # ---- ROCK: flame jets from the two front cannons on each bar's downbeat (never on bar 0: the loop seam)
    if w[1] > 0.02 and live:
        bar = int((t - T.GRID0) // T.BAR)
        u = t - (T.GRID0 + bar * T.BAR)
        if bar > 0 and u < 0.7:
            a = w[1] * min(1.0, u / 0.05) * (1 - u / 0.7) ** 1.4
            for k, p in enumerate(S.PYRO):
                s0, s1 = cam.wp(p), cam.wp(p + np.array([0, 1.0, 0]))
                pxm = float(np.hypot(*(s1 - s0)))                     # screen px per metre at the cannon
                top = 1.5 * min(1.0, u / 0.12)
                sp = cam.wp(p + np.array([0, 0.33 + top * 0.45, 0]))
                add_glow(img, sp[0], sp[1], pxm * 0.55, (255, 120, 50), a * 0.35)        # warm halo
                for j in range(22):
                    f = j / 21
                    jx = (_hash(bar, k, j, 1) - 0.5) * 0.22 * f + 0.05 * math.sin(2 * math.pi * (u * 9 + j * 0.3))
                    pos = p + np.array([jx, 0.33 + top * f, 0.0])
                    sp = cam.wp(pos)
                    col = np.array([255, 248, 210.0]) * (1 - f) ** 2 + np.array([255, 150, 50.0]) * (1 - (1 - f) ** 2)
                    if f > 0.75:
                        col = col * (1 - (f - 0.75) * 2) + np.array([220, 60, 30.0]) * (f - 0.75) * 2
                    add_glow(img, sp[0], sp[1], pxm * (0.13 - 0.06 * f), tuple(col), a * (1.1 - 0.5 * f))
    # ---- EDM: three beams per emitter fanning into the sky; the fan steps to a new shape on every beat
    if w[2] > 0.02:
        nb = int((t - T.GRID0) // T.BEAT)
        bp = T.beat_phase(t)
        ov = Image.new('RGB', (W, H))
        d = ImageDraw.Draw(ov)
        for k, p in enumerate(S.LASERS):
            sp = cam.wp(p)
            for j in range(3):
                def ang(n):
                    return (-0.9 + 0.9 * j) + 0.55 * math.sin(n * 1.7 + k * 2.1 + j)
                a0, a1 = ang(nb - 1), ang(nb)
                aa = a0 + (a1 - a0) * T.ease(min(1.0, bp / 0.2))
                dv = np.array([math.sin(aa) * 0.9, 1.0, -0.35 + 0.15 * math.cos(nb + k)])
                ep = cam.wp(p + dv * 30)
                c = LASER_COL[(k + j) % 2] * (0.55 + 0.45 * math.exp(-bp / 0.25))
                d.line([tuple(sp), tuple(ep)], fill=tuple(int(x) for x in c), width=1)
        img += np.asarray(ov, np.float32) * (1.3 * w[2])
    # ---- JAZZ: two soft spot cones from the inner moving heads down onto the stage, in a blue haze
    if w[3] > 0.02:
        pools = head_pools(t)
        ov = Image.new('L', (W, H))
        d = ImageDraw.Draw(ov)
        for hk in (1, 2):
            px, pz, pr, pc = pools[hk]
            apex = cam.wp(S.HEADS[hk])
            b0, b1 = cam.wp(np.array([px - pr * 0.8, 0.0, pz])), cam.wp(np.array([px + pr * 0.8, 0.0, pz]))
            d.polygon([tuple(apex), tuple(b0), tuple(b1)], fill=255)
        m = np.asarray(ov.filter(ImageFilter.GaussianBlur(3)), np.float32) / 255.0
        img += m[..., None] * np.array([70, 95, 200.0]) * 0.32 * w[3]
    # ---- ORCHESTRA: warm gold motes drifting up through the light
    if w[0] > 0.02:
        for x, z, ph, n in MOTES:
            v = (ph + n * t / T.DUR) % 1.0
            pos = np.array([x + 0.12 * math.sin(2 * math.pi * (v * 2 + ph)), 0.2 + 4.2 * v, z])
            sp = cam.wp(pos)
            add_glow(img, sp[0], sp[1], 1.6, (255, 210, 120), 0.7 * w[0] * math.sin(math.pi * v))
    return img


_fw = np.random.default_rng(9)
FIREWORKS = []
if T.T_WIN is not None:
    for k, dt in enumerate([0.35, 1.4, 2.6, 3.9, 5.2, 6.4]):
        FIREWORKS.append((T.T_WIN + dt, _fw.uniform(70, 290), _fw.uniform(40, 120), _fw.uniform(0, 6.28)))


def _fireworks(img, cam, t):
    for t0, x, y, a0 in FIREWORKS:
        u = t - t0
        if not (0 <= u < 1.6):
            continue
        c = S.TEAM_COL[T.WINNER] if int(t0 * 10) % 3 else np.array([255, 240, 220.0])
        for k in range(18):
            a = a0 + 2 * math.pi * k / 18
            r = 34 * (1 - math.exp(-u / 0.35))
            px, py = x + r * math.cos(a), y + r * math.sin(a) + 14 * u * u
            fade = max(0.0, 1 - u / 1.6)
            add_glow(img, px, py, 2, tuple(c), 0.9 * fade)


LABEL_SPOTS = {0: (-0.55, 1.45), 1: (1.25, 1.05), 2: (-1.75, -0.35), 3: (1.15, -1.25)}   # wall-local, near each base
BAR_Y = 373                      # share bar top (internal px); names + instruments under it, all inside SAFE


def _ui(out, cam, t):
    x0s, y0s, x1s, y1s = SAFE
    # genre labels pop at their bases in the first ~1.3 s (name + instrument), fade by 7.9 s
    if t < T.LABEL_END:
        for k, q in enumerate(T.SHOW):
            t0 = T.LABEL_T[k]
            if t < t0:
                continue
            wx, wy = LABEL_SPOTS[q]
            sp = cam.wp(S.WC + [wx, wy, 0])
            name, instr = T.NAMES[q], T.INSTR[q]
            w1, w2 = len(name) * 12 - 2, len(instr) * 6 - 1
            ww = max(w1, w2)
            bx = int(np.clip(sp[0] - ww / 2, x0s + 2, x1s - ww - 2))
            u = t - t0
            drop = int(round(4 * (1 - min(1.0, u / 0.1)) ** 2))
            by = int(np.clip(sp[1] - 12, y0s + 60, y1s - 80)) - drop
            fade = 1.0 - min(1.0, max(0.0, (t - (T.LABEL_END - 0.4)) / 0.4))
            under = out.copy() if fade < 1 else None
            label(out, name, bx + (ww - w1) // 2, by, tuple(int(c) for c in S.TEAM_COL[q]), 2)
            label(out, instr, bx + (ww - w2) // 2, by + 17, CREAM, 1)
            if under is not None:
                out[:] = (under * (1 - fade) + out * fade).astype(np.uint8)
    _share(out, t)
    for t0, t1, lines in T.CAPTIONS:
        if t0 <= t < t1:
            hi = ui.AMBER
            for q in range(4):
                if any(f'*{T.NAMES[q]}*' in ln for ln in lines):
                    hi = tuple(int(c) for c in np.clip(S.TEAM_COL[q] * 0.8 + 60, 0, 255))
            draw_captions(out, t, [(t0, t1, lines)], hi=hi, style='impact')


def _share(out, t):
    """the scoreboard, on screen the whole video: share bar + crown, each genre's name + lead instrument"""
    order = T.SHOW
    sh = T.smooth_share(t)
    outs = [not T.alive(q, t) for q in order]
    names = [T.NAMES[q] for q in order]
    cols = [tuple(int(c) for c in S.TEAM_COL[q]) for q in order]
    x0, x1 = SAFE[0] + 4, SAFE[2] - 4
    lead = 11
    y = BAR_Y
    sc, slots = ui.share_layout(names, x0, x1, 2)
    ny = y + 6 + lead
    iy = ny + 7 * sc + 2
    bot = iy + 7
    # backing panel (crown room + bar + names + instruments)
    bx0, by0, bx1, by1 = x0 - 4, y - 9, x1 + 4, bot + 2
    reg = out[by0:by1, bx0:bx1].astype(np.float32)
    out[by0:by1, bx0:bx1] = (reg * 0.45 + np.array(INK, float) * 0.25).astype(np.uint8)
    edge = np.array([62, 74, 112], float)
    out[by0, bx0:bx1] = (out[by0, bx0:bx1] * 0.4 + edge * 0.6).astype(np.uint8)
    out[by1 - 1, bx0:bx1] = (out[by1 - 1, bx0:bx1] * 0.6 + edge * 0.4).astype(np.uint8)
    ui._note(bx0, by0, bx1, by1, 'panel', 'share bar')
    if T.T_CRACK <= t:
        sh_draw = list(sh)
    else:
        sh_draw = sh
    info = share_bar(out, [sh_draw[q] for q in order], names, cols, x0=x0, x1=x1, y=y, h=6, out=outs, backing=False,
                     show_pct=False, lead=lead)
    for k, q in enumerate(order):
        sx0, sx1, scx = slots[k]
        s_ = 'OUT' if outs[k] else T.INSTR[q]
        w = len(s_) * 6 - 1
        label(out, s_, int(round(scx - w / 2)), iy, (150, 150, 170) if outs[k] else CREAM, 1)
    # the crown rides above the leader's segment
    if T.T_BELL - 3 <= t < T.T_CRACK:
        lq = T.leader(t)
        if lq in order and not outs[order.index(lq)]:
            xa, xb, xc = info['segments'][order.index(lq)]
            if xb - xa > 10:
                hop = int(round(4 * lead_flare(lq, t) * abs(math.sin(t * 9))))
                _crown(out, int(round(xc)), y - 7 - hop)


def _plain_text(img, s, x, y, col):
    m = font.text_mask(s, 1)
    h, w = m.shape
    if x < 0 or y < 0 or x + w > W or y + h > H:
        return
    img[y:y + h, x:x + w][m] = col


CROWN = np.array([[c == '#' for c in r] for r in "#..#..#|##.#.##|#######|#######".split('|')])


def _crown(img, cx, y):
    h, w = CROWN.shape
    x = cx - w // 2
    if 1 <= x and x + w < W - 1:
        img[y - 1:y + h + 1, x - 1:x + w + 1][font.dilate(CROWN, 1)] = (40, 30, 10)
        img[y:y + h, x:x + w][CROWN] = (255, 216, 90)
