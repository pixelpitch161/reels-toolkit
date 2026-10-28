"""Mood: villain march (#32 Tuba vs Flute v3, ported as it was). A heavy minor march with NO bass and NO lead of its
own (the low fighter is the bass, the high one the top): a snare ostinato (16ths with accents) and a heavy bass drum on
1 and 3, low brass "pah" chords on 2 and 4 (detuned saws with a bright attack bite), a slow horn swell from the second
fight bar, and a sinister music-box lullaby on the quarters (high, a little out of tune) from the third; timpani on the
downbeat, a crash every 4 bars and the brass's 8th-note pahs building through the back half. The VS card is a unison
band hit per word (bass drum, snare, brass, timpani, thud) over a horn pad, a snare pickup as the doors open; the
count-in a snare + brass pah per count, a snare pickup into FIGHT!. After the K.O. a lighter march (snare on the beats
only, no music box) under the solo, the full march under the reassembly, and the last bar builds (snare roll, riser,
horn swell, bass drum) into the title slam. Built for C harmonic minor (Cm Ab Db7 G7: i VI N6 V, the Neapolitan with the
B natural on top); any chords work by the rules below (they give #32's notes for its chords):

    brass      the chord's tones in G3-F4 (#32: Cm G C Eb, Ab Ab C Eb, Db7 Ab B Db F, G7 G B D F); the horn pad their
               lowest three (no low note: the bass is a fighter's)
    music box  one note a quarter, following the LOOP from FIGHT! like a fighter's line: a 4-bar shape (BOX_SHAPE) on
               the chord tones of the loop bar's chord counted up from its root at or above G5 (R, 3, 5, 7 or R'),
               played an octave up. Bar 1: R 3 3-1 R-1; bar 2: 3 R R-1 R; bar 3: R 3 3-1, then its 4th tone an
               octave down; bar 4: R 5 R+12 5 (-1 = a semitone below). #32: C Eb D B / C Ab G Ab / Db F E B / G D G D
    timpani    the chord's root in C3-B3
    card       the tonic's root in C3-B3, its fifth, the root's octave and the third above that (#32: C3 G3 C4 Eb4);
               the stinger adds the fifth on top (G4); the card's timpani on the tonic's root in C2-B2"""
import numpy as np

SNARE16 = {0: 1.0, 3: 0.45, 4: 0.75, 6: 0.5, 8: 0.9, 11: 0.45, 12: 0.7, 13: 0.4, 14: 0.5, 15: 0.62}   # 16th: accent
# the music box's lullaby: per bar of the 4-bar shape, 4 quarters of (chord-tone index, semitones): #32's line
BOX_SHAPE = (((0, 0), (1, 0), (1, -1), (0, -1)),
             ((1, 0), (0, 0), (0, -1), (0, 0)),
             ((0, 0), (1, 0), (1, -1), (3, -12)),
             ((0, 0), (2, 0), (0, 12), (2, 0)))


