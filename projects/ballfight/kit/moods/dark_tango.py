"""Mood: dark tango (#47 Cello vs Contrabass). A dramatic minor tango orquesta with NO piano and NO bass of its own (the
fighters are the leads: the double bass's footfalls are the walking bass line, the cello sings the melody):

    bandoneon   the marcato: a chord stab on every beat (two reeds beating, a nasal reed bark), accents on 1 and 3;
                in the back half the sincopa (3-3-2: 1, the and of 2, 4) on alternate bars and an arrastre (a dragged
                slide into the downbeat) before every phrase; the card's held chord shakes the bellows
    guitar      the strings' chop, played by a muted nylon guitar: a choked strum on 2 and 4 (ghost strokes on 1 and 3
                later on)
    perc        no drum kit: the golpe (a knuckle on the bass's body) on 1 and 3, the chicharra (a bow scraped below
                the bridge) on the off-beats in the back half, a latigo (a whip glissando) on the phrase lines

Tango stops: on every stop a fighter asks for (Sound.stops(): the cello's rope slams) the whole band hits that beat
and cuts until the next one, so the fighter's own crash rings alone. The arrangement thickens with the bars (sincopa,
chicharra, ghost chops, arrastres), never with level (consistent loudness from frame 0). The VS card is three full band
hits over the shaking bellows; the count-in keeps the marcato; the last bar runs the bandoneon up chromatically under a
riser into the title slam's tonic. Built for D minor (Dm A7 Gm E7); any chords work (voicings by rule)."""
import math

import numpy as np
from scipy.signal import lfilter


