"""Shared effects kit (BALLFIGHT v2 redesign, user Oct 24; usable by any reel).

Everything is deterministic: build the effects once from the run's events (in time order), then ask for the state at
any time t, so frames render in any order, in parallel, and patch renders match the full render. Everything is local
and flash-safe: no full-screen flash (user rule); bright bursts stay small and at most 3 per second (FlashBudget);
the ability dim is eased and <= 30 %. `python -m reelkit.flashcheck out.mp4` must still pass on the finished video.

    from reelkit import fx
    FL = fx.Flares()                      # attacks light the scene: short point lights in the fighter's colour
    FL.add(t, pos_world, rgb, radius=2.6, peak=1.2)
    FL.apply(L, t)                        # -> L.points (canvas / floor passes + shade() pick them up: ropes, posts...)
    FL.tint(P, t)                         # light reaching points P (crowd sprites, anything not shaded)

    BU = fx.Bursts()                      # 1-2 frame ink-starburst impact at the hit point (bright ones budgeted)
    BU.add(t, pos_world, rgb, strength)
    BU.draw(img, cam, t)

    SH = fx.Shake(cap=2.5)                # a little camera shake on heavy hits
    SH.add(t, px);  dx, dy = SH.offset(t)

    fx.shimmer_world(img, cam, c, r, width, amp, t)    # heat-shimmer ripple on a shockwave / blast front (local)
    fx.fringe(img, x, y, r, px)                         # chromatic fringing on a big hit (local)

    PA = fx.Particles(seed)               # particles with trails: PRESETS (dust, motes, steam, smoke, shreds, ...)
    PA.emit(t, pos_world, 'dust', n=8, dir=(0, 1, 0), vel=ball_velocity)
    PA.draw(img, cam, t)

    MK = fx.Marks()                       # marks that fade on a floor: scorch, claw, crack, water, smoke_ring, dust, glass
    MK.add(t, 'scorch', u, v, size=0.5)   # (u, v): the caller's floor coords (e.g. the ring frame)
    alb = MK.apply(alb, U, V, t)          # per-pixel, inside the floor pass

    MO = fx.Moments()                     # ability moments: the background dims (<= 30 %, eased) + speed lines
    MO.add(t, key, rgb)
    img = MO.apply_dim(img, t, keep=[(x, y, r)])
    MO.speed_lines(img, t, key, x, y, r, (dx, dy))

Images are float arrays (H, W, 3) in 0..255 at the internal resolution (reelkit.config W x H); functions return the
modified array. World positions are reelkit world coords (y up); `cam` is a reelkit.camera.Cam."""
import bisect
import math
import numpy as np
from PIL import Image, ImageDraw
from .config import W, H, FPS, BAYER_FULL


# ---------------------------------------------------------------- timing helpers
def env_ad(u, attack=0.03, decay=0.25, hold=0.0):
    """0 before u = 0, eases up over `attack`, holds `hold`, then decays exponentially with time constant `decay`"""
    if u < 0:
        return 0.0
    if u < attack:
        x = u / attack
        return x * x * (3 - 2 * x)
    u2 = u - attack - hold
    return 1.0 if u2 < 0 else math.exp(-u2 / decay)


def _smooth(x):
    x = min(1.0, max(0.0, x))
    return x * x * (3 - 2 * x)


def _norm_rgb(rgb):
    c = np.asarray(rgb, float)
    return c / max(1.0, float(c.max()))


class _Timeline:
    """events sorted by start time with a fixed look-back: query the ones alive at t quickly"""

    def __init__(self, span):
        self.span = span
        self.t0 = []
        self.items = []
        self._dirty = False

    def add(self, t0, item):
        self.t0.append(float(t0))
        self.items.append(item)
        self._dirty = True

    def _sort(self):
        if self._dirty:
            o = sorted(range(len(self.t0)), key=lambda i: (self.t0[i], i))
            self.t0 = [self.t0[i] for i in o]
            self.items = [self.items[i] for i in o]
            self._dirty = False

    def alive(self, t, span=None):
        self._sort()
        span = self.span if span is None else span
        i0 = bisect.bisect_left(self.t0, t - span)
        i1 = bisect.bisect_right(self.t0, t)
        return [(self.t0[i], self.items[i]) for i in range(i0, i1)]


