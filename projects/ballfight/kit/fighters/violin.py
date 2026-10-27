"""VIOLIN (violet), as last delivered in #42 Viola vs Violin (built in #28 Drums vs Violin): the bow spun around the
ball like a rapier (blade, faster per landed hit: slow strokes become frantic spiccato), PIZZICATO (an arc of plucked
darts on the 16ths), TREMOLO (haste: the bow a violet blur), SCREECH (wave: a jagged, shivering ring).

Look: #28's bow (pernambuco stick, pale hair ribbon, ebony frog, hatchet tip, a pearl dot) with its motion fan (a white
fan that widens with the spin; TREMOLO's violet blur), the pizzicato darts (a bright head on a plucked string), SCREECH's
jagged ring on the canvas + its heat shimmer; #42's allowed v2 upgrade: rosin dust off the bow on every landed stroke,
attack lights and bursts. It shatters into strings, wood chips (its varnish), the bridge, the scroll and bits of bow.
Sound (#42, in the Pixel Pitch violin's voice): every quarter turn of the bow plays its line (config LINES: one note
per 16th = 16 a bar; else the kit's sketch line) at that moment, snapped to the 16th, so a slow bow plays a few long
strokes and the fast bow (TREMOLO) the line as spiccato; landed bow hits are accents climbing the key's scale,
PIZZICATO plucks, TREMOLO a shiver of 32nds an octave up, SCREECH the beginner's squeal. While the enemy has the melody
(#42's MELODY AT LAST) it drops to soft harmony under it. Hurt, it thins (the splitmix Thinner) and gets scratchy; the
leader adds a double-stop under the line.
Draw code from projects/ballfight_drums_vs_violin/styles.py (#28, imported unchanged by #42), copied unchanged; the
layers from #42's render.py, the sound from #42's audio.py. The title flourish and the reassembly run were written in
C minor (#42); they are moved to the episode's key (no change in C)."""
import math

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, FPS, BAYER_FULL
from reelkit.lighting import shade
from reelkit import fx
import scene as S

from . import base

R = S.R_BALL

# ---------------------------------------------------------------- the bow (#28 styles.py, unchanged)
STICK = np.array([92, 44, 30])          # pernambuco
STICK_HI = np.array([150, 86, 52])
HAIR = np.array([236, 230, 212])
FROG = np.array([26, 22, 30])
PEARL = np.array([226, 236, 255])
TIP = np.array([240, 236, 226])
STICK_W, HAIR_W, HAIR_GAP = 0.055, 0.045, 0.085


def bow_geom(p, th, blen, sd=1.0, y=R):
    """world quads [(Q, colour, kind)] of the bow from the ball's surface to the tip; the hair rides on the leading side"""
    d = np.array([math.cos(th), math.sin(th)])
    nrm = np.array([-d[1], d[0]]) * sd
    r0, r1 = R * 0.86, R + blen
    Wp = lambda r, w: p + d * r + nrm * w
    stick = [Wp(r0, -STICK_W / 2), Wp(r1, -STICK_W / 2 * 0.7), Wp(r1, STICK_W / 2 * 0.7), Wp(r0, STICK_W / 2)]
    hair = [Wp(r0 + 0.16, HAIR_GAP - HAIR_W / 2), Wp(r1 - 0.06, HAIR_GAP * 0.55 - HAIR_W / 2),
            Wp(r1 - 0.06, HAIR_GAP * 0.55 + HAIR_W / 2), Wp(r0 + 0.16, HAIR_GAP + HAIR_W / 2)]
    frog = [Wp(r0 + 0.02, -0.03), Wp(r0 + 0.22, -0.03), Wp(r0 + 0.20, HAIR_GAP + 0.04), Wp(r0 + 0.04, HAIR_GAP + 0.04)]
    tip = [Wp(r1 - 0.13, -0.04), Wp(r1 + 0.02, -0.04), Wp(r1 + 0.02, HAIR_GAP * 0.55 + 0.03), Wp(r1 - 0.08, HAIR_GAP * 0.55 + 0.03)]
    out = []
    for Q, c, kind in ((hair, HAIR, 'hair'), (stick, STICK, 'stick'), (frog, FROG, 'frog'), (tip, TIP, 'tip')):
        out.append((np.array([S.to_world(q[0], q[1], y + (0.012 if kind == 'stick' else 0.0)) for q in Q]), c, kind))
    pearl = S.to_world(*(p + d * (r0 + 0.12) + nrm * (HAIR_GAP * 0.5)), y + 0.02)
    return out, pearl


