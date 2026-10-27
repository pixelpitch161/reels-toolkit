"""#42 BALLFIGHT #16: Viola vs Violin - renderer. #41's pipeline (the shared ring, effects from reelkit.fx, the VS title
card, the HUD ability list) with the VIOLIN exactly as #28 drew it (the bow spun like a rapier, its motion fan, TREMOLO's
violet blur, PIZZICATO's darts, SCREECH's jagged ring; #28's draw functions imported through styles.py) plus its allowed
v2 upgrade (rosin dust off the bow, attack lights), and the new VIOLA: COUNTING RESTS (a giant quarter rest over the
resting ball with the count "1 2 3" above it, then a whole note: the hollow oval standing up as a heavy wheel, rolling
and bobbing, ink with a teal-lit rim, crushing), DOUBLE STOP (two at once), MELODY AT LAST (eighth notes rising off the
ball, a teal halo, the rests down to one), VIOLA JOKE (the crowd rocks with laughter, the violin freezes grey, the
biggest whole note falls on it: shadow first, then the crash).

arena floor (per pixel: spill, corner lights) -> stands + crowd (phones, flashbulbs, the laugh) -> ring skirt -> canvas
(per pixel: the hard spot with real shadows of balls, the bow, darts, whole notes, posts and ropes; the fighters' corner
lights; decals: SCREECH's ring, the rest pool, the joke's target, crush marks) -> back ropes + posts -> fighters, bow,
darts, whole notes (painter) -> front ropes + posts -> sparks, shards -> countdown -> locked DOF on the ring -> glows ->
spot beam -> bloom -> grade -> scrim -> rests + counts, rising eighths -> HP numbers, fight HUD + ability list, captions
-> the VS title card.

    python render.py                                   # sprite cache (check/sprites_ballfight16.npz)
    python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 2.2 6.0 16.6 26.9 44.3 44.8 50.6 54.0 60.7
    python -m reelkit.video render.py audio.wav out.mp4"""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', '..'))
sys.path.insert(0, HERE)
sys.path.append(os.path.join(HERE, '..', 'ballfight'))                  # the BALLFIGHT engine, roster and ring (shared)
sys.path.append(os.path.join(HERE, '..', 'which_instrument_wins'))      # Pixel Pitch bridge + sprite renderer (read-only)
from reelkit.config import W, H, FPS, BAYER_FULL, SAFE, ss, ease, mixc
from reelkit.camera import Cam, interp_keys
from reelkit.raster import Frame, apply_glows
from reelkit.lighting import Lights, shade
from reelkit.shadows import MaskGrid, proj_point
from reelkit.lens import coc_map, dof
from reelkit.post import bloom, grade, vignette, top_scrim
from reelkit import ui
from reelkit import fx
from reelkit.ui import draw_captions, label, fight_hud, CREAM, AMBER, INK
from reelkit.font import ADV, text_mask, dilate
import timeline as T
import scene as S
import sim
import styles as ST
import title as TT                                                  # the shared VS title card (projects/ballfight)
from fighters import FIGHTERS

YY_, XX_ = np.mgrid[0:H, 0:W].astype(np.float64)
RUN = sim.load()
NSIM = len(RUN['HP'])
T_KO = float(RUN['t_ko'])
WIN = int(RUN['winner'])
LOSE = 1 - WIN
NAMES = RUN['names']
CFG = [FIGHTERS[n] for n in NAMES]
COL = [np.array(c['color'], float) for c in CFG]
DARK = [np.array(c['dark'], float) for c in CFG]
EV = RUN['events']
EV_T = np.array([e[0] for e in EV])
R = S.R_BALL
HITSTOP = 0.35
T_HS = T_KO + HITSTOP
BLADE = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'blade']       # the VIOLIN's bow (#28)
VIOLA = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'rests']       # COUNTING RESTS (#42)
CORNER0 = [np.array(c) for c in RUN['corner']]
STORY = sim.story(RUN)

# ---------------------------------------------------------------- the fighters' own colours on the shared ring (series
# rule, user Oct 23). Runtime patches of the imported scene, so projects/ballfight/scene.py stays as #27 uses it.
S.CORNER_COL['red'] = np.clip(COL[0] * 0.9, 0, 255)
S.CORNER_COL['blue'] = np.clip(COL[1] * 0.9, 0, 255)
S.POSTS = {k: S.post_faces(k) for k in S.CORNERS}
S.SKIRT_BAND = np.array([150, 140, 184])                 # a neutral band on the skirt
S.SKIRT_FACES = S.platform_faces()
LIGHT_RGB = [COL[k] / COL[k].max() for k in (0, 1)]     # corner light colour (max channel 1)
CROWD_TINT = [0.78 + 0.42 * LIGHT_RGB[k] for k in (0, 1)]

# ---------------------------------------------------------------- music-locked times shared with audio.py
T_SOLO = T.next_bar(T_KO + 1.6)                       # the winner's solo starts on the first bar line after the K.O. moment
SOLO_T = [T_SOLO + bt * T.BEAT for bt, m, d in T.SOLO[NAMES[WIN]]]


def solo_env(t):
    past = [s for s in SOLO_T if s <= t]
    return math.exp(-(t - past[-1]) / 0.16) if past and t < T.T_REASSEMBLE + 0.3 else 0.0


# ---------------------------------------------------------------- run access
def sim_i(t):
    return int(min(NSIM - 1, max(0, round((t - T.T_FIGHT) * FPS))))


def hp_at(k, t):
    if t < T.T_FIGHT:
        return 100.0
    if t < T_KO:
        return float(RUN['HP'][sim_i(t), k])
    end = float(RUN['HP'][-1, k]) if k == WIN else 0.0
    return end + (100.0 - end) * ease((t - T.T_GLIDE) / (T.T_REV - T.T_GLIDE))


def rev_offset(k, t):
    """the corner rev: a fast small shake (whole cycles in 61 s) + a slow sway, same at both ends of the file"""
    p1, p2 = 61.0 / 419, 61.0 / 353
    a = 0.035
    return np.array([a * math.sin(2 * math.pi * t / p1 + k), a * math.cos(2 * math.pi * t / p2 + 2 * k)])


OMEGA = [CFG[k]['weapon'].get('spin', 0.0) for k in (0, 1)]          # base spin (the rev and the tail use it)
TH0 = [float(RUN['TH'][0, k]) for k in (0, 1)]
SD0 = [float(RUN['SPIN'][0, k]) for k in (0, 1)]


def theta_open(k, t):
    """blade angle while revving: continuous into the fight's first frame (and, shifted by 61 s, from the tail into frame 0)"""
    return TH0[k] + SD0[k] * OMEGA[k] * (t - T.T_FIGHT)


def theta_tail(k, t):
    return theta_open(k, t - T.DUR)


def _wrap(a):
    return (a + math.pi) % (2 * math.pi) - math.pi


P_KO = [RUN['P'][-1, k].copy() for k in (0, 1)]
BASE_LEN = [CFG[k]['weapon'].get('base', 0.0) for k in (0, 1)]


def _glide_arcs():
    """#41: if the straight glides home would cross, each bows sideways (zero at both ends: the corners and the loop seam
    don't move): the smallest a that keeps them 2R + 0.15 apart. Returns (normal, amplitude) per fighter."""
    c = np.zeros(2)
    dl, dw = CORNER0[LOSE] - P_KO[LOSE], CORNER0[WIN] - c
    nl = np.array([-dl[1], dl[0]]) / (np.linalg.norm(dl) + 1e-9)
    nw = np.array([-dw[1], dw[0]]) / (np.linalg.norm(dw) + 1e-9)
    ml, mw = (P_KO[LOSE] + CORNER0[LOSE]) / 2, CORNER0[WIN] / 2
    nl = nl if nl @ (ml - mw) >= 0 else -nl
    nw = nw if nw @ (mw - ml) >= 0 else -nw
    gs = np.linspace(0, 1, 201)
    for a in np.arange(0.0, 2.51, 0.05):
        bl = P_KO[LOSE][None] + dl[None] * gs[:, None] + nl[None] * (a * np.sin(np.pi * gs))[:, None]
        bw = c[None] + dw[None] * gs[:, None] + nw[None] * (0.5 * a * np.sin(np.pi * gs))[:, None]
        if np.hypot(*(bl - bw).T).min() >= 2 * R + 0.15:
            break
    out = [None, None]
    out[LOSE], out[WIN] = (nl, a), (nw, 0.5 * a)
    return out


GLIDE_ARC = _glide_arcs()

# ---------------------------------------------------------------- COUNTING RESTS (#42), from the run's records + events
STUNS = [(e[0], e[2], e[4]) for e in EV if e[1] == 'stun']                     # t, victim, dur (VIOLA JOKE's freeze)
JOKES = [(e[0], e[2], e[3], e[4], e[5], e[6]) for e in EV if e[1] == 'joke']    # t, k, victim, t_land, x, z
MELODIES = [(e[0], e[2], e[3]) for e in EV if e[1] == 'melody']                # t, k, until
LAUGH = 1.9                                                                    # s the crowd laughs after the rimshot
NOTE_MEAN = (ST.NOTE_A + ST.NOTE_B) / 2


def _notes_build():
    """per note id: frames, positions, rolling direction and turn (from the recorded path), radius, times"""
    D = {}
    for i, rows in enumerate(RUN['NOTES']):
        for owner, x, z, r, t0, t1, tl, nid, kind, hit in rows:
            d = D.setdefault(int(nid), dict(owner=int(owner), r=float(r), t0=float(t0), t1=float(t1), tl=float(tl),
                                            kind=int(kind), f=[], p=[], hit=None))
            d['f'].append(i)
            d['p'].append((x, z))
            if hit and d['hit'] is None:
                d['hit'] = T.T_FIGHT + i / FPS
    aims = {e[3]: e[6] for e in EV if e[1] == 'note'}                          # id -> launch aim
    for nid, d in D.items():
        P = np.array(d['p'], float)
        a0 = aims.get(nid, 0.0)
        U = np.zeros_like(P)
        u = np.array([math.cos(a0), math.sin(a0)])
        phi = np.zeros(len(P))
        right = camera(0.0).M[0]
        for j in range(len(P)):
            if j > 0:
                step = P[j] - P[j - 1]
                ds = float(np.hypot(*step))
                if ds > 1e-4:
                    u = step / ds
                sw = S.to_world(step[0], step[1], 0.0) - S.to_world(0.0, 0.0, 0.0)      # rolling right on screen
                sgn = 1.0 if sw @ right >= 0 else -1.0                                   # turns it clockwise
                phi[j] = phi[j - 1] + sgn * ds / (d['r'] * NOTE_MEAN)
            U[j] = u
        d['P'], d['U'], d['PHI'] = P, U, phi
        d['f'] = np.array(d['f'])
    return D


