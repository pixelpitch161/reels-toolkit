"""#35 BALLFIGHT #9: Organ vs Harp - renderer (#28's renderer, copied, with this pair's weapons and abilities; the
new weapon styles live in styles.py).

arena floor (per pixel: spill, corner lights) -> stands + crowd (phones, flashbulbs) -> ring skirt -> canvas (per pixel:
the hard spot with real shadows of balls, pipes, strings, darts, posts and ropes; corner lights; decals: each string's
light on the canvas, the pipes' glow pools, the dome's echo rings, the 47 STRINGS wave, the TOCCATA target) -> back
ropes + posts -> fighters, pipes, string segments, darts, dome / wings (painter) -> front ropes + posts -> effects (pipe
forest, string snaps, sparks, shards) -> locked DOF on the ring -> glows -> spot beam -> bloom -> grade -> scrim ->
countdown, HP numbers, fight HUD, captions.

    python render.py                                   # sprite cache (check/sprites_ballfight9.npz)
    python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 13.0 18.7 23.5 29.6 40.6 42.2 51.7 54 58.6 60.7
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
import title as TT                                               # v3: the shared VS title card (projects/ballfight)
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
BLADE = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'blade']
PULSE = []
LINE_K = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'line']
TRAP_K = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'trap']
LINES, STRINGS = sim.hazards(RUN)
LINE_T = np.array([l['t0'] for l in LINES]) if LINES else np.zeros(0)
STR_LIST = sorted(STRINGS.values(), key=lambda q: q['t0'])
STR_T = np.array([q['t0'] for q in STR_LIST]) if STR_LIST else np.zeros(0)
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


def solo_pipes(t):
    """the organ's solo: a ring of pipes erupts around the champion on every solo note (taller for lower notes)"""
    if NAMES[WIN] != 'ORGAN':
        return []
    out = []
    for j, (bt, m, d) in enumerate(T.SOLO['ORGAN']):
        t0 = T_SOLO + bt * T.BEAT
        if t0 <= t < t0 + 0.7 and t0 < T.T_REASSEMBLE:
            out.append((t0, j, m))
    return out


def solo_strings(t):
    """the harp's solo: shimmering strings fan out around the champion on every solo note"""
    if NAMES[WIN] != 'HARP':
        return []
    out = []
    for j, (bt, m, d) in enumerate(T.SOLO['HARP']):
        t0 = T_SOLO + bt * T.BEAT
        if t0 <= t < t0 + 0.6 and t0 < T.T_REASSEMBLE:
            out.append((t0, j, m))
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
              level=1, shattered=False, om=OMEGA[k], sd=SD0[k])
    if t < T.T_FIGHT:
        st['p'] = CORNER0[k] + rev_offset(k, t)
        st['yaw_p'] = CORNER0[k]                         # the sprite's view ignores the rev shake (no yaw-step flicker)
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
        evt = LINE_T if k in LINE_K else (STR_T if k in TRAP_K else None)  # the ball thumps on every row / string
        if evt is not None and len(evt):
            j = np.searchsorted(evt, min(t, T_KO), 'right') - 1
            if j >= 0:
                st['glow'] = 0.42 * math.exp(-(min(t, T_KO) - evt[j]) / 0.10)
        if t >= T_KO and k == LOSE:
            st['glow'] = 1.0
        return st
    blen_end = float(RUN['BL'][-1, k])
    fr_end = int(RUN['FR'][-1, k])
    gl = ease((t - T.T_GLIDE) / (T.T_REV - T.T_GLIDE))
    rv = rev_offset(k, t) * ss(T.T_REV - 0.3, T.T_REV + 0.2, t)
    corner = CORNER0[k] + rv
    rev_glow = 0.25 * T.beat_pulse(t) * ss(T.T_REV, T.T_REV + 0.3, t)
    if k == WIN:
        centre = np.array([0.0, 0.0])
        u = ease((t - T_HS) / 1.8)
        bob = 0.06 * math.sin(2 * math.pi * (t - T_HS) / (2 * T.BAR)) * ss(T_HS, T_HS + 1.5, t)
        p = P_KO[k] + (centre - P_KO[k]) * u + np.array([bob, -bob])
        st['p'] = p + (corner - p) * gl
        st['yaw_p'] = st['p'] - rv * gl
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
    st['yaw_p'] = st['p'] - rv * gl
    st['th'] = theta_tail(k, t)
    st['shattered'] = t < T.T_GLIDE
    st['vis'] = t >= T.T_GLIDE - 0.12
    st['glow'] = max(rev_glow, 0.9 * (1 - ss(T.T_GLIDE - 0.12, T.T_GLIDE + 0.5, t)))
    st['scale'] = 0.75 + 0.25 * ss(T.T_GLIDE - 0.12, T.T_GLIDE + 0.12, t)
    return st


