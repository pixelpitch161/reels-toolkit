"""ALTO SAX (orchid), new in #51 Alto Sax vs Tenor Sax: the only light-wielder. SPOTLIGHT SOLO (every bar a spotlight
swings down from the rafters and sweeps the floor across the enemy's path; whatever the light catches burns on every
16th; the light widens and burns hotter per landed sweep), OCTAVE KEY (two lights at once: the second square to the
first, sweeping there and back at twice the speed), ALTISSIMO (the light narrows to a white-hot line and burns fast),
ENCORE (the light chases the enemy nonstop and every phone light in the stands joins in).

Look: the alto rebuilt from Commons reference photos (altosax_lowpoly: the bell low and close, the keywork as faces,
tubes x1.45 for the sprite). Each light is a cone from a lamp high in the rafters down to a warm pool on the canvas
(real light: it brightens the balls it passes over), with the arena dimmed only in a ring around the pool; octave
lights are tinted orchid, ALTISSIMO is a narrow white-hot pool with a hard core, the burns leave fading scorch marks
and embers. ENCORE's chase light is the widest, and the phones are small white pinpricks on the canvas with faint rays
back to the stands. It shatters into brass tube pieces, key cups and pearls. VS card: spotlight rays fanning down.
Sound: the bright lead. Every sweep plays its line (config LINES, 8ths) over the light's run, each burn is a staccato
squeal a chord tone up; OCTAVE KEY doubles the line an octave up, ALTISSIMO's burns are altissimo screams with a
scoop; ENCORE is a nonstop 16th run up and down the scale with the phones as little chimes. Alto voice: the rebuilt
model's sax_tone (the old alto's recipe, brighter)."""
import math

import numpy as np
from reelkit.config import W, H, BAYER_FULL
from reelkit import fx
import scene as S

from . import base
from ._reeds import _mask, blend, add, INK, WHITE

R = S.R_BALL
WARM = np.array([1.0, 0.93, 0.80])                      # the pool's light (a tungsten follow-spot)
HOT = np.array([1.05, 1.05, 1.12])                      # ALTISSIMO: white-hot
BRASS = np.array([232, 182, 70], float)
KEYG = np.array([255, 214, 110], float)
PEARL = np.array([250, 246, 236], float)
LAMP_H = 7.6                                            # the rafters: the lamps' height above the canvas
LAMP_BACK = 1.6                                         # ... set back from the pool (away from the camera), so the cone reads


def title_pattern(base):
    """ALTO SAX: spotlight rays fanning down from a lamp above the panel"""
    import title as TT
    XX, YY = TT.XX, TT.YY
    a = np.arctan2(XX - 90.0, YY + 60.0)
    ray = (np.floor((a + 1.2) / 0.16) % 2 == 0) & (np.abs(a) < 0.95)
    k = np.clip(1 - (YY + 60.0) / 700.0, 0.25, 1.0)
    return np.where(ray[..., None], base * (1 - 0.35 * k[..., None]) + np.array([255, 236, 210]) * 0.35 * k[..., None],
                    base)


