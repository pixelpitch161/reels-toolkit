"""#34 BALLFIGHT #8: Trombone vs French Horn - renderer, v3 (v2: the HORN hunts with a pack of ghost hounds, effects from
reelkit.fx; v3: the shared VS title card on bar 0, the count-in on bar 1, FIGHT! on bar 2; v2 kept as render_v2_backup.py). #28's renderer, copied, with this pair's weapons and abilities; the weapon styles
live in styles.py. v1 (curving arrows) is kept as render_v1_backup.py / styles_v1_backup.py.

arena floor (per pixel: spill, corner lights) -> stands + crowd (phones, flashbulbs) -> ring skirt -> canvas (per pixel:
the hard spot with real shadows of balls, the slide, arrows, posts and ropes; the fighters' corner lights; decals: the
shockwaves, the TALLY-HO chevron wave, the slowed ball's wobble, the pedal bell's target) -> back ropes + posts ->
fighters, slide lance, arrows + trails, bell-rim shield (painter) -> front ropes + posts -> effects (the giant bell,
sparks, shards) -> countdown -> locked DOF on the ring -> glows -> spot beam -> bloom -> grade -> scrim -> HP numbers,
fight HUD, captions.

    python render.py                                   # sprite cache (check/sprites_ballfight8.npz)
    python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 2.2 12.9 17.8 26.7 31.6 45.3 50.3 53.5 58.6 60.7
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
import title as TT                                                  # v3: the shared VS title card (projects/ballfight)
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
BLADE = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'blade']       # the slide lance (thrust mode)
PACK = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'pack']         # THE HUNT (v2)
HOUNDS = RUN.get('HOUNDS')
MARK = RUN.get('MARK')
CHG = RUN.get('CHG')
CALL_T = {k: np.array([e[0] for e in EV if e[1] == 'call' and e[2] == k]) for k in PACK}
THR_T = np.array([e[0] for e in EV if e[1] == 'thrust'])
REST = [CFG[k]['weapon'].get('rest', 0.0) for k in (0, 1)]
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


OMEGA = [CFG[k]['weapon'].get('spin', 0.0) for k in (0, 1)]          # base spin (0 for the slide: it just aims)
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
SOLO_N = T.SOLO.get(NAMES[WIN], [])
SOLO_LO = min((m for b, m, d in SOLO_N), default=0)
SOLO_HI = max((m for b, m, d in SOLO_N), default=1)


def solo_ext(t):
    """the winning trombone plays its solo with the slide: a lower note = a longer slide (real slide positions run
    from 1st, closed, to 7th, out), gliding to each note's position as it starts; closed between phrases"""
    reach = float(RUN['BL'][-1, WIN])
    past = [(T_SOLO + b * T.BEAT, m, d) for b, m, d in SOLO_N if T_SOLO + b * T.BEAT <= t]
    if not past or NAMES[WIN] != 'TROMBONE':
        return REST[WIN]
    t0, m, d = past[-1]
    pos = 1 - (m - SOLO_LO) / max(1, SOLO_HI - SOLO_LO)
    tgt = REST[WIN] + 0.25 + (reach - REST[WIN] - 0.25) * pos
    prev = past[-2] if len(past) > 1 else None
    p0 = REST[WIN] if prev is None else REST[WIN] + 0.25 + (reach - REST[WIN] - 0.25) * (1 - (prev[1] - SOLO_LO) / max(1, SOLO_HI - SOLO_LO))
    u = ease(min(1.0, (t - t0) / 0.07))
    return p0 + (tgt - p0) * u
BASE_LEN = [CFG[k]['weapon'].get('base', 0.0) for k in (0, 1)]
FRETS0 = [CFG[k]['weapon'].get('frets0', 0) for k in (0, 1)]


