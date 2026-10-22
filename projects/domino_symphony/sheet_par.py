"""contact sheet rendered on 2 processes: python sheet_par.py out.png t1 t2 ..."""
import sys, os
from multiprocessing import Pool
import numpy as np
from PIL import Image
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def one(t):
    import render as R
    return np.asarray(R.render(int(round(t * 30))))


if __name__ == '__main__':
    out, times = sys.argv[1], [float(a) for a in sys.argv[2:]]
    with Pool(2) as p:
        ims = p.map(one, times)
    cols = min(6, len(ims))
    rows = (len(ims) + cols - 1) // cols
    sh = Image.new('RGB', (cols * 366, rows * 646), 'white')
    for k, im in enumerate(ims):
        sh.paste(Image.fromarray(im), ((k % cols) * 366, (k // cols) * 646))
    sh.save(out)
    print(out)
