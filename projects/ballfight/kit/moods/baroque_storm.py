"""Mood: Baroque storm (#37 Clarinet vs Oboe, ported as it was). A stormy presto with NO woodwinds of its own:
harpsichord continuo (repeated 8th-note bass, chords, 16th arpeggios later), a racing minor string ostinato (violins in
16ths, cellos and basses driving 8ths, violas trembling in the back half), timpani and thunder (under every captioned
ability too). It thickens with parts (violin II, violas, arpeggios, timpani on 3), never with level; after the K.O. it
holds its breath on trembling strings until the solo. Built for D minor (Dm Gm C F Bb A); other chords are voiced by
rule, and its fixed runs are moved to the episode's key."""
import math

import numpy as np

# the harpsichord's right hand and violin I's 16th cell per beat (top, mid, peak, mid), as #37 voiced them
HARP_RH = {'Dm': (62, 65, 69, 74), 'Gm': (62, 67, 70, 74), 'C': (64, 67, 72, 76), 'F': (65, 69, 72, 77),
           'Bb': (65, 70, 74, 77), 'A': (61, 64, 69, 73)}
VLN = {'Dm': (74, 69, 77, 69), 'Gm': (74, 70, 79, 70), 'C': (76, 72, 79, 72), 'F': (77, 72, 81, 72),
       'Bb': (77, 74, 82, 74), 'A': (76, 73, 81, 73)}