class Bed:
    buses = {'drums': (0.9, 0.15), 'brass': (3.0, 0.4), 'pad': (2.0, 0.5), 'musicbox': (1.2, 0.7),
             'solo': (1.6, 0.5), 'fx': (1.5, 0.4)}          # bus: (gain, reverb send) (#32)
    reverb = (2.4, 0.6, 0.20)                              # (dur, decay, wet)
    comp = (0.22, 0.35)                                    # #32's compressor (it also had section automation, dropped in
                                                           # the kit: one master gain)
    roar_gain, doors_gain = 0.16, 0.55
    doors_open_gain, count_whoosh_gain = 0.16, 0.14
    landing = (0.08, 0.06)                                 # caption landing thuds (first line, others)
    pop = (95, 102, 0.12)                                  # the glass pop when the loser is whole again

    def __init__(self, M):
        self.M, self.T, self.A = M, M.T, M.A
        self.SR = M.SR

    # ---------------------------------------------------------------- voicings (rules; #32's tables are what they give)
    def voicing(self, ch):
        """the brass pah: the chord's tones in G3-F4"""
        return tuple(self.T.chord_tones(55, 65, ch))

    def tonic_voicing(self, ch):
        """the card's band hit: the root in C3-B3, its fifth, the root's octave, the third above it"""
        from ballfight.kit.episode import parse_chord
        r, pcs = parse_chord(ch)
        r = self.T.root_above(ch, 48)
        return (r, r + (pcs[2] - pcs[0]) % 12, r + 12, r + 12 + (pcs[1] - pcs[0]) % 12)

    def box_notes(self, b):
        """the music box's 4 quarters in bar b (before its octave up): the shape of the loop's bar on the loop chord's
        tones from its root at or above G5"""
        T = self.T
        j = (b - T.LOOP_START) % len(T.LOOP)
        ch = T.LOOP[j]
        r = T.root_above(ch, 79)
        tones = T.chord_tones(r, r + 24, ch)
        return [tones[i] + d for i, d in BOX_SHAPE[j % 4]]

    # ---------------------------------------------------------------- instruments (#32)
    def brass(self, ms, dur, att=0.012, cut=1500, bite=1.0):
        """low brass section: saws per note, a bright 'bwah' filter bite at the attack, slightly detuned"""
        A = self.A
        n = int(dur * self.SR)
        tt = A.tt(n)
        x = np.zeros(n)
        for j, m in enumerate(ms):
            f = A.mtof(m)
            for d in (-0.004, 0.004):
                ph = (f * (1 + d) * tt + 0.37 * j) % 1.0
                x += 2 * ph - 1
        x /= 2 * max(1, len(ms))
        env = A.adsr(n, att, 0.08, 0.7, min(0.06, dur * 0.3))
        k = np.exp(-tt / 0.07)
        y = A.lp(x, cut * 0.45) * (1 - k * bite) + A.lp(x, cut * 1.6) * k * bite
        return y * env

    def horn_pad(self, ms, dur):
        A = self.A
        n = int(dur * self.SR)
        tt = A.tt(n)
        x = np.zeros(n)
        for m in ms:
            f = A.mtof(m)
            x += np.sin(2 * np.pi * f * tt) + 0.35 * np.sin(4 * np.pi * f * tt) + 0.15 * np.sin(6 * np.pi * f * tt)
        x /= max(1, len(ms))
        return A.lp(x, 1300) * A.adsr(n, min(0.5, dur * 0.3), 0.2, 0.85, min(0.4, dur * 0.3)) * (1 + 0.05 * np.sin(2 * np.pi * 4.8 * tt))

    def music_box(self, m, vel=1.0):
        """a music-box tine: bright sine + an inharmonic partial, short ring, a little detune (out-of-tune, sinister)"""
        A = self.A
        n = int(0.9 * self.SR)
        tt = A.tt(n)
        f = A.mtof(m) * (1 - 0.004)
        x = np.sin(2 * np.pi * f * tt) * np.exp(-tt / 0.32) + 0.35 * np.sin(2 * np.pi * f * 2.76 * tt) * np.exp(-tt / 0.06)
        x += 0.15 * np.sin(2 * np.pi * f * 5.4 * tt) * np.exp(-tt / 0.02)
        return x * vel

    def bass_drum(self, vel=1.0):
        return self.A.kick(0.6, 0.45, 110, 40) * vel

    def march_snare(self, vel=1.0):
        A = self.A
        n = int(0.22 * self.SR)
        tt = A.tt(n)
        x = A.snare(0.11)[:n]
        x = np.pad(x, (0, max(0, n - len(x))))
        rattle = A.bp(np.random.default_rng(31).normal(0, 1, n), 3000, 9000) * np.exp(-tt / 0.07)
        return (x + 0.35 * rattle) * vel

    def timpani(self, m, vel=1.0):
        A = self.A
        n = int(1.2 * self.SR)
        tt = A.tt(n)
        f = A.mtof(m)
        x = np.sin(2 * np.pi * f * tt * (1 + 0.02 * np.exp(-tt / 0.05))) + 0.4 * np.sin(2 * np.pi * f * 1.5 * tt) * np.exp(-tt / 0.2)
        x *= np.exp(-tt / 0.5)
        x += 0.3 * A.bp(np.random.default_rng(int(m)).normal(0, 1, n), 100, 900) * np.exp(-tt / 0.02)
        return x * vel

    def roll(self, dur, f0=0.3, step=None):
        step = step or self.T.S16 / 2
        SR = self.SR
        n = int(dur * SR)
        out = np.zeros(n + SR)
        t = 0.0
        while t < dur - 1e-4:
            a = int(t * SR)
            s = self.march_snare(f0 + (1 - f0) * (t / dur) ** 1.5) * 0.7
            out[a:a + len(s)] += s[:len(out) - a]
            t += step
        return out[:n]

    def bar(self, mix, b, intensity=1.0, light=False):
        """one bar of the march (bar index b): snare ostinato + bass drum, brass 'pah' on 2 and 4, horn swell, music
        box (#32's bed())"""
        T, A, S16 = self.T, self.A, self.T.S16
        t0 = b * T.BAR
        ch = T.CHORDS[b]
        for st, v in SNARE16.items():
            if light and st % 4:
                continue
            mix.place('drums', self.march_snare(v), t0 + st * S16, (0.20 if light else 0.24) * (0.8 + 0.2 * intensity),
                      pan=-0.1)
        for bt in (0, 2):
            mix.place('drums', self.bass_drum(1.0 if bt == 0 else 0.8), t0 + bt * T.BEAT, 0.21 if not light else 0.18)
        vo = self.voicing(ch)
        for bt in (1, 3):
            mix.place('brass', self.brass(vo, 0.22), t0 + bt * T.BEAT, 0.20 if light else 0.24)
            if intensity > 1.35 and not light:
                mix.place('brass', self.brass(vo, 0.12, bite=0.6), t0 + (bt + 0.5) * T.BEAT, 0.12)
        if b >= T.FIGHT_BAR + 1 or light:
            mix.place('pad', self.horn_pad([vo[0], vo[1], vo[2]], T.BAR), t0, 0.07)     # no low note: the bass is a fighter's
        if b >= T.FIGHT_BAR + 2 and not light:                     # the villain's lullaby on top, on the quarters
            for j, m in enumerate(self.box_notes(b)):
                mix.place('musicbox', self.music_box(m + 12, 0.9), t0 + j * 2 * 2 * S16, 0.06, pan=0.35)
        if intensity > 1.3 and not light:                          # timpani on the downbeat + a crash every 4 bars
            mix.place('drums', self.timpani(T.root_above(ch, 48), 0.9), t0, 0.16 * min(1.0, (intensity - 1.3) / 0.2))
            if (b - T.LOOP_START) % 4 == 3:
                mix.place('drums', A.crash(1.6), t0, 0.07)

    def boom(self, mix, t, g):
        """a big bass drum hit (a slam landing, a ball into the ropes)"""
        mix.place('drums', self.bass_drum(1.0), t, g)

    def clash(self, mix, t):
        mix.place('fx', self.A.chime(91, 0.25), t, 0.05)

    # ---------------------------------------------------------------- sections (kit.music calls these in order)
    def title(self, mix, times):
        """the whole band on every word of the VS card: bass drum, snare, brass, timpani, a thud"""
        A, T = self.A, self.T
        ch = T.CHORDS[0]
        vo = self.tonic_voicing(ch)
        for tw_ in times:
            mix.place('drums', self.bass_drum(1.0), tw_, 0.55)
            mix.place('drums', self.march_snare(1.0), tw_, 0.30)
            mix.place('brass', self.brass(vo, 0.42, bite=1.0), tw_, 0.30)
            mix.place('drums', self.timpani(T.root_above(ch, 36), 1.0), tw_, 0.30)
            mix.place('fx', A.thud(0.9, 120, 32, seed=int(tw_ * 50) + 3), tw_, 0.45)

    def vs(self, mix, t):
        """VS: a thunder crack where it splits the divider, a crash"""
        A, SR = self.A, self.SR
        nn = int(1.4 * SR)
        tt0 = A.tt(nn)
        crack = A.bp(np.random.default_rng(91).normal(0, 1, nn), 1500, 9000) * np.exp(-tt0 / 0.035)
        rumble = A.lp(np.random.default_rng(92).normal(0, 1, nn), 160) * np.clip(tt0 / 0.02, 0, 1) * np.exp(-tt0 / 0.5) * 3.0
        mix.place('fx', crack * 0.9 + rumble, t, 0.42)
        mix.place('drums', A.crash(1.6), t, 0.10)

    def title_rest(self, mix):
        """beat 4: the hold, a snare pickup as the doors open, a horn pad under the whole card (never quiet)"""
        T = self.T
        mix.place('pad', self.horn_pad(list(self.tonic_voicing(T.CHORDS[0])), 3 * T.BEAT), 0.0, 0.10)
        for st, v in ((12, 0.45), (13, 0.6), (14, 0.75), (15, 0.9)):
            mix.place('drums', self.march_snare(v), st * T.S16, 0.30)

    def count_in(self, mix):
        """the count-in bar on its chord (the dominant): a snare + a brass pah per count, a snare pickup into FIGHT!"""
        T = self.T
        cb = T.FIGHT_BAR - 1
        vo = self.voicing(T.CHORDS[cb])
        for tc, txt in T.COUNT[:3]:
            mix.place('drums', self.march_snare(1.0), tc, 0.28)
            mix.place('brass', self.brass(vo, 0.16), tc, 0.20)
        for st, v in ((12, 0.45), (13, 0.6), (14, 0.75), (15, 0.9)):
            mix.place('drums', self.march_snare(v), cb * T.BAR + st * T.S16, 0.30)

    def fight_start(self, mix):
        mix.place('drums', self.A.crash(2.0), self.T.T_FIGHT, 0.14)               # the drop
        mix.place('drums', self.bass_drum(1.0), self.T.T_FIGHT, 0.5)

    def fight(self, mix, bar_ko):
        """the march from FIGHT! to the K.O. (heavier as it goes), a whoosh into the final exchange (44 s), a tape stop
        at the K.O."""
        T, A = self.T, self.A
        for b in range(T.FIGHT_BAR, bar_ko + 1):
            self.bar(mix, b, intensity=1.0 + 0.6 * (b / bar_ko))
        mix.place('fx', A.whoosh(3.6, 300, 6000), 44.0 - 3.6, 0.10)
        for k in ('drums', 'brass', 'pad', 'musicbox'):
            if k in mix.buses:
                mix.buses[k] = A.tape_stop(mix.buses[k], self.M.T_KO, 0.42)

    def ko(self, mix):
        mix.place('drums', self.A.crash(2.4), self.M.T_KO, 0.16)
        mix.place('fx', self.A.thud(0.9, 120, 30), self.M.T_KO, 0.7)

    def stinger(self, mix, tw):
        """under "(WINNER) WINS": the brass on the tonic (+ its fifth on top), a bass drum, a crash"""
        r = self.tonic_voicing(self.T.CHORDS[0])
        mix.place('brass', self.brass(r + (r[1] + 12,), 1.2), tw, 0.24)
        mix.place('drums', self.bass_drum(1.0), tw, 0.5)
        mix.place('drums', self.A.crash(2.0), tw, 0.12)

    def after_ko(self, mix, bar_ko, tw, t_solo):
        mix.place('fx', self.A.whoosh(max(0.3, t_solo - tw - 0.1), 300, 5000), tw + 0.1, 0.10)

    def victory(self, mix, b_solo):
        """a lighter march under the solo (snare on the beats, no music box), the full march again under the
        reassembly (the bar before the last)"""
        T = self.T
        for b in range(b_solo, T.N_BARS - 1):
            self.bar(mix, b, light=b < T.N_BARS - 2)

    def last_bars(self, mix):
        """the last bar builds into the title slam on sample 0: a snare roll, a riser, brass pahs on the off-beats, a
        horn swell on the last chord over its root, the bass drum on 1, 3, 4 and the 4's 8th"""
        T, M = self.T, self.M
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.CHORDS[-1]
        mix.place('drums', self.roll(T.DUR - t_last + 0.03, 0.15, T.S16 / 2), t_last, 0.30)
        mix.place('fx', M.riser(T.DUR - t_last), t_last, 0.22)
        vo = self.voicing(ch)
        for bt in range(4):
            mix.place('brass', self.brass(vo, 0.18), t_last + bt * T.BEAT + T.BEAT / 2, 0.18)
        mix.place('pad', self.horn_pad([T.root_above(ch, 36)] + list(vo), T.DUR - t_last + 0.02), t_last, 0.08)
        for st in (0, 8, 12, 14):
            mix.place('drums', self.bass_drum(0.9), t_last + st * T.S16, 0.42)
