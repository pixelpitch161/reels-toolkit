"""OBOE (olive), as built in #37: TUNING A (the only debuffer: every bar a narrow tuning beam; a hit knocks the enemy OUT
OF TUNE for 2 bars: its hits land weaker, it wobbles, and every one of the oboe's 8ths stings it), DOUBLE REED (the
reed's blades scissor out and snap), ENDLESS NOTE (the beam held for a bar, sweeping), ILL WIND (a sour gust: fully
out of tune).

Look: its reed points out of the ball at the aim; the A gathers at the tip while the aim locks, then the beam fires
with the A's sine running along it and a lit lane under it; the detuned enemy gets a sour glass, a seasick tilt, a
sour halo on the canvas and a doubled, warped outline; DOUBLE REED's inked blades; ILL WIND's swirling olive ring,
streaks and smoke ring; cane chips.
Sound: the higher, nasal line (role 'line'): its tuning A (A4) with every beam, the A rings out over a sour twin in the
enemy's own voice when it lands, every sting one of its running 8ths (config LINES); ENDLESS NOTE holds the A,
DOUBLE REED is a reed crow + a scissor snip, ILL WIND a bent, beating A over a gust.
Draw code from projects/ballfight_clarinet_vs_oboe/styles.py + render.py (#37), copied unchanged; sound from #37's
audio.py (the new oboe double reed in the Pixel Pitch folder)."""
import math

import numpy as np
from reelkit.config import W, H, BAYER_FULL, ease
from reelkit import fx
import scene as S

from . import base
from ._reeds import (_mask, blend, add, ribbon, ring_exit, beam_lane, CANE, CANE_D, THREAD, INK, WHITE, SOUR,
                     KO_CHIPS, draw_woodwind_shard, tail_theta)

R = S.R_BALL
BEAM_Y = R * 1.0                                                         # the beam / reed height (ball centre)


# ---------------------------------------------------------------- TUNING A (#37 styles.py)
def reed_polys(cam, p, aim, y, R, opening=0.0, length=0.34, half_w=0.07):
    """the oboe's reed pointing out of the ball at `aim`: two cane blades hinged at the rim; opening (rad) spreads them
    (DOUBLE REED's scissors); returns [(screen poly, shade)]"""
    out = []
    base_ = np.asarray(p, float)
    for sgn in (1, -1):
        a = aim + sgn * opening
        d = np.array([math.cos(a), math.sin(a)])
        nrm = np.array([-d[1], d[0]])
        b0 = base_ + np.array([math.cos(aim), math.sin(aim)]) * (R * 0.92) + nrm * 0.03 * sgn
        tip = b0 + d * length
        q = np.array([b0 + nrm * 0.03, tip + nrm * half_w, tip - nrm * half_w * 0.3, b0 - nrm * 0.03])
        out.append((cam.wp(S.to_world(q[:, 0], q[:, 1], np.full(len(q), y + 0.04 * sgn))), 1.0 if sgn > 0 else 0.82))
    return out


def draw_reed(img, cam, p, aim, y, R, opening=0.0, length=0.34, glow=0.0, col=None):
    big = opening > 0.02 or length > 0.4                     # DOUBLE REED: the blades out, inked and edged in olive
    for poly, sh in reed_polys(cam, p, aim, y, R, opening, length, 0.07 + 0.07 * min(1.0, opening * 2)):
        if big:
            img = blend(img, _mask([], [list(poly) + [poly[0]]], 3), INK, 0.85)
        mk = _mask([poly])
        img = blend(img, mk, CANE * sh, 1.0)
        edge = _mask([], [list(poly) + [poly[0]]], 1)
        img = blend(img, edge, (np.asarray(col) * 0.8 if (big and col is not None) else CANE_D * 0.55), 0.9)
    # the green thread where it meets the ball
    q = np.asarray(p) + np.array([math.cos(aim), math.sin(aim)]) * (R * 0.98)
    s_ = cam.wp(S.to_world(q[0], q[1], y)[None])[0]
    xi, yi = int(round(s_[0])), int(round(s_[1]))
    img[max(0, yi - 1):yi + 2, max(0, xi - 1):xi + 2] = THREAD
    if glow > 0.02 and col is not None:                    # the A gathering at the tip before it sounds
        tip = np.asarray(p) + np.array([math.cos(aim), math.sin(aim)]) * (R + length)
        s2 = cam.wp(S.to_world(tip[0], tip[1], y)[None])[0]
        yy, xx = np.mgrid[0:H, 0:W]
        rr = np.hypot(xx - s2[0], yy - s2[1])
        img = add(img, np.exp(-(rr / (3 + 4 * glow)) ** 2).astype(np.float32), np.asarray(col) * 0.6 + 80, 0.55 * glow)
    return img


