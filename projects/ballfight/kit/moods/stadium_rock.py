"""Mood: minor stadium rock, plugged / unplugged (#44 Electric vs Acoustic). One original minor arena-rock song (huge
drums, a big room) in two arrangements that follow the leader:

    plugged    a fuzz rhythm guitar (dark power chords, palm-muted chugs), a driving pick bass, the stadium kit (snare
               with a big room, hats / ride, crashes, tom fills)
    unplugged  the same song unplugged: a quiet strummed acoustic shimmer up the neck (the bridge's steel-string voice),
               an upright-ish bass walking roots and fifths, hand percussion (cajon slaps, shaker 16ths, tambourine)
    both       the stadium kick (and its claps on 2 and 4 in the big sections) is kept in both

The config names the plugged fighter (PLUGGED, default the left one). While it leads (the series' 5 HP hysteresis) the
band is plugged in; while the other leads it goes unplugged; before anyone leads, on the VS card and in the count-in it
is both at once, and the build into the title slam fades back to both, so frame 0 matches the end. Every lead change
crossfades over one beat, starting on the next 8th; after the K.O. it follows the winner. Both arrangements are matched
to the same K-weighted loudness over the fight before the equal-power crossfade, so the level never moves (consistent
loudness rule). The fighters are the lead and the rhythm (the electric plays the riffs, the acoustic the strums), so the
bed's own guitars stay dark and low. The arrangement thickens with the bars (hats -> 16ths, open chords, toms, crashes),
never with level. Built for E minor (Em C G D / Am B7); any chords work (power chords on their roots)."""
import math

import numpy as np

PLUG = ('p_gtr', 'p_bass', 'p_drums')
UNPLUG = ('u_gtr', 'u_bass', 'u_perc')


