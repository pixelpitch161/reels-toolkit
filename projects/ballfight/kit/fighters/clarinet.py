"""CLARINET (lavender), as built in #37: TWELFTH JUMP (the only teleporter: on every beat the ball vanishes and
reappears past the enemy, the zip line between its two spots cuts), SQUEAK (a shrill ring that stuns), CLARION (jumps
on the 8ths), RHAPSODY (five rising zig-zag jumps through the enemy).

Look: the ball shrinks away in a lavender burst and reappears past the enemy, the zip line arcing like a leap (white
core, fading from the start) and lit on the canvas; lavender sparks and claw marks on a landed cut; SQUEAK's jagged
inked ring and the stunned enemy's circling stars; no speed lines across a teleport.
Sound: the lower, liquid line (role 'line'): every jump plays its line (config LINES, one note a beat; CLARION's 8th
jumps add the passing notes), a landed cut adds the twelfth above (the register key); it plays flat with a beating
second reed while OUT OF TUNE; SQUEAK is the infamous squeal, RHAPSODY five notes gliding up the registers.
Draw code from projects/ballfight_clarinet_vs_oboe/styles.py + render.py (#37), copied unchanged; sound from #37's
audio.py (the Pixel Pitch clarinet's reed synth)."""
import math

import numpy as np
from reelkit.config import W, H, FPS
from reelkit import fx
import scene as S

from . import base
from ._reeds import _mask, blend, add, ribbon, beam_lane, INK, WHITE, LAV_SPARK, KO_CHIPS, draw_woodwind_shard, tail_theta

R = S.R_BALL


# ---------------------------------------------------------------- TWELFTH JUMP (#37 styles.py)
def zip_path(p0, p1, y, n=14):
    """the jump's path: a straight line in the ring plane that arcs up like a leap (higher for longer jumps)"""
    u = np.linspace(0, 1, n)
    xy = p0[None] + (p1 - p0)[None] * u[:, None]
    hop = 0.18 + 0.09 * float(np.hypot(*(p1 - p0)))
    return np.column_stack([xy, y + hop * np.sin(np.pi * u)])


def draw_zip(img, cam, p0, p1, age, col, y, life=0.34, hot=1.0):
    """the zip line from where the clarinet was to where it lands: a tapered lavender bolt with a white core, drawn
    the moment it jumps and fading over `life` s (the tail end first)"""
    if age < 0 or age >= life:
        return img
    k = 1 - age / life
    P = zip_path(np.asarray(p0, float), np.asarray(p1, float), y)
    n = len(P)
    tail = np.clip(np.linspace(0, 1, n) - (1 - k) * 0.9, 0, 1) ** 0.6       # the start fades first
    c = np.asarray(col, float)
    glow = ribbon(cam, P, 0.26 * (0.4 + 0.6 * tail) * (0.6 + 0.4 * hot))
    img = add(img, _mask([glow]), c * 0.6 + 30, 0.34 * k)                     # a luminous lavender halo
    ink = ribbon(cam, P, 0.13 * (0.3 + 0.7 * tail))
    img = blend(img, _mask([ink]), INK, 0.75 * k)                             # ink edge: reads on the lilac canvas
    mid = ribbon(cam, P, 0.085 * (0.3 + 0.7 * tail))
    img = blend(img, _mask([mid]), np.clip(c * 0.85 + 60, 0, 255), 1.0 * k)
    sp = cam.wp(S.to_world(P[:, 0], P[:, 1], P[:, 2]))
    img = blend(img, _mask([], [sp[int(n * (1 - k) * 0.6):]], 2), WHITE, 1.0 * k)
    return img


