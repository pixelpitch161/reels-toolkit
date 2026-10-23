"""Template soundtrack: ambience bed, whooshes on cuts, a simple beat + pads for the song section.
Replace the song with an original arrangement (or a public-domain piece) that the science drives."""
import os
import sys
import numpy as np

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit import audio as A
import timeline as T


def main():
    mix = A.Mix(T.DUR)
    n = int(T.DUR * A.SR)
    wind = A.lp(np.random.default_rng(1).normal(0, 1, n), 500) * 0.15
    mix.place('amb', wind, 0.0, 0.6)
    mix.place('fx', A.whoosh(0.7), T.T_REVEAL - 0.4, 0.25)
    mix.place('fx', A.chime(88, 1.5), 0.2, 0.15)
    chords = [[57, 64, 69, 72], [53, 60, 65, 69], [48, 55, 60, 64], [55, 62, 67, 71]]
    t = T.T_SONG
    bar = 0
    while t < T.T_END:
        mix.place('music', A.pad(chords[bar % 4], 4 * T.BEAT), t, 0.12)
        for b in range(4):
            tb = t + b * T.BEAT
            mix.place('drums', A.kick(), tb, 0.5 if b % 2 == 0 else 0.0)
            mix.place('drums', A.snare(), tb, 0.25 if b % 2 else 0.0)
            mix.place('drums', A.hat(), tb + T.BEAT / 2, 0.12)
            mix.place('music', A.pluck(chords[bar % 4][-1] + 12, 0.5), tb, 0.08)
        t += 4 * T.BEAT
        bar += 1
    mix.place('music', A.pad([57, 64, 69, 76], 6.0), T.T_END, 0.12)
    out = mix.master(reverb_buses={'music': 1.0, 'fx': 0.5},
                     gain_db_pts=[(0, 0), (T.T_SONG - 0.5, 0), (T.T_SONG, 3), (T.T_END, 3), (T.DUR, 0)])
    A.write('audio.wav', out)
    for a, b, rms, pk in A.section_levels('audio.wav', [(0, 4), (4, 35), (35, 52), (52, T.DUR)]):
        print(f'{a:4.0f}-{b:4.0f}s  rms {rms:6.1f} dBFS  peak {pk:5.1f}')


if __name__ == '__main__':
    main()
