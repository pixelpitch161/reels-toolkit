"""#29 BALLFIGHT #3: Trumpet vs Sax - renderer, v3 (BALLFIGHT v2 redesign: VALVE COMBO vs STRAP FLAIL, reelkit.fx).
#28's renderer, copied, with this pair's weapons and abilities; the weapon looks live in styles.py. v2 (beam vs smoke
rings) is kept as render_v2_backup.py / styles_v2_backup.py / audio_v2_backup.py (its seed 337 ran on the old roster).

arena floor (per pixel: spill, corner lights, attack lights) -> stands + crowd (phones, flashbulbs, attack light) ->
ring skirt -> canvas (per pixel: the spot with real shadows of balls, pistons, strap + bell, posts, ropes and the honk
bell; corner + attack lights; fading marks; decals: the HIGH C streak, the fanfare ring, the honk's target) -> back ropes
+ posts -> fighters, pistons, strap flail, smoke screen (painter) -> front ropes + posts -> effects (wrap coils, the
HIGH C sight + blast, fanfare banners, the honk bell, sparks, steam / smoke / brass particles, shards, solo notes) ->
countdown -> locked DOF on the ring -> heat shimmer -> glows -> spot beam -> bloom -> grade -> scrim -> fringe,
ability dim + speed lines, impact bursts -> HP numbers, fight HUD, captions.

    python render.py                                   # sprite cache (check/sprites_ballfight3.npz)
    python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 10.4 17.1 24.0 29.7 42.7 43.9 49.4 53 58.6 60.7
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
COMBO = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'combo']       # TRUMPET: VALVE COMBO (pistons)
FLAIL = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'flail']       # SAX: STRAP FLAIL
BEAM = COMBO                                                            # the trumpet's bell (HIGH C, solo glow)
SMOKE = FLAIL                                                           # the sax's smoke
CORNER0 = [np.array(c) for c in RUN['corner']]
STORY = sim.story(RUN)

# ---------------------------------------------------------------- the fighters' own colours on the shared ring (series
# rule, user Oct 23: every instrument has its own colour; the corners are just left / right). Runtime patches of the
# imported scene, so projects/ballfight/scene.py stays as #27 uses it.
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


# beams: charges (t0, owner, aim, t_fire, length, width) and blasts (t_fire, owner, ox, oz, aim, length, width)
CHARGES = [(e[0], e[2], e[3], e[0] + e[4], e[5], e[6]) for e in EV if e[1] == 'charge']
BEAMS = [(e[0], e[2], e[3], e[4], e[5], e[6], e[7]) for e in EV if e[1] == 'beam']
BEAM_T = np.array([b_[0] for b_ in BEAMS]) if BEAMS else np.zeros(0)
HOLD = [CFG[k]['abilities']['low'].get('hold', 0.12) for k in (0, 1)]   # HIGH C (an ability beam) holds longer
VOLLEY_IDS = set(e[4] for e in EV if e[1] == 'fire')
# VALVE COMBO: every punch (t0, owner, index, aim, reach, combo no., haste time scale)
PUNCHES = [(e[0], e[2], e[3], e[4], e[5], e[6], 1.0 / (CFG[e[2]]['abilities']['unlock2']['mult']
           if float(RUN['HA'][min(len(RUN['HA']) - 1, max(0, int(round((e[0] - T.T_FIGHT) * FPS)))), e[2]]) > 0 else 1.0))
           for e in EV if e[1] == 'punch']
PUNCH_T = np.array([q[0] for q in PUNCHES]) if PUNCHES else np.zeros(0)
# STRAP FLAIL: wraps (t0, owner, t_end) - the strap holds the enemy until t_end (or the K.O.)
WRAPS = [(e[0], e[2], e[3]) for e in EV if e[1] == 'wrap']


def punches_live(t):
    """pistons out at t: [(owner, index, aim, ext, stretch)] (they freeze with the hit-stop)"""
    if not len(PUNCH_T) or t < T.T_FIGHT:
        return []
    tt = min(t, T_KO)
    j0, j1 = np.searchsorted(PUNCH_T, tt - 0.6), np.searchsorted(PUNCH_T, tt, 'right')
    out = []
    for j in range(j0, j1):
        t0, k, i, a, reach, cn, ks = PUNCHES[j]
        w = CFG[k]['weapon']
        u = (tt - t0) / ks
        ext = ST.piston_ext(u, w, reach)
        if ext is not None:
            stretch = max(0.0, 1 - u / w['t_out']) if u < w['t_out'] else 0.0
            off = (i - (w['punches'] - 1) / 2) * w['spread']
            out.append((k, i, a, off, ext, stretch))
    if T_KO <= t < T_HS + 0.2 and t >= T_HS:                              # ... then they pull home after the hit-stop
        out = [(k, i, a, off, ext * max(0.0, 1 - (t - T_HS) / 0.2), 0.0) for k, i, a, off, ext, stretch in out]
    return out


def hc_mode(k, t):
    """0..1: how far the trumpet has turned from 'valves at the enemy' to 'bell down the HIGH C line' (eased in as the
    charge starts, out after the blast)"""
    for t0, who, aim, t_fire, ln, wd in CHARGES:
        if who != k:
            continue
        t_end = t_fire + HOLD[k] + ST.FADE
        if t0 - 0.02 <= t < t_end + 0.2:
            return ss(t0 - 0.02, t0 + 0.13, t) * (1 - ss(t_end, t_end + 0.2, t))
    return 0.0


def hc_aim(k, t):
    """the HIGH C line's aim (the latest charge at or before t)"""
    a = None
    for t0, who, aim, t_fire, ln, wd in CHARGES:
        if who == k and t0 - 0.02 <= t:
            a = aim
    return a


def wrap_at(k, t):
    """(k_in 0..1, t0, t_end) if fighter k's strap is wrapped round the enemy at t, else None"""
    if t >= T_HS + 0.25:
        return None
    tt = min(t, T_KO)
    for t0, who, t_end in WRAPS:
        if who == k and t0 <= tt < min(t_end, T_KO + 1e-6):
            k_in = min(1.0, (tt - t0) / 0.15) * min(1.0, (t_end - tt) / 0.12 if t_end <= T_KO else 1.0)
            if t >= T_HS:                                               # the K.O.: the strap lets go
                k_in *= 1 - (t - T_HS) / 0.25
            return max(0.0, k_in), t0, t_end
    return None


def charge_at(k, t):
    """the open charge of fighter k at t: (t0, aim, t_fire, length) or None"""
    for t0, who, aim, t_fire, ln, wd in CHARGES:
        if who == k and t0 <= t < t_fire and t < T_KO:
            return t0, aim, t_fire, ln
    return None


def beams_live(t):
    """blasts visible at t (they freeze with the hit-stop, then fade)"""
    if not len(BEAM_T):
        return []
    j0, j1 = np.searchsorted(BEAM_T, t - 0.4), np.searchsorted(BEAM_T, min(t, T_KO), 'right')
    out = []
    for j in range(j0, j1):
        tf, k, ox, oz, a, ln, wd = BEAMS[j]
        tt = min(t, T_KO) if t < T_HS else t - HITSTOP
        st = ST.beam_state(tt, tf, HOLD[k])
        if st is not None:
            out.append((k, ox, oz, a, clip_len(ox, oz, a, ln), wd, st[0], st[1], st))
    return out


