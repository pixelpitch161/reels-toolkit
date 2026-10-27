"""Band check for #39: each fighter's band (bassoon 300-1400 Hz: its staccato line's fundamentals + the ~480 / ~1150 Hz
formants; bari 60-500 Hz: its honks, with and without the ropes it rattles) over the bed (drums, guitar, organ, fx),
per section, from the stems audio.py writes to check/. Both must sit clearly on top (mood A: the bed has no horns).

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


def gated(f, bed, sr):
    """the fighter over the bed only while it sounds (its 20 ms envelope within 20 dB of its section peak)"""
    w = int(0.02 * sr)
    e = np.sqrt(np.convolve(f ** 2, np.ones(w) / w, 'same'))
    on = e > e.max() * 0.1
    return db(f[on]) - db(bed[on]) if on.any() else float('nan')


spans = [(2.9, 10), (10, 20), (20, 32), (32, 44), (44, 51.6)]
BED = ('drums', 'guitar', 'organ', 'fx')
for fig, stems, lo, hi in (('bassoon', ('bassoon',), 300, 1400), ('bari', ('bari',), 60, 500),
                           ('bari+ropes', ('bari', 'ropes'), 60, 500)):
    sr, _ = rd(stems[0])
    f = band(sum(rd(k)[1] for k in stems), sr, lo, hi)
    bed = sum(band(rd(k)[1], sr, lo, hi) for k in BED)
    print(f'{fig:11s} {lo}-{hi} Hz over the bed (dB), section / while it sounds:',
          ' '.join(f'{db(f[int(a * sr):int(b * sr)]) - db(bed[int(a * sr):int(b * sr)]):+5.1f}'
                   f'/{gated(f[int(a * sr):int(b * sr)], bed[int(a * sr):int(b * sr)], sr):+5.1f}' for a, b in spans))
