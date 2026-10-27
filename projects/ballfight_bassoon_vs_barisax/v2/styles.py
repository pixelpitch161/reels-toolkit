"""#39 weapon styles (new): the BASSOON's TIPTOE (hop arcs, squash and stretch, the stomp ring, backstab slashes,
tiptoe prints, MOUNTAIN KING's dance trail) and BOCAL HOOK (the silver crook swinging out and hooking), the BARI SAX's
WINDOW RATTLER (v1: the honk's sound rings, the ropes' standing waves, slings), GROWL's gravelly ring, SKANK's off-beat
stabs and LOW A's ring-wide rumble with jumping floor tiles; v2 (Oct 26): WRECKING BALL (the crane line from the
spot's rig, the hook block, the swing cable and hook, the ball's harness straps, the swing's coral trail) and CABLE
SNAP (the hoist, the snapped cable whipping away, the target shadow and ring on the canvas). Styles live in the episode's folder so later crossovers
can import them (series rule).

All functions draw on a float (H, W, 3) image at the internal resolution with a reelkit camera; positions are in the
ring frame (x, z) at a height y, mapped through scene.to_world."""
import math
import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, BAYER_FULL
import scene as S

INK = np.array([18, 10, 26], float)
WHITE = np.array([255, 252, 240], float)
NICKEL = np.array([216, 220, 232], float)
CANE = np.array([232, 208, 150], float)
THREAD = np.array([200, 46, 50], float)


def _mask(polys, lines=(), width=1):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if len(pts) >= 3:
            d.polygon([tuple(map(float, p)) for p in pts], fill=255)
    for pts in lines:
        if len(pts) >= 2:
            d.line([tuple(map(float, p)) for p in pts], fill=255, width=width)
    return np.asarray(m, np.float32) / 255.0


def blend(img, mk, col, alpha):
    if alpha <= 0:
        return img
    img += (np.asarray(col, float)[None, None] - img) * (mk * alpha)[..., None]
    return img


def add(img, mk, col, k):
    img += mk[..., None] * np.asarray(col, float)[None, None] * k
    return img


# ---------------------------------------------------------------- TIPTOE
def hop_height(L):
    """peak height of a hop of length L (ring units): a tall, exaggerated tiptoe even for short hops"""
    return 0.42 + 0.16 * min(2.5, L)


def hop_state(u, L):
    """(height, squash-x, squash-y) of a hopping ball at hop progress u (0..1): it crouches as it leaves, stretches
    tall while it rises and falls fast, round at the top"""
    hgt = hop_height(L) * 4 * u * (1 - u)
    v = abs(1 - 2 * u)                                               # vertical speed (0 at the top)
    sy = 1.0 + 0.16 * v
    return hgt, 1.0 / math.sqrt(sy), sy


def land_squash(age):
    """squash on landing: flat and wide for a couple of frames, then a little bounce back"""
    if age < 0 or age > 0.2:
        return 1.0, 1.0
    if age < 0.07:
        k = 1 - age / 0.07 * 0.3
        return 1.0 + 0.24 * k, 1.0 - 0.26 * k
    u = (age - 0.07) / 0.13
    s = 0.06 * math.sin(math.pi * u) * (1 - u)
    return 1.0 - s, 1.0 + s


def stomp_light(sxp, szp, x, z, age, radius, back=False):
    """(N,) light of a stomp's ring on the canvas: a quick expanding band (a backstab's is bigger and double)"""
    life = 0.32 if back else 0.24
    if age < 0 or age >= life:
        return None
    u = age / life
    rd = np.hypot(sxp - x, szp - z)
    r = 0.35 + (radius + 0.5 + 0.5 * back) * (1 - (1 - u) ** 2)
    band = np.clip(1 - np.abs(rd - r) / (0.07 + 0.05 * back), 0, 1) * (1 - u)
    if back:
        band = np.maximum(band, np.clip(1 - np.abs(rd - r * 0.62) / 0.05, 0, 1) * (1 - u) * 0.8)
    return band


