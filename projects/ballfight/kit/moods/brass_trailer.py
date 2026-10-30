"""Mood: cinematic brass trailer (#34 Trombone vs French Horn, ported as it was; #54 THE REMATCH plays it as the sequel).
Strings, braams and taiko with NO brass of its own: the two brass fighters are the brass section.

    strings     a low-string spiccato 16th ostinato over the bar's root (cellos / basses: detuned saws, a short bow)
    taiko       the trailer pattern on beats 1 + 3, growing through the fight (the and of 2 + 4, then a roll into 4)
    braam       the trailer braam (a low detuned saw stack with a filter swell and grit) on every loop's downbeat
    tremolo     a high string tremolo for the back half's tension (and under the whole title card: never quiet)
    fx          trailer ticking 8ths once the fight heats up

Config (opt-in, #54): `TRAILER = dict(sequel=True)` plays it darker and bigger, still with no brass: a contrabass octave
under the ostinato's 8ths, a dark cello pad under every bar, braams on every second bar (longer, with a sub under them),
the taiko ensemble from the start (big hits on 1 + 3, the roll every bar later), a war-drum fill into each loop, the
full trailer under the winner's solo, big taiko on every beat and a double braam swell into the seam. Without it the bed is #34's, note for note (checked against #34's audio.py with the
kit config examples/trombone_vs_horn: the strings, taiko, braam and tremolo buses bit-identical). Built for D minor (Dm
Bb Gm A); any chords work (roots and voicings by rule; #34's tables are what the rule gives for its chords)."""
import math

import numpy as np

OST = (0, 0, 12, 0, 0, 12, 0, 7, 0, 0, 12, 0, 7, 0, 12, 10)       # the 16th ostinato's intervals over the bar's root


