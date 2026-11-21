"""Builds the OCTOBASS low-poly model and writes studio/crates/bf-content/data/models/OCTOBASS.json.

Made from the photo references in studio/reference/models/octobass/ (see NOTES.md there): Vuillaume's octobass,
about 3.5 m tall, a double-bass shaped body (sloping shoulders, deep C-bout waist, f-holes, bridge, ebony tailpiece)
in deep umber varnish, a very long ebony fingerboard and neck up to the pegbox and scroll, three pale gut strings,
and the stopping mechanism: a row of seven metal finger levers up the side of the upper neck, long linkage rods
running down the neck to a bank of brass hand levers at the shoulder, and brass foot pedals at the base, the whole
instrument standing on its endpin on a dark plinth.

Axes: x runs left to right as the camera sees it, y is up, z points to the camera (the sprite camera looks from +z).
The mechanism is on the instrument's -x side (the player's side in the photos), which turns towards the camera at
the preview yaw of 30 degrees. The body is exaggerated a little wider than the real one (about 0.4 of the height
instead of 0.35) and the levers a little longer, so the waist and the lever row still read at ball size. The model
is built upright, tipped back towards the camera a few degrees (TILT) so the front plate catches the light, then
centred on its bounding box and scaled to fit a unit sphere.

Standard library only. Run: py studio/scripts/models/octobass_model.py
"""

import json
import math
import os
import re

WOOD = (100.0, 64.0, 36.0)
WOOD_DARK = (52.0, 32.0, 16.0)
VARNISH = (150.0, 98.0, 56.0)
VARNISH_HI = (184.0, 124.0, 72.0)
EBONY = (30.0, 24.0, 22.0)
BRIDGE = (222.0, 192.0, 140.0)
STRING = (236.0, 228.0, 206.0)
BRASS = (226.0, 178.0, 78.0)
BRASS_DIM = (160.0, 118.0, 46.0)
STEEL = (196.0, 200.0, 212.0)
PLINTH = (40.0, 30.0, 26.0)

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


def face(pts, rgb, spec, inside=None, two=False, out=None):
    """A flat face; its normal points away from `inside` (a point inside the solid) or along `out`."""
    n = unit(cross(sub(pts[1], pts[0]), sub(pts[2], pts[0])))
    if inside is not None:
        d = sub(centroid(pts), inside)
    else:
        d = out
    if d is not None and n[0] * d[0] + n[1] * d[1] + n[2] * d[2] < 0:
        n = (-n[0], -n[1], -n[2])
        pts = pts[::-1]
    faces.append({"P": [list(p) for p in pts], "rgb": list(rgb), "spec": spec, "n": list(n), "two": two})


def box8(c, rgb, spec, skip=()):
    """A hexahedron from 8 corners: c[0..3] the bottom ring, c[4..7] the top ring, in the same order."""
    inside = centroid(c)
    sides = {"top": [c[4], c[5], c[6], c[7]], "bottom": [c[0], c[1], c[2], c[3]]}
    for i in range(4):
        j = (i + 1) % 4
        sides["s%d" % i] = [c[i], c[j], c[4 + j], c[4 + i]]
    for name, pts in sides.items():
        if name not in skip:
            face(pts, rgb, spec, inside)


def box(x0, x1, y0, y1, z0, z1, rgb, spec, skip=()):
    c = [(x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1)]
    c += [(x0, y1, z0), (x1, y1, z0), (x1, y1, z1), (x0, y1, z1)]
    box8(c, rgb, spec, skip)


