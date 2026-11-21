"""Builds the BASS GUITAR low-poly model and writes studio/crates/bf-content/data/models/BASS GUITAR.json.

Made from the photo references in studio/reference/models/bass_guitar/ (see NOTES.md there): a four-string solid-body
electric bass in the Precision Bass pattern. An offset double-cutaway body in ultraviolet with a bevelled edge of
darker facets, a white pickguard, a split pickup in two staggered halves, a bridge with four saddles and two chrome
control knobs; a long maple neck (34-inch scale) with dark fret lines and dot inlays, a bone nut, and a 4-in-line
headstock with four chrome tuner posts and cloverleaf keys standing out on the bass side; four strings run from the
saddles over the nut to the posts.

It is built flat in inches (the outlines traced from the photos), lying on its back with its face to the camera: x
runs across the body (the bass side, with the tuner keys, to -x), y runs along the instrument (headstock up), z points
to the camera. The neck, headstock and hardware are made a little bigger across than life (NECK_X, HEAD_X), the body a
tenth bigger (BODY_K) and a little thicker, so they read at a small radius; the neck keeps its real 34-inch scale. Then it is laid
diagonally across the view (headstock up and to the right), tipped so its face looks a little up at the camera, and
turned to cancel a roster view yaw of 30.

Axes: x runs left to right as the camera sees it, y is up, z points to the camera (the sprite camera looks from +z).
The model is centred on its bounding box and scaled to fit a unit sphere.

Standard library only. Run: python studio/scripts/models/bass_guitar_model.py
"""

import json
import math
import os
import re

ULTRA = (150.0, 0.0, 255.0)
ULTRA_DARK = (82.0, 0.0, 140.0)
GUARD = (238.0, 236.0, 228.0)
GUARD_EDGE = (176.0, 172.0, 168.0)
MAPLE = (232.0, 196.0, 132.0)
MAPLE_DARK = (190.0, 148.0, 90.0)
FRET = (110.0, 80.0, 56.0)
DOT = (28.0, 22.0, 26.0)
NUT = (246.0, 240.0, 222.0)
CHROME = (214.0, 218.0, 228.0)
CHROME_DIM = (150.0, 154.0, 166.0)
PICKUP = (26.0, 24.0, 30.0)
STRING = (200.0, 202.0, 210.0)

# How it lies in the view: turned about z (screen diagonal), tipped about x, then about y, before the camera's yaw.
TILT_Z = -40.0  # degrees: the headstock rises to the right, the body sits low on the left
FACE_UP = 18.0  # the face looks this far up, towards the sprite camera (which looks down 16 degrees)
TURN_Y = -30.0  # cancels the roster view yaw of 30, so the bass lies face on to the camera

# --- dimensions, inches -----------------------------------------------------------------------------------------
SCALE_LEN = 34.0  # nut to saddles
Y_SADDLE = 3.6  # the saddles' line, from the bottom of the body
Y_NUT = Y_SADDLE + SCALE_LEN
NECK_X = 1.5  # neck, headstock and hardware made this much wider than life
T = 2.4  # body thickness (real: 1.75)
CHAMFER = 0.55  # the bevelled edge round the face
Z_FACE = T / 2
Z_GUARD = Z_FACE + 0.06
Z_BOARD = Z_FACE + 0.62  # the fretboard's top
Y_HEEL = 13.1  # the neck's end inside the body
W_NUT, W_HEEL = 1.75 * NECK_X, 2.35 * NECK_X
BODY_K = 1.1  # the body's outline and pickguard made this much bigger than life, about the saddles' line

faces = []


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def unit(v):
    m = math.sqrt(v[0] ** 2 + v[1] ** 2 + v[2] ** 2)
    return (v[0] / m, v[1] / m, v[2] / m)


def centroid(pts):
    k = len(pts)
    return (sum(p[0] for p in pts) / k, sum(p[1] for p in pts) / k, sum(p[2] for p in pts) / k)


