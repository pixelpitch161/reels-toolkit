"""Mood: sneaky heist caper (#52 Oboe vs Bassoon). A minor tiptoe groove with NO woodwinds of its own (the two double
reeds are the leads: the oboe on the high, nasal melody, the bassoon on the low comic staccato line):

    drums       brushes: a soft kick on 1 (and 3 later), a brush swish on 2 and 4, rim clicks on the off-beats (a
                syncopated "click ... click-click" that tiptoes), a ride ticking the swing 8ths in the back half
    bass        a minor walking upright: a plucked note on every beat, root on 1, chord tones, a chromatic approach into
                the next chord on beat 4
    pizz        pizzicato strings: off-beat chord plucks, broken into a creeping arpeggio in the back half
    vibes       muted vibes stabs (soft mallet, motor tremolo, muffled) on the and of 2 and on 4 of every other bar,
                every bar later
    tension     tiptoe tension: a high tremolo string note (the chord's 5th) shimmering very quietly, and a muted
                clock tick on the 16ths in the last stretch before the K.O.

The arrangement thickens with the bars (kick on 3, the ride, arpeggiated pizz, vibes every bar, the tick), never with
level: a sneaky mood still at the series' loudness from frame 0 (density and the master, not playing quietly). The VS
card is three full caper hits (kick, rim, a vibes + pizz chord, the bass's low root); the count-in keeps the brushes
going; the last bar is a creeping chromatic walk up the dominant into the tonic on the title slam. Built for D minor
(Dm Dm Bb A7 | Dm Gm Bb A7); any chords work (roots by rule)."""
import numpy as np