def bar(p0, p1, r, rgb, spec, sides=4, caps=True):
    """A straight bar (rod, lever) from p0 to p1 with a `sides`-sided section of radius r."""
    d = sub(p1, p0)
    L = math.sqrt(d[0] ** 2 + d[1] ** 2 + d[2] ** 2)
    ax = (d[0] / L, d[1] / L, d[2] / L)
    ref = (0.0, 0.0, 1.0) if abs(ax[2]) < 0.9 else (0.0, 1.0, 0.0)
    u = unit(cross(ax, ref))
    v = cross(u, ax)

    def ring(c):
        return [(c[0] + r * (u[0] * math.cos(t) + v[0] * math.sin(t)),
                 c[1] + r * (u[1] * math.cos(t) + v[1] * math.sin(t)),
                 c[2] + r * (u[2] * math.cos(t) + v[2] * math.sin(t)))
                for t in (2 * math.pi * (k + 0.5) / sides for k in range(sides))]

    a, b = ring(p0), ring(p1)
    mid = ((p0[0] + p1[0]) / 2, (p0[1] + p1[1]) / 2, (p0[2] + p1[2]) / 2)
    for k in range(sides):
        q = (k + 1) % sides
        face([a[k], a[q], b[q], b[k]], rgb, spec, mid)
    if caps:
        face(a, rgb, spec, out=(-ax[0], -ax[1], -ax[2]))
        face(b, rgb, spec, out=ax)


def lerp(a, b, t):
    return a + (b - a) * t


# --- the body outline: half-width against height (bass shape: round lower bout, deep waist, sloping shoulders) ---
OUTLINE = [
    (0.00, 0.26), (0.08, 0.50), (0.22, 0.64), (0.45, 0.71), (0.68, 0.69), (0.84, 0.62), (0.95, 0.53),
    (1.08, 0.51), (1.20, 0.54), (1.36, 0.59), (1.52, 0.59), (1.68, 0.53), (1.82, 0.40), (1.94, 0.26),
    (2.04, 0.14),
]
BODY_TOP = OUTLINE[-1][0]
ZF = 0.16       # half the rib depth
ARCH = 0.08     # how far the front plate's middle rises above the ribs
BACK_ARCH = 0.04


def half_w(y):
    for (ya, wa), (yb, wb) in zip(OUTLINE, OUTLINE[1:]):
        if ya <= y <= yb:
            return lerp(wa, wb, (y - ya) / (yb - ya))
    return OUTLINE[-1][1]


def z_top(u):
    return ZF + ARCH * (1.0 - u * u)


def z_back(u):
    return -ZF - BACK_ARCH * (1.0 - u * u)


def plate_point(x, y, lift=0.0):
    """A point on the front plate's arched surface at (x, y), lifted `lift` off it."""
    w = half_w(y)
    u = max(-1.0, min(1.0, x / w))
    return (x, y, z_top(u) + lift)


# columns across the plate (fractions of the half-width); the outer strips are the dark edge and purfling
COLS = [-1.0, -0.86, -0.55, -0.2, 0.2, 0.55, 0.86, 1.0]
ROWS = [y for y, _ in OUTLINE]

for r in range(len(ROWS) - 1):
    ya, yb = ROWS[r], ROWS[r + 1]
    wa, wb = half_w(ya), half_w(yb)
    for c in range(len(COLS) - 1):
        ua, ub = COLS[c], COLS[c + 1]
        if c in (0, len(COLS) - 2):
            col, spec = WOOD_DARK, 0.3
        elif c == 3:
            col, spec = VARNISH, 0.6
        elif c == 4:
            # the varnish's gloss on the bouts, as on the photos' shiny fronts
            col, spec = (VARNISH_HI if r in (2, 3, 9, 10) else VARNISH), 0.6
        else:
            col, spec = WOOD, 0.5
        inside = (0.0, (ya + yb) / 2, 0.0)
        top = [(ua * wa, ya, z_top(ua)), (ub * wa, ya, z_top(ub)), (ub * wb, yb, z_top(ub)), (ua * wb, yb, z_top(ua))]
        face(top, col, spec, inside)
        back = [(ua * wa, ya, z_back(ua)), (ub * wa, ya, z_back(ub)), (ub * wb, yb, z_back(ub)), (ua * wb, yb, z_back(ua))]
        face(back, WOOD if c % 2 else WOOD_DARK, 0.3, inside)
    # the ribs, left and right
    for s in (-1.0, 1.0):
        rib = [(s * wa, ya, ZF), (s * wb, yb, ZF), (s * wb, yb, -ZF), (s * wa, ya, -ZF)]
        face(rib, WOOD, 0.45, (0.0, (ya + yb) / 2, 0.0))

