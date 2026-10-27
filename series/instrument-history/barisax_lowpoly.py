"""Pixel Pitch: the modern baritone saxophone (low A) - low-poly model (and, below, its voice). No history video yet:
built for BALLFIGHT #13 (Bassoon vs Bari Sax), adapted from the alto (altosax60_lowpoly.py, used read-only as a
library: its swept tubes, key cups, pearl touches and sax tone) and used through projects/which_instrument_wins/
pp_bridge.py. A later Pixel Pitch bari video can grow from this file without touching the model.

    python3 barisax_lowpoly.py               # face count + extent

What changes from the alto (checked against "Baritone Saxophone Amati.png" on Wikimedia Commons, CC BY-SA 4.0, a
gold-lacquer low-A bari product shot, found in Claude in Chrome 2026-10-26; scaled to ~1.10 m tall):
  - everything bigger: a ~0.82 m body (alto 0.49 m) from 92 mm across at the bow to 42 mm at the top;
  - the bell is a long, nearly straight tube running up beside the body (the alto's is short and swept), its rim at
    ~80 % of the height and ~18 cm across, leaning out and toward the player;
  - the signature loop: above the body the tube rises, coils round once (the "pigtail", out toward the back) and only
    then turns into the neck that runs out sideways to the big mouthpiece;
  - the low-A key: an extra big cup low on the body by the bow with its long rod, plus the low B / Bb cups on the bell;
  - bigger key cups and guards throughout.
Local model frame (as the alto): y up, +x toward the bell, +z toward the camera (the pearl side).

v2 (Oct 27, user after BALLFIGHT #14: "completely smooth"): the detail pass below `bari()` - keys on the real
(scaled) surface, dark pad rings, pearls, rods + guards with diamond feet, bands, bow guard, engraving, all as faces.
The voice (bari_tone) is unchanged."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_shapes as S
import trumpet_lowpoly as T
import altosax60_lowpoly as AS                       # read-only: sweep, cup, touch, sax_tone, the modern palette
from pp_framework import lerp, dim, mix

OUT, TAG = "barisax_lowpoly.mp4", "bs"
BPM = 100
SHOW_BARS = 2
HEAD_INTRO = "THE BARI SAX"

_NOTES = [(0, 36, 1, .85), (1, 43, 1, .85), (2, 46, 1, .85), (3, 48, 1, .85), (4, 46, 2, .85), (6, 36, 2, .85)]
SCENES = [
    dict(key="modern", theme="intro", name="BARITONE SAX", credit="LOW A", notes=_NOTES),
    # the same bari for small sprites (BALLFIGHT balls): true heights and positions, every tube's radius x `chunk`
    # (body, bow, bell, the loop, the neck), so the loop and the long bell still read at ~25-60 px.
    dict(key="ball", theme="intro", name="BARITONE SAX", credit="LOW A", notes=_NOTES, chunk=1.35),
]
KICK_KINDS, ACCENT_KINDS = (), ()


def drum_events(c):
    return []


# ---------------------------------------------------------------- the bari (one skeleton, the alto's construction)
C = dict(AS.SAX["modern"])                           # gold lacquer, gold keys, pearl touches (the alto's YAS-62 palette)
YB, YT, BOWR, RB, RT = .155, .975, .056, .046, .021  # bow joint, body top, bow radius, body radius bottom / top
XB = 2 * BOWR                                        # the bell's axis
RBELL0, RIM, Y_RIM = .050, .089, .875               # bell tube at the bow joint, rim radius, rim height
unit = AS.unit


def rbody(y):
    return RB + (RT - RB) * (y - YB) / (YT - YB)


def bpt(f, phi, lift=0.):
    """point + normal on the body at fraction f (0 = bow joint, 1 = top) and azimuth phi (0 = front +z, 90 = bell +x)"""
    y = lerp(YB, YT, f)
    a = math.radians(phi)
    n = np.array([math.sin(a), 0, math.cos(a)])
    return np.array([0, y, 0]) + n * (rbody(y) + lift), n


def bell_path():
    """(points, radii): up from the bow joint, a long straight tube beside the body, then the flare to the rim (leaning
    out toward +x and toward the camera)"""
    pts, rad = [], []
    for k in range(7):
        u = k / 6
        pts.append(np.array([XB + .008 * u, lerp(YB, .70, u), 0]))
        rad.append(lerp(RBELL0, .060, u))
    p0, pc, p1 = pts[-1], np.array([XB + .010, .80, .004]), np.array([XB + .034, Y_RIM, .022])
    for k in range(1, 8):
        u = 1 - (1 - k / 7) ** 1.6
        pts.append((1 - u) ** 2 * p0 + 2 * (1 - u) * u * pc + u * u * p1)
        rad.append(.060 + (RIM - .060) * (k / 7) ** 2.6)
    return pts, rad


def loop_path():
    """(points, radii): the tube rising out of the body top, the pigtail coil (out toward the back), then the neck
    running out sideways (-x) and a little up to the mouthpiece"""
    pts = [np.array([0, YT - .01, 0]), np.array([0, YT + .02, 0])]
    c = np.array([.034, YT + .03, -.012])
    R = .034
    for u in np.linspace(0, 1, 13)[1:]:
        a = math.pi - 2 * math.pi * .86 * u                                  # over the top, round, back under
        pts.append(c + np.array([R * math.cos(a), R * math.sin(a) * 1.08, -.05 * u]))
    q = pts[-1]
    for k, (dx, dy, dz) in enumerate(((-.05, -.004, -.004), (-.11, .004, -.002), (-.17, .018, .004), (-.22, .032, .010))):
        pts.append(q + np.array([dx, dy, dz]))
    n = len(pts)
    rad = [lerp(.0215, .0125, i / (n - 1)) for i in range(n)]
    return pts, rad


def mouthpiece(g, p, d):
    """the bari's big black mouthpiece: cork, body, ligature (the v2 bari draws a gold one over it), reed underneath"""
    p, d = np.asarray(p, float), unit(d)
    dn = unit(np.cross(d, [0, 0, 1.]))
    if dn[1] > 0:
        dn = -dn
    T.tube(g, p - d * .006, p + d * .034, .0135, col=AS.CORK, n=8, spec=0)
    T.tube(g, p - d * .008, p + d * .068, .0175, .0165, col=AS.BLACK, n=10, spec=.7)
    T.tube(g, p + d * .068, p + d * .112, .0165, .008, col=AS.BLACK, n=10, spec=.7)
    T.tube(g, p + d * .05, p + d * .072, .0182, col=AS.NICKEL, n=10, spec=.9)
    AS.slab(g, p + d * .04 + dn * .014, p + d * .115 + dn * .007, [0, 0, 1], .017, AS.REED)
    return p + d * .112


