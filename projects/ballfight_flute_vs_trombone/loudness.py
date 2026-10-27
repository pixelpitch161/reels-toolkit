"""Perceived loudness per section (ITU-R BS.1770 K-weighting, LUFS-style short-term 3 s / momentary 0.4 s): RMS
(reelkit.levels) weighs every frequency alike, the ear (and K-weighting) hears bright clangs louder. #40 v2: the intro
read "a little loud" though its RMS matched the fight (copied from #40, spans at #41's tempo).

    python loudness.py [audio.wav]"""
import sys
import numpy as np
from scipy.io import wavfile
from scipy.signal import lfilter


def kweight(x, sr=48000):
    # BS.1770 stage 1 (high shelf, +4 dB above ~1.5 kHz) and stage 2 (RLB high-pass ~38 Hz), 48 kHz coefficients
    b1, a1 = [1.53512485958697, -2.69169618940638, 1.19839281085285], [1.0, -1.69065929318241, 0.73248077421585]
    b2, a2 = [1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621]
    return lfilter(b2, a2, lfilter(b1, a1, x, axis=0), axis=0)


def lufs(y):
    return -0.691 + 10 * np.log10((y ** 2).mean(0).sum() + 1e-12)


def main(path):
    sr, x = wavfile.read(path)
    x = x.astype(float) / 32768.0
    y = kweight(x, sr)
    import timeline as T
    spans = [(0, T.BAR), (T.BAR, T.T_FIGHT), (T.T_FIGHT, 10), (10, 20), (20, 32), (32, 44), (44, 50.0), (50.0, T.T_REASSEMBLE),
             (T.T_REASSEMBLE, 61)]
    for a, b in spans:
        print(f'{a:5.1f}-{b:5.1f} s  {lufs(y[int(a * sr):int(b * sr)]):6.1f} LUFS')
    fight = lufs(y[int(T.T_FIGHT * sr):int(50 * sr)])
    intro = lufs(y[:int(T.T_FIGHT * sr)])
    print(f'intro (title card + count-in) {intro:.1f} LUFS vs the fight {fight:.1f}: {intro - fight:+.1f} LU')
    w = int(0.4 * sr)
    m = [lufs(y[i:i + w]) for i in range(0, int(4 * sr), w // 2)]
    print('momentary 0-4 s:', ' '.join(f'{v:.0f}' for v in m))
    return intro - fight


if __name__ == '__main__':
    main(sys.argv[1] if len(sys.argv) > 1 else 'audio.wav')
