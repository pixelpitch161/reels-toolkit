"""#39 BALLFIGHT #13: Bassoon vs Bari Sax - renderer. The shared ring, crowd, ropes, HUD + ability list, countdown and
VS title card (as #36 / #37), with the two new fighters' styles from styles.py:
  BASSOON (TIPTOE): the ball stands still between exaggerated staccato hops: it crouches, springs up in a tall arc
    (stretching as it rises and falls), lands flat (squash) with a stomp ring and a puff of maple dust, tiptoe prints
    fading on the canvas; a backstab adds crossed green slashes and a bigger double ring. BOCAL HOOK = the silver crook
    swinging out and hooking the enemy, which whips round (swirl); STACCATO = hops twice as fast; MOUNTAIN KING = the
    accelerating dance round the enemy, its landing spots lit on the canvas.
  BARI SAX v2 (WRECKING BALL, Oct 26): the ball wears a leather harness; every other bar a crane line drops from the
    spot's rig, its hook block stops over the swing's bottom, the hook bites the harness ring, the ball is hoisted back
    and swings through in a pendulum arc (motion smear + a coral trail, a light at the bottom), smashing what's in front
    of it (burst, dust, a skid mark, shake), then unhooks; the block flies back up. GROWL = a gravelly ring + grit; SKANK
    = a swing every bar on the off-beats; CABLE SNAP = hoisted high over the enemy's spot (its shadow and a pulsing ring
    on the canvas), the cable snaps (sparks, the line whips away), the ball falls and craters the canvas (the mark
    fades). v1's WINDOW RATTLER code stays (it draws nothing without the 'rattle' kind). v3 (Oct 27, as #40 v2): the
    swing may carry it out over the ropes (they are pressed down under it and spring back when it lifts off; drawn after
    the front ropes when over them), and a far end outside drops onto the ropes and is slung back in.
Effects from reelkit.fx (attack lights, bursts, particles, marks, ability moments); no full-screen flash.

    python render.py                                   # sprite cache (check/sprites_ballfight13.npz)
    python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 2.4 17.5 21.8 27.5 31.8 44.6 46.0 51.7 54.5 60.8
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
HOPPER = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'tiptoe']   # TIPTOE
RATTLER = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'rattle']   # WINDOW RATTLER (v1)
WRECKER = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'wrecking']  # WRECKING BALL (v2)
SNAPPER = [k for k in (0, 1) if any(a['kind'] == 'snap' for a in CFG[k]['abilities'].values())]   # CABLE SNAP
CORNER0 = [np.array(c) for c in RUN['corner']]
STORY = sim.story(RUN)

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
HOPS = [(e[0], e[2], np.array([e[3], e[4]]), np.array([e[5], e[6]]), e[7], e[8]) for e in EV if e[1] == 'hop']
HOP_T = [np.array([h[0] for h in HOPS if h[1] == k]) for k in (0, 1)]
HOP_K = [[h for h in HOPS if h[1] == k] for k in (0, 1)]
STOMPS = [(e[0], e[2], e[3], e[4], e[5], e[6], e[7], e[8]) for e in EV if e[1] == 'stomp']   # t k x z hit back r kind
STOMP_T = [np.array([q[0] for q in STOMPS if q[1] == k]) for k in (0, 1)]
HOOK_OPEN = [(e[0], e[2], e[3], e[4]) for e in EV if e[1] == 'hook_open']                     # t, k, t_hook, aim
HOOKS = [(e[0], e[2], e[3], e[4], e[5]) for e in EV if e[1] == 'hook']                        # t, k, hit, aim, reach
HONKS = [(e[0], e[2], e[3], e[4], e[5]) for e in EV if e[1] == 'honk']                        # t, k, x, z, 0 bar / 1 skank
SLINGS = [(e[0], e[2], e[3], e[4], e[5], e[6]) for e in EV if e[1] == 'sling']                # t, k, side, a, x, z
QUAKES = [(e[0], e[2], e[3]) for e in EV if e[1] == 'quake']                                  # t, k, until
SWINGS = [e for e in EV if e[1] == 'swing']          # (t, 'swing', k, x0, z0, x1, z1, bx, bz, L, theta, t0, t1, skank)
SMASHES = [(e[0], e[2], e[3], e[4], e[5]) for e in EV if e[1] == 'smash']                    # t, k, x, z, speed
SNAPS = [(e[0], e[2], np.array([e[3], e[4]]), e[5], e[6], e[7]) for e in EV if e[1] == 'snap_aim']   # t, k, spot, ts, t1, r
CRATERS = [(e[0], e[2], e[3], e[4], e[5], e[6]) for e in EV if e[1] == 'crater']             # t, k, x, z, radius, hit
MKS = []                                                                                     # (t0, t1, k)
for e in EV:
    if e[1] == 'mountain':
        ends = [q[0] for q in EV if q[1] == 'mountain_end' and q[2] == e[2] and q[0] >= e[0]]
        MKS.append((e[0], min(ends + [T_KO]), e[2]))


def hop_at(k, t):
    """(hop, u) of fighter k's hop in the air at t, or (None, None)"""
    ts = HOP_T[k]
    if not len(ts) or t >= T_KO or t < T.T_FIGHT:
        return None, None
    j = int(np.searchsorted(ts, t, 'right')) - 1
    if j < 0:
        return None, None
    h = HOP_K[k][j]
    if h[0] <= t < h[4]:
        return h, (t - h[0]) / max(1e-6, h[4] - h[0])
    return None, None


def last_stomp(k, t):
    ts = STOMP_T[k]
    j = int(np.searchsorted(ts, t, 'right')) - 1
    return (t - ts[j]) if j >= 0 else 99.0


def hooked_k(k, t):
    """how hard fighter k is whipping round after being hooked (0..1)"""
    for th, kk, hit, aim, reach in HOOKS:
        if kk == 1 - k and hit and th <= t < th + 0.4 and th < T_KO:
            return 1 - (t - th) / 0.4
    return 0.0


def crook_state(k, t):
    """(aim, ext, curl, glow) of BOCAL HOOK's crook at t, or None"""
    if not (T.T_FIGHT <= t < T_KO):
        return None
    for t0, kk, t_hook, aim in HOOK_OPEN:
        if kk != k or not (t0 <= t < t_hook + 0.42):
            continue
        ab = CFG[k]['abilities']['unlock2']
        i = sim_i(min(t, T_KO))
        row = RUN['HOOK'][i, k] if 'HOOK' in RUN else None
        a = float(row[1]) if row is not None and not np.isnan(row[1]) else aim
        res = [q for q in HOOKS if q[1] == k and abs(q[0] - t_hook) < 0.02]
        hit = bool(res and res[0][2])
        if res:
            a = res[0][3] if t >= t_hook else a
        full = R * 0.4 + ab['reach'] + R * 0.6
        if t < t_hook:
            u = ease((t - t0) / max(1e-6, t_hook - t0))
            return a, 0.35 + (full - 0.35) * u, 0.0, 0.3 * u
        v = (t - t_hook) / 0.42
        curl = (2.4 if hit else 0.6) * min(1.0, v * 4)
        return a, full * (1 - ease(max(0.0, v - 0.45) / 0.55)) + 0.2, curl * (1 - ease(max(0.0, v - 0.6) / 0.4)), \
            (1.0 if hit else 0.4) * (1 - v)
    return None


