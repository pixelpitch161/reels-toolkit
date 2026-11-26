"""MELLOPHONE's low-poly model for BALLFIGHT Studio (a from-scratch fighter, 2026-11-16).

Writes studio/crates/bf-content/data/models/MELLOPHONE.json in the format of the exported models (format 1: faces
with P, n, rgb, spec, emit, gid, two), centred on its bounding box and scaled to fit a unit sphere. Same frame as
TRUMPET.json: x along the instrument (mouthpiece at -x, bell at +x facing forward), y up (the valve buttons on top),
z depth.

A marching mellophone in F (bell-front), silver-plated like most drum-corps horns. The constants marked PHOTO are
measured in pixels on the side photo Wikimedia Commons "Yamaha Mellophone YMP-204MS.tiff" (the 960 px wide copy,
bell to the right, CC BY-SA 4.0 Yamaha Corporation; see studio/reference/models/mellophone/SOURCES.md). Its bell
rim is 500 px tall and the real bell is about 25.4 cm across, so the photo is about 19.7 px per cm. Pixel x grows
to the bell, pixel y grows downward; `P` turns a pixel point into centimetres with the middle valve at x = 0 and the
bell's axis at y = 0.

Run from the clone root: py studio/tools/models/mellophone_model.py
"""

import json
import math
import os

SILVER = [204.0, 208.0, 216.0]
SILVER_HI = [232.0, 235.0, 240.0]
BELL_IN = [168.0, 172.0, 182.0]
BUTTON = [244.0, 244.0, 240.0]
DARK = [30.0, 30.0, 36.0]
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


def sweep(path, radii, rgb, spec=0.9, two=False, caps=False, sides=SIDES):
    """A tube along `path` (points) with a radius per point, `sides` quads round each segment."""
    rings = []
    for i, c in enumerate(path):
        _, u, v = frame_at(path, i)
        r = radii[i] if isinstance(radii, list) else radii
        rings.append([add(c, add(mul(u, r * math.cos(2 * math.pi * j / sides)),
                                   mul(v, r * math.sin(2 * math.pi * j / sides)))) for j in range(sides)])
    for i in range(len(path) - 1):
        for j in range(sides):
            j2 = (j + 1) % sides
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


PX_PER_CM = 19.7      # PHOTO: the bell rim is 500 px tall in the 960 px copy, and the real bell is 25.4 cm
ORIGIN = (432, 258)   # PHOTO: the middle valve's x and the bell's axis y, in pixels


def cm(px):
    return px / PX_PER_CM


def P(px, py, z=0.0):
    return [cm(px - ORIGIN[0]), cm(ORIGIN[1] - py), z]


def X(px):
    return cm(px - ORIGIN[0])


def Y(py):
    return cm(ORIGIN[1] - py)


TUBE = cm(12)          # PHOTO: the main wrap's tubing is about 24 px across (12 mm), a bigger bore than a trumpet
LEAD = cm(8.5)         # PHOTO: the leadpipe, about 17 px across
SLIDE = cm(10)         # PHOTO: the valve slides, about 20 px across
VALVES_PX = [386, 432, 478]   # PHOTO: three piston casings side by side, 46 px apart
CASING_R = cm(17)      # PHOTO: casings about 34 px across
CASING_PY = (470, 250)  # PHOTO: bottom and top of the casings
CAP_PX = 10            # PHOTO: the top and bottom caps, about 10 px tall
BUTTON_PY = (213, 205)  # PHOTO: the finger buttons' underside and top
BUTTON_R = cm(15)      # PHOTO: buttons about 30 px across
# PHOTO: the bell tube's radius (pixels) at pixel x, from where it leaves the wrap to the rim: it widens slowly to
# about x = 680, then flares hard to a 250 px radius at x = 942 (the rim)
BELL_PROFILE = [(220, 19), (400, 21), (560, 28), (680, 42), (760, 62), (820, 92), (860, 128), (890, 170),
                (915, 210), (932, 238), (942, 250)]
WRAP_L = (220, 386, 130)   # PHOTO: the big outer loop's left turn: centre x, centre y, radius (left edge at x = 90)
WRAP_R = (470, 415, 100)   # PHOTO: its right turn back up to the third valve (bottom at y = 515, right edge x = 570)
LEAD_PY = 290          # PHOTO: the leadpipe's height, from the mouthpiece receiver at x = 60
LEAD_R = (490, 380, 90)    # PHOTO: the leadpipe's turn down at the bell end
LEAD_L = (300, 395, 75)    # PHOTO: its turn back up into the first valve
TUNE = (180, 402, 442, 386)  # PHOTO: the main tuning slide: its U's left end x, upper leg y, lower leg y, valve x
INNER = (300, 515, 362, 490)  # PHOTO: the third valve's slide loop: left x, right x, top y, bottom y
CROOK = (600, 700, 385, 440)  # PHOTO: the first valve slide's crook out under the bell: from x, to x, legs y
HOOK_PX = 330          # PHOTO: the finger ring on top of the leadpipe, just before the valves
WRAP_Z = 0.0
LEAD_Z = -1.6          # the leadpipe and its loop sit on the far side of the casings (as on CORNET)
SLIDE_Z = 1.2          # the slides on the near side

