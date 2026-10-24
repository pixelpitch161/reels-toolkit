"""#26 Which Instrument Wins? - renderer.

sky + sun -> far landscape -> colosseum stands (painter, sun + torch light) -> sand floor per pixel (sun with real
shadows of the stands, ring and balls; torch and brazier pools; contact AO) -> pit + the eliminated -> the ring
(rotating, gap cut) -> 16 glass balls with the Pixel Pitch instrument sprites -> locked DOF on the ring -> glows ->
god rays -> bloom -> grade -> scrim -> captions, ball names, roster.

    python -m reelkit.sheet render.py test/sheet.png 0.5 3.6 9 21.5 40.5 46 50 53 59.5
    python -m reelkit.video render.py audio.wav out.mp4"""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', '..'))
sys.path.insert(0, HERE)
from reelkit.config import W, H, FPS, BAYER_FULL, SAFE, SAFE_CX, safe_overshoot, ss, ease, mixc
from reelkit.camera import Cam, interp_keys
from reelkit.raster import Frame, painter, apply_glows
from reelkit.lighting import Lights, shade, fog, sky_gradient
from reelkit.shadows import MaskGrid, directional_mask, Occluders, contact_ao, proj_dir
from reelkit.lens import coc_map, dof
from reelkit.post import god_rays, bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, label, roster_grid, CREAM, AMBER, INK
from reelkit.font import ADV
YY_, XX_ = np.mgrid[0:H, 0:W].astype(np.float64)
import timeline as T
import scene as S
import sim
import sprites as SP
from pp_bridge import NAMES

RUN = sim.load()
NSIM = len(RUN['P'])
T_WIN = RUN['T_WIN']
T_OUT = RUN['T_OUT']
T_LAND = RUN['T_LAND']
ORDER = [int(b) for b in np.argsort(np.where(T_OUT < 0, 1e9, T_OUT))[:15]]
WINNER = int(np.where(T_OUT < 0)[0][0])
PLACE = {b: 16 - k for k, b in enumerate(ORDER)}
EV = RUN['EV']
P0 = RUN['P0']

COLORS = [(255, 96, 96), (255, 160, 64), (250, 214, 72), (160, 226, 80), (72, 206, 120), (64, 214, 200),
          (80, 170, 255), (120, 130, 255), (176, 116, 255), (232, 104, 232), (255, 126, 176), (210, 168, 116),
          (236, 236, 244), (120, 232, 255), (255, 200, 150), (150, 255, 196)]
# per-instrument view inside the ball: (yaw centre, yaw swing, roll) in degrees: the most readable side, never edge-on
VIEW = {'PIANO': (225, 22, 0), 'VIOLIN': (12, 26, 28), 'CELLO': (-12, 26, -26), 'HARP': (8, 26, 0),
        'FLUTE': (0, 18, 32), 'CLARINET': (0, 22, 0), 'SAX': (196, 24, 0), 'TRUMPET': (0, 24, 18),
        'TROMBONE': (0, 18, 34), 'TUBA': (0, 32, 0), 'HORN': (30, 28, 0), 'GUITAR': (12, 28, 30),
        'DRUMS': (12, 30, 0), 'ACCORDION': (72, 22, 0), 'BAGPIPES': (42, 22, 0), 'UKULELE': (12, 28, -30)}

# ---------------------------------------------------------------- camera
AIM = S.RING_C.copy()                                # ring centre on the frame's centre column (x 180), under the captions
SY = 258
KEYS = [  # (t, target, dist, yaw, pitch, fov, sy) - gentle 2.5D drift, one push-in for the duel, pull back for the win
    (0.0, AIM, 14.6, 0.0, 2.0, 50, SY),
    (7.0, AIM, 14.8, -1.5, 3.0, 50, SY),
    (20.0, AIM, 14.6, 1.2, 2.4, 50, SY),
    (36.0, AIM, 14.5, -1.0, 2.2, 50, SY),
    (44.0, AIM, 13.9, 0.0, 2.0, 50, SY),
    (49.6, AIM, 13.9, 0.5, 2.0, 50, SY),
    (52.5, AIM, 15.0, 1.2, 3.2, 50, SY),
    (58.0, AIM, 14.9, 0.5, 2.6, 50, SY),
    (61.0, AIM, 14.6, 0.0, 2.0, 50, SY),
]

# ---------------------------------------------------------------- lights + static shadows
FLOOR = MaskGrid((-24.0, 24.0, -30.0, 18.0, 0.06))
BACK_FACES = [Q for _, items in S.BACK for Q, n, c, e in items]
WALL_MASK = directional_mask(FLOOR, BACK_FACES + [Q for Q, n, c in S.BRAZIER_FACES], S.SUN, blur=1.5)
WP = 61.0 / 160                                     # flicker / flutter periods: whole cycles in 61 s


def flick(t, ph):
    return 0.86 + 0.08 * math.sin(2 * math.pi * (t / (61.0 / 431) + ph)) + 0.06 * math.sin(2 * math.pi * (t / (61.0 / 263) + 2 * ph))