def draw_bow(fr, cam, L, shade, st):
    quads, pearl = bow_geom(st['p'], st['th'], st['blen'], st.get('sd', 1.0))
    for Q, col, kind in quads:
        c = shade(L, np.array([[0, 1.0, 0]]), Q.mean(0)[None], col, rimk=0.35)[0]
        fr.poly(cam, Q, c)
        if kind == 'stick':                                   # a highlight line along the stick (it's round)
            s = cam.wp(Q)
            fr.draw.line([tuple((s[0] + s[3]) / 2), tuple((s[1] + s[2]) / 2)],
                         fill=tuple(int(v) for v in np.clip(shade(L, np.array([[0, 1.0, 0]]), Q.mean(0)[None],
                                                                        STICK_HI, rimk=0.6)[0], 0, 255)), width=1)
    sp = cam.wp(pearl[None])[0]
    fr.draw.point([(sp[0], sp[1])], fill=tuple(int(v) for v in PEARL))


def bow_trail_polys(cam, st, dth, nseg=6):
    """the swept wedge behind the spinning bow (motion blur) as screen polygons, tip-heavy (a thin blade blurs as a fan)"""
    p, th, blen = st['p'], st['th'], st['blen']
    r0, r1 = R + blen * 0.35, R + blen
    out = []
    for j in range(nseg):
        a0 = th - dth * j / nseg
        a1 = th - dth * (j + 1) / nseg
        pts = []
        for a, r in ((a0, r0), (a0, r1), (a1, r1), (a1, r0)):
            q = p + r * np.array([math.cos(a), math.sin(a)])
            pts.append(S.to_world(q[0], q[1], R))
        out.append((cam.wp(np.array(pts)), j / nseg))
    return out


# ---------------------------------------------------------------- pizzicato darts (#28 styles.py, unchanged)
DART_COL = np.array([214, 232, 255])


def dart_quads(x, z, ang, y=0.48, L_=0.34, Wd=0.07):
    d = np.array([math.cos(ang), math.sin(ang)])
    n = np.array([-d[1], d[0]])
    c = np.array([x, z])
    head = [c + d * L_ * 0.5, c + d * L_ * 0.15 - n * Wd, c + d * L_ * 0.05, c + d * L_ * 0.15 + n * Wd]
    body = [c + d * L_ * 0.1 - n * 0.012, c - d * L_ * 0.5 - n * 0.012, c - d * L_ * 0.5 + n * 0.012, c + d * L_ * 0.1 + n * 0.012]
    return (np.array([S.to_world(q[0], q[1], y) for q in head]), np.array([S.to_world(q[0], q[1], y) for q in body]))


def draw_dart(fr, cam, L, shade, x, z, ang, col):
    head, body = dart_quads(x, z, ang)
    fr.poly(cam, body, np.clip(np.array(col) * 0.5 + 120, 0, 255))
    c = shade(L, np.array([[0, 1.0, 0]]), head.mean(0)[None], np.array(col) * 0.45 + DART_COL * 0.55, rimk=0.4)[0]
    fr.poly(cam, head, c + 20)


