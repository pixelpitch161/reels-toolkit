"""Check every frame without rendering: the worst blur (CoC px) on any visible subject point (the nymphs,
the soil face around them, the lantern and the six cicada trees) must be 0. Also compares frame 0 with
the last frame for the seamless loop."""
import os
import sys
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
import numpy as np
import render as R
import timeline as T

worst = np.array([R.subject_blur(R.camera(f / T.FPS)) for f in range(T.NF)])
print('frames with subject blur > 0:', int((worst > 0).sum()), ' max CoC px', worst.max().round(4))
if '--loop' in sys.argv:
    a = R.render(0).astype(float)
    b = R.render(T.NF - 1).astype(float)
    print('loop: mean abs diff frame 0 vs last frame', np.abs(a - b).mean().round(2), '/ 255')
