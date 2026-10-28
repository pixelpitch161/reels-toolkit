"""Mood: brass house (#36 Trombone vs Tuba, ported as it was). A dark minor house groove with NO bass synth and NO brass
of its own (two low-brass fighters are the bass line and the hook): a short four-on-the-floor kick (over in ~0.2 s so
a held bass note stays on top), claps on 2 + 4, offbeat hats, a filtered minor pad pumping under the kick (sidechain),
syncopated synth chord stabs (detuned saws, high and short, not brass). Denser as the fight goes: 16th shakers, then
(the back half) open hats, a ride, a brighter pad and twice the stabs; a filter-sweep riser into the second half of the
file. The VS card is a unison house hit per word (kick, stab, thud, crash) over the pad, a clap pickup as the doors
open; the count-in a kick + stab per count, a clap pickup into the drop. Built for F minor (Fm Db Bbm C); any chords
work by the rules below (they give #36's notes for its chords):

    pad    the chord's four highest tones up to Ab5; the VI chord of a minor key (a major triad a major sixth above the
           tonic) is voiced as VImaj7 without its root (#36's Db: Ab C F Ab, the tonic pad's common tones)
    stabs  the chord's first three tones from C5 up (#36: Fm C F Ab, Db Db F Ab, Bbm Db F Bb, C C E G)"""
import math

import numpy as np

STAB_SLOTS = (3, 6, 10, 14)                    # syncopated 16ths (the "and"s around beats 2 and 4)


