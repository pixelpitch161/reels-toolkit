"""Mood: dark second line (#48 Euphonium vs Trombone). A minor New Orleans brass-band street groove with NO horns of its
own (both fighters are brass and share a register, so they split the front line: the euphonium sings the melody, the
trombone riffs and smears around it):

    drums       the parade bass drum with a cymbal on top (the bass drummer's beater on the head, a stick on the cymbal):
                a syncopated two-bar figure (1, the and of 2, 4 | 1, the and of 2, 3, the and of 4), the cymbal's chick
                on every beat with its accent on 2 and 4; the snare: stick shots on 2 and 4, swung ghost notes between
                and press rolls into the phrase (the second-line buzz); the 16ths swing (SWING16)
    sousa       the sousaphone, low and simple (the Pixel Pitch tuba): root on 1, the fifth on the and of 2, the root an
                octave up on 4 (the bass drum's tresillo), a chromatic approach into the next chord in the back half
    perc        the street: a tambourine on the off-beats and hand claps on 2 and 4 join in the back half

The arrangement thickens with the bars (more ghost notes and press rolls, the tambourine, the claps, the sousaphone's
approach notes), never with level (consistent loudness from frame 0). The VS card is three full band hits with the
cymbal crashing and a held sousaphone note under a press-roll swell; the count-in keeps the groove; the last bar walks
the sousaphone up chromatically into the tonic under a press-roll crescendo and a riser into the title slam. Built for
E minor (Em Am C7 B7); any chords work (roots and fifths by rule)."""
import numpy as np

SWING16 = 0.60                                     # the off 16th at 60 % of the 8th: the second line's swung 16ths
BD_A = (0, 6, 12)                                  # bass drum, bar A: 1, the and of 2, 4 (16ths)
BD_B = (0, 6, 8, 14)                               # bar B: 1, the and of 2, 3, the and of 4 (pushing into the bar line)
GH_A = (2, 7, 9, 15)                               # snare ghost notes (16ths)
GH_B = (3, 6, 9, 11)
GH_X = (1, 5, 10, 13, 14)                          # extra ghosts in the back half


