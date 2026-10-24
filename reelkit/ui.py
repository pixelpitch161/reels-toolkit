"""On-screen text and panels.

Captions: list of (t0, t1, [lines]). In a line, *word* = highlight colour, ~word~ = alt colour.
Two styles:
    style='type'    (default) they type on (55 chars/s), slide in, and fade out.
    style='impact'  each line drops a few px and lands with weight: a tiny scale punch (110 % -> 100 % over
                    0.1 s), a 1-2 px shake of the TEXT only, and a small puff of pixel sand under the letters.
                    Lines land one after another (IMPACT_STAGGER). impact_landings(caps) lists the landing
                    times, so the soundtrack can put a soft thud on each (reelkit.audio.thud()).

Safe zone (config.SAFE): captions start at the top of the safe box and are centred on it. Lines of up to
14 chars use the big size, up to 21 chars the small size; longer lines overflow the box (the safe-zone
check flags them). Keep HUD panels and labels inside the box too: never in the bottom 35 % of the frame
or under the right-hand button column.

Everything drawn here is recorded in DRAWN (reset each frame by the checker), so
`python -m reelkit.safezone render.py` can verify placement."""
import math
import numpy as np
from .config import W, H, SAFE, SAFE_CX, ease, ss
from .font import ADV, text_mask, stamp, rich_line, dilate

CREAM = (252, 240, 214)
AMBER = (255, 198, 92)
SKYB = (140, 200, 255)
MINT = (124, 246, 200)
CORAL = (255, 118, 96)
PINK = (255, 120, 200)
CYAN = (110, 240, 255)
INK = (12, 16, 30)

CAP_Y0 = SAFE[1] + 4          # 100: first caption line
CAP_BIG_MAX = 14              # chars per line at the big size (scale 3) that fit the safe box
CAP_SMALL_MAX = 21            # chars per line at the small size (scale 2)

# (x0, y0, x1, y1, kind, info) for every caption line, label and panel drawn since the last reset
DRAWN = []


def reset_drawn():
    DRAWN.clear()


def _note(x0, y0, x1, y1, kind, info=''):
    DRAWN.append((int(x0), int(y0), int(x1), int(y1), kind, info))


def _plain(line):
    return line.replace('*', '').replace('~', '')


def caption_scale(lines):
    """3 (big) if every line fits the safe box at the big size, else 2 (small)"""
    return 3 if max(len(_plain(l)) for l in lines) <= CAP_BIG_MAX else 2


def draw_captions(img, t, caps, y0=None, cx=None, base=CREAM, hi=AMBER, alt=SKYB, cps=55, style='type',
                  puff=(236, 204, 150)):
    """y0 / cx default to the top centre of the safe box; override only with a reason.
    style='impact': lines drop in and land with a punch, a text-only shake and a sand puff (see module doc);
    puff: colour of the pixel-sand puff (None = no puff)."""
    y0 = CAP_Y0 if y0 is None else y0
    cx = SAFE_CX if cx is None else cx
    if style == 'impact':
        return _draw_impact(img, t, caps, y0, cx, base, hi, alt, puff)
    for t0, t1, lines in caps:
        if t0 <= t < t1:
            u = t - t0
            nreveal = int(u * cps) + 1
            slide = int(round(6 * (1 - ease(u / 0.25))))
            fade = 1 - ss(t1 - 0.12, t1, t)
            y = y0 + slide
            used = 0
            lens = [len(_plain(l)) for l in lines]
            scale = caption_scale(lines)
            for ln, pl in zip(lines, lens):
                rich_line(img, ln, cx, y, scale, base, hi, alt, nchars=nreveal - used, alpha=fade)
                width = pl * ADV * scale - scale
                x0 = int(round(cx - width / 2))
                _note(x0, y, x0 + width, y + 7 * scale, 'caption', _plain(ln))
                used += pl
                y += 7 * scale + 7


# ------------------------------------------------------------------ impact captions
IMPACT_DROP = 4          # px each line falls before it lands (the first line starts at the top of the safe box)
IMPACT_FALL = 0.10       # s of the fall (accelerating, like a dropped weight)
IMPACT_STAGGER = 0.22    # s between one line landing and the next
IMPACT_PUNCH = 1.10      # scale on landing, settling to 1.0 over IMPACT_PUNCH_T
IMPACT_PUNCH_T = 0.10
IMPACT_SHAKE = 2.0       # px of text-only shake on landing, decaying over IMPACT_SHAKE_T
IMPACT_SHAKE_T = 0.18
PUFF_T = 0.45            # s a sand puff lives