# the caps at the bottom block and the neck heel
for y, sgn in ((ROWS[0], -1.0), (ROWS[-1], 1.0)):
    w = half_w(y)
    ring = [(u * w, y, z_top(u)) for u in COLS] + [(u * w, y, z_back(u)) for u in reversed(COLS)]
    face(ring, WOOD_DARK, 0.2, out=(0.0, sgn, 0.0))


# --- f-holes: two dark S strokes either side of the bridge, the top ends nearer the middle ------------------------
def f_hole(side):
    pts = []
    for k in range(7):
        t = k / 6
        y = lerp(1.30, 0.76, t)
        x = side * (0.19 + 0.10 * t + 0.025 * math.sin(2 * math.pi * t))
        pts.append((x, y))
    hw = 0.022
    for (xa, ya), (xb, yb) in zip(pts, pts[1:]):
        quad = [plate_point(xa - hw, ya, 0.006), plate_point(xb - hw, yb, 0.006),
                plate_point(xb + hw, yb, 0.006), plate_point(xa + hw, ya, 0.006)]
        face(quad, EBONY, 0.0, out=(0.0, 0.0, 1.0))
    # the round eyes at both ends and the notches at the bridge line
    for (x, y), rr in ((pts[0], 0.04), (pts[-1], 0.045)):
        ring = [plate_point(x + rr * math.cos(a), y + rr * math.sin(a), 0.007)
                for a in (2 * math.pi * k / 6 for k in range(6))]
        face(ring, EBONY, 0.0, out=(0.0, 0.0, 1.0))
    xm = side * 0.245
    face([plate_point(xm, 1.01, 0.007), plate_point(xm + side * 0.07, 1.01, 0.007),
          plate_point(xm + side * 0.07, 1.035, 0.007), plate_point(xm, 1.035, 0.007)], EBONY, 0.0, out=(0.0, 0.0, 1.0))


f_hole(-1.0)
f_hole(1.0)

# --- the strings' line: tailpiece -> bridge -> nut ---------------------------------------------------------------
Y_TAIL, Y_BRIDGE, Y_NUT = 0.56, 1.02, 3.16
Z_TAIL, Z_BRIDGE, Z_NUT = ZF + ARCH + 0.09, ZF + ARCH + 0.22, 0.21
FB_Y0 = 1.28  # the fingerboard's lower end


def z_string(y):
    if y <= Y_BRIDGE:
        return lerp(Z_TAIL, Z_BRIDGE, (y - Y_TAIL) / (Y_BRIDGE - Y_TAIL))
    return lerp(Z_BRIDGE, Z_NUT, (y - Y_BRIDGE) / (Y_NUT - Y_BRIDGE))


def z_board(y):
    """The fingerboard's playing surface, just under the strings."""
    return z_string(y) - 0.035 - 0.02 * (Y_NUT - y) / (Y_NUT - FB_Y0)


def board_hw(y):
    return lerp(0.088, 0.058, (y - FB_Y0) / (Y_NUT - FB_Y0))


# --- the bridge: a pale maple trapezoid standing on the plate between the f-hole notches ---------------------------
bz0 = z_top(0.0) - 0.01
box8([(-0.15, Y_BRIDGE - 0.015, bz0), (0.15, Y_BRIDGE - 0.015, bz0), (0.15, Y_BRIDGE + 0.015, bz0),
      (-0.15, Y_BRIDGE + 0.015, bz0),
      (-0.075, Y_BRIDGE - 0.01, Z_BRIDGE), (0.075, Y_BRIDGE - 0.01, Z_BRIDGE), (0.075, Y_BRIDGE + 0.01, Z_BRIDGE),
      (-0.075, Y_BRIDGE + 0.01, Z_BRIDGE)], BRIDGE, 0.3)