def draw_shard(P, arr, cam, kind, p, rot, size, curl, wood):
    """one shatter part of a bowed string instrument (#28 / #42): a string curling free, a bit of bow, the bridge, the
    scroll, wood chips in its varnish"""
    sx_, sz_ = p[0], p[2]
    d = np.array([math.cos(rot), math.sin(rot)])
    nn = np.array([-d[1], d[0]])
    if kind == 'string':                                      # a string curling free
        pts = [np.array([sx_, sz_]) + d * (j - 3) * 0.16 + nn * curl * 0.10 * math.sin(j) for j in range(7)]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
        im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
        ImageDraw.Draw(im).line([tuple(q) for q in sp], fill=STRING_COL, width=1)
        return np.asarray(im).astype(np.float32)
    if kind == 'bowbit':                                      # a piece of the bow stick
        q4 = [np.array([sx_, sz_]) + d * 0.18 * a1 + nn * 0.02 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
        P.layer_blend(arr, [sp], STICK * 1.3, 1.0)
        return arr
    if kind == 'bridge':                                      # the maple bridge, pale
        q4 = [np.array([sx_, sz_]) + d * 0.12 * a1 + nn * 0.05 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
        P.layer_blend(arr, [sp], np.array([228, 198, 148]), 1.0)
        return arr
    if kind == 'scroll':                                      # the scroll: a small varnished spiral
        pts = [np.array([sx_, sz_]) + (0.13 - 0.016 * j) * np.array([math.cos(rot + j * 0.9), math.sin(rot + j * 0.9)])
               for j in range(8)]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
        im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
        ImageDraw.Draw(im).line([tuple(q) for q in sp], fill=tuple(int(v) for v in wood * 1.2), width=2)
        return np.asarray(im).astype(np.float32)
    tri = [np.array([sx_, sz_]) + d * size, np.array([sx_, sz_]) - d * size * 0.7 + nn * size * 0.8,
           np.array([sx_, sz_]) - d * size * 0.4 - nn * size * 0.7]       # wood: tumbling chips in its varnish
    sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
    P.layer_blend(arr, [sp], np.clip(wood * (0.75 + 0.25 * math.cos(rot * 2)) * 1.2, 0, 255), 1.0)
    return arr


ROSIN = [(250, 244, 226), (236, 226, 200), (255, 255, 246)]      # #42's rosin dust off the bow
WOOD = np.array([156, 52, 30])                                   # its varnish (shards, K.O. chips)
STRING_COL = (230, 226, 214)


def key_shift(T):
    """semitones from C to the episode's key (nearest: -6..+5): #42's motifs were written in C minor"""
    return (T.KEY_PC + 6) % 12 - 6


# ===================================================================================================== look
class Look(base.Look):
    shards = ['string'] * 4 + ['wood'] * 7 + ['bridge'] * 2 + ['scroll'] + ['bowbit'] * 3
    spark_flash = ('blade',)
    fringe_hits = ('wave',)
    flash_hits = ('wave',)                   # SCREECH (#28; #42's test also let only VIOLA JOKE's note through, but
                                             # #42's run had no SCREECH hit, so either way replays #42)
    rope_kinds = {'wave': 1.0}

    # ---------------------------------------------------------------- the darts in flight (the run's K rows of its own)
    def keys_at(self, t):
        P = self.P
        if 'K' not in P.RUN:
            return []
        if self.T.T_FIGHT <= t < P.T_HS:
            K = P.RUN['K'][P.sim_i(t)]
        elif P.T_KO <= t < P.T_KO + 0.9 and t >= P.T_HS:          # darts in flight at the K.O. drop (#28 / #42)
            K = P.RUN['K'][-1]
        else:
            return []
        return [row for row in K if not np.isnan(row[0]) and int(row[3]) == self.k]

    def now(self, F):
        key = ('violin_keys', self.k)
        if key not in F.__dict__:
            F.__dict__[key] = self.keys_at(F.t)
        return F.__dict__[key]

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'hit' and e[7] in ('blade', 'wave'):
            return {'blade': 0.4, 'wave': 2.0}[e[7]]
        return super().shake(e)

    def spark(self, e):
        how = e[7]
        n, sp, life = (16, 3.2, 0.3) if how == 'wave' else ((8, 2.4, 0.22) if how == 'blade' else (5, 1.8, 0.18))
        return self.P.COL[e[2]] * 0.6 + 100, n, sp, life

    def hit_glow(self, e):
        if e[7] == 'wave':
            return 14, 0.35
        if e[7] == 'blade':
            return 8, 0.22
        return None

    def fx_event(self, e):
        """#42: the bow lands (violet, rosin dust), a dart plucks, SCREECH lands; SCREECH lights the ring"""
        P = self.P
        t_ = e[0]
        if e[1] == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = S.to_world(x, z, R)
            if how == 'blade':
                P.BU.add(t_, pos, P.COL[att], 0.6)
                P.FL.add(t_, pos, P.FL_COL[att], radius=1.2, peak=0.45, decay=0.2)
                P.PA.emit(t_, pos, 'rosin', n=6, col=ROSIN)
            elif how == 'key':
                P.BU.add(t_, pos, P.COL[att], 0.45, bright=False)
                P.PA.emit(t_, pos, 'rosin', n=3, col=ROSIN)
            elif how == 'wave':
                P.BU.add(t_, pos, P.COL[att], 1.4)
                P.PA.emit(t_, pos, 'sparks', n=10, col=[tuple(P.COL[att] * 0.6 + 100)])
        elif e[1] == 'ability' and e[4] == 'wave':
            k = e[2]
            p = P.RUN['P'][P.sim_i(t_), k]
            P.FL.add(t_, S.to_world(p[0], p[1], 1.0), P.FL_COL[k] * 0.9, radius=3.0, peak=0.8, decay=0.6)

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'chips', n=24, spread=180, speed=(0.8, 2.2), col=[tuple(WOOD), tuple(WOOD * 1.3)])

    # ---------------------------------------------------------------- the canvas
    def shadows(self, st, spot):
        if st['shattered']:
            return []
        from reelkit.shadows import proj_point
        quads, _ = bow_geom(st['p'], st['th'], st['blen'], st['sd'])
        return [proj_point(Q, spot) for Q, c, kind in quads if kind in ('stick', 'hair')]

    def shadows_free(self, F, spot):
        from reelkit.shadows import proj_point
        return [proj_point(dart_quads(row[0], row[1], row[2])[0], spot) for row in self.now(F)]

    def canvas_wave(self, Ic, C, w):
        """SCREECH: a jagged, shivering ring (#28)"""
        owner, cx, cz, r, rmax, kind, t0 = w
        P, sxp, szp, ys, xs, t = self.P, C.sxp, C.szp, C.ys, C.xs, C.t
        rd = np.hypot(sxp - cx, szp - cz)
        wdt = 0.32
        k_ = (1 - r / rmax) * 1.3
        ang_ = np.arctan2(szp - cz, sxp - cx)
        rd = rd + 0.10 * (2 * ((7 * ang_ / math.pi + 6 * t) % 1) - 1) + 0.04 * np.sin(23 * ang_ - 50 * t)
        band = np.clip(1 - np.abs(rd - r) / wdt, 0, 1)
        band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
        Ic += (band * k_ * 0.9)[:, None] * (P.COL[owner] / 255.0 * 1.4 + 0.35)[None]
        return Ic

    # ---------------------------------------------------------------- the painter pass
    def items_ball(self, F, st):
        if st['shattered']:
            return []
        tip = st['p'] + (R + st['blen'] * 0.55) * np.array([math.cos(st['th']), math.sin(st['th'])])
        return [(F.cam.depth(S.to_world(tip[0], tip[1], R))[0] + 0.01, ('bow', st))]

    def items_free(self, F):
        return [(F.cam.depth(S.to_world(row[0], row[1], 0.45))[0], ('dart', row)) for row in self.now(F)]

    def draw_item(self, F, payload):
        P, cam, fr = self.P, F.cam, F.fr
        what, obj = payload
        if what == 'bow':                                   # #28's bow, its motion fan and TREMOLO's violet blur
            st = obj
            mult = self.cfg['abilities']['unlock2']['mult'] if st['haste'] > 0 else 1.0
            dth = min(2.4, st['om'] * mult * 0.055) * st['sd']
            arr, depth = fr.array(), fr.depth()
            for pts, u in bow_trail_polys(cam, st, dth):
                if st['haste'] > 0:
                    P.layer_blend(arr, [pts], P.COL[self.k] * 0.55 + 115 * (1 - 0.3 * u), 0.40 * (1 - u))
                else:
                    P.layer_blend(arr, [pts], (236, 232, 222), min(0.30, 0.04 + 0.02 * st['om']) * (1 - u))
            fr.set_array(arr, depth)
            draw_bow(fr, cam, F.L, shade, st)
        elif what == 'dart':
            row = obj
            draw_dart(fr, cam, F.L, shade, row[0], row[1], row[2], P.COL[int(row[3])])

    # ---------------------------------------------------------------- shards (#28 / #42)
    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        return draw_shard(self.P, arr, cam, kind, p, rot, size, curl, WOOD)

    # ---------------------------------------------------------------- lens
    def shimmer(self, img, cam, t, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind == 'wave':
            img = fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.35, 2.6, t, k=1 - r / rmax, seed=owner)
        return img

    def focus_points(self, t, st):
        """the bow tip (#42's focus check)"""
        tip = st['p'] + (R + st['blen']) * np.array([math.cos(st['th']), math.sin(st['th'])])
        return [(S.to_world(tip[0], tip[1], R), 'bow tip')]


# ===================================================================================================== sound (#42)
def scratch(A, dur, seed=5):
    """bow scratch: rosin noise, band-limited, gritty (#28)"""
    SR = A.SR
    n = int(dur * SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    x = A.bp(x, 1800, 7000) * (1 + 0.8 * np.sign(np.sin(2 * np.pi * 37 * A.tt(n))))
    return x * A.expenv(n, dur * 0.4, 0.004)


def pizz(A, m, vel=0.8, muted=False):
    """pizzicato: a plucked string with a little wooden body (#28); `muted` = palm-muted, short and dull (#42)"""
    SR = A.SR
    n = int((0.25 if muted else 0.5) * SR)
    f = A.mtof(m)
    t = A.tt(n)
    dec = 0.05 if muted else 0.16
    x = (np.sin(2 * np.pi * f * t) + 0.5 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.05)
         + 0.25 * np.sin(6 * np.pi * f * t) * np.exp(-t / 0.03)) * np.exp(-t / dec)
    x += 0.15 * A.bp(np.random.default_rng(m).normal(0, 1, n), 800, 3000) * np.exp(-t / 0.01)
    return A.lp(x, 1600 if muted else 4500) * vel


def squeal(A, dur=1.8):
    """SCREECH: the beginner-violin squeal: a wobbling, scratchy whistle sliding up (#28)"""
    SR = A.SR
    n = int(dur * SR)
    t = A.tt(n)
    f = 2300 + 500 * np.sin(2 * np.pi * 6.5 * t) + 900 * t + 300 * np.sign(np.sin(2 * np.pi * 3.1 * t))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) + 0.35 * np.sin(2 * np.pi * np.cumsum(f * 2.02) / SR)
    x = np.tanh(2.8 * x) * np.clip(t / 0.03, 0, 1) * np.exp(-t / (dur * 0.55))
    return A.bp(x, 700, 8000) + 0.5 * scratch(A, dur, 13)