def impact_landings(caps, stagger=IMPACT_STAGGER, fall=IMPACT_FALL):
    """[(t_land, line_index, n_chars)] for every caption line in caps drawn with style='impact'. Put a soft thud
    (reelkit.audio.thud) at each t_land, a little quieter for later lines."""
    out = []
    for t0, t1, lines in caps:
        for k, ln in enumerate(lines):
            tl = t0 + k * stagger + fall
            if tl < t1:
                out.append((tl, k, len(_plain(ln))))
    return out


def _line_ink(line, scale, base, hi, alt):
    """mask + per-pixel colour for a rich caption line at an integer scale"""
    plain, cols = [], []
    mode = 0
    for ch in line:
        if ch == '*':
            mode = 0 if mode == 1 else 1
            continue
        if ch == '~':
            mode = 0 if mode == 2 else 2
            continue
        plain.append(ch)
        cols.append(mode)
    m = text_mask(''.join(plain), scale)
    pal = np.array([base, hi, alt], float)
    ci = np.repeat(np.array(cols, int), ADV * scale)[:m.shape[1]]
    if len(ci) < m.shape[1]:
        ci = np.concatenate([ci, np.full(m.shape[1] - len(ci), ci[-1] if len(ci) else 0)])
    rgb = pal[ci][None, :, :].repeat(m.shape[0], 0)
    return m, rgb


def _resample(a, h, w):
    """nearest-neighbour resize of a 2D/3D array to (h, w)"""
    ys = np.minimum((np.arange(h) * a.shape[0] / h).astype(int), a.shape[0] - 1)
    xs = np.minimum((np.arange(w) * a.shape[1] / w).astype(int), a.shape[1] - 1)
    return a[ys][:, xs]


def _stamp_rgb(frame, mask, rgb, x, y, outline=(10, 20, 26), alpha=1.0):
    """like font.stamp but each ink pixel has its own colour; 1 px outline + drop shadow"""
    Hh, Ww = frame.shape[:2]
    o = dilate(mask, 1)
    layers = [(o, None, x - 1, y + 1), (o, None, x - 1, y - 1), (mask, rgb, x, y)]
    for m, col, mx, my in layers:
        h, w = m.shape
        x0, y0 = max(0, mx), max(0, my)
        x1, y1 = min(Ww, mx + w), min(Hh, my + h)
        if x1 <= x0 or y1 <= y0:
            continue
        sub = m[y0 - my:y1 - my, x0 - mx:x1 - mx]
        reg = frame[y0:y1, x0:x1]
        c = np.array(outline, float)[None] if col is None else col[y0 - my:y1 - my, x0 - mx:x1 - mx][sub]
        if alpha >= 0.999:
            reg[sub] = np.clip(c, 0, 255).astype(np.uint8)
        else:
            reg[sub] = np.clip(reg[sub] * (1 - alpha) + c * alpha, 0, 255).astype(np.uint8)


def _hash01(*xs):
    v = math.sin(sum(x * k for x, k in zip(xs, (12.9898, 78.233, 37.719, 4.581))) + 0.5) * 43758.5453
    return v - math.floor(v)


def _sand_puff(img, t, tl, x0, x1, ybot, col, key):
    """a small puff of pixel sand kicked out from under a line that just landed (deterministic per line)"""
    u = t - tl
    if not (0 <= u < PUFF_T) or col is None:
        return
    n = 12 + int((x1 - x0) / 7)
    fade = 1 - u / PUFF_T
    for k in range(n):
        r1, r2, r3, r4 = (_hash01(key, k, j) for j in (1, 2, 3, 4))
        px0 = x0 + (x1 - x0) * r1
        side = -1 if px0 < (x0 + x1) / 2 else 1
        vx = side * (14 + 46 * r2)
        vy = -(10 + 34 * r3)
        px = px0 + vx * u
        py = ybot + 1 + vy * u + 150 * u * u
        xi, yi = int(round(px)), int(round(py))
        if not (SAFE[0] <= xi < SAFE[2] - 1 and SAFE[1] <= yi < SAFE[3] - 1):
            continue
        a = min(1.0, fade * (0.75 + 0.45 * r4))
        c = np.array(col, float) * (0.85 + 0.25 * r4)
        sz = 2 if r4 > 0.55 and u < 0.25 else 1
        reg = img[yi:yi + sz, xi:xi + sz]
        reg[:] = np.clip(reg * (1 - a) + c * a, 0, 255).astype(np.uint8)


