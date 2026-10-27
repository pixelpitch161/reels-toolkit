"""Band check for #38: each fighter's band (bass clarinet 140-600 Hz: its chalumeau rumble; euphonium 300-1100 Hz: its
tenor melody) over the bed (the drumline, the pit, fx), per section, from the stems audio.py writes to check/. Both must
sit clearly on top (mood A: the bed has no low brass or woodwinds).

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


spans = [(3.8, 10), (10, 20), (20, 32), (32, 44), (44, 51.6)]
BED = ('drums', 'pit', 'fx')
for fig, lo, hi in (('bcl', 140, 600), ('euph', 300, 1100)):
    sr, f = rd(fig)
    f = band(f, sr, lo, hi)
    bed = sum(band(rd(k)[1], sr, lo, hi) for k in BED)
    print(f'{fig:5s} {lo}-{hi} Hz over the bed (dB):',
          ' '.join(f'{db(f[int(a * sr):int(b * sr)]) - db(bed[int(a * sr):int(b * sr)]):+5.1f}' for a, b in spans))
