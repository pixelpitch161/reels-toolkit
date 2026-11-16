"""Builds the PICCOLO low-poly model and writes studio/crates/bf-content/data/models/PICCOLO.json.

Made from the photo references in studio/reference/models/piccolo/ (see NOTES.md there): a concert piccolo in
grenadilla (matte black wood) with silver keywork, in two joints only (headjoint and body, no foot joint). The
headjoint has a silver crown and ring at its end, a gentle bulge round the embouchure and a silver lip plate (made a
little bigger than life so it reads at a small radius); a silver tenon ring joins it to the conical body, which
narrows towards the far end and its silver end ring. Along the body run the keys: two hands of open cups (three and
three) with a small trill cup near the end, all a little oversized, on arms from one long rod held by posts, and the
tall curved lever standing up between the hands.

The instrument is built along its own axis, then laid diagonally across the view: the head end up and to the left,
the end ring down and to the right, the cups and the lip plate facing the camera and a little up.

Axes: x runs left to right, y is up, z points to the viewer (the sprite camera looks from +z). The model is centred
on its bounding box and scaled to fit a unit sphere.

Standard library only. Run: python studio/scripts/models/piccolo_model.py
"""

import json
import math
import os
import re

WOOD = (34.0, 30.0, 34.0)  # grenadilla: matte black with a hint of purple-brown
WOOD_SPEC = 0.4  # the polished sheen in the photos
SILVER = (205.0, 210.0, 220.0)
SILVER_SPEC = 0.9
HOLE = (10.0, 8.0, 12.0)

# How it lies in the view: turned about z (screen diagonal) and a little about y, before the sprite camera's own yaw.
TILT_Z = -36.0  # degrees: the head end rises to the left
TURN_Y = -30.0  # cancels the roster view yaw of 30, so it lies broadside to the camera
FACE_UP = 35.0  # the cups and the lip plate face the viewer, tipped this far up

faces = []


def add(a, b):
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def mul(a, s):
    return (a[0] * s, a[1] * s, a[2] * s)


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def unit(v):
    m = math.sqrt(dot(v, v))
    return (v[0] / m, v[1] / m, v[2] / m)


def centroid(pts):
    k = len(pts)
    return (sum(p[0] for p in pts) / k, sum(p[1] for p in pts) / k, sum(p[2] for p in pts) / k)


def face(pts, rgb, spec, inside=None, two=False):
    """A flat face; its normal points away from `inside` (a point inside the solid)."""
    n = unit(cross(sub(pts[1], pts[0]), sub(pts[2], pts[0])))
    if inside is not None:
        if dot(n, sub(centroid(pts), inside)) < 0:
            n = mul(n, -1.0)
            pts = pts[::-1]
    faces.append({"P": [list(p) for p in pts], "rgb": list(rgb), "spec": spec, "n": list(n), "two": two})


# --- the instrument's own frame ---------------------------------------------------------------------------------
# X: along the tube, head (x < 0) to end (x > 0). F: the side the cups and the lip plate face. G: the third axis.
X = (1.0, 0.0, 0.0)
F = unit((0.0, math.sin(math.radians(FACE_UP)), math.cos(math.radians(FACE_UP))))
G = cross(F, X)


def at(x, ang, r):
    """A point x along the axis, `r` out from it at angle `ang` (radians) round it, 0 = facing F."""
    return add(mul(X, x), add(mul(F, r * math.cos(ang)), mul(G, r * math.sin(ang))))


SIDES = 10


def frustum(x0, r0, x1, r1, rgb, spec, cap0=False, cap1=False, sides=SIDES):
    """A tube section from x0 (radius r0) to x1 (radius r1), with optional flat end caps."""
    inside = mul(X, (x0 + x1) / 2)
    angs = [2 * math.pi * (k + 0.5) / sides for k in range(sides)]
    for k in range(sides):
        a, b = angs[k], angs[(k + 1) % sides]
        face([at(x0, a, r0), at(x0, b, r0), at(x1, b, r1), at(x1, a, r1)], rgb, spec, inside)
    if cap0:
        face([at(x0, a, r0) for a in angs], rgb, spec, mul(X, x0 + 1.0))
    if cap1:
        face([at(x1, a, r1) for a in angs], rgb, spec, mul(X, x1 - 1.0))


