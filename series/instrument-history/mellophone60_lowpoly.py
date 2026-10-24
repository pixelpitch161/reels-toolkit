"""Pixel Pitch: history of the mellophone, 61-second loop.  python3 mellophone60_lowpoly.py [out.mp4] | --still t ...

Modern Yamaha YMP-204M first, then 1845 -> 2000, back to TODAY. Backdrop: inside a giant brass clockwork
(gears tick one tooth per beat, background pistons pump on the notes, steam vents fire on the rimshots).
Beat: dark drumline at 111 BPM (snare-line cadence with rimshots, tenor runs, split basses, halftime 808, minor pad).
Runtime 61.333 s (1840 frames; 48 kHz audio fills whole AAC frames so the loop has no padding gap).
The file loops: the final chord runs up (snare roll) into the downbeat at frame 0, and the picture dissolves back to frame 0.
Models built from reference photos: Yamaha YMP-204MS product photo; Met 504629 alto saxhorn + Musee de la musique Sax alto;
MIMEd 15365 Courtois Koenig horn; Met 503845 over-the-shoulder alto + Met 503839 Seltmann OTS; MIMEd 102850 Besson tenor horn;
NMM 12013 Conn 4-E text + 1911 Conn concert mellophone; Conn 16E mellophonium; Olds Ultratone piston-rotor bugle;
Reynolds M13 marching mellophone; King K-50 two-valve G bugle; King 1121 F marching mellophone.
"""
import math, sys
import numpy as np
from scipy.signal import lfilter, butter
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_shapes as S
import trumpet_lowpoly as T
import piano_lowpoly as P
from pp_framework import mix, dim, lerp, clamp, ease, midi_hz, SR, RW, RH, CX, YY, XX, BAY, HOT