def _cycles():
    """every WRECKING BALL cycle: the cable drop, the hook, the hoist, the swing (from the run's events + the WRECK
    record), cut short where CABLE SNAP takes over"""
    out = []
    lift = CFG[WRECKER[0]]['weapon']['lift16'] * T.S16 if WRECKER else 0.0
    for e in EV:
        if e[1] != 'cable' or e[0] > T_KO:
            continue
        k, t0, t_hook = e[2], e[0], e[3]
        sw = [q for q in SWINGS if q[2] == k and t_hook - 1e-6 <= q[0] <= t_hook + lift + 0.05]
        nxt = [q[0] for q in EV if q[1] in ('cable', 'snap_aim') and q[2] == k and q[0] > t0]
        cut = min(nxt + [T_KO + 1e-3])
        c = dict(k=k, t0=t0, t_hook=t_hook, sw=None, cut=cut, smash=None)
        if sw and sw[0][0] < cut:
            q = sw[0]
            ps, p1, bot = np.array([q[3], q[4]]), np.array([q[5], q[6]]), np.array([q[7], q[8]])
            d = p1 - ps
            u = d / (np.linalg.norm(d) + 1e-9)
            i = sim_i(q[0] - 0.5 / FPS)
            row = RUN['WRECK'][i, k]
            p0 = row[5:7].copy() if row[0] in (2, 3) else ps.copy()
            c.update(sw=q, ps=ps, p1=p1, bot=bot, u=u, L=q[9], th=q[10], ts0=q[11], ts1=q[12], tl0=q[11] - lift, p0=p0,
                     skank=q[13])
            hits = [m for m in SMASHES if m[1] == k and q[11] - 1e-6 <= m[0] <= q[12] + 1e-6]
            c['smash'] = hits[0] if hits else None
            c['end'] = min(q[12], cut)
        else:
            c['end'] = min(t_hook, cut)
        out.append(c)
    return out


CYCLES = _cycles()
VERT = 1.6        # the crane's pendulum drawn 1.6x taller (the view looks down at 58 deg: heights read short): the pivot
                  # and the ball's height stretch together (a vertical stretch of the whole pendulum; ground positions exact)


def cycle_at(k, t, tail=0.55):
    """the wrecking cycle on screen at t (from its cable drop until the block is back up), or None"""
    for c in CYCLES:
        if c['k'] == k and c['t0'] <= t < c['end'] + tail:
            return c
    return None


def wreck_pose(c, t):
    """(p, h, phase) of the wrecking ball in cycle c at t: 'drop' (rolling, the record), 'lift' (hoisted back),
    'swing' (the pendulum, exact between frames), or None after it"""
    if c is None or t < c['t0'] or t >= c['end'] or c['sw'] is None:
        return None
    if t < c['tl0']:
        return None
    L, th, u = c['L'], c['th'], c['u']
    if t < c['ts0']:
        uu = (t - c['tl0']) / max(1e-6, c['ts0'] - c['tl0'])
        e = uu * uu * (3 - 2 * uu)
        return c['p0'] + (c['ps'] - c['p0']) * e, VERT * L * (1 - math.cos(th)) * e, 'lift'
    uu = min(1.0, (t - c['ts0']) / max(1e-6, c['ts1'] - c['ts0']))
    phi = -th * math.cos(math.pi * uu)
    return c['bot'] + u * (L * math.sin(phi)), VERT * L * (1 - math.cos(phi)), 'swing'


def snap_at(k, t):
    """CABLE SNAP on at t: (snap, phase 'hoist' / 'fall', u 0..1) or (None, None, None)"""
    for q in SNAPS:
        t0, kk, spot, ts, t1, rad = q
        if kk == k and t0 <= t < t1 and t0 <= T_KO:
            if t < ts:
                return q, 'hoist', (t - t0) / max(1e-6, ts - t0)
            return q, 'fall', (t - ts) / max(1e-6, t1 - ts)
    return None, None, None


def air_pose(k, t):
    """(p, h) of the bari off the canvas at t (fight time), or None: the swing / hoist (exact), CABLE SNAP (record)"""
    if k not in WRECKER and k not in SNAPPER:
        return None
    sn, ph, u = snap_at(k, t)
    if sn is not None:
        i = sim_i(t)
        return RUN['P'][i, k].copy(), float(RUN['SNAP'][i, k, 1])
    if k in WRECKER:
        pose = wreck_pose(cycle_at(k, t, 0.0), t)
        if pose is not None:
            return pose[0], pose[1]
        i = sim_i(t)
        if T.T_FIGHT <= t < T_KO and int(RUN['WRECK'][i, k, 0]) == 4:     # v3: dropped on the ropes, slung back in
            return RUN['P'][i, k].copy(), VERT * float(RUN['WRECK'][i, k, 1])
    return None


H_KO = [(air_pose(k, T_KO - 1e-4) or (None, 0.0))[1] for k in (0, 1)]     # height at the K.O. (caught mid-swing)
LEAP_H = 1.45     # a hop that sails over the swinging wrecking ball (the engine's rule) is drawn this high, so it clears
_LEAP = {}


def leap_k(k, hop):
    """height factor for one of fighter k's hops: > 1 when it passes over the enemy's swinging wrecking ball"""
    key = (k, hop[0])
    if key not in _LEAP:
        f = 1.0
        o = 1 - k
        if o in WRECKER:
            for u in np.linspace(0.1, 0.9, 9):
                tt = hop[0] + (hop[4] - hop[0]) * u
                pose = wreck_pose(cycle_at(o, tt, 0.0), tt)
                if pose is not None and pose[2] == 'swing':
                    p = hop[2] + (hop[3] - hop[2]) * u
                    if np.hypot(*(p - pose[0])) < 2 * R * 1.1:
                        f = max(1.0, LEAP_H / ST.hop_height(float(np.hypot(*(hop[3] - hop[2])))))
                        break
        _LEAP[key] = f
    return _LEAP[key]


