"""approximate short-term loudness (K-weighting: high-pass + +4 dB shelf above 1.5 kHz), per 2 s window.
    python loudness_check.py audio.wav"""
import sys
import numpy as np
from scipy.io import wavfile
from scipy.signal import butter, sosfilt
sr, x = wavfile.read(sys.argv[1])
x = x.astype(float) / 32768
hp = butter(2, 60, 'hp', fs=sr, output='sos')
sh = butter(2, 1500, 'hp', fs=sr, output='sos')
y = sosfilt(hp, x, axis=0)
y = y + 0.585 * sosfilt(sh, y, axis=0)
w = 2 * sr
vals = []
for a in range(0, len(y) - w + 1, w):
    seg = y[a:a + w]
    vals.append(-0.691 + 10 * np.log10((seg ** 2).mean(0).sum() + 1e-12))
print('  '.join(f'{2*i:2d}:{v:5.1f}' for i, v in enumerate(vals)))
