"""contrabass_clarinet: the CONTRABASS CLARINET's low-poly model, made from scratch (no Pixel Pitch module has one).

Writes studio/crates/bf-content/data/models/CONTRABASS CLARINET.json in the shape the export's `models` writes
(format 1, faces with P, n, rgb, spec, emit, two, gid; y up; centred on its bounding box and scaled to fit a unit
sphere). The body is the folded metal "paperclip": four silver runs joined by U-bends, a black mouthpiece on a crook
at the top left, an up-turned bell at the top right, gold key pads on the front of the runs and a thin endpin.

Run from the repo root: python -m tools.studio_export.new_models.contrabass_clarinet (plain Python, no numpy).
"""
import json
import math
import os

OUT = os.path.join(os.path.dirname(__file__), '..', '..', '..', 'studio', 'crates', 'bf-content', 'data', 'models',
                   'CONTRABASS CLARINET.json')
SIDES = 8
SILVER, SILVER_HI, DARK, GOLD, PIN = (214.0, 216.0, 226.0), (226.0, 228.0, 239.0), (30.0, 26.0, 30.0), \
    (226.0, 204.0, 148.0), (119.0, 120.0, 125.0)


def sub(a, b):
    return [a[i] - b[i] for i in range(3)]


def add(a, b):
    return [a[i] + b[i] for i in range(3)]


def mul(a, s):
    return [v * s for v in a]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def unit(a):
    n = math.sqrt(sum(v * v for v in a)) or 1.0
    return [v / n for v in a]


def arc(cx, cy, r, a0, a1, n):
    return [[cx + r * math.cos(a0 + (a1 - a0) * i / n), cy + r * math.sin(a0 + (a1 - a0) * i / n), 0.0]
            for i in range(n + 1)]


def path():
    """the body's centre line, mouthpiece end first: four runs and three U-bends in the x-y plane"""
    xs, top, bot, half = (-0.24, -0.08, 0.08, 0.24), 0.56, -0.58, 0.08
    pts = [[xs[0], top, 0.0], [xs[0], bot, 0.0]]
    pts += arc(xs[0] + half, bot, half, math.pi, 2 * math.pi, 6)[1:]
    pts += [[xs[1], top, 0.0]]
    pts += arc(xs[1] + half, top, half, math.pi, 0.0, 6)[1:]
    pts += [[xs[2], bot, 0.0]]
    pts += arc(xs[2] + half, bot, half, math.pi, 2 * math.pi, 6)[1:]
    pts += [[xs[3], 0.46, 0.0]]
    return pts


def tube(faces, pts, radii, col, spec):
    """a tube along `pts` (radius `radii[i]` at point i), SIDES quads per segment, normals outward"""
    rings = []
    for i, p in enumerate(pts):
        a = pts[max(0, i - 1)]
        b = pts[min(len(pts) - 1, i + 1)]
        t = unit(sub(b, a))
        ref = [0.0, 0.0, 1.0] if abs(t[2]) < 0.9 else [1.0, 0.0, 0.0]
        u = unit(cross(t, ref))
        v = unit(cross(t, u))
        ring = []
        for s in range(SIDES):
            ang = 2 * math.pi * (s + 0.5) / SIDES
            d = add(mul(u, math.cos(ang)), mul(v, math.sin(ang)))
            ring.append((add(p, mul(d, radii[i])), d))
        rings.append(ring)
    for i in range(len(rings) - 1):
        for s in range(SIDES):
            s2 = (s + 1) % SIDES
            q = [rings[i][s][0], rings[i + 1][s][0], rings[i + 1][s2][0], rings[i][s2][0]]
            n = unit(add(add(rings[i][s][1], rings[i][s2][1]), add(rings[i + 1][s][1], rings[i + 1][s2][1])))
            faces.append(face(q, n, col, spec))


def face(q, n, col, spec, two=False):
    return dict(P=[list(p) for p in q], emit=False, gid=2, n=n, rgb=list(col), spec=spec, two=two)


