"""Mood: dark hot jazz (#46 Trumpet vs Clarinet). A fast minor two-beat from a 1920s hot band with NO horns and NO
clarinet of its own (the fighters are the front line: the trumpet blasts the melody, the clarinet weaves around it):

    tuba        the bass on 1 and 3 (root, then the fifth; the Pixel Pitch tuba), pickups into the changes later on
    guitar      a piano-less rhythm guitar chopping the chord on 2 and 4 (a muted plucked strum, dark voicings: the
                dominant gets its flat ninth), ghost strokes on 1 and 3 in the back half
    drums       brushed cymbals on every beat with a light swung skip, a felt bass drum on 1 and 3, brush taps on 2 and 4
                that become snare press rolls (the buzz) in the back half, a press roll into every phrase, woodblock
                clicks on the off-beats, crashes on the phrase lines

The arrangement thickens with the bars (press rolls, ghost chops, woodblock, tuba pickups), never with level. The VS
card is three full band hits and a held tremolo chord; the count-in keeps the groove (consistent loudness from frame
0); the last bar walks the tuba up chromatically under a press roll and a riser into the title slam's tonic. Built for
D minor (Dm Gm Bb7 A7); any chords work (voicings by rule)."""
import math

import numpy as np
from scipy.signal import lfilter

SWING = 0.58                                       # the off-beat 8th at 58 % of the beat: light swing at a fast two-beat


