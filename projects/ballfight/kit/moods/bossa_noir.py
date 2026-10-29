"""Mood: minor bossa noir (#49 Flute vs Clarinet). A sly, smoky late-night bossa nova with NO woodwinds of its own (the
fighters are the front line: the flute sings the high melody, the clarinet the liquid counter-line below it):

    guitar      a nylon-string guitar comping the bossa rhythm (Karplus-Strong with a soft thumb-and-fingers attack and a
                warm, dark tone): the 2-bar "3-2" bossa figure on the 8ths, the thumb's bass under every chord; ghost
                strokes fill the gaps in the back half
    bass        a soft upright-style bass: the bossa's dotted-quarter + eighth (root on 1, fifth on the and of 2, fifth
                on 3, a pickup into the next root on the and of 4)
    drums       brushes swirling on every 8th (straight 8ths: bossa doesn't swing), a felt bass drum on 1 and the and of
                2 / 3 and the and of 4, cross-stick rim clicks on the bossa clave, a shaker joining in the back half,
                soft crashes on the phrase lines
    keys        a dark Rhodes pad (FM tines through a slow tremolo, low-passed) holding each bar's chord; fuller (a
                second voicing an octave up) in the back half

Bossa is soft by nature: here it stays at the series' loudness from frame 0 by density (four instruments on every bar
from the VS card on, a fuller arrangement later) and the master, never by playing quietly. The VS card is three band
hits (bass + a rolled nylon chord + Rhodes + kick and a choked crash) and a held tremolo chord; the count-in keeps the
groove; the last bar walks the bass up chromatically under rolling rim clicks and a riser into the title slam's tonic.
Built for D minor (Dm7 Dm6 Gm7 C7 Fmaj7 Bbmaj7 Em7b5 A7); any chords work (voicings by rule)."""
import math

import numpy as np
from scipy.signal import lfilter

CLAVE = ((0, 3, 6), (2, 5))                        # the bossa's 2-bar "3-2" figure on the 8ths (bar even / odd)