def disc(center, normal, e1, e2, ra, rb, thick, rgb, spec, sides=12, top_rgb=None):
    """A flat oval slab: centre, the normal of its top, two in-plane axes and their half-sizes, its thickness."""
    top = [add(center, add(mul(e1, ra * math.cos(2 * math.pi * k / sides)), mul(e2, rb * math.sin(2 * math.pi * k /
                                                                                                      sides))))
           for k in range(sides)]
    bot = [sub(p, mul(normal, thick)) for p in top]
    inside = sub(center, mul(normal, thick / 2))
    face(top, top_rgb or rgb, spec, inside)
    for k in range(sides):
        j = (k + 1) % sides
        face([bot[k], bot[j], top[j], top[k]], rgb, spec, inside)


def beam(p0, p1, w, h, up, rgb, spec):
    """A thin box from p0 to p1, `w` wide and `h` deep, its depth along `up`."""
    d = unit(sub(p1, p0))
    side = unit(cross(d, up))
    u = unit(cross(side, d))
    c = []
    for p in (p0, p1):
        for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            c.append(add(p, add(mul(side, sx * w / 2), mul(u, sy * h / 2))))
    inside = mul(add(p0, p1), 0.5)
    face([c[0], c[1], c[2], c[3]], rgb, spec, inside)
    face([c[4], c[5], c[6], c[7]], rgb, spec, inside)
    for k in range(4):
        j = (k + 1) % 4
        face([c[k], c[j], c[4 + j], c[4 + k]], rgb, spec, inside)


# --- the tube ---------------------------------------------------------------------------------------------------
# Real piccolo: about 32 cm long, 1.5 to 1.8 cm across (about 1 : 20). Drawn at 1 : 12 so it stays readable.
L = 2.0
X0, X1 = -L / 2, L / 2


def xt(t):
    return X0 + L * t


# headjoint (about 37 % of the length, crown to tenon ring)
frustum(xt(0.000), 0.064, xt(0.018), 0.072, SILVER, SILVER_SPEC, cap0=True)  # crown, rounded a little
frustum(xt(0.018), 0.080, xt(0.040), 0.080, SILVER, SILVER_SPEC, cap0=True)  # head ring
frustum(xt(0.040), 0.078, xt(0.100), 0.083, WOOD, WOOD_SPEC)
frustum(xt(0.100), 0.083, xt(0.170), 0.086, WOOD, WOOD_SPEC)  # the bulge round the embouchure
frustum(xt(0.170), 0.086, xt(0.250), 0.078, WOOD, WOOD_SPEC)
frustum(xt(0.250), 0.078, xt(0.345), 0.080, WOOD, WOOD_SPEC)
frustum(xt(0.345), 0.090, xt(0.375), 0.090, SILVER, SILVER_SPEC, cap0=True, cap1=True)  # tenon ring
# body: conical, narrowing to the end
frustum(xt(0.375), 0.086, xt(0.420), 0.086, WOOD, WOOD_SPEC)
frustum(xt(0.420), 0.084, xt(0.965), 0.066, WOOD, WOOD_SPEC)
frustum(xt(0.965), 0.074, xt(1.000), 0.074, SILVER, SILVER_SPEC, cap0=True)  # end ring
# the open end: a dark bore just proud of the end ring's cap
face([at(xt(1.0) + 0.002, 2 * math.pi * (k + 0.5) / SIDES, 0.05) for k in range(SIDES)], HOLE, 0.0, mul(X, 0.0))

# lip plate round the embouchure hole: an oval silver plate on the F side, a dark oval hole in it
EMB = xt(0.135)
r_here = 0.086 * math.cos(math.pi / SIDES)
plate_c = add(mul(X, EMB), mul(F, r_here + 0.016))
disc(plate_c, F, X, G, 0.085, 0.070, 0.030, SILVER, SILVER_SPEC, sides=14)
disc(add(plate_c, mul(F, 0.003)), F, X, G, 0.034, 0.024, 0.004, HOLE, 0.0, sides=10)

# --- the keys ---------------------------------------------------------------------------------------------------
CUP_R = 0.072
ROD_ANG = math.radians(80)  # the rod runs along the body this far round from F (the lower, near side)
ROD_OUT = 0.125
ROD_R = 0.018
ROD_T0, ROD_T1 = 0.43, 0.955


def body_r(t):
    return 0.084 + (0.066 - 0.084) * (t - 0.42) / (0.965 - 0.42)


def rod_pt(t):
    return at(xt(t), ROD_ANG, ROD_OUT)


# the long rod and its posts
p_rod0, p_rod1 = rod_pt(ROD_T0), rod_pt(ROD_T1)
beam(p_rod0, p_rod1, ROD_R * 2, ROD_R * 2, F, SILVER, SILVER_SPEC)
for t in (0.44, 0.66, 0.945):
    beam(at(xt(t), ROD_ANG, body_r(t) * 0.9), rod_pt(t), 0.026, 0.026, X, SILVER, SILVER_SPEC)