def _draw_impact(img, t, caps, y0, cx, base, hi, alt, puff):
    for t0, t1, lines in caps:
        if not (t0 <= t < t1):
            continue
        fade = 1 - ss(t1 - 0.12, t1, t)
        scale = caption_scale(lines)
        y = y0
        for k, ln in enumerate(lines):
            pl = _plain(ln)
            ts = t0 + k * IMPACT_STAGGER              # this line starts falling
            tl = ts + IMPACT_FALL                     # ... and lands
            lh = 7 * scale
            if t >= ts:
                m, rgb = _line_ink(ln, scale, base, hi, alt)
                h, w = m.shape
                if t < tl:                            # falling: accelerating drop, fading in
                    u = (t - ts) / IMPACT_FALL
                    dy = -IMPACT_DROP * (1 - u * u)
                    a = min(1.0, 0.25 + u) * fade
                    p, sx, sy = 1.0, 0, 0
                else:
                    u = t - tl
                    dy, a = 0.0, fade
                    p = 1 + (IMPACT_PUNCH - 1) * max(0.0, 1 - u / IMPACT_PUNCH_T) ** 2
                    p = min(p, (SAFE[2] - SAFE[0] - 4) / max(w, 1))       # the punch never leaves the safe box
                    p = max(1.0, p)
                    amp = IMPACT_SHAKE * max(0.0, 1 - u / IMPACT_SHAKE_T)
                    sx = int(round(amp * math.sin(u * 2 * math.pi * 31 + k)))
                    sy = int(round(amp * 0.6 * math.sin(u * 2 * math.pi * 23 + 1.7 + k)))
                if p > 1.0:
                    hh, ww = int(round(h * p)), int(round(w * p))
                    m, rgb = _resample(m, hh, ww), _resample(rgb, hh, ww)
                else:
                    hh, ww = h, w
                x = int(round(cx - ww / 2)) + sx
                yy = int(round(y + lh / 2 - hh / 2 + dy)) + sy
                yy = max(yy, SAFE[1])
                _stamp_rgb(img, m, rgb, x, yy, alpha=a)
                _note(x, yy, x + ww, yy + hh, 'caption', pl)
                w0 = len(pl) * ADV * scale - scale
                xl = int(round(cx - w0 / 2))
                _sand_puff(img, t, tl, xl, xl + w0, y + lh, puff, hash(pl) % 997 + 13 * k + int(t0 * 10))
            y += lh + 7


def panel(draw, x0, y0, x1, y1, fill=(16, 20, 38), edge=(62, 74, 112)):
    """HUD box with a drop shadow (draw on the PIL image after post, before text). Keep it in config.SAFE."""
    draw.rectangle([x0 + 2, y0 + 2, x1 + 2, y1 + 2], fill=(6, 8, 18))
    draw.rectangle([x0, y0, x1, y1], fill=fill)
    draw.rectangle([x0, y0, x1, y1], outline=edge)
    draw.line([x0 + 1, y0 + 1, x1 - 1, y0 + 1], fill=(40, 50, 84))
    _note(x0, y0, x1 + 3, y1 + 3, 'panel')


def slide_offset(t, t_in, t_out, dist=160, dur=0.35):
    """panel slide-in/out offset in px (0 = fully on screen)"""
    a = min(ease((t - t_in) / dur), 1 - ease((t - t_out) / dur))
    return int(round((1 - a) * dist))


def label(img, text, x, y, col, scale=2, outline=(10, 20, 26)):
    m = text_mask(text, scale)
    stamp(img, m, int(x), int(y), col, outline=outline)
    _note(int(x), int(y), int(x) + m.shape[1], int(y) + m.shape[0], 'label', text)
    return m.shape[1]


# ------------------------------------------------------------------ share bar with named leader lines
def share_layout(names, x0, x1, scale=2):
    """fixed name slots under a share bar: each name gets its own width, spread evenly across x0..x1.
    Returns (scale, [(slot_x0, slot_x1, centre)]); drops to scale 1 if the names don't fit at `scale`."""
    for sc in (scale, 1):
        ws = [len(n) * ADV * sc - sc for n in names]
        free = (x1 - x0) - sum(ws)
        if free >= 4 * (len(names) - 1) or sc == 1:
            gap = free / max(1, len(names) - 1) if len(names) > 1 else 0
            x, out = float(x0), []
            if len(names) == 1:
                x = (x0 + x1 - ws[0]) / 2
            for w in ws:
                out.append((int(round(x)), int(round(x)) + w, x + w / 2))
                x += w + gap
            return sc, out
    raise ValueError('names do not fit')