def face(pts, rgb, spec, inside=None, two=False):
    """A flat face; its normal points away from `inside` (a point inside the solid). Degenerate faces are dropped."""
    c = cross(sub(pts[1], pts[0]), sub(pts[2], pts[0]))
    if math.sqrt(c[0] ** 2 + c[1] ** 2 + c[2] ** 2) < 1e-9:
        return
    n = unit(c)
    if inside is not None:
        d = sub(centroid(pts), inside)
        if n[0] * d[0] + n[1] * d[1] + n[2] * d[2] < 0:
            n = (-n[0], -n[1], -n[2])
            pts = pts[::-1]
    faces.append({"P": [list(p) for p in pts], "rgb": list(rgb), "spec": spec, "n": list(n), "two": two})


# --- 2D outlines --------------------------------------------------------------------------------------------------
def area2(poly):
    return sum(poly[i][0] * poly[(i + 1) % len(poly)][1] - poly[(i + 1) % len(poly)][0] * poly[i][1]
               for i in range(len(poly)))


def ccw(poly):
    return poly if area2(poly) > 0 else poly[::-1]


def triangulate(poly):
    """Ear clipping of a simple polygon (counter-clockwise); returns index triples."""
    idx = list(range(len(poly)))
    tris = []

    def inside_tri(p, a, b, c):
        def s(p1, p2, p3):
            return (p1[0] - p3[0]) * (p2[1] - p3[1]) - (p2[0] - p3[0]) * (p1[1] - p3[1])
        d1, d2, d3 = s(p, a, b), s(p, b, c), s(p, c, a)
        return not ((d1 < 0 or d2 < 0 or d3 < 0) and (d1 > 0 or d2 > 0 or d3 > 0))

    guard = 0
    while len(idx) > 3 and guard < 10000:
        guard += 1
        for k in range(len(idx)):
            i0, i1, i2 = idx[k - 1], idx[k], idx[(k + 1) % len(idx)]
            a, b, c = poly[i0], poly[i1], poly[i2]
            if (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) <= 1e-12:
                continue  # reflex (or flat) corner
            if any(inside_tri(poly[j], a, b, c) for j in idx if j not in (i0, i1, i2)):
                continue
            tris.append((i0, i1, i2))
            idx.pop(k)
            break
        else:
            break
    if len(idx) == 3:
        tris.append(tuple(idx))
    return tris


def inset(poly, d):
    """The outline moved inwards by d (counter-clockwise outline)."""
    out = []
    n = len(poly)
    for i in range(n):
        p0, p1, p2 = poly[i - 1], poly[i], poly[(i + 1) % n]

        def nrm(a, b):
            dx, dy = b[0] - a[0], b[1] - a[1]
            m = math.hypot(dx, dy)
            return (-dy / m, dx / m)  # inward for a counter-clockwise outline

        n1, n2 = nrm(p0, p1), nrm(p1, p2)
        k = 1.0 + n1[0] * n2[0] + n1[1] * n2[1]
        out.append((p1[0] + d * (n1[0] + n2[0]) / k, p1[1] + d * (n1[1] + n2[1]) / k))
    return out


def flat(poly, z, rgb, spec, up=True, two=False):
    """A flat (possibly concave) plate at height z, facing +z (up) or -z."""
    for a, b, c in triangulate(poly):
        pts = [(poly[a][0], poly[a][1], z), (poly[b][0], poly[b][1], z), (poly[c][0], poly[c][1], z)]
        cx, cy = (poly[a][0] + poly[b][0] + poly[c][0]) / 3, (poly[a][1] + poly[b][1] + poly[c][1]) / 3
        face(pts, rgb, spec, (cx, cy, z - 1.0 if up else z + 1.0), two)


def walls(poly, z0, z1, rgb, spec, inside_z=None, alt=None):
    """The side walls of a plate from z0 to z1 round its outline (counter-clockwise), facing outwards."""
    n = len(poly)
    for i in range(n):
        a, b = poly[i], poly[(i + 1) % n]
        dx, dy = b[0] - a[0], b[1] - a[1]
        m = math.hypot(dx, dy)
        mid = ((a[0] + b[0]) / 2 + dy / m * -0.5, (a[1] + b[1]) / 2 - dx / m * -0.5)  # a point just inside
        col = alt[i % len(alt)] if alt else rgb
        face([(a[0], a[1], z0), (b[0], b[1], z0), (b[0], b[1], z1), (a[0], a[1], z1)], col, spec,
             (mid[0], mid[1], (z0 + z1) / 2))


def plate(poly, z0, z1, top, side, spec, bottom=False):
    poly = ccw(poly)
    flat(poly, z1, top, spec)
    walls(poly, z0, z1, side, spec)
    if bottom:
        flat(poly, z0, side, spec, up=False)


