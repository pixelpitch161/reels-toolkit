"""TUBA (indigo), as built in #32 v2 and drawn / played in #36 (its latest episode): OOM-PAH SWALLOW (on "oom" the bell
opens and inhales, sucking the enemy in and swallowing it; on "pah" it spits it out into the ropes; the suction's reach
grows per landed swallow), OOMPAH (haste: twice as many oom-pahs), BRASS WALL (shield: the bell becomes a brass dome),
SUBSONIC (slam: the floor ripples like water).

Look: the brass bell growing out of the ball toward its aim (opens on oom, flares on pah, a pah on every solo note when
it wins), the suction cone on the canvas with air streaks racing in, the swallowed enemy shrinking into the mouth and
the tuba ball bulging while it holds it, the pah's blast (puffs, a smoke ring, the spat enemy's trail in its colour, a
crack where it hits the ropes), BRASS WALL's dome, SUBSONIC's floor ripple, brass shards; the VS card's bell-ring panel.
Sound: the bass line (role 'bass'): every oom is the bar's root held into the pah, every pah the fifth above as a short
blast (gone below 25 HP unless it spits), a spit adds the octave and the cannonball's whoosh, a gulp a bubble pop and a
growl; the leader gets a sub-octave under its oom; OOMPAH rips up an octave, BRASS WALL holds a buzzing root under a
metallic shimmer, SUBSONIC growls up from the sub and lands on the lowest note.

Draw code from projects/ballfight_tuba_vs_flute/styles.py (#32 v2, which #36 imported), copied here unchanged; the
layers and the sound as #36's render.py / audio.py had them. The TUBA has never lost (#32 v3: FLUTE won, #36: TUBA won),
so no delivered episode showed its shatter: the shards are drawn as #32's renderer would have (#36's shard drawer was
#34's, where 'tube' is the trombone's curl), the falling apart and the reassembly play as #36's audio would have."""
import math

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, FPS, BAYER_FULL, ss
from reelkit import fx
import scene as S

from . import base

R = S.R_BALL
BRASS = np.array([232, 184, 78])                 # #32's brass (the bell, the dome, the shards)
BRASS_HI = np.array([255, 236, 160])
BRASS_DK = np.array([150, 102, 36])
STREAK_COL = [(200, 196, 255), (170, 160, 240)]  # the inhale's air streaks (#32)


# ---------------------------------------------------------------- #28's mask helper (as #32 imported it)
def mask_of(polys):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        d.polygon([tuple(q) for q in pts], fill=255)
    return np.asarray(m, np.float32) / 255.0


# ---------------------------------------------------------------- the tuba's bell (OOM-PAH SWALLOW) (#32 styles.py)
def bell_rings(p, th, open_k=0.0, flare=0.0, y=R * 1.05):
    """the bell growing out of the tuba ball toward its aim: [(centre world, radius)] from the throat to the mouth, plus
    the mouth's axis. open_k (0..1) widens the mouth while it inhales, flare (0..1) is the "pah" punch"""
    d = np.array([math.cos(th), math.sin(th)])
    L = 0.50 + 0.12 * open_k + 0.18 * flare
    rm = 0.36 + 0.22 * open_k + 0.16 * flare
    out = []
    for u, rr in ((0.0, 0.13), (0.45, 0.15), (0.75, 0.2), (0.92, rm * 0.78), (1.0, rm)):
        c = p + d * (R * 0.85 + L * u)
        out.append((S.to_world(c[0], c[1], y), rr))
    return out, S.to_world(d[0], d[1], 0.0) - S.to_world(0.0, 0.0, 0.0)


def _ring_pts(c, axis, r, n=14):
    """a vertical circle of radius r around world centre c, facing world direction `axis` (horizontal)"""
    ax = axis / (np.linalg.norm(axis) + 1e-9)
    up = np.array([0.0, 1.0, 0.0])
    side = np.cross(up, ax)
    side /= np.linalg.norm(side) + 1e-9
    a = np.linspace(0, 2 * math.pi, n + 1)[:-1]
    return c[None] + r * (np.cos(a)[:, None] * side[None] + np.sin(a)[:, None] * up[None])


