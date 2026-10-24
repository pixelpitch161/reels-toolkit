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


# ---------------------------------------------------------------- 6. particles with trails
# preset fields: n (per emit), speed (lo, hi) m/s, up (vertical bias added to the direction), spread (deg around the
# direction; 180 = all round), g (gravity, + down, - buoyant), drag (1/s), life (lo, hi) s, size (lo, hi) world units,
# grow (size x per s), trail (samples, s between), shape, cols, alpha, wobble (amp m, Hz), twinkle (Hz), ground (settle
# on y = 0), glow (draw a soft halo), spin (rad/s for shards)
PRESETS = {
    # #35
    'dust': dict(n=8, speed=(0.3, 0.9), up=0.8, spread=70, g=-0.25, drag=2.2, life=(0.9, 1.6), size=(0.06, 0.12),
                 grow=1.2, trail=(3, 0.05), shape='blob', cols=[(176, 160, 150), (150, 132, 124), (206, 192, 178)],
                 alpha=0.55, wobble=(0.08, 1.3)),
    'motes': dict(n=6, speed=(0.2, 0.7), up=1.0, spread=80, g=-0.35, drag=1.6, life=(1.0, 1.8), size=(0.018, 0.03),
                  grow=0.0, trail=(5, 0.04), shape='dot', cols=[(255, 246, 222), (255, 230, 170), (230, 236, 255)],
                  alpha=0.95, wobble=(0.12, 1.8), twinkle=7.0, glow=True),
    # #29-#34
    'steam': dict(n=7, speed=(0.8, 1.8), up=1.2, spread=30, g=-0.6, drag=2.6, life=(0.6, 1.2), size=(0.06, 0.1),
                  grow=1.8, trail=(3, 0.05), shape='blob', cols=[(236, 240, 246), (210, 218, 230)], alpha=0.5,
                  wobble=(0.06, 2.0)),
    'smoke': dict(n=6, speed=(0.2, 0.6), up=0.6, spread=90, g=-0.2, drag=1.8, life=(1.4, 2.4), size=(0.08, 0.14),
                  grow=1.0, trail=(3, 0.08), shape='blob', cols=[(110, 116, 126), (140, 146, 152), (90, 94, 104)],
                  alpha=0.45, wobble=(0.1, 0.9)),
    'shreds': dict(n=10, speed=(1.5, 3.5), up=0.8, spread=180, g=6.0, drag=1.2, life=(0.8, 1.4), size=(0.05, 0.09),
                   trail=(3, 0.03), shape='shard', cols=[(64, 196, 88), (190, 40, 50), (240, 230, 210), (30, 60, 120)],
                   alpha=1.0, ground=True, spin=12.0),
    'puffs': dict(n=6, speed=(1.2, 2.4), up=0.2, spread=50, g=0.0, drag=3.5, life=(0.4, 0.8), size=(0.05, 0.08),
                  grow=2.2, trail=(4, 0.03), shape='blob', cols=[(240, 244, 250)], alpha=0.4),
    'shards': dict(n=8, speed=(1.8, 3.6), up=0.9, spread=180, g=7.0, drag=0.8, life=(0.8, 1.3), size=(0.05, 0.1),
                   trail=(3, 0.03), shape='shard', cols=[(24, 22, 30), (50, 46, 60), (190, 245, 60)], alpha=1.0,
                   ground=True, spin=14.0),
    'sparks': dict(n=10, speed=(2.0, 4.5), up=0.6, spread=180, g=6.5, drag=1.0, life=(0.25, 0.55), size=(0.015, 0.025),
                   trail=(5, 0.02), shape='streak', cols=[(255, 236, 160), (255, 200, 90)], alpha=1.0, glow=True,
                   ground=True),
    'glitch': dict(n=6, speed=(0.4, 1.2), up=0.4, spread=180, g=0.0, drag=3.0, life=(0.2, 0.5), size=(0.06, 0.14),
                   trail=(0, 0.0), shape='square', cols=[(70, 228, 255), (255, 60, 200), (240, 255, 255)],
                   alpha=0.9, twinkle=15.0),
    'streaks': dict(n=8, speed=(2.5, 4.0), up=0.0, spread=15, g=0.0, drag=0.6, life=(0.3, 0.6), size=(0.012, 0.02),
                    trail=(6, 0.025), shape='streak', cols=[(220, 228, 255), (170, 180, 230)], alpha=0.6),
    'scales': dict(n=8, speed=(1.0, 2.2), up=1.0, spread=120, g=5.0, drag=1.4, life=(0.7, 1.2), size=(0.03, 0.05),
                   trail=(2, 0.03), shape='shard', cols=[(222, 232, 246), (180, 196, 220)], alpha=1.0, spin=10.0,
                   twinkle=9.0),
    'glass': dict(n=12, speed=(1.0, 3.0), up=0.8, spread=180, g=6.0, drag=1.2, life=(0.6, 1.1), size=(0.015, 0.03),
                  trail=(3, 0.02), shape='dot', cols=[(230, 244, 255), (200, 220, 255)], alpha=1.0, twinkle=12.0,
                  glow=True, ground=True),
    'foam': dict(n=10, speed=(0.8, 2.0), up=1.2, spread=90, g=4.0, drag=1.8, life=(0.6, 1.2), size=(0.03, 0.06),
                 grow=0.6, trail=(3, 0.03), shape='blob', cols=[(250, 252, 255), (210, 236, 250)], alpha=0.85,
                 ground=True),
    'chips': dict(n=8, speed=(1.5, 3.0), up=0.9, spread=150, g=7.0, drag=1.0, life=(0.6, 1.1), size=(0.03, 0.06),
                  trail=(2, 0.03), shape='shard', cols=[(214, 160, 100), (170, 116, 64), (236, 196, 140)], alpha=1.0,
                  ground=True, spin=15.0),
    'embers': dict(n=8, speed=(0.5, 1.5), up=1.2, spread=100, g=-0.4, drag=1.5, life=(0.8, 1.5), size=(0.015, 0.025),
                   trail=(5, 0.04), shape='dot', cols=[(255, 150, 70), (220, 110, 50), (255, 200, 120)], alpha=1.0,
                   wobble=(0.08, 2.5), twinkle=6.0, glow=True),
    'brass': dict(n=10, speed=(2.0, 4.0), up=0.7, spread=160, g=6.5, drag=1.0, life=(0.3, 0.6), size=(0.015, 0.025),
                  trail=(4, 0.02), shape='streak', cols=[(255, 214, 120), (230, 176, 70)], alpha=1.0, glow=True,
                  ground=True),
    # crossovers (#27 / #28 fighters; their uploaded videos are not re-rendered)
    'smear': dict(n=4, speed=(0.1, 0.3), up=0.0, spread=180, g=0.0, drag=4.0, life=(0.15, 0.3), size=(0.04, 0.07),
                  trail=(6, 0.02), shape='streak', cols=[(255, 140, 120)], alpha=0.6),
    'keyshards': dict(n=6, speed=(1.5, 3.0), up=0.8, spread=180, g=7.0, drag=1.0, life=(0.6, 1.0), size=(0.05, 0.08),
                      trail=(2, 0.03), shape='shard', cols=[(246, 244, 236), (24, 22, 30)], alpha=1.0, ground=True,
                      spin=12.0),
    'ripples': dict(n=1, speed=(0.0, 0.0), up=0.0, spread=0, g=0.0, drag=0.0, life=(0.5, 0.6), size=(0.2, 0.2),
                    grow=6.0, trail=(0, 0.0), shape='ring', cols=[(246, 238, 216)], alpha=0.8),
    'rosin': dict(n=8, speed=(0.3, 0.9), up=0.5, spread=120, g=-0.1, drag=2.0, life=(0.6, 1.2), size=(0.012, 0.02),
                  trail=(6, 0.03), shape='dot', cols=[(250, 240, 210), (230, 210, 170)], alpha=0.8, wobble=(0.1, 2.0)),
}
_SHAPES = ('blob', 'dot', 'shard', 'square', 'streak', 'ring')


