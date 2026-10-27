"""Band check for #42: each fighter's band (VIOLA 120-1500 Hz: its inner voice / whole notes C3-G4 + their harmonics;
VIOLIN 350-4000 Hz: the melody G4-C6 + its bright harmonics) over the bed (drums, hats, 808, cellos, basses, fx), per
section, from the stems audio.py writes to check/. Both must sit clearly on top (the bed has no violin or viola). Also
#37's lesson: no 0.5 s window in which a fighter's stem goes silent while it is fighting (after its first sound).

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


import timeline as T
spans = [(T.T_FIGHT, 10), (10, 20), (20, 32), (32, 44), (44, 50.4)]
BED = ('drums', 'hats', 'bass808', 'cellos', 'basses', 'fx')
for fig, stems, lo, hi in (('viola', ('viola',), 120, 1500), ('violin', ('violin',), 350, 4000)):
    sr, _ = rd(stems[0])
    raw = sum(rd(k)[1] for k in stems)
    f = band(raw, sr, lo, hi)
    bed = sum(band(rd(k)[1], sr, lo, hi) for k in BED)
    print(f'{fig:7s} {lo}-{hi} Hz over the bed (dB), section / while it sounds:',
          ' '.join(f'{db(f[int(a * sr):int(b * sr)]) - db(bed[int(a * sr):int(b * sr)]):+5.1f}'
                   f'/{gated(f[int(a * sr):int(b * sr)], bed[int(a * sr):int(b * sr)], sr):+5.1f}' for a, b in spans))
    w = int(0.5 * sr)
    e = np.array([np.sqrt((raw[i:i + w] ** 2).mean()) for i in range(int(T.T_FIGHT * sr), int(50.4 * sr) - w, w // 2)])
    pk = e.max()
    quiet = [round(T.T_FIGHT + j * 0.25, 2) for j, v in enumerate(e) if v < pk * 10 ** (-40 / 20)]
    print(f'        silent 0.5 s windows (40 dB under its peak) while fighting: {quiet[:12]}{" ..." if len(quiet) > 12 else ""}')
