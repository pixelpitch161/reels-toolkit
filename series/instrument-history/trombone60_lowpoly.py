"""Pixel Pitch: history of the trombone, 60-second cut.  python3 trombone60_lowpoly.py [out.mp4] | --still t ..."""
import math, sys
import numpy as np
from PIL import Image, ImageDraw
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_shapes as S
import trumpet_lowpoly as T
import piano_lowpoly as P
import drums60_lowpoly as D
from pp_framework import mix, dim, lerp, clamp, midi_hz, seq, SR, RW, RH, CX, YY, XX, BAY, HOT

OUT, TAG = "trombone60_lowpoly.mp4", "tb"
BPM = 112
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 1.55, -8, 7, .3
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE TROMBONE", "600 YEARS", "TODAY", 1400
INTRO_LINES = (("SEVEN POSITIONS,", "ONE GLORIOUS SLIDE"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("600 YEARS FROM", "THE SLIDE TRUMPET")
OUTRO_CREDIT = "NEW ORLEANS BRASS FUNK"

# ---------------------------------------------------------------- music (cha-cha groove in Bb / C minor)
CH = {"Cm7": (36, (58, 63, 67)), "F7": (41, (57, 60, 63)), "Dm": (38, (57, 62, 65)), "C": (36, (55, 60, 64)),
      "Gm": (43, (55, 58, 62)), "F": (41, (57, 60, 65)), "D": (38, (57, 62, 66)), "Bb": (46, (58, 62, 65)),
      "Eb": (39, (55, 58, 63)), "G7": (43, (53, 59, 62)), "C7": (36, (55, 58, 64)), "Bb7": (46, (56, 62, 65)),
      "Cm": (36, (55, 60, 63)), "Fm": (41, (56, 60, 65)), "Eb7": (39, (55, 61, 63))}
BARS = ["Cm7", "F7", "Cm7", "F7",
        "Dm", ("C", "C", "Dm", "Dm"), "Gm", ("F", "F", "D", "Gm"), "Bb", ("Eb", "Eb", "F7", "F7"),
        "C", ("G7", "G7", "C", "C"), "F", ("C7", "C7", "F", "F"), "F7", ("Bb7", "Bb7", "F7", "F7"),
        "Bb", ("Cm7", "Cm7", "F7", "F7"), ("Cm7", "Cm7", "F7", "F7"), "Bb", "Cm", ("Fm", "Fm", "Cm", "Cm"),
        ("Cm", "Cm", "G7", "G7"), ("Cm", "Cm", "G7", "G7"), "Bb7", "Bb7", ("Eb7", "Eb7", "Eb7", "F7"), "Bb7"]
def riff(b0, r, v=.9):
    return [(b0 + o, r + iv, d, v) for o, iv, d in ((0, 0, .5), (.75, 2, .25), (1, 3, .25), (1.25, 4, .25), (1.5, 7, .5),
                                                     (2, 10, .5), (2.5, 7, .25), (2.75, 5, .25), (3, 2, .5), (3.5, 0, .5))]
SHOW = [(b, m, d, .85) for b, m, d in (
    (0, 60, .5), (.5, 63, .5), (1, 65, .5), (1.5, 67, 1), (2.5, 65, .5), (3, 63, .5), (3.5, 60, .5),
    (4, 65, 1.5), (5.5, 63, .5), (6, 60, .5), (6.5, 57, .5), (7, 60, .5), (7.5, 60, .5),
    (8, 60, 1), (9, 67, .5), (9.5, 65, .5), (10, 67, .5), (10.5, 70, 1), (11.5, 67, .5),
    (12, 65, .5), (12.5, 63, .5), (13, 65, 1), (14, 57, .5), (14.5, 53, .5), (15, 48, .5), (15.5, 53, .5))]
SALSA = [(0, [67, 63], .5, .9), (.5, [67, 63], .5, .85), (1.5, [68, 65], .5, .85), (2, [67, 62], .5, .9), (2.5, [65, 62], .5, .85),
         (3, [63, 59], .5, .85), (3.5, [62, 59], .5, .85), (4, [63, 60], 1, .9), (5, [62, 59], .5, .85), (5.5, [60, 56], .5, .85),
         (6, [59, 55], .5, .85), (6.5, [62, 59], .5, .85), (7, [67, 59], 1, .95)]

SCENES = [
 dict(key="modern", theme="intro", name="MODERN TROMBONE", credit="LATIN JAZZ CHA-CHA", notes=SHOW),
 dict(key="slidetr", theme="slidetr", year=1420, ys="1400s", name="SLIDE TRUMPET", credit="BURGUNDIAN BASSE DANSE",
      fact=("A TRUMPET WITH ONE", "SLIDING MOUTHPIPE"),
      notes=seq(0, [(62, 1), (65, .5), (67, .5), (69, 1), (67, .5), (65, .5), (64, 1), (62, .5), (60, .5), (62, 2)], .85)),
 dict(key="sackbut", theme="sackbut", year=1597, ys="1597", name="SACKBUT", credit="IN THE STYLE OF GABRIELI",
      fact=("SACKBUT MEANS", "'PULL-PUSH'"),
      notes=seq(0, [(55, 1.5), (55, .5), (58, 1), (62, 1), (60, 1.5), (58, .5), (57, 1), (55, 1)], .85)),
 dict(key="church", theme="church", year=1791, ys="1791", name="CLASSICAL TROMBONE", credit="IN THE STYLE OF MOZART",
      fact=("MOZART'S REQUIEM", "HAS A TROMBONE SOLO"),
      notes=seq(0, [(58, 1), (62, 1), (65, 1), (70, 1.5), (69, .25), (67, .25), (65, 1), (63, .5), (62, .5), (60, 1)], .85)),
 dict(key="orch", theme="orch", year=1808, ys="1808", name="ORCHESTRAL TROMBONE", credit="IN THE STYLE OF BEETHOVEN",
      fact=("BEETHOVEN'S 5TH MADE", "IT A SYMPHONY STAR"),
      notes=seq(0, [(60, 1), (64, 1), (67, 2), (65, .5), (64, .5), (62, .5), (64, .5), (62, .5), (60, .5), (59, .5), (60, .5)], .9)),
 dict(key="valve", theme="valve", year=1825, ys="1820s", name="VALVE TROMBONE", credit="19TH-CENTURY BAND MARCH",
      fact=("VALVES SWAPPED IN", "FOR THE SLIDE"),
      notes=seq(0, [(53, .75), (53, .25), (57, .5), (60, .5), (65, 1), (60, 1), (62, .75), (60, .25), (58, .5), (55, .5), (57, 1), (53, 1)], .85)),
 dict(key="tailgate", theme="tailgate", year=1925, ys="1920s", name="TAILGATE TROMBONE", credit="IN THE STYLE OF KID ORY",
      fact=("PLAYED FROM THE BACK", "OF PARADE WAGONS"), gl={0: -5, 6.5: -7},
      notes=seq(0, [(53, 1.5), (51, .5), (53, .5), (55, .5), (57, 1), (58, .5), (56, .5), (55, .5), (53, .5), (51, .5), (53, 1.5)], .9)),
 dict(key="swing", theme="swing", year=1938, ys="1930s", name="SWING TROMBONE", credit="IN THE STYLE OF TOMMY DORSEY",
      fact=("TOMMY DORSEY: THE", "SENTIMENTAL GENTLEMAN"),
      notes=seq(0, [(65, 1.5), (67, .5), (70, 1), (72, 1), (70, 1.5), (67, .5), (65, 1), (63, 1)], .85)),
 dict(key="bebop", theme="bebop", year=1947, ys="1940s", name="BEBOP TROMBONE", credit="IN THE STYLE OF J.J. JOHNSON",
      fact=("J.J. JOHNSON PLAYED", "BEBOP AT FULL SPEED"),
      notes=seq(0, [(m, .5) for m in (58, 60, 62, 65, 64, 62, 61, 60, 58, 57, 58, 62)] + [(65, 1), (70, 1)], .85)),
 dict(key="ska", theme="ska", year=1964, ys="1960s", name="SKA TROMBONE", credit="IN THE STYLE OF DON DRUMMOND",
      fact=("JAMAICA'S DON", "DRUMMOND SHAPED SKA"),
      notes=seq(0, [(60, 1), (63, .5), (65, .5), (67, 1.5), (65, .5), (63, 1), (60, .5), (58, .5), (60, 2)], .85)),
 dict(key="salsa", theme="salsa", year=1972, ys="1970s", name="SALSA TROMBONES", credit="IN THE STYLE OF WILLIE COLON",
      fact=("TWO TROMBONES FRONT", "70S NEW YORK SALSA"), notes=SALSA),
 dict(key="modern", theme="intro", year=2015, ys="2010s", name="MODERN TROMBONE", credit="IN THE STYLE OF TROMBONE SHORTY",
      fact=("TROMBONE SHORTY", "KEEPS IT FUNKY"), gl={12: -10},
      notes=riff(0, 46) + riff(4, 58) + riff(8, 51)[:8] + [(11, 53, .5, .9), (11.5, 57, .5, .9), (12, 70, 4.3, .95)]),
]

# ---------------------------------------------------------------- slide maths
PARTIALS = (46, 53, 58, 62, 65, 68, 70, 72, 74, 76, 77)
FPART = tuple(p - 5 for p in PARTIALS)
def position(m, trig_ok=False):
    while m < 40: m += 12
    while m > 77: m -= 12
    p = next((k for k in range(1, 8) if m + k - 1 in PARTIALS), 1)
    if trig_ok and p >= 6:
        q = next((k for k in range(1, 5) if m + k - 1 in FPART), None)
        if q: return q, True
    return p, False
def gl_of(s, beat):
    for b, v in s.get("gl", {}).items():
        if abs(b - beat) < 1e-3: return v
    return 0
GLT = .14
def track(s, t, pick, trig_ok=False, off=0):
    """slide position at scene time t for the voice picked from s['ev'] (pick(e) -> bool); glides into each note"""
    tops = [e for e in s["ev"] if pick(e)]
    if not tops: return 1., False
    prev = max((e for e in tops if e[0] <= t), default=None)
    nxt = min((e for e in tops if e[0] > t), default=None)
    if prev is None: return float(position(tops[0][1] + off, trig_ok)[0]), position(tops[0][1] + off, trig_ok)[1]
    pa, tr = position(prev[1] + off, trig_ok); pa = float(pa)
    g = gl_of(s, prev[0]/BEAT_S)
    if g and t - prev[0] < GLT:
        p0 = float(position(prev[1] + off + g, trig_ok)[0]); k = (t - prev[0])/GLT; pa = p0 + (pa - p0)*(1 - (1 - k)**2)
    if nxt is None: return pa, tr
    nb = nxt[1] + off + gl_of(s, nxt[0]/BEAT_S)
    pb = float(position(nb, trig_ok)[0]); win = min(.08, (nxt[0] - prev[0])*.5)
    k = clamp((t - (nxt[0] - win))/win)
    return pa + (pb - pa)*k*k*(3 - 2*k), tr

# ---------------------------------------------------------------- instruments (built to real proportions, in metres)
# A modern Bb tenor is ~1.15 m long with the slide closed: a 70 cm slide (tubes ~9 cm apart) and a bell section
# (bell tube + return tube ~10 cm apart, 21.6 cm bell, tuning-slide bow ~44 cm behind the slide lock).
# The two sections join at the slide lock and sit at roughly a right angle to each other, so the slide lies
# almost edge-on when the bell section faces you - like a product photo of the real instrument.
BRASS, NICKEL, CHROME = (230, 182, 80), (214, 216, 226), (236, 238, 244)
_MOD = dict(r=.0095, bell=.108, x0=.046, m=.79, rim=.46, back=.44, slen=.70, tuning=True)
TB = {
 "modern":   dict(_MOD, col=BRASS, slide=NICKEL, trig=True),
 "tailgate": dict(_MOD, col=(208, 168, 90), slide=(204, 204, 212), r=.009, bell=.095, x0=.042, m=.8, wagon=True),
 "swing":    dict(_MOD, col=(234, 192, 98), slide=NICKEL, r=.009, bell=.1, x0=.044, m=.8, hat=True),
 "bebop":    dict(_MOD, col=(238, 198, 112), slide=NICKEL, r=.009, bell=.1, x0=.044, m=.8, mic=True),
 "ska":      dict(_MOD, col=(210, 212, 222), slide=CHROME, bell=.104, checker=True),
 "salsa":    dict(_MOD, col=BRASS, slide=NICKEL, bell=.106, pair=True),
 "valve":    dict(_MOD, col=(226, 178, 82), slide=(226, 178, 82), bell=.1, x0=.044, m=.8, slen=.6),
 # sackbut / classical trombones: narrow bore, small gently flared bell with a garland, flat stays, no tuning slide
 "sackbut":  dict(col=(196, 146, 64), slide=(196, 146, 64), r=.0075, bell=.052, x0=.12, m=.75, rim=.4, back=.42, slen=.63, flat=True, garland=(232, 196, 110)),
 "church":   dict(col=(212, 164, 78), slide=(212, 164, 78), r=.008, bell=.066, x0=.09, m=.78, rim=.42, back=.43, slen=.66, flat=True, garland=(244, 214, 130)),
 "orch":     dict(col=(222, 176, 86), slide=(222, 176, 86), r=.0085, bell=.076, x0=.075, m=.8, rim=.44, back=.43, slen=.68, flat=True, garland=(200, 150, 62)),
 "slidetr":  dict(col=(232, 190, 92), bell=.055, x0=.08, m=.8, garland=(246, 210, 110)),
}
SD, STEP, HB, GDROP = .09, .085, .105, .05    # slide tube spacing, slide travel per position, bell-section spacing, gooseneck drop
def rot_x(a): c, s_ = math.cos(a), math.sin(a); return np.array([[1, 0, 0], [0, c, -s_], [0, s_, c]])
def rot_y(a): c, s_ = math.cos(a), math.sin(a); return np.array([[c, 0, s_], [0, 1, 0], [-s_, 0, c]])
def rot_z(a): c, s_ = math.cos(a), math.sin(a); return np.array([[c, -s_, 0], [s_, c, 0], [0, 0, 1]])
SLIDE_PLANE = rot_x(math.radians(100))                          # slide plane ~80-90 degrees from the bell section
POSE = rot_y(math.radians(-36)) @ rot_z(math.radians(-12))       # bell turned toward the viewer, slide end dipping
ORIGIN = np.array([-.2, .44, 0.])
def xform(g, f0, l0, R=None, t=(0, 0, 0)):
    t = np.asarray(t, float)
    for i in range(f0, len(g.F)):
        P_, n, col, two, emit, gid, spec = g.F[i]
        if R is not None: P_ = P_ @ R.T; n = R @ n
        g.F[i] = (P_ + t, n, col, two, emit, gid, spec)
    for i in range(l0, len(g.L)):
        a, b, col, gid = g.L[i]
        if R is not None: a, b = R @ a, R @ b
        g.L[i] = (a + t, b + t, col, gid)
def box_at(g, p, s, col, emit=False):
    g.box(p[0] - s, p[0] + s, p[1] - s, p[1] + s, p[2] - s, p[2] + s, col, emit=emit)
def path(g, pts, r, col, n=8):
    for a, b in zip(pts, pts[1:]): T.tube(g, a, b, r, col=col, n=n)

def bessel_bell(g, xa, xb, y, z, c, r_back):
    """bell tube from xa to the rim at xb with a Bessel-horn flare r(d) = bell*(x0/(d+x0))^m, d = distance from the rim"""
    L = xb - xa; col = c["col"]; R, x0, m = c["bell"], c["x0"], c["m"]
    def rad(d): return max(r_back, R*(x0/(d + x0))**m)
    ds = [L*(k/16)**2.2 for k in range(17)][::-1]
    for d0, d1 in zip(ds, ds[1:]):
        T.tube(g, (xb - d0, y, z), (xb - d1, y, z), rad(d0), rad(d1), col=col, n=14 if d1 < .12 else 10, spec=.9, two=d1 < .3)
    T.tube(g, (xb - .006, y, z), (xb + .002, y, z), R*1.045, R*1.045, col=col, n=14, spec=.9, two=True)
    g.face(S.ring((xb - .05, y, z), (1, 0, 0), rad(.05)*.8, 12), (28, 18, 10), ref=(xb - 1, y, z), emit=True, two=True)
    if c.get("garland"):
        T.tube(g, (xb - .075, y, z), (xb - .012, y, z), rad(.075)*1.05, rad(.012)*1.05, col=c["garland"], n=14, spec=.6, two=True)
        for d in (.07, .018): T.tube(g, (xb - d - .003, y, z), (xb - d + .003, y, z), rad(d)*1.09, col=dim(c["garland"], .8), n=14, two=True)
    return np.array([xb, y, z])

def slide_part(g, c, pos, fixed=False):
    """slide in its own plane: connecting tube on y=0, leadpipe (mouthpiece) tube on y=SD; rotated into place afterwards"""
    col, sl, r, L = c["col"], c["slide"], c["r"], c["slen"]; R = SD/2; e = 0 if fixed else (pos - 1)*STEP
    xo, xc = .06 + e, L - R + e
    if fixed:
        for y in (0, SD): T.tube(g, (0, y, 0), (xc, y, 0), r, col=col, n=8)
    else:
        inner = CHROME if c.get("tuning") else dim(sl, 1.06)
        for y in (0, SD):
            T.tube(g, (0, y, 0), (xo + .01, y, 0), r*.8, col=inner, n=8)          # inner slide, shows as it extends
            T.tube(g, (xo, y, 0), (xc, y, 0), r, col=sl, n=8)                      # outer slide
            T.tube(g, (xo - .002, y, 0), (xo + .018, y, 0), r*1.3, col=col, n=8)
    T.bend(g, (xc, R, 0), R, r, -90, 90, col=col if fixed else sl, segs=7)        # crook
    g.box(xc + R + r*.4, xc + R + r*.4 + .012, R - .012, R + .012, -.008, .008, (40, 36, 40))
    g.box(xc - .075, xc - .025, -r - .006, -r, -.005, .005, col)                  # water key
    for x, grip in ((xo + .045, True), (.03, False)):
        if fixed and grip: x = xc - .12
        if c.get("flat"): g.box(x - .009, x + .009, 0, SD, -.002, .002, dim(col, .9))
        else:
            T.tube(g, (x, 0, 0), (x, SD, 0), r*.7, col=col, n=6)
            if grip and not fixed: T.tube(g, (x, SD*.28, 0), (x, SD*.72, 0), r*1.3, col=col, n=6)
    T.tube(g, (0, SD, 0), (-.065, SD, 0), r*.95, r*.75, col=NICKEL, n=8)          # mouthpiece
    T.tube(g, (-.065, SD, 0), (-.084, SD, 0), r*.85, r*2.1, col=NICKEL, n=10)
    T.tube(g, (-.084, SD, 0), (-.09, SD, 0), r*2.2, col=NICKEL, n=10)
    T.tube(g, (-.004, 0, 0), (.022, 0, 0), r*1.75, col=col, n=8)                  # slide-lock nut

def bell_section(g, c, trig=None):
    col, r = c["col"], c["r"]; back, rim = -c["back"], c["rim"]; Rt = HB/2
    gn = [(.0, -GDROP, 0), (-.022, -GDROP + .004, 0), (-.042, -GDROP/2, 0), (-.062, -.004, 0), (-.085, 0, 0)]
    path(g, [np.array(p, float) for p in gn], r, col)                              # gooseneck
    T.tube(g, (-.085, 0, 0), (back + Rt, 0, 0), r, col=col, n=8)                  # return tube
    T.bend(g, (back + Rt, Rt, 0), Rt, r, 270, 90, col=col, segs=8)                # tuning-slide bow
    if c.get("tuning"):
        for y in (0, HB): T.tube(g, (back + Rt, y, 0), (back + Rt + .14, y, 0), r*1.25, col=col, n=8)
        for x in (back + Rt + .05, back + Rt + .12): T.tube(g, (x, 0, 0), (x, HB, 0), r*.55, col=col, n=6)
        g.face(S.ring((back - .004, Rt, .012), (0, 0, 1), .014, 8), dim(col, .85), ref=(back, Rt, 0))
    mouth = bessel_bell(g, back + Rt, rim, HB, 0, c, r)
    if c.get("flat"): g.box(-.12, -.1, 0, HB, -.003, .003, dim(col, .9))
    else: T.tube(g, (-.11, 0, 0), (-.11, HB, 0), r*.55, col=col, n=6)
    if trig is not None:                                                           # F attachment: rotor, wrap, thumb trigger
        on = trig; rx = -.18; cx, cy, ax, ay = rx - .12, -.07, .12, .045
        T.tube(g, (rx, 0, -.022), (rx, 0, .022), .018, col=col, n=10)
        g.face(S.ring((rx, 0, .023), (0, 0, 1), .015, 8), HOT if on else dim(col, .8), ref=(rx, 0, 0), emit=on)
        T.tube(g, (rx, 0, 0), (rx, cy, 0), r, col=col, n=8)
        path(g, [np.array([cx + ax*math.cos(a), cy + ay*math.sin(a), 0]) for a in np.linspace(0, 2*math.pi, 19)], r*.95, col)
        pad = np.array([-.07, HB + .035 - (.012 if on else 0), .02])
        for off in (0, .003): g.line((rx + off, .01, .024), pad + [off, 0, 0], dim(col, .7))
        g.box(pad[0] - .016, pad[0] + .016, pad[1] - .006, pad[1] + .006, pad[2] - .012, pad[2] + .012, HOT if on else (236, 230, 214), emit=on)
    return mouth

def trombone(g, c, pos, trig=None, fing=None):
    """local frame: x = forward, bell section in the x-y plane, slide hanging from the slide lock in its own plane"""
    f0, l0 = len(g.F), len(g.L)
    slide_part(g, c, pos, fixed=fing is not None)
    xform(g, f0, l0, R=SLIDE_PLANE, t=(0, -GDROP, 0))
    if fing is not None:                                                           # valve trombone: three pistons on the slide side
        col, r = c["col"], c["r"]; y0 = -GDROP
        for i, (x, dep) in enumerate(((.22, .03), (.16, .02), (.10, .045))):
            T.tube(g, (x, y0 - .045, 0), (x, y0 + .035, 0), .0125, col=col, n=10)
            T.tube(g, (x, y0 + .035, 0), (x, y0 + .042, 0), .0145, col=NICKEL, n=10)
            T.tube(g, (x, y0 - .045, 0), (x, y0 - .052, 0), .0145, col=NICKEL, n=10)
            down = str(i + 1) in fing; top = y0 + .06 - (.014 if down else 0)
            T.tube(g, (x, y0 + .042, 0), (x, top, 0), .003, col=NICKEL, n=6)
            T.tube(g, (x, top, 0), (x, top + .008, 0), .011, col=HOT if down else (236, 230, 214), n=10, spec=.4)
            for dx in (-.02, .02): T.tube(g, (x + dx, y0 - .03, -.018), (x + dx, y0 - .06 - dep, -.018), r*.8, col=col, n=6)
            T.bend(g, (x, y0 - .06 - dep, -.018), .02, r*.8, 180, 360, col=col, segs=4)
    return bell_section(g, c, trig)

def slide_trumpet(g, c, pos):
    """Renaissance slide trumpet: a folded trumpet whose whole body slides along a long mouthpipe"""
    col, r, e = c["col"], .0085, (pos - 1)*.06; G = c["garland"]; y0 = .12
    T.tube(g, (-.40, y0, 0), (-.47, y0, 0), r*.95, r*.75, col=col, n=8)
    T.tube(g, (-.47, y0, 0), (-.49, y0, 0), r*.85, r*2.4, col=col, n=10)
    T.tube(g, (-.40, y0, 0), (-.10 + e, y0, 0), r*.75, col=NICKEL, n=8)
    T.tube(g, (-.12 + e, y0, 0), (.38 + e, y0, 0), r, col=col, n=8)
    T.tube(g, (-.125 + e, y0, 0), (-.095 + e, y0, 0), r*1.6, col=G, n=8)
    T.bend(g, (.38 + e, y0 - .06, 0), .06, r, 90, -90, col=col, segs=6)
    T.tube(g, (.38 + e, y0 - .12, 0), (-.22 + e, y0 - .12, 0), r, col=col, n=8)
    T.bend(g, (-.22 + e, y0 - .18, 0), .06, r, 90, 270, col=col, segs=6)
    T.tube(g, (-.22 + e, y0 - .24, 0), (-.02 + e, y0 - .24, 0), r, col=col, n=8)
    for x, yy in ((-.15, y0 - .24), (.15, y0 - .12)): T.tube(g, (x - .02 + e, yy, 0), (x + .02 + e, yy, 0), r*2.2, col=G, n=8)
    mouth = bessel_bell(g, -.02 + e, .44 + e, y0 - .24, 0, c, r)
    cord = [(-.05 + e + .4*k/8, y0 - .12 - .07*math.sin(math.pi*k/8), .015) for k in range(9)]
    for a, b in zip(cord, cord[1:]): g.line(a, b, (40, 60, 160))
    return mouth

def derby(g, mouth, opn, bell):
    hx = mouth[0] + .03 + .1*opn; y, z = mouth[1], mouth[2]; crown, band = (54, 50, 60), (150, 34, 44)
    g.face(S.ring((hx, y, z), (1, 0, 0), bell*1.3, 12), crown, ref=(hx + 1, y, z), two=True)
    T.tube(g, (hx, y, z), (hx + .08, y, z), bell*.8, bell*.66, col=crown, n=12, spec=.3)
    T.tube(g, (hx + .005, y, z), (hx + .022, y, z), bell*.82, bell*.8, col=band, n=12, spec=.2)
    g.face(S.ring((hx + .08, y, z), (1, 0, 0), bell*.66, 12), dim(crown, .9), ref=(hx, y, z))

def wagon(g):
    wood, dark = (150, 100, 56), (100, 64, 34)
    g.box(-.7, .55, .06, .11, -.24, .2, wood, spec=.1)
    for x in np.linspace(-.6, .45, 5): g.line((x, .111, .2), (x, .111, -.24), dark)
    c = np.array([-.5, .1, -.3])
    T.bend(g, c, .15, .014, 0, 360, col=dark, segs=16)
    for k in range(8):
        a = k*math.pi/4; g.line(c, c + [.14*math.cos(a), .14*math.sin(a), 0], dark)
    box_at(g, c, .028, (60, 40, 22))

def checker(g):
    for i, x in enumerate(np.arange(-.8, .8, .16)):
        for j, z in enumerate(np.arange(-.32, .32, .16)):
            col = (236, 236, 236) if (i + j) % 2 == 0 else (24, 24, 28)
            g.face([(x, .004, z), (x + .16, .004, z), (x + .16, .004, z + .16), (x, .004, z + .16)], col, ref=(x, -1, z))

def mic(g, head):
    base = np.array([.62, 0, -.32]); top = np.array([.62, head[1] + .14, -.32])
    S.rod(g, base, top, .01); g.ngon(base[0], .004, base[2], .12, (40, 38, 46), n=8); S.rod(g, top, head + [.04, .04, 0], .008)
    box_at(g, head, .035, (190, 194, 204)); g.box(head[0] - .036, head[0] + .036, head[1] - .006, head[1] + .006, head[2] - .036, head[2] + .036, (40, 40, 48))

def wah_open(s, t):
    live = [e for e in s["ev"] if e[0] <= t < e[0] + e[3] + .05]
    if not live: return .2
    e = max(live); return .15 + .85*clamp((t - e[0])/max(.12, e[3]*.75))

def build(g, t, ev, fx, s):
    k = s["key"]; c = TB[k]; f0, l0 = len(g.F), len(g.L); origin = ORIGIN
    if k == "slidetr":
        pos, _ = track(s, t, lambda e: e[4], off=-12); pos = 1 + min(pos - 1, 3.5)
        mouths = [slide_trumpet(g, c, pos)]
    elif k == "valve":
        cur = T.sounding(ev); mouths = [trombone(g, c, 1, fing=T.fingering(cur[1] + 12) if cur else "")]
    elif c.get("pair"):
        pa, _ = track(s, t, lambda e: e[4]); pb, _ = track(s, t, lambda e: not e[4])
        mouths = [trombone(g, c, pa)]
        xform(g, f0, l0, R=POSE, t=origin + [.02, -.08, .1]); mouths = [POSE @ mouths[0] + origin + [.02, -.08, .1]]
        f1, l1 = len(g.F), len(g.L); m2 = trombone(g, c, pb)
        o2 = origin + [-.1, .2, -.38]; xform(g, f1, l1, R=POSE, t=o2); mouths.append(POSE @ m2 + o2)
    else:
        pos, tr = track(s, t, lambda e: e[4], trig_ok=bool(c.get("trig")))
        mouths = [trombone(g, c, pos, trig=tr if c.get("trig") else None)]
        if c.get("hat"): derby(g, mouths[0], wah_open(s, t), c["bell"])
    if not c.get("pair"):
        xform(g, f0, l0, R=POSE, t=origin); mouths = [POSE @ m + origin for m in mouths]
    if c.get("wagon"): wagon(g)
    if c.get("checker"): checker(g)
    if c.get("mic"): mic(g, mouths[0] + POSE @ np.array([.2, .04, 0]))
    ax = POSE @ np.array([1., 0, 0])
    for i, mo in enumerate(mouths):
        sub = [e for e in ev if (e[4] if i == 0 else not e[4])] if len(mouths) > 1 else ev
        T.brass_fx(fx, sub, tuple(mo + (ax*.16 if c.get("hat") else 0)))

# ---------------------------------------------------------------- sound
def _t(n): return np.arange(n)/SR
def brass2(f, dur, vel, bright=1., vib=.004, scoop=.03, tilt=.8, gl=0., wah=0., att=.03):
    n = int((dur + .14)*SR); t = _t(n)
    env = np.minimum(1, t/att)**1.5*np.where(t < dur, 1, np.exp(-(t - dur)/.035))*(.88 + .12*np.exp(-t*3))
    bendc = 2**(gl*(1 - np.clip(t/GLT, 0, 1))**2/12) if gl else 1.
    fr = f*bendc*(1 - scoop*np.exp(-t/.025))*(1 + vib*np.sin(2*np.pi*5.4*t)*np.clip((t - .18)/.2, 0, 1))
    ph = 2*np.pi*np.cumsum(fr)/SR
    dark = np.zeros(n); brt = np.zeros(n); sd = sb = 0
    for k in range(1, 34):
        if f*k > 7000: break
        a1 = math.exp(-k/2.2)/k**tilt; a2 = math.exp(-k/(4 + 6*bright))/k**tilt
        s_ = np.sin(k*ph); dark += a1*s_; brt += a2*s_; sd += a1; sb += a2
    mb = np.clip(env*vel, 0, 1)**1.3
    if wah: mb = mb*(1 - wah + wah*np.clip(t/max(dur*.75, .12), 0, 1))
    return (dark/sd*(1 - mb) + brt/sb*mb)*env*vel
TONE = {"modern": dict(bright=1.0, vib=.003, scoop=.03), "slidetr": dict(bright=.8, vib=0, scoop=.04, tilt=.9),
        "sackbut": dict(bright=.3, vib=0, scoop=.025, tilt=1.0, att=.04), "church": dict(bright=.5, vib=.002, scoop=.025, tilt=.95),
        "orch": dict(bright=.9, vib=.002, scoop=.03), "valve": dict(bright=.7, vib=.004, scoop=.03),
        "tailgate": dict(bright=1.1, vib=.007, scoop=.05), "swing": dict(bright=.7, vib=.007, scoop=.02, wah=.75),
        "bebop": dict(bright=.9, vib=.002, scoop=.02, att=.02), "ska": dict(bright=.8, vib=.004, scoop=.03),
        "salsa": dict(bright=1.5, vib=0, scoop=.04, att=.02)}
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]
    x = brass2(midi_hz(m), dur, v, gl=gl_of(s, nt/BEAT_S), **TONE[s["key"]])
    return x*(.75 if cs > 1 else 1.)

# ---------------------------------------------------------------- backing: cha-cha (cowbell, guiro, congas, timbales, tumbao, montuno)
def cow(fa, fb, dec):
    def fn(v):
        n = int(.35*SR); t = _t(n)
        x = np.sign(np.sin(2*np.pi*fa*t)) + .8*np.sign(np.sin(2*np.pi*fb*t))
        return D.bp(x, 500, 4000)*np.exp(-t*dec)*np.minimum(1, t/.001)*.45*v
    return fn
def guiro(dur):
    def fn(v):
        n = int((dur + .03)*SR); t = _t(n)
        am = .5 + .5*np.tanh(4*np.sin(2*np.pi*(46 + 30*t/dur)*t))
        env = np.minimum(1, t/.012)*np.where(t < dur, 1, np.exp(-(t - dur)/.008))
        return D.bp(D.nz(n), 2500, 7000)*am*env*.32*v
    return fn
def conga(f0, slap=False):
    def fn(v):
        n = int(.3*SR); t = _t(n); fr = f0*(1 + .1*np.exp(-t*30))
        x = np.sin(2*np.pi*np.cumsum(fr)/SR)*np.exp(-t*(30 if slap else 14))
        return (x + D.bp(D.nz(n), 900, 5000)*np.exp(-t*(70 if slap else 60))*(.8 if slap else .25))*v
    return fn
def timb(f0):
    def fn(v):
        n = int(.45*SR); t = _t(n)
        x = np.sin(2*np.pi*f0*t)*np.exp(-t*9) + .5*np.sin(2*np.pi*2.71*f0*t)*np.exp(-t*14) + D.bp(D.nz(n), 1500, 7000)*np.exp(-t*45)*.4
        return x*.8*v
    return fn
KIT = {"kick": lambda v: D.kick_s(52, 100, 7, .05, .45, 1.3)*v, "cowL": cow(560, 845, 11), "cowH": cow(690, 1040, 13),
       "gl": guiro(.13), "gs": guiro(.06), "conga_lo": conga(185), "conga_hi": conga(265), "slap": conga(330, True),
       "timbH": timb(520), "timbL": timb(390), "crash": P.crash}
KICK_KINDS, ACCENT_KINDS = ("kick",), ("slap", "timbH", "timbL")
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT; nb = len(BARS)
    for b in range(nb):
        T0 = b*B
        if b == nb - 1: ev += [(T0, "kick", 1.), (T0, "crash", 1.), (T0, "timbL", .9), (T0, "cowL", .8)]; continue
        ev += [(T0, "cowL", .8), (T0 + Q, "cowH", .55), (T0 + 2*Q, "cowL", .7), (T0 + 3*Q, "cowH", .55)]
        ev += [(T0, "gl", .7), (T0 + Q, "gs", .5), (T0 + 1.5*Q, "gs", .5), (T0 + 2*Q, "gl", .7), (T0 + 3*Q, "gs", .5), (T0 + 3.5*Q, "gs", .5)]
        if b == 0: continue
        ev += [(T0, "kick", .8), (T0 + 2*Q, "kick", .7), (T0, "conga_hi", .35), (T0 + Q, "slap", .7), (T0 + 2*Q, "conga_hi", .35),
               (T0 + 3*Q, "conga_lo", .8), (T0 + 3.5*Q, "conga_lo", .75)]
        if b % 2 == 1 and b < nb - 2:
            ev += [(T0 + 3*Q, "timbH", .55), (T0 + 3.25*Q, "timbH", .45), (T0 + 3.5*Q, "timbL", .6), (T0 + 3.75*Q, "timbL", .55)]
        if b in (4, 24): ev.append((T0, "crash", .6))
    return ev
def _chord_at(b, beat):
    ch = BARS[min(b, len(BARS) - 1)]; return ch[int(beat)] if isinstance(ch, tuple) else ch
def _root(name):
    r = CH[name][0]
    while r > 45: r -= 12
    while r < 34: r += 12
    return r
def bass_events(c):
    out = []; nb = len(BARS)
    for b in range(1, nb):
        T0 = b*c.BAR
        if b == nb - 1: out.append((T0, _root(_chord_at(b, 0)), 2.4, 1.)); continue
        out += [(T0, _root(_chord_at(b, 0)), .9*c.BEAT, .8), (T0 + 1.5*c.BEAT, _root(_chord_at(b, 1.5)) + 7, 1.4*c.BEAT, .85),
                (T0 + 3*c.BEAT, _root(_chord_at(b + 1, 0)), 1.0*c.BEAT, .9)]
    return out
def bass_voice(m, dur, v):
    f = midi_hz(m); n = int((dur + .25)*SR); t = _t(n); ph = 2*np.pi*f*t
    x = np.sin(ph) + .3*np.sin(2*ph) + .1*np.sin(3*ph)*np.exp(-t*6)
    env = np.minimum(1, t/.006)*np.exp(-t*1.4)*np.where(t < dur, 1, np.exp(-(t - dur)/.05))
    return np.tanh(1.4*x*env)*v
def _pno(notes, dur, v):
    n = int((dur + .3)*SR); out = np.zeros(n)
    for m in notes:
        x = P.additive(midi_hz(m), dur + .25, v, harm=10, tilt=1.25, d1=1.6); out[:len(x)] += x[:n]
    t = _t(n); return out*np.where(t < dur, 1, np.exp(-(t - dur)/.05))
def pad_voice(notes, dur, v):
    n = int((dur + .5)*SR); out = np.zeros(n); hi = [m + 12 for m in notes]; lo = sorted(notes)
    if abs(dur - (BAR_S + .05)) < .1:
        hits = [(0, [lo[0], lo[0] + 12] + hi[1:]), (1.5, hi), (2.5, [lo[0] + 12] + hi[1:]), (3.5, hi)]
    elif dur < BEAT_S + .2: hits = [(0, hi)]
    else: hits = [(k*.06, [m]) for k, m in enumerate(lo + hi)]
    for b, ms in hits:
        x = _pno(ms, .32*BEAT_S if dur < 2 else 2.2, .6*v); a = int(b*BEAT_S*SR); e = min(n, a + len(x)); out[a:e] += x[:e - a]
    return out
MIX = dict(drums=-21, bass=-23, pad=-29, verb=-24, verb_len=1.3)

# ---------------------------------------------------------------- background: moonlit plaza, papel-picado banners, palms
THEMES = {   # sky top, sky bottom, moon, buildings, window light, ground, banner colours
 "intro":    ((16, 8, 44), (124, 40, 112), (255, 244, 214), (40, 18, 58), (255, 206, 96), (58, 26, 70),
              ((255, 70, 150), (255, 214, 60), (60, 220, 210), (255, 140, 40))),
 "slidetr":  ((10, 34, 54), (196, 118, 62), (255, 232, 184), (30, 28, 40), (255, 176, 70), (60, 44, 44),
              ((210, 40, 40), (240, 200, 70), (40, 80, 190), (240, 236, 220))),
 "sackbut":  ((26, 34, 96), (226, 140, 140), (255, 240, 220), (44, 34, 70), (255, 214, 140), (80, 56, 86),
              ((190, 20, 50), (240, 196, 70), (246, 236, 214), (40, 50, 130))),
 "church":   ((22, 14, 40), (112, 80, 150), (250, 246, 255), (34, 24, 56), (255, 230, 160), (60, 46, 84),
              ((250, 250, 250), (240, 200, 90), (190, 150, 240), (130, 200, 255))),
 "orch":     ((12, 18, 32), (82, 110, 150), (236, 242, 255), (24, 30, 46), (255, 220, 150), (44, 52, 72),
              ((200, 40, 40), (240, 240, 240), (150, 170, 200), (230, 190, 80))),
 "valve":    ((4, 30, 30), (40, 124, 104), (240, 255, 230), (14, 38, 38), (255, 230, 140), (28, 60, 54),
              ((40, 170, 80), (220, 50, 50), (246, 236, 200), (240, 200, 70))),
 "tailgate": ((30, 16, 8), (172, 102, 50), (255, 236, 196), (48, 28, 18), (255, 196, 90), (76, 50, 34),
              ((130, 50, 180), (40, 170, 80), (240, 196, 50), (246, 240, 230))),
 "swing":    ((8, 12, 40), (40, 96, 126), (230, 250, 255), (16, 26, 50), (180, 240, 255), (30, 48, 70),
              ((210, 216, 230), (60, 220, 240), (255, 120, 180), (255, 255, 255))),
 "bebop":    ((30, 6, 12), (140, 40, 52), (255, 230, 200), (40, 12, 20), (255, 180, 90), (66, 24, 32),
              ((255, 180, 60), (220, 40, 50), (246, 230, 200), (60, 30, 40))),
 "ska":      ((6, 28, 12), (64, 146, 64), (255, 250, 220), (12, 34, 18), (255, 220, 70), (30, 60, 34),
              ((220, 40, 40), (250, 210, 40), (40, 180, 70), (240, 240, 240))),
 "salsa":    ((60, 10, 22), (242, 92, 40), (255, 240, 200), (70, 16, 26), (255, 230, 120), (110, 30, 36),
              ((255, 230, 40), (255, 50, 170), (40, 220, 240), (255, 255, 255))),
}
def ped_cols(th): T_ = THEMES[th]; return F.ped_cols_from(mix(T_[5], T_[1], .4))
GY = 300
SKY = {k: F.make_grad(v[0], v[1], y0=0, y1=GY) for k, v in THEMES.items()}
MX, MY = 214, 150
MOON_D = np.hypot(XX - MX, YY - MY)
_rs = np.random.default_rng(7)
STARS = np.zeros((RH, RW), bool); STAR_PH = _rs.random((RH, RW))*6.28
for _ in range(120): STARS[_rs.integers(0, 230), _rs.integers(0, RW)] = True
BLD = np.zeros((RH, RW), bool); WINS = []
x = 0
while x < RW:
    w = int(_rs.integers(26, 46)); top = int(_rs.integers(214, 252))
    BLD[top:GY, x:x + w] = True
    if _rs.random() < .5: BLD[top - 6:top, x + 3:x + w - 3] = True
    for wx in range(x + 5, x + w - 6, 9):
        for wy in range(top + 6, GY - 14, 14): WINS.append((wx, wy, int(_rs.integers(0, 14))))
    x += w + int(_rs.integers(0, 4))
TOWER_X = 38
BLD[178:GY, TOWER_X - 9:TOWER_X + 9] = True
BLD[(np.hypot(XX - TOWER_X, YY - 178) < 10) & (YY < 178)] = True
BLD[156:168, TOWER_X - 1:TOWER_X + 1] = True
WINS.append((TOWER_X - 3, 186, 3))
WINS = [w for w in WINS if w[0] + 4 < RW]
GROUND = YY >= GY
_gz = 1 + (YY - GY)/12
TILES = GROUND & (((np.maximum(YY - GY, 0)**.8) % 7 < 1) | ((((XX - CX)/np.maximum(_gz, 1)) % 14) < 1.1))
TEXTB = ((YY >= 40) & (YY < 100)) | ((YY >= 330) & (YY < 380))
TEXTE = (((YY >= 36) & (YY < 40)) | ((YY >= 100) & (YY < 104)) | ((YY >= 326) & (YY < 330)) | ((YY >= 380) & (YY < 384))) & (BAY < .5)
PALMS = [(18, 1.0, 1), (RW - 16, 1.0, -1), (60, .72, 1), (RW - 58, .72, -1)]

def palm(d, x0, sc, side, T_, kick, col, hi):
    top = (x0 + side*18*sc, GY - 150*sc)
    pts = [(x0 + side*18*sc*(k/8)**1.6, GY - 150*sc*k/8) for k in range(9)]
    for k in range(8):
        d.line([pts[k], pts[k + 1]], fill=col, width=max(2, int((6 - k*.4)*sc)))
    for k in range(9):
        a = math.radians(-90 + side*(-150 + 38*k)) + .08*math.sin(T_*1.3 + k) + .12*kick*side
        L = (34 + 8*math.sin(k*1.7))*sc; prev = top
        for j in range(1, 7):
            u = j/6; p = (top[0] + math.cos(a)*L*u, top[1] + math.sin(a)*L*u + 16*sc*u*u)
            d.line([prev, p], fill=hi if j == 1 else col, width=max(1, int((4 - j*.5)*sc))); prev = p
    for k in range(3): d.ellipse([top[0] - 4 + 3*k - 1, top[1] + 1, top[0] - 4 + 3*k + 3, top[1] + 5], fill=hi)

def banners(d, T_, eq, pulse, cols, sky_at):
    for si, (y0, sag, n0, ph) in enumerate(((104, 10, 0, 0.), (134, 14, 7, 1.7))):
        def yy(x): return y0 + sag*(1 - ((x - CX)/(CX + 6))**2)
        d.line([(x, yy(x)) for x in range(-4, RW + 6, 6)], fill=(30, 20, 30), width=1)
        for i, fx in enumerate(range(6 + si*9, RW, 19)):
            band = (i + n0) % 14; top = yy(fx); w, h = 14, 17
            sw = (2 + 5*eq[band])*math.sin(T_*4.2 + i*1.3 + ph)
            col = cols[(i + si) % len(cols)]
            poly = [(fx - w/2, top), (fx + w/2, top), (fx + w/2 + sw, top + h)]
            for k in range(4, -1, -1): poly.append((fx - w/2 + sw + w*k/4, top + h + (2 if k % 2 else 0)))
            d.polygon(poly, fill=col)
            hc = sky_at(int(top + h/2))
            for hx, hy in ((-3, 5), (3, 5), (0, 10), (-3, 13), (3, 13)):
                px = fx + hx + sw*(hy/h); d.rectangle([px - 1, top + hy, px, top + hy + 1], fill=hc)
            if i % 2 == 0:
                bc = mix((120, 90, 60), (255, 236, 160), .4 + .6*pulse)
                d.rectangle([fx + 9, yy(fx + 9) + 1, fx + 10, yy(fx + 9) + 2], fill=bc)

def background(c, x):
    th0, th1, kb, T_ = x["th0"], x["th1"], x["kb"], x["T"]
    A_, B_ = THEMES[th0], THEMES[th1]
    def col(i): return tuple(int(v) for v in mix(A_[i], B_[i], kb))
    bg = SKY[th0]*(1 - kb) + SKY[th1]*kb
    eq, loud = x["eq"], x["loud"]
    tw = np.sin(STAR_PH + T_*3) > -.2
    bg[STARS & tw] = bg[STARS & tw]*.3 + np.array(col(2))*.7
    r = 15 + 3*loud
    bg[MOON_D < r] = col(2)
    cr = (np.hypot(XX - MX - 4, YY - MY + 3) < r*.28) | (np.hypot(XX - MX + 5, YY - MY - 5) < r*.2)
    bg[cr & (MOON_D < r)] = np.array(col(2))*.88
    gm = (MOON_D >= r) & (MOON_D < r + 16) & (BAY < .45*(1 - (MOON_D - r)/16)); bg[gm] = bg[gm]*.5 + np.array(col(2))*.5
    bc = np.array(col(3), float); bg[BLD] = bc
    wl = np.array(col(4), float)
    for wx, wy, band in WINS:
        k = .25 + .75*eq[band]
        bg[wy:wy + 7, wx:wx + 4] = bc*(1 - k) + wl*k
        bg[wy, wx] = bc; bg[wy, wx + 3] = bc
    gc = np.array(col(5), float); bg[GROUND] = gc; bg[TILES] = gc*.7
    path = GROUND & (np.abs(XX - MX - (YY - GY)*.25) < 4 + (YY - GY)*.12) & (BAY < .35 + .3*loud)
    bg[path] = bg[path]*.55 + np.array(col(2))*.45
    im = Image.fromarray(np.clip(bg, 0, 255).astype(np.uint8)); d = ImageDraw.Draw(im)
    pc = tuple(int(v*.55) for v in bc); hi = mix(pc, col(2), .25)
    for px, sc, side in PALMS: palm(d, px, sc, side, T_, x["pulse"], pc, hi)
    top, bot = np.array(col(0), float), np.array(col(1), float)
    banners(d, T_, eq, x["pulse"], [tuple(int(v) for v in mix(a, b, kb)) for a, b in zip(A_[6], B_[6])],
            lambda y: tuple(int(v) for v in top + (bot - top)*clamp(y/GY)))
    bg = np.asarray(im).astype(float)
    bg[TEXTB] *= .55; bg[TEXTE] *= .75
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])
