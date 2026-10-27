"""DRUMS (orange), as built in #28 Drums vs Violin (its only episode so far): a drumhead ring blasts out of its ball on
every beat (weapon kind 'pulse', +reach per landed pulse), so its attacks are the groove; DRUM ROLL (haste x4: pulses
on the 16ths for a bar), CRASH CYMBAL (shield: a golden ride around the ball; blades bounce off it), DOUBLE KICK (slam:
a giant kick-pedal beater taps an 8th early, then stomps the predicted spot).

Look (#28): the ball thumps on every pulse; each pulse a drumhead ring (cream skin band, a hoop in its colour, six
chrome lugs) expanding to its reach, lighting the canvas under it; a faint dashed ring on the canvas at its current
reach; the cymbal's back and front halves around the ball (lathe grooves, a glint sweeping round); the beater (felt head,
chrome shaft) over the target, which darkens with a flashing rim until the stomp, then a ring; on its drum solo (if it
wins) rings around the champion, sized by drum piece. It shatters into drumhead skin, sticks, cymbal bits and shell
chips. #28 was made before the v2 effects (attack lights, bursts, particles): it has none of its own yet.
Sound (#28, the Pixel Pitch drum kit's voice): every pulse is a drum (kick on beats 1 and 3, snare on 2 and 4: the
fight IS the groove); DRUM ROLL's 16th pulses a snare roll; every landed pulse adds a tom fill; the leader adds off-beat
hats; hurt, the beat thins (fills go < 45 HP, the snare < 25 HP: only beat 4, < 12 HP a ghost), leaving a lonely kick.
The count-in is the drummer's sticks + a snare pickup into a crash on FIGHT!; a crash on the K.O.; its solo is drum
pieces (config SOLO: [(beat, 'K' / 'S' / 'T1' / 'T2' / 'T3' / 'C', velocity)]; else #28's solo); the last bar a roll
and kicks into the seam. Bus 'kit' (as #28).
Draw code from projects/ballfight_drums_vs_violin/styles.py, layers from its render.py, sound from its audio.py (#28),
copied unchanged."""
import math

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, BAYER_FULL
import scene as S

from . import base

R = S.R_BALL

# ---------------------------------------------------------------- drumhead rings (#28 styles.py, unchanged)
SKIN = np.array([246, 238, 216])
CHROME = np.array([214, 220, 236])


def annulus_polys(cam, cx, cz, r_in, r_out, y, n=36):
    """screen quads of a flat ring (no holes needed: one quad per segment)"""
    a = np.linspace(0, 2 * math.pi, n + 1)
    oi = S.to_world(cx + r_out * np.cos(a), cz + r_out * np.sin(a), y)
    ii = S.to_world(cx + r_in * np.cos(a), cz + r_in * np.sin(a), y)
    so, si = cam.wp(oi), cam.wp(ii)
    return [[so[j], so[j + 1], si[j + 1], si[j]] for j in range(n)]


def mask_of(polys):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        d.polygon([tuple(q) for q in pts], fill=255)
    return np.asarray(m, np.float32) / 255.0


def pulse_state(t, t0, reach, speed):
    """(radius beyond the ball's centre, alpha, expanding?) of a pulse fired at t0 that stops at R + reach"""
    u = t - t0
    if u < 0:
        return None
    r = R + min(reach, speed * u)
    t_full = reach / speed
    a = 1.0 if u <= t_full else max(0.0, 1 - (u - t_full) / 0.22)
    if a <= 0:
        return None
    return r, a, u <= t_full