def fighter(k, t):
    """picture state of fighter k at time t: dict(p (ring frame), vis, th, blen, frets, shield, haste, glow, scale)"""
    st = dict(vis=True, shield=0.0, haste=0.0, glow=0.0, scale=1.0, th=0.0, blen=BASE_LEN[k], frets=FRETS0[k],
              level=1, shattered=False, om=OMEGA[k], sd=SD0[k], ext=REST[k], slow=0.0, thr_k=0.0)
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
        st['ext'] = float(RUN['EXT'][i, k])
        st['slow'] = float(RUN['SLW'][i, k])
        if k in BLADE and len(THR_T):                             # the trombone ball thumps on every thrust
            j = np.searchsorted(THR_T, min(t, T_KO), 'right') - 1
            if j >= 0:
                st['thr_k'] = math.exp(-(min(t, T_KO) - THR_T[j]) / 0.1)
                st['glow'] = 0.35 * st['thr_k']
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
        if k in BLADE:                                            # the solo on the slide, closing for the glide home
            e_ko = float(RUN['EXT'][-1, k])
            e = e_ko + (REST[k] - e_ko) * ss(T_HS, T_HS + 0.4, t) if t < T_SOLO else solo_ext(t)
            st['ext'] = e + (REST[k] - e) * ss(T.T_REASSEMBLE - 0.3, T.T_REASSEMBLE, t)
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
    if e[1] == 'hit' and e[7] in ('blade', 'slam', 'wave', 'shockwave', 'charge'):
        SHK.add(e[0], {'blade': 0.55, 'slam': 2.5, 'wave': 2.0, 'shockwave': 1.2, 'charge': 2.2}[e[7]])
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
    kinds = ['glass'] * n_glass + (['tube'] * 6 + ['valve'] * 3 + ['bellbit'] * 6 if loser == 'HORN' else
                                   ['slidebit'] * 4 + ['tube'] * 4 + ['bellbit'] * 6)
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
SPR_PATH = os.path.join(HERE, 'check', 'sprites_ballfight8.npz')


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


# ---------------------------------------------------------------- weapons: see styles.py (slide, arrows, bell rim, giant bell)


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


def under_bell(k, t):
    """is fighter k covered by a giant bell that is down (or nearly) right now?"""
    if not (T.T_FIGHT <= t < T_HS):
        return False
    p = fighter(k, t)['p']
    for owner, cx, cz, t0, t_slam, rad in RUN['SLAMS'][sim_i(t)]:
        h = ST.bell_height(t, t0, t_slam)
        if h is not None and h < 1.6 and math.hypot(p[0] - cx, p[1] - cz) < rad * 1.05:
            return True
    return False


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
        if k in BLADE and not st['shattered']:
            quads, _ = ST.slide_geom(st['p'], st['th'], st['ext'])
            for Q, c, kind in quads:
                if kind in ('outer', 'crook'):
                    polys.append(proj_point(Q, spot))
    for row in keys:
        head, shaft, fl = ST.arrow_quads(row[0], row[1], row[2])
        polys.append(proj_point(head, spot))
        polys.append(proj_point(shaft, spot))
    return CANVAS_GRID.raster(polys, blur=0.7)


# ---------------------------------------------------------------- sparks
def sparks(arr, cam, t):
    for i, e in events_between(t - 0.32, t, ('hit', 'parry', 'block', 'wall', 'cleared', 'fade')):
        dt = t - e[0]
        kind = e[1]
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            col = COL[att] * 0.6 + 100
            n, sp, life = (16, 3.2, 0.3) if how in ('slam', 'wave', 'shockwave') else \
                ((8, 2.4, 0.22) if how == 'blade' else (5, 1.8, 0.18))
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
        if kind == 'hit' and e[7] in ('blade', 'slam') and dt < 0.08:
            s = cam.wp(S.to_world(x, z, R)[None])[0]
            arr[max(0, int(s[1]) - 1):int(s[1]) + 2, max(0, int(s[0]) - 1):int(s[0]) + 2] = 255


