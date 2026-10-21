"""Low-poly x pixel-art history of the piano -> 1080x1920 MP4 with a steady beat.

A tiny software rasterizer (flat, banded shading + z-buffer) draws low-poly 3D models at
270x480. The frame is scaled 4x with nearest-neighbour so the polygons read as pixel art,
then pixel-art text, notes, sparkles and stars are drawn on top.

Music: 120 BPM lo-fi beat (kick / snare / hats / bass / soft pad). Every instrument note is
placed on that grid, so the on-screen key presses land on the beat.

usage:  python3 piano_lowpoly.py [out.mp4]          render the video
        python3 piano_lowpoly.py --still 3.1 7.5     write still_<t>.png frames
"""
import math, subprocess, sys, wave
import numpy as np
from PIL import Image, ImageDraw
from scipy.signal import lfilter

RW, RH, UP, FPS, SR = 270, 480, 4, 30, 44100
BPM = 120; BEAT = 60 / BPM; BAR = 4 * BEAT
OUT = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else "piano_lowpoly.mp4"

_G = {
'=':"..........#####.....#####..........",
'A':".###.#...##...#######...##...##...#", 'B':"####.#...##...#####.#...##...#####.",
'C':".###.#...##....#....#....#...#.###.", 'D':"####.#...##...##...##...##...#####.",
'E':"######....#....####.#....#....#####", 'F':"######....#....####.#....#....#....",
'G':".###.#...##....#.####...##...#.####", 'H':"#...##...##...#######...##...##...#",
'I':".###...#....#....#....#....#...###.", 'J':"..###...#....#....#....#.#..#..##..",
'K':"#...##..#.#.#..##...#.#..#..#.#...#", 'L':"#....#....#....#....#....#....#####",
'M':"#...###.###.#.##.#.##...##...##...#", 'N':"#...###..##.#.##..###...##...##...#",
'O':".###.#...##...##...##...##...#.###.", 'P':"####.#...##...#####.#....#....#....",
'Q':".###.#...##...##...##.#.##..#..##.#", 'R':"####.#...##...#####.#.#..#..#.#...#",
'S':".#####....#.....###.....#....#####.", 'T':"#####..#....#....#....#....#....#..",
'U':"#...##...##...##...##...##...#.###.", 'V':"#...##...##...##...##...#.#.#...#..",
'W':"#...##...##...##.#.##.#.##.#.#.#.#.", 'X':"#...##...#.#.#...#...#.#.#...##...#",
'Y':"#...##...#.#.#...#....#....#....#..", 'Z':"#####....#...#...#...#...#....#####",
'0':".###.#...##..###.#.###..##...#.###.", '1':"..#...##....#....#....#....#...###.",
'2':".###.#...#....#...#...#...#...#####", '3':"#####...#...#.....#.....##...#.###.",
'4':"...#...##..#.#.#..#.#####...#....#.", '5':"######....####.....#....##...#.###.",
'6':"..##..#...#....####.#...##...#.###.", '7':"#####....#...#...#...#....#....#...",
'8':".###.#...##...#.###.#...##...#.###.", '9':".###.#...##...#.####....#...#..##..",
' ':"."*35, ',':"."*25+"..#...#...", '.':"."*30+"..#..", '-':"."*15+".###."+"."*15,
'!':"..#....#....#....#....#.........#..", ':':".....#.........."[:5]+"..#.."+"."*15+"..#.."+"."*5,
"'":"..#....#...#...."+"."*19, '/':"....#....#...#...#...#...#....#....",
'?':".###.#...#....#...#...#.........#..", 's':"...........#####.....###.....#####.",
}

def _rows(c):
    s = (_G.get(c, _G[' ']) + "." * 35)[:35]
    return [s[i*5:(i+1)*5] for i in range(7)]

def text_w(s, sc=1): return len(s) * 6 * sc - sc

def draw_text(d, x, y, s, col, sc=1, shadow=True):
    for k, ch in enumerate(s):
        for r, row in enumerate(_rows(ch)):
            for c, px in enumerate(row):
                if px == '#':
                    X, Y = x + (k*6 + c)*sc, y + r*sc
                    if shadow: d.rectangle([X+sc, Y+sc, X+2*sc-1, Y+2*sc-1], fill=SHADOW)
                    d.rectangle([X, Y, X+sc-1, Y+sc-1], fill=col)