# ---------------------------------------------------------------- v2 (Oct 27): the detail pass
# User, after #40 v1: "the bari sax model needs more detail, as it is completely smooth right now". Two causes: in the
# `ball` scene every tube is fattened x`chunk`, but the keys were still placed on the unscaled body, so they sat buried
# inside it; and they were gold on gold, with rods / guards / engraving drawn as lines, which the BALLFIGHT sprites
# drop. Rebuilt from the same Amati low-A product shot (Commons, re-studied in Claude in Chrome 2026-10-27): what makes a
# bari read as a bari at any size is the crowded key stack - deep cups with a dark pad ring under each bright rim, a
# column of white pearl touches, long rods and big wire guards with diamond-shaped feet running the whole height, the
# key cups standing proud of the silhouette - plus the bands at the bow, the guard and felt at the bottom, the wide
# bright rim and the engraved bell. Everything is a face now (lines don't survive the sprites), placed on the real
# (scaled) surface, and the keys grow with `chunk` too.
METAL = (230, 180, 74)             # the body's lacquer (the alto's gold, a shade deeper so the keys stand off it)
KEYC = (255, 210, 104)             # key cups, arms, rims, bands: the bright gold
ROD = (170, 122, 44)               # rods, guards, posts: a deeper gold that reads as a line on the body
PAD = (58, 34, 20)                 # leather pads: the dark ring round every cup
PEARL = (248, 244, 234)
FELT = (34, 30, 34)


