"""Locked-focus audit over all 1800 frames (no rendering): every on-screen part of the machine (plinth rim,
tiers, every playable element, every gate) must have circle of confusion 0 on every frame.

    python focus_check.py"""
import os
import sys
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
import render as R
import timeline as T

worst = np.array([R.subject_blur(R.camera(f / T.FPS)) for f in range(T.NF)])
print(f'frames with subject blur > 0: {int((worst > 0).sum())} of {T.NF}   max CoC {worst.max():.3f} px')