def center_text(d, y, s, col, sc=1, shadow=True, cx=RW // 2):
    draw_text(d, cx - text_w(s, sc) // 2, y, s, col, sc, shadow)

# ---------------------------------------------------------------- palette / helpers
CREAM, GOLD, GOLD_D = (246, 232, 200), (240, 190, 80), (170, 120, 40)
INK, SHADOW = (14, 8, 24), (8, 4, 16)
BG_TOP, BG_BOT = (14, 10, 34), (60, 30, 78)
HOT, HOTSTR = (255, 200, 90), (255, 246, 196)

def lerp(a, b, t): return a + (b - a) * t
def clamp(x, a=0., b=1.): return max(a, min(b, x))
def ease(t): t = clamp(t); return t*t*(3 - 2*t)
def ease_out_back(t):
    t = clamp(t); c1 = 1.9; return 1 + (c1+1)*(t-1)**3 + c1*(t-1)**2
def mix(c1, c2, t): return tuple(int(round(lerp(a, b, t))) for a, b in zip(c1, c2))
def dim(c, k): return tuple(int(min(255, v*k)) for v in c)
def midi_hz(m): return 440 * 2 ** ((m - 69) / 12)

# ---------------------------------------------------------------- music + story data
CH = {"Am": (45, (57, 60, 64)), "F": (41, (57, 60, 65)), "C": (36, (55, 60, 64)), "G": (43, (55, 59, 62))}
BARS = ["Am", "F", "F", "G", "Am", "F", "C", "G", "Am", "F", "C", "G", "Am", "F", "C", "G", "C"]

SCENES = [
 dict(key="mono", year=-500, name="MONOCHORD", fact=("ONE STRING.", "HALF = AN OCTAVE UP"),
      notes=[(1, 53, .9), (3, 60, .9), (5, 65, .9), (7, 60, .9)],
      cam=1.6, yaw=22),
 dict(key="dulc", year=1100, ys="1100s", name="HAMMERED DULCIMER", fact=("HAMMERS HIT STRINGS:", "THE PIANO'S BIG IDEA"),
      notes=[(0.5 + 0.5*i, m, .8) for i, m in enumerate([67, 71, 74, 71, 67, 71, 74])] +
            [(4 + 0.5*i, m, .8) for i, m in enumerate([69, 72, 76, 72, 69, 72, 76, 81])],
      cam=1.95, yaw=16),
 dict(key="clav", year=1404, name="CLAVICHORD", fact=("KEYS PRESS BRASS", "TANGENTS. SO QUIET!"),
      notes=[(1, 65, .55), (2, 69, .55), (3, 72, .55), (4, 67, .55), (5, 64, .55), (6, 72, .6)],
      cam=2.4, yaw=20),
 dict(key="harp", year=1500, ys="1500s", name="HARPSICHORD", fact=("QUILLS PLUCK STRINGS", "LOUD, BUT NEVER SOFT"),
      notes=[(0.5 + 0.5*i, m, .9) for i, m in enumerate([55, 59, 62, 67, 71, 74, 79])] +
            [(4 + 0.5*i, m, .9) for i, m in enumerate([57, 60, 64, 69, 72, 76, 81, 76])],
      cam=3.25, yaw=-34),
 dict(key="forte", year=1700, name="FORTEPIANO", fact=("CRISTOFORI'S HAMMERS", "PLAY SOFT AND LOUD"),
      notes=[(1, 65, .22), (1.5, 69, .22), (2, 72, .22), (3, 69, .22),
             (4, [48, 60, 64, 67, 72], 1.), (6, [55, 64, 67, 72], 1.)],
      cam=3.0, yaw=-34),
 dict(key="square", year=1760, ys="1760s", name="SQUARE PIANO", fact=("SMALL, BOXY AND", "MADE FOR THE HOME"),
      notes=[(0.5, 74, .6), (1, 71, .6), (1.5, 67, .6), (2, 71, .6), (3, 74, .6),
             (4, 72, .6), (4.5, 76, .6), (5, 81, .6), (6, 76, .6), (7, 72, .6)],
      cam=2.9, yaw=22),
 dict(key="grand", year=1880, ys="1880s", name="GRAND PIANO", fact=("IRON FRAME, CROSSED", "STRINGS, 88 KEYS"),
      notes=[(0, [29, 41], 1.)] +
            [(1 + 0.25*i, m, .8) for i, m in enumerate([53, 57, 60, 65, 69, 72, 77, 81, 84, 89])] +
            [(4, [36, 48, 60, 64, 67], 1.)] +
            [(5 + 0.25*i, m, .8) for i, m in enumerate([84, 79, 76, 72, 67, 64, 60])] +
            [(7, [60, 64, 67, 72], .8),
             # outro: G stabs, a run up, then the final C chord
             (8, [43, 55, 59, 62, 67], .95), (9.5, [55, 59, 62, 67], .7), (10.5, [55, 59, 62, 67], .85)] +
            [(11 + 0.25*i, m, .8) for i, m in enumerate([67, 71, 74, 79])] +
            [(12, [24, 36, 48, 60, 64, 67, 72, 76], 1.)],
      cam=4.0, yaw=-36),
]
for si, s in enumerate(SCENES):
    s["t0"] = (1 + 2*si) * BAR
    s["ev"] = []
    for beat, ms, v in s["notes"]:
        ml = list(ms) if isinstance(ms, (list, tuple)) else [ms]
        hold = 0.22 if len(ml) == 1 else 0.5
        if s["key"] == "grand" and beat >= 12: hold = 2.4
        for m in ml: s["ev"].append((beat*BEAT, m, v, hold, m == max(ml)))
OUTRO_T = 15 * BAR
TOTAL = 16 * BAR + 2.6

def year_str(y):
    if y < 0: return f"{-y} BC"
    if y < 1000: return f"{max(1, y)} AD"
    return str(y)

def drum_events():
    ev = []
    for b in range(17):
        T0 = b * BAR
        if b == 16:
            ev += [(T0, 'crash', 1.0), (T0, 'kick', 1.0)]; continue
        if b == 0:
            ev += [(T0, 'kick', .9), (T0 + 2*BEAT, 'kick', .9), (T0 + 3*BEAT, 'snare', .6),
                   (T0 + 3.5*BEAT, 'snare', .4), (T0 + 3.75*BEAT, 'snare', .55)]
            ev += [(T0 + i*.5*BEAT, 'hat', .5 if i % 2 == 0 else .32) for i in range(8)]
            continue
        ev += [(T0, 'kick', 1.0), (T0 + 2.5*BEAT, 'kick', .85), (T0 + BEAT, 'snare', .8), (T0 + 3*BEAT, 'snare', .8)]
        if b % 2 == 0: ev += [(T0 + 1.75*BEAT, 'kick', .55), (T0 + 3.75*BEAT, 'hat', .3)]
        ev += [(T0 + i*.5*BEAT, 'hat', .55 if i % 2 == 0 else .35) for i in range(8)]
        if b in (1, 13): ev.append((T0, 'crash', .8))
        if b in (13, 14, 15): ev += [(T0 + 1.5*BEAT, 'ohat', .4), (T0 + 3.5*BEAT, 'ohat', .4)]
        if b == 15: ev += [(T0 + 3*BEAT + k*.25*BEAT, 'snare', .45 + .12*k) for k in range(4)]
    return sorted(ev)
DRUMS = drum_events()
KICKS = [t for t, k, v in DRUMS if k == 'kick']

# ---------------------------------------------------------------- 3D engine
F_PX, CX, CY = 330.0, RW / 2, 214.0
LIGHT = np.array([0.45, 1.0, 0.6]); LIGHT /= np.linalg.norm(LIGHT)
ELEV = math.radians(24)
FLIP = [1]

class Geo:
    def __init__(s): s.F = []; s.L = []
    def face(s, pts, col, ref=None, two=False, emit=False, gid=2, spec=0.0):
        P = np.asarray(pts, float); n = np.zeros(3)
        for i in range(len(P)):
            a, b = P[i], P[(i+1) % len(P)]
            n[0] += (a[1]-b[1])*(a[2]+b[2]); n[1] += (a[2]-b[2])*(a[0]+b[0]); n[2] += (a[0]-b[0])*(a[1]+b[1])
        l = np.linalg.norm(n)
        if l < 1e-12: return
        n /= l
        if ref is not None and np.dot(n, P.mean(0) - np.asarray(ref, float)) < 0: n = -n
        s.F.append((P, n, col, two, emit, gid, spec))
    def line(s, a, b, col, gid=3): s.L.append((np.asarray(a, float), np.asarray(b, float), col, gid))
    def box(s, x0, x1, y0, y1, z0, z1, col, **kw):
        c = ((x0+x1)/2, (y0+y1)/2, (z0+z1)/2)
        v = [(x0,y0,z0),(x1,y0,z0),(x1,y1,z0),(x0,y1,z0),(x0,y0,z1),(x1,y0,z1),(x1,y1,z1),(x0,y1,z1)]
        for idx in ((0,1,2,3),(4,5,6,7),(0,4,7,3),(1,5,6,2),(3,2,6,7),(0,1,5,4)):
            s.face([v[i] for i in idx], col, ref=c, **kw)
    def bar(s, p0, p1, y0, y1, w, col, **kw):
        d = np.array([p1[0]-p0[0], p1[1]-p0[1]]); d /= np.linalg.norm(d); n = np.array([-d[1], d[0]]) * w / 2
        a, b, c_, e = np.array(p0)+n, np.array(p1)+n, np.array(p1)-n, np.array(p0)-n
        q = [a, b, c_, e]
        cen = ((p0[0]+p1[0])/2, (y0+y1)/2, (p0[1]+p1[1])/2)
        s.face([(p[0], y1, p[1]) for p in q], col, ref=cen, **kw)
        for k in range(4):
            u, v = q[k], q[(k+1) % 4]
            s.face([(u[0],y0,u[1]),(v[0],y0,v[1]),(v[0],y1,v[1]),(u[0],y1,u[1])], col, ref=cen, **kw)
    def ngon(s, cx, y, cz, r, col, n=8, **kw):
        s.face([(cx + r*math.cos(2*math.pi*k/n + .2), y, cz + r*math.sin(2*math.pi*k/n + .2)) for k in range(n)],
               col, ref=(cx, y-1, cz), **kw)

def prism(g, pts, y0, y1, side, top, **kw):
    cx = sum(p[0] for p in pts)/len(pts); cz = sum(p[1] for p in pts)/len(pts); ref = (cx, (y0+y1)/2, cz)
    g.face([(x, y1, z) for x, z in pts], top, ref=ref, **kw)
    for k in range(len(pts)):
        (xa, za), (xb, zb) = pts[k], pts[(k+1) % len(pts)]
        g.face([(xa,y0,za),(xb,y0,zb),(xb,y1,zb),(xa,y1,za)], side, ref=ref, **kw)

def tri(img, zb, gb, S, D, col, gid):
    x0 = max(int(math.floor(S[:,0].min())), 0); x1 = min(int(math.ceil(S[:,0].max())), RW-1)
    y0 = max(int(math.floor(S[:,1].min())), 0); y1 = min(int(math.ceil(S[:,1].max())), RH-1)
    if x0 > x1 or y0 > y1: return
    (ax, ay), (bx, by), (cx, cy) = S
    area = (bx-ax)*(cy-ay) - (by-ay)*(cx-ax)
    if abs(area) < 1e-9: return
    X, Y = np.meshgrid(np.arange(x0, x1+1) + .5, np.arange(y0, y1+1) + .5)
    w0 = ((cx-bx)*(Y-by) - (cy-by)*(X-bx)) / area
    w1 = ((ax-cx)*(Y-cy) - (ay-cy)*(X-cx)) / area
    w2 = 1 - w0 - w1
    m = (w0 >= -1e-6) & (w1 >= -1e-6) & (w2 >= -1e-6)
    if not m.any(): return
    iz = w0/D[0] + w1/D[1] + w2/D[2]
    sub = zb[y0:y1+1, x0:x1+1]
    m &= iz > sub
    sub[m] = iz[m]
    img[y0:y1+1, x0:x1+1][m] = col
    gb[y0:y1+1, x0:x1+1][m] = gid

class Cam:
    def __init__(s, dist, yaw_deg, scale=1.0):
        s.dist = dist; s.target = np.array([0, 0.18*dist, 0])
        s.eye = s.target + dist*np.array([0, math.sin(ELEV), math.cos(ELEV)])
        f = s.target - s.eye; s.fwd = f/np.linalg.norm(f)
        r = np.cross(s.fwd, [0, 1, 0]); s.right = r/np.linalg.norm(r); s.up = np.cross(s.right, s.fwd)
        a = math.radians(yaw_deg); c, sn = math.cos(a), math.sin(a)
        s.R = np.array([[c, 0, sn], [0, 1, 0], [-sn, 0, c]])
        s.scale = scale
    def world(s, P, scale=None):
        return (np.asarray(P, float) * (s.scale if scale is None else scale)) @ s.R.T
    def proj(s, W):
        Q = W - s.eye; d = Q @ s.fwd
        return np.stack([CX + F_PX*(Q @ s.right)/d, CY - F_PX*(Q @ s.up)/d], -1), d
    def screen(s, p, scale=None):
        S, d = s.proj(s.world(np.array([p]), scale)); return S[0]

def render_geo(geo, cam, img, zb, gb, scale=None):
    sc = cam.scale if scale is None else scale
    if sc < 1e-3: return
    for P, n, col, two, emit, gid, spec in geo.F:
        W = cam.world(P, sc); nw = cam.R @ n
        v = cam.eye - W.mean(0); v /= np.linalg.norm(v)
        dn = nw @ v
        if dn <= 0:
            if not two: continue
            nw = -nw
        S, D = cam.proj(W)
        if (D <= 0.05).any(): continue
        if emit: cc = np.array(col, float)
        else:
            I = 0.36 + 0.64*max(0.0, nw @ LIGHT)
            I = math.floor(I*7)/7 + 0.05
            cc = np.array(col, float)*I
            if spec:
                h = LIGHT + v; h /= np.linalg.norm(h); sp = max(0.0, nw @ h) ** 40
                if sp > 0.3: cc = cc + spec*(110 if sp > 0.7 else 50)
        cc = np.clip(cc, 0, 255).astype(np.uint8)
        for k in range(1, len(P)-1):
            tri(img, zb, gb, S[[0, k, k+1]], D[[0, k, k+1]], cc, gid)
    for a, b, col, gid in geo.L:
        S, D = cam.proj(cam.world(np.stack([a, b]), sc))
        if (D <= 0.05).any(): continue
        n = int(max(abs(S[1,0]-S[0,0]), abs(S[1,1]-S[0,1]))) + 2
        t = np.linspace(0, 1, n)
        X = S[0,0] + (S[1,0]-S[0,0])*t; Y = S[0,1] + (S[1,1]-S[0,1])*t
        IZ = (1/D[0])*(1-t) + (1/D[1])*t
        xi, yi = X.astype(int), Y.astype(int)
        m = (xi >= 0) & (xi < RW) & (yi >= 0) & (yi < RH)
        xi, yi, IZ = xi[m], yi[m], IZ[m]
        ok = IZ*1.004 >= zb[yi, xi]
        img[yi[ok], xi[ok]] = col; gb[yi[ok], xi[ok]] = gid

def outline(img, zb, gb):
    inst = gb == 2; o = np.zeros_like(inst); o1 = np.zeros_like(inst)
    for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
        ng = np.roll(np.roll(gb, dy, 0), dx, 1); nz = np.roll(np.roll(zb, dy, 0), dx, 1)
        o |= inst & ((ng <= 1) | (nz < zb*0.9))
        o1 |= (gb == 1) & (ng == 0)
    o |= o1
    img[o] = (img[o]*0.3 + np.array(INK)*0.7).astype(np.uint8)

# ---------------------------------------------------------------- background (pixel art)
B4 = np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]) / 16
def make_bg(top_c=BG_TOP, bot_c=BG_BOT, beam=(120, 92, 150)):
    yy, xx = np.mgrid[0:RH, 0:RW]
    g = yy / RH; K = 7
    pal = np.array([mix(top_c, bot_c, i/(K-1)) for i in range(K)], float)
    idx = np.clip(np.floor(g*(K-1) + B4[yy % 4, xx % 4]), 0, K-1).astype(int)
    bg = pal[idx]
    # spotlight cone (dithered)
    top, bot = 92, 330
    half = 16 + (yy - top) / (bot - top) * 118
    inside = (yy > top) & (yy < bot) & (np.abs(xx - CX) < half)
    fall = 1 - np.abs(xx - CX) / np.maximum(half, 1)
    lit = inside & (B4[yy % 4, xx % 4] < 0.25 + 0.35*fall)
    bg[lit] = bg[lit]*0.55 + np.array(beam)*0.45
    return bg.astype(np.uint8)