class Bed:
    buses = {'kick': (1.6, 0.12), 'p_gtr': (1.3, 0.25), 'p_bass': (1.6, 0.05), 'p_drums': (1.45, 0.35),
             'u_gtr': (1.1, 0.3), 'u_bass': (1.75, 0.08), 'u_perc': (1.6, 0.3)}
    reverb = (2.8, 0.75, 0.24)                                     # the big room
    comp = (0.20, 0.5)
    roar_gain, doors_gain = 0.24, 0.45
    landing = (0.08, 0.06)
    pop = (95, 102, 0.12)
    build_bars = 2

    def __init__(self, M):
        self.M, self.T, self.A, self.SR = M, M.T, M.A, M.SR
        self._c = {}

    # ---------------------------------------------------------------- notes
    def root(self, ch, lo=40):
        return self.T.root_above(ch, lo)

    def _cache(self, key, fn):
        if key not in self._c:
            self._c[key] = fn()
        return self._c[key]

    def pgtr(self, root, dur, mute=False, open_=False):
        """the plugged rhythm guitar: a power chord (root, fifth, octave; + the tenth when open) through a hard fuzz
        and a dark cab (kept under the electric's riff)"""
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.06) * SR)
            t = A.tt(n)
            x = np.zeros(n)
            for m in (root, root + 7, root + 12) + ((root + 19,) if open_ else ()):
                f = A.mtof(m)
                for j, det in enumerate((-0.0035, 0.0035)):
                    ph = np.cumsum(np.full(n, f * (1 + det))) / SR + 0.31 * j
                    x += 2 * (ph % 1.0) - 1
            x = np.tanh(7.0 * A.hp(x / 6.0, 90))
            x = A.lp(A.lp(x, 850 if mute else 2100), 2600)
            env = np.clip(t / 0.004, 0, 1) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.03))
            if mute:
                env = env * np.exp(-t / 0.08)
            return x * env * 0.32
        return self._cache(('pg', root, round(dur, 3), mute, open_), make)

    def pbass(self, m, dur):
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.04) * SR)
            t = A.tt(n)
            f = A.mtof(m)
            ph = np.cumsum(np.full(n, f)) / SR
            x = A.lp(2 * (ph % 1.0) - 1, 650) * 0.7 + np.sin(2 * np.pi * ph) * 0.7
            x = np.tanh(1.6 * x)
            env = np.clip(t / 0.003, 0, 1) * np.where(t < dur, 0.85 + 0.15 * np.exp(-t / 0.05), np.exp(-(t - dur) / 0.02))
            return x * env * 0.5
        return self._cache(('pb', m, round(dur, 3)), make)

    def ubass(self, m, dur):
        """an upright-ish bass: a round plucked tone with a finger thump"""
        A, SR = self.A, self.SR

        def make():
            n = int((dur + 0.15) * SR)
            t = A.tt(n)
            f = A.mtof(m)
            x = np.sin(2 * np.pi * f * t) + 0.45 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.12) \
                + 0.18 * np.sin(6 * np.pi * f * t) * np.exp(-t / 0.06)
            x = A.lp(x, 700) * np.exp(-t / 0.55) * np.clip(t / 0.004, 0, 1)
            thump = A.lp(np.random.default_rng(m).normal(0, 1, n), 300) * np.exp(-t / 0.015) * 0.6
            return (x + thump) * 0.6
        return self._cache(('ub', m, round(dur, 3)), make)

    def kick(self, v=1.0):
        A = self.A
        return self._cache(('k',), lambda: A.hp(A.kick(1.0, 0.30, 140, 44), 35)) * v

    def snare(self, v=1.0):
        A, SR = self.A, self.SR

        def make():
            x = A.snare(0.16)
            n = len(x)
            t = A.tt(n)
            return (x + 0.4 * A.lp(np.random.default_rng(12).normal(0, 1, n), 2500) * np.exp(-t / 0.05)) * 0.9
        return self._cache(('s',), make) * v

    def tom(self, m, v=1.0):
        A, SR = self.A, self.SR

        def make():
            n = int(0.5 * SR)
            t = A.tt(n)
            f0 = A.mtof(m)
            f = f0 * (1 + 0.5 * np.exp(-t / 0.03))
            x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.22)
            return x + 0.3 * A.bp(np.random.default_rng(m).normal(0, 1, n), 200, 3000) * np.exp(-t / 0.01)
        return self._cache(('t', m), make) * v

    def cajon(self, v=1.0):
        A, SR = self.A, self.SR

        def make():
            n = int(0.25 * SR)
            t = A.tt(n)
            slap = A.bp(np.random.default_rng(21).normal(0, 1, n), 500, 4500) * np.exp(-t / 0.035)
            body = np.sin(2 * np.pi * 180 * t) * np.exp(-t / 0.07)
            return slap * 0.8 + body * 0.6
        return self._cache(('cj',), make) * v

    def shaker(self, v=1.0):
        A, SR = self.A, self.SR

        def make():
            n = int(0.09 * SR)
            t = A.tt(n)
            env = np.clip(t / 0.02, 0, 1) * np.exp(-np.maximum(t - 0.02, 0) / 0.025)
            return A.hp(np.random.default_rng(22).normal(0, 1, n), 6000) * env * 0.45
        return self._cache(('sh',), make) * v

    def tamb(self, v=1.0):
        A, SR = self.A, self.SR

        def make():
            n = int(0.3 * SR)
            t = A.tt(n)
            jing = sum(np.sin(2 * np.pi * f * t + f) for f in (5300, 7100, 9150)) * np.exp(-t / 0.07) * 0.15
            return (A.hp(np.random.default_rng(23).normal(0, 1, n), 7000) * np.exp(-t / 0.06) * 0.5 + jing)
        return self._cache(('tb',), make) * v

    def strum(self, ch, up=False, v=1.0, dur=0.22):
        """the unplugged shimmer: a quiet strum of the chord's tones high up the neck (G3-G5), darkened"""
        A, M, T = self.A, self.M, self.T

        def make():
            tones = T.chord_tones(64, 79, ch)[:4] or T.chord_tones(60, 84, ch)[:4]
            ns = tones[::-1] if up else tones
            d = int(0.010 * self.SR)
            parts = [M.note('ACOUSTIC', int(m), dur, 0.5, 0.7) for m in ns]
            out = np.zeros(max(len(p) for p in parts) + d * len(ns))
            for j, p in enumerate(parts):
                out[j * d:j * d + len(p)] += p
            return A.lp(A.hp(out, 250), 3200) / math.sqrt(len(ns))
        return self._cache(('st', ch, up, round(dur, 3)), make) * v

    # ---------------------------------------------------------------- one bar of the song, both arrangements
    def bar(self, mix, b, dense=0.0, light=False):
        T, A = self.T, self.A
        t0 = b * T.BAR
        ch = T.CHORDS[b % T.N_BARS]
        r = self.root(ch)                                          # E2..D#3 (power chord roots)
        rb = r - 12                                                # the bass an octave down
        e8 = T.BEAT / 2
        big = dense > 0.45 and not light
        # ---- both: the stadium kick (1 and 3, a push into 3 in the big sections) + claps when it's big
        for bt in ((0, 2, 2.5) if big else (0, 2)):
            self.place(mix, 'kick', self.kick(1.0 if bt in (0, 2) else 0.7), t0 + bt * T.BEAT, 0.55)
        # ---- plugged: palm-muted chugs -> open power chords, pick bass 8ths, the stadium kit
        for j in range(8):
            te = t0 + j * e8
            if light and j % 2:
                continue
            open_ = dense > 0.3 and j in (0, 3, 6)
            if open_:
                self.place(mix, 'p_gtr', self.pgtr(r, 1.5 * e8 if j != 6 else 2 * e8, open_=dense > 0.7), te, 0.30)
            elif not (dense > 0.3 and j in (1, 4, 7)):
                self.place(mix, 'p_gtr', self.pgtr(r, e8 * 0.9, mute=True), te, 0.30)
            self.place(mix, 'p_bass', self.pbass(rb if j % 4 else rb, e8 * 0.92), te, 0.42)
        for bt in (1, 3):
            self.place(mix, 'p_drums', self.snare(), t0 + bt * T.BEAT, 0.42)
        hat_n = 16 if dense > 0.6 and not light else 8
        for j in range(hat_n):
            acc = 1.0 if j % (hat_n // 4) == 0 else 0.65
            self.place(mix, 'p_drums', A.hat(0.03), t0 + j * T.BAR / hat_n, 0.10 * acc)
        if b % 4 == 2 or (big and b % 2 == 0):                        # crashes at the phrase starts
            self.place(mix, 'p_drums', A.crash(1.8), t0, 0.16)
        if (b % 4 == 1 and dense > 0.2) or (big and b % 8 == 7):    # a tom fill into the next phrase
            for j, m in enumerate((50, 50, 45, 45, 40, 40)):
                self.place(mix, 'p_drums', self.tom(m), t0 + 2.5 * T.BEAT + j * T.S16, 0.26)
        # ---- unplugged: the strum shimmer, the upright bass, hand percussion
        pat = (0, 2, 3, 5, 6, 7) if not light else (0, 3, 6)        # D . D U . U D U  (8ths)
        for j in pat:
            self.place(mix, 'u_gtr', self.strum(ch, up=j in (3, 5, 7)), t0 + j * e8, 0.22 if j in (0, 2, 6) else 0.16)
        walk = (rb, rb + 7, rb + 12, rb + 7)
        for bt in range(4):
            self.place(mix, 'u_bass', self.ubass(walk[bt] if dense > 0.3 else (rb if bt % 2 == 0 else rb + 7), T.BEAT),
                       t0 + bt * T.BEAT, 0.55)
        for bt in (1, 3):
            self.place(mix, 'u_perc', self.cajon(), t0 + bt * T.BEAT, 0.42)
            self.place(mix, 'u_perc', self.tamb(), t0 + bt * T.BEAT, 0.20)
        for j in range(16):
            if light and j % 2:
                continue
            self.place(mix, 'u_perc', self.shaker(1.0 if j % 4 == 0 else 0.7), t0 + j * T.S16, 0.20)
        if big:
            for bt in (1, 3):                                       # claps on 2 and 4 in the big sections (both)
                self.place(mix, 'kick', A.clap(), t0 + bt * T.BEAT, 0.28)

    def place(self, mix, bus, sig, t, g, pan=0.0):
        mix.place(bus, sig, t, g, pan)

    def hit(self, mix, t, ch, g=1.0, ring=0.9):
        """a band hit on a chord: both arrangements (the envelope picks), kick, crash"""
        A, T = self.A, self.T
        r = self.root(ch)
        self.place(mix, 'p_gtr', self.pgtr(r, ring, open_=True), t, 0.42 * g)
        self.place(mix, 'p_bass', self.pbass(r - 12, ring), t, 0.5 * g)
        self.place(mix, 'p_drums', A.crash(1.8), t, 0.20 * g)
        self.place(mix, 'p_drums', self.snare(), t, 0.36 * g)
        self.place(mix, 'u_gtr', self.strum(ch, dur=0.8), t, 0.40 * g)
        self.place(mix, 'u_bass', self.ubass(r - 12, ring), t, 0.7 * g)
        self.place(mix, 'u_perc', self.cajon(), t, 0.5 * g)
        self.place(mix, 'u_perc', self.tamb(), t, 0.3 * g)
        self.place(mix, 'kick', self.kick(), t, 0.7 * g)
        mix.place('fx', A.thud(0.7, 110, 32, seed=int(t * 50) + 3), t, 0.30 * g)

    def boom(self, mix, t, g):
        self.place(mix, 'kick', self.kick(), t, g)

    def clash(self, mix, t):
        mix.place('fx', self.A.thud(0.15, 200, 90, seed=int(t * 3)), t, 0.06)

    # ---------------------------------------------------------------- sections
    def title(self, mix, times):
        """the VS card at full level: the song's groove on the tonic + a band hit under each word"""
        tonic = self.T.CHORDS[0]
        self.bar(mix, 0, dense=1.0)
        for tw_, g in zip(times, (0.85, 0.8, 0.8)):
            self.hit(mix, tw_, tonic, g, 0.4)

    def vs(self, mix, t):
        mix.place('fx', self.M.thunder(1.5, 91), t, 0.55)            # the thunder crack where it splits the divider

    def title_rest(self, mix):
        T = self.T
        for j, m in enumerate((45, 45, 40, 40)):                      # beat 4: a tom fill as the doors open
            self.place(mix, 'p_drums', self.tom(m + 5), 3 * T.BEAT + j * T.S16, 0.26)
            self.place(mix, 'u_perc', self.cajon(0.8 + 0.07 * j), 3 * T.BEAT + j * T.S16, 0.26)

    def count_in(self, mix):
        """bar 1: the band's count (sticks), dominant chugs, a fill into FIGHT!"""
        T, A = self.T, self.A
        ch = T.CHORDS[1]
        r = self.root(ch)
        n = int(0.05 * self.SR)
        stick = A.bp(np.random.default_rng(31).normal(0, 1, n), 2000, 7000) * np.exp(-A.tt(n) / 0.008) \
            + np.sin(2 * np.pi * 2400 * A.tt(n)) * np.exp(-A.tt(n) / 0.01) * 0.5
        self.bar(mix, 1, dense=0.8)                                   # the groove keeps going under the count
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place('fx', stick, tc, 0.30)
            self.place(mix, 'kick', self.kick(), tc, 0.55)
            self.place(mix, 'p_gtr', self.pgtr(r, T.BEAT * 0.5, open_=True), tc, 0.34)
            self.place(mix, 'p_bass', self.pbass(r - 12, T.BEAT * 0.5), tc, 0.45)
            self.place(mix, 'u_gtr', self.strum(ch, dur=0.4), tc, 0.34)
            self.place(mix, 'u_bass', self.ubass(r - 12, T.BEAT), tc, 0.6)
            self.place(mix, 'u_perc', self.cajon(), tc, 0.4)
        for j in range(12, 16):                                       # the fill into FIGHT!
            self.place(mix, 'p_drums', self.tom((52, 50, 45, 40)[j - 12]), T.BAR + j * T.S16, 0.30)
            self.place(mix, 'p_drums', self.snare(0.7 + 0.1 * (j - 12)), T.BAR + j * T.S16, 0.22)
            self.place(mix, 'u_perc', self.cajon(0.7 + 0.1 * (j - 12)), T.BAR + j * T.S16, 0.34)
            self.place(mix, 'kick', self.kick(0.6), T.BAR + j * T.S16, 0.25)
            self.place(mix, 'p_gtr', self.pgtr(r, T.S16 * 0.8, mute=True), T.BAR + j * T.S16, 0.26)

    def fight_start(self, mix):
        T = self.T
        self.hit(mix, T.T_FIGHT, T.CHORDS[T.LOOP_START], 1.0, 1.2)       # the drop

    def fight(self, mix, bar_ko):
        T, M = self.T, self.M
        for b in range(2, bar_ko + 1):
            self.bar(mix, b, dense=min(1.0, (b - 2) / 22.0))
        for t_, who, name in M.P.STORY['caps']:                         # a band hit under every captioned ability
            tq = M.q16(t_)
            self.place(mix, 'p_drums', self.A.crash(1.4), tq, 0.12)
            self.place(mix, 'u_perc', self.tamb(), tq, 0.25)
            self.place(mix, 'kick', self.kick(), tq, 0.4)
        for k in ('kick',) + PLUG + UNPLUG:
            if k in mix.buses:
                mix.buses[k] = self.A.tape_stop(mix.buses[k], M.T_KO, 0.42)

    def ko(self, mix):
        self.hit(mix, self.M.T_KO, self.T.CHORDS[0], 1.0, 0.6)
        mix.place('fx', self.M.thunder(2.2, 51), self.M.T_KO, 0.45)

    def stinger(self, mix, tw):
        self.hit(mix, tw, self.T.CHORDS[0], 1.0, 1.4)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        """the stadium holds the tonic until the solo: a ringing chord, the kick pulsing, cymbal swells (never quiet)"""
        T, A = self.T, self.A
        tonic = T.CHORDS[0]
        r = self.root(tonic)
        for b in range(bar_ko + 1, int(round(t_solo / T.BAR))):
            tq = b * T.BAR
            self.place(mix, 'p_gtr', self.pgtr(r, T.BAR * 0.95, open_=True), tq, 0.30)
            self.place(mix, 'p_bass', self.pbass(r - 12, T.BAR * 0.95), tq, 0.40)
            self.place(mix, 'u_gtr', self.strum(tonic, dur=1.2), tq, 0.34)
            self.place(mix, 'u_bass', self.ubass(r - 12, T.BAR), tq, 0.6)
            for bt in range(4):
                self.place(mix, 'kick', self.kick(0.8), tq + bt * T.BEAT, 0.42)
                self.place(mix, 'u_perc', self.shaker(), tq + bt * T.BEAT + T.BEAT / 2, 0.22)
            sw = A.crash(T.BAR)[::-1] * 0.5
            self.place(mix, 'p_drums', sw, tq, 0.16)

    def victory(self, mix, b_solo):
        T = self.T
        for b in range(b_solo, T.N_BARS - self.build_bars):
            self.bar(mix, b, dense=0.5, light=True)

    def last_bars(self, mix):
        """the build into the title slam: the full song, snare 8ths -> 16ths, toms, a riser right into the seam; then
        the leader envelope over the whole song (both arrangements fade back to 'both' for frame 0)"""
        T, M, A = self.T, self.M, self.A
        for b in range(T.N_BARS - self.build_bars, T.N_BARS):
            self.bar(mix, b, dense=1.0)
        t_last = (T.N_BARS - 1) * T.BAR
        for j in range(16):                                           # the snare roll into the seam
            v = 0.6 + 0.4 * j / 15
            self.place(mix, 'p_drums', self.snare(v), t_last + j * T.S16, 0.20 + 0.12 * j / 15)
            self.place(mix, 'u_perc', self.cajon(v), t_last + j * T.S16, 0.28 + 0.10 * j / 15)
            if j >= 8:
                self.place(mix, 'p_drums', self.tom((52, 50, 47, 45, 43, 40, 38, 36)[j - 8]), t_last + j * T.S16, 0.22)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.20)
        self.lead_envelope(mix)

    # ---------------------------------------------------------------- plugged / unplugged
    def lead_times(self):
        """[(t, leader)] lead changes with the series' 5 HP hysteresis (leader: 0 / 1, the fighter's side)"""
        M = self.M
        hp = [100.0, 100.0]
        lead, out = None, []
        for e in M.EV:
            if e[0] > M.T_KO:
                break
            if e[1] == 'heal':
                hp[e[2]] += e[3]
            elif e[1] == 'hit':
                hp[e[3]] -= e[4]
                d = hp[0] - hp[1]
                new = 0 if d > 5 else (1 if d < -5 else lead)
                if new != lead:
                    out.append((e[0], new))
                    lead = new
        return out

    def plugged_share(self, n):
        """w(t) per sample: 1 = plugged, 0 = unplugged, 0.5 = both"""
        T, M = self.T, self.M
        SR = self.SR
        plug = M.NAMES.index(getattr(T, 'PLUGGED', M.NAMES[0]))
        e8 = T.BEAT / 2
        pts = [(0.0, 0.5), (T.T_FIGHT, 0.5)]
        w = 0.5

        def go(t, to):
            nonlocal w
            ts = max(math.ceil(t / e8 - 1e-6) * e8, pts[-1][0])
            pts.append((ts, w))
            pts.append((ts + T.BEAT, to))
            w = to
        for t, lead in self.lead_times():
            if t >= T.T_FIGHT:
                go(t, 1.0 if lead == plug else 0.0)
        go(M.T_KO + 0.6, 1.0 if M.WIN == plug else 0.0)                 # the winner's arrangement for the victory
        pts.append((T.T_REASSEMBLE, w))                                  # the build fades back to both for frame 0
        pts.append((T.T_GLIDE, 0.5))
        pts.append((T.DUR + 10, 0.5))
        x = np.array([p[0] for p in pts])
        y = np.array([p[1] for p in pts])
        x = np.maximum.accumulate(x)
        return np.interp(np.arange(n) / SR, x, y)

    def lufs(self, x, a, b):
        from scipy.signal import lfilter
        SR = self.SR
        b1, a1 = [1.53512485958697, -2.69169618940638, 1.19839281085285], [1.0, -1.69065929318241, 0.73248077421585]
        b2, a2 = [1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621]
        y = lfilter(b2, a2, lfilter(b1, a1, x[int(a * SR):int(b * SR)], axis=0), axis=0)
        return -0.691 + 10 * np.log10((y ** 2).mean(0).sum() + 1e-12)

    def lead_envelope(self, mix):
        """match the two arrangements' loudness over the fight, then the equal-power crossfade that follows the
        leader (and print the plugged share's story)"""
        T, M = self.T, self.M
        g = {k: self.buses[k][0] for k in PLUG + UNPLUG}
        n = len(mix.buses['p_gtr'])
        sum_p = sum(mix.buses[k] * g[k] for k in PLUG if k in mix.buses)
        sum_u = sum(mix.buses[k] * g[k] for k in UNPLUG if k in mix.buses)
        lp, lu = self.lufs(sum_p, T.T_FIGHT, M.T_KO), self.lufs(sum_u, T.T_FIGHT, M.T_KO)
        k_u = 10 ** ((lp - lu) / 20)
        w = self.plugged_share(n)
        gp = np.sin(w * math.pi / 2)[:, None]
        gu = (np.cos(w * math.pi / 2) * k_u)[:, None]
        for k in PLUG:
            if k in mix.buses:
                mix.buses[k] = mix.buses[k] * gp[:len(mix.buses[k])]
        for k in UNPLUG:
            if k in mix.buses:
                mix.buses[k] = mix.buses[k] * gu[:len(mix.buses[k])]
        self.matched = (lp, lu, k_u)
        ch = [(round(t, 2), 'plugged' if l == M.NAMES.index(getattr(T, 'PLUGGED', M.NAMES[0])) else
               ('unplugged' if l is not None else 'both')) for t, l in self.lead_times() if t >= T.T_FIGHT]
        print(f'stadium_rock: plugged {lp:.1f} LUFS, unplugged {lu:.1f} (x{k_u:.2f} to match); lead changes -> {ch}')
