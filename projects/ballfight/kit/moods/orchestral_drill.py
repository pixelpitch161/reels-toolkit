"""Mood: orchestral drill (#42 Viola vs Violin, ported as it was). A UK drill beat (a sliding 808 on drill's syncopated
pattern, skittering hi-hat rolls in 16th triplets and 32nds, the half-time clap + snare on beat 3, rim clicks) under a
dark staccato string bed of CELLOS + BASSES only (the Pixel Pitch cello's voice; the basses a bowed saw an octave
down). No violin or viola of its own: the fighters carry the top. Full from frame 0 (the title card plays the whole
beat); denser as the fight goes (ghost claps, hat rolls, the cellos' octave, the basses). Built for C minor
(Cm Ab Fm G, drill's slow two-bar changes); any chords work: the 808 takes each chord's root in F1-E2, the cellos its
tones in C2-G3."""
import numpy as np

# 808 rhythm per bar (16th, length in 16ths, degree, slide into the next): drill's syncopated pattern
P808 = [(0, 6, 0, False), (6, 4, 0, True), (10, 4, 12, False), (14, 2, 7, True)]
# hi-hats: 16th-grid straight 8ths with rolls (triplet 16ths / 32nds) at fixed spots in the bar
HAT_ROLLS = {3: 'trip', 11: '32', 15: 'trip'}
# the cellos' staccato stabs (16ths): a dark ostinato on chord tones in C2-G3
CELLO_PAT = [(0, 0), (3, 1), (6, 2), (8, 0), (10, 1), (11, 2), (14, 1)]


