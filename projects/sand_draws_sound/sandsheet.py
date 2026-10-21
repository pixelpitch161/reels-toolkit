import sys, numpy as np
from PIL import Image, ImageDraw
import timeline as T
d = np.load('drive.npz'); G = int(d['G'])
pos = np.memmap('sand_pos.f32', dtype=np.float32, mode='r', shape=(T.NF, G, 2))
ts = [float(x) for x in sys.argv[2:]]
S = 150; cols = 8
rows = (len(ts) + cols - 1) // cols
sheet = Image.new('RGB', (cols * (S + 4), rows * (S + 14)), (255, 255, 255))
dr = ImageDraw.Draw(sheet)
for k, t in enumerate(ts):
    f = min(T.NF - 1, int(round(t * T.FPS)))
    img = np.full((S, S), 25, np.uint8)
    P = pos[f]
    x = ((P[:, 0] + 1) / 2 * (S - 1)).astype(int); y = ((P[:, 1] + 1) / 2 * (S - 1)).astype(int)
    img[y, x] = 235
    X, Y = (k % cols) * (S + 4), (k // cols) * (S + 14)
    sheet.paste(Image.fromarray(img).convert('RGB'), (X, Y + 12))
    dr.text((X + 2, Y), f"{t:.2f}", fill=(0, 0, 0))
sheet.save(sys.argv[1])
