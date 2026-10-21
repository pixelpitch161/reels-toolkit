"""Low-poly x pixel-art history of the violin -> 1080x1920 MP4 with a steady beat.

Reuses the rasterizer, pixel font, background, turntable and drum kit from piano_lowpoly.py.
Each instrument stands on the turntable; an animated bow crosses the strings, the sounding
string vibrates and a gold dot marks the finger position for every note.

usage:  python3 violin_lowpoly.py [out.mp4]       render the video
        python3 violin_lowpoly.py --still 3.1 7.5  write still_<t>.png frames
"""
import math, subprocess, sys, wave
import numpy as np
from PIL import Image, ImageDraw
from scipy.signal import lfilter
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import piano_lowpoly as P
from piano_lowpoly import (Geo, render_geo, outline, Cam, make_bg, text_w, draw_text, center_text, mix, ease,
                           ease_out_back, clamp, dim, lerp, midi_hz, sprite, NOTE_SPR, SPARK, build_pedestal,
                           kick, snare, hat, crash, bass, pad, RW, RH, UP, FPS, SR, BEAT, BAR,
                           CREAM, GOLD, GOLD_D, INK, BG_BOT, HOT, HOTSTR)

for _c, _rows in {"&": [".##..", "#..#.", "#.#..", ".#...", "#.#.#", "#..#.", ".##.#"],
                  "(": ["...#.", "..#..", ".#...", ".#...", ".#...", "..#..", "...#."],
                  ")": [".#...", "..#..", "...#.", "...#.", "...#.", "..#..", ".#..."]}.items():
    P._G.setdefault(_c, "".join(_rows))
OUT = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else "violin_lowpoly.mp4"
DIST = 3.0
FLIP = [1]

# ---------------------------------------------------------------- music + story
CH = {"D": (38, (57, 62, 66)), "Gm": (43, (55, 58, 62)), "Dm": (38, (57, 62, 65)), "C": (36, (55, 60, 64)),
      "Am": (45, (57, 60, 64)), "G": (43, (55, 59, 62)), "A": (45, (57, 61, 64)), "E": (40, (56, 59, 64))}
BARS = ["D", "D", "Gm", "Dm", "C", "Dm", "Am", "G", "D", "Dm", "A", "C", "C", "Am", "E", "E", "A"]

def seq(start, items, vel=.8):
    out, b = [], start
    for m, d in items: out.append((b, m, d, vel)); b += d
    return out

VIV = [(76, .5), (76, .5), (76, .5), (74, .25), (72, .25), (79, 1.5), (79, .25), (77, .25)]
PAG_A = [(69, .5), (69, .25), (72, .25), (71, .25), (69, .25), (76, .5)]
PAG_E = [(64, .5), (64, .25), (68, .25), (66, .25), (64, .25), (71, .5)]
up12 = lambda ph: [(m + 12, d) for m, d in ph]

SCENES = [
 dict(key="rebab", year=900, ys="900s", name="REBAB", credit="MELODY: MAQAM HIJAZ",
      fact=("ONE OF THE FIRST", "BOWED INSTRUMENTS"), yaw=-10,
      notes=seq(.5, [(69, .5), (70, .5), (69, .5), (67, .5), (66, .5), (63, .5), (62, .5),
                     (67, 1), (70, .5), (69, .5), (67, .5), (66, .5), (67, 1)])),
 dict(key="rebec", year=1100, ys="1100s", name="REBEC", credit="IN THE STYLE OF AN ESTAMPIE",
      fact=("THE REBAB REACHES", "MEDIEVAL EUROPE"), yaw=-10,
      notes=seq(0, [(74, .5), (72, .5), (74, .5), (69, .5), (65, .5), (67, .5), (69, 1),
                    (72, .5), (67, .5), (72, .5), (74, .5), (76, 1), (72, 1)])),
 dict(key="vielle", year=1300, ys="1300s", name="VIELLE", credit="IN THE STYLE OF MACHAUT",
      fact=("THE MEDIEVAL FIDDLE:", "5 STRINGS & A DRONE"), yaw=-10,
      notes=seq(0, [(62, 1), (65, .5), (64, .5), (62, .5), (64, .5), (65, 1),
                    (69, 1.5), (67, .5), (65, .5), (64, .5), (69, 1)])),
 dict(key="lira", year=1490, ys="1490s", name="LIRA DA BRACCIO", credit="IN THE STYLE OF JOSQUIN",
      fact=("LEONARDO DA VINCI", "PLAYED ONE!"), yaw=-10,
      notes=seq(0, [(67, 1), (71, 1), (74, 1.5), (72, .5), (71, 1), (69, .5), (71, .5), (69, 1), (66, 1)])),
 dict(key="amati", year=1560, ys="1560s", name="AMATI VIOLIN", credit="IN THE STYLE OF MONTEVERDI",
      fact=("ANDREA AMATI SETS", "THE VIOLIN'S SHAPE"), yaw=-10,
      notes=seq(0, [(69, .75), (70, .25), (72, .5), (74, .5), (77, 1), (76, .5), (74, .5),
                    (73, 1.5), (76, .5), (76, .5), (73, .5), (69, 1)])),
 dict(key="strad", year=1710, ys="1710s", name="STRADIVARIUS", credit="VIVALDI: SPRING (1725)",
      fact=("STRADIVARI'S GOLDEN", "ERA IN CREMONA"), yaw=-10,
      notes=seq(0, VIV + VIV, .85)),
 dict(key="modern", year=1820, ys="1820s", name="MODERN VIOLIN", credit="PAGANINI: CAPRICE NO. 24",
      fact=("LONGER NECK, CHINREST", "AND A STRONGER BOW"), yaw=-10,
      notes=seq(0, PAG_A + up12(PAG_A) + PAG_E + up12(PAG_E), .9) +
            seq(8, [(m, .25) for m in [76, 80, 83, 88, 83, 80, 76, 71, 68, 71, 76, 80, 83, 88, 83, 80]], .85) +
            [(12, [69, 73, 76, 81], 4.8, 1.0)]),
]
DRONES = {"vielle": [50], "lira": [43, 50]}
for si, s in enumerate(SCENES):
    s["t0"] = (1 + 2*si) * BAR
    s["ev"] = []
    for beat, ms, dur, v in s["notes"]:
        ml = list(ms) if isinstance(ms, (list, tuple)) else [ms]
        for m in ml: s["ev"].append((beat*BEAT, m, v, dur*BEAT, m == max(ml)))
