"""CORNET's low-poly model for BALLFIGHT Studio (a from-scratch fighter, 2026-11-16).

Writes studio/crates/bf-content/data/models/CORNET.json in the format of the exported models (format 1: faces with
P, n, rgb, spec, emit, gid, two), centred on its bounding box and scaled to fit a unit sphere. Same frame as
TRUMPET.json: x along the instrument (mouthpiece at -x, bell at +x), y up (the valve buttons on top), z depth.

The proportions (in centimetres before scaling) follow a B-flat cornet: about 36 cm from mouthpiece to bell rim, a
12 cm bell, three piston valves close to the middle, the main tuning loop below and the shepherd's-crook curl of the
leadpipe near the mouthpiece. The constants marked PHOTO are the ones to check against photos of real cornets.

Run from the clone root: python studio/tools/models/cornet_model.py
"""

import json
import math
import os

BRASS = [228.0, 178.0, 72.0]
BRASS_HI = [240.0, 200.0, 96.0]
BELL_IN = [196.0, 140.0, 52.0]
PEARL = [252.0, 248.0, 238.0]
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


TUBE = 0.55          # PHOTO: tubing radius (about 11 mm bore with the wall)
VALVES_X = [-1.6, 1.0, 3.6]   # PHOTO: three piston casings, close together
CASING_R = 0.85
CASING_Y = (-3.4, 3.0)
BELL_X = (8.5, 17.0)  # PHOTO: the bell section, shorter than a trumpet's
BELL_R = 4.6          # PHOTO: a bell rim of about 12 cm (the flare's last segment reaches the rim)

# the valve casings, their caps, stems and pearl buttons
for x in VALVES_X:
    cylinder_y(x, 0.0, CASING_Y[0], CASING_Y[1], CASING_R, BRASS)
    cylinder_y(x, 0.0, CASING_Y[1], CASING_Y[1] + 0.5, CASING_R * 1.12, BRASS_HI)
    cylinder_y(x, 0.0, CASING_Y[0] - 0.6, CASING_Y[0], CASING_R * 1.12, BRASS_HI)
    cylinder_y(x, 0.0, CASING_Y[1] + 0.5, CASING_Y[1] + 1.6, 0.22, BRASS_HI)
    cylinder_y(x, 0.0, CASING_Y[1] + 1.6, CASING_Y[1] + 2.0, 0.62, PEARL, spec=0.4)

# the bell section: from the third casing up and out to the bell (the bell's flare is exponential)
bell_path = line([VALVES_X[2] + CASING_R, 2.2, 0.0], [BELL_X[0], 2.2, 0.0], 3)
sweep(bell_path, TUBE, BRASS)
n = 10
flare = [[BELL_X[0] + (BELL_X[1] - BELL_X[0]) * k / n, 2.2, 0.0] for k in range(n + 1)]
radii = [TUBE + (BELL_R - TUBE) * (math.exp(3.2 * k / n) - 1) / (math.exp(3.2) - 1) for k in range(n + 1)]
sweep(flare, radii, BRASS, two=True)
# the bell's dark throat, seen from in front
throat_x = BELL_X[1] - 0.9
throat_r = TUBE + (BELL_R - TUBE) * (math.exp(3.2 * (throat_x - BELL_X[0]) / (BELL_X[1] - BELL_X[0])) - 1) / (math.exp(3.2) - 1)
face([[throat_x, 2.2 + throat_r * 0.92 * math.cos(2 * math.pi * j / 10), throat_r * 0.92 * math.sin(2 * math.pi * j / 10)]
      for j in range(10)], DARK, spec=0.1, two=True, out=[1.0, 0.0, 0.0])

# the main tuning loop below: out of the first casing, left, round and back (PHOTO: its length)
loop = (line([VALVES_X[0] - CASING_R, -2.6, 0.0], [-9.0, -2.6, 0.0], 3)
        + arc([-9.0, -0.4], 2.2, 270, 90, 0.0, 0.0, 8)[1:]
        + line([-9.0, 1.8, 0.0], [-5.6, 1.8, 0.0], 2)[1:])
sweep(loop, TUBE, BRASS)
# the slide's brace
cylinder_y(-7.0, 0.0, -2.6, 1.8, 0.16, BRASS_HI)

# the leadpipe with the shepherd's-crook curl near the mouthpiece (PHOTO: the curl's size and place), behind the
# loop (z < 0), into the second casing
lead = (line([-17.5, 0.6, -1.4], [-12.5, 0.6, -1.4], 3)
        + arc([-12.5, -0.9], 1.5, 90, -250, -1.4, -1.4, 14)[1:]
        + line([-12.5 + 1.5 * math.cos(math.radians(-250)), -0.9 + 1.5 * math.sin(math.radians(-250)), -1.4],
               [VALVES_X[1], 0.4, -1.0], 4)[1:])
sweep(lead, TUBE * 0.85, BRASS)

# the mouthpiece: a stem and a cup at the left end
sweep(line([-20.0, 0.6, -1.4], [-17.5, 0.6, -1.4], 2), [0.75, 0.42, 0.35], BRASS_HI, spec=0.9)
cylinder_tip = [[-20.4, 0.6, -1.4], [-20.0, 0.6, -1.4]]
sweep(cylinder_tip, 0.8, DARK, spec=0.1, caps=True)

# the finger hook and the water key, small brass details on top of the leadpipe
sweep(arc([-3.6, 2.8], 0.5, 180, 0, -0.6, -0.6, 4), 0.14, BRASS_HI)

# centre on the bounding box, scale to the unit sphere
pts = [p for f in faces for p in f['P']]
lo = [min(p[i] for p in pts) for i in range(3)]
hi = [max(p[i] for p in pts) for i in range(3)]
c = [(lo[i] + hi[i]) / 2 for i in range(3)]
s = max(math.sqrt(dot(sub(p, c), sub(p, c))) for p in pts)
for f in faces:
    f['P'] = [[(p[i] - c[i]) / s for i in range(3)] for p in f['P']]

doc = {
    "about": "CORNET: a from-scratch low-poly model (studio/tools/models/cornet_model.py), centred on its bounding box "
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
    "name": "CORNET",
    "note": "faces are in the order the model builder made them; the sprite renderer draws them in this order (a later "
            "face wins only where it is strictly nearer).",
    "source": {"module": "studio/tools/models/cornet_model.py", "scene": "modern", "size": 1.0, "yaw": 0},
}
here = os.path.dirname(os.path.abspath(__file__))
out = os.path.join(here, "..", "..", "crates", "bf-content", "data", "models", "CORNET.json")
with open(out, "w") as fh:
    json.dump(doc, fh, indent=1)
print(f"{len(faces)} faces -> {os.path.normpath(out)}")