# per-instrument themes: (sky top, sky bottom, spotlight beam)
THEMES = {
 None:     (BG_TOP, BG_BOT, (120, 92, 150)),
 "mono":   ((6, 26, 32), (18, 74, 76), (96, 170, 158)),
 "dulc":   ((34, 12, 10), (104, 42, 28), (214, 128, 92)),
 "clav":   ((8, 24, 14), (30, 80, 48), (124, 186, 124)),
 "harp":   ((8, 12, 40), (30, 46, 112), (112, 136, 220)),
 "forte":  ((32, 6, 18), (100, 24, 50), (210, 104, 136)),
 "square": ((34, 18, 6), (118, 64, 22), (228, 168, 92)),
 "grand":  ((12, 10, 10), (62, 48, 34), (214, 182, 118)),
}
BGS = {k: make_bg(*v).astype(float) for k, v in THEMES.items()}
BG = BGS[None].astype(np.uint8)
def ped_cols(key):
    b = THEMES[key][1]
    return dict(low_side=dim(b, .55), low_top=dim(b, .85), side=dim(b, .8), top=mix(b, (255, 255, 255), .14), shadow=dim(b, .45))
_r = np.random.default_rng(5)
STARS = [(int(_r.integers(0, RW)), int(_r.integers(95, 330)), _r.random()*6.28) for _ in range(46)]

# ---------------------------------------------------------------- instrument helpers
WHITE = {0, 2, 4, 5, 7, 9, 11}
def keymap(low, nwhite):
    km, wi, m = {}, 0, low
    while wi < nwhite:
        if m % 12 in WHITE: km[m] = (False, wi); wi += 1
        else: km[m] = (True, wi)
        m += 1
    return km

def key_x(km, kw, x0, m):
    blk, pos = km[m]; return x0 + pos*kw if blk else x0 + (pos + .5)*kw

def build_keys(g, x0, x1, zf, ytop, low, nwhite, active, nat=(236, 230, 212), sharp=(30, 24, 28), depth=0.15, spec=0.):
    km = keymap(low, nwhite); kw = (x1 - x0)/nwhite; zb = zf - depth
    g.box(x0, x1, ytop - .022, ytop, zb, zf, nat)
    for m, (blk, pos) in km.items():
        if not blk and m in active:
            xa = x0 + pos*kw
            g.face([(xa+.1*kw, ytop+.002, zb), (xa+.9*kw, ytop+.002, zb), (xa+.9*kw, ytop+.002, zf), (xa+.1*kw, ytop+.002, zf)],
                   HOT, ref=(xa, ytop-1, zf), emit=True)
            g.face([(xa+.1*kw, ytop-.021, zf+.002), (xa+.9*kw, ytop-.021, zf+.002), (xa+.9*kw, ytop, zf+.002), (xa+.1*kw, ytop, zf+.002)],
                   dim(HOT, .8), ref=(xa, ytop, zf-1), emit=True)
    sep = mix(nat, (0, 0, 0), .45) if sum(nat) > 380 else mix(nat, (255, 255, 255), .3)
    for i in range(1, nwhite):
        x = x0 + i*kw; g.line((x, ytop+.001, zb), (x, ytop+.001, zf), sep)
    for m, (blk, pos) in km.items():
        if blk:
            xc = x0 + pos*kw; w = .56*kw; on = m in active
            g.box(xc-w/2, xc+w/2, ytop, ytop+.014, zb, zb + depth*.63, HOT if on else sharp, emit=on, spec=spec)
    return km, kw