def win_k(t):
    """0 -> 1: the colosseum lights up after the crown"""
    return ss(T_WIN + 0.3, T_WIN + 3.0, t) * (1 - ss(T.T_RESET + 0.6, 60.6, t))


def lights(t, cam):
    wk = win_k(t)
    L = Lights(amb=(0.34 + 0.08 * wk, 0.29 + 0.06 * wk, 0.40), sky=(0.08, 0.08, 0.14))
    L.view = cam.view
    L.key('sun', S.SUN, (1.15, 0.70, 0.36))
    L.fill(np.array([0.1, 0.35, 1.0]), (0.26, 0.20, 0.20))         # warm bounce from the sand toward the stands
    for k, p in enumerate(S.TORCHES):
        L.point(p + [0, 0.35, 0], np.array([1.0, 0.55, 0.22]) * (0.55 + 0.35 * wk) * flick(t, k * 0.37), 2.2)
    for k, p in enumerate(S.BRAZIERS):
        L.point(p + [0, 0.5, 0], np.array([1.0, 0.56, 0.24]) * 1.25 * flick(t, 0.5 + k * 0.29), 3.0)
    return L


# ---------------------------------------------------------------- ball states (honest sim, then the scripted tail)
PIT_SLOTS = []
for k in range(15):
    a = k * 2.399963
    rr = 0.55 + 0.42 * math.sqrt(k + 0.5)
    PIT_SLOTS.append(np.array([0.0 + rr * math.cos(a) * 1.05, S.PIT_Y + T.R_BALL, 0.9 + rr * math.sin(a) * 0.62]))
SLOT_OF = {b: PIT_SLOTS[k] for k, b in enumerate(ORDER)}
T_HOVER0 = T_WIN + T.T_LIFT
T_HOVER1 = T_HOVER0 + T.LIFT_DUR
HOVER = np.array([0.0, 0.05])
HOVER_Z = 4.2                                       # the champion floats out toward the viewer
DOF_SHARP = 0.50                                    # crisp band (relative depth): ring, every ball, the pit and the champion
DOF_K = 0.030


def world_of(xy, z):
    return np.array([xy[0], S.RING_C[1] + xy[1], z])


def sim_frame(t):
    return min(NSIM - 1, max(0, int(round(t * FPS)) - 1))


def reset_start(b):
    """when each ball lifts off for the fly-back (first out leaves last); the champion goes mid-way"""
    return T.T_RESET + 0.06 * PLACE[b] if b != WINNER else T.T_RESET + 0.5


def ball_world(b, t):
    """(world position, state 0 in play / 1 falling / 2 in the pit / 3 scripted, sim xy for the yaw)"""
    f = sim_frame(t)
    reset0 = reset_start(b)
    if t >= reset0:                                        # everyone flies back into formation by 61.0 (= frame 0)
        start, xy0 = _tail_pos(b, reset0)
        u = ease((t - reset0) / (T.DUR - reset0))
        tgt = world_of(P0[b], 0.0)
        arc = np.array([0, 1.2 * math.sin(math.pi * u), 0]) if b != WINNER else np.zeros(3)
        p = start + (tgt - start) * u + arc
        xy = (p[0], p[1] - S.RING_C[1])
        return p, 3, xy
    if b == WINNER and t >= T_HOVER0:
        p, xy = _tail_pos(b, t)
        return p, 3, xy
    if b != WINNER and T_OUT[b] >= 0 and t > T_LAND[b] >= 0:
        p, xy = _tail_pos(b, t)
        return p, 2, xy
    st = int(RUN['S'][f, b])
    xy = RUN['P'][f, b]
    return world_of(xy, RUN['Z'][f, b]), st, xy


def _tail_pos(b, t):
    if b == WINNER:
        fw = sim_frame(T_HOVER0)
        a = world_of(RUN['P'][fw, b], 0.0)
        hv = world_of(HOVER, HOVER_Z)
        u = ease((t - T_HOVER0) / T.LIFT_DUR)
        bob = 0.08 * math.sin(2 * math.pi * (t - T_HOVER1) / (2 * T.BAR)) * ss(T_HOVER1 - 0.3, T_HOVER1 + 0.5, t)
        p = a + (hv - a) * u + np.array([0, bob, 0])
        return p, (p[0], p[1] - S.RING_C[1])
    fl = min(NSIM - 1, int(T_LAND[b] * FPS))
    land = world_of(RUN['P'][fl, b], RUN['Z'][fl, b])
    land[1] = S.PIT_Y + T.R_BALL
    slot = SLOT_OF[b]
    u = ease((t - T_LAND[b]) / 0.9)
    hop = 0.35 * abs(math.sin(math.pi * min(1.0, (t - T_LAND[b]) / 0.45))) * (1 - u)
    p = land + (slot - land) * u + np.array([0, hop, 0])
    return p, (p[0], p[1] - S.RING_C[1])