def fighter(k, t):
    """picture state of fighter k at time t: dict(p (ring frame), h (height), sx / sy (squash), vis, th, scale, shield,
    haste, glow, level, shattered, swirl)"""
    st = dict(vis=True, shield=0.0, haste=0.0, glow=0.0, scale=1.0, th=TH0[k], level=1, shattered=False, h=0.0, sx=1.0,
              sy=1.0, swirl=0.0)
    if t < T.T_FIGHT:
        st['p'] = CORNER0[k] + rev_offset(k, t)
        st['glow'] = 0.25 * T.beat_pulse(t)
        if k in HOPPER:                                          # the bassoon bobs on its toes in the corner
            b = T.beat_pulse(t, 0.1)
            st['sx'], st['sy'] = 1.0 + 0.06 * b, 1.0 - 0.07 * b
        return st
    if t < T_HS:
        i = sim_i(min(t, T_KO))
        st['p'] = RUN['P'][i, k].copy()
        st['th'] = float(RUN['TH'][i, k])
        st['shield'] = float(RUN['SH'][i, k])
        st['haste'] = float(RUN['HA'][i, k])
        st['level'] = int(RUN['LV'][i, k])
        st['swirl'] = hooked_k(k, t)
        if k in HOPPER and t < T_KO:
            hop, u = hop_at(k, t)
            if hop is not None:
                st['p'] = hop[2] + (hop[3] - hop[2]) * u
                L = float(np.hypot(*(hop[3] - hop[2])))
                st['h'], st['sx'], st['sy'] = ST.hop_state(u, L)
                st['h'] *= leap_k(k, hop)                       # v2: a leap over the swinging wrecking ball
                st['leap'] = leap_k(k, hop) > 1.0
                st['glow'] = 0.15 + 0.25 * (hop[5] == 2)
            else:
                st['sx'], st['sy'] = ST.land_squash(last_stomp(k, t))
        ap = air_pose(k, min(t, T_KO - 1e-4))                    # v2: swung on the cable / hoisted for CABLE SNAP
        if ap is not None:
            st['p'], st['h'] = ap[0], ap[1]
        if t >= T_KO and k == LOSE:
            st['glow'] = 1.0
        return st
    gl = ease((t - T.T_GLIDE) / (T.T_REV - T.T_GLIDE))
    corner = CORNER0[k] + rev_offset(k, t) * ss(T.T_REV - 0.3, T.T_REV + 0.2, t)
    rev_glow = 0.25 * T.beat_pulse(t) * ss(T.T_REV, T.T_REV + 0.3, t)
    th_end = float(RUN['TH'][-1, k])
    st['th'] = th_end + _wrap(TH0[k] - th_end) * gl
    if k in HOPPER and t >= T.T_REV:                           # back on its toes in the corner
        b = T.beat_pulse(t, 0.1) * ss(T.T_REV, T.T_REV + 0.3, t)
        st['sx'], st['sy'] = 1.0 + 0.06 * b, 1.0 - 0.07 * b
    if k == WIN:
        centre = np.array([0.0, 0.0])
        u = ease((t - T_HS) / 1.8)
        bob = 0.06 * math.sin(2 * math.pi * (t - T_HS) / (2 * T.BAR)) * ss(T_HS, T_HS + 1.5, t)
        p = P_KO[k] + (centre - P_KO[k]) * u + np.array([bob, -bob])
        st['p'] = p + (corner - p) * gl
        st['glow'] = max(rev_glow, (0.35 + 0.5 * solo_env(t)) * ss(T_HS, T_HS + 0.6, t) * (1 - gl))
        st['level'] = 3 if t < T.T_GLIDE else 1
        st['h'] = H_KO[k] * (1 - ss(T_HS, T_HS + 0.4, t))           # v2: caught in the air at the K.O.: it drops
        if k in HOPPER and T_SOLO <= t < T.T_REASSEMBLE:          # the champion tiptoes a little hop on every solo note
            e = solo_env(t)
            st['h'] = 0.35 * (1 - e) * e * 4
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
    if e[1] == 'hit' and e[7] in ('stomp', 'hook', 'sling', 'shockwave', 'quake') and e[0] <= T_KO:
        SHK.add(e[0], {'stomp': 0.4 if e[8] == 0 else 0.9, 'hook': 1.0, 'sling': 1.4, 'shockwave': 1.6, 'quake': 2.2}[e[7]])
for t_, k_, x_, z_, kind_ in HONKS:                    # every honk shakes the floor a little (the window rattler)
    if t_ <= T_KO:
        SHK.add(t_, 0.9 if kind_ == 0 else 0.5)
for t_, k_, x_, z_, sp_ in SMASHES:                    # v2: a WRECKING BALL smash (harder the faster the swing)
    if t_ <= T_KO:
        SHK.add(t_, 1.0 + 1.2 * sp_)
for t_, k_, x_, z_, r_, hit_ in CRATERS:               # CABLE SNAP: the crash
    if t_ <= T_KO:
        SHK.add(t_, 2.5)
        SHK.add(t_ + 0.09, 1.2)
for t_, k_, until_ in QUAKES:                          # LOW A: the ring rumbles for as long as the ropes are held
    tt = t_
    while tt < min(until_, T_KO):
        SHK.add(tt, 1.3)
        tt += T.BEAT / 2
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
_OVER = {}


def bari_over(t):
    """[(ring-frame centre, underside height)] of the bari while it is up on the cable / slung back and its footprint
    reaches the rope line (v3, as #40 v2: it may swing out over the ropes): the ropes are pressed down under it"""
    if t in _OVER:
        return _OVER[t]
    out = []
    if T.T_FIGHT <= t < T_HS:
        for k in WRECKER:
            st = fighter(k, t)
            if st['vis'] and st['h'] > 0.02 and max(abs(st['p'][0]), abs(st['p'][1])) > S.A - R * 1.05:
                out.append((np.asarray(st['p'], float), float(st['h'])))
    if len(_OVER) > 64:
        _OVER.clear()
    _OVER[t] = out
    return out


def _rope_wobbles():
    """v3: when the bari lifts off a rope it pressed (the swing passing over, or slung back in), the rope springs back:
    a synthetic rope event there (it wobbles like a hit rope; speed 3.2 marks it for the audio's twang)"""
    out = []
    prev = {}
    f0, f1 = int(T.T_FIGHT * FPS), int(T_KO * FPS)
    for f in range(f0, f1 + 1):
        t = f / FPS
        cur = {}
        for c, hb in bari_over(t):
            for s_, (ax, sg) in enumerate(((1, 1), (0, 1), (1, -1), (0, -1))):
                if c[ax] * sg > S.A - R * 1.05 and hb < S.ROPE_Y[-1]:
                    cur[s_] = c.copy()
        for s_, c in prev.items():
            if s_ not in cur:
                out.append((t, s_, float(c[0]), float(c[1]), 3.2))
        prev = cur
    return out


ROPE_EV = sorted(ROPE_EV + _rope_wobbles())
ROPE_T = np.array([r[0] for r in ROPE_EV])
WAVE_EV = [(e[0], 1.0 if e[4] == 'quake' else 0.5) for e in EV if e[1] == 'ability' and e[4] in ('shockwave', 'quake')]
RK = RATTLER[0] if RATTLER else None


def rope_amp(s, t):
    """how hard rope side s rings (0..1, the bari's rattle, from the recorded RATTLE): frozen through the hit-stop,
    ringing down after it"""
    if RK is None or t < T.T_FIGHT:
        return 0.0
    if t < T_KO:
        return float(RUN['RATTLE'][sim_i(t), RK, s])
    return float(RUN['RATTLE'][-1, RK, s]) * math.exp(-max(0.0, t - T_HS) / 0.5)


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
    a_r = rope_amp(s, t)                                                    # WINDOW RATTLER: plucked-string waves
    if a_r > 0.01:
        j = int(round((y - S.ROPE_Y[0]) / max(1e-6, S.ROPE_Y[1] - S.ROPE_Y[0])))
        disp += ST.rope_wave(u, t + 0.03 * j + 0.05 * s, a_r, 0.0, mode=3, f=6.5) * (1 - 0.15 * j)
    disp += 0.012 * T.beat_pulse(t, 0.1) * np.sin(np.pi * u)                 # the bass thud
    sag = 0.035 * np.sin(np.pi * u)
    yy = y - sag
    for c, hb in bari_over(t):                                    # v3: the bari swinging over presses the ropes down
        dd = np.hypot(P[:, 0] - c[0], P[:, 1] - c[1])
        under = hb + R - np.sqrt(np.clip(R * R - dd * dd, 0, None)) - 0.03    # the ball's underside above each point
        on = dd < R
        yy = np.where(on, np.minimum(yy, under), yy)
        near = (dd >= R) & (dd < R * 1.7)                        # the rope eases back up either side of it
        if near.any():
            w = 1 - (dd - R) / (R * 0.7)
            yy = np.where(near, np.minimum(yy, yy - np.clip(yy - (hb - 0.03), 0, None) * w * w * 0.6), yy)
    P = P + outn[None] * disp[:, None]
    return S.to_world(P[:, 0], P[:, 1], np.maximum(yy, 0.05))