OUTRO_T = 15 * BAR
TOTAL = 16 * BAR + 2.6

THEMES = {
 None:     P.THEMES[None],
 "rebab":  ((6, 24, 30), (18, 80, 82), (100, 180, 166)),
 "rebec":  ((32, 6, 12), (108, 24, 36), (224, 108, 112)),
 "vielle": ((8, 12, 40), (32, 48, 116), (116, 140, 224)),
 "lira":   ((24, 8, 30), (84, 30, 96), (196, 120, 210)),
 "amati":  ((4, 14, 30), (18, 56, 96), (110, 170, 224)),
 "strad":  ((4, 24, 18), (16, 84, 62), (110, 200, 158)),
 "modern": ((14, 12, 26), (70, 64, 104), (186, 180, 226)),
}
BGS = {k: make_bg(*v).astype(float) for k, v in THEMES.items()}
def ped_cols(key):
    b = THEMES[key][1]
    return dict(low_side=dim(b, .55), low_top=dim(b, .85), side=dim(b, .8), top=mix(b, (255, 255, 255), .14), shadow=dim(b, .45))

def year_str(y): return f"{y} AD" if y < 1000 else str(y)

# ---------------------------------------------------------------- geometry helpers
def slab_xy(g, pts, z0, z1, col, emit=False, spec=0.):
    c = (sum(p[0] for p in pts)/len(pts), sum(p[1] for p in pts)/len(pts), (z0 + z1)/2)
    g.face([(x, y, z1) for x, y in pts], col, ref=c, emit=emit, spec=spec)
    g.face([(x, y, z0) for x, y in pts], col, ref=c, emit=emit, spec=spec)
    for k in range(len(pts)):
        (xa, ya), (xb, yb) = pts[k], pts[(k+1) % len(pts)]
        g.face([(xa, ya, z0), (xb, yb, z0), (xb, yb, z1), (xa, ya, z1)], col, ref=c, emit=emit, spec=spec)

def decal(g, pts, z, col):
    cx = sum(p[0] for p in pts)/len(pts); cy = sum(p[1] for p in pts)/len(pts)
    g.face([(x, y, z) for x, y in pts], col, ref=(cx, cy, z - 1), emit=True)

def thin(g, a, b, w, z, col):
    d = np.array([b[0]-a[0], b[1]-a[1]]); d /= np.linalg.norm(d); n = np.array([-d[1], d[0]])*w/2
    decal(g, [tuple(np.array(a)+n), tuple(np.array(b)+n), tuple(np.array(b)-n), tuple(np.array(a)-n)], z, col)

def xcyl(g, cx, cy, cz, r, hl, col, n=8, spec=0.):
    ring = [(cy + r*math.cos(2*math.pi*k/n), cz + r*math.sin(2*math.pi*k/n)) for k in range(n)]
    c = (cx, cy, cz)
    for x in (cx - hl, cx + hl): g.face([(x, y, z) for y, z in ring], col, ref=c, spec=spec)
    for k in range(n):
        (ya, za), (yb, zb) = ring[k], ring[(k+1) % n]
        g.face([(cx-hl, ya, za), (cx+hl, ya, za), (cx+hl, yb, zb), (cx-hl, yb, zb)], col, ref=c, spec=spec)

