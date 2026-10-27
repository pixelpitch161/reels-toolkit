"""Flash test on a stretch before the full render (BALLFIGHT v2 rule: bright bursts small, <= 3 / s, no full-screen
flash): render frames A..B s in 2 processes and run reelkit.flashcheck.analyse on their luminance.

    python flash_test.py 39.5 50.0     (#38: RHAPSODY, ILL WIND, ENDLESS NOTE, the K.O.)"""
import os
import sys
from multiprocessing import Pool
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(1, os.path.join(HERE, '..', '..'))


def _lum(f):
    import render
    from reelkit.flashcheck import rel_lum
    return rel_lum(render.render(f))


if __name__ == '__main__':
    from reelkit.flashcheck import analyse
    from reelkit.config import FPS
    f0, f1 = int(float(sys.argv[1]) * FPS), int(float(sys.argv[2]) * FPS)
    with Pool(2) as P:
        Y = np.array(P.map(_lum, range(f0, f1)))
    area, flashes, worst = analyse(Y)
    i = int(np.argmax(area))
    print(f'{f1 - f0} frames: general flashes {len(flashes)}, worst in any 1 s {worst}; '
          f'largest share of frame changing >= 0.1: {100 * area[i]:.1f} % at {(f0 + i) / FPS:.2f} s')
