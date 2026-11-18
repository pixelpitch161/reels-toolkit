"""Builds the TARGET DUMMY low-poly model and writes studio/crates/bf-content/data/models/TARGET DUMMY.json.

The TARGET DUMMY is a developer fighter (never offered in the app): a ball that never attacks, for example renders of
another fighter's moves. Its model is a classic straw training dummy: a burlap sack body tied at the waist and neck, a
round sack head with stitched X eyes, sack arms on a wooden crossbar tied at the wrists, a red and white bullseye
painted on its chest, and the wooden post it stands on.

Axes: x runs left to right as the camera sees it, y is up, z points to the camera (the sprite camera looks from +z).
Built upright, tipped a little towards the camera (TILT), centred on its bounding box and scaled to fit a unit sphere.

Standard library only. Run: python studio/scripts/models/target_dummy_model.py
"""

import json
import math
import os
import re

BURLAP = (204.0, 170.0, 116.0)
BURLAP_DARK = (170.0, 136.0, 88.0)
ROPE = (124.0, 92.0, 56.0)
WOOD = (150.0, 102.0, 58.0)
WOOD_DARK = (112.0, 74.0, 40.0)
RED = (214.0, 40.0, 44.0)
WHITE = (240.0, 236.0, 226.0)
STITCH = (52.0, 38.0, 30.0)

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



N = 12  # segments round the body and head
OFF = math.pi / N  # half a segment: a flat face looks at the camera


def ring_pt(r, a, y, cx=0.0, cz=0.0):
    """A point on a circle round the vertical axis through (cx, cz); angle 0 faces the camera (+z)."""
    return (cx + r * math.sin(a), y, cz + r * math.cos(a))


def ang(k):
    return OFF + 2 * math.pi * k / N


def patch(k, row):
    """Burlap with a fixed scatter of darker patches, so the sack reads as cloth."""
    return BURLAP_DARK if (k * 5 + row * 3) % 7 in (0, 3) else BURLAP


def lathe(profile, rgb_of, spec, cx=0.0, cz=0.0, cap_top=True, cap_bottom=True):
    """A body of revolution round the vertical axis: `profile` is (y, r) from the bottom up."""
    for row in range(len(profile) - 1):
        (ya, ra), (yb, rb) = profile[row], profile[row + 1]
        inside = (cx, (ya + yb) / 2, cz)
        for k in range(N):
            a, b = ang(k), ang(k + 1)
            pts = [ring_pt(ra, a, ya, cx, cz), ring_pt(ra, b, ya, cx, cz), ring_pt(rb, b, yb, cx, cz),
                   ring_pt(rb, a, yb, cx, cz)]
            if ra < 1e-6:
                pts = [pts[0], pts[2], pts[3]]
            elif rb < 1e-6:
                pts = [pts[0], pts[1], pts[2]]
            face(pts, rgb_of(k, row), spec, inside)
    for cap, (y, r), up in ((cap_bottom, profile[0], -1), (cap_top, profile[-1], 1)):
        if cap and r > 1e-6:
            c = (cx, y, cz)
            for k in range(N):
                face([c, ring_pt(r, ang(k), y, cx, cz), ring_pt(r, ang(k + 1), y, cx, cz)], rgb_of(k, 0), spec,
                     (cx, y - up, cz))


def band(y0, y1, r, rgb):
    """A rope tied round the body: a short ring standing just proud of it."""
    lathe([(y0, r), (y1, r)], lambda k, row: rgb, 0.15, cap_top=False, cap_bottom=False)


def box(x0, x1, y0, y1, z0, z1, rgb, spec):
    c = [(x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1)]
    c += [(p[0], y1, p[2]) for p in c]
    box8(c, rgb, spec)


def arm(sx):
    """A sack arm along x on side sx (+1 right, -1 left), from the shoulder out, tied at the wrist."""
    M = 8
    x0, x1, r0, r1 = 0.40, 0.86, 0.15, 0.12
    y = 0.22

    def p(x, r, t):
        return (sx * x, y + r * math.sin(t), r * math.cos(t))

    for k in range(M):
        a = math.pi / M + 2 * math.pi * k / M
        b = math.pi / M + 2 * math.pi * (k + 1) / M
        mid = (sx * (x0 + x1) / 2, y, 0.0)
        face([p(x0, r0, a), p(x0, r0, b), p(x1, r1, b), p(x1, r1, a)], patch(k, 5), 0.1, mid)
        # the wrist rope and the straw tuft poking out past it
        face([p(x1 - 0.05, r1 + 0.012, a), p(x1 - 0.05, r1 + 0.012, b), p(x1, r1 + 0.012, b), p(x1, r1 + 0.012, a)],
             ROPE, 0.15, mid)
    # the arm's end, a flat burlap disc, and the crossbar's tip poking out of it
    c = (sx * x1, y, 0.0)
    for k in range(M):
        a = math.pi / M + 2 * math.pi * k / M
        b = math.pi / M + 2 * math.pi * (k + 1) / M
        face([c, p(x1, r1, a), p(x1, r1, b)], BURLAP_DARK, 0.1, (sx * (x1 - 1.0), y, 0.0))
    xa, xb = sorted((sx * x1, sx * (x1 + 0.08)))
    box(xa, xb, y - 0.05, y + 0.05, -0.05, 0.05, WOOD, 0.25)