class Particles:
    """deterministic particles: emit() appends particles drawn from the preset with this system's seeded RNG (so emit
    in the same order every time, e.g. from the run's event list at load), state at t is analytic (linear drag +
    gravity + a wobble), trails are the same particle sampled a little earlier. draw() projects and paints them."""

    def __init__(self, seed=0, max_alive=600):
        self.rng = np.random.default_rng(seed)
        self.cols = {k: [] for k in ('t0', 'p', 'v', 'life', 'size', 'grow', 'col', 'g', 'drag', 'wa', 'wf', 'ph',
                                      'tw', 'shape', 'alpha', 'ground', 'spin', 'glow', 'tn', 'tdt')}
        self.max_alive = max_alive
        self._arr = None

    def emit(self, t0, pos, kind, n=None, dir=(0.0, 1.0, 0.0), vel=(0.0, 0.0, 0.0), col=None, scale=1.0, speed=None,
             spread=None, life=None):
        """n particles of preset `kind` at world `pos`, sprayed around `dir` (+ the preset's up bias), carried by `vel`
        (e.g. the attacker's motion); col overrides the preset's colours (one rgb or a list)"""
        P = PRESETS[kind]
        n = P['n'] if n is None else int(n)
        if n <= 0:
            return
        r = self.rng
        d0 = np.asarray(dir, float)
        d0 = d0 / (np.linalg.norm(d0) + 1e-9)
        sp = math.radians(P['spread'] if spread is None else spread)
        lo, hi = P['speed'] if speed is None else speed
        cols = P['cols'] if col is None else ([col] if np.ndim(col) == 1 else list(col))
        lf = P['life'] if life is None else life
        for _ in range(n):
            # a random direction within `sp` of d0 (uniform on the cap)
            cz = math.cos(sp) + (1 - math.cos(sp)) * r.uniform()
            phi = r.uniform(0, 2 * math.pi)
            a = np.array([1.0, 0, 0]) if abs(d0[0]) < 0.9 else np.array([0, 0, 1.0])
            e1 = np.cross(d0, a)
            e1 /= np.linalg.norm(e1)
            e2 = np.cross(d0, e1)
            sz = math.sqrt(max(0.0, 1 - cz * cz))
            dv = d0 * cz + (e1 * math.cos(phi) + e2 * math.sin(phi)) * sz
            dv = dv + np.array([0, P['up'], 0]) * r.uniform(0.5, 1.0)
            dv /= np.linalg.norm(dv) + 1e-9
            c = self.cols
            c['t0'].append(float(t0))
            c['p'].append(np.asarray(pos, float) + r.normal(0, 0.03, 3))
            c['v'].append(dv * r.uniform(lo, hi) * scale + np.asarray(vel, float))
            c['life'].append(r.uniform(*lf))
            c['size'].append(r.uniform(*P['size']) * scale)
            c['grow'].append(P.get('grow', 0.0))
            c['col'].append(np.asarray(cols[r.integers(len(cols))], float))
            c['g'].append(P['g'])
            c['drag'].append(max(1e-3, P['drag']))
            wa, wf = P.get('wobble', (0.0, 1.0))
            c['wa'].append(wa)
            c['wf'].append(wf * r.uniform(0.7, 1.3))
            c['ph'].append(r.uniform(0, 2 * math.pi))
            c['tw'].append(P.get('twinkle', 0.0))
            c['shape'].append(_SHAPES.index(P['shape']))
            c['alpha'].append(P['alpha'])
            c['ground'].append(bool(P.get('ground', False)))
            c['spin'].append(P.get('spin', 0.0) * r.uniform(-1, 1))
            c['glow'].append(bool(P.get('glow', False)))
            c['tn'].append(P['trail'][0])
            c['tdt'].append(P['trail'][1])
        self._arr = None

    def _arrays(self):
        if self._arr is None:
            self._arr = {k: np.array(v) for k, v in self.cols.items()}
            o = np.argsort(self._arr['t0'], kind='stable')
            self._arr = {k: v[o] for k, v in self._arr.items()}
            self._lmax = float(self._arr['life'].max()) if len(o) else 0.0
        return self._arr

    @staticmethod
    def _pos(A, idx, u):
        """analytic position of particles idx at age u (array): drag + gravity + wobble + ground settle"""
        k = A['drag'][idx]
        g = np.stack([np.zeros_like(k), -A['g'][idx], np.zeros_like(k)], 1)
        vinf = g / k[:, None]
        e = (1 - np.exp(-k * u))[:, None] / k[:, None]
        p = A['p'][idx] + vinf * u[:, None] + (A['v'][idx] - vinf) * e
        wa = A['wa'][idx]
        if np.any(wa > 0):
            w = 2 * np.pi * A['wf'][idx] * u + A['ph'][idx]
            p = p + np.stack([np.sin(w), 0.5 * np.sin(1.7 * w + 1), np.cos(1.3 * w)], 1) * (wa * np.minimum(1, u * 3))[:, None]
        gr = A['ground'][idx]
        p[:, 1] = np.where(gr, np.maximum(p[:, 1], 0.01), p[:, 1])
        return p

    def alive(self, t):
        A = self._arrays()
        if not len(A['t0']):
            return A, np.zeros(0, int)
        i0 = np.searchsorted(A['t0'], t - self._lmax - 1e-6)
        i1 = np.searchsorted(A['t0'], t, 'right')
        idx = np.arange(i0, i1)
        u = t - A['t0'][idx]
        idx = idx[(u >= 0) & (u < A['life'][idx])]
        return A, idx[-self.max_alive:]

    def draw(self, img, cam, t, light=None):
        """paint the live particles (trails first, then heads) onto img; light: optional (N, 3) multiplier function
        P -> RGB gain (e.g. lambda P: 1 + flares.tint(P, t)) so attack lights colour the particles too"""
        A, idx = self.alive(t)
        if not len(idx):
            return img
        u = t - A['t0'][idx]
        f = u / A['life'][idx]
        fade = np.clip(np.minimum(f / 0.12, (1 - f) / 0.35), 0, 1)
        tw = A['tw'][idx]
        fade = fade * np.where(tw > 0, 0.55 + 0.45 * np.sin(2 * np.pi * tw * u + A['ph'][idx] * 3), 1.0)
        P0 = self._pos(A, idx, u)
        Pc = cam.to_cam(P0)
        ok = Pc[:, 2] > cam.near
        if not ok.any():
            return img
        idx, u, fade, P0, Pc = idx[ok], u[ok], fade[ok], P0[ok], Pc[ok]
        s0 = cam.proj(Pc)
        size = A['size'][idx] * (1 + A['grow'][idx] * u)
        rpx = np.maximum(0.5, size * cam.f / Pc[:, 2])
        col = A['col'][idx]
        if light is not None:
            col = np.clip(col * light(P0), 0, 255)
        alpha = A['alpha'][idx] * fade
        base = Image.fromarray(np.clip(img, 0, 255).astype(np.uint8))
        d = ImageDraw.Draw(base, 'RGBA')
        tn = A['tn'][idx]
        # trails
        for j in np.nonzero(tn > 0)[0]:
            K = int(tn[j])
            us = np.maximum(0.0, u[j] - A['tdt'][idx[j]] * np.arange(K + 1))
            pts = cam.wp(self._pos(A, np.full(K + 1, idx[j]), us))
            for q in range(K):
                a = alpha[j] * (1 - (q + 1) / (K + 1)) * 0.7
                wd = max(1, int(round(min(rpx[j], 2.0) * (1 - q / (K + 1)))))
                d.line([tuple(pts[q]), tuple(pts[q + 1])], fill=tuple(int(v) for v in col[j]) + (int(255 * a),), width=wd)
        # heads
        for j in range(len(idx)):
            x, y = float(s0[j, 0]), float(s0[j, 1])
            if not (-8 < x < W + 8 and -8 < y < H + 8):
                continue
            c = tuple(int(v) for v in col[j])
            a = int(255 * alpha[j])
            r = float(rpx[j])
            sh = _SHAPES[int(A['shape'][idx[j]])]
            if A['glow'][idx[j]] and a > 40:
                gr = max(1.5, r * 2.6)
                d.ellipse([x - gr, y - gr, x + gr, y + gr], fill=c + (int(a * 0.25),))
            if sh == 'dot':
                if r < 1.0:
                    d.point((x, y), fill=c + (a,))
                else:
                    d.ellipse([x - r, y - r, x + r, y + r], fill=c + (a,))
            elif sh == 'blob':
                d.ellipse([x - r, y - r * 0.85, x + r, y + r * 0.85], fill=c + (int(a * 0.8),))
                if r > 2:
                    d.ellipse([x - r * 0.5, y - r * 0.6, x + r * 0.3, y], fill=tuple(min(255, v + 25) for v in c) + (int(a * 0.5),))
            elif sh == 'shard':
                ang = A['spin'][idx[j]] * u[j] + A['ph'][idx[j]]
                pts = [(x + r * 1.4 * math.cos(ang + q * 2.3), y + r * 0.9 * math.sin(ang + q * 2.3)) for q in range(3)]
                d.polygon(pts, fill=c + (a,))
            elif sh == 'square':
                d.rectangle([x - r, y - r * 0.6, x + r, y + r * 0.6], fill=c + (a,))
            elif sh == 'streak':
                d.point((x, y), fill=(255, 255, 240, a))
            elif sh == 'ring':
                d.ellipse([x - r, y - r * 0.55, x + r, y + r * 0.55], outline=c + (a,))
        return np.asarray(base).astype(np.float32)