def bsurf(f, phi, ch, lift=0.):
    """point + outward normal on the (scaled) body at fraction f (0 = bow joint, 1 = top), azimuth phi (0 = front +z,
    90 = bell +x), `lift` beyond the surface"""
    y = lerp(YB, YT, f)
    a = math.radians(phi)
    n = np.array([math.sin(a), 0, math.cos(a)])
    return np.array([0, y, 0]) + n * (rbody(y) * ch + lift), n


def keycup(g, p, n, r, h=.010, top=KEYC, pad=True):
    """a sax key cup standing on the surface at p (normal n): a dark pad disc a little wider than the cup (it shows as a
    dark ring round the rim), the bright cup on top. Returns the cup's top centre."""
    p, n = np.asarray(p, float), unit(n)
    hp = h * .45
    if pad:
        S.prism(g, p + n * hp / 2, n, r * 1.2, hp, 10, PAD, dim(PAD, .8))
    S.prism(g, p + n * (hp + h * .3), n, r, h * .6, 10, top, dim(top, .7))
    S.prism(g, p + n * (hp + h * .62), n, r * .55, h * .08, 8, dim(top, .86), dim(top, .7))   # the cup's dimple
    return p + n * (hp + h * .65)


def bar(g, a, b, r, col):
    T.tube(g, a, b, r, col=col, n=5, spec=.85)


def diamond(g, p, n, s, col=KEYC):
    """a guard's diamond-shaped foot, soldered flat on the tube"""
    S.prism(g, p, n, s, .004, 4, col, dim(col, .7), phase=math.pi / 4)