# the cups (closed, as in the photos): left hand three, right hand three, and a small trill cup near the end
CUPS = [(0.48, 1.0), (0.56, 1.0), (0.635, 1.0), (0.735, 1.0), (0.81, 1.0), (0.885, 1.0), (0.948, 0.62)]
for t, s in CUPS:
    r_b = body_r(t)
    up = unit(add(mul(F, math.cos(math.radians(8))), mul(G, math.sin(math.radians(8)))))
    c = add(mul(X, xt(t)), mul(up, r_b + 0.020))
    disc(c, up, X, unit(cross(up, X)), CUP_R * s, CUP_R * s, 0.026, SILVER, SILVER_SPEC, sides=10)
    # the arm from the rod to the cup
    beam(rod_pt(t), sub(c, mul(up, 0.012)), 0.020, 0.014, X, SILVER, SILVER_SPEC)

# the tall curved lever between the hands: it rises from the rod and bends back over the body, ending in a spoon
# (it stands off the rod side, the top of the view, so it shows above the tube's outline instead of pointing at the camera)
t = 0.685
up = unit(add(mul(G, 0.8), mul(F, 0.6)))
p0 = rod_pt(t)
p1 = add(mul(X, xt(t + 0.005)), mul(up, body_r(t) + 0.13))
p2 = add(mul(X, xt(t - 0.02)), mul(up, body_r(t) + 0.235))
p3 = add(mul(X, xt(t - 0.055)), mul(up, body_r(t) + 0.27))
beam(p0, p1, 0.042, 0.020, F, SILVER, SILVER_SPEC)
beam(p1, p2, 0.042, 0.020, F, SILVER, SILVER_SPEC)
beam(p2, p3, 0.042, 0.020, F, SILVER, SILVER_SPEC)
disc(add(p3, mul(X, -0.022)), unit(add(up, mul(X, -0.4))), X, G, 0.050, 0.038, 0.016, SILVER, SILVER_SPEC, sides=10)

# --- lay it across the view -------------------------------------------------------------------------------------


def rot(p):
    a = math.radians(TILT_Z)
    x, y, z = p[0] * math.cos(a) - p[1] * math.sin(a), p[0] * math.sin(a) + p[1] * math.cos(a), p[2]
    b = math.radians(TURN_Y)
    return (x * math.cos(b) + z * math.sin(b), y, -x * math.sin(b) + z * math.cos(b))


for f in faces:
    f["P"] = [list(rot(p)) for p in f["P"]]
    f["n"] = list(rot(f["n"]))

# --- centre, scale, write --------------------------------------------------------------------------------------
pts = [p for f in faces for p in f["P"]]
lo = [min(p[i] for p in pts) for i in range(3)]
hi = [max(p[i] for p in pts) for i in range(3)]
mid = [(lo[i] + hi[i]) / 2 for i in range(3)]
rad = max(math.sqrt(sum((p[i] - mid[i]) ** 2 for i in range(3))) for p in pts)

out_faces = []
for f in faces:
    P = [[round((p[i] - mid[i]) / rad, 6) for i in range(3)] for p in f["P"]]
    out_faces.append({"P": P, "emit": False, "gid": 2, "n": [round(v, 6) for v in f["n"]],
                      "rgb": [float(c) for c in f["rgb"]], "spec": f["spec"], "two": f["two"]})

doc = {
    "about": "PICCOLO: a low-poly grenadilla concert piccolo with silver keys built by "
             "studio/scripts/models/piccolo_model.py, centred on its bounding box and scaled to fit a unit sphere",
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
    "name": "PICCOLO",
    "note": "faces are in the order the model builder made them; the sprite renderer draws them in this order (a later "
            "face wins only where it is strictly nearer). Lines (strings) are not part of the model.",
    "source": {"module": "studio/scripts/models/piccolo_model.py",
               "note": "generated by studio/scripts/models/piccolo_model.py from the photo references in "
                       "studio/reference/models/piccolo/",
               "size": 1.0, "yaw": 0},
}

here = os.path.dirname(os.path.abspath(__file__))
dst = os.path.join(here, "..", "..", "crates", "bf-content", "data", "models", "PICCOLO.json")
with open(dst, "w", encoding="utf-8", newline="\n") as fh:
    text = json.dumps(doc, indent=1, sort_keys=True)
    # one vertex (or normal, or colour) per line, as the other model files have them
    num = r"(-?[0-9.eE+-]+)"
    text = re.sub(r"\[\s*" + num + r",\s*" + num + r",\s*" + num + r"\s*\]", r"[\1, \2, \3]", text)
    fh.write(text + "\n")
print("PICCOLO.json:", len(out_faces), "faces")