def ngon(cx, cy, r, n=8, rot=0.0):
    return [(cx + r * math.cos(rot + 2 * math.pi * k / n), cy + r * math.sin(rot + 2 * math.pi * k / n))
            for k in range(n)]


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


# --- body: the P-bass outline traced from Fender_AP_P-Bass_20230801.jpg (26 px to the inch, neck centre x 0) ---------
BODY_RAW = ccw([
    (0.0, 0.0), (2.4, 0.25), (4.5, 0.8), (5.8, 2.2), (6.5, 4.6), (6.4, 6.6), (6.0, 8.6), (5.2, 10.6), (5.0, 11.6),
    (5.3, 13.2), (5.7, 14.7), (5.5, 15.9), (5.0, 16.4), (4.4, 16.3), (3.6, 15.5), (2.6, 14.4), (1.9, 13.6),
    (1.75, 15.0), (1.7, 16.6),  # the treble side of the neck pocket, up to where the neck leaves the body
    (-1.7, 16.6), (-1.75, 17.2), (-2.1, 18.6), (-2.6, 19.8), (-3.1, 20.4), (-3.7, 20.2), (-4.4, 19.2),
    (-5.0, 17.4), (-5.1, 15.6), (-4.7, 13.3), (-4.9, 11.6), (-5.6, 9.6), (-6.3, 7.4), (-6.5, 5.2), (-6.2, 2.9),
    (-5.3, 1.3), (-4.0, 0.5), (-2.2, 0.15),
])


def body_k(poly):
    return [(x * BODY_K, Y_SADDLE + (y - Y_SADDLE) * BODY_K) for x, y in poly]


BODY = body_k(BODY_RAW)
FACE_POLY = inset(BODY, CHAMFER * 0.9)
flat(FACE_POLY, Z_FACE, ULTRA, 0.6)
# the bevel: facets from the face's edge down to the outline, every third one darker
n = len(BODY)
for i in range(n):
    j = (i + 1) % n
    col = ULTRA_DARK if i % 3 == 1 else ULTRA
    a0, b0 = BODY[i], BODY[j]
    a1, b1 = FACE_POLY[i], FACE_POLY[j]
    face([(a0[0], a0[1], Z_FACE - CHAMFER), (b0[0], b0[1], Z_FACE - CHAMFER), (b1[0], b1[1], Z_FACE),
          (a1[0], a1[1], Z_FACE)], col, 0.6, ((a1[0] + b1[0]) / 2, (a1[1] + b1[1]) / 2, -T / 2))
walls(BODY, -T / 2, Z_FACE - CHAMFER, ULTRA_DARK, 0.5)
flat(BODY, -T / 2, ULTRA_DARK, 0.4, up=False)

# --- pickguard (white, with a grey edge), traced from the same photo -----------------------------------------------
GUARD_RAW = [
    (1.9, 13.4), (2.7, 14.4), (3.6, 15.3), (4.3, 15.7), (4.8, 15.3), (4.8, 13.6), (4.6, 12.0), (4.75, 10.2),
    (5.5, 8.2), (5.8, 6.6), (5.6, 4.2), (5.0, 2.2), (4.2, 1.2), (3.6, 1.3), (3.3, 3.2), (3.0, 5.6), (2.2, 6.2),
    (-0.4, 6.6), (-2.3, 7.6), (-2.9, 9.4), (-2.9, 11.6), (-2.6, 13.6), (-2.0, 15.2), (-1.6, 15.8), (-1.2, 16.0),
    (1.2, 16.0), (1.5, 15.2),
]
GUARD_POLY = body_k(GUARD_RAW)
plate(GUARD_POLY, Z_FACE - 0.05, Z_GUARD, GUARD, GUARD_EDGE, 0.3)


def block(poly, z0, z1, top, side, spec):
    plate(poly, z0, z1, top, side, spec)


# --- split pickup: two staggered black halves (the bass side half nearer the neck) ----------------------------------
SPREAD_SADDLE = [-1.46, -0.49, 0.49, 1.46]  # string x at the saddles
SPREAD_NUT = [x * W_NUT / 2.6 for x in (-0.975, -0.325, 0.325, 0.975)]  # string x at the nut