def clip_len(ox, oz, a, ln, lim=3.25):
    """the visible blast ends at the ropes (it stays inside the ring, and inside the lens's sharp band)"""
    d = (math.cos(a), math.sin(a))
    s_max = ln
    for o, dv in ((ox, d[0]), (oz, d[1])):
        if abs(dv) > 1e-9:
            s_max = min(s_max, ((lim if dv > 0 else -lim) - o) / dv)
    return max(0.2, s_max)


def fire_centre(ox, oz, a):
    """the trumpet ball's centre when it fired (the engine's beam origin sits R + 0.04 out along the aim)"""
    return np.array([ox, oz]) - (R + 0.04) * np.array([math.cos(a), math.sin(a)])


# the winner's solo: one floating note per solo note, from the bell (trumpet) or the ball (sax)
def solo_notes(t):
    out = []
    for st_, (bt, m, d) in zip(SOLO_T, T.SOLO[NAMES[WIN]]):
        u = t - st_
        if 0 <= u < 1.4:
            out.append((u, m, d))
    return out


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


OMEGA = [CFG[k]['weapon'].get('spin', CFG[k]['weapon'].get('spin0', 0.0)) for k in (0, 1)]   # base spin (rev + tail)
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
FRETS0 = [CFG[k]['weapon'].get('frets0', 0) for k in (0, 1)]


def fighter(k, t):
    """picture state of fighter k at time t: dict(p (ring frame), vis, th, blen, frets, shield, haste, glow, scale)"""
    st = dict(vis=True, shield=0.0, haste=0.0, glow=0.0, scale=1.0, th=0.0, blen=BASE_LEN[k], frets=FRETS0[k],
              level=1, shattered=False, om=OMEGA[k], sd=SD0[k], charge=0.0, hc=0.0)
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
        st['frets'] = int(RUN['FR'][i, k])
        st['shield'] = float(RUN['SH'][i, k])
        st['haste'] = float(RUN['HA'][i, k])
        st['level'] = int(RUN['LV'][i, k])
        st['om'] = float(RUN['OM'][i, k])
        st['sd'] = float(RUN['SPIN'][i, k])
        if k in COMBO:                                            # HIGH C: the bell glows as it charges, flares as it fires;
            tt = min(t, T_KO)                                     # every punch thumps the ball
            st['hc'] = hc_mode(k, tt)
            ch = charge_at(k, tt)
            if ch is not None:
                st['charge'] = (tt - ch[0]) / max(1e-3, ch[2] - ch[0])
            j = np.searchsorted(BEAM_T, tt, 'right') - 1
            fire = math.exp(-(tt - BEAM_T[j]) / 0.18) if j >= 0 else 0.0
            jp = np.searchsorted(PUNCH_T, tt, 'right') - 1
            thump = math.exp(-(tt - PUNCH_T[jp]) / 0.08) if jp >= 0 else 0.0
            st['glow'] = max(0.5 * st['charge'] ** 2, 0.8 * fire, 0.3 * thump)
            st['charge'] = max(st['charge'] ** 1.5, fire)
        if k in FLAIL:                                            # the sax glows as its swing builds momentum
            w = CFG[k]['weapon']
            st['glow'] = 0.08 + 0.3 * min(1.0, st['om'] / w['spin_max']) ** 2
            if wrap_at(k, t) is not None:
                st['glow'] = 0.45
        if t >= T_KO and k == LOSE:
            st['glow'] = 1.0
        return st
    blen_end = float(RUN['BL'][-1, k])
    fr_end = int(RUN['FR'][-1, k])
    gl = ease((t - T.T_GLIDE) / (T.T_REV - T.T_GLIDE))
    corner = CORNER0[k] + rev_offset(k, t) * ss(T.T_REV - 0.3, T.T_REV + 0.2, t)
    rev_glow = 0.25 * T.beat_pulse(t) * ss(T.T_REV, T.T_REV + 0.3, t)
    if k == WIN:
        centre = np.array([0.0, 0.0])
        u = ease((t - T_HS) / 1.8)
        bob = 0.06 * math.sin(2 * math.pi * (t - T_HS) / (2 * T.BAR)) * ss(T_HS, T_HS + 1.5, t)
        p = P_KO[k] + (centre - P_KO[k]) * u + np.array([bob, -bob])
        st['p'] = p + (corner - p) * gl
        th_cont = float(RUN['TH'][-1, k]) + float(RUN['SPIN'][-1, k]) * OMEGA[k] * (t - T_HS)
        st['th'] = theta_tail(k, t) + _wrap(th_cont - theta_tail(k, t)) * (1 - gl)
        st['blen'] = blen_end + (BASE_LEN[k] - blen_end) * gl
        st['frets'] = int(round(fr_end + (FRETS0[k] - fr_end) * gl))
        st['glow'] = max(rev_glow, (0.35 + 0.5 * solo_env(t)) * ss(T_HS, T_HS + 0.6, t) * (1 - gl))
        st['charge'] = solo_env(t) * (1 - gl) if k in BEAM else 0.0
        st['level'] = 3 if t < T.T_GLIDE else 1
        st['sd'] = float(RUN['SPIN'][-1, k]) if gl < 0.5 else SD0[k]
        return st
    # the loser: shattered until the reassembly, then glides home
    st['p'] = P_KO[k] + (corner - P_KO[k]) * gl
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
BIG = []                                             # (t, strength) camera shakes
for e in EV:
    if e[1] == 'hit' and e[7] in ('beam', 'slam', 'wave', 'shockwave'):
        BIG.append((e[0], {'beam': 0.7, 'slam': 3.5, 'wave': 2.5, 'shockwave': 1.6}[e[7]]))
    if e[1] == 'slam':
        BIG.append((e[0], 2.5))
BIG.append((T_KO, 5.0))
BIG_T = np.array([b[0] for b in BIG])
BIG_S = np.array([b[1] for b in BIG])


def shake(t):
    m = (BIG_T <= t) & (BIG_T > t - 0.4)
    if not m.any():
        return 0.0, 0.0
    a = float(np.max(BIG_S[m] * np.exp(-(t - BIG_T[m]) / 0.09)))
    return a * math.sin(t * 2 * math.pi * 23), a * 0.7 * math.cos(t * 2 * math.pi * 29)


def camera(t):
    cam = interp_keys(KEYS, t)
    dx, dy = SHK.offset(t)                                # reelkit.fx camera shake on heavy hits
    cam.cx += dx
    cam.cy += dy
    return cam


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
    FL.apply(L, t)                                        # attacks light the scene in the fighter's colour (reelkit.fx)
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
WAVE_EV = [(e[0], 1.0 if e[4] == 'wave' else 0.6) for e in EV if e[1] == 'ability' and e[4] in ('shockwave', 'wave')]
WAVE_EV += [(e[0], 0.9) for e in EV if e[1] == 'slam']


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
    disp += 0.012 * T.beat_pulse(t, 0.1) * np.sin(np.pi * u)                 # the bass thud
    sag = 0.035 * np.sin(np.pi * u)
    P = P + outn[None] * disp[:, None]
    return S.to_world(P[:, 0], P[:, 1], y - sag)