def _line(img, a, b, col, alpha=1.0):
    """1 px line (Bresenham-ish) between two points"""
    (xa, ya), (xb, yb) = a, b
    n = int(max(abs(xb - xa), abs(yb - ya))) + 1
    xs = np.round(np.linspace(xa, xb, n)).astype(int)
    ys = np.round(np.linspace(ya, yb, n)).astype(int)
    ok = (xs >= 0) & (xs < img.shape[1]) & (ys >= 0) & (ys < img.shape[0])
    xs, ys = xs[ok], ys[ok]
    img[ys, xs] = np.clip(img[ys, xs] * (1 - alpha) + np.array(col, float) * alpha, 0, 255).astype(np.uint8)


def share_bar(img, shares, names, colors, x0=None, x1=None, y=None, h=6, out=None, alpha=1.0, name_scale=2,
              backing=True, show_pct=True, ink=INK, lead=15):
    """A battle scoreboard: a thin bar of coloured segments sized by each team's share, with every team's name in
    a FIXED slot under it and a thin leader line from the name up to the centre of its colour segment.
    The lines slide as segments grow and shrink, but the names never move or collide. A team that's out
    (out[q] True): its segment closes, its line disappears, its name dims with a small "OUT".

        shares  per-team share (any positive scale; normalised here). Pass smoothed values for gliding segments.
        names, colors  per team, in bar order (left to right)
        x0, x1, y   bar extent and top (default: the width of the safe box, near its bottom edge)
        lead        px between the bar and the names (room for the leader lines to slant readably)
        alpha       fade the whole piece in/out (fade, don't slide: sliding crosses the safe-box edge)
    Text sits inside config.SAFE by default; everything is recorded for `python -m reelkit.safezone`.
    Returns {'segments': [(xa, xb, xc)], 'slots': [...], 'bottom': y} so a project can add a crown etc."""
    n = len(names)
    x0 = SAFE[0] + 4 if x0 is None else x0
    x1 = SAFE[2] - 4 if x1 is None else x1
    sc, slots = share_layout(names, x0, x1, name_scale)
    name_h = 7 * sc
    y = (SAFE[3] - 4 - (h + lead + name_h + (9 if show_pct else 0))) if y is None else y
    out = [False] * n if out is None else list(out)
    under = img.copy() if alpha < 0.999 else None
    sh = np.array([0.0 if out[q] else max(0.0, float(shares[q])) for q in range(n)])
    tot = sh.sum()
    frac = sh / tot if tot > 0 else np.full(n, 1.0 / n)
    yb = y + h
    ny = yb + lead                                 # top of the names
    bottom = ny + name_h + (9 if show_pct else 0)
    if backing:
        bx0, by0, bx1, by1 = x0 - 4, y - 4, x1 + 4, bottom + 2
        reg = img[by0:by1, bx0:bx1].astype(np.float32)
        img[by0:by1, bx0:bx1] = (reg * 0.45 + np.array(ink, float) * 0.25).astype(np.uint8)
        edge = np.array([62, 74, 112], float)
        img[by0, bx0:bx1] = (img[by0, bx0:bx1] * 0.4 + edge * 0.6).astype(np.uint8)
        img[by1 - 1, bx0:bx1] = (img[by1 - 1, bx0:bx1] * 0.6 + edge * 0.4).astype(np.uint8)
        _note(bx0, by0, bx1, by1, 'panel', 'share bar')
    # segments
    segs, x = [], float(x0)
    img[y - 1:yb + 1, x0 - 1:x1 + 1] = np.array(ink, np.uint8)
    for q in range(n):
        w = (x1 - x0) * frac[q]
        xa, xb = x, x + w
        ia, ib = int(round(xa)), int(round(xb)) if q < n - 1 else x1
        if ib > ia:
            c = np.array(colors[q], float)
            img[y:yb, ia:ib] = np.clip(c, 0, 255).astype(np.uint8)
            img[y:y + 1, ia:ib] = np.clip(c * 0.6 + 100, 0, 255).astype(np.uint8)
            img[yb - 1:yb, ia:ib] = np.clip(c * 0.7, 0, 255).astype(np.uint8)
        segs.append((xa, xb, (xa + xb) / 2))
        x = xb
    # leader lines (team order on both ends, so they never cross), then names + shares
    for q in range(n):
        sx0, sx1, scx = slots[q]
        c = np.array(colors[q], float)
        if not out[q] and frac[q] > 0:
            _line(img, (segs[q][2], yb + 1), (scx, ny - 2), c * 0.85 + 20, 0.9)
            img[yb:yb + 2, int(round(segs[q][2]))] = np.clip(c * 0.6 + 100, 0, 255).astype(np.uint8)
        col = tuple(int(v) for v in np.clip(c * 0.32 + 38, 0, 255)) if out[q] else tuple(int(v) for v in np.clip(c, 0, 255))
        label(img, names[q], sx0, ny, col, sc)
        if show_pct:
            s = 'OUT' if out[q] else f'{int(round(100 * frac[q]))}%'
            w = len(s) * ADV - 1
            label(img, s, int(round(scx - w / 2)), ny + name_h + 2, (150, 150, 170) if out[q] else CREAM, 1)
    _note(x0 - 1, y - 1, x1 + 1, yb + 1, 'panel', 'bar')
    if under is not None:
        img[:] = (under * (1 - alpha) + img * alpha).astype(np.uint8)
    return {'segments': segs, 'slots': slots, 'bottom': bottom, 'scale': sc}


