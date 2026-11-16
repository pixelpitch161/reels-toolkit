"""Builds the SNARE DRUM low-poly model and writes studio/crates/bf-content/data/models/SNARE DRUM.json.

Made from the photo references in studio/reference/models/snare_drum/ (see NOTES.md there): a 14-inch concert/kit
snare with a deep shell in cherry sparkle, ten chrome tube lugs with tension rods up to the top hoop and down to the
bottom hoop, two triple-flange chrome hoops, a white coated batter head, a black snare strainer with its chrome
throw-off lever, and a pair of hickory sticks crossed on the head.

Axes: x runs left to right as the camera sees it, y is up, z points to the camera (the sprite camera looks from +z).
The drum is built upright, then tipped towards the camera (TILT) so the white head reads as a wide oval at ball size.
The model is centred on its bounding box and scaled to fit a unit sphere.

Standard library only. Run: python studio/scripts/models/snare_drum_model.py
"""

import json
import math
import os
import re

CHERRY = (255.0, 48.0, 128.0)
CHERRY_DARK = (150.0, 18.0, 70.0)
CHROME = (214.0, 218.0, 228.0)
CHROME_DIM = (150.0, 154.0, 166.0)
HEAD = (240.0, 236.0, 226.0)
STRAINER = (30.0, 30.0, 34.0)
HICKORY = (226.0, 192.0, 140.0)
TIP = (196.0, 150.0, 96.0)

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
    """A flat face; its normal points away from `inside` (a point inside the solid)."""
    n = unit(cross(sub(pts[1], pts[0]), sub(pts[2], pts[0])))
    if inside is not None:
        c = centroid(pts)
        d = sub(c, inside)
        if n[0] * d[0] + n[1] * d[1] + n[2] * d[2] < 0:
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


def polar(r, a, y):
    """A point on a circle around the drum's axis; angle 0 faces the camera (+z), positive angles go to +x."""
    return (r * math.sin(a), y, r * math.cos(a))


def radial_box(a, r0, r1, half_w, y0, y1, rgb, spec, skip=()):
    """A box standing on the shell at angle a: from radius r0 out to r1, half_w wide along the tangent."""
    t = (math.cos(a), 0.0, -math.sin(a))  # tangent

    def p(r, s, y):
        b = polar(r, a, y)
        return (b[0] + t[0] * s, y, b[2] + t[2] * s)

    c = [p(r0, -half_w, y0), p(r0, half_w, y0), p(r1, half_w, y0), p(r1, -half_w, y0)]
    c += [(q[0], y1, q[2]) for q in c]
    box8(c, rgb, spec, skip)


# --- dimensions (14 x 6.5 inch snare: radius 1, depth 0.93 of the radius) ---------------------------------------
N = 24  # segments around
R = 1.0
DEPTH = 0.88
Y0, Y1 = -DEPTH / 2, DEPTH / 2
HOOP_R = 1.055
HOOP_H = 0.16
LUGS = 10


def ang(k):
    return 2 * math.pi * k / N


def sparkle(k, row):
    """A fixed scatter of darker sparkle facets over the cherry shell."""
    h = (k * 7 + row * 13) % 11
    return CHERRY_DARK if h in (0, 4) else CHERRY


# --- shell: N segments x 3 rows, the sparkle scattered over them --------------------------------------------------
ROWS = 3
for row in range(ROWS):
    ya = Y0 + 0.06 + (DEPTH - 0.12) * row / ROWS
    yb = Y0 + 0.06 + (DEPTH - 0.12) * (row + 1) / ROWS
    for k in range(N):
        a, b = ang(k), ang(k + 1)
        face([polar(R, a, ya), polar(R, b, ya), polar(R, b, yb), polar(R, a, yb)], sparkle(k, row), 0.55,
             (0.0, (ya + yb) / 2, 0.0))

