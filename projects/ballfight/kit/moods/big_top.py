"""Mood: sinister big top (#41 Flute vs Trombone, ported as it was). A dark circus with NO brass and NO flute of its
own: a band organ / calliope (breathy, slightly out-of-tune steam pipes) chunking the chords on the off-beats, a concert
bass drum on the "oom" (beats 1 + 3), a choked cymbal on 2 + 4, snare rolls into the sections, crashes on the phrase
lines; the back half adds the march snare's 8ths, a tambourine, a held band-organ chord and a glockenspiel glint. Leaves
the bass register to a fighter (the trombone in #41) and the top to the other (the flute). Built for E harmonic minor
(Em Am B7 C); other chords are voiced by the same rule (the tables are what the rule gives for #41's chords)."""
import numpy as np

# the calliope's off-beat chords (E4-C5: under the melody) and the back half's held organ chord, as #41 voiced them
VOICE = {'Em': (64, 67, 71), 'Am': (64, 69, 72), 'B7': (63, 66, 69, 71), 'C': (64, 67, 72)}
ORGAN = {'Em': (59, 64, 67), 'Am': (60, 64, 69), 'B7': (59, 63, 69), 'C': (60, 64, 67)}


class Bed:
    buses = {'drums': (1.0, 0.15), 'calliope': (1.2, 0.45), 'glock': (1.0, 0.5)}   # bus: (gain, reverb send)
    reverb = (2.0, 0.55, 0.22)                     # a big tent: (dur, decay, wet)
    comp = (0.15, 0.62)                            # master compressor: threshold, exponent
    roar_gain, doors_gain = 0.15, 0.40
    landing = (0.07, 0.05)                         # caption landing thuds (first line, others)
    pop = (95, 100, 0.10)                          # the glass pop when the loser is whole again (two chimes, gain)

    def __init__(self, M):
        self.M, self.T, self.A = M, M.T, M.A
        self.SR = M.SR

    # ---------------------------------------------------------------- voicings
    def voice(self, ch):
        return VOICE.get(ch) or tuple(self.T.chord_tones(63, 72, ch))

    def organ(self, ch):
        if ch in ORGAN:
            return ORGAN[ch]
        pcs = self.T.CHORD_PCS[ch]
        return tuple(m for m in self.T.chord_tones(59, 69, ch) if not (len(pcs) > 3 and m % 12 == pcs[2]))

    # ---------------------------------------------------------------- instruments
    def calliope(self, ms, dur, vel=1.0, seed=0, att=0.008, vib=0.004):
        """band organ / calliope: breathy steam pipes (sine + a little 2nd / 3rd), each pipe a hair out of tune, a fast
        shallow vibrato, chiff noise on the attack"""
        A, SR = self.A, self.SR
        n = int((dur + 0.06) * SR)
        t = A.tt(n)
        rng = np.random.default_rng(seed)
        x = np.zeros(n)
        for j, m in enumerate(ms):
            f = A.mtof(m) * (1 + rng.uniform(-0.005, 0.005))
            fv = f * (1 + vib * np.sin(2 * np.pi * 6.3 * t + 1.7 * j))
            ph = 2 * np.pi * np.cumsum(fv) / SR
            x += np.sin(ph) + 0.28 * np.sin(2 * ph) + 0.10 * np.sin(3 * ph)
        x /= max(1, len(ms))
        chiff = A.bp(rng.normal(0, 1, n), 1200, 5000) * np.exp(-t / 0.015) * 0.25
        air = A.bp(rng.normal(0, 1, n), 800, 3000) * 0.04
        env = np.clip(t / att, 0, 1) * np.clip((dur + 0.06 - t) / 0.06, 0, 1)
        return (x + chiff + air) * env * vel

    def bass_drum(self, vel=1.0):
        """the concert bass drum ("oom"): a soft boom, over in ~0.2 s and high-passed so a bass line stays on top"""
        A = self.A
        x = A.kick(0.7, 0.2, 105, 50)[:int(0.36 * self.SR)]
        return A.hp(x, 46) * vel

    def choke(self, vel=1.0, seed=0):
        """the "pah" cymbal: a crash choked by hand after ~0.12 s"""
        A = self.A
        n = int(0.22 * self.SR)
        t = A.tt(n)
        x = A.hp(np.random.default_rng(seed).normal(0, 1, n), 4200) * np.exp(-t / 0.05)
        x += 0.15 * np.sin(2 * np.pi * 4130 * t) * np.exp(-t / 0.06)
        return x * np.clip((0.2 - t) / 0.06, 0, 1) * vel

    def snare(self, vel=1.0):
        return self.A.snare(0.13) * vel

    def roll(self, dur, v0=0.4, v1=1.0, rate=None, seed=0):
        """a snare roll (32nds), crescendo from v0 to v1"""
        rate = rate or (self.T.S16 / 2)
        n = int((dur + 0.15) * self.SR)
        out = np.zeros(n)
        k = 0
        while k * rate < dur - 1e-6:
            v = v0 + (v1 - v0) * (k * rate / max(dur, 1e-6))
            s = self.snare(0.6 + 0.4 * ((k % 2) == 0)) * v
            a = int(k * rate * self.SR)
            b = min(n, a + len(s))
            out[a:b] += s[:b - a]
            k += 1
        return out

    def tambourine(self, vel=1.0, seed=0):
        A = self.A
        n = int(0.12 * self.SR)
        t = A.tt(n)
        x = A.bp(np.random.default_rng(seed).normal(0, 1, n), 5500, 12000) * np.exp(-t / 0.04)
        x += 0.2 * sum(np.sin(2 * np.pi * f * t) for f in (6300, 7900, 9100)) * np.exp(-t / 0.05)
        return x * vel

    def bar(self, mix, b, intensity=1.0, light=False):
        """one bar of the big top (bar index b): bass drum on 1 + 3, choked cymbal on 2 + 4, the calliope's off-beat
        chords; the back half (`intensity` > 1.25) adds the march snare 8ths, tambourine 16ths, the held band-organ
        chord and a glockenspiel glint"""
        T, A = self.T, self.A
        t0 = b * T.BAR
        ch = T.CHORDS[b]
        big = intensity > 1.25 and not light
        for bt in range(4):
            tb = t0 + bt * T.BEAT
            if bt in (0, 2):
                mix.place('drums', self.bass_drum(1.0), tb, 0.62 if not light else 0.50)
            else:
                mix.place('drums', self.choke(0.9, 10 * b + bt), tb, 0.16)
                mix.place('drums', self.snare(0.8), tb, 0.10 if not big else 0.16)
            mix.place('calliope', self.calliope(self.voice(ch), 0.13, 1.0, seed=b * 4 + bt), tb + 0.5 * T.BEAT, 0.20,
                      pan=0.15 * (1 if bt % 2 else -1))
            if big:
                for half in (0.0, 0.5):                              # the march snare's 8ths, accent on the beat
                    if not (half == 0.0 and bt in (1, 3)):
                        mix.place('drums', self.snare(0.55 + 0.25 * (half == 0.0)), tb + half * T.BEAT, 0.07)
                for q in range(4):
                    mix.place('drums', self.tambourine(0.6 + 0.4 * (q == 2), 40 + q + bt), tb + q * T.S16, 0.05, pan=0.3)
        if big:
            mix.place('calliope', self.calliope(self.organ(ch), T.BAR - 0.05, 0.8, seed=200 + b, att=0.06, vib=0.006),
                      t0, 0.08)
            if b % 2 == 0:
                g = 84 + T.KEY_PC                                    # the glockenspiel glints round the key (E6 in E)
                mix.place('glock', A.chime(g + (0, 5, 7, 4)[(b // 2) % 4] + 12, 0.8), t0, 0.035)
        elif light:
            mix.place('calliope', self.calliope(self.organ(ch), T.BAR - 0.05, 0.8, seed=300 + b, att=0.08, vib=0.006),
                      t0, 0.07)

    def boom(self, mix, t, g):
        """a big drum hit (a slam landing)"""
        mix.place('drums', self.bass_drum(1.0), t, g)

    def clash(self, mix, t):
        mix.place('fx', self.M.clank(79, int(t * 3)), t, 0.04)

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        """a unison band hit under each word of the VS card"""
        A, ch = self.A, self.T.CHORDS[0]
        for j, tw_ in enumerate(times):
            mix.place('drums', self.bass_drum(1.0), tw_, 0.70)
            mix.place('drums', A.crash(1.0), tw_, 0.06)
            mix.place('calliope', self.calliope(self.voice(ch), 0.30, 1.0, seed=700 + j), tw_, 0.22)
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
        """the held organ under the card (never quiet), a snare roll on beat 4 as the doors open"""
        T = self.T
        mix.place('calliope', self.calliope(self.organ(T.CHORDS[0]), 3 * T.BEAT, 0.8, seed=710, att=0.03), 0.0, 0.08)
        mix.place('drums', self.roll(T.BEAT * 0.95, 0.3, 0.9), 3 * T.BEAT, 0.20)

    def count_in(self, mix):
        """bar 1: the count-in on the second chord (the dominant), a pickup and a roll into FIGHT!"""
        T, SR = self.T, self.SR
        ch = T.CHORDS[1]
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place('drums', self.bass_drum(1.0), tc, 0.62)
            mix.place('drums', self.choke(1.0, 50 + j), tc, 0.14)
            mix.place('calliope', self.calliope(self.voice(ch), 0.18, 1.0, seed=720 + j), tc, 0.20)
        mix.place('calliope', self.calliope(self.organ(ch), T.BAR, 0.7, seed=730, att=0.2) *
                  np.linspace(0.6, 1.0, int((T.BAR + 0.06) * SR)), T.BAR, 0.08)
        for bt in range(4):                                          # the calliope chunks the off-beats of the count too
            mix.place('calliope', self.calliope(self.voice(ch), 0.13, 1.0, seed=740 + bt), T.BAR + (bt + 0.5) * T.BEAT, 0.20)
        t4 = T.BAR + 3 * T.BEAT                                      # beat 4: the pickup - a band hit
        mix.place('drums', self.bass_drum(1.0), t4, 0.55)
        mix.place('calliope', self.calliope(self.voice(ch), 0.18, 1.0, seed=745), t4, 0.18)
        mix.place('drums', self.roll(T.BEAT, 0.5, 1.0), t4, 0.30)    # the roll into FIGHT!

    def fight_start(self, mix):
        mix.place('drums', self.A.crash(1.8), self.T.T_FIGHT, 0.12)  # the band comes in
        mix.place('fx', self.A.thud(0.7, 110, 34), self.T.T_FIGHT, 0.34)

    def fight(self, mix, bar_ko):
        """the big top from FIGHT! to the K.O. (denser as it goes), a roll + riser into the back half, a tape stop at
        the K.O."""
        T, A = self.T, self.A
        for b in range(2, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
        for b in range(2, bar_ko + 1, 8):                            # every 8-bar phrase: a crash on its first beat
            mix.place('drums', A.crash(1.6), b * T.BAR, 0.08)
        t_half = (T.LOOP_START + 2 * len(T.LOOP)) * T.BAR            # the back half (bar 18 in #41)
        mix.place('drums', self.roll(2 * T.BEAT, 0.3, 1.0), t_half - 2 * T.BEAT, 0.20)
        mix.place('fx', self.M.riser(T.BAR, over=0.0, seed=12) * 0.6, t_half - T.BAR, 0.08)
        mix.place('drums', A.crash(2.0), t_half, 0.12)
        for k in ('drums', 'calliope', 'glock'):
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], self.M.T_KO, 0.42)

    def ko(self, mix):
        mix.place('drums', self.A.crash(2.4), self.M.T_KO, 0.16)
        mix.place('fx', self.A.thud(0.9, 120, 30), self.M.T_KO, 0.6)

    def stinger(self, mix, tw):
        mix.place('calliope', self.calliope(self.voice(self.T.CHORDS[0]), 0.35, 1.0, seed=800), tw, 0.22)
        mix.place('drums', self.bass_drum(1.0), tw, 0.65)
        mix.place('drums', self.A.crash(1.8), tw, 0.10)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        mix.place('fx', self.A.whoosh(max(0.2, t_solo - tw - 0.1), 300, 5000), tw + 0.1, 0.08)

    def victory(self, mix, b_solo):
        """the band plays on under the solo (no march snare), a held organ chord, up to the build"""
        T = self.T
        for b in range(b_solo, T.N_BARS - 1):
            self.bar(mix, b, intensity=1.1)
            mix.place('calliope', self.calliope(self.organ(T.CHORDS[b]), T.BAR - 0.05, 0.8, seed=300 + b, att=0.08,
                                                vib=0.006), b * T.BAR, 0.07)

    def last_bars(self, mix):
        """the last bar builds into the title slam on sample 0: a snare roll, the bass drum on every beat, the calliope
        off-beats, a riser and an organ swell right up to the seam"""
        T, M = self.T, self.M
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.CHORDS[-1]
        mix.place('drums', self.roll(T.DUR - t_last + 0.01, 0.4, 1.0), t_last, 0.20)
        for bt in range(4):
            mix.place('drums', self.bass_drum(0.95), t_last + bt * T.BEAT, 0.56)
            mix.place('calliope', self.calliope(self.voice(ch), 0.13, 1.0, seed=900 + bt), t_last + (bt + 0.5) * T.BEAT, 0.20)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.18)
        sw = self.calliope(self.organ(ch), T.DUR - t_last + 0.012, 0.9, seed=950, att=T.DUR - t_last, vib=0.008)
        mix.place('calliope', sw, t_last, 0.10)