# ---------------------------------------------------------------- v2: THE HUNT (hounds, the mark) + reelkit.fx
def fx_clock(t):
    """effects clock: real time in the fight, held through the hit-stop, then running on"""
    if t < T_KO:
        return t
    if t < T_HS:
        return T_KO
    return t - HITSTOP


MARK_FULL = {k: CFG[k]['weapon'].get('mark_beats', 3) * T.BEAT for k in PACK}


def mark_k(o, t):
    """strength of the copper target mark on fighter o (fades over its last 0.3 s; gone after the K.O.)"""
    if MARK is None or not (T.T_FIGHT <= t < T_HS):
        return 0.0
    m = float(MARK[sim_i(min(t, T_KO)), o])
    return min(1.0, m / 0.3) * (1 - ss(T_KO, T_HS, t))


def brand_k(k, t):
    """the flare of the brand right after each horn call"""
    ct = CALL_T.get(k)
    if ct is None or not len(ct) or not (T.T_FIGHT <= t < T_KO):
        return 0.0
    j = np.searchsorted(ct, t, 'right') - 1
    return math.exp(-(t - ct[j]) / 0.18) if j >= 0 else 0.0


def _ko_hounds(k):
    H_ = HOUNDS[-1, k]
    return [(int(h[4]), float(h[0]), float(h[1]), float(h[2])) for h in H_ if not np.isnan(h[0])]


def hounds_at(k, t):
    """the pack of fighter k at time t: [(id, x, z, heading, mode, alpha, gait)]. In the fight: the run's hounds (frozen
    through the hit-stop). After the K.O.: a losing pack bursts into embers; a winning pack circles its horn through the
    solo, then runs back into the bell before the glide home (no hounds on frame 0)."""
    if HOUNDS is None or t < T.T_FIGHT:
        return []
    if t < T_HS:
        out = []
        for h in HOUNDS[sim_i(min(t, T_KO)), k]:
            if not np.isnan(h[0]):
                mode = int(h[3])
                out.append((int(h[4]), float(h[0]), float(h[1]), float(h[2]), mode, 1.0, 1.7 if mode == 3 else 1.0))
        return out
    if k != WIN or t >= T.T_GLIDE:
        return []
    ko = sorted(_ko_hounds(k))
    n = len(ko)
    c = fighter(k, t)['p']
    u = ease((t - T_HS) / 1.2)
    back = ease((t - T.T_REASSEMBLE) / (T.T_GLIDE - 0.1 - T.T_REASSEMBLE))
    out = []
    for j, (hid, x, z, a) in enumerate(ko):
        ang = (t - T_HS) * 1.7 + 2 * math.pi * j / max(1, n)
        tgt = c + 1.3 * np.array([math.cos(ang), math.sin(ang)])
        p = np.array([x, z]) + (tgt - np.array([x, z])) * u
        p = p + (c - p) * back
        head = a + _wrap(ang + math.pi / 2 - a) * min(1.0, u * 1.5)
        out.append((hid, float(p[0]), float(p[1]), head, 0, max(0.0, 1 - back * 1.1), 1.0))
    return out


def hound_prev(k, hid, t, back):
    for h in hounds_at(k, t - back / FPS):
        if h[0] == hid:
            return h[1], h[2], h[3]
    return None