def draw_drumhead(arr, cam, cx, cz, r, a, col, main=True, y=0.34):
    """one pulse ring: hoop (fighter colour) on the outside, the cream skin band inside, chrome lugs on the rim"""
    w_hoop, w_skin = 0.07, 0.12 if main else 0.07
    k = a * (1.0 if main else 0.65)
    m_skin = mask_of(annulus_polys(cam, cx, cz, max(R * 0.9, r - w_hoop - w_skin), r - w_hoop, y))
    m_hoop = mask_of(annulus_polys(cam, cx, cz, r - w_hoop, r, y))
    dz = np.floor(BAYER_FULL * 0.9 + 0.55 * k * 2) / 2 if k < 0.6 else 1.0
    arr += (SKIN[None, None] - arr) * (m_skin * 0.62 * k * dz)[..., None]
    arr += (np.clip(np.array(col) * 1.1 + 30, 0, 255)[None, None] - arr) * (m_hoop * 0.92 * k)[..., None]
    if main and k > 0.3:                                      # six lugs on the hoop
        for j in range(6):
            ang = j * math.pi / 3 + 0.3
            s = cam.wp(S.to_world(cx + r * math.cos(ang), cz + r * math.sin(ang), y + 0.02)[None])[0]
            xi, yi = int(round(s[0])), int(round(s[1]))
            if 0 <= xi < W and 0 <= yi < H:
                arr[yi, xi] = arr[yi, xi] * (1 - k) + CHROME * k


# ---------------------------------------------------------------- CRASH CYMBAL (shield) (#28 styles.py, unchanged)
BRONZE = np.array([226, 176, 70])
BRONZE_DK = np.array([150, 104, 34])


def cymbal_polys(cam, cx, cz, rp_world, y, half):
    """the cymbal's ring (inner 0.92 R .. outer 1.62 R) split in the back half (half=0) and front half (half=1) as seen by
    the camera, so the ball can sit inside it in painter order"""
    n = 40
    r_in, r_out = R * 0.92, R * 1.62
    a = np.linspace(0, 2 * math.pi, n + 1)
    oi = S.to_world(cx + r_out * np.cos(a), cz + r_out * np.sin(a), y)
    ii = S.to_world(cx + r_in * np.cos(a), cz + r_in * np.sin(a), y)
    so, si = cam.wp(oi), cam.wp(ii)
    dz_o = cam.depth(oi)
    c = cam.depth(S.to_world(cx, cz, y)[None])[0]
    polys, ang = [], []
    for j in range(n):
        front = (dz_o[j] + dz_o[j + 1]) / 2 < c
        if front == bool(half):
            polys.append([so[j], so[j + 1], si[j + 1], si[j]])
            ang.append((a[j] + a[j + 1]) / 2)
    return polys, ang


def draw_cymbal(arr, cam, cx, cz, t, k, half, y=R * 1.02):
    """k: 0..1 strength (fades in / out); a glint sweeps around as it spins"""
    polys, ang = cymbal_polys(cam, cx, cz, None, y, half)
    if not polys:
        return
    spin = t * 9.0
    for pts, a in zip(polys, ang):
        g = 0.5 + 0.5 * math.cos(a - spin)
        g2 = max(0.0, math.cos(a - spin)) ** 18
        col = BRONZE_DK + (BRONZE - BRONZE_DK) * g + np.array([255, 250, 230]) * 0.35 * g2
        m = mask_of([pts])
        arr += (np.clip(col, 0, 255)[None, None] - arr) * (m * 0.88 * k)[..., None]
    for rr in (1.15, 1.4):                                    # lathe grooves: two thin darker rings
        a = np.linspace(0, 2 * math.pi, 41)
        P = S.to_world(cx + R * rr * np.cos(a), cz + R * rr * np.sin(a), y + 0.005)
        s = cam.wp(P)
        dz = cam.depth(P)
        c = cam.depth(S.to_world(cx, cz, y)[None])[0]
        im = Image.new('L', (W, H), 0)
        d = ImageDraw.Draw(im)
        for j in range(40):
            if ((dz[j] + dz[j + 1]) / 2 < c) == bool(half):
                d.line([tuple(s[j]), tuple(s[j + 1])], fill=255)
        mk = np.asarray(im, np.float32) / 255.0
        arr += ((BRONZE_DK * 0.8)[None, None] - arr) * (mk * 0.6 * k)[..., None]


# ---------------------------------------------------------------- DOUBLE KICK (slam): the beater (#28 styles.py, unchanged)
FELT = np.array([240, 234, 220])
SHAFT = np.array([200, 206, 220])