class Look(base.Look):
    shards = ['brass'] * 6 + ['key'] * 4 + ['pearl'] * 3
    spark_big = ('phone',)
    spark_flash = ()
    flash_hits = ()
    fringe_hits = ()
    quiet_hits = ('burn', 'phone')                       # the burns are a sting on every 16th: no HP-bar flash
    rope_kinds = {}
    sprite_k = 0.86
    title_pattern = staticmethod(title_pattern)

    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.SPOTS = [dict(t0=e[0], id=e[3], a=np.array([e[4], e[5]]), b=np.array([e[6], e[7]]), t1=e[9], r=e[10],
                           kind=e[11]) for e in EV if e[1] == 'spot' and e[2] == k]
        self.BURNS = [(e[0], e[4], e[5]) for e in EV if e[1] == 'burn' and e[2] == k]
        self.ENC = [(e[0], e[3]) for e in EV if e[1] == 'encore' and e[2] == k]
        self.ALT = [(e[0], e[3]) for e in EV if e[1] == 'altissimo' and e[2] == k]
        self.OCT = [(e[0], e[3]) for e in EV if e[1] == 'octave' and e[2] == k]
        self.PHONE_BURN = [(e[0], e[3], e[4]) for e in EV if e[1] == 'phone_burn' and e[2] == k]

    # ---------------------------------------------------------------- the lights at frame i
    def lights_at(self, F_or_C):
        if not F_or_C.in_fight or 'SPOT' not in self.P.RUN:
            return []
        return [s for s in self.P.RUN['SPOT'][F_or_C.i] if s[0] == self.k]

    def _active(self, spans, t):
        return any(a <= t < b for a, b in spans)

    # ---------------------------------------------------------------- state: its bell glows while a light runs
    def state(self, kk, st, t, phase, gl=0.0):
        if kk != self.k or phase != 'fight':
            return
        tc = min(t, self.P.T_KO)
        for s in self.SPOTS:
            if s['t0'] <= tc < s['t1']:
                st['glow'] = max(st['glow'], 0.25 + 0.15 * (s['kind'] == 0))
        if self._active(self.ENC, tc):
            st['glow'] = max(st['glow'], 0.5)

    def shake(self, e):
        if e[1] == 'encore':
            return 0.8
        return None

    def spark(self, e):
        if e[7] == 'burn':
            return np.array([255, 220, 170], float), 4, 1.2, 0.22
        if e[7] == 'phone':
            return np.array([255, 255, 255], float), 6, 1.6, 0.2
        return np.array([230, 190, 255], float), 5, 1.8, 0.2

    def hit_glow(self, e):
        if e[7] == 'burn':
            return 8, 0.3
        return None

    def light(self, L, t):
        """the lights brighten whatever passes under them (the balls): a point light over each pool"""
        P = self.P
        if not (self.T.T_FIGHT <= t < P.T_KO) or 'SPOT' not in P.RUN:
            return
        n = 0
        for s in P.RUN['SPOT'][P.sim_i(t)]:
            if s[0] != self.k or s[4] == 3 or n >= 2:
                continue
            col = (HOT * 1.4) if s[7] else (WARM * (1.1 if s[4] != 1 else 0.9))
            L.point(S.to_world(s[1], s[2], 1.6), col, 1.4 + s[3])
            n += 1

    # ---------------------------------------------------------------- effects
    def fx_setup(self):
        P, k = self.P, self.k
        col = P.FL_COL[k]
        last = -9.0
        for t0, x, y in self.BURNS:
            if t0 > P.T_KO:
                break
            P.PA.emit(t0, P.up(x, y, 0.15), 'embers', n=2, col=[(255, 220, 160), (236, 140, 255), (255, 250, 230)])
            if t0 - last > 0.35:                                   # a scorch where the light caught it (not every 16th)
                P.MK.add(t0, 'scorch', x, y, size=0.35, life=4.0, col=(130, 60, 150), seed=int(t0 * 71) % 991,
                         strength=0.5)
                last = t0
        for t0, x, y in self.PHONE_BURN:
            if t0 > P.T_KO:
                break
            P.PA.emit(t0, P.up(x, y, 0.1), 'sparks', n=4, col=[(255, 255, 255), (220, 230, 255)])
        for t0, t1 in self.ENC:
            p = P.RUN['P'][P.sim_i(t0), k]
            P.FL.add(t0, P.up(p[0], p[1], R), col, radius=2.6, peak=0.9, decay=0.4)

    def ko_fx(self, pos):
        P = self.P
        P.PA.emit(P.T_KO, pos, 'brass', n=18, col=[tuple(BRASS), tuple(KEYG), (255, 240, 200)])
        P.PA.emit(P.T_KO, pos, 'sparks', n=20, spread=180, speed=(0.8, 2.2), col=[(236, 170, 255), (255, 236, 255)])

    # ---------------------------------------------------------------- the canvas: the pools of light, the dimmed ring
    def _pool(self, C, s):
        rd = np.hypot(C.sxp - s[1], C.szp - s[2])
        return rd

    def canvas(self, Ic, C):
        for s in self.lights_at(C):
            owner, cx, cz, r, kind, t0, t1, hot, sid = s
            rd = self._pool(C, s)
            if kind == 3:                                          # a phone light: a tiny white pinprick
                m = np.clip(1 - rd / 0.13, 0, 1)
                Ic += (np.floor(m * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3 * 1.4)[:, None] * WHITE[None] / 255.0
                continue
            age = C.t - t0
            fade = min(1.0, age / 0.08) * (min(1.0, max(0.0, (t1 - C.t) / 0.1)) if kind != 2 else 1.0)
            edge = 0.10 if not hot else 0.05
            core = np.clip((r - rd) / edge, 0, 1)
            m = np.floor(core * 4 + BAYER_FULL[C.ys, C.xs] * 0.9) / 4
            if hot:
                tint, gain = HOT, 0.75
                m = m + 0.5 * (rd < r * 0.45)
            elif kind == 1:
                tint, gain = WARM * 0.7 + self.col / 255.0 * 0.35, 0.34
            else:
                tint, gain = WARM, 0.40 if kind == 0 else 0.48
            Ic += (m * gain * fade)[:, None] * tint[None]
            rim = np.clip(1 - np.abs(rd - r) / 0.05, 0, 1) * fade
            Ic += (np.floor(rim * 2 + BAYER_FULL[C.ys, C.xs] * 0.9) / 2 * 0.35)[:, None] * (self.col / 255.0)[None]
        return Ic

    def canvas_after(self, Ic, C):
        """the arena dims only in a ring around each pool (the follow-spot's contrast), never the whole canvas"""
        for s in self.lights_at(C):
            if s[4] == 3:
                continue
            rd = self._pool(C, s)
            r = s[3]
            ring = np.clip(1 - (rd - r) / 0.9, 0, 1) * (rd > r)
            ring = np.floor(ring * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
            Ic *= (1 - 0.30 * ring)[:, None]
        return Ic

    # ---------------------------------------------------------------- the cones from the rafters, the phones' rays
    def over_ring(self, F, arr, depth):
        cam = F.cam
        for s in self.lights_at(F):
            owner, cx, cz, r, kind, t0, t1, hot, sid = s
            if kind == 3:                                          # a faint ray back to the phone in the stands
                w = S.to_world(cx, cz, 0.05)
                d = np.array([w[0], 0, w[2]])
                d = d / (np.linalg.norm(d) + 1e-9)
                src = np.array([d[0] * 6.6, -0.2, d[2] * 6.6])
                sp = cam.wp(np.array([w, src]))
                arr = add(arr, _mask([], [sp], 1), np.array([255, 250, 230]), 0.10)
                g = cam.wp(w[None])[0]
                F.fr.glow(g[0], g[1], 5, (255, 255, 255), 0.6)
                continue
            age = F.t - t0
            fade = min(1.0, age / 0.08) * (min(1.0, max(0.0, (t1 - F.t) / 0.1)) if kind != 2 else 1.0)
            base_w = S.to_world(cx, cz, 0.02)
            lamp = base_w + np.array([0.0, LAMP_H, -LAMP_BACK])
            ring = np.array([base_w + [r * math.cos(a), 0.0, r * math.sin(a)] for a in np.linspace(0, 2 * math.pi, 25)[:-1]])
            pts = cam.wp(np.vstack([lamp[None], ring]))
            from scipy.spatial import ConvexHull
            try:
                hull = pts[ConvexHull(pts).vertices]
            except Exception:
                continue
            mk = _mask([hull])
            fall = np.clip((pts[1:, 1].max() - np.arange(H)[:, None] + 40) / 420.0, 0.15, 1.0)   # brighter near the floor
            mk = np.floor(mk * fall * 6 + BAYER_FULL * 0.9) / 6
            col = np.array([255, 255, 255]) if hot else (np.array([255, 236, 200]) if kind != 1 else
                                                         np.array([255, 220, 255]))
            arr = add(arr, mk, col, (0.20 if hot else 0.15) * fade)
            g = pts[0]
            if -20 < g[1] < H:
                F.fr.glow(g[0], g[1], 9, (255, 244, 220), 0.7 * fade)
        return arr, depth

    def glows(self, F, drawn):
        for s in self.lights_at(F):
            if s[4] == 3:
                continue
            g = F.cam.wp(S.to_world(s[1], s[2], 0.05)[None])[0]
            F.fr.glow(g[0], g[1], 22 * s[3] / 0.5, (255, 255, 255) if s[7] else (255, 232, 200), 0.22)

    # ---------------------------------------------------------------- its parts (K.O. shatter)
    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        c = np.array([p[0], p[2]])
        if kind == 'pearl':
            sp = P.screen(cam, np.array([S.to_world(c[0], c[1], p[1])]))[0]
            x, y = int(sp[0]), int(sp[1])
            if 1 <= x < W - 1 and 1 <= y < H - 1:
                arr[y - 1:y + 1, x - 1:x + 1] = PEARL
            return arr
        if kind == 'key':
            ring = [c + np.array([math.cos(a), math.sin(a)]) * size * 0.9 for a in np.linspace(0, 2 * math.pi, 9)[:-1]]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in ring]))
            P.layer_blend(arr, [sp], KEYG * (0.8 + 0.3 * abs(math.cos(rot))), 1.0)
            return blend(arr, _mask([], [np.vstack([sp, sp[:1]])], 1), np.array([90, 50, 20], float), 0.7)
        pts = [c - d * size * 1.8 + nn * curl * 0.04, c + d * size * 1.8 - nn * curl * 0.04]   # a piece of brass tube
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
        arr = blend(arr, _mask([], [sp], 3), np.array([110, 70, 20], float), 0.9)
        return blend(arr, _mask([], [sp], 2), BRASS * (0.8 + 0.35 * abs(math.cos(rot))), 1.0)


