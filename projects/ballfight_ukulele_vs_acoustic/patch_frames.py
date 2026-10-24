"""Re-render frames A:B (seconds) into the kept raw (frames_raw.rgb), then re-encode with mux.py. No full render.

    python patch_frames.py 60.4 61.0 && python mux.py"""
import os
import sys
from multiprocessing import Pool
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(1, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS


def _work(args):
    k, n, f0, f1, nf = args
    import render
    mm = np.memmap(os.path.join(HERE, 'frames_raw.rgb'), dtype=np.uint8, mode='r+', shape=(nf, H, W, 3))
    for f in range(f0 + k, f1, n):
        mm[f] = render.render(f)
    mm.flush()


if __name__ == '__main__':
    s0, s1 = float(sys.argv[1]), float(sys.argv[2])
    nf = os.path.getsize(os.path.join(HERE, 'frames_raw.rgb')) // (W * H * 3)
    f0, f1 = max(0, int(round(s0 * FPS))), min(nf, int(round(s1 * FPS)))
    with Pool(2) as P:
        P.map(_work, [(k, 2, f0, f1, nf) for k in range(2)])
    print(f'patched frames {f0}-{f1 - 1} of {nf}')
