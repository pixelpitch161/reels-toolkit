"""#31 draw functions for the new BALLFIGHT weapon styles and ability looks, kept in the episode's folder (series rule)
so later episodes and crossovers can import them:

    style 'vinyl'   the decks' frisbees: a spinning black record (grooves, a lime label, a glint), flat over the canvas
    style 'arp'     the synth's satellites: glowing note gems orbiting the ball (dim while they rest after a hit)
    'saw'           SAWTOOTH's notes: a little saw-tooth wave glyph flying along its path (arc projectiles)
    drop_*()        BASS DROP: a giant record dropping out of the dark onto the predicted spot (slam)
    vhs()           REWIND: tape-rewind look on the image around the zipping ball (tracking jitter, colour split,
                    scanlines, a rolling noise bar and the ◀◀ glyph)
    sprite()        the Pixel Pitch models tipped toward the camera (`pitch`), so the platter / the keys read

Everything works in the ring frame (sx, sz) + height, through scene.to_world, like #27's renderer."""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw
import scene as S
from reelkit.config import W, H, BAYER_FULL

R = S.R_BALL
HERE = os.path.dirname(os.path.abspath(__file__))


def mask_of(polys):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if len(pts) >= 3:
            d.polygon([tuple(q) for q in pts], fill=255)
    return np.asarray(m, np.float32) / 255.0


def circle_world(cx, cz, r, y, n=14, a0=0.0):
    a = a0 + np.linspace(0, 2 * math.pi, n + 1)[:-1]
    return S.to_world(cx + r * np.cos(a), cz + r * np.sin(a), y)


# ---------------------------------------------------------------- vinyl frisbees (volley projectiles)
VINYL = np.array([30, 28, 38])
GROOVE = np.array([74, 70, 88])
DISC_R = 0.21
DISC_Y = 0.46


def disc_poly(x, z, r=DISC_R, y=DISC_Y, n=14):
    return circle_world(x, z, r, y, n)


def draw_disc(fr, cam, x, z, spin, col, alpha=1.0, r=DISC_R):
    """one record: black body, a groove ring, the owner's label, a glint that turns with the spin, a coloured rim"""
    body = cam.wp(disc_poly(x, z, r))
    fr.draw.polygon([tuple(p) for p in body], fill=tuple(int(v) for v in VINYL * alpha + 0 * (1 - alpha)))
    gr = cam.wp(disc_poly(x, z, r * 0.70))
    fr.draw.polygon([tuple(p) for p in gr], outline=tuple(int(v) for v in GROOVE))
    lab = cam.wp(disc_poly(x, z, r * 0.36, n=8))
    fr.draw.polygon([tuple(p) for p in lab], fill=tuple(int(v) for v in np.clip(np.array(col) * 0.95, 0, 255)))
    # the glint: two points of the rim catching the spot, turning with the record
    for da in (0.0, 0.35):
        q = S.to_world(x + r * 0.84 * math.cos(spin + da), z + r * 0.84 * math.sin(spin + da), DISC_Y + 0.01)
        s = cam.wp(q[None])[0]
        fr.draw.point([(s[0], s[1])], fill=(236, 232, 246))
    fr.draw.polygon([tuple(p) for p in body], outline=tuple(int(v) for v in np.clip(np.array(col) * 0.8 + 30, 0, 255)))


# ---------------------------------------------------------------- arpeggiator satellites (orbit)
SEATS = (0, 4, 2, 6, 1, 5, 3, 7)        # fight.Fight.ORBIT_SEATS
GEM_R = 0.24


def sat_positions(p, th, reach, n):
    """[(j, ring-frame position)] exactly as fight.orbit_sats places them"""
    r = R + reach
    return [(j, p + r * np.array([math.cos(th + SEATS[j] * math.pi / 4), math.sin(th + SEATS[j] * math.pi / 4)]))
            for j in range(n)]


def gem_world(q, size, y=R):
    """a note gem: a diamond standing up (camera-facing in screen space is done by the caller), as a world quad"""
    x, z = q
    return np.array([S.to_world(x, z, y + size), S.to_world(x + size * 0.75, z - size * 0.75, y),
                     S.to_world(x, z, y - size * 0.7), S.to_world(x - size * 0.75, z + size * 0.75, y)])