class Bed:
    buses = {'strings': (0.85, 0.3), 'taiko': (1.2, 0.25), 'braam': (1.0, 0.3), 'tremolo': (1.0, 0.6),
             'pad': (0.9, 0.35)}                     # bus: (gain, reverb send) (#34's; 'pad' only in the sequel)
    reverb = (2.4, 0.6, 0.22)                        # #34: reelkit's default hall, wet 0.22
    comp = (0.22, 0.35)                              # #34's master compressor: threshold, exponent
    roar_gain, doors_gain = 0.16, 0.45
    landing = (0.08, 0.06)
    pop = (95, 102, 0.12)

    def __init__(self, M):
        self.M, self.T, self.A, self.SR = M, M.T, M.A, M.SR
        self.seq = bool(getattr(self.T, 'TRAILER', {}).get('sequel'))
        if self.seq:                                 # the sequel: braams and taiko up front, a darker hall
            self.buses = dict(self.buses, braam=(1.15, 0.32), taiko=(1.3, 0.25))
            self.reverb = (2.8, 0.62, 0.24)
        self._c = {}

    def _cache(self, key, fn):
        if key not in self._c:
            self._c[key] = fn()
        return self._c[key]

    # ---------------------------------------------------------------- harmony
    def root(self, ch):
        """the bar's root in the low register, A1..G#2 (#34's ROOT: Dm 38, Bb 34, Gm 43, A 33)"""
        return self.T.root_above(ch, 33)

    def trem(self, ch):
        """the high tremolo's voicing: the chord's three lowest tones from C#5 (#34's TREM)"""
        return tuple(self.T.chord_tones(73, 84, ch)[:3])

    def key_root(self):
        return self.T.KEY_ROOT                       # the key's root in the C2 octave (D -> 38)

    # ---------------------------------------------------------------- instruments (#34 audio.py, unchanged)
    def strings_low(self, m, dur, vel=1.0):
        """a spiccato cello/bass note: detuned saws, short bow envelope, dark"""
        A, SR = self.A, self.SR
        n = int((dur + 0.08) * SR)
        x = A.saw(A.mtof(m), n, 0.006, 3)
        x = A.lp(x, 700 + 500 * vel) * A.adsr(n, 0.004, 0.06, 0.35, 0.05)
        return x * vel

    def tremolo(self, ms, dur, att=0.4):
        """high string tremolo pad (violins above the horn's register)"""
        A, SR = self.A, self.SR
        n = int(dur * SR)
        t = A.tt(n)
        x = np.zeros(n)
        for m in ms:
            x += A.saw(A.mtof(m), n, 0.004, 3)
        x /= max(1, len(ms))
        x *= 0.75 + 0.25 * np.sign(np.sin(2 * np.pi * 15.0 * t))      # bowed tremolo
        return A.lp(A.hp(x, 900), 5200) * A.adsr(n, att, 0.2, 0.9, min(0.3, dur * 0.3))

    def braam(self, root, dur=1.6, vel=1.0, swell=0.06):
        """the trailer braam: a low detuned saw stack (root, octave, fifth) with a filter swell and grit"""
        A, SR = self.A, self.SR
        n = int(dur * SR)
        t = A.tt(n)
        x = np.zeros(n)
        for m, g in ((root, 1.0), (root + 12, 0.7), (root + 19, 0.35)):
            x += g * A.saw(A.mtof(m), n, 0.01, 4)
        env = np.clip(t / swell, 0, 1) * np.exp(-np.maximum(t - swell, 0) / (dur * 0.45))
        nb = 16
        out = np.zeros(n)
        for k in range(nb):
            a, b = int(k * n / nb), int((k + 1) * n / nb)
            u = (a + b) / 2 / n
            fc = 200 + 1500 * math.exp(-u * 4) * vel
            out[a:b] = A.lp(x[max(0, a - 4096):b], fc)[-(b - a):]
        out = np.tanh(2.2 * out) / np.tanh(2.2)
        return out * env * vel

    def taiko(self, vel=1.0, big=False, seed=0):
        """a big low drum: pitched body sweeping down + a skin slap"""
        A, SR = self.A, self.SR
        dur = 0.9 if big else 0.55
        n = int(dur * SR)
        t = A.tt(n)
        f = 52 + (130 if big else 160) * np.exp(-t / 0.045)
        body = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / (0.32 if big else 0.2))
        slap = A.bp(np.random.default_rng(seed).normal(0, 1, n), 180, 1400) * np.exp(-t / 0.018)
        return (body + 0.45 * slap) * vel

    def tick(self, vel=1.0, seed=1):
        """trailer ticking: a dry wooden click"""
        A, SR = self.A, self.SR
        n = int(0.05 * SR)
        t = A.tt(n)
        return A.bp(np.random.default_rng(seed).normal(0, 1, n), 2500, 7000) * np.exp(-t / 0.006) * vel

    # ---------------------------------------------------------------- the sequel's own (#54)
    def sub(self, root, dur):
        """a sine sub an octave under a braam (felt more than heard), swelling in and dying with it"""
        A, SR = self.A, self.SR

        def make():
            n = int(dur * SR)
            t = A.tt(n)
            env = np.clip(t / 0.08, 0, 1) * np.exp(-np.maximum(t - 0.08, 0) / (dur * 0.5))
            return np.sin(2 * np.pi * A.mtof(root) * t) * env
        return self._cache(('sub', root, round(dur, 3)), make)

    def pad(self, ms, dur):
        """a dark cello pad: detuned saws, slow bow, low-passed (the sequel's floor under every bar)"""
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.1) * SR)
            x = sum(A.saw(A.mtof(m), n, 0.008, 3) for m in ms) / max(1, len(ms))
            return A.lp(x, 900) * A.adsr(n, 0.25, 0.3, 0.85, 0.12)
        return self._cache(('pad', tuple(ms), round(dur, 3)), make)

    def pad_notes(self, ch):
        return tuple(self.T.chord_tones(45, 57, ch)[:3])

    # ---------------------------------------------------------------- one bar
    def bar(self, mix, b, intensity=1.0, light=False):
        """one bar of the trailer bed (bar index b): low-string ostinato, taiko, braam, high tremolo, ticking"""
        T = self.T
        S16 = T.S16
        t0 = b * T.BAR
        ch = T.CHORDS[b]
        r0 = self.root(ch)
        r = r0 + 12 if r0 < 36 else r0
        for st, iv in enumerate(OST):                                     # low strings: spiccato 16ths
            if light and st % 2:
                continue
            acc = 1.0 if st % 4 == 0 else 0.7
            mix.place('strings', self.strings_low(r + iv, S16 * 0.9, acc), t0 + st * S16, 0.11 * (0.8 if light else 1.0))
        # taiko: the trailer pattern, denser as the fight builds
        pat = [(0, 1.0, True), (2, 0.85, False)]
        if intensity > 1.15:
            pat += [(1.5, 0.6, False), (3.5, 0.7, False)]
        if intensity > 1.4 and not light:
            pat += [(2.75, 0.5, False), (3.0, 0.65, False), (3.25, 0.55, False)]
        for bt, v, big in pat:
            mix.place('taiko', self.taiko(v, big, seed=int(bt * 4) + b), t0 + bt * T.BEAT, 0.42 if not light else 0.30)
        if (b - 1) % 4 == 0 and not light:                                # braam on every loop's downbeat
            mix.place('braam', self.braam(r0 - 12 if r0 > 30 else r0, 1.7, 0.8 + 0.2 * min(1.0, intensity - 1)),
                      t0, 0.20)
        if intensity > 1.3 or light:                                      # high string tremolo: the back half's tension
            k = 1.0 if light else min(1.0, (intensity - 1.3) / 0.3)
            mix.place('tremolo', self.tremolo(self.trem(ch), T.BAR + 0.05, 0.3), t0, 0.05 * k)
        if intensity > 1.2 and not light:                                 # ticking 8ths
            for st in range(0, 16, 2):
                mix.place('fx', self.tick(0.6 + 0.4 * (st % 4 == 0), st), t0 + st * S16, 0.04)
        if self.seq:
            self.sequel_bar(mix, b, intensity, light)

    def sequel_bar(self, mix, b, intensity, light):
        """#54: the same bar, darker and bigger"""
        T = self.T
        t0 = b * T.BAR
        ch = T.CHORDS[b]
        r0 = self.root(ch)
        for st in range(0, 16, 2):                                        # a contrabass octave under the 8ths
            acc = 1.0 if st % 4 == 0 else 0.75
            mix.place('strings', self.strings_low(r0, T.S16 * 1.6, acc), t0 + st * T.S16, 0.09 * (0.8 if light else 1.0))
        mix.place('pad', self.pad(self.pad_notes(ch), T.BAR), t0, 0.07 if not light else 0.06)
        if b % 2 == 1 and (b - 1) % 4 != 0 and not light:                 # braams on every second bar (+ #34's)
            mix.place('braam', self.braam(r0 - 12 if r0 > 30 else r0, 1.9, 0.9), t0, 0.16)
        if b % 2 == 1 and not light:                                      # a sub under every braam
            mix.place('braam', self.sub(r0 - 12 if r0 > 30 else r0, 1.9), t0, 0.22)
        if not light:                                                     # the ensemble's big hit on 3 from the start
            mix.place('taiko', self.taiko(0.9, True, seed=300 + b), t0 + 2 * T.BEAT, 0.30)
            if intensity <= 1.4:                                          # (the roll into 4 from the start)
                for j, bt in enumerate((2.75, 3.0, 3.25)):
                    mix.place('taiko', self.taiko(0.45 + 0.1 * j, False, seed=310 + j + b), t0 + bt * T.BEAT, 0.30)
            nb = len(T.LOOP)
            if (b - T.LOOP_START) % nb == nb - 1:                         # a war-drum fill into each loop
                for j in range(8):
                    mix.place('taiko', self.taiko(0.5 + 0.06 * j, False, seed=330 + j), t0 + 3 * T.BEAT + j * T.S16 / 2,
                              0.30)

    # ---------------------------------------------------------------- hits the fighters ask for
    def trailer_hit(self, mix, t, taiko_g, seed, braam_dur=None, braam_g=0.0):
        """a heavy hit: a big taiko (+ a braam on the key's root): #34's flick / rope / stomp hits"""
        mix.place('taiko', self.taiko(1.0, True, seed), t, taiko_g)
        if braam_dur:
            mix.place('braam', self.braam(self.key_root() - 12, braam_dur, 1.0, 0.005), t, braam_g)

    def boom(self, mix, t, g):
        """a slam landing (PEDAL TONE's bell): #34's stomp, a big taiko + a braam"""
        self.trailer_hit(mix, t, 0.7 * g / 0.8, 90, 1.2, 0.30 * g / 0.8)

    def clash(self, mix, t):
        mix.place('fx', self.M.clank(79, int(t * 3)), t, 0.04)

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        """one unison band hit per word: taiko, a braam stab, a boom, low strings"""
        A = self.A
        kr = self.key_root()
        for tw_ in times:
            mix.place('taiko', self.taiko(1.0, True, int(tw_ * 100) + 7), tw_, 0.55)
            mix.place('braam', self.braam(kr - 12, 0.55, 1.0, 0.008), tw_, 0.22)
            mix.place('fx', A.thud(0.9, 120, 32, seed=int(tw_ * 50) + 3), tw_, 0.45)
            mix.place('strings', self.strings_low(kr, 0.4, 1.0), tw_, 0.18)
            if self.seq:
                mix.place('braam', self.sub(kr - 12, 0.8), tw_, 0.22)

    def vs(self, mix, t):
        """a thunder crack where the middle word splits the divider"""
        A, SR = self.A, self.SR
        nn = int(1.4 * SR)
        tt_ = A.tt(nn)
        crack = A.bp(np.random.default_rng(91).normal(0, 1, nn), 1500, 9000) * np.exp(-tt_ / 0.035)
        rumble = A.lp(np.random.default_rng(92).normal(0, 1, nn), 160) * np.clip(tt_ / 0.02, 0, 1) * np.exp(-tt_ / 0.5) * 3.0
        mix.place('fx', crack * 0.9 + rumble, t, 0.40)
        mix.place('fx', A.crash(1.6), t, 0.10)
        if self.seq:                                                      # the sequel: REMATCH lands with a braam
            mix.place('braam', self.braam(self.key_root() - 12, 1.4, 1.0, 0.01), t, 0.26)
            mix.place('braam', self.sub(self.key_root() - 12, 1.4), t, 0.26)

    def title_rest(self, mix):
        """beat 4: the hold + a taiko pickup fill as the doors open; a string tremolo under the whole card"""
        T = self.T
        mix.place('tremolo', self.tremolo(self.trem(T.CHORDS[0]), 3 * T.BEAT + 0.1, 0.05), 0.0, 0.07)
        for st, v in ((12, 0.5), (13, 0.62), (14, 0.74), (15, 0.86)):
            mix.place('taiko', self.taiko(v, False, 40 + st), st * T.S16, 0.36)
        if self.seq:
            mix.place('pad', self.pad(self.pad_notes(T.CHORDS[0]), T.BAR), 0.0, 0.07)

    def count_in(self, mix):
        """bar 1, on the dominant: a taiko + braam stab + low strings per count, the taiko pickup into FIGHT!"""
        T = self.T
        ch = T.CHORDS[1]
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place('taiko', self.taiko(1.0, True, 50 + j), tc, 0.5)
            mix.place('braam', self.braam(self.root(ch), 0.42, 0.9, 0.01), tc, 0.18)
            mix.place('strings', self.strings_low(T.root_above(ch, 45), 0.3), tc, 0.14)
        for st, v in ((12, 0.5), (13, 0.65), (14, 0.8), (15, 0.95)):      # the taiko pickup into FIGHT!
            mix.place('taiko', self.taiko(v, False, 60 + st), T.BAR + st * T.S16, 0.40)
        if self.seq:
            mix.place('pad', self.pad(self.pad_notes(ch), T.BAR), T.BAR, 0.07)

    def fight_start(self, mix):
        T, A = self.T, self.A
        mix.place('braam', self.braam(self.key_root() - 12, 2.2, 1.0), T.T_FIGHT, 0.30)
        mix.place('taiko', self.taiko(1.0, True, 70), T.T_FIGHT, 0.55)
        mix.place('fx', A.crash(1.6), T.T_FIGHT, 0.10)
        if self.seq:
            mix.place('braam', self.sub(self.key_root() - 12, 2.2), T.T_FIGHT, 0.24)

    def fight(self, mix, bar_ko):
        A = self.A
        for b in range(2, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
        mix.place('fx', A.whoosh(3.6, 300, 6000), 44.0 - 3.6, 0.10)
        for k in ('strings', 'taiko', 'braam', 'tremolo', 'pad'):
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], self.M.T_KO, 0.42)

    def ko(self, mix):
        T_KO = self.M.T_KO
        mix.place('taiko', self.taiko(1.0, True, 99), T_KO, 0.6)
        mix.place('braam', self.braam(self.key_root() - 12, 1.6, 1.0, 0.005), T_KO, 0.28)
        mix.place('fx', self.A.thud(0.9, 120, 30), T_KO, 0.7)

    def stinger(self, mix, tw):
        mix.place('braam', self.braam(self.key_root() - 12, 1.4, 0.8), tw, 0.20)
        mix.place('taiko', self.taiko(1.0, True, 101), tw, 0.45)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        T = self.T
        mix.place('fx', self.A.whoosh(t_solo - tw - 0.1, 300, 5000), tw + 0.1, 0.10)
        if self.seq:                                                      # the sequel: no gap before the solo: the
            ch = T.CHORDS[0]                                              # ostinato restarts under the stinger on the
            r0 = self.root(ch)                                            # tonic, a pad and a tremolo hold it
            r = r0 + 12 if r0 < 36 else r0
            n16 = int(round((t_solo - tw) / T.S16))
            for st in range(n16):
                acc = 1.0 if st % 4 == 0 else 0.7
                mix.place('strings', self.strings_low(r + OST[st % 16], T.S16 * 0.9, acc), tw + st * T.S16, 0.11)
            mix.place('pad', self.pad(self.pad_notes(ch), t_solo - tw), tw, 0.08)
            mix.place('tremolo', self.tremolo(self.trem(ch), t_solo - tw + 0.05, 0.2), tw, 0.06)
            for st in range(0, n16, 4):
                mix.place('taiko', self.taiko(0.8, st == 0, seed=500 + st), tw + st * T.S16, 0.36)

    def victory(self, mix, b_solo):
        for b in range(b_solo, self.T.N_BARS - 1):
            if self.seq:                                                  # the sequel: the trailer plays on, full,
                self.bar(mix, b, intensity=1.45)                          # under the winner's solo
            else:
                self.bar(mix, b, light=True)

    def last_bars(self, mix):
        """the last bar builds into the title slam on sample 0: a taiko roll (8ths, then 16ths), a riser, the dominant
        seventh climbing in the low strings, a braam swelling right up to the seam, the tremolo"""
        T, M, A = self.T, self.M, self.A
        t30 = (T.N_BARS - 1) * T.BAR
        ch = T.CHORDS[-1]
        st = 0.0
        while t30 + st < T.DUR - 1e-4:
            v = 0.45 + 0.55 * st / (T.DUR - t30)
            mix.place('taiko', self.taiko(v, False, 200 + int(st * 10)), t30 + st, 0.36)
            st += T.BEAT / 2 if st < 3 * T.BEAT - 1e-6 else T.S16
        mix.place('fx', M.riser(T.DUR - t30), t30, 0.22)
        r = T.root_above(ch, 45)
        pcs = T.CHORD_PCS[ch]
        third = (pcs[1] - pcs[0]) % 12
        for bt in range(4):
            mix.place('strings', self.strings_low((r, r + third, r + 7, r + 10)[bt], T.BEAT * 0.9, 1.0), t30 + bt * T.BEAT,
                      0.16)
        sw = self.braam(self.root(ch), T.DUR - t30 + 0.012, 0.9, T.DUR - t30)     # a braam swelling right up to the seam
        mix.place('braam', sw * np.linspace(0.3, 1.0, len(sw)), t30, 0.16)
        mix.place('tremolo', self.tremolo(self.trem(ch), T.DUR - t30 + 0.012, 1.2), t30, 0.06)
        if self.seq:                                                      # the sequel: a second, higher braam swell,
            for bt in range(4):                                           # big taiko on every beat
                mix.place('taiko', self.taiko(1.0, True, 400 + bt), t30 + bt * T.BEAT, 0.40)
            sw2 = self.braam(self.root(ch) + 12, T.DUR - t30 + 0.012, 0.9, T.DUR - t30)
            mix.place('braam', sw2 * np.linspace(0.2, 1.0, len(sw2)), t30, 0.10)
            mix.place('pad', self.pad(self.pad_notes(ch), T.DUR - t30 + 0.012), t30, 0.08)