# ---------------------------------------------------------------- shards (the loser shatters at the K.O.)
WOOD = np.array([120, 56, 52], float) if NAMES[LOSE] == 'BASSOON' else np.array([234, 188, 86], float)   # maple / lacquer
NICKEL = np.array([214, 216, 226], float)
CANE = np.array([232, 208, 150], float)


def _shards():
    rng = np.random.default_rng(371)
    p0 = P_KO[LOSE]
    parts = []
    kinds = ['glass'] * 24 + ['wood'] * 6 + ['key'] * 8 + (['reed'] * 3 if NAMES[LOSE] == 'BASSOON' else ['bell'] * 3)
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
SPR_PATH = os.path.join(HERE, 'check', 'sprites_ballfight13.npz')


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
        for roll in (roll0,):
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
    """the glass ball with its instrument; #39: at height st['h'] above the canvas (a hop) and squashed / stretched by
    (sx, sy) about its lowest point (so a landing squash stays on the floor)"""
    pw = S.to_world(st['p'][0], st['p'][1], R + st['h'])
    Pc = cam.to_cam(pw[None])[0]
    s = cam.proj(Pc[None])[0]
    rp = R * cam.f / Pc[2] * st['scale']
    sx_, sy_ = st['sx'], st['sy']
    s = np.array([s[0], s[1] + rp * (1 - sy_)])                              # squash about the bottom of the ball
    ri = int(round(rp * max(sx_, sy_)))
    x0, y0 = int(round(s[0])) - ri - 2, int(round(s[1])) - ri - 2
    n = 2 * ri + 5
    yy, xx = np.mgrid[0:n, 0:n]
    dx = (xx + x0 + 0.5 - s[0]) / (rp * sx_)
    dy = (yy + y0 + 0.5 - s[1]) / (rp * sy_)
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
    glass = np.array([236, 232, 226])
    inner = (col * 0.30 + glass * 0.70)[None, None] * (0.70 + 0.36 * band[..., None])
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
    if k in WRECKER and st['scale'] > 0.5:                                  # v2: the bari's harness (straps + ring)
        arr[:] = ST.harness(arr, s, rp, sx_, sy_, st.get('ring_dir'), col)
    rc = np.clip(col * (0.9 + 0.5 * glow) + 50 * glow, 0, 255)
    mr = rim[sl]
    reg[mr] = rc
    me = edge[sl]
    reg[me] = reg[me] * 0.2 + np.array([22, 14, 30]) * 0.8
    spec = inside & ((dx + 0.38) ** 2 + (dy + 0.5) ** 2 < (2.0 / rp) ** 2 + 0.014)
    if st['h'] > 0.02:                                                       # airborne: drawn over what it hops past
        Pc = Pc - np.array([0, 0, 0.6])
    reg[spec[sl]] = np.array([255, 252, 244])
    dreg[m | me] = Pc[2]
    return s, rp


# ---------------------------------------------------------------- v2: the crane (WRECKING BALL / CABLE SNAP)
def _w(p, y):
    return S.to_world(p[0], p[1], y)


def crane_state(k, t):
    """world geometry of the bari's crane at t: dict(top, block, end, attached, pieces=[(points, alpha)]) or None.
    Frozen through the hit-stop; after it any live crane flies back up to the rig."""
    if (k not in WRECKER and k not in SNAPPER) or t < T.T_FIGHT:
        return None
    if t >= T_HS:
        st = crane_state(k, T_KO - 1e-4)
        if st is None:
            return None
        v = (t - T_HS) / 0.6
        if v >= 1.0:
            return None
        b = st['block'].copy()
        b[1] += (ST.Y_RIG - b[1]) * v * v
        st.update(block=b, end=b - np.array([0.0, 0.42, 0.0]), attached=False,
                  pieces=[(P, a * (1 - v)) for P, a in st['pieces']])
        return st
    tf = min(t, T_KO - 1e-4)
    sn, ph, u = snap_at(k, tf)
    if sn is None:                                       # the snapped cable's ends, just after the crash
        for q in SNAPS:
            if q[1] == k and q[4] <= tf < q[4] + 0.6:
                sn, ph, u = q, 'after', (tf - q[4]) / 0.6
    if sn is not None:
        t0, _, spot, ts, t1, rad = sn
        i = sim_i(tf)
        p = RUN['P'][i, k]
        h = float(RUN['SNAP'][i, k, 1]) if ph != 'after' else 0.0
        C = _w(p, R + h)
        ring = C + np.array([0.0, R, 0.0])
        if ph == 'hoist':
            tgt = ring + np.array([0.0, 1.1, 0.0])
            prev = [c for c in CYCLES if c['k'] == k and c['sw'] is not None and abs(c['cut'] - t0) < 1e-3]
            if prev and u < 0.3:                          # out of a swing: the block slides over from its pivot
                pv = _w(prev[0]['bot'], R + VERT * prev[0]['L'])
                tgt = pv + (tgt - pv) * ss(0.0, 0.3, u)
            return dict(top=np.array([tgt[0], ST.Y_RIG, tgt[2]]), block=tgt, end=ring, attached=True, pieces=[])
        # snapped: the upper cable whips up and away, a stub flops on the falling ball
        tsn = tf - ts
        i_s = sim_i(ts)
        ps_ = RUN['P'][i_s, k]
        b0 = _w(ps_, R + float(RUN['SNAP'][i_s, k, 1]) + R + 1.1)
        blk = b0 + np.array([0.35 * tsn, 7.0 * tsn + 6.0 * tsn * tsn, 0.0])
        brk = blk - np.array([0.0, 0.55, 0.0])
        up_piece = ST.cable(blk, brk + np.array([0.2 * math.sin(tsn * 30), 0.0, 0.0]), 6, wig=0.12 * math.exp(-tsn / 0.3),
                            ph=tsn * 40)
        stub_top = ring + np.array([0.15 * math.sin(tsn * 22), 0.5 * math.exp(-tsn / 0.4) + 0.08, 0.1])
        stub = ST.cable(ring, stub_top, 5, wig=0.06, ph=tsn * 25)
        fade = 1.0 - (u if ph == 'after' else 0.0)
        return dict(top=np.array([blk[0], ST.Y_RIG, blk[2]]), block=blk, end=None, attached=False,
                    pieces=[(up_piece, max(0.0, 1 - tsn / 0.7)), (stub, fade)])
    c = cycle_at(k, tf)
    if c is None:
        return None
    if c['sw'] is not None:
        Pv = _w(c['bot'], R + VERT * c['L'])
    else:
        Pv = _w(RUN['P'][sim_i(c['t0']), k], R + VERT * CFG[k]['weapon']['base'])
    i = sim_i(tf)
    pose = wreck_pose(c, tf)
    p = pose[0] if pose is not None else RUN['P'][i, k]
    h = pose[1] if pose is not None else 0.0
    C = _w(p, R + h)
    if tf < c['t_hook'] and tf < c['end']:                   # the drop: the block comes down, the hook reaches
        u = (tf - c['t0']) / max(1e-6, c['t_hook'] - c['t0'])
        y = ST.Y_RIG + (Pv[1] - ST.Y_RIG) * (1 - (1 - u) ** 3)
        blk = np.array([Pv[0], y, Pv[2]])
        ring = C + np.array([0.0, R, 0.0])
        dang = blk - np.array([0.0, 0.42, 0.0])
        end = dang + (ring - dang) * ss(0.55, 1.0, u)
        return dict(top=np.array([Pv[0], ST.Y_RIG, Pv[2]]), block=blk, end=end, attached=False, pieces=[])
    if tf < c['end']:                                        # hooked: hoisted back, the swing
        d = Pv - C
        ring = C + d / (np.linalg.norm(d) + 1e-9) * R
        return dict(top=np.array([Pv[0], ST.Y_RIG, Pv[2]]), block=Pv, end=ring, attached=True, pieces=[])
    v = (tf - c['end']) / 0.55                               # unhooked: the block flies back up
    blk = Pv + np.array([0.0, (ST.Y_RIG - Pv[1]) * v * v, 0.0])
    end_c = cycle_end_ring(c, k)
    dang = blk - np.array([0.0, 0.42, 0.0]) + np.array([0.18 * math.sin(v * 9) * (1 - v), 0.0, 0.0])
    end = end_c + (dang - end_c) * ss(0.0, 0.3, v)
    return dict(top=np.array([Pv[0], ST.Y_RIG, Pv[2]]), block=blk, end=end, attached=False, pieces=[])