# ------------------------------------------------------------------ race legend: one slim lane per racer, live rank
PLACE_TXT = ['1ST', '2ND', '3RD', '4TH', '5TH', '6TH']


def race_lanes(img, lanes, x0=None, x1=None, y1=None, lane_h=11, alpha=1.0, ink=INK, backing=True, ranks=True,
               name_w=6, sub_w=7):
    """A race legend (built for #23 Maze Race): one slim lane per racer, stacked by live rank, each with the racer's
    colour chip, NAME (in its colour), a sub-label (e.g. its instrument), a progress bar and a finishing place.

        lanes  one dict per racer (any fixed order):
               name, sub, color          text + colour
               progress                  0..1 (bar fill)
               slot                      float row position, 0 = top; ease it between ranks so lanes SLIDE when
                                         the ranking changes (the rank numbers on the left stay put)
               place                     None, or 1.. once finished: a '1ST' / '2ND' badge replaces the bar's end
               flash                     0..1 lane highlight in the racer's colour (e.g. on finishing)
               lost                      True: dimmed, 'LOST' badge
               lead                      True: a small arrow by the name (current leader)
               alpha                     optional per-lane fade (e.g. a lane appearing as a label lands in it)
        x0, x1, y1  extent (default: the safe box width, sitting on the safe box's bottom edge)
    Text is scale 1 (7 px); the default 4 lanes x 11 px fit in ~50 px. Everything is recorded for
    `python -m reelkit.safezone`. Returns [(lane_x0, lane_y0, lane_x1, lane_y1)] per racer, in input order."""
    n = len(lanes)
    x0 = SAFE[0] + 4 if x0 is None else x0
    x1 = SAFE[2] - 4 if x1 is None else x1
    y1 = SAFE[3] - 5 if y1 is None else y1
    pad = 3
    y0 = y1 - n * lane_h - 2 * pad
    under = img.copy() if alpha < 0.999 else None
    if backing:
        reg = img[y0:y1, x0 - 3:x1 + 3].astype(np.float32)
        img[y0:y1, x0 - 3:x1 + 3] = (reg * 0.42 + np.array(ink, float) * 0.3).astype(np.uint8)
        edge = np.array([62, 74, 112], float)
        img[y0, x0 - 3:x1 + 3] = (img[y0, x0 - 3:x1 + 3] * 0.4 + edge * 0.6).astype(np.uint8)
        img[y1 - 1, x0 - 3:x1 + 3] = (img[y1 - 1, x0 - 3:x1 + 3] * 0.6 + edge * 0.4).astype(np.uint8)
        _note(x0 - 3, y0, x1 + 3, y1, 'panel', 'race lanes')
    if ranks:
        for k in range(n):
            label(img, str(k + 1), x0, y0 + pad + k * lane_h + 2, (150, 160, 190), 1)
    xc = x0 + 9 if ranks else x0
    xn = xc + 6
    xs = xn + name_w * ADV + 2
    xb0 = xs + sub_w * ADV + 2
    xb1 = x1 - 27
    out = [None] * n
    for i in sorted(range(n), key=lambda i: -lanes[i]['slot']):            # bottom lanes first, top lanes on top
        L = lanes[i]
        a_l = float(L.get('alpha', 1.0))
        if a_l <= 0.01:
            continue
        ly = int(round(y0 + pad + L['slot'] * lane_h))
        col = np.array(L['color'], float)
        lost = bool(L.get('lost', False))
        tc = col * 0.4 + 40 if lost else col
        before = img.copy() if a_l < 0.999 else None
        fl = float(L.get('flash', 0.0))
        if fl > 0:
            reg = img[ly:ly + lane_h - 1, x0 - 2:x1 + 2].astype(np.float32)
            img[ly:ly + lane_h - 1, x0 - 2:x1 + 2] = np.clip(reg * (1 - 0.55 * fl) + col * 0.55 * fl, 0, 255).astype(np.uint8)
        img[ly + 2:ly + 9, xc:xc + 4] = np.clip(tc, 0, 255).astype(np.uint8)
        img[ly + 2, xc:xc + 4] = np.clip(tc * 0.6 + 100, 0, 255).astype(np.uint8)
        label(img, L['name'], xn, ly + 2, tuple(int(v) for v in np.clip(tc, 0, 255)), 1)
        label(img, L.get('sub', ''), xs, ly + 2, (120, 120, 136) if lost else (196, 190, 176), 1)
        # progress bar
        by0, by1 = ly + 3, ly + 8
        img[by0 - 1:by1 + 1, xb0 - 1:xb1 + 1] = np.array(ink, np.uint8)
        img[by0:by1, xb0:xb1] = np.array([34, 38, 58], np.uint8)
        pw = int(round((xb1 - xb0) * min(1.0, max(0.0, float(L.get('progress', 0.0))))))
        if pw > 0:
            img[by0:by1, xb0:xb0 + pw] = np.clip(tc, 0, 255).astype(np.uint8)
            img[by0:by0 + 1, xb0:xb0 + pw] = np.clip(tc * 0.6 + 100, 0, 255).astype(np.uint8)
            img[by0:by1, xb0 + pw - 1:xb0 + pw] = np.clip(tc * 0.5 + 128, 0, 255).astype(np.uint8)
        img[by0 - 1:by1 + 1, xb1:xb1 + 1] = np.array([230, 200, 120], np.uint8)        # the goal post
        _note(xb0 - 1, by0 - 1, xb1 + 1, by1 + 1, 'panel', 'lane ' + L['name'])
        if L.get('place'):
            s = PLACE_TXT[int(L['place']) - 1]
            label(img, s, x1 - 20, ly + 2, (255, 226, 140) if int(L['place']) == 1 else (232, 226, 210), 1)
        elif lost:
            label(img, 'LOST', x1 - 23, ly + 2, (150, 150, 170), 1)
        if L.get('lead') and not lost:
            m = np.array([[1, 0, 0], [1, 1, 0], [1, 1, 1], [1, 1, 0], [1, 0, 0]], bool)       # a small arrow
            yy, xx = np.nonzero(m)
            img[ly + 3 + yy, xn - 5 + xx] = np.array([255, 236, 170], np.uint8)
        if before is not None:
            img[:] = (before * (1 - a_l) + img * a_l).astype(np.uint8)
        out[i] = (x0, ly, x1, ly + lane_h)
    if under is not None:
        img[:] = (under * (1 - alpha) + img * alpha).astype(np.uint8)
    return out