def notes_at(t):
    """[(id, c, u, phi, r, lift, scale, glow, owner, kind)] of the whole notes on screen at t"""
    out = []
    if t < T.T_FIGHT or t >= T_HS + 0.6:
        return out
    tt = min(t, T_KO)
    fi = sim_i(tt)
    fade = 1.0 - ss(T_HS, T_HS + 0.6, t)
    for nid, d in NOTES.items():
        if not (d['f'][0] <= fi <= d['f'][-1]):
            continue
        j = int(fi - d['f'][0])
        c, u, phi = d['P'][j], d['U'][j], d['PHI'][j]
        lift, glow = 0.0, 0.0
        if d['tl'] > d['t0'] + 1e-6 and tt < d['tl']:                          # dropped from above (VIOLA JOKE)
            s_ = (tt - d['t0']) / (d['tl'] - d['t0'])
            lift = 4.6 * (1 - s_ * s_)
            phi = 0.4 * math.sin(6 * s_)
        sc = ease(min(1.0, (tt - d['t0']) / 0.10)) if d['tl'] <= d['t0'] + 1e-6 else 1.0
        sc *= 1.0 - ss(d['t1'] - 0.14, d['t1'], tt)                             # a whole note's length: then it's gone
        if d['hit'] is not None and 0 <= tt - d['hit'] < 0.25:
            glow = 1 - (tt - d['hit']) / 0.25
        if tt - d['t0'] < 0.15:
            glow = max(glow, 1 - (tt - d['t0']) / 0.15)
        sc *= fade
        if sc > 0.03:
            out.append((nid, c, u, phi, d['r'], lift, sc, glow, d['owner'], d['kind']))
    return out


def rest_state(k, t):
    """(resting 0/1, count, melody) for the viola from the record; nothing before the fight or after the K.O."""
    if k not in VIOLA or not (T.T_FIGHT <= t < T_KO):
        return 0, 0, 0
    r = RUN['REST'][sim_i(t), k]
    return int(r[0]), int(r[1]), int(r[2])


def stun_k(k, t):
    tt = min(t, T_KO)
    for t0, v, d in STUNS:
        if v == k and t0 <= tt < t0 + d + 0.15:
            return min(1.0, (tt - t0) / 0.06) * (1 - ss(t0 + d, t0 + d + 0.15, tt))
    return 0.0


def laugh_k(t):
    """the crowd's laugh after VIOLA JOKE's rimshot (0..1)"""
    best = 0.0
    for t0, k, v, tl, x, z in JOKES:
        if t0 <= t < t0 + LAUGH + 0.4:
            best = max(best, min(1.0, (t - t0) / 0.12) * (1 - ss(t0 + LAUGH, t0 + LAUGH + 0.4, t)))
    return best


def melody_k(k, t):
    tt = min(t, T_KO)
    for t0, kk, until in MELODIES:
        if kk == k and t0 <= tt < until:
            return min(1.0, (tt - t0) / 0.2) * (1 - ss(until - 0.25, until, tt))
    return 0.0


def fighter(k, t):
    """picture state of fighter k at time t: dict(p (ring frame), vis, th, blen, shield, haste, glow, scale, ...)"""
    st = dict(vis=True, shield=0.0, haste=0.0, glow=0.0, scale=1.0, th=0.0, blen=BASE_LEN[k], level=1,
              shattered=False, om=OMEGA[k], sd=SD0[k], stun=0.0)
    if t < T.T_FIGHT:
        st['p'] = CORNER0[k] + rev_offset(k, t)
        st['th'] = theta_open(k, t)
        st['glow'] = 0.25 * T.beat_pulse(t)
        return st
    if t < T_HS:
        i = sim_i(min(t, T_KO))
        st['p'] = RUN['P'][i, k].copy()
        st['th'] = float(RUN['TH'][i, k])
        st['blen'] = float(RUN['BL'][i, k])
        st['shield'] = float(RUN['SH'][i, k])
        st['haste'] = float(RUN['HA'][i, k])
        st['level'] = int(RUN['LV'][i, k])
        st['om'] = float(RUN['OM'][i, k])
        st['sd'] = float(RUN['SPIN'][i, k])
        st['stun'] = stun_k(k, t)
        if k in VIOLA:
            st['glow'] = 0.45 * melody_k(k, t) * (0.6 + 0.4 * T.beat_pulse(t, 0.2))
        if t >= T_KO and k == LOSE:
            st['glow'] = 1.0
        return st
    blen_end = float(RUN['BL'][-1, k])
    gl = ease((t - T.T_GLIDE) / (T.T_REV - T.T_GLIDE))
    corner = CORNER0[k] + rev_offset(k, t) * ss(T.T_REV - 0.3, T.T_REV + 0.2, t)
    rev_glow = 0.25 * T.beat_pulse(t) * ss(T.T_REV, T.T_REV + 0.3, t)
    if k == WIN:
        centre = np.array([0.0, 0.0])
        u = ease((t - T_HS) / 1.8)
        bob = 0.06 * math.sin(2 * math.pi * (t - T_HS) / (2 * T.BAR)) * ss(T_HS, T_HS + 1.5, t)
        p = P_KO[k] + (centre - P_KO[k]) * u + np.array([bob, -bob])
        st['p'] = p + (corner - p) * gl + GLIDE_ARC[k][0] * GLIDE_ARC[k][1] * math.sin(math.pi * gl)
        th_cont = float(RUN['TH'][-1, k]) + float(RUN['SPIN'][-1, k]) * OMEGA[k] * (t - T_HS)
        st['th'] = theta_tail(k, t) + _wrap(th_cont - theta_tail(k, t)) * (1 - gl)
        st['blen'] = blen_end + (BASE_LEN[k] - blen_end) * gl
        st['glow'] = max(rev_glow, (0.35 + 0.5 * solo_env(t)) * ss(T_HS, T_HS + 0.6, t) * (1 - gl))
        st['level'] = 3 if t < T.T_GLIDE else 1
        st['sd'] = float(RUN['SPIN'][-1, k]) if gl < 0.5 else SD0[k]
        return st
    # the loser: shattered until the reassembly, then glides home
    st['p'] = P_KO[k] + (corner - P_KO[k]) * gl + GLIDE_ARC[k][0] * GLIDE_ARC[k][1] * math.sin(math.pi * gl)
    st['th'] = theta_tail(k, t)
    st['shattered'] = t < T.T_GLIDE
    st['vis'] = t >= T.T_GLIDE - 0.12
    st['glow'] = max(rev_glow, 0.9 * (1 - ss(T.T_GLIDE - 0.12, T.T_GLIDE + 0.5, t)))
    st['scale'] = 0.75 + 0.25 * ss(T.T_GLIDE - 0.12, T.T_GLIDE + 0.12, t)
    return st


# ---------------------------------------------------------------- camera
AIM = np.array([0.12, 0.25, 0.0])
SY = 288
KEYS = [  # (t, target, dist, yaw, pitch, fov, sy): gentle drift, a slight push-in for the final exchange, back for the win
    (0.0, AIM, 27.5, 0.0, 58.0, 36, SY),
    (8.0, AIM, 27.6, -0.8, 58.4, 36, SY),
    (20.0, AIM, 27.4, 0.7, 57.7, 36, SY),
    (32.0, AIM, 27.5, -0.6, 58.2, 36, SY),
    (44.0, AIM, 26.9, 0.0, 58.0, 36, SY),
    (T_KO, AIM, 26.7, 0.3, 58.0, 36, SY),
    (T_KO + 2.5, AIM, 27.3, 0.4, 58.4, 36, SY),
    (58.0, AIM, 27.4, 0.2, 58.2, 36, SY),
    (61.0, AIM, 27.5, 0.0, 58.0, 36, SY),
]
SHK = fx.Shake(cap=2.5)                              # a little camera shake on heavy hits (reelkit.fx)
for e in EV:
    if e[1] == 'hit' and e[7] in ('blade', 'wave', 'note'):
        big = e[7] == 'note' and e[8] == 2                                    # VIOLA JOKE's note lands
        SHK.add(e[0], 2.5 if big else {'blade': 0.4, 'wave': 2.0, 'note': 1.3}[e[7]])
SHK.add(T_KO, 2.5)


def camera(t):
    cam = interp_keys(KEYS, t)
    dx, dy = SHK.offset(t)
    cam.cx += dx
    cam.cy += dy
    return cam


NOTES = _notes_build()

# ---------------------------------------------------------------- lights
WP = 61.0 / 137


def win_k(t):
    return ss(T_HS + 0.4, T_SOLO, t) * (1 - ss(T.T_GLIDE, T.T_REV, t))


def spot_aim(t):
    """the spot follows the champion during the victory lap (back to the centre for the glide home)"""
    k = ss(T_HS, T_HS + 1.2, t) * (1 - ss(T.T_GLIDE, T.T_REV, t))
    if k <= 0:
        return S.SPOT_AIM
    p = fighter(WIN, t)['p']
    w = S.to_world(p[0], p[1])
    return S.SPOT_AIM + (w * 0.55 - S.SPOT_AIM) * k


def lights(t, cam):
    L = Lights(amb=(0.36, 0.33, 0.44), sky=(0.10, 0.08, 0.14))
    L.view = cam.view
    d = S.SPOT - spot_aim(t)
    L.key('spot', d / np.linalg.norm(d), (1.05, 1.0, 0.96))
    rk = 1.0 + 0.25 * T.beat_pulse(t, 0.2)
    L.point(S.RED_LIGHT, LIGHT_RGB[0] * 1.1 * rk, 2.6)
    L.point(S.BLUE_LIGHT, LIGHT_RGB[1] * 1.1 * rk, 2.6)
    FL.apply(L, fx_clock(t))                                  # attacks light the scene in the fighter's colour
    return L