def draw_beam(img, cam, o, aim, length, width, k, t, col, y, held=False):
    """the tuning A: a narrow beam with the A's sine running along it (scrolling outward), a warm white core"""
    if k <= 0.01:
        return img
    d = np.array([math.cos(aim), math.sin(aim)])
    nrm = np.array([-d[1], d[0]])
    length = min(length, ring_exit(o, d))
    s = np.linspace(0, length, 48)
    P = np.column_stack([o[0] + d[0] * s, o[1] + d[1] * s, np.full(len(s), y)])
    glow = ribbon(cam, P, 0.11 + 0.04 * held)
    img = blend(img, _mask([glow]), np.asarray(col) * 0.7 + 70, 0.30 * k)
    amp = width * 0.75 + 0.02
    wave = o[None] + d[None] * s[:, None] + nrm[None] * (amp * np.sin(2 * np.pi * (s / 0.42 - t * 7.0)))[:, None]
    Wv = np.column_stack([wave, np.full(len(s), y)])
    sp = cam.wp(S.to_world(Wv[:, 0], Wv[:, 1], Wv[:, 2]))
    img = blend(img, _mask([], [sp], 2), np.clip(np.asarray(col) * 0.6 + 110, 0, 255), 0.85 * k)
    core = cam.wp(S.to_world(P[:, 0], P[:, 1], P[:, 2]))
    img = blend(img, _mask([], [core], 1), WHITE, 0.95 * k)
    return img


def wobble_rim(img, x, y, rp, depth, t, col, seed=0):
    """OUT OF TUNE: a doubled, warped outline around the ball (a sour ghost a few px off, its rim waving), stronger
    with the detune depth"""
    if depth <= 0.02:
        return img
    yy, xx = np.mgrid[0:H, 0:W]
    pad = int(rp + 8)
    x0, x1 = max(0, int(x) - pad), min(W, int(x) + pad + 1)
    y0, y1 = max(0, int(y) - pad), min(H, int(y) + pad + 1)
    if x1 <= x0 or y1 <= y0:
        return img
    sub = img[y0:y1, x0:x1]
    X, Y = xx[y0:y1, x0:x1], yy[y0:y1, x0:x1]
    for j, (ox, oy, a_) in enumerate(((2.2, -1.2, 0.85), (-1.8, 1.4, 0.55))):
        sh = depth * (1.0 + 0.6 * math.sin(t * 9.0 + j * 2.0 + seed))
        cx, cy = x + ox * sh, y + oy * sh
        ang = np.arctan2(Y - cy, X - cx)
        rr = np.hypot(X - cx, Y - cy)
        wav = rp * (1.0 + 0.10 * depth * np.sin(5 * ang + t * 13.0 + j * 1.7))
        ring = np.abs(rr - wav) < 0.9
        c = np.asarray(col) * 0.55 + np.array([120, 130, 40]) * 0.45
        sub[ring] = sub[ring] * (1 - a_ * min(1.0, depth * 1.4)) + c * a_ * min(1.0, depth * 1.4)
    return img