def vstring(g, a, b, rest, age, amp=.006):
    if age is not None and age < .55:
        k = 1 - age/.55; a = np.array(a, float); b = np.array(b, float); pts = []
        for i in range(7):
            u = i/6; p = a*(1-u) + b*u; p[1] += amp*k*math.sin(math.pi*u)*FLIP[0]; pts.append(p)
        for i in range(6): g.line(pts[i], pts[i+1], HOTSTR)
    else:
        g.line(a, b, rest)

def ring_age(ev, m, limit=.6):
    ages = [e[0] for e in ev if e[1] == m and e[0] < limit]
    return min(ages) if ages else None

def wing_case(g, xl, xr, zf, back, y0, y1, t, sb_y, out, rim, inwall, board, n=26, spec=0.):
    xs = np.linspace(xl, xr, n+1); xi = np.linspace(xl+t, xr-t, n+1)
    O = [(xl, zf), (xr, zf)] + [(x, back(x)) for x in xs[::-1]]
    I = [(xl+t, zf-t), (xr-t, zf-t)] + [(x, back(x)+t) for x in xi[::-1]]
    K = len(O); ym = (y0+y1)/2
    for k in range(K):
        a, b = O[k], O[(k+1) % K]; ia, ib = I[k], I[(k+1) % K]
        if abs(a[0]-b[0]) + abs(a[1]-b[1]) < 1e-6: continue
        g.face([(a[0],y0,a[1]),(b[0],y0,b[1]),(b[0],y1,b[1]),(a[0],y1,a[1])], out,
               ref=((ia[0]+ib[0])/2, ym, (ia[1]+ib[1])/2), spec=spec)
        g.face([(ia[0],sb_y,ia[1]),(ib[0],sb_y,ib[1]),(ib[0],y1,ib[1]),(ia[0],y1,ia[1])], inwall,
               ref=((a[0]+b[0])/2, ym, (a[1]+b[1])/2))
        g.face([(a[0],y1,a[1]),(b[0],y1,b[1]),(ib[0],y1,ib[1]),(ia[0],y1,ia[1])], rim, ref=(a[0], y1-1, a[1]), spec=spec)
    for i in range(n):
        u0, u1 = xi[i], xi[i+1]
        g.face([(u0,sb_y,zf-t),(u1,sb_y,zf-t),(u1,sb_y,back(u1)+t),(u0,sb_y,back(u0)+t)], board, ref=(u0, sb_y-1, zf))

def wing_lid(g, xl, xr, zf, back, y1, ang, top, under, n=26, spec=0.):
    xs = np.linspace(xl, xr, n+1); c, s_ = math.cos(ang), math.sin(ang)
    def rot(x, y, z): return (xl + (x-xl)*c - (y-y1)*s_, y1 + (x-xl)*s_ + (y-y1)*c, z)
    for i in range(n):
        a, b = xs[i], xs[i+1]; yy = y1 + .004
        q = [rot(a, yy, zf), rot(b, yy, zf), rot(b, yy, back(b)), rot(a, yy, back(a))]
        g.face(q, top, ref=rot(a, y1-1, zf), spec=spec)
        g.face(q, under, ref=rot(a, y1+1, zf))
    return rot

def panel_lid(g, u0, u1, vlen, hy, hz, phi, outc, inc, spec=0.):
    c, s_ = math.cos(phi), math.sin(phi); n_in = np.array([0, -c, s_])
    def P(u, v, off=0.): return (u, hy + v*s_ + off*n_in[1], hz + v*c + off*n_in[2])
    q = [P(u0, 0), P(u1, 0), P(u1, vlen), P(u0, vlen)]
    mid = np.array(P((u0+u1)/2, vlen/2))
    g.face(q, outc, ref=mid + n_in, spec=spec)
    g.face(q, inc, ref=mid - n_in)
    def paint(pts_uv, col):
        g.face([P(u, v, .004) for u, v in pts_uv], col, ref=mid - n_in)
    return P, paint

# ---------------------------------------------------------------- instruments (model space, metres)
def build_mono(g, t, ev, fx, s):
    dark, wood, top = (78, 44, 26), (176, 116, 64), (214, 158, 96)
    g.box(-.57, .57, 0, .03, -.08, .08, dark)
    g.box(-.54, .54, .03, .11, -.065, .065, wood)
    g.face([(-.52, .1106, -.05), (.52, .1106, -.05), (.52, .1106, .05), (-.52, .1106, .05)], top, ref=(0, 0, 0))
    g.ngon(.12, .1112, 0, .032, (60, 34, 20), emit=True)
    s0, s1, ys = -.47, .47, .132
    for x in (s0, s1): g.box(x-.008, x+.008, .11, .137, -.03, .03, dark)
    ratios = {53: 1.0, 60: 2/3, 65: 0.5}; labels = {53: "1/1", 60: "2/3", 65: "1/2"}
    seq = [(nt, m) for nt, m, v, h, tp in s["ev"]]
    cur, lab, prev = 1.0, None, 1.0
    for nt, m in seq:
        if t >= nt - .3:
            cur = lerp(prev, ratios[m], ease((t - (nt - .3)) / .25)); lab = labels[m]; prev = ratios[m]
    bx = s0 + (s1 - s0)*cur
    g.box(bx-.012, bx+.012, .11, .14, -.03, .03, (96, 58, 34))
    last = min([e[0] for e in ev], default=None)
    amp = .012*max(0, 1 - last/1.2) if last is not None else 0
    hot = amp > .002
    pts = []
    for i in range(13):
        u = i/12; x = s0 + (bx - s0)*u
        pts.append((x, ys + amp*math.sin(math.pi*u)*FLIP[0], 0))
    for i in range(12): g.line(pts[i], pts[i+1], HOTSTR if hot else (214, 204, 184))
    g.line((bx, ys, 0), (s1, ys, 0), (150, 140, 130))
    if lab: fx.append(("label", (bx, .26, 0), lab, GOLD, 2))
    for age, m, v, h, tp in ev:
        if age < .9: fx.append(("note", ((s0+bx)/2, .2, 0), age))

