"""Engagement check for a soundtrack: is it loud and busy from frame 0, does it build, does it ever go quiet?
(The user's brief: chaos and sound from frame 0, never quiet, build to the payoff. Reference: 100 Metronomes,
the best performer, runs about -18.5 dBFS and ~4 hits/s in its first 10 s and builds ~8 dB into the back half.)

    python -m reelkit.levels audio.wav            # or a finished .mp4

Prints loudness (RMS dBFS) and rough hits per second in 2 s windows, then a summary:
  opening   first 10 s average            target >= -20 dBFS
  build     30-60 s average minus 0-30 s  target >= +3 dB
  quiet     2 s windows more than 12 dB below the loudest window (a short deliberate breath is fine)
Hit counts are rough (very dense plucks blur together and under-count); loudness is the reliable column."""
import argparse
import os
import subprocess
import sys
import tempfile
import numpy as np
from scipy.io import wavfile
from scipy.signal import find_peaks

WIN = 2.0


def load(path):
    if not path.endswith('.wav'):
        tmp = os.path.join(tempfile.mkdtemp(), 'a.wav')
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', path, '-vn', '-ac', '1', '-ar', '48000', tmp], check=True)
        path = tmp
    sr, x = wavfile.read(path)
    x = x.astype(np.float64)
    if x.ndim > 1:
        x = x.mean(1)
    if np.abs(x).max() > 2:
        x /= 32768.0
    return sr, x


def profile(sr, x, dur=60.0):
    hop = int(sr * 0.01)
    fr = np.sqrt(np.array([(x[i:i + hop] ** 2).mean() for i in range(0, len(x) - hop, hop)]) + 1e-12)
    db = 20 * np.log10(fr)
    prev = np.array([db[max(0, i - 5):i].min() if i else db[0] for i in range(len(db))])
    pk, _ = find_peaks(db - prev, height=6.0, distance=4)
    pk = pk[db[pk] > -45]
    rows = []
    for w in range(int(dur / WIN)):
        a, b = int(w * WIN * sr), int((w + 1) * WIN * sr)
        seg = x[a:b]
        rms = 20 * np.log10(np.sqrt((seg ** 2).mean()) + 1e-12) if len(seg) else -120.0
        hits = int(((pk * 0.01 >= w * WIN) & (pk * 0.01 < (w + 1) * WIN)).sum())
        rows.append((w * WIN, rms, hits / WIN))
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('src', help='audio.wav or a finished video')
    a = ap.parse_args()
    sr, x = load(a.src)
    rows = profile(sr, x, dur=round(len(x) / sr / WIN) * WIN)
    print(' window   dBFS  hits/s')
    for t, r, h in rows:
        print(f'{t:4.0f}-{t + WIN:<3.0f} {r:6.1f} {h:6.1f}')
    r = np.array([v[1] for v in rows])
    h = np.array([v[2] for v in rows])
    half = len(r) // 2
    opening, build = r[:5].mean(), r[half:].mean() - r[:half].mean()
    quiet = [(t, v) for t, v, _ in rows if v < r.max() - 12]
    print(f'opening (0-10 s): {opening:.1f} dBFS, {h[:5].mean():.1f} hits/s   {"OK" if opening >= -20 else "LOW (target >= -20)"}')
    print(f'build (30-60 s vs 0-30 s): {build:+.1f} dB   {"OK" if build >= 3 else "FLAT (target >= +3 dB)"}')
    print(f'quiet windows (> 12 dB below the loudest): {len(quiet)}' + ('' if not quiet else '  at ' + ', '.join(f'{t:.0f} s' for t, _ in quiet)))
    sys.exit(0)


if __name__ == '__main__':
    main()
