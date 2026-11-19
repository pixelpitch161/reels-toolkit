"""FLUGELHORN's low-poly model for BALLFIGHT Studio (a from-scratch fighter, 2026-11-16).

Writes studio/crates/bf-content/data/models/FLUGELHORN.json in the format of the exported models (format 1: faces with
P, n, rgb, spec, emit, gid, two), centred on its bounding box and scaled to fit a unit sphere. Same frame as
TRUMPET.json: x along the instrument (mouthpiece at -x, bell at +x), y up (the valve buttons on top), z depth.

The proportions (in centimetres before scaling) are measured from a side photo of a modern B-flat flugelhorn
(Wikimedia Commons "Yamaha_Flugelhorn_YFH-8310Z.jpg", bell to the right, about 14 px per cm at 960 px wide): about
50 cm from mouthpiece to bell rim, a 16 cm bell that starts widening right after the valves and keeps widening for
more than half the horn's length, a short straight leadpipe into three upright valves, and one long oval loop of
tubing under them. The constants marked PHOTO come from that photo.

Run from the clone root: python studio/tools/models/flugelhorn_model.py
"""

import json
import math
import os

BRASS = [228.0, 178.0, 72.0]
BRASS_HI = [240.0, 200.0, 96.0]
BELL_IN = [196.0, 140.0, 52.0]
SILVER = [214.0, 218.0, 226.0]
DARK = [34.0, 26.0, 22.0]
SIDES = 8

faces = []


def sub(a, b):
    return [a[i] - b[i] for i in range(3)]


def add(a, b):
    return [a[i] + b[i] for i in range(3)]


def mul(a, s):
    return [x * s for x in a]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def dot(a, b):
    return sum(a[i] * b[i] for i in range(3))


def unit(a):
    n = math.sqrt(dot(a, a)) or 1.0
    return [x / n for x in a]


def face(pts, rgb, spec=0.9, two=False, out=None):
    n = unit(cross(sub(pts[1], pts[0]), sub(pts[2], pts[0])))
    if out is not None and dot(n, out) < 0:
        pts = pts[::-1]
        n = mul(n, -1.0)
    faces.append(dict(P=[list(p) for p in pts], emit=False, gid=2, n=n, rgb=list(rgb), spec=spec, two=two))


def frame_at(path, i):
    """A tangent and two normals at point i of a polyline (parallel-transport-free but stable for these paths)."""
    a = path[max(i - 1, 0)]
    b = path[min(i + 1, len(path) - 1)]
    t = unit(sub(b, a))
    ref = [0.0, 0.0, 1.0] if abs(t[2]) < 0.9 else [0.0, 1.0, 0.0]
    u = unit(cross(t, ref))
    v = unit(cross(t, u))
    return t, u, v


def sweep(path, radii, rgb, spec=0.9, two=False, caps=False):
    """A tube along `path` (points) with a radius per point, SIDES quads round each segment."""
    rings = []
    for i, c in enumerate(path):
        _, u, v = frame_at(path, i)
        r = radii[i] if isinstance(radii, list) else radii
        rings.append([add(c, add(mul(u, r * math.cos(2 * math.pi * j / SIDES)),
                                   mul(v, r * math.sin(2 * math.pi * j / SIDES)))) for j in range(SIDES)])
    for i in range(len(path) - 1):
        for j in range(SIDES):
            j2 = (j + 1) % SIDES
            q = [rings[i][j], rings[i + 1][j], rings[i + 1][j2], rings[i][j2]]
            mid = mul(add(path[i], path[i + 1]), 0.5)
            centre = mul(add(add(q[0], q[1]), add(q[2], q[3])), 0.25)
            face(q, rgb, spec, two, out=sub(centre, mid))
    if caps:
        for end, sign in ((0, -1.0), (len(path) - 1, 1.0)):
            t, _, _ = frame_at(path, end)
            face(rings[end], rgb, spec, False, out=mul(t, sign))


def line(a, b, n=2):
    return [add(a, mul(sub(b, a), k / n)) for k in range(n + 1)]


