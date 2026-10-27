"""Mood: prog-rock keyboard duel (#45 Piano vs Organ). A dramatic minor 70s-prog groove with NO keyboards of its own (the
two fighters are the keyboards): big drums (a deep kick, a fat backbeat snare, toms, crashes, hats) and a driving,
slightly overdriven bass guitar picking 8ths. Odd accents: every other bar of the loop is a 3+3+2 push (kick, bass
accents and an octave jump on 8ths 1, 4 and 7) inside a 4/4 bar, so the loop math holds; the rest drive straight 8ths
with the backbeat. Tom fills close each 4-bar phrase; the back half adds 16th hats, a ride bell on the pushes and
heavier toms. Built for A minor (Am F G E7 Dm), any chords work (the bass plays each bar's root, fifths and octaves)."""
import numpy as np


class Bed:
    buses = {'drums': (1.0, 0.12), 'bass': (1.15, 0.06)}      # bus: (gain, reverb send)
    reverb = (1.6, 0.5, 0.18)                       # an arena: (dur, decay, wet)
    comp = (0.15, 0.62)                             # master compressor: threshold, exponent
    roar_gain, doors_gain = 0.15, 0.40
    landing = (0.07, 0.05)                          # caption landing thuds (first line, others)
    pop = (93, 100, 0.10)                           # the glass pop when the loser is whole again

    def __init__(self, M):
        self.M, self.T, self.A = M, M.T, M.A
        self.SR = M.SR
        self._cache = {}

    # ---------------------------------------------------------------- the kit
    def kick(self, vel=1.0):
        if 'kick' not in self._cache:
            A = self.A
            x = A.kick(1.0, 0.24, 150, 46)[:int(0.4 * self.SR)]
            n = len(x)
            t = A.tt(n)
            click = A.bp(np.random.default_rng(31).normal(0, 1, n), 2500, 7000) * np.exp(-t / 0.004) * 0.35
            self._cache['kick'] = A.hp(x + click, 34)
        return self._cache['kick'] * vel

    def snare(self, vel=1.0, seed=0):
        key = ('snare', seed % 4)
        if key not in self._cache:
            A, SR = self.A, self.SR
            n = int(0.5 * SR)
            t = A.tt(n)
            r = np.random.default_rng(40 + seed % 4)
            wires = A.bp(r.normal(0, 1, n), 1800, 9000) * (np.exp(-t / 0.09) * 0.8 + np.exp(-t / 0.25) * 0.12)
            body = np.sin(2 * np.pi * 185 * t + 2.0 * np.exp(-t / 0.01)) * np.exp(-t / 0.07) * 0.7
            crack = A.bp(r.normal(0, 1, n), 3000, 6000) * np.exp(-t / 0.006) * 0.6
            self._cache[key] = np.tanh((wires + body + crack) * 1.4) * 0.8
        return self._cache[key] * vel

    def hat(self, vel=1.0, open_=False, seed=0):
        x = self.A.hat(0.03, open_)
        return x * vel

    def tom(self, f, vel=1.0):
        key = ('tom', int(f))
        if key not in self._cache:
            A, SR = self.A, self.SR
            n = int(0.55 * SR)
            t = A.tt(n)
            fr = f * (1 + 0.5 * np.exp(-t / 0.03))
            ph = 2 * np.pi * np.cumsum(fr) / SR
            x = np.sin(ph) * np.exp(-t / 0.22) + 0.2 * A.bp(np.random.default_rng(int(f)).normal(0, 1, n), 200, 2500) * \
                np.exp(-t / 0.02)
            self._cache[key] = x * 0.9
        return self._cache[key] * vel

    def ride(self, vel=1.0):
        A = self.A
        n = int(0.6 * self.SR)
        t = A.tt(n)
        x = sum(np.sin(2 * np.pi * f * t) for f in (3150, 4420, 5870)) * np.exp(-t / 0.35) * 0.15
        x += A.hp(np.random.default_rng(55).normal(0, 1, n), 6000) * np.exp(-t / 0.08) * 0.2
        return x * vel

    def fill(self, mix, t0, dur, heavy=False):
        """a tom fill in 16ths down the toms over `dur`"""
        T = self.T
        toms = (196, 165, 131, 98) if not heavy else (175, 147, 117, 87)
        n = int(round(dur / T.S16))
        for j in range(n):
            f = toms[min(len(toms) - 1, j * len(toms) // max(1, n))]
            mix.place('drums', self.tom(f, 0.9 + 0.1 * (j % 2 == 0)), t0 + j * T.S16, 0.34 if heavy else 0.28,
                      pan=0.4 - 0.8 * j / max(1, n - 1))

    # ---------------------------------------------------------------- the bass guitar
    def bass_note(self, m, dur, vel=1.0, drive=1.0):
        """a picked electric bass: a few harmonics with a pluck envelope, a pick click, a little overdrive"""
        key = (int(m), round(dur, 3), round(vel, 2), round(drive, 2))
        if key not in self._cache:
            A, SR = self.A, self.SR
            n = int((dur + 0.05) * SR)
            t = A.tt(n)
            f = A.mtof(m)
            ph = 2 * np.pi * f * t
            x = np.sin(ph) + 0.55 * np.sin(2 * ph) * np.exp(-t / 0.25) + 0.30 * np.sin(3 * ph) * np.exp(-t / 0.12) \
                + 0.15 * np.sin(4 * ph) * np.exp(-t / 0.07)
            x *= np.exp(-t / 0.9) * np.clip(t / 0.003, 0, 1) * np.clip((dur + 0.05 - t) / 0.04, 0, 1)
            click = A.bp(np.random.default_rng(int(m)).normal(0, 1, n), 1500, 4000) * np.exp(-t / 0.004) * 0.15
            y = np.tanh((x + click) * (1.3 + 0.7 * drive)) / np.tanh(1.3 + 0.7 * drive)
            self._cache[key] = A.lp(y, 1400) * vel
        return self._cache[key]

    def root(self, ch):
        return self.T.root_above(ch, 28)

    def fifth(self, ch):
        r = self.root(ch)
        return r + 7

    # ---------------------------------------------------------------- one bar
    def push_bar(self, b):
        """3+3+2 push bars: every other bar of the loop (the 2nd, 4th ...), and the loop's last bar"""
        T = self.T
        j = (b - T.LOOP_START) % len(T.LOOP)
        return j % 2 == 1

    def bar(self, mix, b, intensity=1.0, light=False):
        T = self.T
        t0 = b * T.BAR
        ch = T.CHORDS[b % len(T.CHORDS)]
        nxt = T.CHORDS[(b + 1) % len(T.CHORDS)]
        push = self.push_bar(b) and not light
        big = intensity > 1.3 and not light
        r = self.root(ch)
        e8 = T.BEAT / 2
        # drums
        kicks = (0, 3, 6) if push else ((0, 4, 5) if not light else (0, 4))
        snares = (2, 6) if not light else (4,)
        for i in range(8):
            ti = t0 + i * e8
            if i in kicks:
                mix.place('drums', self.kick(1.0), ti, 0.60 if not light else 0.50)
            if i in snares:
                mix.place('drums', self.snare(1.0, b + i), ti, 0.30 if not light else 0.24)
            acc = push and i in (0, 3, 6)
            mix.place('drums', self.hat(1.0, open_=acc and i != 0, seed=i), ti, 0.07 if not acc else 0.09,
                      pan=0.25)
            if big:
                mix.place('drums', self.hat(0.6, seed=i + 9), ti + e8 / 2, 0.04, pan=0.25)
                if acc:
                    mix.place('drums', self.ride(1.0), ti, 0.10, pan=-0.3)
        j = (b - T.LOOP_START) % len(T.LOOP)
        if j % 4 == 3 and not light:                                    # a tom fill closes each 4-bar phrase
            self.fill(mix, t0 + 3 * T.BEAT, T.BEAT, heavy=big)
        # the bass guitar: 8ths on the root, the push's accents jump an octave, the last 8th walks to the next root
        for i in range(8):
            ti = t0 + i * e8
            if push and i in (3, 6):
                m, v = r + 12, 1.0
            elif push and i == 0:
                m, v = r, 1.0
            elif i == 7 and not light:
                nr = self.root(nxt)
                m, v = (nr + 1 if nr + 1 != r else nr - 1), 0.8          # a chromatic approach into the next bar
            elif i == 4 and not push:
                m, v = self.fifth(ch), 0.85
            else:
                m, v = r, 0.75 if i % 2 else 0.9
            mix.place('bass', self.bass_note(m, e8 * 0.92, v, drive=1.0 + 0.6 * big), ti, 0.42)

    def boom(self, mix, t, g):
        mix.place('drums', self.kick(1.0), t, g)
        mix.place('drums', self.tom(87, 1.0), t, g * 0.5)

    def clash(self, mix, t):
        mix.place('fx', self.M.clank(81, int(t * 3)), t, 0.035)

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        """a unison band hit under each word of the VS card: kick, crash, floor tom, the bass on the root"""
        A, ch = self.A, self.T.CHORDS[0]
        r = self.root(ch)
        for j, tw_ in enumerate(times):
            mix.place('drums', self.kick(1.0), tw_, 0.70)
            mix.place('drums', A.crash(1.4), tw_, 0.10)
            mix.place('drums', self.tom(87, 1.0), tw_, 0.30)
            mix.place('bass', self.bass_note(r, 0.5, 1.0, 1.4), tw_, 0.50)
            mix.place('fx', A.thud(0.8, 120, 34, seed=int(tw_ * 50) + 3), tw_, 0.30)

    def vs(self, mix, t):
        """VS: a thunder crack where it splits the divider"""
        A, SR = self.A, self.SR
        nn = int(1.4 * SR)
        tt_ = A.tt(nn)
        crack = A.bp(np.random.default_rng(91).normal(0, 1, nn), 1500, 9000) * np.exp(-tt_ / 0.035)
        rumble = A.lp(np.random.default_rng(92).normal(0, 1, nn), 160) * np.clip(tt_ / 0.02, 0, 1) * np.exp(-tt_ / 0.5) * 3.0
        mix.place('fx', crack * 0.9 + rumble, t, 0.36)

    def title_rest(self, mix):
        """the bass rings on under the card (never quiet), a tom fill on beat 4 as the doors open"""
        T = self.T
        r = self.root(T.CHORDS[0])
        for i in range(6):                                              # the bass pulses 8ths under the card
            mix.place('bass', self.bass_note(r, T.BEAT / 2 * 0.9, 0.7 + 0.2 * (i % 2 == 0)), i * T.BEAT / 2 + 0.004,
                      0.34)
        self.fill(mix, 3 * T.BEAT, T.BEAT, heavy=True)

    def count_in(self, mix):
        """bar 1: the band counts in on the dominant: kick + snare + crash choke on each count, the bass on the
        root, a tom fill on beat 4 into FIGHT!"""
        T = self.T
        ch = T.CHORDS[1]
        r = self.root(ch)
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place('drums', self.kick(1.0), tc, 0.62)
            mix.place('drums', self.snare(1.0, 70 + j), tc, 0.30)
            mix.place('drums', self.A.crash(0.4), tc, 0.06)
            mix.place('bass', self.bass_note(r, T.BEAT * 0.45, 1.0, 1.2), tc, 0.46)
            mix.place('bass', self.bass_note(r + 12, T.BEAT * 0.4, 0.8), tc + T.BEAT / 2, 0.36)
        t4 = T.BAR + 3 * T.BEAT
        mix.place('drums', self.kick(1.0), t4, 0.55)
        self.fill(mix, t4, T.BEAT, heavy=True)
        for i in range(4):                                              # the bass climbs into FIGHT!
            mix.place('bass', self.bass_note(r + (0, 3, 5, 7)[i] - 12 + 12, T.S16 * 0.9, 0.9, 1.2), t4 + i * T.S16, 0.40)

    def fight_start(self, mix):
        mix.place('drums', self.A.crash(1.8), self.T.T_FIGHT, 0.12)
        mix.place('fx', self.A.thud(0.7, 110, 34), self.T.T_FIGHT, 0.34)

    def fight(self, mix, bar_ko):
        """the groove from FIGHT! to the K.O. (denser as it goes), a fill + riser into the back half, a tape stop at
        the K.O."""
        T, A = self.T, self.A
        for b in range(2, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.6 * (b / max(1, bar_ko)))
        for b in range(2, bar_ko + 1, 4):                               # every 4-bar phrase: a crash on its first beat
            mix.place('drums', A.crash(1.6), b * T.BAR, 0.08)
        t_half = (T.LOOP_START + 2 * len(T.LOOP)) * T.BAR               # the back half (bar 18)
        mix.place('fx', self.M.riser(T.BAR, over=0.0, seed=12) * 0.6, t_half - T.BAR, 0.08)
        mix.place('drums', A.crash(2.0), t_half, 0.12)
        for k in ('drums', 'bass'):
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], self.M.T_KO, 0.42)

    def ko(self, mix):
        mix.place('drums', self.A.crash(2.4), self.M.T_KO, 0.16)
        mix.place('fx', self.A.thud(0.9, 120, 30), self.M.T_KO, 0.6)

    def stinger(self, mix, tw):
        mix.place('drums', self.kick(1.0), tw, 0.65)
        mix.place('drums', self.A.crash(1.8), tw, 0.12)
        mix.place('bass', self.bass_note(self.root(self.T.CHORDS[0]), 0.8, 1.0, 1.4), tw, 0.50)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        mix.place('fx', self.A.whoosh(max(0.2, t_solo - tw - 0.1), 300, 5000), tw + 0.1, 0.08)
        T = self.T
        b0 = int(np.ceil((tw + 0.05) / T.BEAT))
        for j in range(b0, int(round(t_solo / T.BEAT))):                 # the kick keeps the pulse up to the solo
            mix.place('drums', self.kick(0.9), j * T.BEAT, 0.48)
            mix.place('drums', self.hat(1.0), j * T.BEAT + T.BEAT / 2, 0.07)

    def victory(self, mix, b_solo):
        """the band plays on under the solo (straight bars, no pushes or fills), up to the build"""
        T = self.T
        for b in range(b_solo, T.N_BARS - 1):
            self.bar(mix, b, intensity=1.15)

    def last_bars(self, mix):
        """the last bar builds into the title slam on sample 0: a 16th snare roll, the kick on every beat, the bass
        climbing the dominant in 8ths, a riser right up to the seam"""
        T, M, A = self.T, self.M, self.A
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.CHORDS[-1]
        r = self.root(ch)
        n = int(round((T.DUR - t_last) / T.S16))
        for j in range(n):
            mix.place('drums', self.snare(0.5 + 0.5 * j / n, j), t_last + j * T.S16, 0.16 + 0.12 * j / n)
        for bt in range(4):
            mix.place('drums', self.kick(1.0), t_last + bt * T.BEAT, 0.58)
        climb = [r, r, r + 4, r + 4, r + 7, r + 7, r + 10, r + 11]
        for i, m in enumerate(climb):
            mix.place('bass', self.bass_note(m, T.BEAT / 2 * 0.9, 0.95, 1.4), t_last + i * T.BEAT / 2, 0.44)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.18)
        mix.place('drums', A.crash(1.0) * np.linspace(0, 1, int(1.0 * self.SR)) ** 2, T.DUR - 1.0 + 0.012, 0.06)
