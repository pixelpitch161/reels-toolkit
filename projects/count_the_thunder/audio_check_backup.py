"""Band checks on the stems written by audio.py:
- every thunder must sit above the music in the low band (40-400 Hz) for its first second
- every count tick must sit above the music in its band (1.5-4 kHz)
- phone check: thunder energy above 250 Hz (small speakers) at each strike, relative to the climax

    python audio_check.py"""
import numpy as np
from scipy.io import wavfile
from scipy.signal import butter, sosfilt
import timeline as T


def load(p):
    sr, x = wavfile.read(p)
    return sr, x.astype(float).mean(1) / 32768


def band(x, sr, lo, hi):
    return sosfilt(butter(4, [lo, hi], 'bp', fs=sr, output='sos'), x)


def db(y):
    return 20 * np.log10(np.sqrt((y ** 2).mean()) + 1e-12)


sr, sci = load('stems/thunder.wav')
_, cnt = load('stems/count.wav')
_, mus = load('stems/music.wav')
lo_s, lo_m = band(sci, sr, 40, 400), band(mus, sr, 40, 400)
print('thunder vs music, 40-400 Hz, first 1.0 s after each thunder:')
worst = 99
for i in range(T.N_STRIKES):
    a = T.thunder_t(i)
    s0, s1 = int(a * sr), int((a + 1.0) * sr)
    d = db(lo_s[s0:s1]) - db(lo_m[s0:s1])
    worst = min(worst, d)
    print(f'  strike {i:2d} @ {a:6.3f}s  {T.true_km(i) * 1000:5.0f} m   thunder {db(lo_s[s0:s1]):6.1f}  music {db(lo_m[s0:s1]):6.1f}  -> {d:+5.1f} dB')
print(f'  worst {worst:+.1f} dB')
hi_c, hi_m = band(cnt, sr, 1500, 4000), band(mus, sr, 1500, 4000)
ds = []
for tt, k in T.count_ticks():
    s0, s1 = int(tt * sr), int((tt + 0.08) * sr)
    ds.append(db(hi_c[s0:s1]) - db(hi_m[s0:s1]))
print(f'count ticks vs music, 1.5-4 kHz: min {min(ds):+.1f}  median {np.median(ds):+.1f} dB')
ph = band(sci, sr, 250, 4000)
ref = None
vals = []
for i in range(T.N_STRIKES):
    a = T.thunder_t(i)
    s0, s1 = int(a * sr), int((a + 0.4) * sr)
    v = 20 * np.log10(np.abs(ph[s0:s1]).max() + 1e-12)
    vals.append(v)
clim = vals[-1]
print('phone band (250 Hz-4 kHz) thunder peak vs climax:', ' '.join(f'{v - clim:+.1f}' for v in vals))
