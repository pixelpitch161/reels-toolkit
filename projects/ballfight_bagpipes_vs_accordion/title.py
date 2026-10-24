"""#30 v3 intro test (user, Oct 24): the VS title card on bar 0.

The frame splits diagonally into two panels in the fighters' colours (tartan green top-left, magenta bottom-right),
each with its fighter's ball big and close, like a fighting-game versus screen, with a small "BALLFIGHT #4" tag at the
top. The words slam in one per beat: BAGPIPES (beat 1 = frame 0), VS (beat 2: it cracks the diagonal divider, sparks),
ACCORDION (beat 3); each slam comes down from huge to size with an overshoot, motion streaks, a shake, a dust burst and a
local light in its colour. Beat 4: everything pulses, then the panels slide apart like doors along the cracked divider
to reveal the ring (VS splits with them). In the last T_CLOSE s of the file the doors slide shut again and BAGPIPES
falls in as a ghost, so the slam on frame 0 is the loop's reset and the seam stays small.

Everything is screen space at the internal resolution (W x H) and deterministic in t.

    import title as TT
    img = TT.draw(img, t)          # float or uint8 (H, W, 3) -> same dtype; no-op when the doors are open"""
import math
import os
import numpy as np
from reelkit.config import W, H, BAYER_FULL, SAFE
from reelkit.font import ADV, text_mask, dilate
from reelkit import ui
import timeline as T

HERE = os.path.dirname(os.path.abspath(__file__))
INK = np.array([16, 10, 26], float)
GOLD = np.array([255, 226, 120], float)
WHITE = np.array([255, 250, 236], float)

# the divider: bottom-left -> top-right; N points to the top-left (the BAGPIPES side)
DA, DB = np.array([0.0, 470.0]), np.array([360.0, 170.0])
DD = (DB - DA) / np.linalg.norm(DB - DA)
N = np.array([DD[1], -DD[0]])
if (np.array([0.0, 0.0]) - DA) @ N < 0:
    N = -N
OPEN_PX = 560.0                                       # how far each door travels to clear the frame
YY, XX = np.mgrid[0:H, 0:W].astype(np.float64)
SIDE = (XX - DA[0]) * N[0] + (YY - DA[1]) * N[1]      # > 0: top-left (BAGPIPES) half

# layout (closed doors): words inside the safe zone at the biggest pixel-font scale that fits
WORDS = {
    'BAGPIPES': dict(scale=6, cx=180, cy=134, side=0),     # 282 px wide: fits x 20-340 above y 290
    'VS': dict(scale=12, cx=180, cy=300, side=-1),          # the largest letters, on the divider (splits with the doors)
    'ACCORDION': dict(scale=5, cx=160, cy=468, side=1),     # 265 px wide: fits left of the button column below y 290
}
BALLS = [dict(cx=78, cy=222, r=56), dict(cx=282, cy=396, r=56)]   # BAGPIPES, ACCORDION: big and close


def _ease_in(u):
    u = min(1.0, max(0.0, u))
    return u * u * u


def _ease_out(u):
    u = min(1.0, max(0.0, u))
    return 1 - (1 - u) ** 3


def door_offset(t):
    """how far the doors are open (px along the divider normal): 0 closed .. OPEN_PX gone; None = no title card"""
    if t >= T.DUR - T.T_CLOSE:                                     # the run-up: the doors slide shut, closed on the last frame
        u = (t - (T.DUR - T.T_CLOSE)) / (T.T_CLOSE - 1.0 / T.FPS)
        return OPEN_PX * (1 - _ease_out(u))
    if t < T.T_DOORS[0]:
        return 0.0
    if t < T.T_DOORS[1]:
        return OPEN_PX * _ease_in((t - T.T_DOORS[0]) / (T.T_DOORS[1] - T.T_DOORS[0]))
    return None


def slam_k(t, t0):
    """the word's size factor: huge at t0, down to 1 in 0.16 s, a small overshoot, settled by 0.26 s"""
    u = (t - t0) / 0.16
    if u < 0:
        return None
    if u < 1:
        return 1 + 1.25 * (1 - u) ** 2
    if u < 1.6:
        return 1 - 0.08 * math.sin(math.pi * (u - 1) / 0.6)
    return 1.0


