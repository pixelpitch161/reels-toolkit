"""render frames at given times and tile a crop of the ring at 2x (for checking details)"""
import os, sys, numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from PIL import Image
import render as R
ts = [float(x) for x in sys.argv[2:]]
tiles = []
for t in ts:
    img = R.render(int(round(t * 30)))
    crop = img[150:470, 20:340]
    tiles.append(np.kron(crop, np.ones((2, 2, 1), np.uint8)))
cols = 3
rows = -(-len(tiles) // cols)
h, w = tiles[0].shape[:2]
out = np.zeros((rows * h, cols * w, 3), np.uint8)
for j, tl in enumerate(tiles):
    out[(j // cols) * h:(j // cols + 1) * h, (j % cols) * w:(j % cols + 1) * w] = tl
Image.fromarray(out).save(sys.argv[1])