def beater_height(t, t0, t_slam, beat):
    """the beater's head height: hovers high during the telegraph, a tap an 8th before the hit (the 'double'), then the
    real stomp; stays down briefly, then lifts away"""
    tap = t_slam - beat / 2
    if t < t0:
        return None
    if t < tap - 0.10:
        return 3.2 - 0.4 * min(1.0, (t - t0) / 0.3)
    if t < tap:
        u = (t - (tap - 0.10)) / 0.10
        return 2.8 - 1.9 * u * u
    if t < tap + 0.08:
        return 0.9 + 1.4 * ((t - tap) / 0.08)
    if t < t_slam - 0.08:
        return 2.3 + 0.6 * min(1.0, (t - tap - 0.08) / 0.1)
    if t < t_slam:
        u = (t - (t_slam - 0.08)) / 0.08
        return 2.9 - 2.5 * u * u
    if t < t_slam + 0.25:
        return 0.40
    if t < t_slam + 0.6:
        return 0.40 + 3.0 * ((t - t_slam - 0.25) / 0.35) ** 2
    return None


def draw_beater(arr, cam, cx, cz, rad, h, alpha=1.0):
    """felt head (a squat disc seen from above) + chrome shaft up toward the pedal's pivot (off to the back)"""
    head_r = rad * 0.55
    top = S.to_world(cx, cz, h + 0.25)
    piv = S.to_world(cx - 0.9, cz + 0.9, h + 3.4)
    s_top, s_piv = cam.wp(np.array([top, piv]))
    im = Image.new('L', (W, H), 0)
    ImageDraw.Draw(im).line([tuple(s_top), tuple(s_piv)], fill=255, width=3)
    mk = np.asarray(im, np.float32) / 255.0
    arr += (SHAFT[None, None] - arr) * (mk * alpha)[..., None]
    a = np.linspace(0, 2 * math.pi, 25)[:-1]
    for yy, col, rr in ((h, FELT * 0.72, head_r), (h + 0.24, FELT, head_r * 0.96)):
        P = S.to_world(cx + rr * np.cos(a), cz + rr * np.sin(a), yy)
        m = mask_of([cam.wp(P)])
        arr += (col[None, None] - arr) * (m * alpha)[..., None]
    P = S.to_world(cx + head_r * 0.45 * np.cos(a), cz + head_r * 0.45 * np.sin(a), h + 0.245)
    m = mask_of([cam.wp(P)])
    arr += (np.array([200, 60, 52])[None, None] - arr) * (m * 0.5 * alpha)[..., None]      # the felt's worn red spot


PIECE = {'K': 36, 'S': 38, 'T1': 48, 'T2': 45, 'T3': 41, 'C': 49}                  # the kit voice's pieces (#28)
SOLO_PIECE_REACH = {'K': 1.5, 'S': 1.1, 'T1': 0.9, 'T2': 1.1, 'T3': 1.3, 'C': 2.3}  # its solo's rings, by piece
DEFAULT_SOLO = [(0, 'C', 1.0), (0, 'K', 1.0), (.5, 'S', .8), (.75, 'S', .6), (1, 'K', .9), (1.5, 'T1', .8),
                (1.75, 'T1', .7), (2, 'T2', .9), (2.25, 'T2', .7), (2.5, 'T3', .9), (2.75, 'T3', .8), (3, 'K', 1.0),
                (3.25, 'S', .7), (3.5, 'S', .8), (3.75, 'S', .9), (4, 'C', .9), (4, 'K', 1.0), (4.5, 'K', .8),
                (4.75, 'S', .8), (5, 'K', .9), (5.25, 'T1', .7), (5.5, 'T2', .8), (5.75, 'T3', .9), (6, 'S', .9),
                (6.25, 'S', .5), (6.5, 'S', .7), (6.75, 'S', .9), (7, 'K', 1.0), (7.25, 'T3', .8), (7.5, 'T2', .8),
                (7.75, 'T1', .8), (8, 'C', 1.0), (8, 'K', 1.0), (8.5, 'S', .9), (9, 'K', .9), (9.25, 'K', .8),
                (9.5, 'S', .9), (10, 'T1', .8), (10.25, 'T2', .8), (10.5, 'T3', .9), (10.75, 'T3', .9), (11, 'C', 1.0),
                (11, 'K', 1.0)]                                   # #28's drum solo (beat, piece, velocity)


