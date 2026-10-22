"""Audio checks on audio.wav + the stems written by audio.py:
- integrated loudness, 2 s window RMS (engagement brief: no window more than 12 dB under the loudest, a strong
  opening), audible hits per second (energy jumps >= 6 dB in 3 bands, within 20 dB of the loudest)
- the science sound on top: bells + soloist + ticks (stems/sci.wav) vs the music (stems/music.wav) in the bells'
  band (250-3000 Hz) per section, and right at each locked tick (15 ms onsets) after the drop
- the loop seam: last 50 ms vs first 50 ms

    python audio_check.py"""
import re
import subprocess
import numpy as np
from scipy.io import wavfile
from scipy.signal import butter, sosfilt
import timeline as T


def load(p):
    sr, x = wavfile.read(p)
    return sr, x.astype(float) / 32768


def db(y):
    return 20 * np.log10(np.sqrt((y ** 2).mean()) + 1e-12)


def lufs(p):
    r = subprocess.run(['ffmpeg', '-nostats', '-i', p, '-af', 'ebur128=framelog=quiet', '-f', 'null', '-'], capture_output=True, text=True).stderr
    m = re.findall(r'I:\s+(-?[\d.]+) LUFS', r)
    return float(m[-1]) if m else float('nan')


def onsets(x, sr):
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


sr, mix = load('audio.wav')
m = mix.mean(1)
print(f'integrated loudness {lufs("audio.wav"):.1f} LUFS')
on = onsets(m, sr)
rows = []
for a in np.arange(0, 60, 2):
    seg = m[int(a * sr):int((a + 2) * sr)]
    rows.append((a, db(seg), ((on >= a) & (on < a + 2)).sum() / 2))
pk = max(r for _, r, _ in rows)
print('2 s windows  t:dBFS/hits per s')
print('  ' + '  '.join(f'{a:2.0f}:{r:5.1f}/{n:3.1f}' for a, r, n in rows[:15]))
print('  ' + '  '.join(f'{a:2.0f}:{r:5.1f}/{n:3.1f}' for a, r, n in rows[15:]))
print(f'  loudest {pk:.1f}, quietest {min(r for _, r, _ in rows):.1f}; windows > 12 dB under the loudest: {[int(a) for a, r, _ in rows if r < pk - 12]}')
_, sci = load('stems/sci.wav')
_, mus = load('stems/music.wav')
band = lambda x, lo, hi: sosfilt(butter(4, [lo, hi], 'bp', fs=sr, output='sos'), x.mean(1))
bs, bm = band(sci, 250, 3000), band(mus, 250, 3000)
print('bells vs music, 250-3000 Hz:')
for a, b in [(0, 6), (6, 13), (13, 21), (21, 25), (25, 30), (30, 34), (34, 38), (38, 46), (46, 52), (52, 58), (58, 60)]:
    s0, s1 = int(a * sr), int(b * sr)
    print(f'  {a:4.0f}-{b:4.0f}s  bells {db(bs[s0:s1]):6.1f}  music {db(bm[s0:s1]):6.1f}  -> {db(bs[s0:s1]) - db(bm[s0:s1]):+5.1f} dB')
d = []
for k in range(int((T.T_JOLT - T.T_DROP) / T.BEAT)):
    t = T.T_DROP + k * T.BEAT
    s0, s1 = int(t * sr), int((t + 0.015) * sr)
    d.append(db(bs[s0:s1]) - db(bm[s0:s1]))
print(f'locked ticks after the drop (first 15 ms): median {np.median(d):+.1f} dB, min {min(d):+.1f} dB over the music')
k = int(0.05 * sr)
print(f'loop seam: last 50 ms {db(m[-k:]):.1f} dBFS vs first 50 ms {db(m[:k]):.1f} dBFS; sample jump {abs(mix[-1] - mix[0]).max():.4f}')