# ---------------------------------------------------------------- flash budget
class FlashBudget:
    """which bright events may stay bright: greedy in time order, an event is bright only if fewer than `max_per_s`
    bright ones started in the preceding `window` s, so any 1 s holds at most `max_per_s` (the flash rule's limit).
    Call `assign(times, priority)` once with every candidate; returns a list of booleans."""

    def __init__(self, max_per_s=3, window=1.0):
        self.max = max_per_s
        self.window = window

    def assign(self, times, priority=None):
        n = len(times)
        pr = priority if priority is not None else [0.0] * n
        order = sorted(range(n), key=lambda i: (round(times[i], 4), -pr[i], i))
        ok = [False] * n
        acc = []
        for i in order:
            t = times[i]
            while acc and acc[0] <= t - self.window:
                acc.pop(0)
            if len(acc) < self.max:
                ok[i] = True
                acc.append(t)
        return ok


# ---------------------------------------------------------------- 1. attacks light the scene
class Flares:
    """short-lived point lights (an attack, an impact, a pipe speaking): each one rises over `attack` s, holds `hold` s
    and decays with `decay`; `apply(L, t)` adds the strongest `max_n` live ones to a reelkit.lighting.Lights, so every
    pass that loops over L.points (the per-pixel canvas / floor passes, shade() on ropes, posts, stands) is lit by them"""

    def __init__(self, max_n=6, span=3.0):
        self.tl = _Timeline(span)
        self.max_n = max_n

    def add(self, t0, pos, rgb, radius=2.6, peak=1.0, attack=0.03, decay=0.25, hold=0.0):
        self.tl.add(t0, (np.asarray(pos, float), _norm_rgb(rgb) * peak, float(radius), attack, decay, hold))

    def at(self, t):
        out = []
        for t0, (p, c, r, a, d, h) in self.tl.alive(t):
            k = env_ad(t - t0, a, d, h)
            if k > 0.02:
                out.append((p, c * k, r))
        out.sort(key=lambda q: -float(q[1].max()))
        return out[:self.max_n]

    def apply(self, L, t):
        for p, c, r in self.at(t):
            L.point(p, c, r)
        return L

    def tint(self, P, t, gain=1.0):
        """(N, 3) RGB light the live flares cast on points P (N, 3) (falloff 1 / (1 + (d / R)^2)), for anything that
        isn't shaded through Lights: crowd sprites, particles, UI glows"""
        P = np.atleast_2d(np.asarray(P, float))
        out = np.zeros((len(P), 3))
        for p, c, r in self.at(t):
            d = np.linalg.norm(P - p[None], axis=1)
            out += (1.0 / (1.0 + (d / r) ** 2))[:, None] * c[None]
        return out * gain


# ---------------------------------------------------------------- 2. impact bursts (ink starbursts, 1-2 frames)
INK = np.array([18, 10, 26], float)


def _blend_patch(img, patch, x0, y0):
    """alpha-composite an RGBA uint8 patch onto the float image at (x0, y0)"""
    h, w = patch.shape[:2]
    xa, ya = max(0, x0), max(0, y0)
    xb, yb = min(W, x0 + w), min(H, y0 + h)
    if xb <= xa or yb <= ya:
        return img
    sub = patch[ya - y0:yb - y0, xa - x0:xb - x0].astype(np.float32)
    a = sub[..., 3:4] / 255.0
    img[ya:yb, xa:xb] = img[ya:yb, xa:xb] * (1 - a) + sub[..., :3] * a
    return img