def build_dulc(g, t, ev, fx, s):
    leg = (96, 56, 32)
    for x in (-.42, .42):
        for z in (-.17, .17): g.box(x-.02, x+.02, 0, .55, z-.02, z+.02, leg)
    g.box(-.42, .42, .14, .17, .15, .19, leg); g.box(-.42, .42, .14, .17, -.19, -.15, leg)
    body = [(-.36, -.22), (.36, -.22), (.52, .22), (-.52, .22)]
    prism(g, body, .55, .62, (122, 66, 34), (216, 168, 106))
    prism(g, [(-.33, -.19), (.33, -.19), (.47, .19), (-.47, .19)], .62, .625, (200, 150, 92), (226, 182, 120))
    for cx in (-.2, .2):
        g.ngon(cx, .6262, .02, .055, (92, 52, 26), emit=True); g.ngon(cx, .627, .02, .035, (170, 112, 60), emit=True)
    zs = np.linspace(-.17, .17, 10)
    def span(z):
        w = lerp(.36, .52, (z + .22)/.44) - .07; return -w, w
    a0, b0 = span(zs[0]); a1, b1 = span(zs[-1])
    for fr in (.36, .84):
        g.bar((lerp(a0, b0, fr), zs[0] - .02), (lerp(a1, b1, fr), zs[-1] + .02), .625, .642, .014, (96, 60, 34))
    pitches = sorted({e[1] for e in s["ev"]})
    def row(m): return len(zs) - 1 - pitches.index(m)*(len(zs) - 1)//max(1, len(pitches) - 1)
    hotrows = {}
    for age, m, v, h, tp in ev:
        if age < .6: hotrows[row(m)] = min(age, hotrows.get(row(m), 9))
    for i, z in enumerate(zs):
        a, b = span(z)
        vstring(g, (a, .646, z), (b, .646, z), (200, 194, 180), hotrows.get(i), amp=.004)
    notes = sorted(s["ev"])
    for hand, hx in ((0, -.2), (1, .2)):
        seq = [(nt, m) for i, (nt, m, v, h, tp) in enumerate(notes) if i % 2 == hand]
        def target(m, i):
            a, b = span(zs[row(m)]); return np.array([lerp(a, b, .22 if hand == 0 else .64) + (i % 2)*.04, .65, zs[row(m)]])
        pos, lift = None, .12
        for i, (nt, m) in enumerate(seq):
            if t < nt:
                if i == 0: pos = target(m, 0)
                else:
                    p0, p1 = target(seq[i-1][1], i-1), target(m, i); pr = (t - seq[i-1][0])/(nt - seq[i-1][0])
                    pos = p0*(1-pr) + p1*pr; lift = .12*math.sin(math.pi*pr)
                break
        if pos is None:
            pos = target(seq[-1][1], len(seq)-1); lift = min(.12, (t - seq[-1][0])*.5)
        head = pos + np.array([0, lift + .012, 0])
        hand_p = np.array([hx*1.15, .8, .4])
        for off in (-.006, .006):
            g.line(hand_p + [off, 0, 0], head + [off, 0, 0], (120, 74, 40))
        g.box(head[0]-.02, head[0]+.02, head[1]-.012, head[1]+.012, head[2]-.012, head[2]+.012, (54, 32, 20))
        if lift < .01 and t > .1: fx.append(("spark", tuple(head + [0, .03, 0]), 0.0))
    for age, m, v, h, tp in ev:
        if age < .9: fx.append(("note", (span(zs[row(m)])[0] + .2, .72, zs[row(m)]), age))

def build_clav(g, t, ev, fx, s):
    leg = (88, 56, 36)
    for x in (-.5, .5):
        for z in (-.14, .14): g.box(x-.022, x+.022, 0, .55, z-.022, z+.022, leg)
    g.box(-.5, .5, .12, .145, -.012, .012, leg)
    out, rim, inwall, board = (104, 66, 42), (146, 96, 58), (78, 50, 32), (196, 156, 100)
    wing_case(g, -.56, .56, .19, lambda x: -.19, .55, .69, .02, .60, out, rim, inwall, board, n=2)
    g.face([(.22, .6006, -.17), (.54, .6006, -.17), (.54, .6006, .04), (.22, .6006, .04)], (228, 192, 132), ref=(0, 0, 0))
    g.bar((.33, -.16), (.31, .04), .60, .622, .012, (124, 82, 42))
    zs = np.linspace(-.15, .03, 7)
    pitches = sorted({e[1] for e in s["ev"]})
    def row(m): return pitches.index(m)*(len(zs) - 1)//max(1, len(pitches) - 1)
    hot = {}
    for age, m, v, h, tp in ev:
        lim = 1.5 if age >= 0 and m == 72 and t > 2.8 else .6
        if age < lim: hot[row(m)] = min(age if lim == .6 else min(age, .3), hot.get(row(m), 9))
    for i, z in enumerate(zs):
        vstring(g, (-.53, .64, z), (.53, .64, z), (200, 196, 186), hot.get(i), amp=.003)
    active = {e[1] for e in ev if e[0] < e[3]}
    km, kw = build_keys(g, -.52, .12, .25, .70, 41, 29, active, depth=.2)
    for age, m, v, h, tp in ev:
        x = key_x(km, kw, -.52, m)
        if age < .6: g.box(x-.006, x+.006, .64, .668, zs[row(m)]-.006, zs[row(m)]+.006, (240, 190, 80), emit=True)
        if age < .9: fx.append(("note", (x, .74, zs[row(m)]), age))
    P, paint = panel_lid(g, -.56, .56, .38, .69, -.19, math.radians(100), out, (54, 108, 84))
    paint([(-.5, .03), (.5, .03), (.5, .35), (-.5, .35)], (122, 172, 214))
    paint([(-.5, .03), (-.12, .22), (.24, .03)], (66, 136, 80))
    paint([(.0, .03), (.3, .18), (.5, .1), (.5, .03)], (46, 104, 62))
    paint([(.26 + .04*math.cos(a), .28 + .04*math.sin(a)) for a in np.linspace(0, 2*math.pi, 9)[:-1]], (252, 212, 96))
    for uv in ([(-.53, .0), (.53, .0), (.53, .025), (-.53, .025)], [(-.53, .355), (.53, .355), (.53, .38), (-.53, .38)],
               [(-.56, 0), (-.53, 0), (-.53, .38), (-.56, .38)], [(.53, 0), (.56, 0), (.56, .38), (.53, .38)]):
        paint(uv, (226, 180, 74))

WING = {
 "harp":  dict(L=2.05, xl=-.48, xr=.48, x0=-.425, x1=.425, nwhite=36, low=36, y0=.55, y1=.80, sb=.71, sy=.726, ky=.745,
               f=lambda u: .13 + .87*(1-u)**1.7, out=(42, 106, 76), rim=(224, 178, 74), inwall=(36, 88, 62),
               board=(228, 198, 142), lid=(42, 106, 76), under=(156, 46, 46), leg=(190, 148, 64), ang=52, spec=0.,
               nat=(42, 32, 30), sharp=(234, 226, 204)),
 "forte": dict(L=1.85, xl=-.38, xr=.38, x0=-.342, x1=.342, nwhite=29, low=41, y0=.55, y1=.78, sb=.70, sy=.715, ky=.735,
               f=lambda u: .16 + .84*(1-u)**1.4, out=(134, 82, 44), rim=(178, 118, 66), inwall=(112, 66, 36),
               board=(224, 186, 126), lid=(134, 82, 44), under=(198, 152, 102), leg=(112, 66, 36), ang=50, spec=0.,
               nat=(236, 230, 212), sharp=(30, 24, 28)),
 "grand": dict(L=2.7, xl=-.76, xr=.76, x0=-.6135, x1=.6135, nwhite=52, low=21, y0=.60, y1=.92, sb=.80, sy=.83, ky=.735,
               f=lambda u: .33 + .67*max(0., math.cos(u*math.pi/2))**1.25, out=(26, 24, 34), rim=(40, 38, 50),
               inwall=(32, 30, 40), board=(208, 162, 72), lid=(26, 24, 34), under=(54, 50, 66), leg=(26, 24, 34), ang=50,
               spec=1., nat=(238, 234, 220), sharp=(24, 20, 26)),
}

