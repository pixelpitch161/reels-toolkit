"""render frames of a kit episode or example (from projects/, PYTHONHASHSEED=0):
    python ../tools/ballfight_port/kitshot.py <episode> <out_dir> f1 f2 ...   -> f####.npy + f####.png"""
import os, sys
import numpy as np
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'projects'))
from ballfight.kit import episode
ep = episode.load(sys.argv[1])
from ballfight.kit import picture as P
out = sys.argv[2]
os.makedirs(out, exist_ok=True)
from PIL import Image
for f in map(int, sys.argv[3:]):
    img = P.render(f)
    np.save(os.path.join(out, f'f{f:04d}.npy'), img)
    Image.fromarray(img).save(os.path.join(out, f'f{f:04d}.png'))
    print('frame', f)
