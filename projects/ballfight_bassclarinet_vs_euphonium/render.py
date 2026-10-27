"""#38 BALLFIGHT #12: Bass Clarinet vs Euphonium - renderer. The shared ring, crowd, ropes, HUD + ability list,
countdown and VS title card (as #37; BASS / CLARINET on two lines), with the two new fighters' styles from styles.py:
  BASS CLARINET (ENDPIN ANCHOR): every bar a silver peg drives into the canvas and the ball whirls round it on its long
    black body like a hammer-thrower, a slate trail of light sweeping the circle, then it pulls the peg and flies off
    on the tangent; MULTIPHONICS = a ghost twin on the far side of the peg; SLAP TONGUE = the reed slaps out like a
    paddle; LIGHTS OUT = the arena eases darker (never black, never a flash), the ball sinks into a pool of shadow and
    slides under the enemy, then erupts in a cracked ring.
  EUPHONIUM (BELL MORTAR): brass shells lob out of the upright bell on real arcs with smoke trails, a shadow circle
    growing where each will land, an amber splash; MARCHING ORDERS = a salvo every beat; IDENTITY CRISIS = it fires the
    enemy's last ability back in amber, its ball briefly showing a bass clarinet; NOT A TUBA = shells rain across the
    whole ring.
Effects from reelkit.fx (attack lights, bursts, particles, marks, ability moments); no full-screen flash.

    python render.py                                   # sprite cache (check/sprites_ballfight12.npz)
    python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 2.6 16.4 21.6 27.4 33.1 44.2 46.0 47.3 51.8 55.0 60.8
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
ANCHOR = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'anchor']    # ENDPIN ANCHOR
MORTAR = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'mortar']    # BELL MORTAR
CORNER0 = [np.array(c) for c in RUN['corner']]
STORY = sim.story(RUN)
SPLASH = {'mortar': CFG[MORTAR[0]]['weapon']['splash'], 'barrage': CFG[MORTAR[0]]['abilities']['low']['splash']}

# ---------------------------------------------------------------- the fighters' own colours on the shared ring (series
# rule: every instrument has its own colour; the corners are just left / right). Runtime patches of the imported scene.
S.CORNER_COL['red'] = np.clip(COL[0] * 0.9, 0, 255)
S.CORNER_COL['blue'] = np.clip(COL[1] * 0.9, 0, 255)
S.POSTS = {k: S.post_faces(k) for k in S.CORNERS}
S.SKIRT_BAND = np.array([150, 140, 184])
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


def _wrap(a):
    return (a + math.pi) % (2 * math.pi) - math.pi


TH0 = [float(RUN['TH'][0, k]) for k in (0, 1)]
P_KO = [RUN['P'][-1, k].copy() for k in (0, 1)]

# ---------------------------------------------------------------- the run's events, by kind
ANCH = [(e[0], e[2], np.array([e[3], e[4]]), e[5], e[6]) for e in EV if e[1] == 'anchor']        # t, k, peg, orbit, t_end
PULLS = [(e[0], e[2], np.array([e[3], e[4]]), np.array([e[5], e[6]])) for e in EV if e[1] == 'pull']
COPIES = [(e[0], e[2], e[3], e[4]) for e in EV if e[1] == 'copy']                                   # t, k, name, kind
LOUT = [(e[0], e[2], e[3]) for e in EV if e[1] == 'lights_out']                                     # t, k, t_end
ERUPTS = [(e[0], e[2], np.array([e[3], e[4]]), e[5]) for e in EV if e[1] == 'erupt']                # t, k, p, hit
TWINS = [(e[0], e[2], e[3]) for e in EV if e[1] == 'twin']                                          # t, k, until
SALVOS = [(e[0], e[2], e[3], e[4]) for e in EV if e[1] == 'mortar']                                 # t, k, n, salvo
LANDS = [(e[0], e[2], e[3], e[4], e[5], e[6], e[7], e[8]) for e in EV if e[1] == 'shell']           # t, k, x, z, hit, salvo, j, kind
LAND_T = np.array([q[0] for q in LANDS])
BARRAGE_SALVOS = {e[4] for e in EV if e[1] == 'barrage'}
SNIPS = [(e[0], e[2], e[3], e[4]) for e in EV if e[1] == 'snip']                                    # t, k, hit, aim
SNIP_OPEN = [(e[0], e[2], e[3], e[4]) for e in EV if e[1] == 'snip_open']                           # t, k, t_snap, aim
SLAP_AB = CFG[ANCHOR[0]]['abilities']['unlock1']                     # SLAP TONGUE (the euphonium's copy uses the same)


def sink_at(k, t):
    """LIGHTS OUT: how far fighter k has sunk into shadow (0..1): eased down over 0.3 s, popping up at the eruption"""
    for t0, kk, te in LOUT:
        if kk == k and t0 <= t < te + 0.16 and t0 < T_KO:
            if t < te:
                return ss(t0, t0 + 0.3, t)
            return 1 - ss(te, te + 0.12, t)
    return 0.0


def dim_at(t):
    """LIGHTS OUT's arena dim: eased in over 0.35 s, held, eased back over 0.35 s after the eruption (<= 0.40)"""
    d = 0.0
    for t0, kk, te in LOUT:
        if t0 < T_KO and t0 <= t < te + 0.4:
            d = max(d, 0.40 * ss(t0, t0 + 0.35, t) * (1 - ss(te + 0.05, te + 0.4, t)))
    return d


