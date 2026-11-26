"""Builds the TIMPANI low-poly model and writes studio/crates/bf-content/data/models/TIMPANI.json.

Made from the photo references in studio/reference/models/timpani/ (see NOTES.md there): one concert pedal timpani
(a 29-inch drum, the usual second drum of a set of four) as the sprite camera sees it, slightly from above: a cream
plastic head inside a thin bright chrome counterhoop, eight chrome tension rods at 45 degrees, a hammered copper
kettle (a short straight collar under the rim, then a parabolic bowl, flat-shaded in small facets so the copper
sparkles), three dark struts curving down to a low cast base ring, a silver foot pedal pointing at the player and a
fan-shaped tuning gauge on the front of the bowl.

Axes: y is up, z points to the player (the sprite camera looks from +z), so the pedal and the gauge face the camera.
The model is centred on its bounding box and scaled to fit a unit sphere, like the other models.

Standard library only. Run: python studio/scripts/models/timpani_model.py
"""

import json
import math
import os
import re

HEAD = (237.0, 230.0, 218.0)  # #EDE6DA
HEAD_EDGE = (218.0, 208.0, 192.0)  # the head darkens a little towards the hoop
CHROME = (226.0, 230.0, 238.0)
COPPER = (184.0, 100.0, 46.0)  # #B8642E
COPPER_DARK = (120.0, 58.0, 26.0)
STRUT = (44.0, 38.0, 40.0)
BASE = (54.0, 54.0, 60.0)
ALU = (176.0, 180.0, 188.0)
RIB = (120.0, 124.0, 132.0)
GAUGE = (36.0, 36.0, 42.0)
TICK = (240.0, 240.0, 236.0)
POINTER = (214.0, 40.0, 36.0)
WHEEL = (22.0, 22.0, 26.0)

faces = []


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def add(a, b):
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def mul(a, s):
    return (a[0] * s, a[1] * s, a[2] * s)


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
        d = sub(centroid(pts), inside)
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


def polar(r, theta, y):
    """theta in degrees: 0 is +x (the player's right), 90 is +z (towards the player and the camera)."""
    t = math.radians(theta)
    return (r * math.cos(t), y, r * math.sin(t))


def radial_box(theta, r0, r1, y0, y1, w, rgb, spec, skip=()):
    """A box whose depth runs along the radius at angle theta, `w` wide (tangentially)."""
    t = math.radians(theta)
    u = (math.cos(t), 0.0, math.sin(t))
    v = (-math.sin(t), 0.0, math.cos(t))
    ring = []
    for r, s in ((r0, -1), (r0, 1), (r1, 1), (r1, -1)):
        ring.append(add(mul(u, r), mul(v, s * w / 2)))
    c = [(p[0], y0, p[2]) for p in ring] + [(p[0], y1, p[2]) for p in ring]
    box8(c, rgb, spec, skip)


def hashf(*k):
    """A fixed pseudo-random number in -1..1 from integers (no random module, so the model never changes)."""
    x = 0
    for q in k:
        x = (x * 1103515245 + 12345 + q * 2654435761) & 0xFFFFFFFF
    x ^= x >> 13
    x = (x * 1274126177) & 0xFFFFFFFF
    return (x / 0xFFFFFFFF) * 2.0 - 1.0


def tint(rgb, f):
    return tuple(round(max(0.0, min(255.0, c * f)), 1) for c in rgb)


# --- proportions (head diameter 1.0) ---------------------------------------------------------------------------
R = 0.5  # bowl and head radius
H = 0.95  # height of the head above the floor
SEG = 24  # sides around the drum
HOOP_IN, HOOP_OUT = 0.485, 0.518
HOOP_TOP = H + 0.014
HOOP_BOT = H - 0.026
COLLAR_BOT = H - 0.09  # the short straight collar under the rim
BOWL_DEPTH = 0.52  # the parabolic part below the collar (bowl depth in all about 0.6 x diameter)


def ang(k):
    return 360.0 * (k + 0.5) / SEG


# --- head ------------------------------------------------------------------------------------------------------
HEAD_IN = 0.43
centre = (0.0, H, 0.0)
below = (0.0, H - 1.0, 0.0)
face([polar(HEAD_IN, ang(k), H) for k in range(SEG)], HEAD, 0.15, below)
for k in range(SEG):
    a, b = ang(k), ang(k + 1)
    face([polar(HEAD_IN, a, H), polar(HOOP_IN + 0.002, a, H), polar(HOOP_IN + 0.002, b, H), polar(HEAD_IN, b, H)],
         HEAD_EDGE, 0.15, below)