def build_wing(g, t, ev, fx, s, kind):
    p = WING[kind]; L, xl, xr = p["L"], p["xl"], p["xr"]
    zf = L/2 - .12; Wd = xr - xl
    back = lambda x: zf - L*p["f"](clamp((x - xl)/Wd))
    sp = p["spec"]
    # legs
    if kind == "grand":
        legs = [(xl+.09, zf+.06), (xr-.09, zf+.06), (xl+.38, back(xl+.38)+.2)]
        for x, z in legs: g.box(x-.05, x+.05, 0, p["y0"], z-.05, z+.05, p["leg"], spec=sp)
        g.box(-.07, .07, .07, p["y0"], zf+.03, zf+.07, p["leg"], spec=sp)
        for x in (-.045, 0, .045): g.box(x-.012, x+.012, .03, .05, zf+.07, zf+.13, (230, 186, 80))
    else:
        for x, z in ((xl+.06, zf+.08), (xr-.06, zf+.08), (xl+.06, zf-L+.15), (xr-.08, back(xr-.08)+.12), (lerp(xl, xr, .4), back(lerp(xl, xr, .4))+.12)):
            g.box(x-.03, x+.03, 0, p["y0"], z-.03, z+.03, p["leg"])
    wing_case(g, xl, xr, zf, back, p["y0"], p["y1"], .035, p["sb"], p["out"], p["rim"], p["inwall"], p["board"], spec=sp)
    # keyboard well: keybed, cheeks, key slip
    ky, x0, x1 = p["ky"], p["x0"], p["x1"]
    g.box(x0, x1, ky - .07, ky - .022, zf, zf + .18, p["inwall"])
    g.box(xl, x0, p["y0"], ky + .05, zf, zf + .19, p["out"], spec=sp)
    g.box(x1, xr, p["y0"], ky + .05, zf, zf + .19, p["out"], spec=sp)
    g.box(x0, x1, ky - .07, ky - .03, zf + .17, zf + .19, p["out"], spec=sp)
    active = {e[1] for e in ev if e[0] < e[3]}
    km, kw = build_keys(g, x0, x1, zf + .17, ky, p["low"], p["nwhite"], active, nat=p["nat"], sharp=p["sharp"], spec=sp)
    sy = p["sy"]
    # interior details
    if kind == "grand":
        for fr in (.3, .55, .78):
            x = xl + Wd*fr; g.bar((x, zf - .12), (x - .14, back(x - .14) + .12), p["sb"], p["sb"] + .016, .03, (176, 132, 52))
        for hx, hz in ((.2, .5), (.45, .42), (.66, .6), (.33, .72), (.12, .8)):
            x = xl + Wd*hx; z = zf - L*hz
            if z > back(x) + .1: g.ngon(x, p["sb"] + .002, z, .045, (52, 38, 18), emit=True)
        g.bar((xl + .1, zf - .16), (xr - .3, zf - .16), sy, sy + .03, .035, (30, 28, 34))
    elif kind == "harp":
        rx, rz = xl + Wd*.58, zf - L*.42
        g.ngon(rx, p["sb"] + .002, rz, .07, (206, 156, 56), emit=True); g.ngon(rx, p["sb"] + .003, rz, .05, p["board"], emit=True)
        for k, (fx_, fz_) in enumerate(((.2, .7), (.42, .28), (.72, .52), (.16, .46), (.6, .72), (.3, .58))):
            x = xl + Wd*fx_; z = zf - L*fz_
            if z > back(x) + .06:
                c = [(206, 62, 62), (72, 104, 204), (244, 204, 84)][k % 3]
                g.ngon(x, p["sb"] + .002, z, .018, c, n=5, emit=True)
        g.bar((x0, zf - .05), (x1, zf - .05), .735, .765, .025, (66, 44, 32))
    # strings
    for m, (blk, pos) in km.items():
        if blk: continue
        x = x0 + (pos + .5)*kw; age = ring_age(ev, m)
        if kind == "grand" and m < 40:
            x2 = x + .34
            vstring(g, (x, sy + .02, zf - .14), (x2, sy + .02, back(x2) + .14), (196, 118, 64), age)
        else:
            vstring(g, (x, sy, zf - .1), (x, sy, back(x) + .07), (214, 216, 226), age)
    if kind == "forte":
        for m, (blk, pos) in km.items():
            if blk: continue
            x = x0 + (pos + .5)*kw; on = m in active
            yb = sy + (.035 if on else .008)
            g.box(x - .3*kw, x + .3*kw, yb, yb + .018, zf - .07, zf - .045, HOT if on else (64, 42, 30), emit=on)
    if kind == "grand":
        g.face([(-.36, .93, zf - .03), (.36, .93, zf - .03), (.36, 1.12, zf - .1), (-.36, 1.12, zf - .1)], p["out"], two=True, spec=sp)
    # lid + prop stick
    ang = math.radians(p["ang"])
    rot = wing_lid(g, xl, xr, zf, back, p["y1"], ang, p["lid"], p["under"], spec=sp)
    px = xl + Wd*.86; pz = (zf + back(px))/2 + .1
    tip = rot(px, p["y1"], pz)
    for off in (-.008, .008):
        g.line((tip[0] + off, p["y1"], pz), (tip[0] + off, tip[1], pz), (60, 40, 28) if kind != "grand" else (70, 66, 80))
    # fx
    for age, m, v, h, tp in ev:
        if m not in km: continue
        x = key_x(km, kw, x0, m)
        if kind == "harp" and age < .15: fx.append(("spark", (x, .79, zf - .05), age))
        if tp and age < .9: fx.append(("note", (x, sy + .2, zf - .35), age))
    if kind == "forte":
        if 1*BEAT - .1 < t < 4*BEAT - .05:
            fx.append(("text2d", 104, "PIANO...", mix((110, 86, 140), (200, 190, 220), ease((t - 1*BEAT)/.3)), 2, 0))
        if t >= 4*BEAT - .02:
            b = int(5*math.sin(clamp((t - 4*BEAT)/.3)*math.pi))
            fx.append(("text2d", 98 - b, "FORTE!", GOLD, 3, 0))

def build_square(g, t, ev, fx, s):
    out, inner, board = (124, 46, 34), (90, 34, 24), (210, 170, 112)
    for x in (-.74, .74):
        for z in (-.27, .27): g.box(x-.028, x+.028, 0, .55, z-.028, z+.028, out, spec=.4)
    g.box(-.8, .8, .55, .80, -.32, -.29, out, spec=.4)
    g.box(-.8, -.77, .55, .80, -.32, .32, out, spec=.4)
    g.box(.77, .8, .55, .80, -.32, .32, out, spec=.4)
    g.box(.12, .8, .55, .80, .29, .32, out, spec=.4)
    g.box(-.8, .12, .55, .70, .29, .32, out, spec=.4)
    g.face([(-.77, .66, -.29), (.77, .66, -.29), (.77, .66, .29), (-.77, .66, .29)], board, ref=(0, 0, 0))
    g.face([(.3, .6608, -.28), (.76, .6608, -.28), (.76, .6608, .12), (.3, .6608, .12)], (234, 202, 142), ref=(0, 0, 0))
    g.bar((.44, -.27), (.39, .11), .66, .682, .014, (120, 70, 40))
    for x in (.32, .48, .64): g.box(x-.02, x+.02, .69, .73, .32, .326, (232, 190, 84), emit=True)
    zs = np.linspace(-.26, .1, 9)
    pitches = sorted({e[1] for e in s["ev"]})
    def row(m): return pitches.index(m)*(len(zs) - 1)//max(1, len(pitches) - 1)
    hot = {}
    for age, m, v, h, tp in ev:
        if age < .6: hot[row(m)] = min(age, hot.get(row(m), 9))
    for i, z in enumerate(zs):
        vstring(g, (-.72, .70, z), (.72, .70, z + .04), (204, 198, 186), hot.get(i), amp=.004)
    g.box(-.78, .11, .62, .708, .2, .38, inner)
    active = {e[1] for e in ev if e[0] < e[3]}
    km, kw = build_keys(g, -.76, .0896, .38, .73, 41, 36, active, depth=.16)
    for age, m, v, h, tp in ev:
        if tp and age < .9: fx.append(("note", (key_x(km, kw, -.76, m), .8, zs[row(m)]), age))
    panel_lid(g, -.8, .8, .64, .80, -.32, math.radians(58), out, (198, 124, 84), spec=.4)

def build_inst(key, g, t, ev, fx, s):
    if key == "mono": build_mono(g, t, ev, fx, s)
    elif key == "dulc": build_dulc(g, t, ev, fx, s)
    elif key == "clav": build_clav(g, t, ev, fx, s)
    elif key == "square": build_square(g, t, ev, fx, s)
    else: build_wing(g, t, ev, fx, s, key)

FOOT = {"mono": .62, "dulc": .62, "clav": .66, "harp": 1.15, "forte": 1.02, "square": .95, "grand": 1.45}

