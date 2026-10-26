"""Band check for #37: each fighter's band (clarinet 140-700 Hz: its chalumeau line; oboe 900-3500 Hz: its nasal
formants) over the bed (strings, harpsichord, drums, fx), per section, from the stems audio.py writes to check/. Both
must sit clearly on top (mood A: the bed has no woodwinds).

    python band_check.py"""
import numpy as np
from scipy.io import wavfile
from scipy.signal import butter, sosfilt


def rd(k):
    sr, x = wavfile.read(f'check/stem_{k}.wav')
    x = x.astype(float)
    x = x.mean(1) if x.ndim > 1 else x
    return sr, x / 32768


def band(x, sr, lo, hi):
    return sosfilt(butter(4, [lo, hi], 'band', fs=sr, output='sos'), x)


def db(x):
    return 20 * np.log10(np.sqrt((x ** 2).mean()) + 1e-9)


spans = [(3.4, 10), (10, 20), (20, 32), (32, 44), (44, 51.8)]
BED = ('strings', 'harpsi', 'drums', 'fx')
for fig, lo, hi in (('clarinet', 140, 700), ('oboe', 900, 3500)):
    sr, f = rd(fig)
    f = band(f, sr, lo, hi)
    bed = sum(band(rd(k)[1], sr, lo, hi) for k in BED)
    print(f'{fig:9s} {lo}-{hi} Hz over the bed (dB):',
          ' '.join(f'{db(f[int(a * sr):int(b * sr)]) - db(bed[int(a * sr):int(b * sr)]):+5.1f}' for a, b in spans))