def draw_gem(fr, cam, q, col, k=1.0, size=GEM_R, y=R):
    """k: 1 = lit, ~0.35 = resting after a hit"""
    Q = gem_world(q, size * (0.75 + 0.25 * k), y)
    s = cam.wp(Q)
    c = np.array(col, float)
    body = np.clip(c * (0.45 + 0.55 * k) + 50 * k, 0, 255)
    fr.draw.polygon([tuple(p) for p in s], fill=tuple(int(v) for v in body), outline=(12, 30, 44))
    core = (s[0] + s[1] + s[2] + s[3]) / 4
    if k > 0.6:                                               # a white-hot core
        cx, cy = core
        fr.draw.polygon([(cx, cy - 2), (cx + 1.5, cy), (cx, cy + 1.5), (cx - 1.5, cy)], fill=(246, 254, 255))
    # the facet: the upper-left half a shade lighter
    fr.draw.polygon([tuple(s[0]), tuple(s[3]), tuple(core)], fill=tuple(int(v) for v in np.clip(body * 1.15 + 20, 0, 255)))


# ---------------------------------------------------------------- SAWTOOTH notes (arc projectiles)
def saw_points(x, z, ang, y=0.5, L_=0.40, amp=0.09, teeth=3):
    """a saw-tooth wave segment along the flight direction: ramps up, drops straight down"""
    d = np.array([math.cos(ang), math.sin(ang)])
    n = np.array([-d[1], d[0]])
    c = np.array([x, z]) - d * L_ / 2
    pts = []
    for j in range(teeth):
        a = c + d * L_ * j / teeth
        b = c + d * L_ * (j + 1) / teeth
        pts += [a - n * amp, b + n * amp, b - n * amp]
    return np.array([S.to_world(q[0], q[1], y) for q in pts])


def orbit_line(cam, p, reach, y=R, n=40):
    """the orbit's path at satellite height, as a screen polyline (closed)"""
    s = cam.wp(circle_world(p[0], p[1], R + reach, y, n))
    return [tuple(q) for q in s] + [tuple(s[0])]


def draw_saw(fr, cam, x, z, ang, col):
    s = cam.wp(saw_points(x, z, ang))
    c = tuple(int(v) for v in np.clip(np.array(col) * 0.6 + 110, 0, 255))
    fr.draw.line([tuple(p) for p in s], fill=c, width=2)
    fr.draw.line([tuple(p) for p in s], fill=(240, 252, 255), width=1)


# ---------------------------------------------------------------- BASS DROP: a giant record dropping (slam)
def drop_height(t, t0, t_slam):
    """the record hangs high while the target blinks, then drops (accelerating) onto the spot at t_slam; it lies there a
    moment and fades"""
    if t < t0:
        return None
    if t < t_slam - 0.32:
        return 6.0 - 0.6 * min(1.0, (t - t0) / 0.4)
    if t < t_slam:
        u = (t - (t_slam - 0.32)) / 0.32
        return 5.4 * (1 - u * u) + 0.06
    if t < t_slam + 0.55:
        return 0.06
    return None


def drop_polys(cam, cx, cz, rad, h, col, spin):
    """the dropping record seen from above, as [(screen polygon, colour)] back to front: black disc, grooves, label"""
    r = rad * 1.1
    out = []
    for rr, c in ((r, VINYL), (r * 0.84, GROOVE * 0.85), (r * 0.80, VINYL), (r * 0.62, GROOVE * 0.75), (r * 0.58, VINYL),
                  (r * 0.34, np.array(col, float) * 0.9), (r * 0.06, np.array([200, 200, 210]))):
        out.append((cam.wp(circle_world(cx, cz, rr, h, 28, spin)), c))
    return out


