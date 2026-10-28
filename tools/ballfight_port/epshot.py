"""render frames with a delivered episode's own render.py (from projects/, PYTHONHASHSEED=0):
    python ../tools/ballfight_port/epshot.py ballfight_<a>_vs_<b> <out_dir> f1 f2 ...   (the folder is not changed;
    its git-ignored check/ caches may be written)"""
import os, sys, importlib.util
import numpy as np
folder = os.path.abspath(sys.argv[1])
sys.path.insert(0, folder)
os.chdir(folder)
spec = importlib.util.spec_from_file_location('render', os.path.join(folder, 'render.py'))
R = importlib.util.module_from_spec(spec)
sys.modules['render'] = R
spec.loader.exec_module(R)
out = sys.argv[2]
os.makedirs(out, exist_ok=True)
from PIL import Image
for f in map(int, sys.argv[3:]):
    img = R.render(f)
    np.save(os.path.join(out, f'f{f:04d}.npy'), img)
    Image.fromarray(img).save(os.path.join(out, f'f{f:04d}.png'))
    print('frame', f)