# --- counterhoop: a thin chrome ring ---------------------------------------------------------------------------
for k in range(SEG):
    a, b = ang(k), ang(k + 1)
    face([polar(HOOP_IN, a, HOOP_TOP), polar(HOOP_OUT, a, HOOP_TOP), polar(HOOP_OUT, b, HOOP_TOP),
          polar(HOOP_IN, b, HOOP_TOP)], CHROME, 1.0, (0.0, HOOP_TOP - 1.0, 0.0))
    face([polar(HOOP_OUT, a, HOOP_BOT), polar(HOOP_OUT, b, HOOP_BOT), polar(HOOP_OUT, b, HOOP_TOP),
          polar(HOOP_OUT, a, HOOP_TOP)], CHROME, 1.0, (0.0, HOOP_TOP, 0.0))
    face([polar(HOOP_IN, a, H), polar(HOOP_IN, b, H), polar(HOOP_IN, b, HOOP_TOP), polar(HOOP_IN, a, HOOP_TOP)],
         tint(CHROME, 0.8), 1.0, polar(HOOP_OUT, (a + b) / 2, H))  # inner wall: faces the middle of the head
    face([polar(HOOP_IN, a, HOOP_BOT), polar(HOOP_OUT, a, HOOP_BOT), polar(HOOP_OUT, b, HOOP_BOT),
          polar(HOOP_IN, b, HOOP_BOT)], tint(CHROME, 0.7), 1.0, (0.0, HOOP_BOT + 1.0, 0.0))

# --- hammered copper bowl --------------------------------------------------------------------------------------
# rings from the rim down: (radius, y). The collar is straight; below it the bowl follows y = bottom + d (r/R)^2.
bottom_y = COLLAR_BOT - BOWL_DEPTH
rings = [(R, HOOP_BOT + 0.004), (R, COLLAR_BOT)]
for s in (0.80, 0.60, 0.42, 0.26, 0.14, 0.06):
    rings.append((R * math.sqrt(s), bottom_y + BOWL_DEPTH * s))


def bowl_pt(i, k):
    r, y = rings[i]
    if 1 < i < len(rings) - 1:  # dents: small fixed wobbles in radius and height (not on the rim or the hub)
        r *= 1.0 + 0.018 * hashf(i, k, 1)
        y += 0.010 * hashf(i, k, 2)
    return polar(r, ang(k) + (4.0 * hashf(i, k, 3) if 1 < i < len(rings) - 1 else 0.0), y)


for i in range(len(rings) - 1):
    yc = (rings[i][1] + rings[i + 1][1]) / 2
    for k in range(SEG):
        p00, p01 = bowl_pt(i, k), bowl_pt(i, k + 1)
        p10, p11 = bowl_pt(i + 1, k), bowl_pt(i + 1, k + 1)
        axis = (0.0, yc, 0.0)
        if i == 0:  # the collar: plain quads, polished
            face([p00, p01, p11, p10], tint(COPPER, 1.04 + 0.05 * hashf(k, 7)), 0.9, axis)
        else:  # hammered: two flat triangles per quad, each a slightly different shade
            face([p00, p01, p11], tint(COPPER, 1.06 + 0.14 * hashf(i, k, 11)), 0.9, axis)
            face([p00, p11, p10], tint(COPPER, 1.06 + 0.14 * hashf(i, k, 13)), 0.9, axis)

# the hub where the pedal spindle meets the bowl bottom
hub_y = rings[-1][1]
face([bowl_pt(len(rings) - 1, k) for k in range(SEG)], COPPER_DARK, 0.4, (0.0, hub_y + 1.0, 0.0))

# --- eight tension rods (at 22.5 + 45 k degrees, so none hides the gauge at the front) ---------------------------
ROD_R = 0.532
for k in range(8):
    th = 22.5 + 45.0 * k
    # square rod head standing on the hoop
    radial_box(th, HOOP_OUT - 0.028, HOOP_OUT + 0.006, HOOP_TOP, HOOP_TOP + 0.024, 0.030, CHROME, 1.0)
    # bracket from the hoop out to the rod
    radial_box(th, HOOP_OUT, ROD_R + 0.012, HOOP_TOP - 0.018, HOOP_TOP, 0.026, CHROME, 1.0)
    # the rod down the outside of the collar
    radial_box(th, ROD_R - 0.007, ROD_R + 0.007, COLLAR_BOT - 0.025, HOOP_TOP - 0.018, 0.014, CHROME, 1.0,
               skip=("top",))
    # the lug (claw) on the bowl it screws into
    radial_box(th, R - 0.004, ROD_R + 0.016, COLLAR_BOT - 0.045, COLLAR_BOT - 0.005, 0.040, tint(CHROME, 0.9), 0.9)

