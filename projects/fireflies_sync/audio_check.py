"""Audio checks on audio.wav + stems written by audio.py:
- short-term level in 2 s windows (engagement brief: start >= ~-20 dBFS, build ~8 dB, no window > 12 dB under the peak)
- note onsets per second in the opening (>= 3 hits/s)
- the science sound on top: bells + female answers (stems/sci.wav) vs the music (stems/music.wav) in the
  bells' band (1-5 kHz) per section, and the female answers vs the music in their own band (250-700 Hz)
- the loop seam: last 50 ms vs first 50 ms

    python audio_check.py"""
import numpy as np
from scipy.io import wavfile
from scipy.signal import butter, sosfilt
import timeline as T
import sim as M


def load(p):
    sr, x = wavfile.read(p)
    return sr, x.astype(float) / 32768


def db(y):
    return 20 * np.log10(np.sqrt((y ** 2).mean()) + 1e-12)


sr, mix = load('audio.wav')
m = mix.mean(1)
w = [db(m[int(a * sr):int((a + 2) * sr)]) for a in np.arange(0, 60, 2)]
pk = max(w)
print('2 s windows (dBFS):', ' '.join(f'{v:5.1f}' for v in w))
print(f'  first 2 s {w[0]:.1f}  loudest {pk:.1f}  quietest {min(w):.1f}  -> range {pk - min(w):.1f} dB (limit 12)')
n0 = ((M.EV_T >= 0) & (M.EV_T < 4)).sum()
print(f'opening flashes (= bell notes) 0-4 s: {n0 / 4:.1f}/s')
_, sci = load('stems/sci.wav')
_, mus = load('stems/music.wav')
band = lambda x, lo, hi: sosfilt(butter(4, [lo, hi], 'bp', fs=sr, output='sos'), x.mean(1))
bs, bm = band(sci, 1000, 5000), band(mus, 1000, 5000)
print('bells vs music in 1-5 kHz:')
for a, b in [(0, 4), (4, 8), (8, 12.2), (12.2, 16.6), (16.6, 21), (21, 25.4), (30, 34.2), (34.2, 38.6), (42, 47.4), (47.4, 51.8), (56, 60)]:
    s0, s1 = int(a * sr), int(b * sr)
    print(f'  {a:5.1f}-{b:5.1f}s  bells {db(bs[s0:s1]):6.1f}  music {db(bm[s0:s1]):6.1f}  -> {db(bs[s0:s1]) - db(bm[s0:s1]):+5.1f} dB')
fs, fm = band(sci, 250, 700), band(mus, 250, 700)
d = []
for t in M.FE_T:
    s0, s1 = int(t * sr), int((t + 0.25) * sr)
    if s1 < len(fs):
        d.append(db(fs[s0:s1]) - db(fm[s0:s1]))
print(f'female answers vs music (250-700 Hz, first 0.25 s): median {np.median(d):+.1f}  min {min(d):+.1f} dB')
k = int(0.05 * sr)
print(f'loop seam: last 50 ms {db(m[-k:]):.1f} dBFS vs first 50 ms {db(m[:k]):.1f} dBFS; sample jump {abs(mix[-1] - mix[0]).max():.4f}')