def ball_yaw(b, xy, t):
    yc, sw, _ = VIEW[NAMES[b]]
    yaw = yc + sw * math.sin(1.25 * xy[0] + 0.45 * xy[1])
    if b == WINNER and T_HOVER0 < t < T.T_RESET + 2.6:
        k = ss(T_HOVER0, T_HOVER1, t) * (1 - ss(T.T_RESET, T.T_RESET + 2.5, t))
        yaw += k * sw * 0.9 * math.sin(2 * math.pi * (t - T_HOVER1) / (2 * T.BAR))    # the champion sways in the spotlight
    return yaw


SOLO_T = [T.T_SOLO + bt * T.BEAT for bt, m, d in T.SOLO]


def solo_env(t):
    """pulse on each note of the champion's solo"""
    past = [s for s in SOLO_T if s <= t]
    return math.exp(-(t - past[-1]) / 0.18) if past else 0.0


def recent_hit(b, t, window=0.25):
    m = (EV[:, 1] == b) & (EV[:, 0] <= t) & (EV[:, 0] > t - window) & (EV[:, 2] != 1) & (EV[:, 3] > 1.5)
    if not m.any():
        return 0.0
    return float(1 - (t - EV[m, 0].max()) / window)


def gap_width(t):
    """visual gap: the run's own, sealing over 0.25 s at the win; closed at frame 0"""
    f = sim_frame(t)
    if t < T_WIN:
        return float(RUN['GW'][f]) if t < NSIM / FPS else 0.0
    w_end = float(RUN['GW'][sim_frame(T_WIN - 1.0 / FPS)])
    return w_end * (1 - ss(T_WIN, T_WIN + 0.25, t))


def ring_angle(t):
    return RUN['TH0'] + T.SPIN * t


# ---------------------------------------------------------------- ring geometry (rebuilt per frame: it turns)
RING_SEG = 72
R_IN, R_OUT, ZD = T.R_RING, T.R_RING + T.RING_T, 0.15
RING_FRONT = np.array([176, 44, 56])
RING_SIDE = np.array([120, 30, 42])
GOLD = np.array([246, 196, 92])


def ring_faces(t):
    th = ring_angle(t)
    w = gap_width(t)
    C = S.RING_C
    out = []
    da = 2 * math.pi / RING_SEG
    for k in range(RING_SEG):
        a0, a1 = th + k * da, th + (k + 1) * da
        # clip to the wall (skip the gap centred on th)
        lo, hi = w / 2, 2 * math.pi - w / 2
        u0, u1 = k * da, (k + 1) * da
        u0, u1 = max(u0, lo), min(u1, hi)
        if u1 <= u0:
            continue
        a0, a1 = th + u0, th + u1
        c0, s0, c1, s1 = math.cos(a0), math.sin(a0), math.cos(a1), math.sin(a1)
        P = lambda r, c, s, z: C + np.array([r * c, r * s, z])
        fq = np.array([P(R_IN, c0, s0, ZD), P(R_IN, c1, s1, ZD), P(R_OUT, c1, s1, ZD), P(R_OUT, c0, s0, ZD)])
        out.append((fq, np.array([0, 0, 1.0]), RING_FRONT, 'front'))
        bq = np.array([P(R_IN, c0, s0, -ZD), P(R_OUT, c0, s0, -ZD), P(R_OUT, c1, s1, -ZD), P(R_IN, c1, s1, -ZD)])
        out.append((bq, np.array([0, 0, -1.0]), RING_FRONT, 'back'))
        am = (a0 + a1) / 2
        nin = -np.array([math.cos(am), math.sin(am), 0])
        iq = np.array([P(R_IN, c0, s0, ZD), P(R_IN, c0, s0, -ZD), P(R_IN, c1, s1, -ZD), P(R_IN, c1, s1, ZD)])
        out.append((iq, nin, RING_SIDE * 1.15, 'in'))
        oq = np.array([P(R_OUT, c0, s0, ZD), P(R_OUT, c1, s1, ZD), P(R_OUT, c1, s1, -ZD), P(R_OUT, c0, s0, -ZD)])
        out.append((oq, -nin, RING_SIDE, 'out'))
    if w > 1e-3:
        for sgn in (-1, 1):
            a = th + sgn * w / 2
            c, s = math.cos(a), math.sin(a)
            tdir = np.array([-s, c, 0]) * sgn
            q = np.array([C + [R_IN * c, R_IN * s, ZD], C + [R_OUT * c, R_OUT * s, ZD], C + [R_OUT * c, R_OUT * s, -ZD], C + [R_IN * c, R_IN * s, -ZD]])
            out.append((q, -tdir, GOLD, 'cap'))
    return out


