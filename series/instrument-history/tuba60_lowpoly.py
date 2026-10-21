"""Pixel Pitch: history of the tuba, 60-second cut.  python3 tuba60_lowpoly.py [out.mp4] | --still t ..."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_shapes as S
import trumpet_lowpoly as T
import piano_lowpoly as P
import cello_lowpoly as C
import drums60_lowpoly as D
from pp_framework import mix, dim, lerp, clamp, midi_hz, seq, SR, RW, RH, CX, YY, XX, BAY, HOT

OUT, TAG = "tuba60_lowpoly.mp4", "tu"
BPM = 120
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 1.45, -10, 10, .3
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE TUBA", "435 YEARS", "TODAY", 1500
INTRO_LINES = (("18 FEET OF TUBING,", "THE LOWEST BRASS"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("435 YEARS FROM", "A WOODEN SERPENT")
OUTRO_CREDIT = "BRASS HOUSE"

# ---------------------------------------------------------------- music (brass house: four on the floor, claps, offbeat hats, piano stabs)
CH = {"Fm": (41, (56, 60, 65)), "Db": (37, (56, 61, 65)), "Eb": (39, (55, 58, 63)), "C": (36, (55, 60, 64)), "Dm": (38, (57, 62, 65)),
      "Bb": (46, (58, 62, 65)), "F7": (41, (57, 60, 63)), "Ab": (44, (56, 60, 63)), "F": (41, (57, 60, 65)), "C7": (36, (55, 58, 64)),
      "Bb7": (46, (56, 62, 65)), "G7": (43, (55, 59, 65)), "Bbm": (46, (58, 61, 65)), "Cm": (36, (55, 60, 63))}
BARS = ["Fm", "Db", "Eb", "C",
        "Dm", ("C", "C", "Dm", "Dm"), "Dm", ("C", "C", "Dm", "Dm"), "Bb", ("F7", "F7", "Bb", "Bb"), "Eb", ("Ab", "Ab", "Eb", "Bb"),
        "F", ("C7", "C7", "F", "F"), "Eb", ("Bb7", "Bb7", "Bb7", "Eb"), "C", ("G7", "C", "G7", "C"), "Bb", ("G7", "C7", "Bb", "F7"),
        "Fm", ("Db", "Db", "Bbm", "Fm"), "Bb", ("Cm", "F7", "Bb", "Bb"), "F7", "F7",
        "Fm", "Db", "Fm", "Fm"]
def tn(items, v=.85, b0=0): return [(b0 + b, m, d, v) for b, m, d in items]
RIFF = [(0, 41, .5), (.75, 53, .25), (1, 41, .5), (1.5, 51, .5), (2, 48, .5), (2.75, 44, .25), (3, 46, .5), (3.5, 48, .5),
        (4, 37, .5), (4.75, 49, .25), (5, 37, .5), (5.5, 48, .5), (6, 44, .5), (6.75, 49, .25), (7, 51, .5), (7.5, 53, .5)]
RIFF2 = [(8, 39, .5), (8.75, 51, .25), (9, 39, .5), (9.5, 46, .5), (10, 43, .5), (10.75, 46, .25), (11, 51, .5), (11.5, 50, .5),
         (12, 36, .5), (12.75, 48, .25), (13, 36, .5), (13.5, 43, .5), (14, 46, .5), (14.5, 47, .5), (15, 48, 1)]

SCENES = [
 dict(key="modern", theme="intro", name="MODERN CC TUBA", credit="BRASS HOUSE", notes=tn(RIFF) + tn(RIFF2)),
 dict(key="serpent", theme="serpent", year=1590, ys="1590", name="SERPENT", credit="FRENCH PLAINCHANT", size=1.3,
      fact=("WOOD AND LEATHER", "FOR CHURCH CHOIRS"),
      notes=seq(0, [(50, 1), (53, 1), (55, .5), (57, .5), (55, 1), (53, .5), (52, .5), (50, 1), (48, 1), (50, 1)], .85)),
 dict(key="ophicleide", theme="ophicleide", year=1817, ys="1817", name="OPHICLEIDE", credit="IN THE STYLE OF BERLIOZ", size=.95,
      fact=("A 'SERPENT WITH", "KEYS' MADE OF BRASS"),
      notes=seq(0, [(53, 1), (52, 1), (53, 1), (50, 1), (52, 1), (48, 1), (50, 2)], .9)),
 dict(key="basstuba", theme="basstuba", year=1835, ys="1835", name="BASS TUBA IN F", credit="PRUSSIAN MILITARY MARCH",
      fact=("PATENTED IN PRUSSIA", "WITH FIVE VALVES"),
      notes=seq(0, [(46, .75), (46, .25), (50, .5), (53, .5), (58, 1.5), (57, .5), (55, .5), (53, .5), (51, .5), (50, .5), (48, 1), (46, 1)], .85)),
 dict(key="euphonium", theme="euphonium", year=1843, ys="1843", name="EUPHONIUM", credit="BRASS BAND HYMN", size=1.15,
      fact=("'SWEET-VOICED': THE", "TUBA'S TENOR SIBLING"),
      notes=seq(0, [(55, 1), (58, 1), (63, 1.5), (62, .5), (60, 1), (58, 1), (60, 1), (58, 1)], .85)),
 dict(key="helicon", theme="helicon", year=1848, ys="1848", name="HELICON", credit="CAVALRY BAND GALOP", size=.9,
      fact=("WORN AROUND THE BODY", "BY BANDS ON HORSEBACK"),
      notes=seq(0, [(41, .5), (48, .5), (45, .5), (48, .5), (41, .5), (48, .5), (45, .5), (48, .5), (43, .5), (48, .5), (46, .5), (48, .5), (41, 1), (36, 1)], .88)),
 dict(key="sousa", theme="sousa", year=1893, ys="1893", name="SOUSAPHONE", credit="IN THE STYLE OF SOUSA", size=.72,
      fact=("BUILT FOR SOUSA. ITS", "BELL CATCHES RAIN"),
      notes=seq(0, [(39, 1), (46, 1), (39, 1), (46, 1), (41, .5), (43, .5), (44, .5), (46, .5), (48, .5), (50, .5), (51, 1)], .88)),
 dict(key="wonder", theme="wonder", year=1908, ys="1908", name="BELL-FRONT SOUSAPHONE", credit="RAGTIME BASS", size=.78,
      fact=("CONN TURNS THE", "BELL TO FACE FORWARD"),
      notes=seq(0, [(36, 1), (43, 1), (41, 1), (43, 1), (38, .5), (39, .5), (40, 1), (43, 1), (36, 1)], .88)),
 dict(key="recording", theme="recording", year=1923, ys="1920s", name="RECORDING TUBA", credit="1920S HOT JAZZ", size=.95,
      fact=("A FORWARD BELL FOR", "EARLY RECORDINGS"),
      notes=seq(0, [(46, 1), (41, 1), (46, 1), (41, 1), (43, 1), (48, .5), (47, .5), (46, 1), (41, 1)], .88)),
 dict(key="concerto", theme="concerto", year=1954, ys="1954", name="F TUBA", credit="IN THE STYLE OF VAUGHAN WILLIAMS",
      fact=("VAUGHAN WILLIAMS", "WRITES IT A CONCERTO"),
      notes=seq(0, [(53, 1), (56, .5), (58, .5), (60, 1.5), (63, .5), (61, 1), (60, .5), (58, .5), (56, 1), (53, 1)], .85)),
 dict(key="fiberglass", theme="fiberglass", year=1961, ys="1961", name="FIBERGLASS SOUSAPHONE", credit="MARCHING BAND FIGHT SONG", size=.78,
      fact=("FIBERGLASS CUTS IT", "TO 18 POUNDS"),
      notes=seq(0, [(46, .5), (50, .5), (53, .5), (58, .5), (57, .5), (55, .5), (53, 1), (48, .5), (51, .5), (55, .5), (53, .5), (50, 1), (46, 1)], .88)),
 dict(key="brassband", theme="brassband", year=1977, ys="1970s", name="SOUSAPHONE", credit="NEW ORLEANS SECOND LINE", size=.78,
      fact=("THE BASS LINE OF", "NEW ORLEANS STREETS"),
      notes=tn([(0, 41, .5), (.75, 41, .25), (1, 44, .5), (1.5, 45, .5), (2, 48, .5), (2.75, 46, .75), (3.5, 45, .5), (4, 41, .5),
                (4.75, 41, .25), (5, 44, .25), (5.25, 45, .25), (5.5, 48, .5), (6, 51, .5), (6.5, 50, .5), (7, 48, .5), (7.5, 45, .5)], .9)),
 dict(key="modern", theme="intro", year=2020, ys="2020s", name="MODERN CC TUBA", credit="BRASS HOUSE",
      fact=("THE DEEPEST VOICE", "IN THE ORCHESTRA"),
      notes=tn(RIFF) + [(8, 41, 9.0, .95)]),
]
TRIM = {"wonder": -.6, "recording": -.4, "concerto": -.3, "ophicleide": .5, "serpent": .3, "basstuba": .2, "euphonium": -.1}
for _s in SCENES: _s["trim"] = TRIM.get(_s["key"], 0.)

# ---------------------------------------------------------------- instruments (modelled on reference photos, metres)
BRASS, NICKEL, PEARL = (226, 178, 86), (214, 216, 226), (244, 240, 228)
def unit(v): v = np.asarray(v, float); return v/np.linalg.norm(v)
def box_at(g, p, s, col, emit=False, spec=0.):
    g.box(p[0] - s, p[0] + s, p[1] - s, p[1] + s, p[2] - s, p[2] + s, col, emit=emit, spec=spec)
def path(g, pts, r0, col, r1=None, n=8, spec=.9):
    """tube swept along a polyline with a rotation-minimising frame, so neighbouring segments line up without seams"""
    P_ = np.array([np.asarray(p, float) for p in pts]); N = len(P_); r1 = r0 if r1 is None else r1
    if N < 2: return
    tg = np.gradient(P_, axis=0); tg /= np.linalg.norm(tg, axis=1)[:, None]
    a = np.array([0, 1., 0]) if abs(tg[0][1]) < .9 else np.array([1., 0, 0])
    u = np.cross(tg[0], a); u /= np.linalg.norm(u); rings = []
    for i in range(N):
        u = u - np.dot(u, tg[i])*tg[i]; u /= np.linalg.norm(u); v = np.cross(tg[i], u)
        r = lerp(r0, r1, i/(N - 1))
        rings.append([P_[i] + r*(math.cos(2*math.pi*k/n)*u + math.sin(2*math.pi*k/n)*v) for k in range(n)])
    for i in range(N - 1):
        ref = (P_[i] + P_[i + 1])/2; A, B = rings[i], rings[i + 1]
        for k in range(n): g.face([A[k], A[(k + 1) % n], B[(k + 1) % n], B[k]], col, ref=ref, spec=spec)
def arc(c, R, z, a0, a1, segs=16):
    return [(c[0] + R*math.cos(math.radians(a)), c[1] + R*math.sin(math.radians(a)), z) for a in np.linspace(a0, a1, segs + 1)]
def flare(g, p0, d, L, r0, R, col, x0=.15, m=1.0, segs=16, r_mid=None, rim_col=None):
    """bell along d from p0 (radius r0) to the rim (radius R) at p0 + d*L: a cone that blooms into a flare near the rim"""
    p0, d = np.asarray(p0, float), unit(d); r_mid = r_mid or r0*1.4
    def rad(s): return max(lerp(r_mid, r0, s/L), R*(x0/(s + x0))**m)
    ss = [L*(k/segs)**2 for k in range(segs + 1)][::-1]
    for s0, s1 in zip(ss, ss[1:]):
        T.tube(g, p0 + d*(L - s0), p0 + d*(L - s1), rad(s0), rad(s1), col=col, n=16 if s1 < .15 else 10, spec=.9, two=s1 < .3)
    rim = p0 + d*L
    T.tube(g, rim - d*.008, rim + d*.003, R*1.035, R*1.035, col=rim_col or col, n=16, two=True)
    g.face(S.ring(rim - d*.07, d, rad(.07)*.8, 14), (26, 16, 10), ref=rim - d, emit=True, two=True)
    return rim
def mouthpiece(g, p, d, r=.009, col=NICKEL):
    p, d = np.asarray(p, float), unit(d)
    T.tube(g, p, p + d*.06, r*1.1, r*.9, col=col); T.tube(g, p + d*.06, p + d*.085, r*.9, r*2.9, col=col, n=10)
    T.tube(g, p + d*.085, p + d*.09, r*3.0, col=col, n=10)
def fing_of(m, n):
    """valves pressed for note m (3-valve fingering, a 4th valve for the low register)"""
    f = T.fingering(m + 12) if m < 60 else T.fingering(m)
    on = {int(ch) - 1 for ch in f}
    if n >= 4 and m < 41: on = {3} | ({0} if m in (39, 40) else set())
    return on

def valve_block(g, kind, n, base, col, cap, on, step=(.045, -.03)):
    """n valves starting at base, each next one offset by step (x, y); returns the leadpipe joint (first valve)"""
    bx, by, bz = base
    for i in range(n):
        x, y = bx + step[0]*i, by + step[1]*i; down = i in on
        if kind == "front":                                          # front-action pistons: casings point at the viewer
            T.tube(g, (x, y, bz - .05), (x, y, bz + .045), .021, col=col, n=10)
            for z in (bz - .05, bz + .045): T.tube(g, (x, y, z - .006), (x, y, z + .006), .025, col=cap, n=10)
            tip = bz + .07 - (.014 if down else 0)
            T.tube(g, (x, y, bz + .05), (x, y, tip), .004, col=cap, n=6)
            T.tube(g, (x, y, tip), (x, y, tip + .01), .015, col=HOT if down else PEARL, n=10, spec=.4)
        elif kind in ("top", "berlin"):                              # top-action pistons
            r, h = (.02, .1) if kind == "top" else (.026, .075)
            T.tube(g, (x, y - h/2, bz), (x, y + h/2, bz), r, col=col, n=10)
            for yy in (y - h/2, y + h/2): T.tube(g, (x, yy - .006, bz), (x, yy + .006, bz), r*1.18, col=cap, n=10)
            top = y + h/2 + .04 - (.016 if down else 0)
            T.tube(g, (x, y + h/2, bz), (x, top, bz), .004, col=cap, n=6)
            T.tube(g, (x, top, bz), (x, top + .01, bz), .016, col=HOT if down else PEARL, n=10, spec=.4)
            for dx in (-.012, .012): T.tube(g, (x + dx, y - h/2, bz - .01), (x + dx, y - h/2 - .09 - .025*(i % 3), bz - .01), .007, col=col, n=6)
            T.bend(g, (x, y - h/2 - .09 - .025*(i % 3), bz - .01), .012, .007, 180, 360, col=col, segs=4)
        else:                                                        # rotary valves with lever paddles
            T.tube(g, (x, y, bz - .03), (x, y, bz + .03), .022, col=col, n=10)
            cp = np.array([x, y, bz + .031]); g.face(S.ring(cp, (0, 0, 1), .019, 10), HOT if down else dim(col, .82), ref=(x, y, bz), emit=down)
            pad = np.array([x + .065, y + .012 - (.012 if down else 0), bz + .045])
            g.line(cp, pad, cap); g.line(cp + [0, .003, 0], pad + [0, .003, 0], cap)
            g.box(pad[0] - .018, pad[0] + .018, pad[1] - .006, pad[1] + .006, pad[2] - .01, pad[2] + .01, HOT if down else PEARL, emit=down, spec=.5)
            for dy in (-.011, .011): T.tube(g, (x - .02, y + dy, bz - .02), (x - .02 - .09 - .02*(i % 3), y + dy, bz - .02), .007, col=col, n=6)
            T.bend(g, (x - .11 - .02*(i % 3), y, bz - .02), .011, .007, 90, 270, col=col, segs=4)
    return np.array([bx, by, bz])

TB = {   # upright tubas: rim height, bell radius, width bell-stack -> back column, valves, colours
 "modern":    dict(H=.96, R=.235, W=.29, vk="rotary", nv=5, col=(228, 176, 84), cap=NICKEL, vbase=(.08, .58, .07), vstep=(.0, -.055)),
 "concerto":  dict(H=.92, R=.215, W=.28, vk="front", nv=4, col=(232, 186, 92), cap=NICKEL, vbase=(.05, .6, .06), vstep=(.032, -.045)),
 "basstuba":  dict(H=.95, R=.15, W=.21, vk="berlin", nv=5, col=(206, 176, 110), cap=(214, 200, 160), vbase=(.03, .62, .05), vstep=(.012, -.07), narrow=True),
 "euphonium": dict(H=.64, R=.15, W=.2, vk="top", nv=4, col=(210, 214, 226), cap=(236, 236, 240), vbase=(.03, .43, .065), vstep=(.042, 0)),
 "recording": dict(H=.9, R=.215, W=.27, vk="top", nv=4, col=(208, 212, 222), cap=(236, 236, 240), vbase=(.03, .5, .075), vstep=(.042, 0), front=True),
}
def tuba(g, c, m):
    col, cap = c["col"], c["cap"]; H, R, W = c["H"], c["R"], c["W"]
    xb, yb = -.08, .2*H/.95 + .02; xr = xb + W; rb = .046*W/.25 + .012
    # bottom bow, back column, top return bow
    path(g, arc(((xb + xr)/2, yb), W/2, 0, 180, 360, 14), rb*.95, col, r1=rb*.8)
    yt = .62*H
    path(g, [(xr, yb, 0), (xr, yt, 0)], rb*.8, col, r1=rb*.68)
    ca = ((xb + xr)/2 + .02, yt); ra = xr - ca[0]                     # the arch over the top of the oval, into the bell stack
    path(g, arc(ca, ra, -.012, 0, 165, 12), rb*.66, col, r1=rb*.6)
    for k, (xa, xc, ylo) in enumerate(((xb + .075, xb + .115, .3*H), (xr - .1, xr - .055, .25*H))):   # nested valve-slide loops
        for x_ in (xa, xc): T.tube(g, (x_, .46*H, -.035), (x_, ylo, -.035), rb*.4, col=col, n=6)
        T.bend(g, ((xa + xc)/2, ylo, -.035), (xc - xa)/2, rb*.4, 180, 360, col=col, segs=5)
    for yy in (yb + .1, yb + .25, yt - .05): g.line((xb + rb, yy, 0), (xr - rb*.7, yy, 0), dim(col, .7))
    # bell
    if c.get("front"):
        yk = H - .17
        flare(g, (xb, yb, 0), (0, 1, 0), yk - yb, rb, rb*1.3, col, x0=.0001, m=1, segs=6, r_mid=rb*1.5)
        k0 = np.array([xb, yk, .16])
        bend = [k0 + .16*np.array([0, math.sin(a), -math.cos(a)]) for a in np.linspace(0, math.pi/2, 9)]
        path(g, bend, rb*1.5, col, r1=rb*1.9, n=12)
        rim = flare(g, bend[-1], (0, 0, 1), .32, rb*1.9, R, col, x0=.12, m=1.0, r_mid=rb*2.0)
    else:
        rim = flare(g, (xb, yb, 0), (0, 1, 0), H - yb, rb, R, col, x0=.15 if R > .2 else .09, m=1.0, r_mid=rb*1.6)
        T.tube(g, rim - np.array([0, .05, 0]), rim - np.array([0, .03, 0]), R*.62, R*.66, col=dim(col, 1.08), n=16, two=True)
    # valves + slides
    on = fing_of(m, c["nv"]); vb = np.array(c["vbase"], float)
    j = valve_block(g, c["vk"], c["nv"], vb, col, cap, on, c["vstep"])
    last = vb + np.array([c["vstep"][0], c["vstep"][1], 0])*(c["nv"] - 1)
    path(g, [last - [0, .05, .04], last - [0, .09, .06], (xr - .03, last[1] - .1, 0), (xr, last[1] - .08, 0)], rb*.55, col)
    # tuning slides (big U loops on the right and top)
    for k, (yy, L) in enumerate(((yt - .14, .1), (yt - .26, .13))):
        for dy in (-.018, .018): T.tube(g, (xr, yy + dy, -.01), (xr + L, yy + dy, -.01), rb*.42, col=col, n=6)
        T.bend(g, (xr + L, yy, -.01), .018, rb*.42, -90, 90, col=col, segs=5)
    # leadpipe + mouthpiece, sticking out to the left of the bell stack toward the player
    mp = np.array([xb - .2, .66*H, .12])
    path(g, [j + [0, .02, .03], j + [-.06, .06, .06], (xb - .02, .64*H, .11), (xb - .12, .66*H, .12), mp], rb*.38, col, r1=rb*.22)
    mouthpiece(g, mp, (-.7, .25, .2), r=.0085)
    # stand on the pedestal
    return rim

def serpent(g, m):
    """S-shaped wooden bass covered in black leather: three curves and a big bottom loop, six finger holes, brass crook"""
    lea, brass = (104, 70, 50), (214, 170, 86)
    pts = []
    for u in np.linspace(0, 1, 30):                                  # the upper S: two wiggles going down
        y = .84 - .5*u; x = .12*math.sin(2*math.pi*u*1.0 + math.pi)*(.55 + .55*u)
        pts.append((x, y))
    cx, cy, Rl = .03, .2, .145                                        # the big bottom loop, ending in the open mouth
    a0 = math.degrees(math.atan2(pts[-1][1] - cy, pts[-1][0] - cx))
    for a in np.linspace(a0, -270 + 35, 26)[1:]:
        pts.append((cx + Rl*math.cos(math.radians(a)), cy + Rl*math.sin(math.radians(a))))
    N = len(pts) - 1
    path(g, [(*p_, 0) for p_ in pts], .013, lea, r1=.058, n=10, spec=.3)
    mouth = np.array([*pts[-1], 0]); d = unit(mouth - np.array([*pts[-2], 0]))
    g.face(S.ring(mouth + d*.001, d, .052, 12), (14, 10, 10), ref=mouth - d, emit=True)
    for idx in (9, 11, 13, 19, 21, 23):                               # two groups of three finger holes
        p = np.array([*pts[idx], 0]); rr = .013 + .045*(idx/N)
        g.face(S.ring(p + [0, 0, rr*.98], (0, 0, 1), .007, 8), (230, 214, 170), ref=p, two=True)
        g.face(S.ring(p + [0, 0, rr + .001], (0, 0, 1), .0045, 8), (16, 10, 10), ref=p, two=True)
    top = np.array([*pts[0], 0])
    crook = [top, top + [0, .05, 0], top + [.02, .09, 0], top + [.07, .1, 0], top + [.12, .09, 0], top + [.13, .06, 0]]
    path(g, crook, .007, brass, r1=.006)
    mouthpiece(g, crook[-1], (0, -1, .2), r=.006, col=(232, 220, 190))
    S.rod(g, (.03, 0, -.06), (.03, cy - Rl, -.02), .012); g.ngon(.03, .004, -.06, .12, (40, 38, 46), n=8)
    return mouth

def ophicleide(g, m):
    """tall upright brass: a wide conical tube up to the bell, a narrow tube beside it, U-bow at the bottom, crook to the side, big keys"""
    col = (216, 186, 112); xw, xn, y0 = 0., .085, .1
    rim = flare(g, (xw, y0, 0), (0, 1, 0), 1.0, .03, .115, col, x0=.06, m=.9, r_mid=.055)
    path(g, arc(((xw + xn)/2, y0), (xn - xw)/2, 0, 180, 360, 10), .028, col, r1=.02)
    path(g, [(xn, y0, 0), (xn, .66, 0)], .02, col, r1=.016)
    crook = [(xn, .66, 0), (xn, .72, 0), (xn + .02, .78, 0), (xn + .07, .8, 0), (xn + .12, .76, 0), (xn + .14, .7, 0), (xn + .2, .68, 0), (.36, .69, 0)]
    path(g, crook, .012, col, r1=.008)
    mouthpiece(g, crook[-1], (1, .1, 0), r=.008)
    for y in (.22, .45, .62): g.line((xw + .03, y, 0), (xn - .015, y, 0), dim(col, .75))
    cur_on = {(m*7 + k) % 11 for k in range(3)} if m < 52 else set()
    for k in range(11):                                              # key cups along the wide tube, with long levers
        y = .2 + .065*k; r = lerp(.032, .05, k/10); side = -1 if k % 2 else 1
        p = np.array([xw + side*r*.25, y, r*.97]); op = k in cur_on
        nrm = unit((side*.25, .35 if op else 0, 1))
        T.tube(g, p, p + nrm*.012, .022, .022, col=HOT if op else (230, 204, 134), n=10, spec=.7)
        g.face(S.ring(p + nrm*.0125, nrm, .022, 10), HOT if op else (240, 220, 160), ref=p, emit=op)
        if k % 3 == 0:
            g.line(p + [0, 0, .012], (xn + .03, y - .05, .03), (180, 160, 110)); box_at(g, (xn + .03, y - .05, .03), .007, (236, 220, 170))
    S.rod(g, (.04, 0, -.06), (.04, y0 - .05, -.02), .014); g.ngon(.04, .004, -.06, .14, (40, 38, 46), n=8)
    return rim

def helicon(g, m, col=(214, 186, 96)):
    """a big circle of tubing that widens as it goes round, the bell rising off the top over the shoulder; rotary valves inside"""
    c, Rc = np.array([0, .46]), .3
    ang = np.linspace(40, -262, 22)
    pts = [(c[0] + Rc*math.cos(math.radians(a)), c[1] + Rc*math.sin(math.radians(a)), .03*math.sin(math.radians(a))) for a in ang]
    path(g, pts, .026, col, r1=.05, n=10)
    p0 = np.array(pts[-1]); rim = flare(g, p0, (.62, .78, .12), .3, .045, .15, col, x0=.08, m=.9, r_mid=.06)
    on = fing_of(m, 4)
    for i in range(4):
        y = .38 - .05*i; x = .16 - .02*i; down = i in on
        T.tube(g, (x, y, -.02), (x, y, .035), .02, col=col, n=10)
        g.face(S.ring((x, y, .036), (0, 0, 1), .017, 10), HOT if down else dim(col, .85), ref=(x, y, 0), emit=down)
        pad = np.array([x + .08, y + .015 - (.01 if down else 0), .05]); g.line((x, y, .036), pad, NICKEL)
        g.box(pad[0] - .014, pad[0] + .014, pad[1] - .005, pad[1] + .005, pad[2] - .008, pad[2] + .008, HOT if down else PEARL, emit=down)
        for dy in (-.01, .01): T.tube(g, (x - .02, y + dy, 0), (x - .12 - .02*i, y + dy, 0), .0065, col=col, n=6)
        T.bend(g, (x - .12 - .02*i, y, 0), .01, .0065, 90, 270, col=col, segs=4)
    path(g, [pts[0], (.2, .5, .01), (.18, .42, 0), (.16, .38, 0)], .018, col)
    mp = np.array([.36, .58, .04]); path(g, [(.16, .4, .01), (.26, .48, .03), mp], .011, col, r1=.008)
    mouthpiece(g, mp, (.3, .6, .2), r=.008)
    S.rod(g, (0, 0, -.06), (0, c[1] - Rc - .02, -.02), .014); g.ngon(0, .004, -.06, .14, (40, 38, 46), n=8)
    return rim

def sousaphone(g, m, col=(228, 182, 90), bellcol=None, up=False, valvecol=None):
    """wrap circle round the body, gooseneck up from the top, a huge bell (forward, or straight up on Sousa's 'rain-catcher')"""
    bellcol = bellcol or col; vcol = valvecol or col
    c, Rc = np.array([0, .5]), .3
    ang = np.linspace(48, -250, 22)
    pts = [(c[0] + Rc*math.cos(math.radians(a)), c[1] + Rc*math.sin(math.radians(a)), .035*math.sin(math.radians(a))) for a in ang]
    path(g, pts, .036, bellcol, r1=.062, n=10)
    p0 = np.array(pts[-1])
    neck = [p0, p0 + [.04, .05, 0], p0 + [.07, .13, 0], p0 + [.08, .24, 0]]
    path(g, neck, .06, bellcol, r1=.075, n=12)
    if up:
        rim = flare(g, neck[-1], (0, 1, 0), .36, .07, .27, bellcol, x0=.12, m=1.0, r_mid=.085)
    else:
        k0 = neck[-1] + [0, 0, .15]
        bend = [k0 + .15*np.array([0, math.sin(a), -math.cos(a)]) for a in np.linspace(0, math.pi/2, 9)]
        path(g, bend, .07, bellcol, r1=.085, n=12)
        rim = flare(g, bend[-1], (0, 0, 1), .38, .085, .32, bellcol, x0=.13, m=1.0, r_mid=.1)
    on = fing_of(m, 3)
    for i, x in enumerate((.08, .125, .17)):
        y = .3; down = i in on
        T.tube(g, (x, y - .05, .06), (x, y + .05, .06), .02, col=vcol, n=10)
        for yy in (y - .05, y + .05): T.tube(g, (x, yy - .006, .06), (x, yy + .006, .06), .024, col=NICKEL, n=10)
        top = y + .09 - (.016 if down else 0)
        T.tube(g, (x, y + .05, .06), (x, top, .06), .004, col=NICKEL, n=6)
        T.tube(g, (x, top, .06), (x, top + .01, .06), .016, col=HOT if down else PEARL, n=10, spec=.4)
        for dx in (-.012, .012): T.tube(g, (x + dx, y - .05, .045), (x + dx, y - .14 - .03*i, .045), .007, col=vcol, n=6)
        T.bend(g, (x, y - .14 - .03*i, .045), .012, .007, 180, 360, col=vcol, segs=4)
    path(g, [pts[0], (.24, .5, .04), (.2, .4, .05), (.17, .36, .06)], .022, vcol)
    mp = np.array([.26, .66, .1]); path(g, [(.08, .36, .06), (.14, .48, .08), (.22, .58, .09), mp], .011, vcol, r1=.008)
    mouthpiece(g, mp, (-.4, .5, .3), r=.008)
    S.rod(g, (0, 0, -.06), (0, c[1] - Rc - .02, -.02), .014); g.ngon(0, .004, -.06, .14, (40, 38, 46), n=8)
    return rim

def cur(s, t, ev):
    live = [e for e in ev if e[0] < e[3] + .03]
    return min(live)[1] if live else (min(ev)[1] if ev else s["ev"][0][1])

def build(g, t, ev, fx, s):
    k = s["key"]; m = cur(s, t, ev)
    if k == "serpent": rim = serpent(g, m); out = (0, -1, 0)
    elif k == "ophicleide": rim = ophicleide(g, m); out = (0, 1, 0)
    elif k == "helicon": rim = helicon(g, m); out = (.6, .8, 0)
    elif k == "sousa": rim = sousaphone(g, m, col=(222, 186, 104), up=True); out = (0, 1, 0)
    elif k == "wonder": rim = sousaphone(g, m, col=(226, 184, 96)); out = (0, 0, 1)
    elif k == "fiberglass": rim = sousaphone(g, m, col=(236, 234, 226), bellcol=(240, 238, 230), valvecol=(226, 184, 96)); out = (0, 0, 1)
    elif k == "brassband": rim = sousaphone(g, m, col=(212, 216, 228)); out = (0, 0, 1)
    else:
        c = TB[k]; rim = tuba(g, c, m); out = (0, 0, 1) if c.get("front") else (0, 1, 0)
    rim = np.asarray(rim, float)
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9: fx.append(("note", tuple(rim + [.3, -.12, 0]), age))
        if tp and age < .6: fx.append(("wave", tuple(rim + [.05, .02, 0]), age, 0))

# ---------------------------------------------------------------- sound
def _t(n): return np.arange(n)/SR
TONE = {"modern": dict(bright=.45, vib=.002, scoop=.02, tilt=1.0), "serpent": dict(bright=.2, vib=0, scoop=.05, rough=.3, tilt=1.15),
        "ophicleide": dict(bright=.55, vib=0, scoop=.03, rough=.12, tilt=.9), "basstuba": dict(bright=.5, vib=0, scoop=.025, tilt=1.0),
        "euphonium": dict(bright=.3, vib=.006, scoop=.02, tilt=1.15), "helicon": dict(bright=.6, vib=0, scoop=.03, tilt=.95),
        "sousa": dict(bright=.7, vib=0, scoop=.03, tilt=.9), "wonder": dict(bright=.6, vib=0, scoop=.03, tilt=.95),
        "recording": dict(bright=.65, vib=.004, scoop=.045, tilt=.9), "concerto": dict(bright=.38, vib=.005, scoop=.02, tilt=1.05),
        "fiberglass": dict(bright=.85, vib=0, scoop=.03, tilt=.9), "brassband": dict(bright=.85, vib=0, scoop=.05, tilt=.85)}
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]; key = s["key"]
    x = T.brass(midi_hz(m), dur, v, **TONE[key])
    t = _t(len(x)); body = np.sin(2*np.pi*midi_hz(m)*t)*np.minimum(1, t/.03)*np.where(t < dur, 1, np.exp(-(t - dur)/.03))
    return x + .35*body*v                                            # a little extra fundamental: the tuba's round low end