def cycle_end_ring(c, k):
    """where the hook let go at the end of cycle c (the harness ring at the far end of the swing)"""
    pose = wreck_pose(c, c['end'] - 1e-4)
    if pose is None:
        p = RUN['P'][sim_i(c['end']), k]
        return _w(p, 2 * R)
    Pv = _w(c['bot'], R + VERT * c['L']) if c['sw'] is not None else _w(pose[0], 2 * R + 1)
    C = _w(pose[0], R + pose[1])
    d = Pv - C
    return C + d / (np.linalg.norm(d) + 1e-9) * R


def draw_crane(arr, cam, cs, col):
    """the crane line from the rig, the hook block, the swing cable and its hook, the snapped pieces"""
    if cs is None:
        return arr
    blk = cs['block']
    arr = ST.line3(arr, cam, ST.cable(cs['top'], blk + np.array([0.0, 0.17, 0.0]), 6), ST.STEEL * 0.75, 2, hi=0.3)
    arr = ST.hook_block(arr, cam, blk)
    if cs['end'] is not None:
        bot = blk - np.array([0.0, 0.2, 0.0])
        slack = 0.0 if cs['attached'] else 0.02
        arr = ST.line3(arr, cam, ST.cable(bot, cs['end'], 8, sag=slack), ST.STEEL, 2)
        tw = blk - cs['end']
        arr = ST.hook_shape(arr, cam, cs['end'], tw / (np.linalg.norm(tw) + 1e-9))
    for P, a in cs['pieces']:
        if a > 0.02:
            arr = ST.line3(arr, cam, P, ST.STEEL, 2, alpha=a)
    return arr


def draw_swing_fx(arr, cam, k, t):
    """the swing's motion smear (ghost balls a few ms back) and its coral trail, under the ball"""
    tf = min(t, T_KO - 1e-4)
    c = cycle_at(k, tf, 0.0)
    if c is None or c['sw'] is None or not (c['ts0'] <= tf < c['ts1'] + 0.12):
        return arr
    pts, spd = [], 0.0
    for j in range(12, -1, -1):
        tt = tf - j * 0.018
        pose = wreck_pose(c, tt)
        if pose is None or pose[2] != 'swing':
            continue
        pts.append(cam.wp(_w(pose[0], R + pose[1])[None])[0])
    uu = min(1.0, max(0.0, (tf - c['ts0']) / max(1e-6, c['ts1'] - c['ts0'])))
    spd = math.sin(math.pi * uu)
    if len(pts) >= 2:
        arr = ST.swing_trail(arr, pts, COL[k], k=0.35 + 0.65 * spd)
    for j in (4, 3, 2, 1):                                   # ghosts, oldest first
        pose = wreck_pose(c, tf - j * 0.014)
        if pose is None or pose[2] != 'swing':
            continue
        Pc = cam.to_cam(_w(pose[0], R + pose[1])[None])[0]
        s_ = cam.proj(Pc[None])[0]
        rp = R * cam.f / Pc[2]
        a = np.linspace(0, 2 * math.pi, 19)[:-1]
        circ = np.column_stack([s_[0] + rp * np.cos(a), s_[1] + rp * np.sin(a)])
        layer_blend(arr, [circ], np.clip(COL[k] * 0.7 + 70, 0, 255), 0.24 * (1 - j / 5) * (0.3 + 0.7 * spd))
    return arr


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
    """how long a use stays lit: STACCATO / SKANK / LOW A's held ropes for as long as they last, MOUNTAIN KING for its
    dance, BOCAL HOOK until it hooks, else a flash"""
    ab = CFG[k]['abilities'][slot]
    if ab['kind'] in ('shield', 'haste', 'offbeat', 'quake'):
        return ab['dur_beats'] * T.BEAT if 'dur_beats' in ab else ab['dur']
    if ab['kind'] == 'mountain':
        return sum(ab['seq16']) * T.BEAT / 4
    if ab['kind'] == 'hook':
        return ab['open16'] * T.BEAT / 4 + 0.3
    if ab['kind'] == 'snap':                         # v2: CABLE SNAP, until the crash
        return ab['tele16'] * T.BEAT / 4
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
FLASH_EV = [e[0] for e in EV if (e[1] == 'hit' and e[7] in ('quake', 'sling', 'hook', 'wreck', 'crash')) or
            e[1] == 'ability'] + \
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
        hh = R + st['h']                                                   # a hop's shadow stays on the canvas
        circ = np.array([[pw[0] + R * st['scale'] * math.cos(u), hh, pw[2] + R * st['scale'] * math.sin(u)] for u in a])
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
            if how == 'stomp':
                n, sp, life = (12, 2.8, 0.26) if e[8] else (5, 1.6, 0.18)       # a backstab throws more
            else:
                n, sp, life = (16, 3.2, 0.3) if how in ('quake', 'sling', 'shockwave', 'hook', 'wreck', 'crash') \
                    else (8, 2.4, 0.22)
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
PA = fx.Particles(seed=39)
MK = fx.Marks()
MO = fx.Moments()
MAPLE_DUST = [(214, 176, 140), (186, 140, 110), (240, 214, 186)]
GREEN_SP = [(150, 240, 180), (60, 190, 120), (240, 255, 240)]
CORAL_SP = [(255, 196, 170), (255, 118, 92), (255, 240, 220)]
GRIT = [(60, 44, 40), (110, 84, 70), (170, 130, 110)]