def solo_pieces(T, name):
    """its solo as (beat, piece, velocity): the config's if it is written in pieces, else #28's"""
    solo = T.SOLO.get(name) or DEFAULT_SOLO
    return solo if all(isinstance(p, str) and p in PIECE for _, p, _ in solo) else DEFAULT_SOLO


# ===================================================================================================== look (#28)
class Look(base.Look):
    shards = ['skin'] * 5 + ['stick'] * 2 + ['cymbit'] * 4 + ['shell'] * 3
    spark_big = ('pulse', 'slam')
    spark_flash = ('pulse', 'slam')
    flash_hits = ('pulse', 'slam')
    fringe_hits = ()                                 # #28 had no fringing
    rope_kinds = {}

    def setup(self):
        P, k = self.P, self.k
        self.PULSE_EV = []
        for e in P.EV:
            if e[1] == 'pulse' and e[2] == k:
                i = int(min(P.NSIM - 1, max(0, round((e[0] - self.T.T_FIGHT) * self.T.FPS))))
                self.PULSE_EV.append((e[0], k, e[3], e[4], float(P.RUN['P'][i, k, 0]), float(P.RUN['P'][i, k, 1]),
                                      self.cfg['weapon']['speed']))
        self.PULSE_T = np.array([p[0] for p in self.PULSE_EV]) if self.PULSE_EV else np.zeros(0)
        self.SHIELD_DUR = [self.cfg['abilities'][s]['dur_beats'] * self.T.BEAT for s in ('unlock1', 'unlock2', 'low')
                           if self.cfg['abilities'][s]['kind'] == 'shield']

    # ---------------------------------------------------------------- the rings (pulses + its solo's rings)
    def solo_pulses(self, t):
        """the drum solo's rings around the champion: one per solo hit, reach by drum piece (crash biggest)"""
        P, T = self.P, self.T
        if P.WIN != self.k:
            return []
        out = []
        for bt, piece, vel in solo_pieces(T, self.name):
            t0 = P.T_SOLO + bt * T.BEAT
            if t0 <= t < t0 + 0.5:
                out.append((t0, self.k, piece != 'T1', SOLO_PIECE_REACH[piece] * (0.7 + 0.3 * vel), 9.0))
        return out

    def rings(self, F):
        """(pulse_list, t_rings) for this frame (cached): the pulses still showing (rings freeze with the hit-stop,
        then go) and the solo's rings"""
        key = ('drums_rings', self.k)
        if key in F.__dict__:
            return F.__dict__[key]
        P, T, t = self.P, self.T, F.t
        t_rings = min(t, P.T_KO)
        pulse_list = []
        if T.T_FIGHT <= t < P.T_HS + 0.3 and len(self.PULSE_T):
            j0, j1 = np.searchsorted(self.PULSE_T, t_rings - 0.6), np.searchsorted(self.PULSE_T, t_rings, 'right')
            pulse_list = [self.PULSE_EV[j][:7] for j in range(j0, j1)]
            if t >= P.T_HS:
                fade_k = 1 - (t - P.T_HS) / 0.3
                pulse_list = [p for p in pulse_list if fade_k > 0]
        pulse_list = pulse_list + [(t0, k, main, reach, float(P.fighter(k, t)['p'][0]), float(P.fighter(k, t)['p'][1]),
                                    spd) for t0, k, main, reach, spd in self.solo_pulses(t)]
        if t >= P.T_HS:
            t_rings = t if not (P.T_HS <= t < P.T_HS + 0.3) else t_rings
        F.__dict__[key] = (pulse_list, t_rings)
        return F.__dict__[key]

    def state(self, kk, st, t, phase, gl=0.0):
        if phase == 'fight' and kk == self.k and len(self.PULSE_T):     # the drum ball thumps on every pulse
            tc = min(t, self.P.T_KO)
            j = np.searchsorted(self.PULSE_T, tc, 'right') - 1
            if j >= 0:
                st['glow'] = 0.45 * math.exp(-(tc - self.PULSE_T[j]) / 0.09)

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'hit' and e[7] in ('pulse', 'slam'):
            return {'pulse': 0.9, 'slam': 3.5}[e[7]]
        if e[1] == 'slam':
            return 2.5
        return None

    def hit_glow(self, e):
        return (14, 0.7) if e[7] in ('pulse', 'slam') else None

    # ---------------------------------------------------------------- the canvas
    def canvas(self, Ic, C):
        """its reach (a faint dashed ring that grows) and each pulse's light under its ring"""
        P, k = self.P, self.k
        sxp, szp, ys, xs = C.sxp, C.szp, C.ys, C.xs
        st = C.fs[k]
        if C.in_fight and st['vis']:
            reach = float(P.RUN['BL'][C.i, k])
            rd = np.hypot(sxp - st['p'][0], szp - st['p'][1])
            ring = np.clip(1 - np.abs(rd - (R + reach)) / 0.05, 0, 1)
            dash = (np.floor((np.arctan2(szp - st['p'][1], sxp - st['p'][0]) + math.pi) * 9) % 2) == 0
            Ic += (ring * dash * 0.20)[:, None] * (P.COL[k] / 255.0 + 0.2)[None]
        pulse_list, t_rings = self.rings(C)
        for t0, kk, main, reach, cx, cz, spd in pulse_list:
            ps = pulse_state(t_rings, t0, reach, spd)
            if ps is None:
                continue
            r, a, grow = ps
            rd = np.hypot(sxp - cx, szp - cz)
            band = np.clip(1 - np.abs(rd - r) / 0.22, 0, 1) * (rd < r + 0.05)
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (band * a * (0.55 if main else 0.3))[:, None] * (P.COL[kk] / 255.0 * 0.9 + 0.25)[None]
        return Ic

    def canvas_after(self, Ic, C):
        """DOUBLE KICK: the target darkens (a flashing rim) until the stomp, then a ring"""
        P, t = self.P, C.t
        sl = P.RUN['SLAMS'][C.i] if C.in_fight and 'SLAMS' in P.RUN else []
        for owner, cx, cz, t0, t_slam, rad in sl:
            if owner != self.k:
                continue
            rd = np.hypot(C.sxp - cx, C.szp - cz)
            if t < t_slam:
                u = (t - t0) / (t_slam - t0)
                m = rd < rad
                Ic[m] *= 1 - 0.55 * u
                ring = np.abs(rd - rad) < 0.05
                Ic[ring] += (P.COL[owner] / 255.0) * (0.4 + 0.5 * (math.sin(t * 40) > 0))
            else:
                u = (t - t_slam) / 0.6
                band = np.clip(1 - np.abs(rd - (rad + 1.6 * u)) / 0.2, 0, 1) * (1 - u)
                Ic += band[:, None] * np.array([0.9, 0.9, 1.0])[None]
        return Ic

    # ---------------------------------------------------------------- the painter pass
    def items_ball(self, F, st):
        """CRASH CYMBAL: its back half behind the ball, its front half in front"""
        if st['shield'] > 0 and self.SHIELD_DUR:
            dz = F.cam.depth(S.to_world(st['p'][0], st['p'][1], R))[0]
            return [(dz + 0.03, ('cym', (st, 0))), (dz - 0.03, ('cym', (st, 1)))]
        return []

    def items_free(self, F):
        """the drumhead rings, drawn first (#28: right after the back ropes, under the balls)"""
        pulse_list, t_rings = self.rings(F)
        return [(1e9, ('rings', None))] if pulse_list else []

    def draw_item(self, F, payload):
        P, cam, fr, t = self.P, F.cam, F.fr, F.t
        what, obj = payload
        arr, depth = fr.array(), fr.depth()
        if what == 'rings':
            pulse_list, t_rings = self.rings(F)
            for t0, kk, main, reach, cx, cz, spd in pulse_list:
                ps = pulse_state(t_rings if t0 < P.T_KO else t, t0, reach, spd)
                if ps is not None:
                    draw_drumhead(arr, cam, cx, cz, ps[0], ps[1], P.COL[kk], bool(main))
        else:
            st, half = obj
            dur = self.SHIELD_DUR[0]
            k_ = min(1.0, st['shield'] / 0.2, (dur - st['shield']) / 0.12 + 0.2)
            draw_cymbal(arr, cam, st['p'][0], st['p'][1], t, k_, half)
        fr.set_array(arr, depth)

    def over_ring(self, F, arr, depth):
        """DOUBLE KICK: a tap an 8th early, then the stomp"""
        P = self.P
        if F.in_fight and 'SLAMS' in P.RUN:
            for owner, cx, cz, t0, t_slam, rad in P.RUN['SLAMS'][F.i]:
                if owner != self.k:
                    continue
                h = beater_height(F.t, t0, t_slam, self.T.BEAT)
                if h is not None:
                    a = min(1.0, (F.t - t0) / 0.2) * (1.0 if F.t < t_slam + 0.4 else max(0.0, 1 - (F.t - t_slam - 0.4) / 0.2))
                    draw_beater(arr, F.cam, cx, cz, rad, h, a)
        return arr, depth

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        """sticks; skin (drumhead), cymbit (bronze), shell (its colour): tumbling flat chips"""
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind == 'stick':
            Lk, Wk = 0.42, 0.05
            c = np.array([214, 172, 112])
            q4 = [np.array([sx_, sz_]) + d * Lk / 2 * a1 + nn * Wk / 2 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], c, 1.0)
            return arr
        col = {'skin': SKIN, 'cymbit': BRONZE, 'shell': P.COL[self.k] * 0.8}[kind]
        sz = size * (1.4 if kind == 'skin' else 1.0)
        tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
               np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        shade_k = 0.75 + 0.25 * math.cos(rot * 2)
        P.layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
        return arr

    def focus_points(self, t, st):
        return []


