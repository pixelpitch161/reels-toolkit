"""Loop check: does the video loop cleanly AT THE FILE BOUNDARY?

Rule: the loop point is the file boundary. The last frame flows straight into frame 0, and the moment the video
"starts over" (a reset, crack, rewind, new run, everything snapping back) lands ON frame 0: never seconds
before the end. (User feedback on #20 v1: the next run started at 58.6 s, so the video visibly started over
about 1.5 s before the file ended and looped.) Build the last seconds as the run-up INTO the reset, and make
frame 0 the reset itself, with chaos following within a second.

    python -m reelkit.loopcheck render.py audio.wav      # before the full render (renders the last 5 s + first 1 s)
    python -m reelkit.loopcheck out.mp4                  # on the finished file (picture + sound)

Checks (picture compared at 360x640, mean absolute RGB difference between frames):
  seam     last frame -> frame 0 change vs the normal frame-to-frame change         FAIL if > 2.5x
  restart  big jumps (cuts, cracks, resets, white flashes) in the last 5 s: the reset belongs at the seam
                                                                                    FAIL if > 6x, warn if > 4x
  repeat   a frame in the last 5 s that matches frame 0 better than the last frame does
           (the opening plays before the end)                                       FAIL
  audio    sample jump and loudness step across the seam                            FAIL if a click or > 6 dB step
Exit code 1 on any FAIL."""
import argparse
import os
import subprocess
import sys
import tempfile
import numpy as np
from multiprocessing import Pool
from .config import W, H, FPS

TAIL_S, HEAD_S = 5.0, 1.0

_MOD = None


def _init(path, cwd):
    global _MOD
    os.chdir(cwd)
    from .video import _load
    _MOD = _load(path)


def _render(f):
    return f, _MOD.render(f)


def frames_from_render(path, idx, procs=None):
    procs = procs or os.cpu_count() or 2
    with Pool(procs, initializer=_init, initargs=(os.path.abspath(path), os.getcwd())) as P:
        out = dict(P.map(_render, idx, chunksize=4))
    return [out[f] for f in idx]


def _probe_duration(path):
    r = subprocess.run(['ffprobe', '-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', path],
                       check=True, capture_output=True, text=True)
    return float(r.stdout.strip())