# ---------------------------------------------------------------- event windows (sparks, flashes, ropes)
def events_between(t0, t1, kinds=None):
    i0, i1 = np.searchsorted(EV_T, t0, 'left'), np.searchsorted(EV_T, t1, 'right')
    out = []
    for i in range(i0, i1):
        if kinds is None or EV[i][1] in kinds:
            out.append((i, EV[i]))
    return out


SIDE_OF = {(1, 1): 0, (0, 1): 1, (1, -1): 2, (0, -1): 3}       # (axis, sign) -> rope side (ROPE_SIDES order)
ROPE_EV = [(e[0], SIDE_OF[(e[3], e[4])], e[5], e[6], e[7]) for e in EV if e[1] == 'rope']
ROPE_T = np.array([r[0] for r in ROPE_EV])
WAVE_EV = [(e[0], 1.0) for e in EV if e[1] == 'ability' and e[4] == 'wave']
WAVE_EV += [(e[3], 0.9) for e in EV if e[1] == 'joke']                       # the big note lands


def side_ends(s):
    a, b = S.ROPE_SIDES[s]
    return S.CORNERS[a], S.CORNERS[b]


def rope_points(s, y, t, n=15):
    """one rope (side s, height y) as world points: slight sag + the give where balls hit it, decaying"""
    a, b = side_ends(s)
    u = np.linspace(0, 1, n)
    P = a[None] + (b - a)[None] * u[:, None]
    mid = (a + b) / 2
    outn = mid / np.linalg.norm(mid)
    disp = np.zeros(n)
    i0 = np.searchsorted(ROPE_T, t - 0.9)
    i1 = np.searchsorted(ROPE_T, t, 'right')
    for j in range(i0, i1):
        te, side, x, z, spd = ROPE_EV[j]
        if side != s:
            continue
        dt = t - te
        uc = float(np.clip(((np.array([x, z]) - a) @ (b - a)) / ((b - a) @ (b - a)), 0, 1))
        amp = 0.16 * min(1.6, spd / 2.6) * math.exp(-dt / 0.22) * math.cos(2 * math.pi * 5.0 * dt)
        disp += amp * np.exp(-((u - uc) / 0.16) ** 2) * (1 - (y - 0.4) * 0.5)
    for te, k in WAVE_EV:
        dt = t - te
        if 0.15 < dt < 1.2:
            disp += 0.07 * k * math.exp(-(dt - 0.15) / 0.3) * math.sin(2 * math.pi * 4.0 * (dt - 0.15)) * np.sin(np.pi * u)
    disp += 0.012 * T.beat_pulse(t, 0.1) * np.sin(np.pi * u)                 # the 808's thud
    sag = 0.035 * np.sin(np.pi * u)
    P = P + outn[None] * disp[:, None]
    return S.to_world(P[:, 0], P[:, 1], y - sag)


# ---------------------------------------------------------------- shards (the loser shatters at the K.O.: scroll, strings,
# bridge, wood; the violin also its bow, as in #28)
def _shards():
    rng = np.random.default_rng(271)
    p0 = P_KO[LOSE]
    parts = []
    n_glass = 24
    kinds = ['glass'] * n_glass + ['string'] * 4 + ['wood'] * 7 + ['bridge'] * 2 + ['scroll'] + \
        (['bowbit'] * 3 if NAMES[LOSE] == 'VIOLIN' else [])
    for kind in kinds:
        a = rng.uniform(0, 2 * math.pi)
        sp = rng.uniform(1.6, 4.2) if kind == 'glass' else rng.uniform(1.0, 3.0)
        parts.append(dict(kind=kind, p=np.array([p0[0], R, p0[1]], float) + rng.normal(0, 0.12, 3) * [1, 0.6, 1],
                          v=np.array([sp * math.cos(a), rng.uniform(1.5, 3.8), sp * math.sin(a)]),
                          rot=rng.uniform(0, 2 * math.pi), w=rng.uniform(-9, 9), size=rng.uniform(0.10, 0.22),
                          curl=rng.uniform(-1, 1)))
    dt = 1.0 / 120
    n = int(7.0 / dt) + 1
    traj = np.zeros((len(parts), n, 4))
    for j, q in enumerate(parts):
        p, v, rot, w = q['p'].copy(), q['v'].copy(), q['rot'], q['w']
        for i in range(n):
            traj[j, i] = (p[0], p[1], p[2], rot)
            v[1] -= 9.0 * dt
            p += v * dt
            rot += w * dt
            if p[1] < 0.02:
                p[1] = 0.02
                v[1] = -v[1] * 0.3
                v[0] *= 0.7
                v[2] *= 0.7
                w *= 0.6
            for ax in (0, 2):
                if abs(p[ax]) > S.A - 0.05:
                    p[ax] = math.copysign(S.A - 0.05, p[ax])
                    v[ax] = -v[ax] * 0.4
    return parts, traj


PARTS, TRAJ = _shards()
WOOD_COL = np.array([120, 50, 28]) if NAMES[LOSE] == 'VIOLA' else np.array([156, 52, 30])   # each one's varnish


def part_time(t):
    """particle clock: slow motion for the first 0.7 s after the hit-stop (25 %), then real time"""
    u = t - T_HS
    if u < 0:
        return 0.0
    if u < T.SLOWMO:
        return u * T.SLOWMO_RATE
    return T.SLOWMO * T.SLOWMO_RATE + (u - T.SLOWMO)


def parts_at(t):
    """[(kind, pos (x, y, z) in ring frame + height, rot, size, curl, alpha)]"""
    if t < T_HS or t >= T.T_GLIDE:
        return []
    pt = part_time(t)
    i = min(TRAJ.shape[1] - 1, int(pt * 120))
    back = ease((t - T.T_REASSEMBLE) / (T.T_GLIDE - 0.1 - T.T_REASSEMBLE))
    tgt = np.array([P_KO[LOSE][0], R, P_KO[LOSE][1]])
    out = []
    for j, q in enumerate(PARTS):
        x, y, z, rot = TRAJ[j, i]
        p = np.array([x, y, z])
        if back > 0:
            arc = np.array([0, 0.9 * math.sin(math.pi * back), 0])
            p = p + (tgt - p) * back + arc
            rot += 6 * back
        out.append((q['kind'], p, rot, q['size'] * (1 - 0.5 * back), q['curl'], 1.0))
    return out


# ---------------------------------------------------------------- sprites (Pixel Pitch models, cached)
_SPR = {}
SPR_PATH = os.path.join(HERE, 'check', 'sprites_ballfight16.npz')


def _sprite_cache():
    if _SPR:
        return _SPR
    if os.path.exists(SPR_PATH):
        z = np.load(SPR_PATH)
        _SPR.update({k: z[k] for k in z.files})
    return _SPR


def render_padded(name, radius_px, yaw_deg, roll_deg=0.0, pad=1.2):
    """sprites.render_sprite with a wider crop: the VIOLA's bridge size (x1.12, so it reads bigger than the violin)
    reaches past the standard crop at close-up sizes; same camera, rasteriser and outline"""
    import sprites as SP
    import piano_lowpoly as P
    RW, RH = P.RW, P.RH
    m = int(math.ceil(radius_px * pad)) + 3
    img = np.zeros((RH, RW, 3), np.uint8)
    zb = np.zeros((RH, RW))
    gb = np.zeros((RH, RW), np.int8)
    cam = SP._Cam(yaw_deg, radius_px, m + 0.5, m + 0.5, roll_deg)
    P.render_geo(SP._geo(name), cam, img, zb, gb, scale=1.0)
    P.outline(img, zb, gb)
    n = 2 * m + 1
    return np.dstack([img[:n, :n], (gb[:n, :n] > 0).astype(np.uint8) * 255])


def sprite(name, radius_px, yaw_deg, roll):
    cache = _sprite_cache()
    r = int(np.clip(round(radius_px), 12, 30))
    yk = int(round(yaw_deg / 6.0)) * 6
    key = f'{name}_{r}_{yk % 360}_{int(roll)}'
    if key not in cache:
        if name == 'VIOLA':
            cache[key] = render_padded(name, r, float(yk), float(roll))
        else:
            import sprites as SP
            cache[key] = SP.render_sprite(name, r, float(yk), float(roll))
    return cache[key]


TT.CLOSEUP.setdefault('VIOLA', lambda r: render_padded('VIOLA', r, float(CFG[VIOLA[0]]['view'][0]),
                                                       float(CFG[VIOLA[0]]['view'][2])))