# --- the hoops: an outer band, an inner lip and a rolled top edge (triple flange) ----------------------------------
for top in (True, False):
    if top:
        ya, yb = Y1 + 0.07 - HOOP_H, Y1 + 0.07
    else:
        ya, yb = Y0 - 0.07, Y0 - 0.07 + HOOP_H
    for k in range(N):
        a, b = ang(k), ang(k + 1)
        # outer band
        face([polar(HOOP_R, a, ya), polar(HOOP_R, b, ya), polar(HOOP_R, b, yb), polar(HOOP_R, a, yb)], CHROME, 0.95,
             (0.0, (ya + yb) / 2, 0.0))
        # the flange's edge (the rim, facing away from the shell) and the inner lip
        ye = yb if top else ya
        face([polar(R - 0.02, a, ye), polar(R - 0.02, b, ye), polar(HOOP_R, b, ye), polar(HOOP_R, a, ye)], CHROME, 0.95,
             (0.0, ye - (0.1 if top else -0.1), 0.0))
        yl = ye - 0.035 if top else ye + 0.035
        face([polar(R - 0.02, a, yl), polar(R - 0.02, b, yl), polar(R - 0.02, b, ye), polar(R - 0.02, a, ye)],
             CHROME_DIM, 0.6, (0.0, (yl + ye) / 2, 0.0), two=True)
        # a flange step half way down the band, so the hoop reads as pressed steel, not a flat ring
        ym = (ya + yb) / 2 + (-0.015 if top else 0.015)
        face([polar(HOOP_R, a, ym), polar(HOOP_R, b, ym), polar(HOOP_R + 0.03, b, ym), polar(HOOP_R + 0.03, a, ym)],
             CHROME_DIM, 0.8, (0.0, ym + (0.1 if top else -0.1), 0.0))

# --- the batter head: a white coated disc just below the top hoop's rim -----------------------------------------
HEAD_Y = Y1 + 0.02
centre = (0.0, HEAD_Y, 0.0)
for k in range(N):
    a, b = ang(k), ang(k + 1)
    face([centre, polar(R - 0.02, a, HEAD_Y), polar(R - 0.02, b, HEAD_Y)], HEAD, 0.12, (0.0, HEAD_Y - 1.0, 0.0))
# the snare side head under the drum (closes the drum when seen from below)
SNARE_Y = Y0 - 0.02
for k in range(N):
    a, b = ang(k), ang(k + 1)
    face([(0.0, SNARE_Y, 0.0), polar(R - 0.02, a, SNARE_Y), polar(R - 0.02, b, SNARE_Y)], HEAD, 0.05,
         (0.0, SNARE_Y + 1.0, 0.0))

# --- the lugs and tension rods ----------------------------------------------------------------------------------
LUG_Y0, LUG_Y1 = -0.17, 0.17
STRAINER_A = math.radians(-62)
for i in range(LUGS):
    a = 2 * math.pi * (i + 0.5) / LUGS
    # tube lug: a chrome block with bevelled ends
    radial_box(a, R - 0.01, R + 0.07, 0.045, LUG_Y0, LUG_Y1, CHROME, 0.95)
    radial_box(a, R - 0.01, R + 0.045, 0.035, LUG_Y1, LUG_Y1 + 0.05, CHROME_DIM, 0.8, skip=("bottom",))
    radial_box(a, R - 0.01, R + 0.045, 0.035, LUG_Y0 - 0.05, LUG_Y0, CHROME_DIM, 0.8, skip=("top",))
    # tension rods: up through the top hoop's claw, down through the bottom's
    radial_box(a, R + 0.035, R + 0.06, 0.012, LUG_Y1 + 0.05, Y1 + 0.08, CHROME_DIM, 0.7, skip=("top", "bottom"))
    radial_box(a, R + 0.035, R + 0.06, 0.012, Y0 - 0.08, LUG_Y0 - 0.05, CHROME_DIM, 0.7, skip=("top", "bottom"))
    # the rod heads above the top hoop and below the bottom one
    radial_box(a, R + 0.02, R + 0.075, 0.028, Y1 + 0.06, Y1 + 0.11, CHROME, 0.95)
    radial_box(a, R + 0.02, R + 0.075, 0.028, Y0 - 0.11, Y0 - 0.06, CHROME, 0.95)