def bari(g, ch=1.0):
    kc = KEYC
    sp = C.get("spec", .85)
    kg = 1 + (ch - 1) * .9                     # keys grow with the chunk too (they must stand off the fattened body)
    kc_ = 1 + (ch - 1) * .35                   # ... but the cups' radius only a little (they would merge into one blob)
    # body, bow, bell (the alto's swept-tube helper; radii x ch for sprites)
    ys = np.linspace(YB, YT, 15)
    AS.sweep(g, [(0, y, 0) for y in ys], [rbody(y) * ch for y in ys], METAL, n=14, spec=sp)
    ang = np.linspace(math.pi, 2 * math.pi, 9)
    bow_r, bow_ax = AS.sweep(g, [(BOWR + BOWR * math.cos(a), YB + BOWR * math.sin(a), 0) for a in ang],
                             [r * ch for r in np.linspace(RB, RBELL0, 9)], METAL, n=14, spec=sp)
    bp, br = bell_path()
    nfl = 6
    bell_r, bell_ax = AS.sweep(g, bp, [r * (ch if r < .07 else 1 + (ch - 1) * .3) for r in br], METAL, n=16, spec=sp,
                               inner=dim(METAL, .62), inner_from=len(bp) - nfl - 1)
    rim = bell_ax[-1]
    d_rim = unit(bell_ax[-1] - bell_ax[-2])
    T.tube(g, rim - d_rim * .010, rim + d_rim * .004, br[-1] * 1.07, col=kc, n=18, spec=sp, two=True)   # the wide rim
    thr = len(bp) - nfl - 1
    g.face(S.ring(bell_ax[thr], unit(bell_ax[thr + 1] - bell_ax[thr]), br[thr] * .97, 12), (24, 16, 10),
           ref=bell_ax[thr] - d_rim, emit=True, two=True)
    # bands: the bow's two joint rings, the bell's collar where the flare starts, the body's top band
    for x0, r0 in ((0, RB), (XB, RBELL0)):
        T.tube(g, (x0, YB + .004, 0), (x0, YB + .020, 0), r0 * 1.12 * ch, col=kc, n=14, spec=sp)
    fc = len(bp) - nfl - 2.2
    i_ = int(fc)
    T.tube(g, bell_ax[i_], bell_ax[i_] + unit(bell_ax[i_ + 1] - bell_ax[i_]) * .016, br[i_] * ch * 1.06, col=kc, n=16,
           spec=sp)
    T.tube(g, (0, YT - .05, 0), (0, YT - .038, 0), rbody(YT - .045) * ch * 1.12, col=kc, n=12, spec=sp)
    # the bow guard: a curved plate under the bow with the felt bumper at the bottom
    bot = bow_ax[4]
    T.tube(g, bot + np.array([-.03, -BOWR * ch * .2, 0]), bot + np.array([.03, -BOWR * ch * .2, 0]), BOWR * ch * .75,
           col=ROD, n=10, spec=.6)
    S.prism(g, bot + np.array([0, -BOWR * ch * .95, 0]), (0, -1, 0), .022 * kg, .012, 8, FELT, dim(FELT, .8))
    # body-to-bell braces (bright)
    for f in (.30, .62, .86):
        a_, _ = bsurf(f, 80, ch, -.004)
        bar(g, a_, (XB - RBELL0 * ch * .9, a_[1], 0), .0045 * kg, kc)
    # the pigtail loop + neck + mouthpiece (gold ligature, as on the Amati)
    lp, lr = loop_path()
    loop_r, loop_ax = AS.sweep(g, lp, [r * ch for r in lr], METAL, n=10, spec=sp)
    nd = unit(loop_ax[-1] - loop_ax[-2])
    mouthpiece(g, loop_ax[-1], nd)
    T.tube(g, loop_ax[-1] + nd * .046, loop_ax[-1] + nd * .070, .0188 * max(1.0, kg * .9), col=kc, n=10, spec=.9)
    T.tube(g, (0, YT - .02, 0), (0, YT + .004, 0), RT * 1.5 * ch, col=kc, n=12, spec=sp)   # the loop's receiver
    T.tube(g, loop_ax[-5], loop_ax[-5] + nd * .012, lr[-5] * ch * 1.25, col=kc, n=10, spec=sp)   # neck tenon band
    up_n = unit(np.array([0, 1., 0]) - nd * (nd @ np.array([0, 1., 0])))
    ok = loop_ax[-3] + up_n * (lr[-3] * ch + .006)                             # octave key pad on the neck + its lever
    keycup(g, loop_ax[-3] + up_n * lr[-3] * ch, up_n, .011 * kg, .008)
    th_, thn = bsurf(.84, 180, ch, .004)
    bar(g, ok, loop_ax[2] + np.array([0, 0, -.03]), .003 * kg, ROD)
    bar(g, loop_ax[2] + np.array([0, 0, -.03]), th_, .003 * kg, ROD)
    S.prism(g, th_ + thn * .006, thn, .012 * kg, .008, 6, kc, dim(kc, .75))   # octave thumb key
    tr, trn = bsurf(.56, 186, ch)                                               # thumb rest (black) + strap ring
    S.prism(g, tr + trn * .008, trn, .016 * kg, .016, 4, FELT, dim(FELT, .8), phase=math.pi / 4)
    sr_, srn = bsurf(.90, 200, ch)
    T.tube(g, sr_ + srn * .006, sr_ + srn * .016, .007 * kg, col=kc, n=8)
    # ---- the long rods: three down the left side on posts (the stack's hinge rods), one up the right
    for phi, f0, f1 in ((-84, .04, .93), (-100, .10, .86), (-70, .32, .90), (74, .22, .70)):
        a_, _ = bsurf(f0, phi, ch, .010 * kg)
        b_, _ = bsurf(f1, phi, ch, .010 * kg)
        bar(g, a_, b_, .0042 * kg, ROD)
        for f in np.linspace(f0, f1, max(2, int((f1 - f0) / .14) + 1)):           # posts holding it
            q, qn = bsurf(f, phi, ch, .004)
            S.prism(g, q + qn * .003, qn, .0062 * kg, .010, 6, kc, dim(kc, .75))
    # ---- the main stack: six deep cups on the front-left, each on an arm from the rod, pearls on the front-right
    MAIN = [(.855, .0165), (.79, .0175), (.725, .0185), (.525, .021), (.46, .022), (.395, .023)]
    for j, (f, cr) in enumerate(MAIN):
        pt, nrm = bsurf(f, -30, ch)
        cen = keycup(g, pt, nrm, cr * kc_, .013 * kg)
        hinge, _ = bsurf(f + .012, -84, ch, .010 * kg)
        bar(g, cen + (hinge - cen) * .15, hinge, .0034 * kg, kc)                 # the key arm back to the rod
        tp_, tn_ = bsurf(f - .016, 24, ch, .020 * kg)                            # the pearl on its post
        post, _ = bsurf(f - .016, 24, ch, .002)
        bar(g, post, tp_, .0045 * kg, kc)
        S.prism(g, tp_ + tn_ * .004, tn_, .0175 * kg, .009, 10, PEARL, dim(PEARL, .74))
        S.prism(g, tp_ - tn_ * .002, tn_, .0205 * kg, .004, 10, kc, dim(kc, .7))     # its gold collar
    for pf in (.90, .935, .97):                                                 # palm keys + their bright touches
        pt, nrm = bsurf(pf, 108, ch)
        keycup(g, pt, nrm, .012 * kc_, .009 * kg)
        tp_, tn_ = bsurf(pf - .02, 64, ch, .012 * kg)
        S.prism(g, tp_, tn_, .009 * kg, .006, 6, kc, dim(kc, .75))
    # ---- side keys between body and bell (big cups facing the bell, under their own guard)
    for f, cr in ((.66, .017), (.60, .018), (.36, .021), (.30, .022)):
        pt, nrm = bsurf(f, 58, ch)
        keycup(g, pt, nrm, cr * kc_, .012 * kg)
    for f0, f1 in ((.57, .69), (.27, .39)):                                     # the side keys' guards
        a_, an = bsurf(f0, 40, ch, .020 * kg)
        b_, bn = bsurf(f1, 40, ch, .020 * kg)
        bar(g, a_, b_, .0034 * kg, ROD)
        for q, qn in ((a_, an), (b_, bn)):
            diamond(g, q - qn * .012 * kg, qn, .013 * kg)
    # ---- the pinky tables: LH (front, mid-body), RH (front, low)
    for f0, phi, n_ in ((.62, 34, 3), (.30, 30, 2)):
        for k in range(n_):
            q, qn = bsurf(f0 - .03 * k, phi, ch, .008 * kg)
            S.prism(g, q, qn, .0085 * kg, .007, 4, kc, dim(kc, .72), phase=math.pi / 4)
    # ---- the low keys: Eb + C cups low on the front under a big wire guard with diamond feet
    for f, phi, r in ((.215, -4, .025), (.12, -28, .027)):
        pt, nrm = bsurf(f, phi, ch)
        keycup(g, pt, nrm, r * kc_, .013 * kg)
    gp = [bsurf(.27, -46, ch, .022 * kg), bsurf(.07, -46, ch, .024 * kg), bsurf(.04, -12, ch, .026 * kg),
          bsurf(.07, 22, ch, .024 * kg), bsurf(.27, 22, ch, .022 * kg)]
    for (a_, _), (b_, _) in zip(gp, gp[1:]):
        bar(g, a_, b_, .0036 * kg, ROD)
    for q, qn in (gp[0], gp[-1]):
        diamond(g, q - qn * .016 * kg, qn, .014 * kg)
    # LOW A: the big cup on the back-left by the bow, its rod up to the thumb
    la, lan = bsurf(.05, -122, ch)
    keycup(g, la, lan, .030 * kc_, .014 * kg)
    a_, _ = bsurf(.07, -108, ch, .012 * kg)
    b_, _ = bsurf(.84, -108, ch, .012 * kg)
    bar(g, a_, b_, .0034 * kg, ROD)
    # ---- the bell keys: low B / Bb cups on the bell's front, each under a guard with diamond feet
    bkd = unit((-.55, 0, .83))
    for j in range(2):
        fi = 1.2 + 1.7 * j
        ax_ = bell_ax[0] * (1 - fi / 6) + bell_ax[6] * (fi / 6)
        r_ = lerp(RBELL0, .060, fi / 6) * ch
        pt = ax_ + bkd * r_
        cen = keycup(g, pt, bkd, (.028 - .002 * j) * kc_, .014 * kg)
        side = unit(np.cross(bkd, [0, 1, 0]))
        feet = [pt + side * .040 * kg * s_ + np.array([0, .045 * kg * u_, 0]) for s_, u_ in ((-1, -1), (-1, 1), (1, 1), (1, -1))]
        arc = [q + bkd * .030 * kg for q in feet]
        for a_, b_ in zip(arc, arc[1:]):
            bar(g, a_, b_, .0034 * kg, ROD)
        for q, a_ in ((feet[0], arc[0]), (feet[-1], arc[-1])):
            bar(g, q, a_, .0034 * kg, ROD)
            diamond(g, q, bkd, .014 * kg)
    # ---- the engraving: two vines of darker leaves up the bell's front (faces, so they survive the sprites)
    kf = AS.k_facing(bell_r, bell_ax, len(bp) - 4, (.2, 0, 1))
    eng = dim(METAL, .58)
    nb = len(bell_ax)
    for v_ in (-1.0, 1.0):
        for j in range(7):
            u = j / 6
            fi = lerp(nb - 7.5, nb - 1.6, u)
            fk = kf + v_ * .7 + .55 * math.sin(u * 7 + v_)
            i0_ = int(fi)
            c_ = bell_ax[i0_] * (1 - fi % 1) + bell_ax[min(nb - 1, i0_ + 1)] * (fi % 1)
            q = AS.rsurf(bell_r, bell_ax, fi, fk, lift=1.006)
            qn = unit(q - c_)
            size = (.010 + .008 * math.sin(u * 3.1)) * (1 + (ch - 1) * .5)
            S.prism(g, q, qn, size, .0012, 4 if j % 2 else 6, eng, eng, phase=.4 * v_ + j)
    return rim