class Bed:
    buses = {'drums': (1.45, 0.14), 'tuba': (1.75, 0.12), 'guitar': (1.6, 0.28)}  # bus: (gain, reverb send)
    reverb = (1.4, 0.45, 0.16)                     # a low-ceilinged club: (dur, decay, wet)
    comp = (0.18, 0.55)                            # master compressor: threshold, exponent
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

    # ---------------------------------------------------------------- harmony
    def root(self, ch):
        return self.T.root_above(ch, 36)                         # the tuba's root, C2..B2

    def fifth(self, ch):
        r = self.root(ch)
        pcs = self.T.CHORD_PCS[ch]
        f = r + ((pcs[2] - pcs[0]) % 12 if len(pcs) > 2 else 7)
        return f if f <= 50 else f - 12

    def voicing(self, ch):
        """the guitar's four-note chop (D3-F4): root low, third and seventh (or the doubled root) in the middle, a
        dominant's flat ninth on top (dark)"""
        T = self.T
        pcs = T.CHORD_PCS[ch]
        r = pcs[0]
        tones = [r, pcs[1], pcs[2]] + ([pcs[3]] if len(pcs) > 3 else [r])
        if len(pcs) > 3 and (pcs[1] - r) % 12 == 4 and (pcs[3] - r) % 12 == 10:
            tones[2] = (r + 1) % 12                              # 7th chords: the flat ninth replaces the fifth
        out = []
        for pc in tones:                                         # each tone at its lowest from D3, a doubled one an octave up
            m = 50 + (pc - 50) % 12
            while m in out:
                m += 12
            out.append(m)
        return tuple(sorted(out))

    # ---------------------------------------------------------------- instruments
    def tuba(self, m, dur, vel=0.9):
        return self.M.note('TUBA', int(m), round(dur, 3), round(dur + 0.12, 2), round(vel, 1))

    def pluck(self, m, dur, seed=0, bright=0.55, ring=0.994):
        """a plucked steel string (Karplus-Strong: a filtered noise burst through a tuned delay loop)"""
        A, SR = self.A, self.SR

        def make():
            f = A.mtof(m)
            N = max(2, int(round(SR / f)))
            n = int(dur * SR)
            exc = np.zeros(n)
            burst = np.random.default_rng(1000 + int(m) + 37 * seed).normal(0, 1, N)
            exc[:N] = A.lp(burst, 1500 + 5000 * bright)
            a = np.zeros(N + 2)
            a[0], a[N], a[N + 1] = 1.0, -0.5 * ring, -0.5 * ring
            return lfilter([1.0], a, exc)
        return self._cache(('pl', int(m), round(dur, 3), seed, bright, ring), make)

    def chop(self, ch, v=1.0, ghost=False, dur=0.15):
        """the rhythm guitar's chop: a fast down-strum of the voicing, choked by the fretting hand after ~0.12 s (a
        ghost stroke: shorter, darker, mostly the pick's scratch)"""
        A, SR = self.A, self.SR

        def make():
            vo = self.voicing(ch)
            d = int(0.006 * SR)
            n = int((dur + 0.05) * SR) + d * len(vo)
            x = np.zeros(n)
            for j, m in enumerate(vo):
                s = self.pluck(m, dur + 0.05, seed=j, bright=0.35 if ghost else 0.55)
                x[j * d:j * d + len(s)] += s * (0.85 + 0.05 * j)
            t = A.tt(n)
            cut = 0.06 if ghost else dur
            env = np.clip(t / 0.002, 0, 1) * np.where(t < cut, np.exp(-t / 0.10), np.exp(-cut / 0.10) * np.exp(-(t - cut) / 0.018))
            scratch = A.bp(np.random.default_rng(77).normal(0, 1, n), 1800, 6500) * np.exp(-t / 0.012) * (0.9 if ghost else 0.35)
            y = A.lp(x / len(vo), 3800) * env + scratch
            return y * (0.45 if ghost else 1.0)
        return self._cache(('ch', ch, ghost, round(dur, 3)), make) * v

    def strum_ring(self, ch, dur, v=1.0):
        """a full strum left ringing (the band hits)"""
        A, SR = self.A, self.SR

        def make():
            vo = self.voicing(ch) + (self.voicing(ch)[-1] + 5,)
            d = int(0.010 * SR)
            n = int((dur + 0.1) * SR) + d * len(vo)
            x = np.zeros(n)
            for j, m in enumerate(vo):
                s = self.pluck(m, dur + 0.1, seed=10 + j, bright=0.7, ring=0.997)
                x[j * d:j * d + len(s)] += s
            t = A.tt(n)
            return A.lp(x / len(vo), 4500) * np.exp(-t / max(0.3, dur * 0.6)) * np.clip(t / 0.002, 0, 1)
        return self._cache(('sr', ch, round(dur, 3)), make) * v

    def tremolo(self, ch, dur, v=1.0):
        """the guitar's tremolo: down-up strums on the 32nds (the card's held chord)"""
        T = self.T
        n = int((dur + 0.2) * self.SR)
        out = np.zeros(n)
        step = T.S16 / 2
        j = 0
        while j * step < dur - 1e-6:
            s = self.chop(ch, 0.55 + 0.15 * (j % 2 == 0), ghost=False, dur=0.10)
            a = int(j * step * self.SR)
            b = min(n, a + len(s))
            out[a:b] += s[:b - a]
            j += 1
        return out * v

    def kick(self, v=1.0):
        """a felt bass drum (two-beat 1 and 3): soft, high-passed so the tuba stays on top"""
        A = self.A
        return self._cache(('k',), lambda: A.hp(A.kick(0.55, 0.22, 110, 52), 42)[:int(0.32 * self.SR)]) * v

    def cymbal(self, v=1.0, seed=0):
        """a brushed cymbal: partials + a soft swish (no stick ping)"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.9 * SR)
            t = A.tt(n)
            parts = (512, 1187, 2411, 3631, 5093, 6911)
            x = sum(np.sin(2 * np.pi * f * t + i) * (0.88 ** i) for i, f in enumerate(parts)) / len(parts)
            x = x * np.exp(-t / 0.42)
            swish = A.bp(np.random.default_rng(31 + seed).normal(0, 1, n), 3500, 12000)
            swish *= np.clip(t / 0.012, 0, 1) * np.exp(-t / 0.10)
            return (x * 0.8 + swish * 0.7) * np.clip(t / 0.003, 0, 1)
        return self._cache(('cy', seed), make) * v

    def brush(self, v=1.0, seed=0):
        """a brush tap on the snare"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.2 * SR)
            t = A.tt(n)
            return A.bp(np.random.default_rng(40 + seed).normal(0, 1, n), 1600, 7500) * np.clip(t / 0.005, 0, 1) * np.exp(-t / 0.06)
        return self._cache(('br', seed), make) * v

    def press(self, dur=0.2, v=1.0, seed=0, v0=None):
        """a snare press roll: the sticks pressed into the head, a buzz of tiny bounces (a rattle at ~28 Hz on the
        snares) with the drum's body under it; v0 makes it a crescendo from v0 to v"""
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.08) * SR)
            t = A.tt(n)
            rng = np.random.default_rng(60 + seed)
            buzz = A.bp(rng.normal(0, 1, n), 1500, 8500)
            rattle = 0.55 + 0.45 * np.abs(np.sin(np.pi * 28.0 * t + rng.uniform(0, 3)))
            body = np.sin(2 * np.pi * 196 * t) * np.exp(-t / 0.05) * 0.5
            env = np.clip(t / 0.004, 0, 1) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.03))
            if v0 is None:
                env = env * (0.55 + 0.45 * np.exp(-t / 0.06))
            else:
                env = env * (v0 / max(v, 1e-6) + (1 - v0 / max(v, 1e-6)) * np.clip(t / dur, 0, 1.05) ** 1.4)
            return (buzz * rattle + body) * env
        return self._cache(('pr', round(dur, 3), seed, v0 if v0 is None else round(v0 / max(v, 1e-6), 3)), make) * v

    def woodblock(self, v=1.0, hi=True):
        A, SR = self.A, self.SR

        def make():
            n = int(0.09 * SR)
            t = A.tt(n)
            f = 1850 if hi else 1320
            x = np.sin(2 * np.pi * f * t) + 0.4 * np.sin(2 * np.pi * f * 2.7 * t) * np.exp(-t / 0.008)
            return x * np.exp(-t / 0.022) * np.clip(t / 0.001, 0, 1)
        return self._cache(('wb', hi), make) * v

    def choke(self, v=1.0, seed=0):
        """a crash choked by hand (the band hits' "pah")"""
        A, SR = self.A, self.SR
        n = int(0.24 * SR)
        t = A.tt(n)
        x = A.hp(np.random.default_rng(80 + seed).normal(0, 1, n), 4000) * np.exp(-t / 0.06)
        return x * np.clip((0.22 - t) / 0.06, 0, 1) * v

    # ---------------------------------------------------------------- one bar
    def bar(self, mix, b, intensity=1.0, light=False):
        """one bar of the two-beat (bar index b): the tuba on 1 + 3, the guitar's chop on 2 + 4, brushed cymbals on
        every beat with the swung skip, the felt bass drum on 1 + 3, brush taps on 2 + 4; the back half (`intensity`
        > 1.25) adds snare press rolls on 2 + 4, ghost chops on 1 + 3, woodblock off-beats and tuba pickups"""
        T, A = self.T, self.A
        t0 = b * T.BAR
        ch = T.CHORDS[b % T.N_BARS]
        nxt = T.CHORDS[(b + 1) % T.N_BARS]
        big = intensity > 1.25 and not light
        r, f5 = self.root(ch), self.fifth(ch)
        mix.place('tuba', self.tuba(r, 0.72 * T.BEAT, 0.95), t0, 0.62)
        mix.place('tuba', self.tuba(f5, 0.72 * T.BEAT, 0.85), t0 + 2 * T.BEAT, 0.56)
        if big:                                                  # a pickup into the next root: the and of 4
            nr = self.root(nxt)
            pk = nr - 1 if nxt != ch else f5 + (2 if f5 + 2 <= 50 else -2)
            mix.place('tuba', self.tuba(pk, 0.32 * T.BEAT, 0.8), t0 + (3 + SWING) * T.BEAT, 0.42)
        for bt in range(4):
            tb = t0 + bt * T.BEAT
            if bt in (0, 2):
                mix.place('drums', self.kick(), tb, 0.50 if not light else 0.40)
                if big:
                    mix.place('guitar', self.chop(ch, ghost=True), tb, 0.30, pan=0.18)
            else:
                mix.place('guitar', self.chop(ch, 1.0), tb, 0.46, pan=0.18)
                if big:
                    mix.place('drums', self.press(0.20, 1.0, seed=b * 4 + bt), tb, 0.13, pan=-0.12)
                else:
                    mix.place('drums', self.brush(1.0, b + bt), tb, 0.16, pan=-0.12)
            mix.place('drums', self.cymbal(0.85 if bt % 2 == 0 else 1.0, seed=bt % 2), tb, 0.055, pan=0.32)
            if bt in (1, 3):                                     # the swung skip after 2 and 4
                mix.place('drums', self.cymbal(0.6, seed=2), tb + SWING * T.BEAT, 0.040, pan=0.32)
            if big and b % 2 == 1 and bt in (1, 3):
                mix.place('drums', self.woodblock(1.0, hi=bt == 1), tb + SWING * T.BEAT, 0.07, pan=-0.3)
        if b >= T.LOOP_START and (b - T.LOOP_START) % 4 == 3 and not light:   # a press roll into every phrase
            mix.place('drums', self.press(T.BEAT * 0.95, 1.0, seed=500 + b, v0=0.35), t0 + 3 * T.BEAT, 0.16, pan=-0.12)

    def boom(self, mix, t, g):
        """a big drum hit (a slam landing)"""
        mix.place('drums', self.kick(1.0), t, g)
        mix.place('tuba', self.tuba(self.root(self.T.chord_at(t)) - 12 if self.root(self.T.chord_at(t)) - 12 >= 33
                                    else self.root(self.T.chord_at(t)), 0.5, 1.0), t, 0.4 * g)

    def clash(self, mix, t):
        mix.place('fx', self.woodblock(1.0, hi=False), t, 0.05)

    def hit(self, mix, t, ch, g=1.0, ring=0.9):
        """a full band hit: tuba root an octave down where it can go, the guitar's strum ringing, kick + choke"""
        A = self.A
        r = self.root(ch)
        mix.place('tuba', self.tuba(r - 12 if r - 12 >= 33 else r, ring, 1.0), t, 0.60 * g)
        mix.place('tuba', self.tuba(r, ring, 0.9), t, 0.30 * g)
        mix.place('guitar', self.strum_ring(ch, ring), t, 0.55 * g, pan=0.18)
        mix.place('drums', self.kick(1.0), t, 0.62 * g)
        mix.place('drums', self.choke(1.0, int(t * 10)), t, 0.16 * g)

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        """a full band hit under each word of the VS card"""
        A, ch = self.A, self.T.CHORDS[0]
        for j, tw_ in enumerate(times):
            self.hit(mix, tw_, ch, 1.0, 0.45)
            mix.place('drums', A.crash(1.0), tw_, 0.07)
            mix.place('fx', A.thud(0.8, 120, 34, seed=int(tw_ * 50) + 3), tw_, 0.30)

    def vs(self, mix, t):
        """VS: a thunder crack where it splits the divider"""
        mix.place('fx', self.M.thunder(1.4, seed=91), t, 0.40)

    def title_rest(self, mix):
        """the held chord under the card (never quiet): the guitar's tremolo, a long tuba root, the cymbal swelling,
        a press roll on beat 4 as the doors open"""
        T, A = self.T, self.A
        ch = T.CHORDS[0]
        mix.place('guitar', self.tremolo(ch, 3 * T.BEAT - 0.1), 0.08, 0.36, pan=0.18)
        mix.place('tuba', self.tuba(self.root(ch), 2.9 * T.BEAT, 0.8), 0.05, 0.40)
        n = int(3 * T.BEAT * self.SR)
        sw = A.hp(np.random.default_rng(5).normal(0, 1, n), 5000) * np.linspace(0.2, 1.0, n) ** 2
        mix.place('drums', sw, 0.0, 0.035)
        mix.place('drums', self.press(T.BEAT * 0.95, 1.0, seed=7, v0=0.3), 3 * T.BEAT, 0.20, pan=-0.12)

    def count_in(self, mix):
        """bar 1: the band keeps the two-beat on the count-in chord (the groove from the card on), a band stab on each
        count, a press roll and a tuba walk-up into FIGHT!"""
        T, A = self.T, self.A
        b = 1
        self.bar(mix, b, intensity=1.4)                          # the full groove (press rolls, ghosts, the pickup)
        ch = T.CHORDS[b]
        for j, (tc, txt) in enumerate(T.COUNT[:3]):              # a band hit on each count
            self.hit(mix, tc, ch, 0.55, 0.30)
        t4 = T.BAR + 3 * T.BEAT
        mix.place('drums', self.press(T.BEAT * 0.98, 1.0, seed=9, v0=0.4), t4, 0.26, pan=-0.12)
        mix.place('tuba', self.tuba(self.root(T.CHORDS[2]) - 2, 0.3 * T.BEAT, 0.85), t4, 0.40)

    def fight_start(self, mix):
        mix.place('drums', self.A.crash(1.8), self.T.T_FIGHT, 0.12)        # the band comes in
        mix.place('fx', self.A.thud(0.7, 110, 34), self.T.T_FIGHT, 0.34)

    def fight(self, mix, bar_ko):
        """the two-beat from FIGHT! to the K.O. (denser as it goes), a long press roll + riser into the back half,
        crashes on the phrase lines, a tape stop at the K.O."""
        T, A = self.T, self.A
        for b in range(2, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
        for b in range(2, bar_ko + 1, 8):
            mix.place('drums', A.crash(1.6), b * T.BAR, 0.08)
        t_half = (T.LOOP_START + 2 * len(T.LOOP)) * T.BAR
        mix.place('drums', self.press(2 * T.BEAT, 1.0, seed=33, v0=0.3), t_half - 2 * T.BEAT, 0.18, pan=-0.12)
        mix.place('fx', self.M.riser(T.BAR, over=0.0, seed=12) * 0.6, t_half - T.BAR, 0.08)
        mix.place('drums', A.crash(2.0), t_half, 0.12)
        for k in ('drums', 'tuba', 'guitar'):
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], self.M.T_KO, 0.42)

    def ko(self, mix):
        mix.place('drums', self.A.crash(2.4), self.M.T_KO, 0.16)
        mix.place('fx', self.A.thud(0.9, 120, 30), self.M.T_KO, 0.6)

    def stinger(self, mix, tw):
        self.hit(mix, tw, self.T.CHORDS[0], 1.0, 0.8)
        mix.place('drums', self.A.crash(1.8), tw, 0.10)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        mix.place('fx', self.A.whoosh(max(0.2, t_solo - tw - 0.1), 300, 5000), tw + 0.1, 0.08)

    def victory(self, mix, b_solo):
        """the band plays on under the solo at full tilt (a victory, never a quiet stretch), up to the build"""
        T = self.T
        for b in range(b_solo, T.N_BARS - 1):
            self.bar(mix, b, intensity=1.4)

    def last_bars(self, mix):
        """the last bar builds into the title slam on sample 0: a press roll crescendo right up to the seam, the tuba
        walking up chromatically into the tonic, the guitar chopping every beat, the bass drum on every beat, a riser"""
        T, M, A = self.T, self.M, self.A
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.CHORDS[-1]
        mix.place('drums', self.press(T.DUR - t_last + 0.012, 1.0, seed=99, v0=0.35), t_last, 0.22, pan=-0.12)
        tgt = self.root(T.CHORDS[0])
        for bt in range(4):
            tb = t_last + bt * T.BEAT
            mix.place('tuba', self.tuba(tgt - 4 + bt, 0.6 * T.BEAT, 0.9), tb, 0.58)
            mix.place('guitar', self.chop(ch, 1.0), tb, 0.44, pan=0.18)
            mix.place('drums', self.kick(0.95), tb, 0.50)
            mix.place('drums', self.cymbal(0.9, seed=bt % 2), tb, 0.06, pan=0.32)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.18)
