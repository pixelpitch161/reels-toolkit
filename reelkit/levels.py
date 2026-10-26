"""Loudness check for a soundtrack: loud and busy from frame 0, CONSISTENT loudness all the way, never quiet.
User, Oct 26: "the videos start out too quiet. The ending loudness is fine, but it should be more consistent overall."
(The old rule asked for a +3 dB build from the first half to the second, which pushed openings down: replaced. Build
energy with the arrangement - density, register, brightness, more parts - not with level.)

    python -m reelkit.levels audio.wav            # or a finished .mp4

Prints loudness (RMS dBFS) and rough hits per second in 2 s windows, then a summary against the ending (the user's
reference level = the average of the last 20 s):
  opening   first 10 s average            target: within 1.5 dB of the ending
  spread    every 2 s window              target: none more than 3 dB below the ending (one deliberate dip allowed,
                                          e.g. a K.O. hit-stop); loudest minus quietest window <= 5 dB
  build     30-60 s average minus 0-30 s  target: <= +1.5 dB (more means the start is too quiet)
Exit code 1 if any target fails. Hit counts are rough (very dense plucks blur together and under-count); loudness is
the reliable column."""
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
    ref = r[-10:].mean()                                  # the ending (last 20 s): the level the user is happy with
    opening, build = r[:5].mean(), r[half:].mean() - r[:half].mean()
    dips = [(t, v) for t, v, _ in rows if v < ref - 3.0]
    ok_open = opening >= ref - 1.5
    ok_dips = len(dips) <= 1
    rest = np.delete(r, np.argmin(r)) if len(dips) == 1 else r   # the one allowed dip doesn't count toward the spread
    spread = np.ptp(rest)
    ok_spread = spread <= 5.0
    ok_build = build <= 1.5
    print(f'ending (last 20 s): {ref:.1f} dBFS  (the reference level)')
    print(f'opening (0-10 s): {opening:.1f} dBFS ({opening - ref:+.1f} dB vs ending), {h[:5].mean():.1f} hits/s   '
          f'{"OK" if ok_open else "TOO QUIET (target within 1.5 dB of the ending)"}')
    print(f'build (30-60 s vs 0-30 s): {build:+.1f} dB   {"OK" if ok_build else "TOO MUCH (target <= +1.5 dB: lift the start, build with the arrangement)"}')
    print(f'spread (loudest - quietest 2 s window, the one allowed dip excluded): {spread:.1f} dB   {"OK" if ok_spread else "UNEVEN (target <= 5 dB)"}')
    print(f'dips (> 3 dB below the ending): {len(dips)}' + ('' if not dips else '  at ' + ', '.join(f'{t:.0f} s ({v - ref:+.1f})' for t, v in dips))
          + ('   OK' if ok_dips else '   TOO MANY (one deliberate dip allowed)'))
    passed = ok_open and ok_dips and ok_spread and ok_build
    print('PASS' if passed else 'FAIL')
    sys.exit(0 if passed else 1)


if __name__ == '__main__':
    main()