def arc(c, r, a0, a1, z0, z1, n=8):
    """An arc in the xy plane round c from angle a0 to a1 (degrees), z moving from z0 to z1."""
    out = []
    for k in range(n + 1):
        a = math.radians(a0 + (a1 - a0) * k / n)
        out.append([c[0] + r * math.cos(a), c[1] + r * math.sin(a), z0 + (z1 - z0) * k / n])
    return out


def cylinder_y(x, z, y0, y1, r, rgb, spec=0.9):
    sweep([[x, y0, z], [x, y1, z]], r, rgb, spec, caps=True)


# Measured from the photo with the middle valve at x = 0 and the valves' middle at y = 0.
TUBE = 0.6           # PHOTO: tubing radius (about 9 mm across; drawn a little thicker so it reads in the ball)
VALVES_X = [-2.5, 0.0, 2.4]   # PHOTO: three piston casings, close together, a little left of the middle
CASING_R = 0.9       # PHOTO: casings about 18 mm across
CASING_Y = (-6.7, 6.0)
BELL_Y = 4.3         # PHOTO: the bell's axis; the bell tube runs straight over the oval loop behind the valve tops
BELL_Z = -1.3        # the bell tube passes behind the casings
BELL_X = (3.3, 26.7)  # PHOTO: the flare starts just past the third valve and ends at the rim
BELL_R = 7.9          # PHOTO: a bell rim of about 16 cm
FLARE = 3.0           # PHOTO: radii of about 1.3 cm at x = 12.6 and 4.8 cm at x = 21.8
LOOP_L = (-8.7, -2.8, 7.1)   # PHOTO: the loop's left bend (centre x, centre y, radius) from the bell tube down
LOOP_B = -9.9                # PHOTO: the loop's bottom run
LOOP_R = (16.8, -6.0, 3.9)   # PHOTO: the loop's right bend, back up toward the third valve
LEAD_Y = 1.2          # PHOTO: the leadpipe's height, straight from the mouthpiece into the first valve

# the valve casings, their caps, stems and buttons (a flugelhorn's are brass, not pearl)
for x in VALVES_X:
    cylinder_y(x, 0.0, CASING_Y[0], CASING_Y[1], CASING_R, BRASS)
    cylinder_y(x, 0.0, CASING_Y[1], CASING_Y[1] + 1.0, CASING_R * 1.15, BRASS_HI)
    cylinder_y(x, 0.0, CASING_Y[0] - 0.8, CASING_Y[0], CASING_R * 1.15, BRASS_HI)
    cylinder_y(x, 0.0, CASING_Y[1] + 1.0, CASING_Y[1] + 2.4, 0.22, BRASS_HI)
    cylinder_y(x, 0.0, CASING_Y[1] + 2.4, CASING_Y[1] + 3.0, 0.85, BRASS_HI, spec=0.6)

# the bell tube: from the loop's left bend straight to the flare, then the long, gradual flare to the rim
sweep(line([LOOP_L[0], BELL_Y, BELL_Z], [BELL_X[0], BELL_Y, BELL_Z], 3), TUBE, BRASS)
n = 16
flare = [[BELL_X[0] + (BELL_X[1] - BELL_X[0]) * k / n, BELL_Y, BELL_Z] for k in range(n + 1)]


def flare_r(t):
    return TUBE + (BELL_R - TUBE) * (math.exp(FLARE * t) - 1) / (math.exp(FLARE) - 1)


radii = [flare_r(k / n) for k in range(n + 1)]
sweep(flare, radii, BRASS, two=True)
# the bell's dark throat, seen from in front
throat_x = BELL_X[1] - 1.2
throat_r = flare_r((throat_x - BELL_X[0]) / (BELL_X[1] - BELL_X[0]))
face([[throat_x, BELL_Y + throat_r * 0.92 * math.cos(2 * math.pi * j / 12), BELL_Z + throat_r * 0.92 * math.sin(2 * math.pi * j / 12)]
      for j in range(12)], DARK, spec=0.1, two=True, out=[1.0, 0.0, 0.0])

# the long oval loop: down round the left bend, along the bottom, up round the right bend and back to the third valve
lx, ly, lr = LOOP_L
rx, ry, rr = LOOP_R
loop = (arc([lx, ly], lr, 90, 270, BELL_Z, 0.0, 12)
        + line([lx, LOOP_B, 0.0], [rx, LOOP_B, 0.0], 4)[1:]
        + arc([rx, ry], rr, 270, 450, 0.0, 0.0, 8)[1:]
        + line([rx, ry + rr, 0.0], [VALVES_X[2] + CASING_R, ry + rr, 0.0], 3)[1:])