# ---------------------------------------------------------------- camera
AIM = np.array([0.12, 0.25, 0.0])
SY = 280                                   # v4: the ring sits 8 px higher, clear of the taller HUD (ability list)
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
    if e[1] == 'hit' and e[7] in ('pipe', 'slam', 'wave'):
        SHK.add(e[0], {'pipe': 1.1, 'slam': 2.5, 'wave': 2.0}[e[7]])
    if e[1] == 'slam':
        SHK.add(e[0], 1.6)
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
    FL.apply(L, hz_clock(t))                                  # attacks light the scene in the fighter's colour
    for k in SHIELD:                                          # a shield glows from inside: wine dome / warm angel light
        st = fighter(k, t)
        sk = shield_k(k, st, t) if T.T_FIGHT <= t < T_HS else 0.0
        if sk > 0:
            pw = S.to_world(st['p'][0], st['p'][1], R * 1.6)
            c = COL[k] / 255.0 if SHIELD[k][0] == 'CATHEDRAL' else np.array([1.0, 0.86, 0.55])
            L.point(pw, c * 0.9 * sk, 2.0)
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
    kinds = ['glass'] * n_glass + (['string'] * 7 + ['maple'] * 5 + ['gold'] * 4 if loser == 'HARP' else
                                   ['pipe'] * 7 + ['case'] * 6 + ['gold'] * 2)
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
SPR_PATH = os.path.join(HERE, 'check', 'sprites_ballfight9.npz')


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
    """precompute every sprite the video can use (both instruments, radii 12..30, yaw window) -> check/sprites_ballfight.npz"""
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


# ---------------------------------------------------------------- weapons: see styles.py (pipes, strings, darts, dome, wings, forest, snap)


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
    nz = np.sqrt(np.clip(1 - d2, 0, 1))
    lam = np.clip(-0.25 * dx - 0.55 * dy + nz * 0.6, 0, 1)                   # lit from the spot above (top of the ball)
    band = np.floor(lam * 4 + BAYER_FULL[:n, :n] * 0.9) / 4
    inner = (col * 0.30 + np.array([236, 232, 226]) * 0.70)[None, None] * (0.70 + 0.36 * band[..., None])
    m = inside[sl]
    reg[m] = reg[m] * 0.12 + inner[sl][m] * 0.88
    # the instrument
    name = CFG[k]['pp']
    yc, sw, roll = CFG[k]['view']
    yp = st.get('yaw_p', st['p'])
    yaw = yc + sw * math.sin(1.25 * yp[0] + 0.45 * yp[1])
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
    title = []                                                      # v3: the VS title card replaces the top caption
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


# ---------------------------------------------------------------- the HUD ability list (series rule, Oct 25): LV2 / LV3 / LOW
SLOTS = (('unlock1', 'LV2'), ('unlock2', 'LV3'), ('low', 'LOW'))
SWEEP_T = 0.45                                       # s the glow takes to sweep across a name as it unlocks


