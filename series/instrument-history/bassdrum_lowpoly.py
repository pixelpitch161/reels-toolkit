"""Pixel Pitch: the marching bass drum (with its felt mallet) - low-poly model. No history video yet: built for BALLFIGHT
#56 (Bass Drum vs Tuba) and used read-only through projects/which_instrument_wins/pp_bridge.py ('BASS DRUM'), the same
way the other instruments' modern showcase models are. A later Pixel Pitch drum video can grow from this file (add SCENES
rows, backdrop, music) without touching the model.

    python3 bassdrum_lowpoly.py             # face count + extent per scene

Reference photos (Claude in Chrome, 2026-10-31, Wikimedia Commons): "Camden Muster bass drum.jpg" (a rope-tensioned
field bass drum standing on its rim on grass: how a bass drum reads as a WHEEL - a short wide cylinder, the white head
nearly as wide as the whole drum, the shell a band round it, dark hoops at both edges), and the concert / marching drums
in the Commons search "concert bass drum tension rods" (a dark-shelled concert drum on its cradle, a wood-shelled one
tilted on a stand, a red marching drum on a bench): rod-tensioned drums - straight chrome T-rods running parallel to the
axis from a claw on each hoop to a lug on the shell, evenly spaced round it, the heads plain white. Measured there:
  - depth ~0.45-0.5 of the diameter (marching 28 x 14 in, concert 36 x 18 in); a marching 28 x 14 in drum here;
  - hoops a touch wider than the shell (~1.04 x) and ~0.035 D deep, dark (black / stained) with a bright edge;
  - 10-12 rods a side on a marching drum (16-20 on a concert drum): 10 here, the two sides offset half a step so each
    lug sits between the other side's (as on the photographed drums), lugs ~0.15 D in from each hoop;
  - the felt mallet: a wooden shaft ~1.2 D long and a round felt head ~0.14 D across.
Finish: a gunmetal-lacquered shell (BALLFIGHT's BASS DRUM colour), chrome hardware, plain white heads - no logos or
lettering anywhere (as the brief asks). Local model frame: y up, +z toward the camera (the batter head faces it), the
drum's axis along z; the mallet leans against the right side of the shell.
"""
import math
import sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import trumpet_lowpoly as T                          # read-only: tube()
from piano_lowpoly import mix                        # read-only

OUT, TAG = "bassdrum_lowpoly.mp4", "bd"
BPM = 126
SHOW_BARS = 2
HEAD_INTRO = "THE BASS DRUM"

_NOTES = [(0, 38, .5, .95), (1, 38, .25, .7), (1.5, 38, .25, .7), (2, 38, .5, .95), (3, 14, 1, .8)]
SCENES = [
    dict(key="modern", theme="intro", name="BASS DRUM", credit="MARCHING 28 x 14 IN", notes=_NOTES),
    # the same drum for small sprites (BALLFIGHT balls): the rods, lugs, hoops and mallet thickened x `chunk`
    dict(key="ball", theme="intro", name="BASS DRUM", credit="MARCHING 28 x 14 IN", notes=_NOTES, chunk=2.2),
]
KICK_KINDS, ACCENT_KINDS = (), ()


def drum_events(c):
    return []


# ---------------------------------------------------------------- measurements (metres)
D = .711                       # 28 in
RAD = D / 2
DEPTH = .356                   # 14 in
HOOP_R, HOOP_W = RAD * 1.04, .035 * D
N_SIDE = 16                    # the shell's facets
N_RODS = 10                    # tension rods a side
LUG_IN = .15 * D               # lug distance in from each hoop

SHELL = (128, 128, 140)        # gunmetal lacquer (BALLFIGHT: BASS DRUM)
SHELL_HI = (160, 160, 174)     # the facet catching the key light (value contrast for small sprites)
HEAD = (238, 236, 228)         # plain white heads
HEAD_BACK = (214, 212, 204)
HOOP = (38, 38, 46)            # black hoops
HOOP_EDGE = (206, 208, 218)    # their bright inner edge
CHROME = (214, 218, 228)       # rods, claws, lugs
LUG = (188, 192, 204)
FELT = (232, 226, 210)         # the mallet's felt head (cream, apart from the head's cool white)
SHAFT = (200, 150, 92)         # its wooden shaft


def _ring(c, r, n, z, phase=0.):
    return [np.array([c[0] + r * math.cos(2 * math.pi * k / n + phase), c[1] + r * math.sin(2 * math.pi * k / n + phase), z])
            for k in range(n)]


