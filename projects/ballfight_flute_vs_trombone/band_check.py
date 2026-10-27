"""Band check for #41: each fighter's band (TROMBONE 70-700 Hz: the bass line E2-C3 + its brass harmonics and smears;
FLUTE 600-3200 Hz: the high melody E5-F#6 + its breath) over the bed (drums, calliope, glockenspiel, fx), per section,
from the stems audio.py writes to check/. Both must sit clearly on top (the bed has no brass and no flute).

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


spans = [(3.6, 10), (10, 20), (20, 32), (32, 44), (44, 50.0)]
BED = ('drums', 'calliope', 'glock', 'fx')
for fig, stems, lo, hi in (('trombone', ('trombone',), 70, 700), ('flute', ('flute',), 600, 3200)):
    sr, _ = rd(stems[0])
    f = band(sum(rd(k)[1] for k in stems), sr, lo, hi)
    bed = sum(band(rd(k)[1], sr, lo, hi) for k in BED)
    print(f'{fig:9s} {lo}-{hi} Hz over the bed (dB), section / while it sounds:',
          ' '.join(f'{db(f[int(a * sr):int(b * sr)]) - db(bed[int(a * sr):int(b * sr)]):+5.1f}'
                   f'/{gated(f[int(a * sr):int(b * sr)], bed[int(a * sr):int(b * sr)], sr):+5.1f}' for a, b in spans))
