"""Mood: big-band mambo (#50 Trumpet vs Trombone). A minor Latin big-band groove with NO trumpets and NO trombones of its
own (the fighters are the brass section: the trumpet takes the high lead, the trombone the low riffs and smears):

    piano       the montuno: a two-bar syncopated figure of the chord in octaves (Pixel Pitch grand), the second bar
                anticipating the next chord on its last 8th; the back half doubles the top an octave up
    bass        the tumbao (Pixel Pitch contrabass, pizz): the fifth on the and of 2, the NEXT chord's root on 4, held
                over the bar line (no downbeat: the mambo's floating bass); the back half adds a ghost on the and of 4
    perc        congas (heel-toe muffs on the 8ths, a slap on 2, the open tones on 4 and the and of 4), timbales (the
                cascara on the shell in the front half, the mambo bell on the quarters in the back half, rimshots and
                an abanico roll into every phrase, a fill INTO every ability the fighters fire), a cowbell, quiet 2-3
                claves
    sax         an alto-sax section pad, kept low (long tones of the chord, mid register: the brass sit above and below)

The arrangement thickens with the bars (mambo bell, conga slaps, octave montuno, bass ghosts), never with level. The VS
card is three full band hits and a rolling montuno over a held sax chord; the count-in keeps the groove (consistent
loudness from frame 0); the last bar walks the bass up chromatically under a timbale roll and a riser into the title
slam's tonic. Built for G minor (Gm Cm D7 Eb7); any chords work (voicings by rule). All parts original."""
import numpy as np

CASCARA = (1, 0, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 0, 1)       # 8ths over two bars (the shell)
CLAVE = (0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0)         # 2-3 son clave, 8ths over two bars
MONTUNO = ((0, 2, 3, 5, 6), (1, 2, 4, 5, 7))                     # the piano's hits (8ths) in bars A and B
CONGA = ('m', 'm', 's', 'm', 'm', 'm', 'o', 'o')                 # muff / slap / open over one bar of 8ths