def _fx_build():
    """every effect from the run's events, once, in time order (deterministic: frames render in any order)"""
    up = lambda x, z, y: S.to_world(x, z, y)
    for t_, k, x, z, hit, back, rad, kind in STOMPS:          # TIPTOE: every landing a stomp; prints on the canvas
        if t_ > T_KO:
            break
        PA.emit(t_, up(x, z, 0.05), 'dust', n=3 + 2 * back, col=MAPLE_DUST, scale=0.8)
        MK.add(t_, 'scorch', x, z, size=0.13, col=(40, 30, 30), life=2.4, seed=int(t_ * 100), strength=0.3)
        FL.add(t_, up(x, z, R * 0.5), COL[k], radius=1.2 + 0.6 * back, peak=0.35 + 0.45 * back + 0.2 * (kind == 2),
               decay=0.18)
    for e in EV:
        t_ = e[0]
        if t_ > T_KO + 1e-6:
            break
        kind = e[1]
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = up(x, z, R)
            if how == 'stomp' and e[8]:                            # a backstab: green slashes, wood chips
                BU.add(t_, pos, COL[att], 1.1)
                PA.emit(t_, pos, 'chips', n=8, col=GREEN_SP + MAPLE_DUST[:1])
                v = RUN['V'][sim_i(t_), e[3]]
                MK.add(t_, 'claw', x, z, size=0.5, angle=math.atan2(v[1], v[0]) + 1.2, col=(20, 60, 36), life=2.6, n=2,
                       seed=int(t_ * 100), strength=0.6)
            elif how == 'stomp':
                PA.emit(t_, pos, 'sparks', n=4, col=GREEN_SP)
            elif how == 'hook':                                    # BOCAL HOOK lands
                BU.add(t_, pos, (220, 226, 240), 1.0)
                FL.add(t_, pos, COL[att], radius=1.6, peak=0.8, decay=0.3)
                PA.emit(t_, pos, 'sparks', n=10, col=[(226, 228, 240), (255, 255, 255)] + GREEN_SP[:1])
            elif how == 'rattle':                                  # the honk shoves
                BU.add(t_, pos, COL[att], 0.8)
                PA.emit(t_, pos, 'puffs', n=5, col=CORAL_SP)
            elif how == 'sling':                                   # slung off a ringing rope
                BU.add(t_, pos, COL[att], 1.3)
                FL.add(t_, pos, COL[att], radius=1.8, peak=0.9, decay=0.3)
                PA.emit(t_, pos, 'sparks', n=12, col=CORAL_SP)
                PA.emit(t_, pos, 'streaks', n=6, col=CORAL_SP[:2], speed=(2.0, 3.2))
            elif how == 'shockwave':                               # GROWL lands: grit
                BU.add(t_, pos, COL[att], 1.2)
                FL.add(t_, pos, COL[att], radius=1.8, peak=0.8, decay=0.35)
                PA.emit(t_, pos, 'chips', n=12, col=GRIT)
            elif how == 'quake':                                   # LOW A lands
                BU.add(t_, pos, COL[att], 1.5)
                FL.add(t_, pos, COL[att], radius=2.2, peak=1.0, decay=0.4)
            elif how == 'wreck':                                   # v2: WRECKING BALL smash: burst, dust, a skid mark
                sp_ = [m[4] for m in SMASHES if abs(m[0] - t_) < 1e-4]
                sp_ = sp_[0] if sp_ else 0.7
                BU.add(t_, pos, COL[att], 1.0 + 0.7 * sp_)
                FL.add(t_, pos, COL[att], radius=2.0, peak=0.6 + 0.5 * sp_, decay=0.32)
                PA.emit(t_, up(x, z, 0.06), 'dust', n=6 + int(8 * sp_), col=MAPLE_DUST, scale=1.1)
                PA.emit(t_, pos, 'sparks', n=10, col=CORAL_SP)
                PA.emit(t_, pos, 'chips', n=5, col=GRIT + MAPLE_DUST[:1])
                v = RUN['V'][sim_i(t_), e[3]]
                MK.add(t_, 'scorch', x, z, size=0.32 + 0.2 * sp_, col=(46, 30, 30), life=3.0, seed=int(t_ * 100),
                       strength=0.45)
            elif how == 'crash':                                   # CABLE SNAP lands on its target
                BU.add(t_, pos, COL[att], 1.8)
                FL.add(t_, pos, COL[att], radius=2.6, peak=1.1, decay=0.45)
        elif kind == 'honk':                                       # every honk: a coral light at the bell, a puff
            k, x, z = e[2], e[3], e[4]
            FL.add(t_, up(x, z, R), COL[k], radius=2.2, peak=0.7 if e[5] == 0 else 0.45, decay=0.3)
            PA.emit(t_, up(x, z, R * 1.4), 'puffs', n=4, col=CORAL_SP, scale=1.2)
        elif kind == 'sling':
            pass
        elif kind == 'swing':                                      # v2: the swing's bottom: a coral light, a honk puff
            k, tb = e[2], (e[11] + e[12]) / 2
            if tb <= T_KO:
                FL.add(tb, up(e[7], e[8], R), COL[k], radius=2.0, peak=0.45, decay=0.25)
        elif kind == 'cable_snap':                                 # CABLE SNAP: the cable parts: steel sparks up high
            k = e[2]
            i_ = sim_i(t_)
            p_ = RUN['P'][i_, k]
            top = up(p_[0], p_[1], R + float(RUN['SNAP'][i_, k, 1]) + 2 * R + 0.5)
            BU.add(t_, top, (230, 234, 245), 0.9, bright=False)
            PA.emit(t_, top, 'sparks', n=14, spread=180, col=[(255, 250, 230), (226, 230, 242), (255, 196, 120)])
            FL.add(t_, top, (255, 240, 220), radius=1.8, peak=0.6, decay=0.25)
        elif kind == 'crater':                                     # the crash: a ring of dust, grit, the crater mark
            k, x, z = e[2], e[3], e[4]
            PA.emit(t_, up(x, z, 0.05), 'dust', n=26, spread=180, col=MAPLE_DUST, scale=1.4, speed=(1.2, 3.0))
            PA.emit(t_, up(x, z, R * 0.5), 'chips', n=14, spread=180, col=GRIT, speed=(1.5, 3.4))
            MK.add(t_, 'crack', x, z, size=1.5, life=6.0, n=7, seed=int(t_ * 10), strength=0.75)
            MK.add(t_, 'scorch', x, z, size=0.95, col=(34, 22, 24), life=6.0, seed=int(t_ * 10) + 1, strength=0.7)
        elif kind == 'ability' and e[4] == 'quake':                # LOW A: dust from every tile, cracks in the canvas
            p = RUN['P'][sim_i(t_), e[2]]
            rng = np.random.default_rng(int(t_ * 100))
            for j in range(10):
                q = rng.uniform(-S.A + 0.3, S.A - 0.3, 2)
                PA.emit(t_ + 0.04 * j, up(q[0], q[1], 0.05), 'dust', n=4, col=MAPLE_DUST)
            MK.add(t_, 'crack', p[0], p[1], size=1.4, life=4.0, n=6, seed=int(t_ * 10), strength=0.7)
        elif kind == 'ability' and e[4] == 'shockwave':            # GROWL leaves the bari: grit
            p = RUN['P'][sim_i(t_), e[2]]
            PA.emit(t_, up(p[0], p[1], R * 0.6), 'chips', n=14, spread=180, col=GRIT, speed=(1.5, 3.0))
        elif kind == 'block':
            BU.add(t_, up(e[4], e[5], R), (255, 226, 150), 0.6, bright=False)
    for t0, t1, k in MKS:                                         # MOUNTAIN KING: a green light that keeps up the dance
        tb = t0
        while tb < t1:
            i = sim_i(tb)
            p = RUN['P'][i, 1 - k]
            FL.add(tb, up(p[0], p[1], R), COL[k], radius=2.0, peak=0.5, decay=0.3)
            tb += T.BEAT / 2
    for t_, who, name in STORY['caps']:                           # ability moments on the captioned abilities
        MO.add(t_, who, COL[who])
    pos = S.to_world(P_KO[LOSE][0], P_KO[LOSE][1], R)
    BU.add(T_KO, pos, COL[LOSE], 2.0)
    FL.add(T_KO, pos, COL[LOSE] * 0.9, radius=2.4, peak=1.1, decay=0.6)
    PA.emit(T_KO, pos, 'chips' if NAMES[LOSE] == 'BASSOON' else 'brass', n=26, spread=180, speed=(0.8, 2.2),
            col=[tuple(WOOD), (214, 216, 226), (232, 208, 150)])