def string_x(i, y):
    t = (y - Y_SADDLE) / SCALE_LEN
    return SPREAD_SADDLE[i] + (SPREAD_NUT[i] - SPREAD_SADDLE[i]) * t


block(rect(-2.25, 8.15, 0.1, 9.15), Z_GUARD - 0.02, Z_GUARD + 0.38, PICKUP, PICKUP, 0.4)
block(rect(-0.1, 7.1, 2.25, 8.1), Z_GUARD - 0.02, Z_GUARD + 0.38, PICKUP, PICKUP, 0.4)
# a pair of pole pieces under each string
for i in range(4):
    y = 8.65 if i < 2 else 7.6
    x = string_x(i, y)
    for dx in (-0.2, 0.2):
        block(ngon(x + dx, y, 0.12, 6), Z_GUARD + 0.38, Z_GUARD + 0.42, CHROME_DIM, CHROME_DIM, 0.6)

# --- bridge: a chrome base plate with four saddles ------------------------------------------------------------------
block(rect(-2.3, 2.0, 2.3, 4.7), Z_FACE - 0.02, Z_FACE + 0.14, CHROME_DIM, CHROME_DIM, 0.9)
block(rect(-2.3, 1.75, 2.3, 2.15), Z_FACE - 0.02, Z_FACE + 0.55, CHROME, CHROME_DIM, 0.95)  # the back wall
for i in range(4):
    x = SPREAD_SADDLE[i]
    block(rect(x - 0.34, 3.0, x + 0.34, 4.1), Z_FACE + 0.14, Z_FACE + 0.56, CHROME, CHROME_DIM, 0.95)

# --- control knobs: volume and tone, chrome, on the guard's lower horn ----------------------------------------------
for cx, cy in body_k([(4.55, 5.2), (4.75, 3.0)]):
    block(ngon(cx, cy, 0.62, 8, 0.2), Z_GUARD, Z_GUARD + 0.62, CHROME, CHROME_DIM, 0.95)
    block(ngon(cx, cy, 0.36, 8, 0.6), Z_GUARD + 0.62, Z_GUARD + 0.68, CHROME_DIM, CHROME_DIM, 0.6)


# --- neck: a tapered maple neck, flat board on top, rounded (chamfered) back ----------------------------------------
def neck_w(y):
    t = (y - Y_HEEL) / (Y_NUT - Y_HEEL)
    return W_HEEL + (W_NUT - W_HEEL) * t


def neck_ring(y):
    w = neck_w(y) / 2
    zt = Z_BOARD
    return [(-w, y, zt), (w, y, zt), (w, y, zt - 0.4), (0.55 * w, y, zt - 1.15), (-0.55 * w, y, zt - 1.15),
            (-w, y, zt - 0.4)]


r0, r1 = neck_ring(Y_HEEL), neck_ring(Y_NUT)
NECK_IN = (0.0, (Y_HEEL + Y_NUT) / 2, Z_BOARD - 0.6)
for k in range(6):
    q = (k + 1) % 6
    col = MAPLE if k == 0 else MAPLE_DARK if k in (3,) else MAPLE
    face([r0[k], r0[q], r1[q], r1[k]], col, 0.35, NECK_IN)
face(r0, MAPLE_DARK, 0.3, NECK_IN)
face(r1, MAPLE_DARK, 0.3, NECK_IN)

# frets: dark lines across the board (20 frets, the real spacing); dot inlays between them
fret_y = [Y_NUT - SCALE_LEN * (1 - 2 ** (-k / 12.0)) for k in range(0, 21)]
for k in range(1, 21):
    y = fret_y[k]
    w = neck_w(y) / 2
    face([(-w, y - 0.06, Z_BOARD + 0.02), (w, y - 0.06, Z_BOARD + 0.02), (w, y + 0.06, Z_BOARD + 0.02),
          (-w, y + 0.06, Z_BOARD + 0.02)], FRET, 0.2, (0.0, y, Z_BOARD - 1.0))
for k in (3, 5, 7, 9, 12, 15, 17, 19):
    y = (fret_y[k - 1] + fret_y[k]) / 2
    xs = (-0.55, 0.55) if k == 12 else (0.0,)
    for x in xs:
        flat(ngon(x, y, 0.26, 6, 0.5), Z_BOARD + 0.03, DOT, 0.2)