def word_state(t, j):
    """(size factor, alpha) of word j at t; BAGPIPES also falls in as a ghost at the end of the file"""
    t0 = T.TITLE[j][0]
    if t >= T.DUR - T.T_CLOSE and (j > 0 or t < T.DUR - 0.12):      # the closing doors carry only the panels, then
        if j > 0 or t < T.DUR - 0.12:                               # BAGPIPES' ghost falls in (the frame-0 slam)
            return None
    if j == 0 and t >= T.DUR - 0.12:                               # the ghost of the frame-0 slam
        u = (t - (T.DUR - 0.12)) / (0.12 - 1.0 / T.FPS)
        u = min(1.0, u)
        return (2.25 if u >= 1.0 - 1e-6 else 2.25 + 0.6 * (1 - u)), u      # on the last frame: exactly frame 0's word
    k = slam_k(t, t0)
    if k is None:
        return None
    pulse = 0.05 * T.beat_pulse(t, 0.12) if (j == 1 and 3 * T.BEAT - 0.07 <= t < T.T_DOORS[0] + 0.05) else 0.0   # VS
    # (only VS pulses in size on beat 4: the names already use the widest scale that fits the safe zone)
    return k + pulse, 1.0


def shake(t):
    """the title's own shake: a decaying jolt after each slam, zero on the slam frame itself (a clean seam on frame 0)"""
    dx = dy = 0.0
    for j, (t0, w) in enumerate(T.TITLE):
        u = t - t0 - 1.0 / T.FPS
        if 0 < u < 0.35:
            a = (7.0 if j == 1 else 5.0) * math.exp(-u / 0.09)
            dx += a * math.sin(2 * math.pi * 23 * u)
            dy += a * 0.7 * math.sin(2 * math.pi * 29 * u + 1.0)
    return dx, dy


# ---------------------------------------------------------------- panels
def _panel_rgb(colour, dark, side):
    """a panel's fill: a gradient from the fighter's deep shade at the far corner to its colour at the divider, a soft
    pattern (tartan for the bagpipes, bellows pleats for the accordion), dithered bands"""
    d = np.abs(SIDE) / 400.0
    k = np.clip(1 - d, 0, 1) ** 1.3
    base = dark[None, None] * (1 - k[..., None]) + colour[None, None] * (0.40 + 0.25 * k[..., None]) * k[..., None] \
        + dark[None, None] * 0.6 * k[..., None]
    if side == 0:                                                     # a large tartan sett, low contrast
        u, v = (XX + YY) / 46.0, (XX - YY) / 46.0
        chk = ((np.floor(u) + np.floor(v)) % 2)
        line = (np.abs((u % 1) - 0.5) < 0.05) | (np.abs((v % 1) - 0.5) < 0.05)
        base = base * (0.92 + 0.12 * chk[..., None])
        base = np.where(line[..., None], base * 0.7 + np.array([200, 50, 46]) * 0.3, base)
    else:                                                             # pleats parallel to the divider
        s = (SIDE / 14.0) % 2
        base = base * np.where((s < 1)[..., None], 1.06, 0.84)
    q = np.floor((base / 255.0) * 10 + BAYER_FULL[..., None] * 0.9) / 10 * 255
    return np.clip(q, 0, 255)


_PANEL = {}


def panels():
    if not _PANEL:
        import render as RD
        for side, k in ((0, 0), (1, 1)):
            _PANEL[side] = _panel_rgb(RD.COL[k], RD.DARK[k], side)
    return _PANEL


# ---------------------------------------------------------------- big balls
_SPR = {}
SPR_PATH = os.path.join(HERE, 'check', 'sprites_title.npz')


