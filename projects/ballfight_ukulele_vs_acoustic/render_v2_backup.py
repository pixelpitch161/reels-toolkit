"""#33 BALLFIGHT #7: Ukulele vs Acoustic Guitar - renderer, v2 (BALLFIGHT v2 redesign, user Oct 24): SURF'S UP vs
SIX-STRING RAKE on reelkit.fx (#28's renderer, copied; the weapon styles live in styles.py; v1 = render_v1_backup.py).

arena floor (per pixel: spill, corner lights, attack lights) -> stands + crowd (phones, flashbulbs, lit by attacks) ->
ring skirt -> canvas (per pixel: the hard spot with real shadows of balls, waves, posts and ropes; the corner lights in
the fighters' colours; attack lights (reelkit.fx flares); fading marks (claw marks from every rake stroke, water from
waves and the spout); decals: the rake's light, CHUNK's splash ring, BIG STRUM's sweep) -> back ropes + posts -> waves,
fighters (the capo), rake strings, whips, the waterspout (painter) -> BIG STRUM's six strings -> front ropes + posts ->
effects (sparks, shards, foam + wood-chip particles) -> countdown -> locked DOF on the ring -> heat shimmer -> glows ->
spot beam -> bloom -> grade -> scrim -> fringing, ability moments, impact bursts -> HP numbers, fight HUD, captions.

    python render.py                                   # sprite cache (check/sprites_ballfight7.npz)
    python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 4.1 9.3 13.5 20.0 25.1 41.7 42.6 52.3 55 59.6
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
BLADE = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'blade']        # none in this pair (kept for focus_check)
SURFER = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'surf']
RAKER = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'rake']
CONER = RAKER                                                               # the capo is drawn on the raker's ball
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


# ---------------------------------------------------------------- v2 attacks, rebuilt from the run's events (closed-form
# motion, the same formulas as the engine, so frames render in any order)
RULES = sim.rules()
DT = RULES.DT


def _surfs():
    """SURF'S UP waves: dict(t0, owner, main, h, o, u, power, hw, t_end, crashed); the end replays the engine's rule on
    its substep grid (the front reaches the ropes, or the wave's life runs out)"""
    out = []
    A = RULES.A
    for e in EV:
        if e[1] != 'surf':
            continue
        k = e[2]
        o = np.array([e[5], e[6]])
        u = np.array([math.cos(e[7]), math.sin(e[7])])
        power, hw = e[8], e[9]
        life = CFG[k]['weapon']['life']
        m = 0
        while True:
            tau = m * DT
            c = o + u * fight_surf_dist(tau, power)
            if abs(c[0]) >= A - 0.05 or abs(c[1]) >= A - 0.05 or tau >= life:
                break
            m += 1
        out.append(dict(t0=e[0], owner=k, main=bool(e[3]), h=e[4], o=o, u=u, power=power, hw=hw,
                        t_end=min(e[0] + m * DT, T_KO), crashed=tau < life and e[0] + m * DT <= T_KO))
    return out


def fight_surf_dist(tau, power):
    from fight import Fight
    return Fight.surf_dist(tau, RULES.V, power, RULES.KNOCK_T)


SURFS = _surfs()
SURF_HIT = {}                                          # t0 of the wave -> time it crashed into the enemy
for e in EV:
    if e[1] == 'hit' and e[7] == 'surf':
        past = [w for w in SURFS if w['owner'] == e[2] and w['t0'] <= e[0] <= w['t_end'] + 1e-6]
        if past:
            SURF_HIT[past[-1]['t0']] = e[0]
RAKES = [dict(t0=e[0], owner=e[2], main=bool(e[3]), L=e[4], c=np.array([e[5], e[6]]), aim=e[7],
              u=np.array([math.cos(e[7]), math.sin(e[7])]), n=np.array([-math.sin(e[7]), math.cos(e[7])]), dir=e[8],
              S=e[9], dur=e[10]) for e in EV if e[1] == 'rake']
RAKE_T = np.array([r_['t0'] for r_ in RAKES]) if RAKES else np.zeros(0)
RAKE_HIT = set()
for e in EV:
    if e[1] == 'hit' and e[7] == 'rake':
        j = int(np.searchsorted(RAKE_T, e[0], 'right')) - 1
        if j >= 0:
            RAKE_HIT.add(j)
GAP = [CFG[k]['weapon'].get('gap', 0.085) for k in (0, 1)]
WHIPS = [dict(t0=e[0], owner=e[2], j=e[3], ang=e[4], len=e[5]) for e in EV if e[1] == 'whip']
T_EXT = [CFG[k]['abilities']['unlock2'].get('t_ext', 0.07) for k in (0, 1)]
SWEEPS = [dict(t0=e[0], owner=e[2], c=np.array([e[3], e[4]]), u=np.array([math.cos(e[5]), math.sin(e[5])]), aim=e[5],
               reach=e[6], dur=e[7]) for e in EV if e[1] == 'sweep']
SPOUT_PATH = RUN.get('X33_SPOUT', [])
SPOUT_T = np.array([q[0] for q in SPOUT_PATH]) if SPOUT_PATH else np.zeros(0)
SPOUTS = []
for e in EV:
    if e[1] == 'spout':
        ends = [x[0] for x in EV if x[1] == 'spout_end' and x[2] == e[2] and x[0] >= e[0]]
        SPOUTS.append(dict(t0=e[0], owner=e[2], p0=np.array([e[3], e[4]]), t_end=min(ends[0] if ends else T_KO, T_KO)))


def surf_height(w, t):
    """the wave's height at t: rises over 0.12 s, rolls, collapses over 0.25 s after it ends"""
    Hh = 0.32 + 0.45 * w['h']
    if t < w['t0']:
        return 0.0
    rise = ss(w['t0'], w['t0'] + 0.12, t)
    fall = 1 - ss(w['t_end'], w['t_end'] + 0.25, t)
    return Hh * rise * fall


def surf_front(w, t):
    tau = min(t, w['t_end']) - w['t0']
    c = w['o'] + w['u'] * fight_surf_dist(max(0.0, tau), w['power'])
    if t > w['t_end']:                                  # collapsing: the foam slides on a little
        c = c + w['u'] * 0.6 * ss(w['t_end'], w['t_end'] + 0.25, t)
    return c


def waves_at(t):
    return [w for w in SURFS if w['t0'] <= t < w['t_end'] + 0.25]


def rakes_at(t):
    """(rake, s_now, s_prev, k) for strokes still showing (the stroke + a 0.14 s fade)"""
    if not len(RAKE_T):
        return []
    from fight import Fight
    out = []
    j0, j1 = np.searchsorted(RAKE_T, t - 0.4), np.searchsorted(RAKE_T, t, 'right')
    for j in range(j0, j1):
        r_ = RAKES[j]
        tau = t - r_['t0']
        if tau > r_['dur'] + 0.2:
            continue
        sn = Fight.rake_lateral(tau, r_['dur'], r_['S'], r_['dir'])
        sp = Fight.rake_lateral(max(0.0, tau - 1.0 / FPS), r_['dur'], r_['S'], r_['dir'])
        k = 1.0 if tau <= r_['dur'] else 1 - (tau - r_['dur']) / 0.2
        out.append((j, r_, sn, sp, k))
    return out


def spout_at(t):
    """(owner, position, strength) of a live waterspout"""
    for sp in SPOUTS:
        if sp['t0'] <= t < sp['t_end'] + 0.4 and len(SPOUT_T):
            tt = min(t, sp['t_end'])
            i = int(np.searchsorted(SPOUT_T, tt))
            if i <= 0:
                p = sp['p0']
            elif i >= len(SPOUT_T):
                p = np.array(SPOUT_PATH[-1][2:4])
            else:
                a, b = SPOUT_PATH[i - 1], SPOUT_PATH[i]
                u = (tt - a[0]) / max(1e-9, b[0] - a[0])
                p = np.array(a[2:4]) * (1 - u) + np.array(b[2:4]) * u
            k = ss(sp['t0'], sp['t0'] + 0.25, t) * (1 - ss(sp['t_end'], sp['t_end'] + 0.4, t))
            return sp['owner'], p, k
    return None


SOLO_STRUMS = [T_SOLO + bt * T.BEAT for bt, m, d in T.SOLO[NAMES[WIN]] if isinstance(m, tuple)]


def solo_rings(t):
    """the champion's solo: every strum in it rings out of the ball as a small cone-coloured ripple"""
    out = []
    for t0 in SOLO_STRUMS:
        if t0 <= t < t0 + 0.45:
            out.append((t0, (t - t0) / 0.45))
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
FRETS0 = [CFG[k]['weapon'].get('frets0', 0) for k in (0, 1)]


def fighter(k, t):
    """picture state of fighter k at time t: dict(p (ring frame), vis, th, blen, frets, shield, haste, glow, scale)"""
    st = dict(vis=True, shield=0.0, haste=0.0, glow=0.0, scale=1.0, th=0.0, blen=BASE_LEN[k], frets=FRETS0[k],
              level=1, shattered=False, om=OMEGA[k], sd=SD0[k], lift=0.0)
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
        tc = min(t, T_KO)
        if k in RAKER and len(RAKE_T):                            # the acoustic flashes on every stroke
            j = np.searchsorted(RAKE_T, tc, 'right') - 1
            if j >= 0:
                st['glow'] = 0.35 * math.exp(-(tc - RAKE_T[j]) / 0.09)
        if k in SURFER:                                           # the uke rides its wave: lifted on the crest, glowing
            for w in SURFS:
                if w['owner'] == k and w['t0'] <= tc < w['t_end'] + 0.25:
                    hgt = surf_height(w, tc)
                    ride = 1 - ss(w['t0'] + 0.55, w['t0'] + 0.9, tc)
                    st['lift'] = max(st['lift'], 0.45 * hgt * ride)
                    st['glow'] = max(st['glow'], 0.45 * math.exp(-(tc - w['t0']) / 0.14))
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
SHK = fx.Shake(cap=2.5)                              # a little camera shake on heavy hits (reelkit.fx)
for e in EV:
    if e[1] == 'hit' and e[7] in ('surf', 'rake', 'sweep', 'spout', 'shockwave'):
        SHK.add(e[0], {'surf': 1.2, 'rake': 0.5, 'sweep': 2.5, 'spout': 1.0, 'shockwave': 1.4}[e[7]])
SHK.add(T_KO, 2.5)


def camera(t):
    cam = interp_keys(KEYS, t)
    dx, dy = SHK.offset(t)
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
    FL.apply(L, t)                                          # attacks light the scene in the fighter's colour (fx)
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
    kinds = ['glass'] * n_glass + (['string'] * 6 + ['top'] * 5 + ['side'] * 3 + ['fret'] * 3 if loser == 'ACOUSTIC' else
                                   ['string'] * 4 + ['top'] * 6 + ['side'] * 2 + ['fret'] * 3)
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
STRING_COL = (212, 176, 118) if NAMES[LOSE] == 'ACOUSTIC' else (240, 236, 226)      # bronze steel vs nylon
WOOD_COL = ({'top': np.array([226, 190, 122]), 'side': np.array([112, 52, 30])} if NAMES[LOSE] == 'ACOUSTIC' else
            {'top': np.array([196, 112, 62]), 'side': np.array([120, 60, 36])})


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
SPR_K = {'UKULELE': 0.70, 'ACOUSTIC': 0.86}                 # little brother, big brother: the uke sits smaller in its ball
_SPR = {}
SPR_PATH = os.path.join(HERE, 'check', 'sprites_ballfight7.npz')


def _sprite_cache():
    if _SPR:
        return _SPR
    if os.path.exists(SPR_PATH):
        z = np.load(SPR_PATH)
        _SPR.update({k: z[k] for k in z.files})
    return _SPR


def sprite(name, radius_px, yaw_deg, roll):
    cache = _sprite_cache()
    r = int(np.clip(round(radius_px), 12, 30))
    yk = int(round(yaw_deg / 6.0)) * 6
    key = f'{name}_{r}_{yk % 360}_{int(roll)}'
    if key not in cache:
        import sprites as SP
        cache[key] = SP.render_sprite(name, r, float(yk), float(roll))
    return cache[key]


def build_sprites():
    """precompute every sprite the video can use (both instruments, radii 12..30, yaw window) -> check/sprites_ballfight7.npz"""
    import sprites as SP
    out = {}
    for k in (0, 1):
        nm = CFG[k]['pp']
        yc, sw, roll = CFG[k]['view']
        for r in range(12, 31):
            for yk in range(int((yc - sw - 6) // 6) * 6, int(yc + sw + 12), 6):
                out[f'{nm}_{r}_{yk % 360}_{int(roll)}'] = SP.render_sprite(nm, r, float(yk), float(roll))
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


# ---------------------------------------------------------------- weapons: see styles.py (cone, dash, capo, darts, pick, tornado)


# ---------------------------------------------------------------- glass balls
def draw_ball(arr, dep, cam, k, st, t):
    pw = S.to_world(st['p'][0], st['p'][1], R + st.get('lift', 0.0))
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
    yaw = yc + sw * math.sin(1.25 * st['p'][0] + 0.45 * st['p'][1])
    if st.get('whirl', 0) > 0.05:                                       # UKE-NADO: the uke twists fast inside its ball
        yaw = yc + sw * math.sin(t * 26.0)
    spr = sprite(name, rp * SPR_K[NAMES[k]], yaw, roll)
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
        e_ = 1 - (1 - min(1.0, u * 1.08)) ** 2.5          # eases out and lands on the last frame at exactly frame 0's
        big_text(img, '3', cx, cy, 78 * (0.15 + 1.1 * e_), (255, 244, 220), alpha=min(1.0, u * 1.6))   # size (78 x 1.25)


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
FLASH_EV = [e[0] for e in EV if (e[1] == 'hit' and e[7] in ('dash', 'slam', 'wave', 'shockwave')) or e[1] == 'ability'] + \
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
    dh = cam.depth(heads)
    sp = cam.wp(heads)
    crowd_lit = 1.0 + 0.9 * FL.tint(heads, t)               # attack lights reach the crowd (fx)
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


# ---------------------------------------------------------------- effects (reelkit.fx, BALLFIGHT v2): built once from the run
def hz_clock(t):
    """marks' clock: real time in the fight, held through the hit-stop, then running on"""
    if t < T_KO:
        return t
    if t < T_HS:
        return T_KO
    return t - HITSTOP


def sweep_front_at(sw, t):
    from fight import Fight
    return Fight.sweep_front(t - sw['t0'], sw['dur'], sw['reach'], R)


FL = fx.Flares(max_n=6)
BU = fx.Bursts()
PA = fx.Particles(seed=33)
MK = fx.Marks(max_live=40)
MO = fx.Moments()
CLAW_COL = (70, 44, 30)


def _fx_build():
    # SURF'S UP: the crest lights the ring as it rolls, foam flies off the lip, the crash leaves water on the canvas
    for w in SURFS:
        k = w['owner']
        dur = w['t_end'] - w['t0']
        Hh = 0.32 + 0.45 * w['h']
        tau = 0.0
        while tau <= dur:
            c = w['o'] + w['u'] * fight_surf_dist(tau, w['power'])
            n = np.array([-w['u'][1], w['u'][0]])
            for side in (-0.8, 0.0, 0.8):
                q = c + n * side * w['hw']
                PA.emit(w['t0'] + tau, S.to_world(q[0], q[1], Hh * 0.9), 'foam', n=1, dir=(w['u'][0], 0.6, w['u'][1]),
                        vel=S.to_world(*(w['u'] * 1.2), 0.0), speed=(0.4, 1.0))
            if tau in (0.0,) or abs(tau - 0.3) < 0.05:
                FL.add(w['t0'] + tau, S.to_world(c[0], c[1], 0.8), COL[k], radius=1.6, peak=0.45, decay=0.22)
            tau += 0.08
        cend = w['o'] + w['u'] * fight_surf_dist(dur, w['power'])
        if w['crashed']:
            PA.emit(w['t_end'], S.to_world(cend[0], cend[1], Hh * 0.7), 'foam', n=14, dir=(w['u'][0], 1.0, w['u'][1]),
                    speed=(0.8, 2.2), spread=80)
        if w['t_end'] < T_KO:
            MK.add(w['t_end'], 'water', cend[0], cend[1], size=0.3 + 0.3 * w['hw'], life=2.5, seed=int(w['t0'] * 97),
                   strength=0.6)
    # SIX-STRING RAKE: every stroke lights the strings and leaves six-line claw marks where it ends
    for j, r_ in enumerate(RAKES):
        k = r_['owner']
        from fight import Fight
        s_end = Fight.rake_lateral(r_['dur'], r_['dur'], r_['S'], r_['dir'])
        mid = r_['c'] + r_['u'] * (R + r_['L'] * 0.5) + r_['n'] * s_end
        FL.add(r_['t0'] + r_['dur'] * 0.5, S.to_world(mid[0], mid[1], 0.7), COL[k] * 0.9, radius=1.3,
               peak=0.35 if r_['main'] else 0.22, decay=0.14)
        if r_['t0'] + r_['dur'] < T_KO:
            MK.add(r_['t0'] + r_['dur'], 'claw', mid[0], mid[1], size=r_['L'] * 0.5, angle=r_['aim'], n=6,
                   col=CLAW_COL, life=2.6, seed=j, strength=0.95 if j in RAKE_HIT else 0.5)
    # hits
    for e in EV:
        if e[1] != 'hit':
            continue
        att, x, z, how = e[2], e[5], e[6], e[7]
        pos = S.to_world(x, z, R)
        if how == 'surf':
            BU.add(e[0], pos, COL[att], 1.0 + 0.5 * min(1.0, e[8] if len(e) > 8 else 1))
            FL.add(e[0], pos, COL[att], radius=1.8, peak=0.85, decay=0.28)
            PA.emit(e[0], pos, 'foam', n=14, speed=(1.0, 2.4), spread=110)
            MK.add(e[0], 'water', x, z, size=0.45, life=2.5, seed=int(e[0] * 100), strength=0.7)
        elif how == 'rake':
            BU.add(e[0], pos, COL[att], 0.8)
            FL.add(e[0], pos, COL[att] * 0.95, radius=1.5, peak=0.65, decay=0.22)
            PA.emit(e[0], pos, 'chips', n=8)
        elif how == 'whip':
            BU.add(e[0], pos, COL[att], 0.4, bright=False)
            PA.emit(e[0], pos, 'chips', n=3, speed=(1.0, 2.0))
        elif how == 'sweep':
            BU.add(e[0], pos, COL[att], 2.0)
            FL.add(e[0], pos, COL[att], radius=2.4, peak=1.1, decay=0.5)
            PA.emit(e[0], pos, 'chips', n=24, speed=(1.5, 3.2), scale=1.2)
            MK.add(e[0], 'claw', x, z, size=1.2, angle=math.atan2(*(SWEEPS[0]['u'][::-1])) + math.pi / 2 if SWEEPS else 0.0,
                   n=6, col=CLAW_COL, life=6.0, seed=77)
        elif how == 'spout':
            BU.add(e[0], pos, COL[att], 1.1)
            FL.add(e[0], pos, COL[att], radius=1.8, peak=0.8, decay=0.3)
            PA.emit(e[0], pos, 'foam', n=12, speed=(1.0, 2.6), spread=150)
        elif how == 'shockwave':
            BU.add(e[0], pos, COL[att], 1.2)
            FL.add(e[0], pos, COL[att], radius=1.8, peak=0.8, decay=0.3)
    # BIG STRUM: the sweep lights the ring as it crosses
    for sw in SWEEPS:
        for u_ in (0.2, 0.5, 0.8):
            f = sweep_front_at(sw, sw['t0'] + sw['dur'] * u_)
            c = sw['c'] + sw['u'] * f
            FL.add(sw['t0'] + sw['dur'] * u_, S.to_world(c[0], c[1], 0.9), COL[sw['owner']], radius=2.6, peak=0.6, decay=0.25)
    # UKE-NADO: foam spins off the base, water trails on the canvas
    for sp in SPOUTS:
        tt_ = sp['t0']
        n = 0
        while tt_ < sp['t_end']:
            pr = spout_at(tt_)
            if pr is not None:
                p_ = pr[1]
                PA.emit(tt_, S.to_world(p_[0], p_[1], 0.1), 'foam', n=2, spread=180, speed=(0.6, 1.6))
                if n % 4 == 0:
                    MK.add(tt_, 'water', p_[0], p_[1], size=0.4, life=2.5, seed=n + 500, strength=0.6)
                if n % 5 == 0:
                    FL.add(tt_, S.to_world(p_[0], p_[1], 1.2), COL[sp['owner']], radius=1.6, peak=0.4, decay=0.3)
            tt_ += 0.06
            n += 1
    # CHUNK: a splash ring of foam
    for e in EV:
        if e[1] == 'ability' and e[3] == 'CHUNK':
            i_ = int(min(NSIM - 1, max(0, round((e[0] - T.T_FIGHT) * FPS))))
            p_ = RUN['P'][i_, e[2]]
            for a in np.linspace(0, 2 * math.pi, 12, endpoint=False):
                q = p_ + 0.7 * np.array([math.cos(a), math.sin(a)])
                PA.emit(e[0], S.to_world(q[0], q[1], 0.1), 'foam', n=1, dir=(math.cos(a), 0.8, math.sin(a)), speed=(1.0, 2.0))
            MK.add(e[0], 'water', p_[0], p_[1], size=0.9, life=3.0, seed=int(e[0] * 10))
    # ability moments on the captioned first uses
    for t, who, name in STORY['caps']:
        MO.add(t, who, COL[who])
    # the K.O.
    pos = S.to_world(P_KO[LOSE][0], P_KO[LOSE][1], R)
    BU.add(T_KO, pos, COL[LOSE], 2.0)
    FL.add(T_KO, pos, COL[LOSE] * 0.9, radius=2.4, peak=1.1, decay=0.6)
    PA.emit(T_KO, pos, 'chips' if NAMES[LOSE] == 'ACOUSTIC' else 'foam', n=30, spread=180, speed=(0.8, 2.4), scale=1.2)


_fx_build()


def shimmers(img, cam, t, ta, sweeps_now, spout_now):
    """heat-shimmer ripples: CHUNK's splash front, BIG STRUM's sweep front, round the waterspout"""
    if T.T_FIGHT <= t < T_HS:
        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][sim_i(t)]:
            img = fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.35, 2.4, t, k=1 - r / rmax, seed=owner)
    for sw in sweeps_now:
        tau = ta - sw['t0']
        if tau <= sw['dur']:
            f = sweep_front_at(sw, ta)
            c = sw['c'] + sw['u'] * f
            img = fx.shimmer_world(img, cam, S.to_world(c[0], c[1], 0.05), 1.6, 0.5, 2.6, t, k=0.8)
    if spout_now is not None:
        owner, p_, k_ = spout_now
        img = fx.shimmer_world(img, cam, S.to_world(p_[0], p_[1], 0.05), 0.9, 0.3, 1.8, t, k=k_)
    return img


def post_fx(img, cam, t, fs, drawn):
    """after the grade: chromatic fringing on big hits, the ability moments (dim + speed lines), the impact bursts"""
    for _, e in events_between(t - 0.12, t, ('hit',)):
        if e[7] in ('surf', 'sweep', 'spout'):
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            big = e[7] == 'sweep'
            img = fx.fringe(img, s_[0], s_[1], 30 if big else 20, 2.8 if big else 1.8, k=1 - (t - e[0]) / 0.12)
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


# ---------------------------------------------------------------- shadows on the canvas (from the spot)
CANVAS_GRID = MaskGrid((-5.4, 5.4, -5.4, 5.4, 0.03))
STATIC_CASTERS = []
for name in S.CORNERS:
    w = S.to_world(*S.CORNERS[name])
    STATIC_CASTERS.append(np.array([w + [-0.09, 0, 0], w + [0.09, 0, 0], w + [0.09, S.POST_H, 0], w + [-0.09, S.POST_H, 0]]))


def canvas_shadows(cam, t, fs, keys, spot):
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
        yb = R + st.get('lift', 0.0)
        pw = S.to_world(st['p'][0], st['p'][1], yb)
        circ = np.array([[pw[0] + R * st['scale'] * math.cos(u), yb, pw[2] + R * st['scale'] * math.sin(u)] for u in a])
        polys.append(proj_point(circ, spot))
    for w in waves_at(t):                                           # the waves' crests cast the spot's shadow too
        Hh = surf_height(w, t)
        if Hh > 0.05:
            for Q in ST.wave_shadow_quads(surf_front(w, t), w['u'], w['hw'], Hh):
                polys.append(proj_point(Q, spot))
    for row in keys:
        head, body = ST.dart_quads(row[0], row[1], row[2])
        polys.append(proj_point(head, spot))
    return CANVAS_GRID.raster(polys, blur=0.7)


# ---------------------------------------------------------------- sparks
def sparks(arr, cam, t):
    for i, e in events_between(t - 0.32, t, ('hit', 'parry', 'block', 'wall', 'cleared')):
        dt = t - e[0]
        kind = e[1]
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            col = COL[att] * 0.6 + 100
            n, sp, life = (16, 3.2, 0.3) if how in ('dash', 'slam', 'wave', 'shockwave') else \
                ((8, 2.4, 0.22) if how == 'cone' else (5, 1.8, 0.18))
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
        if kind == 'hit' and e[7] in ('dash', 'slam') and dt < 0.08:
            s = cam.wp(S.to_world(x, z, R)[None])[0]
            arr[max(0, int(s[1]) - 1):int(s[1]) + 2, max(0, int(s[0]) - 1):int(s[0]) + 2] = 255


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
    ta = min(t, T_KO)                                               # attacks freeze with the hit-stop, then go
    show_att = T.T_FIGHT <= t < T_HS + 0.12
    rakes_now = rakes_at(ta) if show_att else []
    waves_now = [w for w in waves_at(ta)] if show_att else []
    whips_now = [w_ for w_ in WHIPS if 0 <= ta - w_['t0'] <= 2 * T_EXT[w_['owner']]] if show_att else []
    sweeps_now = [sw for sw in SWEEPS if 0 <= ta - sw['t0'] <= sw['dur'] + 0.2] if show_att else []
    spout_now = spout_at(ta) if show_att else None
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
        alb = MK.apply(alb, sxp, szp, hz_clock(t), BAYER_FULL[ys, xs])      # fading marks: claws, water (reelkit.fx)
        sh = CANVAS_GRID.sample(canvas_shadows(cam, t, fs, keys, spot), Xp, Zp)
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
        for j, r_, sn, sp_, k_ in rakes_now:                         # light under each stroke's six strings
            ra = (sxp - r_['c'][0]) * r_['u'][0] + (szp - r_['c'][1]) * r_['u'][1]
            rb = (sxp - r_['c'][0]) * r_['n'][0] + (szp - r_['c'][1]) * r_['n'][1]
            inl = np.clip(1 - np.maximum(R * 0.9 - ra, ra - (R + r_['L'])) / 0.15, 0, 1)
            band = np.clip(1 - np.abs(rb - sn) / 0.34, 0, 1) * inl
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (band * k_ * (0.45 if r_['main'] else 0.3))[:, None] * (COL[r_['owner']] / 255.0 * 0.9 + 0.25)[None]
        for sw in sweeps_now:                                        # BIG STRUM: a bright band under the sweeping strings
            fpos = sweep_front_at(sw, ta)
            ra = (sxp - sw['c'][0]) * sw['u'][0] + (szp - sw['c'][1]) * sw['u'][1] - fpos
            band = np.clip(1 - np.abs(ra + 0.35) / 0.55, 0, 1) * (1 - ss(sw['dur'], sw['dur'] + 0.2, ta - sw['t0']))
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (band * 0.7)[:, None] * (COL[sw['owner']] / 255.0 + 0.3)[None]
        wav = RUN['WAVES'][i] if in_fight else []
        for owner, cx, cz, r, rmax, kind, t0 in wav:
            if kind == 'pulse':
                continue
            rd = np.hypot(sxp - cx, szp - cz)
            wdt = 0.16 if kind == 'shockwave' else 0.32
            k_ = (1 - r / rmax) * (1.0 if kind == 'shockwave' else 1.3)
            if kind == 'wave':                                        # UKE-NADO: a three-armed whirl, spinning
                ang_ = np.arctan2(szp - cz, sxp - cx)
                rd = rd + 0.34 * (((3 * ang_ / (2 * math.pi)) + 2.5 * t) % 1.0 - 0.5)
            band = np.clip(1 - np.abs(rd - r) / wdt, 0, 1)
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            tint = COL[owner] / 255.0 * 0.6 + np.array([0.35, 0.65, 0.85])          # CHUNK: a watery splash ring
            Ic += (band * k_ * 0.9)[:, None] * (tint + (0.35 if kind == 'wave' else 0.15))[None]
        sl = RUN['SLAMS'][i] if in_fight else []
        for owner, cx, cz, t0, t_slam, rad in sl:
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
    rings = solo_rings(t) if t >= T_HS else []
    if rings:
        arr, depth = fr.array(), fr.depth()
        pc = fs[WIN]['p']
        for t0, u in rings:                                           # the champion's strums ripple out of its ball
            a_ = np.linspace(0, 2 * math.pi, 33)
            rr = R * (1.1 + 1.6 * u)
            sp_ = cam.wp(S.to_world(pc[0] + rr * np.cos(a_), pc[1] + rr * np.sin(a_), 0.35))
            ST.blend(arr, ST.mask_of([sp_], width=2), np.clip(COL[WIN] * 0.8 + 60, 0, 255), 0.8 * (1 - u))
        fr.set_array(arr, depth)
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
    for w in waves_now:                                                 # SURF'S UP waves (by the front's centre)
        c_ = surf_front(w, ta)
        items.append((cam.depth(S.to_world(c_[0] - w['u'][0] * 0.3, c_[1] - w['u'][1] * 0.3, 0.3))[0], 'wave', w))
    for j, r_, sn, sp_, k_ in rakes_now:
        mid = r_['c'] + r_['u'] * (R + r_['L'] * 0.5) + r_['n'] * sn
        items.append((cam.depth(S.to_world(mid[0], mid[1], 0.5))[0], 'rake', (r_, sn, sp_, k_)))
    for w_ in whips_now:
        p_ = fs[w_['owner']]['p']
        items.append((cam.depth(S.to_world(p_[0], p_[1], 0.5))[0] - 0.02, 'whip', w_))
    if spout_now is not None:
        items.append((cam.depth(S.to_world(spout_now[1][0], spout_now[1][1], 0.6))[0], 'spout', spout_now))
    for row in keys:
        items.append((cam.depth(S.to_world(row[0], row[1], 0.45))[0], 'key', row))
    items.sort(key=lambda it: -it[0])
    drawn = {}
    for dz, kind, obj in items:
        if kind == 'post':
            draw_post(obj)
        elif kind == 'ball':
            st = fs[obj]
            arr, depth = fr.array(), fr.depth()
            drawn[obj] = draw_ball(arr, depth, cam, obj, st, t)
            if obj in CONER and st['haste'] > 0:                     # CAPO: clamped across the ball while it lasts
                dur = CFG[obj]['abilities']['unlock1']['dur_beats'] * T.BEAT
                k_ = min(1.0, st['haste'] / 0.15, (dur - st['haste']) / 0.12)
                ST.draw_capo(arr, drawn[obj][0], drawn[obj][1], max(0.0, k_), COL[obj], t)
            fr.set_array(arr, depth)
        elif kind == 'wave':
            w = obj
            Hh = surf_height(w, ta if t < T_HS else T_KO)
            if Hh > 0.03:
                ST.draw_wave(fr, cam, L, shade, surf_front(w, ta), w['u'], w['hw'] + 0.3 * R, Hh, ta, COL[w['owner']],
                             lip=ss(w['t_end'], w['t_end'] + 0.25, ta))
        elif kind == 'rake':
            r_, sn, sp_, k_ = obj
            gp = GAP[r_['owner']]
            segs = ST.rake_segments(r_['c'], r_['u'], r_['n'], sn, r_['L'], gp)
            prev = ST.rake_segments(r_['c'], r_['u'], r_['n'], sp_, r_['L'], gp) if abs(sn - sp_) > 0.01 else None
            arr, depth = fr.array(), fr.depth()
            arr = ST.draw_strings(arr, cam, segs, COL[r_['owner']], k_ * (1.0 if r_['main'] else 0.75), 0.5, prev)
            fr.set_array(arr, depth)
        elif kind == 'whip':
            w_ = obj
            arr, depth = fr.array(), fr.depth()
            arr = ST.draw_whip(arr, cam, fs[w_['owner']]['p'], w_['ang'], w_['len'], ta - w_['t0'], T_EXT[w_['owner']],
                               COL[w_['owner']], j=w_['j'])
            fr.set_array(arr, depth)
        elif kind == 'spout':
            owner, p_, k_ = obj
            arr, depth = fr.array(), fr.depth()
            arr = ST.draw_spout(arr, cam, p_, t, k_, COL[owner])
            fr.set_array(arr, depth)
        elif kind == 'key':
            row = obj
            ST.draw_dart(fr, cam, L, shade, row[0], row[1], row[2], COL[int(row[3])])
    if sweeps_now:                                                      # BIG STRUM: six strings across the whole ring
        arr, depth = fr.array(), fr.depth()
        for sw in sweeps_now:
            tau = ta - sw['t0']
            kk = 1 - ss(sw['dur'], sw['dur'] + 0.2, tau)
            f1 = sweep_front_at(sw, ta)
            f0 = sweep_front_at(sw, ta - 1.0 / FPS)
            segs = ST.sweep_segments(sw['c'], sw['u'], f1, S.AP)
            prev = ST.sweep_segments(sw['c'], sw['u'], f0, S.AP) if f1 - f0 > 0.02 and len(segs) else None
            if prev is not None and len(prev) != len(segs):
                prev = None
            arr = ST.draw_strings(arr, cam, segs, COL[sw['owner']], kk, 0.55, prev, thick=(3, 3, 3, 2, 2, 2)[:len(segs)])
        fr.set_array(arr, depth)
    # ---------------- front ropes + posts
    draw_rope_side(0)
    draw_rope_side(1)
    draw_post('front')
    arr, depth = fr.array(), fr.depth()
    # ---------------- shields, slams, shards, sparks
    if in_fight:
        for owner, cx, cz, t0, t_slam, rad in RUN['SLAMS'][i]:
            h = ST.pick_height(t, t0, t_slam)                         # BIG STRUM: the giant pick's downstroke
            if h is not None:
                a = min(1.0, (t - t0) / 0.2) * (1.0 if t < t_slam + 0.4 else max(0.0, 1 - (t - t_slam - 0.4) / 0.2))
                ST.draw_pick(arr, cam, cx, cz, rad * 0.85, h, a, tilt=2.3 - 0.5 * ss(t_slam - 0.1, t_slam, t),
                             swing=ss(t_slam - 0.1, t_slam, t))
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
        elif kind == 'string':
            pts = [np.array([sx_, sz_]) + d * (j - 3) * 0.16 + nn * curl * 0.10 * math.sin(j) for j in range(7)]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            ImageDraw.Draw(im).line([tuple(q) for q in sp], fill=STRING_COL, width=1)
            arr = np.asarray(im).astype(np.float32)
        elif kind == 'fret':
            Lk, Wk = 0.30, 0.035
            c = np.array([222, 224, 236])
            q4 = [np.array([sx_, sz_]) + d * Lk / 2 * a1 + nn * Wk / 2 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            layer_blend(arr, [sp], c, 1.0)
        else:
            # top (spruce / koa top), side (dark sides): tumbling flat wooden chips
            col = WOOD_COL[kind]
            sz = size * (1.3 if kind == 'top' else 1.0)
            tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
                   np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
            shade_k = 0.75 + 0.25 * math.cos(rot * 2)
            layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
    sparks(arr, cam, t)
    arr = PA.draw(arr, cam, t, light=lambda P: 1.0 + 0.6 * FL.tint(P, t))     # sea foam + wood chips (reelkit.fx)
    # countdown in the ring (sharp, over everything in the ring)
    cs = cam.wp(np.array([[0.0, 0.3, 0.0]]))[0]
    fr.set_array(arr, depth)
    # ---------------- lens (focus locked on the ring), glows, the spot beam, bloom, grade
    img, depth = fr.array(), fr.depth()
    zf = cam.depth(S.SUBJECT)[0]
    img = dof(img, coc_map(depth, zf, 26.0, sharp=0.17))
    img = shimmers(img, cam, t, ta, sweeps_now, spout_now)
    for name, lp in (('red', S.RED_LIGHT), ('blue', S.BLUE_LIGHT)):
        s_ = cam.wp(lp[None])[0]
        fr.glow(s_[0], s_[1], 16, tuple(COL[0] if name == 'red' else COL[1]), 0.35 + 0.15 * T.beat_pulse(t, 0.2))
    for k, (s_, rp) in drawn.items():
        st = fs[k]
        if st['glow'] > 0.05:
            fr.glow(s_[0], s_[1], rp * 2.2, tuple(COL[k] * 0.8 + 50), 0.45 * st['glow'])
    for e_i, e in events_between(t - 0.15, t, ('hit',)):                 # (halved: the fx bursts carry the impacts)
        if e[7] in ('surf', 'sweep', 'spout', 'shockwave', 'rake'):
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            big = e[7] != 'rake'
            fr.glow(s_[0], s_[1], 12 if big else 7, tuple(COL[e[2]] * 0.7 + 70), (0.35 if big else 0.22) * (1 - (t - e[0]) / 0.15))
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
    img = post_fx(img, cam, t, fs, drawn)
    img = np.ascontiguousarray(np.clip(img, 0, 255).astype(np.uint8))
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