def build_sprites():
    """precompute every sprite the video can use (both instruments, radii 12..30, yaw window)"""
    out = {}
    for k in (0, 1):
        nm = CFG[k]['pp']
        yc, sw, roll = CFG[k]['view']
        for r in range(12, 31):
            for yk in range(int((yc - sw - 6) // 6) * 6, int(yc + sw + 12), 6):
                out[f'{nm}_{r}_{yk % 360}_{int(roll)}'] = sprite(nm, r, float(yk), float(roll))
    os.makedirs(os.path.dirname(SPR_PATH), exist_ok=True)
    np.savez_compressed(SPR_PATH, **out)
    print(len(out), 'sprites ->', SPR_PATH)


# ---------------------------------------------------------------- drawing helpers
def layer_blend(arr, polys, col, alpha, depth=None, z=None):
    """alpha-blend screen polygons (lists of points) of one colour onto arr"""
    if not polys:
        return
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if len(pts) >= 3:
            d.polygon([tuple(p) for p in pts], fill=255)
    mk = np.asarray(m, np.float32) / 255.0 * alpha
    if depth is not None and z is not None:
        mk = mk * (depth > z - 0.05)
    arr += (np.array(col, float)[None, None] - arr) * mk[..., None]


def screen(cam, P):
    return cam.wp(np.atleast_2d(P))


# ---------------------------------------------------------------- glass balls
def draw_ball(arr, dep, cam, k, st, t):
    pw = S.to_world(st['p'][0], st['p'][1], R)
    Pc = cam.to_cam(pw[None])[0]
    s = cam.proj(Pc[None])[0]
    rp = R * cam.f / Pc[2] * st['scale']
    ri = int(round(rp))
    x0, y0 = int(round(s[0])) - ri - 2, int(round(s[1])) - ri - 2
    n = 2 * ri + 5
    yy, xx = np.mgrid[0:n, 0:n]
    dx = (xx + x0 + 0.5 - s[0]) / rp
    dy = (yy + y0 + 0.5 - s[1]) / rp
    d2 = dx * dx + dy * dy
    inside = d2 <= 1.0
    rim = inside & (d2 > (1 - 2.0 / rp) ** 2)
    edge = (d2 > 1.0) & (d2 <= (1 + 1.3 / rp) ** 2)
    xa, ya = max(0, x0), max(0, y0)
    xb, yb = min(W, x0 + n), min(H, y0 + n)
    if xb <= xa or yb <= ya:
        return s, rp
    sl = (slice(ya - y0, yb - y0), slice(xa - x0, xb - x0))
    reg = arr[ya:yb, xa:xb]
    dreg = dep[ya:yb, xa:xb]
    col = COL[k]
    glow = st['glow']
    frz = st.get('stun', 0.0)                                                # VIOLA JOKE: frozen, it goes grey and still
    nz = np.sqrt(np.clip(1 - d2, 0, 1))
    lam = np.clip(-0.25 * dx - 0.55 * dy + nz * 0.6, 0, 1)                   # lit from the spot above (top of the ball)
    band = np.floor(lam * 4 + BAYER_FULL[:n, :n] * 0.9) / 4
    glass = np.array([236, 232, 226]) * (1 - 0.3 * frz) + np.array([168, 176, 196]) * 0.3 * frz
    tint = col * (1 - frz) + np.array([150, 150, 168]) * frz
    inner = (tint * 0.30 + glass * 0.70)[None, None] * (0.70 + 0.36 * band[..., None])
    m = inside[sl]
    reg[m] = reg[m] * 0.12 + inner[sl][m] * 0.88
    # the instrument
    name = CFG[k]['pp']
    yc, sw, roll = CFG[k]['view']
    yaw = yc + sw * math.sin(1.25 * st['p'][0] + 0.45 * st['p'][1])
    spr = sprite(name, rp * 0.80, yaw, roll)
    sn = spr.shape[0]
    sx0, sy0 = int(round(s[0])) - sn // 2, int(round(s[1])) - sn // 2
    ax0, ay0 = max(0, sx0), max(0, sy0)
    ax1, ay1 = min(W, sx0 + sn), min(H, sy0 + sn)
    if ax1 > ax0 and ay1 > ay0:
        sub = spr[ay0 - sy0:ay1 - sy0, ax0 - sx0:ax1 - sx0]
        ms = sub[..., 3] > 0
        rg = arr[ay0:ay1, ax0:ax1]
        lit = sub[..., :3].astype(np.float32) * (1.06 + 0.3 * glow)
        if frz > 0:
            g_ = lit.mean(-1, keepdims=True)
            lit = lit * (1 - 0.7 * frz) + (g_ * 0.85 + np.array([20, 24, 40])) * 0.7 * frz
        rg[ms] = np.clip(lit[ms], 0, 255)
    rc = np.clip(tint * (0.9 + 0.5 * glow) + 50 * glow, 0, 255)
    mr = rim[sl]
    reg[mr] = rc
    me = edge[sl]
    reg[me] = reg[me] * 0.2 + np.array([22, 14, 30]) * 0.8
    spec = inside & ((dx + 0.38) ** 2 + (dy + 0.5) ** 2 < (2.0 / rp) ** 2 + 0.014)
    reg[spec[sl]] = np.array([255, 252, 244])
    dreg[m | me] = Pc[2]
    return s, rp


# ---------------------------------------------------------------- captions (from the run)
def _caps():
    title = []                                                      # the VS title card replaces the top caption
    abil = [[], []]
    for t, who, name in STORY['caps']:
        abil[who].append((t, t + 2.4, [name] if len(name) <= 14 else name.rsplit(' ', 1)))
    ko = [(T_KO, T_KO + 1.25, ['*K.O.*'])]
    wins = [(T_KO + 1.3, T_KO + 3.55, [NAMES[WIN], 'WINS!'])]
    comment = [(T_KO + 3.65, T.T_REASSEMBLE, ['COMMENT THE', '*NEXT MATCH-UP*'])]
    return title, abil, ko, wins, comment


CAP_TITLE, CAP_ABIL, CAP_KO, CAP_WINS, CAP_COMMENT = _caps()
CAPTIONS = CAP_TITLE + CAP_ABIL[0] + CAP_ABIL[1] + CAP_KO + CAP_WINS + CAP_COMMENT     # for audio (impact thuds)
STATUS = [(e[0], e[2], e[3]) for e in EV if e[1] == 'ability']


# ---------------------------------------------------------------- the HUD ability list (series rule, Oct 25): LV2 / LV3 / LOW
SLOTS = (('unlock1', 'LV2'), ('unlock2', 'LV3'), ('low', 'LOW'))
SWEEP_T = 0.45                                       # s the glow takes to sweep across a name as it unlocks


def _hold(k, slot):
    """how long a use stays lit: a haste / MELODY AT LAST for as long as it lasts, VIOLA JOKE until its note lands"""
    ab = CFG[k]['abilities'][slot]
    if ab['kind'] in ('shield', 'haste', 'melody'):
        return ab['dur_beats'] * T.BEAT if 'dur_beats' in ab else ab['dur']
    if ab['kind'] == 'joke':
        return ab['tele'] * T.BEAT
    return 0.08


def _abil_build():
    unl = [{}, {}]                                   # k -> slot -> unlock time (LOW: its first use)
    uses = [{s_: [] for s_, _ in SLOTS} for _ in (0, 1)]
    for e in EV:
        if e[0] > T_KO + 1e-6:
            break
        if e[1] == 'level':
            unl[e[2]]['unlock1' if e[3] == 2 else 'unlock2'] = e[0]
        elif e[1] == 'ability':
            for slot, _ in SLOTS:
                if CFG[e[2]]['abilities'][slot]['name'] == e[3]:
                    uses[e[2]][slot].append(e[0])
                    if slot == 'low':
                        unl[e[2]].setdefault('low', e[0])
    return unl, uses


ABIL_UNLOCK, ABIL_USES = _abil_build()
ABIL_HOLD = [{slot: _hold(k, slot) for slot, _ in SLOTS} for k in (0, 1)]


def ability_rows(k, t):
    """fight_hud rows for fighter k: grey until unlocked (a glow sweeps across the name as it unlocks), the fighter's
    colour after, white-hot on its pill whenever it is used (held while it lasts, then easing back over 0.5 s). Frozen
    through the hit-stop; back to grey when HP refills on the glide home (as on frame 0)."""
    tc = fx_clock(t) if t < T.T_GLIDE else -1.0
    rows = []
    for slot, tag in SLOTS:
        tu = ABIL_UNLOCK[k].get(slot)
        on = tu is not None and T.T_FIGHT <= tc and tc >= tu
        g, sw = 0.0, None
        if on:
            if tc < tu + SWEEP_T:
                sw = (tc - tu) / SWEEP_T
            past = [te for te in ABIL_USES[k][slot] if te <= tc]
            if past:
                te, hold = past[-1], ABIL_HOLD[k][slot]
                g = 1.0 - ss(te + hold, te + hold + 0.5, tc)
        rows.append(dict(tag=tag, name=CFG[k]['abilities'][slot]['name'], state='unlocked' if on else 'locked',
                         glow=g, sweep=sw))
    return rows


def status(k, t):
    s = [(te, nm) for te, who, nm in STATUS if who == k and te <= t < te + 1.6]
    if not s:
        return '', 0.0
    te, nm = s[-1]
    return nm, 1 - ss(te + 1.3, te + 1.6, t)


# ---------------------------------------------------------------- big countdown text in the ring
def big_text(img, text, cx, cy, size_px, col, alpha=1.0, outline=(16, 10, 26)):
    base = text_mask(text, 4)
    h0, w0 = base.shape
    sc = size_px / h0
    hh, ww = max(1, int(round(h0 * sc))), max(1, int(round(w0 * sc)))
    ys = np.minimum((np.arange(hh) * h0 / hh).astype(int), h0 - 1)
    xs = np.minimum((np.arange(ww) * w0 / ww).astype(int), w0 - 1)
    m = base[ys][:, xs]
    o = dilate(m, 2)
    x0, y0 = int(round(cx - ww / 2)), int(round(cy - hh / 2))
    for mm, c, ox, oy in ((o, np.array([0, 0, 0]), 2, 3), (o, np.array(outline), 0, 0), (m, np.array(col), 0, 0)):
        hm, wm = mm.shape
        oxx, oyy = x0 - (wm - ww) // 2 + ox, y0 - (hm - hh) // 2 + oy
        xa, ya = max(0, oxx), max(0, oyy)
        xb, yb = min(W, oxx + wm), min(H, oyy + hm)
        if xb <= xa or yb <= ya:
            continue
        sub = mm[ya - oyy:yb - oyy, xa - oxx:xb - oxx]
        reg = img[ya:yb, xa:xb].astype(np.float32)
        a = alpha * (0.5 if ox else 1.0)
        reg[sub] = reg[sub] * (1 - a) + c * a
        img[ya:yb, xa:xb] = reg.astype(np.uint8)
    ui._note(x0, y0, x0 + ww, y0 + hh, 'caption', text)


def countdown(img, t, cs):
    """3 . 2 . 1 . FIGHT! in the ring, one count per beat (the band's count-in), on bar 1 after the title card"""
    cx, cy = cs
    items = []
    for j, (tc, txt) in enumerate(T.COUNT):
        t_next = T.COUNT[j + 1][0] if j + 1 < len(T.COUNT) else tc + 0.85
        items.append((tc, t_next, txt))
    for tc, tn, txt in items:
        size = 78 if txt != 'FIGHT!' else 44
        col = (255, 244, 220) if txt != 'FIGHT!' else (255, 214, 96)
        if tc <= t < tn:
            u = t - tc
            punch = 1 + 0.25 * max(0.0, 1 - u / 0.12) ** 2
            out = ss(tn - 0.1, tn, t)
            big_text(img, txt, cx, cy, size * punch * (1 - 0.35 * out), col, alpha=1 - out)


# ---------------------------------------------------------------- static draw lists
STAND_N = np.array([n for Q, n, c, am in S.STANDS])
STAND_C = np.array([Q.mean(0) for Q, n, c, am in S.STANDS])
STAND_A = np.array([c for Q, n, c, am in S.STANDS])
CROWD_P = np.array([c[0] for c in S.CROWD])
CROWD_CI = np.array([c[1] for c in S.CROWD])
CROWD_PH = np.array([c[2] for c in S.CROWD])
CROWD_HT = np.array([c[4] for c in S.CROWD])
BAR_N = np.array([n for Q, n, am in S.BARRIER])
BAR_C = np.array([Q.mean(0) for Q, n, am in S.BARRIER])
FLASH_EV = [e[0] for e in EV if (e[1] == 'hit' and e[7] in ('wave', 'note') and e[8] == 2) or e[1] == 'ability'] + \
           [T_KO + 0.03 * j for j in range(16)]
FLASH_T = np.array(sorted(FLASH_EV))


def led_colour(am, t, wk):
    """LED boards: each fighter's colour on its own side, a chase that steps on the beat (whole bars: loop-safe), gold
    for the champion"""
    beat = t / T.BEAT
    u = (am / (2 * math.pi) * 16 + beat) % 4
    side = COL[0] if math.sin(am) < 0 else COL[1]
    base = side if u < 2 else side * 0.42 + 40
    k = 0.55 + 0.45 * T.beat_pulse(t, 0.18)
    gold = np.array([255, 200, 90])
    return (base * (1 - wk) + gold * wk) * k


def arena(fr, cam, L, t):
    """tiers, people and LED boards in one painter pass (far to near); VIOLA JOKE: the whole crowd rocks with laughter"""
    wk = win_k(t)
    lk = laugh_k(t)
    items = []
    vis = np.einsum('ij,ij->i', STAND_N, cam.pos[None] - STAND_C) > 0
    idx = np.nonzero(vis)[0]
    cols = shade(L, STAND_N[idx], STAND_C[idx], STAND_A[idx], rimk=0.2) * 0.55
    for j, c, d in zip(idx, cols, cam.depth(STAND_C[idx])):
        items.append((d, 0, j, c))
    bvis = np.einsum('ij,ij->i', BAR_N, cam.pos[None] - BAR_C) > 0
    for j, d in zip(np.nonzero(bvis)[0], cam.depth(BAR_C[bvis])):
        items.append((d, 2, j, None))
    heads = CROWD_P + np.stack([np.zeros(len(CROWD_P)), 0.55 * CROWD_HT, np.zeros(len(CROWD_P))], 1)
    crowd_lit = 1.0 + 0.9 * FL.tint(heads, fx_clock(t))       # attack lights reach the crowd
    dh = cam.depth(heads)
    sp = cam.wp(heads)
    for j in range(len(CROWD_P)):
        items.append((dh[j] - 0.3, 1, j, None))
    items.sort(key=lambda it: -it[0])
    beat = int(t / T.BEAT)
    recent = FLASH_T[(FLASH_T <= t) & (FLASH_T > t - 0.1)]
    nflash = len(recent)
    spill = 0.48 + 0.28 * wk
    draw = fr.draw
    for d, kind, j, c in items:
        if kind == 0:
            fr.poly(cam, S.STANDS[j][0], c)
        elif kind == 2:
            Q, n, am = S.BARRIER[j]
            col = led_colour(am, t, wk)
            fr.poly(cam, Q, col * 0.55 + 18)
            top = cam.wp(Q[2:4])
            draw.line([tuple(top[0]), tuple(top[1])], fill=tuple(int(v) for v in np.clip(col * 0.9 + 60, 0, 255)), width=1)
        else:
            x, y = sp[j]
            ph_b = t % T.BEAT
            on_b = ph_b < 0.12 or ph_b > T.BEAT - 0.04
            beat_j = beat + (1 if ph_b > T.BEAT - 0.04 else 0)
            hop = 1 if ((beat_j + CROWD_CI[j]) % 2 == 0 and on_b) or (wk > 0.3 and CROWD_PH[j] < wk and on_b) else 0
            y -= hop
            rh = 0.15 * cam.f / dh[j]
            sway = 0.0
            if lk > 0:                                          # the laugh: heads bob "ha-ha-ha", bodies rock and
                ph = CROWD_PH[j] * 6.28                         # lean back (each at its own phase)
                y -= lk * (1.2 + 1.2 * abs(math.sin(2 * math.pi * 5.2 * t + ph))) * max(1.0, rh * 0.8)
                sway = lk * 1.3 * math.sin(2 * math.pi * 1.7 * t + ph) * max(1.0, rh * 0.6)
            if not (-6 < x < W + 6 and -6 < y < H + 10):
                continue
            side = CROWD_P[j, 0]
            tint = CROWD_TINT[0] if side < -1.5 else (CROWD_TINT[1] if side > 1.5 else np.array([1.0, 0.92, 1.0]))
            body = np.clip(S.CROWD_COLS[CROWD_CI[j]] * tint * spill * crowd_lit[j] * (1 + 0.25 * lk), 0, 255)
            draw.ellipse([x - 1.9 * rh, y + 0.6 * rh, x + 1.9 * rh, y + 3.4 * rh], fill=tuple(int(v) for v in body * 0.8))
            draw.ellipse([x - rh + sway, y - rh, x + rh + sway, y + rh], fill=tuple(int(v) for v in body * 0.62))
            if rh >= 1.5:
                draw.line([x - rh * 0.6 + sway, y - rh * 0.8, x + rh * 0.6 + sway, y - rh * 0.8],
                          fill=tuple(int(v) for v in np.clip(body * 1.5 + 30, 0, 255)))
            if lk > 0.3 and rh >= 1.2 and (j % 3 == 0):         # arms thrown up / clutching (laughing)
                ya_ = y + 0.9 * rh
                draw.line([x - 1.6 * rh, ya_, x - 2.1 * rh + sway, ya_ - 1.6 * rh * lk], fill=tuple(int(v) for v in body * 0.8))
                draw.line([x + 1.6 * rh, ya_, x + 2.1 * rh + sway, ya_ - 1.6 * rh * lk], fill=tuple(int(v) for v in body * 0.8))
            ph = CROWD_PH[j]
            if ph < 0.07 + 0.45 * wk:                                    # phones up (lots more for the champion)
                tw = 0.65 + 0.35 * math.sin(2 * math.pi * (t / WP + ph * 11))
                c2 = tuple(int(v) for v in np.array([255, 248, 226]) * tw)
                py = y - 2.2 * rh
                draw.rectangle([x + rh * 0.3, py - max(1, rh * 0.7), x + rh * 0.3 + max(1, rh * 0.5), py], fill=c2)
            if nflash and (int(ph * 997) + int(recent[-1] * 100)) % 47 < 1 + nflash:
                draw.ellipse([x - 1.6, y - 2.6 * rh - 1.6, x + 1.6, y - 2.6 * rh + 1.6], fill=(255, 255, 255))


# ---------------------------------------------------------------- shadows on the canvas (from the spot)
CANVAS_GRID = MaskGrid((-5.4, 5.4, -5.4, 5.4, 0.03))
STATIC_CASTERS = []
for name in S.CORNERS:
    w = S.to_world(*S.CORNERS[name])
    STATIC_CASTERS.append(np.array([w + [-0.09, 0, 0], w + [0.09, 0, 0], w + [0.09, S.POST_H, 0], w + [-0.09, S.POST_H, 0]]))


def canvas_shadows(cam, t, fs, keys, spot, notes):
    polys = []
    for Q in STATIC_CASTERS:
        polys.append(proj_point(Q, spot))
    for s in range(4):
        for y in S.ROPE_Y:
            P = rope_points(s, y, t, n=5)
            for a, b in zip(P[:-1], P[1:]):
                q = np.array([a + [0, -0.02, 0], b + [0, -0.02, 0], b + [0, 0.02, 0], a + [0, 0.02, 0]])
                polys.append(proj_point(q, spot))
    for k, st in enumerate(fs):
        if not st['vis']:
            continue
        a = np.linspace(0, 2 * math.pi, 13)[:-1]
        pw = S.to_world(st['p'][0], st['p'][1], R)
        circ = np.array([[pw[0] + R * st['scale'] * math.cos(u), R, pw[2] + R * st['scale'] * math.sin(u)] for u in a])
        polys.append(proj_point(circ, spot))
        if k in BLADE and not st['shattered']:
            quads, _ = ST.SV.bow_geom(st['p'], st['th'], st['blen'], st['sd'])
            for Q, c, kind in quads:
                if kind in ('stick', 'hair'):
                    polys.append(proj_point(Q, spot))
    for row in keys:
        head, body = ST.SV.dart_quads(row[0], row[1], row[2])
        polys.append(proj_point(head, spot))
    for nid, c, u, phi, r, lift, sc, glow, owner, kind in notes:
        polys.append(proj_point(ST.note_shadow(cam, c, r * sc, phi, lift), spot))
    return CANVAS_GRID.raster(polys, blur=0.7)


# ---------------------------------------------------------------- sparks
def sparks(arr, cam, t):
    for i, e in events_between(t - 0.32, t, ('hit', 'block', 'wall', 'cleared', 'crushed')):
        dt = t - e[0]
        kind = e[1]
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            col = COL[att] * 0.6 + 100
            n, sp, life = (16, 3.2, 0.3) if how in ('wave', 'note') else \
                ((8, 2.4, 0.22) if how == 'blade' else (5, 1.8, 0.18))
        elif kind == 'block':
            att, x, z = e[2], e[4], e[5]
            col, n, sp, life = np.array([160, 220, 255]), 12, 2.6, 0.3
        else:
            att, x, z = e[2], e[4], e[5]
            col, n, sp, life = np.array([200, 196, 210]), 3, 1.0, 0.15
        if dt > life:
            continue
        rng = np.random.default_rng(i)
        fade = 1 - dt / life
        for j in range(n):
            a = rng.uniform(0, 2 * math.pi)
            v = sp * rng.uniform(0.4, 1.0)
            q = np.array([x + v * dt * math.cos(a), z + v * dt * math.sin(a)])
            hgt = R + rng.uniform(-0.2, 0.3) + 2.5 * dt - 6 * dt * dt
            s = cam.wp(S.to_world(q[0], q[1], max(0.02, hgt))[None])[0]
            xi, yi = int(s[0]), int(s[1])
            if 0 <= xi < W - 1 and 0 <= yi < H - 1:
                arr[yi, xi] = np.clip(arr[yi, xi] * (1 - fade) + np.clip(col + 60, 0, 255) * fade, 0, 255)
        if kind == 'hit' and e[7] in ('blade', 'note') and dt < 0.08:
            s = cam.wp(S.to_world(x, z, R)[None])[0]
            arr[max(0, int(s[1]) - 1):int(s[1]) + 2, max(0, int(s[0]) - 1):int(s[0]) + 2] = 255


# ---------------------------------------------------------------- reelkit.fx
def fx_clock(t):
    """effects clock: real time in the fight, held through the hit-stop, then running on"""
    if t < T_KO:
        return t
    if t < T_HS:
        return T_KO
    return t - HITSTOP


FL = fx.Flares(max_n=6)
BU = fx.Bursts()
PA = fx.Particles(seed=42)
MK = fx.Marks()
MO = fx.Moments()
INKY = [(40, 40, 60), (70, 76, 96), (24, 22, 34)]       # ink flecks off a crushing note
ROSIN = [(250, 244, 226), (236, 226, 200), (255, 255, 246)]


def _fx_build():
    """every effect from the run's events, once, in time order (deterministic: frames render in any order)"""
    up = lambda x, z, y: S.to_world(x, z, y)
    for e in EV:
        t_ = e[0]
        if t_ > T_KO + 1e-6:
            break
        kind = e[1]
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = up(x, z, R)
            if how == 'blade':                                     # the bow lands: violet, rosin dust off the hair (v2)
                BU.add(t_, pos, COL[att], 0.6)
                FL.add(t_, pos, COL[att], radius=1.2, peak=0.45, decay=0.2)
                PA.emit(t_, pos, 'rosin', n=6, col=ROSIN)
            elif how == 'key':                                     # a pizzicato dart plucks
                BU.add(t_, pos, COL[att], 0.45, bright=False)
                PA.emit(t_, pos, 'rosin', n=3, col=ROSIN)
            elif how == 'note':                                    # a whole note crushes (VIOLA JOKE's: the big one)
                big = e[8] == 2
                BU.add(t_, pos, COL[att], 2.0 if big else 1.1)
                FL.add(t_, up(x, z, 0.4), COL[att], radius=2.6 if big else 1.6, peak=1.0 if big else 0.7,
                       decay=0.45 if big else 0.28)
                PA.emit(t_, up(x, z, 0.1), 'dust', n=26 if big else 10, spread=180, speed=(0.6, 1.8), scale=1.3 if big else 1.0)
                PA.emit(t_, pos, 'chips', n=8 if big else 4, col=INKY)
                MK.add(t_, 'crack', x, z, size=1.4 if big else 0.55, n=8 if big else 4, life=6.0 if big else 3.0,
                       seed=int(t_ * 100))
                if big:
                    MK.add(t_, 'scorch', x, z, size=1.2, col=(40, 44, 64), life=6.0, seed=int(t_ * 10) + 1, strength=0.5)
            elif how == 'wave':                                    # SCREECH lands
                BU.add(t_, pos, COL[att], 1.4)
                PA.emit(t_, pos, 'sparks', n=10, col=[tuple(COL[att] * 0.6 + 100)])
        elif kind == 'note':                                       # a note appears: a teal flash at its spot, ink motes
            nid, x, z, r = e[3], e[4], e[5], e[7]
            if e[9] <= t_ + 1e-6:                                  # rolling at once (a dropped one shows on landing)
                FL.add(t_, up(x, z, 0.5), COL[e[2]], radius=1.3, peak=0.45, decay=0.25)
                PA.emit(t_, up(x, z, 0.3), 'motes', n=5, col=[tuple(COL[e[2]] * 0.5 + 120)])
        elif kind == 'joke':                                       # the target glows teal while the note falls
            k, v, tl, x, z = e[2], e[3], e[4], e[5], e[6]
            FL.add(t_ + 0.05, up(x, z, 2.0), COL[k], radius=2.2, peak=0.6, decay=max(0.3, tl - t_))
        elif kind == 'ability' and e[3] == 'SCREECH':
            k = e[2]
            p = RUN['P'][sim_i(t_), k]
            FL.add(t_, up(p[0], p[1], 1.0), COL[k] * 0.9, radius=3.0, peak=0.8, decay=0.6)
        elif kind == 'block':
            BU.add(t_, up(e[4], e[5], R), (255, 226, 150), 0.6, bright=False)
    for t_, who, name in STORY['caps']:                           # ability moments on the captioned abilities
        MO.add(t_, who, COL[who])
    pos = S.to_world(P_KO[LOSE][0], P_KO[LOSE][1], R)
    BU.add(T_KO, pos, COL[LOSE], 2.0)
    FL.add(T_KO, pos, COL[LOSE] * 0.9, radius=2.4, peak=1.1, decay=0.6)
    PA.emit(T_KO, pos, 'chips', n=24, spread=180, speed=(0.8, 2.2), col=[tuple(WOOD_COL), tuple(WOOD_COL * 1.3)])


_fx_build()


def shimmers(img, cam, t):
    """heat-shimmer ripples: SCREECH's ring, the big note's landing"""
    if T.T_FIGHT <= t < T_HS:
        i = sim_i(min(t, T_KO))
        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][i]:
            if kind == 'wave':
                img = fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.35, 2.6, t, k=1 - r / rmax, seed=owner)
    for t0, k, v, tl, x, z in JOKES:
        if 0 <= t - tl < 0.6 and t < T_HS:
            u = (t - tl) / 0.6
            img = fx.shimmer_world(img, cam, S.to_world(x, z, 0.05), 1.2 + 1.8 * u, 0.35, 3.0, t, k=1 - u)
    return img