# ---------------------------------------------------------------- shards (the loser shatters at the K.O.)
def _shards():
    rng = np.random.default_rng(271)
    p0 = P_KO[LOSE]
    parts = []
    loser = NAMES[LOSE]
    n_glass = 24
    kinds = ['glass'] * n_glass + (['tube'] * 4 + ['valve'] * 3 + ['brass'] * 6 if loser == 'TRUMPET' else
                                   ['tube'] * 3 + ['keycap'] * 5 + ['reed'] * 1 + ['brass'] * 6)
    for kind in kinds:
        a = rng.uniform(0, 2 * math.pi)
        sp = rng.uniform(1.6, 4.2) if kind == 'glass' else rng.uniform(1.0, 3.0)
        parts.append(dict(kind=kind, p=np.array([p0[0], R, p0[1]], float) + rng.normal(0, 0.12, 3) * [1, 0.6, 1],
                          v=np.array([sp * math.cos(a), rng.uniform(1.5, 3.8), sp * math.sin(a)]),
                          rot=rng.uniform(0, 2 * math.pi), w=rng.uniform(-9, 9), size=rng.uniform(0.10, 0.22),
                          curl=rng.uniform(-1, 1)))
    # integrate at 120 Hz in particle time (0 .. 7 s): gravity, canvas bounce, friction, ropes as soft walls
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


def part_time(t):
    """particle clock: slow motion for the first 0.6 s after the hit-stop (25 %), then real time"""
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
            # fly back along a little arc, spinning home
            arc = np.array([0, 0.9 * math.sin(math.pi * back), 0])
            p = p + (tgt - p) * back + arc
            rot += 6 * back
        out.append((q['kind'], p, rot, q['size'] * (1 - 0.5 * back), q['curl'], 1.0))
    return out


# ---------------------------------------------------------------- sprites (Pixel Pitch models, cached)
_SPR = {}
SPR_PATH = os.path.join(HERE, 'check', 'sprites_ballfight3.npz')
ROLL_STEP = 10
SPR_RADII = list(range(12, 20))                 # measured: the sprites span 13.3-18.9 px (12 for the regrowing loser)


def _sprite_cache():
    if _SPR:
        return _SPR
    if os.path.exists(SPR_PATH):
        z = np.load(SPR_PATH)
        _SPR.update({k: z[k] for k in z.files})
    return _SPR


def sprite(name, radius_px, yaw_deg, roll):
    cache = _sprite_cache()
    r = int(np.clip(round(radius_px), SPR_RADII[0], SPR_RADII[-1]))
    yk = int(round(yaw_deg / 6.0)) * 6
    key = f'{name}_{r}_{yk % 360}_{int(roll)}'
    if key not in cache:
        import sprites as SP
        cache[key] = SP.render_sprite(name, r, float(yk), float(roll))
    return cache[key]


