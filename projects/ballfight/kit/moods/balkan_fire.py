"""Mood: dark Balkan fire (#28 Drums vs Violin, ported as it was). A DRUM-LESS bed (#28's drums were a fighter):
an upright-bass oom-pah (root on 1, fifth on 3, a chromatic pickup into the next bar later in the fight), musette
accordion stabs on the "pah" (2 and 4; 8th-note pah-pahs when the fire builds), a bellows pad, and a cimbalom shimmer
of 16th arpeggios building through the back half. Fiery, not happy. Built for D harmonic minor (Dm Dm Gm A7); any chords
work: the bass takes each chord's root in A1-G#2 and its fifth nearest, the accordion the chord from its root in
G3-F#4, the cimbalom its tones from C#5.

#28 had no VS title card (it predates it): the card's sections here are new (the oom-pah under the card, a unison
bass + accordion hit per word, a thunder crack for VS). Its count-in is the bass + accordion on the count chord; the
drummer's sticks and pickup are the DRUMS fighter's own."""
import numpy as np


class Bed:
    buses = {'bass': (0.8, 0.0), 'accordion': (2.0, 0.4), 'pad': (1.2, 0.5), 'cimbalom': (1.4, 0.6),
             'solo': (2.0, 0.5), 'fx': (1.5, 0.4)}                    # bus: (gain, reverb send) (#28)
    reverb = (2.4, 0.6, 0.20)
    comp = (0.22, 0.35)                           # #28's gentler compressor (it had section automation, now dropped)
    roar_gain, doors_gain = 0.16, 0.36
    count_whoosh_gain = 0.14
    landing = (0.08, 0.06)
    pop = (95, 102, 0.12)

    def __init__(self, M):
        self.M, self.T, self.A = M, M.T, M.A
        self.SR = M.SR

    # ---------------------------------------------------------------- voicings (#28's tables are what these rules give)
    def bass_pair(self, ch):
        """(root on 1, fifth on 3): the root in A1-G#2, its fifth nearest (#28: Dm 38 33, Gm 43 38, A 33 40)"""
        from ballfight.kit.episode import parse_chord
        r = parse_chord(ch)[0]
        return 33 + (r - 9) % 12, 33 + (r + 7 - 9) % 12

    def voicing(self, ch):
        """the accordion's chord from its root in G3-F#4 (#28: Dm 62 65 69, Gm 55 58 62, A7 57 61 64 67)"""
        from ballfight.kit.episode import parse_chord
        r = 55 + (parse_chord(ch)[0] - 7) % 12
        return tuple(self.T.chord_tones(r, r + 11, ch))

    def cimb(self, ch):
        """the cimbalom's tones from C#5 (#28: Dm 74 77 81 86, Gm 74 79 82 86, A7 73 76 79 81)"""
        return tuple(self.T.chord_tones(73, 86, ch)[:4])

    # ---------------------------------------------------------------- instruments (#28)
    def upright(self, m, dur=0.5, vel=1.0):
        """upright bass pluck: round fundamental, woody thump, quick decay"""
        A, SR = self.A, self.SR
        n = int((dur + 0.1) * SR)
        f = A.mtof(m)
        t = A.tt(n)
        x = np.sin(2 * np.pi * f * t) + 0.35 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.12) + 0.12 * np.sin(6 * np.pi * f * t) * np.exp(-t / 0.06)
        x *= np.exp(-t / max(0.18, dur * 0.7)) * np.clip(t / 0.003, 0, 1)
        x += 0.25 * A.bp(np.random.default_rng(int(m)).normal(0, 1, n), 300, 1600) * np.exp(-t / 0.008)
        fd = int(0.05 * SR)
        x[-fd:] *= np.linspace(1, 0, fd)
        return A.lp(np.tanh(1.6 * x) / np.tanh(1.6), 1500) * vel

    def reed(self, ms, dur, att=0.01, rel=0.05, cut=2600, musette=0.006):
        """accordion reeds: pulse + saw per note, three reeds detuned (musette), a little bellows tremolo"""
        A = self.A
        n = int(dur * self.SR)
        t = A.tt(n)
        x = np.zeros(n)
        for m in ms:
            f = A.mtof(m)
            for d in (-musette, 0.0, musette):
                ph = (f * (1 + d) * t) % 1.0
                x += 0.6 * np.where(ph < 0.32, 1.0, -0.45) + 0.4 * (2 * ph - 1)
        x /= 3 * max(1, len(ms))
        x *= 1 + 0.06 * np.sin(2 * np.pi * 5.3 * t)
        return A.lp(x, cut) * A.adsr(n, att, 0.08, 0.8, min(rel, dur * 0.4))

    def cimbalom(self, m, vel=1.0):
        """hammered strings: bright attack, two slightly detuned strings, ringing"""
        A = self.A
        n = int(0.7 * self.SR)
        f = A.mtof(m)
        t = A.tt(n)
        x = (np.sin(2 * np.pi * f * t) + np.sin(2 * np.pi * f * 1.004 * t)) * 0.5
        x += 0.4 * np.sin(2 * np.pi * 2 * f * t) * np.exp(-t / 0.08) + 0.2 * np.sin(2 * np.pi * 3.01 * f * t) * np.exp(-t / 0.05)
        return x * np.exp(-t / 0.22) * vel

    def bar(self, mix, b, intensity=1.0, light=False):
        """one bar of the drum-less bed (#28's bed()): oom-pah bass + accordion, the pad, the cimbalom shimmer"""
        import math
        T = self.T
        S16 = T.S16
        t0 = b * T.BAR
        ch = T.CHORDS[b]
        nxt = T.CHORDS[(b + 1) % T.N_BARS]
        r, f5 = self.bass_pair(ch)
        mix.place('bass', self.upright(r, 0.42), t0, 0.62)
        mix.place('bass', self.upright(f5, 0.36, 0.9), t0 + 2 * T.BEAT, 0.52)
        if intensity > 1.25 and not light:
            tgt = self.bass_pair(nxt)[0]
            mix.place('bass', self.upright(tgt - 1, 0.18, 0.7), t0 + 3.5 * T.BEAT, 0.40)
        vo = self.voicing(ch)
        for bt in (1, 3):
            mix.place('accordion', self.reed(vo, 0.20), t0 + bt * T.BEAT, 0.20 if light else 0.24)
            if intensity > 1.35 and not light:
                mix.place('accordion', self.reed(vo, 0.12), t0 + (bt + 0.5) * T.BEAT, 0.13)
        if b >= T.FIGHT_BAR + 2 or light:                              # the bellows pad (from 2 bars into the fight)
            mix.place('pad', self.reed([vo[0] - 12, vo[0], vo[2]], T.BAR, att=0.25, rel=0.3, cut=1400), t0, 0.05)
        if intensity > 1.2 and not light:                              # cimbalom shimmer: 16th arpeggios
            k = min(1.0, (intensity - 1.2) / 0.3)
            tones = self.cimb(ch)
            for st in range(16):
                if st % 2 and k < 0.6:
                    continue
                m = tones[(st * 3 + b) % len(tones)]
                mix.place('cimbalom', self.cimbalom(m, 0.7 + 0.3 * (st % 4 == 0)), t0 + st * S16, 0.05 * k,
                          pan=0.3 * math.sin(st))

    def boom(self, mix, t, g):
        mix.place('bass', self.upright(self.T.KEY_ROOT - 12, 0.8), t, g)

    def clash(self, mix, t):
        mix.place('fx', self.A.chime(91, 0.25), t, 0.05)

    def tonic_chord(self):
        """the tonic chord spread wide on the accordion (#28's stinger: D3 A3 D4 F4 A4)"""
        r = self.T.KEY_ROOT + 12
        third = self.T.SCALE[2]                    # the key's third (minor in #28)
        return (r, r + 7, r + 12, r + 12 + third, r + 19)

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        """(new: #28 had no VS card) the oom-pah under the card from frame 0, a unison bass + accordion hit per word"""
        A = self.A
        self.bar(mix, 0, intensity=1.0)
        for j, tw_ in enumerate(times):
            mix.place('bass', self.upright(self.T.KEY_ROOT, 0.5), tw_, 0.62)
            mix.place('accordion', self.reed(self.voicing(self.T.CHORDS[0]), 0.30), tw_, 0.24)
            mix.place('fx', A.thud(0.8, 120, 34, seed=int(tw_ * 50) + 3), tw_, 0.30)

    def vs(self, mix, t):
        A, SR = self.A, self.SR
        nn = int(1.4 * SR)
        tt_ = A.tt(nn)
        crack = A.bp(np.random.default_rng(91).normal(0, 1, nn), 1500, 9000) * np.exp(-tt_ / 0.035)
        rumble = A.lp(np.random.default_rng(92).normal(0, 1, nn), 160) * np.clip(tt_ / 0.02, 0, 1) * np.exp(-tt_ / 0.5) * 3.0
        mix.place('fx', crack * 0.9 + rumble, t, 0.32)

    def title_rest(self, mix):
        pass

    def count_in(self, mix):
        """the bass + an accordion stab on each count, on the count chord (#28: the dominant, A7)"""
        T = self.T
        ch = T.CHORDS[T.FIGHT_BAR - 1]
        r = self.bass_pair(ch)[0]
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            tc = max(tc, 0.004)                            # (#28's "3" 4 ms into the file: no click at the seam)
            mix.place('bass', self.upright(r, 0.4), tc, 0.62)
            mix.place('accordion', self.reed(self.voicing(ch), 0.16), tc, 0.20)

    def fight_start(self, mix):
        mix.place('fx', self.A.thud(0.7, 110, 34), self.T.T_FIGHT, 0.30)

    def fight(self, mix, bar_ko):
        """the bed from FIGHT! to the K.O. (the fire builds), a swell into the final exchange (44 s), a tape stop"""
        T, A = self.T, self.A
        for b in range(T.FIGHT_BAR, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
        mix.place('fx', A.whoosh(3.6, 300, 6000), 44.0 - 3.6, 0.10)
        for k in ('bass', 'accordion', 'pad', 'cimbalom'):
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], self.M.T_KO, 0.42)

    def ko(self, mix):
        mix.place('bass', self.upright(self.T.KEY_ROOT - 12, 1.4), self.M.T_KO, 0.62)
        mix.place('fx', self.A.thud(0.9, 120, 30), self.M.T_KO, 0.7)

    def stinger(self, mix, tw):
        mix.place('accordion', self.reed(self.tonic_chord(), 1.2, rel=0.5), tw, 0.20)
        mix.place('bass', self.upright(self.T.KEY_ROOT, 0.9), tw, 0.6)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        mix.place('fx', self.A.whoosh(t_solo - tw - 0.1, 300, 5000), tw + 0.1, 0.10)

    def victory(self, mix, b_solo):
        """a lighter oom-pah under the solo"""
        for b in range(b_solo, self.T.N_BARS - 1):
            self.bar(mix, b, light=True)

    def last_bars(self, mix):
        """the last bar builds into the slam: the bass climbing the last chord, accordion off-beats, a swelling pad"""
        T, M = self.T, self.M
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.CHORDS[-1]
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.22)
        r = self.bass_pair(ch)[0]
        climb = T.chord_tones(r, r + 11, ch)[:4]
        vo = self.voicing(ch)
        for bt in range(4):
            mix.place('bass', self.upright(climb[bt % len(climb)], 0.36), t_last + bt * T.BEAT, 0.55)
            mix.place('accordion', self.reed(vo, 0.18), t_last + bt * T.BEAT + T.BEAT / 2, 0.18)
        mix.place('pad', self.reed([vo[0] - 12] + list(vo), T.DUR - t_last + 0.02, att=0.6, rel=0.001, cut=1800), t_last, 0.07)