class Bed:
    buses = {'drums': (1.0, 0.12), 'pad': (1.6, 0.4), 'stabs': (1.2, 0.5), 'solo': (1.6, 0.4),
             'fx': (1.4, 0.3)}                 # bus: (gain, reverb send) (#36)
    reverb = (2.4, 0.6, 0.20)                  # (dur, decay, wet)
    comp = (0.22, 0.35)                        # #36's compressor (it also had section automation, dropped in the kit)
    roar_gain, doors_gain = 0.16, 0.45
    doors_open_gain, count_whoosh_gain = 0.16, 0.14
    landing = (0.08, 0.06)                     # caption landing thuds (first line, others)
    pop = (95, 102, 0.12)                      # the glass pop when the loser is whole again

    def __init__(self, M):
        self.M, self.T, self.A = M, M.T, M.A
        self.SR = M.SR

    # ---------------------------------------------------------------- voicings (rules; #36's tables are what they give)
    def pad_notes(self, ch):
        from ballfight.kit.episode import parse_chord
        T = self.T
        r, pcs = parse_chord(ch)
        if len(pcs) == 3 and (r - T.KEY_PC) % 12 == 8 and (pcs[1] - r) % 12 == 4:   # minor key's VI: VImaj7, no root
            pcs = (pcs[1], pcs[2], (r + 11) % 12)
        tones = [m for m in range(56, 81) if m % 12 in pcs]
        return tuple(tones[-4:])

    def stab_notes(self, ch):
        return tuple(self.T.chord_tones(72, 96, ch)[:3])

    # ---------------------------------------------------------------- instruments (#36)
    def kick(self, vel=1.0):
        """a short, punchy house kick: the body is over in ~0.2 s so the bass's held notes stay on top"""
        A = self.A
        x = A.kick(1.0, 0.16, 160, 52)[:int(0.32 * self.SR)]
        return A.hp(x, 48) * vel

    def hat(self, vel=1.0, open_=False, seed=4):
        A = self.A
        n = int((0.32 if open_ else 0.09) * self.SR)
        t = A.tt(n)
        x = A.hp(np.random.default_rng(seed).normal(0, 1, n), 7500) * np.exp(-t / (0.12 if open_ else 0.025))
        return x * 0.5 * vel

    def ride(self, vel=1.0, seed=8):
        A = self.A
        n = int(0.6 * self.SR)
        t = A.tt(n)
        x = A.hp(np.random.default_rng(seed).normal(0, 1, n), 5200) * np.exp(-t / 0.25)
        return (x * 0.4 + 0.18 * np.sin(2 * np.pi * 3170 * t) * np.exp(-t / 0.3)) * vel

    def shaker(self, vel=1.0, seed=9):
        A = self.A
        n = int(0.07 * self.SR)
        t = A.tt(n)
        return A.bp(np.random.default_rng(seed).normal(0, 1, n), 4000, 10000) * np.clip(t / 0.01, 0, 1) * np.exp(-t / 0.02) * vel

    def stab(self, ms, dur=0.11, cut=2600):
        """a house chord stab: detuned saws + a square an octave down, a fast filter envelope (synth, not brass)"""
        A, SR = self.A, self.SR
        n = int((dur + 0.12) * SR)
        t = A.tt(n)
        x = sum(A.saw(A.mtof(m), n, 0.008, 2) for m in ms) / len(ms)
        x = x + 0.3 * sum(A.square(A.mtof(m - 12), n, 0.4) for m in ms[:1])
        out = np.zeros(n)
        nb = 10
        for k in range(nb):
            a, b = int(k * n / nb), int((k + 1) * n / nb)
            fc = 400 + cut * math.exp(-((a + b) / 2 / SR) / 0.05)
            out[a:b] = A.lp(x[max(0, a - 2048):b], fc)[-(b - a):]
        return out * A.adsr(n, 0.003, 0.05, 0.4, 0.08) * np.clip((dur + 0.06 - t) / 0.06, 0, 1)

    def pump(self, n, t0=0.0, depth=0.6):
        """the sidechain pump: the pad ducks under every kick (beats from t0) and swells back"""
        t = self.A.tt(n) + t0
        ph = t % self.T.BEAT
        return 1 - depth * np.exp(-ph / 0.13)

    def pad_bar(self, ch, b, dur, cut, att=0.06):
        n = int(dur * self.SR)
        x = self.A.pad(self.pad_notes(ch), dur, cut, att)
        return x[:n] * self.pump(len(x[:n]), b * self.T.BAR)

    def bar(self, mix, b, intensity=1.0, light=False):
        """one bar of the groove (bar index b): kick, claps, hats (+ 16ths / open hats / ride in the back half), the
        pumping pad, chord stabs (#36's bed())"""
        T, A, S16 = self.T, self.A, self.T.S16
        t0 = b * T.BAR
        ch = T.CHORDS[b]
        big = intensity > 1.3 and not light                                # the back half: the groove opens up
        for bt in range(4):
            mix.place('drums', self.kick(1.0), t0 + bt * T.BEAT, 0.44 if not light else 0.38)
            mix.place('drums', self.hat(0.9, False, 10 + bt), t0 + (bt + 0.5) * T.BEAT, 0.13)
            if bt % 2:
                mix.place('drums', A.clap(), t0 + bt * T.BEAT, 0.30)
            if big:
                mix.place('drums', self.hat(0.9, True, 20 + bt), t0 + (bt + 0.5) * T.BEAT, 0.07)
                mix.place('drums', self.ride(0.8, 30 + bt), t0 + bt * T.BEAT, 0.06)
        if intensity > 1.15 or light:                                       # 16th shaker / hats
            for st in range(16):
                if st % 2:
                    mix.place('drums', self.shaker(0.6 + 0.4 * (st % 4 == 3), 40 + st), t0 + st * S16,
                              0.06 if not light else 0.04)
        cut = 900 + 1500 * min(1.0, max(0.0, (intensity - 1.0) / 0.6)) + (900 if big else 0)
        mix.place('pad', self.pad_bar(ch, b, T.BAR + 0.05, cut if not light else 1300), t0, 0.11)
        if not light:
            slots = STAB_SLOTS if big else STAB_SLOTS[1::2]
            for st in slots:
                mix.place('stabs', self.stab(self.stab_notes(ch), 0.10, 2200 if big else 1500), t0 + st * S16, 0.07,
                          pan=0.25 * (1 if st % 8 > 4 else -1))

    def boom(self, mix, t, g):
        """a big kick (a slam landing, a ball into the ropes)"""
        mix.place('drums', self.kick(1.0), t, g)

    def clash(self, mix, t):
        mix.place('fx', self.M.clank(79, int(t * 3)), t, 0.04)

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        """a unison house hit under each word of the VS card"""
        A, ch = self.A, self.T.CHORDS[0]
        for tw_ in times:
            mix.place('drums', self.kick(1.0), tw_, 0.70)
            mix.place('stabs', self.stab(self.stab_notes(ch), 0.22, 3200), tw_, 0.14)
            mix.place('fx', A.thud(0.9, 120, 32, seed=int(tw_ * 50) + 3), tw_, 0.40)
            mix.place('drums', A.crash(1.2), tw_, 0.06)

    def vs(self, mix, t):
        """VS: a thunder crack where it splits the divider"""
        A, SR = self.A, self.SR
        nn = int(1.4 * SR)
        tt_ = A.tt(nn)
        crack = A.bp(np.random.default_rng(91).normal(0, 1, nn), 1500, 9000) * np.exp(-tt_ / 0.035)
        rumble = A.lp(np.random.default_rng(92).normal(0, 1, nn), 160) * np.clip(tt_ / 0.02, 0, 1) * np.exp(-tt_ / 0.5) * 3.0
        mix.place('fx', crack * 0.9 + rumble, t, 0.40)

    def title_rest(self, mix):
        """the pad under the card (never quiet), a clap pickup on beat 4 as the doors open"""
        T, A = self.T, self.A
        mix.place('pad', A.pad(self.pad_notes(T.CHORDS[0]), 3 * T.BEAT + 0.1, 1100, 0.03), 0.0, 0.10)
        for st, v in ((12, 0.5), (13, 0.65), (14, 0.8), (15, 0.95)):
            mix.place('drums', A.clap(), st * T.S16, 0.22 * v)

    def count_in(self, mix):
        """the count-in bar on its chord (the dominant): a kick + a stab per count, the pad swelling in, a clap pickup
        into the drop"""
        T, A = self.T, self.A
        cb = T.FIGHT_BAR - 1
        ch = T.CHORDS[cb]
        for tc, txt in T.COUNT[:3]:
            mix.place('drums', self.kick(1.0), tc, 0.60)
            mix.place('stabs', self.stab(self.stab_notes(ch), 0.16, 2800), tc, 0.11)
        mix.place('pad', A.pad(self.pad_notes(ch), T.BAR, 900, 0.4) * np.linspace(0.4, 1.0, int(T.BAR * self.SR)),
                  cb * T.BAR, 0.10)
        for st, v in ((12, 0.5), (13, 0.62), (14, 0.76), (15, 0.9)):
            mix.place('drums', A.clap(), cb * T.BAR + st * T.S16, 0.28 * v)

    def fight_start(self, mix):
        mix.place('drums', self.A.crash(1.8), self.T.T_FIGHT, 0.12)               # the drop
        mix.place('fx', self.A.thud(0.7, 110, 34), self.T.T_FIGHT, 0.40)

    def fight(self, mix, bar_ko):
        """the groove from FIGHT! to the K.O. (denser as it goes), a filter-sweep riser + crash into the second half of
        the file (bar N_BARS / 2 + 1: #36's bar 17), a whoosh into the final exchange (44 s), a tape stop at the K.O."""
        T, A = self.T, self.A
        for b in range(T.FIGHT_BAR, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
        t_half = (T.N_BARS // 2 + 1) * T.BAR
        mix.place('fx', self.M.riser(2 * T.BAR, over=0.0, seed=12) * 0.6, t_half - 2 * T.BAR, 0.10)
        mix.place('drums', A.crash(2.0), t_half, 0.10)
        mix.place('fx', A.whoosh(3.6, 300, 6000), 44.0 - 3.6, 0.10)
        for k in ('drums', 'pad', 'stabs'):
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], self.M.T_KO, 0.42)

    def ko(self, mix):
        mix.place('drums', self.A.crash(2.4), self.M.T_KO, 0.16)
        mix.place('fx', self.A.thud(0.9, 120, 30), self.M.T_KO, 0.7)

    def stinger(self, mix, tw):
        mix.place('stabs', self.stab(self.stab_notes(self.T.CHORDS[0]), 0.3, 3200), tw, 0.12)
        mix.place('drums', self.kick(1.0), tw, 0.55)
        mix.place('drums', self.A.crash(1.8), tw, 0.10)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        mix.place('fx', self.A.whoosh(max(0.2, t_solo - tw - 0.1), 300, 5000), tw + 0.1, 0.10)

    def victory(self, mix, b_solo):
        """a lighter groove under the solo (no stabs, a softer kick, shakers), up to the build"""
        for b in range(b_solo, self.T.N_BARS - 1):
            self.bar(mix, b, light=True)

    def last_bars(self, mix):
        """the last bar builds into the title slam on sample 0: a clap roll (8ths, then 16ths for the last beat), the
        kick on every beat, a riser and a pad swelling right up to the seam"""
        T, A, M = self.T, self.A, self.M
        t_last = (T.N_BARS - 1) * T.BAR
        st = 0.0
        while t_last + st < T.DUR - 1e-4:
            v = 0.45 + 0.55 * st / (T.DUR - t_last)
            mix.place('drums', A.clap(), t_last + st, 0.26 * v)
            st += T.BEAT / 2 if st < 3 * T.BEAT - 1e-6 else T.S16
        for bt in range(4):
            mix.place('drums', self.kick(0.9), t_last + bt * T.BEAT, 0.46)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.22)
        sw = A.pad(self.pad_notes(T.CHORDS[-1]), T.DUR - t_last + 0.012, 2400, T.DUR - t_last)
        mix.place('pad', sw * np.linspace(0.3, 1.0, len(sw)), t_last, 0.12)
