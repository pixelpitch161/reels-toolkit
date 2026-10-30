"""Mood: dark drumline (#38 Bass Clarinet vs Euphonium, ported for #53 Flute vs Sousaphone). A minor marching-band
battle cadence with NO low brass or woodwinds of its own (the fighters play the band's bass line and melody):

    drums   the snare line (accents, taps, ghosts, flams, diddle rolls on a 16-step pattern: A / B bars and a fill bar
            ending each 4-bar phrase), quads / tenors tuned high (offbeat answers, sweeps later), five pitched marching
            bass drums on the chord tones (unison hits and split runs), the cymbal line (a crash on each phrase, choked
            crunches on 2 and 4 later)
    pit     the front ensemble: a high marimba ostinato on the chord's tones (8ths) and timpani on the roots

It thickens with the bars (taps -> flams and rolls, tenor sweeps, bass-drum splits, cymbal crunches), never with level
(consistent loudness from frame 0). Every captioned ability gets a unison hit; the VS card is three unison hits with a
snare roll as the doors open; the count-in is the light cadence with a unison hit on each count and a roll into the
drop; after the K.O. the cadence holds its breath (lighter), plays on under the solo, and the last two bars build into
the title slam (bass drums on every beat, a roll and a riser into the seam).

Port check (#53): cadence() / unison() / roll() here against #38's own (projects/ballfight_bassclarinet_vs_euphonium/
audio.py) on #38's C minor chords: every bar 0-31 at #38's densities, light bars and unison hits, rendered into two
mixes - the drums and pit buses are bit-identical. #38's bass-drum tunings (BD_ROOTS) are kept for its four chords;
other chords get the same shape by rule (the four highest chord tones up to E-flat 4, then the root in G2..F#3)."""
import numpy as np

# the cadence (16 16ths per bar). Snare: A accent, t tap, g ghost, f flam, r diddle roll, . rest (#38, unchanged)
SN = {'a': 'A.tgA.ttA.tgArrt', 'b': 'f.t.A.tgf.tgA.rr', 'fill': 'A.A.AttAttArrrrr'}
SN_LIGHT = {'a': 'A.g.A.g.A.g.A.g.', 'b': 'A.g.A.g.A.g.Agrg', 'fill': 'A.g.A.g.A.gtArrr'}
TEN = {'lo': '..1...2...1...3.', 'hi': '..1.2.3.1.2.3434'}     # quads: drum numbers (1 high .. 4 low)
BASS = {'lo': 'U.......U.......', 'hi': 'U...1.2.U..34.5.'}     # bass drums: U unison, 1..5 splits (high .. low)
BD_38 = {'Cm': (60, 55, 51, 48, 43), 'Ab': (63, 60, 56, 51, 44), 'Fm': (60, 56, 53, 48, 41), 'G': (62, 59, 55, 50, 43)}