FL = fx.Flares(max_n=6)
BU = fx.Bursts()
PA = fx.Particles(seed=34)
MK = fx.Marks()
MO = fx.Moments()


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
            if how == 'blade':                                     # the slide lance lands: brass sparks
                BU.add(t_, pos, COL[att], 0.8)
                FL.add(t_, pos, COL[att], radius=1.4, peak=0.6, decay=0.22)
                d = S.to_world(math.cos(fighter(att, t_)['th']), math.sin(fighter(att, t_)['th']), 0.0)
                PA.emit(t_, pos, 'brass', n=9, dir=d + np.array([0, 0.4, 0]))
            elif how == 'bite':                                    # a hound bites: copper embers + bite marks
                BU.add(t_, pos, COL[att], 0.45, bright=False)
                FL.add(t_, pos, COL[att], radius=1.0, peak=0.4, decay=0.2)
                PA.emit(t_, pos, 'embers', n=5)
                MK.add(t_, 'claw', x, z, size=0.3, angle=math.atan2(z - fighter(att, t_)['p'][1], x - fighter(att, t_)['p'][0]),
                       n=2, col=(150, 70, 40), life=3.0, seed=int(t_ * 100), strength=0.7)
            elif how == 'shockwave':
                BU.add(t_, pos, COL[att], 1.0)
                FL.add(t_, pos, COL[att], radius=1.6, peak=0.8, decay=0.3)
                PA.emit(t_, pos, 'brass', n=6)
            elif how == 'charge':                                  # TALLY-HO lands
                BU.add(t_, pos, COL[att], 2.0)
                FL.add(t_, pos, COL[att], radius=2.4, peak=1.2, decay=0.5)
                PA.emit(t_, pos, 'embers', n=26, spread=180, speed=(0.8, 2.2))
                MK.add(t_, 'claw', x, z, size=0.9, n=4, col=(150, 70, 40), life=6.0, seed=int(t_ * 100))
            elif how == 'slam':
                BU.add(t_, pos, COL[att], 2.0)
        elif kind == 'slam':                                      # PEDAL TONE: the bell stomps
            cx, cz = e[3], e[4]
            FL.add(t_, up(cx, cz, 0.15), COL[e[2]], radius=2.4, peak=0.9, decay=0.5)     # low: under the bell's rim
            PA.emit(t_, up(cx, cz, 0.06), 'dust', n=30, spread=180, speed=(0.6, 1.8), scale=1.3)
            PA.emit(t_, up(cx, cz, 0.2), 'brass', n=14, spread=180)
            MK.add(t_, 'crack', cx, cz, size=1.5, n=8, life=7.0, seed=int(t_ * 10))
            MK.add(t_, 'scorch', cx, cz, size=1.2, col=(70, 60, 30), life=7.0, seed=int(t_ * 10) + 1, strength=0.5)
        elif kind == 'hound':                                     # a hound pours out of the bell
            pos = up(e[4], e[5], 0.25)
            PA.emit(t_, pos, 'embers', n=7, speed=(0.6, 1.6))
            FL.add(t_, pos, COL[e[2]], radius=1.0, peak=0.4, decay=0.25)
        elif kind == 'hound_down':                                # knocked out: it bursts into embers
            pos = up(e[4], e[5], 0.25)
            PA.emit(t_, pos, 'embers', n=12, spread=180, speed=(0.8, 2.0))
            BU.add(t_, pos, COL[e[2]], 0.4, bright=False)
            if e[6] == 'blade':
                PA.emit(t_, pos, 'brass', n=5)
        elif kind == 'call':                                      # the call brands the enemy
            k = e[2]
            p_ = fighter(1 - k, t_)['p']
            FL.add(t_, up(p_[0], p_[1], 0.8), COL[k], radius=1.4, peak=0.45, decay=0.3)
            PA.emit(t_, up(p_[0], p_[1], 0.15), 'embers', n=3)
        elif kind == 'pack_charge':
            FL.add(t_, up(e[3], e[4], 1.0), COL[e[2]], radius=2.2, peak=0.9, decay=0.6)
        elif kind == 'block':
            BU.add(t_, up(e[4], e[5], R), (255, 220, 190), 0.6, bright=False)
            PA.emit(t_, up(e[4], e[5], R), 'brass' if e[2] in BLADE else 'embers', n=5)
    for k in PACK:                                                # ember trails behind running hounds
        for f_ in range(0, NSIM, 3):
            t_ = T.T_FIGHT + f_ / FPS
            if t_ > T_KO:
                break
            for h in HOUNDS[f_, k]:
                if not np.isnan(h[0]):
                    d = S.to_world(-math.cos(h[2]), -math.sin(h[2]), 0.0) * (0.6 if int(h[3]) == 3 else 0.3)
                    PA.emit(t_, up(h[0], h[1], 0.22), 'embers', n=1, vel=d, speed=(0.1, 0.4), life=(0.4, 0.8))
        if k == LOSE:                                             # the losing pack bursts at the end of the hit-stop
            for hid, x, z, a in _ko_hounds(k):
                PA.emit(fx_clock(T_HS), up(x, z, 0.25), 'embers', n=14, spread=180, speed=(0.8, 2.2))
        else:                                                     # the winning pack runs back into the bell
            for hid, x, z, a in _ko_hounds(k):
                tb = T.T_GLIDE - 0.25
                c = fighter(k, tb)['p']
                PA.emit(fx_clock(tb), up(c[0], c[1], 0.3), 'embers', n=6, speed=(0.4, 1.2))
    for t_, who, name in STORY['caps']:                           # ability moments on the captioned abilities
        MO.add(t_, who, COL[who])
    pos = S.to_world(P_KO[LOSE][0], P_KO[LOSE][1], R)
    BU.add(T_KO, pos, COL[LOSE], 2.0)
    FL.add(T_KO, pos, COL[LOSE] * 0.9, radius=2.4, peak=1.1, decay=0.6)
    PA.emit(T_KO, pos, 'embers' if NAMES[LOSE] == 'HORN' else 'brass', n=30, spread=180, speed=(0.8, 2.2))