# ---------------------------------------------------------------- 7. marks that fade on a floor
MARK_KINDS = ('scorch', 'claw', 'crack', 'water', 'smoke_ring', 'dust', 'glass')


class Marks:
    """marks on a floor plane, evaluated per pixel in the caller's floor coordinates (u, v) (e.g. the ring frame):
    a mark fades in over 0.06 s, holds, and fades out over the last 40 % of its `life`.
        scorch      a dark, jagged blot with a warm rim (col)
        claw        n parallel gouges along `angle` (length 2 x size), dark with a light lip
        crack       n jagged rays from the centre
        water       a wet, darker blot with a pale rim
        smoke_ring  a soft dark ring
        dust        a pale powdery blot (col)
        glass       sparkly specks (col)
    apply(alb, U, V, t, dither=None) returns the albedo (N, 3) with the live marks blended in; `dither` (N,) in 0..1
    (e.g. BAYER_FULL at the pixels) quantises the strengths into pixel-art steps."""

    def __init__(self, span=12.0, max_live=40):
        self.tl = _Timeline(span)
        self.max_live = max_live

    def add(self, t0, kind, u, v, size=0.5, angle=0.0, col=(40, 30, 34), life=5.0, n=3, seed=0, strength=1.0):
        assert kind in MARK_KINDS, kind
        self.tl.add(t0, dict(kind=kind, u=float(u), v=float(v), size=float(size), angle=float(angle),
                             col=np.asarray(col, float), life=float(life), n=int(n), seed=int(seed), k=float(strength)))

    def live(self, t):
        out = []
        for t0, m in self.tl.alive(t):
            age = t - t0
            if age < 0 or age >= m['life']:
                continue
            fade = _smooth(age / 0.06) * (1 - _smooth((age - 0.6 * m['life']) / (0.4 * m['life'])))
            if fade > 0.01:
                out.append((m, fade * m['k'], age))
        return out[-self.max_live:]

    def apply(self, alb, U, V, t, dither=None):
        live = self.live(t)
        if not live:
            return alb
        alb = alb.copy()
        for m, k, age in live:
            s = m['size']
            ext = s * 1.25
            sel = np.nonzero((np.abs(U - m['u']) < ext) & (np.abs(V - m['v']) < ext))[0]
            if not len(sel):
                continue
            ca, sa = math.cos(m['angle']), math.sin(m['angle'])
            du, dv = U[sel] - m['u'], V[sel] - m['v']
            x = (du * ca + dv * sa) / s
            y = (-du * sa + dv * ca) / s
            r = np.sqrt(x * x + y * y) + 1e-6
            th = np.arctan2(y, x)
            kind = m['kind']
            rng = np.random.default_rng(m['seed'])
            dark = np.zeros(len(sel))
            lite = np.zeros(len(sel))
            if kind == 'scorch':
                rr = r * (1 + 0.2 * np.sin(7 * th + m['seed']) + 0.1 * np.sin(13 * th + 2 * m['seed']))
                dark = np.clip(1 - rr, 0, 1) ** 0.8 * 0.7
                lite = np.clip(1 - np.abs(rr - 1.0) / 0.18, 0, 1) * 0.35
            elif kind == 'claw':
                n = max(1, m['n'])
                sp = 0.9 / max(1, n - 1) if n > 1 else 0
                taper = np.clip(1 - np.abs(x), 0, 1) ** 0.5
                for i in range(n):
                    yi = -0.45 + i * sp if n > 1 else 0.0
                    dd = np.abs(y - yi - 0.06 * np.sin(x * 3 + i))
                    dark = np.maximum(dark, np.clip(1 - dd / 0.07, 0, 1) * taper * 0.75)
                    lite = np.maximum(lite, np.clip(1 - np.abs(dd - 0.1) / 0.035, 0, 1) * taper * 0.4)
            elif kind == 'crack':
                n = max(3, m['n'])
                for i in range(n):
                    a0 = rng.uniform(0, 2 * math.pi)
                    L_ = rng.uniform(0.6, 1.0)
                    ai = a0 + 0.22 * np.sin(r * 11 + i * 3.1)
                    dth = np.abs((th - ai + math.pi) % (2 * math.pi) - math.pi)
                    w = 0.03 / np.maximum(r, 0.05) * (1.2 - r)
                    dark = np.maximum(dark, np.clip(1 - dth / np.maximum(w, 1e-3), 0, 1) * (r < L_) * 0.85)
                dark = np.maximum(dark, np.clip(1 - r / 0.15, 0, 1) * 0.6)
            elif kind == 'water':
                rr = r * (1 + 0.12 * np.sin(5 * th + m['seed']))
                dark = np.clip(1 - rr, 0, 1) ** 0.4 * 0.35
                lite = np.clip(1 - np.abs(rr - 0.92) / 0.08, 0, 1) * 0.45
            elif kind == 'smoke_ring':
                dark = np.clip(1 - np.abs(r - 0.75) / 0.2, 0, 1) ** 1.5 * 0.5
            elif kind == 'dust':
                rr = r * (1 + 0.25 * np.sin(6 * th + m['seed']))
                lite = np.clip(1 - rr, 0, 1) ** 0.7 * 0.5
            elif kind == 'glass':
                h = np.sin(x * 91.7 + y * 47.3 + m['seed']) * 43758.5453
                h = h - np.floor(h)
                lite = ((h > 0.965) & (r < 1)).astype(float) * 0.9
            if dither is not None:
                dq = dither[sel]
                dark = np.floor(dark * 3 + dq * 0.9) / 3
                lite = np.floor(lite * 3 + dq * 0.9) / 3
            a = alb[sel]
            if kind in ('scorch', 'dust', 'glass'):
                a = a * (1 - dark[:, None] * k) + (m['col'][None] - a) * (lite[:, None] * k)
            else:
                a = a * (1 - dark[:, None] * k) + (255.0 - a) * (lite[:, None] * k * 0.6)
            if kind == 'water':
                a = a * (1 - 0.2 * k * (dark[:, None] > 0)) + np.array([60, 90, 140])[None] * 0.2 * k * (dark[:, None] > 0)
            alb[sel] = a
        return alb
