"""Pixel Pitch: the modern alto saxophone, rebuilt (low-poly model and its voice). Built for BALLFIGHT #51 (Alto Sax vs
Tenor Sax) and used through projects/which_instrument_wins/pp_bridge.py ('ALTO SAX'). The first model,
altosax60_lowpoly.py, stays as it is (the alto history video, #26 and #29's retired SAX use it); this file only reads it
as a library (its swept tube, surface helpers, palette and sax_tone).

    python3 altosax_lowpoly.py               # face count + extent per scene

Why a new file (user, Oct 29: "remake the alto sax model or at least reference it against photos using chrome"). Checked
in Claude in Chrome, 2026-10-29, against two product shots on Wikimedia Commons (both CC BY-SA 4.0):
  "4 Saxophone Amati white background.jpg" (Amati Kraslice soprano / alto / tenor / bari side by side at one scale: the
      same family as the bari's reference) and
  "Saksofon altowy Serie III GP firmy Selmer.jpg" (a Selmer Series III alto, three-quarter front).
What the old model got wrong, measured on those photos (heights from the bottom of the bow to the top of the neck):
  - the bell: too small and too high and far from the body. Real: the rim's centre sits at ~45 % of the height, ~17 % of
    the height across, close beside the body, the flare opening hard right at the end and facing up and forward;
  - the neck: too tall and too long (old: rises ~14 %, reaches ~30 % out). Real: it rises ~11 % above the body, turns
    through a short round bend and the mouthpiece tip ends ~27 % out;
  - the keywork: in the photos the keys are what make a sax read as a sax - a column of white pearls, deep cups standing
    proud of the tube on both sides, rods and guards running the full height, big guarded low keys on the bell and the
    bow. The old model draws rods, guards and levers as lines and its cups gold-on-gold, so at ball size (the BALLFIGHT
    sprites drop lines) it reads as a bare wire U;
  - at ball size the thin tube vanishes: the `ball` scene fattens every tube x`chunk` (as the bari's does).
So the alto here is rebuilt on the bari v2's construction (barisax_lowpoly.py: everything as faces, keys placed on the
real scaled surface, dark pad rings, pearls, bright rods with posts, guards with diamond feet, bands, engraving) with the
alto's own measured proportions. `sax()` takes a dimension table, so the tenor (tenorsax_lowpoly.py) shares it.
Local model frame (as the alto / bari): y up, +x toward the bell, +z toward the camera (the pearl side).
"""
import math
import sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_shapes as S
import trumpet_lowpoly as T
import altosax60_lowpoly as AS                       # read-only: sweep, rsurf, k_facing, sax_tone, palette
from pp_framework import lerp, dim

OUT, TAG = "altosax_lowpoly.mp4", "asx"
BPM = 118
SHOW_BARS = 2
HEAD_INTRO = "THE ALTO SAX"
unit = AS.unit

_NOTES = [(0, 61, .5, .85), (.5, 64, .5, .85), (1, 68, 1, .85), (2, 66, .5, .85), (2.5, 64, .5, .85), (3, 61, 1, .85)]
SCENES = [
    dict(key="modern", theme="intro", name="ALTO SAXOPHONE", credit="E-FLAT", notes=_NOTES),
    # the same alto for small sprites (BALLFIGHT balls): true heights, every tube's radius x `chunk`, the keys grown
    dict(key="ball", theme="intro", name="ALTO SAXOPHONE", credit="E-FLAT", notes=_NOTES, chunk=1.45),
]
KICK_KINDS, ACCENT_KINDS = (), ()


def drum_events(c):
    return []


# ---------------------------------------------------------------- palette (gold lacquer, like the Selmer / Amati shots)
METAL = (232, 182, 70)             # the lacquered tube
KEYC = (255, 214, 110)             # key cups, arms, rims, bands: the bright gold
ROD = (168, 120, 40)               # rods, guards, posts: a deeper gold that reads as a line on the body
PAD = (58, 34, 20)                 # leather pads: the dark ring round every cup
PEARL = (250, 246, 236)
FELT = (34, 30, 34)