class Bed:
    buses = {'drums': (1.0, 0.1), 'hats': (1.0, 0.0), 'bass808': (1.0, 0.0), 'cellos': (1.1, 0.35),
             'basses': (1.0, 0.0), 'solo': (1.5, 0.45), 'fx': (1.2, 0.2)}       # bus: (gain, reverb send)
    reverb = (2.2, 0.55, 0.20)                    # a concert-hall arena: (dur, decay, wet)
    comp = (0.15, 0.62)                           # master compressor: threshold, exponent
    roar_gain, doors_gain = 0.14, 0.36
    doors_open_gain, count_whoosh_gain = 0.12, 0.12
    landing = (0.07, 0.05)                        # caption landing thuds (first line, others)
    pop = (95, 100, 0.10)                         # the glass pop when the loser is whole again

    def __init__(self, M):
        self.M, self.T, self.A = M, M.T, M.A
        self.SR = M.SR

    # ---------------------------------------------------------------- instruments (#42)
    def root808(self, ch):
        """the chord's root for the 808 in F1-E2 (#42: C2 Ab1 F1 G1)"""
        from ballfight.kit.episode import parse_chord
        return 29 + (parse_chord(ch)[0] - 5) % 12

    def sub808(self, m0, m1, dur, slide=0.0, vel=1.0):
        """the 808: a sine sub with a punchy pitch drop on the attack, saturated, gliding to m1 over the last `slide` s"""
        A, SR = self.A, self.SR
        n = int((dur + 0.05) * SR)
        t = A.tt(n)
        f0, f1 = A.mtof(m0), A.mtof(m1)
        u = np.clip((t - (dur - slide)) / max(slide, 1e-3), 0, 1) if slide > 0 else np.zeros(n)
        f = f0 * (f1 / f0) ** (u * u * (3 - 2 * u)) * (1 + 0.5 * np.exp(-t / 0.02))
        x = np.sin(2 * np.pi * np.cumsum(f) / SR)
        x = np.tanh(2.2 * x) / np.tanh(2.2)
        env = np.clip(t / 0.004, 0, 1) * np.clip((dur + 0.05 - t) / 0.05, 0, 1) * (0.75 + 0.25 * np.exp(-t / 0.15))
        return A.lp(x, 900) * env * vel

    def hat(self, vel=1.0, open_=False):
        return self.A.hat(0.12 if open_ else 0.03, open_) * vel

    def clap(self, vel=1.0):
        c, s_ = self.A.clap(), self.A.snare(0.16)
        n = max(len(c), len(s_))
        return (np.pad(c, (0, n - len(c))) * 0.8 + np.pad(s_, (0, n - len(s_))) * 0.6) * vel

    def rim(self, vel=1.0, seed=0):
        A = self.A
        n = int(0.06 * self.SR)
        t = A.tt(n)
        x = np.sin(2 * np.pi * 1700 * t) * np.exp(-t / 0.008) + 0.5 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 2500, 7000) * np.exp(-t / 0.006)
        return x * vel

    def cello(self, m, dur, vel=0.85):
        return self.M.note('CELLO', m, round(dur, 3), round(dur + 0.2, 2), round(vel, 1))

    def contrabass(self, m, dur, vel=0.9):
        """the basses: a bowed saw an octave under the cellos, low-passed (the bed's floor between 808 hits)"""
        A = self.A
        n = int((dur + 0.08) * self.SR)
        f = A.mtof(m)
        x = A.saw(f, n, det=0.004, voices=2)
        t = A.tt(n)
        env = np.clip(t / 0.02, 0, 1) * np.clip((dur + 0.08 - t) / 0.08, 0, 1)
        return A.lp(x, 380) * env * vel * 0.6

    @staticmethod
    def addp(*xs):
        n = max(len(x) for x in xs)
        return sum(np.pad(np.asarray(x, float), (0, n - len(x))) for x in xs)

    def bar(self, mix, b, density=1.0, light=False):
        """one bar of the drill beat (#42's bed()): 808 + kick, the half-time clap on beat 3, rim clicks, skittering
        hats, staccato cellos (+ the octave and the basses' long notes in the denser stretches)"""
        T, A, S16 = self.T, self.A, self.T.S16
        t0 = b * T.BAR
        ch = T.CHORDS[b]
        nxt = T.CHORDS[(b + 1) % T.N_BARS]
        root, nroot = self.root808(ch), self.root808(nxt)
        for s16, ln, deg, sl in P808:
            m0 = root + deg
            m1 = nroot if (sl and s16 >= 14) else (root + (12 if s16 == 6 else 0)) if sl else m0
            mix.place('bass808', self.sub808(m0, m1, ln * S16, slide=(0.08 if sl else 0.0), vel=1.0 if s16 == 0 else 0.8),
                      t0 + s16 * S16, 0.62)
        mix.place('drums', A.kick(0.9, 0.18, 140, 55)[:int(0.3 * self.SR)], t0, 0.48)
        mix.place('drums', A.kick(0.6, 0.14, 130, 55)[:int(0.25 * self.SR)], t0 + 10 * S16, 0.30)
        mix.place('drums', self.clap(1.0), t0 + 8 * S16, 0.34)                 # the half-time snare on beat 3
        if density > 1.2 and not light:
            mix.place('drums', self.clap(0.5), t0 + 15 * S16, 0.12)             # a ghost into the next bar
        for s16 in range(0, 16, 2):
            if s16 + 1 in HAT_ROLLS and density > 0.9 and not light:
                continue
            mix.place('hats', self.hat(0.8 if s16 % 4 == 0 else 0.6), t0 + s16 * S16, 0.09, pan=0.25)
        for s16, kind in HAT_ROLLS.items():
            if light or density < 0.9:
                break
            if kind == 'trip':
                for j in range(3):
                    mix.place('hats', self.hat(0.5 + 0.15 * j), t0 + (s16 - 1) * S16 + j * (2 * S16 / 3), 0.08, pan=0.25)
            else:
                for j in range(4):
                    mix.place('hats', self.hat(0.45 + 0.1 * j), t0 + (s16 - 1) * S16 + j * (S16 / 2), 0.07, pan=0.25)
        mix.place('drums', self.rim(0.8, b), t0 + 6 * S16, 0.06, pan=-0.3)
        mix.place('drums', self.rim(0.6, b + 50), t0 + 13 * S16, 0.05, pan=-0.3)
        tones = T.chord_tones(36, 55, ch)
        for s16, j in CELLO_PAT:
            if light and s16 not in (0, 8):
                continue
            m = tones[(j + (1 if s16 >= 8 else 0)) % len(tones)]
            mix.place('cellos', self.cello(m, 0.09 if s16 % 4 else 0.13, 0.85), t0 + s16 * S16, 0.17, pan=-0.2)
            if density > 1.25:                                                 # the octave doubles in the back half
                mix.place('cellos', self.cello(m + 12, 0.08, 0.7), t0 + s16 * S16, 0.09, pan=0.2)
        if density > 1.1 or light:
            mix.place('basses', self.contrabass(root + 12, T.BAR - 0.06, 0.8), t0, 0.12)

    def boom(self, mix, t, g):
        mix.place('bass808', self.sub808(24, 24, 0.9), t, g)

    def clash(self, mix, t):
        mix.place('fx', self.A.chime(91, 0.25), t, 0.04)

    def tonic_hit(self, dur):
        """the cellos' root + fifth on the tonic (#42: C2 + G2)"""
        r = self.T.KEY_ROOT
        return self.addp(self.cello(r, dur, 1.0), self.cello(r + 7, dur, 1.0) * 0.8)

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        """the drill beat full from frame 0 (the levels rule) + a unison band hit under each word"""
        A = self.A
        self.bar(mix, 0, density=1.0)
        for j, tw_ in enumerate(times):
            mix.place('drums', A.kick(1.0, 0.25, 150, 48)[:int(0.4 * self.SR)], tw_, 0.50)
            mix.place('drums', A.crash(1.0), tw_, 0.05)
            mix.place('cellos', self.tonic_hit(0.3), tw_, 0.24)
            mix.place('fx', A.thud(0.8, 120, 34, seed=int(tw_ * 50) + 3), tw_, 0.26)

    def vs(self, mix, t):
        """VS: a thunder crack where it splits the divider"""
        A, SR = self.A, self.SR
        nn = int(1.4 * SR)
        tt_ = A.tt(nn)
        crack = A.bp(np.random.default_rng(91).normal(0, 1, nn), 1500, 9000) * np.exp(-tt_ / 0.035)
        rumble = A.lp(np.random.default_rng(92).normal(0, 1, nn), 160) * np.clip(tt_ / 0.02, 0, 1) * np.exp(-tt_ / 0.5) * 3.0
        mix.place('fx', crack * 0.9 + rumble, t, 0.32)

    def title_rest(self, mix):
        pass

    def count_in(self, mix):
        """the count-in bar on its chord (the dominant): the beat, a kick + a cello note per count, a hat roll into the
        drop"""
        T, A = self.T, self.A
        cb = T.FIGHT_BAR - 1
        self.bar(mix, cb, density=1.0)
        cs = T.chord_tones(43, 54, T.CHORDS[cb])[:3]
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place('drums', A.kick(0.9, 0.2, 150, 50)[:int(0.35 * self.SR)], tc, 0.40)
            mix.place('cellos', self.cello(cs[j % len(cs)], 0.18, 1.0), tc, 0.26)
        for j in range(8):
            mix.place('hats', self.hat(0.5 + 0.06 * j), T.T_PICKUP + j * T.S16 / 2, 0.10, pan=0.25)

    def fight_start(self, mix):
        mix.place('drums', self.A.crash(1.8), self.T.T_FIGHT, 0.10)               # the drop
        mix.place('fx', self.A.thud(0.7, 110, 34), self.T.T_FIGHT, 0.30)

    def fight(self, mix, bar_ko):
        """the drill beat from FIGHT! to the K.O. (denser as it goes), a crash every 8-bar phrase, a riser + crash into
        the back half, a tape stop at the K.O."""
        T, A = self.T, self.A
        for b in range(T.FIGHT_BAR, bar_ko + 1):
            self.bar(mix, b, density=1.0 + 0.6 * (b / bar_ko))
        for b in range(T.FIGHT_BAR, bar_ko + 1, 8):
            mix.place('drums', A.crash(1.6), b * T.BAR, 0.07)
        t_half = (T.LOOP_START + 2 * len(T.LOOP)) * T.BAR               # the back half (bar 18 in #42)
        mix.place('fx', self.M.riser(T.BAR, over=0.0, seed=12) * 0.6, t_half - T.BAR, 0.07)
        mix.place('drums', A.crash(2.0), t_half, 0.10)
        for k in ('drums', 'hats', 'bass808', 'cellos', 'basses'):
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], self.M.T_KO, 0.42)

    def ko(self, mix):
        mix.place('drums', self.A.crash(2.4), self.M.T_KO, 0.14)
        mix.place('fx', self.A.thud(0.9, 120, 30), self.M.T_KO, 0.55)

    def stinger(self, mix, tw):
        mix.place('cellos', self.tonic_hit(0.5), tw, 0.24)
        mix.place('bass808', self.sub808(self.T.KEY_ROOT, self.T.KEY_ROOT, 0.8), tw, 0.6)
        mix.place('drums', self.A.crash(1.8), tw, 0.09)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        mix.place('fx', self.A.whoosh(max(0.2, t_solo - tw - 0.1), 300, 5000), tw + 0.1, 0.08)

    def victory(self, mix, b_solo):
        """a lighter drill beat under the solo, up to the build"""
        for b in range(b_solo, self.T.N_BARS - 2):
            self.bar(mix, b, density=1.0, light=True)

    def last_bars(self, mix):
        """the beat comes back and builds into the slam on sample 0: the cellos climbing the last chord, a hat roll
        speeding into the seam, a riser"""
        T, M = self.T, self.M
        self.bar(mix, T.N_BARS - 2, density=1.3)
        t_last = (T.N_BARS - 1) * T.BAR
        self.bar(mix, T.N_BARS - 1, density=1.5)
        cs = T.chord_tones(43, 55, T.CHORDS[-1])[:4]
        for bt in range(4):
            mix.place('cellos', self.cello(cs[bt % len(cs)], 0.3, 1.0), t_last + bt * T.BEAT, 0.26)
        for j in range(16):
            mix.place('hats', self.hat(0.4 + 0.035 * j), t_last + 2 * T.BEAT + j * T.S16 / 2, 0.10, pan=0.25)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.16)