def draw_hop_arc(img, cam, p0, p1, age_k, col, y0, peak):
    """a faint dotted arc of the hop's path (the tiptoe's trajectory), fading out"""
    if age_k <= 0.02:
        return img
    u = np.linspace(0, 1, 9)
    P = p0[None] + (p1 - p0)[None] * u[:, None]
    hy = y0 + peak * 4 * u * (1 - u)
    sp = cam.wp(S.to_world(P[:, 0], P[:, 1], hy))
    c = np.clip(np.asarray(col) * 0.7 + 90, 0, 255)
    for j, (x, yy) in enumerate(sp[1:-1]):
        xi, yi = int(round(x)), int(round(yy))
        if 1 <= xi < W - 1 and 1 <= yi < H - 1:
            img[yi, xi] = img[yi, xi] * (1 - age_k) + c * age_k
    return img


def backstab_slash(img, cam, x, z, aim, age, col, y=0.35, life=0.28):
    """a backstab: two crossed slashes in the bassoon's green, inked, flicking across the enemy's back"""
    if age < 0 or age >= life:
        return img
    k = 1 - age / life
    d = np.array([math.cos(aim), math.sin(aim)])
    nrm = np.array([-d[1], d[0]])
    grow = min(1.0, age / 0.06)
    for sgn in (1, -1):
        a = np.array([x, z]) - d * 0.22 + nrm * 0.62 * sgn
        b = np.array([x, z]) + d * 0.22 - nrm * 0.62 * sgn
        b = a + (b - a) * grow
        P = np.array([a, b])
        sp = cam.wp(S.to_world(P[:, 0], P[:, 1], np.array([y + 0.3, y - 0.1])))
        img = blend(img, _mask([], [sp], 5), INK, 0.75 * k)
        img = blend(img, _mask([], [sp], 3), np.clip(np.asarray(col) * 0.7 + 90, 0, 255), 1.0 * k)
        img = blend(img, _mask([], [sp], 1), WHITE, 0.9 * k)
    return img


# ---------------------------------------------------------------- BOCAL HOOK
def crook_points(p, aim, ext, R, curl, y):
    """the bocal as a hook: out of the ball's rim along the aim for `ext` (ring units), then curling round by `curl`
    (rad) at the end; returns (n, 3) ring-frame points (x, z, height)"""
    d = np.array([math.cos(aim), math.sin(aim)])
    nrm = np.array([-d[1], d[0]])
    base = np.asarray(p, float) + d * R * 0.9
    n1 = 10
    s = np.linspace(0, 1, n1)
    shaft = base[None] + d[None] * (ext * s)[:, None] + nrm[None] * (0.18 * np.sin(np.pi * s) * ext / 1.6)[:, None]
    hy = y + 0.10 + 0.25 * np.sin(np.pi * s) * min(1.0, ext / 1.2)
    pts = [np.column_stack([shaft, hy])]
    if curl > 0.05:                                                  # the hook at the end, curling back round
        rc = 0.22
        c = shaft[-1] + nrm * rc
        a0 = math.atan2(-nrm[1], -nrm[0])
        aa = a0 + np.linspace(0, curl, 8)[1:]
        hook = c[None] + rc * np.column_stack([np.cos(aa), np.sin(aa)])
        pts.append(np.column_stack([hook, np.full(len(aa), hy[-1])]))
    return np.vstack(pts)