def crisis_at(k, t):
    """IDENTITY CRISIS: for a bar after a copy the euphonium's ball shows the enemy's instrument, tinted (0..1)"""
    for t0, kk, name, kind in COPIES:
        if kk == k and t0 <= t < t0 + T.BAR and t0 < T_KO:
            return min(1.0, (t - t0) / 0.08) * (1 - ss(t0 + T.BAR - 0.3, t0 + T.BAR, t))
    return 0.0


def twin_at(k, t):
    for t0, kk, until in TWINS:
        if kk == k and t0 <= t < until and t0 < T_KO:
            return min(1.0, (t - t0) / 0.15) * (1 - ss(until - 0.15, until, t))
    return 0.0


def anc_at(k, t):
    """the whirl at t: (peg (x, z), th, r, twin, age, om) or None (from the recorded ANC)"""
    if not (T.T_FIGHT <= t < T_KO) or 'ANC' not in RUN:
        return None
    row = RUN['ANC'][sim_i(t), k]
    if np.isnan(row[0]):
        return None
    t0 = max([a[0] for a in ANCH if a[1] == k and a[0] <= t] + [T.T_FIGHT])
    i0 = sim_i(t)
    th_prev = RUN['ANC'][max(0, i0 - 1), k][3]
    om = (row[3] - th_prev) * FPS if not np.isnan(th_prev) else 1.0
    return np.array([row[1], row[2]]), float(row[3]), float(row[4]), float(row[5]), t - t0, om


def snip_state(k, t):
    """(aim, ext, wind-up, slap flash) of SLAP TONGUE's paddle at t, or None"""
    for t0, kk, t_snap, aim in SNIP_OPEN:
        if kk == k and t0 <= t < t_snap + 0.22 and t0 < T_KO:
            i = sim_i(min(t, T_KO))
            row = RUN['SNIP'][i, k] if 'SNIP' in RUN else None
            a = float(row[1]) if row is not None and not np.isnan(row[1]) else aim
            if t < t_snap:
                u = (t - t0) / max(1e-6, t_snap - t0)
                return a, 0.15 + 0.1 * u, u, 0.0
            u = (t - t_snap) / 0.22
            ext = SLAP_AB['reach'] * (math.sin(math.pi * min(1.0, u * 1.6)) if u < 0.6 else (1 - u) / 0.4 * 0.6)
            return a, max(0.12, ext), 0.0, max(0.0, 1 - u / 0.4)
    return None