# the valve casings, their caps, stems and buttons
for vx in VALVES_PX:
    x = X(vx)
    y0, y1 = Y(CASING_PY[0]), Y(CASING_PY[1])
    cylinder_y(x, 0.0, y0, y1, CASING_R, SILVER)
    cylinder_y(x, 0.0, y1, y1 + cm(CAP_PX), CASING_R * 1.12, SILVER_HI)
    cylinder_y(x, 0.0, y0 - cm(CAP_PX), y0, CASING_R * 1.12, SILVER_HI)
    cylinder_y(x, 0.0, y1 + cm(CAP_PX), Y(BUTTON_PY[0]), cm(4), SILVER_HI)
    cylinder_y(x, 0.0, Y(BUTTON_PY[0]), Y(BUTTON_PY[1]), BUTTON_R, BUTTON, spec=0.5)

# the bell tube and the flare, measured radius by radius, then the dark throat seen from in front
bell = [P(px, ORIGIN[1]) for px, _ in BELL_PROFILE]
sweep(bell, [cm(r) for _, r in BELL_PROFILE], SILVER, two=True, sides=14)
tx, tr = BELL_PROFILE[-6]   # the throat sits deep, where the flare is still narrow (as in the front photos)
face([[X(tx), cm(tr) * 0.85 * math.cos(2 * math.pi * j / 14), cm(tr) * 0.85 * math.sin(2 * math.pi * j / 14)]
      for j in range(14)], DARK, spec=0.1, two=True, out=[1.0, 0.0, 0.0])


def ring(px, r, k):
    return [[X(px), cm(r) * k * math.cos(2 * math.pi * j / 14), cm(r) * k * math.sin(2 * math.pi * j / 14)]
            for j in range(15)]


# the shiny inside of the bell, a funnel from just inside the rim down to the throat, so the mouth reads silver
inside = [ring(BELL_PROFILE[-1][0] - 6, BELL_PROFILE[-1][1], 0.97), ring(*BELL_PROFILE[-3], 0.93),
          ring(*BELL_PROFILE[-5], 0.9), ring(tx + 2, tr, 0.86)]
for a, b in zip(inside, inside[1:]):
    for j in range(14):
        face([a[j], a[j + 1], b[j + 1], b[j]], BELL_IN, spec=0.6, two=True)

# the main wrap: the bell tube turns down round the big left loop, runs along the bottom and turns up into the
# third valve
cx, cy, cr = WRAP_L
rx, ry, rr = WRAP_R
wrap = (arc([X(cx), Y(cy)], cm(cr), 90, 270, WRAP_Z, WRAP_Z, 12)
        + line(P(cx, cy + cr), P(rx, ry + rr), 3)[1:]
        + arc([X(rx), Y(ry)], cm(rr), -90, 90, WRAP_Z, WRAP_Z, 8)[1:]
        + line(P(rx, ry - rr), P(VALVES_PX[2] + 12, ry - rr), 1)[1:])
sweep(wrap, TUBE, SILVER)

# the leadpipe: from the mouthpiece straight over the valves, down round the turn at the bell end, back along the
# bottom and up a smaller turn into the first valve
l1x, l1y, l1r = LEAD_R
l2x, l2y, l2r = LEAD_L
lead = (line(P(60, LEAD_PY, LEAD_Z), P(l1x, LEAD_PY, LEAD_Z), 5)
        + arc([X(l1x), Y(l1y)], cm(l1r), 90, -90, LEAD_Z, LEAD_Z, 8)[1:]
        + line(P(l1x, l1y + l1r, LEAD_Z), P(l2x, l2y + l2r, LEAD_Z), 3)[1:]
        + arc([X(l2x), Y(l2y)], cm(l2r), 270, 90, LEAD_Z, LEAD_Z, 6)[1:]
        + line(P(l2x, l2y - l2r, LEAD_Z), P(VALVES_PX[0], l2y - l2r, -0.6), 1)[1:])
sweep(lead, LEAD, SILVER)

# the main tuning slide: a U pointing back to the mouthpiece end, its legs into the first valve
ux, uy0, uy1, uvx = TUNE
tune = (line(P(uvx, uy0, 0.7), P(ux + 20, uy0, 0.7), 2)
        + arc([X(ux + 20), Y((uy0 + uy1) / 2)], cm((uy1 - uy0) / 2), 90, 270, 0.7, 0.7, 6)[1:]
        + line(P(ux + 20, uy1, 0.7), P(uvx, uy1, 0.7), 2)[1:])