_fx_build()


def shimmers(img, cam, t):
    """heat-shimmer ripples: every honk's sound ring, GROWL's, LOW A's broad rumble"""
    if T.T_FIGHT <= t < T_HS:
        i = sim_i(min(t, T_KO))
        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][i]:
            big = kind == 'quake'
            img = fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.45 if big else 0.3,
                                   3.0 if big else 2.2, t, k=1 - r / rmax, seed=owner)
    return img


def post_fx(img, cam, t, fs, drawn):
    """after the grade: chromatic fringing on big hits, the ability moments (dim + speed lines), the impact bursts"""
    for _, e in events_between(t - 0.12, t, ('hit',)):
        if e[7] in ('quake', 'sling', 'hook', 'shockwave', 'wreck', 'crash') or (e[7] == 'stomp' and e[8] == 1):
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            img = fx.fringe(img, s_[0], s_[1], 26, 2.5, k=1 - (t - e[0]) / 0.12)
    if T_KO <= t < T_HS + 0.3:
        s_ = cam.wp(S.to_world(P_KO[LOSE][0], P_KO[LOSE][1], R)[None])[0]
        img = fx.fringe(img, s_[0], s_[1], 34, 3.0, k=1 - ss(T_HS, T_HS + 0.3, t))
    keep = [(s_[0], s_[1], rp * 1.7) for k, (s_, rp) in drawn.items()]
    img = MO.apply_dim(img, t, keep=keep)
    for k, (s_, rp) in drawn.items():
        f1 = fighter(k, max(0.0, t - 0.06))
        s1 = cam.wp(S.to_world(f1['p'][0], f1['p'][1], R + f1['h'])[None])[0]
        v = (s_[0] - s1[0], s_[1] - s1[1])
        img = MO.speed_lines(img, t, k, s_[0], s_[1], rp, v)
    img = BU.draw(img, cam, t)
    return img


# ---------------------------------------------------------------- the weapons on screen
def live_hops(t):
    """[(k, p0, p1, fade, peak)] of the hops whose dotted arcs show at t (the arc fades over the hop + 0.2 s)"""
    out = []
    if not (T.T_FIGHT <= t < T_KO):
        return out
    for k in HOPPER:
        j = int(np.searchsorted(HOP_T[k], t, 'right'))
        for h in HOP_K[k][max(0, j - 2):j]:
            age = t - h[0]
            life = (h[4] - h[0]) + 0.2
            if 0 <= age < life:
                out.append((k, h[2], h[3], 0.55 * (1 - age / life), ST.hop_height(float(np.hypot(*(h[3] - h[2]))))))
    return out


def live_stomps(t):
    """[(k, x, z, age, radius, back)] of the stomp rings showing at t"""
    if not (T.T_FIGHT <= t < T_HS):
        return []
    tc = min(t, T_KO)
    return [(k, x, z, tc - t_, rad, back) for t_, k, x, z, hit, back, rad, kind in STOMPS if t_ <= tc < t_ + 0.34]


def live_slashes(t):
    """[(k, x, z, aim, age)] backstab slashes showing at t"""
    out = []
    if not (T.T_FIGHT <= t < T_HS):
        return out
    tc = min(t, T_KO)
    for _, e in events_between(tc - 0.3, tc, ('hit',)):
        if e[7] == 'stomp' and e[8]:
            v = RUN['V'][sim_i(e[0]), e[3]]
            out.append((e[2], e[5], e[6], math.atan2(v[1], v[0]), tc - e[0]))
    return out


