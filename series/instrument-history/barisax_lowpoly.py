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
Local model frame (as the alto): y up, +x toward the bell, +z toward the camera (the pearl side)."""
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
    """the bari's big black mouthpiece: cork, body, ligature, reed underneath"""
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


def bari(g, ch=1.0):
    met, kc = C["metal"], C["keys"]
    sp = C.get("spec", .85)
    # body, bow, bell (the alto's swept-tube helper; radii x ch for sprites)
    ys = np.linspace(YB, YT, 11)
    AS.sweep(g, [(0, y, 0) for y in ys], [rbody(y) * ch for y in ys], met, n=12, spec=sp)
    ang = np.linspace(math.pi, 2 * math.pi, 9)
    bow_r, bow_ax = AS.sweep(g, [(BOWR + BOWR * math.cos(a), YB + BOWR * math.sin(a), 0) for a in ang],
                             [r * ch for r in np.linspace(RB, RBELL0, 9)], met, n=12, spec=sp)
    bp, br = bell_path()
    nfl = 6
    bell_r, bell_ax = AS.sweep(g, bp, [r * (ch if r < .07 else 1 + (ch - 1) * .3) for r in br], met, n=14, spec=sp,
                               inner=dim(met, .78), inner_from=len(bp) - nfl - 1)
    rim = bell_ax[-1]
    d_rim = unit(bell_ax[-1] - bell_ax[-2])
    T.tube(g, rim - d_rim * .006, rim + d_rim * .003, br[-1] * 1.04, col=met, n=16, spec=sp, two=True)
    thr = len(bp) - nfl - 1
    g.face(S.ring(bell_ax[thr], unit(bell_ax[thr + 1] - bell_ax[thr]), br[thr] * .97, 12), (26, 18, 12),
           ref=bell_ax[thr] - d_rim, emit=True, two=True)
    for x0, r0 in ((0, RB), (XB, RBELL0)):                                   # the bow's joint rings
        T.tube(g, (x0, YB + .002, 0), (x0, YB + .016, 0), r0 * 1.1 * ch, col=met, n=12, spec=sp)
    for f in (.30, .62):                                                      # body-to-bell braces
        a_, _ = bpt(f, 88, .001)
        T.tube(g, a_, (XB - .056 * ch, a_[1], 0), .0035, col=kc, n=4)
    # the pigtail loop + neck + mouthpiece
    lp, lr = loop_path()
    loop_r, loop_ax = AS.sweep(g, lp, [r * ch for r in lr], met, n=10, spec=sp)
    nd = unit(loop_ax[-1] - loop_ax[-2])
    mouthpiece(g, loop_ax[-1], nd)
    T.tube(g, (0, YT - .02, 0), (0, YT + .004, 0), RT * 1.45 * ch, col=met, n=12, spec=sp)   # the loop's receiver
    up_n = unit(np.array([0, 1., 0]) - nd * (nd @ np.array([0, 1., 0])))
    ok = loop_ax[-3] + up_n * (.016 * ch + .004)                              # octave key on the neck + its lever
    S.prism(g, ok, up_n, .007, .004, 6, kc, dim(kc, .8))
    th_, thn = bpt(.82, 180, .010)
    for a_, b_ in ((ok, loop_ax[2] + np.array([0, 0, -.025])), (loop_ax[2] + np.array([0, 0, -.025]), th_)):
        g.line(a_, b_, kc)
    S.prism(g, th_, thn, .007, .004, 6, kc, dim(kc, .8))                     # thumb rest / octave thumb key
    # main stack: six big cups on the front-left, pearl touches on the front-right
    MAIN = [(.82, .019), (.765, .020), (.71, .021), (.50, .025), (.445, .026), (.39, .027)]
    for f, cr in MAIN:
        pt, nrm = bpt(f, -26)
        cen = AS.cup(g, pt, nrm, cr * min(1.0, ch), kc, False)
        tp_, tn_ = bpt(f - .012, 24, .018 * ch)
        AS.touch(g, tp_, tn_, C, False, r=.012)
        g.line(cen, tp_, kc)
    for pf in (.88, .92, .96):                                                # palm keys
        pt, nrm = bpt(pf, 104)
        cen = AS.cup(g, pt, nrm, .013, kc, True)
        tp_, tn_ = bpt(pf - .02, 70, .012)
        AS.touch(g, tp_, tn_, C, False, .007)
        g.line(cen, tp_, kc)
    for phi, f0_, f1_ in ((-78, .30, .86), (-98, .36, .80), (76, .40, .66)):  # key rods
        a_, _ = bpt(f0_, phi, .008)
        b_, _ = bpt(f1_, phi, .008)
        T.tube(g, a_, b_, .0018, col=kc, n=4, spec=.85)
    tbl, tbn = bpt(.60, 50, .012)                                             # LH pinky table, RH pinky keys
    for k in range(3):
        S.prism(g, tbl + np.array([0, -.012 * k, 0]), tbn, .0062, .004, 4, kc, dim(kc, .8))
    rp, rpn = bpt(.30, 48, .014)
    for k in range(2):
        S.prism(g, rp + np.array([0, -.015 * k, 0]), rpn, .007, .004, 4, kc, dim(kc, .8))
    # the low keys: Eb + C under a guard, the LOW A cup by the bow with its long rod, low B / Bb on the bell front
    for (f, phi, r), sh in (((.20, 2, .026), False), ((.12, -28, .028), False)):
        pt, nrm = bpt(f, phi)
        AS.cup(g, pt, nrm, r, kc, sh)
    gp = [bpt(.26, -50, .026)[0], bpt(.08, -50, .028)[0], bpt(.05, -12, .030)[0], bpt(.08, 22, .028)[0], bpt(.26, 24, .026)[0]]
    for a_, b_ in zip(gp, gp[1:]):
        T.tube(g, a_, b_, .0022, col=kc, n=4, spec=.85)
    la, lan = bpt(.04, -120, 0)                                               # LOW A: big cup on the back-left, low down
    AS.cup(g, la, lan, .030, kc, True)
    a_, _ = bpt(.06, -105, .012)
    b_, _ = bpt(.86, -105, .012)
    T.tube(g, a_, b_, .0022, col=kc, n=4, spec=.85)                          # its long rod up to the thumb
    bkd = unit((-.62, 0, .78))
    for j in range(2):                                                        # low B / Bb cups on the bell
        fi = 1.0 + 1.6 * j
        ax_ = bell_ax[0] * (1 - fi / 6) + bell_ax[6] * (fi / 6)
        r_ = lerp(RBELL0, .060, fi / 6) * ch
        pt = ax_ + bkd * r_
        AS.cup(g, pt, bkd, .028 - .002 * j, kc, False)
        gd = [pt + bkd * .022 + np.array([0, .018, 0]) + np.cross(bkd, [0, 1, 0]) * .022 * s_ for s_ in (-1, 1)]
        g.line(gd[0], gd[1], kc)
        g.line(gd[0], gd[0] - [0, .034, 0], kc)
        g.line(gd[1], gd[1] - [0, .034, 0], kc)
    kf = AS.k_facing(bell_r, bell_ax, len(bp) - 4, (.15, 0, 1))              # a little engraving on the bell
    AS.engrave(g, bell_r, bell_ax, "floral", C.get("engr_col", dim(met, .66)), len(bp) - 5.5, len(bp) - 1.6, kf)
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
