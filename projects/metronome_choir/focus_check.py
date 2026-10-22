"""Locked-focus audit over all 1800 frames (no rendering): every metronome on screen (base and top of all 199
plus the soloist) must have circle of confusion 0 on every frame.

    python focus_check.py"""
import os
import sys
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
import render as R
import motion as MO
import timeline as T


class Lite:
    def __init__(self, t):
        self.t = t
        self.cam = R.camera(t)
        self.P = MO.metronome_bases(t)[0]


worst = np.array([R.subject_blur(Lite(f / T.FPS)) for f in range(T.NF)])
print(f'frames with subject blur > 0: {int((worst > 0).sum())} of {T.NF}   max CoC {worst.max():.3f} px')