# the bridge's two feet, a step wider on the plate
box(-0.17, -0.09, Y_BRIDGE - 0.02, Y_BRIDGE + 0.02, bz0 - 0.01, bz0 + 0.03, BRIDGE, 0.3)
box(0.09, 0.17, Y_BRIDGE - 0.02, Y_BRIDGE + 0.02, bz0 - 0.01, bz0 + 0.03, BRIDGE, 0.3)

# --- the tailpiece: ebony, from the bottom block up to the strings' anchor ----------------------------------------
tz = z_top(0.0)
box8([(-0.035, 0.10, tz - 0.01), (0.035, 0.10, tz - 0.01), (0.035, 0.10, tz + 0.04), (-0.035, 0.10, tz + 0.04),
      (-0.09, Y_TAIL, tz - 0.01), (0.09, Y_TAIL, tz - 0.01), (0.09, Y_TAIL, Z_TAIL), (-0.09, Y_TAIL, Z_TAIL)],
     EBONY, 0.4)
# the saddle on the bottom block where the tail gut wraps over
box(-0.08, 0.08, -0.01, 0.05, z_back(0.0) + 0.02, ZF + 0.03, EBONY, 0.3)

# --- neck, fingerboard, pegbox and scroll ---------------------------------------------------------------------------
NECK_Y0 = BODY_TOP - 0.08
# the neck under the fingerboard: from the heel at the shoulders up to the nut, a little narrower than the board
nk = [(-0.055, NECK_Y0, -0.06), (0.055, NECK_Y0, -0.06), (0.06, NECK_Y0, z_board(NECK_Y0) - 0.05),
      (-0.06, NECK_Y0, z_board(NECK_Y0) - 0.05),
      (-0.045, Y_NUT, 0.0), (0.045, Y_NUT, 0.0), (0.05, Y_NUT, z_board(Y_NUT) - 0.05),
      (-0.05, Y_NUT, z_board(Y_NUT) - 0.05)]
box8(nk, WOOD, 0.5)
# the neck heel / button on the shoulders
box(-0.10, 0.10, BODY_TOP - 0.16, BODY_TOP + 0.04, -0.10, ZF + 0.06, WOOD_DARK, 0.3)

# the fingerboard: ebony, tapering, its surface just under the strings
fb = []
for y in (FB_Y0, Y_NUT):
    hw = board_hw(y)
    zs = z_board(y)
    fb.append([(-hw, y, zs - 0.05), (hw, y, zs - 0.05), (hw, y, zs), (-hw, y, zs)])
box8(fb[0] + fb[1], EBONY, 0.5)
# the nut
box(-0.06, 0.06, Y_NUT - 0.01, Y_NUT + 0.02, z_board(Y_NUT) - 0.05, Z_NUT + 0.01, BRIDGE, 0.2)

# the pegbox: an open box above the nut, leaning a little back
PB_Y0, PB_Y1 = Y_NUT, Y_NUT + 0.30
box8([(-0.055, PB_Y0, -0.02), (0.055, PB_Y0, -0.02), (0.055, PB_Y0, z_board(Y_NUT)), (-0.055, PB_Y0, z_board(Y_NUT)),
      (-0.05, PB_Y1, -0.07), (0.05, PB_Y1, -0.07), (0.05, PB_Y1, 0.11), (-0.05, PB_Y1, 0.11)], WOOD, 0.5)
# the pegbox's dark open front
face([(-0.035, PB_Y0 + 0.03, z_board(Y_NUT) + 0.002), (0.035, PB_Y0 + 0.03, z_board(Y_NUT) + 0.002),
      (0.032, PB_Y1 - 0.03, 0.112), (-0.032, PB_Y1 - 0.03, 0.112)], WOOD_DARK, 0.0, out=(0.0, 0.0, 1.0))

# the scroll: an octagonal drum turned sideways, with its darker spiral eye on each cheek
SC = (0.0, PB_Y1 + 0.075, 0.01)
SR, SH = 0.10, 0.058
ring_l, ring_r = [], []
for k in range(8):
    a = 2 * math.pi * (k + 0.5) / 8
    yy, zz = SC[1] + SR * math.sin(a), SC[2] + SR * math.cos(a)
    ring_l.append((-SH, yy, zz))
    ring_r.append((SH, yy, zz))
