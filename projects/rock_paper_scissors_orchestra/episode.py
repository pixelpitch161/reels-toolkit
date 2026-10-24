"""Episode settings shared by the sim, the seed search, the picture and the sound."""
import os
import sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit.config import loop_bpm
import rps

DUR = 61.0
BPM = loop_bpm(110, DUR)          # 110.16: 28 bars in 61 s, so the pulse is in phase at the loop point
BEAT = 60.0 / BPM
BAR = 4 * BEAT
S16 = BEAT / 4

# balance: tuned so every team wins some runs; the seed search picks the story (never edited)
PARAMS = rps.Params(vmax=1.0, acc=4.5, sprint=1.3, sub=3, t_catch0=BAR, t_g0=57.2)