def drum(g, hit=0.0, chunk=1.0, mallet=True):
    """the drum, centred on (0, RAD * 1.05, 0), its axis along z. Returns the batter head's centre"""
    k = chunk
    hw = HOOP_W * min(1.8, k)                      # the hoop band (wider in a sprite so it reads)
    c = (0.0, HOOP_R + .002, 0.0)
    z0, z1 = -DEPTH / 2, DEPTH / 2
    ph = math.pi / N_SIDE
    # the shell: N_SIDE flat facets, the top-left ones a shade lighter (low-poly flat shading reads better with it)
    A, B = _ring(c, RAD, N_SIDE, z0, ph), _ring(c, RAD, N_SIDE, z1, ph)
    for i in range(N_SIDE):
        j = (i + 1) % N_SIDE
        mid = (A[i] + A[j]) / 2
        hi = .5 + .5 * math.cos(math.atan2(mid[1] - c[1], mid[0] - c[0]) - math.radians(125))
        g.face([A[i], A[j], B[j], B[i]], mix(SHELL, SHELL_HI, .55 * hi), ref=np.array([c[0], c[1], 0.]), spec=.55)
    # the heads, set in a hair from each hoop; the batter head lights up on a hit
    g.face(_ring(c, RAD * .985, N_SIDE, z1 + .004, ph), mix(HEAD, (255, 250, 222), min(1.0, max(0.0, hit)) * .8),
           ref=np.array([c[0], c[1], 0.]), emit=hit > .15)
    g.face(_ring(c, RAD * .985, N_SIDE, z0 - .004, ph), HEAD_BACK, ref=np.array([c[0], c[1], 0.]))
    # the hoops: a dark band a touch wider than the shell at each edge, with a bright inner lip
    for zc, sg in ((z1, 1), (z0, -1)):
        T.tube(g, (c[0], c[1], zc - sg * hw * .7), (c[0], c[1], zc + sg * hw * .3), HOOP_R, col=HOOP,
               n=N_SIDE, spec=.35)
        lip = _ring(c, HOOP_R * 1.002, N_SIDE, zc + sg * hw * .3, ph)
        inner = _ring(c, RAD * .94, N_SIDE, zc + sg * hw * .3, ph)
        for i in range(N_SIDE):
            j = (i + 1) % N_SIDE
            g.face([lip[i], lip[j], inner[j], inner[i]], HOOP_EDGE if (i % 2 == 0 or k > 1.2) else mix(HOOP_EDGE, HOOP, .35),
                   ref=np.array([c[0], c[1], zc - sg]), spec=.8, two=True)
    # tension rods: straight chrome T-rods from a claw on each hoop to a lug on the shell; the two sides offset half
    # a step so the lugs alternate round the drum
    rr = .0055 * k
    for zc, sg, off in ((z1, 1, 0.0), (z0, -1, math.pi / N_RODS)):
        for i in range(N_RODS):
            a = 2 * math.pi * i / N_RODS + off + .2
            ca, sa = math.cos(a), math.sin(a)
            r_rod = RAD * 1.055
            p_claw = np.array([c[0] + r_rod * ca, c[1] + r_rod * sa, zc - sg * hw * .2])
            p_lug = np.array([c[0] + r_rod * ca, c[1] + r_rod * sa, zc - sg * LUG_IN])
            T.tube(g, p_claw, p_lug, rr, col=CHROME, n=5, spec=.9)
            # the T-handle sticking out past the hoop
            T.tube(g, p_claw, p_claw + [0, 0, sg * .022], rr * 1.15, col=CHROME, n=5, spec=.9)
            # the lug: a small block on the shell
            n_ = np.array([ca, sa, 0.0])
            t_ = np.array([-sa, ca, 0.0])
            lw, lh, ld = .016 * k, .016 * k, .034
            base = np.array([c[0] + RAD * ca, c[1] + RAD * sa, zc - sg * LUG_IN])
            q = [base + t_ * sx * lw + n_ * sy * lh * 1.2 + np.array([0, 0, sz * ld / 2])
                 for sx, sy, sz in ((-1, 0, -1), (1, 0, -1), (1, 1, -1), (-1, 1, -1), (-1, 0, 1), (1, 0, 1), (1, 1, 1),
                                    (-1, 1, 1))]
            for f in ((3, 2, 6, 7), (0, 1, 2, 3), (4, 5, 6, 7), (1, 2, 6, 5), (0, 3, 7, 4)):
                g.face([q[x] for x in f], LUG, ref=base, spec=.7)
    hd = np.array([c[0], c[1], z1 + .004])
    if mallet:
        # the felt mallet leaning on the right side of the shell, its head by the batter head's rim
        tip = np.array([c[0] + RAD * .80, c[1] + RAD * .30, z1 + .09])
        grip = tip + np.array([.45, -.70, -.02]) * D * (1.2 if k <= 1 else .72) / np.linalg.norm([.45, -.70, -.02])   # (short in a
        # sprite: a full 1.2 D shaft reads as a stray line at ball size)
        T.tube(g, tip, grip, .009 * min(k, 1.8), col=SHAFT, n=6, spec=.3)
        fr = .07 * D * min(k, 1.6)
        # the felt ball: an 18-face low-poly sphere
        for lat in range(3):
            y0, y1 = math.cos(math.pi * lat / 3), math.cos(math.pi * (lat + 1) / 3)
            s0, s1 = math.sin(math.pi * lat / 3), math.sin(math.pi * (lat + 1) / 3)
            for lo in range(6):
                a0, a1 = 2 * math.pi * lo / 6, 2 * math.pi * (lo + 1) / 6
                pts = [tip + fr * np.array([s0 * math.cos(a0), y0, s0 * math.sin(a0)]),
                       tip + fr * np.array([s0 * math.cos(a1), y0, s0 * math.sin(a1)]),
                       tip + fr * np.array([s1 * math.cos(a1), y1, s1 * math.sin(a1)]),
                       tip + fr * np.array([s1 * math.cos(a0), y1, s1 * math.sin(a0)])]
                if lat == 0:
                    pts = pts[1:]
                elif lat == 2:
                    pts = pts[:3]
                g.face(pts, FELT if (lo + lat) % 2 else mix(FELT, (255, 255, 255), .2), ref=tip, spec=.1)
    return hd


def build(g, t, ev, fx, s):
    hit = 0.0
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .35:
            hit = max(hit, (1 - age / .35) * v)
    hd = drum(g, hit=hit, chunk=s.get("chunk", 1.0))
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .6:
            fx.append(("wave", tuple(hd + [0, 0, .05]), age, 0))


if __name__ == "__main__":
    import piano_lowpoly as P
    for sc in SCENES:
        g = P.Geo()
        build(g, 0.0, [], [], dict(sc, ev=[(0.0, 38, .9, .5, True, 1)]))
        V = np.concatenate([f[0] for f in g.F])
        print(sc["key"], len(g.F), "faces", len(g.L), "lines, extent (m):", np.round(V.max(0) - V.min(0), 3))