# --- the post the dummy stands on, and its foot -----------------------------------------------------------------
box(-0.09, 0.09, -1.12, -0.52, -0.09, 0.09, WOOD, 0.25)
box(-0.34, 0.34, -1.20, -1.10, -0.10, 0.10, WOOD_DARK, 0.2)
box(-0.10, 0.10, -1.20, -1.10, -0.34, 0.34, WOOD_DARK, 0.2)

# --- the sack body: wider at the shoulders, gathered at the bottom ----------------------------------------------
BODY = [(-0.62, 0.20), (-0.55, 0.38), (-0.35, 0.46), (-0.05, 0.48), (0.22, 0.50), (0.36, 0.44), (0.44, 0.26),
        (0.47, 0.14)]
lathe(BODY, patch, 0.1)
band(-0.30, -0.22, 0.475, ROPE)  # the waist rope

# --- the arms on their crossbar ----------------------------------------------------------------------------------
arm(1)
arm(-1)

# --- the neck rope and the round sack head ----------------------------------------------------------------------
band(0.46, 0.53, 0.16, ROPE)
HC, HR = 0.80, 0.31
HEAD = [(HC + HR * math.sin(t), HR * math.cos(t)) for t in
        [math.radians(d) for d in (-90, -62, -34, -8, 18, 44, 68, 90)]]
lathe(HEAD, lambda k, row: patch(k, row + 2), 0.1, cap_top=False, cap_bottom=False)

# the stitched X eyes and a stitched smile on the face (thin boxes just proud of the front)
FZ = HR * math.cos(OFF) + 0.012


def stitch(x0, y0, x1, y1, z, w=0.018):
    dx, dy = x1 - x0, y1 - y0
    L = math.hypot(dx, dy)
    nx, ny = -dy / L * w, dx / L * w
    face([(x0 - nx, y0 - ny, z), (x1 - nx, y1 - ny, z), (x1 + nx, y1 + ny, z), (x0 + nx, y0 + ny, z)], STITCH, 0.0,
         (0.0, HC, 0.0))


for ex in (-0.11, 0.11):
    e = 0.05
    stitch(ex - e, HC + 0.05 - e, ex + e, HC + 0.05 + e, FZ)
    stitch(ex - e, HC + 0.05 + e, ex + e, HC + 0.05 - e, FZ)
stitch(-0.10, HC - 0.10, 0.10, HC - 0.10, FZ - 0.004)

# --- the bullseye on the chest: five painted rings, red outside and in the middle ------------------------------
TZ = 0.50 * math.cos(OFF) + 0.012
TY = -0.04
M = 20
for i, (r, rgb) in enumerate([(0.28, RED), (0.225, WHITE), (0.17, RED), (0.115, WHITE), (0.06, RED)]):
    z = TZ + 0.004 * i
    c = (0.0, TY, z)
    for k in range(M):
        a, b = 2 * math.pi * k / M, 2 * math.pi * (k + 1) / M
        face([c, (r * math.cos(a), TY + r * math.sin(a), z), (r * math.cos(b), TY + r * math.sin(b), z)], rgb, 0.05,
             (0.0, TY, z - 1.0))

# --- tip towards the camera, centre, scale, write ---------------------------------------------------------------
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
    "about": "TARGET DUMMY: a low-poly straw training dummy on a wooden post with a bullseye on its chest, built by "
             "studio/scripts/models/target_dummy_model.py, centred on its bounding box and scaled to fit a unit sphere",
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
    "name": "TARGET DUMMY",
    "note": "faces are in the order the model builder made them; the sprite renderer draws them in this order (a later "
            "face wins only where it is strictly nearer). Lines (strings) are not part of the model.",
    "source": {"module": "studio/scripts/models/target_dummy_model.py",
               "note": "generated by studio/scripts/models/target_dummy_model.py (a developer fighter, no photo references)",
               "size": 1.0, "yaw": 0},
}

here = os.path.dirname(os.path.abspath(__file__))
dst = os.path.join(here, "..", "..", "crates", "bf-content", "data", "models", "TARGET DUMMY.json")
with open(dst, "w", encoding="utf-8", newline="\n") as fh:
    text = json.dumps(doc, indent=1, sort_keys=True)
    # one vertex (or normal, or colour) per line, as the other model files have them
    num = r"(-?[0-9.eE+-]+)"
    text = re.sub(r"\[\s*" + num + r",\s*" + num + r",\s*" + num + r"\s*\]", r"[\1, \2, \3]", text)
    fh.write(text + "\n")
print("TARGET DUMMY.json:", len(out_faces), "faces")