def draw_poof(img, cam, p, age, col, y, life=0.22, n=10, r0=0.25, r1=0.95, seed=0):
    """vanish / appear burst: a ring of lavender sparks expanding from the spot (no flash, a few pixels each)"""
    if age < 0 or age >= life:
        return img
    u = age / life
    rr = r0 + (r1 - r0) * (1 - (1 - u) ** 2)
    k = (1 - u) ** 1.2
    rng = np.random.default_rng(seed)
    ph = rng.uniform(0, 2 * math.pi)
    a = ph + np.arange(n) * 2 * math.pi / n
    pts = cam.wp(S.to_world(p[0] + rr * np.cos(a), p[1] + rr * np.sin(a), y + 0.25 * np.sin(a * 3 + ph) * u))
    c = np.clip(np.asarray(col) * 0.8 + 50, 0, 255)
    for x, yy in pts:
        xi, yi = int(round(x)), int(round(yy))
        if 2 <= xi < W - 2 and 2 <= yi < H - 2:
            for dx, dy in ((2, 0), (-2, 0), (0, 2), (0, -2), (1, 1), (-1, 1), (1, -1), (-1, -1)):
                img[yi + dy, xi + dx] = img[yi + dy, xi + dx] * (1 - 0.7 * k) + INK * 0.7 * k
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                img[yi + dy, xi + dx] = img[yi + dy, xi + dx] * (1 - k) + c * k
            img[yi, xi] = img[yi, xi] * (1 - k) + WHITE * k
    return img


# ---------------------------------------------------------------- SQUEAK, stun (#37 styles.py)
def stun_stars(img, x, y, rp, t, k, col):
    """stunned: three little inked stars circling above the ball"""
    if k <= 0.02:
        return img
    c = np.clip(np.asarray(col) * 0.5 + 140, 0, 255)
    for j in range(3):
        a = t * 7.0 + j * 2 * math.pi / 3
        sx, sy = x + rp * 0.95 * math.cos(a), y - rp * 1.1 + rp * 0.28 * math.sin(a)
        xi, yi = int(round(sx)), int(round(sy))
        for dx, dy, cc, w_ in ((0, -2, INK, .8), (0, 2, INK, .8), (-2, 0, INK, .8), (2, 0, INK, .8), (1, 1, INK, .5),
                               (-1, 1, INK, .5), (1, -1, INK, .5), (-1, -1, INK, .5), (0, -1, c, 1.), (0, 1, c, 1.),
                               (-1, 0, c, 1.), (1, 0, c, 1.), (0, 0, WHITE, 1.)):
            if 0 <= xi + dx < W and 0 <= yi + dy < H:
                img[yi + dy, xi + dx] = img[yi + dy, xi + dx] * (1 - k * w_) + cc * k * w_
    return img


def draw_squeak(img, cam, cx, cz, r, rmax, t, col, y=0.25):
    """SQUEAK: the shrill burst as a jagged, inked white ring expanding from the clarinet (a squeal drawn as a sawtooth)"""
    k = 1 - r / rmax
    if k <= 0.02:
        return img
    a = np.linspace(0, 2 * math.pi, 73)
    jag = r + 0.12 * np.where(np.arange(len(a)) % 2 == 0, 1.0, -0.6)
    P = cam.wp(S.to_world(cx + jag * np.cos(a), cz + jag * np.sin(a), np.full(len(a), y)))
    img = blend(img, _mask([], [P], 3), INK, 0.6 * k)
    img = blend(img, _mask([], [P], 1), WHITE * 0.7 + np.asarray(col) * 0.3, 1.0 * k)
    return img


def squeak_light(sxp, szp, cx, cz, r, rmax, t):
    """SQUEAK's shrill ring on the canvas: a jagged, high-frequency band (a squeal drawn as a sawtooth front)"""
    rd = np.hypot(sxp - cx, szp - cz)
    ang = np.arctan2(szp - cz, sxp - cx)
    jag = 0.07 * np.abs(((ang * 18 / math.pi + t * 30) % 2) - 1)
    band = np.clip(1 - np.abs(rd - r - jag) / 0.08, 0, 1)
    return band * (1 - r / rmax)


