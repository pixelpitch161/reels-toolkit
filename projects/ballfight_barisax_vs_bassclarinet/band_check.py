"""Band check for #40: each fighter's band (bari 60-500 Hz: its swing honks + the pump, the bass line; bass clarinet
150-900 Hz: its counter-line an octave up, G3-D4 fundamentals + the chalumeau formant) over the bed (drums, metal,
grind, fx), per section, from the stems audio.py writes to check/. Both must sit clearly on top (the bed has no bass
instrument and no reeds).

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


spans = [(3.9, 10), (10, 20), (20, 32), (32, 44), (44, 50.5)]
BED = ('drums', 'metal', 'grind', 'fx')
for fig, stems, lo, hi in (('bari', ('bari',), 60, 500), ('bass clar.', ('bcl',), 150, 900)):
    sr, _ = rd(stems[0])
    f = band(sum(rd(k)[1] for k in stems), sr, lo, hi)
    bed = sum(band(rd(k)[1], sr, lo, hi) for k in BED)
    print(f'{fig:11s} {lo}-{hi} Hz over the bed (dB), section / while it sounds:',
          ' '.join(f'{db(f[int(a * sr):int(b * sr)]) - db(bed[int(a * sr):int(b * sr)]):+5.1f}'
                   f'/{gated(f[int(a * sr):int(b * sr)], bed[int(a * sr):int(b * sr)], sr):+5.1f}' for a, b in spans))