# the nut: a bone bar across the end of the board
block(rect(-W_NUT / 2, Y_NUT - 0.02, W_NUT / 2, Y_NUT + 0.32), Z_BOARD - 0.4, Z_BOARD + 0.14, NUT, NUT, 0.3)

# --- headstock: the Fender 4-in-line outline traced from Fender_AP_P-Bass_Headstock_20230730.jpg (91 px to the inch,
# nut centre at the origin), widened by HEAD_X; tuners on the bass side (-x) -------------------------------------------
HEAD_X = 1.3
HEAD_RAW = [
    (-0.88, 0.0), (-1.76, 0.27), (-2.03, 0.71), (-1.98, 1.6), (-1.4, 4.2), (-0.9, 6.5), (-0.44, 8.7), (0.11, 9.0),
    (1.2, 8.96), (1.98, 8.4), (2.2, 7.6), (1.87, 6.65), (1.6, 5.8), (1.87, 4.45), (2.03, 3.1), (1.98, 1.8),
    (1.43, 0.6), (0.88, 0.0),
]
Z_HEAD = Z_BOARD - 0.12
HEAD = ccw([(x * HEAD_X, Y_NUT + 0.3 + y) for x, y in HEAD_RAW])
plate(HEAD, Z_HEAD - 0.62, Z_HEAD, MAPLE, MAPLE_DARK, 0.35, bottom=True)
# the short neck piece between the nut and the headstock
block(rect(-W_NUT / 2, Y_NUT + 0.3, W_NUT / 2, Y_NUT + 0.9), Z_HEAD - 0.62, Z_HEAD, MAPLE, MAPLE_DARK, 0.35)

POSTS = [(-0.86, 1.24), (-0.42, 3.15), (0.0, 5.0), (0.44, 7.0)]  # E nearest the nut, G at the top
KEYS = [(-2.64, 2.36), (-2.25, 4.2), (-1.87, 6.1), (-1.43, 8.1)]
POSTS = [(x * HEAD_X, Y_NUT + 0.3 + y) for x, y in POSTS]
KEYS = [(x * HEAD_X - 0.55, Y_NUT + 0.3 + y) for x, y in KEYS]
for (px, py), (kx, ky) in zip(POSTS, KEYS):
    # bushing and post
    block(ngon(px, py, 0.42, 8, 0.2), Z_HEAD, Z_HEAD + 0.1, CHROME, CHROME_DIM, 0.95)
    block(ngon(px, py, 0.22, 6, 0.0), Z_HEAD + 0.1, Z_HEAD + 0.55, CHROME, CHROME_DIM, 0.95)
    # the shaft out from behind the headstock to the key, and the cloverleaf key: a wide flat plate
    zk = Z_HEAD - 0.36
    sx, sy = px - 0.4, py + 0.35
    d = math.hypot(kx - sx, ky - sy)
    ux, uy = (kx - sx) / d, (ky - sy) / d
    w = 0.16
    shaft = [(sx - uy * w, sy + ux * w), (kx - uy * w, ky + ux * w), (kx + uy * w, ky - ux * w),
             (sx + uy * w, sy - ux * w)]
    plate(shaft, zk - 0.12, zk + 0.12, CHROME_DIM, CHROME_DIM, 0.7, bottom=True)
    leaf = [(kx + 0.85 * math.cos(a) + 0.14 * math.cos(3 * a), ky + 0.72 * math.sin(a))
            for a in (2 * math.pi * (k + 0.5) / 10 for k in range(10))]
    plate(leaf, zk - 0.14, zk + 0.14, CHROME, CHROME_DIM, 0.95, bottom=True)
# the string tree for the D and G strings
block(ngon(0.55 * HEAD_X, Y_NUT + 0.3 + 3.0, 0.3, 8), Z_HEAD, Z_HEAD + 0.3, CHROME, CHROME_DIM, 0.95)


# --- strings: flat strips, saddles to nut, then nut to posts --------------------------------------------------------
def strip(p0, p1, w, rgb):
    dx, dy = p1[0] - p0[0], p1[1] - p0[1]
    m = math.hypot(dx, dy)
    nx, ny = -dy / m * w / 2, dx / m * w / 2
    pts = [(p0[0] + nx, p0[1] + ny, p0[2]), (p1[0] + nx, p1[1] + ny, p1[2]), (p1[0] - nx, p1[1] - ny, p1[2]),
           (p0[0] - nx, p0[1] - ny, p0[2])]
    mid = ((p0[0] + p1[0]) / 2, (p0[1] + p1[1]) / 2, (p0[2] + p1[2]) / 2 - 1.0)
    face(pts, rgb, 0.8, mid, two=True)