def draw_tuba_bell(fr, cam, L, shade, p, th, open_k, flare, col, glow=0.0):
    """brass flare (banded segments throat -> mouth, lit from the spot side), a bright lip and, when the mouth faces
    the camera, the dark throat inside (indigo-tinted while it inhales)"""
    rings, axis = bell_rings(p, th, open_k, flare)
    pts = [_ring_pts(c, axis, r) for c, r in rings]
    n = len(pts[0])
    for j in range(len(pts) - 1):
        A_, B_ = pts[j], pts[j + 1]
        for i in range(n):
            q = np.array([A_[i], A_[(i + 1) % n], B_[(i + 1) % n], B_[i]])
            nrm = np.cross(q[1] - q[0], q[3] - q[0])
            nrm = nrm / (np.linalg.norm(nrm) + 1e-9)
            if np.dot(nrm, cam.pos - q.mean(0)) < 0:
                nrm = -nrm
            base_ = BRASS * (0.9 + 0.1 * j / 3) + np.array(col) * 0.06
            c = shade(L, nrm[None], q.mean(0)[None], base_, rimk=0.6)[0]
            fr.poly(cam, q, np.clip(c * (1 + 0.5 * glow) + 30 * glow, 0, 255))
    mouth_c, rm = rings[-1]
    facing = np.dot(axis, cam.pos - mouth_c) > 0
    lip = _ring_pts(mouth_c, axis, rm, n=20)
    if facing:
        inner = _ring_pts(mouth_c - axis * 0.02, axis, rm * 0.82, n=20)
        ink = np.array(col) * (0.18 + 0.35 * open_k) + np.array([10, 6, 16])
        fr.poly(cam, inner, np.clip(ink, 0, 255))
    fr.line(cam, np.vstack([lip, lip[:1]]), np.clip(BRASS_HI * (1 + 0.3 * glow), 0, 255), 1)
    return mouth_c


