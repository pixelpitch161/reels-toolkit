"""Mood: horror suspense trap (#55 Bass Clarinet vs Bassoon). A half-time minor trap beat with a haunted nursery on top
and NO woodwinds of its own (the two low reeds are the leads: the bass clarinet growls the bass line, the bassoon plays
the comic staccato line above it):

    808         a long pure sub 808 on each bar's root (C1..B1, under the bass clarinet's growl), sliding into the next root
                at the end of a phrase (the trap slide), a ghost hit on the "and of 3"
    drums       half-time trap kit: a tight kick on 1 and a syncopated pickup, a clap + snare on 3, closed hats on the 8ths
                that skitter into 16th / triplet rolls at phrase ends, an open hat choked on the "and of 4" later
    musicbox    a creepy detuned music box: a 2-bar lullaby ostinato high up (two tines beating ~18 cents apart, a slow
                warped-tape wobble), doubled an octave down in the back half
    strings     creeping low strings: a dark cluster (root + fifth + the minor ninth's rub) swelling over every 2 bars,
                sul tasto; bowed tremolo from the back half
    swell       reversed swells (a reversed cymbal + a reversed music-box tine) rising into the big hits: FIGHT!, every
                loop, the K.O. and the title slam

The arrangement thickens with the bars (hat rolls, the open hat, the octave music box, the string tremolo, the third kick),
never with level: horror, still at the series' loudness from frame 0. Two fighter moves change the bed:
    LIGHTS OUT  (an event 'lights_out' .. 'erupt' in the run): the bed drops to the music box + the 808 for its bar
                (both a little fuller, so the room goes dark without going quiet: the one allowed dip at most), then
                `erupt()` slams the whole kit back with a reversed swell into it
    MOUNTAIN KING (the run's 'mountain' .. 'mountain_end'): the trap beat keeps going under the bassoon's Grieg quote and
                its hats follow the accelerating hops (a hat on every hop), the 808 pumps the root on the beats
The VS card is three full trap hits (808, kick, clap, a music-box chord, a crash); the count-in rolls the hats up; the last
bar rolls the hats from 16ths to 32nds, slides the 808 up to the tonic and swells into the title slam on frame 0. Built
for E minor (Em Em C B | Em Em Am B); any chords work (roots by rule). Original music."""
import numpy as np