# ===================================================================================================== sound (#28)
def sticks(A, vel=1.0):
    """the drummer clicking the sticks together for the count"""
    n = int(0.09 * A.SR)
    t = A.tt(n)
    x = A.bp(np.random.default_rng(9).normal(0, 1, n), 1800, 6500) * np.exp(-t / 0.012)
    x += 0.6 * np.sin(2 * np.pi * 2350 * t) * np.exp(-t / 0.02) + 0.3 * np.sin(2 * np.pi * 3900 * t) * np.exp(-t / 0.01)
    return x * vel


def upright(A, m, dur=0.5, vel=1.0):
    """upright bass pluck (#28's bed): DOUBLE KICK's stomp lands on its low root"""
    SR = A.SR
    n = int((dur + 0.1) * SR)
    f = A.mtof(m)
    t = A.tt(n)
    x = np.sin(2 * np.pi * f * t) + 0.35 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.12) + 0.12 * np.sin(6 * np.pi * f * t) * np.exp(-t / 0.06)
    x *= np.exp(-t / max(0.18, dur * 0.7)) * np.clip(t / 0.003, 0, 1)
    x += 0.25 * A.bp(np.random.default_rng(int(m)).normal(0, 1, n), 300, 1600) * np.exp(-t / 0.008)
    fd = int(0.05 * SR)
    x[-fd:] *= np.linspace(1, 0, fd)
    return A.lp(np.tanh(1.6 * x) / np.tanh(1.6), 1500) * vel