def post_fx(img, cam, t, fs, drawn):
    """after the grade: chromatic fringing on big hits, the ability moments (dim + speed lines), the impact bursts"""
    for _, e in events_between(t - 0.12, t, ('hit',)):
        if e[7] == 'wave' or (e[7] == 'note' and e[8] == 2):
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            img = fx.fringe(img, s_[0], s_[1], 26, 2.5, k=1 - (t - e[0]) / 0.12)
    if T_KO <= t < T_HS + 0.3:
        s_ = cam.wp(S.to_world(P_KO[LOSE][0], P_KO[LOSE][1], R)[None])[0]
        img = fx.fringe(img, s_[0], s_[1], 34, 3.0, k=1 - ss(T_HS, T_HS + 0.3, t))
    keep = [(s_[0], s_[1], rp * 1.7) for k, (s_, rp) in drawn.items()]
    img = MO.apply_dim(img, t, keep=keep)
    for k, (s_, rp) in drawn.items():
        p1 = fighter(k, max(0.0, t - 0.06))['p']
        s1 = cam.wp(S.to_world(p1[0], p1[1], R)[None])[0]
        img = MO.speed_lines(img, t, k, s_[0], s_[1], rp, (s_[0] - s1[0], s_[1] - s1[1]))
    img = BU.draw(img, cam, t)
    return img