def rot_y(a):
    c, s_ = math.cos(a), math.sin(a)
    return np.array([[c, 0, s_], [0, 1, 0], [-s_, 0, c]])


def rot_z(a):
    c, s_ = math.cos(a), math.sin(a)
    return np.array([[c, -s_, 0], [s_, c, 0], [0, 0, 1]])


POSE = rot_z(math.radians(-14)) @ rot_y(math.radians(18))    # stood a little off vertical, bell toward the camera


def xform(g, f0, l0, R=None, t=(0, 0, 0)):
    t = np.asarray(t, float)
    for i in range(f0, len(g.F)):
        P_, n, col, two, emit, gid, spec = g.F[i]
        if R is not None:
            P_ = P_ @ R.T
            n = R @ n
        g.F[i] = (P_ + t, n, col, two, emit, gid, spec)
    for i in range(l0, len(g.L)):
        a, b, col, gid = g.L[i]
        if R is not None:
            a, b = R @ a, R @ b
        g.L[i] = (a + t, b + t, col, gid)


def build(g, t, ev, fx, s):
    f0, l0 = len(g.F), len(g.L)
    rim = bari(g, s.get("chunk", 1.0))
    cen = np.array([0.0, .55, 0])
    o = np.array([0, .62, 0]) - POSE @ cen
    xform(g, f0, l0, R=POSE, t=o)
    rim = POSE @ rim + o
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9:
            fx.append(("note", tuple(rim + [.08, .12, 0]), age))
        if tp and age < .6:
            fx.append(("wave", tuple(rim + [.02, .02, 0]), age, 0))