for k in range(8):
    q = (k + 1) % 8
    face([ring_l[k], ring_l[q], ring_r[q], ring_r[k]], VARNISH if k % 2 else WOOD, 0.5, SC)
face(ring_l, WOOD, 0.4, out=(-1.0, 0.0, 0.0))
face(ring_r, WOOD, 0.4, out=(1.0, 0.0, 0.0))
for s in (-1.0, 1.0):
    eye = [(s * (SH + 0.004), SC[1] + 0.055 * math.sin(a), SC[2] + 0.055 * math.cos(a))
           for a in (2 * math.pi * (k + 0.5) / 6 for k in range(6))]
    face(eye, WOOD_DARK, 0.0, out=(s, 0.0, 0.0))

# the tuning machines: steel shafts out of the pegbox's cheeks with brass keys (two on the left, one on the right)
for side, y in ((-1.0, PB_Y0 + 0.08), (-1.0, PB_Y0 + 0.20), (1.0, PB_Y0 + 0.14)):
    zc = lerp(0.04, 0.02, (y - PB_Y0) / (PB_Y1 - PB_Y0))
    bar((side * 0.05, y, zc), (side * 0.15, y, zc), 0.014, STEEL, 0.9)
    box(side * 0.15 - 0.02 if side > 0 else -0.19, side * 0.19 if side > 0 else -0.15 + 0.02,
        y - 0.035, y + 0.035, zc - 0.02, zc + 0.02, BRASS, 0.9)

# --- the three strings: pale strips from the tailpiece over the bridge to the nut -------------------------------
for k, (xt, xb, xn) in enumerate(((-0.05, -0.052, -0.03), (0.0, 0.0, 0.0), (0.05, 0.052, 0.03))):
    hw = 0.011 - 0.002 * k
    for (ya, xa), (yb, xb2) in (((Y_TAIL, xt), (Y_BRIDGE, xb)), ((Y_BRIDGE, xb), (Y_NUT, xn))):
        za, zb = z_string(ya) + 0.004, z_string(yb) + 0.004
        face([(xa - hw, ya, za), (xa + hw, ya, za), (xb2 + hw, yb, zb), (xb2 - hw, yb, zb)], STRING, 0.3,
             out=(0.0, 0.3, 1.0), two=True)

# --- the stopping mechanism ---------------------------------------------------------------------------------------
# a steel rail along the left edge of the upper fingerboard carrying seven finger levers; each lever is a steel
# stop across the board (pressing the strings down) and a brass arm sticking out to the left
LEVER_YS = [2.42 + 0.105 * k for k in range(7)]
RAIL_X = -board_hw(2.8) - 0.025
bar((RAIL_X, LEVER_YS[0] - 0.06, z_board(LEVER_YS[0] - 0.06) - 0.01),
    (RAIL_X, LEVER_YS[-1] + 0.06, z_board(LEVER_YS[-1] + 0.06) - 0.01), 0.022, STEEL, 0.9)
for k, y in enumerate(LEVER_YS):
    zb = z_board(y)
    hw = board_hw(y)
    # the stop across the fingerboard, just above the strings
    box(RAIL_X, hw + 0.01, y - 0.012, y + 0.012, zb + 0.03, zb + 0.055, STEEL, 0.9)
    # the arm out to the side, angled a little down and towards the camera, with a knob on its end
    tip = (RAIL_X - 0.24, y - 0.05, zb + 0.03)
    bar((RAIL_X, y, zb + 0.01), tip, 0.018, BRASS, 0.95)
    box(tip[0] - 0.035, tip[0] + 0.005, tip[1] - 0.025, tip[1] + 0.025, tip[2] - 0.025, tip[2] + 0.025, BRASS_DIM, 0.7)