def sprite_jobs():
    """every sprite the video can use: the sax in its yaw window; the trumpet side-on (bell right, or bell left with the
    yaw turned 180) in its yaw window x every roll from -90 to 90 deg (it turns to its aim)"""
    jobs = set()
    for k in (0, 1):
        nm = CFG[k]['pp']
        yc, sw, roll = CFG[k]['view']
        for r in SPR_RADII:
            if k in COMBO:                                       # every roll: the trumpet turns all the way round
                for rl in range(-180, 180, ROLL_STEP):
                    jobs.add((nm, r, int(round(yc / 6.0)) * 6 % 360, rl))
            else:
                for yk in range(int((yc - sw - 6) // 6) * 6, int(yc + sw + 12), 6):
                    jobs.add((nm, r, yk % 360, int(roll)))
    return sorted(jobs)


def _sprite_job(job):
    import sprites as SP
    nm, r, yk, rl = job
    return f'{nm}_{r}_{yk}_{rl}', SP.render_sprite(nm, r, float(yk), float(rl))


def build_sprites():
    """precompute every sprite the video can use -> check/sprites_ballfight3.npz (2 processes)"""
    from multiprocessing import Pool
    with Pool(2) as P:
        out = dict(P.map(_sprite_job, sprite_jobs(), chunksize=8))
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


def add_light(arr, polys, col, k):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if len(pts) >= 3:
            d.polygon([tuple(p) for p in pts], fill=255)
    mk = np.asarray(m, np.float32) / 255.0
    arr += mk[..., None] * np.array(col, float)[None, None] * k


def ring_circle(cx, cz, r, y=0.02, n=40):
    a = np.linspace(0, 2 * math.pi, n + 1)[:-1]
    return S.to_world(cx + r * np.cos(a), cz + r * np.sin(a), y)


def screen(cam, P):
    return cam.wp(np.atleast_2d(P))


# ---------------------------------------------------------------- weapons: see styles.py (bow, drumhead, darts, cymbal, beater)


# ---------------------------------------------------------------- glass balls
def screen_angle(cam, p, th):
    s0 = cam.wp(S.to_world(p[0], p[1], R)[None])[0]
    q = p + np.array([math.cos(th), math.sin(th)])
    s1 = cam.wp(S.to_world(q[0], q[1], R)[None])[0]
    return math.degrees(math.atan2(s1[1] - s0[1], s1[0] - s0[0]))


def trumpet_alpha(cam, k, st, t):
    """the side-on trumpet's screen rotation (deg, y down): valves up (-90 deg) turned to the punch aim, blended toward
    'bell (0 deg) down the HIGH C line' by the HIGH C mode"""
    a_valves = screen_angle(cam, st['p'], st['th']) + 90.0
    hc = st.get('hc', 0.0)
    if hc <= 0:
        return a_valves
    a_bell = screen_angle(cam, st['p'], hc_aim(k, t))
    return a_valves + ((a_bell - a_valves + 180) % 360 - 180) * hc


def aim_roll(cam, p, th, yc):
    """(sprite roll, base yaw) so a side-on instrument (bell right at yaw yc) points along th on screen with its valves up"""
    s0 = cam.wp(S.to_world(p[0], p[1], R)[None])[0]
    q = p + np.array([math.cos(th), math.sin(th)])
    s1 = cam.wp(S.to_world(q[0], q[1], R)[None])[0]
    phi = math.degrees(math.atan2(s1[1] - s0[1], s1[0] - s0[0]))         # screen angle, y down
    if abs(phi) <= 90:
        return -phi, yc
    return (180 - phi) if phi > 0 else (-180 - phi), yc + 180


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
    nz = np.sqrt(np.clip(1 - d2, 0, 1))
    lam = np.clip(-0.25 * dx - 0.55 * dy + nz * 0.6, 0, 1)                   # lit from the spot above (top of the ball)
    band = np.floor(lam * 4 + BAYER_FULL[:n, :n] * 0.9) / 4
    inner = (col * 0.30 + np.array([236, 232, 226]) * 0.70)[None, None] * (0.70 + 0.36 * band[..., None])
    m = inside[sl]
    reg[m] = reg[m] * 0.12 + inner[sl][m] * 0.88
    # the instrument
    name = CFG[k]['pp']
    yc, sw, roll = CFG[k]['view']
    if k in COMBO:                                   # v3: the trumpet turns its valves toward the punches (the pistons
        yaw = yc                                     # come out of them); for HIGH C it swings its bell down the line
        roll = int(round(-trumpet_alpha(cam, k, st, t) / ROLL_STEP)) * ROLL_STEP
        roll = (roll + 180) % 360 - 180
    else:
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
        rg[ms] = np.clip(lit[ms], 0, 255)
    rc = np.clip(col * (0.9 + 0.5 * glow) + 50 * glow, 0, 255)
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
    title = [(1.0 / FPS, T.T_FIGHT - 0.06, [f'*{NAMES[0]}*', f'VS ~{NAMES[1]}~'])]
    abil = [[], []]
    for t, who, name in STORY['caps']:
        abil[who].append((t, t + 2.4, [name]))
    ko = [(T_KO, T_KO + 1.25, ['*K.O.*'])]
    wins = [(T_KO + 1.3, T_KO + 3.55, [NAMES[WIN], 'WINS!'])]
    comment = [(T_KO + 3.65, T.T_REASSEMBLE, ['COMMENT THE', '*NEXT MATCH-UP*'])]
    return title, abil, ko, wins, comment


CAP_TITLE, CAP_ABIL, CAP_KO, CAP_WINS, CAP_COMMENT = _caps()
CAPTIONS = CAP_TITLE + CAP_ABIL[0] + CAP_ABIL[1] + CAP_KO + CAP_WINS + CAP_COMMENT     # for audio (impact thuds)
STATUS = [(e[0], e[2], e[3]) for e in EV if e[1] == 'ability']


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
    """3 . 2 . 1 . FIGHT! in the ring, one count per beat (the band's count-in); the '3' flies in during the last
    beat of the file and lands ON frame 0"""
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
    t_in = T.DUR - T.BEAT                                 # the '3' flies in during the last beat
    if t >= t_in:
        u = (t - t_in) / T.BEAT
        e_ = 1 - (1 - u) ** 2.5                           # eases out: nearly landed on the last frame, lands on frame 0
        big_text(img, '3', cx, cy, 78 * (0.15 + 1.1 * e_), (255, 244, 220), alpha=min(1.0, u * 1.6))


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
FLASH_EV = [e[0] for e in EV if (e[1] == 'hit' and e[7] in ('slam', 'wave', 'shockwave')) or e[1] == 'ability'] + \
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
    """tiers, people and LED boards in one painter pass (far to near)"""
    wk = win_k(t)
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
    crowd_lit = 1.0 + 0.9 * FL.tint(heads, t)                    # attack lights reach the crowd (reelkit.fx)
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
            if not (-6 < x < W + 6 and -6 < y < H + 10):
                continue
            side = CROWD_P[j, 0]
            tint = CROWD_TINT[0] if side < -1.5 else (CROWD_TINT[1] if side > 1.5 else np.array([1.0, 0.92, 1.0]))
            body = np.clip(S.CROWD_COLS[CROWD_CI[j]] * tint * spill * crowd_lit[j], 0, 255)
            draw.ellipse([x - 1.9 * rh, y + 0.6 * rh, x + 1.9 * rh, y + 3.4 * rh], fill=tuple(int(v) for v in body * 0.8))
            draw.ellipse([x - rh, y - rh, x + rh, y + rh], fill=tuple(int(v) for v in body * 0.62))
            # rim light from the ring on the heads (they face it)
            if rh >= 1.5:
                draw.line([x - rh * 0.6, y - rh * 0.8, x + rh * 0.6, y - rh * 0.8], fill=tuple(int(v) for v in np.clip(body * 1.5 + 30, 0, 255)))
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


def canvas_shadows(cam, t, fs, keys, spot, slams=()):
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
    for pk, pi, pa, poff, pext, pstr in punches_live(t):    # the pistons
        if fs[pk]['vis'] and not fs[pk]['shattered']:
            a0, tip, d_ = ST.piston_geom(fs[pk]['p'], pa, poff, pext)
            n_ = np.array([-d_[1], d_[0]]) * 0.05
            polys.append(proj_point(np.array([S.to_world(q[0], q[1], R) for q in (a0 - n_, tip - n_, tip + n_, a0 + n_)]), spot))
    for k in FLAIL:                                    # the strap + the bell head (or the wrap)
        st = fs[k]
        if st['shattered'] or not st['vis']:
            continue
        if wrap_at(k, t) is None:
            L_ = R + st['blen']
            pts = ST.strap_curve(st['p'], st['th'], L_, st['om'], CFG[k]['weapon']['spin_max'], st['sd'], n=6)
            hp = pts[-1]
        else:
            pts = [st['p'], fs[1 - k]['p']]
            hp = None
        for a_, b_ in zip(pts[:-1], pts[1:]):
            q = np.array([S.to_world(a_[0], a_[1], R * 0.95) + [0, -0.02, 0], S.to_world(b_[0], b_[1], R * 0.95) + [0, -0.02, 0],
                          S.to_world(b_[0], b_[1], R * 0.95) + [0, 0.02, 0], S.to_world(a_[0], a_[1], R * 0.95) + [0, 0.02, 0]])
            polys.append(proj_point(q, spot))
        if hp is not None:
            w_ = S.to_world(hp[0], hp[1])
            polys.append(proj_point(np.array([[w_[0] + 0.18 * math.cos(u), R, w_[2] + 0.18 * math.sin(u)]
                                              for u in np.linspace(0, 2 * math.pi, 9)[:-1]]), spot))
    a8 = np.linspace(0, 2 * math.pi, 9)[:-1]
    for row in keys:                                  # smoke rings / notes: small soft blobs
        rr = 0.16 if int(row[6]) in VOLLEY_IDS else 0.07
        circ = np.array([[S.to_world(row[0], row[1])[0] + rr * math.cos(u), 0.5, S.to_world(row[0], row[1])[2] + rr * math.sin(u)]
                         for u in a8])
        polys.append(proj_point(circ, spot))
    for owner, cx, cz, t0, t_slam, rad in slams:      # the honk bell's shadow grows darker as it hovers lower
        h = ST.honk_height(t, t0, t_slam)
        if h is not None:
            w = S.to_world(cx, cz)
            mr = rad * 0.72
            circ = np.array([[w[0] + mr * math.cos(u), h, w[2] + mr * math.sin(u)] for u in np.linspace(0, 2 * math.pi, 17)[:-1]])
            polys.append(proj_point(circ, spot))
    return CANVAS_GRID.raster(polys, blur=0.7)


# ---------------------------------------------------------------- sparks
def puffs(arr, cam, t):
    """smoke rings that break up (pierced by a beam, blown away by a shockwave, hitting the ropes, landing, or fading)"""
    for i, e in events_between(t - 0.45, t, ('cleared', 'fade', 'wall', 'hit')):
        if e[1] == 'hit':
            if e[2] not in SMOKE or e[7] != 'key':
                continue
            x, z = e[5], e[6]
        else:
            if e[2] not in SMOKE:
                continue
            x, z = e[4], e[5]
        u = (t - e[0]) / 0.45
        tint = ST.SMOKE * 0.6 + COL[SMOKE[0]] * 0.4
        for j in range(3):
            a = i * 2.1 + j * 2.09
            ST.draw_puff(arr, cam, x + 0.18 * u * math.cos(a), z + 0.18 * u * math.sin(a), 0.5 + 0.25 * u,
                         0.07 + 0.12 * u, tint, 0.5 * (1 - u))


def sparks(arr, cam, t):
    for i, e in events_between(t - 0.32, t, ('hit', 'parry', 'block', 'wall', 'cleared')):
        dt = t - e[0]
        kind = e[1]
        if kind in ('wall', 'cleared') and e[2] in SMOKE:
            continue                                               # smoke rings break up in puffs (puffs())
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            col = COL[att] * 0.6 + 100
            n, sp, life = (16, 3.2, 0.3) if how in ('slam', 'wave', 'shockwave', 'beam') else \
                ((9, 2.6, 0.22) if how in ('piston', 'flail') else (5, 1.8, 0.18))
        elif kind == 'parry':
            att, x, z = e[2], e[4], e[5]
            col, n, sp, life = np.array([230, 236, 255]), 6, 2.4, 0.2
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
        if kind == 'hit' and e[7] in ('beam', 'slam', 'flail') and dt < 0.08:
            s = cam.wp(S.to_world(x, z, R)[None])[0]
            arr[max(0, int(s[1]) - 1):int(s[1]) + 2, max(0, int(s[0]) - 1):int(s[0]) + 2] = 255


# ---------------------------------------------------------------- effects (reelkit.fx, BALLFIGHT v2): built once from the run
FL = fx.Flares(max_n=6)
BU = fx.Bursts()
PA = fx.Particles(seed=29)
MK = fx.Marks()
MO = fx.Moments()
SHK = fx.Shake(cap=2.5)
SMOKE_COLS = [(110, 132, 132), (140, 160, 158), (92, 108, 112)]          # sax smoke, a touch of teal


def fx_t(t):
    """effects clock: real time in the fight, held through the hit-stop, then running on"""
    return t if t < T_KO else (T_KO if t < T_HS else t - HITSTOP)


def _wdir(d):
    return tuple(S.to_world(d[0], d[1], 0.0))


def _fx_build():
    kt, ks = (COMBO[0] if COMBO else 0), (FLAIL[0] if FLAIL else 1)
    wt = CFG[kt]['weapon']
    wrapped = [(t0, min(t_end, T_KO)) for t0, who, t_end in WRAPS]
    # VALVE COMBO: steam bursts from the valve as each piston fires
    for t0, k, i, a, reach, cn, ks_ in PUNCHES:
        p = RUN['P'][sim_i(t0), k]
        a0, tip, d = ST.piston_geom(p, a, (i - (wt['punches'] - 1) / 2) * wt['spread'], 0.0)
        PA.emit(t0, S.to_world(a0[0], a0[1], R * 1.15), 'steam', n=3, dir=(0.0, 1.0, 0.0), vel=_wdir(d * 0.6))
    # STRAP FLAIL: smoke streams off the bell head along its path (thicker as it speeds up; a spiral in BEBOP RUN)
    tb = T.T_FIGHT
    while tb < T_KO:
        if not any(a <= tb < b for a, b in wrapped):
            i = sim_i(tb)
            p, th, om = RUN['P'][i, ks], float(RUN['TH'][i, ks]), float(RUN['OM'][i, ks])
            hp = p + (R + float(RUN['BL'][i, ks])) * np.array([math.cos(th), math.sin(th)])
            n = int(om > 6.0) + 2 * int(float(RUN['HA'][i, ks]) > 0) + int(int(tb / 0.07) % 2 == 0)
            PA.emit(tb, S.to_world(hp[0], hp[1], R), 'smoke', n=n, col=SMOKE_COLS, speed=(0.05, 0.25), scale=0.7,
                    life=(0.6, 1.0))
        tb += 0.07
    # hits, blocks, wraps
    for e in EV:
        if e[0] > T_KO + 1e-6:
            break
        if e[1] == 'hit':
            att, x, z, how, sub = e[2], e[5], e[6], e[7], e[8]
            pos = S.to_world(x, z, R)
            if how == 'piston':
                big = sub == wt['punches'] - 1
                BU.add(e[0], pos, COL[att], 1.2 if big else 0.7, bright=big)
                FL.add(e[0], pos, COL[att], radius=1.4, peak=0.85 if big else 0.55, decay=0.2)
                PA.emit(e[0], pos, 'brass', n=8 if big else 4)
                PA.emit(e[0], pos, 'steam', n=2)
                MK.add(e[0], 'scorch', x, z, size=0.22 if big else 0.14, col=(150, 110, 30), life=2.5,
                       seed=int(e[0] * 100), strength=0.6)
                if big:
                    SHK.add(e[0], 0.9)
            elif how == 'flail':
                mom = sub / 10.0
                BU.add(e[0], pos, COL[att], 0.8 + 0.8 * mom, bright=mom > 0.5)
                FL.add(e[0], pos, COL[att], radius=1.6, peak=0.55 + 0.45 * mom, decay=0.26)
                PA.emit(e[0], pos, 'smoke', n=6, col=SMOKE_COLS, speed=(0.3, 0.9))
                p_s = RUN['P'][sim_i(e[0]), att]
                tang = math.atan2(z - p_s[1], x - p_s[0]) + math.pi / 2
                MK.add(e[0], 'claw', x, z, size=0.42, angle=tang, n=2, col=(40, 70, 66), life=3.5, seed=int(e[0] * 100))
                SHK.add(e[0], 0.5 + 1.2 * mom)
            elif how == 'beam':                                          # HIGH C lands
                BU.add(e[0], pos, COL[att], 2.0)
                FL.add(e[0], pos, COL[att], radius=2.4, peak=1.2, decay=0.4)
                PA.emit(e[0], pos, 'brass', n=18, speed=(2.0, 4.5))
                SHK.add(e[0], 2.5)
            elif how == 'slam':
                BU.add(e[0], pos, COL[att], 2.0)
                SHK.add(e[0], 2.5)
            elif how in ('shockwave', 'wave'):
                BU.add(e[0], pos, COL[att], 1.4)
                SHK.add(e[0], 1.5)
        elif e[1] == 'block':
            BU.add(e[0], S.to_world(e[4], e[5], R), (200, 230, 226), 0.6, bright=False)
            PA.emit(e[0], S.to_world(e[4], e[5], R), 'smoke', n=4, col=SMOKE_COLS)
        elif e[1] == 'wrap':                                            # the strap snaps round the enemy
            pe = RUN['P'][sim_i(e[0]), 1 - e[2]]
            PA.emit(e[0], S.to_world(pe[0], pe[1], R), 'smoke', n=8, col=SMOKE_COLS, spread=180, speed=(0.3, 0.8))
        elif e[1] == 'beam':                                            # HIGH C fires: the bell flares, the line scorches
            tf, k, ox, oz, a, ln, wd = e[0], e[2], e[3], e[4], e[5], e[6], e[7]
            d = np.array([math.cos(a), math.sin(a)])
            for u in (0.15, 0.45, 0.75):
                q = np.array([ox, oz]) + d * ln * u
                FL.add(tf, S.to_world(q[0], q[1], R), COL[k], radius=2.2, peak=0.7, decay=0.35, hold=0.15)
                MK.add(tf, 'scorch', q[0], q[1], size=0.5 * wd + 0.2, angle=a, col=(170, 120, 30), life=6.0,
                       seed=int(tf * 10 + u * 7), strength=0.8)
            PA.emit(tf, S.to_world(ox, oz, R), 'brass', n=14, dir=_wdir(d), spread=40, speed=(2.0, 4.0))
            SHK.add(tf, 1.2)
        elif e[1] == 'slam':                                            # HONK: the bell slams down: a smoke-ring crater
            cx, cz = e[3], e[4]
            FL.add(e[0], S.to_world(cx, cz, 1.0), COL[e[2]], radius=2.4, peak=1.1, decay=0.5)
            MK.add(e[0], 'smoke_ring', cx, cz, size=1.4, life=7.0, seed=int(e[0] * 10), strength=0.9)
            MK.add(e[0], 'scorch', cx, cz, size=0.9, col=(30, 60, 58), life=7.0, seed=int(e[0] * 10) + 1, strength=0.5)
            PA.emit(e[0], S.to_world(cx, cz, 0.1), 'smoke', n=24, col=SMOKE_COLS, spread=180, speed=(0.6, 1.6), scale=1.3)
            SHK.add(e[0], 2.0)
        elif e[1] == 'ability' and e[3] == 'FANFARE':
            p = RUN['P'][sim_i(e[0]), e[2]]
            FL.add(e[0], S.to_world(p[0], p[1], 1.0), COL[e[2]], radius=2.6, peak=0.9, decay=0.45)
    for t, who, name in STORY['caps']:
        MO.add(t, who, COL[who])
    # the K.O.
    p = P_KO[LOSE]
    pos = S.to_world(p[0], p[1], R)
    BU.add(T_KO, pos, COL[LOSE], 2.0)
    FL.add(T_KO, pos, COL[LOSE] * 0.9, radius=2.4, peak=1.1, decay=0.6)
    PA.emit(T_KO, pos, 'brass', n=26, spread=180, speed=(1.0, 3.0))
    PA.emit(T_KO, pos, 'smoke' if NAMES[LOSE] == 'SAX' else 'steam', n=14, col=SMOKE_COLS if NAMES[LOSE] == 'SAX' else None,
            spread=180, speed=(0.4, 1.2))
    SHK.add(T_KO, 2.5)
    # the champion's solo: a puff from the horn on every note (effects clock)
    for t0 in SOLO_T:
        if t0 < T.T_REASSEMBLE:
            c = fighter(WIN, t0)['p']
            PA.emit(t0 - HITSTOP, S.to_world(c[0], c[1], R * 1.6), 'steam' if NAMES[WIN] == 'TRUMPET' else 'smoke', n=2,
                    col=None if NAMES[WIN] == 'TRUMPET' else SMOKE_COLS)
            FL.add(t0 - HITSTOP, S.to_world(c[0], c[1], 1.0), COL[WIN], radius=1.6, peak=0.5, decay=0.2)


_fx_build()


def shimmers(img, cam, t, fs):
    """heat shimmer: the fanfare ring, the honk shock ring, and the air over the HIGH C blast"""
    if T.T_FIGHT <= t < T_HS:
        for owner, cx, cz, r_, rmax, kind_, t0 in RUN['WAVES'][sim_i(t)]:
            img = fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r_, 0.35, 2.6, t, k=1 - r_ / rmax, seed=owner)
    tc = fx_t(t)
    for e in EV:
        if e[1] == 'slam':
            u = (tc - e[0]) / 0.6
            if 0 <= u < 1:
                img = fx.shimmer_world(img, cam, S.to_world(e[3], e[4], 0.05), 1.35 + 1.6 * u, 0.35, 3.0, t, k=1 - u)
    for k, ox, oz, a_, ln, wd, ext, al, bst in beams_live(t):           # HIGH C: the air shivers along the blast
        d = np.array([math.cos(a_), math.sin(a_)])
        for u in (0.25, 0.55, 0.85):
            q = np.array([ox, oz]) + d * ln * ext * u
            img = fx.shimmer_world(img, cam, S.to_world(q[0], q[1], R), 0.35 + 0.6 * wd * (1 + bst[3]), 0.3, 2.4, t,
                                   k=al, seed=int(u * 10))
    return img


def post_fx(img, cam, t, fs, drawn):
    """after the grade: chromatic fringing on big hits, the ability moments (dim + speed lines), the impact bursts"""
    for _, e in events_between(t - 0.12, t, ('hit',)):
        if e[7] in ('beam', 'slam', 'shockwave', 'wave') or (e[7] == 'flail' and e[8] >= 7):
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            img = fx.fringe(img, s_[0], s_[1], 26 if e[7] != 'flail' else 18, 2.5 if e[7] != 'flail' else 1.6,
                            k=1 - (t - e[0]) / 0.12)
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
    ko_fall = 0.0
    if T_KO <= t < T_KO + 0.9 and t >= T_HS:                      # keys in flight at the K.O. drop and fade
        K = RUN['K'][-1]
        keys = [row for row in K if not np.isnan(row[0])]
        ko_fall = (t - T_HS) / (T_KO + 0.9 - T_HS)
    blasts = beams_live(t)                                          # beam blasts (they freeze with the hit-stop)
    sights = []                                                     # open charges: the locked aim
    for k in BEAM:
        ch = charge_at(k, min(t, T_KO)) if T.T_FIGHT <= t < T_KO else None
        if ch is not None:
            sights.append((k, ch[1], ch[3], fs[k]['charge']))
    slams = RUN['SLAMS'][i] if in_fight else []
    pun = punches_live(t)
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
        alb = MK.apply(alb, sxp, szp, min(t, T_KO) if t < T_HS else t - HITSTOP, BAYER_FULL[ys, xs])   # fading marks
        sh = CANVAS_GRID.sample(canvas_shadows(cam, t, fs, keys, spot, slams), Xp, Zp)
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
        # decals: shockwave light rings, the slam's shadow
        for k, ox, oz, a_, ln, wd, ext, al, bst in blasts:           # each blast throws a streak of light on the canvas
            d = np.array([math.cos(a_), math.sin(a_)])
            bm = ST.bell_mouth(fire_centre(ox, oz, a_), a_)
            lv = (ln + R + 0.04 - ST.BELL_IN * R) * ext
            rx, rz = sxp - bm[0], szp - bm[1]
            along = np.clip(rx * d[0] + rz * d[1], lv * bst[2], lv)
            off = np.hypot(rx - along * d[0], rz - along * d[1])
            band = np.clip(1 - off / (0.35 + wd * 1.5), 0, 1) ** 1.5
            band = np.floor(band * 4 + BAYER_FULL[ys, xs] * 0.9) / 4
            Ic += (band * al * 0.75)[:, None] * (COL[k] / 255.0 * 0.9 + 0.3)[None]
        for k, a_, ln, ck in sights:                                 # the charging bell lights a faint lane
            st = fs[k]
            d = np.array([math.cos(a_), math.sin(a_)])
            rx, rz = sxp - st['p'][0], szp - st['p'][1]
            along = rx * d[0] + rz * d[1]
            off = np.hypot(rx - along * d[0], rz - along * d[1])
            lane = (along > R) & (along < R + ln) & (off < 0.06)
            Ic += (lane * 0.18 * ck)[:, None] * (COL[k] / 255.0)[None]
        wav = RUN['WAVES'][i] if in_fight else []
        for owner, cx, cz, r, rmax, kind, t0 in wav:
            if kind == 'pulse':
                continue
            rd = np.hypot(sxp - cx, szp - cz)
            wdt = 0.16 if kind == 'shockwave' else 0.32
            k_ = (1 - r / rmax) * (1.0 if kind == 'shockwave' else 1.3)
            if kind == 'wave':                                        # the screech: a jagged, shivering ring
                ang_ = np.arctan2(szp - cz, sxp - cx)
                rd = rd + 0.10 * (2 * ((7 * ang_ / math.pi + 6 * t) % 1) - 1) + 0.04 * np.sin(23 * ang_ - 50 * t)
            band = np.clip(1 - np.abs(rd - r) / wdt, 0, 1)
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (band * k_ * 0.9)[:, None] * (COL[owner] / 255.0 * 1.4 + (0.35 if kind == 'wave' else 0.15))[None]
        for owner, cx, cz, t0, t_slam, rad in slams:
            rd = np.hypot(sxp - cx, szp - cz)
            if t < t_slam:
                u = (t - t0) / (t_slam - t0)
                m = rd < rad
                Ic[m] *= 1 - 0.55 * u
                ring = np.abs(rd - rad) < 0.05
                Ic[ring] += (COL[owner] / 255.0) * (0.4 + 0.5 * (math.sin(t * 40) > 0))
            else:
                u = (t - t_slam) / 0.6
                band = np.clip(1 - np.abs(rd - (rad + 1.6 * u)) / 0.2, 0, 1) * (1 - u)
                Ic += band[:, None] * np.array([0.9, 0.9, 1.0])[None]
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
    # ---------------- fighters, weapons, keys, posts (painter by depth)
    items = []
    for name in ('red', 'blue'):
        items.append((cam.depth(S.to_world(*S.CORNERS[name], 0.7))[0], 'post', name))
    for k, st in enumerate(fs):
        if not st['vis']:
            continue
        pw = S.to_world(st['p'][0], st['p'][1], R)
        dz = cam.depth(pw)[0]
        items.append((dz, 'ball', k))
        if st['shield'] > 0:
            items.append((dz + 0.03, 'smoke_back', k))
            items.append((dz - 0.03, 'smoke_front', k))
        if k in FLAIL and not st['shattered'] and wrap_at(k, t) is None:     # the strap flail (unless it's wrapped)
            hp = st['p'] + (R + st['blen']) * np.array([math.cos(st['th']), math.sin(st['th'])])
            items.append((cam.depth(S.to_world(hp[0], hp[1], R))[0], 'flail', k))
    for pk, pi, pa, poff, pext, pstr in pun:                          # VALVE COMBO: the pistons out right now
        if fs[pk]['vis'] and not fs[pk]['shattered']:
            a0, tip, d_ = ST.piston_geom(fs[pk]['p'], pa, poff, pext)
            mid = (a0 + tip) / 2
            items.append((cam.depth(S.to_world(mid[0], mid[1], R))[0], 'piston', (pk, pi, pa, poff, pext, pstr)))
    for row in keys:
        items.append((cam.depth(S.to_world(row[0], row[1], 0.5))[0], 'key', row))
    items.sort(key=lambda it: -it[0])
    drawn = {}
    drawn_bell = {k: cam.wp(S.to_world(*ST.bell_mouth(fs[k]['p'], hc_aim(k, t) if fs[k]['hc'] > 0.5 else fs[k]['th']),
                                       R)[None])[0]
                  for k in BEAM if fs[k]['vis'] and not fs[k]['shattered']}
    for dz, kind, obj in items:
        if kind == 'post':
            draw_post(obj)
        elif kind == 'ball':
            st = fs[obj]
            arr, depth = fr.array(), fr.depth()
            drawn[obj] = draw_ball(arr, depth, cam, obj, st, t)
            fr.set_array(arr, depth)
        elif kind == 'flail':
            st = fs[obj]
            w = CFG[obj]['weapon']
            L_ = R + st['blen']
            arr, depth = fr.array(), fr.depth()
            if st['om'] > 4.0:                                       # the swept arc behind a fast head
                ST.draw_swing_trail(arr, cam, st['p'], st['th'], L_, min(2.2, st['om'] * 0.07) * st['sd'], COL[obj],
                                    k=min(1.0, (st['om'] - 4.0) / 5.0))
            ST.draw_strap(arr, cam, ST.strap_curve(st['p'], st['th'], L_, st['om'], w['spin_max'], st['sd']), COL[obj])
            ST.draw_flail_head(arr, cam, st['p'] + L_ * np.array([math.cos(st['th']), math.sin(st['th'])]), st['th'],
                               COL[obj])
            fr.set_array(arr, depth)
        elif kind == 'piston':
            pk, pi, pa, poff, pext, pstr = obj
            arr, depth = fr.array(), fr.depth()
            ST.draw_piston(arr, cam, fs[pk]['p'], pa, poff, pext, pstr, COL[pk])
            fr.set_array(arr, depth)
        elif kind in ('smoke_back', 'smoke_front'):
            st = fs[obj]
            dur = CFG[obj]['abilities']['unlock1']['dur_beats'] * T.BEAT
            k_ = min(1.0, st['shield'] / 0.25, (dur - st['shield']) / 0.2 + 0.15)
            arr, depth = fr.array(), fr.depth()
            ST.draw_smoke_screen(arr, cam, st['p'][0], st['p'][1], t, k_, 1 if kind == 'smoke_front' else 0, COL[obj])
            fr.set_array(arr, depth)
        elif kind == 'key':
            row = obj
            arr, depth = fr.array(), fr.depth()
            own = int(row[3])
            fade_k = 1.0 if t < T_HS else max(0.0, 1 - (t - T_HS) / 0.4)
            if int(row[6]) in VOLLEY_IDS:
                ST.draw_smoke_ring(arr, cam, row[0], row[1], row[2], int(row[4]), t, COL[own], alpha=fade_k)
            elif fade_k > 0.3:
                ST.draw_note(arr, cam, row[0], row[1], row[2], COL[own])
            fr.set_array(arr, depth)
    # ---------------- front ropes + posts
    draw_rope_side(0)
    draw_rope_side(1)
    draw_post('front')
    arr, depth = fr.array(), fr.depth()
    # ---------------- wraps, the HIGH C sight + blast, banners, the honk bell, shards, sparks
    for k in FLAIL:                                                      # STRAP FLAIL: the strap coiled round the enemy
        wr = wrap_at(k, t)
        if wr is not None and fs[k]['vis'] and fs[1 - k]['vis']:
            pe, ps_ = fs[1 - k]['p'], fs[k]['p']
            ST.draw_wrap(arr, cam, ps_, pe, wr[0], COL[k], t, side=fs[k]['sd'])
            d_ = (pe - ps_) / max(1e-6, float(np.hypot(*(pe - ps_))))
            ST.draw_flail_head(arr, cam, pe + d_ * (R + 0.1), math.atan2(d_[1], d_[0]), COL[k])
    if in_fight:                                                         # FANFARE: golden banners on the ring
        for owner, cx, cz, r_, rmax, kind_, t0 in RUN['WAVES'][i]:
            if kind_ == 'shockwave':
                ST.draw_banners(arr, cam, np.array([cx, cz]), r_, min(1.0, (1 - r_ / rmax) * 1.6), t)
    for k, a_, ln, ck in sights:                                        # the locked aim while the bell charges
        ST.draw_sight(arr, cam, fs[k]['p'], a_, ln, ck, t, COL[k])
    for k, ox, oz, a_, ln, wd, ext, al, bst in blasts:                 # the blasts, out of the trumpet's bell
        pc = fire_centre(ox, oz, a_)
        ST.draw_beam(arr, cam, pc, a_, ln, wd, bst, COL[k], t)
        ST.draw_flash(arr, cam, pc, a_, max(0.0, 1 - bst[3] / 0.10), COL[k])
    for owner, cx, cz, t0, t_slam, rad in slams:                       # HONK: the giant bell over the predicted spot
        h = ST.honk_height(t, t0, t_slam)
        if h is not None:
            a = min(1.0, (t - t0) / 0.15) * (1.0 if t < t_slam + 0.35 else max(0.0, 1 - (t - t_slam - 0.35) / 0.25))
            ST.draw_honk_bell(arr, cam, cx, cz, rad, h, a, blast=max(0.0, 1 - abs(t - t_slam) / 0.12), col=COL[int(owner)])
    for kind, p, rot, size, curl, a in parts_at(t):
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind == 'glass':
            tri = [np.array([sx_, sz_]) + d * size, np.array([sx_, sz_]) - d * size * 0.6 + nn * size * 0.7,
                   np.array([sx_, sz_]) - d * size * 0.5 - nn * size * 0.6]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
            layer_blend(arr, [sp], COL[LOSE] * 0.35 + 170, 0.85)
            arr_pts = sp.astype(int)
            for (xq, yq) in arr_pts:
                if 0 <= xq < W and 0 <= yq < H:
                    arr[yq, xq] = COL[LOSE] * 0.6 + 100
        elif kind == 'tube':
            # a bent length of brass tubing: a short arc, 2 px, brass with a bright edge
            pts = [np.array([sx_, sz_]) + d * 0.22 * math.cos(j * 0.5 * curl + 1.0) + nn * 0.22 * math.sin(j * 0.5 * curl + 1.0)
                   for j in range(6)]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            dr = ImageDraw.Draw(im)
            dr.line([tuple(q) for q in sp], fill=tuple(int(v) for v in ST.BRASS_DK), width=2)
            dr.line([tuple(q) for q in sp], fill=tuple(int(v) for v in ST.BRASS_HI), width=1)
            arr = np.asarray(im).astype(np.float32)
        elif kind in ('valve', 'keycap'):
            # a valve cap / a key button: a small pearl-topped disc
            rr = 0.09 if kind == 'valve' else 0.07
            cen = S.to_world(sx_, sz_, p[1])
            ring = np.array([cen + [rr * math.cos(u), 0, rr * math.sin(u)] for u in np.linspace(0, 2 * math.pi, 9)[:-1]])
            layer_blend(arr, [screen(cam, ring)], ST.BRASS if kind == 'valve' else ST.BRASS_DK, 1.0)
            ring2 = np.array([cen + [rr * 0.6 * math.cos(u), 0.01, rr * 0.6 * math.sin(u)] for u in np.linspace(0, 2 * math.pi, 7)[:-1]])
            layer_blend(arr, [screen(cam, ring2)], ST.PEARL, 1.0)
        elif kind == 'reed':
            q4 = [np.array([sx_, sz_]) + d * 0.32 / 2 * a1 + nn * 0.07 / 2 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            layer_blend(arr, [sp], np.array([226, 198, 128]), 1.0)
        else:
            # brass: tumbling flat chips of the bell / body, banded as they turn
            sz = size * 1.1
            tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
                   np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
            shade_k = 0.7 + 0.45 * max(0.0, math.cos(rot * 2))
            layer_blend(arr, [sp], np.clip(ST.BRASS * shade_k, 0, 255), 1.0)
    for u, m, dd_ in solo_notes(t):                                   # the champion's solo: notes rising from the horn
        st = fs[WIN]
        if WIN in BEAM:
            org = ST.bell_mouth(st['p'], st['th'])
        else:
            org = st['p']
        side = 0.25 * math.sin(m * 1.7)
        ST.draw_note_rise(arr, cam, org[0] + side * u, org[1] - side * u, R + 0.4 + 1.3 * u, COL[WIN], 1 - u / 1.4)
    sparks(arr, cam, t)
    tc = fx_t(t)
    arr = PA.draw(arr, cam, tc, light=lambda P: 1.0 + 0.6 * FL.tint(P, tc))   # steam, smoke, brass sparks (reelkit.fx)
    # countdown in the ring (sharp, over everything in the ring)
    cs = cam.wp(np.array([[0.0, 0.3, 0.0]]))[0]
    fr.set_array(arr, depth)
    # ---------------- lens (focus locked on the ring), glows, the spot beam, bloom, grade
    img, depth = fr.array(), fr.depth()
    zf = cam.depth(S.SUBJECT)[0]
    img = dof(img, coc_map(depth, zf, 26.0, sharp=0.17))
    img = shimmers(img, cam, t, fs)                                # heat shimmer (reelkit.fx)
    for name, lp in (('red', S.RED_LIGHT), ('blue', S.BLUE_LIGHT)):
        s_ = cam.wp(lp[None])[0]
        fr.glow(s_[0], s_[1], 16, tuple(COL[0] if name == 'red' else COL[1]), 0.35 + 0.15 * T.beat_pulse(t, 0.2))
    for k, (s_, rp) in drawn.items():
        st = fs[k]
        if st['glow'] > 0.05:
            fr.glow(s_[0], s_[1], rp * 2.2, tuple(COL[k] * 0.8 + 50), 0.45 * st['glow'])
    for k, sm in drawn_bell.items():                                   # the bell mouth glows as it charges / fires / solos
        ck = fs[k]['charge']
        if ck > 0.05:
            fr.glow(sm[0], sm[1], 6 + 8 * ck, tuple(COL[k] * 0.6 + 100), 0.35 + 0.55 * ck)
    for k, ox, oz, a_, ln, wd, ext, al, bst in blasts:                 # glow along the blast
        for u in (0.15, 0.5, 0.85):
            q = np.array([ox, oz]) + u * ln * ext * np.array([math.cos(a_), math.sin(a_)])
            s_ = cam.wp(S.to_world(q[0], q[1], R)[None])[0]
            fr.glow(s_[0], s_[1], 10 + 8 * wd, tuple(COL[k] * 0.7 + 70), (0.35 if wd < 0.6 else 0.22) * al)
    for e_i, e in events_between(t - 0.15, t, ('hit',)):
        if e[7] in ('beam', 'slam', 'wave', 'shockwave'):
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            big = e[7] != 'beam'
            fr.glow(s_[0], s_[1], 14 if big else 8, tuple(COL[e[2]] * 0.7 + 70), (0.35 if big else 0.22) * (1 - (t - e[0]) / 0.15))
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
    # the can + truss at the top of frame
    if -30 < s0[1] < 60:
        img = apply_glows(img, [(s0[0], s0[1], 30, (255, 244, 220), 0.9), (s0[0], s0[1], 10, (255, 255, 255), 1.0)])
    img = grade(bloom(np.clip(img, 0, 255), thr=225, k=0.22), contrast=0.18, sat=1.18)
    img = np.clip(img * vignette(strength=0.45) * top_scrim(0.45, 100, 90), 0, 255)
    img = np.clip(post_fx(img, cam, t, fs, drawn), 0, 255).astype(np.uint8)   # fringe, ability dim + lines, bursts
    img = np.ascontiguousarray(img)
    # ---------------- UI: countdown, HP numbers in the balls, fight HUD, captions
    countdown(img, t, cs)
    for k, (s_, rp) in drawn.items():
        st = fs[k]
        if st['scale'] < 0.95 or T.BAR - 0.05 <= t < T.BAR + 0.85:          # hidden while FIGHT! covers the balls
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
    fight_hud(img, hud[0], hud[1])
    draw_captions(img, t, CAP_TITLE, style='impact', hi=tuple(int(v) for v in COL[0]), alt=tuple(int(v) for v in COL[1]))
    tag_k = ss(0.40, 0.55, t) * (1 - ss(T.T_FIGHT - 0.2, T.T_FIGHT - 0.06, t))
    if tag_k > 0.02:                                                   # the series tag, small, under the title
        w_ = len(T.SERIES) * ADV - 1
        x_, y_ = int(round(180 - w_ / 2)), 157
        reg = img[y_ - 2:y_ + 9, x_ - 3:x_ + w_ + 3].astype(np.float32)
        img[y_ - 2:y_ + 9, x_ - 3:x_ + w_ + 3] = (reg * (1 - 0.6 * tag_k) + np.array(INK) * 0.6 * tag_k).astype(np.uint8)
        if tag_k > 0.5:
            label(img, T.SERIES, x_, y_, AMBER, 1)
    for k in (0, 1):
        c = tuple(int(v) for v in np.clip(COL[k] * 0.8 + 60, 0, 255))
        draw_captions(img, t, CAP_ABIL[k], style='impact', base=c, puff=tuple(int(v) for v in COL[k]))
    draw_captions(img, t, CAP_KO, style='impact', hi=(255, 214, 96))
    wc = tuple(int(v) for v in np.clip(COL[WIN] * 0.8 + 60, 0, 255))
    draw_captions(img, t, CAP_WINS, style='impact', base=wc)
    draw_captions(img, t, CAP_COMMENT, style='impact', hi=(255, 214, 96))
    return img


if __name__ == '__main__':
    build_sprites()