sweep(loop, TUBE, BRASS)

# the leadpipe: short and straight from the mouthpiece into the first valve, with its tuning collar
sweep(line([-20.3, LEAD_Y, 0.0], [VALVES_X[0] - CASING_R, LEAD_Y, 0.0], 4), TUBE * 0.85, BRASS)
sweep(line([-14.5, LEAD_Y, 0.0], [-12.5, LEAD_Y, 0.0], 1), TUBE * 1.35, BRASS_HI)

# the valve slides under the casings: small U-bends
for x0, x1, depth in ((VALVES_X[0], VALVES_X[1], 2.2), (VALVES_X[1], VALVES_X[2], 3.0)):
    cx = (x0 + x1) / 2
    r_ = (x1 - x0) / 2
    bend = (line([x0, CASING_Y[0] + 1.5, 0.9], [x0, CASING_Y[0] + 1.5 - depth, 0.9], 1)
            + arc([cx, CASING_Y[0] + 1.5 - depth], r_, 180, 360, 0.9, 0.9, 6)[1:]
            + line([x1, CASING_Y[0] + 1.5 - depth, 0.9], [x1, CASING_Y[0] + 1.5, 0.9], 1)[1:])
    sweep(bend, TUBE * 0.75, BRASS)

# the mouthpiece: a silver tapering stem and the cup's rim at the left end
sweep(line([-23.3, LEAD_Y, 0.0], [-20.3, LEAD_Y, 0.0], 2), [1.15, 0.5, 0.4], SILVER, spec=1.0)
sweep([[-23.7, LEAD_Y, 0.0], [-23.3, LEAD_Y, 0.0]], 1.25, DARK, spec=0.1, caps=True)

# the third valve's trigger and the finger hook, silver
sweep(arc([VALVES_X[2] + 2.2, -3.2], 1.0, 200, 20, 1.2, 1.2, 5), 0.18, SILVER)
sweep(arc([VALVES_X[2] + 1.6, BELL_Y + 0.9], 0.6, 180, -20, BELL_Z, BELL_Z, 4), 0.15, SILVER)

# centre on the bounding box, scale to the unit sphere
pts = [p for f in faces for p in f['P']]
lo = [min(p[i] for p in pts) for i in range(3)]
hi = [max(p[i] for p in pts) for i in range(3)]
c = [(lo[i] + hi[i]) / 2 for i in range(3)]
s = max(math.sqrt(dot(sub(p, c), sub(p, c))) for p in pts)
for f in faces:
    f['P'] = [[(p[i] - c[i]) / s for i in range(3)] for p in f['P']]

doc = {
    "about": "FLUGELHORN: a from-scratch low-poly model (studio/tools/models/flugelhorn_model.py), centred on its bounding box "
             "and scaled to fit a unit sphere, in TRUMPET.json's frame (bell at +x, valve buttons up)",
    "faces": faces,
    "fields": {
        "P": "the face's vertices, k x 3 (x, y, z; y up), in drawing order (a fan from the first vertex)",
        "emit": "its colour is used as it is, unshaded",
        "gid": "group id: 1 marks the ground group, anything else is the instrument (the sprite renderer maps it to 2)",
        "n": "its unit normal",
        "rgb": "its colour, 0..255",
        "spec": "specular strength (0: none)",
        "two": "drawn from both sides",
    },
    "format": 1,
    "name": "FLUGELHORN",
    "note": "faces are in the order the model builder made them; the sprite renderer draws them in this order (a later "
            "face wins only where it is strictly nearer).",
    "source": {"module": "studio/tools/models/flugelhorn_model.py", "scene": "modern", "size": 1.0, "yaw": 0},
}
here = os.path.dirname(os.path.abspath(__file__))
out = os.path.join(here, "..", "..", "crates", "bf-content", "data", "models", "FLUGELHORN.json")
with open(out, "w") as fh:
    json.dump(doc, fh, indent=1)
print(f"{len(faces)} faces -> {os.path.normpath(out)}")