class Bed:
    buses = {'perc': (1.35, 0.12), 'bass': (1.7, 0.08), 'piano': (1.25, 0.22), 'sax': (0.85, 0.35)}
    reverb = (1.6, 0.5, 0.17)                      # a dance hall: (dur, decay, wet)
    comp = (0.18, 0.55)                            # master compressor: threshold, exponent
    roar_gain, doors_gain = 0.16, 0.40
    landing = (0.07, 0.05)                         # caption landing thuds (first line, others)
    pop = (91, 98, 0.10)                           # the glass pop when the loser is whole again (two chimes, gain)

    def __init__(self, M):
        self.M, self.T, self.A, self.SR = M, M.T, M.A, M.SR
        self._c = {}

    def _cache(self, key, fn):
        if key not in self._c:
            self._c[key] = fn()
        return self._c[key]

    # ---------------------------------------------------------------- harmony
    def root(self, ch):
        return self.T.root_above(ch, 36)                         # the bass's root, C2..B2

    def fifth(self, ch):
        r = self.root(ch)
        pcs = self.T.CHORD_PCS[ch]
        return r + ((pcs[2] - pcs[0]) % 12 if len(pcs) > 2 else 7)

    def tones(self, ch, lo=55):
        """the chord's tones from `lo` up, one octave (root, third, fifth, seventh if any)"""
        return sorted(lo + (pc - lo) % 12 for pc in self.T.CHORD_PCS[ch])

    # ---------------------------------------------------------------- pitched instruments (Pixel Pitch voices)
    def piano(self, m, dur=0.16, vel=0.85):
        return self.M.note('PIANO', int(m), round(dur, 3), round(dur + 0.18, 2), round(vel, 1))

    def octave(self, m, dur=0.16, vel=0.85, top=False):
        """a montuno hit: the note in octaves (and a third octave on top in the back half)"""
        x = self.piano(m, dur, vel) + 0.8 * self.piano(m - 12, dur, vel)
        if top:
            x = x + 0.45 * self._pad(self.piano(m + 12, dur, vel), len(x))
        return x * 0.6

    def chord(self, ch, dur=0.3, vel=0.9):
        """a piano stab: the tones of the chord plus the root an octave down"""
        ts = self.tones(ch, 55)
        x = self.piano(ts[0] - 12, dur, vel)
        for m in ts:
            x = x + self._pad(self.piano(m, dur, vel), len(x))
        return x / (0.6 + 0.4 * len(ts))

    def bass(self, m, dur, vel=0.9):
        return self.M.note('CONTRABASS', int(m), round(min(dur, 0.45), 3), round(dur + 0.12, 2), round(vel, 1))

    def sax(self, m, dur, vel=0.6):
        return self.M.note('SAX', int(m), round(dur, 3), round(dur + 0.2, 2), round(vel, 1))

    def pad(self, ch, dur):
        """the sax section: three tones of the chord around A3-F4, soft attack, a slow fade at the end"""
        A = self.A

        def make():
            ts = self.tones(ch, 57)[:3]
            n = int((dur + 0.2) * self.SR)
            x = np.zeros(n)
            for j, m in enumerate(ts):
                s = self.sax(m, dur, 0.55)[:n]
                x[:len(s)] += s * (1.0 - 0.12 * j)
            t = A.tt(n)
            return A.lp(x / len(ts), 2600) * np.clip(t / 0.09, 0, 1)
        return self._cache(('pad', ch, round(dur, 3)), make)

    def _pad(self, x, n):
        return x[:n] if len(x) >= n else np.pad(x, (0, n - len(x)))

    # ---------------------------------------------------------------- percussion (synthesised)
    def conga(self, kind='o', v=1.0, low=False):
        """a conga: open tone (a ringing skin), slap (a sharp crack), muff (the heel-toe pat); low = the tumba"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.35 * SR)
            t = A.tt(n)
            f = (168 if low else 232) * (1.0 + 0.10 * np.exp(-t / 0.012))
            ph = 2 * np.pi * np.cumsum(f) / SR
            skin = np.sin(ph) + 0.35 * np.sin(1.59 * ph) * np.exp(-t / 0.03)
            rng = np.random.default_rng(300 + (kind == 's') + 2 * low)
            if kind == 'o':
                x = skin * np.exp(-t / 0.16) + 0.25 * A.bp(rng.normal(0, 1, n), 600, 3000) * np.exp(-t / 0.008)
            elif kind == 's':
                x = 0.55 * skin * np.exp(-t / 0.035) + 1.1 * A.bp(rng.normal(0, 1, n), 900, 6000) * np.exp(-t / 0.018)
            else:
                x = 0.6 * skin * np.exp(-t / 0.025) + 0.15 * A.bp(rng.normal(0, 1, n), 400, 2000) * np.exp(-t / 0.006)
            return x * np.clip(t / 0.0015, 0, 1)
        return self._cache(('cg', kind, low), make) * v

    def timbale(self, v=1.0, hi=True, rim=False):
        """a timbale: a bright metal-shell drum, ringing; rim = the rimshot's crack on top"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.5 * SR)
            t = A.tt(n)
            f = 520 if hi else 390
            x = sum(np.sin(2 * np.pi * f * k * t + k) * a for k, a in ((1, 1.0), (1.48, 0.55), (2.13, 0.35), (2.9, 0.2)))
            x = x * np.exp(-t / 0.22) / 2.1
            crack = A.bp(np.random.default_rng(410 + hi).normal(0, 1, n), 1800, 9000) * np.exp(-t / (0.02 if rim else 0.006))
            return (x + crack * (1.0 if rim else 0.35)) * np.clip(t / 0.001, 0, 1)
        return self._cache(('tb', hi, rim), make) * v

    def shell(self, v=1.0):
        """the cascara: a stick on the timbale's metal shell (a dry click with a little ping)"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.08 * SR)
            t = A.tt(n)
            ping = np.sin(2 * np.pi * 1620 * t) * np.exp(-t / 0.012)
            return (0.6 * ping + A.bp(np.random.default_rng(420).normal(0, 1, n), 2500, 8000) * np.exp(-t / 0.006)) * np.clip(t / 0.0008, 0, 1)
        return self._cache(('sh',), make) * v

    def bell(self, v=1.0, mouth=True):
        """the mambo bell (cowbell): two inharmonic partials, the mouth open and ringing or the heel damped"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.3 * SR)
            t = A.tt(n)
            x = np.sign(np.sin(2 * np.pi * 562 * t)) * 0.5 + np.sign(np.sin(2 * np.pi * 838 * t)) * 0.5
            x = A.bp(x, 450, 3200) * np.exp(-t / (0.09 if mouth else 0.03))
            return x * np.clip(t / 0.0008, 0, 1)
        return self._cache(('bl', mouth), make) * v

    def clave(self, v=1.0):
        A, SR = self.A, self.SR

        def make():
            n = int(0.06 * SR)
            t = A.tt(n)
            return np.sin(2 * np.pi * 2480 * t) * np.exp(-t / 0.014) * np.clip(t / 0.0006, 0, 1)
        return self._cache(('cl',), make) * v

    def bombo(self, v=1.0):
        """a soft bass drum for weight under the band hits"""
        A = self.A
        return self._cache(('bo',), lambda: A.hp(A.kick(0.6, 0.24, 120, 50), 40)[:int(0.3 * self.SR)]) * v

    def roll(self, mix, t0, dur, g0=0.10, g1=0.24, seed=0):
        """an abanico: a timbale roll on the 32nds, a crescendo, ending on a rimshot (place it to finish ON the target)"""
        T = self.T
        step = T.S16 / 2
        k = max(1, int(round(dur / step)))
        for j in range(k):
            a = j / max(1, k - 1)
            mix.place('perc', self.timbale(0.8 + 0.2 * (j % 2), hi=(j + seed) % 3 != 2), t0 + j * step, g0 + (g1 - g0) * a,
                      pan=-0.25 if j % 2 else -0.1)
        mix.place('perc', self.timbale(1.0, hi=True, rim=True), t0 + k * step, g1 * 1.05, pan=-0.18)

    # ---------------------------------------------------------------- one bar
    def bar(self, mix, b, intensity=1.0, light=False):
        """one bar of the mambo (bar index b): montuno, tumbao, congas, cascara (front) or mambo bell (back), clave,
        the sax pad; the back half (`intensity` > 1.25) adds the octave-up montuno, bass ghosts and extra slaps"""
        T = self.T
        t0 = b * T.BAR
        e8 = T.BEAT / 2
        ch = T.CHORDS[b % T.N_BARS]
        nxt = T.CHORDS[(b + 1) % T.N_BARS]
        big = intensity > 1.25 and not light
        # the montuno
        side = b % 2
        ts, tn = self.tones(ch, 67), self.tones(nxt, 67)
        for j, e in enumerate(MONTUNO[side]):
            last = side == 1 and e == 7
            src = tn if last else ts
            m = src[(j + 2 * side) % len(src)] if not last else tn[0]
            mix.place('piano', self.octave(m, 0.18 if e in (3, 7) else 0.14, 0.9 if e in (0, 3, 7) else 0.75, top=big),
                      t0 + e * e8, 0.34, pan=0.22)
        # the tumbao
        mix.place('bass', self.bass(self.fifth(ch), 0.9 * T.BEAT, 0.85), t0 + 3 * e8, 0.62)
        mix.place('bass', self.bass(self.root(nxt), 1.9 * T.BEAT, 0.95), t0 + 6 * e8, 0.68)
        if big:
            mix.place('bass', self.bass(self.root(nxt) + 12 if self.root(nxt) + 12 <= 55 else self.root(nxt), 0.3 * T.BEAT, 0.6),
                      t0 + 7 * e8, 0.30)
        # congas
        for e, k in enumerate(CONGA):
            g = {'m': 0.10, 's': 0.22, 'o': 0.26}[k]
            mix.place('perc', self.conga(k, 1.0, low=(k == 'o' and e == 7)), t0 + e * e8, g, pan=0.35)
            if big and e == 5:
                mix.place('perc', self.conga('s', 0.8), t0 + e * e8, 0.14, pan=0.40)
        # timbales: the cascara on the shell (front) / the mambo bell on the quarters (back), the clave
        for e in range(8):
            j = side * 8 + e
            if CASCARA[j] and not big:
                mix.place('perc', self.shell(0.9 + 0.1 * (e % 2 == 0)), t0 + e * e8, 0.10, pan=-0.22)
            if CLAVE[j]:
                mix.place('perc', self.clave(), t0 + e * e8, 0.035, pan=0.05)
        for bt in range(4):
            if big:
                mix.place('perc', self.bell(1.0, mouth=bt % 2 == 0), t0 + bt * T.BEAT, 0.13, pan=-0.28)
            else:
                mix.place('perc', self.bell(0.8, mouth=True), t0 + bt * T.BEAT, 0.07, pan=-0.28)
        if big:
            mix.place('perc', self.timbale(1.0, hi=False, rim=True), t0 + 3 * e8, 0.09, pan=-0.2)
        # the sax pad, one chord a bar, low
        mix.place('sax', self.pad(ch, 0.94 * T.BAR), t0 + 0.01, 0.30, pan=-0.05)
        # an abanico into every phrase
        if b >= T.LOOP_START and (b - T.LOOP_START) % 4 == 3 and not light:
            self.roll(mix, t0 + 3.5 * T.BEAT, 0.5 * T.BEAT - T.S16 / 2, 0.08, 0.22, seed=b)

    def boom(self, mix, t, g):
        """a big drum hit (a slam landing)"""
        mix.place('perc', self.bombo(1.0), t, g)
        mix.place('perc', self.timbale(1.0, hi=False, rim=True), t, 0.5 * g)
        mix.place('bass', self.bass(self.root(self.T.chord_at(t)), 0.5, 1.0), t, 0.45 * g)

    def clash(self, mix, t):
        mix.place('fx', self.conga('s', 1.0), t, 0.06)

    def hit(self, mix, t, ch, g=1.0, ring=0.9):
        """a full band hit: piano stab, bass root, sax chord, timbale rimshot, bombo, a conga slap"""
        mix.place('piano', self.chord(ch, ring, 0.95), t, 0.52 * g, pan=0.22)
        mix.place('bass', self.bass(self.root(ch), ring, 1.0), t, 0.70 * g)
        mix.place('sax', self.pad(ch, ring), t, 0.36 * g, pan=-0.05)
        mix.place('perc', self.timbale(1.0, hi=True, rim=True), t, 0.22 * g, pan=-0.18)
        mix.place('perc', self.bombo(1.0), t, 0.55 * g)
        mix.place('perc', self.conga('s', 1.0), t, 0.20 * g, pan=0.35)

    def ability_fills(self, mix):
        """a timbale fill INTO every ability the fighters fire (the fill's rimshot lands on the ability), each at most
        once per half bar"""
        T, M = self.T, self.M
        last = -9.0
        for e in M.EV:
            t = e[0]
            if t > M.T_KO + 0.01:
                break
            if e[1] != 'ability' or t - last < T.BAR / 2:
                continue
            last = t
            tq = M.q16(t)
            self.roll(mix, tq - 3 * T.S16, 3 * T.S16 - T.S16 / 2, 0.10, 0.24, seed=int(t * 7))
            mix.place('perc', self.bell(1.0), tq, 0.10, pan=-0.28)

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        """a full band hit under each word of the VS card"""
        A, ch = self.A, self.T.CHORDS[0]
        for tw_ in times:
            self.hit(mix, tw_, ch, 1.0, 0.45)
            mix.place('perc', A.crash(1.0), tw_, 0.07)
            mix.place('fx', A.thud(0.8, 120, 34, seed=int(tw_ * 50) + 3), tw_, 0.30)

    def vs(self, mix, t):
        """VS: a thunder crack where it splits the divider"""
        mix.place('fx', self.M.thunder(1.4, seed=91), t, 0.40)

    def title_rest(self, mix):
        """under the card (never quiet): the piano rolling the chord in 8ths, the sax pad held, the bass root, conga
        open tones, the cascara, an abanico on beat 4 as the doors open"""
        T = self.T
        ch = T.CHORDS[0]
        ts = self.tones(ch, 67)
        for e in range(1, 7):
            mix.place('piano', self.octave(ts[e % len(ts)], 0.14, 0.8), e * T.BEAT / 2, 0.30, pan=0.22)
        mix.place('sax', self.pad(ch, 3 * T.BEAT), 0.05, 0.34, pan=-0.05)
        mix.place('bass', self.bass(self.root(ch), 2.0 * T.BEAT, 0.9), 0.05, 0.50)
        for e in range(8):
            if CASCARA[e]:
                mix.place('perc', self.shell(), e * T.BEAT / 2, 0.10, pan=-0.22)
            if e in (6, 7):
                mix.place('perc', self.conga('o', 1.0, low=e == 7), e * T.BEAT / 2, 0.24, pan=0.35)
        self.roll(mix, 3 * T.BEAT, T.BEAT - T.S16 / 2, 0.10, 0.26, seed=7)

    def count_in(self, mix):
        """bar 1: the band keeps the mambo on the count-in chord (the groove from the card on), a band stab on each
        count, a timbale roll and a bass walk-up into FIGHT!"""
        T = self.T
        b = 1
        self.bar(mix, b, intensity=1.4, light=True)
        ch = T.CHORDS[b]
        for tc, txt in T.COUNT[:3]:
            self.hit(mix, tc, ch, 0.55, 0.30)
        t4 = T.BAR + 3 * T.BEAT
        self.roll(mix, t4, T.BEAT - T.S16 / 2, 0.12, 0.28, seed=9)
        mix.place('bass', self.bass(self.root(T.CHORDS[2]) - 2, 0.3 * T.BEAT, 0.85), t4, 0.40)

    def fight_start(self, mix):
        mix.place('perc', self.A.crash(1.8), self.T.T_FIGHT, 0.12)          # the band comes in
        mix.place('fx', self.A.thud(0.7, 110, 34), self.T.T_FIGHT, 0.34)

    def fight(self, mix, bar_ko):
        """the mambo from FIGHT! to the K.O. (denser as it goes), timbale fills into the abilities, a long roll + riser
        into the back half, crashes on the phrase lines, a tape stop at the K.O."""
        T, A = self.T, self.A
        for b in range(2, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
        for b in range(2, bar_ko + 1, 8):
            mix.place('perc', A.crash(1.6), b * T.BAR, 0.08)
        self.ability_fills(mix)
        t_half = (T.LOOP_START + 2 * len(T.LOOP)) * T.BAR
        self.roll(mix, t_half - 2 * T.BEAT, 2 * T.BEAT - T.S16 / 2, 0.08, 0.26, seed=33)
        mix.place('fx', self.M.riser(T.BAR, over=0.0, seed=12) * 0.6, t_half - T.BAR, 0.08)
        mix.place('perc', A.crash(2.0), t_half, 0.12)
        for k in ('perc', 'bass', 'piano', 'sax'):
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], self.M.T_KO, 0.42)

    def ko(self, mix):
        mix.place('perc', self.A.crash(2.4), self.M.T_KO, 0.16)
        mix.place('fx', self.A.thud(0.9, 120, 30), self.M.T_KO, 0.6)

    def stinger(self, mix, tw):
        self.hit(mix, tw, self.T.CHORDS[0], 1.0, 0.8)
        mix.place('perc', self.A.crash(1.8), tw, 0.10)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        mix.place('fx', self.A.whoosh(max(0.2, t_solo - tw - 0.1), 300, 5000), tw + 0.1, 0.08)

    def victory(self, mix, b_solo):
        """the band plays on under the solo at full tilt (a victory, never a quiet stretch), up to the build"""
        T = self.T
        for b in range(b_solo, T.N_BARS - 1):
            self.bar(mix, b, intensity=1.4)

    def last_bars(self, mix):
        """the last bar builds into the title slam on sample 0: a timbale roll right up to the seam, the bass walking
        up chromatically into the tonic, piano stabs and the mambo bell on every beat, conga slaps, a riser"""
        T, M = self.T, self.M
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.CHORDS[-1]
        tgt = self.root(T.CHORDS[0])
        for bt in range(4):
            tb = t_last + bt * T.BEAT
            mix.place('bass', self.bass(tgt - 4 + bt, 0.6 * T.BEAT, 0.9), tb, 0.62)
            mix.place('piano', self.chord(ch, 0.22, 0.9), tb, 0.40, pan=0.22)
            mix.place('perc', self.bell(1.0, mouth=bt % 2 == 0), tb, 0.13, pan=-0.28)
            mix.place('perc', self.conga('s' if bt % 2 else 'o', 1.0), tb + T.BEAT / 2, 0.20, pan=0.35)
            mix.place('perc', self.bombo(0.9), tb, 0.40)
        mix.place('sax', self.pad(ch, T.DUR - t_last), t_last, 0.34, pan=-0.05)
        self.roll(mix, t_last + 2 * T.BEAT, T.DUR - t_last - 2 * T.BEAT - T.S16 / 2 + 0.012, 0.10, 0.30, seed=99)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.18)