def build_pedestal(g, dist, pulse, foot, pc):
    R, h = .33*dist, .045*dist
    def octo(r): return [(r*math.cos(math.pi/8 + k*math.pi/4), r*math.sin(math.pi/8 + k*math.pi/4)) for k in range(8)]
    prism(g, octo(R*1.1), -2*h, -h, pc['low_side'], pc['low_top'], gid=1)
    prism(g, octo(R), -h, 0, pc['side'], pc['top'], gid=1)
    pts = octo(R)
    for k in range(8):
        (xa, za), (xb, zb) = pts[k], pts[(k+1) % 8]
        nx, nz = (xa + xb)/2, (za + zb)/2; ln = math.hypot(nx, nz); nx, nz = nx/ln, nz/ln
        tx, tz = (xb - xa), (zb - za); tl = math.hypot(tx, tz); tx, tz = tx/tl, tz/tl
        for fr, base in ((.3, (255, 206, 110)), (.7, (255, 120, 176))):
            cx, cz = lerp(xa, xb, fr) + nx*.006*R, lerp(za, zb, fr) + nz*.006*R
            w = .045*R
            col = mix(dim(base, .3), base, pulse)
            g.face([(cx - tx*w, -.32*h, cz - tz*w), (cx + tx*w, -.32*h, cz + tz*w), (cx + tx*w, -.68*h, cz + tz*w), (cx - tx*w, -.68*h, cz - tz*w)],
                   col, ref=(0, -.5*h, 0), emit=True, gid=1)
    if foot > .01:
        g.face([(x, .002*dist, z) for x, z in octo(foot)], pc['shadow'], ref=(0, -1, 0), emit=True, gid=1)

# ---------------------------------------------------------------- pixel sprites
NOTE_SPR = ["..##.", "..#.#", "..#..", "..#..", ".##..", "###..", ".#..."]
SPARK = ["..#..", "..#..", "##.##", "..#..", "..#.."]
def sprite(d, spr, x, y, col, sc=1, shadow=True):
    for r, row in enumerate(spr):
        for c, px in enumerate(row):
            if px == '#':
                X, Y = x + c*sc, y + r*sc
                if shadow: d.rectangle([X+1, Y+1, X+sc, Y+sc], fill=SHADOW)
                d.rectangle([X, Y, X+sc-1, Y+sc-1], fill=col)