# --- three dark struts from the rim lugs, bowing out, down to the base ring ------------------------------------
BASE_R_TOP, BASE_R_BOT = 0.27, 0.31
BASE_TOP, BASE_BOT = 0.10, 0.035
strut_path = [(0.535, COLLAR_BOT + 0.02), (0.555, 0.62), (0.50, 0.40), (0.40, 0.20), (0.29, BASE_BOT + 0.01)]
STRUT_ANGLES = (30.0, 150.0, 270.0)  # pedal at the front (90) between two of them, one behind
for th in STRUT_ANGLES:
    t = math.radians(th)
    u = (math.cos(t), 0.0, math.sin(t))
    v = (-math.sin(t), 0.0, math.cos(t))
    W, T = 0.046, 0.026
    for (ra, ya), (rb, yb) in zip(strut_path, strut_path[1:]):
        def corners(r, y):
            base = (u[0] * r, y, u[2] * r)
            return [add(base, add(mul(v, -W / 2), mul(u, -T / 2))), add(base, add(mul(v, W / 2), mul(u, -T / 2))),
                    add(base, add(mul(v, W / 2), mul(u, T / 2))), add(base, add(mul(v, -W / 2), mul(u, T / 2)))]
        box8(corners(ra, ya) + corners(rb, yb), STRUT, 0.35, skip=("top", "bottom"))
    # a caster under the base at each strut
    radial_box(th, BASE_R_BOT - 0.05, BASE_R_BOT + 0.01, 0.0, BASE_BOT, 0.04, WHEEL, 0.3)

# --- base: a low cast ring (an 12-sided frustum) and the pedal spindle -----------------------------------------
BSEG = 12
for k in range(BSEG):
    a, b = 360.0 * k / BSEG, 360.0 * (k + 1) / BSEG
    face([polar(BASE_R_BOT, a, BASE_BOT), polar(BASE_R_BOT, b, BASE_BOT), polar(BASE_R_TOP, b, BASE_TOP),
          polar(BASE_R_TOP, a, BASE_TOP)], BASE, 0.4, (0.0, (BASE_TOP + BASE_BOT) / 2, 0.0))
face([polar(BASE_R_TOP, 360.0 * k / BSEG, BASE_TOP) for k in range(BSEG)], tint(BASE, 1.1), 0.4,
     (0.0, BASE_TOP - 1.0, 0.0))
for k in range(6):
    a, b = 60.0 * k, 60.0 * (k + 1)
    face([polar(0.024, a, BASE_TOP), polar(0.024, b, BASE_TOP), polar(0.024, b, hub_y), polar(0.024, a, hub_y)],
         CHROME, 1.0, (0.0, (BASE_TOP + hub_y) / 2, 0.0))

# --- the pedal: a ribbed aluminium footplate pointing at the player, toe end raised towards the drum -------------
PW = 0.13
z_heel, z_toe = 0.66, BASE_R_BOT - 0.01
y_heel, y_toe = 0.045, 0.19
PT = 0.022


def pedal_pt(x, s, up):
    """s = 0 at the toe (by the base), 1 at the heel (player side); up = 0 underside, 1 top."""
    z = z_toe + (z_heel - z_toe) * s
    y = y_toe + (y_heel - y_toe) * s + up * PT
    return (x, y, z)


c = [pedal_pt(-PW / 2, 0, 0), pedal_pt(PW / 2, 0, 0), pedal_pt(PW / 2, 1, 0), pedal_pt(-PW / 2, 1, 0)]
c += [pedal_pt(-PW / 2, 0, 1), pedal_pt(PW / 2, 0, 1), pedal_pt(PW / 2, 1, 1), pedal_pt(-PW / 2, 1, 1)]
box8(c, ALU, 0.8)
for j in range(6):  # raised ribs across the plate
    s0 = 0.10 + 0.14 * j
    s1 = s0 + 0.05
    c = [pedal_pt(-PW / 2 + 0.012, s0, 1), pedal_pt(PW / 2 - 0.012, s0, 1), pedal_pt(PW / 2 - 0.012, s1, 1),
         pedal_pt(-PW / 2 + 0.012, s1, 1)]
    c += [(p[0], p[1] + 0.008, p[2]) for p in c]
    box8(c, RIB, 0.6, skip=("bottom",))
# the hinge block and the heel stop on the floor
box8([(-0.05, 0.0, z_heel - 0.06), (0.05, 0.0, z_heel - 0.06), (0.05, 0.0, z_heel + 0.01), (-0.05, 0.0, z_heel + 0.01),
      (-0.05, y_heel, z_heel - 0.06), (0.05, y_heel, z_heel - 0.06), (0.05, y_heel, z_heel + 0.01),
      (-0.05, y_heel, z_heel + 0.01)], BASE, 0.4)