# ------------------------------------------------------------------ roster grid ("Who will win?" elimination formats)
def roster_layout(names, x0, x1, cols=4, scale=1, chip=11, gap=2):
    """column x positions for roster_grid: each column is as wide as its longest name (+ the chip slot), and the
    spare width is spread between columns. Returns [(chip_x, name_x)] per column."""
    rows = [names[i:i + cols] for i in range(0, len(names), cols)]
    widths = []
    for c in range(cols):
        col = [r[c] for r in rows if c < len(r)]
        widths.append(chip + gap + max(len(n) for n in col) * ADV * scale - scale)
    spare = (x1 - x0) - sum(widths)
    if spare < 0:
        raise ValueError('roster names do not fit: shorten them or drop to fewer columns')
    sp = spare / max(1, cols - 1)
    out, x = [], float(x0)
    for w in widths:
        out.append((int(round(x)), int(round(x)) + chip + gap))
        x += w + sp
    return out


def roster_grid(img, names, colors, place=None, x0=None, x1=None, y0=None, cols=4, flash=None, winner=None,
                alpha=1.0, ink=INK, chip=11, row_h=10):
    """Scoreboard for elimination formats: every entrant's name in a cols-wide grid (row-major, e.g. 4 x 4 for 16).
    Survivors: a small chip in their colour + the name lit. Knocked out (place[i] = finishing place): the name dims
    and is struck through, and the chip slot shows the place ("16", "15" ...). winner (index): gold chip "1", gold
    name and a gold frame. flash[i] in 0..1 brightens a cell for a moment (just knocked out, crowned).
    Defaults sit at the bottom of the safe box; every name and number goes through label() so
    `python -m reelkit.safezone` checks it. Fade with alpha, don't slide. Returns the panel's top y."""
    n = len(names)
    x0 = SAFE[0] + 4 if x0 is None else x0
    x1 = SAFE[2] - 4 if x1 is None else x1
    nrows = (n + cols - 1) // cols
    h = nrows * row_h - (row_h - 7)
    y0 = (SAFE[3] - 4 - h - 3) if y0 is None else y0
    place = [None] * n if place is None else list(place)
    flash = [0.0] * n if flash is None else list(flash)
    under = img.copy() if alpha < 0.999 else None
    lay = roster_layout(names, x0, x1, cols, 1, chip)
    bx0, by0, bx1, by1 = x0 - 4, y0 - 4, x1 + 4, y0 + h + 4
    reg = img[by0:by1, bx0:bx1].astype(np.float32)
    img[by0:by1, bx0:bx1] = (reg * 0.42 + np.array(ink, float) * 0.3).astype(np.uint8)
    edge = np.array([62, 74, 112], float)
    img[by0, bx0:bx1] = (img[by0, bx0:bx1] * 0.4 + edge * 0.6).astype(np.uint8)
    img[by1 - 1, bx0:bx1] = (img[by1 - 1, bx0:bx1] * 0.6 + edge * 0.4).astype(np.uint8)
    _note(bx0, by0, bx1, by1, 'panel', 'roster')
    gold = np.array(AMBER, float)
    for i, nm in enumerate(names):
        r, c = divmod(i, cols)
        cx, nx = lay[c]
        y = y0 + r * row_h
        f = float(np.clip(flash[i], 0, 1))
        col = np.array(colors[i], float)
        if winner == i:
            w = len(nm) * ADV - 1
            img[y - 2, cx - 2:nx + w + 2] = gold.astype(np.uint8)
            img[y + 8, cx - 2:nx + w + 2] = gold.astype(np.uint8)
            img[y - 2:y + 9, cx - 2] = gold.astype(np.uint8)
            img[y - 2:y + 9, nx + w + 1] = gold.astype(np.uint8)
            label(img, '1', cx + 3, y, tuple(int(v) for v in gold), 1)
            label(img, nm, nx, y, tuple(int(v) for v in np.clip(gold * (1 - f) + 255 * f, 0, 255)), 1)
        elif place[i] is None:
            img[y:y + 7, cx + 2:cx + 9] = np.array(ink, np.uint8)
            img[y + 1:y + 6, cx + 3:cx + 8] = np.clip(col, 0, 255).astype(np.uint8)
            img[y + 1, cx + 3:cx + 8] = np.clip(col * 0.6 + 100, 0, 255).astype(np.uint8)
            label(img, nm, nx, y, tuple(int(v) for v in np.clip(np.array(CREAM) * (1 - f) + 255 * f, 0, 255)), 1)
        else:
            dimc = np.clip(col * 0.25 + 70 + f * 150, 0, 255)
            s = str(place[i])
            label(img, s, cx + (chip - (len(s) * ADV - 1)) // 2, y, (150, 146, 160) if f < 0.5 else (255, 240, 220), 1)
            w = label(img, nm, nx, y, tuple(int(v) for v in dimc), 1)
            img[y + 3, nx - 1:nx + w + 1] = np.clip(dimc * 0.9, 0, 255).astype(np.uint8)
    if under is not None:
        img[:] = (under * (1 - alpha) + img * alpha).astype(np.uint8)
    return by0