# ===================================================================================================== sound
class Sound(base.Sound):
    bus_gain, send = 1.55, 0.30
    band = (900, 3500)                                     # the bright lead's band
    colour_args = dict(cut0=180, cut_span=600, drive=4.0)

    def __init__(self, M, k):
        super().__init__(M, k)
        import pp_bridge as B
        self.B = B
        self.mod = B.module('altosax_lowpoly')
        self.pan = -0.18 if k == 0 else 0.18
        self.lo, self.hi = M.CFG[k]['register']
        self.g0 = M.inst_gain(M.CFG[k]['pp'])
        self._c = {}

    def tone(self, m, dur, vel=0.85, **kw):
        key = (int(m), round(dur, 3), round(vel, 2), tuple(sorted(kw.items())))
        if key not in self._c:
            self.mod.AS._RN = np.random.default_rng(int(m) * 1000 + int(round(dur * 1000)))   # deterministic breath
            x = self.mod.alto_tone(self.mod._midi_hz(int(m)), dur, vel, **kw)
            self._c[key] = self.M.fit(self.B._resample(np.asarray(x, float)), dur + 0.25) * self.g0
        return self._c[key]

    def voice(self, m, dur, vel=0.85):
        return self.tone(m, dur, vel)

    def fit_reg(self, m, lo=None, hi=None):
        lo, hi = lo or self.lo, hi or self.hi
        while m > hi:
            m -= 12
        while m < lo:
            m += 12
        return m

    def line(self, t):
        return self.fit_reg(self.T.line_at(self.name, t))

    def up_tone(self, t, m):
        """the next chord tone above m (the burn's squeal)"""
        ch = self.T.chord_at(t)
        ups = [c for c in self.T.chord_tones(m + 3, m + 12, ch)]
        return ups[0] if ups else m + 12

    def title_blast(self, mix, t):
        """a bright alto squeal sliding up into the top note (config TITLE_HITS)"""
        hits = self.T.TITLE_HITS.get(self.name) or (self.line(0.0),)
        M = self.M
        for j, m in enumerate(hits):
            x = M.gliss(self.tone(m, 0.55 - 0.1 * j, 1.0, growl=0.1), 3.0, 0.08)
            mix.place(self.bus, x, t + 0.03 * j, 0.55 - 0.08 * j, pan=self.pan)

    def count_in(self, mix):
        T = self.T
        for j in range(4):                                     # a pickup into FIGHT!: four 8ths up the line
            tj = T.BAR + 3 * T.BEAT + j * T.BEAT / 4
            mix.place(self.bus, self.tone(self.line(T.T_FIGHT + j * T.BEAT / 2), T.S16 * 0.9, 0.85), tj, 0.36,
                      pan=self.pan)

    def play(self, mix):
        M, T, k = self.M, self.T, self.k
        q16 = M.q16
        th = M.Thinner(510 + k)
        hot_spans = [(e[0], e[3]) for e in M.EV if e[1] == 'altissimo' and e[2] == k]
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if e[2] != k:
                continue
            if kind == 'spot':                                 # the sweep plays the line over the light's run (8ths)
                t0, t1, sk = q16(e[8]), e[9], e[11]
                n8 = max(1, int(round((t1 - t0) / (T.BEAT / 2))))
                for j in range(n8):
                    tj = t0 + j * T.BEAT / 2
                    if not th.keep(round(tj / T.S16) + 7 * sk, M.hp_at(k, tj) / 100.0):
                        continue
                    m = self.line(tj) + (12 if sk == 1 else 0)
                    m = self.fit_reg(m, self.lo, self.hi + (12 if sk == 1 else 0))
                    sig, g, _ = self.colour(self.tone(m, T.BEAT * 0.45, 0.85), tj)
                    mix.place(self.bus, sig, tj, (0.40 if sk == 0 else 0.22) * g, pan=self.pan + (0.15 if sk else 0))
            elif kind == 'burn':                               # each burn: a staccato squeal a chord tone up
                tq = q16(t)
                hot = any(a <= t < b for a, b in hot_spans)
                m = self.up_tone(tq, self.line(tq))
                if hot:
                    x = M.gliss(self.tone(m + 12, T.S16 * 1.6, 1.0, growl=0.15), 1.5, 0.05)
                    g = 0.34
                else:
                    x = self.tone(m, T.S16 * 0.8, 0.9)
                    g = 0.24
                sig, gg, _ = self.colour(x, t)
                mix.place(self.bus, sig, tq, g * gg, pan=self.pan * 0.6)
            elif kind == 'encore':                             # ENCORE: a nonstop 16th run, up and down the scale
                t0, t1 = q16(t), e[3]
                n = int((t1 - t0) / T.S16)
                for j in range(n):
                    tj = t0 + j * T.S16
                    if tj > M.T_KO:
                        break
                    sc = T.scale_notes(self.lo + 7, self.hi, T.chord_at(tj))
                    ph = j % (2 * len(sc) - 2)
                    m = sc[ph] if ph < len(sc) else sc[2 * len(sc) - 2 - ph]
                    sig, g, _ = self.colour(self.tone(m, T.S16 * 0.95, 0.9), tj)
                    mix.place(self.bus, sig, tj, 0.30 * g, pan=self.pan * math.cos(j * 0.4))
            elif kind == 'phones':                             # the phones lighting up: a little chime sparkle
                tq = q16(t)
                mix.place('fx', M.A.chime(96 + (int(t * 8) % 3) * 2, 0.25), tq, 0.035, pan=0.3 * math.sin(t * 5))
            elif kind == 'ability' and e[4] in ('octave', 'altissimo'):   # a rising scoop announcing it
                tq = q16(t)
                m = self.fit_reg(self.line(tq) + 12, self.lo, self.hi + 7)
                mix.place(self.bus, M.gliss(self.tone(m, T.BEAT * 0.9, 1.0), 5.0, 0.12), tq, 0.40, pan=self.pan)

    def fall_apart(self, mix):
        M = self.M
        t0 = M.P.T_HS
        m = self.line(t0)
        mix.place(self.bus, M.bend(self.tone(m + 12, 1.2, 1.0, growl=0.3), -12.0, 0.08, 1.1), t0, 0.45, pan=self.pan)
        for j in range(4):
            mix.place('fx', M.clank(88 - 3 * j, seed=j, dur=0.25), t0 + 0.06 + 0.07 * j, 0.05)

    def stinger(self, mix, tw):
        m = self.fit_reg(self.T.root_above(self.T.CHORDS[0], self.lo + 12))
        mix.place('solo', self.M.gliss(self.tone(m + 12, 0.9, 1.0), 4.0, 0.1), tw, 0.55)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            x = self.tone(m, max(0.1, d * T.BEAT * 0.95), 0.95)
            if d >= 1.5:
                x = self.M.vibrato(x, 0.3, 5.4, 0.2)
            mix.place('solo', x, ts, 0.60)

    def reassemble(self, mix):
        T, M = self.T, self.M
        sc = T.scale_notes(self.lo + 5, self.hi, T.CHORDS[0])
        for j, m in enumerate(sc[:8]):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / 8)
            mix.place('solo', self.tone(m, 0.12, 0.8), tj, 0.30)

    def last_bar(self, mix):
        """a rising lead line into the title slam: the scale climbing in 8ths to the top tonic on frame 0"""
        T = self.T
        t_last = (T.N_BARS - 1) * T.BAR
        tgt = self.fit_reg(T.root_above(T.CHORDS[0], self.lo + 12), self.lo + 10, self.hi)
        sc = [m for m in T.scale_notes(tgt - 14, tgt - 1, T.chord_at(t_last))][-8:]
        for j, m in enumerate(sc):
            mix.place(self.bus, self.tone(m, T.BEAT * 0.45, 0.9), t_last + j * T.BEAT / 2, 0.34, pan=self.pan)