SW = 0.2  # strings drawn wider than life so they read as lines
for i in range(4):
    a = (SPREAD_SADDLE[i], 3.55, Z_FACE + 0.6)
    b = (SPREAD_NUT[i], Y_NUT + 0.1, Z_BOARD + 0.16)
    strip(a, b, SW, STRING)
    px, py = POSTS[i]
    strip(b, (px, py, Z_HEAD + 0.45), SW, STRING)


# --- lay it across the view, centre, scale, write -----------------------------------------------------------------
def rot(p):
    a = math.radians(TILT_Z)
    x, y, z = p[0] * math.cos(a) - p[1] * math.sin(a), p[0] * math.sin(a) + p[1] * math.cos(a), p[2]
    e = math.radians(FACE_UP)  # about x: the face (+z) turns up towards +y
    y, z = y * math.cos(e) + z * math.sin(e), -y * math.sin(e) + z * math.cos(e)
    b = math.radians(TURN_Y)
    return (x * math.cos(b) + z * math.sin(b), y, -x * math.sin(b) + z * math.cos(b))


for f in faces:
    f["P"] = [list(rot(p)) for p in f["P"]]
    f["n"] = list(rot(f["n"]))

pts = [p for f in faces for p in f["P"]]
lo = [min(p[i] for p in pts) for i in range(3)]
hi = [max(p[i] for p in pts) for i in range(3)]
mid = [(lo[i] + hi[i]) / 2 for i in range(3)]
rad = max(math.sqrt(sum((p[i] - mid[i]) ** 2 for i in range(3))) for p in pts)

out_faces = []
for f in faces:
    P = [[(p[i] - mid[i]) / rad for i in range(3)] for p in f["P"]]
    out_faces.append({"P": P, "emit": False, "gid": 2, "n": f["n"], "rgb": [float(c) for c in f["rgb"]],
                      "spec": f["spec"], "two": f["two"]})

doc = {
    "about": "BASS GUITAR: a low-poly four-string P-style electric bass in ultraviolet with a white pickguard and a "
             "maple neck, laid diagonally, built by studio/scripts/models/bass_guitar_model.py, centred on its "
             "bounding box and scaled to fit a unit sphere",
    "faces": out_faces,
    "fields": {
        "P": "the face's vertices, k x 3 (x, y, z; y up), in drawing order (a fan from the first vertex)",
        "emit": "its colour is used as it is, unshaded",
        "gid": "group id: 1 marks the ground group, anything else is the instrument (the sprite renderer maps it to 2)",
        "n": "its unit normal",
        "rgb": "its colour, 0..255 (floats as the bridge gives them; the sprite renderer truncates each to an int)",
        "spec": "specular strength (0: none)",
        "two": "drawn from both sides",
    },
    "format": 1,
    "name": "BASS GUITAR",
    "note": "faces are in the order the model builder made them; the sprite renderer draws them in this order (a later "
            "face wins only where it is strictly nearer). The strings are thin two-sided faces, not lines.",
    "source": {"module": "studio/scripts/models/bass_guitar_model.py",
               "note": "generated by studio/scripts/models/bass_guitar_model.py from the photo references in "
                       "studio/reference/models/bass_guitar/",
               "size": 1.0, "yaw": 0},
}

here = os.path.dirname(os.path.abspath(__file__))
dst = os.path.join(here, "..", "..", "crates", "bf-content", "data", "models", "BASS GUITAR.json")
with open(dst, "w", encoding="utf-8", newline="\n") as fh:
    text = json.dumps(doc, indent=1, sort_keys=True)
    # one vertex (or normal, or colour) per line, as the other model files have them
    num = r"(-?[0-9.eE+-]+)"
    text = re.sub(r"\[\s*" + num + r",\s*" + num + r",\s*" + num + r"\s*\]", r"[\1, \2, \3]", text)
    fh.write(text + "\n")
print("BASS GUITAR.json:", len(out_faces), "faces")