def octo(cx, cy, rx, ry, n=8):
    return [(cx + rx*math.cos(2*math.pi*k/n + math.pi/n), cy + ry*math.sin(2*math.pi*k/n + math.pi/n)) for k in range(n)]

def body(g, prof, L, y0, d, top, side):
    pts = [(y0 + u*L, hw*L) for u, hw in prof]; zf, zb = d/2, -d/2
    for (ya, wa), (yb, wb) in zip(pts, pts[1:]):
        ref = (0, (ya + yb)/2, 0)
        g.face([(-wa, ya, zf), (wa, ya, zf), (wb, yb, zf), (-wb, yb, zf)], top, ref=ref)
        g.face([(-wa, ya, zb), (wa, ya, zb), (wb, yb, zb), (-wb, yb, zb)], dim(side, .8), ref=ref)
        g.face([(wa, ya, zb), (wb, yb, zb), (wb, yb, zf), (wa, ya, zf)], side, ref=ref)
        g.face([(-wa, ya, zb), (-wb, yb, zb), (-wb, yb, zf), (-wa, ya, zf)], side, ref=ref)
    for (y, w), sgn in ((pts[0], -1), (pts[-1], 1)):
        g.face([(-w, y, zb), (w, y, zb), (w, y, zf), (-w, y, zf)], side, ref=(0, y - sgn, 0))

def stand(g, y_top):
    metal = (44, 40, 50)
    g.ngon(0, .004, -.02, .2, metal, spec=.6)
    g.box(-.016, .016, 0, y_top, -.14, -.11, metal, spec=.6)
    g.box(-.13, .13, y_top - .03, y_top, -.14, .05, metal, spec=.6)

# ---------------------------------------------------------------- strings, bow, fx (shared)
def string_idx(strings, m):
    c = [i for i, o in enumerate(strings) if o <= m]
    return c[-1] if c else 0

def bow_track(ev):
    groups = {}
    for nt, m, v, dur, tp in ev: groups[nt] = max(groups.get(nt, 0), dur)
    segs, s, d = [], 0.0, 1
    for t0 in sorted(groups):
        dur = groups[t0]; trav = clamp(dur*.55, .12, .7); d = -d
        tgt = clamp(s + d*trav, -.42, .42)
        if abs(tgt - s) < .08: d = -d; tgt = clamp(s + d*trav, -.42, .42)
        segs.append((t0, t0 + dur, s, tgt)); s = tgt
    return segs

def bow_pos(segs, t):
    s = 0.0
    for t0, t1, s0, s1 in segs:
        if t < t0: break
        s = lerp(s0, s1, clamp((t - t0)/(t1 - t0))) if t < t1 else s1
    return s

def draw_bow(g, contact, ang, s, L, bulge, stick_col, frog_col, tipcap=(230, 226, 214)):
    d = np.array([math.cos(ang), math.sin(ang), 0.]); n = np.array([-math.sin(ang), math.cos(ang), 0.])
    c = np.array(contact, float); mid = c - s*L*d
    frog, tip = mid - d*L/2, mid + d*L/2
    hair = (238, 234, 216)
    g.line(frog, tip, hair); g.line(frog + [0, 0, .004], tip + [0, 0, .004], hair)
    pts = []
    for i in range(11):
        u = i/10; pts.append(frog + d*L*u + n*(lerp(.05, .018, u) + bulge*math.sin(math.pi*u)) + [0, 0, .01])
    for off in (0, .007):
        for i in range(10): g.line(pts[i] + n*off, pts[i+1] + n*off, stick_col)
    f = frog + n*.025 + [0, 0, .006]
    g.box(f[0]-.03, f[0]+.03, f[1]-.022, f[1]+.022, f[2]-.012, f[2]+.012, frog_col, spec=.4)
    tp = tip + n*.012
    g.box(tp[0]-.012, tp[0]+.012, tp[1]-.012, tp[1]+.012, tp[2]-.006, tp[2]+.012, tipcap)