# ---------------------------------------------------------------- the viola's rests, counts and rising eighths (after grade)
REST_G = ST.glyph(ST.REST_GLYPH, 2)
EIGHTH_G = ST.glyph(ST.EIGHTH_GLYPH, 1)
NOTE_EV = [(e[0], e[2]) for e in EV if e[1] == 'note' and e[8] == 0]          # the weapon's own notes (t, k)


CAP_BAND = SAFE[1] + 34                                                        # the top captions' band (2 lines)
CAP_LIVE = [(a, b) for a, b, _ in CAP_ABIL[0] + CAP_ABIL[1] + CAP_KO + CAP_WINS + CAP_COMMENT]


def caption_up(t):
    return any(a - 0.05 <= t < b + 0.1 for a, b in CAP_LIVE)


def _safe_x(x, w, y, h):
    """keep a small label inside the safe box and out of the button column"""
    x1 = 300 if y + h > 290 else SAFE[2]
    return int(min(max(x, SAFE[0] + 1), x1 - w - 1))


def draw_rests(img, t, cam, drawn):
    for k in VIOLA:
        if k not in drawn:
            continue
        resting, count, mel = rest_state(k, t)
        (sx, sy), rp = drawn[k]
        if resting and count > 0:
            # the giant quarter rest over the ball (it settles in on each counted beat, sways a little)
            t_beat = T.T_FIGHT + math.floor((t - T.T_FIGHT) / T.BEAT + 1e-6) * T.BEAT
            u = min(1.0, (t - t_beat) / 0.08)
            gy = sy - rp - REST_G.shape[0] / 2 - 3 - 3 * (1 - u)
            col = np.clip(COL[k] * 0.55 + 110, 0, 255)
            ST.stamp_glyph(img, REST_G, sx + 1.5 * math.sin(2 * math.pi * t / T.BAR), gy, col, alpha=0.4 + 0.6 * u,
                           outline=tuple(int(v) for v in DARK[k] * 0.45))
            # the small count above it: "1", "1 2", "1 2 3" (the newest bright)
            digits = [str(j + 1) for j in range(count)]
            txt_w = len(digits) * (ADV + 2) - 2
            y_ = int(round(gy - REST_G.shape[0] / 2 - 11))
            x_ = _safe_x(int(round(sx - txt_w / 2)), txt_w, y_, 7)
            if y_ < CAP_BAND and caption_up(t):                       # under a caption: beside the rest glyph instead
                y_ = int(round(gy - 3))
                x_ = int(round(sx + REST_G.shape[1] / 2 + 4))
                if x_ + txt_w > (300 if y_ + 7 > 290 else SAFE[2]) - 1:
                    x_ = int(round(sx - REST_G.shape[1] / 2 - 4 - txt_w))
            if SAFE[1] <= y_ <= SAFE[3] - 8 and not (y_ < CAP_BAND and caption_up(t)):
                for j, dg in enumerate(digits):
                    c_ = (255, 250, 236) if j == count - 1 else tuple(int(v) for v in np.clip(COL[k] * 0.5 + 90, 0, 255))
                    label(img, dg, x_ + j * (ADV + 2), y_, c_, 1)
        mk = melody_k(k, t)
        rising = [te for te, kk in NOTE_EV if kk == k and 0 <= min(t, T_KO) - te < 0.9 and mk > 0]
        if t >= T_HS and k == WIN:                                     # the champion's solo: eighths on each note
            rising = [s_ for s_ in SOLO_T if 0 <= t - s_ < 0.9]
        for j, te in enumerate(rising):
            age = (min(t, T_KO) if t < T_HS else t) - te
            x_ = sx + (8 + 3 * j) * (1 if (int(te * 10) % 2) else -1) + 4 * math.sin(age * 7)
            y_ = sy - rp * 0.4 - 26 * age
            ST.stamp_glyph(img, EIGHTH_G, x_, y_, tuple(int(v) for v in np.clip(COL[k] * 0.5 + 125, 0, 255)),
                           alpha=1 - ss(0.5, 0.9, age), shadow=False)


