"""Mood: dark disco (#51 Alto Sax vs Tenor Sax). A minor four-on-the-floor groove with NO saxes of its own (the two saxes
are the disco leads: the alto on the bright lead, the tenor on the husky, honking low line):

    drums       four-on-the-floor kick, a clap + snare on 2 and 4, open hi-hats on every off-beat 8th (the disco
                "tss"), closed 16ths ticking under them in the back half, a crash on every phrase line
    bass        an octave-jumping synth bass (root low, root high on the 8ths: the disco octave bass), a filtered saw with a
                plucky envelope, a chromatic pickup into each chord change in the back half
    strings     staccato string stabs (detuned saws, band-passed) on the and of 2 and on 4, off-beat stabs in the back half
    pad         a filtered minor pad under it all, its low-pass breathing open and shut over two bars
    shimmer     the mirror ball: tiny high glints (bell partials on random-looking but seeded 16ths), very quiet

The arrangement thickens with the bars (16th hats, off-beat stabs, the bass's pickups, more glints), never with level
(consistent loudness from frame 0). The VS card is three full band hits (kick, clap, a string stab, the bass's low
root) over a held pad; the count-in keeps the groove; the last bar is a filter-sweep riser with the octave bass climbing
into the tonic on the title slam. Built for A minor (Am F Dm E7 | Am F G E7); any chords work (roots by rule)."""
import numpy as np