def fighter(k, t):
    """picture state of fighter k at time t: dict(p (ring frame), vis, th, scale, shield, haste, glow, sink, crisis, ...)"""
    st = dict(vis=True, shield=0.0, haste=0.0, glow=0.0, scale=1.0, th=TH0[k], level=1, shattered=False, sink=0.0,
              crisis=0.0, twin=0.0, anc=None)
    if t < T.T_FIGHT:
        st['p'] = CORNER0[k] + rev_offset(k, t)
        st['glow'] = 0.25 * T.beat_pulse(t)
        return st
    if t < T_HS:
        i = sim_i(min(t, T_KO))
        st['p'] = RUN['P'][i, k].copy()
        st['th'] = float(RUN['TH'][i, k])
        st['shield'] = float(RUN['SH'][i, k])
        st['haste'] = float(RUN['HA'][i, k])
        st['level'] = int(RUN['LV'][i, k])
        st['sink'] = sink_at(k, t)
        st['crisis'] = crisis_at(k, t)
        st['twin'] = twin_at(k, t)
        st['anc'] = anc_at(k, t)
        if st['anc'] is not None:
            st['glow'] = max(st['glow'], 0.35 * max(0.0, 1 - st['anc'][4] / 0.4))
        if st['crisis'] > 0:
            st['glow'] = max(st['glow'], 0.5 * st['crisis'])
        if t >= T_KO and k == LOSE:
            st['glow'] = 1.0
        return st
    gl = ease((t - T.T_GLIDE) / (T.T_REV - T.T_GLIDE))
    corner = CORNER0[k] + rev_offset(k, t) * ss(T.T_REV - 0.3, T.T_REV + 0.2, t)
    rev_glow = 0.25 * T.beat_pulse(t) * ss(T.T_REV, T.T_REV + 0.3, t)
    th_end = float(RUN['TH'][-1, k])
    st['th'] = th_end + _wrap(TH0[k] - th_end) * gl
    if k == WIN:
        centre = np.array([0.0, 0.0])
        u = ease((t - T_HS) / 1.8)
        bob = 0.06 * math.sin(2 * math.pi * (t - T_HS) / (2 * T.BAR)) * ss(T_HS, T_HS + 1.5, t)
        p = P_KO[k] + (centre - P_KO[k]) * u + np.array([bob, -bob])
        st['p'] = p + (corner - p) * gl
        st['glow'] = max(rev_glow, (0.35 + 0.5 * solo_env(t)) * ss(T_HS, T_HS + 0.6, t) * (1 - gl))
        st['level'] = 3 if t < T.T_GLIDE else 1
        return st
    # the loser: shattered until the reassembly, then glides home
    st['p'] = P_KO[k] + (corner - P_KO[k]) * gl
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
    if e[0] > T_KO:
        break
    if e[1] == 'hit' and e[7] in ('anchor', 'snip', 'shell', 'erupt'):
        SHK.add(e[0], {'anchor': 1.1 if e[8] == 0 else 0.7, 'snip': 1.3, 'shell': 0.5, 'erupt': 2.4}[e[7]])
    elif e[1] == 'anchor':
        SHK.add(e[0], 0.5)                                            # the peg going in
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
WAVE_EV = [(e[0], 1.0) for e in ERUPTS] + [(e[0], 0.5) for e in EV if e[1] == 'ability' and e[4] == 'barrage']


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
WOOD = np.array([34, 26, 30], float)
NICKEL = np.array([214, 216, 226], float)