class Bed:
    buses = {'drums': (1.25, 0.12), 'bass': (1.50, 0.06), 'pizz': (1.00, 0.28), 'vibes': (0.80, 0.40),
             'tension': (0.55, 0.45)}                    # bus: (gain, reverb send)
    reverb = (1.6, 0.5, 0.15)                            # a dim back room: (dur, decay, wet)
    comp = (0.18, 0.55)                                  # master compressor: threshold, exponent
    roar_gain, doors_gain = 0.15, 0.40
    landing = (0.07, 0.05)
    pop = (93, 100, 0.10)

    def __init__(self, M):
        self.M, self.T, self.A, self.SR = M, M.T, M.A, M.SR
        self._c = {}

    def _cache(self, key, fn):
        if key not in self._c:
            self._c[key] = fn()
        return self._c[key]

    # ---------------------------------------------------------------- harmony
    def root(self, ch):
        return self.T.root_above(ch, 33)                       # the upright's root, A1..G#2

    def voicing(self, ch, lo=57, n=4):
        return self.T.chord_tones(lo, lo + 12, ch)[:n]

    def walk(self, ch, nxt):
        """four walking-bass notes for a bar of `ch` heading to `nxt`: root, a chord tone up, another, a chromatic
        approach into the next root"""
        r = self.root(ch)
        tones = self.T.chord_tones(r, r + 12, ch)
        n_r = self.root(nxt)
        third = tones[1] if len(tones) > 1 else r + 3
        fifth = tones[2] if len(tones) > 2 else r + 7
        appr = n_r + 1 if n_r + 1 <= r + 12 else n_r - 1
        if abs(appr - fifth) > 7:
            appr = n_r - 1
        return [r, third, fifth, appr]

    # ---------------------------------------------------------------- instruments
    def kick(self, v=1.0):
        A = self.A
        return self._cache('kick', lambda: A.hp(A.kick(0.9, 0.18, 120, 46), 35)[:int(0.3 * self.SR)]) * v

    def brush(self, v=1.0, seed=0):
        """a brush swish on the snare: a band-passed noise swell and fall"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.24 * SR)
            t = A.tt(n)
            x = A.bp(np.random.default_rng(60 + seed).normal(0, 1, n), 1800, 9000)
            env = np.clip(t / 0.03, 0, 1) * np.exp(-np.clip(t - 0.03, 0, None) / 0.07)
            tap = A.bp(np.random.default_rng(70 + seed).normal(0, 1, n), 900, 4000) * np.exp(-t / 0.012)
            return x * env * 0.9 + tap * 0.6
        return self._cache(('brush', seed % 3), make) * v

    def rim(self, v=1.0, seed=0):
        """a rim click: a hard, short, woody tick"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.06 * SR)
            t = A.tt(n)
            x = np.sin(2 * np.pi * 1650 * t) * np.exp(-t / 0.006) + np.sin(2 * np.pi * 820 * t) * np.exp(-t / 0.01) * 0.5
            return x + A.bp(np.random.default_rng(90 + seed).normal(0, 1, n), 2500, 8000) * np.exp(-t / 0.003) * 0.5
        return self._cache(('rim', seed % 2), make) * v

    def ride(self, v=1.0):
        A, SR = self.A, self.SR

        def make():
            n = int(0.5 * SR)
            t = A.tt(n)
            x = A.hp(np.random.default_rng(31).normal(0, 1, n), 5000) * np.exp(-t / 0.18) * 0.5
            return x + sum(np.sin(2 * np.pi * f * t) for f in (3150, 4630, 5960)) * np.exp(-t / 0.25) * 0.05
        return self._cache('ride', make) * v

    def upright(self, m, dur, v=1.0):
        """a plucked upright: a thumpy sine + a little 2nd harmonic, a soft attack thud"""
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.1) * SR)
            t = A.tt(n)
            f = A.mtof(m)
            x = np.sin(2 * np.pi * f * t) + 0.35 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.08) + \
                0.12 * np.sin(6 * np.pi * f * t) * np.exp(-t / 0.04)
            env = np.clip(t / 0.004, 0, 1) * np.exp(-t / 0.45) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.03))
            thud = A.lp(np.random.default_rng(int(m)).normal(0, 1, n), 400) * np.exp(-t / 0.01) * 0.3
            return A.hp(x * env + thud, 30)
        return self._cache(('up', int(m), round(dur, 3)), make) * v

    def pizz(self, ms, v=1.0):
        """pizzicato strings: a short plucked chord (Karplus-ish: bright attack, quick fall)"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.35 * SR)
            t = A.tt(n)
            x = np.zeros(n)
            for m in ms:
                f = A.mtof(m)
                x += (np.sin(2 * np.pi * f * t) + 0.5 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.03)
                      + 0.25 * np.sin(6 * np.pi * f * t) * np.exp(-t / 0.015)) * np.exp(-t / 0.09)
            return A.hp(x / max(1, len(ms)), 140) * np.clip(t / 0.002, 0, 1)
        return self._cache(('pz', tuple(ms)), make) * v

    def vibes(self, ms, dur=0.5, v=1.0):
        """muted vibes: soft-mallet bars with the motor tremolo, muffled"""
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.2) * SR)
            t = A.tt(n)
            x = sum(np.sin(2 * np.pi * A.mtof(m) * t) + 0.25 * np.sin(2 * np.pi * A.mtof(m) * 3.9 * t) * np.exp(-t / 0.05)
                    for m in ms) / max(1, len(ms))
            trem = 1 - 0.35 * (0.5 + 0.5 * np.sin(2 * np.pi * 5.5 * t))
            env = np.clip(t / 0.003, 0, 1) * np.exp(-t / 0.35) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.05))
            return A.lp(x * trem * env, 2600)
        return self._cache(('vb', tuple(ms), round(dur, 3)), make) * v

    def trem(self, m, dur, v=1.0):
        """a high tremolo string note (tiptoe tension): detuned saws, bowed fast"""
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.2) * SR)
            t = A.tt(n)
            x = A.bp(A.saw(A.mtof(m), n, det=0.006, voices=3), 900, 5000)
            tr = 0.6 + 0.4 * np.abs(np.sin(2 * np.pi * 7.5 * t))
            env = np.clip(t / 0.15, 0, 1) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.08))
            return x * tr * env
        return self._cache(('tr', int(m), round(dur, 3)), make) * v

    def tick(self, v=1.0):
        A, SR = self.A, self.SR

        def make():
            n = int(0.03 * SR)
            t = A.tt(n)
            return np.sin(2 * np.pi * 2900 * t) * np.exp(-t / 0.002)
        return self._cache('tick', make) * v

    # ---------------------------------------------------------------- one bar
    def bar(self, mix, b, intensity=1.0, light=False):
        """one bar of the caper (bar index b); intensity 1.0 .. 1.6 adds parts"""
        T = self.T
        t0 = b * T.BAR
        ch = T.CHORDS[b % T.N_BARS]
        nxt = T.CHORDS[(b + 1) % T.N_BARS]
        dense = intensity > 1.25 and not light
        e8 = T.BEAT / 2
        sw = e8 * 1.25                                         # the swung "and"
        for bt in range(4):
            tb = t0 + bt * T.BEAT
            if bt == 0 or (dense and bt == 2):
                mix.place('drums', self.kick(1.0), tb, 0.55)
            if bt in (1, 3):
                mix.place('drums', self.brush(1.0, seed=b + bt), tb, 0.34, pan=-0.1)
            if bt in (0, 2) or (bt == 3 and b % 2):            # rim clicks tiptoeing on the swung ands
                mix.place('drums', self.rim(1.0, seed=bt), tb + sw, 0.15, pan=0.2)
            if dense:
                mix.place('drums', self.ride(1.0), tb, 0.10, pan=0.3)
                mix.place('drums', self.ride(1.0), tb + sw, 0.06, pan=0.3)
        for j, m in enumerate(self.walk(ch, nxt)):              # the walking upright
            mix.place('bass', self.upright(m, T.BEAT * 0.9), t0 + j * T.BEAT, 0.50 if j == 0 else 0.44)
        vo = self.voicing(ch, 55)
        if dense:                                              # a creeping pizz arpeggio on the 8ths
            arp = [vo[0], vo[1], vo[2], vo[1], vo[-1], vo[1], vo[2], vo[0] + 12]
            for j, m in enumerate(arp):
                mix.place('pizz', self.pizz((m,)), t0 + j * e8 + (sw - e8) * (j % 2), 0.20 if j % 2 == 0 else 0.15,
                          pan=-0.25)
        else:                                                  # off-beat chord plucks
            for bt in range(4):
                mix.place('pizz', self.pizz(tuple(vo)), t0 + bt * T.BEAT + sw, 0.20, pan=-0.25)
        if dense or b % 2 == 1:                                # muted vibes stabs
            vv = self.voicing(ch, 62, 3)
            mix.place('vibes', self.vibes(tuple(vv), 0.4), t0 + 1.5 * T.BEAT, 0.16, pan=0.3)
            mix.place('vibes', self.vibes(tuple(vv), 0.6), t0 + 3 * T.BEAT, 0.14, pan=0.3)
        fifth = [m for m in self.T.chord_tones(76, 88, ch)][-1:]
        if fifth:
            mix.place('tension', self.trem(fifth[0], T.BAR * 0.96), t0, 0.05)
        if intensity > 1.5 and not light:
            for j in range(16):
                mix.place('tension', self.tick(1.0), t0 + j * T.S16, 0.05 if j % 4 else 0.08, pan=0.35)

    def clash(self, mix, t):
        mix.place('fx', self.rim(1.0, seed=1), t, 0.12)

    def hit(self, mix, t, ch, g=1.0):
        """a full caper hit: kick, rim, brush, a vibes + pizz chord and the upright's low root"""
        mix.place('drums', self.kick(1.0), t, 0.62 * g)
        mix.place('drums', self.rim(1.0), t, 0.24 * g)
        mix.place('drums', self.brush(1.0, seed=2), t, 0.28 * g)
        mix.place('vibes', self.vibes(tuple(self.voicing(ch, 62, 3)), 0.8), t, 0.22 * g, pan=0.3)
        mix.place('pizz', self.pizz(tuple(self.voicing(ch, 50))), t, 0.30 * g, pan=-0.25)
        mix.place('bass', self.upright(self.root(ch), 0.6), t, 0.55 * g)

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        A, ch = self.A, self.T.CHORDS[0]
        for tw_ in times:
            self.hit(mix, tw_, ch, 1.0)
            mix.place('drums', A.crash(1.2), tw_, 0.07)
            mix.place('fx', A.thud(0.8, 120, 34, seed=int(tw_ * 50) + 3), tw_, 0.28)

    def vs(self, mix, t):
        mix.place('fx', self.M.thunder(1.4, seed=91), t, 0.40)

    def title_rest(self, mix):
        """under the card (never quiet): the tremolo tension, rim clicks tiptoeing, the upright creeping up"""
        T = self.T
        ch = T.CHORDS[0]
        mix.place('tension', self.trem(self.T.chord_tones(76, 88, ch)[-1], 3 * T.BEAT), 0.05, 0.08)
        for bt in range(4):
            mix.place('drums', self.rim(1.0, seed=bt), bt * T.BEAT + T.BEAT / 2 * 1.25, 0.15, pan=0.2)
        mix.place('drums', self.brush(1.0, seed=1), 3 * T.BEAT, 0.34)
        r = self.root(ch)
        mix.place('bass', self.upright(r + 7, T.BEAT * 0.45), 3 * T.BEAT, 0.44)
        mix.place('bass', self.upright(r + 8, T.BEAT * 0.45), 3.5 * T.BEAT, 0.44)

    def count_in(self, mix):
        T = self.T
        self.bar(mix, 1, intensity=1.1)
        ch = T.CHORDS[1]
        for tc, txt in T.COUNT[:3]:
            mix.place('vibes', self.vibes(tuple(self.voicing(ch, 62, 3)), 0.3), tc, 0.18, pan=0.3)

    def fight_start(self, mix):
        mix.place('drums', self.A.crash(1.8), self.T.T_FIGHT, 0.12)
        mix.place('fx', self.A.thud(0.7, 110, 34), self.T.T_FIGHT, 0.32)

    def fight(self, mix, bar_ko):
        T, A = self.T, self.A
        for b in range(2, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.65 * (b / bar_ko))
        for b in range(2, bar_ko + 1, 4):
            mix.place('drums', A.crash(1.4), b * T.BAR, 0.06)
        t_half = (T.LOOP_START + len(T.LOOP)) * T.BAR
        if t_half < self.M.T_KO:
            mix.place('fx', self.M.riser(T.BAR, over=0.0, seed=12) * 0.6, t_half - T.BAR, 0.08)
            mix.place('drums', A.crash(2.0), t_half, 0.12)
        for k in self.buses:
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], self.M.T_KO, 0.42)

    def ko(self, mix):
        mix.place('drums', self.A.crash(2.4), self.M.T_KO, 0.16)
        mix.place('fx', self.A.thud(0.9, 120, 30), self.M.T_KO, 0.6)

    def stinger(self, mix, tw):
        self.hit(mix, tw, self.T.CHORDS[0], 1.0)
        mix.place('drums', self.A.crash(1.8), tw, 0.10)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        T = self.T
        mix.place('fx', self.A.whoosh(max(0.2, t_solo - tw - 0.1), 300, 5000), tw + 0.1, 0.08)
        for b in range(int(tw // T.BAR) + 1, int(round(t_solo / T.BAR))):
            self.bar(mix, b, intensity=1.2)

    def victory(self, mix, b_solo):
        T = self.T
        for b in range(b_solo, T.N_BARS - 1):
            self.bar(mix, b, intensity=1.3)

    def last_bars(self, mix):
        """the last bar creeps into the title slam: the upright walking chromatically up in 8ths to the tonic, rim
        clicks on every 8th, brushes on every beat, the tremolo rising, a riser right up to the seam"""
        T, M = self.T, self.M
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.chord_at(t_last)
        tgt = self.root(T.CHORDS[0]) + 12
        e8 = T.BEAT / 2
        for bt in range(4):
            tb = t_last + bt * T.BEAT
            mix.place('drums', self.kick(1.0), tb, 0.50)
            mix.place('drums', self.brush(1.0, seed=bt), tb, 0.30)
            mix.place('drums', self.rim(1.0, seed=bt), tb + e8, 0.16, pan=0.2)
        for j in range(8):
            mix.place('bass', self.upright(tgt - 8 + j, e8 * 0.85), t_last + j * e8, 0.46)
        vv = self.voicing(ch, 62, 3)
        mix.place('vibes', self.vibes(tuple(vv), 0.4), t_last + 1.5 * T.BEAT, 0.16, pan=0.3)
        mix.place('pizz', self.pizz(tuple(self.voicing(ch, 55))), t_last + 3.5 * T.BEAT, 0.22, pan=-0.25)
        top = self.T.chord_tones(76, 88, ch)[-1]
        mix.place('tension', self.trem(top, T.DUR - t_last), t_last, 0.08)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.18)