class Bed:
    buses = {'drums': (1.0, 0.18), 'pit': (0.9, 0.30)}             # bus: (gain, reverb send) as #38's master
    reverb = (2.0, 0.5, 0.22)                                      # #38's: (dur, decay, wet)
    comp = (0.20, 0.5)                                             # #38's master compressor: threshold, exponent
    roar_gain, doors_gain = 0.22, 0.60
    landing = (0.08, 0.06)
    pop = (95, 102, 0.12)

    def __init__(self, M):
        self.M, self.T, self.A, self.SR = M, M.T, M.A, M.SR
        self._c = {}

    def _cache(self, key, fn):
        if key not in self._c:
            self._c[key] = fn()
        return self._c[key]

    # ---------------------------------------------------------------- harmony
    def bd_notes(self, ch):
        if ch in BD_38:
            return BD_38[ch]
        T = self.T
        tones = sorted(T.chord_tones(48, 63, ch), reverse=True)[:4]
        root = T.root_above(ch, 43)
        return tuple(tones) + (root,)

    def troot(self, ch):
        return self.T.root_above(ch, 36)                           # the timpani's root, C2..B2 (#38's CHORD_ROOT)

    # ---------------------------------------------------------------- instruments (#38's, unchanged)
    def msnare(self, kind='A'):
        """a marching snare: high-tension crack, tight wires, a little ring; kind A accent / t tap / g ghost"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.28 * SR)
            t = A.tt(n)
            r = np.random.default_rng(5 + ord(kind))
            wires = A.bp(r.normal(0, 1, n), 1800, 9500) * np.exp(-t / (0.075 if kind == 'A' else 0.05))
            crack = A.bp(r.normal(0, 1, n), 3000, 11000) * np.exp(-t / 0.004) * (1.6 if kind == 'A' else 0.8)
            body = (0.5 * np.sin(2 * np.pi * 380 * t) + 0.6 * np.sin(2 * np.pi * 620 * t)) * np.exp(-t / 0.03)
            x = (0.8 * wires + crack + 0.7 * body) * np.clip(t / 0.0008, 0, 1)
            return x * {'A': 1.0, 't': 0.42, 'g': 0.20, 'f': 1.0}[kind]
        return self._cache(('sn', kind), make)

    def tenor(self, i):
        """quads: four pitched tenor drums (high to low), tuned high so a low fighter owns the low mids"""
        A, SR = self.A, self.SR

        def make():
            f = (494.0, 415.0, 349.0, 294.0)[i]
            n = int(0.32 * SR)
            t = A.tt(n)
            fr = f * (1 + 0.12 * np.exp(-t / 0.02))
            body = np.sin(2 * np.pi * np.cumsum(fr) / SR) * np.exp(-t / 0.13)
            stick = A.bp(np.random.default_rng(31 + i).normal(0, 1, n), 1500, 7000) * np.exp(-t / 0.004)
            return A.hp(0.7 * body + 0.8 * stick, 260)
        return self._cache(('tn', i), make)

    def bassdrum(self, m, vel=1.0):
        """a pitched marching bass drum (felt mallet): a low tuned body (two octaves under the label) + a soft beater"""
        A, SR = self.A, self.SR

        def make():
            f = A.mtof(m - 24)
            n = int(0.45 * SR)
            t = A.tt(n)
            fr = f * (1 + 0.25 * np.exp(-t / 0.015))
            body = np.sin(2 * np.pi * np.cumsum(fr) / SR) * np.exp(-t / 0.22)
            beat = A.lp(np.random.default_rng(int(m)).normal(0, 1, n), 500) * np.exp(-t / 0.008)
            return (body + 0.35 * beat) * np.clip(t / 0.002, 0, 1)
        return self._cache(('bd', int(m)), make) * vel

    def choke(self, seed=0):
        A, SR = self.A, self.SR
        n = int(0.22 * SR)
        t = A.tt(n)
        x = A.bp(np.random.default_rng(40 + seed).normal(0, 1, n), 3000, 12000)
        return x * np.exp(-t / 0.045) * np.clip(t / 0.002, 0, 1)

    def marimba(self, m, vel=1.0):
        A, SR = self.A, self.SR

        def make():
            n = int(0.7 * SR)
            t = A.tt(n)
            f = A.mtof(m)
            x = (np.sin(2 * np.pi * f * t) * np.exp(-t / 0.35) + 0.25 * np.sin(2 * np.pi * 3.93 * f * t) *
                 np.exp(-t / 0.06) + 0.08 * np.sin(2 * np.pi * 9.2 * f * t) * np.exp(-t / 0.02))
            click = A.bp(np.random.default_rng(m).normal(0, 1, n), 1000, 4000) * np.exp(-t / 0.003) * 0.3
            return (x + click) * np.clip(t / 0.001, 0, 1)
        return self._cache(('ma', m), make) * vel

    def timp(self, m, vel=1.0):
        A, SR = self.A, self.SR

        def make():
            n = int(1.1 * SR)
            t = A.tt(n)
            f = A.mtof(m)
            fr = f * (1 + 0.04 * np.exp(-t / 0.05))
            ph = 2 * np.pi * np.cumsum(fr) / SR
            x = (np.sin(ph) + 0.3 * np.sin(1.5 * ph) * np.exp(-t / 0.2) + 0.12 * np.sin(2 * ph) * np.exp(-t / 0.15))
            x = x * np.exp(-t / 0.55) * np.clip(t / 0.003, 0, 1)
            mallet = A.lp(np.random.default_rng(m + 5).normal(0, 1, n), 700) * np.exp(-t / 0.012) * 0.5
            return x + mallet
        return self._cache(('t', m), make) * vel

    # ---------------------------------------------------------------- one bar, a unison hit, a roll (#38's)
    def cadence(self, mix, b, dense=0.5, light=False):
        """one bar of the drumline + pit (bar index b). dense 0..1 thickens the parts (never the level)"""
        T, A = self.T, self.A
        S16 = T.S16
        t0 = b * T.BAR
        ch = T.CHORDS[b % T.N_BARS]
        phrase = (b - 2) % 4 if b >= 2 else b
        pk = 'fill' if phrase == 3 else ('a' if phrase % 2 == 0 else 'b')
        sn = (SN_LIGHT if (light or dense < 0.3) else SN)[pk]
        for i, c in enumerate(sn):
            ts = t0 + i * S16
            if c == '.':
                continue
            if c == 'r':                                               # diddle: two 32nds
                for j in range(2):
                    mix.place('drums', self.msnare('t'), ts + j * S16 / 2, 0.30 * (1.0 if j == 0 else 0.85), pan=0.05)
            elif c == 'f':                                             # flam: grace 18 ms early
                mix.place('drums', self.msnare('g'), ts - 0.018, 0.30, pan=0.05)
                mix.place('drums', self.msnare('A'), ts, 0.30, pan=0.05)
            else:
                mix.place('drums', self.msnare(c), ts, 0.30, pan=0.05)
        if not light:
            ten = TEN['hi' if dense > 0.55 else 'lo']
            for i, c in enumerate(ten):
                if c != '.':
                    mix.place('drums', self.tenor(int(c) - 1), t0 + i * S16, 0.10, pan=-0.25 + 0.15 * int(c))
        bd = self.bd_notes(ch)
        bp = BASS['hi' if dense > 0.4 and not light else 'lo']
        for i, c in enumerate(bp):
            ts = t0 + i * S16
            if c == 'U':
                for m in bd:
                    mix.place('drums', self.bassdrum(m), ts, 0.10)
            elif c != '.':
                mix.place('drums', self.bassdrum(bd[int(c) - 1]), ts, 0.22, pan=-0.3 + 0.15 * int(c))
        if phrase == 0:
            mix.place('drums', A.crash(1.4), t0, 0.07 if not light else 0.05)
        if dense > 0.65 and not light:                                 # cymbal crunches on 2 and 4
            for bt in (1, 3):
                mix.place('drums', self.choke(b * 4 + bt), t0 + bt * T.BEAT, 0.07)
        tones = T.chord_tones(72, 91, ch)                              # the pit: marimba 8ths + timpani roots
        for e8 in range(8):
            m = tones[(e8 * 2 + (e8 // 4)) % len(tones)]
            mix.place('pit', self.marimba(m), t0 + e8 * T.BEAT / 2, 0.06 if e8 % 2 == 0 else 0.045, pan=0.3)
        root = self.troot(ch)
        mix.place('pit', self.timp(root), t0, 0.30)
        if dense > 0.5 and not light:
            mix.place('pit', self.timp(root + 7 if root + 7 <= 50 else root - 5), t0 + 2 * T.BEAT, 0.22)

    def unison(self, mix, t, ch, g=1.0):
        """a drumline unison hit: all bass drums + accent snare + the quads + crash + timpani + a low thud"""
        A = self.A
        for m in self.bd_notes(ch):
            mix.place('drums', self.bassdrum(m), t, 0.14 * g)
        mix.place('drums', self.msnare('A'), t, 0.34 * g)
        for i in range(4):
            mix.place('drums', self.tenor(i), t, 0.08 * g)
        mix.place('drums', A.crash(1.4), t, 0.08 * g)
        mix.place('pit', self.timp(self.troot(ch)), t, 0.5 * g)
        mix.place('fx', A.thud(0.7, 110, 34, seed=int(t * 50) + 3), t, 0.30 * g)

    def roll(self, mix, t0, t1, g0=0.12, g1=0.32):
        """a snare diddle roll on 32nds from t0 to t1, crescendo"""
        S16 = self.T.S16
        n = max(1, int((t1 - t0) / (S16 / 2)))
        for j in range(n):
            u = j / max(1, n - 1)
            mix.place('drums', self.msnare('t'), t0 + j * S16 / 2, g0 + (g1 - g0) * u, pan=0.05)

    def press_roll(self, mix, t0, t1, g0=0.12, g1=0.32):
        """a fighter's fanfare roll (the kit's name for it)"""
        self.roll(mix, t0, t1, g0, g1)

    def boom(self, mix, t, g):
        self.unison(mix, t, self.T.chord_at(t), 0.8 * g)

    def clash(self, mix, t):
        mix.place('fx', self.A.thud(0.15, 200, 90, seed=int(t * 3)), t, 0.06)     # #38's ball clash

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        """bar 0: the cadence at full level and a unison hit under each word of the VS card"""
        self.cadence(mix, 0, dense=0.5)
        for g, tw_ in zip((0.75, 0.7, 0.72), times):
            self.unison(mix, tw_, self.T.CHORDS[0], g)

    def vs(self, mix, t):
        mix.place('fx', self.M.thunder(1.4, seed=91), t, 0.55)

    def title_rest(self, mix):
        """beat 4: a snare roll as the doors open, a whoosh"""
        T = self.T
        self.roll(mix, 3 * T.BEAT, T.BAR - 0.02, 0.10, 0.30)
        mix.place('fx', self.A.whoosh(T.T_DOORS[1] - T.T_DOORS[0] + 0.1, 300, 5000), T.T_DOORS[0] - 0.05, 0.14)

    def count_in(self, mix):
        """bar 1: the snare line's light cadence on the count chord, a unison hit on each count, a roll into FIGHT!"""
        T = self.T
        self.cadence(mix, 1, dense=0.3, light=True)
        for tc, txt in T.COUNT[:3]:
            self.unison(mix, tc, T.CHORDS[1], 0.75)
        self.roll(mix, T.BAR + 3 * T.BEAT, T.T_FIGHT - 0.02, 0.12, 0.34)
        mix.place('fx', self.A.whoosh(T.BEAT, 400, 6000), T.BAR + 3 * T.BEAT, 0.12)

    def fight_start(self, mix):
        T = self.T
        self.unison(mix, T.T_FIGHT, T.CHORDS[2], 1.1)                  # the drop
        mix.place('fx', self.M.thunder(1.8, seed=7), T.T_FIGHT, 0.36)

    def fight(self, mix, bar_ko):
        """the cadence from FIGHT! to the K.O., thickening; a unison hit under every captioned ability; a tape stop"""
        T, A, M = self.T, self.A, self.M
        for b in range(2, bar_ko + 1):
            self.cadence(mix, b, dense=min(1.0, 0.45 + (b - 2) / 24.0))
        for t_, who, name in M.P.STORY['caps']:
            self.unison(mix, M.q16(t_), T.chord_at(t_), 0.6)
        for k in ('drums', 'pit'):
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], M.T_KO, 0.42)

    def ko(self, mix):
        M = self.M
        self.unison(mix, M.T_KO, self.T.chord_at(M.T_KO), 1.0)
        mix.place('fx', M.thunder(2.4, seed=51), M.T_KO, 0.50)

    def stinger(self, mix, tw):
        self.unison(mix, tw, self.T.chord_at(tw), 0.9)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        """the drumline holds its breath (never quiet) until the solo"""
        for b in range(bar_ko + 1, int(round(t_solo / self.T.BAR))):
            self.cadence(mix, b, dense=0.6)

    def victory(self, mix, b_solo):
        for b in range(b_solo, self.T.N_BARS - 2):
            self.cadence(mix, b, dense=0.85)

    def last_bars(self, mix):
        """the last two bars build into the title slam: the full cadence, the bass drums on every beat of the last
        bar on its chord, a snare roll and a riser into the seam"""
        T, M = self.T, self.M
        for b in (T.N_BARS - 2, T.N_BARS - 1):
            self.cadence(mix, b, dense=1.0)
        tl = (T.N_BARS - 1) * T.BAR
        for bt in range(4):
            for m in self.bd_notes(T.CHORDS[-1]):
                mix.place('drums', self.bassdrum(m), tl + bt * T.BEAT, 0.09)
        self.roll(mix, tl + 2 * T.BEAT, T.DUR - 1e-3, 0.16, 0.48)
        mix.place('fx', M.riser(T.DUR - tl), tl, 0.30)