# ---------------------------------------------------------------- sound: the bari's honk
# The alto's recipe (altosax60_lowpoly.sax_tone: harmonic stack + formant, breath, scoop, growl, slap) rebuilt for the
# bari's range (concert C2 = written low A up to about A4): the alto rolls its harmonics off by number, which at 65 Hz
# leaves only a dull hum, so here they roll off by frequency, through the bari's own colour: a low, wide formant near
# 560 Hz and a second bump near 1.35 kHz (the alto's 1.1 kHz moved down), a strong low end (the big conical bore), a
# heavier breath and a bite on the attack. `honk` (0..1) adds the growl, the scoop and the slap of a hard-blown low
# note: the window-rattling honk.
SR = AS.SR
FORMANTS = ((560., 300., 1.7), (1350., 520., .8), (2600., 800., .2))          # (Hz, width, gain)


def _midi_hz(m):
    return 440.0 * 2 ** ((m - 69) / 12.0)


def bari_tone(f, dur, v, honk=0.0, bright=.6):
    n = int((dur + .16) * SR)
    t = np.arange(n) / SR
    rel = .035 if dur < .15 else .06
    env = np.minimum(1, t / .016) ** 1.3 * np.where(t < dur, 1, np.exp(-(t - dur) / rel)) * (1 + .2 * np.exp(-t / .04))
    fr = f * (1 - (.035 + .04 * honk) * np.exp(-t / .045)) * (1 + .003 * (1 - honk) * np.sin(2 * np.pi * 4.8 * t) *
                                                             np.clip((t - .2) / .3, 0, 1))
    ph = 2 * np.pi * np.cumsum(fr) / SR
    x = np.zeros(n)
    tot = 0.0
    br = bright + .25 * honk
    for k in range(1, 90):
        fk = f * k
        if fk > 8000:
            break
        a = k ** -.7 * (.3 + sum(g * math.exp(-((fk - c) / w) ** 2) for c, w, g in FORMANTS))
        a *= math.exp(-fk / (1300 + 3200 * br * v))
        x += a * np.sin(k * ph + .4 * k)
        tot += a
    x /= tot + 1e-9
    gr = .05 + .22 * honk                                                       # growl: a rough flutter on the honk
    x *= 1 + gr * np.sin(2 * np.pi * 29 * t + 3 * np.sin(2 * np.pi * 7 * t))
    rng = np.random.default_rng(int(f * 10) % 100003)
    from scipy.signal import butter, sosfilt
    nz = sosfilt(butter(2, [500, 3200], 'band', fs=SR, output='sos'), rng.standard_normal(n))
    x += nz * (.05 + .05 * honk) * (.5 + 1.6 * np.exp(-t * 18))
    if honk:                                                                   # the slap of the reed on a hard attack
        k = int(.03 * SR)
        tt = t[:k]
        x[:k] += .5 * honk * (np.sin(2 * np.pi * 150 * tt) * np.exp(-tt * 100) + nz[:k] * np.exp(-tt * 260) * 2)
    return x * env * v


def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]
    return bari_tone(_midi_hz(m), dur, v)

if __name__ == "__main__":
    import piano_lowpoly as P
    for sc in SCENES:
        g = P.Geo()
        build(g, 0.0, [], [], sc)
        V = np.concatenate([f[0] for f in g.F])
        print(sc["key"], len(g.F), "faces", len(g.L), "lines, extent (m):", np.round(V.max(0) - V.min(0), 3))
