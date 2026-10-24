"""Re-encode the kept raw frames (frames_raw.rgb from reelkit.video) into a loop-clean mp4. No re-render.

Same picture/sound settings as reelkit.video, plus two fixes that only matter at the loop seam:
  - the last frame is forced to be a keyframe like frame 0, so both sides of the seam carry matching compression noise
    (a GOP's last P-frame vs the opening I-frame read as a 3.4x seam jump on this dense dithered board);
  - x264's macroblock tree is off: it gave frame 0 (referenced by everything after it) a finer quantiser than the
    last frame, so the two sides of the seam were coded differently (3.5x at CRF 18; 1.5x with mbtree off + the
    keyframe, tested on the seam frames);
  - AAC encodes 1024-sample frames and 61.0 s isn't a whole number of them, so the encoder would pad the last one with
    13 ms of zeros (a dropout before the loop). audio.py builds the soundtrack as a loop exactly 2860 AAC frames long
    (61.013 s) and it goes in untrimmed, so the decoded audio wraps sample-perfectly. The file reads 61.01 s.

    python mux.py --patch 0:4     # re-render frames 0-4 s into the kept raw first (a fix that touches only those)

    python mux.py                 # -> maze_race.mp4"""
import os
import subprocess
import sys

import numpy as np
from scipy.io import wavfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, UP, FPS


def _patch(args):
    k, nproc, f0, f1, nf = args
    os.chdir(HERE)
    sys.path.insert(0, HERE)
    import render
    mm = np.memmap('frames_raw.rgb', dtype=np.uint8, mode='r+', shape=(nf, H, W, 3))
    for f in range(f0 + k, f1, nproc):
        mm[f] = render.render(f)
    mm.flush()


def patch(f0, f1, raw='frames_raw.rgb'):
    from multiprocessing import Pool
    nf = os.path.getsize(raw) // (W * H * 3)
    n = os.cpu_count() or 2
    with Pool(n) as P:
        P.map(_patch, [(k, n, f0, f1, nf) for k in range(n)])
    print(f'patched frames {f0}-{f1 - 1}')


def main(raw='frames_raw.rgb', audio='audio.wav', out='maze_race.mp4', crf=18):
    nf = os.path.getsize(raw) // (W * H * 3)
    sr, x = wavfile.read(audio)
    if len(x) % 1024:
        raise SystemExit('audio.wav is not a whole number of AAC frames: rebuild it with audio.py')
    loop = audio
    cmd = ['ffmpeg', '-y', '-loglevel', 'error',
           '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', f'{W}x{H}', '-r', str(FPS), '-i', raw,
           '-i', loop,
           '-vf', f'scale={W * UP}:{H * UP}:flags=neighbor',
           '-c:v', 'libx264', '-preset', 'slow', '-crf', str(crf), '-pix_fmt', 'yuv420p',
           '-profile:v', 'high', '-tune', 'animation', '-x264-params', 'mbtree=0',
           '-force_key_frames', f'expr:eq(n,0)+eq(n,{nf - 1})',
           '-c:a', 'aac', '-b:a', '256k', '-ar', '48000',
           '-movflags', '+faststart', out]
    subprocess.run(cmd, check=True)
    print('wrote', out, f'({nf} frames, audio {len(x) / sr:.4f} s = {len(x) // 1024} AAC frames)')


if __name__ == '__main__':
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument('--patch', default=None, help="re-render 'A:B' seconds into the kept raw first")
    a = ap.parse_args()
    if a.patch:
        s0, s1 = (float(v) for v in a.patch.split(':'))
        patch(int(round(s0 * FPS)), int(round(s1 * FPS)))
    main()
