"""Builds the MARIMBA low-poly model and writes studio/crates/bf-content/data/models/MARIMBA.json.

Made from the photo references in studio/reference/models/marimba/ (see NOTES.md there): a 5-octave concert
marimba (C2..C7, 61 bars) with rosewood bars in two rows (naturals in front, accidentals raised behind), brass
resonator tubes tapering under them (the longest bass tubes ending in rounded U bends), wooden end boards and rails
on a trapezoid plan, and black metal end frames on wheels joined by a stretcher bar.

Axes: x runs bass (left, -x) to treble (right, +x) as the player sees it, y is up, z points to the player (the
sprite camera looks from +z). The model is centred on its bounding box and scaled to fit a unit sphere.

Standard library only. Run: python studio/scripts/models/marimba_model.py
"""

import json
import math
import os
import re

ROSEWOOD = (150.0, 70.0, 46.0)
RAIL_WOOD = (196.0, 112.0, 62.0)
BRASS = (222.0, 176.0, 84.0)
METAL = (34.0, 34.0, 40.0)
WHEEL = (20.0, 20.0, 24.0)

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
    sides = {
        "top": [c[4], c[5], c[6], c[7]],
        "bottom": [c[0], c[1], c[2], c[3]],
    }
    for i in range(4):
        j = (i + 1) % 4
        sides["s%d" % i] = [c[i], c[j], c[4 + j], c[4 + i]]
    for name, pts in sides.items():
        if name not in skip:
            face(pts, rgb, spec, inside)


def box(x0, x1, y0, y1, z0, z1, rgb, spec, skip=()):
    """An axis-aligned box. Side names: s0 = back (-z), s1 = right (+x), s2 = front (+z), s3 = left (-x)."""
    c = [(x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1), (x0, y1, z0), (x1, y1, z0), (x1, y1, z1), (x0, y1, z1)]
    box8(c, rgb, spec, skip)


def tube(cx, cz, r, y_top, y_bot, rgb, spec, sides=6, round_bottom=False):
    """A vertical prism (no top, no flat bottom unless asked); `round_bottom` adds a low dome for a U-bend look."""
    ring = [(cx + r * math.cos(2 * math.pi * (k + 0.5) / sides), cz + r * math.sin(2 * math.pi * (k + 0.5) / sides))
            for k in range(sides)]
    for k in range(sides):
        a, b = ring[k], ring[(k + 1) % sides]
        face([(a[0], y_bot, a[1]), (b[0], y_bot, b[1]), (b[0], y_top, b[1]), (a[0], y_top, a[1])], rgb, spec,
             (cx, (y_top + y_bot) / 2, cz))
    if round_bottom:
        tip = (cx, y_bot - r * 0.9, cz)
        for k in range(sides):
            a, b = ring[k], ring[(k + 1) % sides]
            face([(a[0], y_bot, a[1]), tip, (b[0], y_bot, b[1])], rgb, spec, (cx, y_bot, cz))


# --- keyboard layout -------------------------------------------------------------------------------------------
LOW_MIDI, HIGH_MIDI = 36, 96  # C2..C7: 61 bars
NATURAL = {0, 2, 4, 5, 7, 9, 11}

H = 1.00  # height of the natural bars' tops above the floor
FRONT_Z = 0.42  # the natural bars' front ends line up just behind the front rail
GAP = 0.025  # gap between natural row and accidental row (in z)
RAISE = 0.03  # accidentals sit this much higher
STEP_GAP = 0.006  # gap between neighbouring naturals

notes = list(range(LOW_MIDI, HIGH_MIDI + 1))
naturals = [m for m in notes if m % 12 in NATURAL]
n_nat = len(naturals)


def t_of(m):
    return (m - LOW_MIDI) / (HIGH_MIDI - LOW_MIDI)


def bar_len(m):
    # 0.50 at the bass to 0.20 at the treble, falling faster at first (as the photos show)
    t = t_of(m)
    return 0.20 + 0.30 * (1 - t) ** 1.35


def bar_w(m):
    return 0.060 - 0.022 * t_of(m)


def bar_thick(m):
    return 0.034 - 0.014 * t_of(m)


# x of each natural: graduated widths placed side by side, centred later
x = 0.0
nat_x = {}
for m in naturals:
    w = bar_w(m)
    nat_x[m] = x + w / 2
    x += w + STEP_GAP
total = x - STEP_GAP
for m in nat_x:
    nat_x[m] -= total / 2