class Bed:
    buses = {'bandoneon': (1.55, 0.26), 'guitar': (1.25, 0.20), 'perc': (1.3, 0.12)}   # bus: (gain, reverb send)
    reverb = (1.6, 0.5, 0.17)                      # a milonga hall: (dur, decay, wet)
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
    def root(self, ch, lo=38):
        return self.T.root_above(ch, lo)

    def voicing(self, ch, lo=62):
        """the bandoneon's right-hand chord: the chord's tones from D4 up (a 7th chord's seventh on top), four notes"""
        pcs = self.T.CHORD_PCS[ch]
        out = []
        for pc in pcs:
            m = lo + (pc - lo) % 12
            while m in out:
                m += 12
            out.append(m)
        if len(out) < 4:
            out.append(min(out) + 12)
        return tuple(sorted(out)[:4])

    # ---------------------------------------------------------------- instruments
    def reed(self, m, dur, vel=1.0, att=0.006, bark=1.0):
        """one bandoneon note: two free reeds a few cents apart (the beating), a nasal formant, a bellows envelope"""
        A, SR = self.A, self.SR

        def make():
            f = A.mtof(m)
            n = int((dur + 0.08) * SR)
            t = A.tt(n)
            x = np.zeros(n)
            for det, ph in ((-3.5, 0.0), (3.5, 1.3)):
                ff = f * 2 ** (det / 1200)
                for k in range(1, 40):
                    fk = ff * k
                    if fk > 7000:
                        break
                    g = 1.0 / k ** 0.75 * (1 + 1.4 * math.exp(-((math.log(fk / 1250.0)) / 0.35) ** 2)
                                           + 0.7 * math.exp(-((math.log(fk / 2600.0)) / 0.3) ** 2))
                    x += g * np.sin(2 * np.pi * fk * t + ph * k)
            x /= np.abs(x).max() + 1e-9
            env = np.clip(t / att, 0, 1) * np.where(t < dur, 0.72 + 0.28 * np.exp(-t / 0.05) * bark, 1.0)
            env *= np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.025))
            return A.lp(x * env, 6000)
        return self._cache(('reed', int(m), round(dur, 3), round(att, 3), bark), make) * vel

    def stab(self, ch, dur=0.16, vel=1.0, low=False):
        """a marcato chord stab (the left hand's low octave under it when `low`)"""
        def make():
            vo = self.voicing(ch)
            x = sum(self.reed(m, dur, 1.0) for m in vo) / len(vo)
            if low:
                r = self.root(ch, 45)
                lo = self.reed(r, dur, 1.0, bark=0.6) * 0.8
                n = max(len(x), len(lo))
                x = np.pad(x, (0, n - len(x))) + np.pad(lo, (0, n - len(lo)))
            return x
        return self._cache(('stab', ch, round(dur, 3), low), make) * vel

    def bellows_shake(self, ch, dur, vel=1.0):
        """the held chord with the bellows shaken (a tremolo at ~7 Hz)"""
        def make():
            x = self.stab(ch, dur, 1.0, low=True)
            t = self.A.tt(len(x))
            return x * (0.75 + 0.25 * np.sin(2 * np.pi * 7.2 * t))
        return self._cache(('shake', ch, round(dur, 3)), make) * vel

    def pluck(self, m, dur, seed=0, bright=0.35, ring=0.992):
        """a nylon string (Karplus-Strong)"""
        A, SR = self.A, self.SR

        def make():
            f = A.mtof(m)
            N = max(2, int(round(SR / f)))
            n = int(dur * SR)
            exc = np.zeros(n)
            burst = np.random.default_rng(2000 + int(m) + 37 * seed).normal(0, 1, N)
            exc[:N] = A.lp(burst, 1200 + 4000 * bright)
            a = np.zeros(N + 2)
            a[0], a[N], a[N + 1] = 1.0, -0.5 * ring, -0.5 * ring
            return lfilter([1.0], a, exc)
        return self._cache(('pl', int(m), round(dur, 3), seed, bright, ring), make)

    def chop(self, ch, v=1.0, ghost=False, dur=0.12):
        """the strings' chop on a muted nylon guitar: a fast down-strum choked at once (a ghost: mostly the scratch)"""
        A, SR = self.A, self.SR

        def make():
            pcs = self.T.CHORD_PCS[ch]
            vo = sorted(set(50 + (pc - 50) % 12 for pc in pcs))[:4]
            d = int(0.005 * SR)
            n = int((dur + 0.05) * SR) + d * len(vo)
            x = np.zeros(n)
            for j, m in enumerate(vo):
                s = self.pluck(m, dur + 0.05, seed=j, bright=0.25 if ghost else 0.4)
                x[j * d:j * d + len(s)] += s
            t = A.tt(n)
            cut = 0.04 if ghost else dur
            env = np.clip(t / 0.002, 0, 1) * np.where(t < cut, np.exp(-t / 0.07), np.exp(-cut / 0.07) * np.exp(-(t - cut) / 0.012))
            scratch = A.bp(np.random.default_rng(78).normal(0, 1, n), 1500, 6000) * np.exp(-t / 0.01) * (0.9 if ghost else 0.4)
            return (A.lp(x / max(1, len(vo)), 3200) * env + scratch) * (0.45 if ghost else 1.0)
        return self._cache(('ch', ch, ghost, round(dur, 3)), make) * v

    def golpe(self, v=1.0, seed=0):
        """a knuckle on the bass's body: a wooden thump"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.18 * SR)
            t = A.tt(n)
            body = np.sin(2 * np.pi * (150 + 60 * np.exp(-t / 0.01)) * t) * np.exp(-t / 0.045)
            knock = A.bp(np.random.default_rng(90 + seed).normal(0, 1, n), 350, 1400) * np.exp(-t / 0.012)
            return A.hp(body * 0.9 + knock * 0.8, 110)
        return self._cache(('golpe', seed), make) * v

    def chicharra(self, v=1.0, seed=0):
        """the chicharra: a bow scraped below the bridge (a pitched rasp, ~42 Hz grain)"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.13 * SR)
            t = A.tt(n)
            grain = (np.sin(2 * np.pi * 42 * t) > 0.2).astype(float)
            x = A.bp(np.random.default_rng(120 + seed).normal(0, 1, n), 1800, 6500) * grain
            return x * np.clip(t / 0.008, 0, 1) * np.exp(-t / 0.06)
        return self._cache(('chi', seed), make) * v

    def latigo(self, v=1.0):
        """the latigo: a whip glissando up the strings"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.16 * SR)
            t = A.tt(n)
            f = 500 * (6.0 ** np.clip(t / 0.12, 0, 1))
            ph = 2 * np.pi * np.cumsum(f) / SR
            saw = 2 * ((ph / (2 * np.pi)) % 1) - 1
            return A.lp(saw, 5000) * np.clip(t / 0.01, 0, 1) * np.exp(-np.maximum(t - 0.1, 0) / 0.02) * 0.6
        return self._cache(('lat',), make) * v

    # ---------------------------------------------------------------- one bar
    def bar(self, mix, b, intensity=1.0, light=False):
        """one bar of the marcato (bar index b): bandoneon stabs on every beat (1 and 3 accented), the guitar's chop on
        2 and 4, the golpe on 1 and 3; the back half (`intensity` > 1.25) plays the sincopa on odd bars, adds the
        chicharra on the off-beats, ghost chops and an arrastre into every phrase"""
        T = self.T
        t0 = b * T.BAR
        ch = T.CHORDS[b % T.N_BARS]
        big = intensity > 1.25 and not light
        sincopa = big and b % 2 == 1
        hits = ((0, 1.0), (1.5, 0.75), (3, 0.85)) if sincopa else ((0, 1.0), (1, 0.62), (2, 0.9), (3, 0.62))
        for bt, v in hits:
            mix.place('bandoneon', self.stab(ch, 0.15 if v < 0.8 else 0.19, 1.0, low=(bt == 0)), t0 + bt * T.BEAT,
                      0.34 * v, pan=-0.12)
        for bt in range(4):
            tb = t0 + bt * T.BEAT
            if bt in (0, 2):
                mix.place('perc', self.golpe(1.0, b * 4 + bt), tb, 0.30 if not light else 0.24)
                if big:
                    mix.place('guitar', self.chop(ch, ghost=True), tb, 0.28, pan=0.2)
            else:
                mix.place('guitar', self.chop(ch, 1.0), tb, 0.44, pan=0.2)
            if big and bt in (1, 3):
                mix.place('perc', self.chicharra(1.0, b + bt), tb + T.BEAT / 2, 0.11, pan=0.3)
        if b >= T.LOOP_START and (b - T.LOOP_START) % 4 == 3 and not light:   # the arrastre into the phrase
            nxt = T.CHORDS[(b + 1) % T.N_BARS]
            for j, m in enumerate(self.voicing(nxt)):
                mix.place('bandoneon', self.A.lp(self.reed(m - 1, T.BEAT * 0.45, 1.0, att=0.08), 4000),
                          t0 + 3.5 * T.BEAT, 0.07, pan=-0.12)

    def hit(self, mix, t, ch, g=1.0, ring=0.6):
        """a full band hit: the bandoneon's chord with its low octave, the guitar, the golpe"""
        mix.place('bandoneon', self.stab(ch, ring, 1.0, low=True), t, 0.46 * g, pan=-0.12)
        mix.place('guitar', self.chop(ch, 1.0, dur=0.2), t, 0.5 * g, pan=0.2)
        mix.place('perc', self.golpe(1.0, int(t * 10)), t, 0.42 * g)

    def boom(self, mix, t, g):
        self.hit(mix, t, self.T.chord_at(t), 0.8 * g, 0.4)
        mix.place('fx', self.A.thud(0.6, 110, 32, seed=int(t * 7)), t, 0.4 * g)

    def clash(self, mix, t):
        mix.place('perc', self.golpe(0.8, int(t * 31) % 50), t, 0.10)

    # ---------------------------------------------------------------- the tango stops
    def stop_times(self):
        out = []
        for s in getattr(self.M, 'SOUNDS', []):
            fn = getattr(s, 'stops', None)
            if fn is not None:
                out += list(fn())
        return sorted(t for t in out if self.T.T_FIGHT <= t < self.M.T_KO)

    def apply_stops(self, mix, stops):
        """each stop: the band hits on it, then cuts until the next beat (short fades: no clicks)"""
        T, SR = self.T, self.SR
        for ts in stops:
            self.hit(mix, ts, T.chord_at(ts), 1.0, 0.18)
        for k in self.buses:
            if k not in mix.buses:
                continue
            x = mix.buses[k]
            n = len(x)
            g = np.ones(n)
            for ts in stops:
                a = int((ts + 0.16) * SR)
                nb = math.ceil(ts / T.BEAT + 1e-6) * T.BEAT
                b = int((nb - 0.012) * SR)
                if b - a < int(0.05 * SR):
                    continue
                f = int(0.012 * SR)
                g[a:a + f] = np.minimum(g[a:a + f], np.linspace(1, 0, f))
                g[a + f:b - f] = 0.0
                g[b - f:b] = np.minimum(g[b - f:b], np.linspace(0, 1, f))
            mix.buses[k] = x * (g[:, None] if x.ndim == 2 else g)

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        ch = self.T.CHORDS[0]
        for tw_ in times:
            self.hit(mix, tw_, ch, 1.0, 0.4)
            mix.place('fx', self.A.thud(0.8, 120, 34, seed=int(tw_ * 50) + 3), tw_, 0.30)
            mix.place('perc', self.latigo(1.0), tw_ - 0.09, 0.10)

    def vs(self, mix, t):
        mix.place('fx', self.M.thunder(1.4, seed=91), t, 0.40)

    def title_rest(self, mix):
        """the held chord under the card (never quiet): the bellows shaking, the golpe on each beat, a latigo and an
        arrastre on beat 4 as the doors open"""
        T = self.T
        ch = T.CHORDS[0]
        mix.place('bandoneon', self.bellows_shake(ch, 3 * T.BEAT - 0.1), 0.08, 0.36, pan=-0.12)
        for bt in range(1, 4):
            mix.place('perc', self.golpe(0.9, bt), bt * T.BEAT, 0.26)
        mix.place('perc', self.latigo(1.0), 3.5 * T.BEAT, 0.10)
        for m in self.voicing(T.CHORDS[1]):
            mix.place('bandoneon', self.reed(m - 1, T.BEAT * 0.45, 1.0, att=0.08), 3.5 * T.BEAT, 0.08, pan=-0.12)

    def count_in(self, mix):
        """bar 1: the marcato on the count-in chord (the groove from the card on), a band hit on each count"""
        T = self.T
        self.bar(mix, 1, intensity=1.0)
        ch = T.CHORDS[1]
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            self.hit(mix, tc, ch, 0.5, 0.2)
        mix.place('perc', self.latigo(1.0), T.T_FIGHT - 0.1, 0.12)

    def fight_start(self, mix):
        self.hit(mix, self.T.T_FIGHT, self.T.CHORDS[2], 0.9, 0.3)
        mix.place('fx', self.A.thud(0.7, 110, 34), self.T.T_FIGHT, 0.34)

    def fight(self, mix, bar_ko):
        """the marcato from FIGHT! to the K.O. (denser as it goes), latigos on the phrase lines, the stops, a tape
        stop at the K.O."""
        T, A = self.T, self.A
        for b in range(2, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.6 * (b / max(1, bar_ko)))
        for b in range(2, bar_ko + 1, 4):
            mix.place('perc', self.latigo(1.0), b * T.BAR - 0.1, 0.10)
        t_half = (T.LOOP_START + len(T.LOOP)) * T.BAR
        mix.place('fx', self.M.riser(T.BAR, over=0.0, seed=12) * 0.6, t_half - T.BAR, 0.07)
        self.apply_stops(mix, self.stop_times())
        for k in self.buses:
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], self.M.T_KO, 0.42)

    def ko(self, mix):
        self.hit(mix, self.M.T_KO, self.T.chord_at(self.M.T_KO), 1.0, 0.5)
        mix.place('fx', self.A.thud(0.9, 120, 30), self.M.T_KO, 0.6)

    def stinger(self, mix, tw):
        self.hit(mix, tw, self.T.CHORDS[0], 1.0, 0.7)
        mix.place('perc', self.latigo(1.0), tw - 0.1, 0.12)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        mix.place('fx', self.A.whoosh(max(0.2, t_solo - tw - 0.1), 300, 5000), tw + 0.1, 0.08)
        b0 = int(math.ceil((tw + 0.5) / self.T.BAR))
        for b in range(b0, int(round(t_solo / self.T.BAR))):
            self.bar(mix, b, intensity=1.0, light=True)

    def theme(self):
        """the tango's theme: the line of the fighter whose role is 'melody' (config LINES), or None"""
        for n in self.T.FIGHTERS:
            if self.T.ROLES.get(n) == 'melody' and n in self.T.LINES:
                return n
        return None

    def victory(self, mix, b_solo):
        """the band plays on under the solo at full tilt (a victory, never a quiet stretch), up to the build: the
        bandoneon takes up the theme an octave up (legato, under the solo), its left hand on the roots, the chop on
        every 8th in the second half of each bar"""
        T = self.T
        th = self.theme()
        for b in range(b_solo, T.N_BARS - 1):
            self.bar(mix, b, intensity=1.4)
            ch = T.CHORDS[b % T.N_BARS]
            for bt in (0, 2):
                mix.place('bandoneon', self.reed(self.root(ch, 38) + (0 if bt == 0 else 7), 0.5 * T.BEAT, 1.0, bark=0.5),
                          b * T.BAR + bt * T.BEAT, 0.16, pan=-0.12)
            for bt in (2.5, 3.5):
                mix.place('guitar', self.chop(ch, 0.8), b * T.BAR + bt * T.BEAT, 0.30, pan=0.2)
            if th is None:
                continue
            for bt in range(4):
                tb = b * T.BAR + bt * T.BEAT
                m = T.line_at(th, tb)
                while m < 62:
                    m += 12
                while m > 81:
                    m -= 12
                g = 0.13 if b < T.N_BARS - 2 else 0.22                 # fuller as the loser's pieces fly home
                mix.place('bandoneon', self.reed(m, T.BEAT * 0.95, 1.0, att=0.03, bark=0.4), tb, g, pan=0.1)
                if b == T.N_BARS - 2:
                    mix.place('bandoneon', self.reed(m - 12, T.BEAT * 0.95, 1.0, att=0.03, bark=0.4), tb, 0.12, pan=-0.1)

    def last_bars(self, mix):
        """the last bar builds into the title slam on sample 0: stabs on every 8th, the bandoneon's top line climbing
        chromatically into the tonic, golpes on every beat, a riser right up to the seam"""
        T, M = self.T, self.M
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.CHORDS[-1]
        for j in range(8):
            tj = t_last + j * T.BEAT / 2
            mix.place('bandoneon', self.stab(ch, 0.14, 1.0, low=True), tj, 0.34 + 0.02 * j, pan=-0.12)
            mix.place('guitar', self.chop(ch, 1.0), tj, 0.40, pan=0.2)
            mix.place('perc', self.golpe(1.0, 300 + j), tj, 0.30 if j % 2 == 0 else 0.18)
            if j % 2 == 1:
                mix.place('perc', self.chicharra(1.0, 400 + j), tj, 0.12, pan=0.3)
        top = max(self.voicing(T.CHORDS[0])) + 12
        for j in range(4):
            mix.place('bandoneon', self.reed(top - 4 + j, T.BEAT * 0.9, 1.0, att=0.02), t_last + j * T.BEAT, 0.20,
                      pan=-0.12)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.18)