def inhale_light(sxp, szp, p, th, reach, k, t, ys, xs, half=0.8):
    """the suction cone on the canvas: a wedge in front of the bell (half-angle `half` rad, out to the reach) with
    streaks of air racing IN toward the mouth. Returns (bright, dark) per pixel: thin bright streaks over darker gaps"""
    rel_x, rel_z = sxp - p[0], szp - p[1]
    rd = np.hypot(rel_x, rel_z)
    ang = np.arctan2(rel_z, rel_x)
    da = (ang - th + math.pi) % (2 * math.pi) - math.pi
    rmax = R * 2 + reach
    inside = (np.abs(da) < half) & (rd > R * 0.95) & (rd < rmax)
    fall = np.clip(1 - np.abs(da) / half, 0, 1) ** 0.7 * np.clip(1 - (rd - R) / (rmax - R), 0, 1) ** 0.5
    lane = np.sin(da * 9.0) > 0.55                                   # radial lanes
    flow = (rd * 3.2 + t * 9.0 + np.floor(da * 9.0 / math.pi) * 0.37) % 1.0   # dashes travelling inward
    bright = (lane & (flow < 0.28)) * fall * inside
    dark = fall * inside * 0.5
    q = lambda v: np.floor(v * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
    return q(bright) * k, q(dark) * k


# ---------------------------------------------------------------- BRASS WALL: the bell becomes a brass dome (#32 styles.py)
def draw_dome(arr, cam, cx, cz, k, t, col):
    """a brass dome over the tuba ball: banded hemisphere (seen from the 3/4 camera), a gleam sweeping round, a rim
    ring on the canvas; translucent at the edges so the tuba still reads inside"""
    rr = R * 1.45
    a = np.linspace(0, 2 * math.pi, 41)
    for j, (h, f) in enumerate(((0.0, 1.0), (rr * 0.45, 0.9), (rr * 0.75, 0.68), (rr * 0.93, 0.38))):
        P = S.to_world(cx + rr * f * np.cos(a), cz + rr * f * np.sin(a), h)
        m = mask_of([cam.wp(P)])
        g = 0.62 + 0.12 * j
        col_j = BRASS * g + np.array(col) * 0.10
        arr += (np.clip(col_j, 0, 255)[None, None] - arr) * (m * (0.30 + 0.12 * j) * k)[..., None]
    sweep = t * 3.0
    P = S.to_world(cx + rr * 0.7 * math.cos(sweep), cz + rr * 0.7 * math.sin(sweep), rr * 0.55)
    s = cam.wp(P[None])[0]
    yy, xx = int(round(s[1])), int(round(s[0]))
    if 1 <= yy < H - 1 and 1 <= xx < W - 1:
        arr[yy - 1:yy + 2, xx - 1:xx + 2] = arr[yy - 1:yy + 2, xx - 1:xx + 2] * (1 - k) + BRASS_HI * k
    P = S.to_world(cx + rr * np.cos(a), cz + rr * np.sin(a), 0.02)
    im = Image.new('L', (W, H), 0)
    ImageDraw.Draw(im).line([tuple(q) for q in cam.wp(P)], fill=255, width=1)
    mk = np.asarray(im, np.float32) / 255.0
    arr += (BRASS_HI[None, None] - arr) * (mk * 0.8 * k)[..., None]


# ---------------------------------------------------------------- SUBSONIC: the floor ripples like water (#32 styles.py)
def ripple_light(sxp, szp, cx, cz, t, t0, t_slam, rad, ys, xs):
    """telegraph: rings sucked IN toward the predicted spot (a sub note building); the hit: water ripples racing OUT
    across the canvas, fading over ~1.6 s. Returns (light add, albedo darken) per pixel"""
    rd = np.hypot(sxp - cx, szp - cz)
    add = np.zeros_like(rd)
    dark = np.zeros_like(rd)
    if t < t_slam:
        u = (t - t0) / max(1e-6, t_slam - t0)
        for j in range(3):
            ph = (u * 3 + j / 3) % 1.0
            r_ = rad * 1.6 * (1 - ph)
            add += np.clip(1 - np.abs(rd - r_) / 0.05, 0, 1) * math.sin(math.pi * ph) * 0.5
        dark += (rd < rad) * 0.45 * u
    else:
        u = t - t_slam
        if u < 1.6:
            for j in range(4):
                r_ = 0.2 + 3.2 * max(0.0, u - j * 0.12)
                if r_ <= 0.2:
                    continue
                band = np.cos(np.clip((rd - r_) / 0.16, -1.5, 1.5) * math.pi / 3) * (np.abs(rd - r_) < 0.24)
                add += band * (1 - u / 1.6) * (0.75 - 0.12 * j)
                dark += np.clip(1 - np.abs(rd - (r_ - 0.12)) / 0.08, 0, 1) * (1 - u / 1.6) * 0.35
    q = lambda v: np.floor(v * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
    return q(np.clip(add, 0, 1.2)), q(np.clip(dark, 0, 0.8))


# ===================================================================================================== look (#36)
class Look(base.Look):
    shards = ['brass'] * 7 + ['tube'] * 4 + ['valve'] * 3
    spark_flash = ('slam', 'cannon')                 # a 3 px white point where SUBSONIC / the spat enemy lands (#36)
    flash_hits = ('slam', 'wave', 'shockwave', 'cannon')
    fringe_hits = ('slam', 'wave', 'shockwave', 'cannon')

    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.OOMS = [(e[0], e[2], e[3], e[4], e[5]) for e in EV if e[1] == 'oom' and e[2] == k]   # (t0, k, t_in, t_pah, reach)
        self.OOM_T = np.array([o[0] for o in self.OOMS]) if self.OOMS else np.zeros(0)
        self.PAHS = [(e[0], e[2], e[3], e[4]) for e in EV if e[1] == 'pah' and e[2] == k]          # (t, k, caught, angle)
        self.HELD = []                                                                             # (t_gulp, t_spit, k)
        for e in EV:
            if e[1] == 'gulp' and e[2] == k:
                n = [q for q in self.PAHS if q[0] >= e[0] - 1e-6 and q[2] == 1]
                self.HELD.append((e[0], n[0][0] if n else 99.0, k))
        self.SUBS = []                                                                             # (t0, t_slam, cx, cz, rad, k)
        low = self.cfg['abilities']['low']
        for e in EV:
            if e[1] == 'ability' and e[4] == 'slam' and e[2] == k:
                sl = [q for q in EV if q[1] == 'slam' and q[2] == k and q[0] >= e[0]]
                if sl:
                    self.SUBS.append((e[0], sl[0][0], sl[0][3], sl[0][4], low['radius'], k))

    def title_pattern(self, base_):
        """the VS card panel: the bell's mouth seen head-on, rings of brass, centred on its ball (#32 / #36)"""
        import title as TT
        b = TT.BALLS[self.k]
        rr = np.hypot(TT.XX - (b['cx'] - 8.0), TT.YY - (b['cy'] - 12.0))
        s_ = (rr / 24.0) % 1.0
        base_ = base_ * np.where((s_ < 0.5)[..., None], 1.05, 0.90)
        return np.where((np.abs(s_ - 0.5) < 0.04)[..., None], base_ * 0.72 + np.array([255, 214, 120]) * 0.28, base_)

    # ---------------------------------------------------------------- OOM-PAH SWALLOW from the run's events (#32 v2 / #36)
    def held_phase(self, kk, t):
        """('in', u, swallower) while kk is gulped into the bell (0.14 s), ('held', u, ..) inside, ('out', u, ..) for
        0.12 s after it is spat out; None otherwise"""
        for tg, tsp, sw in self.HELD:
            if sw == kk:
                continue
            if tg <= t < tg + 0.14:
                return 'in', (t - tg) / 0.14, sw
            if tg + 0.14 <= t < tsp:
                return 'held', (t - tg) / max(1e-6, tsp - tg), sw
            if tsp <= t < tsp + 0.12:
                return 'out', (t - tsp) / 0.12, sw
        return None

    def holding(self, t):
        """how full its ball looks (0..1): it bulges while the enemy is inside"""
        for tg, tsp, sw in self.HELD:
            if tg <= t < tsp + 0.1:
                return ss(tg, tg + 0.12, t) * (1 - ss(tsp, tsp + 0.1, t))
        return 0.0

    def bell_state(self, t):
        """(open_k, flare, glow) of the bell: it opens while it inhales (oom), flares on pah (bigger with a spit)"""
        open_k = flare = 0.0
        if len(self.OOM_T):
            j = int(np.searchsorted(self.OOM_T, t, 'right')) - 1
            if j >= 0:
                t0, _, t_in, t_pah, reach = self.OOMS[j]
                open_k = ss(t0 - 0.02, t0 + 0.08, t) * (1 - ss(t_in, t_in + 0.15, t))
        for tp, kk, caught, a in self.PAHS:
            if 0 <= t - tp < 0.3:
                flare = max(flare, (1 - (t - tp) / 0.3) ** 1.5 * (1.0 if caught else 0.55))
        return open_k, flare, 0.6 * open_k + 0.8 * flare

    def inhaling(self, t):
        """(reach, strength) while the bell inhales, else None"""
        if not len(self.OOM_T):
            return None
        j = int(np.searchsorted(self.OOM_T, t, 'right')) - 1
        if j < 0:
            return None
        t0, _, t_in, t_pah, reach = self.OOMS[j]
        if t0 <= t < t_in + 0.12:
            return reach, ss(t0, t0 + 0.1, t) * (1 - ss(t_in, t_in + 0.12, t))
        return None

    def mouth(self, t):
        P, k = self.P, self.k
        i = P.sim_i(t)
        p, th = P.RUN['P'][i, k], float(P.RUN['TH'][i, k])
        return p, th, p + (R * 0.85 + 0.5) * np.array([math.cos(th), math.sin(th)])

    def state(self, kk, st, t, phase, gl=0.0):
        if phase != 'fight':
            return
        P, k = self.P, self.k
        if kk != k:                                                 # the enemy, swallowed by the bell
            hv = self.held_phase(kk, t)
            if hv is not None:
                ph, u, sw = hv
                if ph == 'held':
                    st['vis'] = False
                elif ph == 'in':
                    i = P.sim_i(min(t, P.T_KO))
                    pre = P.RUN['P'][P.sim_i(t - u * 0.14 - 1.0 / FPS), kk]
                    th_sw = float(P.RUN['TH'][i, sw])
                    mouth = P.RUN['P'][i, sw] + (R * 0.85 + 0.5) * np.array([math.cos(th_sw), math.sin(th_sw)])
                    st['p'] = pre + (mouth - pre) * (u * u)
                    st['scale'] = 1 - 0.75 * u
                else:
                    st['scale'] = 0.6 + 0.4 * u
            return
        hb = self.holding(t)                                        # it bulges while it holds the enemy
        if hb > 0:
            st['scale'] = 1 + hb * (0.10 + 0.04 * math.sin(2 * math.pi * t * 7))
            st['glow'] = max(st['glow'], 0.55 * hb)

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'hit' and e[7] in ('cannon', 'spit', 'gulp', 'slam'):
            return {'cannon': 2.0, 'spit': 1.2, 'gulp': 0.8, 'slam': 2.5}[e[7]]
        return super().shake(e)                                     # SUBSONIC's 'slam' event: 1.6 (#36)

    def hit_glow(self, e):
        if e[7] in ('cannon', 'gulp', 'slam', 'wave', 'shockwave'):
            return 14, 0.35
        return None

    def fx_setup(self):
        """oom: the bell inhales, air streaks pour into it; pah: a blast of air (and the spat enemy) (#36, #32's)"""
        P, k = self.P, self.k
        up = P.up
        for t0, _, t_in, t_pah, reach in self.OOMS:
            if t0 > P.T_KO:
                break
            p, th, m = self.mouth(t0)
            P.FL.add(t0, up(m[0], m[1], 0.8), P.FL_COL[k], radius=1.8, peak=0.45, decay=0.45)
            rng = np.random.default_rng(int(t0 * 100))
            for j in range(4):
                tj = t0 + (t_in - t0) * j / 4
                if tj > P.T_KO:
                    break
                p, th, m = self.mouth(tj)
                d = np.array([math.cos(th), math.sin(th)])
                nrm = np.array([-d[1], d[0]])
                for q in range(6):
                    src = p + d * (R + reach * rng.uniform(0.5, 1.0)) + nrm * rng.uniform(-1, 1) * reach * 0.6
                    to = m - src
                    P.PA.emit(tj, up(src[0], src[1], R * rng.uniform(0.5, 1.4)), 'streaks', n=1,
                              dir=up(to[0], to[1], 0.0) - up(0, 0, 0), col=STREAK_COL)
        for tp, _, caught, a in self.PAHS:
            if tp > P.T_KO:
                break
            p, th, m = self.mouth(tp)
            pos = up(m[0], m[1], R)
            d3 = up(math.cos(th), math.sin(th), 0.0) - up(0, 0, 0)
            if caught:
                P.BU.add(tp, pos, (255, 214, 120), 1.4)
                P.FL.add(tp, pos, (255, 214, 120), radius=2.0, peak=0.9, decay=0.3)
                P.PA.emit(tp, pos, 'puffs', n=12, dir=d3)
                P.MK.add(tp, 'smoke_ring', m[0], m[1], size=0.7, life=2.5, seed=int(tp * 10), strength=0.6)
                vic = 1 - k
                for j in range(15):                                 # the cannonball's trail, in the enemy's colour
                    tj = tp + j / FPS
                    if tj > P.T_KO:
                        break
                    q = P.RUN['P'][P.sim_i(tj), vic]
                    P.PA.emit(tj, up(q[0], q[1], R), 'streaks', n=1, dir=-d3, speed=(0.3, 0.6),
                              col=[tuple(int(v) for v in P.COL[vic] * 0.5 + 120)])
            else:
                P.PA.emit(tp, pos, 'puffs', n=5, dir=d3)
                P.FL.add(tp, pos, (255, 214, 120), radius=1.4, peak=0.35, decay=0.25)

    def fx_event(self, e):
        P, k = self.P, self.k
        t_ = e[0]
        up = P.up
        if e[1] == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = up(x, z, R)
            if how == 'gulp':                                      # the bell swallows
                P.BU.add(t_, pos, P.COL[att], 0.9)
                P.FL.add(t_, pos, P.FL_COL[att], radius=1.6, peak=0.7, decay=0.3)
                P.PA.emit(t_, pos, 'streaks', n=8, col=[STREAK_COL[0]])
            elif how == 'spit':
                P.PA.emit(t_, pos, 'brass', n=6)
            elif how == 'cannon':                                  # spat into the ropes
                P.BU.add(t_, pos, (255, 236, 200), 1.8)
                P.FL.add(t_, pos, (255, 214, 140), radius=2.2, peak=1.1, decay=0.35)
                P.PA.emit(t_, pos, 'brass', n=22)
                P.MK.add(t_, 'crack', x, z, size=0.6, n=5, life=4.0, seed=int(t_ * 100))
            elif how == 'slam':                                    # SUBSONIC lands on the enemy
                P.BU.add(t_, pos, P.COL[att], 1.6)
        elif e[1] == 'block' and e[2] == k:                        # something bounces off BRASS WALL: brass chips
            P.PA.emit(t_, up(e[4], e[5], R), 'brass', n=5)

    def fx_after(self):
        P = self.P
        up = P.up
        for t0, t_slam, cx, cz, rad, owner in self.SUBS:           # SUBSONIC: the floor ripples like water
            if t_slam > P.T_KO:
                continue
            P.FL.add(t_slam, up(cx, cz, 1.0), P.FL_COL[owner], radius=2.6, peak=1.0, decay=0.6)
            P.MK.add(t_slam, 'water', cx, cz, size=1.8, life=5.0, seed=int(t_slam * 10), strength=0.8)
            P.PA.emit(t_slam, up(cx, cz, 0.05), 'foam', n=16, spread=180, col=[(200, 196, 255), (236, 236, 255)])

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'brass', n=30, spread=180, speed=(0.8, 2.2))

    # ---------------------------------------------------------------- canvas
    def canvas_before(self, Ic, C):
        """the suction cone while the bell inhales (#36 drew it before every other decal: it darkens the bare light)"""
        P, k = self.P, self.k
        st = C.fs[k]
        inh = self.inhaling(C.t) if (C.in_fight and st['vis'] and C.t < P.T_KO) else None
        if inh is not None:
            reach, kk = inh
            br_, dk_ = inhale_light(C.sxp, C.szp, st['p'], st['th'], reach, kk, C.t, C.ys, C.xs)
            Ic *= (1 - 0.45 * dk_)[:, None]
            Ic += (br_ * 1.1)[:, None] * (np.array([0.85, 0.85, 1.2]))[None]
        return Ic

    def canvas_after(self, Ic, C):
        P = self.P
        for t0, t_slam, cx, cz, rad, owner in self.SUBS:           # SUBSONIC: the floor ripples like water
            if t0 <= C.t < t_slam + 1.6 and t0 < P.T_HS:
                add_, dark_ = ripple_light(C.sxp, C.szp, cx, cz, C.t, t0, t_slam, rad, C.ys, C.xs)
                Ic *= (1 - dark_ * 0.6)[:, None]
                Ic += add_[:, None] * (P.COL[owner] / 255.0 * 0.9 + 0.25)[None]
        return Ic

    # ---------------------------------------------------------------- the bell + BRASS WALL's dome (painter)
    def items_ball(self, F, st):
        out = []
        if st['shield'] > 0:
            dz = F.cam.depth(S.to_world(st['p'][0], st['p'][1], R))[0]
            out.append((dz - 0.05, 'dome'))
        if not st['shattered'] and st['scale'] > 0.5:
            rings, _ = bell_rings(st['p'], st['th'])
            out.append((F.cam.depth(rings[-1][0][None])[0], 'bell'))
        return out

    def draw_item(self, F, payload):
        P, k = self.P, self.k
        st = F.fs[k]
        t = F.t
        if payload == 'dome':                                      # BRASS WALL: the bell becomes a brass dome
            ab = self.cfg['abilities']
            slot = 'unlock2' if ab['unlock2']['kind'] == 'shield' else 'unlock1'
            dur = ab[slot]['dur_beats'] * P.T.BEAT
            k_ = min(1.0, st['shield'] / 0.2, (dur - st['shield']) / 0.12 + 0.2)
            arr, depth = F.fr.array(), F.fr.depth()
            draw_dome(arr, F.cam, st['p'][0], st['p'][1], k_, t, P.COL[k])
            F.fr.set_array(arr, depth)
            return
        ok, fl, gl = self.bell_state(t) if t < P.T_KO else (0.0, 0.0, 0.0)
        if k == P.WIN and t >= P.T_HS:                              # the champion's solo: a pah on every solo note
            fl = max([max(0.0, 1 - (t - ts) / 0.25) * 0.7 for ts in P.SOLO_T if 0 <= t - ts] + [0.0])
        draw_tuba_bell(F.fr, F.cam, F.L, P.shade, st['p'], st['th'], ok, fl, P.COL[k], gl)

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        """#32 v3's tuba shatter (the last one delivered): straight brass tubing, valve caps, bell chips"""
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind == 'tube':                                         # a bent brass pipe
            Lk, Wk = 0.46, 0.08
            q4 = [np.array([sx_, sz_]) + d * Lk / 2 * a1 + nn * Wk / 2 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], np.clip(BRASS * (0.8 + 0.2 * math.cos(rot * 2)), 0, 255), 1.0)
            hi = P.screen(cam, np.array([S.to_world(*(np.array([sx_, sz_]) + d * Lk / 2 * a1 - nn * Wk * 0.25), p[1]) for a1 in (-0.8, 0.8)]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            ImageDraw.Draw(im).line([tuple(q) for q in hi], fill=tuple(int(v) for v in BRASS_HI))
            return np.asarray(im).astype(np.float32)
        if kind == 'valve':                                        # a valve cap: a small brass disc with a pearl top
            rr = 0.11
            a8 = np.linspace(0, 2 * math.pi, 9)[:-1]
            sp = P.screen(cam, np.array([S.to_world(sx_ + rr * math.cos(u), sz_ + rr * math.sin(u), p[1]) for u in a8]))
            P.layer_blend(arr, [sp], BRASS, 1.0)
            cq = P.screen(cam, S.to_world(sx_, sz_, p[1])[None])[0]
            xi, yi = int(round(cq[0])), int(round(cq[1]))
            if 0 <= xi < W and 0 <= yi < H:
                arr[yi, xi] = np.array([240, 236, 226])
            return arr
        sz = size * 1.3                                            # brass: tumbling chips of the bell / body
        tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
               np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        shade_k = 0.75 + 0.25 * math.cos(rot * 2)
        P.layer_blend(arr, [sp], np.clip(BRASS * shade_k, 0, 255), 1.0)
        return arr

    # ---------------------------------------------------------------- lens
    def shimmer_after(self, img, cam, t):
        P = self.P
        for t0, t_slam, cx, cz, rad, owner in self.SUBS:           # SUBSONIC's ripples
            if t_slam <= t < t_slam + 1.0 and t_slam < P.T_HS:
                u = t - t_slam
                img = fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), 0.2 + 3.2 * u, 0.4, 3.0, t, k=1 - u)
        for tp, kk, caught, a in self.PAHS:                        # the pah blast
            if caught and 0 <= t - tp < 0.35 and tp < P.T_KO:
                p_, th_, m_ = self.mouth(tp)
                img = fx.shimmer_world(img, cam, S.to_world(m_[0], m_[1], R), 0.3 + 2.0 * (t - tp), 0.25, 2.0, t,
                                       k=1 - (t - tp) / 0.35)
        return img

    def focus_points(self, t, st):
        P = self.P
        if P.T.T_FIGHT <= t < P.T_HS and st['vis'] and not st['shattered']:     # the bell's mouth
            m = st['p'] + (R * 0.85 + 0.5) * np.array([math.cos(st['th']), math.sin(st['th'])])
            return [(S.to_world(m[0], m[1], R), 'bell')]
        return []