def _shards():
    rng = np.random.default_rng(381)
    p0 = P_KO[LOSE]
    parts = []
    kinds = ['glass'] * 24 + (['wood'] * 6 + ['key'] * 8 + ['neck'] * 2 + ['bell'] * 2 if NAMES[LOSE] == 'BASS CLARINET'
                              else ['brass'] * 8 + ['valve'] * 4 + ['bell'] * 2)
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
SPR_PATH = os.path.join(HERE, 'check', 'sprites_ballfight12.npz')


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
        yc, sw, roll0 = CFG[k]['view']
        rolls = [roll0]
        if k in MORTAR:                                       # IDENTITY CRISIS: the enemy's instrument in this ball
            yc2, sw2, r2 = CFG[1 - k]['view']
            for r in range(12, 31):
                for yk in range(int((yc - sw - 6) // 6) * 6, int(yc + sw + 12), 6):
                    out[f'{CFG[1 - k]["pp"]}_{r}_{yk % 360}_{int(r2)}'] = SP.render_sprite(CFG[1 - k]['pp'], r, float(yk), float(r2))
        for roll in rolls:
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


# ---------------------------------------------------------------- glass balls
def draw_ball(arr, dep, cam, k, st, t):
    sink = st['sink']
    pw = S.to_world(st['p'][0], st['p'][1], R * (1 - 0.75 * sink))      # LIGHTS OUT: sinks into the canvas
    Pc = cam.to_cam(pw[None])[0]
    s = cam.proj(Pc[None])[0]
    rp = R * cam.f / Pc[2] * st['scale'] * (1 - 0.35 * sink)
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
    dark = 1 - 0.72 * sink                                                   # in shadow: the glass goes dark
    nz = np.sqrt(np.clip(1 - d2, 0, 1))
    lam = np.clip(-0.25 * dx - 0.55 * dy + nz * 0.6, 0, 1)                   # lit from the spot above (top of the ball)
    band = np.floor(lam * 4 + BAYER_FULL[:n, :n] * 0.9) / 4
    glass = np.array([236, 232, 226])
    inner = (col * 0.30 + glass * 0.70)[None, None] * (0.70 + 0.36 * band[..., None]) * dark
    m = inside[sl]
    reg[m] = reg[m] * 0.12 + inner[sl][m] * 0.88
    # the instrument (IDENTITY CRISIS: for a bar it shows the enemy's instrument, tinted in its own colour)
    cr = st['crisis']
    who = (1 - k) if cr > 0.5 else k
    name = CFG[who]['pp']
    yc, sw, roll = CFG[who]['view']
    yaw = CFG[k]['view'][0] + CFG[k]['view'][1] * math.sin(1.25 * st['p'][0] + 0.45 * st['p'][1])
    spr = sprite(name, rp * 0.80, yaw, roll)
    sn = spr.shape[0]
    sx0, sy0 = int(round(s[0])) - sn // 2, int(round(s[1])) - sn // 2
    ax0, ay0 = max(0, sx0), max(0, sy0)
    ax1, ay1 = min(W, sx0 + sn), min(H, sy0 + sn)
    if ax1 > ax0 and ay1 > ay0:
        sub = spr[ay0 - sy0:ay1 - sy0, ax0 - sx0:ax1 - sx0]
        ms = sub[..., 3] > 0
        rg = arr[ay0:ay1, ax0:ax1]
        lit = sub[..., :3].astype(np.float32) * (1.06 + 0.3 * glow) * dark
        if cr > 0.5:                                                         # an amber bass clarinet: who am I?
            lit = lit * 0.55 + col[None, None] * 0.55
        rg[ms] = np.clip(lit[ms], 0, 255)
    rc = np.clip((col * (0.9 + 0.5 * glow) + 50 * glow) * (1 - 0.5 * sink), 0, 255)
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
        abil[who].append((t, t + 2.4, [name]))
    ko = [(T_KO, T_KO + 1.25, ['*K.O.*'])]
    wins = [(T_KO + 1.3, T_KO + 3.55, [NAMES[WIN], 'WINS!'])]
    comment = [(T_KO + 3.65, T.T_REASSEMBLE, ['COMMENT THE', '*NEXT MATCH-UP*'])]
    return title, abil, ko, wins, comment


CAP_TITLE, CAP_ABIL, CAP_KO, CAP_WINS, CAP_COMMENT = _caps()
CAPTIONS = CAP_TITLE + CAP_ABIL[0] + CAP_ABIL[1] + CAP_KO + CAP_WINS + CAP_COMMENT     # for audio (impact thuds)


# ---------------------------------------------------------------- the HUD ability list (series rule, Oct 25): LV2 / LV3 / LOW
SLOTS = (('unlock1', 'LV2'), ('unlock2', 'LV3'), ('low', 'LOW'))
SWEEP_T = 0.45                                       # s the glow takes to sweep across a name as it unlocks


def _hold(k, slot):
    """how long a use stays lit: a haste / MULTIPHONICS for as long as it lasts, LIGHTS OUT until it erupts, NOT A TUBA
    for its rain, SLAP TONGUE until it slaps, IDENTITY CRISIS for its bar, else a flash"""
    ab = CFG[k]['abilities'][slot]
    if ab['kind'] in ('shield', 'haste', 'twin', 'lights_out'):
        return ab['dur_beats'] * T.BEAT if 'dur_beats' in ab else ab['dur']
    if ab['kind'] == 'barrage':
        return ab['flight_beats'] * T.BEAT + ab['beats'] * T.BEAT
    if ab['kind'] == 'snip':
        return ab['open16'] * T.BEAT / 4
    if ab['kind'] == 'copy':
        return T.BAR
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
FLASH_EV = [e[0] for e in EV if (e[1] == 'hit' and e[7] in ('erupt', 'snip', 'anchor')) or e[1] == 'ability'] + \
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


def canvas_shadows(cam, t, fs, spot):
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
        if not st['vis'] or st['scale'] < 0.3:
            continue
        a = np.linspace(0, 2 * math.pi, 13)[:-1]
        pw = S.to_world(st['p'][0], st['p'][1], R)
        circ = np.array([[pw[0] + R * st['scale'] * math.cos(u), R, pw[2] + R * st['scale'] * math.sin(u)] for u in a])
        polys.append(proj_point(circ, spot))
    return CANVAS_GRID.raster(polys, blur=0.7)


# ---------------------------------------------------------------- sparks
def sparks(arr, cam, t):
    for i, e in events_between(t - 0.32, t, ('hit', 'block')):
        dt = t - e[0]
        kind = e[1]
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            col = COL[att] * 0.6 + 100
            n, sp, life = (16, 3.2, 0.3) if how in ('erupt', 'snip') else ((10, 2.6, 0.24) if how == 'anchor' else (7, 2.2, 0.22))
        else:
            att, x, z = e[2], e[4], e[5]
            col, n, sp, life = np.array([160, 220, 255]), 12, 2.6, 0.3
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
PA = fx.Particles(seed=38)
MK = fx.Marks()
MO = fx.Moments()
SLATE_SPARK = [(196, 210, 236), (110, 132, 170), (240, 244, 255)]
BRASS_SPARK = [(255, 214, 120), (240, 172, 32), (255, 246, 210)]
SMOKE_C = [(150, 144, 136), (190, 182, 170), (120, 116, 112)]
CHIPS = [(40, 34, 38), (214, 216, 226), (24, 20, 24)]


def _ball_top(k, t_):
    p = RUN['P'][sim_i(t_), k]
    return S.to_world(p[0], p[1], R * 1.7)


def _fx_build():
    """every effect from the run's events, once, in time order (deterministic: frames render in any order)"""
    up = lambda x, z, y: S.to_world(x, z, y)
    for t_, k, c, orbit, te in ANCH:                          # ENDPIN ANCHOR: the peg goes in
        if t_ > T_KO:
            break
        pos = up(c[0], c[1], 0.05)
        FL.add(t_, up(c[0], c[1], 0.3), COL[k], radius=1.8, peak=0.5, decay=0.3)
        PA.emit(t_, pos, 'dust', n=8, col=[(170, 160, 190), (130, 122, 150)])
        MK.add(t_, 'crack', c[0], c[1], size=0.32, angle=t_ * 3.0, col=(56, 48, 72), life=3.5, n=3, seed=int(t_ * 10),
               strength=0.7)
    for t_, k, n, salvo in SALVOS:                            # BELL MORTAR: smoke out of the bell
        if t_ > T_KO:
            break
        PA.emit(t_, _ball_top(k, t_), 'smoke', n=3 + min(4, n), col=SMOKE_C, scale=0.8)
        FL.add(t_, _ball_top(k, t_), COL[k], radius=1.2, peak=0.35, decay=0.2)
    for t_, k, x, z, hit, salvo, j, kind in LANDS:            # the shells land
        if t_ > T_KO:
            break
        pos = up(x, z, 0.08)
        PA.emit(t_, pos, 'dust', n=2 if kind == 'barrage' else 3, col=[(196, 178, 150), (150, 132, 110)])
        if hit:                                               # only a hit scorches the canvas (misses just kick dust)
            MK.add(t_, 'scorch', x, z, size=0.30, col=(44, 34, 30), life=2.5, n=1, seed=int(t_ * 100) + j,
                   strength=0.5)
            FL.add(t_, up(x, z, 0.4), COL[k], radius=1.6, peak=0.7, decay=0.25)
            PA.emit(t_, up(x, z, 0.3), 'sparks', n=8, col=BRASS_SPARK)
        elif j == 0:
            FL.add(t_, up(x, z, 0.3), COL[k], radius=1.2, peak=0.35, decay=0.2)
    for t_, k, p, hit in ERUPTS:                              # LIGHTS OUT erupts
        if t_ > T_KO:
            break
        pos = up(p[0], p[1], 0.2)
        BU.add(t_, pos, COL[k] * 0.6 + 100, 1.8)
        FL.add(t_, pos, COL[k], radius=2.6, peak=1.2, decay=0.45)
        PA.emit(t_, pos, 'dust', n=22, spread=180, col=[(170, 160, 190), (110, 100, 130)], speed=(1.5, 3.2))
        PA.emit(t_, pos, 'chips', n=10, col=[(60, 52, 76), (110, 132, 170)])
        MK.add(t_, 'crack', p[0], p[1], size=1.2, angle=0.4, col=(40, 34, 56), life=6.0, n=6, seed=int(t_ * 10),
               strength=0.85)
    for t_, k, name, kind in COPIES:                          # IDENTITY CRISIS: an amber flare, question-mark motes
        if t_ > T_KO:
            break
        FL.add(t_, _ball_top(k, t_), COL[k], radius=1.8, peak=0.7, decay=0.35)
        PA.emit(t_, _ball_top(k, t_), 'motes', n=10, col=BRASS_SPARK + SLATE_SPARK)
    for e in EV:
        t_ = e[0]
        if t_ > T_KO + 1e-6:
            break
        kind = e[1]
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = up(x, z, R)
            if how == 'anchor':                                    # the whirling body smashes in
                BU.add(t_, pos, COL[att] * 0.6 + 100, 1.1 if e[8] == 0 else 0.8)
                FL.add(t_, pos, COL[att], radius=1.6, peak=0.7, decay=0.25)
                PA.emit(t_, pos, 'sparks', n=10, col=SLATE_SPARK)
            elif how == 'snip':                                    # SLAP TONGUE: a cane smack
                BU.add(t_, pos, (240, 220, 160), 1.3)
                FL.add(t_, pos, COL[att], radius=1.8, peak=0.9, decay=0.3)
                PA.emit(t_, pos, 'chips', n=10, col=[(232, 210, 152), (196, 168, 110)])
            elif how == 'shell':
                BU.add(t_, up(x, z, R * 0.6), COL[att], 0.8)
            elif how == 'erupt':
                BU.add(t_, pos, (240, 236, 255), 1.4)
        elif kind == 'block':
            BU.add(t_, up(e[4], e[5], R), (255, 226, 150), 0.6, bright=False)
    for t_, who, name in STORY['caps']:                           # ability moments on the captioned abilities
        MO.add(t_, who, COL[who])
    pos = S.to_world(P_KO[LOSE][0], P_KO[LOSE][1], R)
    BU.add(T_KO, pos, COL[LOSE], 2.0)
    FL.add(T_KO, pos, COL[LOSE] * 0.9, radius=2.4, peak=1.1, decay=0.6)
    PA.emit(T_KO, pos, 'sparks', n=26, spread=180, speed=(0.8, 2.2),
            col=[(255, 214, 120), (240, 172, 32), (214, 216, 226)] if NAMES[LOSE] == 'EUPHONIUM' else CHIPS)


_fx_build()


def shimmers(img, cam, t):
    """heat-shimmer ripples: the eruption's ring, the barrage's landings"""
    for t_, k, p, hit in ERUPTS:
        u = t - t_
        if 0 <= u < 0.5 and t_ < T_KO:
            img = fx.shimmer_world(img, cam, S.to_world(p[0], p[1], 0.05), 0.4 + 2.2 * u / 0.5, 0.4, 3.0, t,
                                   k=1 - u / 0.5, seed=k)
    return img


def lights_out_dim(img, t, drawn):
    """LIGHTS OUT: the whole arena eases darker (<= 40 %), both balls stay readable (soft keep circles)"""
    d = dim_at(t)
    if d <= 0:
        return img
    keep = np.zeros((H, W), np.float32)
    for k, (s_, rp) in drawn.items():
        r = rp * 1.8
        x0, x1 = int(max(0, s_[0] - r)), int(min(W, s_[0] + r + 1))
        y0, y1 = int(max(0, s_[1] - r)), int(min(H, s_[1] + r + 1))
        if x1 > x0 and y1 > y0:
            dd = np.hypot(XX_[y0:y1, x0:x1] - s_[0], YY_[y0:y1, x0:x1] - s_[1]) / r
            keep[y0:y1, x0:x1] = np.maximum(keep[y0:y1, x0:x1], np.clip(1.4 - dd * 1.4, 0, 1))
    return img * (1 - d * (1 - keep))[..., None]


def post_fx(img, cam, t, fs, drawn):
    """after the grade: chromatic fringing on big hits, the ability moments (dim + speed lines), the impact bursts"""
    for _, e in events_between(t - 0.12, t, ('hit',)):
        if e[7] in ('erupt', 'snip') or (e[7] == 'anchor' and e[8] == 0):
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
        v = (s_[0] - s1[0], s_[1] - s1[1])
        img = MO.speed_lines(img, t, k, s_[0], s_[1], rp, v)
    img = BU.draw(img, cam, t)
    return img


# ---------------------------------------------------------------- the weapons on screen
def live_shells(t):
    """the shells showing at t (in flight or landed < 0.6 s ago), from the recorded SHELLS"""
    if not (T.T_FIGHT <= t < T_HS) or 'SHELLS' not in RUN:
        return []
    return RUN['SHELLS'][sim_i(min(t, T_KO))]


def recent_lands(t, span=0.4):
    i0, i1 = np.searchsorted(LAND_T, t - span), np.searchsorted(LAND_T, t, 'right')
    return [LANDS[i] for i in range(i0, i1) if LANDS[i][0] <= T_KO]


# ---------------------------------------------------------------- frame
def render(f):
    t = f / FPS
    cam = camera(t)
    L = lights(t, cam)
    fs = [fighter(k, t) for k in (0, 1)]
    i = sim_i(t)
    in_fight = T.T_FIGHT <= t < T_HS
    spot = S.SPOT
    aim = spot_aim(t)
    axis = (aim - spot) / np.linalg.norm(aim - spot)
    wk = win_k(t)
    shells = live_shells(t)
    lands = recent_lands(t) if in_fight else []

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
        sh = CANVAS_GRID.sample(canvas_shadows(cam, t, fs, spot), Xp, Zp)
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
        # decals: the whirl's swept trail + the peg's dent, the shells' growing shadows + splash rings, LIGHTS OUT's pool
        # and eruption
        shade_c = np.zeros(len(xs))
        for k in (0, 1):
            st = fs[k]
            if in_fight and st['anc'] is not None:
                c, th, r, tw, age, om = st['anc']
                w_ = ST.whirl_light(sxp, szp, c, r, th, om, R, age)
                if tw > 0.5:
                    w_ = w_ + 0.6 * ST.whirl_light(sxp, szp, c, r, th + math.pi, om, R, age)
                w_ = np.floor(w_ * 4 + BAYER_FULL[ys, xs] * 0.9) / 4
                Ic += (w_ * 0.9)[:, None] * (COL[k] / 255.0 * 1.2 + 0.25)[None]
                dent, ring = ST.peg_light(sxp, szp, c, age)
                shade_c = np.maximum(shade_c, dent * 0.6)
                Ic += (ring * 0.8)[:, None] * (COL[k] / 255.0 + 0.3)[None]
            if in_fight and st['sink'] > 0.02:
                shade_c = np.maximum(shade_c, ST.shadow_pool(sxp, szp, st['p'], R, st['sink']) * 0.75)
        for sh in shells:
            kind = 'barrage' if sh[7] in BARRAGE_SALVOS else 'mortar'
            sd, ring = ST.target_light(sxp, szp, sh, t, SPLASH[kind])
            if sd is not None:
                shade_c = np.maximum(shade_c, sd * 0.7)
                ring = np.floor(ring * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
                Ic += (ring * 0.9)[:, None] * (COL[sh[0]] / 255.0 * 1.2 + 0.2)[None]
        for t_, k, x, z, hit, salvo, j, kind in lands:
            sp_ = ST.splash_light(sxp, szp, x, z, t - t_, SPLASH[kind])
            if sp_ is not None:
                Ic += (sp_ * (1.1 if hit else 0.6))[:, None] * (COL[k] / 255.0 * 1.2 + 0.25)[None]
        for t_, k, p, hit in ERUPTS:
            if in_fight and t_ <= t < t_ + 0.6:
                e_ = ST.erupt_light(sxp, szp, p, t - t_, CFG[k]['abilities']['low']['radius'] if k in ANCHOR
                                    else CFG[1 - k]['abilities']['low']['radius'])
                e_ = np.floor(e_ * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
                Ic += (e_ * 1.2)[:, None] * (COL[k] / 255.0 * 1.2 + 0.3)[None]
        Ic *= (1 - shade_c)[:, None]
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
    # ---------------- fighters + posts (painter by depth)
    items = []
    for name in ('red', 'blue'):
        items.append((cam.depth(S.to_world(*S.CORNERS[name], 0.7))[0], 'post', name))
    for k, st in enumerate(fs):
        if not st['vis'] or st['scale'] < 0.05:
            continue
        pw = S.to_world(st['p'][0], st['p'][1], R)
        dz = cam.depth(pw)[0]
        items.append((dz, 'ball', k))
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
    # ---------------- front ropes + posts
    draw_rope_side(0)
    draw_rope_side(1)
    draw_post('front')
    arr, depth = fr.array(), fr.depth()
    # ---------------- the endpins + twin ghosts, the slap paddles, the shells in the air, shards, sparks (over the ring,
    # under the UI)
    for k in (0, 1):
        st = fs[k]
        if in_fight and st['anc'] is not None and st['vis']:
            c, th, r, tw, age, om = st['anc']
            k_in = min(1.0, age / 0.06)
            if tw > 0.5:                                              # MULTIPHONICS: the twin, opposite the peg
                q = c + r * np.array([math.cos(th + math.pi), math.sin(th + math.pi)])
                arr = ST.draw_ghost_ball(arr, cam, q, R, COL[k], st['twin'] * k_in)
                arr = ST.draw_arm(arr, cam, c, q, R, COL[k], ghost=True, k=k_in * st['twin'])
            arr = ST.draw_arm(arr, cam, c, st['p'], R, COL[k], k=k_in)
        sn = snip_state(k, t) if in_fight else None
        if sn is not None and st['vis']:
            a_, ext, wind, flash = sn
            arr = ST.draw_slap(arr, cam, st['p'], a_, R, ext, wind, flash, COL[k])
    for sh in shells:
        arr = ST.draw_shell(arr, cam, sh, min(t, T_KO), R, COL[sh[0]])
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
        elif kind in ('wood', 'brass', 'neck'):                    # a stick of the body / a length of tubing / the neck
            ln_, wd_ = {'wood': (0.30, 0.04), 'brass': (0.22, 0.045), 'neck': (0.16, 0.03)}[kind]
            q4 = [np.array([sx_, sz_]) + d * ln_ * a1 + nn * wd_ * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            layer_blend(arr, [sp], {'wood': WOOD, 'brass': ST.BRASS, 'neck': NICKEL}[kind], 1.0)
        else:                                                      # key / valve (a chip), bell (a big curved chip)
            col = {'key': NICKEL, 'valve': ST.BRASS_HI, 'bell': NICKEL if NAMES[LOSE] == 'BASS CLARINET' else ST.BRASS}[kind]
            sz = size * (0.7 if kind in ('key', 'valve') else 1.6)
            tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
                   np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
            shade_k = 0.75 + 0.25 * math.cos(rot * 2)
            layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
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
    for e_i, e in events_between(t - 0.15, t, ('hit',)):
        if e[7] in ('erupt', 'snip', 'anchor', 'shell'):
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            big = e[7] in ('erupt', 'snip')
            fr.glow(s_[0], s_[1], 14 if big else 8, tuple(COL[e[2]] * 0.7 + 70), (0.35 if big else 0.2) * (1 - (t - e[0]) / 0.15))
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
    img = lights_out_dim(img, t, drawn)                              # LIGHTS OUT: eased, never black, never a flash
    img = post_fx(img, cam, t, fs, drawn)
    img = np.clip(img, 0, 255).astype(np.uint8)
    img = np.ascontiguousarray(img)
    # ---------------- UI: countdown, HP numbers in the balls, fight HUD, captions
    countdown(img, t, cs)
    card_closed = TT.door_offset(t) == 0.0                            # the title card covers the HUD: skip it
    for k, (s_, rp) in (drawn.items() if not card_closed else ()):
        st = fs[k]
        if st['scale'] < 0.95 or st['sink'] > 0.3 or T.T_FIGHT - 0.05 <= t < T.T_FIGHT + 0.85:  # hidden: FIGHT! / sunk
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
            if e[3] == k:
                fl = max(fl, 1 - (t - e[0]) / 0.12)
        lvl = fs[k]['level'] if T.T_FIGHT <= t else 1
        hud.append(dict(name=NAMES[k], color=COL[k], hp=hp_at(k, t), trail=max(hp_at(k, t), hp_lag[k]), level=lvl,
                        status='', status_k=0.0, flash=fl, ko=(k == LOSE and T_KO <= t < T.T_GLIDE), t=t,
                        win=(k == WIN and T_KO + 1.3 <= t < T.T_GLIDE)))
    if not card_closed:
        fight_hud(img, hud[0], hud[1], abilities=(ability_rows(0, t), ability_rows(1, t)))
    draw_captions(img, t, CAP_TITLE, style='impact', hi=tuple(int(v) for v in COL[0]), alt=tuple(int(v) for v in COL[1]))
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