# the long linkage rods: from the lever row down beside the neck to the hand levers on the shoulder
HAND_Y = BODY_TOP - 0.20
for k, x in enumerate((-0.12, -0.16, -0.20)):
    ytop = LEVER_YS[2 * k + 1]
    bar((x, HAND_Y, 0.22), (x, ytop, z_board(ytop) - 0.005), 0.011, STEEL, 0.9, caps=False)
# the hand levers' pivot block on the left shoulder and three brass handles fanned out to the left
box(-0.30, -0.08, HAND_Y - 0.06, HAND_Y + 0.06, ZF - 0.02, 0.27, STEEL, 0.8)
for k, (dx, dy) in enumerate(((-0.42, 0.20), (-0.48, 0.07), (-0.44, -0.07))):
    p0 = (-0.26, HAND_Y + 0.03 * (1 - k), 0.25)
    p1 = (p0[0] + dx, p0[1] + dy, 0.30)
    bar(p0, p1, 0.022, BRASS, 0.95)
    box(p1[0] - 0.04, p1[0] + 0.03, p1[1] - 0.035, p1[1] + 0.035, p1[2] - 0.03, p1[2] + 0.03, BRASS_DIM, 0.8)

# --- endpin, plinth and the foot pedals at the base ---------------------------------------------------------------
bar((0.0, -0.20, 0.0), (0.0, 0.02, 0.0), 0.03, STEEL, 0.9)
box(-0.55, 0.55, -0.32, -0.20, -0.36, 0.42, PLINTH, 0.3)
box(-0.48, 0.48, -0.20, -0.17, -0.30, 0.36, WOOD_DARK, 0.3)
for x in (-0.20, 0.0, 0.20):
    # a pedal arm from its hinge at the plinth's back half out over the front, with a wide brass foot plate
    bar((x, -0.15, -0.05), (x, -0.13, 0.48), 0.022, STEEL, 0.9)
    box(x - 0.07, x + 0.07, -0.16, -0.10, 0.44, 0.60, BRASS, 0.9)
# the pedal rods rising up the left rib to the mechanism
bar((-0.20, -0.13, 0.10), (-0.66, 0.42, 0.10), 0.012, STEEL, 0.9, caps=False)

# --- tip back towards the camera, centre, scale, write -------------------------------------------------------------
TILT = math.radians(8)
ct, st = math.cos(TILT), math.sin(TILT)


def tilt(p):
    # about the x axis: the top (+y) leans towards the camera (+z)
    return (p[0], p[1] * ct - p[2] * st, p[1] * st + p[2] * ct)


for f in faces:
    f["P"] = [list(tilt(p)) for p in f["P"]]
    f["n"] = list(tilt(f["n"]))

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
    "about": "OCTOBASS: a low-poly 3.5 m octobass in umber varnish with its lever mechanism, built by "
             "studio/scripts/models/octobass_model.py, centred on its bounding box and scaled to fit a unit sphere",
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
    "name": "OCTOBASS",
    "note": "faces are in the order the model builder made them; the sprite renderer draws them in this order (a later "
            "face wins only where it is strictly nearer). Lines (strings) are not part of the model.",
    "source": {"module": "studio/scripts/models/octobass_model.py",
               "note": "generated by studio/scripts/models/octobass_model.py from the photo references in "
                       "studio/reference/models/octobass/",
               "size": 1.0, "yaw": 0},
}

here = os.path.dirname(os.path.abspath(__file__))
dst = os.path.join(here, "..", "..", "crates", "bf-content", "data", "models", "OCTOBASS.json")
with open(dst, "w", encoding="utf-8", newline="\n") as fh:
    text = json.dumps(doc, indent=1, sort_keys=True)
    # one vertex (or normal, or colour) per line, as the other model files have them
    num = r"(-?[0-9.eE+-]+)"
    text = re.sub(r"\[\s*" + num + r",\s*" + num + r",\s*" + num + r"\s*\]", r"[\1, \2, \3]", text)
    fh.write(text + "\n")
print("OCTOBASS.json:", len(out_faces), "faces")