X_LEFT = -total / 2
X_RIGHT = total / 2


def acc_x(m):
    return (nat_x[m - 1] + nat_x[m + 1]) / 2


def nat_back_z(xq):
    """The back end of the natural row at x (interpolated so the accidentals follow a smooth line)."""
    ms = sorted(nat_x, key=lambda q: nat_x[q])
    for a, b in zip(ms, ms[1:]):
        if nat_x[a] <= xq <= nat_x[b]:
            u = (xq - nat_x[a]) / (nat_x[b] - nat_x[a])
            return FRONT_Z - (bar_len(a) * (1 - u) + bar_len(b) * u)
    return FRONT_Z - bar_len(ms[0] if xq < 0 else ms[-1])


def shade(rgb, k):
    # a small, fixed per-bar variation, like rosewood grain differing from bar to bar
    f = 1.0 + 0.07 * math.sin(k * 2.399) - 0.03 * math.cos(k * 1.13)
    return tuple(round(min(255.0, c * f), 1) for c in rgb)


bars = []  # (midi, cx, cz_center, top_y, length, width, thick)
for m in notes:
    L, w, th = bar_len(m), bar_w(m), bar_thick(m)
    if m % 12 in NATURAL:
        cx = nat_x[m]
        z1 = FRONT_Z
        z0 = z1 - L
        top = H
    else:
        cx = acc_x(m)
        w *= 0.92
        z1 = nat_back_z(cx) - GAP
        z0 = z1 - L * 0.97
        top = H + RAISE
    bars.append((m, cx, z0, z1, top, w, th))

# --- the rails and end boards ----------------------------------------------------------------------------------
back_left = min(b[2] for b in bars[:3]) - 0.03
back_right = min(b[2] for b in bars[-3:]) - 0.03
RAIL_T = 0.035
END_T = 0.05
xl = X_LEFT - 0.02
xr = X_RIGHT + 0.02
rail_top = H - 0.035
rail_bot = H - 0.13
front_rail_z = FRONT_Z + 0.015

# front rail: straight, along the player's side
box(xl, xr, rail_bot, rail_top, front_rail_z, front_rail_z + RAIL_T, RAIL_WOOD, 0.25)
# back rail: angled in towards the treble (trapezoid plan)
c = [(xl, rail_bot, back_left - RAIL_T), (xr, rail_bot, back_right - RAIL_T), (xr, rail_bot, back_right),
     (xl, rail_bot, back_left)]
c += [(p[0], rail_top + RAISE, p[2]) for p in c]
box8(c, RAIL_WOOD, 0.25)
# middle rail between the rows (carries the inner ends of both rows)
mid_l = nat_back_z(X_LEFT + 0.05) - GAP / 2
mid_r = nat_back_z(X_RIGHT - 0.05) - GAP / 2
c = [(xl, rail_bot + 0.03, mid_l - 0.012), (xr, rail_bot + 0.03, mid_r - 0.012), (xr, rail_bot + 0.03, mid_r + 0.012),
     (xl, rail_bot + 0.03, mid_l + 0.012)]
c += [(p[0], rail_top - 0.005, p[2]) for p in c]
box8(c, RAIL_WOOD, 0.2, skip=("bottom",))

# end boards
END_TOP = H + RAISE + 0.02
for xa, xb, zb in ((xl - END_T, xl, back_left - RAIL_T - 0.02), (xr, xr + END_T, back_right - RAIL_T - 0.02)):
    box(xa, xb, rail_bot - 0.02, END_TOP, zb, front_rail_z + RAIL_T + 0.02, RAIL_WOOD, 0.25)

# a dark deck under the bars, so the narrow gaps between bars read as shadow, not as holes, at small sizes
DECK = (66.0, 34.0, 26.0)
yd = H - 0.045
face([(xl, yd, back_left), (xr, yd, back_right), (xr, yd, front_rail_z), (xl, yd, front_rail_z)], DECK, 0.0,
     (0.0, yd - 1.0, 0.0))

# --- the bars --------------------------------------------------------------------------------------------------
for k, (m, cx, z0, z1, top, w, th) in enumerate(bars):
    box(cx - w / 2, cx + w / 2, top - th, top, z0, z1, shade(ROSEWOOD, k), 0.2, skip=("bottom",))

