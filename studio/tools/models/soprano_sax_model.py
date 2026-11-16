"""SOPRANO SAX's low-poly model for BALLFIGHT Studio (a from-scratch fighter, 2026-11-16).

Writes studio/crates/bf-content/data/models/SOPRANO SAX.json in the format of the exported models (format 1: faces
with P, n, rgb, spec, emit, gid, two), centred on its bounding box and scaled to fit a unit sphere, in CLARINET.json's
frame. Built from six photos of real straight sopranos (Wikimedia Commons, downloaded on Aedan's PC to
F:\\!Ai Stuff\\InstrumentBattleMachine\\fighter-refs\\soprano-sax\\, guide in reference.md there). The constants and
colours marked PHOTO come from them.

Run from the clone root: python studio/tools/models/soprano_sax_model.py
"""

import json
import math
import os

# PHOTO colours, sampled from the photo guide (gold lacquer ramp, pearls, mouthpiece, cork, pads)
GOLD = [212.0, 169.0, 100.0]      # base lacquer #D4A964 (photo 04, warm)
GOLD_HI = [245.0, 210.0, 156.0]   # light #F5D29C
GOLD_DK = [128.0, 79.0, 39.0]     # rods, mid-dark #804F27
NECK = GOLD                       # the neck: the body's lacquer in every photo
RING = [74.0, 65.0, 41.0]         # the receiver ring #4A4129
CUP = [221.0, 186.0, 108.0]       # key cups #DDBA6C
PEARL = [252.0, 242.0, 240.0]     # mother-of-pearl touches #FCF2F0
RIM = [255.0, 244.0, 204.0]       # the bell's rolled rim #FFF4CC
THROAT = [60.0, 34.0, 12.0]       # inside the bell, toward #0F0601
MOUTH = [14.0, 14.0, 16.0]        # hard rubber mouthpiece #0B0B0B
LIG = [104.0, 93.0, 59.0]         # ligature #685D3B
CORK = [60.0, 48.0, 32.0]         # cork #3C3020
FELT = [52.0, 55.0, 47.0]         # thumb rest and felts #34372F
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


# All sizes in centimetres, measured on the full-length Yamaha YSS-875EX photo (01 in the PC's fighter-refs folder,
# 4435 px from mouthpiece tip to bell rim for a 60 cm horn). The axis runs down y from the mouthpiece tip (y = +30)
# to the bell rim (y = -30); +x is the player's right (the side rods and low keys), +z the front (the pearl keys).
L = 60.0                      # PHOTO: overall length (the museum's Adolphe Sax soprano is 60.2 cm)
TOP = L / 2
GIRTH = 1.6                   # the thin top end, thickened for the sprite


def y_at(pct):
    """The height of a point `pct` per cent down the horn."""
    return TOP - L * pct / 100.0


def body_r(pct):
    """PHOTO: the bore's outer radius at `pct`: a straight cone from the neck tip to just above the bell. The photo's
    radii (0.24, 0.26, 0.34, 0.95, 1.83, 1.9 cm) are thickened toward the top by GIRTH so the thin horn still reads
    at sprite size, as CLARINET.json is."""
    stations = [(7, 0.24 * GIRTH), (10, 0.26 * GIRTH), (20, 0.34 * GIRTH), (50, 0.95 * 1.25), (90, 1.83 * 1.08),
                (94, 1.9 * 1.05)]
    for (p0, r0), (p1, r1) in zip(stations, stations[1:]):
        if p0 <= pct <= p1:
            return r0 + (r1 - r0) * (pct - p0) / (p1 - p0)
    return stations[-1][1] if pct > 94 else stations[0][1]


def axis(pcts):
    return [[0.0, y_at(p), 0.0] for p in pcts]


# the mouthpiece: a wedge beak at the very top, then the barrel (0 to 7 %), about 3x the neck tip
sweep(axis([0.0, 1.2, 3.0, 7.0]), [0.35, 0.66, 0.76, 0.7], MOUTH, spec=0.3, caps=True)
# the ligature band (4 to 6.5 %) and its two screw knobs to the player's right
sweep(axis([4.0, 6.5]), 0.82, LIG, spec=0.8, caps=True)
for pct in (4.6, 5.9):
    sweep([[0.8, y_at(pct), 0.0], [1.35, y_at(pct), 0.0]], 0.16, GOLD_HI, spec=0.9, caps=True)
# the cork (7 to 10 %)
sweep(axis([7.0, 10.0]), 0.36, CORK, spec=0.1)
# the neck (10 to 20 %), paler, and the receiver ring at 20 %
sweep(axis([10.0, 15.0, 20.0]), [body_r(10), body_r(15), body_r(20)], NECK, spec=0.8)
sweep(axis([19.4, 21.0]), body_r(20) + 0.14, RING, spec=0.6, caps=True)
# the octave key: a thin lever up the back of the neck from the ring, curling over at 15 %
sweep([[0.0, y_at(21), -(body_r(21) + 0.16)], [0.0, y_at(16), -(body_r(16) + 0.2)],
       [0.0, y_at(14.6), -(body_r(14.6) + 0.05)]], 0.08, GOLD_DK, spec=0.8)
# the body: a straight cone (20 to 94 %)
pcts = [20 + 74 * k / 10 for k in range(11)]
sweep(axis(pcts), [body_r(p) for p in pcts], GOLD, spec=0.9)
# the bell: a short flare in the last 6 %, a rolled rim, and the dark throat inside
bell = [94.0, 96.0, 98.0, 99.4, 100.0]
sweep(axis(bell), [1.9, 2.1, 2.7, 3.4, 3.75], GOLD, spec=1.0, two=True)
sweep(axis([99.5, 100.2]), 3.82, RIM, spec=1.0, caps=False)
face([[3.4 * math.cos(2 * math.pi * j / SIDES), y_at(99.6), 3.4 * math.sin(2 * math.pi * j / SIDES)]
      for j in range(SIDES)], THROAT, spec=0.0, two=True)