class Bed:
    buses = {'strings': (1.6, 0.45), 'harpsi': (1.3, 0.3), 'drums': (1.0, 0.15)}
    reverb = (2.2, 0.5, 0.22)
    comp = (0.20, 0.5)
    roar_gain, doors_gain = 0.22, 0.45
    landing = (0.08, 0.06)
    pop = (95, 102, 0.12)
    build_bars = 2

    def __init__(self, M):
        self.M, self.T, self.A, self.SR = M, M.T, M.A, M.SR
        self._bc = {}
        self.off = (self.T.KEY_PC - 2 + 6) % 12 - 6               # #37's fixed runs were in D

    # ---------------------------------------------------------------- voicings
    def root(self, ch):
        return self.T.root_above(ch, 36)                          # the continuo's roots (D2 G2 C2 F2 Bb2 A2 in D minor)

    def harp_rh(self, ch):
        return HARP_RH.get(ch) or tuple(self.T.chord_tones(61, 80, ch)[:4])

    def vln(self, ch):
        if ch in VLN:
            return VLN[ch]
        t = self.T.chord_tones(69, 84, ch)
        return (t[1], t[0], t[2], t[0])

    # ---------------------------------------------------------------- instruments (#37)
    def harpsi(self, m, dur=0.7, vel=1.0):
        """a harpsichord string: bright plucked additive (plucked near the end), the 4' octave, a jack click"""
        A, SR = self.A, self.SR
        key = ('h', m, round(dur, 2))
        if key not in self._bc:
            n = int(dur * SR)
            t = A.tt(n)
            f = A.mtof(m)
            x = np.zeros(n)
            for k in range(1, 32):
                fk = f * k
                if fk > 12000:
                    break
                a = abs(math.sin(math.pi * k * 0.13)) / k ** 0.75
                x += a * np.sin(2 * np.pi * fk * t + k) * np.exp(-t * (1.2 + 0.35 * k) * (f / 220) ** 0.35)
            f2 = 2 * f
            for k in range(1, 12):
                if f2 * k > 12000:
                    break
                x += 0.32 * abs(math.sin(math.pi * k * 0.2)) / k * np.sin(2 * np.pi * f2 * k * t) * np.exp(-t * (1.6 + 0.4 * k))
            click = A.bp(np.random.default_rng(m).normal(0, 1, n), 2500, 9000) * np.exp(-t / 0.004) * 0.6
            self._bc[key] = (x / 4 + click) * np.clip(t / 0.0015, 0, 1)
        return self._bc[key] * vel

    def strng(self, m, dur, vel=1.0, cut=2400, vib=0.0, att=0.008):
        """a string section note: 3 detuned saws (an ensemble), dark low-pass, bow attack; vibrato on long notes"""
        A, SR = self.A, self.SR
        key = ('s', m, round(dur, 3), cut, round(vib, 3), round(att, 3))
        if key not in self._bc:
            n = int((dur + 0.08) * SR)
            t = A.tt(n)
            f = A.mtof(m)
            x = np.zeros(n)
            for v, dt in enumerate((-0.006, 0.0, 0.007)):
                fr = f * (1 + dt) * (1 + vib * np.sin(2 * np.pi * (5.2 + 0.4 * v) * t + v))
                ph = np.cumsum(fr) / SR
                x += 2 * ((ph + 0.3 * v) % 1.0) - 1
            x = A.lp(A.hp(x / 3, 90), cut)
            env = np.clip(t / att, 0, 1) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.03))
            self._bc[key] = x * env
        return self._bc[key] * vel

    def timp(self, m, vel=1.0):
        A, SR = self.A, self.SR
        key = ('t', m)
        if key not in self._bc:
            n = int(1.1 * SR)
            t = A.tt(n)
            f = A.mtof(m)
            fr = f * (1 + 0.04 * np.exp(-t / 0.05))
            ph = 2 * np.pi * np.cumsum(fr) / SR
            x = (np.sin(ph) + 0.5 * np.sin(1.5 * ph) * np.exp(-t / 0.3) + 0.25 * np.sin(2 * ph) * np.exp(-t / 0.2))
            x = x * np.exp(-t / 0.55) * np.clip(t / 0.003, 0, 1)
            mallet = A.lp(np.random.default_rng(m + 5).normal(0, 1, n), 1200) * np.exp(-t / 0.012) * 0.8
            self._bc[key] = x + mallet
        return self._bc[key] * vel

    def bar(self, mix, b, dense=0.0, light=False):
        """one bar of the storm (bar index b). dense 0..1: the arrangement thickens (violin II, violas, 16th arpeggios,
        timpani on 3), never the level"""
        T, A, SR = self.T, self.A, self.SR
        t0 = b * T.BAR
        ch = T.CHORDS[b % T.N_BARS]
        root = self.root(ch)
        lo = root - 12 if root >= 43 else root                               # basses: D1-A1 region (+ the octave)
        for e8 in range(8):                                                  # continuo + cellos / basses: driving 8ths
            te = t0 + e8 * T.BEAT / 2
            mix.place('harpsi', self.harpsi(lo + (12 if e8 % 2 else 0), 0.5), te, 0.16, pan=-0.15)
            if not light:
                mix.place('strings', self.strng(lo, 0.17, 1.0, 900), te, 0.10, pan=0.1)
                mix.place('strings', self.strng(lo + 12, 0.17, 0.8, 1200), te, 0.08, pan=0.1)
        for bt in (0, 2):                                                    # the harpsichord's right hand: chords
            for j, m in enumerate(self.harp_rh(ch)):
                mix.place('harpsi', self.harpsi(m + 12, 0.6), t0 + bt * T.BEAT + j * 0.008, 0.07, pan=0.2)
        cell = self.vln(ch)
        for st in range(16):                                                 # violin I: the racing 16ths
            if light and st % 2:
                continue
            m = cell[st % 4] - (12 if st // 4 in (1, 3) and st % 4 == 2 else 0)
            acc = 1.0 if st % 4 == 0 else 0.72
            mix.place('strings', self.strng(m, 0.075, acc, 2600), t0 + st * T.S16, 0.075, pan=-0.3)
            if dense > 0.35 and not light:                                   # violin II: a third / sixth under
                m2 = max([x for x in T.chord_tones(m - 9, m - 3, ch)] or [m - 12])
                mix.place('strings', self.strng(m2, 0.075, acc * 0.9, 2300), t0 + st * T.S16, 0.055, pan=0.35)
        if dense > 0.6 and not light:                                        # violas: a trembling chord (32nds)
            tones = T.chord_tones(57, 69, ch)[:2]
            for s32 in range(32):
                for m in tones:
                    mix.place('strings', self.strng(m, 0.045, 0.6 + 0.4 * (s32 % 2 == 0), 1800), t0 + s32 * T.S16 / 2, 0.028)
        if dense > 0.5 and not light:                                        # 16th arpeggios on the harpsichord
            arp = self.harp_rh(ch)
            for st in range(16):
                mix.place('harpsi', self.harpsi(arp[st % 4] + 12 * (st // 8 % 2), 0.3), t0 + st * T.S16, 0.045, pan=0.3)
        mix.place('drums', self.timp(root - 12 if root > 40 else root), t0, 0.40 if not light else 0.3)
        if dense > 0.45 or light:
            mix.place('drums', self.timp(root - 5 if root - 5 >= 33 else root + 7), t0 + 2 * T.BEAT, 0.30)
        mix.place('drums', A.hp(A.kick(1.0, 0.14, 150, 50)[:int(0.25 * SR)], 45), t0, 0.22)           # modern punch under
        mix.place('drums', A.hp(A.kick(1.0, 0.14, 150, 50)[:int(0.25 * SR)], 45), t0 + 2 * T.BEAT, 0.18)
        if not light:
            for bt in (1, 3):                                                # a snare-ish string chop on 2 and 4
                mix.place('strings', A.hp(self.strng(T.chord_tones(62, 74, ch)[0], 0.06, 1.0, 3500), 200), t0 + bt * T.BEAT, 0.07)

    def tutti(self, mix, t, ch, g=1.0, dur=0.25):
        """a storm hit: strings + harpsichord + timpani + a low thunder thud on the chord"""
        A, T = self.A, self.T
        for m in T.chord_tones(50, 77, ch):
            mix.place('strings', self.strng(m, dur, 1.0, 3000, att=0.004), t, 0.06 * g)
        for m in self.harp_rh(ch):
            mix.place('harpsi', self.harpsi(m, 0.8), t, 0.08 * g)
            mix.place('harpsi', self.harpsi(m + 12, 0.8), t, 0.05 * g)
        root = self.root(ch)
        mix.place('drums', self.timp(root - 12 if root > 40 else root), t, 0.6 * g)
        mix.place('fx', A.thud(0.8, 110, 32, seed=int(t * 50) + 3), t, 0.35 * g)
        mix.place('drums', A.crash(1.2), t, 0.05 * g)

    def boom(self, mix, t, g):
        mix.place('drums', self.timp(self.root(self.T.CHORDS[0])), t, g)

    def clash(self, mix, t):
        mix.place('fx', self.A.thud(0.15, 200, 90, seed=int(t * 3)), t, 0.06)

    # ---------------------------------------------------------------- sections
    def title(self, mix, times):
        """the storm at full level on the card, a tutti under each word"""
        tonic = self.T.CHORDS[0]
        self.bar(mix, 0, dense=0.7)
        for tw_, g in zip(times, (0.8, 0.75, 0.75)):
            self.tutti(mix, tw_, tonic, g)

    def vs(self, mix, t):
        mix.place('fx', self.M.thunder(1.4, 91), t, 0.55)            # the thunder crack where it splits the divider

    def title_rest(self, mix):
        T = self.T
        for st in range(12, 16):                                     # beat 4: a string run up as the doors open
            mix.place('strings', self.strng((69, 72, 74, 77)[st - 12] + self.off, 0.08, 1.0, 3000), st * T.S16, 0.12)

    def count_in(self, mix):
        T = self.T
        self.bar(mix, 1, dense=0.4)
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            self.tutti(mix, tc, T.CHORDS[1], 0.8)
        for st in range(12, 16):                                     # the run into FIGHT!
            mix.place('strings', self.strng((69, 73, 76, 81)[st - 12] + self.off, 0.08, 1.0, 3200), T.BAR + st * T.S16, 0.12)

    def fight_start(self, mix):
        self.tutti(mix, self.T.T_FIGHT, self.T.CHORDS[self.T.LOOP_START], 1.1)     # the drop
        mix.place('fx', self.M.thunder(1.8, 7), self.T.T_FIGHT, 0.40)

    def fight(self, mix, bar_ko):
        T, M = self.T, self.M
        for b in range(2, bar_ko + 1):
            self.bar(mix, b, dense=min(1.0, (b - 2) / 16.0))
        for t_, who, name in M.P.STORY['caps']:                      # thunder under every captioned ability
            mix.place('fx', M.thunder(1.6, int(t_ * 10), crack=False), M.q16(t_), 0.35)
        for k in ('strings', 'harpsi', 'drums'):
            if k in mix.buses:
                mix.buses[k] = self.A.tape_stop(mix.buses[k], M.T_KO, 0.42)

    def ko(self, mix):
        self.tutti(mix, self.M.T_KO, self.T.CHORDS[0], 1.0, 0.5)
        mix.place('fx', self.M.thunder(2.4, 51), self.M.T_KO, 0.55)

    def stinger(self, mix, tw):
        self.tutti(mix, tw, self.T.CHORDS[0], 1.0, 0.6)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        """the storm holds its breath but never goes quiet: trembling strings + timpani until the solo"""
        T = self.T
        tonic = T.CHORDS[0]
        r = self.root(tonic)
        for b in range(bar_ko + 1, int(round(t_solo / T.BAR))):
            tq = b * T.BAR
            for m in T.chord_tones(50, 69, tonic):
                for s32 in range(32):
                    mix.place('strings', self.strng(m, 0.045, 0.8, 2200), tq + s32 * T.S16 / 2, 0.035)
            mix.place('drums', self.timp(r), tq, 0.4)
            mix.place('drums', self.timp(r + 7), tq + 2 * T.BEAT, 0.3)

    def victory(self, mix, b_solo):
        T = self.T
        for b in range(b_solo, T.N_BARS - self.build_bars):
            self.bar(mix, b, dense=0.5, light=True)
            for m in T.chord_tones(50, 69, T.CHORDS[b]):             # a held string chord under the solo
                mix.place('strings', self.strng(m, T.BAR, 0.7, 1800, vib=0.004, att=0.08), b * T.BAR, 0.08)

    def last_bars(self, mix):
        """the storm returns for the last bars and builds into the title slam: timpani 8ths, a roll into the seam"""
        T, M = self.T, self.M
        for b in range(T.N_BARS - self.build_bars, T.N_BARS):
            self.bar(mix, b, dense=1.0)
        t_last = (T.N_BARS - 1) * T.BAR
        dom = self.root(T.CHORDS[-1])
        for bt in range(4):
            mix.place('drums', self.timp(dom), t_last + bt * T.BEAT, 0.36)
            mix.place('drums', self.timp(dom), t_last + (bt + 0.5) * T.BEAT, 0.24)
        st = 0.0
        while t_last + 2 * T.BEAT + st < T.DUR - 1e-4:               # a timpani roll into the seam
            mix.place('drums', self.timp(dom, 0.6), t_last + 2 * T.BEAT + st, 0.20)
            st += T.S16
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.20)