class Bed:
    buses = {'drums': (1.55, 0.14), 'bass': (1.9, 0.10), 'guitar': (1.85, 0.26), 'keys': (1.35, 0.34)}  # (gain, send)
    reverb = (1.7, 0.5, 0.18)                      # a small smoky club, a little velvet: (dur, decay, wet)
    comp = (0.16, 0.55)                            # master compressor: threshold, exponent
    roar_gain, doors_gain = 0.16, 0.40
    landing = (0.07, 0.05)                         # caption landing thuds (first line, others)
    pop = (93, 98, 0.10)                           # the glass pop when the loser is whole again (two chimes, gain)

    def __init__(self, M):
        self.M, self.T, self.A, self.SR = M, M.T, M.A, M.SR
        self._c = {}

    def _cache(self, key, fn):
        if key not in self._c:
            self._c[key] = fn()
        return self._c[key]

    def t8(self, b, j):
        return b * self.T.BAR + j * self.T.BEAT / 2

    # ---------------------------------------------------------------- harmony
    def root(self, ch):
        return self.T.root_above(ch, 33)                         # the bass's root, A1..G#2

    def fifth(self, ch):
        r = self.root(ch)
        pcs = self.T.CHORD_PCS[ch]
        f = r + ((pcs[2] - pcs[0]) % 12 if len(pcs) > 2 else 7)
        return f if f <= 47 else f - 12

    def voicing(self, ch, lo=52):
        """the guitar's four-note shape (E3 up): third, seventh (or sixth / doubled root), fifth, a ninth on top for
        the minor and major-seventh chords (the bossa colour), the dominant's flat ninth (noir)"""
        T = self.T
        pcs = T.CHORD_PCS[ch]
        r = pcs[0]
        tones = [pcs[1], pcs[3] if len(pcs) > 3 else r, pcs[2]]
        third = (pcs[1] - r) % 12
        sev = (pcs[3] - r) % 12 if len(pcs) > 3 else None
        if third == 4 and sev == 10:
            tones.append((r + 1) % 12)                           # dominant: the flat ninth
        elif (pcs[2] - r) % 12 == 6:
            tones.append((r + 5) % 12)                           # half-diminished: the eleventh
        else:
            tones.append((r + 2) % 12)                           # minor / major / sixth: the ninth
        out = []
        for pc in tones:
            m = lo + (pc - lo) % 12
            while m in out:
                m += 12
            out.append(m)
        return tuple(sorted(out))

    # ---------------------------------------------------------------- instruments
    def pluck(self, m, dur, seed=0, bright=0.3, ring=0.996):
        """a plucked nylon string (Karplus-Strong: a soft, filtered burst through a tuned loop; dark and round)"""
        A, SR = self.A, self.SR

        def make():
            f = A.mtof(m)
            N = max(2, int(round(SR / f)))
            n = int(dur * SR)
            exc = np.zeros(n)
            burst = np.random.default_rng(2000 + int(m) + 37 * seed).normal(0, 1, N)
            exc[:N] = A.lp(A.lp(burst, 900 + 3000 * bright), 900 + 3000 * bright)   # the flesh of the finger
            a = np.zeros(N + 2)
            a[0], a[N], a[N + 1] = 1.0, -0.5 * ring, -0.5 * ring
            y = lfilter([1.0], a, exc)
            t = A.tt(n)
            return y * np.clip(t / 0.003, 0, 1)
        return self._cache(('pl', int(m), round(dur, 3), seed, bright, ring), make)

    def comp_chord(self, ch, v=1.0, dur=0.32, ghost=False, roll=0.004):
        """the guitar's bossa stroke: the fingers pluck the voicing together (a hair rolled), damped after `dur`; a
        ghost stroke is short and soft"""
        A, SR = self.A, self.SR

        def make():
            vo = self.voicing(ch)
            d = int(roll * SR)
            ln = dur + 0.08
            n = int(ln * SR) + d * len(vo)
            x = np.zeros(n)
            for j, m in enumerate(vo):
                s = self.pluck(m, ln, seed=j, bright=0.22 if ghost else 0.34)
                x[j * d:j * d + len(s)] += s * (0.85 + 0.06 * j)
            t = A.tt(n)
            cut = 0.07 if ghost else dur
            env = np.where(t < cut, 1.0, np.exp(-(t - cut) / 0.03))
            y = A.lp(x / len(vo), 3200) * env
            return y * (0.42 if ghost else 1.0)
        return self._cache(('cc', ch, ghost, round(dur, 3), roll), make) * v

    def thumb(self, m, dur, v=1.0):
        """the guitar's thumb on the bass string (soft, round)"""
        A = self.A
        return self._cache(('th', int(m), round(dur, 3)),
                           lambda: A.lp(self.pluck(m, dur, seed=9, bright=0.18, ring=0.995), 1400)) * v

    def roll_ring(self, ch, dur, v=1.0):
        """a slow rolled chord left ringing (the band hits)"""
        A, SR = self.A, self.SR

        def make():
            vo = self.voicing(ch) + (self.voicing(ch)[-1] + 5,)
            d = int(0.022 * SR)
            n = int((dur + 0.15) * SR) + d * len(vo)
            x = np.zeros(n)
            for j, m in enumerate(vo):
                s = self.pluck(m, dur + 0.15, seed=10 + j, bright=0.45, ring=0.998)
                x[j * d:j * d + len(s)] += s
            t = A.tt(n)
            return A.lp(x / len(vo), 4000) * np.exp(-t / max(0.35, dur * 0.7))
        return self._cache(('rr', ch, round(dur, 3)), make) * v

    def tremolo(self, ch, dur, v=1.0):
        """the nylon guitar's tremolo (the card's held chord): soft strokes on the 16ths"""
        T = self.T
        n = int((dur + 0.3) * self.SR)
        out = np.zeros(n)
        step = T.S16
        j = 0
        while j * step < dur - 1e-6:
            s = self.comp_chord(ch, 0.55 + 0.2 * (j % 2 == 0), dur=0.12)
            a = int(j * step * self.SR)
            b = min(n, a + len(s))
            out[a:b] += s[:b - a]
            j += 1
        return out * v

    def upright(self, m, dur, v=1.0):
        """a soft upright-style bass: a plucked low string (KS) on a sine body, the finger's thump, dark"""
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.10) * SR)
            t = A.tt(n)
            f = A.mtof(m)
            body = (np.sin(2 * np.pi * f * t) + 0.35 * np.sin(4 * np.pi * f * t) + 0.12 * np.sin(6 * np.pi * f * t))
            body *= np.exp(-t / 0.55)
            st = self.pluck(m + 12, dur + 0.10, seed=3, bright=0.12, ring=0.993)[:n]
            thump = A.lp(np.random.default_rng(int(m)).normal(0, 1, n), 300) * np.exp(-t / 0.015) * 0.6
            env = np.clip(t / 0.006, 0, 1) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.05))
            y = A.lp(body * 0.8 + st * 0.5 + thump, 1100) * env
            return y
        return self._cache(('ub', int(m), round(dur, 3)), make) * v

    def rhodes(self, ch, dur, v=1.0, up=0):
        """a dark Rhodes pad: FM tines (index falling from bright to round), a slow tremolo, low-passed"""
        A, SR = self.A, self.SR

        def make():
            vo = [m + 12 * up for m in self.voicing(ch, lo=55)]
            n = int((dur + 0.3) * SR)
            t = A.tt(n)
            x = np.zeros(n)
            for j, m in enumerate(vo):
                f = A.mtof(m)
                idx = 0.4 + 1.6 * np.exp(-t / 0.25)
                tine = np.sin(2 * np.pi * f * t + idx * np.sin(2 * np.pi * f * t + j))
                bell = np.sin(2 * np.pi * f * 7.0 * t) * np.exp(-t / 0.03) * 0.15
                x += (tine + bell) * (1.0 - 0.08 * j)
            trem = 1.0 - 0.22 * (0.5 + 0.5 * np.sin(2 * np.pi * 4.6 * t))
            env = np.clip(t / 0.008, 0, 1) * np.exp(-t / (dur * 1.4)) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.08))
            return A.lp(x / len(vo), 2200 if not up else 3000) * trem * env
        return self._cache(('rh', ch, round(dur, 3), up), make) * v

    def kick(self, v=1.0):
        """a felt bass drum: soft, high-passed under the upright"""
        A = self.A
        return self._cache(('k',), lambda: A.hp(A.kick(0.5, 0.20, 100, 50), 38)[:int(0.30 * self.SR)]) * v

    def brush(self, v=1.0, seed=0, long=False):
        """a brush on the snare: a tap (short) or a swirl (a sweep across the beat)"""
        A, SR = self.A, self.SR

        def make():
            n = int((0.42 if long else 0.16) * SR)
            t = A.tt(n)
            x = A.bp(np.random.default_rng(40 + seed).normal(0, 1, n), 1800, 8000)
            if long:
                env = np.sin(np.pi * np.clip(t / (n / SR), 0, 1)) ** 1.5
            else:
                env = np.clip(t / 0.004, 0, 1) * np.exp(-t / 0.05)
            return x * env
        return self._cache(('br', seed % 8, long), make) * v

    def rim(self, v=1.0, seed=0):
        """a cross-stick click on the rim (woody, hollow)"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.10 * SR)
            t = A.tt(n)
            tone = np.sin(2 * np.pi * 1650 * t) * 0.6 + np.sin(2 * np.pi * 520 * t) * 0.8
            click = A.bp(np.random.default_rng(70 + seed).normal(0, 1, n), 1500, 6500) * np.exp(-t / 0.004)
            return (tone * np.exp(-t / 0.018) + click) * np.clip(t / 0.0006, 0, 1)
        return self._cache(('rim', seed % 4), make) * v

    def shaker(self, v=1.0, seed=0, accent=False):
        A, SR = self.A, self.SR

        def make():
            n = int(0.12 * SR)
            t = A.tt(n)
            x = A.hp(np.random.default_rng(90 + seed).normal(0, 1, n), 6000)
            return x * np.clip(t / 0.012, 0, 1) * np.exp(-t / (0.035 if accent else 0.022))
        return self._cache(('sh', seed % 4, accent), make) * v

    def choke(self, v=1.0, seed=0):
        """a crash choked by hand (the band hits' "pah")"""
        A, SR = self.A, self.SR
        n = int(0.24 * SR)
        t = A.tt(n)
        x = A.hp(np.random.default_rng(80 + seed).normal(0, 1, n), 4000) * np.exp(-t / 0.06)
        return x * np.clip((0.22 - t) / 0.06, 0, 1) * v

    # ---------------------------------------------------------------- one bar
    def bar(self, mix, b, intensity=1.0, light=False):
        """one bar of the bossa (bar index b): bass dotted-quarter + eighth, the guitar on the clave figure with the
        thumb on 1 and 3, the Rhodes holding the chord, brushes on the 8ths, kick and rim clicks; the back half
        (`intensity` > 1.25) adds the shaker, guitar ghost strokes, the Rhodes' upper voicing and bass pickups"""
        T, A = self.T, self.A
        t0 = b * T.BAR
        ch = T.CHORDS[b % T.N_BARS]
        nxt = T.CHORDS[(b + 1) % T.N_BARS]
        big = intensity > 1.25 and not light
        r, f5 = self.root(ch), self.fifth(ch)
        e8 = T.BEAT / 2
        # the bass: root on 1 (dotted quarter), fifth on the and of 2, fifth on 3 (dotted quarter), a pickup on the and of 4
        mix.place('bass', self.upright(r, 1.4 * T.BEAT, 1.0), t0, 0.62)
        mix.place('bass', self.upright(f5, 0.45 * T.BEAT, 0.8), t0 + 3 * e8, 0.46)
        mix.place('bass', self.upright(f5, 1.4 * T.BEAT, 0.9), t0 + 2 * T.BEAT, 0.54)
        nr = self.root(nxt)
        pk = (nr - 1 if nxt != ch else r) if big else r
        mix.place('bass', self.upright(pk, 0.45 * T.BEAT, 0.8), t0 + 7 * e8, 0.42 if big else 0.36)
        # the guitar: the thumb on 1 and 3, the fingers on the clave
        mix.place('guitar', self.thumb(r + 12, 0.9 * T.BEAT), t0, 0.36, pan=0.2)
        mix.place('guitar', self.thumb(f5 + 12, 0.9 * T.BEAT), t0 + 2 * T.BEAT, 0.32, pan=0.2)
        hits = CLAVE[b % 2]
        for j in hits:
            mix.place('guitar', self.comp_chord(ch, 1.0, dur=0.30), t0 + j * e8, 0.50, pan=0.2)
        if big:
            for j in range(8):
                if j not in hits and j % 2 == 1:
                    mix.place('guitar', self.comp_chord(ch, ghost=True), t0 + j * e8, 0.34, pan=0.2)
        # the Rhodes: the chord held across the bar (a second voicing an octave up in the back half)
        mix.place('keys', self.rhodes(ch, T.BAR * 0.96), t0, 0.30 if not light else 0.24, pan=-0.2)
        if big:
            mix.place('keys', self.rhodes(ch, T.BAR * 0.9, up=1), t0 + 0.01, 0.12, pan=-0.28)
        # drums: brushes on the 8ths (a swirl on each beat, taps on the off-beats), kick, rim clicks on the clave
        for j in range(8):
            tj = t0 + j * e8
            if j % 2 == 0:
                mix.place('drums', self.brush(1.0, seed=b + j, long=True), tj, 0.065, pan=-0.15)
            else:
                mix.place('drums', self.brush(1.0, seed=b + j), tj, 0.10 if j in (3, 7) else 0.075, pan=-0.15)
            if big:
                mix.place('drums', self.shaker(1.0, seed=j, accent=j % 2 == 1), tj, 0.045, pan=0.35)
        for j in (0, 3, 4, 7):                                    # kick: 1, and of 2, 3, and of 4
            mix.place('drums', self.kick(1.0 if j in (0, 4) else 0.7), t0 + j * e8, 0.42 if not light else 0.34)
        for j in CLAVE[b % 2]:                                    # rim clicks on the clave with the guitar
            mix.place('drums', self.rim(1.0, seed=j), t0 + j * e8, 0.11, pan=0.12)

    def boom(self, mix, t, g):
        """a big drum hit (a slam landing)"""
        mix.place('drums', self.kick(1.0), t, g)
        r = self.root(self.T.chord_at(t))
        mix.place('bass', self.upright(r, 0.5, 1.0), t, 0.45 * g)

    def clash(self, mix, t):
        mix.place('fx', self.rim(1.0, seed=3), t, 0.05)

    def hit(self, mix, t, ch, g=1.0, ring=0.9):
        """a full band hit: the bass root, a rolled nylon chord ringing, the Rhodes, kick + a choked crash"""
        r = self.root(ch)
        mix.place('bass', self.upright(r, ring, 1.0), t, 0.66 * g)
        mix.place('guitar', self.roll_ring(ch, ring), t, 0.62 * g, pan=0.2)
        mix.place('keys', self.rhodes(ch, ring), t, 0.34 * g, pan=-0.2)
        mix.place('drums', self.kick(1.0), t, 0.60 * g)
        mix.place('drums', self.choke(1.0, int(t * 10)), t, 0.14 * g)

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        """a full band hit under each word of the VS card"""
        A, ch = self.A, self.T.CHORDS[0]
        for tw_ in times:
            self.hit(mix, tw_, ch, 1.0, 0.45)
            mix.place('drums', A.crash(1.0), tw_, 0.07)
            mix.place('fx', A.thud(0.8, 120, 34, seed=int(tw_ * 50) + 3), tw_, 0.30)

    def vs(self, mix, t):
        """VS: a thunder crack where it splits the divider"""
        mix.place('fx', self.M.thunder(1.4, seed=91), t, 0.40)

    def title_rest(self, mix):
        """the held chord under the card (never quiet): the nylon tremolo, the Rhodes, a long bass root, brushes
        swelling, rim clicks rolling on beat 4 as the doors open"""
        T, A = self.T, self.A
        ch = T.CHORDS[0]
        mix.place('guitar', self.tremolo(ch, 3 * T.BEAT - 0.1), 0.08, 0.42, pan=0.2)
        mix.place('keys', self.rhodes(ch, 3 * T.BEAT), 0.05, 0.30, pan=-0.2)
        mix.place('bass', self.upright(self.root(ch), 2.9 * T.BEAT, 0.8), 0.05, 0.46)
        n = int(3 * T.BEAT * self.SR)
        sw = A.bp(np.random.default_rng(5).normal(0, 1, n), 1800, 8000) * np.linspace(0.2, 1.0, n) ** 2
        mix.place('drums', sw, 0.0, 0.05)
        for k in range(4):
            mix.place('drums', self.rim(0.6 + 0.13 * k, seed=k), 3 * T.BEAT + k * T.S16, 0.13)

    def count_in(self, mix):
        """bar 1: the band keeps the bossa on the count-in chord (the groove from the card on), a band stab on each
        count, rim clicks and a bass walk-up into FIGHT!"""
        T = self.T
        b = 1
        self.bar(mix, b, intensity=1.4)
        ch = T.CHORDS[b]
        for tc, txt in T.COUNT[:3]:
            self.hit(mix, tc, ch, 0.5, 0.28)
        t4 = T.BAR + 3 * T.BEAT
        for k in range(4):
            mix.place('drums', self.rim(0.6 + 0.13 * k, seed=k), t4 + k * T.S16, 0.14)
        mix.place('bass', self.upright(self.root(T.CHORDS[2]) - 1, 0.4 * T.BEAT, 0.85), t4 + 2 * T.S16, 0.44)

    def fight_start(self, mix):
        mix.place('drums', self.A.crash(1.8), self.T.T_FIGHT, 0.11)
        mix.place('fx', self.A.thud(0.7, 110, 34), self.T.T_FIGHT, 0.34)

    def fight(self, mix, bar_ko):
        """the bossa from FIGHT! to the K.O. (denser as it goes), a riser + rim roll into the back half, soft crashes
        on the phrase lines, a tape stop at the K.O."""
        T, A = self.T, self.A
        for b in range(2, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
        for b in range(2, bar_ko + 1, 8):
            mix.place('drums', A.crash(1.6), b * T.BAR, 0.07)
        t_half = (T.LOOP_START + 2 * len(T.LOOP)) * T.BAR
        for k in range(8):
            mix.place('drums', self.rim(0.5 + 0.06 * k, seed=k), t_half - T.BEAT * 2 + k * T.BEAT / 4, 0.12)
        mix.place('fx', self.M.riser(T.BAR, over=0.0, seed=12) * 0.6, t_half - T.BAR, 0.08)
        mix.place('drums', A.crash(2.0), t_half, 0.10)
        for k in ('drums', 'bass', 'guitar', 'keys'):
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], self.M.T_KO, 0.42)

    def ko(self, mix):
        mix.place('drums', self.A.crash(2.4), self.M.T_KO, 0.15)
        mix.place('fx', self.A.thud(0.9, 120, 30), self.M.T_KO, 0.6)

    def stinger(self, mix, tw):
        self.hit(mix, tw, self.T.CHORDS[0], 1.0, 0.8)
        mix.place('drums', self.A.crash(1.8), tw, 0.09)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        mix.place('fx', self.A.whoosh(max(0.2, t_solo - tw - 0.1), 300, 5000), tw + 0.1, 0.08)

    def victory(self, mix, b_solo):
        """the band plays on under the solo at full density (a victory, never a quiet stretch), up to the build"""
        T = self.T
        for b in range(b_solo, T.N_BARS - 1):
            self.bar(mix, b, intensity=1.4)

    def last_bars(self, mix):
        """the last bar builds into the title slam on sample 0: rim clicks rolling up to the seam, the bass walking up
        chromatically into the tonic, the guitar on every 8th, the kick on every beat, the Rhodes, a riser"""
        T, M, A = self.T, self.M, self.A
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.CHORDS[-1]
        tgt = self.root(T.CHORDS[0])
        mix.place('keys', self.rhodes(ch, T.BAR), t_last, 0.30, pan=-0.2)
        mix.place('keys', self.rhodes(ch, T.BAR, up=1), t_last, 0.12, pan=-0.28)
        for bt in range(4):
            tb = t_last + bt * T.BEAT
            mix.place('bass', self.upright(tgt - 4 + bt, 0.6 * T.BEAT, 0.9), tb, 0.60)
            mix.place('drums', self.kick(0.95), tb, 0.46)
            mix.place('drums', self.brush(1.0, seed=bt, long=True), tb, 0.07, pan=-0.15)
            for h in range(2):
                mix.place('guitar', self.comp_chord(ch, 1.0, dur=0.18), tb + h * T.BEAT / 2, 0.46, pan=0.2)
        n16 = int((T.DUR - t_last) / T.S16 + 1e-6)
        for k in range(n16):
            mix.place('drums', self.rim(0.45 + 0.55 * k / max(1, n16 - 1), seed=k), t_last + k * T.S16, 0.13)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.18)