class Bed:
    buses = {'808': (0.95, 0.04), 'drums': (1.20, 0.10), 'musicbox': (0.90, 0.50), 'strings': (0.60, 0.40),
             'swell': (0.80, 0.35)}                     # bus: (gain, reverb send)
    reverb = (2.4, 0.6, 0.18)                          # a big, cold room: (dur, decay, wet)
    comp = (0.17, 0.58)                                # master compressor: threshold, exponent
    roar_gain, doors_gain = 0.14, 0.40
    landing = (0.07, 0.05)
    pop = (93, 100, 0.10)

    def __init__(self, M):
        self.M, self.T, self.A, self.SR = M, M.T, M.A, M.SR
        self._c = {}
        EV = M.EV
        self.dark = []                                 # LIGHTS OUT windows: (t0, t_erupt)
        for e in EV:
            if e[1] == 'lights_out' and e[0] < M.T_KO:
                self.dark.append((e[0], min(e[3], M.T_KO)))
        self.mk = []                                   # MOUNTAIN KING: (t0, t1, [hop times])
        for e in EV:
            if e[1] == 'mountain' and e[0] < M.T_KO:
                ends = [q[0] for q in EV if q[1] == 'mountain_end' and q[2] == e[2] and q[0] >= e[0]]
                t1 = min(ends[0] if ends else M.T_KO, M.T_KO)
                hops = [q[0] for q in EV if q[1] == 'hop' and q[2] == e[2] and e[0] <= q[0] <= t1 and q[8] == 2]
                self.mk.append((e[0], t1, hops))

    def _cache(self, key, fn):
        if key not in self._c:
            self._c[key] = fn()
        return self._c[key]

    def in_dark(self, t):
        return any(a - 0.02 <= t < b for a, b in self.dark)

    def in_mk(self, t):
        return any(a <= t < b for a, b, _ in self.mk)

    # ---------------------------------------------------------------- harmony
    def root(self, ch):
        return self.T.root_above(ch, 24)                       # the 808's root, C1..B1: a sub under the growl

    def box_motif(self, ch, ch2):
        """the music box's 2-bar lullaby over chords ch, ch2: 8 notes a bar, high (E6..), the chord's tones with a
        semitone's sigh under each phrase end"""
        T = self.T
        a = T.chord_tones(88, 100, ch) or T.chord_tones(84, 100, ch)
        b = T.chord_tones(88, 100, ch2) or T.chord_tones(84, 100, ch2)
        a = (a * 3)[:4]
        b = (b * 3)[:4]
        bar1 = [a[2], a[1], a[0], a[1], a[2], a[3] if len(set(a)) > 3 else a[2] + 12, a[2], a[2] - 1]
        bar2 = [b[1], b[0], b[1], b[2], b[1], b[0], b[0] - 1, b[0] - 2]
        return bar1, bar2

    # ---------------------------------------------------------------- instruments
    def b808(self, m, dur, to=None, slide=0.09, v=1.0):
        """a sub 808: a sine with a pitch drop at the attack, a long decay, a touch of saturation; `to` slides into that
        note over the last `slide` s (the trap slide)"""
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.06) * SR)
            t = A.tt(n)
            f0 = A.mtof(m)
            semis = 4.0 * np.exp(-t / 0.014)                   # the click-drop into pitch
            if to is not None:
                semis = semis + (to - m) * np.clip((t - (dur - slide)) / slide, 0, 1) ** 1.5
            ph = 2 * np.pi * np.cumsum(f0 * 2 ** (semis / 12)) / SR
            x = np.sin(ph)                                     # a pure sub: the band above it is the bass clarinet's
            env = np.clip(t / 0.003, 0, 1) * np.exp(-t / max(0.25, dur * 0.9)) * \
                np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.02))
            return A.lp(np.tanh(1.1 * x * env) / np.tanh(1.1), 90)
        return self._cache(('808', round(m, 2), round(dur, 3), to, round(slide, 3)), make) * v

    def kick(self, v=1.0):
        A = self.A
        return self._cache('kick', lambda: A.hp(A.kick(1.0, 0.06, 170, 48)[:int(0.12 * self.SR)], 40)) * v

    def clap(self, v=1.0, seed=0):
        """a trap clap layered with a tight snare: three quick noise bursts + a short tail"""
        A, SR = self.A, self.SR

        def make():
            n = int(0.32 * SR)
            t = A.tt(n)
            rng = np.random.default_rng(140 + seed)
            x = np.zeros(n)
            for j, dt in enumerate((0.0, 0.009, 0.019)):
                i0 = int(dt * SR)
                x[i0:] += A.bp(rng.normal(0, 1, n - i0), 900, 5000) * np.exp(-A.tt(n - i0) / (0.006 if j < 2 else 0.09))
            sn = A.hp(A.snare(0.10), 450)[:n]                # (no body: 60-500 Hz is the growl's)
            return x * 0.9 + np.pad(sn, (0, n - len(sn))) * 0.5
        return self._cache(('clap', seed % 2), make) * v

    def hat(self, v=1.0, open_=False, seed=0):
        A = self.A
        return self._cache(('hat', open_, seed % 3),
                           lambda: A.hp(A.hat(0.18 if open_ else 0.022, open_=open_), 7000 + 400 * (seed % 3))) * v

    def tine(self, m, dur=0.9, v=1.0, det=0.18):
        """a music-box tine: a bright pluck with an inharmonic partial, two tines `det` semitones apart (the beating
        detune), a slow warped-tape wobble"""
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.1) * SR)
            t = A.tt(n)
            wob = 0.06 * np.sin(2 * np.pi * 0.7 * t) + 0.03 * np.sin(2 * np.pi * 3.1 * t)
            x = np.zeros(n)
            for d_, g_ in ((0.0, 1.0), (det, 0.7)):
                f = A.mtof(m + d_) * 2 ** (wob / 12)
                ph = 2 * np.pi * np.cumsum(f) / SR
                x += g_ * (np.sin(ph) + 0.30 * np.sin(5.93 * ph) * np.exp(-t / 0.05) + 0.12 * np.sin(2 * ph))
            env = np.clip(t / 0.0015, 0, 1) * np.exp(-t / 0.42) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.05))
            return A.hp(x * env / 1.7, 300)
        return self._cache(('tine', round(m, 2), round(dur, 3), round(det, 3)), make) * v

    def strings(self, ms, dur, trem=False, v=1.0):
        """creeping low strings: detuned saws, dark low-pass, a slow swell (sul tasto); bowed tremolo when `trem`"""
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.25) * SR)
            t = A.tt(n)
            x = sum(A.saw(A.mtof(m), n, det=0.005, voices=3) for m in ms) / max(1, len(ms))
            x = A.lp(x, 700)
            env = np.clip(t / (dur * 0.7), 0, 1) ** 1.4 * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.12))
            if trem:
                env = env * (0.55 + 0.45 * np.abs(np.sin(2 * np.pi * 6.5 * t)))
            return x * env
        return self._cache(('str', tuple(ms), round(dur, 3), trem), make) * v

    def rev_swell(self, dur, m=None, v=1.0):
        """a reversed swell: a reversed crash (+ a reversed music-box tine on `m`), opening up as it rises, cut dead at
        the end (the hit lands after it)"""
        A, SR = self.A, self.SR

        def make():
            n = int(dur * SR)
            c = A.crash(max(1.2, dur + 0.3))[:n]
            c = np.pad(c, (0, n - len(c)))[::-1]
            if m is not None:
                tn = self.tine(m, dur, 1.0, det=0.25)[:n]
                c = c * 0.8 + np.pad(tn, (0, n - len(tn)))[::-1] * 0.6
            t = A.tt(n)
            fade = np.clip((n - np.arange(n)) / (0.004 * SR), 0, 1)
            return c * (t / max(dur, 1e-3)) ** 1.5 * fade
        return self._cache(('rev', round(dur, 3), m), make) * v

    # ---------------------------------------------------------------- one bar
    def bar(self, mix, b, intensity=1.0, light=False):
        """one bar of the trap beat (bar index b); intensity 1.0 .. 1.6 adds parts. Inside LIGHTS OUT only the music box
        and the 808 play; inside MOUNTAIN KING the hats follow the hops"""
        T = self.T
        t0 = b * T.BAR
        ch = T.CHORDS[b % T.N_BARS]
        nxt = T.CHORDS[(b + 1) % T.N_BARS]
        dense = intensity > 1.25 and not light
        e8, s16 = T.BEAT / 2, T.S16
        phrase_end = (b - T.LOOP_START) % 4 == 3
        dark = lambda t: self.in_dark(t)
        r = self.root(ch)
        # ---- the 808: the root on 1 (sliding into the next root at a phrase end), a ghost on the "and of 3"
        slide_to = self.root(nxt) if phrase_end and self.root(nxt) != r else None
        if self.in_mk(t0 + T.BAR / 2):                         # MOUNTAIN KING: pump the root on the beats
            for bt in range(4):
                mix.place('808', self.b808(r, T.BEAT * 0.9), t0 + bt * T.BEAT, 0.48 if bt == 0 else 0.40)
        else:
            g808 = 0.62 if dark(t0) else 0.52
            mix.place('808', self.b808(r, T.BEAT * 2.4, to=None), t0, g808)
            mix.place('808', self.b808(r, T.BEAT * 1.0, to=slide_to, slide=0.12), t0 + 2.5 * T.BEAT,
                      (0.50 if dark(t0 + 2.5 * T.BEAT) else 0.40))
        # ---- drums (half-time): kick 1 + the pickup, clap on 3, hats
        for bt in range(4):
            tb = t0 + bt * T.BEAT
            if bt == 0 and not dark(tb):
                mix.place('drums', self.kick(1.0), tb, 0.55)
            if bt == 2 and not dark(tb):
                mix.place('drums', self.clap(1.0, seed=b), tb, 0.50, pan=0.05)
        if not dark(t0 + 1.75 * T.BEAT):                       # the syncopated pickup kick
            mix.place('drums', self.kick(1.0), t0 + 1.75 * T.BEAT, 0.44)
        if dense and not dark(t0 + 3.5 * T.BEAT):
            mix.place('drums', self.kick(1.0), t0 + 3.5 * T.BEAT, 0.40)
        hats = []
        mk_win = [w for w in self.mk if w[0] < t0 + T.BAR and w[1] > t0]
        for j in range(8):                                     # 8ths
            hats.append((t0 + j * e8, 0.20 if j % 2 == 0 else 0.15))
        if phrase_end or dense:                                # skitter: 16ths over beat 3, a triplet roll into the next
            hats += [(t0 + 2 * T.BEAT + s16, 0.12), (t0 + 2 * T.BEAT + 3 * s16, 0.12)]
            tri = T.BEAT / 3 / 2
            hats += [(t0 + 3 * T.BEAT + j * tri, 0.10 + 0.012 * j) for j in range(6)]
        if intensity > 1.45 and not light:                    # late: 16ths on beat 2 too
            hats += [(t0 + T.BEAT + s16, 0.11), (t0 + T.BEAT + 3 * s16, 0.11)]
        for th, g in hats:
            if dark(th):
                continue
            if any(w[0] <= th < w[1] for w in mk_win):         # MOUNTAIN KING: the hops take over the hats
                continue
            mix.place('drums', self.hat(1.0, seed=int(round((th - t0) / s16))), th, g, pan=0.25)
        for w in mk_win:
            for th in w[2]:
                if t0 <= th < t0 + T.BAR:
                    mix.place('drums', self.hat(1.0, seed=int(th * 37)), th, 0.18, pan=0.25)
        if dense and not dark(t0 + 3.5 * T.BEAT) and not mk_win:
            mix.place('drums', self.hat(1.0, open_=True), t0 + 3.5 * T.BEAT, 0.10, pan=0.25)
        # ---- the music box: the lullaby, two bars per phrase (in the dark: fuller, an octave doubled)
        bar1, bar2 = self.box_motif(T.CHORDS[(b - b % 2) % T.N_BARS], T.CHORDS[(b - b % 2 + 1) % T.N_BARS])
        notes = bar1 if b % 2 == 0 else bar2
        for j, m in enumerate(notes):
            tj = t0 + j * e8
            dk = dark(tj)
            g = (0.24 if dk else 0.17) * (1.0 if j % 2 == 0 else 0.8)
            mix.place('musicbox', self.tine(m, e8 * 1.8), tj, g, pan=-0.3)
            if dk or dense:                                    # the octave under (back half, and in the dark)
                mix.place('musicbox', self.tine(m - 12, e8 * 1.8, det=0.22), tj, g * 0.6, pan=-0.35)
        # ---- creeping low strings, swelling over 2 bars
        if b % 2 == 0 and not (dark(t0) or dark(t0 + T.BAR)):
            sr = T.root_above(ch, 40)
            mix.place('strings', self.strings((sr, sr + 7, sr + 13), 2 * T.BAR * 0.97, trem=dense), t0, 0.42)

    def clash(self, mix, t):
        mix.place('fx', self.hat(1.0, seed=1), t, 0.10)

    def boom(self, mix, t, g=1.0):
        mix.place('808', self.b808(self.root(self.T.chord_at(t)), 0.8), t, 0.5 * g)

    def hit(self, mix, t, ch, g=1.0):
        """a full trap hit: the 808, kick, clap, a music-box chord"""
        mix.place('808', self.b808(self.root(ch), 0.9), t, 0.60 * g)
        mix.place('drums', self.kick(1.0), t, 0.62 * g)
        mix.place('drums', self.clap(1.0), t, 0.40 * g)
        for m in self.T.chord_tones(88, 100, ch)[:3]:
            mix.place('musicbox', self.tine(m, 0.8), t, 0.14 * g, pan=-0.3)

    def erupt(self, mix, t):
        """LIGHTS OUT erupts: the whole kit slams back in, a reversed swell into it (called by the bass clarinet's
        sound at the eruption)"""
        T, A = self.T, self.A
        ch = T.chord_at(t)
        sw = min(1.2, T.BEAT * 2)
        mix.place('swell', self.rev_swell(sw), t - sw, 0.22)
        self.hit(mix, t, ch, 1.1)
        mix.place('drums', A.crash(2.0), t, 0.12)

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        A, ch = self.A, self.T.CHORDS[0]
        for tw_ in times:
            self.hit(mix, tw_, ch, 1.0)
            mix.place('drums', A.crash(1.2), tw_, 0.06)

    def vs(self, mix, t):
        mix.place('fx', self.M.thunder(1.4, seed=91), t, 0.40)

    def title_rest(self, mix):
        """under the card (never quiet): the lullaby ticking, the strings creeping, the hats, the 808 sliding up"""
        T = self.T
        ch = T.CHORDS[0]
        bar1, _ = self.box_motif(ch, T.CHORDS[1])
        for j, m in enumerate(bar1):
            mix.place('musicbox', self.tine(m, T.BEAT * 0.9), j * T.BEAT / 2, 0.15, pan=-0.3)
        for j in range(8):
            mix.place('drums', self.hat(1.0, seed=j), j * T.BEAT / 2 + 0.004, 0.16 if j % 2 == 0 else 0.12, pan=0.25)
        sr = T.root_above(ch, 40)
        mix.place('strings', self.strings((sr, sr + 7, sr + 13), T.BAR * 0.97), 0.0, 0.42)
        mix.place('808', self.b808(self.root(ch), T.BEAT * 0.9, to=self.root(T.CHORDS[1]) + 12, slide=0.2), 3 * T.BEAT,
                  0.42)

    def count_in(self, mix):
        T = self.T
        self.bar(mix, 1, intensity=1.1)
        for tc, txt in T.COUNT[:3]:                            # a clap on every count
            mix.place('drums', self.clap(1.0, seed=1), tc, 0.30)
        tri = T.BEAT / 6                                       # the hats roll up into FIGHT!
        for j in range(6):
            mix.place('drums', self.hat(1.0, seed=j), T.BAR + 3 * T.BEAT + j * tri, 0.10 + 0.02 * j, pan=0.25)
        sw = T.BEAT
        mix.place('swell', self.rev_swell(sw, m=self.T.chord_tones(88, 100, T.CHORDS[2])[0]), T.T_FIGHT - sw, 0.20)

    def fight_start(self, mix):
        mix.place('drums', self.A.crash(1.8), self.T.T_FIGHT, 0.12)
        mix.place('fx', self.A.thud(0.7, 110, 34), self.T.T_FIGHT, 0.30)

    def fight(self, mix, bar_ko):
        T, A, M = self.T, self.A, self.M
        for b in range(2, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.65 * (b / bar_ko))
        for b in range(2 + 4, bar_ko + 1, 4):                  # a reversed swell into every loop
            if not self.in_dark(b * T.BAR):
                sw = T.BEAT * 1.5
                mix.place('swell', self.rev_swell(sw), b * T.BAR - sw, 0.16)
                mix.place('drums', A.crash(1.4), b * T.BAR, 0.06)
        sw = min(1.4, T.BEAT * 2.5)                            # ... and into the K.O.
        mix.place('swell', self.rev_swell(sw, m=self.T.chord_tones(88, 100, T.chord_at(M.T_KO))[0]), M.T_KO - sw, 0.22)
        for k in self.buses:
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], M.T_KO, 0.42)

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
        """the last bar swells into the title slam: hats 16ths -> 32nds, claps on 2 and 4, the 808 sliding up to the tonic,
        the music box spiralling up, a reversed swell + riser right up to the seam"""
        T, M = self.T, self.M
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.chord_at(t_last)
        r = self.root(ch)
        tgt = self.root(T.CHORDS[0])
        mix.place('808', self.b808(r, T.BAR - 0.02, to=tgt + (12 if tgt < r else 0), slide=T.BEAT), t_last, 0.52)
        mix.place('drums', self.kick(1.0), t_last, 0.62)
        mix.place('drums', self.kick(1.0), t_last + 1.75 * T.BEAT, 0.44)
        for bt in (1, 3):
            mix.place('drums', self.clap(1.0, seed=bt), t_last + bt * T.BEAT, 0.42)
        n16 = 8
        for j in range(n16):
            mix.place('drums', self.hat(1.0, seed=j), t_last + j * T.S16, 0.13, pan=0.25)
        s32 = T.S16 / 2
        for j in range(16):
            th = t_last + 2 * T.BEAT + j * s32
            if th < T.DUR - 0.01:
                mix.place('drums', self.hat(1.0, seed=j), th, 0.10 + 0.006 * j, pan=0.25)
        top = self.T.chord_tones(88, 100, ch)
        for j in range(8):
            m = top[j % len(top)] + 12 * (j // len(top))
            mix.place('musicbox', self.tine(min(m, 103), T.BEAT * 0.5), t_last + j * T.BEAT / 2, 0.15, pan=-0.3)
        sr = T.root_above(ch, 40)
        mix.place('strings', self.strings((sr, sr + 7, sr + 13), T.BAR, trem=True), t_last, 0.42)
        sw = T.BAR - 0.02
        mix.place('swell', self.rev_swell(sw), T.DUR - sw, 0.20)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.16)