class Look(base.Look):
    shards = ['wood'] * 6 + ['key'] * 8 + ['bell'] * 2
    spark_big = ('wave', 'snip', 'stun')
    flash_hits = ('wave', 'snip')
    teleports = True
    rope_kinds = {'wave': 1.0, 'shockwave': 0.5, 'stun': 0.5}

    def setup(self):
        P, k = self.P, self.k
        self.JUMPS = [(e[0], e[2], np.array([e[3], e[4]]), np.array([e[5], e[6]]), e[7], e[8])
                      for e in P.EV if e[1] == 'jump' and e[2] == k]
        self.JUMP_T = np.array([j[0] for j in self.JUMPS])
        self.STUNS = [(e[0], e[2], e[4]) for e in P.EV if e[1] == 'stun' and e[3] == k]   # t, victim, dur

    def jump_gap(self, j):
        """time to its next jump (CLARION halves it): the vanish / appear windows shrink to fit"""
        ts = self.JUMP_T
        return float(ts[j + 1] - ts[j]) if j + 1 < len(ts) else 1.0

    def jump_phase(self, t):
        """(scale, phase): the ball shrinks away over the last frames before a jump and grows back in after it"""
        ts = self.JUMP_T
        if not len(ts) or t >= self.P.T_KO:
            return 1.0, None
        j = int(np.searchsorted(ts, t, 'right'))                 # next jump index
        sc = 1.0
        ph = None
        fr = 1.0 / FPS
        if j < len(ts):
            win = 2 * fr * min(1.0, self.jump_gap(max(0, j - 1)) / 0.4)
            if ts[j] - win <= t < ts[j]:
                u = (t - (ts[j] - win)) / win
                sc, ph = 1 - 0.5 * u, 'out'
        if j > 0:
            win = 2.5 * fr * min(1.0, self.jump_gap(j - 1) / 0.4)
            if ts[j - 1] <= t < ts[j - 1] + win:
                u = (t - ts[j - 1]) / win
                sc, ph = 0.5 + 0.5 * u, 'in'
        return sc, ph

    def stun_at(self, kk, t):
        for t0, v, dur in self.STUNS:
            if v == kk and t0 <= t < t0 + dur and t0 < self.P.T_KO:
                return min(1.0, (t0 + dur - t) / 0.12, (t - t0) / 0.05)
        return 0.0

    def state(self, kk, st, t, phase, gl=0.0):
        if phase == 'fight':
            st['stun'] = self.stun_at(kk, t)                      # its SQUEAK freezes the enemy
            if kk == self.k:
                sc, ph = self.jump_phase(t)
                st['scale'], st['jump'] = sc, ph
                if ph is not None:
                    st['glow'] = 0.6 * (1 - abs(sc - 0.75) / 0.25)
        elif phase == 'tail' and kk == self.k:
            tail_theta(self.P, kk, st, t, gl)

    def live_zips(self, F):
        """[(p0, p1, age, hot)] of its zip lines showing at t"""
        key = ('zips', self.k)
        if key not in F.__dict__:
            out = []
            P, t = self.P, F.t
            if P.T.T_FIGHT <= t < P.T_HS:
                tc = min(t, P.T_KO)
                for tj, k, p0, p1, hit, kind in self.JUMPS:
                    if tj <= tc < tj + 0.34:
                        out.append((p0, p1, tc - tj, 1.0 if kind == 1 else 0.6 + 0.4 * hit))
            F.__dict__[key] = out
        return F.__dict__[key]

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'hit' and e[7] in ('cut', 'stun', 'wave'):
            return {'cut': 0.6 if e[8] == 0 else 1.2, 'stun': 0.8, 'wave': 2.0}[e[7]]
        return super().shake(e)

    def spark(self, e):
        P = self.P
        if e[7] == 'cut':
            return P.COL[e[2]] * 0.6 + 100, 10, 2.6, 0.24
        if e[7] in self.spark_big:
            return P.COL[e[2]] * 0.6 + 100, 16, 3.2, 0.3
        return P.COL[e[2]] * 0.6 + 100, 6, 2.0, 0.2

    def hit_glow(self, e):
        if e[7] in ('wave', 'snip') or (e[7] == 'cut' and e[8] == 1):
            return 14, 0.35
        if e[7] in ('cut', 'tune', 'stun'):
            return 8, 0.2
        return None

    def fringe(self, e):
        return e[7] in ('wave', 'snip') or (e[7] == 'cut' and e[8] == 1)

    def fx_setup(self):
        P = self.P
        up = P.up
        for tj, k, p0, p1, hit, kind in self.JUMPS:              # TWELFTH JUMP: lavender light at both ends, motes
            if tj > P.T_KO:
                break
            P.FL.add(tj, up(p1[0], p1[1], R), P.COL[k], radius=1.6, peak=0.45 if kind == 0 else 0.7, decay=0.22)
            P.PA.emit(tj, up(p0[0], p0[1], R), 'motes', n=4, col=LAV_SPARK)
            P.PA.emit(tj, up(p1[0], p1[1], R), 'motes', n=3, col=LAV_SPARK)
            if hit:
                mid = (p0 + p1) / 2
                P.MK.add(tj, 'claw', mid[0], mid[1], size=min(1.4, 0.35 * float(np.hypot(*(p1 - p0)))),
                         angle=math.atan2(p1[1] - p0[1], p1[0] - p0[0]), col=(150, 130, 210), life=2.2, n=1,
                         seed=int(tj * 100), strength=0.55)

    def fx_event(self, e):
        P = self.P
        t_ = e[0]
        if e[1] == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = P.up(x, z, R)
            if how == 'cut':                                       # the zip line cuts: lavender sparks
                P.BU.add(t_, pos, P.COL[att], 0.7 if e[8] == 0 else 1.1)
                P.FL.add(t_, pos, P.COL[att], radius=1.4, peak=0.6, decay=0.22)
                P.PA.emit(t_, pos, 'sparks', n=8, col=LAV_SPARK)
            elif how == 'stun':                                    # SQUEAK lands
                P.BU.add(t_, pos, (255, 250, 255), 0.8)
            elif how == 'wave':
                P.BU.add(t_, pos, P.COL[att], 1.5)
                P.FL.add(t_, pos, P.COL[att], radius=2.2, peak=1.0, decay=0.4)
        elif e[1] == 'ability' and e[4] == 'stun':                # SQUEAK: a shrill flare at the clarinet
            p = P.RUN['P'][P.sim_i(t_), e[2]]
            P.FL.add(t_, P.up(p[0], p[1], R), (255, 250, 255), radius=1.6, peak=0.6, decay=0.2)

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'sparks', n=26, spread=180, speed=(0.8, 2.2), col=KO_CHIPS)

    # ---------------------------------------------------------------- canvas, effects over the ring
    def canvas(self, Ic, C):
        P = self.P
        for p0, p1, age, hot in self.live_zips(C):                 # the zip lines' lit paths
            o = np.asarray(p0)
            d_ = np.asarray(p1) - o
            Ln = float(np.hypot(*d_))
            if Ln > 1e-6:
                lane = beam_lane(C.sxp, C.szp, o, math.atan2(d_[1], d_[0]), Ln, 0.04, C.ys, C.xs)
                Ic += (lane * 0.45 * (1 - age / 0.34) * hot)[:, None] * (P.COL[self.k] / 255.0 + 0.2)[None]
        return Ic

    def canvas_wave(self, Ic, C, w):
        from reelkit.config import BAYER_FULL
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind == 'stun':                                         # SQUEAK's shrill ring
            band = squeak_light(C.sxp, C.szp, cx, cz, r, rmax, C.t)
            band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
            Ic += (band * 1.1)[:, None] * np.array([1.05, 1.0, 1.15])[None]
        return Ic

    def over_ring(self, F, arr, depth):
        P = self.P
        col = P.COL[self.k]
        for p0, p1, age, hot in self.live_zips(F):
            arr = draw_zip(arr, F.cam, p0, p1, age, col, R, hot=hot)
            arr = draw_poof(arr, F.cam, p0, age, col, R, seed=abs(int(p0[0] * 1000)) + 1)
            arr = draw_poof(arr, F.cam, p1, age - 0.02, col, R, life=0.2, r0=0.9, r1=0.3, seed=abs(int(p1[1] * 1000)) + 2)
        if F.in_fight:
            for owner, cx, cz, r, rmax, kind, t0 in P.RUN['WAVES'][F.i]:
                if kind == 'stun' and owner == self.k:
                    arr = draw_squeak(arr, F.cam, cx, cz, r, rmax, F.t, P.COL[owner])
        return arr, depth

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        return draw_woodwind_shard(self.P, arr, cam, kind, p, rot, size)

    def shimmer(self, img, cam, t, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        return fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.4 if kind == 'wave' else 0.25,
                                3.0 if kind == 'wave' else 2.0, t, k=1 - r / rmax, seed=owner)

    def after_grade(self, img, F, drawn):
        if F.in_fight:                                             # the stunned enemy's stars
            for kk, (s_, rp) in drawn.items():
                img = stun_stars(img, s_[0], s_[1], rp, F.t, F.fs[kk].get('stun', 0.0), self.col)
        return img


