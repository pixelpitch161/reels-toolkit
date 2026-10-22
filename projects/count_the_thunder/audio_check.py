"""Band checks on the stems written by audio.py:
- the key thunders (counted, lock-in, rod, second cell) must sit above the music in the low band
  (40-400 Hz) for their first second; the chaos smear is reported for information
- every count tick must sit above the music in its band (1.5-4 kHz)
- phone check: thunder energy above 250 Hz (small speakers) at the key strikes, relative to the climax

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
KEY = [i for i in range(T.N_STRIKES) if T.KIND[i] in ('count', 'lock', 'rod', 'cell2')]
print('key thunders vs music, 40-400 Hz, first 1.0 s:')
worst, chaos = 99, []
for i in range(T.N_STRIKES):
    a = T.thunder_t(i)
    if a < 0 or a >= T.DUR - 1.0 or (T.T_CO <= a < T.T_REW):
        continue
    s0, s1 = int(a * sr), int((a + 1.0) * sr)
    d = db(lo_s[s0:s1]) - db(lo_m[s0:s1])
    if i in KEY:
        worst = min(worst, d)
        print(f'  {T.KIND[i]:5s} {i:2d} @ {a:6.3f}s  {T.true_km(i) * 1000:5.0f} m  -> {d:+5.1f} dB')
    else:
        chaos.append(d)
print(f'  worst key {worst:+.1f} dB   chaos smear: median {np.median(chaos):+.1f}, min {min(chaos):+.1f} dB')
hi_c, hi_m = band(cnt, sr, 1500, 4000), band(mus, sr, 1500, 4000)
ds = []
for tt, k in T.count_ticks():
    s0, s1 = int(tt * sr), int((tt + 0.08) * sr)
    ds.append(db(hi_c[s0:s1]) - db(hi_m[s0:s1]))
print(f'count ticks vs music, 1.5-4 kHz: min {min(ds):+.1f}  median {np.median(ds):+.1f} dB')
ph = band(sci, sr, 250, 4000)
vals = []
for i in KEY:
    a = T.thunder_t(i)
    if a >= T.DUR - 0.4:
        continue
    s0, s1 = int(a * sr), int((a + 0.4) * sr)
    vals.append((i, 20 * np.log10(np.abs(ph[s0:s1]).max() + 1e-12)))
clim = dict(vals)[T.I_CLIMAX]
print('phone band (250 Hz-4 kHz) key thunder peak vs climax:', ' '.join(f'{v - clim:+.1f}' for i, v in vals))