class Sound(base.Sound):
    bus_gain, send = 1.6, 0.4
    play_first = True                                        # #42 scored the violin's part before the viola's
    band = (350, 4000)                                       # #42's band check: the line G4-C6 + bright harmonics
    TITLE = (67, 70, 72, 75, 77, 79, 84)                    # #42's flourish up to a high C (in C minor)
    REASSEMBLE = (67, 72, 75, 79, 84, 87)                   # #42's rising pizz run home (in C minor)

    def __init__(self, M, k):
        super().__init__(M, k)
        self.pan = 0.2 if k == 1 else -0.2                  # #42 / #28: the right corner, 0.2

    def bow_note(self, m, n16=2, vel=0.8):
        """a bowed violin note held for n16 sixteenths (the bow decides the length) (#28)"""
        dur = max(0.07, n16 * self.T.S16 * 0.92)
        return self.M.note(self.M.CFG[self.k]['pp'], m, round(dur, 3), round(dur + 0.25, 2), round(vel, 1))

    def voice(self, m, dur, vel=0.85):
        return self.bow_note(m, max(1, int(round(dur / self.T.S16))), vel)

    def melody_windows(self):
        """the enemy's MELODY AT LAST spans (#42: from the beat after it fires to its end or the K.O.)"""
        T, M = self.T, self.M
        out = []
        for e in M.EV:
            if e[1] == 'melody' and e[2] != self.k:
                s = math.ceil((e[0] + 1e-6) / T.BEAT) * T.BEAT
                out.append((s, min(e[3], M.T_KO)))
        return out

    def title_blast(self, mix, t):
        T = self.T
        sh = key_shift(T)
        ms = T.TITLE_HITS.get(self.name, tuple(m + sh for m in self.TITLE))
        for j, m in enumerate(ms):
            last = j == len(ms) - 1
            mix.place(self.bus, self.bow_note(m, 5 if last else 1, 0.95), t + j * T.S16 / 2, 0.36 if last else 0.28,
                      pan=self.pan)

    def count_in(self, mix):
        """a pizz on each count (the count-in chord, high), then a bowed pickup into FIGHT!"""
        T, A = self.T, self.M.A
        ms = T.chord_tones(79, 91, T.CHORDS[T.FIGHT_BAR - 1])[:3]
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place(self.bus, pizz(A, ms[j % len(ms)], 0.9), tc + T.S16 / 2, 0.18, pan=self.pan)
        mix.place(self.bus, self.bow_note(ms[0], 3, 0.9), T.T_PICKUP + 2 * T.S16, 0.22, pan=self.pan)

    def play(self, mix):
        M, T, A, k = self.M, self.T, self.M.A, self.k
        S16, q16 = T.S16, M.q16
        mel_win = self.melody_windows()

        def in_melody(t):
            return any(a <= t < b for a, b in mel_win)
        # the bow's quarter turns play the line (#28)
        th = M.RUN['TH'][:, k]
        d = (np.diff(th) + np.pi) % (2 * np.pi) - np.pi
        phi = np.concatenate([[0.0], np.cumsum(np.abs(d))])
        tt = T.T_FIGHT + np.arange(len(phi)) / T.FPS
        trig, kq = [], 1
        for i in range(1, len(phi)):
            while phi[i] >= kq * math.pi / 2:
                u = (kq * math.pi / 2 - phi[i - 1]) / max(1e-9, phi[i] - phi[i - 1])
                trig.append(tt[i - 1] + u / T.FPS)
                kq += 1
        trig = sorted(set(round(q16(t) / S16) for t in trig if t < M.T_KO))
        thin = M.Thinner(3)
        for j, slot in enumerate(trig):
            tq = slot * S16
            nxt = trig[j + 1] * S16 if j + 1 < len(trig) else tq + 4 * S16
            n16 = int(np.clip(round((nxt - tq) / S16), 1, 8))
            hp = M.hp_at(k, tq) / 100.0
            if not thin.keep(slot, hp):
                continue
            m = T.line_at(self.name, tq)
            soft = in_melody(tq)
            if soft:                                                   # the enemy sings: harmonise softly
                m = min(T.chord_tones(64, 79, T.chord_at(tq)), key=lambda x: abs(x - (m - 3)))
            vel = 0.62 + 0.25 * min(1.0, 2.0 / n16)
            sig, g, lead = self.colour(self.bow_note(m, n16, vel), tq, 1.0)
            mix.place(self.bus, sig, tq, (0.14 if soft else 0.34) * g, pan=self.pan)
            if (1 - hp) > 0.25:
                mix.place(self.bus, scratch(A, min(0.25, n16 * S16), slot), tq, 0.09 * (1 - hp) ** 1.3, pan=self.pan)
            if lead > 0.04 and not soft:                               # the leader: a double-stop a sixth below
                under = [x for x in T.chord_tones(m - 10, m - 7, T.chord_at(tq))]
                if under:
                    mix.place(self.bus, self.colour(self.bow_note(under[0], n16, 0.6), tq)[0], tq, 0.14 * g, pan=self.pan)
        climb = [m for m in range(74, 99) if (m - T.KEY_ROOT) % 12 in T.SCALE]
        ab = self.M.CFG[k]['abilities']
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'hit' and e[2] == k:
                tq = q16(t)
                how, sub = e[7], e[8]
                pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
                if how == 'blade':                                    # accents climbing the scale, one per landed hit
                    m = climb[int(sub) % len(climb)]
                    sig, g, _ = self.colour(self.bow_note(m, 1, 1.0), t)
                    mix.place(self.bus, sig, tq, 0.30 * g, pan=pan)
                    mix.place(self.bus, scratch(A, 0.05, int(t * 11)), tq, 0.05, pan=pan)
                elif how == 'key':                                    # a pizzicato dart lands
                    ms = T.chord_tones(69, 93, T.chord_at(tq))
                    mix.place(self.bus, self.colour(pizz(A, ms[int(sub) * 2 % len(ms)], 0.9), t)[0], tq, 0.30, pan=pan)
                elif how == 'wave':
                    mix.place('fx', A.thud(0.45, 110, 40), tq, 0.30)
            # (#42 also tested 'crushed' darts against the wrong field, so that pluck never sounded: left out)
            elif kind == 'ability' and e[2] == k:
                tq = q16(t)
                if e[4] == 'arc':                                      # PIZZICATO: the darts launch on the 16ths
                    n = ab['unlock1']['n']
                    ms = T.chord_tones(69, 93, T.chord_at(tq))
                    for j in range(n):
                        mix.place(self.bus, self.colour(pizz(A, ms[j * 2 % len(ms)], 0.8), t)[0], tq + j * S16, 0.24,
                                  pan=self.pan)
                elif e[4] == 'haste':                                  # TREMOLO: a shiver of 32nds an octave up
                    dur = ab['unlock2']['dur']
                    j = 0
                    while j * S16 / 2 < dur:
                        tk = tq + j * S16 / 2
                        mix.place(self.bus, self.colour(self.bow_note(T.line_at(self.name, tk) + 12, 1, 0.5), t)[0], tk,
                                  0.07 * (0.5 + 0.5 * j * S16 / 2 / dur), pan=self.pan)
                        j += 1
                elif e[4] == 'wave':                                   # SCREECH
                    mix.place(self.bus, squeal(A, 1.9), tq, 0.14, pan=self.pan)
                    mix.place(self.bus, self.colour(self.bow_note(96, 6, 1.0), t, 2.0)[0], tq, 0.16, pan=self.pan)

    def fall_apart(self, mix):
        """a scratchy run down"""
        M, A = self.M, self.M.A
        for j, m in enumerate((88, 84, 79, 75, 72, 67)):
            mix.place(self.bus, self.bow_note(m, 1, 0.8), M.P.T_HS + 0.1 * j, 0.20, pan=(j - 2.5) / 6)
            mix.place(self.bus, A.hp(scratch(A, 0.06, 40 + j), 3000), M.P.T_HS + 0.1 * j, 0.08, pan=(j - 2.5) / 6)

    def stinger(self, mix, tw):
        T = self.T
        for j, m in enumerate(T.chord_tones(72, 84, T.CHORDS[0])[:4]):
            mix.place('solo', self.bow_note(m, 1 if j < 3 else 8, 1.0), tw + j * T.S16 / 2, 0.30)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            mix.place('solo', self.bow_note(m, max(1, int(round(d * 4))), 0.9), ts, 0.46)

    def reassemble(self, mix):
        T, M = self.T, self.M
        sh = key_shift(T)
        for j, m in enumerate(self.REASSEMBLE):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / 6)
            mix.place(self.bus, pizz(M.A, m + sh, 0.7), tj, 0.18, pan=self.pan)

    def last_bar(self, mix):
        """climbing the last chord into the slam, on the off-8ths"""
        T = self.T
        t_last = (T.N_BARS - 1) * T.BAR
        ms = T.chord_tones(79, 91, T.CHORDS[-1])[:4]
        for bt in range(4):
            mix.place(self.bus, self.bow_note(ms[bt % len(ms)], 2, 0.9), t_last + bt * T.BEAT + 2 * T.S16, 0.16,
                      pan=self.pan)