# --- resonator tubes -------------------------------------------------------------------------------------------
TUBE_TOP = rail_bot + 0.02
FLOOR = 0.0
LOWEST_BOTTOM = 0.14  # the bass tubes end just above the floor, between the wheels
for k, (m, cx, z0, z1, top, w, th) in enumerate(bars):
    t = t_of(m)
    # length falls steeply then levels out (the curve of tube bottoms in the photos)
    length = 0.06 + (TUBE_TOP - LOWEST_BOTTOM - 0.06) * (1 - t) ** 1.6
    r = w * 0.50
    cz = (z0 + z1) / 2
    if m % 12 not in NATURAL:
        length *= 0.94
    tube(cx, cz, r, TUBE_TOP, TUBE_TOP - length, BRASS, 0.9, round_bottom=(m - LOW_MIDI) < 8)

# --- end frames, stretcher and wheels ---------------------------------------------------------------------------
LEG = 0.028
WHEEL_R = 0.045
for xc, zb in ((xl - END_T / 2, back_left), (xr + END_T / 2, back_right)):
    zf = front_rail_z + 0.01
    zbk = zb - 0.01
    foot_y = WHEEL_R * 2
    for zc in (zf - 0.06, zbk + 0.06):
        box(xc - LEG / 2, xc + LEG / 2, foot_y, rail_bot - 0.02, zc - LEG / 2, zc + LEG / 2, METAL, 0.4,
            skip=("top", "bottom"))
    # foot bar along z
    box(xc - LEG / 2, xc + LEG / 2, foot_y, foot_y + LEG, zbk, zf, METAL, 0.4, skip=("bottom",))
    # cross bar at mid height
    box(xc - LEG / 2, xc + LEG / 2, 0.45, 0.45 + LEG * 0.8, zbk + 0.06, zf - 0.06, METAL, 0.4)
    # wheels: octagonal discs facing x
    for zc in (zbk, zf):
        ring = [(zc + WHEEL_R * math.cos(2 * math.pi * (i + 0.5) / 8), WHEEL_R + WHEEL_R * math.sin(
            2 * math.pi * (i + 0.5) / 8)) for i in range(8)]
        for side in (-1, 1):
            xs = xc + side * 0.02
            face([(xs, p[1], p[0]) for p in ring], WHEEL, 0.3, (xc, WHEEL_R, zc))
        for i in range(8):
            a, b = ring[i], ring[(i + 1) % 8]
            face([(xc - 0.02, a[1], a[0]), (xc + 0.02, a[1], a[0]), (xc + 0.02, b[1], b[0]), (xc - 0.02, b[1], b[0])],
                 WHEEL, 0.3, (xc, WHEEL_R, zc))

# stretcher bar between the end frames, behind the tubes, and a diagonal brace at the bass end
zs = (back_left + back_right) / 2 - 0.02
box(xl, xr, 0.45, 0.45 + LEG, zs - LEG / 2, zs + LEG / 2, METAL, 0.4)
c = [(xl, 0.12, zs - LEG / 2), (xl + 0.55, 0.45, zs - LEG / 2), (xl + 0.55, 0.45, zs + LEG / 2),
     (xl, 0.12, zs + LEG / 2)]
c += [(p[0], p[1] + LEG, p[2]) for p in c]
box8(c, METAL, 0.4)

# --- centre, scale, write --------------------------------------------------------------------------------------
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
    "about": "MARIMBA: a low-poly 5-octave concert marimba built by studio/scripts/models/marimba_model.py, centred on "
             "its bounding box and scaled to fit a unit sphere",
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
    "name": "MARIMBA",
    "note": "faces are in the order the model builder made them; the sprite renderer draws them in this order (a later "
            "face wins only where it is strictly nearer). Lines (strings) are not part of the model.",
    "source": {"module": "studio/scripts/models/marimba_model.py",
               "note": "generated by studio/scripts/models/marimba_model.py from the photo references in "
                       "studio/reference/models/marimba/",
               "size": 1.0, "yaw": 0},
}

here = os.path.dirname(os.path.abspath(__file__))
dst = os.path.join(here, "..", "..", "crates", "bf-content", "data", "models", "MARIMBA.json")
with open(dst, "w", encoding="utf-8", newline="\n") as fh:
    text = json.dumps(doc, indent=1, sort_keys=True)
    # one vertex (or normal, or colour) per line, as the other model files have them
    num = r"(-?[0-9.eE+-]+)"
    text = re.sub(r"\[\s*" + num + r",\s*" + num + r",\s*" + num + r"\s*\]", r"[\1, \2, \3]", text)
    fh.write(text + "\n")
print("MARIMBA.json:", len(out_faces), "faces")