# the six pearl touches down the front, in two hands of three, each in a gold cup twice its size; they grow slightly
for n, pct in enumerate((31.0, 35.5, 40.0, 52.5, 56.0, 60.0)):
    r = body_r(pct)
    s = 0.42 + 0.03 * n
    sweep([[0.0, y_at(pct), r - 0.05], [0.0, y_at(pct), r + 0.28]], s, CUP, spec=0.9, caps=True)
    sweep([[0.0, y_at(pct), r + 0.28], [0.0, y_at(pct), r + 0.36]], s * 0.8, PEARL, spec=1.0, caps=True)

# the palm-key fins at the upper right (22 to 34 %, the same side as the low keys in photos 01 and 02): flat, swept-back paddles, the clearest break in the outline
for pct0, pct1, out in ((22.0, 29.0, 0.6), (26.0, 34.0, 0.75)):
    r0, r1 = body_r(pct0), body_r(pct1)
    fin = [[r0, y_at(pct0), 0.1], [r0 + out, y_at(pct0 + 1.2), 0.25], [r1 + out * 0.8, y_at(pct1), 0.25],
           [r1, y_at(pct1), 0.1]]
    face(fin, GOLD_HI, spec=0.9, two=True)

# the side rods: a long pair on each side (41 to 87 % by the low keys, 19 to 59 % on the other), on ball posts
for dz, (p0, p1), side in ((0.25, (41.0, 87.0), 1.0), (-0.25, (43.0, 85.0), 1.0),
                           (0.2, (19.0, 59.0), -1.0), (-0.2, (21.0, 57.0), -1.0)):
    rod = [[side * (body_r(p) + 0.35), y_at(p), dz] for p in (p0, (p0 + p1) / 2, p1)]
    sweep(rod, 0.11, GOLD_DK, spec=0.9)
    for p in (p0, (p0 + p1) / 2, p1):
        sweep([[side * body_r(p), y_at(p), dz], [side * (body_r(p) + 0.38), y_at(p), dz]], 0.1, GOLD_DK, spec=0.8)

# the big low-key cups on the right (80 to 94 %), about as wide as the tube there, standing off on posts
for pct in (78.0, 82.0, 86.0):
    r = body_r(pct)
    sweep([[r - 0.1, y_at(pct), 0.0], [r + 0.55, y_at(pct), 0.0]], r * 0.62, CUP, spec=0.9, caps=True)
# two side-key cups at the upper right
for pct in (33.0, 44.0):
    r = body_r(pct)
    sweep([[r - 0.05, y_at(pct), 0.15], [r + 0.3, y_at(pct), 0.15]], 0.45, CUP, spec=0.9, caps=True)

# the thumb rest on the back (62 %) and black felts on the right edge (35 and 55 %)
sweep([[0.0, y_at(62.0), -body_r(62.0)], [0.0, y_at(62.0), -(body_r(62.0) + 0.5)]], 0.3, FELT, spec=0.2, caps=True)
for pct in (35.0, 55.0):
    r = body_r(pct)
    sweep([[r + 0.3, y_at(pct), 0.45], [r + 0.55, y_at(pct), 0.45]], 0.2, FELT, spec=0.1, caps=True)

# held as a player holds it: about 28 degrees off vertical, bell forward and down (+x), mouthpiece at -x, as CLARINET
TILT = math.radians(28.0)     # PHOTO: photo 06 measures 29 to 30 degrees
for f in faces:
    f['P'] = [[p[0] * math.cos(TILT) - p[1] * math.sin(TILT), p[0] * math.sin(TILT) + p[1] * math.cos(TILT), p[2]]
              for p in f['P']]
    n = f['n']
    f['n'] = [n[0] * math.cos(TILT) - n[1] * math.sin(TILT), n[0] * math.sin(TILT) + n[1] * math.cos(TILT), n[2]]

# centre on the bounding box, scale to the unit sphere
pts = [p for f in faces for p in f['P']]
lo = [min(p[i] for p in pts) for i in range(3)]
hi = [max(p[i] for p in pts) for i in range(3)]
c = [(lo[i] + hi[i]) / 2 for i in range(3)]
s = max(math.sqrt(dot(sub(p, c), sub(p, c))) for p in pts)
for f in faces:
    f['P'] = [[(p[i] - c[i]) / s for i in range(3)] for p in f['P']]

doc = {
    "about": "SOPRANO SAX: a from-scratch low-poly model (studio/tools/models/soprano_sax_model.py) built from photos "
             "of real straight sopranos, centred on its bounding box and scaled to fit a unit sphere, in CLARINET.json's "
             "frame (mouthpiece up at -x, bell down at +x, keys to the front)",
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
    "name": "SOPRANO SAX",
    "note": "faces are in the order the model builder made them; the sprite renderer draws them in this order (a later "
            "face wins only where it is strictly nearer).",
    "source": {"module": "studio/tools/models/soprano_sax_model.py", "scene": "modern", "size": 1.0, "yaw": 0},
}
here = os.path.dirname(os.path.abspath(__file__))
out = os.path.join(here, "..", "..", "crates", "bf-content", "data", "models", "SOPRANO SAX.json")
with open(out, "w") as fh:
    json.dump(doc, fh, indent=1)
print(f"{len(faces)} faces -> {os.path.normpath(out)}")