# --- the snare strainer (throw-off) between two lugs, and the butt plate opposite ----------------------------------
radial_box(STRAINER_A, R - 0.01, R + 0.11, 0.075, -0.20, 0.12, STRAINER, 0.35)
# the chrome throw-off lever, standing up from the strainer's top
radial_box(STRAINER_A, R + 0.05, R + 0.16, 0.03, 0.06, 0.30, CHROME, 0.95)
# the tension knob on top of the strainer
radial_box(STRAINER_A, R + 0.02, R + 0.08, 0.035, 0.12, 0.19, CHROME, 0.95)
radial_box(STRAINER_A + math.pi, R - 0.01, R + 0.08, 0.06, -0.24, -0.04, STRAINER, 0.35)
# the air vent badge on the front of the shell
radial_box(math.radians(10), R - 0.01, R + 0.02, 0.05, -0.05, 0.05, CHROME, 0.95)


# --- the sticks: two hickory sticks crossed on the head -----------------------------------------------------------
def stick(p0, p1, r, rgb, tip_rgb, sides=6):
    """A hexagonal stick from butt p0 to tip p1, tapering over its last fifth to a small bead."""
    d = sub(p1, p0)
    L = math.sqrt(d[0] ** 2 + d[1] ** 2 + d[2] ** 2)
    ax = (d[0] / L, d[1] / L, d[2] / L)
    up = (0.0, 1.0, 0.0)
    u = unit(cross(ax, up))
    v = cross(u, ax)

    def ring(c, rr):
        return [(c[0] + rr * (u[0] * math.cos(t) + v[0] * math.sin(t)),
                 c[1] + rr * (u[1] * math.cos(t) + v[1] * math.sin(t)),
                 c[2] + rr * (u[2] * math.cos(t) + v[2] * math.sin(t)))
                for t in (2 * math.pi * (k + 0.5) / sides for k in range(sides))]

    def at(s):
        return (p0[0] + d[0] * s, p0[1] + d[1] * s, p0[2] + d[2] * s)

    stations = [(0.0, r), (0.78, r), (0.95, r * 0.45), (0.97, r * 0.7), (1.0, r * 0.25)]
    rings = [ring(at(s), rr) for s, rr in stations]
    face(rings[0], rgb, 0.3, at(0.1))
    for j in range(len(rings) - 1):
        col = tip_rgb if j >= 2 else rgb
        mid = at((stations[j][0] + stations[j + 1][0]) / 2)
        for k in range(sides):
            q = (k + 1) % sides
            face([rings[j][k], rings[j][q], rings[j + 1][q], rings[j + 1][k]], col, 0.3, mid)
    face(rings[-1], tip_rgb, 0.3, at(0.9))


SR = 0.055
# stick A lies on the head, butt off the back-left rim, tip resting towards the front right
stick((-1.05, HEAD_Y + 0.06 + SR, -0.55), (0.80, HEAD_Y + SR, 0.55), SR, HICKORY, TIP)
# stick B crosses over it, butt off the back-right rim
stick((1.05, HEAD_Y + 0.06 + SR, -0.60), (-0.75, HEAD_Y + 2.2 * SR, 0.50), SR, HICKORY, TIP)

# --- tip towards the camera, centre, scale, write ---------------------------------------------------------------
TILT = math.radians(20)
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
    "about": "SNARE DRUM: a low-poly 14-inch snare in cherry sparkle with crossed sticks, built by "
             "studio/scripts/models/snare_drum_model.py, centred on its bounding box and scaled to fit a unit sphere",
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
    "name": "SNARE DRUM",
    "note": "faces are in the order the model builder made them; the sprite renderer draws them in this order (a later "
            "face wins only where it is strictly nearer). Lines (strings) are not part of the model.",
    "source": {"module": "studio/scripts/models/snare_drum_model.py",
               "note": "generated by studio/scripts/models/snare_drum_model.py from the photo references in "
                       "studio/reference/models/snare_drum/",
               "size": 1.0, "yaw": 0},
}

here = os.path.dirname(os.path.abspath(__file__))
dst = os.path.join(here, "..", "..", "crates", "bf-content", "data", "models", "SNARE DRUM.json")
with open(dst, "w", encoding="utf-8", newline="\n") as fh:
    text = json.dumps(doc, indent=1, sort_keys=True)
    # one vertex (or normal, or colour) per line, as the other model files have them
    num = r"(-?[0-9.eE+-]+)"
    text = re.sub(r"\[\s*" + num + r",\s*" + num + r",\s*" + num + r"\s*\]", r"[\1, \2, \3]", text)
    fh.write(text + "\n")
print("SNARE DRUM.json:", len(out_faces), "faces")