def big_sprite(k):
    if k not in _SPR:
        import render as RD
        nm = RD.CFG[k]['pp']
        key = f'{nm}_title'
        if os.path.exists(SPR_PATH):
            z = np.load(SPR_PATH)
            _SPR.update({int(kk.split('_')[-1]): z[kk] for kk in z.files})
        if k not in _SPR:
            import sprites as SP
            yc, sw, roll = RD.CFG[k]['view']
            _SPR[k] = SP.render_sprite(nm, int(BALLS[k]['r'] * 0.8), float(yc), float(roll))
            np.savez_compressed(SPR_PATH, **{f'title_{kk}': v for kk, v in _SPR.items()})
    return _SPR[k]


def draw_big_ball(layer, alpha, k, cx, cy, rp, glow, col):
    """a glass ball (as in the ring, at close-up size) with the Pixel Pitch instrument inside"""
    ri = int(rp) + 3
    x0, y0 = int(cx) - ri, int(cy) - ri
    n = 2 * ri + 1
    yy, xx = np.mgrid[0:n, 0:n]
    dx = (xx + x0 + 0.5 - cx) / rp
    dy = (yy + y0 + 0.5 - cy) / rp
    d2 = dx * dx + dy * dy
    inside = d2 <= 1.0
    rim = inside & (d2 > (1 - 3.0 / rp) ** 2)
    edge = (d2 > 1.0) & (d2 <= (1 + 2.5 / rp) ** 2)
    nz = np.sqrt(np.clip(1 - d2, 0, 1))
    lam = np.clip(-0.25 * dx - 0.55 * dy + nz * 0.6, 0, 1)
    band = np.floor(lam * 4 + BAYER_FULL[:n, :n] * 0.9) / 4
    inner = (col * 0.30 + np.array([236, 232, 226]) * 0.70)[None, None] * (0.70 + 0.36 * band[..., None])
    xa, ya, xb, yb = max(0, x0), max(0, y0), min(W, x0 + n), min(H, y0 + n)
    if xb <= xa or yb <= ya:
        return
    sl = (slice(ya - y0, yb - y0), slice(xa - x0, xb - x0))
    reg, ar = layer[ya:yb, xa:xb], alpha[ya:yb, xa:xb]
    m = inside[sl]
    reg[m] = inner[sl][m]
    spr = big_sprite(k)
    sn = spr.shape[0]
    sx0, sy0 = int(round(cx)) - sn // 2, int(round(cy)) - sn // 2
    ax0, ay0, ax1, ay1 = max(0, sx0), max(0, sy0), min(W, sx0 + sn), min(H, sy0 + sn)
    if ax1 > ax0 and ay1 > ay0:
        sub = spr[ay0 - sy0:ay1 - sy0, ax0 - sx0:ax1 - sx0]
        ms = sub[..., 3] > 0
        rg = layer[ay0:ay1, ax0:ax1]
        rg[ms] = np.clip(sub[..., :3].astype(float)[ms] * (1.06 + 0.3 * glow), 0, 255)
    reg[rim[sl]] = np.clip(col * (0.9 + 0.5 * glow) + 50 * glow, 0, 255)
    reg[edge[sl]] = INK
    spec = inside & ((dx + 0.38) ** 2 + (dy + 0.5) ** 2 < 0.012)
    reg[spec[sl]] = np.array([255, 252, 244])
    ar[(inside | edge)[sl]] = 1.0