class Bed:
    buses = {'drums': (1.40, 0.14), 'sousa': (1.75, 0.10), 'perc': (1.10, 0.22)}   # bus: (gain, reverb send)
    reverb = (1.6, 0.5, 0.17)                      # a street between brick walls: (dur, decay, wet)
    comp = (0.18, 0.55)                            # master compressor: threshold, exponent
    roar_gain, doors_gain = 0.16, 0.40
    landing = (0.07, 0.05)                         # caption landing thuds (first line, others)
    pop = (93, 98, 0.10)                           # the glass pop when the loser is whole again

    def __init__(self, M):
        self.M, self.T, self.A, self.SR = M, M.T, M.A, M.SR
        self._c = {}

    def _cache(self, key, fn):
        if key not in self._c:
            self._c[key] = fn()
        return self._c[key]

    def t16(self, b, j):
        """the time of 16th j in bar b, swung (the off 16th of each 8th pair late)"""
        T = self.T
        e8 = T.BEAT / 2
        return b * T.BAR + (j // 2) * e8 + (j % 2) * SWING16 * e8

    # ---------------------------------------------------------------- harmony
    def root(self, ch):
        return self.T.root_above(ch, 36)                         # the sousaphone's root, C2..B2

    def fifth(self, ch):
        r = self.root(ch)
        pcs = self.T.CHORD_PCS[ch]
        f = r + ((pcs[2] - pcs[0]) % 12 if len(pcs) > 2 else 7)
        return f if f <= 50 else f - 12

    # ---------------------------------------------------------------- instruments
    def sousa(self, m, dur, vel=0.9):
        return self.M.note('TUBA', int(m), round(dur, 3), round(dur + 0.12, 2), round(vel, 1))

    def bass_drum(self, v=1.0, big=False):
        """the parade bass drum: a deep, loose head (longer than a kit kick), high-passed so the sousaphone stays on top"""
        A = self.A

        def make():
            x = A.kick(0.75 if big else 0.55, 0.30 if big else 0.22, 95, 46)
            return A.hp(x, 38)[:int((0.6 if big else 0.4) * self.SR)]
        return self._cache(('bd', big), make) * v

    def chick(self, v=1.0, accent=False, seed=0):
        """the cymbal on top of the bass drum, hit with a stick: a short trashy ching (choked by the player's hand
        on the off-accents)"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.5 * SR)
            t = A.tt(n)
            parts = (431, 1093, 2239, 3517, 4871, 6907, 8123)
            x = sum(np.sin(2 * np.pi * f * t + 1.7 * i) * (0.86 ** i) for i, f in enumerate(parts)) / len(parts)
            noise = A.bp(np.random.default_rng(21 + seed).normal(0, 1, n), 3000, 11000)
            dec = 0.22 if accent else 0.07
            return (x * 0.7 + noise * 0.8) * np.exp(-t / dec) * np.clip(t / 0.002, 0, 1)
        return self._cache(('ck', accent, seed), make) * v

    def snare(self, v=1.0, ghost=False, seed=0):
        """a parade snare: a stick shot (head + snares) or a ghost tap"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.25 * SR)
            t = A.tt(n)
            r = np.random.default_rng(50 + seed)
            wires = A.bp(r.normal(0, 1, n), 1800, 9000) * np.exp(-t / (0.045 if ghost else 0.12))
            body = (np.sin(2 * np.pi * 186 * t) + 0.5 * np.sin(2 * np.pi * 331 * t)) * np.exp(-t / 0.05)
            x = wires * (0.8 if ghost else 1.0) + body * (0.25 if ghost else 0.7)
            return x * np.clip(t / 0.001, 0, 1) * (0.32 if ghost else 1.0)
        return self._cache(('sn', ghost, seed % 4), make) * v

    def press(self, dur=0.2, v=1.0, seed=0, v0=None):
        """a snare press roll: the sticks pressed into the head, a buzz of tiny bounces with the drum's body under it;
        v0 makes it a crescendo from v0 to v (#46's buzz, a parade snare's pitch)"""
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.08) * SR)
            t = A.tt(n)
            rng = np.random.default_rng(60 + seed)
            buzz = A.bp(rng.normal(0, 1, n), 1600, 9000)
            rattle = 0.55 + 0.45 * np.abs(np.sin(np.pi * 30.0 * t + rng.uniform(0, 3)))
            body = np.sin(2 * np.pi * 186 * t) * np.exp(-t / 0.05) * 0.5
            env = np.clip(t / 0.004, 0, 1) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.03))
            if v0 is None:
                env = env * (0.55 + 0.45 * np.exp(-t / 0.06))
            else:
                env = env * (v0 / max(v, 1e-6) + (1 - v0 / max(v, 1e-6)) * np.clip(t / dur, 0, 1.05) ** 1.4)
            return (buzz * rattle + body) * env
        return self._cache(('pr', round(dur, 3), seed, v0 if v0 is None else round(v0 / max(v, 1e-6), 3)), make) * v

    def tambourine(self, v=1.0, seed=0):
        A, SR = self.A, self.SR

        def make():
            n = int(0.18 * SR)
            t = A.tt(n)
            jingle = sum(np.sin(2 * np.pi * f * t + i) for i, f in enumerate((5100, 6300, 7400, 8900))) / 4
            noise = A.bp(np.random.default_rng(70 + seed).normal(0, 1, n), 5000, 12000)
            return (jingle * 0.5 + noise) * np.exp(-t / 0.05) * np.clip(t / 0.002, 0, 1)
        return self._cache(('tb', seed % 3), make) * v

    def clap(self, v=1.0, seed=0):
        """a few hands clapping together (three bursts a few ms apart)"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.2 * SR)
            t = A.tt(n)
            r = np.random.default_rng(90 + seed)
            x = np.zeros(n)
            for j, d in enumerate((0.0, 0.007, 0.016)):
                a = int(d * SR)
                burst = A.bp(r.normal(0, 1, n - a), 900, 4500) * np.exp(-A.tt(n - a) / (0.012 if j < 2 else 0.04))
                x[a:] += burst * (0.8 if j < 2 else 1.0)
            return x * 0.6
        return self._cache(('cl', seed % 3), make) * v

    # ---------------------------------------------------------------- one bar
    def bar(self, mix, b, intensity=1.0, light=False):
        """one bar of the second line (bar index b): the bass drum's figure (A on even bars, B on odd) with the cymbal's
        chick on every beat (accent on 2 + 4), stick shots on 2 + 4 and ghost notes, the sousaphone's tresillo; the back
        half (`intensity` > 1.25) adds ghosts, a press roll into beat 4, the tambourine's off-beats, claps on 2 + 4 and
        the sousaphone's approach into the next chord"""
        T = self.T
        t0 = b * T.BAR
        ch = T.CHORDS[b % T.N_BARS]
        nxt = T.CHORDS[(b + 1) % T.N_BARS]
        big = intensity > 1.25 and not light
        r, f5 = self.root(ch), self.fifth(ch)
        odd = b % 2 == 1
        for j in (BD_B if odd else BD_A):
            mix.place('drums', self.bass_drum(1.0 if j in (0, 12) else 0.85), self.t16(b, j), 0.50 if not light else 0.42)
        for bt in range(4):
            acc = bt in (1, 3)
            mix.place('drums', self.chick(1.0, accent=acc, seed=bt), t0 + bt * T.BEAT, 0.075 if acc else 0.05, pan=0.3)
            if acc:
                mix.place('drums', self.snare(1.0, seed=b * 4 + bt), t0 + bt * T.BEAT, 0.30, pan=-0.12)
        ghosts = (GH_B if odd else GH_A) + (GH_X if big else ())
        for j in ghosts:
            mix.place('drums', self.snare(1.0, ghost=True, seed=j + b), self.t16(b, j), 0.30, pan=-0.12)
        # the sousaphone: root on 1, the fifth on the and of 2, the root an octave up on 4 (where it fits)
        mix.place('sousa', self.sousa(r, 0.80 * T.BEAT, 0.95), t0, 0.62)
        mix.place('sousa', self.sousa(f5, 0.40 * T.BEAT, 0.85), self.t16(b, 6), 0.52)
        top = r + 12 if r + 12 <= 52 else r
        mix.place('sousa', self.sousa(top, 0.45 * T.BEAT, 0.85), t0 + 3 * T.BEAT, 0.50)
        if big:
            nr = self.root(nxt)
            pk = nr - 1 if nxt != ch else f5
            mix.place('sousa', self.sousa(pk, 0.28 * T.BEAT, 0.8), self.t16(b, 14), 0.42)
            mix.place('drums', self.press(0.85 * T.BEAT, 1.0, seed=b, v0=0.4), self.t16(b, 10), 0.13, pan=-0.12)
            for bt in range(4):
                mix.place('perc', self.tambourine(1.0, seed=bt), t0 + (bt + SWING16 / 2 + 0.25) * T.BEAT, 0.07, pan=0.35)
                if bt in (1, 3):
                    mix.place('perc', self.clap(1.0, seed=b + bt), t0 + bt * T.BEAT, 0.12, pan=-0.3)
        if b >= T.LOOP_START and (b - T.LOOP_START) % 4 == 3 and not light:   # a press roll into every phrase
            mix.place('drums', self.press(T.BEAT * 0.95, 1.0, seed=500 + b, v0=0.35), t0 + 3 * T.BEAT, 0.17, pan=-0.12)

    def press_roll(self, mix, t0, t1, g0=0.12, g1=0.32):
        """a snare press-roll crescendo from t0 to t1 (a fighter's fanfare, e.g. MARCHING ORDERS)"""
        mix.place('drums', self.press(max(0.05, t1 - t0), 1.0, seed=int(t0 * 10), v0=g0 / max(g1, 1e-6)), t0, g1 * 0.5,
                  pan=-0.12)

    def boom(self, mix, t, g):
        """a big drum hit (a slam landing): the bass drum + the sousaphone's low root"""
        mix.place('drums', self.bass_drum(1.0, big=True), t, g * 0.7)
        r = self.root(self.T.chord_at(t))
        mix.place('sousa', self.sousa(r - 12 if r - 12 >= 33 else r, 0.5, 1.0), t, 0.4 * g)

    def clash(self, mix, t):
        mix.place('fx', self.snare(1.0, ghost=True, seed=3), t, 0.10)

    def hit(self, mix, t, ch, g=1.0, ring=0.9):
        """a full band hit: the sousaphone's root (an octave down where it can go), the big bass drum, a stick shot and
        the cymbal crashed open"""
        A = self.A
        r = self.root(ch)
        mix.place('sousa', self.sousa(r - 12 if r - 12 >= 33 else r, ring, 1.0), t, 0.60 * g)
        mix.place('sousa', self.sousa(r, ring, 0.9), t, 0.32 * g)
        mix.place('drums', self.bass_drum(1.0, big=True), t, 0.62 * g)
        mix.place('drums', self.snare(1.0, seed=int(t * 10)), t, 0.32 * g, pan=-0.12)
        mix.place('drums', self.chick(1.0, accent=True, seed=5), t, 0.12 * g, pan=0.3)

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
        """the held chord under the card (never quiet): a long sousaphone root, the cymbal swelling, the snare's press
        roll swelling into beat 4 as the doors open"""
        T, A = self.T, self.A
        ch = T.CHORDS[0]
        mix.place('sousa', self.sousa(self.root(ch), 2.9 * T.BEAT, 0.85), 0.05, 0.46)
        n = int(3 * T.BEAT * self.SR)
        sw = A.hp(np.random.default_rng(5).normal(0, 1, n), 5000) * np.linspace(0.2, 1.0, n) ** 2
        mix.place('drums', sw, 0.0, 0.04)
        mix.place('drums', self.press(2 * T.BEAT, 1.0, seed=6, v0=0.25), T.BEAT, 0.14, pan=-0.12)
        mix.place('drums', self.press(T.BEAT * 0.95, 1.0, seed=7, v0=0.3), 3 * T.BEAT, 0.22, pan=-0.12)
        for bt in (1, 2, 3):                                    # the bass drum keeps the beat under the card
            mix.place('drums', self.bass_drum(0.8), bt * T.BEAT + T.BEAT / 2, 0.30)

    def count_in(self, mix):
        """bar 1: the band keeps the second line on the count-in chord (the groove from the card on), a band hit on
        each count, a press roll and a sousaphone walk-up into FIGHT!"""
        T = self.T
        b = 1
        self.bar(mix, b, intensity=1.4)
        ch = T.CHORDS[b]
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            self.hit(mix, tc, ch, 0.55, 0.30)
        t4 = T.BAR + 3 * T.BEAT
        mix.place('drums', self.press(T.BEAT * 0.98, 1.0, seed=9, v0=0.4), t4, 0.26, pan=-0.12)
        mix.place('sousa', self.sousa(self.root(T.CHORDS[2]) - 2, 0.3 * T.BEAT, 0.85), t4, 0.40)
        mix.place('sousa', self.sousa(self.root(T.CHORDS[2]) - 1, 0.3 * T.BEAT, 0.85), t4 + T.BEAT / 2, 0.42)

    def fight_start(self, mix):
        mix.place('drums', self.A.crash(1.8), self.T.T_FIGHT, 0.12)        # the band comes in
        mix.place('fx', self.A.thud(0.7, 110, 34), self.T.T_FIGHT, 0.34)

    def fight(self, mix, bar_ko):
        """the second line from FIGHT! to the K.O. (denser as it goes), a long press roll + riser into the back half,
        crashes on the phrase lines, a tape stop at the K.O."""
        T, A = self.T, self.A
        for b in range(2, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
        for b in range(2, bar_ko + 1, 8):
            mix.place('drums', A.crash(1.6), b * T.BAR, 0.08)
        t_half = (T.LOOP_START + 2 * len(T.LOOP)) * T.BAR
        if t_half < self.M.T_KO:
            mix.place('drums', self.press(2 * T.BEAT, 1.0, seed=33, v0=0.3), t_half - 2 * T.BEAT, 0.18, pan=-0.12)
            mix.place('fx', self.M.riser(T.BAR, over=0.0, seed=12) * 0.6, t_half - T.BAR, 0.08)
            mix.place('drums', A.crash(2.0), t_half, 0.12)
        for k in ('drums', 'sousa', 'perc'):
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], self.M.T_KO, 0.42)

    def ko(self, mix):
        mix.place('drums', self.A.crash(2.4), self.M.T_KO, 0.16)
        mix.place('fx', self.A.thud(0.9, 120, 30), self.M.T_KO, 0.6)

    def stinger(self, mix, tw):
        self.hit(mix, tw, self.T.CHORDS[0], 1.0, 0.8)
        mix.place('drums', self.A.crash(1.8), tw, 0.10)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        """from the stinger to the solo the band holds the groove (never a quiet stretch)"""
        T = self.T
        mix.place('fx', self.A.whoosh(max(0.2, t_solo - tw - 0.1), 300, 5000), tw + 0.1, 0.08)
        for b in range(int(tw // T.BAR) + 1, int(round(t_solo / T.BAR))):
            self.bar(mix, b, intensity=1.2)

    def victory(self, mix, b_solo):
        """the band plays on under the solo at full tilt (a victory parade), up to the build"""
        T = self.T
        for b in range(b_solo, T.N_BARS - 1):
            self.bar(mix, b, intensity=1.4)

    def last_bars(self, mix):
        """the last bar builds into the title slam on sample 0: a press-roll crescendo right up to the seam, the
        sousaphone walking up chromatically into the tonic, the bass drum on every beat, claps, a riser"""
        T, M = self.T, self.M
        t_last = (T.N_BARS - 1) * T.BAR
        mix.place('drums', self.press(T.DUR - t_last + 0.012, 1.0, seed=99, v0=0.35), t_last, 0.24, pan=-0.12)
        tgt = self.root(T.CHORDS[0])
        for bt in range(4):
            tb = t_last + bt * T.BEAT
            mix.place('sousa', self.sousa(tgt - 4 + bt, 0.6 * T.BEAT, 0.9), tb, 0.60)
            mix.place('drums', self.bass_drum(0.95), tb, 0.50)
            mix.place('drums', self.chick(1.0, accent=bt % 2 == 1, seed=bt), tb, 0.07, pan=0.3)
            mix.place('perc', self.clap(1.0, seed=bt), tb, 0.12, pan=-0.3)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.18)