class Bed:
    buses = {'drums': (1.30, 0.10), 'bass': (1.45, 0.04), 'strings': (0.90, 0.30), 'pad': (0.70, 0.35),
             'shimmer': (0.45, 0.50)}                    # bus: (gain, reverb send)
    reverb = (1.8, 0.55, 0.16)                           # a club: (dur, decay, wet)
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
        return self.T.root_above(ch, 33)                       # the bass's root, A1..G#2

    def voicing(self, ch, lo=57):
        """the chord's tones from `lo` up, three or four of them (the stab / pad voicing)"""
        tones = self.T.chord_tones(lo, lo + 12, ch)
        return tones[:4]

    # ---------------------------------------------------------------- instruments
    def kick(self, v=1.0, big=False):
        A = self.A

        def make():
            x = A.kick(1.0, 0.30 if big else 0.20, 160, 44)
            return A.hp(x, 32)[:int((0.5 if big else 0.32) * self.SR)]
        return self._cache(('kick', big), make) * v

    def clap(self, v=1.0, seed=0):
        A, SR = self.A, self.SR

        def make():
            n = int(0.25 * SR)
            t = A.tt(n)
            r = np.random.default_rng(80 + seed)
            x = np.zeros(n)
            for j, d in enumerate((0.0, 0.009, 0.019)):
                a = int(d * SR)
                x[a:] += A.bp(r.normal(0, 1, n - a), 900, 5200) * np.exp(-A.tt(n - a) / (0.011 if j < 2 else 0.06))
            sn = A.bp(r.normal(0, 1, n), 2000, 9000) * np.exp(-t / 0.08) * 0.5 + np.sin(2 * np.pi * 200 * t) * \
                np.exp(-t / 0.04) * 0.4
            return x * 0.6 + sn
        return self._cache(('clap', seed % 3), make) * v

    def hat(self, v=1.0, open_=False, seed=0):
        A, SR = self.A, self.SR

        def make():
            n = int((0.32 if open_ else 0.07) * SR)
            t = A.tt(n)
            x = A.hp(np.random.default_rng(40 + seed).normal(0, 1, n), 7500)
            return x * np.exp(-t / (0.11 if open_ else 0.02)) * np.clip(t / 0.001, 0, 1)
        return self._cache(('hat', open_, seed % 4), make) * v

    def bass(self, m, dur, v=1.0):
        """a filtered-saw disco bass: a plucky filter envelope over a saw + sub"""
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.05) * SR)
            t = A.tt(n)
            f = A.mtof(m)
            x = A.saw(f, n, det=0.004, voices=2) * 0.7 + np.sin(2 * np.pi * f * t) * 0.6
            out = np.zeros(n)
            nb = 12
            for k in range(nb):                                   # the filter closing over the note (a pluck)
                a, b = int(k * n / nb), int((k + 1) * n / nb)
                fc = 300 + 1500 * np.exp(-((a + b) / 2 / SR) / 0.07)
                out[a:b] = A.lp(x[max(0, a - 1024):b], fc)[-(b - a):]
            env = np.clip(t / 0.003, 0, 1) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.02))
            return out * env
        return self._cache(('bass', int(m), round(dur, 3)), make) * v

    def stab(self, ch, v=1.0, dur=0.16, lo=57):
        """a staccato string stab: detuned saws, band-passed, a quick bite"""
        A, SR = self.A, self.SR

        def make():
            ms = self.voicing(ch, lo)
            n = int((dur + 0.12) * SR)
            t = A.tt(n)
            x = sum(A.saw(A.mtof(m), n, det=0.007, voices=3) for m in ms) / max(1, len(ms))
            x = A.bp(x, 350, 2600)
            env = np.clip(t / 0.006, 0, 1) * np.where(t < dur, np.exp(-t / 0.25), np.exp(-dur / 0.25) * np.exp(-(t - dur) / 0.05))
            return x * env
        return self._cache(('stab', ch, round(dur, 3), lo), make) * v

    def pad(self, ch, dur, v=1.0):
        """the filtered pad: a saw chord whose low-pass breathes open and shut over the chord"""
        A, SR = self.A, self.SR

        def make():
            ms = self.voicing(ch, 52)
            n = int((dur + 0.3) * SR)
            t = A.tt(n)
            x = sum(A.saw(A.mtof(m), n, det=0.005, voices=3) for m in ms) / max(1, len(ms))
            out = np.zeros(n)
            nb = 16
            for k in range(nb):
                a, b = int(k * n / nb), int((k + 1) * n / nb)
                u = (a + b) / 2 / n
                fc = 500 + 700 * np.sin(np.pi * u) ** 2
                out[a:b] = A.lp(x[max(0, a - 2048):b], fc)[-(b - a):]
            out = A.hp(out, 160)
            env = np.clip(t / 0.25, 0, 1) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.12))
            return out * env
        return self._cache(('pad', ch, round(dur, 3)), make) * v

    def glint(self, m, v=1.0):
        """a mirror-ball glint: a tiny high bell partial"""
        A = self.A

        def make():
            return A.chime(int(m), 0.5) * 0.6
        return self._cache(('gl', int(m)), make) * v

    # ---------------------------------------------------------------- one bar
    def bar(self, mix, b, intensity=1.0, light=False):
        """one bar of the groove (bar index b)"""
        T, A = self.T, self.A
        t0 = b * T.BAR
        ch = T.CHORDS[b % T.N_BARS]
        nxt = T.CHORDS[(b + 1) % T.N_BARS]
        big = intensity > 1.25 and not light
        e8 = T.BEAT / 2
        for bt in range(4):
            tb = t0 + bt * T.BEAT
            mix.place('drums', self.kick(1.0), tb, 0.55 if not light else 0.46)
            mix.place('drums', self.hat(1.0, open_=True, seed=bt), tb + e8, 0.075, pan=0.25)
            if bt in (1, 3):
                mix.place('drums', self.clap(1.0, seed=b + bt), tb, 0.26, pan=-0.05)
        if big:
            for j in range(16):
                if j % 2 == 0:
                    mix.place('drums', self.hat(1.0, seed=j), t0 + j * T.S16, 0.035 if j % 4 else 0.05, pan=0.3)
        # the octave bass: root low on the beat, high on the and
        r = self.root(ch)
        for j in range(8):
            m = r if j % 2 == 0 else r + 12
            if big and j == 7 and nxt != ch:                        # a chromatic pickup into the next chord
                nr = self.root(nxt)
                m = nr + 12 - 1 if nr + 12 - 1 > r else nr + 12 + 1
            mix.place('bass', self.bass(m, e8 * 0.8, 1.0), t0 + j * e8, 0.42 if j % 2 == 0 else 0.34)
        # string stabs on the and of 2 and on 4 (plus the off-beats of 1 and 3 in the back half)
        for t_ in ((1.5, 3.0) + ((0.5, 2.5) if big else ())):
            mix.place('strings', self.stab(ch, 1.0), t0 + t_ * T.BEAT, 0.20 if t_ in (1.5, 3.0) else 0.14, pan=0.15)
        # the pad over the bar
        mix.place('pad', self.pad(ch, T.BAR * 0.98), t0, 0.16)
        # the mirror ball: a few glints on hashed 16ths
        ng = 2 + (2 if big else 0)
        for g in range(ng):
            j = int(self.M.h01(b, g, 7) * 16)
            m = 96 + int(self.M.h01(b, g, 11) * 4) * 3
            mix.place('shimmer', self.glint(m), t0 + j * T.S16, 0.05, pan=float(self.M.h01(b, g) * 1.2 - 0.6))

    def clash(self, mix, t):
        mix.place('fx', self.hat(1.0, open_=False, seed=2), t, 0.10)

    def hit(self, mix, t, ch, g=1.0):
        """a full band hit: kick, clap, a long string stab and the bass's low root"""
        mix.place('drums', self.kick(1.0, big=True), t, 0.6 * g)
        mix.place('drums', self.clap(1.0, seed=int(t * 10)), t, 0.28 * g)
        mix.place('strings', self.stab(ch, 1.0, dur=0.4), t, 0.28 * g, pan=0.15)
        mix.place('bass', self.bass(self.root(ch), 0.5, 1.0), t, 0.5 * g)

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
        """the held chord under the card (never quiet): the pad, open hats ticking, the kick keeping the floor"""
        T, A = self.T, self.A
        ch = T.CHORDS[0]
        mix.place('pad', self.pad(ch, 2.9 * T.BEAT), 0.05, 0.22)
        for bt in range(4):
            mix.place('drums', self.hat(1.0, open_=True, seed=bt), bt * T.BEAT + T.BEAT / 2, 0.08, pan=0.25)
        mix.place('drums', self.kick(0.9), 3 * T.BEAT, 0.45)
        mix.place('bass', self.bass(self.root(ch) + 12, T.BEAT * 0.4), 3 * T.BEAT + T.BEAT / 2, 0.34)
        n = int(3 * T.BEAT * self.SR)
        sw = A.hp(np.random.default_rng(5).normal(0, 1, n), 5000) * np.linspace(0.2, 1.0, n) ** 2
        mix.place('shimmer', sw, 0.0, 0.03)

    def count_in(self, mix):
        T = self.T
        b = 1
        self.bar(mix, b, intensity=1.3)
        ch = T.CHORDS[b]
        for tc, txt in T.COUNT[:3]:
            mix.place('strings', self.stab(ch, 1.0, dur=0.22), tc, 0.18, pan=0.15)

    def fight_start(self, mix):
        mix.place('drums', self.A.crash(1.8), self.T.T_FIGHT, 0.12)
        mix.place('fx', self.A.thud(0.7, 110, 34), self.T.T_FIGHT, 0.32)

    def fight(self, mix, bar_ko):
        T, A = self.T, self.A
        for b in range(2, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
        for b in range(2, bar_ko + 1, 4):
            mix.place('drums', A.crash(1.4), b * T.BAR, 0.07)
        t_half = (T.LOOP_START + 2 * len(T.LOOP)) * T.BAR
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
            self.bar(mix, b, intensity=1.4)

    def last_bars(self, mix):
        """the last bar builds into the title slam on sample 0: the kick on every beat, the octave bass climbing in 8ths
        to the tonic, stabs on every off-beat, the pad's filter sweeping open, a riser right up to the seam"""
        T, M = self.T, self.M
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.chord_at(t_last)
        tgt = self.root(T.CHORDS[0])
        e8 = T.BEAT / 2
        for bt in range(4):
            tb = t_last + bt * T.BEAT
            mix.place('drums', self.kick(1.0), tb, 0.55)
            mix.place('drums', self.hat(1.0, open_=True, seed=bt), tb + e8, 0.08, pan=0.25)
            if bt in (1, 3):
                mix.place('drums', self.clap(1.0, seed=bt), tb, 0.26)
            mix.place('strings', self.stab(ch, 1.0), tb + e8, 0.18, pan=0.15)
        walk = [tgt - 5, tgt + 7, tgt - 4, tgt + 8, tgt - 3, tgt + 9, tgt - 1, tgt + 11]
        for j, m in enumerate(walk):
            mix.place('bass', self.bass(m, e8 * 0.8), t_last + j * e8, 0.40)
        mix.place('pad', self.pad(ch, T.DUR - t_last + 0.012), t_last, 0.18)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.18)
