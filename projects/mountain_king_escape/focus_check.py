"""Locked-focus audit over all 1800 frames (no rendering): every intact ring (24 points round each, front and
back edge) and the ball must have circle of confusion 0 whenever they are on screen.

    python focus_check.py"""
import os
import sys
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
import render as R
import timeline as T

worst = np.array([R.subject_blur(f / T.FPS) for f in range(T.NF)])
print(f'frames with subject blur > 0: {int((worst > 0).sum())} of {T.NF}   max CoC {worst.max():.3f} px')
if (worst > 0).any():
    print('worst at', [round(f / T.FPS, 2) for f in np.argsort(-worst)[:8]])
