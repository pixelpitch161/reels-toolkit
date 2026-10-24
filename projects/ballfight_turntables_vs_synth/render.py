"""#31 BALLFIGHT #5: Turntables vs Synth - renderer (#28's renderer, copied, with this pair's weapons and abilities; the
new styles live in styles.py).

arena floor (per pixel: spill, corner lights) -> stands + crowd (phones, flashbulbs) -> ring skirt -> canvas (per pixel:
the hard spot with real shadows of balls, satellites, discs, posts and ropes; the two corner lights; decals: the
synth's orbit trace, solo groove rings, white-noise static, the bass drop's target) -> back ropes + posts -> REWIND
ghosts -> fighters, satellites, discs, saw notes (painter) -> front ropes + posts -> effects (the dropping record,
sparks, shards) -> locked DOF on the ring -> glows -> spot beam -> bloom -> grade -> scrim -> REWIND tape look ->
countdown, HP numbers in the balls, fight HUD, captions.

    python render.py                                   # sprite cache (check/sprites_ballfight5.npz)
    python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 13.6 18.4 28.0 33.7 42.4 47.6 49.8 53 58.6 60.7
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
from reelkit.config import W, H, FPS, BAYER_FULL, SAFE, ss, ease
from reelkit.camera import interp_keys
from reelkit.raster import Frame, apply_glows
from reelkit.lighting import Lights, shade
from reelkit.shadows import MaskGrid, proj_point
from reelkit.lens import coc_map, dof
from reelkit.post import bloom, grade, vignette, top_scrim
from reelkit import ui
from reelkit.ui import draw_captions, label, fight_hud, AMBER, INK
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
VOLLEY = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'volley']
ORBIT = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'orbit']
CORNER0 = [np.array(c) for c in RUN['corner']]
STORY = sim.story(RUN)

# ---------------------------------------------------------------- the fighters' own colours on the shared ring (series
# rule: every instrument has its own colour; the corners are just left / right). Runtime patches of the imported scene,
# so projects/ballfight/scene.py stays as #27 uses it.
S.CORNER_COL['red'] = np.clip(COL[0] * 0.9, 0, 255)
S.CORNER_COL['blue'] = np.clip(COL[1] * 0.9, 0, 255)
S.POSTS = {k: S.post_faces(k) for k in S.CORNERS}
S.SKIRT_BAND = np.array([150, 140, 184])                 # a neutral band on the skirt
S.SKIRT_FACES = S.platform_faces()
LIGHT_RGB = [COL[k] / COL[k].max() for k in (0, 1)]     # corner light colour (max channel 1)
CROWD_TINT = [0.78 + 0.42 * LIGHT_RGB[k] for k in (0, 1)]

# ---------------------------------------------------------------- music-locked times shared with audio.py
T_SOLO = T.next_bar(T_KO + 1.6)                       # the winner's solo starts on the first bar line after the K.O. moment
SOLO_EV = [(T_SOLO + bt * T.BEAT,) + tuple(x) for bt, *x in T.SOLO[NAMES[WIN]]]
SOLO_T = sorted(set(e[0] for e in SOLO_EV))


def solo_env(t):
    past = [s for s in SOLO_T if s <= t]
    return math.exp(-(t - past[-1]) / 0.16) if past and t < T.T_REASSEMBLE + 0.3 else 0.0


def solo_rings(t):
    """the champion's solo throws groove rings off the ball: a big one on every stab / long note, small ones on the rest"""
    out = []
    for e in SOLO_EV:
        t0 = e[0]
        if t0 <= t < t0 + 0.6:
            big = (e[1] == 'stab') if NAMES[WIN] == 'TURNTABLES' else (e[2] >= 0.75)
            out.append((t0, 1.9 if big else 1.1))
    return out


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