# ---------------------------------------------------------------- sprites + glass balls
def draw_ball(arr, dep, cam, b, p, t, alpha=1.0, glow=0.0):
    Pc = cam.to_cam(p[None])[0]
    if Pc[2] < 0.5:
        return None
    s = cam.proj(Pc[None])[0]
    rp = T.R_BALL * cam.f / Pc[2]
    ri = int(round(rp))
    x0, y0 = int(round(s[0])) - ri - 1, int(round(s[1])) - ri - 1
    n = 2 * ri + 3
    if x0 + n <= 0 or y0 + n <= 0 or x0 >= W or y0 >= H:
        return None
    yy, xx = np.mgrid[0:n, 0:n]
    dx = (xx + x0 + 0.5 - s[0]) / rp
    dy = (yy + y0 + 0.5 - s[1]) / rp
    d2 = dx * dx + dy * dy
    inside = d2 <= 1.0
    rim = inside & (d2 > (1 - 1.6 / rp) ** 2)
    edge = (d2 > 1.0) & (d2 <= (1 + 1.2 / rp) ** 2)
    xa, ya = max(0, x0), max(0, y0)
    xb, yb = min(W, x0 + n), min(H, y0 + n)
    sl = (slice(ya - y0, yb - y0), slice(xa - x0, xb - x0))
    reg = arr[ya:yb, xa:xb]
    dreg = dep[ya:yb, xa:xb]
    col = np.array(COLORS[b], float)
    # glass interior: light tint of the ball colour, sphere-shaded toward the sun (upper right), banded
    nz = np.sqrt(np.clip(1 - d2, 0, 1))
    lam = np.clip(0.55 * dx[...] * 0.35 - 0.55 * dy * 0.6 + nz * 0.55, 0, 1)
    band = np.floor(lam * 4 + BAYER_FULL[:n, :n] * 0.9) / 4
    inner = (col * 0.35 + np.array([236, 230, 220]) * 0.65)[None, None] * (0.72 + 0.34 * band[..., None])
    m = inside[sl]
    reg[m] = reg[m] * (1 - 0.88 * alpha) + inner[sl][m] * 0.88 * alpha
    # instrument sprite
    rs = max(6, int(round(rp * 0.82)))
    yaw = ball_yaw(b, ((p[0], p[1] - S.RING_C[1])), t)
    spr = SP.sprite(NAMES[b], rs, yaw)
    sn = spr.shape[0]
    sx0, sy0 = int(round(s[0])) - sn // 2, int(round(s[1])) - sn // 2 + 1
    ax0, ay0 = max(0, sx0), max(0, sy0)
    ax1, ay1 = min(W, sx0 + sn), min(H, sy0 + sn)
    if ax1 > ax0 and ay1 > ay0:
        sub = spr[ay0 - sy0:ay1 - sy0, ax0 - sx0:ax1 - sx0]
        ms = sub[..., 3] > 0
        rg = arr[ay0:ay1, ax0:ax1]
        lit = sub[..., :3].astype(np.float32) * (1.05 + 0.25 * glow)
        rg[ms] = rg[ms] * (1 - alpha) + np.clip(lit[ms], 0, 255) * alpha
    # rim in the ball colour (brightens on a hit), dark outline outside, specular glints
    rc = np.clip(col * (0.85 + 0.5 * glow) + 40 * glow, 0, 255)
    mr = rim[sl]
    reg[mr] = reg[mr] * (1 - alpha) + rc * alpha
    me = edge[sl]
    reg[me] = reg[me] * (1 - 0.8 * alpha) + np.array([24, 16, 30]) * 0.8 * alpha
    spec = inside & ((dx - 0.42) ** 2 + (dy + 0.48) ** 2 < (1.7 / rp) ** 2 + 0.012)
    ms2 = spec[sl]
    reg[ms2] = reg[ms2] * (1 - alpha) + np.array([255, 252, 240]) * alpha
    dreg[m | me] = Pc[2]
    return s, rp


# ---------------------------------------------------------------- captions (from the run)
def _cap_times():
    caps = [(0.0, 2.3, ['16 *INSTRUMENTS*', '1 *SURVIVOR*']),
            (2.4, 5.4, ['FALL OUT =', "*YOU'RE OUT*"]),
            (6.7, 8.9, ['*BAGPIPES*', 'IS STUCK!']),
            (9.05, 11.2, ['*BAGPIPES*', 'IS OUT']),
            (15.75, 18.0, ['*2 AT ONCE!*']),
            (19.5, 20.8, ['*UKULELE*', 'HANGING ON...']),
            (20.85, 22.9, ['*UKULELE*', 'IS OUT']),
            (23.45, 26.2, ['*VIOLIN*', 'STILL ALIVE?!']),
            (36.3, 39.2, ['*FINAL 4*']),
            (40.3, 43.0, ['*FINAL 3*']),
            (43.2, 45.4, ['*VIOLIN*', 'VS *CELLO*']),
            (T_WIN + 0.05, 51.6, ['*CELLO*', 'WINS!']),
            (51.7, 58.25, ['WHO DID YOU', '*PICK?*']),
            (58.35, 61.0, ['*RUN IT BACK*'])]
    return caps


CAPTIONS = _cap_times()

# ---------------------------------------------------------------- static draw lists
FAR_COL = np.array([120, 70, 110])


def sky(cam):
    img = sky_gradient(cam, (40, 38, 104), (244, 132, 92), horizon2=(200, 84, 124), glow_dir=S.SUN,
                       glow_col=(255, 180, 110), glow_k=0.28)
    return img


