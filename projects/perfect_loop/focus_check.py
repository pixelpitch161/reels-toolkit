"""check every frame without rendering: the worst blur on any bob (centre and rim) must be 0"""
import os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
import numpy as np
import render as R
import timeline as T
worst = []
for f in range(T.NF):
    cam, P, th = R.scene_state(f / T.FPS)
    zf, sharp, K = R.focus(cam, P)
    worst.append(R.subject_blur(cam, P, zf, sharp, K))
worst = np.array(worst)
print('frames with subject blur > 0:', int((worst > 0).sum()), ' max', worst.max().round(4))