# ===================================================================================================== sound (#36)
def sub_drone(A, dur, f0=30.0, f1=48.0):
    """SUBSONIC's telegraph: a sub-bass growl climbing toward the slam (#32)"""
    n = int(dur * A.SR)
    tt = A.tt(n)
    u = tt / dur
    f = f0 + (f1 - f0) * u ** 1.5
    ph = 2 * np.pi * np.cumsum(f) / A.SR
    x = np.sin(ph) + 0.4 * np.sin(2 * ph) * (0.5 + 0.5 * np.sin(2 * np.pi * 9 * tt))
    return np.tanh(2.0 * x) * np.clip(tt / 0.1, 0, 1) * (0.4 + 0.6 * u)


class Sound(base.Sound):
    bus_gain, send = 2.4, 0.08                                     # #36's mix
    band = (40, 260)                                               # the bass line + its buzz (#36's band check)
    colour_args = dict(cut0=70, cut_span=420, drive=4.5, low=True, low_cut=(30, 90))   # keeps its fundamental: raspy,
                                                                                         # a little thinner (#32)

    def __init__(self, M, k):
        super().__init__(M, k)
        self.side = -1 if k == 0 else 1                            # #36: the right corner (+), #32: the left (-)

    def tuba(self, m, beats=1.0, vel=0.85):
        """a tuba note held for `beats` (its Pixel Pitch voice, stretched with a soft buzzing sustain under it) (#32)"""
        M, A = self.M, self.M.A
        T = self.T
        name = M.CFG[self.k]['pp']
        dur = beats * T.BEAT * 0.92
        x = M.note(name, m, round(min(dur, 0.4), 3), round(dur + 0.2, 2), round(vel, 1)).copy()
        if dur > 0.36:
            n = len(x)
            tt = A.tt(n)
            f = A.mtof(m)
            ph = (f * tt) % 1.0
            sus = A.lp(0.6 * (2 * ph - 1) + 0.4 * np.sin(2 * np.pi * f * tt), 700) * 0.22 * vel
            env = np.clip((tt - 0.12) / 0.15, 0, 1) * np.clip((dur - tt) / 0.06, 0, 1)
            x += sus * env * M.inst_gain(name) * 0.5
        return x

    def root(self, t):
        """its oom: the bar's root in A1-G#2 (#36: F2 Db2 Bb1 C2; #32: C2 Ab2 Db2 G2)"""
        return self.T.root_above(self.T.chord_at(t), self.M.CFG[self.k]['register'][0])

    def fifth(self, t):
        """its pah: the fifth above the root, or below when that would climb past C3 (#36: always above, C3 at most;
        #32: G2 Eb2 Ab2 D2)"""
        r = self.root(t)
        return r + 7 if r + 7 <= 48 else r - 5

    # the score, in the order kit.music calls it
    def title_blast(self, mix, t):
        """a low blast: the key's root an octave down under it (#36: F1 + F2), or config TITLE_HITS (first = loudest)"""
        T = self.T
        hits = T.TITLE_HITS.get(self.name, (T.KEY_ROOT - 12, T.KEY_ROOT))
        for j, m in enumerate(hits):
            mix.place(self.bus, self.tuba(m, 2.0, 1.0), t, 0.55 if j == 0 else 0.50)

    def count_in(self, mix):
        """an oom on each count, the count chord's root"""
        T = self.T
        r = T.root_above(T.CHORDS[T.FIGHT_BAR - 1], self.M.CFG[self.k]['register'][0])
        for tc, txt in T.COUNT[:3]:
            mix.place(self.bus, self.tuba(r, 0.8, 0.95), tc, 0.55)

    def play(self, mix):
        M, T, k = self.M, self.T, self.k
        A, EV, q16, SR = M.A, M.EV, M.q16, M.SR
        sd = self.side
        for e in EV:                                               # the bass line: oom = the root, pah = the fifth
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'oom' and e[2] == k:
                tq = q16(t)
                root = self.root(tq + 0.01)
                beats = max(1.0, (q16(e[3]) - tq) / T.BEAT + 0.4)
                sig, g, lead = self.colour(self.tuba(root, beats, 0.9), t, 0.8)
                mix.place(self.bus, sig, tq, 0.62 * g, pan=0.12 * sd)
                n = int((e[3] - t + 0.1) * SR)                     # the inhale: air sucked in, rising into the bell
                if n > 400:
                    tt_ = A.tt(n)
                    air = A.bp(np.random.default_rng(int(t * 10)).normal(0, 1, n), 300, 2400) * (tt_ / tt_[-1]) ** 2
                    mix.place('fx', air, tq, 0.05, pan=0.1 * sd)
                if lead > 0.04:                                    # the leader: a sub-octave under the oom
                    n = int(beats * T.BEAT * 0.9 * SR)
                    tt_ = A.tt(n)
                    sub = np.sin(2 * np.pi * A.mtof(root - 12) * tt_) * A.adsr(n, 0.02, 0.2, 0.7, 0.1)
                    mix.place(self.bus, sub, tq, 0.22 * min(1.0, lead * 4))
            elif kind == 'pah' and e[2] == k:
                tq = q16(t)
                hp = M.hp_at(k, t)
                caught = bool(e[3])
                if hp > 25 or caught:                              # pah: the fifth as a short blast (gone below 25 HP)
                    sig, g, _ = self.colour(self.tuba(self.fifth(tq + 0.01), 0.6, 0.95 if caught else 0.75), t, 0.8)
                    mix.place(self.bus, sig, tq, (0.62 if caught else 0.45) * g, pan=0.12 * sd)
                if caught:                                         # the spit: an octave blast + the cannonball's whoosh
                    mix.place(self.bus, self.colour(self.tuba(self.root(tq + 0.01) + 12, 0.5, 1.0), t, 0.8)[0], tq, 0.34)
                    mix.place('fx', A.whoosh(0.6, 500, 7000), tq, 0.16)
            elif kind == 'gulp' and e[2] == k:                     # gulp: a low bubble pop + a growl from the bell
                tq = q16(t)
                n = int(0.22 * SR)
                tt_ = A.tt(n)
                pop = np.sin(2 * np.pi * np.cumsum(180 * np.exp(-tt_ / 0.06) + 50) / SR) * np.exp(-tt_ / 0.08)
                mix.place('fx', pop, tq, 0.32)
                mix.place(self.bus, self.colour(self.tuba(29, 0.5, 0.9), t, 1.2)[0], tq, 0.30)
            elif kind == 'hit' and e[2] == k and e[7] == 'cannon':  # into the ropes
                tq = q16(t)
                mix.place('fx', A.thud(0.5, 120, 40, seed=int(t * 7)), tq, 0.42)
                mix.place('drums', A.crash(1.2), tq, 0.08)
                M.bed.boom(mix, tq, 0.30)
        ab = M.CFG[k]['abilities']
        for e in EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'block' and e[2] == k:                      # something bangs off BRASS WALL: a dull thud (+ the
                mix.place('fx', A.thud(0.2, 160, 80), q16(t), 0.05)   # ring's shared clank)
            elif kind == 'ability' and e[2] == k:
                name, tq = e[3], q16(t)
                if name == ab['unlock1']['name']:                  # OOMPAH: it rips up an octave into the double time
                    x = self.tuba(self.root(tq + 0.01) + 12, 0.8, 1.0)
                    mix.place(self.bus, self.colour(M.gliss(x, 12, 0.22), t, 0.8)[0], tq, 0.36)
                elif name == ab['unlock2']['name']:                # BRASS WALL: a buzzing root under a metallic shimmer
                    dur = ab['unlock2']['dur_beats'] * T.BEAT
                    mix.place(self.bus, self.colour(self.tuba(self.root(tq + 0.01), dur / T.BEAT, 0.8), t, 0.6)[0],
                              tq, 0.20)
                    n = int(dur * SR)
                    tt_ = A.tt(n)
                    shim = sum(np.sin(2 * np.pi * A.mtof(m) * tt_) for m in (89, 96, 101)) / 3
                    shim = shim * (0.6 + 0.4 * np.sin(2 * np.pi * 6.0 * tt_)) * A.adsr(n, 0.05, 0.3, 0.6, 0.5)
                    mix.place('fx', shim, tq, 0.025)
                    mix.place('drums', A.crash(1.2), tq, 0.08)
                elif name == ab['low']['name']:                    # SUBSONIC: the sub growls up toward the slam
                    tele = ab['low']['tele'] * T.BEAT
                    mix.place(self.bus, sub_drone(A, tele + 0.05), tq, 0.40)
            elif kind == 'slam' and e[2] == k:                     # SUBSONIC lands: a ring-shaking low note
                ts = e[0]
                mix.place(self.bus, self.tuba(29, 2.5, 1.0), ts, 0.70)
                mix.place(self.bus, sub_drone(A, 1.2, 26, 30), ts, 0.30)
                mix.place('fx', A.thud(0.9, 90, 28), ts, 0.65)
                M.bed.boom(mix, ts, 0.6)

    def fall_apart(self, mix):
        """the tuba collapses: a falling groan on the key's root, valves rattling (#36)"""
        M = self.M
        x = self.tuba(self.T.KEY_ROOT, 2.0, 1.0)
        mix.place(self.bus, M.gliss(x, -7, 0.9) * np.linspace(1, 0.3, len(x)), M.P.T_HS, 0.40)
        for j, m in enumerate((77, 72, 82, 69, 75)):
            mix.place('fx', M.clank(m, 20 + j), M.P.T_HS + 0.09 * j, 0.08, pan=(j - 2) / 5)

    def stinger(self, mix, tw):
        """on "TUBA WINS": the root, its octave below and the fifth, held (#36)"""
        r = self.T.KEY_ROOT
        mix.place('solo', self.tuba(r - 12, 2.0, 1.0), tw, 0.55)
        mix.place('solo', self.tuba(r, 2.0, 1.0), tw, 0.50)
        mix.place('solo', self.tuba(r + 7, 2.0, 0.9), tw, 0.30)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:                         # a late K.O. leaves less room: stop at the reassembly
                break
            mix.place('solo', self.tuba(m, d * 0.95, 0.95), ts, 0.62)

    def reassemble(self, mix):
        """its pieces fly home as a rising run: the key's root, the fourth and the fifth above it, the root an octave up,
        the tonic chord's third and fifth (#36: F1 Bb1 C2 F2 Ab2 C3)"""
        from ballfight.kit.episode import parse_chord
        T, M = self.T, self.M
        r = T.KEY_ROOT
        pcs = parse_chord(T.CHORDS[0])[1]
        run = (r - 12, r - 7, r - 5, r, r + (pcs[1] - pcs[0]) % 12, r + 7)
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place(self.bus, self.tuba(m, 0.35, 0.7), tj, 0.32)

    def last_bar(self, mix):
        """the bass climbs the last chord (the dominant) as a dominant seventh from its root under the key's root, into
        the title slam (#36: C2 E2 G2 Bb2; #32: G1 B1 D2 F2)"""
        if self.role != 'bass':
            return
        from ballfight.kit.episode import parse_chord
        T = self.T
        ch = T.CHORDS[-1]
        rpc, pcs = parse_chord(ch)
        r = T.KEY_ROOT - (T.KEY_PC - rpc) % 12
        climb = (r, r + (pcs[1] - pcs[0]) % 12, r + 7, r + 10)
        t_last = (T.N_BARS - 1) * T.BAR
        for bt in range(4):
            mix.place(self.bus, self.tuba(climb[bt], 0.9, 0.85), t_last + bt * T.BEAT, 0.50)