def backdrop(fr, cam, L, t):
    # far ridges + aqueduct (hazy silhouettes in the sunset), then the stands, near-to-far by painter
    for Q, n in S.FAR1:
        fr.poly(cam, Q, np.array([196, 104, 118]), depth=False)
    for Q, n in S.FAR2:
        fr.poly(cam, Q, np.array([162, 84, 110]), depth=False)
    for Q in S.AQUEDUCT:
        fr.poly(cam, Q, np.array([150, 80, 104]), depth=False)
    for base, hgt in S.CYPRESS:
        q = np.array([base + [-0.7, 0, 0], base + [0.7, 0, 0], base + [0.25, hgt, 0], base + [-0.25, hgt, 0]])
        fr.poly(cam, q, np.array([96, 54, 84]), depth=False)
    items = []
    for C, its in S.BACK:
        d = cam.depth(C)[0]
        if d < 1.0:
            continue
        nf = its[0][1]
        if np.dot(nf, cam.pos - its[0][0].mean(0)) <= 0 and nf[1] < 0.5:
            continue
        items.append((d, its))
    items.sort(key=lambda a: -a[0])
    allQ = [(Q, n, c, e) for _, its in items for Q, n, c, e in its]
    if not allQ:
        return
    N = np.array([n for _, n, _, _ in allQ])
    Cs = np.array([Q.mean(0) for Q, _, _, _ in allQ])
    A = np.array([c for _, _, c, _ in allQ])
    vis = np.ones((len(allQ), 1), np.float32)
    cols = shade(L, N, Cs, A, kvis=vis, rimk=0.7)
    dist = np.linalg.norm(Cs - cam.pos, axis=1)
    cols = fog(cols, dist, np.array([226, 150, 132]), 32, 80, 0.2)
    for (Q, n, c, e), cc in zip(allQ, cols):
        fr.poly(cam, Q, cc)


def crowd(arr, cam, t):
    beat = int(t / T.BEAT)
    wk = win_k(t)
    for a, s, ci, ph in S.CROWD:
        r0 = S.PODIUM_R + s * S.STEP_W + 0.55
        y0 = S.PODIUM_H + s * S.STEP_H + 0.35
        p = S.arena_pt(a, r0, y0)
        sp = cam.wp(p[None])[0]
        x, y = int(sp[0]), int(sp[1])
        if (beat + ci) % 2 == 0 and (t % T.BEAT) < 0.18:
            y -= 1
        if not (0 <= x < W - 1 and 0 <= y < H - 2):
            continue
        c = S.CROWD_COLS[ci] * 0.62
        arr[y:y + 2, x] = c
        if wk > 0 and ph < wk * 0.55:                    # phone lights / lanterns raised when the winner is crowned
            tw = 0.6 + 0.4 * math.sin(2 * math.pi * (t / (61.0 / 122) + ph * 7))
            arr[y - 1, x] = np.minimum(255, arr[y - 1, x] * 0.3 + np.array([255, 230, 170]) * tw)


def pennants(fr, cam, t):
    for (p, ph), col in zip(S.PENNANTS, S.PENNANT_COLS):
        top = p + [0, 2.2, 0]
        fr.line(cam, [p, top], (70, 50, 46), 1)
        wv = lambda u: 0.25 * math.sin(2 * math.pi * (t / (61.0 / 80) - u * 0.8 + ph))
        q = np.array([top, top + [1.4, wv(1.0) * 0.6 - 0.3, wv(1.0)], top + [0, -0.7, 0]])
        fr.poly(cam, q, col * 0.9, depth=False)


def sun_over(img, depth, s, cam):
    """the sun disc over the graded frame (the top scrim would grey a white-hot disc): banded, only on sky pixels"""
    R0 = 0.024 * cam.f
    x0, x1 = int(max(0, s[0] - 3 * R0)), int(min(W, s[0] + 3 * R0 + 1))
    y0, y1 = int(max(0, s[1] - 3 * R0)), int(min(H, s[1] + 3 * R0 + 1))
    if x1 <= x0 or y1 <= y0:
        return
    yy, xx = np.mgrid[y0:y1, x0:x1]
    d = np.hypot(xx + 0.5 - s[0], yy + 0.5 - s[1]) / R0
    sky = depth[y0:y1, x0:x1] > 1e5
    reg = img[y0:y1, x0:x1].astype(np.float32)
    halo = np.clip(1 - (d - 1) / 2.0, 0, 1) ** 2
    halo = np.floor(halo * 4 + BAYER_FULL[y0:y1, x0:x1] * 0.95) / 4
    m = sky & (d > 1)
    reg[m] = np.minimum(255, reg[m] + halo[m, None] * np.array([150, 60, 20]) * 0.55)
    for rr, col in ((1.0, (255, 120, 64)), (0.8, (255, 172, 96)), (0.55, (255, 222, 160)), (0.3, (255, 246, 214))):
        mm = sky & (d <= rr)
        reg[mm] = col
    img[y0:y1, x0:x1] = reg.astype(np.uint8)


