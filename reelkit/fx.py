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