class Look(base.Look):
    shards = ['wood'] * 6 + ['key'] * 8 + ['reed'] * 3
    spark_big = ('wave', 'snip', 'stun')
    flash_hits = ('wave', 'snip')
    quiet_hits = ('sting',)
    rope_kinds = {'wave': 1.0, 'shockwave': 0.5, 'stun': 0.5}

    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.TUNES = [(e[0], e[2], np.array([e[3], e[4]]), e[5], e[6], e[7]) for e in EV if e[1] == 'tune' and e[2] == k]
        self.LOCKS = [(e[0], e[2], e[3]) for e in EV if e[1] == 'tune_lock' and e[2] == k]
        self.ENDLESS = []                                                                    # (t0, t1, k)
        for e in EV:
            if e[1] == 'endless' and e[2] == k:
                ends = [q[0] for q in EV if q[1] == 'endless_end' and q[2] == e[2] and q[0] >= e[0]]
                self.ENDLESS.append((e[0], min([e[3]] + ends + [P.T_KO]), e[2]))
        self.SNIP_OPEN = [(e[0], e[2], e[3], e[4]) for e in EV if e[1] == 'snip_open' and e[2] == k]

    def detune_at(self, kk, t):
        """(depth x fade) of ball kk's OUT OF TUNE status at t (from the recorded DET)"""
        P = self.P
        if not (P.T.T_FIGHT <= t < P.T_KO) or 'DET' not in P.RUN:
            return 0.0
        left, dep = P.RUN['DET'][P.sim_i(t), kk]
        return float(dep) * min(1.0, float(left) / 0.25)

    def lock_k(self, t):
        """how far the A has gathered at the reed's tip: from the aim lock to the shot (0..1)"""
        for t0, kk, aim in self.LOCKS:
            if t0 <= t:
                nxt = [q[0] for q in self.TUNES if q[0] >= t0]
                if nxt and t < nxt[0]:
                    return (t - t0) / max(1e-6, nxt[0] - t0)
        return 0.0

    def snip_state(self, t):
        """(aim, opening rad, length, snapping flash) of DOUBLE REED's blades at t, or None"""
        P, k = self.P, self.k
        for t0, kk, t_snap, aim in self.SNIP_OPEN:
            if t0 <= t < t_snap + 0.25 and t0 < P.T_KO:
                ab = self.cfg['abilities']['unlock1']
                i = P.sim_i(min(t, P.T_KO))
                row = P.RUN['SNIP'][i, k] if 'SNIP' in P.RUN else None
                a = float(row[1]) if row is not None and not np.isnan(row[1]) else aim
                if t < t_snap:
                    u = (t - t0) / (t_snap - t0)
                    return a, math.radians(ab['half']) * 0.75 * ease(u), 0.34 + (ab['reach'] - 0.34) * ease(u), 0.0
                u = (t - t_snap) / 0.25
                return a, 0.0, ab['reach'] * (1 - u) + 0.34 * u, 1 - u
        return None

    def state(self, kk, st, t, phase, gl=0.0):
        if phase == 'fight':
            st['det'] = self.detune_at(kk, t)                    # its A knocks the enemy OUT OF TUNE
            if kk == self.k:
                st['lock'] = self.lock_k(t) if t < self.P.T_KO else 0.0
                st['glow'] = max(st['glow'], 0.35 * st['lock'])
        elif phase == 'tail' and kk == self.k:
            tail_theta(self.P, kk, st, t, gl)

    def sprite_rolls(self, kk, roll):
        return [roll + 4 * j for j in range(-3, 4)] if kk == self.enemy else []   # the seasick wobble's steps

    def ball(self, kk, st, t, glass, roll):
        """OUT OF TUNE: the glass sours, the instrument rolls seasick (4-degree steps: cached sprites)"""
        sour = min(1.0, st.get('det', 0.0) * 1.2)
        glass = glass * (1 - 0.35 * sour) + np.array([186, 196, 120]) * 0.35 * sour
        if sour > 0.05:
            roll = roll + 4 * int(round(3.0 * sour * math.sin(t * 9.0 + kk)))
        return glass, roll

    def live_beams(self, F):
        """[(o, aim, len, width, alpha, held)] of its A beams showing at t (a shot lives `hold` s + a 0.12 s fade;
        ENDLESS NOTE's beam rides the oboe's recorded aim)"""
        key = ('beams', self.k)
        if key not in F.__dict__:
            P, t = self.P, F.t
            out = []
            if P.T.T_FIGHT <= t < P.T_HS:
                tc = min(t, P.T_KO)
                for tt, k, o, aim, ln, wd in self.TUNES:
                    hold = self.cfg['weapon']['hold']
                    if tt <= tc < tt + hold + 0.12:
                        kk = 1.0 if tc < tt + hold else 1 - (tc - tt - hold) / 0.12
                        out.append((o, aim, ln, wd, kk, False))
                for t0, t1, k in self.ENDLESS:
                    if t0 <= tc < t1 + 0.12:
                        i = P.sim_i(tc)
                        ab = self.cfg['abilities']['unlock2']
                        a = float(P.RUN['TH'][i, k])
                        p = P.RUN['P'][i, k]
                        o = p + np.array([math.cos(a), math.sin(a)]) * (R + 0.04)
                        kk = min(1.0, (tc - t0) / 0.08) * (1.0 if tc < t1 else 1 - (tc - t1) / 0.12)
                        out.append((o, a, ab['len'], ab['width'], kk, True))
            F.__dict__[key] = out
        return F.__dict__[key]

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'hit' and e[7] in ('tune', 'snip', 'wave'):
            return {'tune': 0.5, 'snip': 1.3, 'wave': 2.0}[e[7]]
        return super().shake(e)

    def spark(self, e):
        P = self.P
        if e[7] == 'sting':                                        # the stings: a few sour flecks, no more
            return P.COL[e[2]] * 0.5 + 110, 3, 1.2, 0.16
        if e[7] in self.spark_big:
            return P.COL[e[2]] * 0.6 + 100, 16, 3.2, 0.3
        return P.COL[e[2]] * 0.6 + 100, 6, 2.0, 0.2

    def hit_glow(self, e):
        if e[7] in ('wave', 'snip'):
            return 14, 0.35
        if e[7] in ('tune', 'stun', 'cut'):
            return 8, 0.2
        return None

    def fringe(self, e):
        return e[7] in ('wave', 'snip')

    def fx_setup(self):
        P = self.P
        up = P.up
        for tt, k, o, aim, ln, wd in self.TUNES:                   # TUNING A: olive light along the beam
            if tt > P.T_KO:
                break
            for f in (0.15, 0.5):
                q = o + np.array([math.cos(aim), math.sin(aim)]) * ln * f
                P.FL.add(tt, up(q[0], q[1], R), P.COL[k], radius=1.4, peak=0.5, decay=0.25)
        for t0, t1, k in self.ENDLESS:                             # ENDLESS NOTE: a held olive glow, re-lit every beat
            tb = t0
            while tb < t1:
                i = P.sim_i(tb)
                p, a = P.RUN['P'][i, k], float(P.RUN['TH'][i, k])
                q = p + np.array([math.cos(a), math.sin(a)]) * 1.6
                P.FL.add(tb, up(q[0], q[1], R), P.COL[k], radius=1.8, peak=0.55, decay=0.35)
                tb += P.T.BEAT

    def fx_event(self, e):
        P = self.P
        t_ = e[0]
        if e[1] == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = P.up(x, z, R)
            if how == 'tune':                                      # the A lands: a sour burst
                P.BU.add(t_, pos, P.COL[att], 0.9)
                P.FL.add(t_, pos, P.COL[att], radius=1.6, peak=0.7, decay=0.3)
                P.PA.emit(t_, pos, 'motes', n=8, col=SOUR)
            elif how == 'endless':
                P.PA.emit(t_, pos, 'motes', n=4, col=SOUR)
            elif how == 'snip':                                    # DOUBLE REED snaps: cane splinters
                P.BU.add(t_, pos, (232, 210, 152), 1.3)
                P.FL.add(t_, pos, P.COL[att], radius=1.8, peak=0.9, decay=0.3)
                P.PA.emit(t_, pos, 'chips', n=12, col=[(232, 210, 152), (196, 168, 110)])
                P.MK.add(t_, 'claw', x, z, size=0.4, angle=0.6, col=(110, 96, 60), life=3.0,
                         n=2, seed=int(t_ * 10), strength=0.6)
            elif how == 'wave':                                    # ILL WIND lands
                P.BU.add(t_, pos, P.COL[att], 1.5)
                P.FL.add(t_, pos, P.COL[att], radius=2.2, peak=1.0, decay=0.4)
                P.PA.emit(t_, pos, 'motes', n=16, spread=180, col=SOUR)
        elif e[1] == 'ability' and e[4] == 'wave':                 # ILL WIND: the gust leaves the oboe
            p = P.RUN['P'][P.sim_i(t_), e[2]]
            P.PA.emit(t_, P.up(p[0], p[1], R * 0.5), 'streaks', n=24, spread=180, col=SOUR, speed=(2.0, 3.5))
            P.MK.add(t_, 'smoke_ring', p[0], p[1], size=1.4, life=3.0, seed=int(t_ * 10), strength=0.5)

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'chips', n=26, spread=180, speed=(0.8, 2.2), col=KO_CHIPS)

    # ---------------------------------------------------------------- canvas, reed, beams
    def canvas(self, Ic, C):
        P = self.P
        for o, a_, ln, wd, kk, held in self.live_beams(C):         # the beams' lit lanes
            lane = beam_lane(C.sxp, C.szp, o, a_, ln, wd, C.ys, C.xs)
            Ic += (lane * 0.42 * kk)[:, None] * (P.COL[self.k] / 255.0 * 0.9 + 0.25)[None]
        for kk in (0, 1):                                          # the sour halo of a detuned ball
            st = C.fs[kk]
            if C.in_fight and st.get('det', 0.0) > 0.02:
                rd = np.hypot(C.sxp - st['p'][0], C.szp - st['p'][1])
                ang_ = np.arctan2(C.szp - st['p'][1], C.sxp - st['p'][0])
                wob = R * 1.35 + 0.08 * np.sin(5 * ang_ - 12 * C.t)
                band = np.clip(1 - np.abs(rd - wob) / 0.05, 0, 1)
                Ic += (band * 0.5 * st['det'])[:, None] * np.array([0.62, 0.72, 0.18])[None]
        return Ic

    def canvas_wave(self, Ic, C, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind == 'stun' or kind == 'pulse':
            return Ic
        rd = np.hypot(C.sxp - cx, C.szp - cz)                      # ILL WIND: a swirling sour ring
        k_ = (1 - r / rmax) * 1.3
        band = np.clip(1 - np.abs(rd - r) / 0.32, 0, 1)
        band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
        swirl = 0.5 + 0.5 * np.sin(np.arctan2(C.szp - cz, C.sxp - cx) * 6 + rd * 3 - C.t * 8)
        Ic += (band * k_ * (0.6 + 0.5 * swirl))[:, None] * (self.P.COL[owner] / 255.0 * 1.3 + 0.2)[None]
        return Ic

    def items_ball(self, F, st):
        if st['shattered']:
            return []
        tip = st['p'] + (R + 0.2) * np.array([math.cos(st['th']), math.sin(st['th'])])
        dz_r = F.cam.depth(S.to_world(tip[0], tip[1], R))[0] + 0.01
        if F.in_fight and self.snip_state(F.t) is not None:        # DOUBLE REED's blades: over both balls
            dz_r = -1e9
        return [(dz_r, 'reed')]

    def draw_item(self, F, payload):
        P, k = self.P, self.k
        st = F.fs[k]
        fr = F.fr
        arr, depth = fr.array(), fr.depth()
        sn = self.snip_state(F.t) if F.in_fight else None
        if sn is not None:
            a_, opening, ln, flash = sn
            arr = draw_reed(arr, F.cam, st['p'], a_, BEAM_Y, R, opening, ln, glow=flash, col=P.COL[k])
        else:
            g_ = st.get('lock', 0.0) if F.in_fight and F.t < P.T_KO else 0.0
            if k == P.WIN and F.t >= P.T_HS:                       # the champion's A on every solo note
                g_ = P.solo_env(F.t)
            arr = draw_reed(arr, F.cam, st['p'], st['th'], BEAM_Y, R, 0.0, 0.34, glow=g_, col=P.COL[k])
        fr.set_array(arr, depth)

    def over_ring(self, F, arr, depth):
        for o, a_, ln, wd, kk, held in self.live_beams(F):
            arr = draw_beam(arr, F.cam, o, a_, ln, wd, kk, F.t, self.P.COL[self.k], BEAM_Y, held)
        return arr, depth

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        return draw_woodwind_shard(self.P, arr, cam, kind, p, rot, size)

    def shimmer(self, img, cam, t, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        return fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.4 if kind == 'wave' else 0.25,
                                3.0 if kind == 'wave' else 2.0, t, k=1 - r / rmax, seed=owner)

    def after_grade(self, img, F, drawn):
        if F.in_fight:                                             # OUT OF TUNE outlines, in the oboe's colour
            for kk, (s_, rp) in drawn.items():
                img = wobble_rim(img, s_[0], s_[1], rp, F.fs[kk].get('det', 0.0), F.t, self.col, seed=kk)
        return img


# ===================================================================================================== sound (#37)
class Sound(base.Sound):
    bus_gain, send = 1.3, 0.3
    band = (900, 3500)                                             # its nasal line
    colour_args = dict(cut0=60, cut_span=260, drive=4.0)
    A4 = 69                                                        # the tuning A: its idiom, whatever the key

    def __init__(self, M, k):
        super().__init__(M, k)
        self.OBM = M.B.module('oboe_lowpoly')                      # the oboe's double reed (Pixel Pitch folder)
        self._vc = {}

    def oboe(self, m, dur, vel=0.85):
        key = (round(m, 3), round(dur, 3), round(vel, 2))
        if key not in self._vc:
            M, OBM = self.M, self.OBM
            x = M.fit(M.B._resample(np.asarray(OBM.double_reed(OBM._midi_hz(m), dur, vel), float)), dur + 0.12, fade=0.03)
            self._vc[key] = x * (0.16 / (np.sqrt((x[:int(0.25 * M.SR)] ** 2).mean()) + 1e-9)) * 0.85
        return self._vc[key]

    def voice(self, m, dur, vel=0.85):
        return self.oboe(m, dur, vel)

    def note(self, m, dur, vel, t):
        return self.colour(self.oboe(m, dur, vel), t)

    def title_blast(self, mix, t):
        """its A, held (the tuning note), with the octave"""
        hits = self.T.TITLE_HITS.get(self.name, (self.A4, self.A4 + 12))
        a_hold = 1.6 * self.T.BEAT
        mix.place(self.bus, self.oboe(hits[0], a_hold, 1.0), t, 0.52)
        mix.place(self.bus, self.oboe(hits[-1], a_hold, 0.9), t, 0.30)

    def play(self, mix):
        M, T, k = self.M, self.T, self.k
        A, q16, S16 = M.A, M.q16, T.S16
        victim = M.SOUNDS[self.enemy]
        o_dropped = False
        ab = M.CFG[k]['abilities']
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'tune' and e[2] == k:                       # the tuning A with every beam
                tq = q16(t)
                sig, g, lead = self.note(self.A4, 0.75 * T.BEAT, 0.95, t)
                mix.place(self.bus, sig, tq, 0.48 * g, pan=0.22)
                if lead > 0.04:
                    mix.place(self.bus, self.note(self.A4 + 12, 0.75 * T.BEAT, 0.6, t)[0], tq, 0.14 * g, pan=0.25)
            elif kind == 'hit' and e[2] == k and e[7] == 'tune':   # it lands: the A rings out against a sour twin
                tq = q16(t)
                mix.place(self.bus, self.note(self.A4 + 12, 0.25, 0.9, t)[0], tq + S16, 0.22, pan=0.2)
                sour = victim.voice(self.A4 - 0.5, 0.3, 0.7) * 0.5                 # in the enemy's own voice
                mix.place(victim.bus, M.colour(sour, self.enemy, t, **victim.colour_args)[0], tq + S16, 0.18, pan=-0.15)
            elif kind == 'hit' and e[2] == k and e[7] == 'sting':  # each sting: one of the oboe's running 8ths
                tq = q16(t)
                hp = M.hp_at(k, t) / 100.0
                o_dropped = M.thin_37(round(tq / S16), 11, hp, o_dropped)
                if o_dropped:
                    continue
                sig, g, _ = self.note(T.line_at(self.name, tq), 0.13, 0.78, t)
                mix.place(self.bus, sig, tq, 0.30 * g, pan=0.25)
            elif kind == 'endless' and e[2] == k:                  # ENDLESS NOTE: the A held for a bar (circular breathing)
                tq = q16(t)
                dur = e[3] - t
                sig, g, _ = self.note(self.A4, dur, 1.0, t)
                mix.place(self.bus, sig * np.linspace(0.85, 1.1, len(sig)), tq, 0.46 * g, pan=0.2)
                mix.place(self.bus, self.note(self.A4 - 12, dur, 0.7, t)[0], tq, 0.14 * g, pan=0.2)
            elif kind == 'snip_open' and e[2] == k:                # DOUBLE REED: a reed crow as the blades open
                mix.place(self.bus, M.gliss(self.oboe(86, 0.12, 1.0), 3.0, 0.06), q16(t), 0.22, pan=0.25)
            elif kind == 'snip' and e[2] == k:
                mix.place('fx', self.scissor(int(t * 10)), q16(t), 0.30 if e[3] else 0.16, pan=0.15)
                if e[3]:
                    mix.place('fx', A.thud(0.3, 160, 60, seed=int(t * 7)), q16(t), 0.25)
            elif kind == 'ability' and e[2] == k and e[3] == ab['low']['name']:   # ILL WIND: a bent, beating A
                tq = q16(t)
                dur = 2 * T.BEAT
                y = M.bend(self.oboe(self.A4 + 12, dur, 1.0), -1.5, 0.05, dur)
                y2 = M.bend(self.oboe(self.A4 + 11.5, dur, 0.8), -1.8, 0.05, dur)
                mix.place(self.bus, self.colour(y + 0.6 * y2, t)[0], tq, 0.36, pan=0.2)
                mix.place('fx', A.whoosh(1.2, 200, 3000, rise=False), tq, 0.22)
                mix.place('fx', M.thunder(1.4, 33, crack=False), tq, 0.30)

    def scissor(self, seed=0):
        """DOUBLE REED's snip: two bright metallic scrapes a few ms apart"""
        A = self.M.A
        n = int(0.12 * A.SR)
        t = A.tt(n)
        r = np.random.default_rng(seed)
        a = A.bp(r.normal(0, 1, n), 3000, 9000) * np.exp(-t / 0.012)
        b = np.roll(A.bp(r.normal(0, 1, n), 2500, 7000) * np.exp(-t / 0.018), int(0.028 * A.SR))
        return a + b * 0.9

    def fall_apart(self, mix):                                     # a falling, sour A, keys clattering
        M = self.M
        x = self.oboe(self.A4, 1.0, 1.0)
        mix.place(self.bus, M.bend(x, -8.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), M.P.T_HS, 0.40)
        for j, m in enumerate((86, 81, 91, 77, 84)):
            mix.place('fx', M.clank(m, 20 + j, 0.35, ((1.0, 0.10), (2.76, 0.05), (5.4, 0.025))), M.P.T_HS + 0.08 * j,
                      0.07, pan=(j - 2) / 5)

    def stinger(self, mix, tw):
        mix.place('solo', self.oboe(self.A4, 0.9, 1.0), tw, 0.45)
        mix.place('solo', self.oboe(self.A4 + 5, 0.9, 0.9), tw, 0.30)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            mix.place('solo', self.oboe(m, d * T.BEAT * 0.95, 0.95), ts, 0.52)

    def reassemble(self, mix):
        T, M = self.T, self.M
        run = [62, 65, 69, 74, 77, 81]
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place(self.bus, self.oboe(m, 0.12, 0.7), tj, 0.20)