# ---------------------------------------------------------------- the alto's measured proportions (metres)
# YB / YT: bow joint / neck socket heights; BOWR: the bow's radius; RB / RT: body radius at the bow / socket; RBELL0: the
# bell tube at the bow joint; bell: the straight run (to `y_run`, radius `r_run`) then the flare to the rim (centre
# `rim`, radius `rim_r`) via the control point `ctrl`; neck: the crook from the socket (offsets from (0, YT, 0));
# mp: mouthpiece length / radius; key: key size scale; low_keys: the bell keys' guard size.
ALTO = dict(
    YB=.105, YT=.565, BOWR=.047, RB=.027, RT=.0125, RBELL0=.031,
    y_run=.205, r_run=.037, ctrl=(.112, .262, .008), rim=(.122, .300, .026), rim_r=.058, flare_pow=2.4, rim_tilt=None,
    neck=[(0, 0, 0), (0, .024, 0), (-.003, .046, 0), (-.013, .064, 0), (-.031, .076, 0), (-.054, .081, 0),
          (-.076, .080, 0), (-.094, .077, 0)],
    neck_r=(.0122, .0088), mp=(.084, .0112), key=1.0,
    metal=METAL, keyc=KEYC, rod=ROD,
)


def rbody(D, y):
    return D['RB'] + (D['RT'] - D['RB']) * (y - D['YB']) / (D['YT'] - D['YB'])


def bsurf(D, f, phi, ch, lift=0.):
    """point + outward normal on the (scaled) body at fraction f (0 = bow joint, 1 = socket), azimuth phi (0 = front +z,
    90 = bell side +x), `lift` beyond the surface"""
    y = lerp(D['YB'], D['YT'], f)
    a = math.radians(phi)
    n = np.array([math.sin(a), 0, math.cos(a)])
    return np.array([0, y, 0]) + n * (rbody(D, y) * ch + lift), n


def bell_path(D):
    """(points, radii): up from the bow joint beside the body (the straight run), then the flare to the rim"""
    XB = 2 * D['BOWR']
    pts, rad = [], []
    for k in range(6):
        u = k / 5
        pts.append(np.array([XB + .006 * u, lerp(D['YB'], D['y_run'], u), 0]))
        rad.append(lerp(D['RBELL0'], D['r_run'], u))
    p0, pc, p1 = pts[-1], np.array(D['ctrl'], float), np.array(D['rim'], float)
    for k in range(1, 9):
        u = 1 - (1 - k / 8) ** 1.5
        pts.append((1 - u) ** 2 * p0 + 2 * (1 - u) * u * pc + u * u * p1)
        rad.append(D['r_run'] + (D['rim_r'] - D['r_run']) * (k / 8) ** D['flare_pow'])
    return pts, rad


def keycup(g, p, n, r, h=.008, top=KEYC, pad=True):
    """a key cup standing on the surface at p (normal n): a dark pad disc a little wider than the cup (a dark ring round
    the rim), the bright cup, its dimple. Returns the cup's top centre."""
    p, n = np.asarray(p, float), unit(n)
    hp = h * .45
    if pad:
        S.prism(g, p + n * hp / 2, n, r * 1.22, hp, 10, PAD, dim(PAD, .8))
    S.prism(g, p + n * (hp + h * .3), n, r, h * .6, 10, top, dim(top, .7))
    S.prism(g, p + n * (hp + h * .62), n, r * .55, h * .08, 8, dim(top, .86), dim(top, .7))
    return p + n * (hp + h * .65)


def bar(g, a, b, r, col):
    T.tube(g, a, b, r, col=col, n=5, spec=.85)


def diamond(g, p, n, s, col=KEYC):
    S.prism(g, p, n, s, .003, 4, col, dim(col, .7), phase=math.pi / 4)


