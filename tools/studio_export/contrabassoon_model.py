"""CONTRABASSOON's model for BALLFIGHT Studio: writes studio/crates/bf-content/data/models/CONTRABASSOON.json.

A fighter made from scratch (Nov 16, 2026), so no Pixel Pitch module stands behind its model: this script builds it
from reference photos of a modern Heckel-system contrabassoon (Wikimedia Commons "Contrabassoon2.jpg"; Heckel's and
the Vienna Symphonic Library's instrument pages):

- four parallel dark red-brown wooden tubes side by side, the tube folded back on itself four times;
- the two left tubes joined at the top by a wooden U-bend, the two middle ones at the bottom by a silver U-bend, the
  two right ones at the top by a small silver U-bend;
- silver ferrules at the joints, silver key rods and pads down the front;
- the right tube turning into a silver metal bell partway down that opens toward the floor;
- a thin silver S-shaped crook (bocal) with the black double reed coming out near the top;
- a short silver endpin with a black tip below the bottom bend.

Run from the repo root with the reference Python (numpy only):
    .studio-tools\\py313\\Scripts\\python.exe tools/studio_export/contrabassoon_model.py
"""

import json
import math
from pathlib import Path

import numpy as np

OUT = Path(__file__).resolve().parents[2] / "studio/crates/bf-content/data/models/CONTRABASSOON.json"

WOOD = (104.0, 34.0, 30.0)
WOOD_DK = (78.0, 24.0, 22.0)
SILVER = (214.0, 216.0, 226.0)
SILVER_DK = (171.0, 172.0, 180.0)
BLACK = (34.0, 32.0, 34.0)
REED = (196.0, 168.0, 110.0)

faces = []


def face(pts, rgb, spec):
    """One face; its normal points away from the winding's right-hand normal (the models' convention)."""
    p = np.asarray(pts, float)[::-1]
    n = -np.cross(p[1] - p[0], p[2] - p[0])
    ln = np.linalg.norm(n)
    if ln < 1e-12:
        return
    faces.append(dict(P=p, n=n / ln, rgb=rgb, spec=spec))


def frame(d):
    d = np.asarray(d, float)
    d = d / np.linalg.norm(d)
    a = np.array([1.0, 0, 0]) if abs(d[0]) < 0.9 else np.array([0, 0, 1.0])
    u = np.cross(d, a)
    u /= np.linalg.norm(u)
    return u, np.cross(d, u)


def ring(c, d, r, seg):
    u, v = frame(d)
    return [np.asarray(c) + r * (math.cos(2 * math.pi * j / seg) * u + math.sin(2 * math.pi * j / seg) * v)
            for j in range(seg)]


def sweep(rings, rgb, spec, cap0=False, cap1=False):
    """Quads between consecutive rings (each a list of points), optionally closing the ends."""
    seg = len(rings[0])
    for a, b in zip(rings, rings[1:]):
        for j in range(seg):
            k = (j + 1) % seg
            face([a[j], a[k], b[k], b[j]], rgb, spec)
    if cap0:
        face(list(rings[0]), rgb, spec)
    if cap1:
        face(list(reversed(rings[-1])), rgb, spec)


def tube(p0, p1, r, rgb, spec, seg=10, caps=True):
    d = np.subtract(p1, p0)
    sweep([ring(p0, d, r, seg), ring(p1, d, r, seg)], rgb, spec, caps, caps)


def cone(p0, p1, r0, r1, rgb, spec, seg=12, cap1=False):
    d = np.subtract(p1, p0)
    sweep([ring(p0, d, r0, seg), ring(p1, d, r1, seg)], rgb, spec, False, cap1)


def ubend(c, half, r, up, rgb, spec, steps=8, seg=10):
    """A half torus joining (c.x - half, c.y) and (c.x + half, c.y), bulging up (up=1) or down (up=-1), in x-y."""
    rings = []
    for i in range(steps + 1):
        a = math.pi * i / steps
        p = np.array([c[0] - half * math.cos(a), c[1] + up * half * math.sin(a), c[2]])
        tang = np.array([half * math.sin(a), up * half * math.cos(a), 0.0])
        rings.append(ring(p, tang, r, seg))
    sweep(rings, rgb, spec)


def box(c, s, rgb, spec):
    c, s = np.asarray(c, float), np.asarray(s, float) / 2
    x, y, z = s
    v = [c + np.array([sx * x, sy * y, sz * z]) for sx in (-1, 1) for sy in (-1, 1) for sz in (-1, 1)]
    for q in ([0, 1, 3, 2], [4, 6, 7, 5], [0, 4, 5, 1], [2, 3, 7, 6], [0, 2, 6, 4], [1, 5, 7, 3]):
        face([v[i] for i in q], rgb, spec)