def _hold(k, slot):
    """how long a use stays lit (as #36): a shield / haste for as long as it lasts, a slam until it lands, else a flash"""
    ab = CFG[k]['abilities'][slot]
    if ab['kind'] in ('shield', 'haste'):
        return ab['dur_beats'] * T.BEAT if 'dur_beats' in ab else ab['dur']
    if ab['kind'] == 'slam':
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
    colour after, white-hot on its pill on every use (held while CATHEDRAL / PULL ALL STOPS / ANGEL WINGS last and while
    TOCCATA's telegraph runs, then easing back over 0.5 s). Frozen through the hit-stop (the fight clock); back to grey
    when HP refills on the glide home."""
    tc = hz_clock(t) if t < T.T_GLIDE else -1.0
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
    # v3: no fly-in at the end of the file: the title card's slam on frame 0 is the loop's reset (title.py)


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
FLASH_EV = [e[0] for e in EV if (e[1] == 'hit' and e[7] in ('pipe', 'slam', 'wave', 'shockwave')) or e[1] == 'ability'] + \
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
    crowd_lit = 1.0 + 0.9 * FL.tint(heads, hz_clock(t))       # attack lights reach the crowd
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


# ---------------------------------------------------------------- hazards at a picture time (pipes, strings, slams)
def hz_clock(t):
    """hazard clock: real time in the fight, held through the hit-stop, then running on (rows sink, strings fade)"""
    if t < T_KO:
        return t
    if t < T_HS:
        return T_KO
    return t - HITSTOP


HARP_AT_LAY = {sid: RUN['P'][sim_i(q['t0']), q['owner']].copy() for sid, q in STRINGS.items()}
SLAM_EV = [(e[0], e[2], e[3], e[4]) for e in EV if e[1] == 'slam']          # (t_slam, owner, cx, cz)
SHIELD = {}
for _k in (0, 1):
    for _slot, _ab in CFG[_k]['abilities'].items():
        if _ab['kind'] == 'shield':
            SHIELD[_k] = (_ab['name'], _ab['dur_beats'] * T.BEAT)
SHIELD_ON = {k: [e[0] for e in EV if e[1] == 'ability' and e[2] == k and e[3] == SHIELD[k][0]] for k in SHIELD}


def pipes_at(t):
    """[(sx, sz, h, glow, owner, r)]: every pipe out of the floor (rows, the TOCCATA forest, the organ's solo ring)"""
    out = []
    tc = hz_clock(t)
    if tc >= T.T_FIGHT and len(LINE_T):
        j1 = int(np.searchsorted(LINE_T, tc, 'right'))
        for l in LINES[max(0, j1 - 8):j1]:
            for sx, sz, h, g in ST.row_pipes(l, tc):
                out.append((sx, sz, h, g, l['owner'], ST.PIPE_R))
    rad = CFG[LINE_K[0]]['abilities']['low']['radius'] if LINE_K else 1.4
    for ts, owner, cx, cz in SLAM_EV:
        if ts - 0.05 <= tc < ts + 1.0:
            for sx, sz, h, g, r in ST.forest_pipes(tc, ts, cx, cz, rad, int(ts * 1000)):
                out.append((sx, sz, h, g, owner, r))
    for t0, j, m in solo_pipes(t):                                     # the champion's solo ring
        c = fighter(WIN, t)['p']
        a = 2 * math.pi * j / 8.0 + 0.4
        u = t - t0
        k = min(1.0, u / 0.07) * (1 - min(1.0, max(0.0, (u - 0.45) / 0.2)))
        mm = m[1] if isinstance(m, tuple) else m
        h = (0.7 + 1.6 * (84 - mm) / 48.0) * (1 - (1 - k) ** 2)
        out.append((c[0] + 1.25 * math.cos(a), c[1] + 1.25 * math.sin(a), h, math.exp(-u / 0.2), WIN, 0.14))
    return out


def strings_at(t):
    """[(string, world points, alpha, twang)] of the harp's strings on screen at time t"""
    out = []
    if not STR_LIST:
        return out
    tc = hz_clock(t)
    fade = 1.0 - ss(T_HS + 0.5, T_HS + 1.1, t) if t >= T_KO else 1.0
    if fade <= 0 or tc < T.T_FIGHT:
        return out
    j1 = int(np.searchsorted(STR_T, tc, 'right'))
    for q in STR_LIST[max(0, j1 - 12):j1]:
        P, a = ST.string_points(q, tc, harp_p=q.get('harp_p'))
        if P is None:
            continue
        out.append((q, P, a * fade, ST.string_twang(q, tc)))
    return out


for _sid, _q in STRINGS.items():
    _q['harp_p'] = HARP_AT_LAY[_sid]


# ---------------------------------------------------------------- effects (reelkit.fx, BALLFIGHT v2): built once from the run
ORG, HRP = LINE_K[0], TRAP_K[0]
DUST_COL = [(178, 160, 150), (150, 128, 126), (208, 190, 176), (176, 120, 136)]     # stone dust with a hint of wine
FL = fx.Flares(max_n=6)
BU = fx.Bursts()
PA = fx.Particles(seed=35)
MK = fx.Marks()
MO = fx.Moments()


def _fx_build():
    # the organ's rows: each pipe kicks up dust as it bursts out; light runs along the row; the floor stays dusty a while
    for l in LINES:
        n = max(1, int(l['L'] / ST.PIPE_DX))
        for j in range(n):
            x = (j + 0.5) * l['L'] / n
            te = l['t0'] + x / l['speed']
            p = l['p0'] + l['u'] * x
            PA.emit(te, S.to_world(p[0], p[1], 0.06), 'dust', n=2, col=DUST_COL, vel=S.to_world(*(l['u'] * 0.3), 0.0))
            if j % 3 == 1:
                FL.add(te, S.to_world(p[0], p[1], 1.0), COL[ORG], radius=1.2, peak=0.6, decay=0.18)
            if j % 3 == 0:
                MK.add(te, 'dust', p[0], p[1], size=0.24, col=(196, 180, 172), life=2.4, seed=l['id'] * 31 + j,
                       strength=0.7)
    # the harp's strings: a pearl light along the new string, motes sprinkled down it as it's drawn across
    for sid, q in STRINGS.items():
        a, b = q['a'], q['b']
        if np.linalg.norm(q['harp_p'] - b) < np.linalg.norm(q['harp_p'] - a):
            a, b = b, a
        mid = (a + b) / 2
        FL.add(q['t0'] + ST.DRAW_IN * 0.5, S.to_world(mid[0], mid[1], ST.STR_Y + 0.5), COL[HRP] * 0.9, radius=1.6, peak=0.4,
               decay=0.3)
        for j in range(6):
            u = (j + 0.5) / 6
            p = a + (b - a) * u
            PA.emit(q['t0'] + ST.DRAW_IN * u, S.to_world(p[0], p[1], ST.STR_Y), 'motes', n=1)
    # a thin trail of motes off the harp ball on every beat of the fight
    tb = T.T_FIGHT
    while tb < T_KO:
        i = sim_i(tb)
        p, v = RUN['P'][i, HRP], RUN['V'][i, HRP]
        PA.emit(tb, S.to_world(p[0], p[1], R * 1.2), 'motes', n=1, vel=S.to_world(*(-0.25 * v), 0.0))
        tb += T.BEAT
    # hits: bursts, lights, particles, marks
    for e in EV:
        if e[0] > T_KO + 1e-6:
            break
        if e[1] == 'hit':
            att, x, z, how, sub = e[2], e[5], e[6], e[7], e[8]
            pos = S.to_world(x, z, R)
            if how == 'pipe':
                BU.add(e[0], pos, COL[att], 1.3)
                FL.add(e[0], pos, COL[att], radius=1.6, peak=0.9, decay=0.28)
                PA.emit(e[0], S.to_world(x, z, 0.1), 'dust', n=10, col=DUST_COL, speed=(0.6, 1.6))
                MK.add(e[0], 'crack', x, z, size=0.55, n=5, life=4.0, seed=int(e[0] * 100))
            elif how == 'string':
                q = STRINGS.get(sub)
                ang = math.atan2(*(q['b'] - q['a'])[::-1]) if q is not None else 0.0
                BU.add(e[0], pos, COL[att], 0.7)
                FL.add(e[0], pos, COL[att] * 0.9, radius=1.4, peak=0.6, decay=0.24)
                PA.emit(e[0], pos, 'motes', n=8, speed=(0.5, 1.4))
                MK.add(e[0], 'claw', x, z, size=0.42, angle=ang, n=1, col=(255, 246, 222), life=3.0, seed=int(e[0] * 100))
            elif how == 'key':
                BU.add(e[0], pos, COL[att], 0.35, bright=False)
                PA.emit(e[0], pos, 'motes', n=3)
            elif how == 'slam':
                BU.add(e[0], pos, COL[att], 2.0)
            elif how == 'wave':
                BU.add(e[0], pos, COL[att], 1.6)
                PA.emit(e[0], pos, 'motes', n=12, speed=(1.0, 2.2))
        elif e[1] == 'block':
            BU.add(e[0], S.to_world(e[4], e[5], R), (220, 230, 255), 0.6, bright=False)
            PA.emit(e[0], S.to_world(e[4], e[5], R), 'motes' if e[2] == HRP else 'dust', n=5,
                    col=None if e[2] == HRP else DUST_COL)
    # TOCCATA's forest: dust everywhere, a cracked crater, a big wine light
    for ts, owner, cx, cz in SLAM_EV:
        c = S.to_world(cx, cz, 0.1)
        FL.add(ts, S.to_world(cx, cz, 1.2), COL[owner], radius=2.4, peak=1.2, decay=0.5)
        PA.emit(ts, c, 'dust', n=30, col=DUST_COL, spread=180, speed=(0.6, 1.8), scale=1.3)
        MK.add(ts, 'crack', cx, cz, size=1.6, n=8, life=7.0, seed=int(ts * 10))
        MK.add(ts, 'scorch', cx, cz, size=1.2, col=(120, 40, 70), life=7.0, seed=int(ts * 10) + 1, strength=0.6)
    # abilities: 47 STRINGS lights the ring pearl and scars it; every captioned ability gets its moment
    for e in EV:
        if e[1] == 'ability' and e[3] == '47 STRINGS':
            p = RUN['P'][sim_i(e[0]), e[2]]
            FL.add(e[0], S.to_world(p[0], p[1], 1.0), COL[e[2]] * 0.9, radius=2.6, peak=0.75, decay=0.5)
            MK.add(e[0], 'smoke_ring', p[0], p[1], size=1.5, life=5.0, seed=47, strength=0.7)
    for t, who, name in STORY['caps']:
        MO.add(t, who, COL[who])
    # the K.O.
    p = P_KO[LOSE]
    pos = S.to_world(p[0], p[1], R)
    BU.add(T_KO, pos, COL[LOSE], 2.0)
    FL.add(T_KO, pos, COL[LOSE] * 0.9, radius=2.4, peak=1.1, decay=0.6)
    PA.emit(T_KO, pos, 'motes' if NAMES[LOSE] == 'HARP' else 'dust', n=30, spread=180, speed=(0.8, 2.2),
            col=None if NAMES[LOSE] == 'HARP' else DUST_COL)
    # the organ's solo: every solo pipe kicks up a little dust
    if NAMES[WIN] == 'ORGAN':
        for j, (bt, m, d) in enumerate(T.SOLO['ORGAN']):
            t0 = T_SOLO + bt * T.BEAT
            if t0 < T.T_REASSEMBLE:
                c = fighter(WIN, t0)['p']
                a = 2 * math.pi * j / 8.0 + 0.4
                PA.emit(t0 - HITSTOP, S.to_world(c[0] + 1.25 * math.cos(a), c[1] + 1.25 * math.sin(a), 0.06), 'dust',
                        n=3, col=DUST_COL)
                FL.add(t0 - HITSTOP, S.to_world(c[0] + 1.25 * math.cos(a), c[1] + 1.25 * math.sin(a), 1.2), COL[WIN],
                       radius=1.4, peak=0.7, decay=0.25)


_fx_build()


def shimmers(img, cam, t, fs):
    """heat-shimmer ripples: the 47 STRINGS wave front, TOCCATA's shock ring, the dome's echo rings"""
    if T.T_FIGHT <= t < T_HS:
        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][sim_i(t)]:
            img = fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.4, 3.0, t, k=1 - r / rmax, seed=owner)
    tc = hz_clock(t)
    rad = CFG[ORG]['abilities']['low']['radius']
    for ts, owner, cx, cz in SLAM_EV:
        u = (tc - ts) / 0.6
        if 0 <= u < 1:
            img = fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), rad + 1.6 * u, 0.35, 3.0, t, k=1 - u)
    for k in SHIELD:
        if SHIELD[k][0] == 'CATHEDRAL' and fs[k]['shield'] > 0 and T.T_FIGHT <= t < T_HS:
            past = [x for x in SHIELD_ON[k] if x <= t]
            if past:
                for rr_, a_ in ST.echo_rings(t, past[-1], 2 * T.BEAT):
                    img = fx.shimmer_world(img, cam, S.to_world(fs[k]['p'][0], fs[k]['p'][1], 0.05), rr_, 0.2, 1.6, t,
                                           k=a_ / 0.6)
    return img