OUT, TAG = "mellophone60_lowpoly.mp4", "mp"
BPM = 111
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
N_ROWS = 11                                                    # 10 history rows + the closing modern row
LOOP_FRAMES = 1840                                             # the 61 s rule (60.5-61.5 s): 61.333 s incl. the final chord;
LOOP_LEN = LOOP_FRAMES/30                                      # 1840 frames at 48 kHz = exactly 2875 AAC frames, so the encoder
AUDIO_RATE = 48000                                             # adds no silent padding at the loop point (61.000 s would)
FINAL_LEN = LOOP_LEN - (SHOW_BARS + SCENE_BARS*N_ROWS + OUTRO_BARS)*BAR_S + 1e-6
END_FADE = .004                                                # declick only: the loop restarts at the file end
LOOP_END = dict(spin=.4, gap=.1, dissolve=.42)                 # outro horn spins away, picture dissolves into frame 0
LIGHTING = True
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 2.05, -14, 10, .3
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE MELLOPHONE", "181 YEARS", "TODAY", 1840
INTRO_LINES = (("HALF THE TUBING", "OF A FRENCH HORN"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("181 YEARS FROM", "SAX'S ALTO HORN")
OUTRO_CREDIT = "DARK DRUMLINE"
T3 = 1/3

# ---------------------------------------------------------------- harmony (C minor, dark)
CH = {"Cm": (36, (55, 60, 63)), "Ab": (44, (56, 60, 63)), "Fm": (41, (56, 60, 65)), "G7": (43, (56, 59, 65)), "G": (43, (55, 59, 62)),
      "Db": (37, (56, 61, 65)), "C7": (36, (55, 58, 64)), "Bbm": (46, (58, 61, 65)), "Fm7": (41, (56, 60, 63)),
      "Db7": (37, (56, 59, 65)), "C7#9": (36, (52, 58, 63))}
BARS = ["Cm", "Ab", "Fm", "G7",
        ("Cm", "Cm", "Fm", "Fm"), ("G7", "G7", "Cm", "Cm"),            # 1845 polka
        "Fm", ("Db", "Db", "C7", "C7"),                                # 1855 waltz
        "Fm", ("C7", "C7", "C7", "Fm"),                                # 1860s quickstep
        ("Cm", "Fm", "Cm", "Cm"), ("Ab", "Ab", "G", "G"),              # 1880s hymn
        "Fm", ("Bbm", "Bbm", "C7", "C7"),                              # 1925 dance band
        "Cm", ("Db", "Db", "G7", "G7"),                                # 1960 Kenton
        "Cm", ("Ab", "Ab", "G", "G"),                                  # 1968 fanfare
        "Fm", ("Db", "Db", "C7", "C7"),                                # 1969 marching band
        "Fm7", ("Db7", "Db7", "C7#9", "C7#9"),                         # 1977 jazz-rock
        "Ab", ("Fm", "Fm", "G", "G"),                                  # 2000 opener
        "Cm", "Ab",                                                    # 2020s modern
        ("Fm", "Fm", "G7", "G7"),                                      # outro
        "Cm"]                                                          # final chord (runs up into the loop)
def tn(items, v=.85, b0=0): return [(b0 + b, m, d, v) for b, m, d in items]
RIFF1 = [(0, 60, .5), (.75, 60, .25), (1, 63, .25), (1.25, 62, .25), (1.5, 60, .5), (2.5, 67, .5), (3, 68, 1.),
         (4, 67, .5), (4.75, 65, .25), (5, 63, .5), (5.5, 65, .5), (6, 67, 1.5), (7.5, 63, .5)]
RIFF2 = [(8, 65, .5), (8.75, 65, .25), (9, 68, .25), (9.25, 67, .25), (9.5, 65, .5), (10.5, 72, .5), (11, 73, 1.),
         (12, 71, .75), (12.75, 72, .25), (13, 71, .5), (13.5, 68, .5), (14, 67, 1.), (15, 65, .5), (15.5, 59, .5)]
OUTRO = [(8, 65, .5), (8.75, 65, .25), (9, 68, .5), (9.5, 67, .5), (10, 71, 1.), (11, 67, .5), (11.5, 71, .5)]
FINAL_BEATS = FINAL_LEN/BEAT_S

SCENES = [
 dict(key="modern", theme="intro", name="YAMAHA YMP-204M", credit="DARK DRUMLINE", notes=tn(RIFF1) + tn(RIFF2)),
 dict(key="saxhorn", theme="saxhorn", year=1845, ys="1845", name="ALTO SAXHORN", credit="PARISIAN POLKA", size=.8,
      fact=("SAX PATENTS 7", "SAXHORN SIZES"),
      notes=tn([(0, 72, .25), (.5, 71, .25), (1, 72, .5), (1.5, 67, .5), (2, 68, .25), (2.5, 67, .25), (3, 65, .5), (3.5, 63, .5),
                (4, 62, .25), (4.5, 63, .25), (5, 65, .5), (5.5, 67, .5), (6, 71, .25), (6.5, 74, .25), (7, 72, .75)], .85)),
 dict(key="koenig", theme="koenig", year=1855, ys="1855", name="KOENIG HORN", credit="SECOND EMPIRE WALTZ", size=.92,
      fact=("1855 PARIS EXPO:", "FIRST-CLASS MEDAL"),
      notes=tn([(0, 68, 1.), (1, 72, 1.), (2, 77, 1.), (3, 77, 1.), (4, 73, .5), (4.5, 72, .5), (5, 70, 1.), (6, 67, 1.), (7, 64, 1.)], .82)),
 dict(key="ots", theme="ots", year=1862, ys="1860s", name="OVER-THE-SHOULDER HORN", credit="CIVIL WAR QUICKSTEP", size=.86,
      fact=("BELL POINTS BACK", "FOR TROOPS BEHIND"),
      notes=tn([(0, 60, .75), (.75, 60, .25), (1, 65, .75), (1.75, 65, .25), (2, 68, .75), (2.75, 67, .25), (3, 65, .5), (3.5, 67, .5),
                (4, 68, .75), (4.75, 70, .25), (5, 72, .5), (5.5, 67, .5), (6, 64, .75), (6.75, 67, .25), (7, 65, 1.)], .88)),
 dict(key="tenor", theme="tenor", year=1885, ys="1880s", name="E-FLAT TENOR HORN", credit="BRASS BAND HYMN", size=.8,
      fact=("IN BRITAIN IT'S", "THE 'TENOR HORN'"),
      notes=tn([(0, 67, 1.), (1, 68, 1.), (2, 67, 1.), (3, 65, .5), (3.5, 63, .5), (4, 65, 1.), (5, 63, 1.), (6, 62, 2.)], .8)),
 dict(key="concert", theme="concert", year=1925, ys="1925", name="CONCERT MELLOPHONE", credit="1920S DANCE BAND", size=.92,
      fact=("4 SWAP SLIDES:", "F, E-FLAT, D OR C"),
      notes=tn([(0, 72, 1.25), (1.5, 72, .5), (2, 70, .25), (2.25, 72, .25), (2.5, 68, 1.), (3.5, 65, .5),
                (4, 70, .5), (4.5, 73, .5), (5.5, 73, .5), (6, 72, .5), (6.5, 70, .5), (7, 67, .5), (7.5, 64, .5)], .85)),
 dict(key="mellophonium", theme="mellophonium", year=1960, ys="1960", name="CONN MELLOPHONIUM", credit="KENTON BIG BAND", size=.95,
      fact=("KENTON'S SECTION", "WINS 2 GRAMMYS"),
      notes=tn([(0, (60, 67), .25), (.5, (63, 67, 72), 1.25), (2, (67, 75), .5), (2.5, 74, .25), (2.75, 72, .25), (3, (68, 72, 77), 1.),
                (4, (68, 73, 77), .5), (4.5, 73, .5), (5, (65, 68, 72), 1.), (6, 71, .5), (6.5, 68, .5), (7, (62, 65, 67), 1.)], .9)),
 dict(key="bugle68", theme="bugle68", year=1968, ys="1968", name="PISTON-ROTOR BUGLE", credit="SCOUT CORPS FANFARE", size=.95,
      fact=("KANSTUL'S G-F", "PISTON-ROTOR BUGLE"),
      notes=tn([(0, 67, .5), (.5, 72, .25), (.75, 72, .25), (1, 75, .5), (1.5, 72, 1.), (2.5, 67, .25), (2.75, 72, .25), (3, 75, .5), (3.5, 72, .5),
                (4, 75, .75), (4.75, 72, .25), (5, 68, .5), (5.5, 72, .5), (6, 74, T3), (6 + T3, 72, T3), (6 + 2*T3, 71, T3), (7, 67, 1.)], .9)),
 dict(key="march69", theme="march69", year=1969, ys="1969", name="MARCHING MELLOPHONE", credit="60S MARCHING BAND", size=.95,
      fact=("FIRST BELL-FRONT", "MARCHING BRASS"),
      notes=tn([(0, 72, .5), (1, 68, .5), (2, 65, .5), (2.5, 67, .25), (2.75, 68, .25), (3, 70, .5), (3.5, 72, .5),
                (4, 73, .5), (5, 72, .5), (6, 70, .5), (6.5, 67, .5), (7, 64, .5), (7.5, 67, .5)], .9)),
 dict(key="bugle77", theme="bugle77", year=1977, ys="1977", name="TWO-VALVE G BUGLE", credit="70S DRUM CORPS JAZZ-ROCK", size=.95,
      fact=("DRUM CORPS SWITCH", "TO TWO VALVES"),
      notes=tn([(0, 65, .25), (.25, 68, .25), (.5, 70, .25), (.75, 72, .5), (1.5, 72, .25), (1.75, 75, .5), (2.5, 72, .25), (2.75, 70, .25),
                (3, 68, .5), (3.5, 65, .25), (3.75, 63, .25), (4, 65, .5), (4.5, 68, .25), (4.75, 71, .5), (5.5, 68, .5),
                (6, 76, .5), (6.5, 75, .5), (7, 72, .25), (7.25, 70, .25), (7.5, 67, .5)], .9)),
 dict(key="f2000", theme="f2000", year=2000, ys="2000", name="F MELLOPHONE", credit="DRUM CORPS OPENER", size=.95,
      fact=("DCI DROPS ITS", "G-ONLY RULE"),
      notes=tn([(0, (63, 68, 72), 1.5), (2, 70, .25), (2.25, 72, .25), (2.5, 75, .25), (2.75, 77, .25), (3, 75, 1.),
                (4, (65, 68, 72), .75), (4.75, 72, .25), (5, 73, .5), (5.5, 72, .5), (6, (67, 71, 74), 2.)], .92)),
 dict(key="modern", theme="intro", year=2020, ys="2020s", name="YAMAHA YMP-204M", credit="DARK DRUMLINE",
      fact=("SAME FINGERINGS", "AS A TRUMPET"),
      notes=tn(RIFF1) + tn(OUTRO) + [(12, (60, 67), FINAL_BEATS + .5, .95)]),
]
TRIM = {"tenor": .4, "koenig": .3, "concert": .1, "mellophonium": -.4, "f2000": -.3, "bugle77": -.3}
for _s in SCENES: _s["trim"] = TRIM.get(_s["key"], 0.)

# ---------------------------------------------------------------- geometry helpers (metres-ish, y up, bell-front horns point +x)
LACQ, SILVER, CHROME, NICKEL, PEARL = (232, 184, 88), (206, 210, 220), (226, 230, 238), (214, 216, 226), (244, 240, 228)
DARKBASE = (40, 38, 46)
def unit(v): v = np.asarray(v, float); return v/np.linalg.norm(v)
def box_at(g, p, s, col, emit=False, spec=0.):
    g.box(p[0] - s, p[0] + s, p[1] - s, p[1] + s, p[2] - s, p[2] + s, col, emit=emit, spec=spec)
def path(g, pts, r0, col, r1=None, n=8, spec=.9):
    """tube swept along a polyline with a rotation-minimising frame (no seams between segments)"""
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
def arc(c, R, z, a0, a1, segs=16, ry=None):
    ry = R if ry is None else ry
    return [(c[0] + R*math.cos(math.radians(a)), c[1] + ry*math.sin(math.radians(a)), z) for a in np.linspace(a0, a1, segs + 1)]
def flare(g, p0, d, L, r0, R, col, x0=.12, m=1.0, segs=16, r_mid=None, inner=(26, 16, 10), rim_col=None, throat=None):
    """bell along d from p0 (radius r0) to the rim (radius R) at p0 + d*L; optional coloured throat (gold-plated interior)"""
    p0, d = np.asarray(p0, float), unit(d); r_mid = r_mid or r0*1.3
    def rad(s): return max(lerp(r_mid, r0, s/L), R*(x0/(s + x0))**m)
    ss = [L*(k/segs)**2 for k in range(segs + 1)][::-1]
    for s0, s1 in zip(ss, ss[1:]):
        T.tube(g, p0 + d*(L - s0), p0 + d*(L - s1), rad(s0), rad(s1), col=col, n=16 if s1 < .15 else 10, spec=.9, two=s1 < .3)
    rim = p0 + d*L
    T.tube(g, rim - d*.008, rim + d*.003, R*1.035, R*1.035, col=rim_col or col, n=16, two=True)
    if throat: T.tube(g, rim - d*.05, rim - d*.006, rad(.05)*.96, rad(.006)*.96, col=throat, n=16, two=True)
    g.face(S.ring(rim - d*.07, d, rad(.07)*.8, 14), inner, ref=rim - d, emit=True, two=True)
    return rim
def mouthpiece(g, p, d, r=.008, col=NICKEL):
    p, d = np.asarray(p, float), unit(d)
    T.tube(g, p, p + d*.055, r*1.1, r*.9, col=col); T.tube(g, p + d*.055, p + d*.078, r*.9, r*2.8, col=col, n=10)
    T.tube(g, p + d*.078, p + d*.083, r*2.9, col=col, n=10)
def stand(g, x, ytop, z=0.):
    S.rod(g, (x, 0, z - .03), (x, ytop, z), .012); g.ngon(x, .004, z - .03, .13, DARKBASE, n=8)

def piston(g, x, y0, y1, z, col, cap, down, btn=PEARL, r=.017, stem=.05, axis=(0, 1, 0)):
    """piston valve: casing y0..y1 along axis from (x, y0, z), caps, stem and finger button (lit when pressed)"""
    a = unit(axis); base = np.array([x, y0, z], float); top = base + a*(y1 - y0)
    T.tube(g, base, top, r, col=col, n=10)
    for p in (base, top): T.tube(g, p - a*.006, p + a*.006, r*1.2, col=cap, n=10)
    tip = top + a*(stem - (.018 if down else 0))
    T.tube(g, top, tip, .004, col=cap, n=6)
    T.tube(g, tip, tip + a*.01, .0145, col=HOT if down else btn, n=10, spec=.4)
    return top
def rotor(g, c, nrm, r, col, cap, down, lever=(1, 0, 0), arm=.07):
    """rotary valve: short drum facing nrm, cap disc, lever arm with a paddle"""
    c, nrm = np.asarray(c, float), unit(nrm)
    T.tube(g, c - nrm*.022, c + nrm*.022, r, col=col, n=12)
    g.face(S.ring(c + nrm*.023, nrm, r*.86, 12), HOT if down else dim(col, .85), ref=c, emit=down)
    lv = unit(lever); pad = c + nrm*.03 + lv*arm + np.array([0, -.012 if down else 0, 0])
    g.line(c + nrm*.024, pad, cap); g.line(c + nrm*.024 + [0, .003, 0], pad + [0, .003, 0], cap)
    box_at(g, pad, .011, HOT if down else PEARL, emit=down, spec=.5)
def u_loop(g, a, b, d, col, r=.0065, ln=.08):
    """valve slide: two parallel tubes from points a, b along direction d, joined by a half-round bend"""
    a, b, d = np.asarray(a, float), np.asarray(b, float), unit(d)
    path(g, [a, a + d*ln], r, col, n=6); path(g, [b, b + d*ln], r, col, n=6)
    c = (a + b)/2 + d*ln; h = (a - b)/2
    path(g, [c + h*math.cos(t) + d*abs(h).max()*math.sin(t)*1.0 for t in np.linspace(0, math.pi, 7)], r, col, n=6)

def fing(m, transpose):
    """valves down for sounding pitch m on an instrument whose written pitch is m + transpose (trumpet fingering table)"""
    return {int(ch) - 1 for ch in T.fingering(m + transpose - 2)}

# ---------------------------------------------------------------- the modern YMP-204M (Yamaha product photo) and its bell-front relatives
def bellfront(g, m, c):
    """trumpet-like marching mellophone: leadpipe in front, big oval wrap behind it, pistons on top, huge forward bell.
       c: col, cap, trim, bellR, wrap ('oval' Yamaha | 'long' King 1121 | 'tall' 1960s Olds/Reynolds), valves, keyed"""
    col, cap, trim = c["col"], c["cap"], c.get("trim", c["col"])
    yb, zb = .6, -.034                                          # bell-tube axis (behind the valve stems)
    xs = c.get("vx", (-.1, -.065, -.03))
    on = fing(m, 7)
    wrap = c["wrap"]
    xbow = {"oval": -.3, "long": -.34, "tall": -.4}[wrap]
    ybot = {"oval": .42, "long": .43, "tall": .47}[wrap]
    # bell tube from the back bow to the bell, then the flare
    path(g, [(xbow, yb, zb), (xs[-1] + .03, yb, zb)], .011, col, r1=.019)
    rim = flare(g, (xs[-1] + .03, yb, zb), (1, 0, 0), .45, .019, c["bellR"], col, x0=.11, m=1.0, r_mid=.024,
                throat=c.get("throat"), rim_col=trim)
    # back bow + bottom tube into the valves
    cb = (xbow, (yb + ybot)/2); rr = (yb - ybot)/2
    path(g, [(x_, y_, zb*(1 - k/12)) for k, (x_, y_, _) in enumerate(arc(cb, rr*.9 if wrap != "tall" else rr*.7, 0, 90, 270, 12, ry=rr))], .011, col)
    path(g, [(xbow, ybot, 0), (xs[0] - .02, ybot, 0), (xs[0], ybot + .025, 0)], .011, col)
    if wrap == "long":                                           # King 1121: a second long flat loop under the bell
        path(g, arc((.02, .465), .2, -.02, 200, 340, 12, ry=.045), .0095, col)
        path(g, [(-.17, .45, -.02), (xs[1], .45, -.01)], .0095, col)
    # pistons
    vy0, vy1 = .45, .555
    for i, x in enumerate(xs):
        piston(g, x, vy0, vy1, 0, trim if c.get("nickel") else col, cap, i in on, btn=c.get("btn", PEARL), stem=yb - vy1 + .045)
    # valve slides: 1st to the back, 2nd down, 3rd forward (with a finger ring)
    if len(xs) >= 1: u_loop(g, (xs[0] - .016, .51, 0), (xs[0] - .016, .487, 0), (-1, 0, 0), col, ln=.08)
    if len(xs) >= 2: u_loop(g, (xs[1] - .008, vy0 - .006, 0), (xs[1] + .008, vy0 - .006, 0), (0, -1, 0), col, ln=.035)
    if len(xs) >= 3:
        u_loop(g, (xs[2] + .016, .52, 0), (xs[2] + .016, .497, 0), (1, 0, 0), col, ln=.085)
        path(g, arc((xs[2] + .115, .508), .012, .0, 0, 360, 8), .003, trim, n=5)
    # leadpipe in front of the valves, into the main tuning slide hanging behind the third valve
    mp = np.array([-.47, .582, .03]); x_end = xs[-1] + .13
    path(g, [mp + [.08, 0, 0], (x_end - .02, .582, .03)], .0085, col, r1=.0105)
    path(g, [(x_end - .02, .582, .03), (x_end + .015, .565, .02), (x_end + .02, .5, .008), (x_end - .01, .465, 0),
             (xs[-1] + .03, .465, 0), (xs[-1] + .012, .47, 0)], .0105, col)
    mouthpiece(g, mp + [.08, 0, 0], (-1, 0, 0), r=.0085)
    for x_ in (-.36, -.2):                                       # braces between leadpipe and bell tube
        g.line((x_, .582, .03), (x_, yb, zb), dim(col, .7))
    box_at(g, (x_end + .02, .48, .01), .006, trim)               # water key
    if c.get("lyre"):
        path(g, [(-.25, .582, .042), (-.25, .64, .05)], .004, trim, n=5)
        g.box(-.28, -.22, .64, .69, .048, .052, (60, 62, 70))
    stand(g, -.06, ybot - .01)
    return rim, (1, 0, 0)

MELLO = {
 "modern": dict(col=LACQ, cap=(240, 200, 110), trim=(240, 196, 100), bellR=.165, wrap="oval", throat=None),
 "f2000":  dict(col=SILVER, cap=CHROME, trim=CHROME, bellR=.165, wrap="long", lyre=True, btn=(248, 248, 252)),
 "march69": dict(col=(222, 176, 82), cap=NICKEL, trim=NICKEL, bellR=.145, wrap="tall", nickel=True, btn=(236, 232, 218)),
}

def bugle68(g, m):
    """Olds Ultratone G-F piston-rotor mellophone bugle: silver, long bell section, long oval loop, ONE piston + ONE rotor"""
    col, cap = (220, 216, 202), (238, 236, 228)
    on = fing(m, 5); pdown, rdown = (0 in on or 1 in on), (2 in on or 0 in on)
    yb, zb = .59, -.03
    path(g, [(-.36, yb, zb), (-.05, yb, zb)], .011, col, r1=.02)
    rim = flare(g, (-.05, yb, zb), (1, 0, 0), .5, .02, .172, col, x0=.13, m=1.0, r_mid=.026)
    path(g, arc((-.36, .52), .07, 0, 90, 270, 10), .011, col)              # back bow
    path(g, [(-.36, .45, 0), (.0, .45, 0), (.07, .46, 0)], .011, col)      # long bottom of the oval
    path(g, arc((.07, .49), .03, 0, -90, 90, 8), .011, col)                # front bend back up
    path(g, [(.07, .52, 0), (-.02, .52, 0)], .011, col)
    piston(g, -.04, .47, .575, 0, col, cap, pdown, btn=PEARL, stem=.06)
    u_loop(g, (-.04, .465, 0), (-.04 + .016, .465, 0), (0, -1, 0), col, ln=.03)
    rotor(g, (-.17, .515, .022), (0, 0, 1), .028, col, cap, rdown, lever=(.55, .85, 0), arm=.075)
    path(g, [(-.17, .495, .0), (-.17, .46, 0)], .009, col)                 # rotor slide down into the oval
    u_loop(g, (-.2, .515, 0), (-.2, .49, 0), (-1, 0, 0), col, ln=.07)      # long rotor slide (the F 'slide')
    mp = np.array([-.47, .565, .03])
    path(g, [mp + [.08, 0, 0], (-.06, .565, .03), (-.045, .56, .02)], .0085, col, r1=.0105)
    mouthpiece(g, mp + [.08, 0, 0], (-1, 0, 0), r=.0085)
    for x_ in (-.3, -.12): g.line((x_, .565, .03), (x_, yb, zb), dim(col, .7))
    stand(g, -.1, .445)
    return rim, (1, 0, 0)

def bugle77(g, m):
    """two-valve G mellophone bugle (King K-50 type): chrome, TWO upright pistons side by side, fat round wrap"""
    col, cap = CHROME, (240, 242, 248)
    on = fing(m, 5); on = {0 if 2 in on else i for i in on if i in (0, 1)} | ({0, 1} if 2 in on else set())
    yb, zb = .61, -.034
    path(g, [(-.27, yb, zb), (-.02, yb, zb)], .012, col, r1=.02)
    rim = flare(g, (-.02, yb, zb), (1, 0, 0), .46, .02, .16, col, x0=.12, m=1.0, r_mid=.026)
    path(g, [(x_, y_, zb*(1 - k/16)) for k, (x_, y_, _) in enumerate(arc((-.27, .5), .125, 0, 90, 270, 16, ry=.11))], .013, col)   # fat round back bow
    path(g, [(-.27, .39, 0), (-.1, .39, 0)], .013, col)
    path(g, arc((-.1, .43), .04, 0, -90, 0, 6), .012, col)
    for i, x in enumerate((-.085, -.045)):
        piston(g, x, .455, .565, 0, col, cap, i in on, btn=(30, 30, 36), stem=.09)
    u_loop(g, (-.101, .51, 0), (-.101, .487, 0), (-1, 0, 0), col, ln=.09)
    u_loop(g, (-.045 - .008, .449, 0), (-.045 + .008, .449, 0), (0, -1, 0), col, ln=.04)
    mp = np.array([-.45, .575, .03])
    path(g, [mp + [.08, 0, 0], (.06, .575, .03), (.085, .55, .02), (.08, .49, .01), (.04, .47, 0), (-.03, .47, 0)], .0095, col, r1=.011)
    mouthpiece(g, mp + [.08, 0, 0], (-1, 0, 0), r=.0085)
    g.line((-.22, .575, .03), (-.22, yb, zb), dim(col, .7))
    stand(g, -.15, .385)
    return rim, (1, 0, 0)

def mellophonium(g, m):
    """Conn 16E: a concert mellophone's round wrap with the bell 'unwound' straight forward - long horn-like flare, three pistons"""
    col, cap = (230, 180, 84), NICKEL
    cc, R = np.array([-.17, .5]), .085
    for k, (Rk, z) in enumerate(((R, 0.), (R*.86, -.014))):
        path(g, arc(cc, Rk, z, 0, 360, 22), .0085, col)
    path(g, arc(cc, R*.72, -.026, 30, 300, 14), .0085, (214, 150, 96))     # rose-brass tuning loop
    on = fing(m, 7)
    xs = (-.11, -.075, -.04)
    for i, x in enumerate(xs):
        piston(g, x, .52, .615, .02, col, cap, i in on, btn=PEARL, stem=.05)
        path(g, [(x, .515, .02), (x, .49, .01), (x - .01, .47, 0)], .006, col, n=6)
    path(g, [(xs[-1] + .01, .54, .02), (.0, .53, .01), (.03, .52, 0)], .012, col, r1=.016)
    rim = flare(g, (.03, .52, 0), (1, .03, 0), .5, .016, .145, col, x0=.06, m=.85, r_mid=.02)   # long, gradual horn flare
    mp = np.array([-.42, .63, .02])
    path(g, [mp + [.075, 0, 0], (-.24, .62, .02), (-.16, .61, .02), (-.12, .6, .02)], .0085, col)
    mouthpiece(g, mp + [.075, 0, 0], (-1, 0, 0), r=.0085)
    stand(g, -.17, .415)
    return rim, (1, .03, 0)

def concert(g, m):
    """Conn 4-E concert mellophone: silver satin, gold bell interior, double-loop round wrap, tilted front-action pistons,
       12-inch bell pointing back-left (away from the bell-front direction)"""
    col, cap, gold = (200, 204, 212), (226, 228, 236), (236, 196, 96)
    cc, R = np.array([.08, .56]), .16
    for Rk, z in ((R, 0.), (R*.9, -.016)): path(g, arc(cc, Rk, z, 0, 360, 30), .0095, col)
    br = arc(cc, R + .012, .006, 225, 196, 6)
    path(g, br, .01, col, r1=.024)
    bdir = unit((-.93, -.24, -.28))
    rim = flare(g, br[-1], bdir, .27, .024, .15, col, x0=.1, m=.8, r_mid=.03, throat=gold)
    on = fing(m, 7)
    ax = unit((-.45, .8, .38))                                    # pistons tilted toward the player (front action)
    for i in range(3):
        base = np.array([.11 + .035*i, .5 - .03*i, .01])
        top = base + ax*.095
        T.tube(g, base, top, .016, col=col, n=10)
        for p_ in (base, top): T.tube(g, p_ - ax*.006, p_ + ax*.006, .019, col=cap, n=10)
        down = i in on; tip = top + ax*(.045 - (.016 if down else 0))
        T.tube(g, top, tip, .004, col=cap, n=6); T.tube(g, tip, tip + ax*.01, .014, col=HOT if down else PEARL, n=10, spec=.4)
        u_loop(g, base + [.0, -.008, 0], base + [.0, .008, 0], (1, -.25, 0), col, ln=.06 + .015*i)
    for k, (a0, a1) in enumerate(((40, 140), (150, 200))):        # swap-slide crooks inside the wrap
        path(g, arc(cc, R*.62 - .02*k, -.02, a0, a1, 8), .0075, col)
    mp = np.array([.25, .78, .01])
    path(g, [mp, (.21, .72, .01), (.18, .66, .01), (.14, .58, .01)], .0085, col, r1=.0095)
    mouthpiece(g, mp, (.35, 1, 0), r=.0082)
    stand(g, cc[0], cc[1] - R)
    return rim, tuple(bdir)

def koenig(g, m):
    """Courtois Koenig horn (MIMEd 15365): silver-plated circular corpus, BELL DIRECTED DOWNWARDS, three Perinet valves"""
    col, cap = (208, 212, 220), (230, 232, 238)
    cc, R = np.array([.0, .58]), .15
    path(g, arc(cc, R, 0, -86, 266, 36), .0115, col, r1=.0135)        # circular corpus (wraps once, like a tenor cor)
    p_out = np.array([cc[0], cc[1] - R, 0.])
    rim = flare(g, p_out, (0, -1, 0), .29, .014, .1, col, x0=.08, m=.9, r_mid=.019)   # bell directed downwards
    path(g, arc(cc, R*.8, -.015, 100, 260, 12), .009, col)            # false loop completing the corpus
    path(g, arc((cc[0] - .02, cc[1] + .01), .06, -.012, 0, 360, 14), .008, col)   # looped tuning slide (Eb / Db)
    on = fing(m, 9)
    for i, x in enumerate((.035, .07, .105)):
        piston(g, x, .64, .74, .016, col, cap, i in on, btn=(240, 236, 220), stem=.045)
        u_loop(g, (x - .008, .635, .016), (x + .008, .635, .016), (0, -1, 0), col, ln=.035 + .012*i)
    mp = np.array([-.15, .84, .012])
    path(g, [mp, (-.1, .78, .012), (-.04, .745, .014), (.035, .74, .016)], .0085, col)
    mouthpiece(g, mp, (-.6, .8, 0), r=.008)
    S.rod(g, (.2, 0, -.03), (.2, .56, -.01), .012); g.ngon(.2, .004, -.03, .13, DARKBASE, n=8)
    path(g, [(.2, .56, -.01), (.16, .575, -.005), (cc[0] + R + .01, cc[1] - .01, 0)], .006, (60, 58, 66), n=6)
    return rim, (0, -1, 0)

def ots(g, m):
    """over-the-shoulder E-flat alto saxhorn (Met 503845): compact valve loop with three rotary valves in front,
       the long bell tube running BACKWARD over the player's shoulder"""
    col, cap = (198, 164, 92), (222, 200, 150)
    cl = np.array([.18, .52])
    path(g, arc(cl, .11, 0, 0, 360, 20, ry=.05), .0095, col)
    path(g, [(.07, .52, 0), (-.05, .53, 0)], .012, col, r1=.016)
    rim = flare(g, (-.05, .53, 0), unit((-1, .12, 0)), .46, .016, .115, col, x0=.08, m=.85, r_mid=.02)
    on = fing(m, 9)
    for i, x in enumerate((.13, .18, .23)):
        rotor(g, (x, .575, .0), (0, 1, 0), .02, col, cap, i in on, lever=(.15, 0, 1), arm=.06)
        u_loop(g, (x - .009, .55, -.012), (x + .009, .55, -.012), (0, -.3, -1), col, r=.0055, ln=.05 + .012*i)
    mp = np.array([.03, .6, .0])
    path(g, [(.26, .56, 0), (.2, .6, 0), (.1, .6, 0), mp + [.075, 0, 0]], .008, col)
    mouthpiece(g, mp + [.075, 0, 0], (-1, 0, 0), r=.008, col=(220, 214, 196))
    stand(g, .18, .468)
    return rim, tuple(unit((-1, .12, 0)))

def upright(g, m, c):
    """bell-up horns: saxhorn (bell column LEFT, valves on the right) and Besson tenor horn (bell column RIGHT)"""
    col, cap = c["col"], c["cap"]; sgn = c["side"]               # +1: bell column at -x (saxhorn); -1: mirrored (tenor horn)
    xb, xr = -.075*sgn, .075*sgn
    yb0, W = c["ybow"], abs(xr - xb)
    path(g, [(x_, y_, 0) for x_, y_, _ in arc(((xb + xr)/2, yb0), W/2, 0, 180, 360, 14)], .016, col, r1=.012)
    rim = flare(g, (xb, yb0, 0), (0, 1, 0), c["H"] - yb0, .016, c["R"], col, x0=.1, m=.95, r_mid=.022)
    path(g, [(xr, yb0, 0), (xr, c["ytop"], 0)], .012, col, r1=.0105)
    on = fing(m, 9)
    vx = [xr + sgn*(.05 + .036*i) for i in range(3)] if sgn > 0 else [xr + sgn*(.05 + .036*i) for i in range(3)][::-1]
    vy0 = c["vy"]
    for i, x in enumerate(vx):
        piston(g, x, vy0, vy0 + .1, .0, col, cap, i in on, btn=PEARL, stem=.04)
        u_loop(g, (x - .008, vy0 - .006, 0), (x + .008, vy0 - .006, 0), (0, -1, 0), col, ln=.06 + .025*i)
    path(g, [(xr, c["ytop"], 0), ((xr + vx[0])/2, c["ytop"] + .03, 0), (vx[0], vy0 + .1, 0)], .0105, col)
    mp_dir = np.array([-sgn, 0, 0])                              # leadpipe crosses in front of the bell column
    ymp = vy0 + c.get("mp_dy", -.03)
    near = min(vx, key=abs)
    p0 = np.array([near, ymp, .03]); mp = np.array([xb - sgn*.13, ymp, .03])
    path(g, [(near, vy0 + .02, 0), p0 + [0, 0, 0], mp], .0085, col)
    mouthpiece(g, mp, mp_dir, r=.0085)
    if c.get("loop"):                                            # Besson: big main tuning loop under the valves
        path(g, arc(((xr + vx[1])/2, vy0 - .1), .055, -.015, 0, 360, 14, ry=.08), .009, col)
    for yy in (yb0 + .12, c["ytop"] - .05): g.line((xb, yy, 0), (xr, yy, 0), dim(col, .7))
    stand(g, (xb + xr)/2, yb0 - W/2 - .005)
    return rim, (0, 1, 0)

UPRIGHT = {
 "saxhorn": dict(col=(220, 176, 86), cap=(236, 214, 150), side=1, ybow=.19, H=.8, R=.095, ytop=.53, vy=.41),
 "tenor":   dict(col=(232, 188, 96), cap=(244, 214, 140), side=-1, ybow=.18, H=.79, R=.112, ytop=.55, vy=.42, loop=True, mp_dy=-.04),
}

def cur(s, t, ev):
    live = [e for e in ev if e[0] < e[3] + .03]
    return min(live)[1] if live else (min(ev)[1] if ev else s["ev"][0][1])

def build(g, t, ev, fx, s):
    k = s["key"]; m = cur(s, t, ev)
    if k in MELLO: rim, d = bellfront(g, m, MELLO[k])
    elif k == "bugle68": rim, d = bugle68(g, m)
    elif k == "bugle77": rim, d = bugle77(g, m)
    elif k == "mellophonium": rim, d = mellophonium(g, m)
    elif k == "concert": rim, d = concert(g, m)
    elif k == "koenig": rim, d = koenig(g, m)
    elif k == "ots": rim, d = ots(g, m)
    else: rim, d = upright(g, m, UPRIGHT[k])
    rim, d = np.asarray(rim, float), np.asarray(d, float)
    left = d[0] < -.3
    off = np.array([.15, -.03, 0]) if d[1] > .7 else d*.08 + [0, .14, 0]    # bell-up horns: notes rise beside the bell
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9: fx.append(("note", tuple(rim + off), age))
        if tp and age < .6: fx.append(("wavel" if left else "wave", tuple(rim + d*.05 + ([0, .04, 0] if abs(d[1]) > .7 else [0, 0, 0])), age, 0))

# ---------------------------------------------------------------- sound: the horns
def _t(n): return np.arange(n)/SR
TONE = {"modern": dict(bright=.6, vib=.003, scoop=.03, tilt=.95), "saxhorn": dict(bright=.45, vib=0, scoop=.035, rough=.06, tilt=1.0),
        "koenig": dict(bright=.35, vib=.004, scoop=.025, tilt=1.1), "ots": dict(bright=.6, vib=0, scoop=.04, rough=.1, tilt=.9),
        "tenor": dict(bright=.32, vib=.007, scoop=.02, tilt=1.12), "concert": dict(bright=.42, vib=.004, scoop=.03, tilt=1.05),
        "mellophonium": dict(bright=1.0, vib=.003, scoop=.04, tilt=.85, fall=1.), "bugle68": dict(bright=.9, vib=0, scoop=.03, tilt=.85),
        "march69": dict(bright=.8, vib=0, scoop=.025, tilt=.9), "bugle77": dict(bright=1.25, vib=.004, scoop=.05, tilt=.8, fall=1.),
        "f2000": dict(bright=.95, vib=.002, scoop=.03, tilt=.85)}
def brass(f, dur, vel, bright=1., vib=.004, scoop=.03, rough=0., tilt=.85, fall=0.):
    """conical-bore brass: dark at low velocity, brighter when pushed; optional 'fall' (pitch drop) off long notes"""
    n = int((dur + .12)*SR); t = _t(n)
    env = np.minimum(1, t/.03)**1.5*np.where(t < dur, 1, np.exp(-(t - dur)/.035))*(.86 + .14*np.exp(-t*3))
    fr = f*(1 - scoop*np.exp(-t/.025))*(1 + vib*np.sin(2*np.pi*5.3*t)*np.clip((t - .2)/.2, 0, 1))
    if fall and dur >= .4:
        k = np.clip((t - dur*.72)/(dur*.28 + .05), 0, 1); fr = fr*2**(-fall*1.6*k**2/12); env = env*(1 - .6*k)
    ph = 2*np.pi*np.cumsum(fr)/SR
    dark = np.zeros(n); brt = np.zeros(n); sd = sb = 0
    for k in range(1, 28):
        if f*k > 7000: break
        a1 = math.exp(-k/2.2)/k**tilt; a2 = math.exp(-k/(4.5 + 6*bright))/k**tilt
        s_ = np.sin(k*ph); dark += a1*s_; brt += a2*s_; sd += a1; sb += a2
    mb = np.clip(env*vel, 0, 1)**1.3
    x = (dark/sd*(1 - mb) + brt/sb*mb)*env
    if rough: x = x + rough*np.sin(ph*.5)*env*.3
    return x*vel
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]; key = s["key"]
    x = brass(midi_hz(m), dur, v, **TONE[key])
    t = _t(len(x)); body = np.sin(2*np.pi*midi_hz(m)*t)*np.minimum(1, t/.03)*np.where(t < dur, 1, np.exp(-(t - dur)/.03))
    x = x + .22*body*v                                            # the mellophone's round, horn-like core
    return x*(.62 if cs > 1 else 1.0)

# ---------------------------------------------------------------- backing: dark drumline (snare line, tenors, split basses, halftime 808, minor pad)
_r = np.random.default_rng(23)
def _bp(lo, hi): return butter(2, [lo/(SR/2), hi/(SR/2)], "band")
_SNB = _bp(1500, 7000)
def snare(v, rim=False):
    """high-tension marching snare: dry crack, short wire burst; rimshots add a metallic ping"""
    n = int(.2*SR); t = _t(n)
    nz = lfilter(*_SNB, _r.normal(0, 1, n))*np.exp(-t*(26 if rim else 34))
    tone = np.sin(2*np.pi*345*t)*np.exp(-t*48) + .45*np.sin(2*np.pi*610*t)*np.exp(-t*70)
    x = nz*.9 + tone*.55
    x[:int(.0015*SR)] += _r.normal(0, 1, int(.0015*SR))*.8
    if rim: x = x*1.25 + sum(np.sin(2*np.pi*f*t + k)*np.exp(-t*22) for k, f in enumerate((1180, 1760, 2390)))*.22
    return np.tanh(1.4*x)*v
def tap(v): return snare(v)*.35
def tenor(f0):
    def fn(v):
        n = int(.35*SR); t = _t(n); fr = f0*(1 + .22*np.exp(-t*30)); ph = 2*np.pi*np.cumsum(fr)/SR
        x = (np.sin(ph) + .25*np.sin(2.3*ph))*np.exp(-t*11) + lfilter(*_bp(900, 5000), _r.normal(0, 1, n))*np.exp(-t*60)*.25
        return x*v*.8
    return fn
def bassd(f0):
    def fn(v):
        n = int(.5*SR); t = _t(n); fr = f0*(1 + .12*np.exp(-t*25))
        x = np.sin(2*np.pi*np.cumsum(fr)/SR)*np.exp(-t*7.5) + lfilter(*_bp(150, 900), _r.normal(0, 1, n))*np.exp(-t*80)*.35
        return np.tanh(1.3*x)*v
    return fn
def rollup(v, dur=1.0):
    """buzz roll crescendo that runs straight into the loop point"""
    n = int(dur*SR); out = np.zeros(n); k = 0; tt = 0.
    while tt < dur - .005:
        a = int(tt*SR); g_ = (tt/dur)**1.6
        h = snare(v*(.12 + .88*g_), rim=False)[:n - a]; out[a:a + len(h)] += h*(.5 + .5*g_)
        tt += .038 - .012*g_; k += 1
    return out
def plate(v):
    n = int(1.8*SR); t = _t(n); nz = np.diff(_r.normal(0, 1, n + 1))
    x = sum(np.sin(2*np.pi*f*t + k) for k, f in enumerate((3100, 4300, 5600, 7100)))*.04
    return (nz*.15 + x)*np.exp(-t*2.6)*np.minimum(1, t/.003)*v
TENORS = [tenor(f) for f in (420, 340, 270, 215, 520)]
BASSES = [bassd(f) for f in (62, 73, 87, 104, 123)]
KIT = {"sn": snare, "rim": lambda v: snare(v, True), "tap": tap, "crash": plate, "rollfit": lambda v: rollup(v, 2*BEAT_S),
       **{f"t{i}": TENORS[i] for i in range(5)}, **{f"b{i}": BASSES[i] for i in range(5)}}
KICK_KINDS, ACCENT_KINDS = ("b0",), ("rim",)
# one bar of snare-line cadence (16ths): accents, ghost taps, the halftime rimshot on beat 3
SN_A = {0: ("sn", .85), 3: ("sn", .7), 6: ("sn", .75), 8: ("rim", 1.), 11: ("sn", .7), 14: ("sn", .8)}
SN_B = {0: ("sn", .85), 2: ("sn", .55), 3: ("sn", .7), 8: ("rim", 1.), 10: ("sn", .6), 11: ("sn", .7), 13: ("sn", .6)}
TAPS = [1, 2, 4, 5, 7, 9, 10, 12, 13, 15]
BASS_A = [(0, 0), (3, 2), (6, 1), (10, 3), (11, 4), (13, 2)]
BASS_B = [(0, 0), (2, 1), (5, 2), (7, 3), (10, 4), (12, 3), (14, 1)]
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT; s16 = Q/4
    starts = {0} | {4 + 2*k for k in range(N_ROWS)} | {4 + 2*N_ROWS}
    nb = len(BARS) - 1                                            # last entry = the final chord
    for b in range(nb):
        T0 = b*B; odd = b % 2
        pat = SN_B if odd else SN_A
        for st, (k, v) in pat.items(): ev.append((T0 + st*s16, k, v))
        ev += [(T0 + st*s16, "tap", .5 + .2*((st*7) % 3)/2) for st in TAPS if st not in pat]
        for st, d in (BASS_B if odd else BASS_A): ev.append((T0 + st*s16, f"b{d}", .9 if d == 0 else .75))
        if odd or b == nb - 1:                                    # tenor sweep across the quads into the next bar
            ev += [(T0 + (12 + i)*s16, f"t{i % 4}", .8) for i in range(4)]
            ev += [(T0 + 14.5*s16, "t4", .6)]
        if b in starts: ev.append((T0, "crash", .8 if b else 1.))
    T0 = nb*B                                                     # final chord: unison hit, hold, then roll into the loop
    ev += [(T0, "crash", 1.), (T0, "b0", 1.), (T0, "b2", .8), (T0, "rim", 1.)]
    rl = 2*Q
    ev.append((c.TOTAL - rl, "rollfit", 1.))
    return ev
def _chords(b):
    ch = BARS[b]; return list(ch) if isinstance(ch, tuple) else [ch]*4
def bass_events(c):
    out = []; nb = len(BARS) - 1; Q = c.BEAT
    for b in range(nb):
        T0 = b*c.BAR; cs = _chords(b)
        out.append((T0, CH[cs[0]][0], 1.6*Q, 1.))
        if cs[2] != cs[0]: out.append((T0 + 2*Q, CH[cs[2]][0], 1.6*Q, .9))
        elif b % 2: out.append((T0 + 2.75*Q, CH[cs[2]][0] + 12, .6*Q, .7))
    out.append((nb*c.BAR, CH[BARS[-1]][0], FINAL_LEN + .3, 1.))
    return out
def bass_voice(m, dur, v):
    """halftime 808: sine with a short pitch drop and a touch of drive (2nd harmonic keeps it audible on phones)"""
    n = int((dur + .05)*SR); t = _t(n); f = midi_hz(m)*(1 + .5*np.exp(-t*40))
    ph = 2*np.pi*np.cumsum(f)/SR
    x = np.sin(ph) + .18*np.sin(2*ph)
    env = np.minimum(1, t/.004)*np.exp(-t*(.35 if dur > 2 else 1.1))*np.clip((dur + .05 - t)/.05, 0, 1)
    return np.tanh(1.6*x*env)*v
def pad_voice(notes, dur, v):
    """dark minor pad (detuned, filtered) for whole bars; low string marcato stabs for chords that change every beat"""
    n = int((dur + .5)*SR); out = np.zeros(n); t = _t(n)
    long_ = dur > 1.2
    for mm in notes:
        f = midi_hz(mm)
        for det in (-.0045, .0045):
            ph = 2*np.pi*f*(1 + det)*t
            x = sum(np.sin(k*ph)/k**1.5 for k in range(1, 7))
            out += x*.4
        out += .5*np.sin(np.pi*f*t)                              # sub-octave body
    att = .35 if long_ else .02; rel = .4 if long_ else .12
    env = np.minimum(1, t/att)*np.clip((dur + rel - t)/rel, 0, 1)
    if not long_: env *= np.exp(-t*4)
    return out*env*v*.25
MIX = dict(drums=-20, bass=-22, pad=-28, verb=-24, verb_len=1.3)

# ---------------------------------------------------------------- background: inside a giant brass clockwork
THEMES = {   # gear metal, glow (light colour)
 "intro":        ((96, 102, 116), (255, 168, 60)),     # gunmetal + amber
 "saxhorn":      ((196, 150, 72), (150, 26, 40)),      # brass + oxblood
 "koenig":       ((60, 120, 86), (236, 190, 80)),      # Second Empire green + gold
 "ots":          ((54, 76, 140), (150, 150, 158)),     # Union blue + gunsmoke grey
 "tenor":        ((52, 48, 50), (214, 40, 44)),        # colliery soot + band-uniform red
 "concert":      ((190, 194, 204), (240, 214, 160)),   # silver + champagne
 "mellophonium": ((36, 44, 96), (246, 246, 255)),      # midnight blue + spotlight white
 "bugle68":      ((150, 140, 96), (255, 140, 40)),     # scout khaki + orange
 "march69":      ((48, 110, 60), (250, 250, 244)),     # field green + white
 "bugle77":      ((200, 96, 36), (226, 232, 244)),     # burnt orange + chrome
 "f2000":        ((176, 182, 196), (60, 150, 255)),    # silver + electric blue
}
def ped_cols(th): T_ = THEMES[th]; return F.ped_cols_from(mix(dim(T_[0], .55), (40, 30, 30), .35))
# gears: centre x, y, radius, teeth, depth (0 far .. 1 near), direction
GEARS = [(-6, 168, 112, 24, 0., 1), (262, 92, 74, 16, .35, -1), (282, 300, 96, 20, 0., -1), (8, 352, 66, 14, .55, -1),
         (176, -6, 52, 12, .25, 1)]
_GA = []
for gx, gy, gr, gn, gd, gs in GEARS:
    r = np.hypot(XX - gx, YY - gy); a = np.arctan2(YY - gy, XX - gx)
    _GA.append((r, a))
SKY = {k: None for k in THEMES}
def _sky(th):
    if SKY[th] is None:
        met, glow = THEMES[th]
        SKY[th] = F.make_grad(mix((6, 4, 6), met, .1), mix(dim(glow, .5), (40, 18, 10), .55), y0=0, y1=RH)
    return SKY[th]
PIPE_Y = 432
VENTS = [40, 135, 230]
PISTONS = [(26, 372), (244, 372)]
GAUGES = [(56, 268, 11), (214, 252, 9)]
TEXTB = ((YY >= 40) & (YY < 102)) | ((YY >= 330) & (YY < 380))
TEXTE = (((YY >= 36) & (YY < 40)) | ((YY >= 102) & (YY < 106)) | ((YY >= 326) & (YY < 330)) | ((YY >= 380) & (YY < 384))) & (BAY < .5)
GRATE = (YY >= 410) & (((XX + (YY - 410)//2) % 9 == 0) | ((YY - 410) % 7 == 0))

def gear_layer(bg, i, rot, metal, glow, x):
    gx, gy, gr, gn, gd, gs = GEARS[i]; r, a = _GA[i]
    pitch = 2*np.pi/gn
    ph = np.mod(a - gs*rot*pitch, pitch)/pitch                     # 0..1 across one tooth
    tooth = (ph > .18) & (ph < .62)
    body = r < gr
    teeth = (r >= gr) & (r < gr + 9) & tooth
    hub = r < gr*.22
    ring_in = (r < gr*.74) & (r > gr*.28)
    spoke_n = 5 if gn > 14 else 4
    spk = np.mod(a - gs*rot*pitch + .3, 2*np.pi/spoke_n)/(2*np.pi/spoke_n)
    holes = ring_in & ~((spk < .16) | (spk > .9))
    m_ = (body | teeth) & ~holes
    near = .45 + .55*gd
    base = np.array(metal, float)*near
    shade = np.clip(.62 + .5*(-(XX - gx)*.6 - (YY - gy))/(gr + 9), .35, 1.3)        # lit from the upper left
    col = base[None, None, :]*shade[..., None]
    lt = np.array(glow, float)
    rimlit = ((r > gr - 3) & (r < gr + 9)) & (((YY - gy) > gr*.25) | ((XX - gx)*(1 if gx < CX else -1) > gr*.5))
    col = np.where(rimlit[..., None], col*.7 + lt*.35*(.6 + .4*x["loud"]), col)     # coppery under-light on the edges
    col = np.where(hub[..., None], col*.55, col)
    rivet = (np.abs(r - gr*.84) < 1.2) & (np.mod(a - gs*rot*pitch, 2*np.pi/(gn//2))/(2*np.pi/(gn//2)) < .07)
    col = np.where(rivet[..., None], np.minimum(255, col*1.45 + 20), col)
    edge = m_ & ((np.abs(r - gr*.74) < .8) | (np.abs(r - gr*.28) < .8))
    col = np.where(edge[..., None], col*.6, col)
    dith = (BAY < .5) & ~(np.abs(r - gr*.5) < gr*.5)
    bg[m_] = np.clip(np.where(dith[..., None], col*.96, col), 0, 255)[m_]

def background(c, x):
    th0, th1, kb, T_ = x["th0"], x["th1"], x["kb"], x["T"]
    A_, B_ = THEMES[th0], THEMES[th1]
    metal = mix(A_[0], B_[0], kb); glow = mix(A_[1], B_[1], kb)
    bg = _sky(th0)*(1 - kb) + _sky(th1)*kb
    # furnace light from below
    fl = np.clip((YY - 250)/230, 0, 1)**1.6*(.5 + .25*x["pulse"])
    bg = bg + np.array(glow, float)[None, None, :]*fl[..., None]*.35
    # clockwork: every gear ticks one tooth per beat (snaps in the first quarter of the beat, then holds)
    bp = x["beat_ph"]; rot = x["beat_i"] + ease(bp/.22)
    for i in sorted(range(len(GEARS)), key=lambda k: GEARS[k][4]): gear_layer(bg, i, rot, metal, glow, x)
    # brass pipe with steam vents along the floor; vents fire on the rimshots
    mt = np.array(metal, float)
    pipe = (YY >= PIPE_Y - 5) & (YY < PIPE_Y + 5)
    bg[pipe] = mt*.7; bg[(YY == PIPE_Y - 4) & pipe] = mt*1.15; bg[(YY >= PIPE_Y + 3) & pipe] = mt*.45
    bg[GRATE] = bg[GRATE]*.55
    for vx in VENTS:
        bg[(YY >= PIPE_Y - 12) & (YY < PIPE_Y - 4) & (np.abs(XX - vx) <= 3)] = mt*.85
        bg[(YY >= PIPE_Y - 14) & (YY < PIPE_Y - 11) & (np.abs(XX - vx) <= 5)] = mt*1.05
    hits = [t for t, k, v in c.DRUMS if k == "rim" and 0 <= T_ - t < 1.2]
    for j, t0 in enumerate(hits):
        age = T_ - t0; vx = VENTS[int(round(t0/c.BEAT)) % 3]
        rad = 3 + age*34; cy = PIPE_Y - 16 - age*70
        puff = ((XX - vx)**2 + ((YY - cy)*1.25)**2 < rad**2) & (BAY < .85*(1 - age/1.2))
        bg[puff] = bg[puff]*.35 + np.array((236, 232, 226))*.65
    # background pistons pump on the horn's notes (alternating)
    ons = [o for o in c.ONSETS if o <= T_]; idx = len(ons)
    for j, (px, py) in enumerate(PISTONS):
        lastj = max([o for k_, o in enumerate(ons) if (k_ % 2) == j], default=-9)
        dy = int(14*math.exp(-(T_ - lastj)*7))
        bg[(YY >= py - 40) & (YY < PIPE_Y - 5) & (np.abs(XX - px) <= 7)] = mt*.5                       # cylinder
        bg[(YY >= py - 40) & (YY < PIPE_Y - 5) & (XX == px - 6)] = mt*.85
        rod_top = py - 70 + dy
        bg[(YY >= rod_top) & (YY < py - 40) & (np.abs(XX - px) <= 2)] = np.minimum(255, mt*1.25 + 30)  # piston rod
        bg[(YY >= rod_top - 5) & (YY < rod_top) & (np.abs(XX - px) <= 9)] = mt*.95                      # crosshead
    # pressure gauges: needles twitch with the loudness
    for gx, gy, gr in GAUGES:
        rr = np.hypot(XX - gx, YY - gy)
        bg[rr < gr + 2] = mt*.6; bg[rr < gr] = np.array((236, 228, 206))*.85
        ang = math.radians(-210 + 240*clamp(.15 + .8*x["loud"]))
        for k in range(gr - 2):
            bg[int(gy - k*math.sin(ang)), int(gx + k*math.cos(ang))] = np.array(glow)*.5 + 60
        bg[(rr < 2)] = (30, 20, 20)
    bg[TEXTB] *= .55; bg[TEXTE] *= .75
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])