def draw_crook(img, cam, p, aim, ext, R, curl, y, col, glow=0.0):
    """BOCAL HOOK: a silver crook swinging out of the ball (inked, a cane-coloured reed nub at the root, lit green while
    it hooks)"""
    P = crook_points(p, aim, ext, R, curl, y)
    sp = cam.wp(S.to_world(P[:, 0], P[:, 1], P[:, 2]))
    img = blend(img, _mask([], [sp], 7), INK, 0.85)
    img = blend(img, _mask([], [sp], 4), NICKEL * 0.82 + np.asarray(col) * 0.18, 1.0)
    img = blend(img, _mask([], [sp[: max(2, len(sp) * 2 // 3)]], 1), WHITE, 0.9)
    if glow > 0.02:
        img = add(img, _mask([], [sp], 7), np.asarray(col) * 0.7 + 60, 0.35 * glow)
    r0 = sp[0]
    xi, yi = int(round(r0[0])), int(round(r0[1]))
    img[max(0, yi - 1):yi + 2, max(0, xi - 1):xi + 2] = THREAD
    return img


def swirl(img, x, y, rp, k, t, col):
    """a hooked enemy spun round: two curved speed arcs whipping round the ball"""
    if k <= 0.02:
        return img
    c = np.clip(np.asarray(col) * 0.6 + 100, 0, 255)
    for j in range(2):
        a0 = t * 18.0 + j * math.pi
        aa = a0 + np.linspace(0, 1.6, 9)
        pts = np.column_stack([x + rp * 1.25 * np.cos(aa), y + rp * 1.25 * np.sin(aa)])
        img = blend(img, _mask([], [pts], 3), INK, 0.5 * k)
        img = blend(img, _mask([], [pts], 1), c, 1.0 * k)
    return img


# ---------------------------------------------------------------- WINDOW RATTLER, GROWL, SKANK, LOW A
def honk_light(sxp, szp, cx, cz, r, rmax, t, kind):
    """(N,) light of a honk's sound rings on the canvas: three ripples behind the front (a low note's long wave), a
    quake's broad rumbling band, GROWL's gravelly, broken ring"""
    rd = np.hypot(sxp - cx, szp - cz)
    k = max(0.0, 1 - r / rmax)
    if kind == 'quake':
        band = np.clip(1 - np.abs(rd - r) / 0.45, 0, 1) * (0.6 + 0.4 * np.sin(rd * 9 - t * 30))
        return band * (0.4 + 0.6 * k)
    if kind == 'shockwave':
        ang = np.arctan2(szp - cz, sxp - cx)
        grit = 0.5 + 0.5 * np.sign(np.sin(ang * 23 + rd * 7))
        return np.clip(1 - np.abs(rd - r) / 0.16, 0, 1) * (0.45 + 0.55 * grit) * k
    out = np.zeros_like(rd)
    for j, (off, a) in enumerate(((0.0, 1.0), (0.32, 0.6), (0.62, 0.35))):
        rr = r - off
        if rr > 0:
            out = np.maximum(out, np.clip(1 - np.abs(rd - rr) / 0.07, 0, 1) * a)
    return out * k


def draw_honk_front(img, cam, cx, cz, r, rmax, col, y=0.3, kind='rattle'):
    """the honk's front as an inked coral ring just above the canvas (so it reads over the balls' shadows)"""
    k = max(0.0, 1 - r / rmax)
    if k <= 0.02:
        return img
    a = np.linspace(0, 2 * math.pi, 61)
    wob = 0.04 * np.sin(a * 11) if kind == 'rattle' else 0.09 * np.where(np.arange(len(a)) % 2, 1.0, -0.8)
    P = cam.wp(S.to_world(cx + (r + wob) * np.cos(a), cz + (r + wob) * np.sin(a), np.full(len(a), y)))
    img = blend(img, _mask([], [P], 3), INK, 0.45 * k)
    img = blend(img, _mask([], [P], 1), np.clip(np.asarray(col) * 0.75 + 70, 0, 255), 0.9 * k)
    return img


def rope_wave(u, t, a, t0, mode=3, f=6.5):
    """(n,) standing-wave displacement of a ringing rope at positions u (0..1): a plucked string's mode shape
    (antinodes between the posts) oscillating, scaled by the ring amplitude a"""
    if a <= 1e-3:
        return np.zeros_like(u)
    ph = 2 * math.pi * f * max(0.0, t - t0)
    shape = np.sin(np.pi * mode * u) * 0.7 + np.sin(np.pi * u) * 0.5
    return 0.19 * a * shape * math.cos(ph)


def tile_jump(sxp, szp, t, k, seed=0):
    """(N,) brightness wobble of the canvas tiles while LOW A shakes the ring: each tile jumps on its own phase"""
    if k <= 0.01:
        return None
    tx, tz = np.floor(sxp * 1.6), np.floor(szp * 1.6)
    h = np.sin(tx * 12.9898 + tz * 78.233 + seed) * 43758.5453
    ph = (h - np.floor(h)) * 2 * math.pi
    return k * np.maximum(0.0, np.sin(t * 34.0 + ph)) ** 3


def mk_trail(cam, centre, ang0, step, n_done, R, gap, y=0.06):
    """MOUNTAIN KING's dance: the circle of landing spots round the enemy (the ones so far), as screen points"""
    rr = 2 * R + gap
    a = ang0 + math.radians(step) * np.arange(n_done)
    P = np.column_stack([centre[0] + rr * np.cos(a), centre[1] + rr * np.sin(a)])
    return cam.wp(S.to_world(P[:, 0], P[:, 1], np.full(len(P), y)))


# ---------------------------------------------------------------- v2: WRECKING BALL, CABLE SNAP
STEEL = np.array([132, 138, 156], float)
STEEL_HI = np.array([226, 230, 242], float)
LEATHER = np.array([62, 36, 32], float)
Y_RIG = 9.5                                       # the spot's truss (scene.SPOT): the crane line hangs from it


def line3(img, cam, P, col, width=2, ink=0.8, hi=0.5, alpha=1.0):
    """a world polyline as an inked steel line with a 1-px highlight"""
    sp = cam.wp(np.atleast_2d(P))
    if ink > 0:
        img = blend(img, _mask([], [sp], width + 2), INK, ink * alpha)
    img = blend(img, _mask([], [sp], width), col, alpha)
    if hi > 0 and width >= 2:
        img = blend(img, _mask([], [sp], 1), STEEL_HI, hi * alpha)
    return img


def cable(a, b, n=10, sag=0.0, wig=0.0, ph=0.0):
    """world points from a to b: a straight cable (sag > 0: slack, hanging down in the middle; wig: a travelling
    wiggle, e.g. a snapped end whipping)"""
    u = np.linspace(0.0, 1.0, n)
    P = a[None] + (b - a)[None] * u[:, None]
    P[:, 1] -= sag * np.sin(np.pi * u)
    if wig:
        d = b - a
        side = np.array([-d[2], 0.0, d[0]])
        side = side / (np.linalg.norm(side) + 1e-9)
        P += side[None] * (wig * np.sin(np.pi * u * 3 + ph) * u)[:, None]
    return P


def hook_block(img, cam, c, size=0.24, col=None):
    """the crane's hook block at world point c: a small inked steel box (a lit face and a dark side)"""
    col = STEEL if col is None else np.asarray(col, float)
    Q = [c + np.array([dx, dy, 0.0]) for dx, dy in ((-size, -size * 1.2), (size, -size * 1.2), (size * 0.8, size),
                                                     (-size * 0.8, size))]
    sp = cam.wp(np.array(Q))
    img = blend(img, _mask([sp]), INK, 0.9)
    inner = sp.mean(0) + (sp - sp.mean(0)) * 0.7
    img = blend(img, _mask([inner]), col * 0.85, 1.0)
    top = inner[2:4]
    img = blend(img, _mask([], [top], 1), STEEL_HI, 0.7)
    return img


def hook_shape(img, cam, end, toward, size=0.14):
    """a steel hook at world point end (the harness ring), opening away from the cable direction `toward`"""
    s0 = cam.wp(end[None])[0]
    s1 = cam.wp((end + toward * 0.2)[None])[0]
    d = s1 - s0
    d = d / (np.linalg.norm(d) + 1e-9)
    n = np.array([-d[1], d[0]])
    r = max(2.0, size * 18)
    a = np.linspace(0.2 * math.pi, 1.6 * math.pi, 9)
    pts = s0[None] + (np.cos(a)[:, None] * n[None] - np.sin(a)[:, None] * d[None]) * r * 0.6 - d[None] * r * 0.3
    pts = np.vstack([s0[None] + d[None] * r * 0.8, pts])
    img = blend(img, _mask([], [pts], 4), INK, 0.85)
    img = blend(img, _mask([], [pts], 2), STEEL * 1.1, 1.0)
    return img


def harness(img, s, rp, sx, sy, ring_dir, col):
    """the bari's harness on its ball (screen space, centre s, radius rp, squash sx / sy): two leather straps over the
    top crossing at a steel ring on the side facing the cable (ring_dir: screen unit vector, None = straight up)"""
    ring_dir = np.array([0.0, -1.0]) if ring_dir is None else np.asarray(ring_dir, float)
    top = np.array([s[0], s[1]]) + ring_dir * rp * np.array([sx, sy]) * 0.92
    straps = []
    for sgn in (-1.0, 1.0):
        b = np.linspace(0.0, math.pi * 0.92, 12)
        perp = np.array([-ring_dir[1], ring_dir[0]]) * sgn
        pts = top[None] + (perp[None] * np.sin(b)[:, None] * 0.62 - ring_dir[None] * (1 - np.cos(b))[:, None] * 0.9) * \
            rp * np.array([sx, sy])[None]
        straps.append(pts)
    belt_a = np.linspace(0.15 * math.pi, 0.85 * math.pi, 12)
    belt = np.column_stack([s[0] + np.cos(belt_a) * rp * sx * 0.96, s[1] + np.sin(belt_a) * rp * sy * 0.30 + rp * 0.08])
    straps.append(belt)
    img = blend(img, _mask([], straps, 5), INK, 0.6)
    img = blend(img, _mask([], straps, 3), LEATHER * 0.7 + np.asarray(col) * 0.3, 1.0)
    img = blend(img, _mask([], [st[::3] for st in straps], 1), np.asarray(col) * 0.6 + 90, 0.5)
    xi, yi = int(round(top[0])), int(round(top[1]))
    if 2 <= xi < W - 2 and 2 <= yi < H - 2:
        img[yi - 2:yi + 3, xi - 2:xi + 3] = INK
        img[yi - 1:yi + 2, xi - 1:xi + 2] = STEEL_HI
        img[yi, xi] = STEEL
    return img


def swing_trail(img, pts, col, k=1.0):
    """the swing's coral trail behind the ball: a tapering band through the recent screen positions (oldest first)"""
    n = len(pts)
    if n < 2 or k <= 0.02:
        return img
    c = np.clip(np.asarray(col) * 0.8 + 70, 0, 255)
    for j in range(n - 1):
        u = (j + 1) / (n - 1)
        w = max(1, int(round(2 + 14 * u)))
        seg = [pts[j], pts[j + 1]]
        img = blend(img, _mask([], [seg], w), c, 0.65 * k * u)
        if w >= 4:
            img = add(img, _mask([], [seg], max(1, w // 3)), WHITE * 0.5, 0.35 * k * u)
    return img


def snap_target(sxp, szp, x, z, R, radius, u, t):
    """(shade, ring) on the canvas for CABLE SNAP's target: the falling ball's shadow (grows and darkens as it comes
    down, u 0..1) and a pulsing danger ring at the crater's reach"""
    rd = np.hypot(sxp - x, szp - z)
    r_sh = R * (0.45 + 0.6 * u)
    shade = np.clip(1 - (rd - r_sh) / 0.10, 0, 1) * (0.25 + 0.6 * u)
    rr = R + radius
    pulse = 0.55 + 0.45 * math.cos(2 * math.pi * 6.0 * t)
    ring = np.clip(1 - np.abs(rd - rr) / 0.05, 0, 1) * pulse * (0.5 + 0.5 * u)
    tick = np.clip(1 - np.abs(rd - rr * 0.55) / 0.035, 0, 1) * (np.abs(np.sin(np.arctan2(szp - z, sxp - x) * 2)) > 0.92)
    return shade, np.maximum(ring, tick * 0.8)