def post_fx(img, cam, t, fs, drawn):
    """after the grade: chromatic fringing on big hits, the ability moments (dim + speed lines), the impact bursts"""
    for _, e in events_between(t - 0.12, t, ('hit',)):
        if e[7] in ('pipe', 'slam', 'wave'):
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            img = fx.fringe(img, s_[0], s_[1], 26 if e[7] != 'pipe' else 18, 2.5 if e[7] != 'pipe' else 1.6,
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


# ---------------------------------------------------------------- shadows on the canvas (from the spot)
CANVAS_GRID = MaskGrid((-5.4, 5.4, -5.4, 5.4, 0.03))
STATIC_CASTERS = []
for name in S.CORNERS:
    w = S.to_world(*S.CORNERS[name])
    STATIC_CASTERS.append(np.array([w + [-0.09, 0, 0], w + [0.09, 0, 0], w + [0.09, S.POST_H, 0], w + [-0.09, S.POST_H, 0]]))


def canvas_shadows(cam, t, fs, keys, spot, pipes, strs):
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
    for sx, sz, h, g, owner, r in pipes:
        if h > 0.05:
            polys.append(proj_point(ST.pipe_shadow_quad(sx, sz, h, r), spot))
    for q, P, a, tw in strs:
        if a > 0.3:
            for p0, p1 in zip(P[:-1:2], P[2::2]):
                polys.append(proj_point(np.array([p0 + [0, -0.012, 0], p1 + [0, -0.012, 0], p1 + [0, 0.012, 0],
                                                  p0 + [0, 0.012, 0]]), spot))
    for row in keys:
        head, body = ST.dart_quads(row[0], row[1], row[2])
        polys.append(proj_point(head, spot))
    return CANVAS_GRID.raster(polys, blur=0.7)


# ---------------------------------------------------------------- sparks
BIG_HITS = ('pipe', 'slam', 'wave', 'shockwave')


def sparks(arr, cam, t):
    for i, e in events_between(t - 0.32, t, ('hit', 'block', 'wall', 'cleared')):
        dt = t - e[0]
        kind = e[1]
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            col = COL[att] * 0.6 + 100
            n, sp, life = (16, 3.2, 0.3) if how in BIG_HITS else ((9, 2.6, 0.24) if how == 'string' else (5, 1.8, 0.18))
        elif kind == 'block':
            att, x, z = e[2], e[4], e[5]
            col, n, sp, life = np.array([255, 226, 170]), 12, 2.6, 0.3
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
        if kind == 'hit' and e[7] in ('pipe', 'slam') and dt < 0.08:
            s = cam.wp(S.to_world(x, z, R)[None])[0]
            arr[max(0, int(s[1]) - 1):int(s[1]) + 2, max(0, int(s[0]) - 1):int(s[0]) + 2] = 255


def shield_k(k, st, t):
    """0..1 strength of fighter k's shield look (fades in over 0.12 s and out over the last 0.2 s)"""
    if st['shield'] <= 0 or k not in SHIELD:
        return 0.0
    dur = SHIELD[k][1]
    return float(min(1.0, st['shield'] / 0.2, (dur - st['shield']) / 0.12 + 0.2))


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
    pipes = pipes_at(t)
    strs = strings_at(t)
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
        alb = MK.apply(alb, sxp, szp, hz_clock(t), BAYER_FULL[ys, xs])      # fading marks (reelkit.fx)
        sh = CANVAS_GRID.sample(canvas_shadows(cam, t, fs, keys, spot, pipes, strs), Xp, Zp)
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
        # decals: each string's light on the canvas under it
        for q, P, a, tw in strs:
            a2, b2 = q['a'], q['b']
            d = b2 - a2
            ln = float(np.hypot(*d))
            u_ = np.clip(((sxp - a2[0]) * d[0] + (szp - a2[1]) * d[1]) / (ln * ln), 0, 1)
            dd_ = np.hypot(sxp - (a2[0] + u_ * d[0]), szp - (a2[1] + u_ * d[1]))
            band = np.clip(1 - dd_ / 0.16, 0, 1) ** 2
            Ic += (band * a * (0.22 + 0.4 * tw))[:, None] * (COL[q['owner']] / 255.0 * 0.6 + 0.3)[None]
        # the pipes' glow pools (owner colour) while they speak
        for psx, psz, h, g, owner, r in pipes:
            if g > 0.15:
                m_ = (np.abs(sxp - psx) < 0.5) & (np.abs(szp - psz) < 0.5)
                if m_.any():
                    dd_ = np.hypot(sxp[m_] - psx, szp[m_] - psz)
                    Ic[m_] += (np.clip(1 - dd_ / 0.45, 0, 1) * g * 0.35)[:, None] * (COL[owner] / 255.0 + 0.15)[None]
        # the dome's echo rings
        for k in SHIELD:
            if SHIELD[k][0] == 'CATHEDRAL' and fs[k]['shield'] > 0 and in_fight:
                past = [x for x in SHIELD_ON[k] if x <= t]
                if past:
                    for rr_, a_ in ST.echo_rings(t, past[-1], 2 * T.BEAT):
                        rd = np.hypot(sxp - fs[k]['p'][0], szp - fs[k]['p'][1])
                        band = np.clip(1 - np.abs(rd - rr_) / 0.08, 0, 1)
                        Ic += (band * a_ * 0.7)[:, None] * (COL[k] / 255.0 + 0.25)[None]
        wav = RUN['WAVES'][i] if in_fight else []
        for owner, cx, cz, r, rmax, kind, t0 in wav:
            rd = np.hypot(sxp - cx, szp - cz)
            wdt = 0.16 if kind == 'shockwave' else 0.30
            k_ = (1 - r / rmax) * (1.0 if kind == 'shockwave' else 1.3)
            band = np.clip(1 - np.abs(rd - r) / wdt, 0, 1)
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (band * k_ * 0.9)[:, None] * (COL[owner] / 255.0 * 1.2 + 0.2)[None]
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
    # ---------------- fighters, pipes, strings, darts, shields, posts (painter by depth)
    items = []
    for name in ('red', 'blue'):
        items.append((cam.depth(S.to_world(*S.CORNERS[name], 0.7))[0], 'post', name))
    for k, st in enumerate(fs):
        if not st['vis']:
            continue
        pw = S.to_world(st['p'][0], st['p'][1], R)
        dz = cam.depth(pw)[0]
        items.append((dz, 'ball', k))
        sk = shield_k(k, st, t)
        if sk > 0:
            if SHIELD[k][0] == 'CATHEDRAL':
                items.append((dz - 0.05, 'dome', (k, sk)))
            else:
                items.append((dz + 0.05, 'wings', (k, sk)))
                items.append((dz - 0.05, 'halo', (k, sk)))
    for pp in pipes:
        items.append((cam.depth(S.to_world(pp[0], pp[1], 0.0))[0], 'pipe', pp))
    for q, P, a, tw in strs:
        n_ = len(P)
        for c0 in range(0, n_ - 1, 4):
            seg = P[c0:min(n_, c0 + 5)]
            items.append((cam.depth(seg.mean(0)[None])[0], 'string', (q, seg, a, tw)))
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
            fr.set_array(arr, depth)
        elif kind == 'pipe':
            psx, psz, h, g, owner, r = obj
            ST.draw_pipe(fr, cam, L, shade, psx, psz, h, g, COL[owner], r)
        elif kind == 'string':
            q, seg, a, tw = obj
            arr, depth = fr.array(), fr.depth()
            ST.draw_string(arr, cam, seg, a, COL[q['owner']], tw)
            fr.set_array(arr, depth)
        elif kind == 'key':
            row = obj
            ST.draw_dart(fr, cam, L, shade, row[0], row[1], row[2], COL[int(row[3])])
        elif kind in ('dome', 'wings', 'halo'):
            k, sk = obj
            p = fs[k]['p']
            arr, depth = fr.array(), fr.depth()
            if kind == 'dome':
                ST.draw_dome(arr, cam, p[0], p[1], t, sk, COL[k], 0.0)
            elif kind == 'wings':
                ST.draw_wings(arr, cam, p[0], p[1], t, sk)
            else:
                ST.draw_halo(arr, cam, p[0], p[1], t, sk)
            fr.set_array(arr, depth)
    # ---------------- front ropes + posts
    draw_rope_side(0)
    draw_rope_side(1)
    draw_post('front')
    arr, depth = fr.array(), fr.depth()
    # ---------------- 47 STRINGS snaps, the harp's solo strings, shards, sparks
    wav = RUN['WAVES'][i] if in_fight else []
    for owner, cx, cz, r, rmax, kind, t0 in wav:
        if kind == 'wave' and NAMES[owner] == 'HARP':
            a_ = max(0.0, 1 - r / rmax) ** 0.6
            segs = []
            for p0, p1 in ST.snap_segments(cx, cz, r, t, t0):
                s0, s1 = cam.wp(np.array([S.to_world(p0[0], p0[1], ST.STR_Y), S.to_world(p1[0], p1[1], ST.STR_Y)]))
                segs.append((s0, s1))
            mk = ST.line_mask(segs)
            arr += (ST.PEARL[None, None] - arr) * (mk * 0.95 * a_)[..., None]
    for t0, j, m in solo_strings(t):
        c = fighter(WIN, t)['p']
        mm = m[1] if isinstance(m, tuple) else m
        a = 2 * math.pi * ((mm - 50) / 36.0) + 0.3
        u = t - t0
        al = math.exp(-u / 0.25)
        r0, r1 = R + 0.15, R + 0.9 + 0.5 * (1 - math.exp(-u / 0.1))
        segs = []
        for da in (-0.12, 0.0, 0.12):
            pts = [S.to_world(c[0] + rr_ * math.cos(a + da), c[1] + rr_ * math.sin(a + da), ST.STR_Y + 0.25) for rr_ in (r0, r1)]
            sp_ = cam.wp(np.array(pts))
            segs.append((sp_[0], sp_[1]))
        mk = ST.line_mask(segs)
        arr += (ST.PEARL[None, None] - arr) * (mk * al)[..., None]
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
        elif kind == 'string':
            pts = [np.array([sx_, sz_]) + d * (j - 3) * 0.16 + nn * curl * 0.10 * math.sin(j) for j in range(7)]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            mk = ST.line_mask(list(zip(sp[:-1], sp[1:])))
            arr += (ST.PEARL[None, None] - arr) * mk[..., None]
        elif kind == 'pipe':
            q4 = [np.array([sx_, sz_]) + d * 0.22 * a1 + nn * 0.06 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            layer_blend(arr, [sp], ST.TIN * (0.8 + 0.2 * math.cos(rot * 2)), 1.0)
        else:
            col = {'maple': np.array([214, 160, 96]), 'gold': ST.GOLD, 'case': np.array([92, 52, 40])}[kind]
            sz = size * (1.2 if kind == 'case' else 0.9)
            tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
                   np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
            shade_k = 0.75 + 0.25 * math.cos(rot * 2)
            layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
    sparks(arr, cam, t)
    tc = hz_clock(t)
    arr = PA.draw(arr, cam, tc, light=lambda P: 1.0 + 0.6 * FL.tint(P, tc))   # organ dust + harp shimmer motes
    cs = cam.wp(np.array([[0.0, 0.3, 0.0]]))[0]
    fr.set_array(arr, depth)
    # ---------------- lens (focus locked on the ring), glows, the spot beam, bloom, grade
    img, depth = fr.array(), fr.depth()
    zf = cam.depth(S.SUBJECT)[0]
    img = dof(img, coc_map(depth, zf, 26.0, sharp=0.17))
    img = shimmers(img, cam, t, fs)                                # heat-shimmer ripples on shockwave fronts (reelkit.fx)
    for name, lp in (('red', S.RED_LIGHT), ('blue', S.BLUE_LIGHT)):
        s_ = cam.wp(lp[None])[0]
        fr.glow(s_[0], s_[1], 16, tuple(COL[0] if name == 'red' else COL[1]), 0.35 + 0.15 * T.beat_pulse(t, 0.2))
    for k, (s_, rp) in drawn.items():
        st = fs[k]
        if st['glow'] > 0.05:
            fr.glow(s_[0], s_[1], rp * 2.2, tuple(COL[k] * 0.8 + 50), 0.45 * st['glow'])
    for psx, psz, h, g, owner, r in pipes:                            # the speaking pipe tops glow
        if g > 0.3:
            s_ = cam.wp(S.to_world(psx, psz, h)[None])[0]
            fr.glow(s_[0], s_[1], 5, tuple(COL[owner] * 0.7 + 70), 0.35 * g)
    for e_i, e in events_between(t - 0.15, t, ('hit',)):
        if e[7] in BIG_HITS or e[7] == 'string':
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            big = e[7] != 'string'
            fr.glow(s_[0], s_[1], 12 if big else 7, tuple(COL[e[2]] * 0.7 + 70), (0.4 if big else 0.25) * (1 - (t - e[0]) / 0.15))
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
    img = post_fx(img, cam, t, fs, drawn)                          # fringe, ability dim + speed lines, impact bursts
    img = np.ascontiguousarray(np.clip(img, 0, 255).astype(np.uint8))
    # ---------------- UI: countdown, HP numbers in the balls, fight HUD, captions
    countdown(img, t, cs)
    card_closed = TT.door_offset(t) == 0.0                            # v3: the title card covers the HUD: skip it
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
        fl = 0.0
        for _, e in events_between(t - 0.12, t, ('hit',)):
            if e[3] == k and e[7] not in ('key', 'string'):
                fl = max(fl, 1 - (t - e[0]) / 0.12)
        lvl = fs[k]['level'] if T.T_FIGHT <= t else 1
        hud.append(dict(name=NAMES[k], color=COL[k], hp=hp_at(k, t), trail=max(hp_at(k, t), hp_lag[k]), level=lvl,
                        flash=fl, ko=(k == LOSE and T_KO <= t < T.T_GLIDE), t=t,
                        win=(k == WIN and T_KO + 1.3 <= t < T.T_GLIDE)))
    if not card_closed:                                                # v4: the ability list replaces pips + status line
        fight_hud(img, hud[0], hud[1], abilities=(ability_rows(0, t), ability_rows(1, t)))
    for k in (0, 1):
        c = tuple(int(v) for v in np.clip(COL[k] * 0.8 + 60, 0, 255))
        draw_captions(img, t, CAP_ABIL[k], style='impact', base=c, puff=tuple(int(v) for v in COL[k]))
    draw_captions(img, t, CAP_KO, style='impact', hi=(255, 214, 96))
    wc = tuple(int(v) for v in np.clip(COL[WIN] * 0.8 + 60, 0, 255))
    draw_captions(img, t, CAP_WINS, style='impact', base=wc)
    draw_captions(img, t, CAP_COMMENT, style='impact', hi=(255, 214, 96))
    img = TT.draw(img, t)                                           # v3: the VS title card, over everything
    return img


if __name__ == '__main__':
    build_sprites()