def play(g, t, ev, fx, s, strings, TBN, contact, base_ang, bowcfg, drones_TBN=()):
    """strings: open midi per string; TBN: per string (tail, bridge, nut) points; contact: bow contact."""
    active = {}
    for age, m, v, dur, tp in ev:
        if age < dur + .05:
            i = string_idx(strings, m); active.setdefault(i, []).append((age, m))
    for i, (T_, B_, N_) in enumerate(TBN):
        g.line(T_, B_, (150, 146, 140))
        if i in active:
            B, N = np.array(B_), np.array(N_)
            pts = []
            for k in range(9):
                u = k/8; p = B*(1-u) + N*u; p[0] += .006*math.sin(math.pi*u)*FLIP[0]; pts.append(p)
            for k in range(8): g.line(pts[k], pts[k+1], HOTSTR)
            age, m = min(active[i])
            uf = 1 - 2 ** (-(m - strings[i])/12)
            if uf > .01:
                fp = N + (B - N)*uf + [0, 0, .006]
                g.box(fp[0]-.014, fp[0]+.014, fp[1]-.014, fp[1]+.014, fp[2]-.004, fp[2]+.01, HOT, emit=True)
        else:
            g.line(B_, N_, (214, 210, 200))
    for T_, B_, N_ in drones_TBN:   # drones sound for the whole scene
        B, N = np.array(B_), np.array(N_)
        g.line(T_, B_, (150, 146, 140))
        mid = (B + N)/2 + [.004*FLIP[0], 0, 0]
        g.line(B, mid, HOTSTR); g.line(mid, N, HOTSTR)
    segs = s.setdefault("_bow", bow_track(s["ev"]))
    cur = [i for i in active] or [len(strings)//2]
    ang = base_ang + math.radians((sum(cur)/len(cur) - (len(strings)-1)/2)*-4)
    draw_bow(g, contact, ang, bow_pos(segs, t), *bowcfg)
    if active: fx.append(("dust", contact, t))
    for age, m, v, dur, tp in ev:
        if tp and age < .9: fx.append(("note", (contact[0] + .3, contact[1] + .5, contact[2]), age))

# ---------------------------------------------------------------- instruments
VPROF = [(0, .06), (.03, .17), (.1, .26), (.22, .292), (.34, .27), (.40, .22), (.44, .17), (.53, .157),
         (.62, .17), (.66, .21), (.72, .235), (.80, .236), (.9, .21), (.96, .15), (1, .05)]
VC = {
 "rebec":  dict(L=.95, d=.15, prof=[(0, .05), (.04, .15), (.15, .215), (.32, .21), (.5, .165), (.7, .1), (.85, .07), (1, .06)],
                top=(200, 140, 78), side=(150, 96, 52), fb0=.55, neck=.24, fbcol=(140, 84, 42), head="sickle", holes="rose",
                strings=[62, 69, 76], bridge=.27, tail=(.02, .16), bow=(1.0, .13, (112, 72, 40), (80, 50, 30))),
 "vielle": dict(L=1.0, d=.11, prof=[(0, .06), (.05, .2), (.2, .27), (.38, .25), (.5, .22), (.62, .24), (.8, .255), (.94, .19), (1, .08)],
                top=(178, 104, 52), side=(128, 70, 34), fb0=.62, neck=.2, fbcol=(70, 44, 28), head="leaf", holes="c",
                strings=[55, 62, 67, 74], bridge=.3, tail=(.02, .15), bow=(1.05, .12, (110, 70, 40), (80, 50, 30))),
 "lira":   dict(L=.98, d=.1, prof=[(0, .07), (.04, .18), (.14, .27), (.28, .285), (.38, .25), (.44, .19), (.53, .175), (.62, .19),
                (.68, .23), (.8, .24), (.92, .2), (1, .07)],
                top=(160, 86, 42), side=(112, 58, 28), fb0=.58, neck=.26, fbcol=(46, 32, 26), head="leaf", holes="c",
                strings=[55, 62, 69, 76], bridge=.3, tail=(.02, .15), bow=(1.1, .1, (110, 70, 40), (80, 50, 30))),
 "amati":  dict(L=.95, d=.1, prof=VPROF, top=(218, 152, 64), side=(172, 110, 44), fb0=.55, neck=.3, fbcol=(34, 26, 26),
                head="scroll", holes="f", strings=[55, 62, 69, 76], bridge=.33, tail=(.03, .2), bow=(1.15, .06, (96, 58, 32), (60, 40, 26))),
 "strad":  dict(L=.95, d=.1, prof=VPROF, top=(204, 88, 36), side=(152, 60, 26), fb0=.55, neck=.31, fbcol=(30, 24, 26),
                head="scroll", holes="f", strings=[55, 62, 69, 76], bridge=.33, tail=(.03, .2), bow=(1.2, .025, (90, 52, 28), (50, 34, 24))),
 "modern": dict(L=.95, d=.1, prof=VPROF, top=(156, 52, 30), side=(110, 36, 22), fb0=.55, neck=.35, fbcol=(26, 22, 24),
                head="scroll", holes="f", strings=[55, 62, 69, 76], bridge=.33, tail=(.03, .2), bow=(1.25, -.012, (70, 40, 24), (26, 22, 26)),
                chinrest=True),
}

def build_viol(g, t, ev, fx, s, key):
    c = VC[key]; L, d = c["L"], c["d"]; y0 = .2; yt = y0 + L; zf = d/2
    stand(g, y0 + .02)
    body(g, c["prof"], L, y0, d, c["top"], c["side"])
    yfb0, yn = y0 + c["fb0"]*L, yt + c["neck"]
    # neck + fingerboard
    slab_xy(g, [(-.036, yt - .03), (.036, yt - .03), (.028, yn), (-.028, yn)], -.035, zf, c["side"])
    slab_xy(g, [(-.046, yfb0), (.046, yfb0), (.031, yn), (-.031, yn)], zf, zf + .035, c["fbcol"], spec=.3)
    # head
    nstr = len(c["strings"]) + len(DRONES.get(key, []))
    if c["head"] == "scroll":
        slab_xy(g, [(-.033, yn), (.033, yn), (.03, yn + .16), (-.03, yn + .16)], -.045, zf, c["side"])
        xcyl(g, 0, yn + .2, -.01, .052, .036, c["side"])
        xcyl(g, 0, yn + .2, -.01, .028, .04, dim(c["side"], .7))
        for k, yy in enumerate((.035, .065, .095, .125)):
            sg = -1 if k % 2 == 0 else 1
            g.box(min(sg*.03, sg*.08), max(sg*.03, sg*.08), yn + yy - .01, yn + yy + .01, -.022, .002, (30, 24, 24))
    elif c["head"] == "sickle":
        slab_xy(g, [(-.032, yn), (.032, yn), (.03, yn + .15), (-.03, yn + .15)], -.04, zf, c["side"])
        slab_xy(g, [(-.03, yn + .13), (.03, yn + .13), (.026, yn + .2), (-.026, yn + .2)], -.13, -.03, c["side"])
        for k, yy in enumerate((.04, .08, .12)):
            sg = -1 if k % 2 == 0 else 1
            g.box(min(sg*.03, sg*.08), max(sg*.03, sg*.08), yn + yy - .01, yn + yy + .01, -.02, .004, (70, 44, 26))
    else:  # flat leaf head with front pegs
        slab_xy(g, octo(0, yn + .13, .09, .13), -.02, zf, c["side"])
        for k in range(nstr):
            px = (-.045 + .09*(k % 2)) if nstr > 1 else 0; py = yn + .05 + .17*(k//2)/max(1, (nstr - 1)//2)
            g.box(px - .012, px + .012, py - .012, py + .012, zf, zf + .045, (40, 30, 26))
    # bridge, tailpiece
    yb = y0 + c["bridge"]*L; ta, tb = y0 + c["tail"][0]*L, y0 + c["tail"][1]*L
    sb, sn = .05, .022
    g.box(-sb - .02, sb + .02, yb - .01, yb + .01, zf, zf + .07, (228, 198, 148))
    slab_xy(g, [(-.05, ta), (.05, ta), (.032, tb), (-.032, tb)], zf, zf + .022, (30, 26, 28), spec=.3)
    # sound holes
    hz = zf + .002; dark = (22, 14, 12)
    if c["holes"] == "f":
        for sg in (-1, 1):
            a, b = (sg*.105*L, y0 + .31*L), (sg*.075*L, y0 + .5*L)
            thin(g, a, b, .014, hz, dark)
            for (x, y) in (a, b): decal(g, octo(x, y, .016, .016, 6), hz, dark)
    elif c["holes"] == "c":
        for sg in (-1, 1):
            cx, cy, r = sg*.08*L, y0 + .44*L, .07*L
            angs = np.linspace(-1.0, 1.0, 5) + (0 if sg > 0 else math.pi)
            P_ = [(cx + r*math.cos(a), cy + r*math.sin(a)) for a in angs]
            for k in range(4): thin(g, P_[k], P_[k+1], .014, hz, dark)
    else:
        for sg in (-1, 1): decal(g, octo(sg*.09*L, y0 + .46*L, .022, .022, 6), hz, dark)
        decal(g, octo(0, y0 + .6*L, .03, .03, 8), hz, dark)
    if c.get("chinrest"):
        slab_xy(g, octo(-.13*L, y0 + .07*L, .1, .065), zf, zf + .045, (26, 22, 26), spec=.6)
        g.box(.02, .045, tb - .02, tb + .005, zf + .02, zf + .04, (200, 200, 210), spec=.6)
    # strings
    st = c["strings"]; n = len(st)
    TBN = []
    for i in range(n):
        u = i/(n - 1)
        TBN.append(((lerp(-.03, .03, u), tb, zf + .026), (lerp(-sb, sb, u), yb, zf + .072), (lerp(-sn, sn, u), yn, zf + .042)))
    drones = [((-.04 - .015*k, tb, zf + .026), (-sb - .03 - .015*k, yb, zf + .07), (-.045 - .015*k, yn + .04, zf + .045))
              for k in range(len(DRONES.get(key, [])))]
    contact = (0, yb + .3*(yfb0 - yb), zf + .09)
    play(g, t, ev, fx, s, st, TBN, contact, math.radians(-14), c["bow"], drones)
    if key == "strad" and int(t*6) % 2 == 0:
        fx.append(("spark", (.2, yt - .1, zf), 0)); fx.append(("spark", (-.22, y0 + .25, zf), 0))

def build_rebab(g, t, ev, fx, s):
    wood, light, skin = (118, 64, 30), (156, 90, 42), (234, 214, 172)
    g.box(-.016, .016, 0, .3, -.016, .016, (90, 54, 30))
    xcyl(g, 0, .03, 0, .03, .03, (200, 170, 90))
    g.box(-.2, .2, .3, .74, -.09, .06, wood)
    g.face([(-.17, .33, .061), (.17, .33, .061), (.17, .71, .061), (-.17, .71, .061)], skin, ref=(0, .5, 0))
    for k in range(7):
        x = -.15 + .05*k
        for y in (.315, .725): decal(g, octo(x, y, .008, .008, 4), .0615, (240, 236, 220))
    g.box(-.026, .026, .74, 1.52, -.03, .03, light)
    g.box(-.045, .045, 1.52, 1.6, -.04, .04, wood)
    xcyl(g, 0, 1.64, 0, .035, .035, (200, 170, 90))
    for y in (1.38, 1.46): g.box(-.085, .085, y - .012, y + .012, -.012, .012, (70, 40, 22))
    g.box(-.055, .055, .39, .41, .061, .1, (226, 196, 146))
    TBN = [((x*.8, .315, .065), (x, .40, .102), (x*.6, 1.40, .042)) for x in (-.02, .02)]
    play(g, t, ev, fx, s, [62, 69], TBN, (0, .56, .118), math.radians(-6), (.95, .16, (100, 62, 34), (70, 44, 26)))

def build_inst(key, g, t, ev, fx, s):
    if key == "rebab": build_rebab(g, t, ev, fx, s)
    else: build_viol(g, t, ev, fx, s, key)

# ---------------------------------------------------------------- frame
def scene_at(T):
    if T < BAR: return None
    return min(6, int((T/BAR - 1)//2))

def frame(T, fi=0):
    FLIP[0] = 1 if fi % 2 == 0 else -1
    si = scene_at(T); s = SCENES[si] if si is not None else None
    key = s["key"] if s else None
    prevkey = (SCENES[si-1]["key"] if si > 0 else None) if s else None
    kb = ease((T - s["t0"])/(.5*BEAT)) if s else 1.0
    img = (BGS[prevkey]*(1 - kb) + BGS[key]*kb).astype(np.uint8)
    pc0, pc1 = ped_cols(prevkey), ped_cols(key); pc = {k: mix(pc0[k], pc1[k], kb) for k in pc1}
    for x, y, ph in P.STARS:
        b = .5 + .5*math.sin(T*2.4 + ph)
        if b > .35: img[y, x] = mix((70, 58, 100), (230, 220, 255), b)
    rend = np.zeros((RH, RW, 3), np.uint8); zb = np.zeros((RH, RW)); gb = np.zeros((RH, RW), np.int8)
    last_kick = max([k for k in P.KICKS if k <= T], default=-9)
    pulse = math.exp(-(T - last_kick)*7)
    fx = []; scale, spin = 0.0, 0.0
    if s:
        t = T - s["t0"]
        pin = t/(.35*BEAT); scale = ease_out_back(pin); spin = -100*(1 - ease(pin))
        if si < 6:
            pout = (t - (2*BAR - .35*BEAT))/(.35*BEAT)
            if pout > 0: scale *= 1 - ease(pout); spin += 100*pout*pout
    yaw = (s["yaw"] if s else 0) + 22*math.sin(2*math.pi*T/8) + spin
    cam = Cam(DIST, yaw, scale*.86)
    gp = Geo(); build_pedestal(gp, DIST, pulse, .3*scale if s else 0, pc)
    render_geo(gp, cam, rend, zb, gb, scale=1.0)
    if s and scale > .01:
        t = T - s["t0"]
        ev = [(t - nt, m, v, dur, tp) for nt, m, v, dur, tp in s["ev"] if t >= nt]
        gi = Geo(); build_inst(key, gi, t, ev, fx, s)
        render_geo(gi, cam, rend, zb, gb)
    outline(rend, zb, gb)
    m = gb > 0; img[m] = rend[m]
    im = Image.fromarray(img); d = ImageDraw.Draw(im)
    hl = "900 YEARS"; hw = text_w(hl, 3)
    downbeat = math.exp(-((T % BAR))*6)
    d.rectangle([(RW - hw)//2 - 8, 9, (RW + hw)//2 + 7, 36], fill=INK)
    d.rectangle([(RW - hw)//2 - 8, 9, (RW + hw)//2 + 7, 36], outline=mix(GOLD_D, (255, 230, 150), downbeat))
    center_text(d, 13, hl, CREAM, 3)
    outro = T >= OUTRO_T
    if s is None:
        k = ease(T/.5)
        center_text(d, 150, "HOW WE GOT", mix(BG_BOT, CREAM, k), 3)
        center_text(d, 180, "THE VIOLIN", mix(BG_BOT, GOLD, k), 3)
    else:
        t = T - s["t0"]
        prev = SCENES[si-1]["year"] if si > 0 else 600
        k = ease(t/BEAT)
        center_text(d, 45, year_str(int(lerp(prev, s["year"], k))) if k < 1 else s["ys"], GOLD, 3)
        center_text(d, 74, s["name"], CREAM, 2)
        ck = ease((t - .3)/.3)
        center_text(d, 92, s["credit"], mix(BG_BOT, (226, 196, 130), ck), 1)
        if not outro:
            fk = ease((t - .2)/.3)
            for li, line in enumerate(s["fact"]):
                center_text(d, 336 + li*19, line, mix(BG_BOT, CREAM, fk), 2)
        else:
            fk = ease((T - OUTRO_T)/.3)
            center_text(d, 336, "FROM A SIMPLE REBAB", mix(BG_BOT, CREAM, fk), 2)
            center_text(d, 355, "TO THE MODERN VIOLIN", mix(BG_BOT, GOLD, fk), 2)
    rr = np.random.default_rng(fi)
    for f in fx:
        if f[0] == "note":
            S = cam.screen(f[1]); a = f[2]/.9
            sprite(d, NOTE_SPR, int(S[0]) - 5 + int(4*math.sin(f[2]*8)), int(S[1]) - int(f[2]*40) - 14,
                   mix((120, 96, 140), (255, 246, 214), (1 - a)**.6), 2)
        elif f[0] == "spark":
            S = cam.screen(f[1]); sprite(d, SPARK, int(S[0]) - 2, int(S[1]) - 2, (255, 246, 190), 1, False)
        elif f[0] == "dust":
            S = cam.screen(f[1])
            for _ in range(3):
                x = int(S[0] + rr.normal(0, 6)); y = int(S[1] + rr.normal(0, 3) - rr.random()*6)
                d.point((x, y), fill=(255, 250, 230))
    if T >= 16*BAR:
        tb = T - 16*BAR; r2 = np.random.default_rng(11)
        for k in range(26):
            ang = r2.random()*6.28; sp = 40 + r2.random()*80
            x = P.CX + math.cos(ang)*sp*min(1, tb*2.2); y = 200 + math.sin(ang)*sp*.8*min(1, tb*2.2) + tb*tb*20
            if tb < 2.2: sprite(d, SPARK, int(x), int(y), mix((255, 240, 180), (120, 96, 140), clamp(tb/2.2)), 1, False)
    n = len(SCENES); xs = [30 + i*(RW - 60)//(n - 1) for i in range(n)]
    cur = -1 if si is None else si
    d.line([(xs[0], 392), (xs[-1], 392)], fill=(84, 66, 106))
    if cur >= 0: d.line([(xs[0], 392), (xs[-1] if outro else xs[cur], 392)], fill=GOLD_D)
    bob = int(2*math.exp(-((T % BEAT))*14))
    for i, x in enumerate(xs):
        if i < cur or outro: d.rectangle([x-2, 390, x+2, 394], fill=GOLD_D)
        elif i == cur:
            d.rectangle([x-3, 389-bob, x+3, 395-bob], fill=GOLD); d.rectangle([x-4, 388-bob, x+4, 396-bob], outline=(255, 230, 150))
        else: d.rectangle([x-2, 390, x+2, 394], fill=(84, 66, 106))
    return im

# ---------------------------------------------------------------- audio
rng = np.random.default_rng(9)
def _t(n): return np.arange(n)/SR
TONE = {
 "rebab":  dict(tilt=1.25, form=[(900, 1.0, .5), (2200, .5, .4)], vib=.004, vdel=.12, att=.05, noise=.05, slide=True),
 "rebec":  dict(tilt=1.05, form=[(1500, 1.0, .5), (3000, .4, .4)], vib=.002, vdel=.2, att=.04, noise=.035),
 "vielle": dict(tilt=1.15, form=[(700, 1.0, .5), (2000, .5, .5)], vib=.002, vdel=.2, att=.06, noise=.03),
 "lira":   dict(tilt=1.15, form=[(600, 1.0, .5), (2400, .6, .45)], vib=.003, vdel=.2, att=.06, noise=.025),
 "amati":  dict(tilt=1.0, form=[(450, .8, .4), (2700, 1.0, .45)], vib=.003, vdel=.18, att=.045, noise=.02),
 "strad":  dict(tilt=.95, form=[(450, .8, .4), (2800, 1.2, .45)], vib=.0045, vdel=.15, att=.04, noise=.02),
 "modern": dict(tilt=.95, form=[(450, .8, .4), (3000, 1.3, .45)], vib=.006, vdel=.1, att=.03, noise=.02),
}
def bowed(f, dur, vel, kind, drone=False):
    p = TONE[kind]; rel = .4 if drone else .15
    n = int((dur + rel)*SR); t = _t(n)
    vib = p["vib"]*np.clip((t - p["vdel"])/.25, 0, 1)*np.sin(2*np.pi*5.7*t + rng.random()*6)
    fm = 1 + vib
    if p.get("slide"): fm = fm * 2 ** (-np.exp(-t/.05)/12)
    ph = 2*np.pi*np.cumsum(f*fm)/SR
    sig = np.zeros(n); tot = 0
    for k in range(1, 40):
        fk = f*k
        if fk > 6000: break
        gk = (.25 + sum(a*math.exp(-(math.log(fk/fc)/w)**2) for fc, a, w in p["form"])) / k**p["tilt"]
        sig += gk*np.sin(k*ph); tot += gk
    sig /= tot
    env = np.minimum(1, t/p["att"]) * np.where(t < dur, 1.0, np.exp(-(t - dur)/(rel/4)))
    if not drone: env = env*(.88 + .12*np.minimum(1, t/max(dur, .05)))
    sig = lfilter([.35], [1, -.65], sig)   # gentle low-pass for a smoother tone
    return sig*env*vel

def render_audio(path="audio_violin.wav"):
    n = int((TOTAL + 3)*SR)
    drums, bassb, padb, inst = (np.zeros(n) for _ in range(4))
    def add(buf, T0, x):
        a = int(T0*SR); e = min(n, a + len(x)); buf[a:e] += x[:e-a]
    for T0, k, v in P.DRUMS:
        add(drums, T0, {'kick': kick, 'snare': snare, 'hat': hat, 'crash': crash}[k](v) if k != 'ohat' else hat(v, True))
    for b in range(1, 17):
        T0 = b*BAR; root = CH[BARS[b]][0]
        if b == 16: add(bassb, T0, bass(root, 2.6, .9)); continue
        for beat, mm, dur in ((0, root, .9), (1.5, root, .4), (2.5, root, .9), (3.5, root + 7, .4)):
            add(bassb, T0 + beat*BEAT, bass(mm, dur*BEAT, .8))
    for b in range(17):
        add(padb, b*BAR, pad(CH[BARS[b]][1], 2.6 if b == 16 else BAR + .05, .5 if b else .3))
    def rms(x):
        x = x[np.abs(x) > 1e-6]; return np.sqrt((x**2).mean()) if len(x) else 1.
    def at(x, db): return x*(10**(db/20))/rms(x)
    for s in SCENES:
        sb = np.zeros(n)
        for nt, m, v, dur, tp in s["ev"]:
            add(sb, s["t0"] + nt, bowed(midi_hz(m), dur, v, s["key"]))
        for dm in DRONES.get(s["key"], []):
            add(sb, s["t0"], bowed(midi_hz(dm), 2*BAR - .1, .3, s["key"], drone=True))
        a = int(s["t0"]*SR); e = int((s["t0"] + 2*BAR)*SR)
        inst += sb*(10**(-15/20))/(np.sqrt((sb[a:e]**2).mean()) + 1e-9)
    drums, bassb, padb = at(drums, -24), at(bassb, -27), at(padb, -32)
    wet_in = inst + padb*.6
    r = np.random.default_rng(1); irn = int(1.6*SR)
    ir = r.normal(0, 1, irn)*np.exp(-np.arange(irn)/SR*3.4); ir[0] = 0; ir /= np.sqrt((ir**2).sum())
    Lf = 1 << (n + irn).bit_length()
    wet = np.fft.irfft(np.fft.rfft(wet_in, Lf)*np.fft.rfft(ir, Lf), Lf)[:n]
    out = drums + bassb + padb + inst + at(wet, -23)
    out = out[:int(TOTAL*SR)]
    fade = int(.8*SR); out[-fade:] *= np.linspace(1, 0, fade)
    out = np.tanh(1.6*out/np.abs(out).max())/np.tanh(1.6)*.94
    with wave.open(path, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(SR); w.writeframes((out*32767).astype(np.int16).tobytes())
    return path

if __name__ == "__main__":
    if "--still" in sys.argv:
        for ts in sys.argv[sys.argv.index("--still") + 1:]:
            T = float(ts); frame(T, int(T*FPS)).resize((RW*2, RH*2), Image.NEAREST).save(f"vstill_{ts}.png")
        sys.exit()
    wav = render_audio()
    nframes = int(TOTAL*FPS)
    p = subprocess.Popen(["ffmpeg", "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24",
        "-s", f"{RW}x{RH}", "-r", str(FPS), "-i", "-", "-i", wav,
        "-vf", f"scale={RW*UP}:{RH*UP}:flags=neighbor", "-c:v", "libx264", "-pix_fmt", "yuv420p",
        "-crf", "16", "-preset", "medium", "-c:a", "aac", "-b:a", "192k", "-shortest", OUT], stdin=subprocess.PIPE)
    for fi in range(nframes):
        p.stdin.write(frame(fi/FPS, fi).tobytes())
    p.stdin.close(); p.wait()
    print("wrote", OUT, f"{TOTAL:.1f}s")
