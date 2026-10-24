"""fighters (uke + acoustic stems) vs the bed (everything musical but fx/solo) in 300-5000 Hz, per section, plus each bed bus"""
import numpy as np
from scipy.io import wavfile
from scipy.signal import butter, sosfilt
SOS = butter(4, [300, 5000], 'bp', fs=48000, output='sos')
def ld(k):
    sr, x = wavfile.read(f'check/stem_{k}.wav'); return sosfilt(SOS, x.astype(float).mean(1) / 32768)
fight = ld('uke') + ld('acoustic')
beds = {k: ld(k) for k in ('perc', 'twang', 'whistle', 'choir', 'bell', 'strings')}
bed = sum(beds.values())
db = lambda y, a, b: 20 * np.log10(np.sqrt((y[int(a * 48000):int(b * 48000)] ** 2).mean()) + 1e-12)
for a, b in ((2, 8), (8, 16), (16, 24), (24, 32), (32, 40), (40, 46), (46, 51.8)):
    print(f'{a:4.0f}-{b:4.1f}  fighters {db(fight, a, b):6.1f}  bed {db(bed, a, b):6.1f}  diff {db(fight, a, b) - db(bed, a, b):5.1f} dB   '
          + '  '.join(f'{k} {db(v, a, b):6.1f}' for k, v in beds.items()))