_fx_build()


def shimmers(img, cam, t):
    """heat-shimmer ripples: SAD TROMBONE's shockwave, TALLY-HO's front, PEDAL TONE's stomp"""
    if T.T_FIGHT <= t < T_HS:
        i = sim_i(min(t, T_KO))
        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][i]:
            if kind in ('shockwave', 'wave'):
                img = fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.4, 3.0, t, k=1 - r / rmax, seed=owner)
        for owner, ox, oz, ux, uz, s_c, wd, t0c in CHG[i] if CHG is not None else []:
            fc = np.array([ox, oz]) + np.array([ux, uz]) * s_c
            img = fx.shimmer_world(img, cam, S.to_world(fc[0], fc[1], 0.05), wd * 0.55, 0.3, 2.2, t, k=0.8, seed=7)
    for e in EV:
        if e[1] == 'slam' and 0 <= t - e[0] < 0.6 and t < T_HS:
            u = (t - e[0]) / 0.6
            rad = CFG[e[2]]['abilities']['low']['radius']
            img = fx.shimmer_world(img, cam, S.to_world(e[3], e[4], 0.05), rad + 1.6 * u, 0.35, 3.0, t, k=1 - u)
    return img


def post_fx(img, cam, t, fs, drawn):
    """after the grade: chromatic fringing on big hits, the brand sigil, the ability moments (dim + speed lines), the
    impact bursts"""
    for _, e in events_between(t - 0.12, t, ('hit',)):
        if e[7] in ('slam', 'wave', 'shockwave', 'charge'):
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            img = fx.fringe(img, s_[0], s_[1], 26, 2.5, k=1 - (t - e[0]) / 0.12)
    if T_KO <= t < T_HS + 0.3:
        s_ = cam.wp(S.to_world(P_KO[LOSE][0], P_KO[LOSE][1], R)[None])[0]
        img = fx.fringe(img, s_[0], s_[1], 34, 3.0, k=1 - ss(T_HS, T_HS + 0.3, t))
    for k in PACK:                                                # the brand: a copper crosshair over the marked ball
        o = 1 - k
        mk_ = mark_k(o, t)
        if mk_ > 0.01 and o in drawn:
            s_, rp = drawn[o]
            b = brand_k(k, t)
            img = ST.draw_sigil(img, s_[0], s_[1] - rp - 7 - 2 * b, 4 + 2 * b, COL[k], mk_)
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
    trails = {}                                                     # arrow id -> its last few positions (curving trails)
    if keys:
        i_now = sim_i(min(t, T_KO))
        for back in range(1, 7):
            for row in RUN['K'][max(0, i_now - back)]:
                if not np.isnan(row[0]):
                    trails.setdefault(int(row[4]), []).insert(0, (row[0], row[1]))
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
        for k in PACK:                                                # THE HUNT: the copper target mark on the branded enemy
            o = 1 - k
            mk_ = mark_k(o, t)
            if mk_ > 0.01 and fs[o]['vis']:
                ST.target_decal(Ic, sxp, szp, fs[o]['p'][0], fs[o]['p'][1], t, mk_, COL[k], brand_k(k, t),
                                BAYER_FULL[ys, xs])
            for hid, hx, hz, ha, hmode, ha_, hg in hounds_at(k, t):     # ghosts cast no shadow: a copper glow instead
                rd = np.hypot(sxp - hx, szp - hz)
                Ic += (np.clip(1 - rd / 0.5, 0, 1) ** 2 * 0.35 * ha_)[:, None] * (COL[k] / 255.0 + 0.1)[None]
            if in_fight:
                for owner, ox, oz, ux, uz, s_c, wd, t0c in CHG[i]:
                    ST.charge_band(Ic, sxp, szp, (ox, oz), (ux, uz), s_c, wd, 1.0, COL[owner], BAYER_FULL[ys, xs])
        for k in (0, 1):                                              # SAD TROMBONE: the slowed ball drags a droopy wobble
            st = fs[k]
            if in_fight and st['slow'] > 0:
                sk = min(1.0, st['slow'] / 0.25)
                rd = np.hypot(sxp - st['p'][0], szp - st['p'][1])
                ang_ = np.arctan2(szp - st['p'][1], sxp - st['p'][0])
                for jj, rr in enumerate((R * 1.25, R * 1.6)):
                    wob = rr + 0.07 * np.sin(5 * ang_ - 11 * t + jj * 2.0)
                    band = np.clip(1 - np.abs(rd - wob) / 0.05, 0, 1)
                    Ic += (band * 0.55 * sk * (1 - 0.35 * jj))[:, None] * (COL[1 - k] / 255.0 + 0.15)[None]
        for k in BLADE:                                               # the lance's line of attack, lit as it thrusts
            st = fs[k]
            if in_fight and st['thr_k'] > 0.05:
                d_ = np.array([math.cos(st['th']), math.sin(st['th'])])
                rx, rz = sxp - st['p'][0], szp - st['p'][1]
                along = rx * d_[0] + rz * d_[1]
                across = np.abs(-rx * d_[1] + rz * d_[0])
                lane = np.clip(1 - across / 0.22, 0, 1) * (along > R) * (along < R + st['ext'] + 0.1)
                lane = np.floor(lane * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
                Ic += (lane * 0.30 * st['thr_k'])[:, None] * (COL[k] / 255.0 * 0.8 + 0.2)[None]
        wav = RUN['WAVES'][i] if in_fight else []
        for owner, cx, cz, r, rmax, kind, t0 in wav:
            if kind == 'pulse':
                continue
            rd = np.hypot(sxp - cx, szp - cz)
            wdt = 0.16 if kind == 'shockwave' else 0.32
            k_ = (1 - r / rmax) * (1.0 if kind == 'shockwave' else 1.3)
            if kind == 'wave':                                        # TALLY-HO: a ring of chevrons charging outward
                ang_ = np.arctan2(szp - cz, sxp - cx)
                rd = rd + 0.16 * np.abs(((ang_ * 10 / math.pi) % 2) - 1)
            band = np.clip(1 - np.abs(rd - r) / wdt, 0, 1)
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (band * k_ * 0.9)[:, None] * (COL[owner] / 255.0 * 1.4 + (0.35 if kind == 'wave' else 0.15))[None]
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
        if k in BLADE and not st['shattered']:
            tip = st['p'] + (R + st['ext'] * 0.55) * np.array([math.cos(st['th']), math.sin(st['th'])])
            items.append((cam.depth(S.to_world(tip[0], tip[1], R))[0] + 0.01, 'slide', k))
        if st['shield'] > 0:
            items.append((dz + 0.03, 'rim_back', k))
            items.append((dz - 0.03, 'rim_front', k))
    for row in keys:
        items.append((cam.depth(S.to_world(row[0], row[1], 0.45))[0], 'key', row))
    for k in PACK:
        for hd in hounds_at(k, t):
            items.append((cam.depth(S.to_world(hd[1], hd[2], 0.24))[0], 'hound', (k,) + hd))
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
        elif kind == 'slide':
            st = fs[obj]
            prev = fighter(obj, max(0.0, t - 1.0 / FPS))
            arr, depth = fr.array(), fr.depth()
            if st['ext'] - prev['ext'] > 0.12 and abs(_wrap(st['th'] - prev['th'])) < 0.3:   # a thrust going out: streak
                a_ = 0.55 if st['haste'] > 0 else 0.38
                layer_blend(arr, ST.slide_trail_polys(cam, st, prev['ext']), COL[obj] * 0.6 + 100, a_)
            fr.set_array(arr, depth)
            hk = 0.6 if st['haste'] > 0 else 0.0
            ST.draw_slide(fr, cam, L, shade, st, COL[obj], max(hk, 0.8 * st['thr_k']))
        elif kind in ('rim_back', 'rim_front'):
            st = fs[obj]
            dur = CFG[obj]['abilities']['unlock1']['dur_beats'] * T.BEAT
            k_ = min(1.0, st['shield'] / 0.2, (dur - st['shield']) / 0.12 + 0.2)
            arr, depth = fr.array(), fr.depth()
            ST.draw_bellrim(arr, cam, st['p'][0], st['p'][1], t, k_, 1 if kind == 'rim_front' else 0, COL[obj])
            fr.set_array(arr, depth)
        elif kind == 'hound':
            k, hid, hx, hz, ha, hmode, alpha, gait = obj
            arr, depth = fr.array(), fr.depth()
            for back, a_ in ((3, 0.16),):                            # a ghostly afterimage
                pb = hound_prev(k, hid, t, back)
                if pb is not None:
                    ST.draw_hound(arr, cam, pb[0], pb[1], pb[2], t - back / FPS, COL[k], a_ * alpha, hid, gait)
            ST.draw_hound(arr, cam, hx, hz, ha, t, COL[k], alpha, hid, gait, glow=0.6 if hmode == 3 else 0.0)
            fr.set_array(arr, depth)
        elif kind == 'key':
            row = obj
            tr = trails.get(int(row[4]), [])
            if len(tr) >= 2:
                sp_ = ST.trail_points(cam, tr + [(row[0], row[1])])
                im = Image.fromarray(np.clip(fr.array(), 0, 255).astype(np.uint8))
                dr = ImageDraw.Draw(im)
                for jj in range(len(sp_) - 1):
                    a_ = (jj + 1) / len(sp_)
                    c_ = tuple(int(v) for v in np.clip(COL[int(row[3])] * (0.35 + 0.5 * a_) + 40 * a_, 0, 255))
                    dr.line([tuple(sp_[jj]), tuple(sp_[jj + 1])], fill=c_, width=1)
                fr.set_array(np.asarray(im).astype(np.float32), fr.depth())
            ST.draw_arrow(fr, cam, L, shade, row[0], row[1], row[2], COL[int(row[3])])
    # ---------------- front ropes + posts
    draw_rope_side(0)
    draw_rope_side(1)
    draw_post('front')
    arr, depth = fr.array(), fr.depth()
    # ---------------- shields, slams, shards, sparks
    if in_fight:
        for owner, cx, cz, t0, t_slam, rad in RUN['SLAMS'][i]:
            h = ST.bell_height(t, t0, t_slam)                          # PEDAL TONE: the giant bell drops in and stomps
            if h is not None:
                fr.set_array(arr, depth)
                ST.draw_giant_bell(fr, cam, L, shade, cx, cz, rad, h, COL[owner])
                arr, depth = fr.array(), fr.depth()
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
        elif kind == 'tube':                                      # a curl of brass tubing
            pts = [np.array([sx_, sz_]) + 0.16 * np.array([math.cos(rot + j * 0.5 * (1 + curl)), math.sin(rot + j * 0.5 * (1 + curl))])
                   for j in range(7)]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            ImageDraw.Draw(im).line([tuple(q) for q in sp], fill=tuple(int(v) for v in ST.BRASS), width=2)
            arr = np.asarray(im).astype(np.float32)
        elif kind == 'slidebit':
            q4 = [np.array([sx_, sz_]) + d * 0.22 * a1 + nn * 0.025 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            layer_blend(arr, [sp], ST.BRASS_HI, 1.0)
        else:
            # valve (a rotary valve: a squat disc), bellbit (a curved chip of the bell), glassless brass chips
            col = {'valve': ST.BRASS_DK * 1.3, 'bellbit': ST.COPPER if NAMES[LOSE] == 'HORN' else ST.BRASS}[kind]
            sz = size * (1.5 if kind == 'bellbit' else 0.8)
            tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
                   np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
            shade_k = 0.75 + 0.25 * math.cos(rot * 2)
            layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
    sparks(arr, cam, t)
    tc_ = fx_clock(t)
    arr = PA.draw(arr, cam, tc_, light=lambda P: 1.0 + 0.6 * FL.tint(P, tc_))   # brass sparks + copper embers
    # countdown in the ring (sharp, over everything in the ring)
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
    for k in BLADE:                                                   # the crook flares as the lance strikes
        st = fs[k]
        if st['thr_k'] > 0.2 and not st['shattered'] and st['vis']:
            tp = st['p'] + (R + st['ext'] + 0.07) * np.array([math.cos(st['th']), math.sin(st['th'])])
            s_ = cam.wp(S.to_world(tp[0], tp[1], R)[None])[0]
            fr.glow(s_[0], s_[1], 7, tuple(COL[k] * 0.6 + 90), 0.35 * st['thr_k'])
    for k in PACK:                                                    # a faint copper aura on every ghost hound
        for hd in hounds_at(k, t):
            s_ = cam.wp(S.to_world(hd[1], hd[2], 0.45)[None])[0]
            fr.glow(s_[0], s_[1], 9, tuple(COL[k] * 0.7 + 60), (0.30 if hd[4] == 3 else 0.18) * hd[5])
    for e_i, e in events_between(t - 0.15, t, ('hit',)):
        if e[7] in ('slam', 'wave', 'shockwave', 'blade'):
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            big = e[7] != 'blade'
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
    img = post_fx(img, cam, t, fs, drawn)
    img = np.clip(img, 0, 255).astype(np.uint8)
    img = np.ascontiguousarray(img)
    # ---------------- UI: countdown, HP numbers in the balls, fight HUD, captions
    countdown(img, t, cs)
    card_closed = TT.door_offset(t) == 0.0                            # v3: the title card covers the HUD: skip it
    for k, (s_, rp) in (drawn.items() if not card_closed else ()):
        st = fs[k]
        if st['scale'] < 0.95 or T.T_FIGHT - 0.05 <= t < T.T_FIGHT + 0.85:  # hidden while FIGHT! covers the balls
            continue
        if under_bell(k, t):                                                  # trapped under PEDAL TONE's bell
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
        fight_hud(img, hud[0], hud[1])
    draw_captions(img, t, CAP_TITLE, style='impact', hi=tuple(int(v) for v in COL[0]), alt=tuple(int(v) for v in COL[1]))
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
