"""Engagement/level probe: integrated LUFS (ffmpeg), 2 s window RMS (dBFS), onset rate per window.
    python levels.py file.wav [file2.wav ...]"""
import subprocess
import sys
import re
import numpy as np
from scipy.io import wavfile
from scipy.signal import stft


def lufs(p):
    r = subprocess.run(['ffmpeg', '-nostats', '-i', p, '-af', 'ebur128=framelog=quiet', '-f', 'null', '-'],
                       capture_output=True, text=True).stderr
    m = re.findall(r'I:\s+(-?[\d.]+) LUFS', r)
    return float(m[-1]) if m else float('nan')


def onsets(x, sr):
    """audible hits: 5 ms-frame energy jumps >= 6 dB over the previous 40 ms in any of 3 bands,
    and the hit is within 20 dB of the loudest 2 s of the file in that band (rain grain doesn't count)"""
    from scipy.signal import butter, sosfilt
    hop = int(0.005 * sr)
    hits = []
    for lo, hi in ((30, 250), (250, 2500), (2500, 12000)):
        y = sosfilt(butter(4, [lo, hi], 'bp', fs=sr, output='sos'), x)
        n = len(y) // hop
        e = 10 * np.log10((y[:n * hop].reshape(n, hop) ** 2).mean(1) + 1e-12)
        ref = np.percentile(e, 99.5)
        last = -99
        for i in range(8, n):
            prev = 10 * np.log10(np.mean(10 ** (e[i - 8:i] / 10)))
            if e[i] - prev >= 6 and e[i] > ref - 20 and i - last > 18:
                hits.append(i * hop / sr)
                last = i
    hits = np.sort(np.array(hits))
    out = []
    for h in hits:
        if not out or h - out[-1] > 0.06:
            out.append(h)
    return np.array(out)


for p in sys.argv[1:]:
    sr, x = wavfile.read(p)
    x = x.astype(float).mean(1) / 32768
    on = onsets(x, sr)
    print(p.split('/')[-2:], f'I = {lufs(p):.1f} LUFS')
    rows = []
    for a in np.arange(0, len(x) / sr - 1.99, 2.0):
        seg = x[int(a * sr):int((a + 2) * sr)]
        r = 20 * np.log10(np.sqrt((seg ** 2).mean()) + 1e-12)
        n = ((on >= a) & (on < a + 2)).sum() / 2
        rows.append((a, r, n))
    pk = max(r for _, r, _ in rows)
    print('  ' + '  '.join(f'{a:2.0f}:{r:5.1f}/{n:3.1f}' for a, r, n in rows))
    print(f'  peak window {pk:.1f} dBFS; min {min(r for _, r, _ in rows):.1f}; windows >12 dB under peak: {[int(a) for a, r, _ in rows if r < pk - 12]}')