OMEGA = [CFG[k]['weapon'].get('spin', 0.0) for k in (0, 1)]          # base orbit speed (the rev and the tail use it)
TH0 = [float(RUN['TH'][0, k]) for k in (0, 1)]
SD0 = [float(RUN['SPIN'][0, k]) for k in (0, 1)]
N0 = [int(CFG[k]['weapon'].get('n0', 0)) for k in (0, 1)]
BASE = [CFG[k]['weapon'].get('base', 0.0) for k in (0, 1)]


def theta_open(k, t):
    """orbit angle while revving: continuous into the fight's first frame (and, shifted by 61 s, from the tail into frame 0)"""
    return TH0[k] + SD0[k] * OMEGA[k] * (t - T.T_FIGHT)


def theta_tail(k, t):
    return theta_open(k, t - T.DUR)


def _wrap(a):
    return (a + math.pi) % (2 * math.pi) - math.pi


P_KO = [RUN['P'][-1, k].copy() for k in (0, 1)]

# REWIND windows: (t0, t1, k, heal) from the run's own events (the zip is a rule in the simulation)
RW_WIN = []
for e in EV:
    if e[1] == 'heal':
        t1 = next((f[0] for f in EV if f[1] == 'rewound' and f[2] == e[2] and f[0] >= e[0]), e[0] + T.BEAT)
        RW_WIN.append((e[0], t1, e[2], e[3]))
FIRE_T = [np.array([e[0] for e in EV if e[1] == 'fire' and e[2] == k]) for k in (0, 1)]


def rewinding(k, t):
    """(strength 0..1 of the tape look, the window) for fighter k at time t"""
    for t0, t1, kk, heal in RW_WIN:
        if kk == k and t0 - 0.06 <= t < t1 + 0.3:
            return ss(t0 - 0.06, t0 + 0.04, t) * (1 - ss(t1, t1 + 0.3, t)), (t0, t1)
    return 0.0, None


def fighter(k, t):
    """picture state of fighter k at time t: dict(p (ring frame), vis, th, reach, n (satellites), haste, glow, scale)"""
    st = dict(vis=True, haste=0.0, glow=0.0, scale=1.0, th=0.0, reach=BASE[k], n=N0[k], level=1, shattered=False)
    if t < T.T_FIGHT:
        st['p'] = CORNER0[k] + rev_offset(k, t)
        st['th'] = theta_open(k, t)
        st['glow'] = 0.25 * T.beat_pulse(t)
        return st
    if t < T_HS:
        i = sim_i(min(t, T_KO))
        st['p'] = RUN['P'][i, k].copy()
        st['th'] = float(RUN['TH'][i, k])
        st['reach'] = float(RUN['BL'][i, k])
        st['n'] = int(RUN['NV'][i, k])
        st['haste'] = float(RUN['HA'][i, k])
        st['level'] = int(RUN['LV'][i, k])
        if k in VOLLEY and len(FIRE_T[k]):                         # the decks thump on every throw
            j = np.searchsorted(FIRE_T[k], min(t, T_KO), 'right') - 1
            if j >= 0:
                st['glow'] = 0.45 * math.exp(-(min(t, T_KO) - FIRE_T[k][j]) / 0.09)
        if t >= T_KO and k == LOSE:
            st['glow'] = 1.0
        return st
    n_end = int(RUN['NV'][-1, k])
    reach_end = float(RUN['BL'][-1, k])
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
        st['reach'] = reach_end + (BASE[k] - reach_end) * gl
        st['n'] = n_end if gl < 0.5 else N0[k]
        st['glow'] = max(rev_glow, (0.35 + 0.5 * solo_env(t)) * ss(T_HS, T_HS + 0.6, t) * (1 - gl))
        st['level'] = 3 if t < T.T_GLIDE else 1
        return st
    # the loser: shattered until the reassembly, then glides home
    st['p'] = P_KO[k] + (corner - P_KO[k]) * gl
    st['th'] = theta_tail(k, t)
    st['shattered'] = t < T.T_GLIDE
    st['vis'] = t >= T.T_GLIDE - 0.12
    st['glow'] = max(rev_glow, 0.9 * (1 - ss(T.T_GLIDE - 0.12, T.T_GLIDE + 0.5, t)))
    st['scale'] = 0.75 + 0.25 * ss(T.T_GLIDE - 0.12, T.T_GLIDE + 0.12, t)
    return st


