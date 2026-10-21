"""Contact sheets straight from a render module (fast visual checks before a full render).

    python -m reelkit.sheet render.py test/sheet.png 1.0 6.5 13 26 45 58.5
    python -m reelkit.sheet render.py test/frame.png --frame 26.0      # one full-size frame"""
import argparse
import os
import numpy as np
from PIL import Image
from .config import W, H, FPS, UP
from .video import _load


def sheet(mod, times, out, cols=7):
    rows = (len(times) + cols - 1) // cols
    img = Image.new('RGB', (cols * (W + 6), rows * (H + 6)), (255, 255, 255))
    for k, t in enumerate(times):
        img.paste(Image.fromarray(mod.render(int(round(t * FPS)))), ((k % cols) * (W + 6), (k // cols) * (H + 6)))
    img.save(out)
    return out


def frame(mod, t, out):
    Image.fromarray(mod.render(int(round(t * FPS)))).resize((W * UP, H * UP), Image.NEAREST).save(out)
    return out


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('render_py')
    ap.add_argument('out')
    ap.add_argument('times', nargs='*', type=float)
    ap.add_argument('--frame', type=float, default=None)
    a = ap.parse_args()
    os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
    mod = _load(a.render_py)
    if a.frame is not None:
        print(frame(mod, a.frame, a.out))
    else:
        print(sheet(mod, a.times, a.out))