# the pedal rod running up from the toe into the base
radial_box(90.0, BASE_R_TOP - 0.12, z_toe + 0.02, BASE_TOP, y_toe + 0.04, 0.03, CHROME, 1.0)

# --- tuning gauge: a dark fan plate on a bracket at the front of the bowl, with ticks and a red pointer -----------
GZ = ROD_R + 0.025  # just in front of the collar, clear of the rods
g_top, g_bot = HOOP_BOT - 0.012, HOOP_BOT - 0.20
gw_top, gw_bot = 0.20, 0.07


def gpt(x, y, dz=0.0):
    return (x, y, GZ + dz)


plate = [gpt(-gw_bot / 2, g_bot), gpt(gw_bot / 2, g_bot), gpt(gw_top / 2, g_top), gpt(-gw_top / 2, g_top)]
box8([(p[0], p[1], p[2] - 0.012) for p in plate] + list(plate), GAUGE, 0.3)
# chrome edge along the top of the fan
box8([gpt(-gw_top / 2, g_top - 0.012, -0.012), gpt(gw_top / 2, g_top - 0.012, -0.012), gpt(gw_top / 2, g_top - 0.012),
      gpt(-gw_top / 2, g_top - 0.012), gpt(-gw_top / 2, g_top + 0.004, -0.012), gpt(gw_top / 2, g_top + 0.004, -0.012),
      gpt(gw_top / 2, g_top + 0.004), gpt(-gw_top / 2, g_top + 0.004)], CHROME, 1.0)
# bracket from the hoop to the plate
radial_box(90.0, HOOP_OUT - 0.005, GZ - 0.012, g_top - 0.03, HOOP_BOT + 0.004, 0.03, CHROME, 1.0)
# tick marks fanned around the pivot near the bottom of the plate
pivot = (0.0, g_bot + 0.02)
for j in range(7):
    a = math.radians(90.0 + (j - 3) * 15.0)
    r0, r1 = 0.125, 0.155
    d = (math.cos(a), math.sin(a))
    e = (-d[1] * 0.006, d[0] * 0.006)
    q = [(pivot[0] + d[0] * r0 - e[0], pivot[1] + d[1] * r0 - e[1]),
         (pivot[0] + d[0] * r0 + e[0], pivot[1] + d[1] * r0 + e[1]),
         (pivot[0] + d[0] * r1 + e[0], pivot[1] + d[1] * r1 + e[1]),
         (pivot[0] + d[0] * r1 - e[0], pivot[1] + d[1] * r1 - e[1])]
    face([gpt(x, y, 0.002) for x, y in q], TICK, 0.2, (0.0, H * 0.5, 0.0))
# the pointer, leaning a little to one side, as if mid-tune
a = math.radians(90.0 - 18.0)
d = (math.cos(a), math.sin(a))
e = (-d[1] * 0.011, d[0] * 0.011)
tip = (pivot[0] + d[0] * 0.16, pivot[1] + d[1] * 0.16)
face([gpt(pivot[0] - e[0], pivot[1] - e[1], 0.004), gpt(pivot[0] + e[0], pivot[1] + e[1], 0.004),
      gpt(tip[0], tip[1], 0.004)], POINTER, 0.3, (0.0, H * 0.5, 0.0))

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
    "about": "TIMPANI: a low-poly concert pedal timpani built by studio/scripts/models/timpani_model.py, centred on its "
             "bounding box and scaled to fit a unit sphere",
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
    "name": "TIMPANI",
    "note": "faces are in the order the model builder made them; the sprite renderer draws them in this order (a later "
            "face wins only where it is strictly nearer). Lines (strings) are not part of the model.",
    "source": {"module": "studio/scripts/models/timpani_model.py",
               "note": "generated by studio/scripts/models/timpani_model.py from the photo references in "
                       "studio/reference/models/timpani/",
               "size": 1.0, "yaw": 0},
}

here = os.path.dirname(os.path.abspath(__file__))
dst = os.path.join(here, "..", "..", "crates", "bf-content", "data", "models", "TIMPANI.json")
with open(dst, "w", encoding="utf-8", newline="\n") as fh:
    text = json.dumps(doc, indent=1, sort_keys=True)
    # one vertex (or normal, or colour) per line, as the other model files have them
    num = r"(-?[0-9.eE+-]+)"
    text = re.sub(r"\[\s*" + num + r",\s*" + num + r",\s*" + num + r"\s*\]", r"[\1, \2, \3]", text)
    fh.write(text + "\n")
print("TIMPANI.json:", len(out_faces), "faces")