# the four wooden tubes, left to right (x), front is +z
TOP, BOT = 0.78, -0.80
X = [-0.165, -0.055, 0.055, 0.165]
R = [0.052, 0.046, 0.046, 0.050]
BELL_Y = -0.12  # where the right tube turns into the metal bell section
tube((X[0], BOT + 0.02, 0), (X[0], TOP, 0), R[0], WOOD, 0.45)
tube((X[1], BOT, 0), (X[1], TOP, 0), R[1], WOOD_DK, 0.45)
tube((X[2], BOT, 0), (X[2], TOP - 0.04, 0), R[2], WOOD, 0.45)
tube((X[3], BELL_Y, 0), (X[3], TOP - 0.04, 0), R[3], WOOD_DK, 0.45)
# the bends: wooden on top left, silver at the bottom middle and top right
ubend(((X[0] + X[1]) / 2, TOP, 0), (X[1] - X[0]) / 2, 0.05, 1, WOOD_DK, 0.35)
ubend(((X[1] + X[2]) / 2, BOT, 0), (X[2] - X[1]) / 2, 0.046, -1, SILVER, 0.9)
ubend(((X[2] + X[3]) / 2, TOP - 0.04, 0), (X[3] - X[2]) / 2, 0.044, 1, SILVER, 0.9)
# the metal bell: a silver tube down from the right tube, flaring toward the floor
tube((X[3], BELL_Y, 0), (X[3], -0.40, 0), R[3] * 0.95, SILVER, 0.9, caps=False)
cone((X[3], -0.40, 0), (X[3] + 0.01, -0.50, 0), R[3] * 0.95, 0.075, SILVER, 0.9)
cone((X[3] + 0.01, -0.50, 0), (X[3] + 0.02, -0.56, 0), 0.075, 0.115, SILVER_DK, 0.7)
# silver ferrules at the joints
for x, r in zip(X, R):
    for y in (0.62, 0.18, -0.36, -0.70):
        if x == X[3] and y < BELL_Y:
            continue
        tube((x, y - 0.018, 0), (x, y + 0.018, 0), r * 1.16, SILVER, 0.95)
# the crook: an S of thin silver tube out of the left tube near the top, then the black-wrapped double reed
crook = [(X[0] - 0.02, 0.50, 0.04), (X[0] - 0.09, 0.53, 0.10), (X[0] - 0.13, 0.60, 0.10), (X[0] - 0.10, 0.68, 0.06),
         (X[0] - 0.05, 0.71, 0.08), (X[0] - 0.04, 0.76, 0.14)]
for a, b in zip(crook, crook[1:]):
    tube(a, b, 0.013, SILVER, 0.95, seg=6)
tube(crook[-1], (X[0] - 0.035, 0.79, 0.17), 0.016, BLACK, 0.3, seg=6)
tube((X[0] - 0.035, 0.79, 0.17), (X[0] - 0.03, 0.83, 0.20), 0.019, REED, 0.2, seg=6)
# key rods and pads down the front
for x, r, ys in ((X[0], R[0], (0.40, 0.05, -0.30)), (X[1], R[1], (0.52, -0.10)),
                 (X[2], R[2], (0.30, -0.45)), (X[3], R[3], (0.45, 0.10))):
    tube((x + 0.012, -0.62 if x != X[3] else BELL_Y + 0.02, r + 0.012), (x + 0.012, 0.60, r + 0.012),
         0.006, SILVER_DK, 0.8, seg=4)
    for y in ys:
        tube((x, y, r - 0.005), (x, y, r + 0.02), 0.026, SILVER, 0.95, seg=10)
# levers bridging the tubes
for y, a, b in ((0.34, X[0], X[2]), (-0.05, X[1], X[3]), (-0.52, X[0], X[2])):
    box(((a + b) / 2, y, 0.07), (b - a, 0.012, 0.012), SILVER, 0.9)
# the endpin and its black tip
tube(((X[1] + X[2]) / 2, BOT - 0.05, 0), ((X[1] + X[2]) / 2, -0.97, 0), 0.008, SILVER, 0.9, seg=6)
tube(((X[1] + X[2]) / 2, -0.97, 0), ((X[1] + X[2]) / 2, -1.0, 0), 0.02, BLACK, 0.3, seg=6)

# centre on the bounding box and scale to fit a unit sphere, as every model file is
P = np.concatenate([f["P"] for f in faces])
centre = (P.min(0) + P.max(0)) / 2
scale = 1.0 / np.max(np.linalg.norm(P - centre, axis=1))
out = []
for f in faces:
    out.append(dict(P=[[float(c) for c in (q - centre) * scale] for q in f["P"]], emit=False, gid=2,
                    n=[float(c) for c in f["n"]], rgb=[float(c) for c in f["rgb"]], spec=f["spec"], two=False))
doc = {
    "about": "CONTRABASSOON: built by tools/studio_export/contrabassoon_model.py from reference photos (a fighter made "
             "from scratch), centred on its bounding box and scaled to fit a unit sphere",
    "faces": out,
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
    "name": "CONTRABASSOON",
    "note": "faces are in the order the model builder made them; the sprite renderer draws them in this order (a later "
            "face wins only where it is strictly nearer).",
    "source": {"module": "tools/studio_export/contrabassoon_model.py", "scene": "ball", "size": 1.0, "yaw": 0},
}
OUT.write_text(json.dumps(doc, indent=1) + "\n", encoding="utf-8")
print(f"{len(out)} faces -> {OUT}")