class Sound(base.Sound):
    bus_gain, send = 2.6, 0.15
    band = (60, 6000)
    own_block = True                                 # the bow (or a dart) bounces off the cymbal: a bright chime

    def __init__(self, M, k):
        super().__init__(M, k)
        self.bus = 'kit'                             # (#28's bus; a mood's 'drums' bus stays its own)
        self.side = -1.0 if k == 0 else 1.0          # #28: the left corner
        self.pan = 0.15 * self.side

    def drum(self, piece, vel=0.9):
        """a kit piece in the drum fighter's own Pixel Pitch voice"""
        m = PIECE[piece]
        cap = 1.8 if piece == 'C' else 0.45
        return self.M.note(self.M.CFG[self.k]['pp'], m, 0.2, cap, round(0.5 + 0.5 * min(1.0, vel), 1)) * (0.55 + 0.45 * vel)

    def voice(self, m, dur, vel=0.85):
        return self.drum('T1' if m >= 47 else ('T2' if m >= 43 else 'T3'), vel)

    def roll(self, dur, f0=0.3, step=None, piece='S'):
        A, SR = self.M.A, self.M.SR
        step = step or self.T.S16 / 2
        n = int(dur * SR)
        out = np.zeros(n + SR)
        t = 0.0
        while t < dur - 1e-4:
            a = int(t * SR)
            d_ = self.drum(piece, round(0.4 + 0.6 * (t / dur), 1))
            sn = A.snare(0.05)
            s = d_ * 0.5
            s[:len(sn)] += sn[:len(s)] * 0.5
            s *= (f0 + (1 - f0) * (t / dur) ** 1.5)
            out[a:a + len(s)] += s[:len(out) - a]
            t += step
        return out[:n]

    def title_blast(self, mix, t):
        """a crash + kick as its name slams in, a tom fill down after it (config TITLE_HITS: pieces)"""
        T = self.T
        hits = T.TITLE_HITS.get(self.name, ('C', 'K', 'T1', 'T2', 'T3'))
        mix.place(self.bus, self.drum(hits[0], 1.0), t, 0.40, pan=self.pan)
        for j, pc in enumerate(hits[1:]):
            mix.place(self.bus, self.drum(pc, 1.0 - 0.08 * j), t + j * T.S16 / 2, 0.40 if j == 0 else 0.30, pan=self.pan)

    def count_in(self, mix):
        """the drummer's count: sticks on 3, 2, 1, a snare pickup on the last four 16ths, a crash on FIGHT!"""
        T, A = self.T, self.M.A
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place(self.bus, sticks(A), max(tc, 0.004), 0.30)
        c0 = T.T_FIGHT - T.BAR
        for st, v in ((12, 0.45), (13, 0.6), (14, 0.75), (15, 0.9)):
            mix.place(self.bus, self.drum('S', v), c0 + st * T.S16, 0.32)
        mix.place(self.bus, self.drum('C', 1.0), T.T_FIGHT, 0.40)

    def play(self, mix):
        """the pulses are the kit; landed pulses add tom fills; then its abilities, the cymbal's blocks, the stomps"""
        M, T, A, k = self.M, self.T, self.M.A, self.k
        S16, q16 = T.S16, M.q16
        pan = self.pan
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'pulse' and e[2] == k:
                tq = q16(t)
                hp = M.hp_at(k, t)
                main_ = bool(e[3])
                pos = int(round(tq / T.BEAT)) % 4
                if not main_:                                  # DRUM ROLL: 16th pulses = a snare roll
                    sig, g, _ = self.colour(self.drum('S', 0.5), t, 0.6)
                    mix.place(self.bus, sig, tq, 0.22 * g, pan=pan)
                    continue
                lead = (hp - M.hp_at(self.enemy, t)) / 100.0
                if pos in (0, 2):
                    sig, g, _ = self.colour(self.drum('K', 1.0), t, 0.4)
                    mix.place(self.bus, sig, tq, 0.52 * g)
                else:
                    snare_ok = hp > 25 or (hp > 12 and pos == 3)
                    if snare_ok:
                        sig, g, _ = self.colour(self.drum('S', 0.9), t, 0.6)
                        mix.place(self.bus, sig, tq, 0.40 * g, pan=pan)
                    elif hp > 12:
                        mix.place(self.bus, A.hp(self.drum('S', 0.3), 900), tq, 0.06, pan=pan)   # a ghost of the snare
                if lead > 0.04:                                # the leader: off-beat hats
                    mix.place(self.bus, A.hat(0.03), tq + T.BEAT / 2, 0.10, pan=-0.3 * self.side)
                    if pos == 3:
                        mix.place(self.bus, A.hat(open_=True), tq + T.BEAT / 2, 0.05, pan=-0.3 * self.side)
            elif kind == 'hit' and e[2] == k and e[7] == 'pulse':
                tq = q16(t)
                hp = M.hp_at(k, t)
                fill = ('T1', 'T2', 'T3') if hp > 45 else (('T3',) if hp > 25 else ())
                for j, pc in enumerate(fill):
                    sig, g, _ = self.colour(self.drum(pc, 0.85 - 0.1 * j), t, 0.5)
                    mix.place(self.bus, sig, tq + (j + 1) * S16, 0.30 * g, pan=0.3 * self.side - 0.3 * j * self.side)
                mix.place('fx', A.thud(0.3, 130, 55, seed=int(t * 7)), tq, 0.12)
        root = T.KEY_ROOT - 12                                     # the stomp's low root (#28: D1)
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'block' and e[2] == k:                  # the bow (or a dart) bounces off the cymbal
                mix.place('fx', A.chime(98, 0.5) + A.chime(105, 0.5) * 0.5, q16(t), 0.07)
            elif kind == 'ability' and e[2] == k:
                tq = q16(t)
                if e[4] == 'haste':                            # DRUM ROLL: a crash + kick as it ends
                    mix.place(self.bus, self.drum('C', 0.9), tq + 4 * T.BEAT, 0.28)
                    mix.place(self.bus, self.drum('K', 1.0), tq + 4 * T.BEAT, 0.4)
                elif e[4] == 'shield':                         # CRASH CYMBAL: a crash and its wash while it lasts
                    mix.place(self.bus, self.drum('C', 1.0), tq, 0.42)
                    dur = self.M.CFG[k]['abilities']['unlock2']['dur_beats'] * T.BEAT
                    n = int(dur * M.SR)
                    wash = A.bp(np.random.default_rng(4).normal(0, 1, n), 4000, 12000) * A.adsr(n, 0.05, 0.2, 0.6, 0.4)
                    mix.place('fx', wash, tq, 0.03)
                elif e[4] == 'slam':                           # DOUBLE KICK: the beater rises
                    mix.place('fx', A.whoosh(2 * T.BEAT, 200, 2500), tq, 0.10)
            elif kind == 'slam' and e[2] == k:                 # DOUBLE KICK: the tap an 8th early, then the stomp
                ts = e[0]
                mix.place(self.bus, self.drum('K', 0.75), q16(ts - T.BEAT / 2), 0.40)
                mix.place(self.bus, self.drum('K', 1.0), ts, 0.62)
                mix.place('bass', upright(A, root, 0.8), ts, 0.5)
                mix.place('fx', A.thud(0.6, 90, 32), ts, 0.55)
                mix.place(self.bus, self.drum('C', 0.8), ts, 0.16)

    def ko(self, mix):
        mix.place(self.bus, self.drum('C', 1.0), self.M.T_KO, 0.45)

    def fall_apart(self, mix):
        """the kit collapsing"""
        M = self.M
        for j, pc in enumerate(('C', 'T1', 'T2', 'T3', 'S', 'K')):
            mix.place(self.bus, self.drum(pc, 0.8), M.P.T_HS + 0.08 * j, 0.26, pan=(j - 2.5) / 6)

    def stinger(self, mix, tw):
        for pc, v in (('C', 1.0), ('K', 1.0), ('T3', 0.9)):
            mix.place('solo', self.drum(pc, v), tw, 0.40)
        mix.place('solo', self.drum('T3', 0.8), tw + self.T.S16, 0.26)

    def solo(self, mix, t0):
        T = self.T
        for bt, pc, vel in solo_pieces(T, self.name):
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            mix.place('solo', self.drum(pc, vel), ts, 0.48 if pc != 'C' else 0.36,
                      pan={'T1': -0.25, 'T2': 0.0, 'T3': 0.25}.get(pc, 0.0))

    def reassemble(self, mix):
        T, M = self.T, self.M
        for j in range(8):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / 8)
            mix.place(self.bus, self.drum(('T3', 'T2', 'T1', 'S')[j % 4], 0.5 + 0.06 * j), tj, 0.18)

    def last_bar(self, mix):
        """a roll into the seam, kicks on the bar's push"""
        T = self.T
        t_last = (T.N_BARS - 1) * T.BAR
        mix.place(self.bus, self.roll(T.DUR - t_last + 0.03, 0.15, T.S16 / 2), t_last, 0.30)
        for st in (0, 8, 12, 14):
            mix.place(self.bus, self.drum('K', 0.9), t_last + st * T.S16, 0.42)