# ---------------------------------------------------------------- frame
def scene_at(T):
    if T < BAR: return None
    return min(6, int((T/BAR - 1)//2))

def frame(T, fi=0):
    FLIP[0] = 1 if fi % 2 == 0 else -1
    rend = np.zeros((RH, RW, 3), np.uint8); zb = np.zeros((RH, RW)); gb = np.zeros((RH, RW), np.int8)
    last_kick = max([k for k in KICKS if k <= T], default=-9)
    pulse = math.exp(-(T - last_kick)*7)
    si = scene_at(T)
    s = SCENES[si] if si is not None else None
    key = s["key"] if s else None
    prevkey = (SCENES[si-1]["key"] if si > 0 else None) if s else None
    kb = ease((T - s["t0"])/(.5*BEAT)) if s else 1.0
    img = (BGS[prevkey]*(1 - kb) + BGS[key]*kb).astype(np.uint8)
    pc0, pc1 = ped_cols(prevkey), ped_cols(key)
    pc = {k: mix(pc0[k], pc1[k], kb) for k in pc1}
    for x, y, ph in STARS:
        b = .5 + .5*math.sin(T*2.4 + ph)
        if b > .35: img[y, x] = mix((70, 58, 100), (230, 220, 255), b)
    dist = s["cam"] if s else SCENES[0]["cam"]
    fx = []
    scale, spin = 0.0, 0.0
    if s:
        t = T - s["t0"]
        pin = t/(.35*BEAT); scale = ease_out_back(pin); spin = -100*(1 - ease(pin))
        if si < 6:
            pout = (t - (2*BAR - .35*BEAT))/(.35*BEAT)
            if pout > 0: scale *= 1 - ease(pout); spin += 100*pout*pout
    yaw = (s["yaw"] if s else 20) + 14*math.sin(2*math.pi*T/8) + spin
    cam = Cam(dist, yaw, scale)
    gp = Geo(); build_pedestal(gp, dist, pulse, FOOT[s["key"]]*scale if s else 0, pc)
    render_geo(gp, cam, rend, zb, gb, scale=1.0)
    if s and scale > .01:
        t = T - s["t0"]
        ev = [(t - nt, m, v, h, tp) for nt, m, v, h, tp in s["ev"] if t >= nt]
        gi = Geo(); build_inst(s["key"], gi, t, ev, fx, s)
        render_geo(gi, cam, rend, zb, gb)
    outline(rend, zb, gb)
    m = gb > 0
    img[m] = rend[m]
    im = Image.fromarray(img); d = ImageDraw.Draw(im)
    # headline
    hl = "2,400 YEARS"; hw = text_w(hl, 3)
    downbeat = math.exp(-((T % BAR))*6)
    d.rectangle([(RW - hw)//2 - 8, 9, (RW + hw)//2 + 7, 36], fill=INK)
    d.rectangle([(RW - hw)//2 - 8, 9, (RW + hw)//2 + 7, 36], outline=mix(GOLD_D, (255, 230, 150), downbeat))
    center_text(d, 13, hl, CREAM, 3)
    outro = T >= OUTRO_T
    if s is None:
        k = ease(T/.5)
        center_text(d, 150, "HOW WE GOT", mix(BG_BOT, CREAM, k), 3)
        center_text(d, 180, "THE PIANO", mix(BG_BOT, GOLD, k), 3)
    else:
        t = T - s["t0"]
        prev = SCENES[si-1]["year"] if si > 0 else -800
        k = ease(t/BEAT)
        ys = year_str(int(lerp(prev, s["year"], k))) if k < 1 else s.get("ys", year_str(s["year"]))
        center_text(d, 45, ys, GOLD, 3)
        center_text(d, 74, s["name"], CREAM, 2)
        if not outro:
            fk = ease((t - .2)/.3)
            for li, line in enumerate(s["fact"]):
                center_text(d, 336 + li*19, line, mix(BG_BOT, CREAM, fk), 2)
        else:
            fk = ease((T - OUTRO_T)/.3)
            center_text(d, 336, "FROM ONE STRING", mix(BG_BOT, CREAM, fk), 2)
            center_text(d, 355, "TO 88 KEYS.", mix(BG_BOT, GOLD, fk), 2)
    # fx overlays
    for f in fx:
        if f[0] == "note":
            S = cam.screen(f[1]); a = f[2]/.9
            sprite(d, NOTE_SPR, int(S[0]) - 5 + int(4*math.sin(f[2]*8)), int(S[1]) - int(f[2]*40) - 14,
                   mix((120, 96, 140), (255, 246, 214), (1 - a)**.6), 2)
        elif f[0] == "spark":
            S = cam.screen(f[1]); sprite(d, SPARK, int(S[0]) - 2, int(S[1]) - 2, (255, 250, 210), 1, False)
        elif f[0] == "label" and scale > .6:
            S = cam.screen(f[1]); w = text_w(f[2], f[4])
            draw_text(d, int(S[0]) - w//2, int(S[1]) - 14, f[2], f[3], f[4])
        elif f[0] == "text2d":
            center_text(d, f[1], f[2], f[3], f[4])
    if T >= 16*BAR:   # final burst
        tb = T - 16*BAR
        rr = np.random.default_rng(11)
        for k in range(26):
            ang = rr.random()*6.28; sp = 40 + rr.random()*80
            x = CX + math.cos(ang)*sp*min(1, tb*2.2); y = 220 + math.sin(ang)*sp*.8*min(1, tb*2.2) + tb*tb*20
            col = mix((255, 240, 180), (120, 96, 140), clamp(tb/2.2))
            if tb < 2.2: sprite(d, SPARK, int(x), int(y), col, 1, False)
    # timeline
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
rng = np.random.default_rng(3)
def _t(n): return np.arange(n)/SR

def kick(v):
    n = int(.4*SR); t = _t(n); f = 46 + 110*np.exp(-t*34)
    x = np.sin(2*np.pi*np.cumsum(f)/SR)*np.exp(-t*7.5)
    x[:180] += rng.normal(0, .2, 180)
    return np.tanh(1.7*x)*v
def snare(v):
    n = int(.25*SR); t = _t(n); nz = np.diff(rng.normal(0, 1, n+1))
    nz = lfilter([1], [1, -0.55], nz)
    return (nz*np.exp(-t*17)*.32 + np.sin(2*np.pi*186*t)*np.exp(-t*30)*.55)*v
def hat(v, open_=False):
    n = int((.3 if open_ else .06)*SR); t = _t(n); nz = np.diff(np.diff(rng.normal(0, 1, n+2)))
    return nz*np.exp(-t*(9 if open_ else 75))*.09*v
def crash(v):
    n = int(2.4*SR); t = _t(n); nz = np.diff(rng.normal(0, 1, n+1))
    return nz*np.exp(-t*2.0)*np.minimum(1, t/.004)*.16*v

def bass(m, dur, v):
    f = midi_hz(m); n = int(dur*SR); t = _t(n)
    x = np.sin(2*np.pi*f*t) + .3*np.sin(4*np.pi*f*t) + .12*np.sin(6*np.pi*f*t)
    env = np.minimum(1, t/.006)*np.exp(-t*1.8)
    r = int(.03*SR); env[-r:] *= np.linspace(1, 0, r)
    return np.tanh(1.4*x*env)*v

def pad(notes, dur, v):
    n = int(dur*SR); t = _t(n); x = np.zeros(n)
    for m in notes:
        f = midi_hz(m)
        for det in (-.0025, .0025):
            for k in range(1, 6): x += np.sin(2*np.pi*f*(1 + det)*k*t + k)/k**1.6
    env = np.minimum(1, t/.35)*np.minimum(1, (dur - t)/.4)
    return lfilter([.12], [1, -.88], x*env)*v

def ks_pluck(f, dur, bright=0.5, decay=0.996, seed=0):
    N = int(SR/f); n = int(dur*SR); r = np.random.default_rng(seed)
    buf = r.uniform(-1, 1, N)
    for _ in range(int((1-bright)*4)): buf = 0.5*(buf + np.roll(buf, 1))
    out = np.zeros(n + N + 1); out[:N] = buf; i = N
    while i < n:
        j = min(n, i + N)
        prev = out[i-N:j-N]; prev1 = out[i-N-1:j-N-1] if i-N-1 >= 0 else np.concatenate(([0], out[i-N:j-N-1]))
        out[i:j] = decay*0.5*(prev + prev1); i = j
    return out[:n]

def additive(f, dur, vel, harm=12, B=0.0004, tilt=1.2, d1=1.2, attack=0.004, vibr=0):
    n = int(dur*SR); tt = _t(n); sig = np.zeros(n)
    for k in range(1, harm+1):
        fk = f*k*math.sqrt(1 + B*k*k)
        if fk > SR/2.2: break
        amp = vel ** (0.12*(k-1)) / k**tilt
        dk = d1*(1 + 0.45*(k-1))*(1 + f/900)
        ph = 2*math.pi*fk*tt
        if vibr: ph = ph + vibr*np.sin(2*math.pi*5.5*tt)*k*40
        sig += amp*np.exp(-dk*tt)*(np.sin(ph) + 0.6*np.sin(ph*1.0007 + 1.3))
    return sig*np.minimum(1, tt/attack)

def inst_note(k, m, v, nt, i):
    f = midi_hz(m)
    if k == "mono":   return ks_pluck(f, 2.2, .35, .9985, i)*.9*v
    if k == "dulc":   return (ks_pluck(f, 1.8, .85, .9975, i)*.55 + ks_pluck(f*1.003, 1.8, .85, .9975, i+9)*.35)*v
    if k == "harp":   return (ks_pluck(f, 1.3, 1., .993, i)*.6 + ks_pluck(f*2, 1.3, 1., .99, i+5)*.2)*v
    if k == "clav":   return additive(f, 1.9, .5, harm=8, tilt=1.6, d1=1.5, attack=.01, vibr=.004 if nt > 2.8 else 0)*.4*v
    if k == "forte":  return additive(f, 2.0, v, harm=10, tilt=1.3, d1=2.4, attack=.003)*.55*(.3 + v)
    if k == "square": return additive(f, 2.0, v, harm=10, tilt=1.35, d1=1.8, attack=.003)*.5*v
    dur = 5.0 if nt >= 12*BEAT else 2.6
    x = additive(f, dur, v, harm=14, tilt=1.1, d1=.7, attack=.002)*.5*(.4 + v)
    if m < 40: x = x + additive(f*2, dur, v, harm=8, tilt=1.2, d1=.8)[:len(x)]*.2
    return x

def render_audio(path="audio_lp.wav"):
    n = int((TOTAL + 3)*SR)
    drums, bassb, padb, inst = (np.zeros(n) for _ in range(4))
    def add(buf, T0, x, g=1.):
        a = int(T0*SR); e = min(n, a + len(x)); buf[a:e] += x[:e-a]*g
    for T0, k, v in DRUMS:
        add(drums, T0, {'kick': kick, 'snare': snare, 'hat': hat, 'crash': crash}[k](v) if k != 'ohat' else hat(v, True))
    for b in range(1, 17):
        T0 = b*BAR; root = CH[BARS[b]][0]
        if b == 16:
            add(bassb, T0, bass(root, 2.6, .9)); continue
        for beat, mm, dur in ((0, root, .9), (1.5, root, .4), (2.5, root, .9), (3.5, root + 7, .4)):
            add(bassb, T0 + beat*BEAT, bass(mm, dur*BEAT, .8))
    for b in range(17):
        add(padb, b*BAR, pad(CH[BARS[b]][1], 2.6 if b == 16 else BAR + .05, .5 if b else .3))
    def rms(x):
        x = x[np.abs(x) > 1e-6]; return np.sqrt((x**2).mean()) if len(x) else 1.
    def at(x, db): return x * (10**(db/20)) / rms(x)
    for s in SCENES:      # level each instrument separately so quiet ones aren't buried
        sb = np.zeros(n)
        for i, (nt, m, v, h, tp) in enumerate(s["ev"]):
            add(sb, s["t0"] + nt, inst_note(s["key"], m, v, nt, i))
        a = int(s["t0"]*SR); e = int((s["t0"] + 2*BAR)*SR)
        g = (10**(-15/20)) / (np.sqrt((sb[a:e]**2).mean()) + 1e-9)
        inst += sb * g
    drums, bassb, padb = at(drums, -24), at(bassb, -27), at(padb, -32)
    wet_in = inst + padb*.6
    r = np.random.default_rng(1); irn = int(1.4*SR)
    ir = r.normal(0, 1, irn)*np.exp(-np.arange(irn)/SR*3.8); ir[0] = 0; ir /= np.sqrt((ir**2).sum())
    Lf = 1 << (n + irn).bit_length()
    wet = np.fft.irfft(np.fft.rfft(wet_in, Lf)*np.fft.rfft(ir, Lf), Lf)[:n]
    out = drums + bassb + padb + inst + at(wet, -25)
    out = out[:int(TOTAL*SR)]
    fade = int(.8*SR); out[-fade:] *= np.linspace(1, 0, fade)
    out = np.tanh(1.6*out/np.abs(out).max())/np.tanh(1.6)*.94
    with wave.open(path, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(SR); w.writeframes((out*32767).astype(np.int16).tobytes())
    return path

# ---------------------------------------------------------------- main
if __name__ == "__main__":
    if "--still" in sys.argv:
        for ts in sys.argv[sys.argv.index("--still") + 1:]:
            T = float(ts); frame(T, int(T*FPS)).resize((RW*2, RH*2), Image.NEAREST).save(f"still_{ts}.png")
        sys.exit()
    wav = render_audio()
    nframes = int(TOTAL*FPS)
    p = subprocess.Popen(["ffmpeg", "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24",
        "-s", f"{RW}x{RH}", "-r", str(FPS), "-i", "-", "-i", wav,
        "-vf", f"scale={RW*UP}:{RH*UP}:flags=neighbor", "-c:v", "libx264", "-pix_fmt", "yuv420p",
        "-crf", "16", "-preset", "medium", "-c:a", "aac", "-b:a", "192k", "-shortest", OUT], stdin=subprocess.PIPE)
    for fi in range(nframes):
        p.stdin.write(frame(fi/FPS, fi).tobytes())
        if fi % 150 == 0: print("frame", fi, "/", nframes, flush=True)
    p.stdin.close(); p.wait()
    print("wrote", OUT, f"{TOTAL:.1f}s")
