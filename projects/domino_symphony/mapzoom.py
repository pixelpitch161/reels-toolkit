"""zoomed top-down map of layout.npz: python mapzoom.py out.png x0 x1 z0 z1 [scale]"""
import sys
import numpy as np
from PIL import Image, ImageDraw
c = dict(np.load('layout.npz'))
out, x0, x1, z0, z1 = sys.argv[1], *map(float, sys.argv[2:6])
sc = float(sys.argv[6]) if len(sys.argv) > 6 else 400.0
W, H = int((x1 - x0) * sc), int((z1 - z0) * sc)
im = Image.new('RGB', (W, H), (196, 150, 100))
d = ImageDraw.Draw(im)
tp = lambda x, z: ((x - x0) * sc, (z - z0) * sc)
sel = np.where((c['cx'] > x0 - 0.2) & (c['cx'] < x1 + 0.2) & (c['cz'] > z0 - 0.2) & (c['cz'] < z1 + 0.2))[0]
for i in sel:
    x, z, fx, fz = c['cx'][i], c['cz'][i], c['fx'][i], c['fz'][i]
    w, th = c['dw'][i] / 2, c['dt'][i] / 2
    lx, lz = fz, -fx
    corners = [(x + fx * th + lx * w, z + fz * th + lz * w), (x + fx * th - lx * w, z + fz * th - lz * w),
               (x - fx * th - lx * w, z - fz * th - lz * w), (x - fx * th + lx * w, z - fz * th + lz * w)]
    col = (int(c['cr'][i]), int(c['cg'][i]), int(c['cb'][i]))
    d.polygon([tp(*p) for p in corners], fill=col, outline=(40, 30, 30) if sc > 300 else None)
im.save(out)
print(out, im.size)
# mark close pairs (different lanes, not chain neighbours)
from scipy.spatial import cKDTree
P = np.stack([c['cx'], c['cz']], 1)
for a, b in cKDTree(P).query_pairs(0.022):
    if (c['lane'][a] == c['lane'][b] and c['lane'][a] not in (900, 901)) or c['nxt'][a] == b or c['nxt'][b] == a:
        continue
    for i in (a, b):
        x, y = tp(c['cx'][i], c['cz'][i])
        d.ellipse([x - 5, y - 5, x + 5, y + 5], outline=(255, 0, 0), width=2)
im.save(out)