def mk_state(t):
    """(k, centre, ang0, n_done) of MOUNTAIN KING's dance at t, or None"""
    for t0, t1, k in MKS:
        if t0 <= t < min(t1 + 0.4, T_KO):
            hs = [h for h in HOP_K[k] if t0 - 1e-6 <= h[0] <= t and h[5] == 2]
            if not hs:
                return None
            i = sim_i(t)
            c = RUN['P'][i, 1 - k]
            d = hs[0][3] - RUN['P'][sim_i(hs[0][4]), 1 - k]
            return k, c, math.atan2(d[1], d[0]), len(hs), (1.0 if t < t1 else 1 - (t - t1) / 0.4)
    return None


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
    stomps = live_stomps(t)
    cranes = {k_: crane_state(k_, t) for k_ in WRECKER}

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
        # decals: the stomp rings, the honks' sound rings, GROWL, LOW A's rumble + jumping tiles
        for k_, x_, z_, age, rad, back in stomps:
            band = ST.stomp_light(sxp, szp, x_, z_, age, rad, back)
            if band is not None:
                band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
                Ic += (band * (0.9 + 0.5 * back))[:, None] * (COL[k_] / 255.0 * 1.2 + 0.25)[None]
        wav = RUN['WAVES'][i] if in_fight else []
        for owner, cx, cz, r, rmax, kind, t0 in wav:
            band = ST.honk_light(sxp, szp, cx, cz, r, rmax, t, kind)
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (band * (1.3 if kind == 'quake' else 1.0))[:, None] * (COL[owner] / 255.0 * 1.3 + 0.2)[None]
        tcn = min(t, T_KO)
        for t0_, k_s, spot_, ts_, t1_, rad_ in SNAPS:                 # v2: CABLE SNAP's target: shadow + danger ring
            if t0_ <= tcn < t1_ and t < T_HS:
                u_ = (tcn - t0_) / max(1e-6, t1_ - t0_)
                shd, rng_ = ST.snap_target(sxp, szp, spot_[0], spot_[1], R, rad_, u_, t)
                Ic *= (1 - 0.8 * shd)[:, None]
                rng_ = np.floor(rng_ * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
                Ic += (1.1 * rng_)[:, None] * (COL[k_s] / 255.0 * 1.3 + 0.2)[None]
        for t_q, k_q, until in QUAKES:                                 # LOW A: the floor tiles jump
            if t_q <= t < min(until, T_KO) + 0.3:
                kq = min(1.0, (t - t_q) / 0.1) * (1 - ss(min(until, T_KO), min(until, T_KO) + 0.3, t))
                tj = ST.tile_jump(sxp, szp, t, kq)
                if tj is not None:
                    Ic *= (1 + 0.35 * tj)[:, None]
                    Ic += (0.25 * tj)[:, None] * (COL[k_q] / 255.0)[None]
        col = alb * Ic
        arr[ys, xs] = np.floor(col / 5 + BAYER_FULL[ys, xs, None]) * 5
        depth[ys, xs] = dc[ys, xs]
    fr.set_array(arr, depth)

    # ---------------- back ropes + posts
    def draw_rope_side(s):
        a_r = rope_amp(s, t)                                       # WINDOW RATTLER: a ringing rope glows coral
        for j, y in enumerate(S.ROPE_Y):
            P = rope_points(s, y, t)
            base = S.ROPE_COL[j] if j != 1 else COL[0 if s in (0, 3) else 1] * 0.92    # middle rope: the fighter's side
            if a_r > 0.02:
                base = base * (1 - 0.85 * min(1.0, a_r)) + COL[RK] * np.array([0.95, 0.8, 0.8]) * 0.85 * min(1.0, a_r)
            c = shade(L, np.array([[0, 1.0, 0.3]]) / np.linalg.norm([0, 1, 0.3]), P.mean(0)[None], base, rimk=0.5)[0]
            if a_r > 0.02:
                c = np.clip(c + np.array([60, 10, 0]) * a_r, 0, 255)
            fr.line(cam, P, c, 2 + (a_r > 0.35))
            if a_r > 0.2 and j == 1:                               # the antinodes shimmer with light
                for q in P[2::4]:
                    s_ = cam.wp(q[None])[0]
                    fr.glow(s_[0], s_[1], 7, tuple(COL[RK] * 0.8 + 50), 0.22 * a_r)

    def draw_post(name):
        for Q, n, c in S.POSTS[name]:
            if np.dot(n, cam.pos - Q.mean(0)) > 0:
                cc = shade(L, n[None], Q.mean(0)[None], c, rimk=0.6)[0]
                fr.poly(cam, Q, cc)
    draw_post('back')
    draw_rope_side(2)
    draw_rope_side(3)
    crane_first = any(fs[k_].get('leap') for k_ in HOPPER)          # v2: a leap over the swing passes in front of
    if crane_first:                                                 # the cable: the crane goes under the balls
        arr, depth = fr.array(), fr.depth()
        for k_ in WRECKER:
            arr = draw_crane(arr, cam, cranes[k_], COL[k_])
        fr.set_array(arr, depth)
    # ---------------- fighters, reeds, posts (painter by depth)
    items = []
    for name in ('red', 'blue'):
        items.append((cam.depth(S.to_world(*S.CORNERS[name], 0.7))[0], 'post', name))
    for k, st in enumerate(fs):
        if not st['vis'] or st['scale'] < 0.05:
            continue
        pw = S.to_world(st['p'][0], st['p'][1], R)
        dz = cam.depth(pw)[0] - (0.6 if st['h'] > 0.02 else 0.0)        # a hop clears whatever it passes over
        items.append((dz, 'ball', k))
        if k in HOPPER and in_fight and crook_state(k, t) is not None:  # BOCAL HOOK: over both balls
            items.append((-1e9, 'crook', k))
    items.sort(key=lambda it: -it[0])
    drawn = {}
    late = [k_ for k_ in WRECKER if in_fight and fs[k_]['vis'] and fs[k_]['h'] > 0.02
            and max(fs[k_]['p'][0], fs[k_]['p'][1]) > S.A - R * 1.05]      # v3: up over a front rope: drawn after it

    def draw_fighter(obj):
        st = fs[obj]
        arr, depth = fr.array(), fr.depth()
        if obj in WRECKER and T.T_FIGHT <= t < T_HS:            # v2: smear + trail, the harness ring's side
            arr = draw_swing_fx(arr, cam, obj, t)
            cs_ = cranes[obj]
            if cs_ is not None and cs_['attached']:
                a_ = cam.wp(S.to_world(st['p'][0], st['p'][1], R + st['h'])[None])[0]
                b_ = cam.wp(cs_['block'][None])[0]
                d_ = b_ - a_
                st['ring_dir'] = d_ / (np.linalg.norm(d_) + 1e-9)
        drawn[obj] = draw_ball(arr, depth, cam, obj, st, t)
        fr.set_array(arr, depth)
    for dz, kind, obj in items:
        if kind == 'post':
            draw_post(obj)
        elif kind == 'ball' and obj in late:
            continue
        elif kind == 'ball':
            st = fs[obj]
            arr, depth = fr.array(), fr.depth()
            if obj in WRECKER and T.T_FIGHT <= t < T_HS:            # v2: smear + trail, the harness ring's side
                arr = draw_swing_fx(arr, cam, obj, t)
                cs_ = cranes[obj]
                if cs_ is not None and cs_['attached']:
                    a_ = cam.wp(S.to_world(st['p'][0], st['p'][1], R + st['h'])[None])[0]
                    b_ = cam.wp(cs_['block'][None])[0]
                    d_ = b_ - a_
                    st['ring_dir'] = d_ / (np.linalg.norm(d_) + 1e-9)
            drawn[obj] = draw_ball(arr, depth, cam, obj, st, t)
            fr.set_array(arr, depth)
        elif kind == 'crook':
            st = fs[obj]
            arr, depth = fr.array(), fr.depth()
            a_, ext, curl, g_ = crook_state(obj, t)
            arr = ST.draw_crook(arr, cam, st['p'], a_, ext, R, curl, R + st['h'], COL[obj], glow=g_)
            fr.set_array(arr, depth)
    # ---------------- front ropes + posts
    draw_rope_side(0)
    draw_rope_side(1)
    draw_post('front')
    for obj in late:
        draw_fighter(obj)
    arr, depth = fr.array(), fr.depth()
    for k_ in (WRECKER if not crane_first else ()):                 # v2: the crane, over the ring
        arr = draw_crane(arr, cam, cranes[k_], COL[k_])
    # ---------------- hop arcs, the honks' fronts, backstab slashes, MOUNTAIN KING's dance, shards, sparks
    for k_, p0, p1, fade, peak in live_hops(t):
        arr = ST.draw_hop_arc(arr, cam, p0, p1, fade, COL[k_], R, peak)
    if in_fight:
        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][i]:
            arr = ST.draw_honk_front(arr, cam, cx, cz, r, rmax, COL[owner], kind='shockwave' if kind == 'shockwave' else 'rattle')
    for k_, x_, z_, a_, age in live_slashes(t):
        arr = ST.backstab_slash(arr, cam, x_, z_, a_, age, COL[k_], y=R)
    mk = mk_state(t) if in_fight else None
    if mk is not None:
        k_m, c_m, a0_m, n_m, kk_m = mk
        ab_m = CFG[k_m]['abilities']['low']
        pts = ST.mk_trail(cam, c_m, a0_m, ab_m['step'], n_m, R, ab_m['gap'])
        ccol = np.clip(COL[k_m] * 0.7 + 90, 0, 255)
        for j, (x_, y_) in enumerate(pts):
            xi, yi = int(round(x_)), int(round(y_))
            if 2 <= xi < W - 2 and 2 <= yi < H - 2:
                kj = kk_m * (0.5 + 0.5 * (j == len(pts) - 1))
                arr[yi - 1:yi + 2, xi - 1:xi + 2] = arr[yi - 1:yi + 2, xi - 1:xi + 2] * (1 - kj) + ccol * kj
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
        elif kind in ('wood', 'reed'):                             # a stick of the body / a cane blade
            ln_, wd_ = (0.26, 0.035) if kind == 'wood' else (0.12, 0.03)
            q4 = [np.array([sx_, sz_]) + d * ln_ * a1 + nn * wd_ * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            layer_blend(arr, [sp], WOOD if kind == 'wood' else CANE, 1.0)
        else:                                                      # key (silver chip), bell (a black curved chip)
            col = NICKEL if kind == 'key' else WOOD * 1.3
            sz = size * (0.7 if kind == 'key' else 1.5)
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
        if e[7] in ('quake', 'sling', 'hook', 'shockwave', 'rattle', 'wreck', 'crash') or (e[7] == 'stomp' and e[8] == 1):
            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]
            big = e[7] in ('quake', 'sling', 'shockwave', 'wreck', 'crash')
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
    # a hooked fighter whipping round (after the grade: crisp, in the hooker's colour)
    for k, (s_, rp) in drawn.items():
        st = fs[k]
        if in_fight:
            img = ST.swirl(img, s_[0], s_[1], rp, st['swirl'], t, COL[1 - k])
    img = post_fx(img, cam, t, fs, drawn)
    img = np.clip(img, 0, 255).astype(np.uint8)
    img = np.ascontiguousarray(img)
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
