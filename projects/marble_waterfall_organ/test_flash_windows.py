import sys, numpy as np
from multiprocessing import Pool
import flash_check as FC

def one(f):
    import render as R
    return FC.rel_lum(R.render(f))

if __name__ == '__main__':
    for t0, t1 in [(39.6, 41.0), (41.7, 42.6), (49.7, 50.6), (51.7, 52.6), (53.7, 54.6)]:
        fs = list(range(int(t0 * 30), int(t1 * 30) + 1))
        with Pool(2) as p:
            Y = np.stack(p.map(one, fs))
        area, flashes, worst = FC.analyse(Y)
        print(f'{t0:5.1f}-{t1:5.1f}s  max share changing {area.max()*100:5.1f}% at {fs[int(area.argmax())]/30:.2f}s  flashes {len(flashes)}')