def _decode(path, start, dur):
    cmd = ['ffmpeg', '-v', 'error', '-ss', f'{start:.4f}', '-i', path, '-t', f'{dur:.4f}',
           '-vf', f'scale={W}:{H}:flags=area', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-']
    raw = subprocess.run(cmd, check=True, capture_output=True).stdout
    return np.frombuffer(raw, np.uint8).reshape(-1, H, W, 3)


def frames_from_mp4(path):
    d = _probe_duration(path)
    tail = _decode(path, max(0.0, d - TAIL_S), TAIL_S + 0.5)
    head = _decode(path, 0.0, HEAD_S)
    return list(tail), list(head), d


def mad(a, b):
    return float(np.abs(a.astype(np.int16) - b.astype(np.int16)).mean())


def check_picture(tail, head, t_tail0, end=60.0):
    steps = [mad(a, b) for a, b in zip(tail[:-1], tail[1:])] + [mad(a, b) for a, b in zip(head[:-1], head[1:])]
    normal = max(float(np.median(steps)), 0.5)
    seam = mad(tail[-1], head[0])
    res, fails = [], 0
    r = seam / normal
    ok = r <= 2.5
    fails += not ok
    res.append(f"  {'OK  ' if ok else 'FAIL'} seam: last frame -> frame 0 changes {seam:.2f} vs normal step {normal:.2f} ({r:.1f}x, limit 2.5x)")
    big = [(i, s / normal) for i, s in enumerate(steps) if s / normal > 4]          # tail and head
    ntail = len(tail) - 1
    jumps = [(i, q) for i, q in big if i < ntail]
    # a reset is a one-off; jumps that recur at a similar size (beat pulses, flashes on every bar) are rhythm
    recurring = lambda q: sum(1 for _, q2 in big if q / 1.6 <= q2 <= q * 1.6) >= 3
    n_pulse = 0
    for i, q in jumps:
        t = t_tail0 + (i + 1) / FPS
        if recurring(q):
            n_pulse += 1
            continue
        bad = q > 6
        fails += bad
        res.append(f"  {'FAIL' if bad else 'warn'} restart: a one-off {q:.0f}x jump at {t:.2f} s ({end - t:.2f} s before the end). "
                   "If this is the reset / restart, move it onto frame 0 (the file boundary)")
    if n_pulse:
        res.append(f'  OK   restart: {n_pulse} recurring jumps in the last 5 s look like beat pulses, not a reset')
    if not jumps:
        res.append('  OK   restart: no big jump in the last 5 s (the reset sits at the seam)')
    d0 = [mad(f, head[0]) for f in tail[:-FPS]]          # ignore the final second (a held end pose is fine)
    if d0:
        i = int(np.argmin(d0))
        if r > 1.5 and d0[i] < 0.5 * seam and d0[i] < 3.0:     # a smooth seam is fine even if the scene repeats
            fails += 1
            t = t_tail0 + i / FPS
            res.append(f'  FAIL repeat: the frame at {t:.2f} s already looks like frame 0 (diff {d0[i]:.2f} vs {seam:.2f} at the seam): '
                       'the opening plays before the end')
        else:
            res.append('  OK   repeat: no copy of the opening inside the last 5 s')
    return fails, res


def check_audio(wav_path):
    from scipy.io import wavfile
    sr, x = wavfile.read(wav_path)
    x = x.astype(np.float64)
    if x.ndim > 1:
        x = x.mean(1)
    if np.abs(x).max() > 2:
        x /= 32768.0
    n = int(0.02 * sr)
    a, b = x[-n:], x[:n]
    lv = lambda s: 20 * np.log10(np.sqrt((s ** 2).mean()) + 1e-9)
    step = abs(lv(a) - lv(b))
    typical = np.median(np.abs(np.diff(x[: sr])))
    jump = abs(x[-1] - x[0])
    click = jump > max(0.08, 12 * typical)
    fails = int(step > 6) + int(click)
    res = [f"  {'FAIL' if step > 6 else 'OK  '} audio level across the seam: last 20 ms {lv(a):.1f} dBFS vs first 20 ms {lv(b):.1f} ({step:.1f} dB, limit 6)",
           f"  {'FAIL' if click else 'OK  '} audio click: sample jump {jump:.3f} at the seam (typical step {typical:.4f})"]
    return fails, res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('src', help='render.py (before the full render) or a finished .mp4')
    ap.add_argument('audio', nargs='?', default=None, help='audio.wav (optional with render.py)')
    ap.add_argument('--procs', type=int, default=None)
    a = ap.parse_args()
    fails, lines = 0, []
    if a.src.endswith('.py'):
        from .video import project_nf
        NF = project_nf(a.src)
        nt, nh = int(TAIL_S * FPS), int(HEAD_S * FPS)
        idx = list(range(NF - nt, NF)) + list(range(nh))
        fr = frames_from_render(a.src, idx, a.procs)
        tail, head, t0 = fr[:nt], fr[nt:], (NF - nt) / FPS
        end = NF / FPS
        wav = a.audio
    else:
        tail, head, d = frames_from_mp4(a.src)
        t0, end = d - len(tail) / FPS, d
        tmp = tempfile.mkdtemp()
        wav = os.path.join(tmp, 'a.wav')
        has_audio = subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', a.src, '-vn', '-ac', '1', '-ar', '48000', wav]).returncode == 0
        wav = wav if has_audio else None
    f, r = check_picture(tail, head, t0, end)
    fails += f
    lines += r
    if wav:
        f, r = check_audio(wav)
        fails += f
        lines += r
    print('loop check (the loop point must be the file boundary):')
    print('\n'.join(lines))
    print('PASS' if not fails else f'{fails} FAIL(s)')
    sys.exit(1 if fails else 0)


if __name__ == '__main__':
    main()