# ---------------------------------------------------------------- frame
def render(f):
    t = f / FPS
    cam = camera(t)
    L = lights(t, cam)
    fs = [fighter(k, t) for k in (0, 1)]
    i = sim_i(t)
    in_fight = T.T_FIGHT <= t < T_HS
    keys = []
    if in_fight:
        K = RUN['K'][i]
        keys = [row for row in K if not np.isnan(row[0])]
    if T_KO <= t < T_KO + 0.9 and t >= T_HS:                      # darts in flight at the K.O. drop and fade
        K = RUN['K'][-1]
        keys = [row for row in K if not np.isnan(row[0])]
    notes = notes_at(t)
    spot = S.SPOT
    aim = spot_aim(t)
    axis = (aim - spot) / np.linalg.norm(aim - spot)
    wk = win_k(t)

    fr = Frame((10, 8, 18))
    # ---------------- arena floor: per pixel (spill under the ring, corner lights)
    arr, depth = fr.array(), fr.depth()
    X, Z, dd = cam.ground_hit(XX_, YY_, S.FLOOR_Y)
    rr = np.hypot(X, Z)
    I = np.full(X.shape + (3,), 0.0) + np.array([0.20, 0.18, 0.26])
    I += (np.exp(-np.maximum(rr - 4.6, 0) / 3.0) * 0.55)[..., None] * np.array([1.0, 0.95, 1.05])
    for lp, lc, R_ in L.points:
        dist = np.sqrt((lp[0] - X) ** 2 + (lp[1] - S.FLOOR_Y) ** 2 + (lp[2] - Z) ** 2)
        I += ((lp[1] - S.FLOOR_Y) / dist / (1 + (dist / (R_ * 1.8)) ** 2))[..., None] * lc[None, None] * 0.9
    alb = S.ARENA_FLOOR[None, None] * (1 + 0.06 * ((np.floor(X * 1.2) + np.floor(Z * 1.2)) % 2))[..., None]
    arr[:] = np.floor(alb * I / 6 + BAYER_FULL[..., None]) * 6
    depth[:] = dd
    fr.set_array(arr, depth)
    arena(fr, cam, L, t)
    # ---------------- ring skirt (front sides only face the camera)
    for Q, n, c in S.SKIRT_FACES:
        if np.dot(n, cam.pos - Q.mean(0)) > 0:
            cc = shade(L, n[None], Q.mean(0)[None], c, rimk=0.3)[0]
            fr.poly(cam, Q, cc)
    # ---------------- canvas: per pixel
    arr, depth = fr.array(), fr.depth()
    Xc, Zc, dc = cam.ground_hit(XX_, YY_, 0.0)
    sx, sz = S.to_ring(Xc, Zc)
    on = (np.abs(sx) <= S.AP) & (np.abs(sz) <= S.AP)
    ys, xs = np.nonzero(on)
    if len(xs):
        Xp, Zp = Xc[ys, xs], Zc[ys, xs]
        sxp, szp = sx[ys, xs], sz[ys, xs]
        alb = S.canvas_albedo(sxp, szp)
        alb = MK.apply(alb, sxp, szp, fx_clock(t), BAYER_FULL[ys, xs])          # fading marks (reelkit.fx)
        sh = CANVAS_GRID.sample(canvas_shadows(cam, t, fs, keys, spot, notes), Xp, Zp)
        v = np.stack([Xp - spot[0], -spot[1] * np.ones_like(Xp), Zp - spot[2]], 1)
        dist = np.linalg.norm(v, axis=1)
        cosang = (v @ axis) / dist
        cone = np.clip((cosang - S.SPOT_COS[0]) / (S.SPOT_COS[1] - S.SPOT_COS[0]), 0, 1)
        lam = spot[1] / dist
        Ic = np.tile(L.amb * 0.75, (len(Xp), 1))
        Ic += ((1 - 0.85 * sh) * cone * lam * 0.86)[:, None] * np.array([1.0, 0.98, 0.95])[None]
        for lp, lc, R_ in L.points:
            dl = np.sqrt((lp[0] - Xp) ** 2 + lp[1] ** 2 + (lp[2] - Zp) ** 2)
            Ic += (lp[1] / dl / (1 + (dl / R_) ** 2))[:, None] * lc[None] * 0.8
        # decals: the resting viola's pool (it pulses on each count), SCREECH's ring, the big note's target
        for k in VIOLA:
            st = fs[k]
            resting, count, mel = rest_state(k, t)
            mk_ = melody_k(k, t)
            if in_fight and (resting or mk_ > 0):
                rd = np.hypot(sxp - st['p'][0], szp - st['p'][1])
                ph = ((t - T.T_FIGHT) % T.BEAT) / T.BEAT
                ring_r = R * (1.15 + 0.7 * ph)
                band = np.clip(1 - np.abs(rd - ring_r) / 0.06, 0, 1) * (1 - ph) * (0.6 if resting else 0.4)
                pool = np.clip(1 - rd / (R * 1.5), 0, 1) * 0.10 * (1 + mk_)
                Ic += (band * 0.8 + pool)[:, None] * (COL[k] / 255.0 + 0.1)[None]
        wav = RUN['WAVES'][i] if in_fight else []
        for owner, cx, cz, r, rmax, kind, t0 in wav:
            rd = np.hypot(sxp - cx, szp - cz)
            wdt = 0.32
            k_ = (1 - r / rmax) * 1.3
            ang_ = np.arctan2(szp - cz, sxp - cx)                    # the screech: a jagged, shivering ring (#28)
            rd = rd + 0.10 * (2 * ((7 * ang_ / math.pi + 6 * t) % 1) - 1) + 0.04 * np.sin(23 * ang_ - 50 * t)
            band = np.clip(1 - np.abs(rd - r) / wdt, 0, 1)
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (band * k_ * 0.9)[:, None] * (COL[owner] / 255.0 * 1.4 + 0.35)[None]
        for t0, k, v_, tl, x, z in JOKES:                            # VIOLA JOKE: the target under the falling note
            if t0 <= t < tl and t < T_HS:
                u = (t - t0) / (tl - t0)
                rd = np.hypot(sxp - x, szp - z)
                rad = CFG[k]['abilities']['low']['size'] * float(RUN['BL'][sim_i(t0), k])
                m = rd < rad * (0.4 + 0.6 * u)
                Ic[m] *= 1 - 0.5 * u
                ring = np.abs(rd - rad) < 0.05
                Ic[ring] += (COL[k] / 255.0) * (0.4 + 0.5 * (math.sin(t * 40) > 0))
            elif 0 <= t - tl < 0.6 and t < T_HS:
                u = (t - tl) / 0.6
                rd = np.hypot(sxp - x, szp - z)
                band = np.clip(1 - np.abs(rd - (1.2 + 1.8 * u)) / 0.2, 0, 1) * (1 - u)
                Ic += band[:, None] * np.array([0.6, 0.95, 0.9])[None]
        col = alb * Ic
        arr[ys, xs] = np.floor(col / 5 + BAYER_FULL[ys, xs, None]) * 5
        depth[ys, xs] = dc[ys, xs]
    fr.set_array(arr, depth)
    # ---------------- back ropes + posts
    def draw_rope_side(s):
        for j, y in enumerate(S.ROPE_Y):
            P = rope_points(s, y, t)
            base = S.ROPE_COL[j] if j != 1 else COL[0 if s in (0, 3) else 1] * 0.92    # middle rope: the fighter's side
            c = shade(L, np.array([[0, 1.0, 0.3]]) / np.linalg.norm([0, 1, 0.3]), P.mean(0)[None], base, rimk=0.5)[0]
            fr.line(cam, P, c, 2)

    def draw_post(name):
        for Q, n, c in S.POSTS[name]:
            if np.dot(n, cam.pos - Q.mean(0)) > 0:
                cc = shade(L, n[None], Q.mean(0)[None], c, rimk=0.6)[0]
                fr.poly(cam, Q, cc)
    draw_post('back')
    draw_rope_side(2)
    draw_rope_side(3)
    # ---------------- fighters, bow, darts, whole notes, posts (painter by depth)
    items = []
    for name in ('red', 'blue'):
        items.append((cam.depth(S.to_world(*S.CORNERS[name], 0.7))[0], 'post', name))
    for k, st in enumerate(fs):
        if not st['vis']:
            continue
        pw = S.to_world(st['p'][0], st['p'][1], R)
        dz = cam.depth(pw)[0]
        items.append((dz, 'ball', k))
        if k in BLADE and not st['shattered']:
            tip = st['p'] + (R + st['blen'] * 0.55) * np.array([math.cos(st['th']), math.sin(st['th'])])
            items.append((cam.depth(S.to_world(tip[0], tip[1], R))[0] + 0.01, 'bow', k))
    for row in keys:
        items.append((cam.depth(S.to_world(row[0], row[1], 0.45))[0], 'key', row))
    for nt in notes:
        nid, c, u, phi, r, lift, sc, glow, owner, kind = nt
        items.append((cam.depth(S.to_world(c[0], c[1], 0.5 * r * sc + lift))[0], 'note', nt))
    items.sort(key=lambda it: -it[0])
    drawn = {}
    for dz, kind, obj in items:
        if kind == 'post':
            draw_post(obj)
        elif kind == 'ball':
            st = fs[obj]
            arr, depth = fr.array(), fr.depth()
            drawn[obj] = draw_ball(arr, depth, cam, obj, st, t)
            fr.set_array(arr, depth)
        elif kind == 'bow':                                            # #28's bow, its motion fan and TREMOLO's blur
            st = fs[obj]
            mult = CFG[obj]['abilities']['unlock2']['mult'] if st['haste'] > 0 else 1.0
            dth = min(2.4, st['om'] * mult * 0.055) * st['sd']
            arr, depth = fr.array(), fr.depth()
            for pts, u in ST.SV.bow_trail_polys(cam, st, dth):
                if st['haste'] > 0:
                    layer_blend(arr, [pts], COL[obj] * 0.55 + 115 * (1 - 0.3 * u), 0.40 * (1 - u))
                else:
                    layer_blend(arr, [pts], (236, 232, 222), min(0.30, 0.04 + 0.02 * st['om']) * (1 - u))
            fr.set_array(arr, depth)
            ST.SV.draw_bow(fr, cam, L, shade, st)
        elif kind == 'key':
            row = obj
            ST.SV.draw_dart(fr, cam, L, shade, row[0], row[1], row[2], COL[int(row[3])])
        elif kind == 'note':
            nid, c, u, phi, r, lift, sc, glow, owner, nkind = obj
            ST.draw_note(fr, cam, L, shade, c, r, phi, COL[owner], lift=lift, scale=sc,
                         glow=max(glow, 0.5 * melody_k(owner, t)))
    # ---------------- front ropes + posts
    draw_rope_side(0)
    draw_rope_side(1)
    draw_post('front')
    arr, depth = fr.array(), fr.depth()
    # ---------------- shards, sparks
    for kind, p, rot, size, curl, a in parts_at(t):
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind == 'glass':
            tri = [np.array([sx_, sz_]) + d * size, np.array([sx_, sz_]) - d * size * 0.6 + nn * size * 0.7,
                   np.array([sx_, sz_]) - d * size * 0.5 - nn * size * 0.6]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
            layer_blend(arr, [sp], COL[LOSE] * 0.35 + 170, 0.85)
            for (xq, yq) in sp.astype(int):
                if 0 <= xq < W and 0 <= yq < H:
                    arr[yq, xq] = COL[LOSE] * 0.6 + 100
        elif kind == 'string':                                    # a string curling free (#28)
            pts = [np.array([sx_, sz_]) + d * (j - 3) * 0.16 + nn * curl * 0.10 * math.sin(j) for j in range(7)]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            ImageDraw.Draw(im).line([tuple(q) for q in sp], fill=(230, 226, 214), width=1)
            arr = np.asarray(im).astype(np.float32)
        elif kind == 'bowbit':                                    # a piece of the bow stick (#28)
            q4 = [np.array([sx_, sz_]) + d * 0.18 * a1 + nn * 0.02 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            layer_blend(arr, [sp], ST.SV.STICK * 1.3, 1.0)
        elif kind == 'bridge':                                    # the maple bridge, pale, with its two feet
            q4 = [np.array([sx_, sz_]) + d * 0.12 * a1 + nn * 0.05 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            layer_blend(arr, [sp], np.array([228, 198, 148]), 1.0)
        elif kind == 'scroll':                                    # the scroll: a small varnished spiral
            pts = [np.array([sx_, sz_]) + (0.13 - 0.016 * j) * np.array([math.cos(rot + j * 0.9), math.sin(rot + j * 0.9)])
                   for j in range(8)]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            ImageDraw.Draw(im).line([tuple(q) for q in sp], fill=tuple(int(v) for v in WOOD_COL * 1.2), width=2)
            arr = np.asarray(im).astype(np.float32)
        else:                                                     # wood: tumbling flat chips in its varnish
            tri = [np.array([sx_, sz_]) + d * size, np.array([sx_, sz_]) - d * size * 0.7 + nn * size * 0.8,
                   np.array([sx_, sz_]) - d * size * 0.4 - nn * size * 0.7]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
            layer_blend(arr, [sp], np.clip(WOOD_COL * (0.75 + 0.25 * math.cos(rot * 2)) * 1.2, 0, 255), 1.0)
    sparks(arr, cam, t)
    tc_ = fx_clock(t)
    arr = PA.draw(arr, cam, tc_, light=lambda P: 1.0 + 0.6 * FL.tint(P, tc_))
    cs = cam.wp(np.array([[0.0, 0.3, 0.0]]))[0]
    fr.set_array(arr, depth)
    # ---------------- lens (focus locked on the ring), glows, the spot beam, bloom, grade
    img, depth = fr.array(), fr.depth()
    zf = cam.depth(S.SUBJECT)[0]
    img = dof(img, coc_map(depth, zf, 26.0, sharp=0.17))
    img = shimmers(img, cam, t)
    for name, lp in (('red', S.RED_LIGHT), ('blue', S.BLUE_LIGHT)):
        s_ = cam.wp(lp[None])[0]
        fr.glow(s_[0], s_[1], 16, tuple(COL[0] if name == 'red' else COL[1]), 0.35 + 0.15 * T.beat_pulse(t, 0.2))
    for k, (s_, rp) in drawn.items():
        st = fs[k]
        if st['glow'] > 0.05:
            fr.glow(s_[0], s_[1], rp * 2.2, tuple(COL[k] * 0.8 + 50), 0.45 * st['glow'])
    for nt in notes:                                                  # a soft teal glow round each rolling note
        nid, c, u, phi, r, lift, sc, glow, owner, kind = nt
        s_ = cam.wp(S.to_world(c[0], c[1], 0.5 * r * sc + lift)[None])[0]
        fr.glow(s_[0], s_[1], 10 + 14 * r * sc, tuple(COL[owner] * 0.7 + 60), 0.10 + 0.25 * glow)
    for e_i, e in events_between(t - 0.15, t, ('hit',)):
        if e[7] in ('wave', 'blade', 'note'):
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            big = e[7] != 'blade'
            fr.glow(s_[0], s_[1], 14 if big else 8, tuple(COL[e[2]] * 0.7 + 70),
                    (0.35 if big else 0.22) * (1 - (t - e[0]) / 0.15))
    if T_KO <= t < T_HS + 0.25:
        s_ = cam.wp(S.to_world(P_KO[LOSE][0], P_KO[LOSE][1], R)[None])[0]
        fr.glow(s_[0], s_[1], 46, (255, 240, 220), 0.9 * (1 - ss(T_KO, T_HS + 0.25, t)))
    img = apply_glows(img, fr.glows)
    # the spot's beam: a soft wedge of light from the can down to the pool
    s0 = cam.wp(spot[None])[0]
    pool_r = spot[1] * math.tan(math.acos(S.SPOT_COS[0])) * 0.92
    circ = cam.wp(np.array([aim + [pool_r * math.cos(a), 0.0, pool_r * math.sin(a)] for a in np.linspace(0, 2 * math.pi, 33)[:-1]]))
    pts = np.vstack([s0[None], circ])
    from scipy.spatial import ConvexHull
    hull = pts[ConvexHull(pts).vertices]
    m = Image.new('L', (W, H), 0)
    ImageDraw.Draw(m).polygon([tuple(p) for p in hull], fill=255)
    mk = np.asarray(m, np.float32) / 255.0
    fall = np.clip(1 - (YY_ - s0[1]) / 520.0, 0.25, 1.0)
    beam = np.floor((mk * fall * (0.10 + 0.04 * wk)) * 10 + BAYER_FULL * 0.9) / 10
    img = img + beam[..., None] * np.array([255, 246, 230])[None, None] * 0.55
    if -30 < s0[1] < 60:
        img = apply_glows(img, [(s0[0], s0[1], 30, (255, 244, 220), 0.9), (s0[0], s0[1], 10, (255, 255, 255), 1.0)])
    img = grade(bloom(np.clip(img, 0, 255), thr=225, k=0.22), contrast=0.18, sat=1.18)
    img = np.clip(img * vignette(strength=0.45) * top_scrim(0.45, 100, 90), 0, 255)
    img = post_fx(img, cam, t, fs, drawn)
    img = np.clip(img, 0, 255).astype(np.uint8)
    img = np.ascontiguousarray(img)
    # ---------------- the rests + counts, rising eighths (crisp, over the scene)
    if not (TT.door_offset(t) == 0.0):
        draw_rests(img, t, cam, drawn)
    # ---------------- UI: countdown, HP numbers in the balls, fight HUD, captions
    countdown(img, t, cs)
    card_closed = TT.door_offset(t) == 0.0                            # the title card covers the HUD: skip it
    for k, (s_, rp) in (drawn.items() if not card_closed else ()):
        st = fs[k]
        if st['scale'] < 0.95 or T.T_FIGHT - 0.05 <= t < T.T_FIGHT + 0.85:  # hidden while FIGHT! covers the balls
            continue
        hp = hp_at(k, t)
        txt = str(int(math.ceil(hp - 1e-6)))
        w_ = len(txt) * ADV - 1
        x = int(round(s_[0] - w_ / 2))
        y = int(round(s_[1] + rp - 10))
        if SAFE[0] <= x and x + w_ <= (300 if y + 7 > 290 else SAFE[2]) and SAFE[1] <= y and y + 7 <= SAFE[3]:
            img[y - 1:y + 8, x - 2:x + w_ + 2] = (img[y - 1:y + 8, x - 2:x + w_ + 2] * 0.35 + np.array(INK) * 0.65).astype(np.uint8)
            label(img, txt, x, y, (255, 244, 224), 1)
    hp_lag = [hp_at(k, max(0.0, t - 0.45)) for k in (0, 1)]
    hud = []
    for k in (0, 1):
        stt, sk = status(k, t)
        fl = 0.0
        for _, e in events_between(t - 0.12, t, ('hit',)):
            if e[3] == k and e[7] != 'key':
                fl = max(fl, 1 - (t - e[0]) / 0.12)
        lvl = fs[k]['level'] if T.T_FIGHT <= t else 1
        hud.append(dict(name=NAMES[k], color=COL[k], hp=hp_at(k, t), trail=max(hp_at(k, t), hp_lag[k]), level=lvl,
                        status=stt, status_k=sk, flash=fl, ko=(k == LOSE and T_KO <= t < T.T_GLIDE), t=t,
                        win=(k == WIN and T_KO + 1.3 <= t < T.T_GLIDE)))
    if not card_closed:
        fight_hud(img, hud[0], hud[1], abilities=(ability_rows(0, t), ability_rows(1, t)))
    for k in (0, 1):
        c = tuple(int(v) for v in np.clip(COL[k] * 0.8 + 60, 0, 255))
        draw_captions(img, t, CAP_ABIL[k], style='impact', base=c, puff=tuple(int(v) for v in COL[k]))
    draw_captions(img, t, CAP_KO, style='impact', hi=(255, 214, 96))
    wc = tuple(int(v) for v in np.clip(COL[WIN] * 0.8 + 60, 0, 255))
    draw_captions(img, t, CAP_WINS, style='impact', base=wc)
    draw_captions(img, t, CAP_COMMENT, style='impact', hi=(255, 214, 96))
    img = TT.draw(img, t)                                           # the VS title card, over everything
    return img


if __name__ == '__main__':
    build_sprites()