sweep(tune, SLIDE, SILVER)
sweep([P(ux + 20, uy0 - 6, 0.7), P(ux + 20, uy1 + 6, 0.7)], cm(4), SILVER_HI)   # the slide's brace

# the third valve's slide: a rounded loop over the middle of the horn, on the near side
ix0, ix1, iy0, iy1 = INNER
ir = (iy1 - iy0) / 2
iyc = (iy0 + iy1) / 2
loop = (line(P(VALVES_PX[2], iy0, SLIDE_Z), P(ix0 + ir, iy0, SLIDE_Z), 2)
        + arc([X(ix0 + ir), Y(iyc)], cm(ir), 90, 270, SLIDE_Z, SLIDE_Z, 8)[1:]
        + line(P(ix0 + ir, iy1, SLIDE_Z), P(ix1 - 30, iy1, SLIDE_Z), 2)[1:]
        + arc([X(ix1 - 30), Y(iy1 - 30)], cm(30), -90, 0, SLIDE_Z, SLIDE_Z, 3)[1:])
sweep(loop, SLIDE, SILVER)

# the first valve slide's crook out under the bell, with a little curl below it (the water key's tube)
fx0, fx1, fy0, fy1 = CROOK
fr = (fy1 - fy0) / 2
crook = (line(P(VALVES_PX[2] + 15, fy0, 0.5), P(fx1 - fr, fy0, 0.5), 3)
         + arc([X(fx1 - fr), Y((fy0 + fy1) / 2)], cm(fr), 90, -90, 0.5, 0.5, 6)[1:]
         + line(P(fx1 - fr, fy1, 0.5), P(fx0, fy1, 0.5), 1)[1:]
         + arc([X(fx0), Y(fy1 + 25)], cm(25), 90, 270, 0.5, 0.5, 5)[1:]
         + line(P(fx0, fy1 + 50, 0.5), P(fx1 - 20, fy1 + 50, 0.5), 2)[1:])
sweep(crook, SLIDE, SILVER)

# the mouthpiece: a trumpet-style shank tapering into the receiver, and the cup's rim at the left end
sweep(line(P(28, LEAD_PY, LEAD_Z), P(60, LEAD_PY, LEAD_Z), 2), [cm(13), cm(7), cm(6)], SILVER_HI)
sweep([P(12, LEAD_PY, LEAD_Z), P(28, LEAD_PY, LEAD_Z)], cm(19), SILVER_HI, caps=True)
sweep([P(10, LEAD_PY, LEAD_Z), P(12, LEAD_PY, LEAD_Z)], cm(15), DARK, spec=0.1, caps=True)

# the finger ring on top of the leadpipe, just before the valves
sweep(arc([X(HOOK_PX), Y(LEAD_PY - 12)], cm(10), 180, -20, LEAD_Z, LEAD_Z, 4), cm(3), SILVER_HI)

# braces between the leadpipe, the bell tube and the wrap
for a, b in ((P(250, LEAD_PY, LEAD_Z), P(250, 262, 0.0)), (P(560, 300, 0.0), P(560, 268, 0.0)),
             (P(150, LEAD_PY, LEAD_Z), P(150, 290, 0.0))):
    sweep([a, b], cm(4), SILVER_HI)

# centre on the bounding box, scale to the unit sphere
pts = [p for f in faces for p in f['P']]
lo = [min(p[i] for p in pts) for i in range(3)]
hi = [max(p[i] for p in pts) for i in range(3)]
c = [(lo[i] + hi[i]) / 2 for i in range(3)]
s = max(math.sqrt(dot(sub(p, c), sub(p, c))) for p in pts)
for f in faces:
    f['P'] = [[(p[i] - c[i]) / s for i in range(3)] for p in f['P']]

doc = {
    "about": "MELLOPHONE: a from-scratch low-poly model (studio/tools/models/mellophone_model.py), centred on its "
             "bounding box and scaled to fit a unit sphere, in TRUMPET.json's frame (bell at +x, valve buttons up)",
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
    "name": "MELLOPHONE",
    "note": "faces are in the order the model builder made them; the sprite renderer draws them in this order (a later "
            "face wins only where it is strictly nearer).",
    "source": {"module": "studio/tools/models/mellophone_model.py", "scene": "modern", "size": 1.0, "yaw": 0},
}
here = os.path.dirname(os.path.abspath(__file__))
out = os.path.join(here, "..", "..", "crates", "bf-content", "data", "models", "MELLOPHONE.json")
with open(out, "w") as fh:
    json.dump(doc, fh, indent=1)
print(f"{len(faces)} faces -> {os.path.normpath(out)}")