# ---------------------------------------------------------------- satellites: which rest after a hit / a parry
SAT_REST = [[] for _ in range(8)]                    # per satellite: [(t, rest s)]


def _sat_rest():
    if not ORBIT:
        return
    k = ORBIT[0]
    w = CFG[k]['weapon']
    for e in EV:
        if e[1] == 'hit' and e[2] == k and e[7] == 'orbit':
            SAT_REST[int(e[8])].append((e[0], w['cooldown']))
        elif e[1] == 'parry' and e[3] == k:
            i = sim_i(e[0])
            sats = ST.sat_positions(RUN['P'][i, k], float(RUN['TH'][i, k]), float(RUN['BL'][i, k]), int(RUN['NV'][i, k]))
            if sats:
                j = min(sats, key=lambda s: float(np.hypot(s[1][0] - e[4], s[1][1] - e[5])))[0]
                SAT_REST[j].append((e[0], w.get('parry_cool', w['cooldown'])))


_sat_rest()


def sat_k(j, t):
    """1 = lit, dims to 0.3 while the satellite rests after a hit, brightens back"""
    for t0, rest in SAT_REST[j]:
        if t0 <= t < t0 + rest:
            u = (t - t0) / rest
            return 0.3 + 0.7 * u ** 3
    return 1.0


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
    if e[1] == 'hit' and e[7] in ('orbit', 'slam', 'wave'):
        BIG.append((e[0], {'orbit': 0.6, 'slam': 3.5, 'wave': 2.5}[e[7]]))
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
    dx, dy = shake(t)
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
A_ = S.A


def _rope_ev():
    out = [(e[0], SIDE_OF[(e[3], e[4])], e[5], e[6], e[7]) for e in EV if e[1] == 'rope']
    for e in EV:                                                      # a ricocheting record twangs the rope a little
        if e[1] == 'ricochet':
            x, z = e[3], e[4]
            ax = 0 if abs(abs(x) - A_) < abs(abs(z) - A_) else 1
            sg = int(math.copysign(1, x if ax == 0 else z))
            out.append((e[0], SIDE_OF[(ax, sg)], x, z, 0.9))
    out.sort(key=lambda r: r[0])
    return out


ROPE_EV = _rope_ev()
ROPE_T = np.array([r[0] for r in ROPE_EV])
WAVE_EV = [(e[0], 1.0 if e[4] == 'wave' else 0.6) for e in EV if e[1] == 'ability' and e[4] in ('shockwave', 'wave')]
WAVE_EV += [(e[0], 0.9) for e in EV if e[1] == 'slam']


def side_ends(s):
    a, b = S.ROPE_SIDES[s]
    return S.CORNERS[a], S.CORNERS[b]


def rope_points(s, y, t, n=15):
    """one rope (side s, height y) as world points: slight sag + the give where balls / records hit it, decaying"""
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
    kinds = ['glass'] * n_glass + (['vinyl'] * 8 + ['label'] * 3 + ['arm'] * 2 + ['plinth'] * 3 if loser == 'TURNTABLES' else
                                   ['key'] * 7 + ['bkey'] * 4 + ['knob'] * 4 + ['gem'] * 4)
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


# ---------------------------------------------------------------- sprites (Pixel Pitch models tipped toward the camera)
_SPR = {}
SPR_PATH = os.path.join(HERE, 'check', 'sprites_ballfight5.npz')


def _sprite_cache():
    if _SPR:
        return _SPR
    if os.path.exists(SPR_PATH):
        z = np.load(SPR_PATH)
        _SPR.update({k: z[k] for k in z.files})
    return _SPR


def _key(nm, r, yk, roll, pitch):
    return f'{nm}_{r}_{yk % 360}_{int(roll)}_{int(pitch)}'