# ---------------------------------------------------------------- REWIND: the tape look around the zipping ball
def vhs(img, cx, cy, k, t, seed=0):
    """img uint8 (H, W, 3); a band around screen point (cx, cy): rows jitter sideways, red / blue split apart, every other
    row darker, a bright noise bar rolls up through it; k 0..1 strength"""
    if k <= 0.01:
        return img
    rng = np.random.default_rng(int(t * 30) * 7 + seed)
    y0, y1 = max(0, int(cy - 42)), min(H, int(cy + 42))
    if y1 <= y0:
        return img
    band = img[y0:y1].astype(np.float32)
    out = band.copy()
    for r in range(y1 - y0):
        u = 1 - abs((y0 + r) - cy) / 42.0
        sh = int(round(k * u * (rng.integers(-3, 4) + 2 * math.sin(0.5 * (y0 + r) + 40 * t))))
        if sh:
            out[r] = np.roll(band[r], sh, axis=0)
    split = max(1, int(round(2 * k)))
    out[:, :, 0] = np.roll(out[:, :, 0], split, axis=1)
    out[:, :, 2] = np.roll(out[:, :, 2], -split, axis=1)
    rows = np.arange(y1 - y0)
    out[(rows + y0) % 2 == 0] *= 1 - 0.18 * k
    bar = int(((1 - (t * 1.7) % 1.0)) * (y1 - y0))
    for r in range(max(0, bar - 1), min(y1 - y0, bar + 2)):
        noise = rng.uniform(0.0, 1.0, W) > 0.55
        out[r, noise] = out[r, noise] * (1 - 0.5 * k) + 230 * 0.5 * k
    wv = np.clip(1 - np.abs((rows + y0) - cy) / 42.0, 0, 1)[:, None, None] * k
    img[y0:y1] = np.clip(band * (1 - wv) + out * wv, 0, 255).astype(np.uint8)
    return img


def rewind_glyph(img, x, y, col, k):
    """◀◀ as two little filled triangles"""
    if k <= 0.05:
        return
    im = Image.fromarray(img)
    d = ImageDraw.Draw(im)
    c = tuple(int(v) for v in np.clip(np.array(col) * 0.6 + 100, 0, 255))
    for ox in (0, 7):
        d.polygon([(x + ox, y), (x + ox + 6, y - 4), (x + ox + 6, y + 4)], fill=(10, 8, 18))
        d.polygon([(x + ox + 1, y), (x + ox + 6, y - 3), (x + ox + 6, y + 3)], fill=c)
    img[:] = np.asarray(im)


# ---------------------------------------------------------------- sprites tipped toward the camera
_FACES = {}


def render_sprite(name, radius_px, yaw_deg, roll_deg=0.0, pitch_deg=0.0):
    """which_instrument_wins' sprite renderer (the series' own rasteriser) with an extra pitch (model tipped toward the
    camera about its x axis) so flat instruments read from the 3/4 top-down camera"""
    sys.path.append(os.path.join(HERE, '..', 'which_instrument_wins'))
    import sprites as SP
    import piano_lowpoly as P
    RW, RH = P.RW, P.RH
    cx = cy = radius_px + 3
    img = np.zeros((RH, RW, 3), np.uint8)
    zb = np.zeros((RH, RW))
    gb = np.zeros((RH, RW), np.int8)
    cam = SP._Cam(yaw_deg, radius_px, cx + 0.5, cy + 0.5, roll_deg)
    a, b, y = math.radians(pitch_deg), math.radians(roll_deg), math.radians(yaw_deg)
    Rx = np.array([[1, 0, 0], [0, math.cos(a), -math.sin(a)], [0, math.sin(a), math.cos(a)]])
    Ry = np.array([[math.cos(y), 0, math.sin(y)], [0, 1, 0], [-math.sin(y), 0, math.cos(y)]])
    Rz = np.array([[math.cos(b), -math.sin(b), 0], [math.sin(b), math.cos(b), 0], [0, 0, 1]])
    cam.R = Rz @ Rx @ Ry
    P.render_geo(SP._geo(name), cam, img, zb, gb, scale=1.0)
    P.outline(img, zb, gb)
    n = 2 * radius_px + 7
    return np.dstack([img[:n, :n], (gb[:n, :n] > 0).astype(np.uint8) * 255])