def mouthpiece(g, p, d, L, r):
    """black mouthpiece on a cork, a bright ligature, the reed underneath"""
    p, d = np.asarray(p, float), unit(d)
    dn = unit(np.cross(d, [0, 0, 1.]))
    if dn[1] > 0:
        dn = -dn
    T.tube(g, p - d * .004, p + d * L * .3, r * .78, col=AS.CORK, n=8, spec=0)
    T.tube(g, p - d * .006, p + d * L * .6, r * 1.03, r, col=AS.BLACK, n=10, spec=.7)
    T.tube(g, p + d * L * .6, p + d * L, r, r * .5, col=AS.BLACK, n=10, spec=.7)
    T.tube(g, p + d * L * .42, p + d * L * .62, r * 1.12, col=KEYC, n=10, spec=.9)
    AS.slab(g, p + d * L * .36 + dn * r * .84, p + d * L * 1.02 + dn * r * .48, [0, 0, 1], r * 1.05, AS.REED)
    return p + d * L


def sax(g, D, ch=1.0, neck_fn=None):
    """the saxophone of dimension table D (radii x ch for sprites). Returns (mouthpiece tip, bell rim centre)."""
    met, kc, rod = D['metal'], D['keyc'], D['rod']
    ks = D['key']
    sp = .85
    kg = 1 + (ch - 1) * .9                     # keys stand off the fattened body
    kc_ = 1 + (ch - 1) * .35                   # ... but cups widen only a little (they would merge into one blob)
    YB, YT, BOWR, RB, RT, RBELL0 = D['YB'], D['YT'], D['BOWR'], D['RB'], D['RT'], D['RBELL0']
    XB = 2 * BOWR
    surf = lambda f, phi, lift=0.: bsurf(D, f, phi, ch, lift)
    # ---- body, bow, bell
    ys = np.linspace(YB, YT, 13)
    AS.sweep(g, [(0, y, 0) for y in ys], [rbody(D, y) * ch for y in ys], met, n=12, spec=sp)
    ang = np.linspace(math.pi, 2 * math.pi, 9)
    bow_r, bow_ax = AS.sweep(g, [(BOWR + BOWR * math.cos(a), YB + BOWR * math.sin(a), 0) for a in ang],
                             [r * ch for r in np.linspace(RB, RBELL0, 9)], met, n=12, spec=sp)
    bp, br = bell_path(D)
    nfl = 6
    brs = [r * (ch if r < D['r_run'] * 1.25 else 1 + (ch - 1) * .3) for r in br]   # the rim stays its true size
    bell_r, bell_ax = AS.sweep(g, bp, brs, met, n=16, spec=sp, inner=dim(met, .6), inner_from=len(bp) - nfl - 1)
    rim = bell_ax[-1]
    d_rim = unit(bell_ax[-1] - bell_ax[-2])
    T.tube(g, rim - d_rim * .008, rim + d_rim * .003, brs[-1] * 1.07, col=kc, n=18, spec=sp, two=True)   # rim
    thr = len(bp) - nfl - 1
    g.face(S.ring(bell_ax[thr], unit(bell_ax[thr + 1] - bell_ax[thr]), brs[thr] * .97, 12), (24, 16, 10),
           ref=bell_ax[thr] - d_rim, emit=True, two=True)
    # bands: the bow's two joint rings, the bell's collar where the flare starts, the body's top band
    for x0, r0 in ((0, RB), (XB, RBELL0)):
        T.tube(g, (x0, YB + .003, 0), (x0, YB + .015, 0), r0 * 1.12 * ch, col=kc, n=12, spec=sp)
    i_ = len(bp) - nfl - 2
    T.tube(g, bell_ax[i_], bell_ax[i_] + unit(bell_ax[i_ + 1] - bell_ax[i_]) * .011, brs[i_] * 1.07, col=kc, n=14,
           spec=sp)
    T.tube(g, (0, YT - .016, 0), (0, YT + .002, 0), RT * 1.45 * ch, col=kc, n=10, spec=sp)   # neck receiver
    # bow guard + felt bumper under the bow
    bot = bow_ax[4]
    T.tube(g, bot + np.array([-.022, -BOWR * ch * .12, 0]), bot + np.array([.022, -BOWR * ch * .12, 0]),
           RB * ch * 1.18, col=rod, n=10, spec=.6)
    S.prism(g, bot + np.array([0, -RB * ch * 1.15, 0]), (0, -1, 0), .015 * kg * ks, .009, 8, FELT, dim(FELT, .8))
    # body-to-bell braces
    for f in (.26, .50):
        a_, _ = surf(f, 82, -.003)
        y_ = a_[1]
        j = int(np.argmin([abs(p[1] - y_) for p in bp[:6]]))
        bar(g, a_, (bp[j][0] - brs[j] * .9, y_, 0), .0034 * kg * ks, kc)
    # ---- neck + mouthpiece (+ octave key on the neck and its lever down the back)
    npts = [np.array([0, YT, 0]) + np.array(q) for q in D['neck']]
    r0, r1 = D['neck_r']
    nr = [lerp(r0, r1, i / (len(npts) - 1)) * ch for i in range(len(npts))]
    neck_r, neck_ax = AS.sweep(g, npts, nr, met, n=10, spec=sp)
    nd = unit(neck_ax[-1] - neck_ax[-2])
    tip = mouthpiece(g, neck_ax[-1], nd, D['mp'][0], D['mp'][1] * max(1.0, 1 + (ch - 1) * .5))
    T.tube(g, neck_ax[-2], neck_ax[-2] + unit(neck_ax[-1] - neck_ax[-2]) * .007, nr[-2] * 1.25, col=kc, n=10, spec=sp)
    ki = len(neck_ax) - 3
    tg_ = unit(neck_ax[ki + 1] - neck_ax[ki - 1])
    up_n = unit(np.array([0, 1., 0]) - tg_ * (tg_ @ np.array([0, 1., 0])))
    keycup(g, neck_ax[ki] + up_n * nr[ki], up_n, .0085 * kc_ * ks, .006 * kg)
    th_, thn = surf(.83, 180, .003)
    lv0 = neck_ax[ki] + up_n * (nr[ki] + .004)
    lv1 = neck_ax[1] + np.array([0, 0, -nr[1] - .007])
    bar(g, lv0, lv1, .0024 * kg * ks, rod)
    bar(g, lv1, th_, .0024 * kg * ks, rod)
    S.prism(g, th_ + thn * .004, thn, .0095 * kg * ks, .006, 6, kc, dim(kc, .75))    # octave thumb key
    tr, trn = surf(.52, 186)                                                          # thumb rest + strap ring
    S.prism(g, tr + trn * .006, trn, .011 * kg * ks, .011, 4, FELT, dim(FELT, .8), phase=math.pi / 4)
    sr_, srn = surf(.74, 196)
    T.tube(g, sr_ + srn * .004, sr_ + srn * .012, .0052 * kg * ks, col=kc, n=8)
    # ---- the long rods on posts: three down the left of the stack, one up the right
    for phi, f0, f1 in ((-84, .05, .95), (-100, .12, .86), (-68, .34, .92), (74, .22, .72)):
        a_, _ = surf(f0, phi, .008 * kg * ks)
        b_, _ = surf(f1, phi, .008 * kg * ks)
        bar(g, a_, b_, .0031 * kg * ks, rod)
        for f in np.linspace(f0, f1, max(2, int((f1 - f0) / .15) + 1)):
            q, qn = surf(f, phi, .003)
            S.prism(g, q + qn * .002, qn, .0046 * kg * ks, .008, 6, kc, dim(kc, .75))
    # ---- the main stack: six cups front-left on arms from the rod, pearls front-right on posts
    MAIN = [(.86, .0118), (.795, .0124), (.73, .013), (.53, .0145), (.465, .015), (.40, .0156)]
    for f, cr in MAIN:
        pt, nrm = surf(f, -30)
        cen = keycup(g, pt, nrm, cr * kc_ * ks, .010 * kg * ks)
        hinge, _ = surf(f + .012, -84, .008 * kg * ks)
        bar(g, cen + (hinge - cen) * .15, hinge, .0026 * kg * ks, kc)
        tp_, tn_ = surf(f - .016, 24, .016 * kg * ks)
        post, _ = surf(f - .016, 24, .001)
        bar(g, post, tp_, .0034 * kg * ks, kc)
        S.prism(g, tp_ + tn_ * .003, tn_, .0128 * kg * ks, .007, 10, PEARL, dim(PEARL, .74))
        S.prism(g, tp_ - tn_ * .0015, tn_, .0152 * kg * ks, .003, 10, kc, dim(kc, .7))
    for pf in (.90, .935, .97):                                                 # palm keys + their bright touches
        pt, nrm = surf(pf, 108)
        keycup(g, pt, nrm, .0088 * kc_ * ks, .007 * kg * ks)
        tp_, tn_ = surf(pf - .02, 62, .010 * kg * ks)
        S.prism(g, tp_, tn_, .0068 * kg * ks, .005, 6, kc, dim(kc, .75))
    p_, n_ = surf(.86, 16, .009 * kg * ks)                                      # front F (a small pearl)
    S.prism(g, p_, n_, .0058 * kg * ks, .004, 6, PEARL, dim(PEARL, .74))
    # ---- side keys facing the bell, under their own guards
    for f, cr in ((.66, .0118), (.60, .0124), (.36, .0142), (.30, .0148)):
        pt, nrm = surf(f, 58)
        keycup(g, pt, nrm, cr * kc_ * ks, .009 * kg * ks)
    for f0, f1 in ((.57, .69), (.27, .39)):
        a_, an = surf(f0, 40, .016 * kg * ks)
        b_, bn = surf(f1, 40, .016 * kg * ks)
        bar(g, a_, b_, .0026 * kg * ks, rod)
        for q, qn in ((a_, an), (b_, bn)):
            diamond(g, q - qn * .009 * kg * ks, qn, .0095 * kg * ks, kc)
    # ---- the pinky tables: LH (front, mid-body), RH (front, low)
    for f0, phi, n_k in ((.62, 34, 3), (.30, 30, 2)):
        for k in range(n_k):
            q, qn = surf(f0 - .032 * k, phi, .007 * kg * ks)
            S.prism(g, q, qn, .0064 * kg * ks, .005, 4, kc, dim(kc, .72), phase=math.pi / 4)
    # ---- low Eb + C cups low on the front under a big wire guard with diamond feet
    for f, phi, r in ((.215, -4, .0175), (.12, -28, .019)):
        pt, nrm = surf(f, phi)
        keycup(g, pt, nrm, r * kc_ * ks, .010 * kg * ks)
    gp = [surf(.27, -46, .017 * kg * ks), surf(.07, -46, .019 * kg * ks), surf(.04, -12, .021 * kg * ks),
          surf(.07, 22, .019 * kg * ks), surf(.27, 22, .017 * kg * ks)]
    for (a_, _), (b_, _) in zip(gp, gp[1:]):
        bar(g, a_, b_, .0027 * kg * ks, rod)
    for q, qn in (gp[0], gp[-1]):
        diamond(g, q - qn * .012 * kg * ks, qn, .010 * kg * ks, kc)
    # ---- low C# on the bow (front of the U)
    q_ = AS.rsurf(bow_r, bow_ax, 5.4, AS.k_facing(bow_r, bow_ax, 5, (-.3, -.2, 1)), 1.0)
    keycup(g, q_, unit(q_ - bow_ax[5]), .016 * kc_ * ks, .010 * kg * ks)
    # ---- the bell keys: low B / Bb cups on the bell's front, each under a guard with diamond feet
    bkd = unit((-.55, 0, .83))
    for j in range(2):
        fi = 1.0 + 1.6 * j
        ax_ = bell_ax[0] * (1 - fi / 5) + bell_ax[5] * (fi / 5)
        r_ = lerp(RBELL0, D['r_run'], fi / 5) * ch
        pt = ax_ + bkd * r_
        keycup(g, pt, bkd, (.019 - .0015 * j) * kc_ * ks, .010 * kg * ks)
        side = unit(np.cross(bkd, [0, 1, 0]))
        feet = [pt + side * .028 * kg * ks * s_ + np.array([0, .03 * kg * ks * u_, 0])
                for s_, u_ in ((-1, -1), (-1, 1), (1, 1), (1, -1))]
        arc = [q + bkd * .022 * kg * ks for q in feet]
        for a_, b_ in zip(arc, arc[1:]):
            bar(g, a_, b_, .0026 * kg * ks, rod)
        for q, a_ in ((feet[0], arc[0]), (feet[-1], arc[-1])):
            bar(g, q, a_, .0026 * kg * ks, rod)
            diamond(g, q, bkd, .010 * kg * ks, kc)
    # ---- engraving: two vines of darker leaves up the bell's front (faces, so they survive the sprites)
    kf = AS.k_facing(bell_r, bell_ax, len(bp) - 4, (.2, 0, 1))
    eng = dim(met, .58)
    nb = len(bell_ax)
    for v_ in (-1.0, 1.0):
        for j in range(6):
            u = j / 5
            fi = lerp(nb - 6.5, nb - 1.6, u)
            fk = kf + v_ * .8 + .55 * math.sin(u * 7 + v_)
            i0_ = int(fi)
            c_ = bell_ax[i0_] * (1 - fi % 1) + bell_ax[min(nb - 1, i0_ + 1)] * (fi % 1)
            q = AS.rsurf(bell_r, bell_ax, fi, fk, lift=1.006)
            S.prism(g, q, unit(q - c_), (.0075 + .006 * math.sin(u * 3.1)) * (1 + (ch - 1) * .5) * ks, .001,
                    4 if j % 2 else 6, eng, eng, phase=.4 * v_ + j)
    return tip, rim