# ---------------------------------------------------------------- words
def word_masks(text, scale, k):
    """(fill, outline, highlight) bool masks of a pixel-font word at integer scale, resized by k (nearest)"""
    base = text_mask(text, scale)
    if abs(k - 1) > 1e-3:
        h0, w0 = base.shape
        hh, ww = max(1, int(round(h0 * k))), max(1, int(round(w0 * k)))
        ys = np.minimum((np.arange(hh) * h0 / hh).astype(int), h0 - 1)
        xs = np.minimum((np.arange(ww) * w0 / ww).astype(int), w0 - 1)
        m = base[ys][:, xs]
    else:
        m = base
    r = max(2, int(round(scale * 0.6 * k)))
    o = dilate(m, r)
    gh = max(1, m.shape[0] // 7 * 2)                                 # the top two font rows: a highlight
    hi = np.zeros_like(m)
    hi[:gh] = m[:gh]
    return m, o, hi, r


def stamp_word(layer, alpha, text, scale, k, cx, cy, col, a=1.0, note=False):
    m, o, hi, r = word_masks(text, scale, k)
    hm, wm = m.shape
    x0, y0 = int(round(cx - wm / 2)), int(round(cy - hm / 2))
    for mask, c, ox, oy, rr in ((o, INK * 0.5, 3, 4, r), (o, INK, 0, 0, r), (m, col, 0, 0, 0),
                                (hi, np.clip(col * 0.55 + 255 * 0.45, 0, 255), 0, 0, 0)):
        mh, mw = mask.shape
        X0, Y0 = x0 - rr + ox, y0 - rr + oy
        xa, ya, xb, yb = max(0, X0), max(0, Y0), min(W, X0 + mw), min(H, Y0 + mh)
        if xb <= xa or yb <= ya:
            continue
        sub = mask[ya - Y0:yb - Y0, xa - X0:xb - X0]
        reg, ar = layer[ya:yb, xa:xb], alpha[ya:yb, xa:xb]
        reg[sub] = reg[sub] * (1 - a) + c * a
        ar[sub] = np.maximum(ar[sub], a)
    if note:                                                         # the settled word, for reelkit.safezone
        ui._note(x0 - r, y0 - r, x0 + wm + r, y0 + hm + r, 'caption', text)
    return x0, y0, wm, hm


# ---------------------------------------------------------------- effects (screen space, deterministic)
def _rng(j):
    return np.random.default_rng(3000 + j)


def streaks(img, t, j, cx, cy, w, h, col):
    """motion streaks either side of a slamming word, shooting outward and fading over 0.25 s"""
    u = (t - T.TITLE[j][0] - 1.0 / T.FPS) / 0.25                 # from the frame after the slam (a clean seam)
    if not 0 <= u < 1:
        return
    rg = _rng(j)
    for i in range(14):
        y = int(cy + rg.uniform(-0.6, 0.6) * h)
        side = 1 if i % 2 else -1
        x_in = cx + side * (w / 2 + 4 + 40 * u)
        L = rg.uniform(30, 80) * (1 - u)
        xa, xb = sorted((int(x_in), int(x_in + side * L)))
        xa, xb = max(0, xa), min(W, xb)
        if 0 <= y < H and xb > xa:
            img[y, xa:xb] = img[y, xa:xb] * (0.35 + 0.65 * u) + np.clip(col * 0.5 + 140, 0, 255) * (0.65 * (1 - u))


def dust(img, t, j, cx, cy, w, h):
    """a dust burst along the word's base: puffs rolling outward, growing, fading over 0.6 s"""
    u = (t - T.TITLE[j][0] - 1.0 / T.FPS) / 0.6
    if not 0 <= u < 1:
        return
    rg = _rng(j + 10)
    for i in range(12):
        x0 = cx + rg.uniform(-0.55, 0.55) * w
        vx = (x0 - cx) / (w / 2 + 1e-6) * rg.uniform(40, 90)
        x = x0 + vx * (1 - (1 - u) ** 2) * 0.8
        y = cy + h * 0.5 + rg.uniform(-4, 4) - 18 * u
        rad = 3 + 12 * u * rg.uniform(0.7, 1.2)
        a = 0.55 * (1 - u) ** 1.5
        x0i, x1i = int(max(0, x - rad)), int(min(W, x + rad + 1))
        y0i, y1i = int(max(0, y - rad)), int(min(H, y + rad + 1))
        if x1i <= x0i or y1i <= y0i:
            continue
        yy, xx = np.mgrid[y0i:y1i, x0i:x1i]
        dd = ((xx - x) ** 2 + (yy - y) ** 2) / (rad * rad)
        m = np.clip(1 - dd, 0, 1)
        m = np.floor(m * 3 + BAYER_FULL[y0i:y1i, x0i:x1i] * 0.9) / 3
        img[y0i:y1i, x0i:x1i] += (np.array([236, 226, 214]) - img[y0i:y1i, x0i:x1i]) * (m * a)[..., None]


def light(img, t, j, cx, cy, col):
    """a local light in the word's colour behind it (soft, small: flash-safe)"""
    u = (t - T.TITLE[j][0])
    if not 0 <= u < 0.5:
        return
    k = min(1.0, u / 0.06) * math.exp(-max(0.0, u - 0.06) / 0.14) * 0.55     # rises from 0 on the slam frame
    r = 110.0
    x0i, x1i = int(max(0, cx - r)), int(min(W, cx + r))
    y0i, y1i = int(max(0, cy - r * 0.6)), int(min(H, cy + r * 0.6))
    yy, xx = np.mgrid[y0i:y1i, x0i:x1i]
    dd = ((xx - cx) / r) ** 2 + ((yy - cy) / (r * 0.6)) ** 2
    img[y0i:y1i, x0i:x1i] += (np.clip(1 - dd, 0, 1) ** 2 * k)[..., None] * col[None, None] * 0.6


CRACK = None


def crack_pts():
    """the divider crack from VS outward to both ends: jagged polylines (+ branches), deterministic"""
    global CRACK
    if CRACK is None:
        rg = np.random.default_rng(77)
        c = np.array([180.0, 320.0])
        lines = []
        for sgn in (1, -1):
            p, pts = c.copy(), [c.copy()]
            while 0 <= p[0] <= W:
                p = p + DD * sgn * rg.uniform(10, 18) + N * rg.uniform(-4, 4)
                pts.append(p.copy())
                if rg.random() < 0.18:                               # a branch
                    q, br = p.copy(), [p.copy()]
                    for _ in range(3):
                        q = q + DD * sgn * rg.uniform(5, 9) + N * rg.choice([-1, 1]) * rg.uniform(5, 10)
                        br.append(q.copy())
                    lines.append(br)
            lines.append(pts)
        CRACK = lines
    return CRACK


def draw_crack(layer, alpha, t):
    """VS cracks the divider: a white-gold flash running out from the centre, then a dark crack line"""
    u = (t - T.TITLE[1][0])
    if u < 0:
        return
    from PIL import Image, ImageDraw
    grow = min(1.0, u / 0.12)
    im = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(im)
    for pts in crack_pts():
        n = max(2, int(len(pts) * grow))
        d.line([tuple(p) for p in pts[:n]], fill=255, width=2)
    m = np.asarray(im, np.float32) / 255.0 > 0.5
    hot = math.exp(-u / 0.12)
    c = INK * (1 - hot) + GOLD * hot
    layer[m] = c
    alpha[m] = 1.0


def sparks(img, t):
    u = t - T.TITLE[1][0]
    if not 0 <= u < 0.45:
        return
    rg = _rng(99)
    for i in range(20):
        a = rg.uniform(0, 2 * math.pi)
        v = rg.uniform(90, 220)
        p0 = np.array([180.0, 300.0])
        p = p0 + v * u * np.array([math.cos(a), math.sin(a)]) + np.array([0, 160 * u * u])
        q = p - v * 0.03 * np.array([math.cos(a), math.sin(a)])
        k = 1 - u / 0.45
        for s in np.linspace(0, 1, 5):
            x, y = (q + (p - q) * s).astype(int)
            if 0 <= x < W and 0 <= y < H:
                img[y, x] = img[y, x] * (1 - k) + GOLD * k


# ---------------------------------------------------------------- the card
def draw(img, t):
    off = door_offset(t)
    if off is None or off >= OPEN_PX - 1:
        return img
    import render as RD
    u8 = img.dtype == np.uint8
    out = img.astype(np.float64)
    out *= 0.55                                                      # the arena dims behind the card
    pn = panels()
    sx, sy = shake(t)
    for side in (0, 1):
        layer = pn[side].copy()
        alpha = (SIDE > 0) if side == 0 else (SIDE <= 0)
        alpha = alpha.astype(np.float64) * 0.94
        # the divider's bright edge on each panel
        edge = np.abs(SIDE) < 2.2
        layer[edge] = np.clip(RD.COL[side] * 0.5 + 150, 0, 255)
        alpha[edge] = 1.0
        b = BALLS[side]
        ts = T.TITLE[0 if side == 0 else 2][0]
        gl = 0.25 + 0.5 * (min(1.0, (t - ts) / 0.06) * math.exp(-max(0.0, t - ts - 0.06) / 0.2)
                           if ts <= t < T.DUR - 1 else 0.0)
        bob = 3.0 * math.sin(2 * math.pi * t / T.BAR + side * math.pi)
        draw_big_ball(layer, alpha, side, b['cx'], b['cy'] + bob, b['r'] * (1 + 0.03 * T.beat_pulse(t, 0.15)), gl,
                      RD.COL[side])
        if side == 0:
            ui_tag(layer, alpha)
        # words on this panel (VS on both, clipped to the half-plane so it splits with the doors)
        for j, (t0, text) in enumerate(T.TITLE):
            spec = WORDS[text]
            if spec['side'] not in (side, -1):
                continue
            st = word_state(t, j)
            if st is None:
                continue
            k, a = st
            col = GOLD if text == 'VS' else np.clip(RD.COL[0 if text == 'BAGPIPES' else 1] * 0.85 + 50, 0, 255)
            wl = np.zeros_like(layer)
            wa = np.zeros_like(alpha)
            settled = abs(k - 1) < 0.06 and a >= 1.0 and off == 0.0
            x0, y0, wm, hm = stamp_word(wl, wa, text, spec['scale'], k, spec['cx'], spec['cy'], col, a,
                                        note=settled and (spec['side'] == side or side == 0))
            if spec['side'] == -1:                                   # VS: keep only this side's half
                wa = wa * ((SIDE > 0) if side == 0 else (SIDE <= 0))
            m = wa > 0
            layer[m] = layer[m] * (1 - wa[m, None]) + wl[m] * wa[m, None]
            alpha[m] = np.maximum(alpha[m], wa[m])
        if t < T.DUR - 1:
            draw_crack(layer, alpha, t)
        # slide the door: integer pixel shift along its normal (+ the shake)
        vec = N * off * (1 if side == 0 else -1) + np.array([sx, sy])
        dx, dy = int(round(vec[0])), int(round(vec[1]))
        xs0, xs1 = max(0, -dx), min(W, W - dx)
        ys0, ys1 = max(0, -dy), min(H, H - dy)
        if xs1 <= xs0 or ys1 <= ys0:
            continue
        src = layer[ys0:ys1, xs0:xs1]
        sa = alpha[ys0:ys1, xs0:xs1][..., None]
        dst = out[ys0 + dy:ys1 + dy, xs0 + dx:xs1 + dx]
        out[ys0 + dy:ys1 + dy, xs0 + dx:xs1 + dx] = dst * (1 - sa) + src * sa
    if off == 0.0 and t < T.DUR - 1:                                 # slam effects over the closed card
        for j, (t0, text) in enumerate(T.TITLE):
            spec = WORDS[text]
            st = word_state(t, j)
            if st is None:
                continue
            col = GOLD if text == 'VS' else RD.COL[0 if text == 'BAGPIPES' else 1]
            w_px = len(text) * ADV * spec['scale']
            h_px = 7 * spec['scale']
            light(out, t, j, spec['cx'] + sx, spec['cy'] + sy, col)
            streaks(out, t, j, spec['cx'] + sx, spec['cy'] + sy, w_px, h_px, col)
            dust(out, t, j, spec['cx'] + sx, spec['cy'] + sy, w_px, h_px)
        sparks(out, t)
    out = np.clip(out, 0, 255)
    return out.astype(np.uint8) if u8 else out


def ui_tag(layer, alpha):
    """the small series tag at the top of the card"""
    txt = T.SERIES
    m = text_mask(txt, 1)
    o = dilate(m, 1)
    w_ = m.shape[1]
    x0, y0 = int(round(180 - w_ / 2)), 100
    for mask, c, rr in ((o, INK, 1), (m, np.array([255, 214, 96]), 0)):
        mh, mw = mask.shape
        sub = layer[y0 - rr:y0 - rr + mh, x0 - rr:x0 - rr + mw]
        sub[mask] = c
        alpha[y0 - rr:y0 - rr + mh, x0 - rr:x0 - rr + mw][mask] = 1.0