def pigeons(img, cam, t):
    for k in range(5):
        per = 61.0 / (7 + k)
        a = 2 * math.pi * (t / per + k * 0.21)
        c = np.array([-9.0 + k * 1.3, 21.0 + 1.5 * math.sin(k), -26.0])
        p = c + np.array([5.5 * math.cos(a), 1.2 * math.sin(2 * a), 2.0 * math.sin(a)])
        s = cam.wp(p[None])[0]
        x, y = int(s[0]), int(s[1])
        flap = int(t * 8 + k) % 2
        if 2 <= x < W - 3 and 2 <= y < H - 3:
            col = np.array([62, 40, 60])
            img[y, x] = col
            img[y - flap, x - 1] = col
            img[y - flap, x + 1] = col
            img[y - 1 + flap, x - 2] = col
            img[y - 1 + flap, x + 2] = col


# ---------------------------------------------------------------- frame
def render(f):
    t = f / FPS
    cam = interp_keys(KEYS, t)
    L = lights(t, cam)
    wk = win_k(t)
    fr = Frame(sky(cam))
    ss_ = cam.screen_of_dir(S.SUN)
    if ss_ is not None:
        fr.disc(ss_, 0.023 * cam.f, (255, 112, 52), n=16)
        fr.disc(ss_, 0.018 * cam.f, (255, 160, 84), n=16)
        fr.disc(ss_, 0.012 * cam.f, (255, 214, 150), n=16)
        fr.glow(ss_[0], ss_[1], 0.07 * cam.f, (255, 96, 40), 0.5)
    backdrop(fr, cam, L, t)
    pennants(fr, cam, t)
    arr, depth = fr.array(), fr.depth()
    crowd(arr, cam, t)
    fr.set_array(arr, depth)
    # ---------------- sand floor: per-pixel sun (stands, ring, balls), fire pools, AO
    arr, depth = fr.array(), fr.depth()
    dw = cam.pixel_rays(XX_, YY_)
    below = dw[..., 1] < -1e-4
    ys, xs = np.nonzero(below)
    # dynamic shadow casters on the floor: ring faces + balls (projected along the sun)
    rf = ring_faces(t)
    dyn = [proj_dir(Q, S.SUN) for Q, n, c, kind in rf if kind == 'front']
    states = [ball_world(b, t) for b in range(16)]
    for p, st, xy in states:
        ang = np.linspace(0, 2 * math.pi, 11)[:-1]
        circ = np.array([[p[0] + T.R_BALL * math.cos(a), p[1] + T.R_BALL * math.sin(a), p[2]] for a in ang])
        if p[1] - T.R_BALL > 0.05:
            dyn.append(proj_dir(circ, S.SUN))
    DYN = FLOOR.raster(dyn, blur=0.8)
    if len(xs):
        X, Z, d = cam.ground_hit(xs.astype(float), ys.astype(float))
        rr = np.hypot(X - S.ARENA_C[0], Z - S.ARENA_C[2])
        ok = rr < S.PODIUM_R
        inpit = np.hypot(X, Z) < S.PIT_R
        ok &= ~inpit
        xs, ys, X, Z, d = xs[ok], ys[ok], X[ok], Z[ok], d[ok]
        alb = S.sand_albedo(X, Z)
        sh = np.maximum(FLOOR.sample(WALL_MASK, X, Z), FLOOR.sample(DYN, X, Z))
        I = np.tile(L.amb + L.sky, (len(X), 1))
        I += ((1 - sh) * 0.42)[:, None] * L.keys[0][2][None]          # grazing sun: art-directed strength
        for lp, lc, R_ in L.points:
            dist = np.sqrt((lp[0] - X) ** 2 + lp[1] ** 2 + (lp[2] - Z) ** 2)
            I += (lp[1] / dist / (1 + (dist / R_) ** 2))[:, None] * lc[None] * 1.1
        feet = [(0, 0, S.PIT_R + 0.05, 0.35)] + [(b[0], b[2], 0.3, 0.45) for b in S.BRAZIERS]
        I *= contact_ao(X, Z, feet, 0.4)[:, None]
        col = alb * I
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = d
    fr.set_array(arr, depth)
    # braziers
    for Q, n, c in S.BRAZIER_FACES:
        if np.dot(n, cam.pos - Q.mean(0)) > 0:
            cc = shade(L, n[None], Q.mean(0)[None], c)[0]
            fr.poly(cam, Q, cc)
    for k, bz in enumerate(S.BRAZIERS):
        s_ = cam.wp(bz + [0, 0.25, 0])
        dd = cam.depth(bz)[0]
        fl = flick(t, 0.5 + k * 0.29)
        fr.disc(s_ - [0, 0.2 * cam.f / dd], 0.22 * cam.f / dd * fl, (255, 170, 70), n=6, z=dd)
        fr.disc(s_ - [0, 0.24 * cam.f / dd], 0.12 * cam.f / dd, (255, 236, 170), n=6, z=dd)
        fr.glow(s_[0], s_[1] - 0.25 * cam.f / dd, 0.9 * cam.f / dd, (255, 150, 60), 0.5 * fl)
    for k, tp in enumerate(S.TORCHES):
        s_ = cam.wp(tp)
        dd = cam.depth(tp)[0]
        if dd > 1:
            fl = flick(t, k * 0.37)
            fr.disc(s_, max(1.2, 0.16 * cam.f / dd * fl), (255, 170, 80), n=6, z=dd)
            fr.glow(s_[0], s_[1], 0.7 * cam.f / dd, (255, 120, 44), (0.35 + 0.25 * wk) * fl)
    if wk > 0:
        for k, tp in enumerate(S.WIN_TORCHES):
            on = ss(T_WIN + 0.6 + 0.07 * abs(k - len(S.WIN_TORCHES) / 2), T_WIN + 1.0 + 0.07 * abs(k - len(S.WIN_TORCHES) / 2), t) * wk
            if on <= 0:
                continue
            s_ = cam.wp(tp)
            dd = cam.depth(tp)[0]
            fl = flick(t, k * 0.53)
            fr.disc(s_, max(1.2, 0.18 * cam.f / dd), (255, 140, 60), n=6, z=dd)
            fr.glow(s_[0], s_[1], 0.8 * cam.f / dd, (255, 100, 36), 0.45 * on * fl)
    # ---------------- pit: dark well with a stone lip, the eliminated resting inside
    pit_rim = np.array([[S.PIT_R * math.cos(a), 0.01, S.PIT_R * math.sin(a)] for a in np.linspace(0, 2 * math.pi, 33)[:-1]])
    lip = np.array([[(S.PIT_R + 0.28) * math.cos(a), 0.012, (S.PIT_R + 0.28) * math.sin(a)] for a in np.linspace(0, 2 * math.pi, 33)[:-1]])
    lc = shade(L, np.array([[0, 1.0, 0]]), np.zeros((1, 3)), np.array([150, 116, 96]))[0]
    fr.poly(cam, lip, lc)
    fr.poly(cam, pit_rim, np.array([40, 24, 34]))
    # far inner wall of the pit (visible from above)
    for k in range(32):
        a0, a1 = 2 * math.pi * k / 32, 2 * math.pi * (k + 1) / 32
        q = np.array([[S.PIT_R * math.cos(a0), 0.01, S.PIT_R * math.sin(a0)], [S.PIT_R * math.cos(a1), 0.01, S.PIT_R * math.sin(a1)],
                      [S.PIT_R * math.cos(a1), S.PIT_Y, S.PIT_R * math.sin(a1)], [S.PIT_R * math.cos(a0), S.PIT_Y, S.PIT_R * math.sin(a0)]])
        nn = -np.array([math.cos((a0 + a1) / 2), 0, math.sin((a0 + a1) / 2)])
        if np.dot(nn, cam.pos - q.mean(0)) > 0:
            cc = shade(L, nn[None], q.mean(0)[None], np.array([110, 82, 76]))[0] * 0.75
            fr.poly(cam, q, cc)
    arr, depth = fr.array(), fr.depth()
    # balls resting in the pit (behind the near lip)
    pit_balls = sorted([(cam.depth(p)[0], b, p) for b, (p, st, xy) in enumerate(states) if st == 2], key=lambda a: -a[0])
    for _, b, p in pit_balls:
        draw_ball(arr, depth, cam, b, p, t, alpha=0.92)
    fr.set_array(arr, depth)
    # ---------------- the ring (painter)
    th, w = ring_angle(t), gap_width(t)
    items = []
    for Q, n, c, kind in rf:
        if np.dot(n, cam.pos - Q.mean(0)) <= 0:
            continue
        items.append((cam.depth(Q.mean(0))[0], Q, n, c, kind))
    items.sort(key=lambda a: -a[0])
    if items:
        N = np.array([it[2] for it in items])
        Cc = np.array([it[1].mean(0) for it in items])
        A = np.array([it[3] for it in items])
        em = np.array([0.55 if it[4] == 'cap' else 0.0 for it in items])
        cols = shade(L, N, Cc, A, rimk=0.9) + em[:, None] * GOLD[None] * 0.5
        for it, cc in zip(items, cols):
            fr.poly(cam, it[1], cc)
        # gold trim on the front face edges, pulsing gently on the beat
        pulse = math.exp(-((t % T.BEAT) / 0.12))
        trim = np.clip(GOLD * (0.85 + 0.25 * pulse), 0, 255)
        for rr_ in (R_IN + 0.01, R_OUT - 0.01):
            us = np.linspace(w / 2, 2 * math.pi - w / 2, 97)
            pts = np.array([S.RING_C + [rr_ * math.cos(th + u), rr_ * math.sin(th + u), ZD + 0.01] for u in us])
            fr.line(cam, pts, trim, 1)
        # rivets
        for k in range(24):
            u = (k + 0.5) * 2 * math.pi / 24
            if u < w / 2 + 0.05 or u > 2 * math.pi - w / 2 - 0.05:
                continue
            p = S.RING_C + [(R_IN + R_OUT) / 2 * math.cos(th + u), (R_IN + R_OUT) / 2 * math.sin(th + u), ZD + 0.01]
            sp = cam.wp(p[None])[0]
            fr.disc(sp, 1.0, (255, 220, 140), n=4, z=cam.depth(p)[0])
    if w > 1e-3:
        for sgn in (-1, 1):
            a = th + sgn * w / 2
            p = S.RING_C + [(R_IN + R_OUT) / 2 * math.cos(a), (R_IN + R_OUT) / 2 * math.sin(a), ZD]
            sp = cam.wp(p[None])[0]
            fr.glow(sp[0], sp[1], 0.35 * cam.f / cam.depth(p)[0], (255, 190, 90), 0.4)
    if T_WIN <= t < T_WIN + 0.6:                                  # the seal on the win
        a = ring_angle(t)
        p = S.RING_C + [(R_IN + R_OUT) / 2 * math.cos(a), (R_IN + R_OUT) / 2 * math.sin(a), ZD]
        sp = cam.wp(p[None])[0]
        fr.glow(sp[0], sp[1], 0.45 * cam.f / cam.depth(p)[0], (255, 210, 130), 0.5 * (1 - (t - T_WIN) / 0.6))
    arr, depth = fr.array(), fr.depth()
    # ---------------- balls: in play (and scripted) far-to-near, then the falling ones
    drawn = {}
    order = sorted([(cam.depth(p)[0], b, p, st) for b, (p, st, xy) in enumerate(states) if st != 2], key=lambda a: -a[0])
    for dz, b, p, st in order:
        glow = recent_hit(b, t) if st == 0 else 0.0
        if b == WINNER and t >= T_WIN:
            glow = max(glow, (0.35 + 0.4 * solo_env(t)) * ss(T_WIN, T_WIN + 0.4, t) * (1 - ss(T.T_RESET, T.T_RESET + 1.0, t)))
        r = draw_ball(arr, depth, cam, b, p, t, glow=glow)
        if r is not None:
            drawn[b] = (r, st)
            if b == WINNER and t >= T_HOVER0 and t < T.T_RESET + 1.0:
                k = ss(T_HOVER0, T_HOVER1, t) * (1 - ss(T.T_RESET, T.T_RESET + 1.0, t))
                fr.glow(r[0][0], r[0][1], r[1] * 2.6, (255, 210, 120), (0.32 + 0.22 * solo_env(t)) * k)
    fr.set_array(arr, depth)
    # ---------------- lens (focus locked on the ring), glows, rays, bloom, grade
    img, depth = fr.array(), fr.depth()
    zf = cam.depth(S.SUBJECT)[0]
    img = dof(img, coc_map(depth, zf, DOF_K * cam.f * min(1.2, 4.5 / zf), sharp=DOF_SHARP))
    img = apply_glows(img, fr.glows)
    if ss_ is not None:
        img = god_rays(img, depth, ss_, (255, 150, 100), 0.18)
    img = grade(bloom(np.clip(img, 0, 255), thr=230, k=0.25), contrast=0.22, sat=1.2)
    img = np.clip(img * vignette(strength=0.5) * top_scrim(), 0, 255).astype(np.uint8)
    img = np.ascontiguousarray(img)
    if ss_ is not None:
        sun_over(img, depth, ss_, cam)
    pigeons(img, cam, t)
    # ---------------- UI: roster (reelkit defaults: bottom of the safe zone, left of the button column), ball names, captions
    place = [PLACE[b] if (T_OUT[b] >= 0 and t >= T_OUT[b] and t < T.T_RESET + 0.06 * PLACE[b] + 1.0) else None for b in range(16)]
    flash = [max(0.0, 1 - (t - T_OUT[b]) / 0.6) if T_OUT[b] >= 0 and 0 <= t - T_OUT[b] < 0.6 else 0.0 for b in range(16)]
    win = WINNER if T_WIN <= t < T.T_RESET + 1.6 else None
    if win is not None:
        flash[WINNER] = max(0.0, 1 - (t - T_WIN) / 0.8)
    roster_top = roster_grid(img, NAMES, COLORS, place=place, flash=flash, winner=win)
    boxes = []
    for b in range(16):
        if b not in drawn:
            continue
        (s, rp), st = drawn[b]
        if st != 0 and not (b == WINNER and T_WIN <= t < T.T_RESET) and not (st == 3 and t >= 60.25):
            continue
        nm = NAMES[b]
        wdt = len(nm) * ADV - 1
        x = int(round(s[0] - wdt / 2))
        y = int(round(s[1] + rp + 2))
        for dy in (0, 8, -int(2 * rp + 12)):
            bx = (x, y + dy, x + wdt, y + dy + 7)
            if all(bx[2] < o[0] - 1 or bx[0] > o[2] + 1 or bx[3] < o[1] - 1 or bx[1] > o[3] + 1 for o in boxes):
                break
        else:
            continue
        if safe_overshoot(*bx) > 0 or bx[1] < SAFE[1] + 52 or bx[3] > roster_top - 2:
            continue                                     # stay out of the caption band, the roster and the button column
        boxes.append(bx)
        col = AMBER if b == WINNER and t >= T_WIN else CREAM
        label(img, nm, bx[0], bx[1], col, 1)
    draw_captions(img, t, CAPTIONS, style='impact')
    return img