def rot_y(a):
    c, s_ = math.cos(a), math.sin(a)
    return np.array([[c, 0, s_], [0, 1, 0], [-s_, 0, c]])


def rot_z(a):
    c, s_ = math.cos(a), math.sin(a)
    return np.array([[c, -s_, 0], [s_, c, 0], [0, 0, 1]])


POSE = rot_z(math.radians(-6)) @ rot_y(math.radians(16))     # a hair off vertical, the bell turned toward the camera


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


def build_model(g, D, s, pose=POSE, lift=.40):
    f0, l0 = len(g.F), len(g.L)
    tip, rim = sax(g, D, s.get("chunk", 1.0))
    cen = np.array([0.0, (D['YB'] + D['YT']) / 2, 0])
    o = np.array([0, lift, 0]) - pose @ cen
    xform(g, f0, l0, R=pose, t=o)
    return pose @ tip + o, pose @ rim + o


def build(g, t, ev, fx, s):
    tip, rim = build_model(g, ALTO, s)
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9:
            fx.append(("note", tuple(rim + [.06, .10, 0]), age))
        if tp and age < .6:
            fx.append(("wave", tuple(rim + [.02, .02, 0]), age, 0))


# ---------------------------------------------------------------- sound: the alto's bright lead
# The alto keeps the old model's voice recipe (altosax60_lowpoly.sax_tone with its modern YAS-62 tone), a shade brighter
# and without the history video's echo (a BALLFIGHT mood adds its own room): the bright, singing lead.
SR = AS.SR
TONE = dict(AS.TONES["modern"], bright=.72)


def _midi_hz(m):
    return 440.0 * 2 ** ((m - 69) / 12.0)


def alto_tone(f, dur, v, **kw):
    a = dict(TONE)
    a.update(kw)
    return AS.sax_tone(f, dur, v, **a)


def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]
    return alto_tone(_midi_hz(m), dur, v)


if __name__ == "__main__":
    import piano_lowpoly as P
    for sc in SCENES:
        g = P.Geo()
        build(g, 0.0, [], [], sc)
        V = np.concatenate([f[0] for f in g.F])
        print(sc["key"], len(g.F), "faces", len(g.L), "lines, extent (m):", np.round(V.max(0) - V.min(0), 3))
