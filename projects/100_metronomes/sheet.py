import sys, numpy as np
from PIL import Image
from render import render, W, H, FPS
ts = [float(x) for x in sys.argv[2:]]
cols = 7
rows = (len(ts) + cols - 1) // cols
sc = 0.5
w, h = int(W * sc * 2), int(H * sc * 2)   # 360x640 thumbnails
sheet = Image.new('RGB', (cols * (w + 6), rows * (h + 6)), (255, 255, 255))
for k, t in enumerate(ts):
    im = Image.fromarray(render(int(round(t * FPS)))).resize((w, h), Image.NEAREST)
    sheet.paste(im, ((k % cols) * (w + 6), (k // cols) * (h + 6)))
sheet.save(sys.argv[1])
print('saved', sys.argv[1])