def disc(faces, c, r, n, col, spec, sides=SIDES):
    """a flat polygon facing `n` (a pad, a cap)"""
    n = unit(n)
    ref = [0.0, 1.0, 0.0] if abs(n[1]) < 0.9 else [1.0, 0.0, 0.0]
    u = unit(cross(n, ref))
    v = cross(n, u)
    pts = [add(c, add(mul(u, r * math.cos(2 * math.pi * s / sides)), mul(v, r * math.sin(2 * math.pi * s / sides))))
           for s in range(sides)]
    faces.append(face(pts[::-1], n, col, spec))


def build():
    faces = []
    body = path()
    tube(faces, body, [0.055] * len(body), SILVER, 0.95)
    # the bell: up from the last run's top, flaring
    bell = [[0.24, 0.46 + 0.05 * j, 0.0] for j in range(8)]
    tube(faces, bell, [0.055 + 0.11 * (j / 7) ** 2.2 for j in range(8)], SILVER_HI, 0.95)
    # the bell's mouth: a dark disc just inside the rim, seen from above and the front
    disc(faces, [0.24, 0.80, 0.0], 0.15, [0.0, 1.0, 0.0], DARK, 0.0)
    # the crook and the mouthpiece, up and out from the first run
    crook = [[-0.24, 0.56, 0.0], [-0.26, 0.64, 0.0], [-0.30, 0.70, 0.0], [-0.34, 0.74, 0.0]]
    tube(faces, crook, [0.045] * len(crook), SILVER, 0.9)
    mouth = [[-0.34, 0.74, 0.0], [-0.37, 0.80, 0.0], [-0.39, 0.86, 0.0]]
    tube(faces, mouth, [0.04, 0.035, 0.02], DARK, 0.45)
    # gold key pads on the front of the runs (z+), and their mirror on the back
    for x in (-0.24, -0.08, 0.08, 0.24):
        for j, y in enumerate((-0.38, -0.18, 0.02, 0.22, 0.40)):
            if x == 0.24 and y > 0.4:
                continue
            for z in (1.0, -1.0):
                disc(faces, [x, y + 0.03 * (j % 2), 0.058 * z], 0.032, [0.0, 0.0, z], GOLD, 0.2, sides=6)
    # the endpin: a thin dark rod under the lowest bend
    pin = [[0.0, -0.66, 0.0], [0.0, -0.86, 0.0]]
    tube(faces, pin, [0.014, 0.008], PIN, 0.0)
    return faces


def normalise(faces):
    pts = [p for f in faces for p in f['P']]
    lo = [min(p[i] for p in pts) for i in range(3)]
    hi = [max(p[i] for p in pts) for i in range(3)]
    c = [(lo[i] + hi[i]) / 2 for i in range(3)]
    s = max(math.sqrt(sum((p[i] - c[i]) ** 2 for i in range(3))) for p in pts)
    for f in faces:
        f['P'] = [[(p[i] - c[i]) / s for i in range(3)] for p in f['P']]


def run():
    faces = build()
    normalise(faces)
    doc = dict(
        about='CONTRABASS CLARINET: made from scratch by tools/studio_export/new_models/contrabass_clarinet.py (no '
              'Pixel Pitch module has the instrument), centred on its bounding box and scaled to fit a unit sphere',
        faces=faces,
        fields={'P': "the face's vertices, k x 3 (x, y, z; y up), in drawing order (a fan from the first vertex)",
                'emit': 'its colour is used as it is, unshaded',
                'gid': 'group id: 1 marks the ground group, anything else is the instrument',
                'n': 'its unit normal', 'rgb': 'its colour, 0..255', 'spec': 'specular strength (0: none)',
                'two': 'drawn from both sides'},
        format=1,
        name='CONTRABASS CLARINET',
        note='faces are in the order the builder made them; the sprite renderer draws them in this order',
        source={'module': 'tools/studio_export/new_models/contrabass_clarinet.py', 'scene': 'ball', 'size': 1.0,
                'yaw': 0})
    with open(OUT, 'w', encoding='utf-8', newline='\n') as f:
        json.dump(doc, f, indent=1)
        f.write('\n')
    print(f'{os.path.normpath(OUT)}: {len(faces)} faces')


if __name__ == '__main__':
    run()