# ---------------------------------------------------------------- backing: brass house (kick on every beat, claps, offbeat open hats, piano stabs)
def _clap(v):
    a, b = C.clap(1.)*.7, P.snare(1.)*.35; n = max(len(a), len(b)); return (np.pad(a, (0, n - len(a))) + np.pad(b, (0, n - len(b))))*v
KIT = {"kick": lambda v: D.kick_s(52, 125, 8, .12, .42, 1.5, .25)*v, "clap": _clap, "hatO": lambda v: P.hat(v, True)*.8,
       "hat": lambda v: P.hat(v)*.7, "crash": P.crash}
KICK_KINDS, ACCENT_KINDS = ("kick",), ("clap",)
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT; nb = len(BARS)
    for b in range(nb):
        T0 = b*B
        if b == nb - 1: ev += [(T0, "kick", 1.), (T0, "crash", 1.)]; continue
        ev += [(T0 + k*Q, "kick", 1. if k == 0 else .9) for k in range(4)]
        ev += [(T0 + Q, "clap", .9), (T0 + 3*Q, "clap", .9)]
        ev += [(T0 + (k + .5)*Q, "hatO", .7) for k in range(4)]
        ev += [(T0 + k*.25*Q, "hat", .45 if k % 2 else .25) for k in range(16) if k % 4 != 2]
        if b in (0, 4, 16, 26): ev.append((T0, "crash", .7))
    return ev