# ===================================================================================================== sound (#37)
class Sound(base.Sound):
    bus_gain, send = 1.5, 0.35
    band = (140, 700)                                              # its chalumeau line
    colour_args = dict(cut0=60, cut_span=260, drive=4.0)

    def __init__(self, M, k):
        super().__init__(M, k)
        self.CLM = M.B.module('clarinet60_lowpoly')                # the Pixel Pitch clarinet's reed synth (read-only)
        self._vc = {}

    def clar(self, m, dur, vel=0.85, tone='boehm'):
        """one clarinet note (fractional midi = detuned), dur s, from the series' reed synth"""
        key = (round(m, 3), round(dur, 3), round(vel, 2), tone)
        if key not in self._vc:
            M, CLM = self.M, self.CLM
            x = CLM.reed(CLM.midi_hz(m), dur, vel, **CLM.TONES[tone])
            x = M.fit(M.B._resample(np.asarray(x, float)), dur + 0.14, fade=0.03)
            self._vc[key] = x * (0.16 / (np.sqrt((x[:int(0.25 * M.SR)] ** 2).mean()) + 1e-9))
        return self._vc[key]

    def voice(self, m, dur, vel=0.85):
        return self.clar(m, dur, vel)

    def note(self, m, dur, vel, t):
        """the clarinet as the run has it at t: flat by up to a quarter-tone while OUT OF TUNE, with a beating second
        reed (sour), thinner as it's hurt"""
        dep = self.detune(t)
        x = self.clar(m - 0.5 * dep, dur, vel)
        if dep > 0.25:
            x = x + 0.32 * dep * self.clar(m - 0.5 * dep - 0.22, dur, vel * 0.9)
        return self.colour(x, t)

    def title_blast(self, mix, t):
        """a glissando blast up through the registers (config TITLE_HITS)"""
        hits = self.T.TITLE_HITS.get(self.name, (50, 53, 57, 62, 65, 69, 74))
        for j, m in enumerate(hits):
            x = self.clar(m, 0.10 if j < len(hits) - 1 else 0.5, 1.0)
            mix.place(self.bus, self.M.gliss(x, 2.0, 0.04), t + j * 0.032, 0.42)

    def play(self, mix):
        M, T, k = self.M, self.T, self.k
        A, q16, S16 = M.A, M.q16, T.S16
        jumps = [e for e in M.EV if e[1] == 'jump' and e[2] == k and e[0] <= M.T_KO + 0.01]
        jt = [e[0] for e in jumps]
        rh_n = 0
        last_slot = None
        dropped = False
        for j, e in enumerate(jumps):
            t, hit, kind = e[0], e[7], e[8]
            tq = q16(t)
            slot = round(tq / S16)
            if slot == last_slot:
                continue
            last_slot = slot
            gap = (jt[j + 1] - t) if j + 1 < len(jt) else T.BEAT
            if kind == 1:                                        # RHAPSODY: five rising notes gliding up the registers
                m = (50, 57, 62, 69, 74)[min(rh_n, 4)]
                rh_n += 1
                sig, g, _ = self.note(m, 0.22, 1.0, t)
                mix.place(self.bus, M.gliss(sig, 5.0, 0.07), tq, 0.46 * g, pan=-0.2)
                mix.place('fx', A.whoosh(0.18, 600, 7000), tq - 0.05, 0.06)
                continue
            hp = M.hp_at(k, t) / 100.0
            dropped = M.thin_37(slot, 3, hp, dropped)            # sparser when hurt (never two notes in a row)
            if dropped:
                continue
            on_beat = slot % 4 == 0
            m = T.line_at(self.name, tq)
            if not on_beat:                                      # CLARION's 8ths: a passing note toward the next beat
                nxt = T.line_at(self.name, tq + T.BEAT / 2)
                pas = [x for x in T.scale_notes(min(m, nxt), max(m, nxt), T.chord_at(tq)) if x not in (m, nxt)]
                m = pas[len(pas) // 2] if pas else nxt
            dur = min(0.40, max(0.12, gap * 0.92))
            sig, g, lead = self.note(m, dur, 0.85 if on_beat else 0.7, t)
            mix.place(self.bus, sig, tq, (0.40 if on_beat else 0.30) * g, pan=-0.22)
            if lead > 0.04 and on_beat:                          # the leader: a third under (fuller)
                under = T.chord_tones(m - 5, m - 3, T.chord_at(tq))
                if under:
                    mix.place(self.bus, self.note(under[-1], dur, 0.6, t)[0], tq, 0.16 * g, pan=-0.3)
            if hit:                                              # a landed cut: the twelfth above (the register key)
                mix.place(self.bus, self.note(m + 19, 0.09, 0.85, t)[0], tq + 0.03, 0.26 * g, pan=-0.1)
                mix.place('fx', A.whoosh(0.10, 1500, 9000), tq - 0.02, 0.05)
        ab = M.CFG[k]['abilities']
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'stun' and e[3] == k:                     # the stunned enemy: a dizzy "boing"
                n = int(0.4 * M.SR)
                tt_ = A.tt(n)
                boing = np.sin(2 * np.pi * np.cumsum(420 + 140 * np.sin(2 * np.pi * 9 * tt_)) / M.SR) * np.exp(-tt_ / 0.15)
                mix.place('fx', boing, q16(t) + 0.03, 0.08)
            elif kind == 'ability' and e[2] == k:
                name, tq = e[3], q16(t)
                if name == ab['unlock1']['name']:                # SQUEAK: a shrill, wobbling squeal
                    x = self.clar(93.0, 0.22, 1.0, 'goodman')
                    x = np.tanh(3.0 * M.gliss(x, -1.5, 0.05) / (np.abs(x).max() + 1e-9)) * 0.5
                    mix.place(self.bus, x, tq, 0.34, pan=-0.2)
                elif name == ab['unlock2']['name']:              # CLARION: a rip up a twelfth into the clarion register
                    for j, m in enumerate((57, 62, 69, 76)):
                        mix.place(self.bus, self.note(m, 0.09, 0.9, t)[0], tq + j * 0.035, 0.30, pan=-0.2)
                elif name == ab['low']['name']:                  # RHAPSODY
                    mix.place('drums', A.crash(1.6), tq, 0.08)

    def fall_apart(self, mix):                                   # a falling bent note, keys clattering
        M = self.M
        x = self.clar(62, 1.0, 1.0)
        mix.place(self.bus, M.bend(x, -8.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), M.P.T_HS, 0.40)
        for j, m in enumerate((86, 81, 91, 77, 84)):
            mix.place('fx', M.clank(m, 20 + j, 0.35, ((1.0, 0.10), (2.76, 0.05), (5.4, 0.025))), M.P.T_HS + 0.08 * j,
                      0.07, pan=(j - 2) / 5)

    def stinger(self, mix, tw):
        for j, m in enumerate((50, 53, 57, 62)):
            mix.place('solo', self.clar(m, 0.9 - 0.06 * j, 1.0), tw + j * 0.03, 0.30)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            mix.place('solo', self.M.gliss(self.clar(m, d * T.BEAT * 0.95, 0.95, 'boehm'), 1.0, 0.03), ts, 0.50)

    def reassemble(self, mix):
        T, M = self.T, self.M
        run = [62, 65, 69, 74, 77, 81]
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place(self.bus, self.clar(m - 12, 0.12, 0.7), tj, 0.22)