def sprite(k, radius_px, yaw_deg):
    cache = _sprite_cache()
    nm = CFG[k]['pp']
    roll, pitch = CFG[k]['view'][2], CFG[k].get('pitch', 0)
    r = int(np.clip(round(radius_px), 12, 30))
    yk = int(round(yaw_deg / 6.0)) * 6
    key = _key(nm, r, yk, roll, pitch)
    if key not in cache:
        cache[key] = ST.render_sprite(nm, r, float(yk), float(roll), float(pitch))
    return cache[key]


def build_sprites():
    """precompute every sprite the video can use (both instruments, radii 12..30, yaw window) -> check/sprites_ballfight5.npz"""
    out = {}
    for k in (0, 1):
        nm = CFG[k]['pp']
        yc, sw, roll = CFG[k]['view']
        pitch = CFG[k].get('pitch', 0)
        for r in range(12, 31):
            for yk in range(int((yc - sw - 6) // 6) * 6, int(yc + sw + 12), 6):
                out[_key(nm, r, yk, roll, pitch)] = ST.render_sprite(nm, r, float(yk), float(roll), float(pitch))
    os.makedirs(os.path.dirname(SPR_PATH), exist_ok=True)
    np.savez_compressed(SPR_PATH, **out)
    print(len(out), 'sprites ->', SPR_PATH)


# ---------------------------------------------------------------- drawing helpers
def layer_blend(arr, polys, col, alpha):
    """alpha-blend screen polygons (lists of points) of one colour onto arr"""
    if not polys:
        return
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if len(pts) >= 3:
            d.polygon([tuple(p) for p in pts], fill=255)
    mk = np.asarray(m, np.float32) / 255.0 * alpha
    arr += (np.array(col, float)[None, None] - arr) * mk[..., None]


def screen(cam, P):
    return cam.wp(np.atleast_2d(P))


def ring_outline(arr, cam, cx, cz, r, y, col, alpha, width=1):
    s = cam.wp(ST.circle_world(cx, cz, r, y, 28))
    im = Image.new('L', (W, H), 0)
    ImageDraw.Draw(im).polygon([tuple(p) for p in s], outline=255)
    if width > 1:
        ImageDraw.Draw(im).line([tuple(p) for p in s] + [tuple(s[0])], fill=255, width=width)
    mk = np.asarray(im, np.float32) / 255.0 * alpha
    arr += (np.array(col, float)[None, None] - arr) * mk[..., None]


# ---------------------------------------------------------------- glass balls
LIFT = {'TURNTABLES': 30}                  # the deck is black: lift its sprite so it reads through the lime glass


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
    # the instrument (tipped toward the camera); the decks wiggle while they scratch
    yc, sw, roll = CFG[k]['view']
    yaw = yc + sw * math.sin(1.25 * st['p'][0] + 0.45 * st['p'][1])
    if k in VOLLEY and (st['haste'] > 0 or (k == WIN and solo_env(t) > 0.2 and t > T_SOLO)):
        yaw += 12 * math.sin(2 * math.pi * t / (T.BEAT / 2))
    spr = sprite(k, rp * 0.80, yaw)
    sn = spr.shape[0]
    sx0, sy0 = int(round(s[0])) - sn // 2, int(round(s[1])) - sn // 2
    ax0, ay0 = max(0, sx0), max(0, sy0)
    ax1, ay1 = min(W, sx0 + sn), min(H, sy0 + sn)
    if ax1 > ax0 and ay1 > ay0:
        sub = spr[ay0 - sy0:ay1 - sy0, ax0 - sx0:ax1 - sx0]
        ms = sub[..., 3] > 0
        rg = arr[ay0:ay1, ax0:ax1]
        lit = sub[..., :3].astype(np.float32) * (1.12 + 0.3 * glow) + LIFT.get(NAMES[k], 10)
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
FLASH_EV = [e[0] for e in EV if (e[1] == 'hit' and e[7] in ('slam', 'wave')) or e[1] == 'ability'] + \
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
            body = S.CROWD_COLS[CROWD_CI[j]] * tint * spill
            draw.ellipse([x - 1.9 * rh, y + 0.6 * rh, x + 1.9 * rh, y + 3.4 * rh], fill=tuple(int(v) for v in body * 0.8))
            draw.ellipse([x - rh, y - rh, x + rh, y + rh], fill=tuple(int(v) for v in body * 0.62))
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
        pw = S.to_world(st['p'][0], st['p'][1], R)
        circ = np.array([[pw[0] + R * st['scale'] * math.cos(u), R, pw[2] + R * st['scale'] * math.sin(u)] for u in a])
        polys.append(proj_point(circ, spot))
        if k in ORBIT and not st['shattered']:
            for j, q in ST.sat_positions(st['p'], st['th'], st['reach'], st['n']):
                polys.append(proj_point(ST.gem_world(q, ST.GEM_R * 0.8), spot))
    for row in keys:
        if int(row[3]) in VOLLEY:
            polys.append(proj_point(ST.disc_poly(row[0], row[1], ST.DISC_R * 0.9, n=8), spot))
    return CANVAS_GRID.raster(polys, blur=0.7)


# ---------------------------------------------------------------- sparks
def sparks(arr, cam, t):
    for i, e in events_between(t - 0.32, t, ('hit', 'parry', 'block', 'wall', 'cleared', 'ricochet')):
        dt = t - e[0]
        kind = e[1]
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            col = COL[att] * 0.6 + 100
            n, sp, life = (16, 3.2, 0.3) if how in ('slam', 'wave', 'shockwave') else \
                ((9, 2.6, 0.24) if how == 'orbit' else (5, 1.8, 0.18))
        elif kind == 'parry':
            att, x, z = e[3], e[4], e[5]
            col, n, sp, life = np.array([200, 246, 255]), 7, 2.4, 0.22
        elif kind == 'ricochet':
            att, x, z = e[2], e[3], e[4]
            col, n, sp, life = COL[e[2]] * 0.5 + 110, 4, 1.4, 0.16
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
        if kind == 'hit' and e[7] in ('slam', 'orbit') and dt < 0.08:
            s = cam.wp(S.to_world(x, z, R)[None])[0]
            arr[max(0, int(s[1]) - 1):int(s[1]) + 2, max(0, int(s[0]) - 1):int(s[0]) + 2] = 255


def draw_shards(arr, cam, t):
    for kind, p, rot, size, curl, a in parts_at(t):
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        c0 = np.array([sx_, sz_])
        if kind == 'glass':
            tri = [c0 + d * size, c0 - d * size * 0.6 + nn * size * 0.7, c0 - d * size * 0.5 - nn * size * 0.6]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
            layer_blend(arr, [sp], COL[LOSE] * 0.35 + 170, 0.85)
            for (xq, yq) in sp.astype(int):
                if 0 <= xq < W and 0 <= yq < H:
                    arr[yq, xq] = COL[LOSE] * 0.6 + 100
        elif kind in ('arm', 'key', 'bkey'):
            Lk, Wk = {'arm': (0.46, 0.04), 'key': (0.30, 0.09), 'bkey': (0.22, 0.06)}[kind]
            c = {'arm': np.array([206, 210, 222]), 'key': np.array([240, 238, 230]), 'bkey': np.array([34, 32, 40])}[kind]
            q4 = [c0 + d * Lk / 2 * a1 + nn * Wk / 2 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            layer_blend(arr, [sp], c * (0.8 + 0.2 * math.cos(rot * 2)), 1.0)
        elif kind in ('knob', 'gem'):
            sp = screen(cam, S.to_world(sx_, sz_, p[1])[None])[0]
            xi, yi = int(sp[0]), int(sp[1])
            c = COL[LOSE] * 0.7 + 70 if kind == 'gem' else np.array([60, 58, 70])
            arr[max(0, yi - 1):yi + 1, max(0, xi - 1):xi + 1] = c
            if kind == 'knob' and 0 <= yi < H and 0 <= xi < W:
                arr[yi, xi] = COL[LOSE] * 0.8 + 50
        else:
            # vinyl (black wedges with grooves), label (lime bits), plinth (dark body chips): tumbling flat chips
            col = {'vinyl': ST.VINYL * 1.3, 'label': COL[LOSE] * 0.95, 'plinth': np.array([58, 56, 66])}[kind]
            sz = size * (1.5 if kind == 'vinyl' else 1.0)
            tri = [c0 + d * sz, c0 - d * sz * 0.7 + nn * sz * 0.8, c0 - d * sz * 0.4 - nn * sz * 0.7]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
            shade_k = 0.75 + 0.25 * math.cos(rot * 2)
            layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
            if kind == 'vinyl':
                g = (sp[0] * 0.5 + sp[1] * 0.25 + sp[2] * 0.25).astype(int)
                if 0 <= g[0] < W and 0 <= g[1] < H:
                    arr[g[1], g[0]] = ST.GROOVE * 1.4


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
    if T_KO <= t < T_KO + 0.9 and t >= T_HS:                      # records in flight at the K.O. drop and fade
        K = RUN['K'][-1]
        keys = [row for row in K if not np.isnan(row[0])]
        ko_fall = (t - T_HS) / (T_KO + 0.9 - T_HS)
    rings = solo_rings(t) if t >= T_SOLO - 0.01 else []
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
        # decals: the synth's orbit trace (dashes that scroll with the orbit; wobbles with the LFO)
        for k in ORBIT:
            st = fs[k]
            if st['vis'] and not st['shattered']:
                rd = np.hypot(sxp - st['p'][0], szp - st['p'][1])
                ring = np.clip(1 - np.abs(rd - (R + st['reach'])) / 0.07, 0, 1)
                ang_ = np.arctan2(szp - st['p'][1], sxp - st['p'][0])
                dash = (np.floor((ang_ - st['th'] + 4 * math.pi) * 16 / math.pi) % 2) == 0
                k_ = 0.30 + (0.20 if st['haste'] > 0 else 0.0)
                Ic += (ring * dash * k_)[:, None] * (COL[k] / 255.0 + 0.2)[None]
        for t0, reach in rings:                                       # the champion's solo: groove rings of light
            st = fs[WIN]
            u = t - t0
            r = R + min(reach, 6.0 * u)
            a = 1.0 if u < reach / 6.0 else max(0.0, 1 - (u - reach / 6.0) / 0.25)
            rd = np.hypot(sxp - st['p'][0], szp - st['p'][1])
            band = np.clip(1 - np.abs(rd - r) / 0.16, 0, 1) * (rd < r + 0.05)
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (band * a * 0.5)[:, None] * (COL[WIN] / 255.0 * 0.9 + 0.25)[None]
        wav = RUN['WAVES'][i] if in_fight else []
        for owner, cx, cz, r, rmax, kind, t0 in wav:
            rd = np.hypot(sxp - cx, szp - cz)
            wdt = 0.16 if kind == 'shockwave' else 0.32
            k_ = (1 - r / rmax) * (1.0 if kind == 'shockwave' else 1.3)
            if kind == 'wave':                                        # WHITE NOISE: a ring of static
                ang_ = np.arctan2(szp - cz, sxp - cx)
                rd = rd + 0.08 * np.sin(31 * ang_ + 70 * t) + 0.05 * np.sin(13 * ang_ - 40 * t)
            band = np.clip(1 - np.abs(rd - r) / wdt, 0, 1)
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (band * k_ * 0.9)[:, None] * (COL[owner] / 255.0 * 1.4 + (0.35 if kind == 'wave' else 0.15))[None]
            if kind == 'wave':                                        # ... with snow in it
                h = (np.floor(sxp * 23) * 7.3 + np.floor(szp * 23) * 3.1 + math.floor(t * 30) * 1.7) % 1.0
                snow = (h > 0.72) & (np.abs(rd - r) < wdt * 1.6)
                Ic[snow] += 0.9 * k_
        sl = RUN['SLAMS'][i] if in_fight else []
        for owner, cx, cz, t0, t_slam, rad in sl:
            rd = np.hypot(sxp - cx, szp - cz)
            if t < t_slam:
                u = (t - t0) / (t_slam - t0)
                m = rd < rad * 1.1
                Ic[m] *= 1 - 0.6 * u                                  # the record's shadow darkens as it falls
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
    # ---------------- REWIND: ghosts of the ball along the path it is zipping back over
    for k in (0, 1):
        rk, win_ = rewinding(k, t)
        if rk > 0 and win_ is not None and fs[k]['vis']:
            arr, depth = fr.array(), fr.depth()
            for g in range(1, 6):
                tg = max(win_[0], t - 0.035 * g)
                if tg >= t - 1e-6:
                    continue
                pg = RUN['P'][sim_i(tg), k]
                ring_outline(arr, cam, pg[0], pg[1], R * (1 - 0.06 * g), R, COL[k] * 0.7 + 70, 0.55 * rk * (1 - g / 6.5), 2)
            fr.set_array(arr, depth)
    # ---------------- the synth's orbit: a thin line of light at satellite height (never overlaps the ball)
    for k in ORBIT:
        st = fs[k]
        if st['vis'] and not st['shattered']:
            arr, depth = fr.array(), fr.depth()
            im = Image.new('L', (W, H), 0)
            ImageDraw.Draw(im).line(ST.orbit_line(cam, st['p'], st['reach']), fill=255, width=1)
            mk = np.asarray(im, np.float32) / 255.0 * (0.42 + (0.25 if st['haste'] > 0 else 0.0))
            arr += (np.clip(COL[k] * 0.8 + 60, 0, 255)[None, None] - arr) * mk[..., None]
            fr.set_array(arr, depth)
    # ---------------- fighters, satellites, records, saw notes, posts (painter by depth)
    items = []
    for name in ('red', 'blue'):
        items.append((cam.depth(S.to_world(*S.CORNERS[name], 0.7))[0], 'post', name))
    for k, st in enumerate(fs):
        if not st['vis']:
            continue
        pw = S.to_world(st['p'][0], st['p'][1], R)
        dz = cam.depth(pw)[0]
        items.append((dz, 'ball', k))
        if k in ORBIT and not st['shattered']:
            for j, q in ST.sat_positions(st['p'], st['th'], st['reach'], st['n']):
                items.append((cam.depth(S.to_world(q[0], q[1], R))[0], 'sat', (k, j, q)))
    for row in keys:
        items.append((cam.depth(S.to_world(row[0], row[1], 0.45))[0], 'key', row))
    items.sort(key=lambda it: -it[0])
    drawn = {}
    sat_glow = []
    for dz, kind, obj in items:
        if kind == 'post':
            draw_post(obj)
        elif kind == 'ball':
            st = fs[obj]
            arr, depth = fr.array(), fr.depth()
            drawn[obj] = draw_ball(arr, depth, cam, obj, st, t)
            fr.set_array(arr, depth)
        elif kind == 'sat':
            k, j, q = obj
            st = fs[k]
            sk = sat_k(j, t) if in_fight else 1.0
            if st['haste'] > 0:                                       # LFO: the orbit smears a little
                arr, depth = fr.array(), fr.depth()
                for g in (1, 2, 3):
                    thg = st['th'] - 0.12 * g * float(RUN['SPIN'][i, k])
                    qg = st['p'] + (R + st['reach']) * np.array([math.cos(thg + ST.SEATS[j] * math.pi / 4),
                                                                 math.sin(thg + ST.SEATS[j] * math.pi / 4)])
                    layer_blend(arr, [cam.wp(ST.gem_world(qg, ST.GEM_R * (0.8 - 0.15 * g)))], COL[k] * 0.6 + 80, 0.35 / g)
                fr.set_array(arr, depth)
            ST.draw_gem(fr, cam, q, COL[k], sk)
            sat_glow.append((q, sk, k))
        elif kind == 'key':
            row = obj
            own = int(row[3])
            if own in VOLLEY:
                ST.draw_disc(fr, cam, row[0], row[1], t * 14.0 + row[4] * 1.7, COL[own], r=ST.DISC_R * (1 - 0.4 * ko_fall))
            else:
                ST.draw_saw(fr, cam, row[0], row[1], row[2], COL[own])
    # ---------------- front ropes + posts
    draw_rope_side(0)
    draw_rope_side(1)
    draw_post('front')
    arr, depth = fr.array(), fr.depth()
    # ---------------- BASS DROP: the giant record falls on the spot; shards, sparks
    if in_fight:
        for owner, cx, cz, t0, t_slam, rad in RUN['SLAMS'][i]:
            h = ST.drop_height(t, t0, t_slam)
            if h is not None:
                a = min(1.0, (t - t0) / 0.2) * (1.0 if t < t_slam + 0.35 else max(0.0, 1 - (t - t_slam - 0.35) / 0.2))
                for s_, c in ST.drop_polys(cam, cx, cz, rad, h, COL[owner], t * 3.0):
                    layer_blend(arr, [s_], np.clip(c, 0, 255), a)
    draw_shards(arr, cam, t)
    sparks(arr, cam, t)
    cs = cam.wp(np.array([[0.0, 0.3, 0.0]]))[0]
    fr.set_array(arr, depth)
    # ---------------- lens (focus locked on the ring), glows, the spot beam, bloom, grade
    img, depth = fr.array(), fr.depth()
    zf = cam.depth(S.SUBJECT)[0]
    img = dof(img, coc_map(depth, zf, 26.0, sharp=0.17))
    for name, lp in (('red', S.RED_LIGHT), ('blue', S.BLUE_LIGHT)):
        s_ = cam.wp(lp[None])[0]
        fr.glow(s_[0], s_[1], 16, tuple(COL[0] if name == 'red' else COL[1]), 0.35 + 0.15 * T.beat_pulse(t, 0.2))
    for k, (s_, rp) in drawn.items():
        st = fs[k]
        if st['glow'] > 0.05:
            fr.glow(s_[0], s_[1], rp * 2.2, tuple(COL[k] * 0.8 + 50), 0.45 * st['glow'])
    for q, sk, k in sat_glow:
        s_ = cam.wp(S.to_world(q[0], q[1], R)[None])[0]
        fr.glow(s_[0], s_[1], 10, tuple(COL[k] * 0.7 + 70), 0.55 * sk)
    for e_i, e in events_between(t - 0.15, t, ('hit',)):
        if e[7] in ('slam', 'wave', 'shockwave', 'orbit'):
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            big = e[7] != 'orbit'
            fr.glow(s_[0], s_[1], 14 if big else 9, tuple(COL[e[2]] * 0.7 + 70), (0.7 if big else 0.5) * (1 - (t - e[0]) / 0.15))
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
    img = np.clip(img * vignette(strength=0.45) * top_scrim(0.45, 100, 90), 0, 255).astype(np.uint8)
    img = np.ascontiguousarray(img)
    # ---------------- REWIND: the tape look around the zipping ball (+ the ◀◀ glyph)
    for k, (s_, rp) in drawn.items():
        rk, win_ = rewinding(k, t)
        if rk > 0:
            img = ST.vhs(img, s_[0], s_[1], rk, t, seed=k)
            gx, gy = int(round(s_[0] - 6)), int(round(s_[1] - rp - 9))
            if SAFE[0] <= gx and gx + 13 <= SAFE[2] and SAFE[1] <= gy - 4 and gy + 4 <= SAFE[3]:
                ST.rewind_glyph(img, gx, gy, COL[k], rk)
                ui._note(gx, gy - 4, gx + 13, gy + 4, 'label', 'rewind glyph')
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