def starburst(img, x, y, r, rgb, k=1.0, spikes=9, seed=0, hollow=False, bright=True):
    """an ink-outlined spiky star centred at (x, y), radius r px: a hot core in the colour (bright) or a darker ink
    star (not bright: for hits past the flash budget). hollow: outline + a few spikes only (the second frame)"""
    rng = np.random.default_rng(seed)
    n = 2 * spikes
    rot = rng.uniform(0, 2 * math.pi)
    rad = np.where(np.arange(n) % 2 == 0, r * rng.uniform(0.75, 1.0, n), r * rng.uniform(0.32, 0.45, n))
    ang = rot + np.arange(n) * 2 * math.pi / n + rng.uniform(-0.12, 0.12, n)
    m = int(math.ceil(r)) + 3
    pts = [(m + float(rr * math.cos(a)), m + float(rr * math.sin(a))) for rr, a in zip(rad, ang)]
    im = Image.new('RGBA', (2 * m + 1, 2 * m + 1), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    c = np.asarray(rgb, float)
    a = int(255 * min(1.0, k))
    if hollow:
        d.polygon(pts, outline=tuple(int(v) for v in np.clip(c * 0.6 + 100, 0, 255)) + (a,))
    else:
        core = np.clip(c * 0.35 + 190, 0, 255) if bright else np.clip(c * 0.55 + 30, 0, 255)
        d.polygon(pts, fill=tuple(int(v) for v in core) + (a,), outline=tuple(int(v) for v in INK) + (a,))
        inner = [(m + (px - m) * 0.45, m + (py - m) * 0.45) for px, py in pts]
        hot = (255, 252, 240) if bright else tuple(int(v) for v in np.clip(c * 0.8 + 60, 0, 255))
        d.polygon(inner, fill=hot + (a,))
    return _blend_patch(img, np.asarray(im), int(round(x)) - m, int(round(y)) - m)


class Bursts:
    """impact bursts: frame 0 a filled starburst, frame 1 a larger hollow one, then gone. Bright cores are budgeted
    (FlashBudget: at most `max_per_s` per second); the rest draw as dark ink stars, so dense hits never strobe"""

    def __init__(self, max_per_s=3, base_px=7.0, per_strength=9.0, frames=2):
        self.tl = _Timeline(frames / FPS + 1e-6)
        self.budget = FlashBudget(max_per_s)
        self.base, self.per = base_px, per_strength
        self.frames = frames
        self._assigned = False
        self._ev = []

    def add(self, t0, pos, rgb, strength=1.0, seed=None, bright=True):
        """strength 0..2 (size); bright=False: never a bright core (light taps)"""
        e = dict(pos=np.asarray(pos, float), rgb=np.asarray(rgb, float), s=float(strength), bright=bright,
                 seed=int(seed if seed is not None else len(self._ev) * 7919 + 13))
        self._ev.append((float(t0), e))
        self.tl.add(t0, e)
        self._assigned = False

    def _assign(self):
        if self._assigned:
            return
        cand = [(t, e) for t, e in self._ev if e['bright']]
        ok = self.budget.assign([t for t, e in cand], [e['s'] for t, e in cand])
        for (t, e), o in zip(cand, ok):
            e['lit'] = o
        for t, e in self._ev:
            e.setdefault('lit', False)
        self._assigned = True

    def draw(self, img, cam, t):
        self._assign()
        for t0, e in self.tl.alive(t):
            fi = int((t - t0) * FPS + 1e-6)
            if fi >= self.frames:
                continue
            s = cam.wp(e['pos'][None])[0]
            r = self.base + self.per * e['s']
            if fi == 0:
                img = starburst(img, s[0], s[1], r, e['rgb'], 1.0, seed=e['seed'], bright=e['lit'])
            else:
                img = starburst(img, s[0], s[1], r * 1.3, e['rgb'], 0.75, seed=e['seed'] + 1, hollow=True)
        return img

    def bright_times(self):
        self._assign()
        return [t for t, e in self._ev if e['lit']]


# ---------------------------------------------------------------- 3. camera shake
class Shake:
    """a little camera shake on heavy hits: each add() is a decaying jolt of `px` screen pixels (internal res); the sum
    is capped at `cap` px. Add the offset to cam.cx / cam.cy."""

    def __init__(self, cap=2.5, decay=0.09, span=0.45):
        self.tl = _Timeline(span)
        self.cap, self.decay = cap, decay

    def add(self, t0, px):
        self.tl.add(t0, float(px))

    def offset(self, t):
        a = 0.0
        for t0, px in self.tl.alive(t):
            a = max(a, px * math.exp(-(t - t0) / self.decay))
        a = min(a, self.cap)
        return a * math.sin(t * 2 * math.pi * 23), a * 0.7 * math.cos(t * 2 * math.pi * 29)


# ---------------------------------------------------------------- 4. heat shimmer + 5. chromatic fringing (local warps)
def _bbox(cx, cy, rx, ry):
    x0, x1 = max(0, int(cx - rx) - 1), min(W, int(cx + rx) + 2)
    y0, y1 = max(0, int(cy - ry) - 1), min(H, int(cy + ry) + 2)
    return x0, x1, y0, y1


def shimmer(img, cx, cy, ax, ay, width, amp, t=0.0, k=1.0, seed=0):
    """heat-shimmer ripple on an elliptical front (screen centre cx, cy; semi-axes ax, ay px): pixels within `width` px of
    the front are pushed in / out along the radius by up to `amp` px with a wobbling, travelling profile, like air
    over a blast. Local: only the front's bounding box is touched"""
    if k <= 0.01 or amp * k < 0.4:
        return img
    x0, x1, y0, y1 = _bbox(cx, cy, ax + width + amp, ay + width + amp)
    if x1 <= x0 or y1 <= y0:
        return img
    yy, xx = np.mgrid[y0:y1, x0:x1].astype(np.float32)
    dx, dy = xx - cx, yy - cy
    rho = np.sqrt((dx / max(ax, 1.0)) ** 2 + (dy / max(ay, 1.0)) ** 2) + 1e-6
    dist = (rho - 1.0) * 0.5 * (ax + ay)                       # signed px from the front
    prof = np.cos(np.clip(dist / width, -1, 1) * math.pi / 2) ** 2
    ang = np.arctan2(dy, dx)
    wob = 1.0 + 0.45 * np.sin(ang * 9 + t * 31 + seed) * np.sin(ang * 4 - t * 17)
    disp = amp * k * prof * wob * np.sin(np.clip(dist / width, -1, 1) * math.pi)
    sx = np.clip(np.round(xx - np.cos(ang) * disp), 0, W - 1).astype(int)
    sy = np.clip(np.round(yy - np.sin(ang) * disp), 0, H - 1).astype(int)
    m = prof > 0.02
    reg = img[y0:y1, x0:x1].copy()
    reg[m] = img[sy[m], sx[m]]
    img[y0:y1, x0:x1] = reg
    return img


def ellipse_of(cam, c, r):
    """screen centre + semi-axes of a horizontal world circle (centre c, radius r)"""
    c = np.asarray(c, float)
    P = np.array([c + [r, 0, 0], c - [r, 0, 0], c + [0, 0, r], c - [0, 0, r], c + [r * 0.7071, 0, r * 0.7071],
                  c - [r * 0.7071, 0, r * 0.7071], c + [r * 0.7071, 0, -r * 0.7071], c - [r * 0.7071, 0, -r * 0.7071]])
    s = cam.wp(P)
    cx, cy = (s[:, 0].max() + s[:, 0].min()) / 2, (s[:, 1].max() + s[:, 1].min()) / 2
    return cx, cy, (s[:, 0].max() - s[:, 0].min()) / 2, (s[:, 1].max() - s[:, 1].min()) / 2


def shimmer_world(img, cam, c, r, width, amp, t=0.0, k=1.0, seed=0):
    """shimmer on a world circle on the floor (a shockwave front): width in world units, amp in px"""
    cx, cy, ax, ay = ellipse_of(cam, c, r)
    wpx = max(2.0, width * ax / max(r, 1e-3))
    return shimmer(img, cx, cy, ax, ay, wpx, amp, t, k, seed)


def fringe(img, cx, cy, r, px=2.0, k=1.0):
    """chromatic fringing around a big hit: inside a disc of radius r px, red is sampled from further out and blue from
    further in (by up to `px` px), fading to nothing at the edge. Local"""
    if k <= 0.01:
        return img
    x0, x1, y0, y1 = _bbox(cx, cy, r, r)
    if x1 <= x0 or y1 <= y0:
        return img
    yy, xx = np.mgrid[y0:y1, x0:x1].astype(np.float32)
    dx, dy = xx - cx, yy - cy
    d = np.sqrt(dx * dx + dy * dy) + 1e-6
    f = np.clip(1 - d / r, 0, 1) ** 0.6 * np.clip(d / 3.0, 0, 1) * px * k
    ux, uy = dx / d, dy / d
    rx = np.clip(np.round(xx - ux * f), 0, W - 1).astype(int)
    ry = np.clip(np.round(yy - uy * f), 0, H - 1).astype(int)
    bx = np.clip(np.round(xx + ux * f), 0, W - 1).astype(int)
    by = np.clip(np.round(yy + uy * f), 0, H - 1).astype(int)
    reg = img[y0:y1, x0:x1].copy()
    m = f > 0.3
    reg[..., 0][m] = img[ry[m], rx[m], 0]
    reg[..., 2][m] = img[by[m], bx[m], 2]
    img[y0:y1, x0:x1] = reg
    return img