def bass_events(c): return []
def bass_voice(m, dur, v): return np.zeros(int(dur*SR))
def stab(notes, v, dur=.22):
    n = int((dur + .1)*SR); out = np.zeros(n)
    for mm in list(notes) + [notes[-1] + 12]:
        x = P.additive(midi_hz(mm), dur, .7*v, harm=10, tilt=1.15, d1=.9); out[:len(x)] += x[:n]
    t = _t(n); return out*np.minimum(1, (dur + .08 - t)/.06).clip(0, 1)
def pad_voice(notes, dur, v):
    n = int((dur + .5)*SR); out = np.zeros(n)
    if dur > 2.2:
        for mm in notes: x = P.additive(midi_hz(mm), 2.4, .5*v, harm=10, tilt=1.2, d1=1.5); out[:len(x)] += x[:n]
        return out
    offs = [.5, 1.5, 2.5, 3.25] if abs(dur - (BAR_S + .05)) < .1 else [.5]
    for o in offs:
        x = stab(notes, v); a = int(o*BEAT_S*SR); e = min(n, a + len(x)); out[a:e] += x[:e - a]
    return out
MIX = dict(drums=-20, bass=-40, pad=-26, verb=-24, verb_len=1.1)

# ---------------------------------------------------------------- background: a football stadium at night - floodlights, crowd in the stands, a striped field
THEMES = {   # sky top, sky bottom, stands, crowd a, crowd b, floodlight, field
 "intro":      ((6, 8, 26), (40, 40, 96), (60, 56, 80), (255, 200, 90), (230, 90, 70), (255, 236, 170), (40, 110, 60)),
 "serpent":    ((12, 6, 22), (70, 40, 90), (60, 44, 70), (200, 170, 230), (150, 110, 190), (230, 210, 255), (44, 70, 60)),
 "ophicleide": ((24, 4, 10), (120, 40, 50), (80, 40, 44), (255, 190, 140), (200, 70, 70), (255, 220, 190), (60, 90, 50)),
 "basstuba":   ((4, 10, 30), (40, 70, 130), (50, 60, 90), (220, 230, 255), (90, 130, 210), (230, 240, 255), (40, 90, 70)),
 "euphonium":  ((4, 20, 24), (30, 110, 120), (40, 70, 76), (180, 255, 230), (60, 190, 170), (220, 255, 240), (40, 110, 80)),
 "helicon":    ((8, 18, 8), (60, 100, 50), (56, 70, 44), (240, 230, 150), (150, 180, 80), (250, 250, 200), (50, 110, 40)),
 "sousa":      ((26, 14, 6), (150, 90, 50), (90, 60, 40), (255, 210, 150), (200, 120, 70), (255, 230, 180), (90, 100, 50)),
 "wonder":     ((30, 20, 4), (170, 120, 40), (100, 76, 40), (255, 230, 140), (220, 150, 60), (255, 240, 170), (100, 110, 46)),
 "recording":  ((14, 12, 12), (90, 84, 76), (70, 66, 60), (230, 220, 200), (160, 150, 140), (250, 244, 230), (70, 80, 64)),
 "concerto":   ((20, 8, 20), (140, 70, 100), (80, 50, 70), (255, 200, 220), (200, 110, 150), (255, 230, 240), (60, 100, 70)),
 "fiberglass": ((6, 22, 30), (230, 120, 60), (60, 90, 100), (255, 170, 80), (80, 200, 210), (255, 240, 200), (60, 130, 70)),
 "brassband":  ((16, 6, 30), (110, 40, 130), (60, 40, 80), (255, 210, 60), (90, 220, 110), (255, 240, 180), (50, 110, 60)),
}
def ped_cols(th): T_ = THEMES[th]; return F.ped_cols_from(mix(T_[6], T_[2], .45))
SKYB, GY = 196, 300
SKYG = {k: F.make_grad(v[0], v[1], y0=0, y1=SKYB) for k, v in THEMES.items()}
_r = np.random.default_rng(5)
STARS = (YY < 150) & (_r.random((RH, RW)) < .004)
STANDS = (YY >= SKYB) & (YY < GY)
ROWS = STANDS & (((YY - SKYB) % 6) == 0)
ROOF = (YY >= SKYB - 4) & (YY < SKYB)
HEADS = []
for ry in range(SKYB + 2, GY - 3, 6):
    for x in range(2, RW - 2, 3):
        if _r.random() < .78: HEADS.append((x + int(_r.integers(-1, 2)), ry, int(_r.integers(0, 3)), (ry//6) % 2))
LIT = _r.random(len(HEADS)) < .07
HX = np.array([h[0] for h in HEADS]); HY = np.array([h[1] for h in HEADS]); HC = np.array([h[2] for h in HEADS]); HP = np.array([h[3] for h in HEADS])
FIELD = YY >= GY
_dep = np.log1p(np.maximum(YY - GY, 0)/5.)/np.log(1.32)
STRIPE = FIELD & ((np.floor(_dep) % 2) == 0)
_xb = CX + (XX - CX)*200/np.maximum(YY - 270, 1)
YARD = FIELD & (np.abs(((_xb - CX + 20) % 44) - 22) > 20.6)
TOWERS = [(6, 30), (240, 264)]
LAMPS = np.zeros((RH, RW), bool); TOWER = np.zeros((RH, RW), bool)
for x0, x1 in TOWERS:
    TOWER |= (YY >= 128) & (YY < GY) & (np.abs(XX - (x0 + x1)//2) <= 1)
    TOWER |= (YY >= 112) & (YY < 130) & (XX >= x0) & (XX < x1)
    for r in range(3):
        for k in range(4):
            LAMPS |= (YY >= 114 + r*5) & (YY < 117 + r*5) & (XX >= x0 + 2 + k*6) & (XX < x0 + 6 + k*6)
BEAMS = np.zeros((RH, RW), bool)
for x0, x1 in TOWERS:
    xc = (x0 + x1)/2; tgt = CX
    u = (YY - 124)/(GY + 30 - 124)
    xm = xc + (tgt - xc)*u; w = 4 + 40*u
    BEAMS |= (YY > 124) & (YY < GY + 30) & (np.abs(XX - xm) < w) & (BAY < .12)
TEXTB = ((YY >= 40) & (YY < 100)) | ((YY >= 330) & (YY < 380))
TEXTE = (((YY >= 36) & (YY < 40)) | ((YY >= 100) & (YY < 104)) | ((YY >= 326) & (YY < 330)) | ((YY >= 380) & (YY < 384))) & (BAY < .5)

def background(c, x):
    th0, th1, kb, T_ = x["th0"], x["th1"], x["kb"], x["T"]
    A_, B_ = THEMES[th0], THEMES[th1]
    def col(i): return np.array(mix(A_[i], B_[i], kb), float)
    bg = SKYG[th0]*(1 - kb) + SKYG[th1]*kb
    bg[STARS & (np.sin(T_*3 + XX*.7) > -.2)] = (230, 230, 255)
    st = col(2); bg[STANDS] = st*.75; bg[ROWS] = st*1.15; bg[ROOF] = st*1.3
    ca, cb, lt = col(3), col(4), col(5)
    bob = (x["beat_ph"] < .3)
    hy = HY - (bob & (HP == (x["beat_i"] % 2))).astype(int)
    sil = st*.5
    for dy in (0, 1): bg[hy + dy, HX] = sil; bg[hy + dy, HX + 1] = sil*.9
    lit = LIT & ((x["beat_i"] + HC) % 3 == 0)
    bg[hy[lit], HX[lit]] = np.where((HC[lit] == 0)[:, None], ca, cb)
    fl = col(6); bg[FIELD] = fl*.85; bg[STRIPE] = fl; bg[YARD] = fl*.55 + np.array((236, 236, 226))*.45
    bg[TOWER] = (20, 18, 26)
    pulse = x["pulse"]; glow = .55 + .45*pulse
    bg[LAMPS] = lt*glow
    bg[BEAMS] = bg[BEAMS]*.55 + lt*.45*(.4 + .6*x["loud"])
    bg[TEXTB] *= .55; bg[TEXTE] *= .75
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])
